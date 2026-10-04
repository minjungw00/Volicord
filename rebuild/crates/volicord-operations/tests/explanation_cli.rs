//! Real executable lifecycle coverage; labeled interpretations test structure only.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use std::process::Command;

fn invoke(
    f: &reading_fixture::Fixture,
    args: &[&str],
) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args([
            "--runtime",
            f.operations.layout().root().to_str().ok_or("runtime")?,
            "--project",
            &f.project.to_string(),
            "--json",
        ])
        .args(args)
        .current_dir(&f.repository)
        .output()?;
    assert!(
        output.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}
fn contains_answer(value: &Value, kind: &str, state: &str, fingerprint: Option<&str>) -> bool {
    match value {
        Value::Object(fields) => {
            (fields.get("explanation_state") == Some(&json!(state))
                && fingerprint.is_none_or(|fp| {
                    value["provenance"]["fingerprint"] == fp
                        && value["provenance"]["subject"]["kind"] == kind
                }))
                || fields
                    .values()
                    .any(|v| contains_answer(v, kind, state, fingerprint))
        }
        Value::Array(items) => items
            .iter()
            .any(|v| contains_answer(v, kind, state, fingerprint)),
        _ => false,
    }
}
#[test]
fn actual_work_and_decision_prepare_record_delete_readback(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let before = f.operations.canonical_basis(f.project)?;
    for (kind, selector, id, read, questions) in [
        (
            "work",
            "--work",
            f.goals["relay"].to_string(),
            "status",
            vec![
                "purpose",
                "reported_change",
                "expected_effect",
                "verification",
                "next_step",
            ],
        ),
        (
            "decision",
            "--decision",
            f.decisions["project"].to_string(),
            "decisions",
            vec![
                "user_rationale",
                "recommendation",
                "consequences",
                "applicability",
            ],
        ),
    ] {
        for language in ["en", "ko"] {
            let prepared = invoke(
                &f,
                &[
                    kind,
                    "explain",
                    "prepare",
                    selector,
                    &id,
                    "--language",
                    language,
                ],
            )?;
            let plan = &prepared["plan"];
            assert_eq!(prepared["operation"], "explanation_prepare");
            let keys: Vec<_> = plan["evidence"]
                .as_array()
                .ok_or("evidence")?
                .iter()
                .map(|e| e["key"].clone())
                .collect();
            let response = json!({"format_kind":"volicord_explanation", "format_version":1,
                "plan_fingerprint":plan["fingerprint"], "language":language,
                "generator":{"host":"cli-lifecycle-test","session":"synthetic","agent":null,"model":null},
                "paragraphs":questions.iter().map(|q| json!({"question":q,
                    "text":format!("Labeled lifecycle fixture {q}"),"evidence_keys":keys})).collect::<Vec<_>>()});
            let input = f._temporary.path().join("response.json");
            std::fs::write(&input, serde_json::to_vec(&response)?)?;
            let recorded = invoke(
                &f,
                &[
                    kind,
                    "explain",
                    "record",
                    selector,
                    &id,
                    "--language",
                    language,
                    "--input",
                    input.to_str().ok_or("input")?,
                ],
            )?;
            assert_eq!(recorded["operation"], "explanation_record");
            assert_eq!(recorded["explanation"]["realization"], response);
            let readback = invoke(&f, &[read, "--language", language])?;
            assert!(
                contains_answer(&readback, kind, "current", plan["fingerprint"].as_str()),
                "{readback}"
            );
            let mut bad = response;
            bad["plan_fingerprint"] = json!("sha256:".to_owned() + &"0".repeat(64));
            std::fs::write(&input, serde_json::to_vec(&bad)?)?;
            let rejected = Command::new(env!("CARGO_BIN_EXE_volicord"))
                .args([
                    "--runtime",
                    f.operations.layout().root().to_str().ok_or("runtime")?,
                    "--project",
                    &f.project.to_string(),
                    "--json",
                    kind,
                    "explain",
                    "record",
                    selector,
                    &id,
                    "--language",
                    language,
                    "--input",
                    input.to_str().ok_or("input")?,
                ])
                .output()?;
            assert!(!rejected.status.success());
            assert_ne!(
                rejected.status.code(),
                Some(2),
                "must be Product rejection, not parser error"
            );
        }
        let deleted = invoke(&f, &[kind, "explain", "delete", selector, &id])?;
        assert_eq!(deleted["operation"], "explanation_delete");
        for language in ["en", "ko"] {
            let readback = invoke(&f, &[read, "--language", language])?;
            assert!(!readback.to_string().contains("Labeled lifecycle fixture"));
        }
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn campaign_explanations_preserve_corrected_history() -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::explanation_size_scenario(
        "metadata_heavy",
    )?)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("crates")?
        .parent()
        .ok_or("rebuild")?;
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    assert!(head.status.success());
    // Pin executable bytes against concurrent support builds in Cargo target.
    let binary = f._temporary.path().join("volicord");
    std::fs::copy(env!("CARGO_BIN_EXE_volicord"), &binary)?;
    let config = json!({"binary": binary,
        "runtime": f.operations.layout().root(), "repository": f.repository,
        "project": f.project.to_string(), "work": f.goals["relay"].to_string(),
        "decision": f.decisions["project"].to_string(),
        "candidate_head": String::from_utf8(head.stdout)?.trim()});
    let output = Command::new("python3")
        .arg(root.join("validation/dogfood/explanation_product_support.py"))
        .arg(serde_json::to_string(&config)?)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()?;
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
