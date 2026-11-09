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

fn stdio_call(
    layout: &RuntimeLayout,
    name: &str,
    args: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_volicord-mcp"))
        .env("VOLICORD_RUNTIME_DIR", layout.root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("stdin")?;
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{}})
    )?;
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}})
    )?;
    drop(input);
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response = String::from_utf8(output.stdout)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|v| v["id"] == 1)
        .ok_or("response")?;
    Ok(response)
}
#[test]
fn mcp_expiry_batches_inspect_and_restart_through_real_stdio(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::TimestampMicros;
    use volicord_privacy::*;
    let root = tempfile::tempdir()?;
    let layout = RuntimeLayout::new(root.path().join("runtime"))?;
    let ops = LocalOperations::new(layout.clone());
    let project = ops.initialize_project("Expiry MCP", None)?.project.id;
    let before = ops.canonical_basis(project)?;
    let mut store = PrivacyStore::open(layout.privacy_store())?;
    let record = store.record_managed_derived(ManagedDerivedDraft {
        project_id: project,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "expiry fixture".into(),
        analysis_snapshot: None,
        included_sources: vec![],
        canonical_links: vec![],
        content: "withheld expired text".into(),
        uncertainty: None,
        retained_until: Some(TimestampMicros::from_unix_micros(2)),
        retention_basis: "fixture TTL".into(),
    })?;
    drop(store);
    for args in [
        json!({"project_id":project.to_string(),"limit":0}),
        json!({"project_id":project.to_string(),"limit":257}),
        json!({"project_id":"ffffffffffffffffffffffffffffffff","limit":1}),
        json!({"project_id":project.to_string(),"limit":1,"source_ids":[]}),
    ] {
        assert_eq!(
            stdio_call(&layout, "expiry_cleanup", args)?["result"]["isError"],
            true
        );
    }
    assert!(PrivacyStore::open(layout.privacy_store())?
        .get_derived(project, record.id)?
        .content
        .is_some());
    let response = stdio_call(
        &layout,
        "expiry_cleanup",
        json!({"project_id":project.to_string(),"limit":1}),
    )?;
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["structuredContent"]["managed"]["removed_ids"],
        json!([record.id.to_string()])
    );
    let response = stdio_call(
        &layout,
        "expiry_cleanup",
        json!({"project_id":project.to_string(),"limit":1}),
    )?;
    assert_eq!(
        response["result"]["structuredContent"]["managed"]["removed_ids"],
        json!([])
    );
    let status = stdio_call(
        &layout,
        "privacy_status",
        json!({"project_id":project.to_string()}),
    )?;
    assert_eq!(
        status["result"]["structuredContent"]["managed_records"][0]["expired_by_cleanup"],
        true
    );
    assert!(!status.to_string().contains("withheld expired text"));
    assert_eq!(ops.canonical_basis(project)?, before);
    Ok(())
}
