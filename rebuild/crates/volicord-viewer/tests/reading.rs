#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
mod reading_fixture;
use reading_fixture::fixture;
use volicord_context::ContextItemId;
use volicord_operations::LocalOperations;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerRequest, ViewerServer, ViewerView};

fn exchange(server: &ViewerServer, path: &str) -> String {
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
    let mut output = Vec::new();
    server
        .serve_connection(&mut request.as_bytes(), &mut output)
        .expect("HTTP");
    String::from_utf8(output).expect("UTF-8")
}

#[test]
fn exact_routes_keep_older_work_failed_states_and_decision_rationales(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
    let server = ViewerServer::new(
        viewer,
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    for locale in ["en", "ko"] {
        let page = exchange(
            &server,
            &format!(
                "/?view=work&work={}&locale={locale}&language=fr-CA",
                fixture.goals["older"]
            ),
        );
        assert!(page.starts_with("HTTP/1.1 200"));
        for key in ["change", "verification_only", "later_change"] {
            assert!(page.contains(&fixture.checkpoints[key].to_string()));
        }
        let states = page
            .split("class=\"record-facts\"")
            .nth(1)
            .ok_or("facts")?
            .split("</div>")
            .next()
            .ok_or("facts end")?;
        for expected in if locale == "en" {
            ["completed", "not run", "not requested", "rejected"]
        } else {
            ["완료", "실행하지 않음", "요청하지 않음", "거부됨"]
        } {
            assert!(
                states.contains(expected),
                "missing localized {expected}: {states}"
            );
        }
        assert!(page.contains(if locale == "en" { "failed" } else { "실패" }));
        assert!(
            page.contains("Interpretation has not been generated")
                || page.contains("해석이 아직 생성되지")
        );
        assert!(!page.contains(&fixture.decisions["other_work"].to_string()));
        assert!(page.contains("user_rationale"));
        assert!(page.contains("recommendation_rationale"));
    }
    let goal_only = exchange(
        &server,
        &format!("/?view=work&work={}", fixture.goals["goal_only"]),
    );
    assert!(goal_only.contains("Goal-only Work"));
    let without_purpose = exchange(&server, "/?view=overview");
    assert!(without_purpose.contains("PROJECT OVERVIEW"));
    assert!(!without_purpose.contains("request_authenticity"));
    assert!(fixture.repository.is_dir());
    assert!(before.context_items.iter().any(|c| c.id == fixture.purpose));
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn malformed_removed_and_foreign_selectors_never_fallback() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    for path in [
        "/?level=overview",
        "/?view=invalid",
        "/?view=work&work=bad",
        "/?view=overview&work=bad",
        "/?view=repository&work=bad",
        "/?view=repository&scope=work",
        "/?view=code",
        "/?view=code&scope=repository&work=bad",
        "/?view=tools&tool=invalid",
        "/?view=decisions&decision=bad",
        "/?view=code&scope=repository&entity=",
    ] {
        let expected = if ["/?view=tools", "/?view=decisions", "/?view=code"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            "HTTP/1.1 404"
        } else {
            "HTTP/1.1 400"
        };
        assert!(exchange(&server, path).starts_with(expected), "{path}");
    }
    for path in [
        format!("/?view=work&work={}", ContextItemId::from_bytes([250; 16])),
        format!("/?view=work&work={}", fixture.purpose),
    ] {
        assert!(
            exchange(&server, &path).starts_with("HTTP/1.1 404"),
            "{path}"
        );
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn bounded_whole_snapshot_has_only_unique_existing_fragment_targets(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
    let request = ViewerRequest {
        project_id: fixture.project,
        locale: ViewerLocale::English,
        view: ViewerView::Overview,
        requested_language: "en".into(),
        guarded_request: None,
    };
    let page = viewer
        .render_snapshot(
            &request,
            volicord_context::TimestampMicros::from_unix_micros(123),
        )?
        .html;
    assert!(fixture.goals.len() >= 3);
    for work in fixture.goals.values() {
        assert_eq!(
            page.matches(&format!("id=\"work-{work}\"")).count(),
            1,
            "same-title Works retain their independent snapshot destinations"
        );
    }
    let mut ids = std::collections::BTreeSet::new();
    for rest in page.split(" id=\"").skip(1) {
        let id = rest.split('"').next().ok_or("id")?;
        assert!(ids.insert(id), "duplicate {id}");
    }
    for rest in page.split(" href=\"").skip(1) {
        let href = rest.split('"').next().ok_or("href")?;
        assert!(href.starts_with('#'));
        assert!(ids.contains(&href[1..]), "{href}");
    }
    let older = page
        .split(&format!("id=\"work-{}\"", fixture.goals["older"]))
        .nth(1)
        .ok_or("older Work section")?
        .split("<section class=\"snapshot-page\"")
        .next()
        .ok_or("section end")?;
    assert!(older.contains(&fixture.decisions["explicit"].to_string()));
    assert!(!older.contains(&fixture.decisions["other_work"].to_string()));
    assert!(!page.contains("class=\"document-preview\""));
    for forbidden in [
        "<form",
        "<script",
        " src=",
        "request_authenticity",
        "href=\"/?",
        "href=\"http",
    ] {
        assert!(!page.contains(forbidden));
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn work_index_pages_reach_items_omitted_from_initial_lists(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::*;
    let fixture = fixture()?;
    let canonical = fixture.operations.canonical_basis(fixture.project)?;
    let mut store = Store::open(fixture.operations.layout().canonical_store())?;
    let source = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::CurrentHostUserTurn { .. }))
        .ok_or("user Source")?
        .source
        .id;
    let mut goals = Vec::new();
    for n in 0_u128..70 {
        goals.push(
            store
                .record_context_item(
                    OperationId::from_bytes((90_000 + n).to_le_bytes()),
                    fixture.project,
                    ContextItemDraft {
                        expected_project_revision: canonical.project.revision,
                        role: ContextItemRole::Goal,
                        statement: format!("Paged Goal {n}"),
                        provenance_role: StatementProvenanceRole::UserStatement,
                        author: Principal {
                            kind: PrincipalKind::User,
                            identity: "fixture".into(),
                        },
                        source_basis: vec![source],
                        applicability: ApplicabilityScope::default(),
                    },
                )?
                .value
                .id,
        );
    }
    drop(store);
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let first = exchange(&server, "/?view=work&locale=ko&language=fr-CA");
    let second = exchange(&server, "/?view=work&page=1&locale=ko&language=fr-CA");
    assert!(first.contains("page=1"));
    assert!(first.contains("language=fr-CA"));
    // Catalogue now uses recent recording order and eight-item pages.
    let omitted = goals
        .iter()
        .rev()
        .find(|id| !first.contains(&format!("work={id}")))
        .ok_or("all unexpectedly included")?;
    assert!(second.contains(&format!("work={omitted}")));
    assert!(exchange(&server, &format!("/?view=work&work={omitted}")).starts_with("HTTP/1.1 200"));
    for path in [
        "/?view=overview&page=1",
        "/?view=work&page=-1",
        "/?view=work&page=1000000",
        "/?view=work&page=1&work=bad",
    ] {
        assert!(exchange(&server, path).starts_with("HTTP/1.1 400"));
    }
    let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
    let (snapshot, profile) = viewer.render_snapshot_profiled(
        &reading_request(fixture.project, ViewerView::Overview),
        volicord_context::TimestampMicros::from_unix_micros(123),
    )?;
    assert_eq!(profile.project_projection_passes, 1);
    assert_eq!(profile.projection.analysis_snapshot_decodes, 0);
    assert_eq!(profile.health_analysis_snapshot_decodes, 0);
    assert_eq!(profile.document_generations, 0);
    assert!(snapshot.html.contains("id=\"works\""));

    Ok(())
}

fn reading_request(project: volicord_context::ProjectId, view: ViewerView) -> ViewerRequest {
    ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        view,
        requested_language: "en".into(),
        guarded_request: None,
    }
}

#[test]
fn requested_sections_preserve_metadata_and_refuse_incomplete_documents(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    for view in [
        ViewerView::Overview,
        ViewerView::Work {
            work: Some(fixture.goals["older"]),
        },
        ViewerView::Work { work: None },
    ] {
        let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
        let request = reading_request(fixture.project, view.clone());
        let (page, profile) = viewer.render_profiled(&request, "test-token")?;
        assert_eq!(profile.projection.analysis_snapshot_decodes, 0);
        assert_eq!(profile.projection.analysis_metadata_decodes, 1);
        assert_eq!(
            profile.projection.candidate_reads,
            usize::from(matches!(view, ViewerView::Work { work: Some(_) }))
        );
        assert_eq!(profile.health_analysis_snapshot_decodes, 0);
        assert_eq!(profile.document_generations, 0);
        assert_eq!(profile.document_preview, std::time::Duration::ZERO);
        assert!(!page.html.contains("Code bodies not requested"));
        assert!(!page.html.contains("id=\"diagnostics\""));
        assert!(!page.html.contains("Material limitations"));
        let (projection, _) = viewer.operations().project_projection_read_profiled(
            fixture.project,
            WorkSelector::ExactWork(fixture.goals["older"]),
            ProjectionDetail::default(),
            ProjectionReadRequirements {
                code: false,
                inspection: false,
            },
        )?;
        assert_eq!(projection.sections.code, ReadSectionState::NotRequested);
        assert_eq!(
            projection.sections.inspection,
            ReadSectionState::NotRequested
        );
        assert_eq!(
            projection.candidate_dependency,
            CandidateDependencyState::NotRequested
        );
        assert!(!projection.resume.snapshots.is_empty());
        assert!(!projection.overview.capability_reports.is_empty());
        assert_eq!(
            projection
                .selected_work
                .as_ref()
                .ok_or("Work")?
                .reading
                .code_availability,
            ReadingAvailability::NotRequested
        );
        let document = DocumentRequest {
            requested_language: "en".into(),
            fixed_locale: FixedLocale::English,
            generated_at: volicord_context::TimestampMicros::from_unix_micros(123),
            generator: GeneratorIdentity {
                generator: "test".into(),
                agent: None,
                model: None,
            },
            requested_destinations: Vec::new(),
        };
        assert!(viewer
            .operations()
            .documents_from_projection(&projection, &document)
            .is_err());
        assert!(
            prepare_narrative_plan(&projection, &document, DocumentKind::HandoffResume).is_err()
        );
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn repository_document_is_separate_from_overview_and_preserves_repository_selection(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture()?;
    let before = f.operations.canonical_basis(f.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let request = ViewerRequest {
        project_id: f.project,
        locale: ViewerLocale::Korean,
        view: ViewerView::RepositoryExplanation,
        requested_language: "fr-CA".into(),
        guarded_request: None,
    };
    let page = viewer.render(&request, "authenticity")?.html;
    assert!(page.contains("id=\"repository-explanation\""));
    assert!(
        !page.contains("data-work-id="),
        "repository document never selects a Work"
    );
    assert!(page.contains("class=\"doc-head\""));
    assert!(
        !page.contains("id=\"code\""),
        "structure inspection is a separate destination"
    );
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn requested_sections_on_large_repository() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    for index in 0..192 {
        std::fs::write(
            fixture.repository.join(format!("module_{index:03}.py")),
            format!("def function_{index:03}():\n    return {index}\n"),
        )?;
    }
    let analysis = fixture
        .operations
        .analyze(fixture.project, Vec::new())?
        .value
        .ok_or("analysis outcome")?
        .analysis;
    assert!(!analysis.structural_facts.is_empty());
    let workloads = [
        ("overview", ViewerView::Overview),
        (
            "work",
            ViewerView::Work {
                work: Some(fixture.goals["older"]),
            },
        ),
        ("repository", ViewerView::RepositoryExplanation),
        ("snapshot", ViewerView::Overview),
    ];
    let budgets: serde_json::Value = serde_json::from_str(include_str!(
        "../../../validation/end-to-end/multi-repository/viewer-read-budgets.json"
    ))?;
    let enforce_latency = std::env::var_os("VOLICORD_VIEWER_BUDGETS").is_some();
    let before = fixture.operations.canonical_basis(fixture.project)?;
    use sha2::{Digest, Sha256};
    for (name, view) in workloads {
        // Cold means fresh LocalOperations/Viewer adapter, not flushed OS caches.
        let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
        for sample in 0..9 {
            let request = reading_request(fixture.project, view.clone());
            let (page, p) = if name == "snapshot" {
                viewer.render_snapshot_profiled(
                    &request,
                    volicord_context::TimestampMicros::from_unix_micros(123),
                )?
            } else {
                viewer.render_profiled(&request, "test-token")?
            };
            assert_eq!(p.project_projection_passes, 1);
            assert_eq!(p.projection.analysis_snapshot_decodes, 0);
            assert_eq!(p.projection.analysis_metadata_decodes, 1);
            assert_eq!(
                p.projection.candidate_reads,
                usize::from(matches!(name, "work" | "snapshot"))
            );
            assert_eq!(p.health_analysis_snapshot_decodes, 0);
            assert_eq!(p.document_generations, 0);
            let metrics = serde_json::json!({"workload":name,"sample":sample,"adapter":"fresh-first-then-warm","total_us":p.total.as_micros(),"canonical_us":p.projection.canonical_read.as_micros(),"candidate_us":p.projection.candidate_read.as_micros(),"guarded_us":p.guarded_read.as_micros(),"analysis_us":p.projection.repository_analysis_read.as_micros(),"projection_us":p.projection.projection_build.as_micros(),"understanding_us":p.understanding.as_micros(),"health_us":p.health_read.as_micros(),"privacy_us":p.privacy_read.as_micros(),"documents_us":p.document_preview.as_micros(),"html_us":p.html_render.as_micros(),"projection_passes":p.project_projection_passes,"health_graph_decodes":p.health_analysis_snapshot_decodes,"graph_decodes":p.projection.analysis_snapshot_decodes,"metadata_decodes":p.projection.analysis_metadata_decodes,"candidate_reads":p.projection.candidate_reads,"documents":p.document_generations,"explanation_basis_preparations":p.projection.explanation_basis_preparations,"classified_works":p.projection.work_read_cost.classified_works,"materialized_works":p.projection.work_read_cost.materialized_works,"indexed_checkpoints":p.projection.work_read_cost.indexed_checkpoints,"materialized_checkpoints":p.projection.work_read_cost.materialized_checkpoints,"evidence_input_bytes":p.projection.work_read_cost.evidence_input_bytes,"bytes":page.html.len(),"html_sha256":format!("{:x}",Sha256::digest(page.html.as_bytes())),"canonical_basis_sha256":page.canonical_read_fingerprint,"requested_work_id":(name == "work").then(||fixture.goals["older"].to_string()),"language":request.requested_language,"fixed_locale":"en"});
            println!("VIEWER_READ_SAMPLE {metrics}");
            if enforce_latency {
                let limit = &budgets["ceilings_us"][name];
                for (stage, field) in [
                    ("total", "total_us"),
                    ("canonical", "canonical_us"),
                    ("analysis", "analysis_us"),
                    ("projection", "projection_us"),
                    ("documents", "documents_us"),
                    ("html", "html_us"),
                    ("candidate", "candidate_us"),
                    ("health", "health_us"),
                    ("privacy", "privacy_us"),
                    ("understanding", "understanding_us"),
                ] {
                    assert!(
                        metrics[field].as_u64().ok_or("sample value")?
                            <= limit[stage].as_u64().ok_or("stage ceiling")?,
                        "{name}/{sample} stage {stage}: {metrics}"
                    );
                }
            }
        }
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}
