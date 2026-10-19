//! Authored transport controls, not live model generation or a user verdict.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use std::error::Error;
use volicord_host::HostAdapter;
use volicord_projections::*;

fn call(host: &mut HostAdapter, args: Value) -> Result<Value, Box<dyn Error>> {
    let result = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"decision_explanation","arguments":args}})).ok_or("response")?;
    if result["result"]["isError"] != false {
        return Err(result.to_string().into());
    }
    Ok(result["result"]["structuredContent"].clone())
}
#[test]
fn recorded_recommendation_article_preserves_missing_user_reason_and_scope(
) -> Result<(), Box<dyn Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let decision = f.decisions["explicit"];
    // Independent source-first record oracle: the contemporaneous recommendation
    // exists; user rationale is absent. No fixture Source body supplies code proof.
    let prepared = f.operations.prepare_explanation(
        f.project,
        ExplanationSubject::Decision(decision),
        "en",
    )?;
    let recommendation = prepared
        .evidence
        .iter()
        .find(|e| e.key == "recommendation")
        .ok_or("recommendation")?;
    assert_eq!(
        recommendation.content["rationale"],
        "Prefer a bounded local boundary"
    );
    assert!(prepared
        .evidence
        .iter()
        .find(|e| e.key == "user_rationale")
        .ok_or("reason")?
        .content
        .is_null());
    let mut host = HostAdapter::new(f.operations);
    let scope = json!({"project_id":f.project.to_string(),"decision_id":decision.to_string()});
    let mut args = scope.clone();
    args["action"] = json!("generate");
    args["language"] = json!("en");
    let generated = call(&mut host, args)?;
    assert_eq!(
        generated["generation_status"],
        "awaiting_host_interpretation"
    );
    assert_eq!(
        generated["source_inventory"]["body_authority"],
        "not_granted"
    );
    let units = [
        ("heading", "Choosing a bounded boundary", vec!["question_context"]),
        ("synopsis", "The recorded recommendation favors a bounded local boundary. The user's choice is separate; no user reason is recorded.", vec!["choice","recommendation","user_rationale"]),
        ("section_heading", "Why the agent recommended a local boundary", vec!["recommendation"]),
        ("narrative", "The recorded recommendation rationale is: Prefer a bounded local boundary. This retained reason does not verify the implementation.", vec!["recommendation"]),
        ("section_heading", "The recorded choice and its reach", vec!["choice","applicability"]),
        ("user_rationale", "No user rationale was recorded; the recommendation does not supply one.", vec!["user_rationale"]),
    ];
    let mut args = scope.clone();
    args["action"] = json!("compose");
    args["generation_id"] = generated["generation_id"].clone();
    args["draft"] = json!({"plan_fingerprint":generated["plan"]["fingerprint"],"language":"en","generator":{"host":"test","session":"record-control","agent":null,"model":null},"paragraphs":units.iter().map(|(q,t,k)|json!({"question":q,"text":t,"evidence_keys":k,"sources":[]})).collect::<Vec<_>>()});
    let composed = call(&mut host, args.clone())?;
    // Decision composition cannot borrow a Work file even with valid-looking offsets.
    args["draft"]["paragraphs"][2]["sources"] = json!([{"source_id":"01".repeat(16),"field":"body","use_kind":"current_code","start_byte":0,"end_byte":1,"claim":null}]);
    assert!(call(&mut host, args).is_err());
    let mut args = scope.clone();
    args["action"] = json!("record");
    args["generation_id"] = generated["generation_id"].clone();
    args["realization"] = composed["realization"].clone();
    let recorded = call(&mut host, args)?;
    assert_eq!(
        recorded["explanation"]["semantic_approval_status"],
        "not_established"
    );
    let projection = host.operations().project_projection(f.project)?;
    let decision = projection
        .decision_catalog
        .iter()
        .find(|d| d.decision.decision_id == decision)
        .ok_or("decision")?;
    let public =
        volicord_operations::decision_reading_json(&decision.decision, "en", FixedLocale::English);
    assert_eq!(public["answers"]["explanation_state"], "current");
    assert!(public["evidence"]["user_rationale"].is_null());
    assert_eq!(
        public["evidence"]["recommendation_rationale"],
        "Prefer a bounded local boundary"
    );
    Ok(())
}
