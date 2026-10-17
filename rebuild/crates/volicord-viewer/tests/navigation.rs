//! Public destinations must connect genuine Project-scoped records, without writes.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerRequest, ViewerTool, ViewerView};

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
        assert!(overview.contains("class=\"repository-entry\""));
        assert!(overview.contains("view=repository"));
        assert!(overview.contains("view=decisions"));
        assert!(overview.contains(&f.decisions["project"].to_string()));
        assert!(!overview.contains("<form"));
        let menu = overview
            .split("class=\"tools-menu\"")
            .nth(1)
            .ok_or("tools menu")?
            .split("</details>")
            .next()
            .ok_or("tools close")?;
        for key in ["status", "documents", "memory", "evidence"] {
            assert!(menu.contains(&format!("tool={key}")));
        }
        for tool in [
            ViewerTool::Status,
            ViewerTool::Documents,
            ViewerTool::Memory,
            ViewerTool::Evidence,
        ] {
            request.view = ViewerView::Tools { tool };
            let html = viewer.render(&request, "test-authenticity")?.html;
            assert!(html.contains("class=\"page-head\""));
            if tool == ViewerTool::Memory {
                assert!(html.contains("<form"));
                assert!(html.contains("test-authenticity"));
            }
            if matches!(tool, ViewerTool::Status | ViewerTool::Evidence) {
                assert!(!html.contains("<form"));
            }
        }
        request.view = ViewerView::Work {
            work: Some(f.goals["older"]),
        };
        let work = viewer.render(&request, "test-authenticity")?.html;
        let opening = work.split("class=\"audit\"").next().ok_or("opening")?;
        assert!(opening.contains(&f.decisions["explicit"].to_string()));
        assert!(!work.contains(&f.decisions["other_work"].to_string()));
        request.view = ViewerView::Decisions {
            decision: Some(f.decisions["explicit"]),
        };
        let decision = viewer.render(&request, "test-authenticity")?.html;
        assert!(decision.contains("DECISION DOCUMENT"));
        assert!(decision.contains("class=\"doc\""));
        assert!(decision.contains(&format!("work={}", f.goals["older"])));
        let snapshot = viewer
            .render_snapshot(&request, TimestampMicros::from_unix_micros(123))?
            .html;
        for id in [
            "analysis-tool",
            "documents-tool",
            "memory-tool",
            "evidence-tool",
            "repository-explanation",
            "decision-reading",
        ] {
            assert!(snapshot.contains(&format!("id=\"{id}\"")));
            assert!(snapshot.contains(&format!("href=\"#{id}\"")));
        }
        assert!(!snapshot.contains("<form"));
        assert!(!snapshot.contains("<script"));
        assert!(!snapshot.contains("test-authenticity"));
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn workless_project_keeps_repository_and_empty_decisions_reachable(
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
    request.view = ViewerView::Decisions { decision: None };
    assert!(viewer
        .render(&request, "test")?
        .html
        .contains("No Decisions have been recorded"));
    Ok(())
}
