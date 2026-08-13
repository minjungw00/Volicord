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

#[test]
fn current_host_decision_and_checkpoint_calls_preserve_user_turn_sources() {
    use volicord_context::{
        AgentRecommendation, Availability, NonUserQuestionOutcome, OperationId, Principal,
        PrincipalKind, QuestionAlternative, QuestionDraft, QuestionMateriality,
        QuestionResearchState, SourceDraft, SourcePayload, Store,
    };

    let (_temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let mut store = Store::open(adapter.operations().layout().canonical_store())
        .expect("open canonical test support store");
    let canonical_project = store.get_project(project_id).expect("load Project");
    let basis = store
        .record_source(
            OperationId::from_bytes([201; 16]),
            project_id,
            SourceDraft {
                expected_project_revision: canonical_project.revision,
                payload: SourcePayload::File {
                    locator: "src/policy.rs".into(),
                    snapshot: "v08-fixture".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "v08-fixture".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )
        .expect("record Question basis")
        .value;
    let question = store
        .create_question(
            OperationId::from_bytes([202; 16]),
            project_id,
            QuestionDraft {
                expected_project_revision: canonical_project.revision,
                prompt_basis: "Choose the V08 storage boundary".into(),
                source_basis: vec![basis.id],
                dependencies: Vec::new(),
                alternatives: vec![
                    QuestionAlternative {
                        key: "local".into(),
                        label: "Local".into(),
                        consequence: "Keep canonical data local".into(),
                    },
                    QuestionAlternative {
                        key: "remote".into(),
                        label: "Remote".into(),
                        consequence: "Require a separate provider decision".into(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("local".into()),
                    rationale: "The accepted product boundary is local-first".into(),
                    source_basis: vec![basis.id],
                },
                trade_offs: vec!["Remote augmentation remains separate".into()],
                uncertainty: Vec::new(),
                material_scope: vec!["storage".into()],
                materiality: QuestionMateriality::Material,
                presentation_order: 1,
                why_it_matters_now: "The host journey needs an exact user choice".into(),
                established_facts: Vec::new(),
                assumptions: Vec::new(),
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["V08 Decision transport".into()],
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state: QuestionResearchState::ReadyToAsk,
            },
        )
        .expect("create Question")
        .value;
    drop(store);

    let decision = structured(&call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id": project,
            "question_id": question.id.to_string(),
            "question_revision": question.revision,
            "alternative_key": "local",
            "user_turn": "Use the local storage boundary",
            "user_rationale": "Canonical project memory remains local"
        }),
    ))
    .clone();
    assert_eq!(decision["all_succeeded"], true, "{decision}");
    let decision_source = decision["user_response_source_id"]
        .as_str()
        .expect("Decision Source")
        .to_owned();

    let checkpoint = structured(&call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id": project,
            "user_turn": "Record a handoff checkpoint",
            "goal": "Complete the clean host journey",
            "next_step": "Run maintained V08 assertions",
            "known_limits": ["V11 is independent"]
        }),
    ))
    .clone();
    let checkpoint_source = checkpoint["user_response_source_id"]
        .as_str()
        .expect("Checkpoint Source")
        .to_owned();
    assert_ne!(decision_source, checkpoint_source);

    let canonical = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical basis");
    for source_id in [decision_source, checkpoint_source] {
        let source = canonical
            .sources
            .iter()
            .find(|basis| basis.source.id.to_string() == source_id)
            .expect("current-host Source remains canonical");
        assert!(matches!(
            source.source.payload,
            SourcePayload::CurrentHostUserTurn { ref host, .. } if host == "codex"
        ));
        assert_eq!(source.source.actor.kind, PrincipalKind::User);
    }
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
