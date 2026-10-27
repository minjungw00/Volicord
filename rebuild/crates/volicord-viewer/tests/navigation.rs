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
        let (decision_heading, other_locale_heading) = match locale {
            ViewerLocale::English => ("DECISION DOCUMENT", "판단 문서"),
            ViewerLocale::Korean => ("판단 문서", "DECISION DOCUMENT"),
        };
        let decision_eyebrow = format!("<p class=\"eyebrow\">{decision_heading} · ");
        let other_locale_eyebrow = format!("<p class=\"eyebrow\">{other_locale_heading} · ");
        assert!(work.contains(&decision_eyebrow));
        assert!(!work.contains(&other_locale_eyebrow));
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
        assert!(snapshot.contains(&decision_eyebrow));
        assert!(!snapshot.contains(&other_locale_eyebrow));
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
        let body = doc.split("class=\"audit\"").next().ok_or("reading body")?;
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
