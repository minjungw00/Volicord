use serde_json::{json, Value};
use volicord_host::HostAdapter;
use volicord_operations::{LocalOperations, RuntimeLayout};

fn call(adapter: &mut HostAdapter, name: &str, args: Value) -> Value {
    adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}})).expect("response")
}
#[test]
fn mcp_collection_controls_are_scoped_validated_and_inspectable(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = RuntimeLayout::new(root.path().join("runtime"))?;
    let ops = LocalOperations::new(layout.clone());
    let project = ops.initialize_project("Controls", None)?.project.id;
    let mut adapter = HostAdapter::new(ops);
    adapter.handle(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}));
    for args in [
        json!({"action":"disable","project_id":project.to_string(),"basis":"explicit control","candidate_kind":"unknown"}),
        json!({"action":"disable","project_id":project.to_string(),"basis":"explicit control","session":""}),
        json!({"action":"disable","project_id":"ffffffffffffffffffffffffffffffff","basis":"unknown Project"}),
        json!({"action":"disable","project_id":project.to_string(),"basis":"explicit control","extra":true}),
    ] {
        assert_eq!(
            call(&mut adapter, "candidate_collection", args)["result"]["isError"],
            true
        );
    }
    assert!(adapter
        .operations()
        .collection_policies(project)?
        .is_empty());
    let result = call(
        &mut adapter,
        "candidate_collection",
        json!({"action":"disable","project_id":project.to_string(),"basis":"explicit control","session":"session-a","source_operation":"inventory","candidate_kind":"observation"}),
    );
    assert_eq!(result["result"]["isError"], false, "{result}");
    assert_eq!(
        result["result"]["structuredContent"]["policies"][0]["candidate_kind"],
        "observation"
    );
    drop(adapter);
    let mut adapter = HostAdapter::new(LocalOperations::new(layout));
    adapter.handle(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}));
    let result = call(
        &mut adapter,
        "candidate_collection",
        json!({"action":"inspect","project_id":project.to_string()}),
    );
    assert_eq!(
        result["result"]["structuredContent"]["policies"][0]["opted_out"],
        true
    );
    let result = call(
        &mut adapter,
        "candidate_collection",
        json!({"action":"enable","project_id":project.to_string(),"basis":"explicit resume","session":"session-a","source_operation":"inventory","candidate_kind":"observation"}),
    );
    assert_eq!(
        result["result"]["structuredContent"]["policies"][0]["opted_out"],
        false
    );
    Ok(())
}
