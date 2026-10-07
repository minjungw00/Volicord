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
        assert_eq!(
            profile.projection.candidate_reads,
            usize::from(matches!(
                view,
                ViewerView::Overview | ViewerView::Work { .. }
            ))
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
                usize::from(matches!(name, "overview" | "work" | "snapshot"))
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
        assert!(overview.contains("id=\"overview\""));
        assert!(!overview.contains("Material limitations"));
        assert!(!overview.contains("id=\"diagnostics\""));
        assert_eq!(overview.matches("aria-current=\"page\"").count(), 1);
        assert!(!overview.contains("integrity diagnostics not requested"));
        for disclosure in overview
            .split("<details class=\"category-details\">")
            .skip(1)
        {
            let closed = disclosure
                .split("</details>")
                .next()
                .ok_or("count disclosure")?;
            assert!(!closed.contains(" open"));
            assert!(closed.contains("class=\"category-count\""));
        }
        assert!(!overview.contains("additional Works; open Work navigation"));
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
        assert!(
            works.contains("data-question=\"VerificationCoverage\""),
            "historical verification must remain ordinary"
        );
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

#[test]
fn scoped_code_keeps_work_meaning_direction_and_missing_flow_honest(
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
    let code = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", f.goals["older"]),
    );
    assert!(
        code.find("class=\"code-work-meaning\"")
            .ok_or("work meaning")?
            < code.find("data-diagram=").ok_or("diagram")?
    );
    assert!(code.contains("data-relationship-role=\"syntactic-call\""));
    assert!(code.contains("CallsSyntactically:"));
    assert!(code.contains("→"));
    assert!(code.contains("does not prove runtime or data flow"));
    let empty = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", f.goals["goal_only"]),
    );
    assert!(empty.contains("data-flow-state=\"NoResolvedCalls\""));
    assert!(!empty.contains("class=\"diagram-edge\""));
    assert!(!empty.contains("data-diagram=\"architecture-topology\""));
    assert!(empty.contains("volicord analyze"));
    let analysis = exchange(&server, "/?view=tools&tool=status");
    assert!(analysis.contains("data-analysis-state="));
    assert!(
        analysis
            .find("class=\"analysis-summary\"")
            .ok_or("status")?
            < analysis
                .find("class=\"runtime-diagnostics\"")
                .ok_or("audit")?
    );
    assert!(!analysis.contains("action=\"/analyze"));
    Ok(())
}

#[test]
fn disconnected_structure_is_grounded_without_inventing_call_flow(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    std::fs::create_dir(&repository)?;
    // Enough independent modules to exercise the bounded repository selection
    // without relying on a small source's file-to-module containment edge.
    for number in 0..80 {
        std::fs::write(
            repository.join(format!("isolated_{number:03}.py")),
            "# A module with no declarations or dependencies.\npass\n",
        )?;
    }
    let operations = LocalOperations::new(volicord_operations::RuntimeLayout::new(
        temporary.path().join("runtime"),
    )?);
    let project = operations
        .initialize_project("Disconnected structure", Some(&repository))?
        .project
        .id;
    operations.analyze(project, Vec::new())?;
    let projection = operations
        .project_projection_selected(project, volicord_projections::WorkSelector::Repository)?;
    assert!(!projection.repository_map.entities.is_empty());
    assert!(
        projection
            .repository_map
            .relations
            .iter()
            .all(|r| r.target_entity.is_none()),
        "selected entities={:?}; resolved={:?}",
        projection
            .repository_map
            .entities
            .iter()
            .map(|e| (&e.kind, &e.display_name))
            .collect::<Vec<_>>(),
        projection
            .repository_map
            .relations
            .iter()
            .map(|r| (&r.kind, &r.source_entity, &r.target_entity))
            .collect::<Vec<_>>()
    );
    let before = operations.canonical_basis(project)?;
    let viewer = ViewerAdapter::new(operations);
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let snapshot = viewer
            .render_snapshot(
                &ViewerRequest {
                    project_id: project,
                    locale,
                    view: ViewerView::Overview,
                    requested_language: if locale == ViewerLocale::English {
                        "en"
                    } else {
                        "ko"
                    }
                    .into(),
                    guarded_request: None,
                },
                volicord_context::TimestampMicros::from_unix_micros(1),
            )?
            .html;
        assert!(snapshot.contains("data-diagram=\"architecture-topology\""));
        assert!(snapshot.contains("class=\"diagram-node\""));
        assert!(!snapshot.contains("class=\"diagram-edge\""));
        assert!(snapshot.contains("data-flow-state=\"NoResolvedCalls\""));
        let flow = snapshot
            .split("data-diagram=\"flow-topology\"")
            .nth(1)
            .ok_or("flow figure")?
            .split("</figure>")
            .next()
            .ok_or("flow end")?;
        assert!(!flow.contains("diagram-node"));
        for attribute in snapshot.split("data-entity-id=\"").skip(1) {
            let identity = attribute.split('"').next().ok_or("entity identity")?;
            assert!(projection
                .repository_map
                .entities
                .iter()
                .any(|entity| entity.identity == identity));
            let fragment = identity
                .bytes()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert!(snapshot.contains(&format!("id=\"entity-{fragment}\"")));
        }
    }
    assert_eq!(before, viewer.operations().canonical_basis(project)?);
    Ok(())
}

#[test]
fn unrelated_analyzer_failure_stays_in_analysis_while_related_limits_remain_ordinary(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture()?;
    std::fs::write(f.repository.join("unrelated.rs"), "fn broken( {\n")?;
    f.operations.analyze(f.project, Vec::new())?;
    let before = f.operations.canonical_basis(f.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let work = exchange(&server, &format!("/?view=work&work={}", f.goals["older"]));
    assert!(!work.contains("unrelated.rs"));
    assert!(!work.contains("id=\"diagnostics\""));
    assert!(work.contains("data-question=\"VerificationCoverage\""));
    let code = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", f.goals["older"]),
    );
    assert!(!code.contains("unrelated.rs"));
    assert!(!code.contains("Material limitations"));
    let analysis = exchange(&server, "/?view=tools&tool=status");
    assert!(analysis.contains("unrelated.rs"));
    assert!(analysis.contains("class=\"global-diagnostics\""));
    // Actual source change invalidates related code; the answer limitation must
    // be adjacent to its explanation, outside any diagnostic disclosure.
    std::fs::write(
        f.repository.join("python/worker.py"),
        "def changed():\n    return 2\n",
    )?;
    let changed = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", f.goals["older"]),
    );
    let limits = changed
        .split("class=\"contextual-limits\"")
        .nth(1)
        .ok_or("related limitation")?
        .split("</aside>")
        .next()
        .ok_or("end")?;
    assert!(limits.contains("stale") || limits.contains("repository has changed"));
    assert!(limits.contains("Open Analysis"));
    assert!(!limits.contains("<details"));
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn offline_repository_code_uses_live_repository_limits_without_another_analysis_read(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{build_project_understanding, UnderstandingBound, WorkSelector};
    use volicord_repository_intelligence::{CapabilityState, Language};
    use volicord_viewer::CodeScope;

    let f = fixture()?;
    std::fs::create_dir_all(f.repository.join("outside_work"))?;
    std::fs::write(
        f.repository.join("outside_work/query.cpp"),
        "void broken( {\n",
    )?;
    // Force Repository topology truncation while the latest Work has no code seeds.
    for index in 0..80 {
        std::fs::write(
            f.repository
                .join(format!("outside_work/module_{index:03}.py")),
            "def entry():\n    return helper()\ndef helper():\n    return 1\n",
        )?;
    }
    f.operations.analyze(f.project, Vec::new())?;
    let canonical_before = f.operations.canonical_basis(f.project)?;
    let work = f
        .operations
        .project_projection_selected(f.project, WorkSelector::LatestWork)?;
    let repository = f
        .operations
        .project_projection_selected(f.project, WorkSelector::Repository)?;
    assert!(work
        .answer_capability_gaps
        .iter()
        .all(|g| g.language != Some(Language::Cpp)));
    let cpp = repository
        .answer_capability_gaps
        .iter()
        .find(|g| {
            g.language == Some(Language::Cpp)
                && matches!(g.state, CapabilityState::Partial | CapabilityState::Failed)
        })
        .ok_or("Repository C++ gap")?;
    assert!(
        repository.current_work_topology.omitted_entity_count
            > work.current_work_topology.omitted_entity_count
    );
    assert!(
        repository.current_work_topology.omitted_relation_count
            > work.current_work_topology.omitted_relation_count
    );
    let bound = UnderstandingBound {
        max_items_per_section: 32,
    };
    let live_understanding = build_project_understanding(&repository, bound);
    let offline_understanding = build_project_understanding(&work.for_repository_reading(), bound);
    assert_eq!(
        offline_understanding.architecture,
        live_understanding.architecture
    );
    for section in ["architecture.components", "architecture.relationships"] {
        let expected = live_understanding
            .omissions
            .iter()
            .find(|o| o.section == section)
            .ok_or("Repository omission")?;
        assert!(expected.omitted_count > 0);
        assert!(offline_understanding.omissions.contains(expected));
    }
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let code_section = |html: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(html
            .split("id=\"code\"")
            .nth(1)
            .ok_or("Code section")?
            .split("</section>")
            .next()
            .ok_or("Code section end")?
            .to_owned())
    };
    let contextual_limits = |html: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(html
            .split("<aside class=\"contextual-limits\">")
            .nth(1)
            .ok_or("Contextual limits")?
            .split("</aside>")
            .next()
            .ok_or("Limits end")?
            .to_owned())
    };
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let mut request = reading_request(
            f.project,
            ViewerView::Code {
                scope: CodeScope::Repository,
                entity: None,
            },
        );
        request.locale = locale;
        let live = viewer.render(&request, "test-token")?;
        request.view = ViewerView::Tools {
            tool: ViewerTool::Documents,
        };
        let (snapshot, profile) = viewer.render_snapshot_profiled(
            &request,
            volicord_context::TimestampMicros::from_unix_micros(123),
        )?;
        assert_eq!(profile.projection.analysis_snapshot_decodes, 1);
        assert_eq!(profile.document_generations, 4);
        let live_code = code_section(&live.html)?;
        let offline_code = code_section(&snapshot.html)?;
        assert!(offline_code.contains(&cpp.reason));
        // Only the navigation target differs between live and offline limit disclosures.
        assert_eq!(
            contextual_limits(&offline_code)?
                .split("<p class=\"next-action\">")
                .next(),
            contextual_limits(&live_code)?
                .split("<p class=\"next-action\">")
                .next(),
        );
        assert!(offline_code.contains("diagram-bounds"));
    }
    assert_eq!(canonical_before, f.operations.canonical_basis(f.project)?);
    let after = f
        .operations
        .project_projection_selected(f.project, WorkSelector::Repository)?;
    assert_eq!(repository.repository_analysis, after.repository_analysis);
    assert_eq!(repository.repository_map, after.repository_map);
    Ok(())
}

#[test]
fn decision_only_work_code_limits_reach_latest_and_exact_viewer_reads(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::WorkSelector;
    use volicord_repository_intelligence::{CapabilityState, Language};

    let mut scenario: serde_json::Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    scenario["prior_checkpoint_count"] = serde_json::json!(0);
    scenario["later_checkpoint_count"] = serde_json::json!(0);
    scenario["unassociated_checkpoint"] = serde_json::json!(false);
    for work in scenario["works"].as_array_mut().ok_or("Works")? {
        work["checkpoints"] = serde_json::json!([]);
        work["paths"] = serde_json::json!([]);
    }
    scenario["decisions"][0]["scope"] = serde_json::json!("goal_only");
    let f = reading_fixture::fixture_scenario(scenario)?;
    std::fs::write(f.repository.join("native/query.c"), "void broken( {\n")?;
    std::fs::write(f.repository.join("unrelated.rs"), "fn broken( {\n")?;
    f.operations.analyze(f.project, Vec::new())?;
    let before = f.operations.canonical_basis(f.project)?;
    let work = f.goals["goal_only"];
    assert!(before.checkpoint_history.is_empty());
    assert!(before
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("Goal")?
        .applicability
        .paths
        .is_empty());
    let latest = f
        .operations
        .project_projection_selected(f.project, WorkSelector::LatestWork)?;
    assert_eq!(
        latest.selected_work.as_ref().map(|w| w.work_item_id),
        Some(work)
    );
    assert!(latest
        .current_work_topology
        .entities
        .iter()
        .any(|e| e.locator == "native/query.c"));
    let gap = latest
        .answer_capability_gaps
        .iter()
        .find(|g| {
            g.language == Some(Language::C)
                && matches!(g.state, CapabilityState::Partial | CapabilityState::Failed)
        })
        .ok_or("Decision-only Work C limitation")?;
    assert!(!latest
        .answer_capability_gaps
        .iter()
        .any(|g| g.language == Some(Language::Rust)));
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    // The adapter's unbound Work scope reads LatestWork; HTTP Work-code links
    // deliberately require an exact identity and are exercised separately below.
    for scope in [
        volicord_viewer::CodeScope::Work(None),
        volicord_viewer::CodeScope::Work(Some(work)),
    ] {
        let request = reading_request(
            f.project,
            ViewerView::Code {
                scope,
                entity: None,
            },
        );
        let page = viewer.render(&request, "test-token")?;
        let limits = page
            .html
            .split("class=\"contextual-limits\"")
            .nth(1)
            .ok_or("ordinary Work code limitations")?
            .split("</aside>")
            .next()
            .ok_or("limits end")?;
        assert!(limits.contains(&gap.reason));
        assert!(limits.contains("Open Analysis"));
        assert!(!limits.contains("<details"));
        assert!(!limits.contains("unrelated.rs"));
    }
    let server = ViewerServer::new(
        viewer,
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let page = exchange(&server, &format!("/?view=code&scope=work&work={work}"));
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains(&gap.reason));
    let after = f
        .operations
        .project_projection_selected(f.project, WorkSelector::LatestWork)?;
    assert_eq!(latest.repository_analysis, after.repository_analysis);
    assert_eq!(latest.repository_map, after.repository_map);
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn code_reading_explains_source_operations_with_work_and_exact_entity_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
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
    for locale in ["en", "ko"] {
        let page = exchange(
            &server,
            &format!(
                "/?view=code&scope=work&work={}&locale={locale}",
                fixture.goals["older"]
            ),
        );
        assert!(page.contains("name.strip()"));
        assert!(page.contains("return format_name(name)"));
        assert!(page.contains("data-code-behavior="));
        assert!(page.contains("data-explanation-state=\"Current\""));
        assert!(
            page.contains("Body expressions and exact source evidence")
                || page.contains("본문 표현식과 정확한 소스 근거")
        );
        assert!(page.contains("ZeroBasedUtf8Byte"));
        assert!(page.contains("Source:"));
        assert!(page.contains("supporting static relations"));
        assert!(!page.contains("runtime sequence"));
    }
    let after_analysis = fixture.operations.canonical_basis(fixture.project)?;
    assert_eq!(before, after_analysis);
    let _ = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", fixture.goals["older"]),
    );
    assert_eq!(
        after_analysis,
        fixture.operations.canonical_basis(fixture.project)?
    );
    Ok(())
}

#[test]
fn polyglot_code_reading_keeps_rust_python_and_typescript_behavior_separate(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scenario: serde_json::Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    let work = scenario["works"]
        .as_array_mut()
        .ok_or("works")?
        .iter_mut()
        .find(|w| w["key"] == "older")
        .ok_or("older")?;
    work["paths"] = serde_json::json!([
        "src/lib.rs",
        "python/worker.py",
        "runtime/query_boundary.ts",
        "native/query.c"
    ]);
    let fixture = reading_fixture::fixture_scenario(scenario)?;
    std::fs::create_dir_all(fixture.repository.join("src"))?;
    std::fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn normalize(name: &str) -> String { name.trim().to_lowercase() }",
    )?;
    fixture.operations.analyze(fixture.project, Vec::new())?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
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
                "/?view=code&scope=work&work={}&locale={locale}",
                fixture.goals["older"]
            ),
        );
        for expression in [
            "name.trim().to_lowercase()",
            "return name.strip()",
            "return value;",
        ] {
            assert!(page.contains(expression), "{locale}: {expression}");
        }
        assert!(page.contains("data-explanation-state=\"Unsupported\"")); // C body reading, structural support retained.
        assert!(page.contains("Current"));
        assert!(
            page.contains("No observed") || page.contains("runtime/data flow were not observed")
        );
        assert!(page.contains("native/query.c"));
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn changed_removed_and_unavailable_sources_never_render_current_behavior(
) -> Result<(), Box<dyn std::error::Error>> {
    for case in ["changed", "removed", "unavailable"] {
        let fixture = fixture()?;
        let server = ViewerServer::new(
            ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
            fixture.project,
            ViewerLocale::English,
            ViewerView::Overview,
            "en".into(),
            "127.0.0.1:3219".parse()?,
        )?;
        let path = format!("/?view=code&scope=work&work={}", fixture.goals["older"]);
        let current = exchange(&server, &path);
        assert!(
            current.contains("format_name has a return expression")
                || current.contains("format_name: has a return expression")
                || current.contains("declares inputs: `(name)`")
        );
        match case {
            "changed" => std::fs::write(
                fixture.repository.join("python/worker.py"),
                "def format_name(name):\n    return name.upper()\n",
            )?,
            "removed" => std::fs::remove_file(fixture.repository.join("python/worker.py"))?,
            "unavailable" => std::fs::remove_dir_all(&fixture.repository)?,
            _ => unreachable!(),
        }
        let before = fixture.operations.canonical_basis(fixture.project)?;
        for locale in ["en", "ko"] {
            let historical = exchange(&server, &format!("{path}&locale={locale}"));
            assert!(historical.starts_with("HTTP/1.1 200"));
            assert!(
                !historical.contains("data-explanation-state=\"Current\""),
                "{case} / {locale}"
            );
            assert!(
                historical.contains(if case == "unavailable" {
                    "data-explanation-state=\"Unavailable\""
                } else {
                    "data-explanation-state=\"Stale\""
                }),
                "{case}"
            );
            assert!(!historical.contains("has a return expression"));
            assert!(
                historical.contains("Current behavior cannot be established")
                    || historical.contains("현재 동작을 확인할 수 없습니다")
            );
        }
        assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
        if case == "changed" {
            fixture.operations.analyze(fixture.project, Vec::new())?;
            let refreshed = exchange(&server, &path);
            assert!(refreshed.contains("data-explanation-state=\"Current\""));
            assert!(refreshed.contains("return name.upper()"));
            assert!(!refreshed.contains("return name.strip()"));
        }
    }
    Ok(())
}

#[test]
fn presentation_keeps_subject_navigation_and_readable_history_before_raw_evidence(
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
        let detail = exchange(
            &server,
            &format!(
                "/?view=work&work={}&locale={locale}&language=fr-CA",
                f.goals["older"]
            ),
        );
        assert!(detail.contains("<main id=\"viewer-content\" tabindex=\"-1\">"));
        assert!(detail.contains("class=\"subject-navigation\""));
        let nav = detail
            .split("<nav aria-label=\"Viewer\">")
            .nth(1)
            .ok_or("nav")?
            .split("</nav>")
            .next()
            .ok_or("nav end")?;
        assert!(nav.contains(&format!(
            "view=work&amp;work={}&amp;locale={locale}&amp;language=fr-CA",
            f.goals["older"]
        )));
        assert!(nav.contains(&format!("scope=work&amp;work={}", f.goals["older"])));
        let history = detail
            .split("class=\"verification-timeline\"")
            .nth(1)
            .ok_or("timeline")?;
        assert!(history.contains(if locale == "en" {
            "Verification"
        } else {
            "검증"
        }));
        assert!(
            history.find("<li>").ok_or("history item")?
                < history.find("<pre>").ok_or("exact history")?
        );
        assert!(detail.contains("data-reading-role=\"audit-history\""));
        let overview = exchange(&server, &format!("/?view=overview&locale={locale}"));
        assert!(
            overview
                .find("class=\"project-purpose\"")
                .ok_or("purpose")?
                < overview.find("class=\"work-list\"").ok_or("works")?
        );
    }
    Ok(())
}

#[test]
fn language_controls_preserve_page_subject_and_arbitrary_request_without_writes(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture()?;
    f.operations.analyze(f.project, Vec::new())?;
    let p = f
        .operations
        .project_projection_selected(f.project, volicord_projections::WorkSelector::Repository)?;
    assert!(!p.repository_map.entities.is_empty());
    let before = f.operations.canonical_basis(f.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let cases = [
        "view=work&page=1".to_string(),
        "view=decisions&page=1".into(),
        format!("view=work&work={}", f.goals["older"]),
        format!("view=decisions&decision={}", f.decisions["explicit"]),
    ];
    for fields in cases {
        let page = exchange(&server, &format!("/?{fields}&locale=ko&language=fr-CA"));
        assert!(page.starts_with("HTTP/1.1 200"));
        let settings = page
            .split("class=\"reading-settings\"")
            .nth(1)
            .ok_or("settings")?;
        assert!(settings.contains("method=\"get\" action=\"/\""));
        assert!(settings.contains("name=\"language\" value=\"fr-CA\""));
        assert!(settings.contains(&format!(
            "{}&amp;locale=en&amp;language=fr-CA",
            fields.replace('&', "&amp;")
        )));
        assert!(settings.contains("내용을 생성하거나 번역하지 않습니다"));
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}
