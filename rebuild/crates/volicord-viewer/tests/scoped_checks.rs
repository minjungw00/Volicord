//! Original authored task/control plus actual command assertions; no human verdict.
#[path = "../../volicord-operations/tests/support/scoped_check_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use serde_json::{json, Value};
use std::{error::Error, fs};
use volicord_context::TimestampMicros;
use volicord_host::HostAdapter;
use volicord_operations::*;
use volicord_projections::*;
use volicord_viewer::*;

#[test]
fn scoped_comparison_has_equal_cli_mcp_public_live_and_offline_facts_without_an_article(
) -> Result<(), Box<dyn Error>> {
    let f = fixture::fixture()?;
    let before = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let report = f.ops.read_work_verification(f.project, f.work)?;
    let json_report = serde_json::to_value(&report)?;
    assert!(matches!(
        report.checks[1].retest_comparison,
        RetestComparison::Comparable {
            earlier_assertion: volicord_context::AssertionState::Failed,
            later_assertion: volicord_context::AssertionState::Passed,
            ..
        }
    ));
    let mut host = HostAdapter::new(LocalOperations::new(f.ops.layout().clone()));
    let mcp=host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"work_explanation","arguments":{"action":"verification","project_id":f.project.to_string(),"work_item_id":f.work.to_string()}}})).ok_or("MCP")?;
    assert_eq!(mcp["result"]["isError"], false, "{mcp}");
    assert_eq!(
        mcp["result"]["structuredContent"]["verification_report"],
        json_report
    );
    let projection = f
        .ops
        .project_projection_selected(f.project, WorkSelector::ExactWork(f.work))?;
    assert_eq!(
        projection
            .selected_work
            .ok_or("work")?
            .reading
            .verification_report,
        report
    );
    let mut captures = Vec::new();
    for (language, locale) in [("en", ViewerLocale::English), ("ko", ViewerLocale::Korean)] {
        let argv = vec![
            "--runtime".to_string(),
            f.ops.layout().root().to_string_lossy().into_owned(),
            "--project".into(),
            f.project.to_string(),
            "--json".into(),
            "work".into(),
            "verification".into(),
            "--work".into(),
            f.work.to_string(),
        ];
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run_cli(argv.iter().map(String::as_str), &mut stdout, &mut stderr),
            CliExit::SUCCESS,
            "{}",
            String::from_utf8_lossy(&stderr)
        );
        let cli: Value = serde_json::from_slice(&stdout)?;
        assert_eq!(cli["verification_report"], json_report);
        let adapter = ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()));
        let request = ViewerRequest {
            project_id: f.project,
            locale,
            view: ViewerView::Work { work: Some(f.work) },
            requested_language: language.into(),
            guarded_request: None,
        };
        let snapshot = adapter.render_snapshot(&request, TimestampMicros::from_unix_micros(123))?;
        let server = ViewerServer::new(
            ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone())),
            f.project,
            locale,
            ViewerView::Overview,
            language.into(),
            "127.0.0.1:3219".parse()?,
        )?;
        let request=format!("GET /?view=work&work={}&locale={language}&language={language} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n",f.work);
        let mut out = Vec::new();
        server.serve_connection(&mut request.as_bytes(), &mut out)?;
        let live = String::from_utf8(out)?;
        assert!(live.starts_with("HTTP/1.1 200"), "{live}");
        let label = if language == "en" {
            "Explicit comparable rerun"
        } else {
            "명시적 비교 가능 재실행"
        };
        for html in [&live, &snapshot.html] {
            assert!(html.contains(label));
            assert!(html.contains("value.equals.seven"));
            for check in &report.checks {
                assert!(html.contains(check.original_outcome.as_deref().ok_or("quote")?));
            }
            for receipt in [&f.failed, &f.passed] {
                assert_eq!(
                    html.matches(&format!("data-execution-source=\"{}\"", receipt.source_id))
                        .count(),
                    1
                );
            }
            // Inspect the independently serialized typed model, rather than using
            // rendered text as the oracle for expected original identities.
            let needle = if language == "en" {
                "Complete verification read model (JSON)"
            } else {
                "전체 검증 읽기 모델 (JSON)"
            };
            let start = html.find(needle).ok_or("JSON disclosure")?;
            let encoded = html[start..]
                .split_once("<pre>")
                .ok_or("JSON start")?
                .1
                .split_once("</pre>")
                .ok_or("JSON end")?
                .0;
            let decoded = encoded
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&");
            let embedded: Value = serde_json::from_str(&decoded)?;
            assert_eq!(embedded, json_report);
        }
        assert!(!snapshot.html.contains("<script"));
        assert!(!snapshot.html.contains("<form"));
        let destination = f.temp.path().join(format!("verification-{language}.html"));
        f.ops.publish_viewer_snapshot(
            &snapshot.html,
            &destination,
            f.project,
            &snapshot.canonical_read_fingerprint,
            &snapshot.explanations,
            &snapshot.verification_basis,
        )?;
        assert_eq!(fs::read_to_string(&destination)?, snapshot.html);
        captures.push((language, live, snapshot.html));
    }
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    assert_eq!(
        privacy.managed_derived,
        f.ops.privacy_status(f.project)?.managed_derived
    );
    if let Some(root) = std::env::var_os("VOLICORD_SCOPED_CHECK_JOURNEY_CAPTURE") {
        let root = std::path::PathBuf::from(root);
        assert!(root.is_absolute());
        fs::create_dir(&root)?;
        fs::write(root.join("classification.txt"),"Task-owned controlled execution. Actual failing and passing Python assertions, original exact Sources and Checkpoints. Explicit local capture. Trusted SessionStart capture Blocked (absent); ordinary-user capture, model/human review and acceptance unobserved.\n")?;
        fs::write(
            root.join("verification-report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        fs::write(root.join("mcp.json"), serde_json::to_vec_pretty(&mcp)?)?;
        for (language, live, offline) in captures {
            fs::write(root.join(format!("live-{language}.http")), live)?;
            fs::write(root.join(format!("offline-{language}.html")), offline)?;
        }
        for (label, receipt) in [("failure", &f.failed), ("rerun", &f.passed)] {
            fs::write(
                root.join(format!("original-{label}.json")),
                serde_json::to_vec_pretty(&f.ops.read_explanation_source(
                    f.project,
                    f.work,
                    receipt.source_id,
                )?)?,
            )?;
        }
    }
    Ok(())
}
