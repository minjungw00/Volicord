//! A corrupt local length header must fail as HTTP, never abort or report zero.
use std::io::Cursor;
use volicord_operations::{LocalOperations, RuntimeLayout, WorkTransition};
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerServer, ViewerView};

#[test]
fn corrupt_canonical_count_is_a_bounded_unavailable_read() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("Corrupt-count control", None)?
        .project
        .id;
    let goal = ops
        .transition_work(
            project,
            WorkTransition::StartNew {
                host: "test".into(),
                session: "test".into(),
                user_turn: "Read this Goal".into(),
                statement: "Read this Goal".into(),
            },
        )?
        .context_item_id;
    let db = rusqlite::Connection::open(ops.layout().canonical_store())?;
    db.execute(
        "UPDATE context_items SET applicability_paths=?1 WHERE id=?2",
        rusqlite::params![
            u64::MAX.to_be_bytes().as_slice(),
            goal.as_bytes().as_slice()
        ],
    )?;
    let server = ViewerServer::new(
        ViewerAdapter::new(ops),
        project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    for path in ["/inspection.json?class=work", "/inspection?class=work"] {
        let mut output = Vec::new();
        server.serve_connection(
            &mut Cursor::new(format!(
                "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n"
            )),
            &mut output,
        )?;
        let response = String::from_utf8(output)?;
        assert!(response.starts_with("HTTP/1.1 503"), "{response}");
        assert!(response.len() < 2048);
        assert!(!response.contains("permitted_match_count"));
        assert!(!response.contains("applicability_paths"));
        assert!(!response.contains("Read this Goal"));
    }
    Ok(())
}
