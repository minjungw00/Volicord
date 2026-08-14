use serde_json::{json, Value};
use std::{collections::BTreeSet, fs};
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
    for tool in listed["result"]["tools"].as_array().expect("tool array") {
        assert_schema_is_closed_and_described(&tool["inputSchema"]);
        assert_eq!(
            schema_shapes(&tool["inputSchema"]),
            expected_shapes(tool["name"].as_str().expect("tool name")),
            "schema/handler field contract drift for {}",
            tool["name"]
        );
    }
}

#[test]
fn schema_validation_rejects_unknown_missing_and_malformed_arguments_before_mutation() {
    let (_temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let before = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical basis before invalid calls");

    for name in HOST_TOOL_NAMES {
        let response = call(&mut adapter, name, json!({"unexpected":true}));
        assert_eq!(response["result"]["isError"], true, "{name}: {response}");
        assert!(structured(&response)["error"]
            .as_str()
            .is_some_and(|error| error.contains("is not allowed")));
    }

    for (name, arguments, expected) in [
        ("recall", json!({}), "project_id is required"),
        (
            "decision_record",
            json!({"project_id":project}),
            "question_id is required",
        ),
        (
            "canonical_mutate",
            json!({"action":"forget","project_id":project,"user_turn":"forget"}),
            "does not match any allowed shape",
        ),
        (
            "document_preview",
            json!({"project_id":project,"kind":"handoff-resume","format":"pdf"}),
            "is not an allowed value",
        ),
        (
            "guarded_interaction",
            json!({"confirmation_request_id":"00000000000000000000000000000000","decision":"confirm"}),
            "does not match any allowed shape",
        ),
    ] {
        let response = call(&mut adapter, name, arguments);
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert!(
            structured(&response)["error"]
                .as_str()
                .is_some_and(|error| error.contains(expected)),
            "{response}"
        );
    }

    let health = call(&mut adapter, "project_health", json!({}));
    assert_eq!(health["result"]["isError"], false, "{health}");
    let recall = call(&mut adapter, "recall", json!({"project_id":project}));
    assert_eq!(recall["result"]["isError"], false, "{recall}");

    let after = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical basis after invalid calls");
    assert_eq!(before, after);
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

fn assert_schema_is_closed_and_described(schema: &Value) {
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        assert!(!variants.is_empty());
        for variant in variants {
            assert_schema_is_closed_and_described(variant);
        }
        return;
    }
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    let properties = schema["properties"].as_object().expect("properties");
    assert!(!properties.is_empty());
    for property in properties.values() {
        assert!(property["description"].as_str().is_some());
    }
}

fn schema_shapes(schema: &Value) -> Vec<(BTreeSet<String>, BTreeSet<String>)> {
    let variants = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(schema));
    variants
        .iter()
        .map(|variant| {
            let properties = variant["properties"]
                .as_object()
                .expect("properties")
                .keys()
                .cloned()
                .collect();
            let required = variant["required"]
                .as_array()
                .expect("required")
                .iter()
                .map(|value| value.as_str().expect("required name").to_owned())
                .collect();
            (properties, required)
        })
        .collect()
}

fn expected_shapes(name: &str) -> Vec<(BTreeSet<String>, BTreeSet<String>)> {
    let shape = |properties: &[&str], required: &[&str]| {
        (
            properties.iter().map(|value| (*value).to_owned()).collect(),
            required.iter().map(|value| (*value).to_owned()).collect(),
        )
    };
    match name {
        "project_initialize" => vec![shape(&["display_name", "repository"], &["display_name"])],
        "project_health" => vec![shape(&["project_id"], &[])],
        "recall"
        | "repository_understanding"
        | "canonical_inspect"
        | "candidate_inspect"
        | "privacy_status" => {
            vec![shape(&["project_id"], &["project_id"])]
        }
        "repository_analyze" => vec![shape(&["project_id", "excluded_paths"], &["project_id"])],
        "inquiry_frontier" => vec![shape(&["project_id", "material_scope"], &["project_id"])],
        "decision_record" => vec![shape(
            &[
                "project_id",
                "question_id",
                "question_revision",
                "alternative_key",
                "user_turn",
                "user_rationale",
            ],
            &[
                "project_id",
                "question_id",
                "question_revision",
                "alternative_key",
                "user_turn",
            ],
        )],
        "checkpoint_record" => vec![shape(
            &[
                "project_id",
                "user_turn",
                "goal",
                "next_step",
                "known_limits",
            ],
            &["project_id", "user_turn", "goal", "next_step"],
        )],
        "canonical_mutate" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "alternative_key",
                    "rationale",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "alternative_key",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "record_kind",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "record_kind",
                ],
            ),
        ],
        "document_preview" => vec![shape(
            &["project_id", "kind", "format", "language", "locale"],
            &["project_id", "kind"],
        )],
        "guarded_interaction" => vec![
            shape(&["confirmation_request_id"], &["confirmation_request_id"]),
            shape(
                &[
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                    "decision",
                    "user_turn",
                ],
                &[
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                    "decision",
                    "user_turn",
                ],
            ),
        ],
        _ => panic!("unexpected public tool {name}"),
    }
}
