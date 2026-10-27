//! Full reader boundary, including script-free publication and projected read cost.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_viewer::*;

#[test]
fn ordinary_readers_leave_unrequested_candidate_inspection_explicitly_unrequested(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{
        CandidateDependencyState, ProjectionDetail, ProjectionReadRequirements, ReadSectionState,
        WorkSelector,
    };
    for state in ["empty", "unavailable", "corrupt", "unsupported"] {
        let f = reading_fixture::fixture()?;
        let path = f.operations.layout().candidate_store();
        match state {
            "unavailable" => {
                std::fs::remove_file(&path)?;
                std::fs::create_dir(&path)?;
            }
            "corrupt" => {
                rusqlite::Connection::open(&path)?.execute("DROP TABLE candidates", [])?;
            }
            "unsupported" => {
                rusqlite::Connection::open(&path)?.execute(
                    "UPDATE metadata SET value = '999' WHERE key = 'schema_version'",
                    [],
                )?;
            }
            _ => assert!(f
                .operations
                .candidate_basis(f.project)?
                .candidates
                .is_empty()),
        }
        let canonical = f.operations.canonical_basis(f.project)?;
        let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
        let (projection, profile) = viewer.operations().project_projection_read_profiled(
            f.project,
            WorkSelector::Repository,
            ProjectionDetail::default(),
            ProjectionReadRequirements {
                code: false,
                inspection: false,
            },
        )?;
        assert_eq!(
            projection.candidate_dependency,
            CandidateDependencyState::NotRequested
        );
        assert_eq!(
            projection.sections.inspection,
            ReadSectionState::NotRequested
        );
        assert_eq!(profile.candidate_reads, 0);
        for view in [
            ViewerView::Overview,
            ViewerView::RepositoryExplanation,
            ViewerView::Work { work: None },
        ] {
            let request = ViewerRequest {
                project_id: f.project,
                locale: ViewerLocale::English,
                view,
                requested_language: "en".into(),
                guarded_request: None,
            };
            let (page, profile) = viewer.render_profiled(&request, "unused")?;
            assert_eq!(profile.projection.candidate_reads, 0, "{state}");
            assert!(!page.html.contains("/inspection"));
            assert!(!page.html.contains("view=tools"));
            assert_reader_forms(
                &page.html,
                matches!(request.view, ViewerView::Work { work: None }),
            );
        }
        assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    }
    Ok(())
}

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
                assert_reader_forms(
                    &page.html,
                    matches!(request.view, ViewerView::Work { work: None })
                        && page.html.contains("data-viewer-mode=\"live\""),
                );
                let header = page
                    .html
                    .split("class=\"project-navigation\"")
                    .nth(1)
                    .ok_or("navigation")?
                    .split("</nav>")
                    .next()
                    .ok_or("navigation end")?;
                assert_eq!(header.matches("class=\"nav-link\"").count(), 3);
                assert!(!header.contains("Read-only"));
            }
        }
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    assert_eq!(privacy, f.operations.privacy_status(f.project)?);
    Ok(())
}

// Retired management forms stay absent; the catalogue has one read-only GET search.
fn assert_reader_forms(html: &str, catalogue: bool) {
    let forms = html
        .split("<form")
        .skip(1)
        .map(|s| s.split('>').next().expect("form opening"))
        .collect::<Vec<_>>();
    assert_eq!(forms.len(), usize::from(catalogue));
    for form in forms {
        assert!(form.contains("method=\"get\""));
        assert!(form.contains("action=\"/\""));
        assert!(form.contains("role=\"search\""));
    }
}
