use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;
use volicord_host::{run_stdio, HostAdapter, HOST_TOOL_NAMES};
use volicord_operations::{LocalOperations, RuntimeLayout};

fn setup() -> (tempfile::TempDir, HostAdapter, String) {
    let temporary = tempdir().expect("temporary directory");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).expect("repository");
    let operations = LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("runtime layout"),
    );
    let project = operations
        .initialize_project("Host Project", Some(&repository))
        .expect("initialize Project")
        .project
        .id
        .to_string();
    (temporary, HostAdapter::new(operations), project)
}

#[test]
fn initializes_and_discovers_only_high_level_product_capabilities() {
    let (_temporary, mut adapter, _project) = setup();
    let initialized = adapter
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}))
        .expect("initialize response");
    assert_eq!(initialized["result"]["serverInfo"]["name"], "volicord");
    let listed = adapter
        .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
        .expect("tools response");
    let names = listed["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, HOST_TOOL_NAMES);
    assert!(!listed.to_string().contains("database"));
    assert!(!listed.to_string().contains("legacy"));
}

#[test]
fn health_distinguishes_connection_from_degraded_capability() {
    let (temporary, mut adapter, project) = setup();
    fs::remove_dir(temporary.path().join("repository")).expect("remove bound repository");
    let response = call(
        &mut adapter,
        "project_health",
        json!({"project_id":project}),
    );
    let content = structured(&response);
    assert_eq!(content["connection"], "connected");
    assert_eq!(content["capability_state"], "degraded");
    assert_eq!(content["repository_available"], false);
}

#[test]
fn stdio_ends_cleanly_at_eof_and_preserves_ordered_responses() {
    let (_temporary, mut adapter, project) = setup();
    let input = format!(
        "{}\n{}\n{}\n",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_health","arguments":{"project_id":project}}})
    );
    let mut output = Vec::new();
    run_stdio(&mut adapter, input.as_bytes(), &mut output).expect("stdio completes at EOF");
    let lines = String::from_utf8(output)
        .expect("UTF-8 output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("JSON response"))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["id"], 1);
    assert_eq!(lines[1]["id"], 2);
}

#[test]
fn recall_documents_and_inspection_are_read_only_host_calls() {
    let (_temporary, mut adapter, project) = setup();
    let before = adapter
        .operations()
        .canonical_basis(parse_project(&project))
        .expect("before");
    for tool in [
        "recall",
        "repository_understanding",
        "canonical_inspect",
        "candidate_inspect",
        "privacy_status",
    ] {
        let response = call(&mut adapter, tool, json!({"project_id":project}));
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
    }
    let response = call(
        &mut adapter,
        "document_preview",
        json!({"project_id":project,"kind":"handoff-resume","language":"ja"}),
    );
    assert_eq!(response["result"]["isError"], false);
    assert!(structured(&response)["content"]
        .as_str()
        .is_some_and(|value| value.contains("ja")));
    let after = adapter
        .operations()
        .canonical_basis(parse_project(&project))
        .expect("after");
    assert_eq!(before, after);
}

#[test]
fn guarded_transport_and_fallback_keep_one_exact_logical_request() {
    use volicord_context::{Clock, Principal, PrincipalKind, SystemClock, TimestampMicros};
    use volicord_operations::{
        GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, RequestingProvenance,
    };
    let (_temporary, mut adapter, project) = setup();
    adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}));
    let project_id = parse_project(&project);
    let now = SystemClock.now().expect("clock");
    let request = adapter
        .operations()
        .create_guarded_request(GuardedEffectDraft {
            project_id,
            exact_action: "publish".into(),
            target: "registry/example".into(),
            expected_effect: "public release".into(),
            risk: GuardedRisk {
                category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
                concrete_consequence: "public artifact".into(),
            },
            scope: vec!["release:example".into()],
            expires_at: TimestampMicros::from_unix_micros(now.as_unix_micros() + 60_000_000),
            requesting_provenance: RequestingProvenance {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".into(),
                },
                host: Some("codex".into()),
                session: Some("host-test".into()),
                basis: vec!["test".into()],
            },
        })
        .expect("Guarded request");
    let shown = call(
        &mut adapter,
        "guarded_interaction",
        json!({"confirmation_request_id":request.confirmation_request_identity.to_string()}),
    );
    let shown = structured(&shown);
    assert_eq!(shown["host_elicitation_available"], false);
    assert_eq!(
        shown["confirmation_request_id"],
        request.confirmation_request_identity.to_string()
    );
    assert_eq!(shown["request_revision"], request.request_revision);
    assert_eq!(shown["effect_fingerprint"], request.effect_fingerprint);
    assert_eq!(shown["fallback"]["cli"][2], "confirm");

    let confirmed = call(
        &mut adapter,
        "guarded_interaction",
        json!({
            "confirmation_request_id":request.confirmation_request_identity.to_string(),
            "request_revision":request.request_revision,
            "effect_fingerprint":request.effect_fingerprint,
            "decision":"confirm",
            "user_turn":"confirm this exact release"
        }),
    );
    let confirmed = structured(&confirmed);
    assert_eq!(
        confirmed["confirmation_request_id"],
        request.confirmation_request_identity.to_string()
    );
    assert_eq!(confirmed["request_revision"], request.request_revision);
    let source = confirmed["user_response_source_id"]
        .as_str()
        .expect("response Source");
    let canonical = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical");
    assert!(canonical
        .sources
        .iter()
        .any(|basis| basis.source.id.to_string() == source));
}

fn call(adapter: &mut HostAdapter, name: &str, arguments: Value) -> Value {
    adapter.handle(json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":name,"arguments":arguments}})).expect("tool response")
}
fn structured(response: &Value) -> &Value {
    &response["result"]["structuredContent"]
}
fn parse_project(value: &str) -> volicord_context::ProjectId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::ProjectId::from_bytes(bytes)
}
