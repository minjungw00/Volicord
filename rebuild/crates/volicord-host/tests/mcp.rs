use serde_json::{json, Value};
use std::{collections::BTreeSet, fs};
use tempfile::tempdir;
use volicord_host::{run_stdio, HostAdapter, HOST_TOOL_NAMES};
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_privacy::{
    ProviderIntentProvenance, ProviderOptInPolicy, ProviderRetentionPolicy, SecretFilteringPolicy,
    SourceExclusionPolicy,
};

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
            "context_record",
            json!({"project_id":project}),
            "user_turn is required",
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
fn codex_host_exposes_guarded_provider_dispatch_and_durable_unavailable_outcome() {
    use volicord_context::{Clock, Principal, PrincipalKind, SystemClock};

    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src")).expect("source directory");
    fs::write(
        repository.join("src/lib.rs"),
        "// SECRET=host-fixture\npub fn host_path() {}\n",
    )
    .expect("source file");
    let project_id = parse_project(&project);
    adapter
        .operations()
        .analyze(project_id, Vec::new())
        .expect("analysis");
    let opt_in_source = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "privacy-host".into(),
            "enable provider".into(),
        )
        .expect("opt-in source");
    adapter
        .operations()
        .enable_provider(
            ProviderOptInPolicy {
                project_id,
                provider: "configured-provider".into(),
                model: "configured-model".into(),
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: vec!["src/lib.rs".into()],
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "host fixture".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: true,
                    line_markers: vec!["SECRET".into()],
                    replacement: "[filtered]".into(),
                    known_limits: vec!["marker filtering is incomplete".into()],
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: "configured provider policy".into(),
                    provider_known_limits: Vec::new(),
                },
            },
            ProviderIntentProvenance {
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-host-user".into(),
                },
                host: "codex".into(),
                session: "privacy-host".into(),
                user_turn_source: parse_source_identity(&opt_in_source.identity),
                basis: "explicit host fixture opt-in".into(),
            },
        )
        .expect("provider opt-in");
    adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{"elicitation":{}}}}));
    let now = SystemClock.now().expect("clock");
    let prepared = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"prepare",
            "project_id":project,
            "provider":"configured-provider",
            "model":"configured-model",
            "purpose":"background semantic analysis",
            "requested_capability":"semantic",
            "source_paths":["src/lib.rs"],
            "expiration_unix_micros":u64::try_from(now.as_unix_micros() + 60_000_000).expect("positive expiration")
        }),
    );
    assert_eq!(prepared["result"]["isError"], false, "{prepared}");
    let prepared = structured(&prepared);
    assert_eq!(prepared["state"], "awaiting_exact_confirmation");
    assert_eq!(prepared["dispatch_occurred"], false);
    let request = prepared["guarded_request"].clone();
    let provider_request_id = prepared["provider_request"]["provider_request_id"]
        .as_str()
        .expect("provider request ID")
        .to_owned();

    let mismatched = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":format!("sha256:{}", "0".repeat(64))
        }),
    );
    assert_eq!(
        structured(&mismatched)["guarded_outcome"]["rejection"],
        "mismatched"
    );
    assert_eq!(
        structured(&mismatched)["provider_request"]["outcome"],
        "prepared"
    );

    let missing = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(
        structured(&missing)["guarded_outcome"]["kind"],
        "not_dispatched"
    );
    assert_eq!(
        structured(&missing)["guarded_outcome"]["rejection"],
        "missing"
    );
    assert_eq!(
        structured(&missing)["provider_request"]["outcome"],
        "prepared"
    );

    let confirmed = call(
        &mut adapter,
        "guarded_interaction",
        json!({
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"],
            "decision":"confirm",
            "user_turn":"confirm this exact provider request"
        }),
    );
    assert_eq!(confirmed["result"]["isError"], false, "{confirmed}");
    let dispatched = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    let dispatched = structured(&dispatched);
    assert_eq!(dispatched["guarded_outcome"]["kind"], "not_dispatched");
    assert_eq!(dispatched["guarded_outcome"]["confirmation_consumed"], true);
    assert_eq!(
        dispatched["provider_request"]["outcome"],
        "provider_unavailable"
    );
    assert!(dispatched["provider_request"]["manifest"]
        .as_array()
        .expect("manifest")
        .iter()
        .all(|entry| entry["transmission_outcome"] == "not_transmitted"));
    let operation_id = dispatched["operation_id"].as_str().expect("operation ID");

    let inspected = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"inspect",
            "project_id":project,
            "operation_id":operation_id,
            "provider_request_id":provider_request_id
        }),
    );
    assert_eq!(structured(&inspected), dispatched);

    let reused = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(reused["result"]["isError"], true, "{reused}");
    assert!(structured(&reused)["error"]
        .as_str()
        .is_some_and(|error| error.contains("live provider preparation is unavailable")));
    let local = call(
        &mut adapter,
        "canonical_inspect",
        json!({"project_id":project}),
    );
    assert_eq!(local["result"]["isError"], false, "{local}");
    let structural = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project,"excluded_paths":[]}),
    );
    assert_eq!(structural["result"]["isError"], false, "{structural}");
}

#[test]
fn explicit_provider_denial_discards_the_live_preparation() {
    let (_temporary, mut adapter, project, request) = setup_guarded_provider_request(60_000_000);
    let denied = call(
        &mut adapter,
        "guarded_interaction",
        json!({
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"],
            "decision":"deny",
            "user_turn":"deny this exact provider request"
        }),
    );
    assert_eq!(structured(&denied)["decision"], "denied", "{denied}");

    let dispatch = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(dispatch["result"]["isError"], true, "{dispatch}");
    assert!(structured(&dispatch)["error"]
        .as_str()
        .is_some_and(|error| error.contains("live provider preparation is unavailable")));
    assert_eq!(
        call(
            &mut adapter,
            "canonical_inspect",
            json!({"project_id":project})
        )["result"]["isError"],
        false
    );
}

#[test]
fn subsequent_host_interaction_cleans_an_expired_provider_preparation() {
    let (_temporary, mut adapter, project, request) = setup_guarded_provider_request(100_000);
    std::thread::sleep(std::time::Duration::from_millis(150));

    let health = call(
        &mut adapter,
        "project_health",
        json!({"project_id":project}),
    );
    assert_eq!(health["result"]["isError"], false, "{health}");

    let dispatch = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(dispatch["result"]["isError"], true, "{dispatch}");
    assert!(structured(&dispatch)["error"]
        .as_str()
        .is_some_and(|error| error.contains("live provider preparation is unavailable")));
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

#[test]
fn current_host_goal_context_is_canonical_and_recalled_from_exact_user_text() {
    let (_temporary, mut adapter, project) = setup();
    let user_turn =
        "For this work, make grounded checkpoints available to ordinary Codex sessions.";
    let statement = "make grounded checkpoints available to ordinary Codex sessions";
    let recorded = structured(&call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":user_turn,
            "role":"goal",
            "statement":statement,
        }),
    ))
    .clone();
    assert_eq!(recorded["role"], "goal", "{recorded}");
    assert_eq!(
        recorded["source_id"].as_str().map(str::len),
        Some(32),
        "{recorded}"
    );
    assert_eq!(
        recorded["context_item_id"].as_str().map(str::len),
        Some(32),
        "{recorded}"
    );

    let recall = structured(&call(&mut adapter, "recall", json!({"project_id":project}))).clone();
    assert_eq!(recall["goals"], json!([statement]), "{recall}");

    let rejected = call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"The user only stated a narrow goal.",
            "role":"goal",
            "statement":"An agent-authored expansion that the user did not state",
        }),
    );
    assert_eq!(rejected["result"]["isError"], true, "{rejected}");
    assert!(structured(&rejected)["error"]
        .as_str()
        .is_some_and(|error| error.contains("occur verbatim")));
}

#[test]
fn supported_candidate_research_is_source_grounded_and_separate_from_promotion_and_decision() {
    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src")).expect("source directory");
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn research_fixture() {}\n",
    )
    .expect("source file");
    let project_id = parse_project(&project);
    let analyzed = adapter
        .operations()
        .analyze(project_id, Vec::new())
        .expect("analysis")
        .value
        .expect("completed analysis");
    let repository_source = analyzed.analysis.repository_source.identity().to_string();
    let wrong_source = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "candidate-research".into(),
            "this user turn is not repository research".into(),
        )
        .expect("non-repository Source")
        .identity;
    let mut arguments =
        question_candidate_arguments(&project, &repository_source, 1, "Choose repository policy");
    arguments["research_state"] = json!("research_required");
    arguments["research_state_basis"] =
        json!("repository facts must be established before asking for user judgment");
    let submitted = structured(&call(&mut adapter, "candidate_manage", arguments)).clone();
    assert_eq!(submitted["research_state"], "research_required");
    let candidate_id = submitted["candidate_id"]
        .as_str()
        .expect("Candidate identity")
        .to_owned();
    assert!(adapter
        .operations()
        .inquiry_frontier(project_id, Vec::new())
        .expect("frontier before research")
        .questions
        .is_empty());

    let premature = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"mark_research_ready",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    );
    assert_eq!(premature["result"]["isError"], true, "{premature}");

    let mismatched_source = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"attach_repository_research",
            "project_id":project,
            "candidate_id":candidate_id,
            "capability":"structural",
            "coverage":"src/lib.rs",
            "freshness":"current",
            "source_ids":[wrong_source],
            "evidence_assessment":"sufficient",
            "limits":[]
        }),
    );
    assert_eq!(
        mismatched_source["result"]["isError"], true,
        "{mismatched_source}"
    );

    let insufficient = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"attach_repository_research",
            "project_id":project,
            "candidate_id":candidate_id,
            "capability":"structural",
            "coverage":"src/lib.rs declarations only",
            "freshness":"current",
            "source_ids":[repository_source],
            "evidence_assessment":"insufficient",
            "limits":["runtime behavior remains unknown"]
        }),
    ))
    .clone();
    assert_eq!(insufficient["research_state"], "research_required");
    assert_eq!(insufficient["promoted"], false);
    assert_eq!(insufficient["canonical_mutation"], false);
    assert!(adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after research attachment")
        .active_questions
        .is_empty());

    let still_premature = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"mark_research_ready",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    );
    assert_eq!(
        still_premature["result"]["isError"], true,
        "{still_premature}"
    );

    let sufficient = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"attach_repository_research",
            "project_id":project,
            "candidate_id":candidate_id,
            "capability":"structural",
            "coverage":"current repository policy implementation and call sites",
            "freshness":"current",
            "source_ids":[repository_source],
            "evidence_assessment":"sufficient",
            "limits":["external runtime behavior is excluded"]
        }),
    ))
    .clone();
    assert_eq!(sufficient["research_state"], "research_required");
    assert_eq!(sufficient["promoted"], false);
    assert_eq!(
        sufficient["repository_research"][1]["analysis_snapshot"],
        analyzed.analysis.identity.to_string()
    );
    assert_eq!(
        sufficient["repository_research"][1]["repository_snapshot"],
        analyzed.analysis.repository_snapshot.to_string()
    );

    let ready = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"mark_research_ready",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    ))
    .clone();
    assert_eq!(ready["research_state"], "ready_to_ask");
    assert_eq!(ready["promoted"], false);
    let inspected = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(inspected["candidates"][0]["research_state"], "ready_to_ask");
    assert_eq!(
        inspected["candidates"][0]["repository_research"]
            .as_array()
            .expect("research evidence")
            .len(),
        2
    );

    let promoted = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"promote_question",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    ))
    .clone();
    let frontier = structured(&call(
        &mut adapter,
        "inquiry_frontier",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(
        frontier["questions"][0]["identity"],
        promoted["question_id"]
    );
    let decided = structured(&call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id":project,
            "question_id":promoted["question_id"],
            "question_revision":frontier["questions"][0]["revision"],
            "alternative_key":"local",
            "user_turn":"Choose the local repository policy",
            "user_rationale":"Keep this Project local-first"
        }),
    ))
    .clone();
    assert_eq!(decided["all_succeeded"], true, "{decided}");
}

#[test]
fn supported_candidate_path_requires_explicit_promotion_and_current_host_decision() {
    use volicord_context::{
        Availability, OperationId, Principal, PrincipalKind, SourceDraft, SourcePayload, Store,
    };

    let (_temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let mut store = Store::open(adapter.operations().layout().canonical_store())
        .expect("open canonical test support store");
    let canonical_project = store.get_project(project_id).expect("load Project");
    let source = store
        .record_source(
            OperationId::from_bytes([211; 16]),
            project_id,
            SourceDraft {
                expected_project_revision: canonical_project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "candidate-host-fixture".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "candidate-host-fixture".into(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".into(),
                }),
                availability: Availability::Available,
            },
        )
        .expect("record Candidate Source")
        .value;
    drop(store);

    let submitted = structured(&call(
        &mut adapter,
        "candidate_manage",
        question_candidate_arguments(&project, &source.id.to_string(), 1, "Choose storage"),
    ))
    .clone();
    assert_eq!(submitted["state"], "stored", "{submitted}");
    assert_eq!(submitted["research_state"], "ready_to_ask");
    assert_eq!(
        submitted["research_state_basis"],
        "the unresolved branch is purely a user judgment"
    );
    assert_eq!(submitted["canonical_mutation"], false);
    let candidate_id = submitted["candidate_id"]
        .as_str()
        .expect("Candidate identity")
        .to_owned();

    let before_promotion = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical before promotion");
    assert!(before_promotion.active_questions.is_empty());
    assert!(before_promotion.active_decisions.is_empty());
    let inspected = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    let candidate = inspected["candidates"]
        .as_array()
        .expect("Candidate inspection")
        .iter()
        .find(|value| value["identity"] == candidate_id)
        .expect("submitted Candidate");
    assert_eq!(candidate["disposition"]["state"], "pending_or_retained");
    assert_eq!(candidate["origin"]["actor_kind"], "agent");
    assert_eq!(candidate["research_state"], "ready_to_ask");
    assert!(candidate["repository_research"]
        .as_array()
        .expect("repository research")
        .is_empty());
    assert_eq!(
        candidate["observation_basis"]["source_ids"][0],
        source.id.to_string()
    );
    assert!(candidate["observation_basis"]["other"]
        .as_str()
        .is_some_and(|basis| basis.contains("purely a user judgment")));
    assert_eq!(
        candidate["collection_scope"]["source_operation"],
        "design-review"
    );
    assert_eq!(inspected["read_only"], true);
    assert!(adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after inspection")
        .active_questions
        .is_empty());

    let promoted = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"promote_question",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    ))
    .clone();
    let question_id = promoted["question_id"]
        .as_str()
        .expect("promoted Question identity")
        .to_owned();
    let frontier = structured(&call(
        &mut adapter,
        "inquiry_frontier",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(frontier["questions"][0]["identity"], question_id);
    let after_promotion = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after promotion");
    assert_eq!(after_promotion.active_questions.len(), 1);
    assert!(after_promotion.active_decisions.is_empty());

    let decided = structured(&call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id":project,
            "question_id":question_id,
            "question_revision":frontier["questions"][0]["revision"],
            "alternative_key":"local",
            "user_turn":"Choose the local Candidate alternative",
            "user_rationale":"Keep canonical state local"
        }),
    ))
    .clone();
    assert_eq!(decided["all_succeeded"], true, "{decided}");
    let after_decision = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after explicit response");
    assert_eq!(after_decision.active_decisions.len(), 1);

    let second = structured(&call(
        &mut adapter,
        "candidate_manage",
        question_candidate_arguments(&project, &source.id.to_string(), 2, "Choose cache"),
    ))
    .clone();
    let dismissed = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"dismiss",
            "project_id":project,
            "candidate_id":second["candidate_id"],
            "reason":"not material to the current work"
        }),
    ))
    .clone();
    assert_eq!(dismissed["disposition"]["state"], "dismissed");
    assert_eq!(
        adapter
            .operations()
            .canonical_basis(project_id)
            .expect("canonical after dismissal")
            .active_questions
            .len(),
        0
    );
}

fn setup_guarded_provider_request(
    expiration_delta_micros: i64,
) -> (tempfile::TempDir, HostAdapter, String, Value) {
    use volicord_context::{Clock, Principal, PrincipalKind, SystemClock};

    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src")).expect("source directory");
    fs::write(repository.join("src/lib.rs"), "pub fn host_path() {}\n").expect("source file");
    let project_id = parse_project(&project);
    adapter
        .operations()
        .analyze(project_id, Vec::new())
        .expect("analysis");
    let opt_in_source = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "privacy-host".into(),
            "enable provider".into(),
        )
        .expect("opt-in source");
    adapter
        .operations()
        .enable_provider(
            ProviderOptInPolicy {
                project_id,
                provider: "configured-provider".into(),
                model: "configured-model".into(),
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: vec!["src/lib.rs".into()],
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "host cleanup fixture".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: false,
                    line_markers: Vec::new(),
                    replacement: "[filtered]".into(),
                    known_limits: Vec::new(),
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: "configured provider policy".into(),
                    provider_known_limits: Vec::new(),
                },
            },
            ProviderIntentProvenance {
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-host-user".into(),
                },
                host: "codex".into(),
                session: "privacy-host".into(),
                user_turn_source: parse_source_identity(&opt_in_source.identity),
                basis: "explicit host fixture opt-in".into(),
            },
        )
        .expect("provider opt-in");
    let now = SystemClock.now().expect("clock");
    let expiration = now
        .as_unix_micros()
        .checked_add(expiration_delta_micros)
        .and_then(|value| u64::try_from(value).ok())
        .expect("positive supported expiration");
    let prepared = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"prepare",
            "project_id":project,
            "provider":"configured-provider",
            "model":"configured-model",
            "purpose":"background semantic analysis",
            "requested_capability":"semantic",
            "source_paths":["src/lib.rs"],
            "expiration_unix_micros":expiration
        }),
    );
    assert_eq!(prepared["result"]["isError"], false, "{prepared}");
    let request = structured(&prepared)["guarded_request"].clone();
    (temporary, adapter, project, request)
}

fn question_candidate_arguments(project: &str, source: &str, order: u64, prompt: &str) -> Value {
    json!({
        "action":"submit_question",
        "project_id":project,
        "source_ids":[source],
        "source_operation":"design-review",
        "repository_snapshot":"candidate-host-fixture",
        "research_state":"ready_to_ask",
        "research_state_basis":"the unresolved branch is purely a user judgment",
        "retention_basis":"retain through explicit inquiry disposition",
        "bounded_summary":format!("material Candidate: {prompt}"),
        "prompt":prompt,
        "why_now":"the implementation result depends on this choice",
        "affected_scope":["storage"],
        "established_facts":["Canonical context is local"],
        "assumptions":["the Project remains local-first"],
        "uncertainty":["future scale is unknown"],
        "alternatives":[
            {"key":"local","label":"Local","consequence":"Keep canonical state local"},
            {"key":"remote","label":"Remote","consequence":"Require a separate provider boundary"}
        ],
        "recommendation_key":"local",
        "recommendation_rationale":"matches the local-first contract",
        "trade_offs":["remote augmentation remains separate"],
        "known_limits":["provider behavior is not evaluated"],
        "what_unlocks":["the storage implementation"],
        "materiality_rationale":"the choice changes durable behavior",
        "duplicate_basis":"canonical inspection found no matching Question",
        "presentation_order":order
    })
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

fn parse_source_identity(value: &str) -> volicord_context::SourceId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::SourceId::from_bytes(bytes)
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
        "background_semantic_operation" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "provider",
                    "model",
                    "purpose",
                    "requested_capability",
                    "source_paths",
                    "expiration_unix_micros",
                ],
                &[
                    "action",
                    "project_id",
                    "provider",
                    "model",
                    "purpose",
                    "requested_capability",
                    "source_paths",
                    "expiration_unix_micros",
                ],
            ),
            shape(
                &[
                    "action",
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                ],
                &[
                    "action",
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "operation_id",
                    "provider_request_id",
                ],
                &[
                    "action",
                    "project_id",
                    "operation_id",
                    "provider_request_id",
                ],
            ),
        ],
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
        "context_record" => vec![shape(
            &["project_id", "user_turn", "role", "statement"],
            &["project_id", "user_turn", "role", "statement"],
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
        "candidate_manage" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "source_ids",
                    "source_operation",
                    "repository_snapshot",
                    "research_state",
                    "research_state_basis",
                    "retention_basis",
                    "bounded_summary",
                    "prompt",
                    "why_now",
                    "affected_scope",
                    "established_facts",
                    "assumptions",
                    "uncertainty",
                    "alternatives",
                    "recommendation_key",
                    "recommendation_rationale",
                    "trade_offs",
                    "known_limits",
                    "what_unlocks",
                    "materiality_rationale",
                    "duplicate_basis",
                    "presentation_order",
                ],
                &[
                    "action",
                    "project_id",
                    "source_ids",
                    "source_operation",
                    "research_state",
                    "research_state_basis",
                    "retention_basis",
                    "bounded_summary",
                    "prompt",
                    "why_now",
                    "affected_scope",
                    "alternatives",
                    "recommendation_key",
                    "recommendation_rationale",
                    "materiality_rationale",
                    "duplicate_basis",
                    "presentation_order",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "candidate_id",
                    "capability",
                    "coverage",
                    "freshness",
                    "source_ids",
                    "evidence_assessment",
                    "limits",
                ],
                &[
                    "action",
                    "project_id",
                    "candidate_id",
                    "capability",
                    "coverage",
                    "freshness",
                    "source_ids",
                    "evidence_assessment",
                ],
            ),
            shape(
                &["action", "project_id", "candidate_id"],
                &["action", "project_id", "candidate_id"],
            ),
            shape(
                &["action", "project_id", "candidate_id"],
                &["action", "project_id", "candidate_id"],
            ),
            shape(
                &["action", "project_id", "candidate_id", "reason"],
                &["action", "project_id", "candidate_id", "reason"],
            ),
            shape(
                &["action", "project_id", "candidate_id", "basis"],
                &["action", "project_id", "candidate_id", "basis"],
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
