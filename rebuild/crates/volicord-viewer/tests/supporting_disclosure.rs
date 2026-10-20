//! Authored adverse controls through actual storage and live/offline consumers.
//! These are not host-authored explanation or human acceptance evidence.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_viewer::*;

#[test]
fn unrelated_pass_keeps_failure_visible_and_exact_details_disclosed(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input: Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    input["prior_checkpoint_count"] = json!(0);
    input["later_checkpoint_count"] = json!(0);
    input["unassociated_checkpoint"] = json!(false);
    input["decisions"] = json!([]);
    input["works"] = json!([
        {"key":"failed","title":"Identical missing publication","paths":[],"checkpoints":[
            {"key":"failure","work":"Paused","verification":"Failed","review":"NotRequested","acceptance":"NotRequested","state_change":null},
            {"key":"unrelated-pass","work":"Paused","verification":"Passed","review":"NotRequested","acceptance":"NotRequested","state_change":null}]},
        {"key":"no-checks","title":"Identical missing publication","paths":[],"checkpoints":[]}
    ]);
    let f = reading_fixture::fixture_scenario(input)?;
    let before = f.operations.canonical_basis(f.project)?;
    let privacy = f.operations.privacy_status(f.project)?;
    let adapter = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let mut cases = Vec::new();
    for (language, locale) in [("en", ViewerLocale::English), ("ko", ViewerLocale::Korean)] {
        for key in ["failed", "no-checks"] {
            let work = f.goals[key];
            let request = ViewerRequest {
                project_id: f.project,
                locale,
                view: ViewerView::Work { work: Some(work) },
                requested_language: language.into(),
                guarded_request: None,
            };
            let snapshot =
                adapter.render_snapshot(&request, TimestampMicros::from_unix_micros(123))?;
            let wire=format!("GET /?view=work&work={work}&language={language}&locale={language} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
            let mut bytes = Vec::new();
            server.serve_connection(&mut wire.as_bytes(), &mut bytes)?;
            let live = String::from_utf8(bytes)?;
            assert!(live.starts_with("HTTP/1.1 200"));
            for html in [&live, &snapshot.html] {
                let mut ids = std::collections::BTreeSet::new();
                for rest in html.split(" id=\"").skip(1) {
                    assert!(ids.insert(rest.split('"').next().ok_or("id")?));
                }
                for rest in html.split(" href=\"#").skip(1) {
                    assert!(ids.contains(rest.split('"').next().ok_or("target")?));
                }
                // Check the selected Work only: the full offline document contains both cases.
                let selected = html
                    .split(&format!("id=\"work-{work}\""))
                    .nth(1)
                    .ok_or("Work")?
                    .split("</article>")
                    .next()
                    .ok_or("article")?;
                let warning =
                    selected.contains("data-warning=\"failure\" data-severity=\"danger\"");
                assert_eq!(warning, key == "failed");
                assert!(selected.contains("<details class=\"reading-section verification-report\""));
                assert!(!selected.contains("verification-report\" open"));
                if key == "failed" {
                    assert!(selected.contains("class=\"failure-check-links\""));
                    assert!(
                        html.contains(&format!("data-checkpoint=\"{}\"", f.checkpoints["failure"]))
                    );
                    assert!(html.contains(&format!(
                        "data-checkpoint=\"{}\"",
                        f.checkpoints["unrelated-pass"]
                    )));
                }
            }
            cases.push(json!({"key":format!("{key}-{language}"),"runtime":f.operations.layout().root(),"project":f.project.to_string(),"work":work.to_string(),"locale":language,"language":language,"expected_failure":key=="failed","evidence_class":"authored adverse control"}));
        }
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    assert_eq!(privacy, f.operations.privacy_status(f.project)?);
    if let Some(root) = std::env::var_os("VOLICORD_DISCLOSURE_CONTROL_CAPTURE") {
        std::fs::create_dir_all(&root)?;
        std::fs::write(
            std::path::Path::new(&root).join("cases.json"),
            serde_json::to_vec_pretty(&cases)?,
        )?;
        let _retained = f._temporary.keep();
    }
    Ok(())
}
