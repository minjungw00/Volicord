//! Independent canonical-state oracle; authored support, not Human usability evidence.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;

use serde_json::{json, Value};
use volicord_context::TimestampMicros;
use volicord_operations::LocalOperations;
use volicord_projections::{work_answers, FixedLocale, WorkSelector};
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerRequest, ViewerServer, ViewerView};

// Read text outside every disclosure, independent of production rendering helpers.
fn ordinary_text(html: &str) -> String {
    let mut depth = 0usize;
    let mut text = String::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        if depth == 0 {
            text.push_str(&rest[..start]);
        }
        rest = &rest[start + 1..];
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[..end];
        if tag.starts_with("details") {
            depth += 1;
        } else if tag == "/details" {
            depth -= 1;
        }
        rest = &rest[end + 1..];
    }
    if depth == 0 {
        text.push_str(rest);
    }
    text
}

fn check_primary(
    html: &str,
    verification: &str,
    review: &str,
    acceptance: &str,
    history: Option<&str>,
    micros: &[String],
) -> Result<(), &'static str> {
    let text = ordinary_text(html);
    for state in [verification, review, acceptance] {
        if text.matches(state).count() != 1 {
            return Err("current state missing or duplicated");
        }
    }
    if history.is_some_and(|summary| text.matches(summary).count() != 1) {
        return Err("historical summary missing or duplicated");
    }
    if micros.iter().any(|timestamp| text.contains(timestamp)) {
        return Err("raw timestamp in ordinary reading");
    }
    Ok(())
}

fn audit_body(html: &str) -> Result<&str, &'static str> {
    html.split("<details class=\"work-state-history\" data-reading-role=\"audit-history\">")
        .nth(1)
        .and_then(|tail| tail.split("</details>").next())
        .ok_or("closed audit history missing")
}

fn check_audit(html: &str, exact_records: &[String]) -> Result<(), &'static str> {
    let audit = audit_body(html)?;
    if exact_records.iter().any(|record| !audit.contains(record)) {
        return Err("exact observation lost");
    }
    Ok(())
}

#[test]
fn current_state_historical_risk_and_exact_audit_are_separate(
) -> Result<(), Box<dyn std::error::Error>> {
    for case in [
        "retained",
        "recovered",
        "current_failure",
        "only_current",
        "blank_later",
    ] {
        let mut input: Value = serde_json::from_str(reading_fixture::SCENARIO)?;
        input["prior_checkpoint_count"] = json!(0);
        input["later_checkpoint_count"] = json!(0);
        input["unassociated_checkpoint"] = json!(false);
        input["works"][1]["checkpoints"] = json!([]);
        let checkpoints = input["works"][0]["checkpoints"]
            .as_array_mut()
            .ok_or("history")?;
        checkpoints[0]["state_change"] = json!("Reported relay change");
        if case == "only_current" {
            checkpoints.truncate(1);
        }
        if case != "retained" {
            let last = checkpoints.last_mut().ok_or("last")?;
            last["verification"] = json!(if case == "recovered" {
                "Passed"
            } else {
                "Failed"
            });
            last["review"] = json!("Reviewed");
            last["acceptance"] = json!(if case == "recovered" {
                "Accepted"
            } else {
                "Rejected"
            });
            last["next_step"] = json!("Inspect relay behavior");
            if case == "blank_later" {
                last["verification"] = Value::Null;
                last["state_change"] = Value::Null;
            }
        }
        let f = reading_fixture::fixture_scenario(input)?;
        let work = f.goals["older"];
        let before = f.operations.canonical_basis(f.project)?;
        let canonical_bytes = std::fs::read(f.operations.layout().canonical_store())?;
        let projection = f
            .operations
            .project_projection_selected(f.project, WorkSelector::ExactWork(work))?;
        let w = projection.selected_work.as_ref().ok_or("Work")?;
        // Compare the complete read model directly with canonical input, without formatting it.
        assert_eq!(w.reading.states.len(), before.checkpoint_history.len());
        for (state, cp) in w.reading.states.iter().zip(&before.checkpoint_history) {
            assert_eq!(state.checkpoint_id, cp.id);
            assert_eq!(state.checkpoint_revision, cp.revision);
            assert_eq!(state.observed_at, cp.recorded_at);
            assert_eq!(state.work_state, cp.work_state);
            assert_eq!(state.work_source_basis, cp.source_basis);
            assert_eq!(state.verification, cp.verification);
            assert_eq!(state.user_review, cp.user_review);
            assert_eq!(state.user_acceptance, cp.user_acceptance);
        }
        let micros: Vec<_> = before
            .checkpoint_history
            .iter()
            .map(|cp| cp.recorded_at.as_unix_micros().to_string())
            .collect();
        for (locale, fixed, language) in [
            (ViewerLocale::English, FixedLocale::English, "en"),
            (ViewerLocale::Korean, FixedLocale::Korean, "ko"),
        ] {
            let adapter = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
            let request = ViewerRequest {
                project_id: f.project,
                locale,
                view: ViewerView::Work { work: Some(work) },
                requested_language: language.into(),
                guarded_request: None,
            };
            let server = ViewerServer::new(
                adapter,
                f.project,
                locale,
                request.view.clone(),
                language.into(),
                "127.0.0.1:3219".parse()?,
            )?;
            let http = format!("GET /?view=work&work={work}&locale={language} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
            let mut response = Vec::new();
            server.serve_connection(&mut http.as_bytes(), &mut response)?;
            let live = String::from_utf8(response)?;
            assert!(live.starts_with("HTTP/1.1 200"));
            let snapshot = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()))
                .render_snapshot(&request, TimestampMicros::from_unix_micros(123))?
                .html;
            if let Some(root) = std::env::var_os("VOLICORD_WORK_HISTORY_CAPTURE") {
                std::fs::create_dir_all(&root)?;
                for (mode, html) in [("live", &live), ("offline", &snapshot)] {
                    std::fs::write(
                        std::path::Path::new(&root).join(format!("{case}-{language}-{mode}.html")),
                        html,
                    )?;
                }
            }
            let answers = work_answers(w, language, fixed);
            let fact = |question: &str| -> Result<&str, Box<dyn std::error::Error>> {
                Ok(answers
                    .facts
                    .iter()
                    .find(|a| a.question == question)
                    .ok_or("fact")?
                    .text
                    .as_str())
            };
            // Expected states are independent fixture facts, not helper-generated expectations.
            let expected = match (case, language) {
                ("retained", "en") => (
                    "Automated verification: not run",
                    "User review: not requested",
                    "User acceptance: rejected",
                ),
                ("retained", _) => (
                    "자동 검증: 실행하지 않음",
                    "사용자 검토: 요청하지 않음",
                    "사용자 수락: 거부됨",
                ),
                ("recovered", "en") => (
                    "Automated verification: passed",
                    "User review: reviewed",
                    "User acceptance: accepted",
                ),
                ("recovered", _) => (
                    "자동 검증: 통과",
                    "사용자 검토: 검토됨",
                    "사용자 수락: 수락됨",
                ),
                (_, "en") => (
                    "Automated verification: failed",
                    "User review: reviewed",
                    "User acceptance: rejected",
                ),
                (_, _) => (
                    "자동 검증: 실패",
                    "사용자 검토: 검토됨",
                    "사용자 수락: 거부됨",
                ),
            };
            assert_eq!(
                (
                    fact("VerificationState")?,
                    fact("UserReview")?,
                    fact("UserAcceptance")?
                ),
                expected
            );
            let summary = answers
                .facts
                .iter()
                .find(|a| a.question == "HistoricalAdversity");
            assert_eq!(summary.is_some(), case != "only_current");
            if let Some(summary) = summary {
                let failure_key = format!(
                    "checkpoint:{}@1:verification",
                    f.checkpoints["verification_only"]
                );
                let rejection_key =
                    format!("checkpoint:{}@1:user_acceptance", f.checkpoints["change"]);
                assert_eq!(
                    summary.evidence_keys,
                    if case == "blank_later" {
                        vec![rejection_key]
                    } else {
                        vec![failure_key, rejection_key]
                    }
                );
                let history_meaning = match (case, language) {
                    ("blank_later", "en") => "acceptance rejection observations: 1",
                    ("blank_later", _) => "수락 거부 관찰 1건",
                    (_, "en") => {
                        "verification failure observations: 1; acceptance rejection observations: 1"
                    }
                    (_, _) => "검증 실패 관찰 1건; 수락 거부 관찰 1건",
                };
                assert!(summary.text.contains(history_meaning));
                if case == "blank_later" {
                    assert!(!summary.text.contains(if language == "en" {
                        "verification failure"
                    } else {
                        "검증 실패"
                    }));
                }
            }
            assert!(!answers
                .facts
                .iter()
                .any(|a| a.question == "AdverseObservation"));
            let summary_text = summary.map(|a| a.text.as_str());
            let http = format!(
                "GET /?view=overview&locale={language} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n"
            );
            let mut response = Vec::new();
            server.serve_connection(&mut http.as_bytes(), &mut response)?;
            let overview = String::from_utf8(response)?;
            assert!(overview.starts_with("HTTP/1.1 200"));
            for html in [&overview, &snapshot] {
                let card = html.split(&format!("class=\"understanding-card work-item work-summary\" data-work-id=\"{work}\""))
                    .nth(1).ok_or("compact Work card")?.split("</article>").next().ok_or("card end")?;
                check_primary(
                    card,
                    expected.0,
                    expected.1,
                    expected.2,
                    summary_text,
                    &micros,
                )?;
            }
            for html in [&live, &snapshot] {
                // Isolate the selected Work in an offline whole-project snapshot.
                let selected = html
                    .split(&format!("id=\"work-{work}\""))
                    .nth(1)
                    .ok_or("selected section")?
                    .split("</section>")
                    .next()
                    .ok_or("section end")?;
                check_primary(
                    selected,
                    expected.0,
                    expected.1,
                    expected.2,
                    summary_text,
                    &micros,
                )?;
                let exact_records: Vec<_> = w
                    .reading
                    .states
                    .iter()
                    .map(|state| {
                        // Full, untruncated record including arrays, exact time, revision and Sources.
                        let escaped = format!("{state:?}")
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                            .replace('"', "&quot;")
                            .replace('\'', "&#39;");
                        escaped
                    })
                    .collect();
                check_audit(selected, &exact_records)?;
                assert!(audit_body(selected)?.starts_with(if language == "en" {
                    "<summary>Verification and original state observations</summary>"
                } else {
                    "<summary>검증 및 원래 상태 관찰</summary>"
                }));
                let missing_event = selected.replace(&exact_records[0], "removed observation");
                assert_eq!(
                    check_audit(&missing_event, &exact_records),
                    Err("exact observation lost")
                );
                if case == "recovered" {
                    assert!(!ordinary_text(selected).contains(if language == "en" {
                        "Automated verification: failed"
                    } else {
                        "자동 검증: 실패"
                    }));
                }
                // Sensitivity controls challenge semantic placement, retention and duplication.
                let hidden = selected.replace(
                    &format!("<p data-question=\"VerificationState\">{}</p>", expected.0),
                    &format!(
                        "<details><summary>audit</summary><p>{}</p></details>",
                        expected.0
                    ),
                );
                assert_eq!(
                    check_primary(
                        &hidden,
                        expected.0,
                        expected.1,
                        expected.2,
                        summary_text,
                        &micros
                    ),
                    Err("current state missing or duplicated")
                );
                let duplicated = format!("{selected}<p>{}</p>", expected.0);
                assert!(check_primary(
                    &duplicated,
                    expected.0,
                    expected.1,
                    expected.2,
                    summary_text,
                    &micros
                )
                .is_err());
                let leaked = format!("{selected}<p>{}</p>", micros[0]);
                assert_eq!(
                    check_primary(
                        &leaked,
                        expected.0,
                        expected.1,
                        expected.2,
                        summary_text,
                        &micros
                    ),
                    Err("raw timestamp in ordinary reading")
                );
                let removed =
                    selected.replace("class=\"work-state-history\"", "class=\"removed-history\"");
                assert_eq!(audit_body(&removed), Err("closed audit history missing"));
                if case == "recovered" {
                    let old_current = selected.replace(
                        expected.0,
                        if language == "en" {
                            "Automated verification: failed"
                        } else {
                            "자동 검증: 실패"
                        },
                    );
                    assert_eq!(
                        check_primary(
                            &old_current,
                            expected.0,
                            expected.1,
                            expected.2,
                            summary_text,
                            &micros
                        ),
                        Err("current state missing or duplicated")
                    );
                }
            }
            assert!(micros.iter().all(|time| !answers.text().contains(time)));
        }
        assert_eq!(
            canonical_bytes,
            std::fs::read(f.operations.layout().canonical_store())?
        );
        assert_eq!(before, f.operations.canonical_basis(f.project)?);
    }
    Ok(())
}
