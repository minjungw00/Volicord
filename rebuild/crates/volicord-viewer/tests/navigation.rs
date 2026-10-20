//! Public destinations must connect genuine Project-scoped records, without writes.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerRequest, ViewerView};

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
        assert!(!overview.contains("class=\"project-navigation\""));
        assert!(overview.contains("view=repository"));
        assert!(!overview.contains("view=decisions"));
        let masthead = overview.split("</header>").next().ok_or("header")?;
        assert_eq!(masthead.matches("<nav ").count(), 1);
        assert!(!masthead.contains("Read-only"));
        assert!(!masthead.contains("읽기 전용"));
        assert!(!overview.contains("<form"));
        request.view = ViewerView::Work {
            work: Some(f.goals["older"]),
        };
        let work = viewer.render(&request, "test-authenticity")?.html;
        let opening = work.split("class=\"audit\"").next().ok_or("opening")?;
        assert!(opening.contains(&f.decisions["explicit"].to_string()));
        assert!(!work.contains(&f.decisions["other_work"].to_string()));
        assert!(work.contains("DECISION DOCUMENT"));
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
    let answers = operations.read_repository_explanation(project, "en")?;
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
        assert!(
            doc.find("data-question=\"RepositoryScopeSummary\"")
                .ok_or("short ordinary scope")?
                < doc.find("class=\"document-nav\"").ok_or("reading links")?
        );
        assert_eq!(doc.matches("class=\"document-nav\"").count(), 1);
        let body = doc.split("class=\"audit\"").next().ok_or("reading body")?;
        assert!(!body.contains("data-question=\"RepositoryScope\""));
        assert!(!body.contains("data-question=\"RepositoryCoverage\""));
        assert!(doc.contains(fact("RepositoryScope")?));
        assert!(doc.contains(fact("RepositoryCoverage")?));
        assert!(!doc.contains("Inspect exact states"));
        assert!(!doc.contains("Read the explanation's stated limits"));
        let nav = doc
            .split("class=\"document-nav\"")
            .nth(1)
            .ok_or("nav")?
            .split("</nav>")
            .next()
            .ok_or("nav end")?;
        assert!(!nav.contains("Original instructions"));
        assert!(!nav.contains("Verification report"));
        for destination in nav
            .split("href=\"#")
            .skip(1)
            .filter_map(|s| s.split('"').next())
        {
            assert!(
                doc.contains(&format!("id=\"{destination}\"")),
                "missing actual section {destination}"
            );
        }
        assert!(doc.contains("Full inventory and coverage path lists are not embedded"));
        assert!(doc.contains("repository explain evidence --analysis"));
        assert!(!doc.contains("<li>legacy</li>"));
        assert!(!doc.contains("unexamined = True"));
        assert!(doc.contains("Included, excluded and unexamined boundaries"));
    }
    assert_eq!(before, viewer.operations().canonical_basis(project)?);
    // A missing dependency may not preserve a plausible scope card as current explanation.
    viewer
        .operations()
        .delete_repository_source_observation(project, analysis, source)?;
    let unavailable = viewer
        .operations()
        .read_repository_explanation(project, "en")?;
    assert!(unavailable.provenance.is_none());
    assert!(!unavailable
        .facts
        .iter()
        .any(|a| a.question == "RepositoryScope"));
    Ok(())
}
