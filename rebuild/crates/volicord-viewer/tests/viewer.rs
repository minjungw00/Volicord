//! Full reader boundary, including script-free publication and projected read cost.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_viewer::*;

#[test]
fn every_reader_and_snapshot_omits_retired_surfaces_without_loading_bulk_projections(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture()?;
    let canonical = f.operations.canonical_basis(f.project)?;
    let privacy = f.operations.privacy_status(f.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        for view in [
            ViewerView::Overview,
            ViewerView::RepositoryExplanation,
            ViewerView::Work { work: None },
            ViewerView::Work {
                work: Some(f.goals["older"]),
            },
        ] {
            let request = ViewerRequest {
                project_id: f.project,
                locale,
                view,
                requested_language: "en".into(),
                guarded_request: None,
            };
            for (page, profile) in [
                viewer.render_profiled(&request, "never-expose-this-token")?,
                viewer
                    .render_snapshot_profiled(&request, TimestampMicros::from_unix_micros(123))?,
            ] {
                assert_eq!(profile.projection.analysis_snapshot_decodes, 0);
                assert_eq!(profile.document_generations, 0);
                assert_eq!(profile.health_read, std::time::Duration::ZERO);
                assert_eq!(profile.privacy_read, std::time::Duration::ZERO);
                for forbidden in [
                    "tools-menu",
                    "view=tools",
                    "view=decisions",
                    "view=code",
                    "/inspection",
                    "/memory/",
                    "/documents/export",
                    "document-preview",
                    "id=\"analysis-tool\"",
                    "id=\"documents-tool\"",
                    "id=\"memory-tool\"",
                    "id=\"evidence-tool\"",
                    "id=\"repository-code\"",
                    "id=\"code\"",
                    "never-expose-this-token",
                ] {
                    assert!(!page.html.contains(forbidden), "{forbidden}");
                }
                assert!(!page.html.contains("<form"));
                let header = page.html.split("</header>").next().ok_or("header")?;
                assert_eq!(header.matches("class=\"nav-link\"").count(), 3);
                assert!(!header.contains("Read-only"));
            }
        }
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    assert_eq!(privacy, f.operations.privacy_status(f.project)?);
    Ok(())
}
