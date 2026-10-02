//! Transport probes use labeled test interpretations; capability evidence is
//! independently generated through the active host and installed browser path.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use volicord_host::HostAdapter;
use volicord_operations::LocalOperations;
use volicord_projections::*;

#[test]
fn mcp_consumers_preserve_question_answers_and_separate_original_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = reading_fixture::rich_scenario()?;
    let works = input["works"].as_array_mut().ok_or("works")?;
    let n = works
        .iter()
        .position(|w| w["key"] == "relay")
        .ok_or("relay")?;
    let last = works.remove(n);
    works.push(last);
    let f = reading_fixture::fixture_scenario(input)?;
    let work = f.goals["relay"];
    let subject = ExplanationSubject::Work(work);
    let plan = f.operations.prepare_explanation(f.project, subject, "ko")?;
    let response = ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: "ko".into(),
        generator: ExplanationGenerator {
            host: "transport-test".into(),
            session: "synthetic".into(),
            agent: None,
            model: None,
        },
        paragraphs: [
            (ExplanationQuestion::Purpose, "goal"),
            (ExplanationQuestion::ReportedChange, "result"),
            (ExplanationQuestion::ExpectedEffect, "result"),
            (ExplanationQuestion::Verification, "verification"),
            (ExplanationQuestion::NextStep, "next_step"),
        ]
        .into_iter()
        .map(|(q, key)| ExplanationParagraph {
            question: q,
            text: format!("Transport-only Korean-requested paragraph {q:?}"),
            evidence_keys: vec![key.into()],
        })
        .collect(),
    };
    f.operations
        .record_explanation(f.project, subject, "ko", response)?;
    let before = f.operations.canonical_basis(f.project)?;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    for tool in ["recall", "repository_understanding"] {
        for locale in ["en", "ko"] {
            let result=host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,"arguments":{"project_id":f.project.to_string(),"requested_language":"ko","fixed_locale":locale}}})).ok_or("MCP response")?;
            assert_eq!(result["result"]["isError"], false, "{result}");
            let data: Value = result["result"]["structuredContent"].clone();
            let selected = &data["selected_work"];
            assert_eq!(selected["work_item_id"], work.to_string(), "{data}");
            assert_eq!(
                selected["answers"]["prose"][1]["text"],
                "Transport-only Korean-requested paragraph ReportedChange"
            );
            assert_eq!(selected["answers"]["provenance"]["subject"]["kind"], "work");
            assert_eq!(
                selected["answers"]["provenance"]["fingerprint"],
                plan.fingerprint
            );
            assert!(selected["answers"].to_string().contains(if locale == "ko" {
                "통과"
            } else {
                "passed"
            }));
            assert!(!selected["answers"].to_string().contains("감사 참고"));
            let basis = selected["answers"]["provenance"]["evidence"]
                .as_array()
                .ok_or("answer basis")?
                .iter()
                .find(|e| e["key"] == "result")
                .ok_or("exact result basis")?;
            assert_eq!(basis["identity"], f.checkpoints["relay-change"].to_string());
            assert_eq!(basis["revision"], 1);
            assert_eq!(basis["field"], "state_change");
            if let Some(original) = selected["evidence"]["result"]["original_text"].as_str() {
                assert!(original.contains("감사 참고"));
            } else {
                assert_eq!(
                    selected["evidence"]["transport_omission"]["reason"],
                    "serialized_byte_budget"
                );
                assert!(
                    selected["evidence"]["transport_omission"]["omitted_field_count"]
                        .as_u64()
                        .ok_or("omission count")?
                        > 0
                );
            }
        }
    }
    // Deleting the retained answer must not restore the retired excerpt reader.
    f.operations.delete_explanations(f.project, subject)?;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let result=host.handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"recall","arguments":{"project_id":f.project.to_string(),"requested_language":"ko","fixed_locale":"ko"}}})).ok_or("MCP response")?;
    let selected = &result["result"]["structuredContent"]["selected_work"];
    assert_eq!(selected["answers"]["explanation_state"], "unavailable");
    assert!(!selected["answers"].to_string().contains("감사 참고"));
    assert!(!selected["answers"].to_string().contains("Transport-only"));
    assert!(
        selected["evidence"]["result"]["original_text"].is_string()
            || selected["evidence"]["transport_omission"]["omitted_field_count"]
                .as_u64()
                .is_some_and(|n| n > 0)
    );
    assert_eq!(f.operations.canonical_basis(f.project)?, before);
    Ok(())
}
