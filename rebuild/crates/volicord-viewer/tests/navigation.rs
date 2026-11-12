//! Public destinations must connect genuine Project-scoped records, without writes.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerRequest, ViewerView};

#[test]
fn catalogue_does_not_count_unrequested_history_as_omitted_content(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture()?;
    let before = f.operations.canonical_basis(f.project)?;
    let (p, _) = f.operations.project_catalogue_projection_profiled(
        f.project,
        &volicord_projections::WorkCatalogueQuery::default(),
        "en",
    )?;
    assert_eq!(p.work_count, 4);
    assert_eq!(
        p.work_catalogue.as_ref().ok_or("catalogue")?.matched_count,
        4
    );
    assert!(p.work_history.is_empty());
    assert_eq!(p.work_read_cost.materialized_works, 0);
    assert!(
        !p.issues
            .iter()
            .any(|issue| issue.affected_scope == "work_history"),
        "a card-only request did not ask for history bodies"
    );
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn filtered_pages_report_only_the_matching_suffix_as_omitted_content(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    let mut scenario: serde_json::Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    scenario["works"][1]["checkpoints"][0]["work"] = "Completed".into();
    let f = reading_fixture::fixture_scenario(scenario)?;
    let before = f.operations.canonical_basis(f.project)?;
    let candidates = f.operations.candidate_basis(f.project)?;
    let mut matching = [f.goals["older"], f.goals["same_title"]];
    matching.sort();
    for page in 0..=2 {
        let p = build_project_projection(ProjectProjectionInputs {
            canonical: &before,
            selection: WorkSelector::Repository,
            detail: ProjectionDetail {
                work_page: page,
                work_state: Some(UnderstandingWorkState::Completed),
                ..Default::default()
            },
            requirements: ProjectionReadRequirements::default(),
            metadata: &[],
            analyses: &[],
            analysis_issues: &[],
            applicability: volicord_inquiry::ApplicabilityQuery {
                project_id: f.project,
                paths: Vec::new(),
                components: Vec::new(),
                work_contexts: Vec::new(),
                current_assumptions: Vec::new(),
                met_revisit_triggers: Vec::new(),
            },
            candidates: CandidateProjectionInput::Available(&candidates),
            candidate_content_access: CandidateContentAccess::PolicyWithheld,
            observed_at: before.project.updated_at,
            bound: ProjectionBound {
                max_items_per_section: 1,
            },
        })?;
        assert_eq!(p.work_count, 4);
        assert_eq!(
            p.work_history
                .iter()
                .map(|w| w.work_item_id)
                .collect::<Vec<_>>(),
            matching.get(page).copied().into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            p.issues
                .iter()
                .filter(|issue| issue.affected_scope == "work_history")
                .map(|issue| issue.omitted_count)
                .sum::<usize>(),
            usize::from(page == 0)
        );
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn filtered_work_scope_does_not_report_other_states_as_omitted_content(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{ProjectionDetail, UnderstandingWorkState, WorkSelector};
    let f = reading_fixture::fixture()?;
    let before = f.operations.canonical_basis(f.project)?;
    for (selector, state, expected) in [
        (
            WorkSelector::Repository,
            UnderstandingWorkState::Completed,
            Some(f.goals["older"]),
        ),
        (
            WorkSelector::ExactWork(f.goals["older"]),
            UnderstandingWorkState::Completed,
            Some(f.goals["older"]),
        ),
        (
            WorkSelector::ExactWork(f.goals["older"]),
            UnderstandingWorkState::Paused,
            None,
        ),
        (
            WorkSelector::Repository,
            UnderstandingWorkState::Abandoned,
            None,
        ),
    ] {
        let (p, _) = f.operations.project_projection_detail_profiled(
            f.project,
            selector,
            ProjectionDetail {
                work_state: Some(state),
                ..Default::default()
            },
        )?;
        assert_eq!(p.work_count, 4);
        assert_eq!(
            p.work_history
                .iter()
                .map(|w| w.work_item_id)
                .collect::<Vec<_>>(),
            expected.into_iter().collect::<Vec<_>>()
        );
        assert!(
            !p.issues
                .iter()
                .any(|issue| issue.affected_scope == "work_history"),
            "{selector:?}/{state:?}: the complete matching scope has no omitted history"
        );
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn selected_work_keeps_project_navigation_totals_without_other_work_bodies(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture()?;
    let before = f.operations.canonical_basis(f.project)?;
    let selected = f.operations.project_projection_selected(
        f.project,
        volicord_projections::WorkSelector::ExactWork(f.goals["older"]),
    )?;
    assert_eq!(selected.work_count, 4);
    assert_eq!(selected.work_history.len(), 1);
    assert_eq!(selected.work_history[0].work_item_id, f.goals["older"]);
    assert_eq!(selected.work_read_cost.classified_works, 4);
    assert_eq!(selected.work_read_cost.indexed_checkpoints, 164);
    assert_eq!(selected.work_read_cost.materialized_works, 1);
    assert!(
        !selected
            .issues
            .iter()
            .any(|issue| issue.affected_scope == "work_history"),
        "other Works intentionally outside exact reading are not omitted payload"
    );
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    // The fixture has one completed, one in-progress, one paused and one open Work.
    // Equal titles and the long distractor history must not change those identities.
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        for view in [
            ViewerView::Overview,
            ViewerView::RepositoryExplanation,
            ViewerView::Work { work: None },
            ViewerView::Work {
                work: Some(f.goals["older"]),
            },
            ViewerView::Work {
                work: Some(f.goals["goal_only"]),
            },
        ] {
            let html = viewer
                .render(
                    &ViewerRequest {
                        project_id: f.project,
                        locale,
                        requested_language: "en".into(),
                        view: view.clone(),
                        guarded_request: None,
                    },
                    "project-totals",
                )?
                .html;
            let navigation = html
                .split("class=\"project-navigation\"")
                .nth(1)
                .ok_or("navigation")?
                .split("</nav>")
                .next()
                .ok_or("navigation end")?;
            assert!(
                navigation.contains("<span class=\"nav-count\">4</span>"),
                "{view:?}: Project total"
            );
            let states = html
                .split("class=\"rail-states\"")
                .nth(1)
                .ok_or("states")?
                .split("</div>")
                .next()
                .ok_or("states end")?;
            for state in ["in_progress", "paused", "completed", "open"] {
                let row = states
                    .split(&format!("data-state=\"{state}\""))
                    .nth(1)
                    .ok_or("state shortcut")?
                    .split("</a>")
                    .next()
                    .ok_or("shortcut end")?;
                assert!(
                    row.contains("<span class=\"nav-count\">1</span>"),
                    "{view:?}: {state}"
                );
            }
            if matches!(view, ViewerView::Work { work: Some(_) }) {
                assert!(
                    !html.contains(&f.decisions["other_work"].to_string()),
                    "other Work Decision stays scoped"
                );
            }
        }
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn native_snapshot_context_is_independent_of_the_export_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture()?;
    let before = f.operations.canonical_basis(f.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        for view in [
            ViewerView::RepositoryExplanation,
            ViewerView::Work {
                work: Some(f.goals["older"]),
            },
        ] {
            let request = ViewerRequest {
                project_id: f.project,
                locale,
                requested_language: "en".into(),
                view,
                guarded_request: None,
            };
            let snapshot = viewer
                .render_snapshot(&request, TimestampMicros::from_unix_micros(123))?
                .html;
            let header = snapshot
                .split("class=\"mobile-location\"")
                .nth(1)
                .ok_or("native location")?
                .split("</summary>")
                .next()
                .ok_or("header end")?;
            for kind in ["overview", "repository", "work", "support"] {
                assert_eq!(
                    header
                        .matches(&format!("data-page-context=\"{kind}\""))
                        .count(),
                    1
                );
            }
            let nav = snapshot
                .split("class=\"project-navigation\"")
                .nth(1)
                .ok_or("navigation")?
                .split("</nav>")
                .next()
                .ok_or("navigation end")?;
            assert_eq!(nav.matches("class=\"nav-link\"").count(), 3);
            assert_eq!(nav.matches("class=\"native-current\"").count(), 3);
            assert!(
                !nav.contains("aria-current=\"page\""),
                "a frozen request cannot select a native fragment destination"
            );
            assert!(!snapshot.contains("<script"));
            assert!(
                !snapshot.contains("<button"),
                "native disclosure needs no live-only controls"
            );
            assert!(!snapshot.contains("<form"));
            assert!(snapshot.contains("<summary class=\"mobile-header\"><span class=\"menu-label\">"),"the summary's accessible name must include its visible Project and native location");
            let live = viewer.render(&request, "test")?.html;
            assert!(live.contains("<button class=\"rail-close\""));
            assert!(live.contains("<button class=\"nav-backdrop\""));
            assert!(
                !live.contains("data-page-context=\""),
                "live location remains request-owned"
            );
        }
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn project_destinations_and_exact_related_records_are_readable_without_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture()?;
    let before = f.operations.canonical_basis(f.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let mut request = ViewerRequest {
            project_id: f.project,
            locale,
            requested_language: "en".into(),
            view: ViewerView::Overview,
            guarded_request: None,
        };
        let overview = viewer.render(&request, "test-authenticity")?.html;
        assert!(!overview.contains("class=\"repository-entry\""));
        assert!(!overview.contains("RECENT WORK"));
        assert!(!overview.contains("Project Decisions"));
        assert!(overview.contains("class=\"project-navigation\""));
        assert!(overview.contains("view=repository"));
        assert!(!overview.contains("view=decisions"));
        let navigation = overview
            .split("class=\"project-navigation\"")
            .nth(1)
            .ok_or("navigation")?
            .split("</nav>")
            .next()
            .ok_or("navigation end")?;
        assert_eq!(navigation.matches("class=\"nav-link\"").count(), 3);
        assert!(!navigation.contains("Read-only"));
        assert!(!navigation.contains("읽기 전용"));
        assert!(!overview.contains("class=\"document-nav\""));
        assert!(!overview.contains("<form"));
        request.view = ViewerView::Work {
            work: Some(f.goals["older"]),
        };
        let work = viewer.render(&request, "test-authenticity")?.html;
        let opening = work.split("class=\"audit\"").next().ok_or("opening")?;
        assert!(opening.contains(&f.decisions["explicit"].to_string()));
        assert!(!work.contains(&f.decisions["other_work"].to_string()));
        let decision_heading = match locale {
            ViewerLocale::English => "Related Decisions",
            ViewerLocale::Korean => "관련 결정",
        };
        assert!(work.contains(decision_heading));
        assert_eq!(work.matches("class=\"document-nav\"").count(), 1);
        assert_eq!(work.matches("<article class=\"doc\"").count(), 1);
        assert!(work.contains("data-toc-entry"));
        assert!(!work.contains("DECISION DOCUMENT"));
        assert!(!work.contains("판단 문서"));
        assert!(work.contains("class=\"recorded-choice") || work.contains("recorded-choice"));
        let snapshot = viewer
            .render_snapshot(&request, TimestampMicros::from_unix_micros(123))?
            .html;
        for id in [
            "analysis-tool",
            "documents-tool",
            "memory-tool",
            "evidence-tool",
            "decision-reading",
            "code",
            "repository-code",
        ] {
            assert!(!snapshot.contains(&format!("id=\"{id}\"")));
            assert!(!snapshot.contains(&format!("href=\"#{id}\"")));
        }
        assert!(snapshot.contains("id=\"repository-explanation\""));
        assert!(!snapshot.contains("<form"));
        assert!(!snapshot.contains("<script"));
        assert!(!snapshot.contains("test-authenticity"));
        assert!(snapshot.contains(decision_heading));
        assert!(!snapshot.contains("DECISION DOCUMENT"));
        assert!(!snapshot.contains("판단 문서"));
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn workless_project_keeps_all_three_reader_destinations_reachable(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let operations = LocalOperations::new(volicord_operations::RuntimeLayout::new(
        root.path().join("runtime"),
    )?);
    let project = operations.initialize_project("No Work", None)?.project.id;
    let viewer = ViewerAdapter::new(operations);
    let mut request = ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        requested_language: "en".into(),
        view: ViewerView::Overview,
        guarded_request: None,
    };
    let overview = viewer.render(&request, "test")?.html;
    assert!(overview.contains("No Project purpose has been recorded"));
    assert!(overview.contains("No Work has been recorded"));
    assert!(overview.contains("view=repository"));
    for view in [
        ViewerView::RepositoryExplanation,
        ViewerView::Work { work: None },
    ] {
        request.view = view;
        assert!(!viewer.render(&request, "test")?.html.contains("tools-menu"));
    }
    Ok(())
}

#[test]
fn repository_scope_precedes_explanation_with_exact_boundaries_in_supporting_records(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_operations::{
        RepositoryClaimRange, RepositoryExplanationDraft, RepositoryFileObservationRequest,
        RepositoryReaderUnit, RuntimeLayout,
    };
    use volicord_projections::*;
    let root = tempfile::tempdir()?;
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("rebuild/crates/relay"))?;
    std::fs::create_dir_all(repo.join("legacy"))?;
    let code = "def deliver(value):\n    return value\n";
    std::fs::write(repo.join("rebuild/crates/relay/main.py"), code)?;
    std::fs::write(repo.join("legacy/old.py"), "unexamined = True\n")?;
    let operations = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = operations
        .initialize_project("Bounded repository", Some(&repo))?
        .project
        .id;
    let analysis = operations
        .analyze(project, vec!["legacy".into()])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let observation = operations.observe_repository_file(RepositoryFileObservationRequest {
        project_id: project,
        analysis_snapshot_id: analysis,
        path: "rebuild/crates/relay/main.py".into(),
        byte_range: None,
        retain_seconds: 3600,
    })?;
    let source_hex = observation["source_id"].as_str().ok_or("Source identity")?;
    let mut source_bytes = [0; 16];
    for (index, byte) in source_bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&source_hex[index * 2..index * 2 + 2], 16)?;
    }
    let source = volicord_context::SourceId::from_bytes(source_bytes);
    let plan =
        operations.prepare_explanation(project, ExplanationSubject::Repository(analysis), "en")?;
    let realization = operations.compose_repository_explanation(
        project,
        analysis,
        RepositoryExplanationDraft {
            diagrams: Vec::new(),
            plan_fingerprint: plan.fingerprint,
            language: "en".into(),
            generator: ExplanationGenerator {
                host: "independent-scope-control".into(),
                session: "test".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![RepositoryReaderUnit {
                question: ExplanationQuestion::Observation,
                text: "The bounded relay function returns its argument.".into(),
                evidence_keys: vec!["repository_basis".into()],
                sources: vec![RepositoryClaimRange {
                    source_id: source,
                    start_byte: 0,
                    end_byte: code.len(),
                    claim: None,
                }],
            }],
        },
    )?;
    operations.record_explanation(
        project,
        ExplanationSubject::Repository(analysis),
        "en",
        realization,
    )?;
    let before = operations.canonical_basis(project)?;
    let answers = operations.read_repository_explanation(
        project,
        "en",
        volicord_projections::FixedLocale::English,
    )?;
    let fact = |key: &str| {
        answers
            .facts
            .iter()
            .find(|a| a.question == key)
            .map(|a| a.text.as_str())
            .ok_or("scope fact")
    };
    assert!(fact("RepositoryScope")?.contains("Cited Source area: rebuild/crates/relay (1 files)"));
    assert!(fact("RepositoryScope")?.contains("Observed inventory: rebuild/crates/relay (1 files)"));
    assert!(fact("RepositoryCoverage")?.contains("Excluded boundaries: 1"));
    assert!(
        fact("RepositoryCoverage")?.contains("Complete semantic understanding is not established")
    );
    let viewer = ViewerAdapter::new(operations);
    let request = ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        requested_language: "en".into(),
        view: ViewerView::RepositoryExplanation,
        guarded_request: None,
    };
    for html in [
        viewer.render(&request, "test")?.html,
        viewer
            .render_snapshot(&request, TimestampMicros::from_unix_micros(123))?
            .html,
    ] {
        let doc = html
            .split("id=\"repository-explanation\"")
            .nth(1)
            .ok_or("repository page")?;
        assert!(doc.contains("data-question=\"RepositoryScopeSummary\""));
        // A heading-free article does not receive invented supporting entries.
        assert!(doc.matches("class=\"document-nav\"").count() <= 1);
        assert!(doc.contains("data-reading-layout=\"grouped\""));
        for group in ["work", "checks", "records"] {
            assert!(doc.contains(&format!("data-reading-group=\"{group}\"")));
        }
        let body = doc
            .split("data-reading-group=\"records\"")
            .next()
            .ok_or("reading body")?;
        assert!(!body.contains("data-question=\"RepositoryScope\""));
        assert!(!body.contains("data-question=\"RepositoryCoverage\""));
        assert!(doc.contains(fact("RepositoryScope")?));
        assert!(doc.contains(fact("RepositoryCoverage")?));
        assert!(!doc.contains("Inspect exact states"));
        assert!(!doc.contains("Read the explanation's stated limits"));
        if let Some(nav) = doc.split("class=\"document-nav\"").nth(1) {
            let nav = nav.split("</nav>").next().ok_or("nav end")?;
            assert!(!nav.contains("Original instructions"));
            for destination in nav
                .split("href=\"#")
                .skip(1)
                .filter_map(|s| s.split('"').next())
            {
                assert!(
                    doc.contains(&format!("id=\"{destination}\"")),
                    "missing actual heading {destination}"
                );
            }
        }
    }
    assert_eq!(before, viewer.operations().canonical_basis(project)?);
    // A missing dependency may not preserve a plausible scope card as current explanation.
    viewer
        .operations()
        .delete_repository_source_observation(project, analysis, source)?;
    let unavailable = viewer.operations().read_repository_explanation(
        project,
        "en",
        volicord_projections::FixedLocale::English,
    )?;
    assert!(unavailable.provenance.is_none());
    assert!(!unavailable
        .facts
        .iter()
        .any(|a| a.question == "RepositoryScope"));
    Ok(())
}

#[test]
fn explicit_project_choice_keeps_equal_names_and_foreign_work_separate(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Cursor;
    use volicord_viewer::ViewerServer;
    let f = reading_fixture::fixture()?;
    let other = f
        .operations
        .initialize_project("Equal Project", None)?
        .project
        .id;
    let another = f
        .operations
        .initialize_project("Equal Project", None)?
        .project
        .id;
    let choices = f.operations.project_choices(0)?;
    assert!(choices.iter().any(|p| p.id == other));
    assert!(choices.iter().any(|p| p.id == another));
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let request = |query: &str| -> Result<String, Box<dyn std::error::Error>> {
        let mut input = Cursor::new(
            format!("GET /?{query} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n").into_bytes(),
        );
        let mut output = Vec::new();
        server.serve_connection(&mut input, &mut output)?;
        Ok(String::from_utf8(output)?)
    };
    let page = request(&format!("project={other}&view=overview"))?;
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains(&format!("project={other}&amp;locale=en")));
    assert!(page.contains("No Work has been recorded"));
    let absent = request(&format!(
        "project={other}&view=work&work={}",
        f.goals["older"]
    ))?;
    assert!(absent.starts_with("HTTP/1.1 404"));
    assert!(absent.contains("class=\"project-rail\""));
    assert!(!absent.contains("class=\"document-nav\""));
    assert!(!absent.contains("class=\"reading-section recorded-choice\""));
    for state in [
        "open",
        "in_progress",
        "paused",
        "completed",
        "abandoned",
        "superseded",
    ] {
        let page = request(&format!("view=work&state={state}"))?;
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(page.contains("data-destination=\"work\""));
    }
    assert!(
        request("project=00000000000000000000000000000000&view=overview")?
            .starts_with("HTTP/1.1 404")
    );
    Ok(())
}

#[test]
fn local_project_choice_reads_are_bounded_and_stably_paged(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let ops = LocalOperations::new(volicord_operations::RuntimeLayout::new(
        root.path().join("runtime"),
    )?);
    let mut ids = Vec::new();
    for _ in 0..35 {
        ids.push(ops.initialize_project("Repeated name", None)?.project.id);
    }
    ids.sort();
    let first = ops.project_choices(0)?;
    let last = ops.project_choices(32)?;
    assert_eq!(first.len(), 32);
    assert_eq!(last.len(), 3);
    assert_eq!(
        first.iter().chain(&last).map(|p| p.id).collect::<Vec<_>>(),
        ids
    );
    assert!(ops.project_choices(64)?.is_empty());
    Ok(())
}
