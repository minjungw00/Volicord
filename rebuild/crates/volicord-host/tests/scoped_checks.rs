#[path = "../../volicord-operations/tests/support/scoped_check_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use serde_json::{json, Value};
use std::error::Error;
use volicord_host::HostAdapter;
fn call(host: &mut HostAdapter, args: Value) -> Value {
    host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"work_source","arguments":args}})).unwrap_or(Value::Null)
}
#[test]
fn explicit_host_attachment_reuses_exact_source_but_cannot_execute_commands(
) -> Result<(), Box<dyn Error>> {
    let f = fixture::fixture()?;
    let baseline = f.ops.canonical_basis(f.project)?;
    let mut host = HostAdapter::new(volicord_operations::LocalOperations::new(
        f.ops.layout().clone(),
    ));
    for action in ["run", "run_check", "execute"] {
        let denied = call(
            &mut host,
            json!({"action":action,"project_id":f.project.to_string(),"work_item_id":f.work.to_string(),"program":"python3","arguments":["-c","print('unauthorized')"]}),
        );
        assert!(
            denied["result"]["isError"] == true || !denied["error"].is_null(),
            "{denied}"
        );
    }
    let mut args = json!({"action":"record_check","project_id":f.project.to_string(),"work_item_id":f.other.to_string(),"recording":fixture::recording(f.work,f.passed.source_id,true)});
    let denied = call(&mut host, args.clone());
    assert_eq!(denied["result"]["isError"], true, "{denied}");
    assert!(denied["result"]["structuredContent"]["explanation_production"].is_null());
    assert_eq!(baseline, f.ops.canonical_basis(f.project)?);
    args["work_item_id"] = json!(f.work.to_string());
    let recorded = call(&mut host, args);
    assert_eq!(recorded["result"]["isError"], false, "{recorded}");
    let latest = f
        .ops
        .canonical_basis(f.project)?
        .latest_checkpoint
        .ok_or("checkpoint")?;
    assert_eq!(latest.verification[0].source_id, Some(f.passed.source_id));
    assert_eq!(
        latest.verification[0]
            .scoped_run
            .as_ref()
            .ok_or("scoped")?
            .assertion_state,
        volicord_context::AssertionState::Passed
    );
    Ok(())
}

#[test]
fn scoped_rejection_reports_predicate_and_bounded_basis_without_recording(
) -> Result<(), Box<dyn Error>> {
    let f = fixture::fixture()?;
    let baseline = f.ops.canonical_basis(f.project)?;
    let mut host = HostAdapter::new(volicord_operations::LocalOperations::new(
        f.ops.layout().clone(),
    ));
    let mut request = fixture::recording(f.work, f.passed.source_id, true);
    request.assertion_evidence = None;
    request.outcome = "SECRET operator output".into();
    request.next_step = "SECRET conversation".into();
    let denied = call(
        &mut host,
        json!({"action":"record_check","project_id":f.project.to_string(),
        "work_item_id":f.work.to_string(),"recording":request}),
    );
    assert_eq!(denied["result"]["isError"], true, "{denied}");
    let text = serde_json::to_string(&denied)?;
    assert!(text.contains("assertion_evidence_presence"), "{denied}");
    assert!(text.contains("scoped_check_basis_rejected"), "{denied}");
    assert!(text.contains(&f.work.to_string()), "{denied}");
    assert!(text.contains(&f.passed.source_id.to_string()), "{denied}");
    assert!(!text.contains("SECRET"));
    assert!(!text.contains("main.py"));
    assert_eq!(baseline, f.ops.canonical_basis(f.project)?);
    let accepted = call(
        &mut host,
        json!({"action":"record_check","project_id":f.project.to_string(),
        "work_item_id":f.work.to_string(),"recording":fixture::recording(f.work,f.passed.source_id,true)}),
    );
    assert_eq!(accepted["result"]["isError"], false, "{accepted}");
    Ok(())
}

#[test]
fn recorded_check_exposes_pause_and_independent_reading_followup() -> Result<(), Box<dyn Error>> {
    let f = fixture::fixture()?;
    let baseline = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let mut host = HostAdapter::new(volicord_operations::LocalOperations::new(
        f.ops.layout().clone(),
    ));
    for (index, passed) in [false, true].into_iter().enumerate() {
        let source = if passed {
            f.passed.source_id
        } else {
            f.failed.source_id
        };
        let recorded = call(
            &mut host,
            json!({"action":"record_check","project_id":f.project.to_string(),
                "work_item_id":f.work.to_string(),"recording":fixture::recording(f.work,source,passed)}),
        );
        assert_eq!(recorded["result"]["isError"], false, "{recorded}");
        let result = &recorded["result"]["structuredContent"];
        let basis = f.ops.canonical_basis(f.project)?;
        let latest = basis.latest_checkpoint.as_ref().ok_or("checkpoint")?;
        assert_eq!(latest.work_state, volicord_context::WorkState::Paused);
        assert_eq!(latest.work_item_id, Some(f.work));
        assert_eq!(latest.verification[0].source_id, Some(source));
        assert_eq!(
            basis.checkpoint_history.len(),
            baseline.checkpoint_history.len() + index + 1
        );
        assert_eq!(result["receipt"]["checkpoint_id"], latest.id.to_string());
        assert_eq!(result["recorded_work_state"], "paused");
        let production = &result["explanation_production"];
        assert_eq!(production["project_id"], f.project.to_string());
        assert_eq!(production["work_item_id"], f.work.to_string());
        assert_eq!(production["reason"], "boundary");
        assert_eq!(production["scopes"], json!(["summary", "article"]));
        assert_eq!(
            production["repository_reading"]["inspect"]["tool"],
            "repository_understanding"
        );
        assert_eq!(
            production["repository_reading"]["analysis_inspect"]["arguments"]["project_id"],
            f.project.to_string()
        );
        assert_eq!(
            production["related_decision_reading"]["tool"],
            "decision_explanation"
        );
        let inspection = &production["repository_reading"]["analysis_inspect"];
        let inspected = host
            .handle(
                json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":inspection["tool"],"arguments":inspection["arguments"]}}),
            )
            .ok_or("inspection response")?;
        assert_eq!(inspected["result"]["isError"], false, "{inspected}");
        assert_eq!(f.ops.canonical_basis(f.project)?, basis);
        assert_eq!(basis.context_items, baseline.context_items);
        assert_eq!(basis.active_decisions, baseline.active_decisions);
        assert_eq!(basis.superseded_decisions, baseline.superseded_decisions);
        assert_eq!(basis.sources, baseline.sources);
        assert_eq!(f.ops.privacy_status(f.project)?, privacy);
    }
    Ok(())
}
