#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
mod reading_fixture;
use reading_fixture::fixture;
use volicord_context::{ContextItemId, DecisionId};
use volicord_operations::LocalOperations;
use volicord_viewer::{
    ViewerAdapter, ViewerLocale, ViewerRequest, ViewerServer, ViewerTool, ViewerView,
};

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
            .split("class=\"fact-states\"")
            .nth(1)
            .ok_or("states")?
            .split("</dl>")
            .next()
            .ok_or("end")?;
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
        let decision = exchange(
            &server,
            &format!(
                "/?view=decisions&decision={}&locale={locale}",
                fixture.decisions["explicit"]
            ),
        );
        assert!(decision.starts_with("HTTP/1.1 200"));
        assert!(decision.contains("user_rationale"));
        assert!(decision.contains("recommendation_rationale"));
    }
    let goal_only = exchange(
        &server,
        &format!("/?view=work&work={}", fixture.goals["goal_only"]),
    );
    assert!(goal_only.contains("Goal-only Work"));
    let code = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", fixture.goals["older"]),
    );
    assert!(code.contains("Selected Work context"));
    assert!(code.contains("python/worker.py"));
    assert!(code.contains("CallsSyntactically"));
    let without_purpose = exchange(&server, "/?view=overview");
    assert!(without_purpose.contains("Project Understanding"));
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
        "/?view=code",
        "/?view=code&scope=repository&work=bad",
        "/?view=tools&tool=invalid",
        "/?view=decisions&decision=bad",
        "/?view=code&scope=repository&entity=",
    ] {
        assert!(
            exchange(&server, path).starts_with("HTTP/1.1 400"),
            "{path}"
        );
    }
    for path in [
        format!("/?view=work&work={}", ContextItemId::from_bytes([250; 16])),
        format!("/?view=work&work={}", fixture.purpose),
        format!(
            "/?view=decisions&decision={}",
            DecisionId::from_bytes([250; 16])
        ),
        "/?view=code&scope=repository&entity=foreign-entity".into(),
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
        view: ViewerView::Tools {
            tool: ViewerTool::Documents,
        },
        requested_language: "en".into(),
        guarded_request: None,
    };
    let page = viewer
        .render_snapshot(
            &request,
            volicord_context::TimestampMicros::from_unix_micros(123),
        )?
        .html;
    let mut ids = std::collections::BTreeSet::new();
    for rest in page.split(" id=\"").skip(1) {
        assert!(ids.insert(rest.split('"').next().ok_or("id")?));
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
        .split("</section>")
        .next()
        .ok_or("section end")?;
    assert!(older.contains(&fixture.decisions["explicit"].to_string()));
    assert!(!older.contains(&fixture.decisions["other_work"].to_string()));
    assert_eq!(page.matches("class=\"document-preview\"").count(), 4);
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
    let omitted = goals
        .iter()
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
    assert_eq!(profile.projection.analysis_snapshot_decodes, 1);
    assert_eq!(profile.health_analysis_snapshot_decodes, 0);
    assert_eq!(profile.document_generations, 4);
    assert!(snapshot.html.contains("work_history"));

    Ok(())
}

#[test]
fn code_detail_selects_beyond_map_bounds_and_keeps_readable_real_endpoints(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    for n in 0..90 {
        std::fs::write(
            fixture
                .repository
                .join(format!("long_shared_module_prefix_{n:03}.py")),
            format!("def distinguishable_symbol_{n}():\n    return 1\n"),
        )?;
    }
    std::fs::write(
        fixture.repository.join("unsafe<&>.py"),
        "def safe():\n    return 1\n",
    )?;
    // More real Work-path seeds than the parent map can display. Omission is
    // not evidence of scope exclusion, regardless of generated identity order.
    let native = fixture.repository.join("native/query.c");
    let mut native_code = std::fs::read_to_string(&native)?;
    for n in 0..90 {
        native_code.push_str(&format!(
            "\nint scoped_symbol_{n}(void) {{ return {n}; }}\n"
        ));
    }
    std::fs::write(native, native_code)?;
    let analysis = fixture
        .operations
        .analyze(fixture.project, Vec::new())?
        .value
        .ok_or("analysis output")?
        .analysis;
    let projection = fixture.operations.project_projection_selected(
        fixture.project,
        volicord_projections::WorkSelector::Repository,
    )?;
    let outside = analysis
        .structural_facts
        .iter()
        .find(|f| {
            // Self-authored, disconnected paths are outside this Work; an
            // arbitrary omitted entity can instead be one of its valid seeds.
            f.entity.area.path.starts_with("long_shared_module_prefix_")
                && !projection
                    .repository_map
                    .entities
                    .iter()
                    .any(|e| e.identity == f.entity.identity)
        })
        .ok_or("no omitted entity")?;
    let encode = |id: &str| id.bytes().map(|b| format!("%{b:02X}")).collect::<String>();
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let page = exchange(
        &server,
        &format!(
            "/?view=code&scope=repository&entity={}",
            encode(&outside.entity.identity)
        ),
    );
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains("Diagram focus: selected entity"));
    assert!(page.contains(&format!(
        "data-selected=\"true\" data-entity-id=\"{}\"",
        outside.entity.identity
    )));
    let (focused_projection, focus) = fixture.operations.project_projection_detail_profiled(
        fixture.project,
        volicord_projections::WorkSelector::Repository,
        volicord_projections::ProjectionDetail {
            entity: Some(outside.entity.identity.clone()),
            ..Default::default()
        },
    )?;
    assert_eq!(focus.analysis_snapshot_decodes, 1);
    let understanding = volicord_projections::build_project_understanding(
        &focused_projection,
        volicord_projections::UnderstandingBound {
            max_items_per_section: 32,
        },
    );
    let allowed = focused_projection
        .selected_entity_neighbors
        .iter()
        .map(|e| e.identity.as_str())
        .chain(std::iter::once(outside.entity.identity.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(understanding
        .architecture
        .components
        .iter()
        .all(|e| allowed.contains(e.identity.as_str())));
    assert!(understanding
        .architecture
        .components
        .iter()
        .any(|e| e.identity == outside.entity.identity));
    assert!(understanding
        .architecture
        .relationships
        .iter()
        .all(|r| focused_projection
            .selected_entity_relations
            .iter()
            .any(|actual| actual == r)));
    assert!(!page.contains("selected by grounded connection to a current-work seed"));
    assert!(page.contains("Incoming relationships"));
    assert!(page.contains("Outgoing relationships"));
    assert!(page.contains("Source locator and retained range"));
    assert!(page.contains(&outside.entity.identity));
    let unsafe_entity = analysis
        .structural_facts
        .iter()
        .find(|f| f.entity.area.path == "unsafe<&>.py")
        .ok_or("unsafe locator fixture")?;
    let unsafe_page = exchange(
        &server,
        &format!(
            "/?view=code&scope=repository&entity={}",
            encode(&unsafe_entity.entity.identity)
        ),
    );
    assert!(unsafe_page.starts_with("HTTP/1.1 200"));
    assert!(unsafe_page.contains("unsafe&lt;&amp;&gt;.py"));
    assert!(!unsafe_page.contains("unsafe<&>.py"));
    let omitted_seed = analysis
        .structural_facts
        .iter()
        .find(|f| {
            f.entity.area.path == "native/query.c"
                && !projection
                    .repository_map
                    .entities
                    .iter()
                    .any(|e| e.identity == f.entity.identity)
        })
        .ok_or("no omitted Work seed")?;
    let seed_page = exchange(
        &server,
        &format!(
            "/?view=code&scope=work&work={}&entity={}",
            fixture.goals["older"],
            encode(&omitted_seed.entity.identity)
        ),
    );
    assert!(seed_page.starts_with("HTTP/1.1 200"));
    assert!(seed_page.contains(&format!(
        "data-selected=\"true\" data-entity-id=\"{}\"",
        omitted_seed.entity.identity
    )));
    assert!(exchange(
        &server,
        &format!(
            "/?view=code&scope=work&work={}&entity={}",
            fixture.goals["older"],
            encode(&outside.entity.identity)
        )
    )
    .starts_with("HTTP/1.1 404"));
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
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
        ViewerView::Decisions {
            decision: Some(fixture.decisions["explicit"]),
        },
        ViewerView::Work { work: None },
        ViewerView::Decisions { decision: None },
    ] {
        let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
        let request = reading_request(fixture.project, view.clone());
        let (page, profile) = viewer.render_profiled(&request, "test-token")?;
        assert_eq!(profile.projection.analysis_snapshot_decodes, 0);
        assert_eq!(profile.projection.analysis_metadata_decodes, 1);
        assert_eq!(profile.projection.candidate_reads, 0);
        assert_eq!(profile.health_analysis_snapshot_decodes, 0);
        assert_eq!(profile.document_generations, 0);
        assert_eq!(profile.document_preview, std::time::Duration::ZERO);
        assert!(!page.html.contains("Code bodies not requested"));
        assert!(page.html.contains("Repository and runtime diagnostics"));
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
    for (tool, graph, candidate, docs) in [
        (ViewerTool::Memory, 0, 1, 0),
        (ViewerTool::Documents, 1, 1, 4),
        (ViewerTool::Status, 1, 0, 0),
        (ViewerTool::Evidence, 1, 0, 0),
    ] {
        let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
        let (_, profile) = viewer.render_profiled(
            &reading_request(fixture.project, ViewerView::Tools { tool }),
            "test-token",
        )?;
        assert_eq!(profile.projection.analysis_snapshot_decodes, graph);
        assert_eq!(profile.projection.candidate_reads, candidate);
        assert_eq!(profile.health_analysis_snapshot_decodes, 0);
        assert_eq!(profile.document_generations, docs);
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn thin_reads_do_not_claim_graph_integrity_and_full_reads_keep_canonical_remainder(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let directory = fixture
        .operations
        .layout()
        .analysis_project_dir(fixture.project);
    let manifest = std::fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .ok_or("manifest")?;
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest)?)?;
    std::fs::write(
        directory.join("blobs").join(format!(
            "{}.values",
            value["values_blob"].as_str().ok_or("values blob")?
        )),
        b"corrupt graph body",
    )?;
    let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
    let (page, profile) = viewer.render_profiled(
        &reading_request(
            fixture.project,
            ViewerView::Work {
                work: Some(fixture.goals["older"]),
            },
        ),
        "test-token",
    )?;
    assert_eq!(profile.projection.analysis_snapshot_decodes, 0);
    assert_eq!(profile.projection.analysis_metadata_decodes, 1);
    assert!(!page
        .html
        .contains("Stored graph integrity diagnostics not requested"));
    assert!(page.html.contains("data-question=\"VerificationState\""));
    assert!(page.html.contains("Failed")); // independent canonical verification, still readable
    let (projection, profile) = viewer.operations().project_projection_read_profiled(
        fixture.project,
        WorkSelector::ExactWork(fixture.goals["older"]),
        ProjectionDetail::default(),
        ProjectionReadRequirements {
            code: true,
            inspection: false,
        },
    )?;
    assert_eq!(profile.analysis_snapshot_decodes, 1); // failed attempts count too
    assert_eq!(projection.sections.code, ReadSectionState::Unavailable);
    assert!(projection.selected_work.is_some());
    assert!(projection
        .issues
        .iter()
        .any(|i| i.kind == ProjectionIssueKind::FailedCapability));
    let (page, profile) = viewer.render_profiled(
        &reading_request(
            fixture.project,
            ViewerView::Code {
                scope: volicord_viewer::CodeScope::Work(Some(fixture.goals["older"])),
                entity: None,
            },
        ),
        "test-token",
    )?;
    assert_eq!(profile.projection.analysis_snapshot_decodes, 1);
    assert_eq!(profile.health_analysis_snapshot_decodes, 0);
    assert!(page.html.contains("Stored analysis is unavailable"));
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let unavailable = exchange(
        &server,
        &format!(
            "/?view=code&scope=work&work={}&entity=unverifiable-entity",
            fixture.goals["older"]
        ),
    );
    assert!(unavailable.starts_with("HTTP/1.1 200"));
    assert!(unavailable.contains("Entity detail unavailable"));
    assert!(unavailable.contains("cannot verify the requested entity identity"));
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
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
    let entity = analysis
        .structural_facts
        .iter()
        .find(|e| e.entity.area.path == "python/worker.py")
        .ok_or("worker entity")?
        .entity
        .identity
        .clone();
    let workloads = [
        ("overview", ViewerView::Overview),
        (
            "work",
            ViewerView::Work {
                work: Some(fixture.goals["older"]),
            },
        ),
        (
            "decision",
            ViewerView::Decisions {
                decision: Some(fixture.decisions["explicit"]),
            },
        ),
        (
            "code",
            ViewerView::Code {
                scope: volicord_viewer::CodeScope::Work(Some(fixture.goals["older"])),
                entity: Some(entity),
            },
        ),
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
            if name == "code" {
                assert!(page.html.contains("Diagram focus: selected entity"));
                assert!(page.html.contains("data-selected=\"true\""));
            }
            let full = name == "code" || name == "snapshot";
            assert_eq!(p.project_projection_passes, 1);
            assert_eq!(p.projection.analysis_snapshot_decodes, usize::from(full));
            assert_eq!(p.projection.analysis_metadata_decodes, usize::from(!full));
            assert_eq!(
                p.projection.candidate_reads,
                usize::from(name == "snapshot")
            );
            assert_eq!(p.health_analysis_snapshot_decodes, 0);
            assert_eq!(
                p.document_generations,
                if name == "snapshot" { 4 } else { 0 }
            );
            let metrics = serde_json::json!({"workload":name,"sample":sample,"adapter":"fresh-first-then-warm","total_us":p.total.as_micros(),"canonical_us":p.projection.canonical_read.as_micros(),"candidate_us":p.projection.candidate_read.as_micros(),"guarded_us":p.guarded_read.as_micros(),"analysis_us":p.projection.repository_analysis_read.as_micros(),"projection_us":p.projection.projection_build.as_micros(),"understanding_us":p.understanding.as_micros(),"health_us":p.health_read.as_micros(),"privacy_us":p.privacy_read.as_micros(),"documents_us":p.document_preview.as_micros(),"html_us":p.html_render.as_micros(),"projection_passes":p.project_projection_passes,"health_graph_decodes":p.health_analysis_snapshot_decodes,"graph_decodes":p.projection.analysis_snapshot_decodes,"metadata_decodes":p.projection.analysis_metadata_decodes,"candidate_reads":p.projection.candidate_reads,"documents":p.document_generations,"explanation_basis_preparations":p.projection.explanation_basis_preparations,"classified_works":p.projection.work_read_cost.classified_works,"materialized_works":p.projection.work_read_cost.materialized_works,"indexed_checkpoints":p.projection.work_read_cost.indexed_checkpoints,"materialized_checkpoints":p.projection.work_read_cost.materialized_checkpoints,"evidence_input_bytes":p.projection.work_read_cost.evidence_input_bytes,"bytes":page.html.len(),"html_sha256":format!("{:x}",Sha256::digest(page.html.as_bytes())),"canonical_basis_sha256":page.canonical_read_fingerprint,"requested_work_id":(name == "work" || name == "code").then(||fixture.goals["older"].to_string()),"language":request.requested_language,"fixed_locale":"en"});
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

#[test]
fn ordinary_hierarchy_separates_catalog_detail_and_audit_in_both_locales(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture()?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    for locale in ["en", "ko"] {
        let overview = exchange(
            &server,
            &format!("/?view=overview&locale={locale}&language={locale}"),
        );
        assert!(
            overview.find("id=\"overview\"").ok_or("Overview")?
                < overview.find("id=\"limitations\"").ok_or("limits")?
        );
        assert_eq!(overview.matches("aria-current=\"page\"").count(), 1);
        assert!(!overview.contains("integrity diagnostics not requested"));
        let list = exchange(
            &server,
            &format!("/?view=work&locale={locale}&language={locale}"),
        );
        let works = list
            .split("id=\"works\"")
            .nth(1)
            .ok_or("catalog")?
            .split("</section>")
            .next()
            .ok_or("catalog end")?;
        assert!(
            works.matches("work-summary").count() >= 3,
            "distinct Works must have separate cards"
        );
        assert!(works.contains("data-work-state=\"completed\""));
        assert!(works.contains("data-work-state=\"open\""));
        assert!(works.contains("data-question=\"RecordedNextStep\""));
        assert!(!works.contains("work-audit"));
        assert!(!works.contains("result-evidence"));
        let detail = exchange(
            &server,
            &format!(
                "/?view=work&work={}&locale={locale}&language={locale}",
                f.goals["older"]
            ),
        );
        assert!(
            detail.find("answer-unavailable").ok_or("answer")?
                < detail.find("class=\"work-audit\"").ok_or("audit")?
        );
        assert!(detail.contains(if locale == "en" {
            "Selected Work"
        } else {
            "선택한 작업"
        }));
        let decision = exchange(
            &server,
            &format!(
                "/?view=decisions&decision={}&locale={locale}&language={locale}",
                f.decisions["explicit"]
            ),
        );
        for label in if locale == "en" {
            [
                "Choice",
                "Alternatives and expected trade-offs",
                "Declared applicability",
                "User rationale is not recorded",
            ]
        } else {
            [
                "선택",
                "대안과 예상 절충",
                "선언된 적용 범위",
                "사용자 근거가 기록되지",
            ]
        } {
            assert!(decision.contains(label), "missing {label}");
        }
    }
    Ok(())
}
