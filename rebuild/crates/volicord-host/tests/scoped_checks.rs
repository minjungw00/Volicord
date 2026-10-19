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
