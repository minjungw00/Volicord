#![forbid(unsafe_code)]

mod support;

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    ffi::OsStr,
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin as ProcessStdin, Command, Output, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};

use serde_json::{json, Value};
use support::binary_fixture::{run_child, ChildStdin};
use support::json::adapter_tool_response;
use toml_edit::DocumentMut;
use volicord_host_contract::{
    project_mcp_tool, CodexHookPromptCorrelation, HostNativeCorrelation, HostSessionId, HostTurnId,
    McpServerKey,
};
use volicord_mcp::{
    ManagedMcpInvocationPurpose, ManagedMcpLaunchSpec, ManagedMcpMaterializationInput,
    ManagedMcpWorkingDirectory, VOLICORD_HOME_ENV,
};
use volicord_store::agent_connections::{agent_connection_record, AgentConnectionRecord};
use volicord_store::diagnostic_findings::{
    diagnostic_occurrences_for_runtime_session, stored_diagnostic_findings_by_ids,
};
use volicord_store::export::read_authority_bundle_snapshot;
use volicord_store::guards::{
    agent_session, agent_session_matches_current_integration,
    current_project_agent_session_coordinates, repository_observation, RepositoryObservationState,
    RepositoryObservationUnavailableReason,
};
use volicord_store::inspection::{
    inspect_runtime_home, AgentConnectionInspectionRecord, DatabaseInspection,
    RegistryInspectionSnapshot,
};
use volicord_store::operational_sessions::{
    connection_integration_revision, current_managed_runtime_sessions,
    latest_current_managed_runtime_session, mcp_runtime_session_for_process,
    McpRuntimeSessionStart,
};
use volicord_test_support::{
    core_fixtures::ExactToolCallTranscript, IsolatedGitRepository, TempRuntimeHome,
};
use volicord_types::diagnostics::DiagnosticFindingId;
use volicord_types::guard_manifest::{
    guard_manifest_from_json, GuardManagedArtifact, GuardManagedOwnership, GuardManifest,
};
use volicord_types::integration_revision::McpRuntimeSessionSource;
use volicord_types::integration_verification::{
    GuardProbeObservationStage, GuardVerificationRepairReason, IntegrationVerificationWorkflowState,
};
use volicord_types::tool_names::{AgentToolId, ToolVerificationRole};
use volicord_types::values::{AgentConnectionMode, GuardHookPhase};

const FUTURE_VERSION: &str = "999.0.0";
const NEXT_FUTURE_VERSION: &str = "1000.0.0";
const NATIVE_SESSION_999: &str = "future.session.999";
const NATIVE_SESSION_1000: &str = "future.session.1000";
const NATIVE_THREAD: &str = "future.thread.operational";
const MCP_FIXTURE_MODE: &str = "VOLICORD_TEST_MCP_FIXTURE";
const CODEX_VERSION_ENV: &str = "VOLICORD_TEST_CODEX_VERSION";
const EARLY_EXIT_STDERR_BYTES: usize = 3 * 1024;
const CODEX_COMPATIBILITY_VERSION: &str = "0.108.0-alpha.12";
const CODEX_COMPATIBILITY_REVISION: &str = "2025-06-18";
const INTEGRATION_VERIFICATION_TURN_ID: &str = "future.turn.integration-verification";
const INTEGRATION_VERIFICATION_TOOL_USE_ID: &str = "future.tool-use.guard-probe";
const TRANSFORMED_TRACKED_PATH: &str = "fixtures/transformed-record.txt";
const MCP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, PartialEq, Eq)]
struct GuardAttemptTerminalSnapshot {
    status: String,
    status_read_count: i64,
    probe_acknowledged_at: String,
    completed_at: Option<String>,
    matched_prompt_event_id: Option<String>,
    matched_pre_tool_event_id: Option<String>,
    matched_post_tool_event_id: Option<String>,
}

fn host_session_correlation(session_id: &str) -> HostNativeCorrelation {
    HostNativeCorrelation::CodexHookPrompt(CodexHookPromptCorrelation {
        session_id: HostSessionId::parse(session_id).expect("valid test session"),
        turn_id: HostTurnId::parse("turn.session-coordinate").expect("valid test turn"),
    })
}

fn managed_host_round_trip_tool() -> AgentToolId {
    ToolVerificationRole::ManagedHostRoundTrip.tool()
}

fn main() {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args == [OsStr::new("--version")] {
        println!(
            "codex-cli {}",
            env::var(CODEX_VERSION_ENV).unwrap_or_else(|_| FUTURE_VERSION.to_owned())
        );
        return;
    }
    if args.first().is_some_and(|arg| arg == "mcp") {
        match env::var(MCP_FIXTURE_MODE).as_deref() {
            Ok("startup_failure") => {
                eprintln!("deterministic MCP fixture startup failure");
                std::process::exit(70);
            }
            Ok("early_stdio_exit") if args.iter().any(|arg| arg == "preflight") => {
                let connection_id = args
                    .windows(2)
                    .find(|pair| pair[0] == "--connection")
                    .and_then(|pair| pair[1].to_str())
                    .expect("fixture preflight connection ID");
                println!(
                    "{{\"operation\":\"mcp_preflight\",\"status\":\"passed\",\"side_effects\":[],\"evidence_class\":\"read_only_preflight\",\"configuration\":\"valid\",\"canonical_managed_entry\":\"passed\",\"transport\":\"stdio\",\"connection_id\":\"{connection_id}\",\"mode\":\"workflow\",\"enabled\":true,\"registry_read\":\"passed\",\"project_state_read\":\"passed\",\"writeability\":{{\"status\":\"not_checked\",\"requirement\":\"requires_active_verification\"}},\"effective_tool_mode\":\"requires_active_verification\",\"tools_list_schema_validation\":\"passed\",\"protocol_profiles\":[\"2025-11-25\"],\"host_contracts\":[{{\"profile\":\"codex\",\"digest\":\"sha256:fixture\"}}],\"projects\":[]}}"
                );
                return;
            }
            Ok("early_stdio_exit") if args.iter().any(|arg| arg == "serve") => {
                eprint!("{}", "x".repeat(EARLY_EXIT_STDERR_BYTES));
                std::process::exit(23);
            }
            _ => {}
        }
    }

    if let Err(error) = run_operational_regressions() {
        panic!("operational host end-to-end regression failed: {error}");
    }
}

fn run_operational_regressions() -> Result<(), Box<dyn Error>> {
    planning_schema_recovery_reaches_implementation()
        .map_err(|error| format!("planning schema-recovery regression: {error}"))?;
    planning_product_explicit_shaping_journey()
        .map_err(|error| format!("planning-product shaping regression: {error}"))?;
    codex_2025_06_18_compatibility_records_managed_runtime_facts()
        .map_err(|error| format!("compatibility regression: {error}"))?;
    verification_tool_designation_mismatch_is_typed()
        .map_err(|error| format!("designation regression: {error}"))?;
    status_tool_self_observation_preserves_missing_probe_reason()
        .map_err(|error| format!("status self-observation regression: {error}"))?;
    managed_launch_contracts_survive_filtered_environments()
        .map_err(|error| format!("filtered environment regression: {error}"))?;
    complete_managed_activation_journey_and_read_only_status()
        .map_err(|error| format!("activation journey regression: {error}"))?;
    connection_list_evaluates_multiple_memberships_independently()
        .map_err(|error| format!("membership regression: {error}"))?;
    connection_mode_transition_rebinds_guard_revision()
        .map_err(|error| format!("mode transition regression: {error}"))?;
    connection_mode_preflight_failure_preserves_connection()
        .map_err(|error| format!("preflight failure regression: {error}"))?;
    connection_removal_after_operational_observations()
        .map_err(|error| format!("connection removal regression: {error}"))?;
    drift_verification_preserves_owned_configuration_and_removal()
        .map_err(|error| format!("drift regression: {error}"))?;
    dry_run_has_no_mutation().map_err(|error| format!("dry-run regression: {error}"))?;
    protocol_failures_are_authoritative()
        .map_err(|error| format!("protocol failure regression: {error}"))?;
    local_process_and_configuration_failures_are_structured()
        .map_err(|error| format!("local failure regression: {error}"))?;
    guard_failures_are_current_and_structured()
        .map_err(|error| format!("Guard failure regression: {error}"))?;
    Ok(())
}

fn planning_schema_recovery_reaches_implementation() -> Result<(), Box<dyn Error>> {
    const SESSION: &str = "future.session.planning-recovery";
    const INVALID_TURN: &str = "future.turn.planning-recovery.invalid";
    const INVALID_TOOL_USE: &str = "future.tool-use.planning-recovery.invalid";
    const BASELINE: &str = "planning_recovery_baseline";
    const IMPLEMENTATION_PATH: &str = "implementation/bounded-preparation.md";

    let fixture = OperationalFixture::planning_product("planning-recovery")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&init, 0, "init", "action_required")?;
    let connection_id = fixture.connection_id();
    let project_id = fixture.project_id();
    let snapshot = fixture.registry_snapshot();
    let manifest = snapshot
        .guard_installations
        .iter()
        .find(|installation| installation.project_id == project_id)
        .map(|installation| guard_manifest_from_json(&installation.manifest_json))
        .transpose()?
        .ok_or("planning recovery Guard Installation")?;
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        "future.session.planning-recovery-observation",
        &manifest,
    )?;

    let prompt = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-recovery.intake",
        "prompt": "Prepare development from the planning documents and stop after the bounded work is ready to implement."
    });
    assert!(!prompt["prompt"]
        .as_str()
        .expect("planning recovery prompt")
        .to_ascii_lowercase()
        .contains("volicord"));
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &prompt,
        )?
        .status
        .success());
    let repository_before = fixture.repository_snapshot()?;

    let mut command = fixture.managed_mcp_command(&connection_id)?;
    let mut child = LiveMcpChild::spawn(&mut command)?;
    child.write(&json_lines(&[
        initialize_request(FUTURE_VERSION),
        initialized_notification(),
        tools_list_request(),
    ])?)?;
    let startup = child.read_responses(2)?;
    let checkpoint_tool = startup[1]["result"]["tools"]
        .as_array()
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool["name"] == AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name())
        })
        .ok_or("record-shaping tool schema")?;
    let checkpoint_schema = &checkpoint_tool["inputSchema"];
    let checkpoint_properties = checkpoint_schema["properties"]
        .as_object()
        .ok_or("record-shaping input properties")?;
    assert!(checkpoint_properties["checkpoint_operation"].is_object());
    assert!(checkpoint_schema["required"]
        .as_array()
        .is_some_and(|required| required.iter().any(|field| field == "checkpoint_operation")));
    let schema_enums = json_values_for_key(checkpoint_schema, "enum")
        .into_iter()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    for discriminator in [
        "create_initial",
        "replace_current",
        "implementation_boundary_missing",
        "user_product_decision_required",
        "user_technical_decision_required",
        "user_scope_decision_required",
        "repository_file",
    ] {
        assert!(
            schema_enums.contains(discriminator),
            "runtime checkpoint schema must advertise `{discriminator}`"
        );
    }
    assert!(schema_accepts_json_null(
        &checkpoint_properties["baseline_ref"]
    ));
    let checkpoint_summary = checkpoint_schema["description"]
        .as_str()
        .ok_or("record-shaping semantic summary")?;
    for semantic_guidance in [
        "/checkpoint_operation/operation",
        "Creates the first shaping checkpoint",
        "Replaces the exact current shaping checkpoint",
    ] {
        assert!(
            checkpoint_summary.contains(semantic_guidance),
            "runtime semantic summary must explain `{semantic_guidance}`: {checkpoint_summary}"
        );
    }
    let variant_meanings = json_values_for_key(checkpoint_schema, "x-volicord-variant-meaning")
        .into_iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(variant_meanings.contains("Identifies an exact repository-file source."));
    let mut call_id = 100;

    let projects = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::LIST_PROJECTS,
        json!({}),
        SESSION,
        "future.turn.planning-recovery.intake",
    )?;
    call_id += 1;
    assert!(projects["projects"]
        .as_array()
        .is_some_and(|projects| projects
            .iter()
            .any(|project| project["project_selector"] == project_id)));

    let empty_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": null, "detail": "full"}),
        SESSION,
        "future.turn.planning-recovery.intake",
    )?;
    call_id += 1;
    assert_eq!(method_result(&empty_status)["active_task"], Value::Null);
    let initial_state_version = method_result(&empty_status)["base"]["state_version"]
        .as_u64()
        .ok_or("empty planning state version")?;

    let intake = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::INTAKE,
        json!({
            "project_selector": project_id,
            "detail": "full",
            "plain_language_request": "Prepare the first bounded development step from the planning documents.",
            "requested_mode": "work",
            "requested_control_level": "auto",
            "resume_policy": "create_new",
            "acceptance_policy": "required",
            "lineage": null,
            "initial_scope": {
                "boundary": "Prepare one bounded implementation note from the current plans.",
                "non_goals": ["Implement unrelated capabilities."],
                "acceptance_criteria": [{
                    "statement": "The bounded preparation is ready for implementation.",
                    "evidence_requirement": "not_required"
                }]
            },
            "initial_context_refs": [],
            "initial_source_refs": []
        }),
        SESSION,
        "future.turn.planning-recovery.intake",
    )?;
    call_id += 1;
    let intake_result = method_result(&intake);
    assert_typed_mutation_state(
        intake_result,
        initial_state_version + 1,
        "work",
        Some("shaping"),
        "shaping_required",
    );
    let task_id = required_string(&intake_result["task_ref"], "record_id")?;
    let action_form_ref = required_transition_form_ref(&intake)?;
    let state_db = fixture.project_state_db_path();
    let before_invalid = rusqlite::Connection::open(&state_db)?;
    let invalid_counts = (
        before_invalid.query_row("SELECT state_version FROM project_state", [], |row| {
            row.get::<_, u64>(0)
        })?,
        table_count(&before_invalid, "shaping_checkpoints")?,
        table_count(&before_invalid, "user_action_requests")?,
        table_count(&before_invalid, "unrecorded_changes")?,
    );
    drop(before_invalid);

    let invalid_arguments = json!({
        "project_selector": project_id,
        "detail": "full",
        "task_id": task_id,
        "action_form_ref": action_form_ref,
        "checkpoint_operation": {"operation": "create"},
        "scope_revision": 0,
        "baseline_ref": null
    });
    let connection = agent_connection_record(&fixture.runtime_home, &connection_id)?
        .ok_or("planning recovery Agent Connection")?;
    let server = McpServerKey::parse(&connection.server_name)?;
    let checkpoint_callable = project_mcp_tool(&server, AgentToolId::RECORD_SHAPING_CHECKPOINT)?;
    let invalid_pre = json!({
        "hook_event_name": "PreToolUse",
        "session_id": SESSION,
        "turn_id": INVALID_TURN,
        "tool_use_id": INVALID_TOOL_USE,
        "tool_name": checkpoint_callable.callable_name().as_str(),
        "tool_input": invalid_arguments
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PreTool),
            &invalid_pre,
        )?
        .status
        .success());
    let invalid_response = live_mcp_raw_call(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        invalid_arguments,
        SESSION,
        INVALID_TURN,
    )?;
    call_id += 1;
    assert_eq!(invalid_response["result"]["isError"], true);
    let invalid = &invalid_response["result"]["structuredContent"];
    assert_eq!(invalid["reported_issue_count"], 1);
    assert_eq!(
        invalid["issues"][0]["path"],
        "/checkpoint_operation/operation"
    );
    assert_eq!(invalid["failure"]["reached_core"], false);
    assert_eq!(invalid["failure"]["checkpoint_recorded"], false);
    assert_eq!(invalid["failure"]["user_action_created"], false);
    assert_eq!(invalid["failure"]["product_repository_changed"], false);
    assert_eq!(invalid["failure"]["repair_required"], false);
    assert_eq!(
        invalid["retry_contract"]["fixed_arguments"]["checkpoint_operation"]["operation"],
        "create_initial"
    );
    assert_eq!(
        invalid["retry_contract"]["fixed_arguments"]["scope_revision"],
        0
    );
    assert_eq!(
        invalid["retry_contract"]["fixed_arguments"]["baseline_ref"],
        Value::Null
    );
    assert_eq!(
        invalid["retry_contract"]["action_form_ref"],
        action_form_ref
    );
    for forbidden_field in [
        "finalize_advice",
        "result_summary",
        "residual_risks",
        "recovery_constraints",
    ] {
        assert!(!json_key_exists(invalid, forbidden_field));
    }
    assert!(!invalid_response["result"]["content"][0]["text"]
        .as_str()
        .is_some_and(|text| text.to_ascii_lowercase().contains("corrupt")));
    let after_invalid = rusqlite::Connection::open(&state_db)?;
    assert_eq!(
        (
            after_invalid.query_row("SELECT state_version FROM project_state", [], |row| {
                row.get::<_, u64>(0)
            })?,
            table_count(&after_invalid, "shaping_checkpoints")?,
            table_count(&after_invalid, "user_action_requests")?,
            table_count(&after_invalid, "unrecorded_changes")?,
        ),
        invalid_counts
    );
    let observation_id: String = after_invalid.query_row(
        "SELECT repository_observation_id FROM repository_observations WHERE host_tool_use_id = ?1",
        [INVALID_TOOL_USE],
        |row| row.get(0),
    )?;
    drop(after_invalid);
    assert_eq!(fixture.repository_snapshot()?, repository_before);

    let continuation_prompt = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-recovery.corrected",
        "prompt": "Continue with the current retry contract."
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &continuation_prompt,
        )?
        .status
        .success());
    let terminal = repository_observation(&fixture.runtime_home, &project_id, &observation_id)?
        .ok_or("terminal malformed-call observation")?;
    assert_eq!(terminal.state, RepositoryObservationState::Unavailable);
    assert_eq!(
        terminal.unavailable_reason,
        Some(RepositoryObservationUnavailableReason::PostToolNotObserved)
    );
    assert!(terminal.delta.is_none());
    let after_terminalization = fixture.diagnostic_registry_snapshot()?;
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &continuation_prompt,
        )?
        .status
        .success());
    let replayed_diagnostics = fixture.diagnostic_registry_snapshot()?;
    assert_eq!(
        replayed_diagnostics.current_count, after_terminalization.current_count,
        "prompt replay must not duplicate a current finding"
    );
    assert_eq!(
        replayed_diagnostics.current_timestamps, after_terminalization.current_timestamps,
        "prompt replay must preserve current finding identity"
    );
    let replayed_terminal =
        repository_observation(&fixture.runtime_home, &project_id, &observation_id)?
            .ok_or("replayed malformed-call observation")?;
    assert_eq!(replayed_terminal, terminal);
    assert_eq!(
        table_count(
            &rusqlite::Connection::open(&state_db)?,
            "unrecorded_changes"
        )?,
        invalid_counts.3
    );

    let mismatch_response = live_mcp_raw_call(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        json!({
            "project_selector": project_id,
            "detail": "full",
            "task_id": task_id,
            "action_form_ref": action_form_ref,
            "checkpoint_operation": {"operation": "create_initial"},
            "scope_revision": 0,
            "baseline_ref": "0123456789012345678901234567890123456789",
            "summary": "This schema-valid attempt carries the wrong authority basis.",
            "implementation_boundary": "No Product Repository write is authorized.",
            "gaps": [],
            "source_refs": [],
            "evidence_refs": []
        }),
        SESSION,
        "future.turn.planning-recovery.corrected",
    )?;
    call_id += 1;
    assert_eq!(mismatch_response["result"]["isError"], true);
    let mismatch = &mismatch_response["result"]["structuredContent"];
    assert_eq!(mismatch["code"], "ACTION_FORM_ARGUMENT_MISMATCH");
    assert_eq!(
        mismatch["action_form_argument_mismatches"][0]["path"],
        "/baseline_ref"
    );
    assert_eq!(
        mismatch["action_form_argument_mismatches"][0]["expected_value"],
        Value::Null
    );
    assert!(mismatch["action_form_argument_mismatches"][0]["received_value"].is_string());
    assert_eq!(mismatch["failure"]["reached_core"], false);
    assert_eq!(
        mismatch["action_form_argument_mismatches"][0]["state_change_applied"],
        false
    );
    assert_eq!(mismatch["failure"]["current_baseline_valid"], true);
    assert_eq!(mismatch["failure"]["repair_required"], false);
    assert_eq!(
        mismatch["retry_contract"]["action_form_ref"],
        action_form_ref
    );
    for forbidden_field in [
        "finalize_advice",
        "result_summary",
        "residual_risks",
        "recovery_constraints",
    ] {
        assert!(!json_key_exists(mismatch, forbidden_field));
    }
    let after_mismatch = rusqlite::Connection::open(&state_db)?;
    assert_eq!(
        after_mismatch.query_row("SELECT state_version FROM project_state", [], |row| {
            row.get::<_, u64>(0)
        })?,
        invalid_counts.0
    );
    drop(after_mismatch);

    let action = |judgment_kind: &str, question: &str, options: Value| {
        json!({
            "action_type": "choice",
            "judgment_kind": judgment_kind,
            "presentation": "short",
            "question": question,
            "options": options,
            "context": {
                "summary": "The planning-only repository needs one bounded user-owned decision.",
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": ["This decision authorizes no Product Repository write."]
            },
            "affected_refs": [],
            "sensitive_action_scope": null
        })
    };
    let options = json!([{
        "option_id": "recommended",
        "label": "Use recommendation",
        "description": "Use the smallest bounded recommendation.",
        "consequence": "Only this exact decision is accepted.",
        "is_default": true
    }, {
        "option_id": "alternative",
        "label": "Use alternative",
        "description": "Use the documented alternative.",
        "consequence": "Only this exact alternative is accepted.",
        "is_default": false
    }]);
    let successful_response = live_mcp_raw_call(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        bound_action_arguments(
            &intake,
            AgentToolId::RECORD_SHAPING_CHECKPOINT,
            json!({
                "project_selector": project_id,
                "detail": "full",
                "summary": "The initial bounded proposal requires product, technical, and scope decisions.",
                "implementation_boundary": "Create only the bounded preparation note.",
                "gaps": [{
                    "gap_kind": "user_product_decision_required",
                    "summary": "Confirm the bounded product recommendation.",
                    "affected_refs": [],
                    "user_action": {"action": action("product_decision", "Use the bounded product recommendation?", options.clone()), "expires_at": null}
                }, {
                    "gap_kind": "user_technical_decision_required",
                    "summary": "Confirm the bounded technical recommendation.",
                    "affected_refs": [],
                    "user_action": {"action": action("technical_decision", "Use the bounded technical recommendation?", options), "expires_at": null}
                }, {
                    "gap_kind": "user_scope_decision_required",
                    "summary": "Confirm the exact scope boundary.",
                    "affected_refs": [],
                    "user_action": {"action": action("scope_decision", "Accept the bounded scope?", Value::Null), "expires_at": null}
                }],
                "source_refs": [{
                    "source_kind": "repository_file",
                    "source": {
                        "repository_path": "plans/product.md",
                        "baseline_commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "content_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                        "line_range": null
                    }
                }],
                "evidence_refs": []
            }),
        )?,
        SESSION,
        "future.turn.planning-recovery.corrected",
    )?;
    call_id += 1;
    assert_eq!(successful_response["result"]["isError"], false);
    let successful = &successful_response["result"]["structuredContent"];
    let successful_result = method_result(successful);
    assert_typed_mutation_state(
        successful_result,
        invalid_counts.0 + 1,
        "work",
        Some("shaping"),
        "awaiting_user_action",
    );
    assert_eq!(
        successful_result["shaping_checkpoint"]["readiness"],
        "blocked"
    );
    let request_refs = successful_result["created_user_action_request_refs"]
        .as_array()
        .ok_or("atomic shaping UserAction refs")?;
    assert_eq!(request_refs.len(), 3);
    assert!(successful_response["result"]["content"][0]["text"]
        .as_str()
        .is_some_and(|text| text.contains("committed Core authority")));
    assert!(successful["presentation"]["must_surface"]
        .as_array()
        .is_some_and(|facts| facts
            .iter()
            .any(|fact| { fact["fact_kind"] == "user_action_request_exists" })));
    for forbidden_field in [
        "finalize_advice",
        "result_summary",
        "residual_risks",
        "recovery_constraints",
    ] {
        assert!(!json_key_exists(successful, forbidden_field));
    }
    let state = rusqlite::Connection::open(&state_db)?;
    assert_eq!(
        state.query_row("SELECT COUNT(*) FROM shaping_checkpoints", [], |row| {
            row.get::<_, i64>(0)
        })?,
        1
    );
    assert_eq!(table_count(&state, "user_action_requests")?, 3);
    assert_eq!(table_count(&state, "change_units")?, 0);
    assert_eq!(table_count(&state, "write_tickets")?, 0);
    assert_eq!(table_count(&state, "unrecorded_changes")?, 0);
    drop(state);
    assert_eq!(fixture.repository_snapshot()?, repository_before);

    let awaiting_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-recovery.corrected",
    )?;
    call_id += 1;
    let awaiting_task = &method_result(&awaiting_status)["active_task"];
    assert_eq!(awaiting_task["work_phase"], "shaping");
    assert_eq!(awaiting_task["workflow"]["kind"], "awaiting_user_action");
    assert_eq!(
        awaiting_task["pending_user_action_summaries"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );

    let chat = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-recovery.chat",
        "prompt": "I accept all three recommendations."
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &chat,
        )?
        .status
        .success());
    assert_eq!(
        table_count(
            &rusqlite::Connection::open(&state_db)?,
            "user_action_resolutions"
        )?,
        0
    );

    let request_ids = request_refs
        .iter()
        .map(|reference| required_string(reference, "record_id"))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, request_id) in request_ids.iter().enumerate() {
        let choice = if index == 2 { "accept" } else { "recommended" };
        let resolved = fixture.run_inbox(&[
            "resolve",
            request_id,
            "--choice",
            choice,
            "--repo",
            fixture.repo_root.to_str().ok_or("UTF-8 repository path")?,
            "--json",
        ])?;
        assert_eq!(resolved.status.code(), Some(0));
        let _: Value = serde_json::from_slice(&resolved.stdout)?;
    }
    assert_eq!(fixture.repository_snapshot()?, repository_before);

    let decisions_ready = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-recovery.apply",
    )?;
    call_id += 1;
    assert_eq!(
        method_result(&decisions_ready)["active_task"]["workflow"]["kind"],
        "ready_to_apply_decisions"
    );
    let update_scope_form_ref = required_transition_form_ref(&decisions_ready)?;
    assert_ne!(update_scope_form_ref, action_form_ref);
    let scope_arguments = bound_action_arguments(
        &decisions_ready,
        AgentToolId::UPDATE_SCOPE,
        json!({
            "project_selector": project_id,
            "detail": "workflow",
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": "Create only the bounded preparation note.",
            "non_goals": ["Add unrelated product behavior."],
            "acceptance_criteria": null,
            "autonomy_boundary": "No other Product Repository path may change.",
            "baseline_ref": BASELINE,
            "change_unit": {
                "scope_summary": "Create the bounded preparation note.",
                "affected_paths": [IMPLEMENTATION_PATH]
            }
        }),
    )?;
    let expected_scope_resolution_refs = scope_arguments["related_scope_decision_refs"].clone();
    assert!(expected_scope_resolution_refs
        .as_array()
        .is_some_and(|refs| !refs.is_empty()));
    let before_scope_mismatch = rusqlite::Connection::open(&state_db)?;
    let before_scope_counts = (
        before_scope_mismatch.query_row("SELECT state_version FROM project_state", [], |row| {
            row.get::<_, u64>(0)
        })?,
        table_count(&before_scope_mismatch, "change_units")?,
        table_count(&before_scope_mismatch, "write_tickets")?,
    );
    drop(before_scope_mismatch);
    let mut tampered_scope_arguments = scope_arguments.clone();
    tampered_scope_arguments["related_scope_decision_refs"] = json!([]);
    let scope_mismatch_response = live_mcp_raw_call(
        &mut child,
        call_id,
        AgentToolId::UPDATE_SCOPE,
        tampered_scope_arguments,
        SESSION,
        "future.turn.planning-recovery.apply",
    )?;
    call_id += 1;
    assert_eq!(scope_mismatch_response["result"]["isError"], true);
    let scope_mismatch = &scope_mismatch_response["result"]["structuredContent"];
    assert_eq!(scope_mismatch["code"], "ACTION_FORM_ARGUMENT_MISMATCH");
    assert_eq!(scope_mismatch["reached_core"], false);
    assert_eq!(scope_mismatch["committed"], false);
    assert_eq!(
        scope_mismatch["action_form_argument_mismatches"][0]["path"],
        "/related_scope_decision_refs"
    );
    assert_eq!(
        scope_mismatch["action_form_argument_mismatches"][0]["expected_value"],
        expected_scope_resolution_refs
    );
    assert_eq!(
        scope_mismatch["action_form_argument_mismatches"][0]["received_value"],
        json!([])
    );
    let after_scope_mismatch = rusqlite::Connection::open(&state_db)?;
    assert_eq!(
        (
            after_scope_mismatch.query_row(
                "SELECT state_version FROM project_state",
                [],
                |row| row.get::<_, u64>(0),
            )?,
            table_count(&after_scope_mismatch, "change_units")?,
            table_count(&after_scope_mismatch, "write_tickets")?,
        ),
        before_scope_counts
    );
    drop(after_scope_mismatch);
    let scope = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::UPDATE_SCOPE,
        scope_arguments,
        SESSION,
        "future.turn.planning-recovery.apply",
    )?;
    call_id += 1;
    assert_eq!(scope["workflow"]["kind"], "ready_for_implementation");
    let advance_form_ref = required_transition_form_ref(&scope)?;
    assert_ne!(advance_form_ref, update_scope_form_ref);
    let ready_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-recovery.advance",
    )?;
    call_id += 1;
    assert_eq!(
        method_result(&ready_status)["active_task"]["workflow"]["kind"],
        "ready_for_implementation"
    );
    assert_eq!(
        required_transition_form_ref(&ready_status)?,
        advance_form_ref
    );
    let advance_arguments = bound_action_arguments(
        &ready_status,
        AgentToolId::ADVANCE_TASK,
        json!({
            "project_selector": project_id,
            "detail": "workflow"
        }),
    )?;
    let expected_advance_resolution_ids = advance_arguments["user_action_resolution_ids"].clone();
    assert!(expected_advance_resolution_ids
        .as_array()
        .is_some_and(|ids| ids.len() >= 2));
    let before_advance_counts = (
        method_result(&ready_status)["base"]["state_version"]
            .as_u64()
            .ok_or("ready state version")?,
        table_count(&rusqlite::Connection::open(&state_db)?, "write_tickets")?,
    );
    let mut tampered_advance_arguments = advance_arguments.clone();
    tampered_advance_arguments["user_action_resolution_ids"] = json!([]);
    let advance_mismatch_response = live_mcp_raw_call(
        &mut child,
        call_id,
        AgentToolId::ADVANCE_TASK,
        tampered_advance_arguments,
        SESSION,
        "future.turn.planning-recovery.advance",
    )?;
    call_id += 1;
    assert_eq!(advance_mismatch_response["result"]["isError"], true);
    let advance_mismatch = &advance_mismatch_response["result"]["structuredContent"];
    assert_eq!(advance_mismatch["code"], "ACTION_FORM_ARGUMENT_MISMATCH");
    assert_eq!(advance_mismatch["reached_core"], false);
    assert_eq!(advance_mismatch["committed"], false);
    assert_eq!(
        advance_mismatch["action_form_argument_mismatches"][0]["path"],
        "/user_action_resolution_ids"
    );
    assert_eq!(
        advance_mismatch["action_form_argument_mismatches"][0]["expected_value"],
        expected_advance_resolution_ids
    );
    assert_eq!(
        (
            rusqlite::Connection::open(&state_db)?.query_row(
                "SELECT state_version FROM project_state",
                [],
                |row| row.get::<_, u64>(0),
            )?,
            table_count(&rusqlite::Connection::open(&state_db)?, "write_tickets")?,
        ),
        before_advance_counts
    );
    let advanced = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::ADVANCE_TASK,
        advance_arguments,
        SESSION,
        "future.turn.planning-recovery.advance",
    )?;
    call_id += 1;
    assert_eq!(advanced["workflow"]["kind"], "implementation");
    assert_eq!(
        advanced["presentation"]["task_phase"]["work_phase"],
        "implementation"
    );
    let current = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-recovery.advance",
    )?;
    call_id += 1;
    assert_eq!(
        method_result(&current)["active_task"]["workflow"]["kind"],
        advanced["workflow"]["kind"]
    );
    assert_eq!(fixture.repository_snapshot()?, repository_before);
    assert_eq!(
        table_count(&rusqlite::Connection::open(&state_db)?, "write_tickets")?,
        0
    );

    let checkpoint_id: String = rusqlite::Connection::open(&state_db)?.query_row(
        "SELECT shaping_checkpoint_id FROM shaping_checkpoints
          WHERE project_id = ?1 AND task_id = ?2",
        (&project_id, &task_id),
        |row| row.get(0),
    )?;
    let corrupt_conn = rusqlite::Connection::open(&state_db)?;
    corrupt_conn.pragma_update(None, "ignore_check_constraints", true)?;
    corrupt_conn.execute(
        "UPDATE shaping_checkpoints SET baseline_ref = ?3
          WHERE project_id = ?1 AND shaping_checkpoint_id = ?2",
        (&project_id, &checkpoint_id, " baseline"),
    )?;
    corrupt_conn.pragma_update(None, "ignore_check_constraints", false)?;
    drop(corrupt_conn);
    let repository_before_corrupt_read = fixture.repository_snapshot()?;
    let corrupt_before = rusqlite::Connection::open(&state_db)?;
    let corrupt_counts = (
        table_count(&corrupt_before, "authority_events")?,
        table_count(&corrupt_before, "tool_invocations")?,
        table_count(&corrupt_before, "write_tickets")?,
        table_count(&corrupt_before, "runs")?,
        table_count(&corrupt_before, "unrecorded_changes")?,
        table_count(&corrupt_before, "repository_observations")?,
    );
    drop(corrupt_before);
    let corrupt_turn = "future.turn.planning-recovery.corrupt-store";
    let corrupt_tool_use = "future.tool-use.planning-recovery.corrupt-store";
    let corrupt_status_arguments =
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"});
    let status_callable = project_mcp_tool(&server, AgentToolId::STATUS)?;
    let corrupt_pre = json!({
        "hook_event_name": "PreToolUse",
        "session_id": SESSION,
        "turn_id": corrupt_turn,
        "tool_use_id": corrupt_tool_use,
        "tool_name": status_callable.callable_name().as_str(),
        "tool_input": corrupt_status_arguments
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PreTool),
            &corrupt_pre,
        )?
        .status
        .success());
    let corrupt_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        corrupt_status_arguments,
        SESSION,
        corrupt_turn,
    )?;
    call_id += 1;
    let corrupt_result = method_result(&corrupt_status);
    assert_eq!(corrupt_result["base"]["response_kind"], "rejected");
    assert_eq!(
        corrupt_result["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(corrupt_result["base"]["effect_kind"], "no_effect");
    let after_corrupt_call = rusqlite::Connection::open(&state_db)?;
    let corrupt_observation_id: String = after_corrupt_call.query_row(
        "SELECT repository_observation_id FROM repository_observations
          WHERE host_tool_use_id = ?1",
        [corrupt_tool_use],
        |row| row.get(0),
    )?;
    assert_eq!(
        (
            table_count(&after_corrupt_call, "authority_events")?,
            table_count(&after_corrupt_call, "tool_invocations")?,
            table_count(&after_corrupt_call, "write_tickets")?,
            table_count(&after_corrupt_call, "runs")?,
            table_count(&after_corrupt_call, "unrecorded_changes")?,
        ),
        (
            corrupt_counts.0,
            corrupt_counts.1,
            corrupt_counts.2,
            corrupt_counts.3,
            corrupt_counts.4,
        )
    );
    assert_eq!(
        table_count(&after_corrupt_call, "repository_observations")?,
        corrupt_counts.5 + 1
    );
    drop(after_corrupt_call);
    assert_eq!(
        fixture.repository_snapshot()?,
        repository_before_corrupt_read
    );

    let corrupt_continuation = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-recovery.corrupt-continuation",
        "prompt": "Report the stored-state corruption without retrying the mutation."
    });
    for _ in 0..2 {
        assert!(fixture
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
                &corrupt_continuation,
            )?
            .status
            .success());
    }
    let corrupt_observation =
        repository_observation(&fixture.runtime_home, &project_id, &corrupt_observation_id)?
            .ok_or("terminal corrupt-store observation")?;
    assert_eq!(
        corrupt_observation.state,
        RepositoryObservationState::Unavailable
    );
    assert_eq!(
        corrupt_observation.unavailable_reason,
        Some(RepositoryObservationUnavailableReason::PostToolNotObserved)
    );
    assert!(corrupt_observation.delta.is_none());
    assert_eq!(
        table_count(
            &rusqlite::Connection::open(&state_db)?,
            "repository_observations"
        )?,
        corrupt_counts.5 + 1
    );
    assert_eq!(
        table_count(
            &rusqlite::Connection::open(&state_db)?,
            "unrecorded_changes"
        )?,
        corrupt_counts.4
    );
    let projects_after_corruption = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::LIST_PROJECTS,
        json!({}),
        SESSION,
        "future.turn.planning-recovery.corrupt-continuation",
    )?;
    assert!(method_result(&projects_after_corruption)["projects"]
        .as_array()
        .is_some_and(|projects| !projects.is_empty()));
    let mut cli_status = fixture.base_command(env!("CARGO_BIN_EXE_volicord"), FUTURE_VERSION);
    let cli_status = cli_status
        .arg("status")
        .arg("--repo")
        .arg(&fixture.repo_root)
        .arg("--task")
        .arg(&task_id)
        .arg("--json")
        .output()?;
    assert!(cli_status.status.code().is_some());
    let cli_output = format!(
        "{}{}",
        String::from_utf8_lossy(&cli_status.stdout),
        String::from_utf8_lossy(&cli_status.stderr)
    );
    assert!(
        cli_output.contains("PERSISTED_DATA_CORRUPT"),
        "{cli_output}"
    );
    let cli_after_corruption = fixture
        .base_command(env!("CARGO_BIN_EXE_volicord"), FUTURE_VERSION)
        .arg("--version")
        .output()?;
    assert!(cli_after_corruption.status.success());

    let expected_tools = vec![
        AgentToolId::LIST_PROJECTS.wire_name(),
        AgentToolId::STATUS.wire_name(),
        AgentToolId::INTAKE.wire_name(),
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        AgentToolId::STATUS.wire_name(),
        AgentToolId::STATUS.wire_name(),
        AgentToolId::UPDATE_SCOPE.wire_name(),
        AgentToolId::UPDATE_SCOPE.wire_name(),
        AgentToolId::STATUS.wire_name(),
        AgentToolId::ADVANCE_TASK.wire_name(),
        AgentToolId::ADVANCE_TASK.wire_name(),
        AgentToolId::STATUS.wire_name(),
        AgentToolId::STATUS.wire_name(),
        AgentToolId::LIST_PROJECTS.wire_name(),
    ];
    assert_eq!(child.transcript().tool_names(), expected_tools);
    let checkpoint_calls = child
        .transcript()
        .calls()
        .iter()
        .filter(|call| {
            call.pointer("/params/name").and_then(Value::as_str)
                == Some(AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name())
        })
        .collect::<Vec<_>>();
    assert_eq!(checkpoint_calls.len(), 3);
    assert_eq!(
        checkpoint_calls[2].pointer("/params/arguments/baseline_ref"),
        Some(&Value::Null)
    );
    for call in child.transcript().calls() {
        let encoded = serde_json::to_string(call)?;
        for forbidden in [
            "--help",
            "strings",
            "source grep",
            "finalize_advice",
            "\"null\"",
        ] {
            assert!(
                !encoded.contains(forbidden),
                "forbidden recovery behavior: {encoded}"
            );
        }
    }
    let output = child.finish()?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    Ok(())
}

fn planning_product_explicit_shaping_journey() -> Result<(), Box<dyn Error>> {
    const SESSION: &str = "future.session.planning-product";
    const BASELINE: &str = "planning_product_baseline";
    const IMPLEMENTATION_PATH: &str = "implementation/release-preparation.md";

    let fixture = OperationalFixture::planning_product("planning-product")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&init, 0, "init", "action_required")?;
    let connection_id = fixture.connection_id();
    let project_id = fixture.project_id();
    let snapshot = fixture.registry_snapshot();
    let manifest = snapshot
        .guard_installations
        .iter()
        .find(|installation| installation.project_id == project_id)
        .map(|installation| guard_manifest_from_json(&installation.manifest_json))
        .transpose()?
        .ok_or("planning Product Repository Guard Installation")?;
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        "future.session.planning-product-observation",
        &manifest,
    )?;

    let prompt = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-product.intake",
        "prompt": "Prepare development from the planning documents and carry the bounded work through completion."
    });
    assert!(!prompt["prompt"]
        .as_str()
        .expect("planning prompt")
        .to_ascii_lowercase()
        .contains("volicord"));
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &prompt,
        )?
        .status
        .success());

    let before_analysis = fixture.repository_snapshot()?;
    for path in [
        "plans/product.md",
        "plans/experience.md",
        "plans/technical.md",
    ] {
        let text = fs::read_to_string(fixture.repo_root.join(path))?;
        assert!(
            text.starts_with('#'),
            "planning analysis must read Markdown"
        );
    }
    assert!(!before_analysis
        .keys()
        .any(|path| path.starts_with("src") || path == Path::new("Cargo.toml")));

    let mut command = fixture.managed_mcp_command(&connection_id)?;
    let mut child = LiveMcpChild::spawn(&mut command)?;
    child.write(&json_lines(&[
        initialize_request(FUTURE_VERSION),
        initialized_notification(),
        tools_list_request(),
    ])?)?;
    let startup = child.read_responses(2)?;
    assert!(startup[1]["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools
            .iter()
            .any(|tool| tool["name"] == "volicord.record_shaping_checkpoint")));
    assert!(startup[1]["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools
            .iter()
            .any(|tool| tool["name"] == "volicord.finalize_advice")));
    let mut call_id = 10;

    let projects = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::LIST_PROJECTS,
        json!({}),
        SESSION,
        "future.turn.planning-product.intake",
    )?;
    call_id += 1;
    assert!(projects["projects"]
        .as_array()
        .is_some_and(|projects| projects
            .iter()
            .any(|project| project["project_selector"] == project_id)));

    let empty_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": null, "detail": "full"}),
        SESSION,
        "future.turn.planning-product.intake",
    )?;
    call_id += 1;
    let empty_status_result = method_result(&empty_status);
    assert_eq!(empty_status_result["active_task"], Value::Null);
    let initial_state_version = empty_status_result["base"]["state_version"]
        .as_u64()
        .ok_or("empty status state_version")?;

    let intake = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::INTAKE,
        json!({
            "project_selector": project_id,
            "detail": "full",
            "plain_language_request": "Prepare and implement the first bounded release described by the planning documents.",
            "requested_mode": "work",
            "requested_control_level": "auto",
            "resume_policy": "create_new",
            "acceptance_policy": "required",
            "lineage": null,
            "initial_scope": {
                "boundary": "Prepare one bounded implementation note from the current planning documents.",
                "non_goals": ["Implement unrelated product capabilities."],
                "acceptance_criteria": [{
                    "statement": "The bounded release preparation is recorded and reviewed.",
                    "evidence_requirement": "not_required"
                }]
            },
            "initial_context_refs": [],
            "initial_source_refs": []
        }),
        SESSION,
        "future.turn.planning-product.intake",
    )?;
    call_id += 1;
    let intake_result = method_result(&intake);
    assert_typed_mutation_state(
        intake_result,
        initial_state_version + 1,
        "work",
        Some("shaping"),
        "shaping_required",
    );
    let task_id = required_string(&intake_result["task_ref"], "record_id")?;
    assert!(intake_result["change_unit_ref"].is_null());
    assert_eq!(
        intake["presentation"]["task_phase"]["work_phase"],
        "shaping"
    );
    let initial_checkpoint_action_form_ref = required_transition_form_ref(&intake)?;

    let action = |kind: &str, question: &str, options: Value, produced_at_state_version: u64| {
        json!({
            "action_type": "choice",
            "judgment_kind": kind,
            "presentation": "short",
            "question": question,
            "options": options,
            "context": {
                "summary": "The planning-only repository needs one bounded user-owned decision.",
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": ["This decision authorizes no Product Repository write."]
            },
            "affected_refs": [{
                "record_kind": "task",
                "record_id": task_id,
                "project_id": project_id,
                "task_id": task_id,
                "produced_at_state_version": produced_at_state_version
            }],
            "sensitive_action_scope": null
        })
    };
    let recommendations = json!([
        {
            "option_id": "recommended",
            "label": "Use recommendation",
            "description": "Use the smallest bounded release recommendation.",
            "consequence": "The selected recommendation is applied during scope update.",
            "is_default": true
        },
        {
            "option_id": "alternative",
            "label": "Use alternative",
            "description": "Use the documented alternative.",
            "consequence": "The alternative is applied during scope update.",
            "is_default": false
        }
    ]);
    let shaping = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        bound_action_arguments(
            &intake,
            AgentToolId::RECORD_SHAPING_CHECKPOINT,
            json!({
                "project_selector": project_id,
                "detail": "full",
                "summary": "The initial proposal requires one user-owned scope decision before the plan can proceed.",
                "implementation_boundary": "Create only the release-preparation note at the bounded path.",
                "gaps": [{
                    "gap_kind": "user_scope_decision_required",
                    "summary": "Confirm the initial scope proposal.",
                    "affected_refs": [],
                    "user_action": {"action": action("scope_decision", "Accept the initial scope proposal?", Value::Null, initial_state_version + 1), "expires_at": null}
                }],
                "source_refs": [],
                "evidence_refs": []
            }),
        )?,
        SESSION,
        "future.turn.planning-product.shaping",
    )?;
    call_id += 1;
    let shaping_result = method_result(&shaping);
    assert_typed_mutation_state(
        shaping_result,
        initial_state_version + 2,
        "work",
        Some("shaping"),
        "awaiting_user_action",
    );
    assert_eq!(shaping_result["shaping_checkpoint"]["readiness"], "blocked");
    let retired_checkpoint_id = required_string(
        &shaping_result["shaping_checkpoint"],
        "shaping_checkpoint_id",
    )?;
    let retired_request_refs = shaping_result["created_user_action_request_refs"]
        .as_array()
        .ok_or("shaping UserAction request refs")?;
    assert_eq!(retired_request_refs.len(), 1);
    let retired_request_id = required_string(&retired_request_refs[0], "record_id")?;
    assert_eq!(shaping["presentation"]["next_actor"], "user");
    assert_eq!(
        shaping["presentation"]["required_user_action"]["chat_reply_is_resolution"],
        false
    );
    assert_eq!(
        shaping["presentation"]["required_user_action"]["list_command"],
        format!("volicord inbox --task {task_id} --json")
    );
    assert_eq!(fixture.repository_snapshot()?, before_analysis);

    let pending_replacement = live_mcp_error(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        json!({
            "project_selector": project_id,
            "detail": "full",
            "task_id": task_id,
                "action_form_ref": initial_checkpoint_action_form_ref,
                "checkpoint_operation": {
                    "operation": "replace_current",
                    "expected_current_checkpoint_id": retired_checkpoint_id,
                    "retired_non_authorizing_request_refs": [],
                    "stale_authority_actions": [],
                    "carry_forward_application_refs": []
                },
                "scope_revision": 0,
                "baseline_ref": null,
                "summary": "A replacement cannot detach pending user authority.",
                "implementation_boundary": "Retain every live user-owned decision.",
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
        }),
        SESSION,
        "future.turn.planning-product.pending-replacement",
    )?;
    call_id += 1;
    assert_eq!(pending_replacement["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(
        pending_replacement["workflow_admission"]["called_method"],
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name()
    );
    assert_eq!(pending_replacement["reached_core"], false);
    assert_eq!(pending_replacement["committed"], false);

    let early_write = live_mcp_error(
        &mut child,
        call_id,
        AgentToolId::PREPARE_WRITE,
        json!({
            "project_selector": project_id,
            "detail": "full",
            "task_id": task_id,
            "change_unit_id": null,
            "intended_operation": "Create the bounded release-preparation note.",
            "intended_paths": [IMPLEMENTATION_PATH],
            "product_file_write_intended": true,
            "sensitive_categories": [],
            "baseline_ref": BASELINE
        }),
        SESSION,
        "future.turn.planning-product.early-write",
    )?;
    call_id += 1;
    assert_eq!(early_write["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(
        early_write["workflow_admission"]["called_method"],
        AgentToolId::PREPARE_WRITE.wire_name()
    );
    assert_eq!(early_write["reached_core"], false);
    assert_eq!(early_write["committed"], false);
    assert_eq!(fixture.repository_snapshot()?, before_analysis);

    let state_db = fixture.project_state_db_path();
    let state = rusqlite::Connection::open(&state_db)?;
    assert_eq!(table_count(&state, "change_units")?, 0);
    assert_eq!(table_count(&state, "write_tickets")?, 0);
    assert_eq!(table_count(&state, "user_action_resolutions")?, 0);
    let before_chat_version: u64 =
        state.query_row("SELECT state_version FROM project_state", [], |row| {
            row.get(0)
        })?;
    drop(state);
    let chat_prompt = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-product.chat-acceptance",
        "prompt": "모든 추천안을 수락합니다."
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &chat_prompt,
        )?
        .status
        .success());
    let state = rusqlite::Connection::open(&state_db)?;
    assert_eq!(table_count(&state, "user_action_resolutions")?, 0);
    assert_eq!(
        state.query_row("SELECT state_version FROM project_state", [], |row| row
            .get::<_, u64>(0))?,
        before_chat_version
    );
    drop(state);

    let before_recovery_state = rusqlite::Connection::open(&state_db)?;
    let recovery_safety_counts = (
        table_count(&before_recovery_state, "repository_observations")?,
        table_count(&before_recovery_state, "unrecorded_changes")?,
        table_count(&before_recovery_state, "write_tickets")?,
    );
    drop(before_recovery_state);
    let rejected = fixture.run_inbox(&[
        "resolve",
        &retired_request_id,
        "--choice",
        "reject",
        "--repo",
        fixture.repo_root.to_str().ok_or("UTF-8 repository path")?,
        "--json",
    ])?;
    assert_eq!(rejected.status.code(), Some(0));
    let rejected: Value = serde_json::from_slice(&rejected.stdout)?;
    assert_typed_mutation_state(
        &rejected,
        initial_state_version + 3,
        "work",
        Some("shaping"),
        "decision_recovery_required",
    );
    assert_eq!(
        rejected["user_action_resolution"]["body"]["resolution_outcome"],
        "rejected"
    );
    assert_eq!(
        required_transition_method(&rejected["state"]["workflow"]),
        Some("volicord.record_shaping_checkpoint")
    );
    let recovery_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "full"}),
        SESSION,
        "future.turn.planning-product.recovery-status",
    )?;
    call_id += 1;
    let recovery_status_workflow = &method_result(&recovery_status)["active_task"]["workflow"];
    for field in [
        "kind",
        "next_actor",
        "transition_catalog",
        "blocking_reason",
    ] {
        assert_eq!(
            rejected["state"]["workflow"][field], recovery_status_workflow[field],
            "rejected result and subsequent status disagree on {field}"
        );
    }
    let recovery_checkpoint_action_form_ref = required_transition_form_ref(&recovery_status)?;

    let recovered_shaping = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        bound_action_arguments(
            &recovery_status,
            AgentToolId::RECORD_SHAPING_CHECKPOINT,
            json!({
                "project_selector": project_id,
                "detail": "full",
                    "summary": "The revised plan replaces the rejected proposal with three current user-owned decisions.",
                    "implementation_boundary": "Create only the release-preparation note at the bounded path.",
                    "gaps": [
                        {
                            "gap_kind": "user_product_decision_required",
                            "summary": "Confirm the revised product recommendation.",
                            "affected_refs": [],
                            "user_action": {"action": action("product_decision", "Use the revised product recommendation?", recommendations.clone(), initial_state_version + 3), "expires_at": null}
                        },
                        {
                            "gap_kind": "user_technical_decision_required",
                            "summary": "Confirm the technical recommendation.",
                            "affected_refs": [],
                            "user_action": {"action": action("technical_decision", "Use the recommended technical boundary?", recommendations, initial_state_version + 3), "expires_at": null}
                        },
                        {
                            "gap_kind": "user_scope_decision_required",
                            "summary": "Confirm the exact scope boundary.",
                            "affected_refs": [],
                            "user_action": {"action": action("scope_decision", "Accept the bounded scope?", Value::Null, initial_state_version + 3), "expires_at": null}
                        }
                    ],
                    "source_refs": [],
                    "evidence_refs": []
            }),
        )?,
        SESSION,
        "future.turn.planning-product.recover-shaping",
    )?;
    call_id += 1;
    let recovered_shaping_result = method_result(&recovered_shaping);
    assert_typed_mutation_state(
        recovered_shaping_result,
        initial_state_version + 4,
        "work",
        Some("shaping"),
        "awaiting_user_action",
    );
    let checkpoint_id = required_string(
        &recovered_shaping_result["shaping_checkpoint"],
        "shaping_checkpoint_id",
    )?;
    let request_refs = recovered_shaping_result["created_user_action_request_refs"]
        .as_array()
        .ok_or("recovered shaping UserAction request refs")?;
    assert_eq!(request_refs.len(), 3);
    assert!(request_refs
        .iter()
        .all(|reference| reference["record_id"] != retired_request_id));
    let recovered_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "full"}),
        SESSION,
        "future.turn.planning-product.recovered-status",
    )?;
    call_id += 1;
    let recovered_status_workflow = &method_result(&recovered_status)["active_task"]["workflow"];
    for field in [
        "kind",
        "next_actor",
        "transition_catalog",
        "blocking_reason",
    ] {
        assert_eq!(
            recovered_shaping_result["workflow"][field], recovered_status_workflow[field],
            "recovery mutation and subsequent status disagree on {field}"
        );
    }
    assert!(recovered_status_workflow["required_refs"]
        .as_array()
        .is_some_and(|refs| refs
            .iter()
            .all(|reference| reference["record_id"] != retired_request_id)));

    let state = rusqlite::Connection::open(&state_db)?;
    let retired_status: (String, i64) = state.query_row(
        "SELECT basis_status, EXISTS (
             SELECT 1 FROM user_action_resolutions AS resolution
              WHERE resolution.project_id = request.project_id
                AND resolution.user_action_request_id = request.user_action_request_id
         )
           FROM user_action_requests AS request
          WHERE user_action_request_id = ?1",
        [&retired_request_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(retired_status, ("superseded".to_owned(), 1));
    let predecessor: String = state.query_row(
        "SELECT predecessor_shaping_checkpoint_id FROM shaping_checkpoints WHERE shaping_checkpoint_id = ?1",
        [&checkpoint_id],
        |row| row.get(0),
    )?;
    assert_eq!(predecessor, retired_checkpoint_id);
    assert_eq!(
        (
            table_count(&state, "repository_observations")?,
            table_count(&state, "unrecorded_changes")?,
            table_count(&state, "write_tickets")?,
        ),
        recovery_safety_counts,
        "retirement and successor creation must not create repository authority effects"
    );
    let open_observations: u64 = state.query_row(
        "SELECT COUNT(*) FROM repository_observations WHERE state = 'open'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(open_observations, 0);
    drop(state);
    assert_eq!(fixture.repository_snapshot()?, before_analysis);

    let inbox = fixture.run_inbox(&["--task", &task_id, "--json"])?;
    assert_eq!(inbox.status.code(), Some(0));
    let inbox: Value = serde_json::from_slice(&inbox.stdout)?;
    assert_eq!(
        inbox["pending_user_action_inbox_items"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    let request_ids = request_refs
        .iter()
        .map(|reference| required_string(reference, "record_id"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut resolution_refs = Vec::new();
    for (index, request_id) in request_ids.iter().enumerate() {
        let choice = if index == 2 { "accept" } else { "recommended" };
        let resolved = fixture.run_inbox(&[
            "resolve",
            request_id,
            "--choice",
            choice,
            "--repo",
            fixture.repo_root.to_str().ok_or("UTF-8 repository path")?,
            "--json",
        ])?;
        assert_eq!(
            resolved.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&resolved.stderr)
        );
        let resolved: Value = serde_json::from_slice(&resolved.stdout)?;
        assert_typed_mutation_state(
            &resolved,
            initial_state_version + 5 + index as u64,
            "work",
            Some("shaping"),
            if index == 2 {
                "ready_to_apply_decisions"
            } else {
                "awaiting_user_action"
            },
        );
        resolution_refs.push(resolved["user_action_resolution_ref"].clone());
    }

    let decision_application_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-product.decision-application-status",
    )?;
    call_id += 1;
    let resolved_replacement = live_mcp_error(
        &mut child,
        call_id,
        AgentToolId::RECORD_SHAPING_CHECKPOINT,
        json!({
            "project_selector": project_id,
            "detail": "full",
            "task_id": task_id,
                "action_form_ref": recovery_checkpoint_action_form_ref,
                "checkpoint_operation": {
                    "operation": "replace_current",
                    "expected_current_checkpoint_id": checkpoint_id,
                    "retired_non_authorizing_request_refs": [],
                    "stale_authority_actions": [],
                    "carry_forward_application_refs": []
                },
                "scope_revision": 0,
                "baseline_ref": null,
                "summary": "A replacement cannot detach resolved unapplied user authority.",
                "implementation_boundary": "Apply each decision through its semantic owner.",
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
        }),
        SESSION,
        "future.turn.planning-product.resolved-replacement",
    )?;
    call_id += 1;
    assert_eq!(resolved_replacement["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(
        resolved_replacement["workflow_admission"]["required_method"],
        AgentToolId::UPDATE_SCOPE.wire_name()
    );
    assert_eq!(resolved_replacement["reached_core"], false);
    assert_eq!(resolved_replacement["committed"], false);

    let scope = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::UPDATE_SCOPE,
        bound_action_arguments(
            &decision_application_status,
            AgentToolId::UPDATE_SCOPE,
            json!({
                "project_selector": project_id,
                "detail": "workflow",
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": "Create only the bounded release-preparation note.",
                "non_goals": ["Add product runtime behavior."],
            "acceptance_criteria": null,
            "autonomy_boundary": "No additional Product Repository paths may change.",
            "baseline_ref": BASELINE,
            "change_unit": {
                    "scope_summary": "Create the bounded release-preparation note.",
                    "affected_paths": [IMPLEMENTATION_PATH]
                }
            }),
        )?,
        SESSION,
        "future.turn.planning-product.apply-decisions",
    )?;
    call_id += 1;
    assert_compact_mutation_state(
        &scope,
        initial_state_version + 8,
        "work",
        Some("shaping"),
        "ready_for_implementation",
    );
    let state = rusqlite::Connection::open(&state_db)?;
    let gap_statuses = state
        .prepare(
            "SELECT gap_kind, status FROM shaping_checkpoint_gaps
              WHERE shaping_checkpoint_id = ?1 ORDER BY gap_kind",
        )?
        .query_map([&checkpoint_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    assert!(gap_statuses
        .iter()
        .any(|(kind, status)| { kind == "user_scope_decision_required" && status == "applied" }));
    assert!(gap_statuses.iter().all(|(kind, status)| {
        if kind == "user_scope_decision_required" {
            status == "applied"
        } else {
            status == "accepted"
        }
    }));
    drop(state);
    assert_eq!(
        table_count(&rusqlite::Connection::open(&state_db)?, "write_tickets")?,
        0
    );

    let advanced = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::ADVANCE_TASK,
        bound_action_arguments(
            &scope,
            AgentToolId::ADVANCE_TASK,
            json!({
                "project_selector": project_id,
                "detail": "workflow"
            }),
        )?,
        SESSION,
        "future.turn.planning-product.advance",
    )?;
    call_id += 1;
    assert_compact_mutation_state(
        &advanced,
        initial_state_version + 9,
        "work",
        Some("implementation"),
        "implementation",
    );
    let state = rusqlite::Connection::open(&state_db)?;
    let unapplied_gap_count: u64 = state.query_row(
        "SELECT COUNT(*) FROM shaping_checkpoint_gaps
          WHERE shaping_checkpoint_id = ?1 AND status <> 'applied'",
        [&checkpoint_id],
        |row| row.get(0),
    )?;
    assert_eq!(unapplied_gap_count, 0);
    drop(state);
    assert_eq!(
        table_count(&rusqlite::Connection::open(&state_db)?, "write_tickets")?,
        0
    );
    for fact_kind in [
        "entered_implementation",
        "phase_transition_created_no_write_ticket",
        "product_repository_writes_require_prepare_write",
    ] {
        assert!(advanced["presentation"]["must_surface"]
            .as_array()
            .is_some_and(|facts| facts.iter().any(|fact| fact["fact_kind"] == fact_kind)));
    }

    let prepared = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::PREPARE_WRITE,
        bound_action_arguments(
            &advanced,
            AgentToolId::PREPARE_WRITE,
            json!({
                "project_selector": project_id,
                "detail": "workflow",
                "intended_operation": "Create the bounded release-preparation note.",
                "intended_paths": [IMPLEMENTATION_PATH],
                "product_file_write_intended": true,
                "sensitive_categories": []
            }),
        )?,
        SESSION,
        "future.turn.planning-product.prepare-write",
    )?;
    call_id += 1;
    let prepared_result = method_result(&prepared);
    assert_compact_mutation_state(
        &prepared,
        initial_state_version + 10,
        "work",
        Some("implementation"),
        "implementation",
    );
    assert_eq!(prepared_result["decision"], "allowed");
    let write_ticket_id = required_string(prepared_result, "write_ticket_id")?;
    assert_eq!(
        fixture.repository_snapshot()?,
        before_analysis,
        "workflow authority mutations must not edit the Product Repository"
    );

    let write_tool_use = "future.tool-use.planning-product-write";
    let write_input = json!({"file_path": fixture.repo_root.join(IMPLEMENTATION_PATH)});
    let write_pre = json!({
        "hook_event_name": "PreToolUse",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-product.write",
        "tool_use_id": write_tool_use,
        "tool_name": "Write",
        "tool_input": write_input
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PreTool),
            &write_pre
        )?
        .status
        .success());
    fs::create_dir_all(fixture.repo_root.join("implementation"))?;
    fs::write(
        fixture.repo_root.join(IMPLEMENTATION_PATH),
        "# Release preparation\n\nImplement only the approved bounded first release.\n",
    )?;
    let write_post = json!({
        "hook_event_name": "PostToolUse",
        "session_id": SESSION,
        "turn_id": "future.turn.planning-product.write",
        "tool_use_id": write_tool_use,
        "tool_name": "Write",
        "tool_input": write_input,
        "tool_response": {"success": true}
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PostTool),
            &write_post
        )?
        .status
        .success());
    let state = rusqlite::Connection::open(&state_db)?;
    let observation: (String, String) = state.query_row(
        "SELECT repository_observation_id, state FROM repository_observations WHERE host_tool_use_id = ?1 ORDER BY started_at DESC LIMIT 1",
        [write_tool_use],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(observation.1, "complete");
    let observation = repository_observation(&fixture.runtime_home, &project_id, &observation.0)?
        .ok_or("bounded write repository observation")?;
    assert_eq!(
        observation
            .terminal_result
            .as_ref()
            .and_then(|result| result.delta.as_ref())
            .map(|delta| delta.transition_count),
        Some(1)
    );
    assert!(observation
        .terminal_result
        .as_ref()
        .is_some_and(|result| result.unrecorded_changes.is_empty()));
    assert_eq!(table_count(&state, "unrecorded_changes")?, 0);
    drop(state);

    let recorded = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::RECORD_RUN,
        bound_action_arguments(
            &prepared,
            AgentToolId::RECORD_RUN,
            json!({
                "project_selector": project_id,
                "detail": "workflow",
                "run_id": null,
                "write_ticket_id": write_ticket_id,
                "performed_operation": "Create the bounded release-preparation note.",
                "summary": "The approved bounded release preparation was recorded.",
                "observed_changes": {
                    "changed_paths": [IMPLEMENTATION_PATH],
                    "product_file_write_observed": true,
                    "sensitive_categories": [],
                    "baseline_ref": BASELINE
                },
                "artifact_inputs": [],
                "evidence_updates": [],
                "evidence_observations": [],
                "close_assessment": {
                    "result_summary": "The bounded release preparation is complete and self-checked.",
                    "result_refs": [],
                    "residual_risks": [],
                    "sensitive_categories": [],
                    "recovery_constraints": []
                }
            }),
        )?,
        SESSION,
        "future.turn.planning-product.record-run",
    )?;
    call_id += 1;
    let recorded_result = method_result(&recorded);
    assert_compact_mutation_state(
        &recorded,
        initial_state_version + 11,
        "work",
        Some("implementation"),
        "implementation",
    );
    assert!(recorded_result["run_ref"].is_object());
    let run_id = required_string(&recorded_result["run_ref"], "record_id")?;
    let state = rusqlite::Connection::open(&state_db)?;
    let (run_kind, observed_changes_json): (String, String) = state.query_row(
        "SELECT kind, observed_changes_json FROM runs WHERE run_id = ?1",
        [&run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let observed_changes: Value = serde_json::from_str(&observed_changes_json)?;
    assert_eq!(
        observed_changes["changed_paths"],
        json!([IMPLEMENTATION_PATH])
    );
    let (ticket_status, consumed_by_run_id, attempt_scope_json): (String, String, String) = state
        .query_row(
        "SELECT status, consumed_by_run_id, attempt_scope_json
               FROM write_tickets WHERE write_ticket_id = ?1",
        [&write_ticket_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let attempt_scope: Value = serde_json::from_str(&attempt_scope_json)?;
    assert_eq!(ticket_status, "consumed");
    assert_eq!(consumed_by_run_id, run_id);
    assert_eq!(
        attempt_scope["intended_operation"],
        "Create the bounded release-preparation note."
    );
    assert_eq!(run_kind, "implementation");
    drop(state);
    let close_review = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::CHECK_CLOSE,
        bound_action_arguments(
            &recorded,
            AgentToolId::CHECK_CLOSE,
            json!({
                "project_selector": project_id
            }),
        )?,
        SESSION,
        "future.turn.planning-product.close-review",
    )?;
    call_id += 1;
    let close_review_result = method_result(&close_review);
    assert_eq!(close_review_result["close_state"], "blocked");
    assert!(close_review_result["blockers"]
        .as_array()
        .is_some_and(|blockers| blockers
            .iter()
            .any(|blocker| blocker["code"] == "missing_final_acceptance")));
    let close_review_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-product.close-review-status",
    )?;
    call_id += 1;
    let final_action = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::REQUEST_USER_ACTION,
        bound_action_arguments(
            &close_review_status,
            AgentToolId::REQUEST_USER_ACTION,
            json!({
                "project_selector": project_id,
                "detail": "workflow",
                "request": {
                    "action": action("final_acceptance", "Accept the completed bounded result?", Value::Null, initial_state_version + 11),
                    "required_for": ["close_complete"],
                    "expires_at": null
                }
            }),
        )?,
        SESSION,
        "future.turn.planning-product.final-acceptance",
    )?;
    call_id += 1;
    let final_action_result = method_result(&final_action);
    assert_compact_mutation_state(
        &final_action,
        initial_state_version + 12,
        "work",
        Some("implementation"),
        "implementation",
    );
    let final_request_id = required_string(
        &final_action_result["user_action_request_summary"],
        "user_action_request_id",
    )?;
    let final_resolution = fixture.run_inbox(&[
        "resolve",
        &final_request_id,
        "--choice",
        "accept",
        "--repo",
        fixture.repo_root.to_str().ok_or("UTF-8 repository path")?,
        "--json",
    ])?;
    assert_eq!(final_resolution.status.code(), Some(0));
    let final_resolution: Value = serde_json::from_slice(&final_resolution.stdout)?;
    assert_typed_mutation_state(
        &final_resolution,
        initial_state_version + 13,
        "work",
        Some("implementation"),
        "implementation",
    );

    let final_status = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::STATUS,
        json!({"project_selector": project_id, "task_id": task_id, "detail": "workflow"}),
        SESSION,
        "future.turn.planning-product.final-status",
    )?;
    call_id += 1;
    let ready = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::CHECK_CLOSE,
        bound_action_arguments(
            &final_status,
            AgentToolId::CHECK_CLOSE,
            json!({
                "project_selector": project_id
            }),
        )?,
        SESSION,
        "future.turn.planning-product.close",
    )?;
    call_id += 1;
    assert_eq!(method_result(&ready)["close_state"], "ready");
    let closed = live_mcp_call(
        &mut child,
        call_id,
        AgentToolId::CLOSE_TASK,
        bound_action_arguments(
            &final_status,
            AgentToolId::CLOSE_TASK,
            json!({
                "project_selector": project_id,
                "detail": "workflow",
                "intent": "complete",
                "close_reason": "completed_self_checked",
                "superseding_task_id": null,
                "user_note": "The bounded planning-product journey is complete."
            }),
        )?,
        SESSION,
        "future.turn.planning-product.close",
    )?;
    assert_compact_mutation_state(
        &closed,
        initial_state_version + 14,
        "work",
        Some("implementation"),
        "terminal",
    );
    assert_eq!(closed["authority_receipt"]["close_state"], "closed");
    assert_eq!(closed["authority_receipt"]["close_blockers"], json!([]));
    let audit = read_authority_bundle_snapshot(&fixture.runtime_home, &fixture.repo_root)?;
    assert!(audit.records.iter().any(|record| {
        record.table == "user_action_requests"
            && record.row["user_action_request_id"] == retired_request_id
            && record.row["basis_status"] == "superseded"
    }));
    assert!(audit.records.iter().any(|record| {
        record.table == "user_action_resolutions"
            && record.row["user_action_request_id"] == retired_request_id
    }));
    assert!(audit.records.iter().any(|record| {
        record.table == "shaping_checkpoints"
            && record.row["shaping_checkpoint_id"] == checkpoint_id
            && record.row["predecessor_shaping_checkpoint_id"] == retired_checkpoint_id
    }));
    let historical_applications = audit
        .records
        .iter()
        .filter(|record| record.table == "shaping_decision_applications")
        .collect::<Vec<_>>();
    assert_eq!(historical_applications.len(), 3);
    assert!(historical_applications
        .iter()
        .all(|record| record.row["authority_status"] == "superseded"));
    let output = child.finish()?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    Ok(())
}

fn connection_list_evaluates_multiple_memberships_independently() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::initialized("operational-list-memberships")?;
    let second_repo = fixture
        ._temporary_root
        .root_path()
        .join("second-product-repository");
    fs::create_dir_all(&second_repo)?;
    let second_git = IsolatedGitRepository::initialize_at(&second_repo)?;
    second_git.write("README.md", b"# Second Product Repository\n")?;
    second_git.commit_all("second repository baseline")?;

    let add = fixture.run_connection_add_for_repo(FUTURE_VERSION, &second_repo)?;
    let add_report = assert_connection_report(&add, 0, "add", "action_required")?;
    assert_eq!(
        add_report["connection"]["repository"],
        second_repo.to_string_lossy().as_ref()
    );

    let snapshot = fixture.registry_snapshot();
    assert_eq!(snapshot.agent_connections.len(), 1);
    assert_eq!(snapshot.connection_projects.len(), 2);
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let first_project = snapshot
        .projects
        .iter()
        .find(|project| project.repo_root == fixture.repo_root)
        .ok_or("first Product Repository")?;
    let second_project = snapshot
        .projects
        .iter()
        .find(|project| project.repo_root == second_repo)
        .ok_or("second Product Repository")?;
    let first_manifest = snapshot
        .guard_installations
        .iter()
        .find(|installation| installation.project_id == first_project.project_id)
        .map(|installation| guard_manifest_from_json(&installation.manifest_json))
        .transpose()?
        .ok_or("first Guard Installation")?;

    let host_session_id = format!("{}.list-memberships", first_project.project_id);
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &first_project.project_id,
        FUTURE_VERSION,
        &host_session_id,
        &first_manifest,
    )?;

    let before_list = fixture.content_snapshot()?;
    let output = fixture.run_connection_list_all(FUTURE_VERSION, true, false)?;
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let memberships = report["connections"][0]["memberships"]
        .as_array()
        .ok_or("Connection memberships")?;
    assert_eq!(memberships.len(), 2);
    let first = memberships
        .iter()
        .find(|membership| membership["project_id"] == first_project.project_id)
        .ok_or("first current membership")?;
    let second = memberships
        .iter()
        .find(|membership| membership["project_id"] == second_project.project_id)
        .ok_or("second current membership")?;
    assert_eq!(first["current_state"]["state"], "available");
    assert_eq!(first["current_state"]["status"], "complete");
    assert_eq!(first["current_state"]["activation"], "complete");
    assert_eq!(second["current_state"]["state"], "available");
    assert_eq!(second["current_state"]["status"], "action_required");
    assert_ne!(
        second["current_state"]["activation"], "complete",
        "the unobserved membership must retain its current activation work"
    );
    for membership in memberships {
        assert_eq!(
            membership["current_state"]["evaluated_at"],
            report["generated_at"]
        );
    }

    let filtered =
        fixture.run_connection_list_for_repo(FUTURE_VERSION, true, false, &second_repo)?;
    let filtered: Value = serde_json::from_slice(&filtered.stdout)?;
    assert_eq!(
        filtered["connections"][0]["memberships"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        filtered["connections"][0]["memberships"][0]["project_id"],
        second_project.project_id
    );
    assert_eq!(fixture.content_snapshot()?, before_list);
    Ok(())
}

fn verification_tool_designation_mismatch_is_typed() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::initialized("operational-verification-tool-mismatch")?;
    let connection_id = fixture.connection_id();
    fixture.run_successful_managed_mcp(
        &connection_id,
        &fixture.project_id(),
        FUTURE_VERSION,
        "future.session.verification.mismatch",
    )?;
    let runtime = latest_current_managed_runtime_session(&fixture.runtime_home, &connection_id)?
        .ok_or("managed runtime for verification-tool mismatch")?;
    let registry = rusqlite::Connection::open(fixture.runtime_home.join("registry.sqlite"))?;
    assert_eq!(
        registry.execute(
            "UPDATE mcp_runtime_sessions SET verification_tool_name = ?2 WHERE runtime_session_id = ?1",
            [&runtime.runtime_session_id, AgentToolId::STATUS.wire_name()],
        )?,
        1
    );
    drop(registry);

    let output = fixture.run_connection("verify", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&output, 1, "verify", "failed")?;
    let check = report["checks"]
        .as_array()
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["id"] == "managed_capability_proof")
        })
        .ok_or("mismatched managed_capability_proof check")?;
    assert_eq!(check["status"], "failed");
    assert_eq!(check["code"], "tool_round_trip_designation_mismatch");
    assert_eq!(
        check["details"]["verification_tool"]["expected_tool_identity"],
        managed_host_round_trip_tool().wire_name()
    );
    assert_eq!(
        check["details"]["verification_tool"]["observed_tool_identity"],
        AgentToolId::STATUS.wire_name()
    );
    let finding = report["findings"]
        .as_array()
        .and_then(|findings| {
            findings
                .iter()
                .find(|finding| finding["code"] == "mcp.tool_verification.designation_mismatch")
        })
        .ok_or("typed verification-tool mismatch finding")?;
    assert_eq!(
        finding["facts"]["data"]["expected_tool_name"],
        managed_host_round_trip_tool().wire_name()
    );
    assert_eq!(
        finding["facts"]["data"]["observed_tool_name"],
        AgentToolId::STATUS.wire_name()
    );

    let verbose = fixture.run_connection_verbose("status", FUTURE_VERSION)?;
    assert_eq!(verbose.status.code(), Some(1));
    assert!(verbose.stderr.is_empty());
    let verbose = String::from_utf8(verbose.stdout)?;
    assert!(verbose.contains("Expected verification tool: volicord.list_projects"));
    assert!(verbose.contains("Observed verification tool: volicord.status"));
    Ok(())
}

fn status_tool_self_observation_preserves_missing_probe_reason() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::initialized("operational-status-self-observation")?;
    let connection_id = fixture.connection_id();
    let project_id = fixture.project_id();
    let snapshot = fixture.registry_snapshot();
    let manifest = guard_manifest_from_json(&snapshot.guard_installations[0].manifest_json)?;
    let connection = agent_connection_record(&fixture.runtime_home, &connection_id)?
        .ok_or("managed Guard Connection should exist")?;
    let status_callable = project_mcp_tool(
        &McpServerKey::parse(&connection.server_name)?,
        AgentToolId::GET_INTEGRATION_VERIFICATION,
    )?;

    let mut command = fixture.managed_mcp_command(&connection_id)?;
    let mut child = LiveMcpChild::spawn(&mut command)?;
    child.write(&json_lines(&[
        initialize_request(FUTURE_VERSION),
        initialized_notification(),
        tools_list_request(),
    ])?)?;
    child.read_responses(2)?;

    let prompt = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "future.session.status-self-observation",
        "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
        "prompt": "Verify missing Guard probe hooks."
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &prompt,
        )?
        .status
        .success());

    child.write(&json_lines(&[
        managed_tool_call_in_turn(
            3,
            managed_host_round_trip_tool().wire_name(),
            json!({}),
            "future.session.status-self-observation",
            INTEGRATION_VERIFICATION_TURN_ID,
        ),
        managed_tool_call_in_turn(
            4,
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            json!({"project_selector": project_id}),
            "future.session.status-self-observation",
            INTEGRATION_VERIFICATION_TURN_ID,
        ),
    ])?)?;
    let begin_responses = child.read_responses(2)?;
    let begin = adapter_tool_response(&begin_responses[1])?;
    let verification_id = begin["verification_id"]
        .as_str()
        .ok_or("begin response verification ID")?
        .to_owned();

    child.write(&json_lines(&[managed_tool_call_in_turn(
        5,
        AgentToolId::GUARD_PROBE.wire_name(),
        json!({"verification_id": verification_id}),
        "future.session.status-self-observation",
        INTEGRATION_VERIFICATION_TURN_ID,
    )])?)?;
    let probe_responses = child.read_responses(1)?;
    let probe = adapter_tool_response(&probe_responses[0])?;
    assert_eq!(
        probe["workflow"]["kind"],
        IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND
    );

    let status_tool_use_id = "future.tool-use.integration-verification-status";
    let status_pre = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "future.session.status-self-observation",
        "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
        "tool_use_id": status_tool_use_id,
        "tool_name": status_callable.callable_name().as_str(),
        "tool_input": {"verification_id": verification_id},
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PreTool),
            &status_pre,
        )?
        .status
        .success());
    let registry_path = fixture.runtime_home.join("registry.sqlite");
    let registry = rusqlite::Connection::open(&registry_path)?;
    let before_status: (String, i64) = registry.query_row(
        "SELECT status, status_read_count
           FROM guard_integration_verification_runs
          WHERE verification_id = ?1",
        [&verification_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(before_status, ("awaiting_observation".to_owned(), 0));
    drop(registry);

    child.write(&json_lines(&[managed_tool_call_in_turn(
        6,
        AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        json!({"verification_id": verification_id}),
        "future.session.status-self-observation",
        INTEGRATION_VERIFICATION_TURN_ID,
    )])?)?;
    let status_responses = child.read_responses(1)?;
    let verification = adapter_tool_response(&status_responses[0])?;
    assert_eq!(
        verification["workflow"]["kind"],
        IntegrationVerificationWorkflowState::REPAIR_REQUIRED_KIND
    );
    assert_eq!(
        verification["workflow"]["reason"],
        GuardVerificationRepairReason::HookEventNotObserved.as_str()
    );

    let status_post = json!({
        "hook_event_name": "PostToolUse",
        "session_id": "future.session.status-self-observation",
        "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
        "tool_use_id": status_tool_use_id,
        "tool_name": status_callable.callable_name().as_str(),
        "tool_input": {"verification_id": verification_id},
        "tool_response": {"success": true},
    });
    assert!(fixture
        .run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PostTool),
            &status_post,
        )?
        .status
        .success());

    let registry = rusqlite::Connection::open(&registry_path)?;
    let terminal: (String, i64, String) = registry.query_row(
        "SELECT status, status_read_count, repair_reason
           FROM guard_integration_verification_runs
          WHERE verification_id = ?1",
        [&verification_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(
        terminal,
        (
            "repair_required".to_owned(),
            1,
            GuardVerificationRepairReason::HookEventNotObserved
                .as_str()
                .to_owned(),
        )
    );
    let unrelated_count: i64 = registry.query_row(
        "SELECT COUNT(*)
           FROM guard_probe_observations
          WHERE verification_id = ?1 AND stage = ?2",
        rusqlite::params![
            verification_id,
            GuardProbeObservationStage::UnrelatedRoutedTool.as_str()
        ],
        |row| row.get(0),
    )?;
    let mismatch_count: i64 = registry.query_row(
        "SELECT COUNT(*)
           FROM guard_probe_observations
          WHERE verification_id = ?1
            AND stage IN ('callable_identity_unknown', 'callable_identity_mismatch')",
        [&verification_id],
        |row| row.get(0),
    )?;
    assert_eq!(unrelated_count, 1);
    assert_eq!(mismatch_count, 0);
    drop(registry);

    let output = child.finish()?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let status = fixture.run_connection("status", FUTURE_VERSION, true)?;
    assert!(status.stderr.is_empty());
    let report: Value = serde_json::from_slice(&status.stdout)?;
    let finding = report["findings"]
        .as_array()
        .and_then(|findings| {
            findings
                .iter()
                .find(|finding| finding["code"] == "guard.probe.hook_event_not_observed")
        })
        .ok_or("missing hook-event-not-observed Connection finding")?;
    let finding_id = finding["id"]
        .as_str()
        .ok_or("hook-event-not-observed finding ID")?;
    assert!(report["root_cause_ids"]
        .as_array()
        .is_some_and(|roots| roots.iter().any(|root| root == finding_id)));
    let report_text = serde_json::to_string(&report)?;
    assert!(!report_text.contains("guard.probe.callable_mismatch"));
    assert!(!report_text.contains("callable_identity_mismatch"));

    for rendered in [
        fixture.run_connection("status", FUTURE_VERSION, false)?,
        fixture.run_connection_verbose("status", FUTURE_VERSION)?,
    ] {
        assert!(rendered.stderr.is_empty());
        let rendered = String::from_utf8(rendered.stdout)?;
        assert!(rendered.contains("guard.probe.hook_event_not_observed"));
        assert!(!rendered.contains("guard.probe.callable_mismatch"));
        assert!(!rendered.contains("callable_identity_mismatch"));
    }
    Ok(())
}

fn codex_2025_06_18_compatibility_records_managed_runtime_facts() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-codex-2025-06-18")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    let init_report = assert_connection_report(&init, 0, "init", "action_required")?;
    assert_check(&init_report, "mcp_server", "passed", None);
    let mcp_details = init_report["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|check| check["id"] == "mcp_server"))
        .and_then(|check| check["details"].as_object())
        .ok_or("MCP server check should expose structured details")?;
    assert_eq!(
        mcp_details["preflight"]["evidence"]["writeability"],
        json!({
            "status": "not_checked",
            "requires": "connection_verify"
        })
    );
    assert_eq!(
        mcp_details["preflight"]["evidence"]["side_effects"],
        json!([])
    );
    assert!(mcp_details.get("self_test").is_none());
    assert!(mcp_details["preflight"].get("storage").is_none());
    let active = &mcp_details["last_active_verification"];
    assert_eq!(
        active["protocol_conformance"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|probe| probe["revision"].as_str())
            .collect::<Vec<_>>(),
        vec![
            "2024-10-07",
            "2024-11-05",
            "2025-03-26",
            "2025-06-18",
            "2025-11-25"
        ]
    );
    assert_eq!(
        active["protocol_conformance"][0]["safe_read_only_tool"],
        managed_host_round_trip_tool().wire_name()
    );
    assert!(active["protocol_conformance"]
        .as_array()
        .is_some_and(|probes| probes.len() == 5
            && probes.iter().all(|probe| {
                probe["status"] == "passed"
                    && probe["requested_revision"] == probe["negotiated_revision"]
                    && probe["schema_validation"] == true
                    && probe["safe_read_only_tool_completed"] == true
                    && probe["shutdown_completed"] == true
            })));
    assert_eq!(active["source"], "connection_verify");
    assert!(active["observed_at"].as_str().is_some());
    let codex_probe = &active["host_compatibility"][0];
    assert_eq!(codex_probe["profile"], "codex");
    assert_eq!(
        codex_probe["requested_revision"],
        CODEX_COMPATIBILITY_REVISION
    );
    assert_eq!(
        codex_probe["negotiated_revision"],
        CODEX_COMPATIBILITY_REVISION
    );
    assert_eq!(codex_probe["status"], "passed");

    let snapshot = fixture.registry_snapshot();
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let project_id = snapshot.projects[0].project_id.clone();
    fixture.assert_cli_verification_observations_are_isolated(&connection_id)?;

    let output = fixture.run_managed_mcp_messages(
        &connection_id,
        json_lines(&[
            codex_compatibility_initialize_request(),
            initialized_notification(),
            tools_list_request(),
            managed_tool_call(
                3,
                managed_host_round_trip_tool().wire_name(),
                json!({}),
                "codex.compatibility.session",
            ),
        ])?,
    )?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let responses = json_rpc_responses(&output.stdout)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        CODEX_COMPATIBILITY_REVISION
    );
    let actual_tools = responses[1]["result"]["tools"]
        .as_array()
        .ok_or("Codex tools/list should return an array")?
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .ok_or("Codex tool name should be a string")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_tools = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();
    assert_eq!(actual_tools, expected_tools);
    assert_eq!(responses[2]["result"]["isError"], false);
    assert!(adapter_tool_response(&responses[2])?["projects"]
        .as_array()
        .is_some_and(|projects| projects
            .iter()
            .any(|project| project["project_selector"] == project_id)));

    let session = latest_current_managed_runtime_session(&fixture.runtime_home, &connection_id)?
        .ok_or("Codex compatibility managed runtime session should be recorded")?;
    assert_eq!(session.session_source, McpRuntimeSessionSource::ManagedHost);
    assert_eq!(
        session.attempted_client_name.as_deref(),
        Some("codex-mcp-client")
    );
    assert_eq!(
        session.attempted_client_version.as_deref(),
        Some(CODEX_COMPATIBILITY_VERSION)
    );
    assert_eq!(
        session.requested_protocol_version.as_deref(),
        Some(CODEX_COMPATIBILITY_REVISION)
    );
    assert_eq!(
        session.selected_protocol_version.as_deref(),
        Some(CODEX_COMPATIBILITY_REVISION)
    );
    assert_eq!(
        session.negotiated_protocol_version.as_deref(),
        Some(CODEX_COMPATIBILITY_REVISION)
    );
    assert!(session.initialize_completed_at.is_some());
    assert!(session.initialized_notification_at.is_some());
    assert!(session.tools_list_observed_at.is_some());
    assert_eq!(session.required_tools_present, Some(true));
    assert_eq!(
        session.verification_tool_name.as_deref(),
        Some(managed_host_round_trip_tool().wire_name())
    );
    assert_eq!(
        session
            .verification_tool_name
            .as_deref()
            .map(AgentToolId::from_wire_name)
            .transpose()?,
        Some(managed_host_round_trip_tool())
    );
    assert!(session.verification_tool_observed_at.is_some());
    Ok(())
}

fn managed_launch_contracts_survive_filtered_environments() -> Result<(), Box<dyn Error>> {
    for (prefix, shared) in [
        ("operational-personal-managed-launch", false),
        ("operational-shared-managed-launch", true),
    ] {
        let fixture = OperationalFixture::with_scope(prefix, shared)?;
        let init = fixture.run_init(FUTURE_VERSION, None, false)?;
        let init_report = assert_connection_report(&init, 0, "init", "action_required")?;
        assert_check(&init_report, "mcp_server", "passed", None);
        assert_check(&init_report, "host_session", "pending", None);
        assert_check(&init_report, "required_tools", "pending", None);
        assert_check(&init_report, "tool_round_trip", "pending", None);

        let snapshot = fixture.registry_snapshot();
        let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
        let project_id = snapshot.projects[0].project_id.clone();
        fixture.assert_cli_verification_observations_are_isolated(&connection_id)?;

        let initialize_only = fixture.run_managed_mcp_messages(
            &connection_id,
            json_lines(&[initialize_request(FUTURE_VERSION)])?,
        )?;
        assert_eq!(initialize_only.status.code(), Some(0));
        assert!(initialize_only.stderr.is_empty());
        assert_eq!(json_rpc_responses(&initialize_only.stdout)?.len(), 1);
        let partial =
            latest_current_managed_runtime_session(&fixture.runtime_home, &connection_id)?
                .ok_or("managed initialize-only session should be recorded")?;
        assert!(partial.initialize_completed_at.is_some());
        assert!(partial.initialized_notification_at.is_none());
        assert!(partial.tools_list_observed_at.is_none());
        assert!(partial.required_tools_present.is_none());
        assert!(partial.verification_tool_name.is_none());
        assert!(partial.verification_tool_observed_at.is_none());

        let partial_status = fixture.run_connection("status", FUTURE_VERSION, true)?;
        let partial_report = assert_connection_report(&partial_status, 1, "status", "failed")?;
        assert_check(&partial_report, "host_session", "failed", None);
        assert_check(&partial_report, "tool_round_trip", "blocked", None);

        fixture.run_successful_managed_mcp(
            &connection_id,
            &project_id,
            FUTURE_VERSION,
            &format!(
                "acceptance.session.{}",
                if shared { "shared" } else { "personal" }
            ),
        )?;
        assert!(
            current_managed_runtime_sessions(&fixture.runtime_home, &connection_id)?
                .iter()
                .any(|session| {
                    session.initialize_completed_at.is_some()
                        && session.initialized_notification_at.is_some()
                        && session.tools_list_observed_at.is_some()
                        && session.required_tools_present == Some(true)
                        && session.verification_tool_name.as_deref()
                            == Some(managed_host_round_trip_tool().wire_name())
                        && session.verification_tool_observed_at.is_some()
                })
        );
    }
    Ok(())
}

fn connection_mode_transition_rebinds_guard_revision() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-connection-mode-transition")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&init, 0, "init", "action_required")?;

    let before_no_op = fixture.registry_snapshot();
    let no_op = fixture.run_connection_mode("workflow", FUTURE_VERSION, true)?;
    assert_eq!(no_op.status.code(), Some(0));
    assert!(no_op.stderr.is_empty());
    let no_op: Value = serde_json::from_slice(&no_op.stdout)?;
    assert_eq!(no_op["operation"], "mode");
    assert_eq!(no_op["status"], "complete");
    assert_eq!(
        no_op["operation_details"]["result"]["kind"],
        "mode_transition"
    );
    assert_eq!(no_op["operation_details"]["result"]["changed"], false);
    assert_eq!(no_op["activation_plan"]["required_steps"], json!([]));
    assert_eq!(
        no_op["operation_details"]["result"]["previous_integration_revision"],
        no_op["operation_details"]["result"]["current_integration_revision"]
    );
    let after_no_op = fixture.registry_snapshot();
    assert_eq!(
        after_no_op.agent_connections, before_no_op.agent_connections,
        "mode no-op changed the Connection row or verification report"
    );
    assert_eq!(
        after_no_op.guard_installations, before_no_op.guard_installations,
        "mode no-op changed a Guard manifest or timestamp"
    );

    let connection_id = before_no_op.agent_connections[0]
        .connection_internal_id
        .clone();
    let project_id = before_no_op.projects[0].project_id.clone();
    let workflow_manifest =
        guard_manifest_from_json(&before_no_op.guard_installations[0].manifest_json)?;
    let reused_native_session = "session.same";
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
        &workflow_manifest,
    )?;
    let workflow_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(workflow_manifest.guard_installation_id.as_str()),
        &host_session_correlation(reused_native_session),
    )?
    .session_id;
    assert_connection_report(
        &fixture.run_connection("verify", FUTURE_VERSION, true)?,
        0,
        "verify",
        "complete",
    )?;

    let repository_before = fixture.repository_snapshot()?;
    let read_only = fixture.run_connection_mode("read-only", FUTURE_VERSION, true)?;
    assert_eq!(
        read_only.status.code(),
        Some(0),
        "mode transition failed: stdout={} stderr={}",
        String::from_utf8_lossy(&read_only.stdout),
        String::from_utf8_lossy(&read_only.stderr)
    );
    assert!(read_only.stderr.is_empty());
    let read_only_report: Value = serde_json::from_slice(&read_only.stdout)?;
    assert_eq!(read_only_report["operation"], "mode");
    assert_eq!(read_only_report["status"], "action_required");
    assert_eq!(read_only_report["connection"]["mode"], "read_only");
    assert_eq!(
        read_only_report["operation_details"]["result"]["kind"],
        "mode_transition"
    );
    assert_eq!(
        read_only_report["operation_details"]["result"]["changed"],
        true
    );
    assert_ne!(
        read_only_report["operation_details"]["result"]["previous_integration_revision"],
        read_only_report["operation_details"]["result"]["current_integration_revision"]
    );
    assert_eq!(
        read_only_report["operation_details"]["result"]["rebound_guard_installation_ids"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        read_only_report["activation_plan"]["required_steps"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(fixture.repository_snapshot()?, repository_before);

    let read_only_snapshot = fixture.registry_snapshot();
    assert_eq!(read_only_snapshot.agent_connections[0].mode, "read_only");
    assert!(read_only_snapshot.agent_connections[0]
        .verification_report_json
        .is_none());
    let read_only_manifest =
        guard_manifest_from_json(&read_only_snapshot.guard_installations[0].manifest_json)?;
    assert_manifest_rebound_only(&workflow_manifest, &read_only_manifest);
    assert_eq!(
        read_only_manifest.integration_revision.as_str(),
        read_only_report["operation_details"]["result"]["current_integration_revision"]
            .as_str()
            .expect("current revision")
    );
    assert!(
        latest_current_managed_runtime_session(&fixture.runtime_home, &connection_id)?.is_none()
    );

    let generation_before_replay = read_only_snapshot.agent_connections[0].integration_generation;
    let revision_before_replay = connection_integration_revision(
        &fixture.agent_connection_record(&read_only_snapshot.agent_connections[0]),
    )?;
    let manifest_before_replay = read_only_snapshot.guard_installations[0]
        .manifest_json
        .clone();
    let repository_before_replay = fixture.repository_snapshot()?;
    let config_target = PathBuf::from(&read_only_snapshot.agent_connections[0].config_target);
    let config_before_replay = fs::read(&config_target)?;
    let replay = fixture.run_init(FUTURE_VERSION, None, false)?;
    let replay = assert_connection_report(&replay, 0, "init", "action_required")?;
    assert_eq!(replay["connection"]["mode"], "read_only");
    let after_replay = fixture.registry_snapshot();
    assert_eq!(after_replay.agent_connections[0].mode, "read_only");
    assert_eq!(
        after_replay.agent_connections[0].integration_generation,
        generation_before_replay
    );
    assert_eq!(
        connection_integration_revision(
            &fixture.agent_connection_record(&after_replay.agent_connections[0])
        )?,
        revision_before_replay
    );
    assert_eq!(
        after_replay.guard_installations[0].manifest_json,
        manifest_before_replay
    );
    assert_eq!(fixture.repository_snapshot()?, repository_before_replay);
    assert_eq!(fs::read(&config_target)?, config_before_replay);

    let registry_before_dry_run = fixture.registry_snapshot();
    let repository_before_dry_run = fixture.repository_snapshot()?;
    let config_before_dry_run = fs::read(&config_target)?;
    let dry_run = fixture.run_init(FUTURE_VERSION, None, true)?;
    let dry_run = assert_connection_report(&dry_run, 0, "init", "action_required")?;
    assert_eq!(dry_run["operation_details"]["dry_run"], true);
    assert_eq!(dry_run["connection"]["mode"], "read_only");
    assert_eq!(fixture.registry_snapshot(), registry_before_dry_run);
    assert_eq!(fixture.repository_snapshot()?, repository_before_dry_run);
    assert_eq!(fs::read(&config_target)?, config_before_dry_run);

    let pending = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let pending = assert_connection_report(&pending, 0, "status", "action_required")?;
    assert_check(&pending, "guard_observation", "pending", None);
    assert_check(&pending, "host_session", "pending", None);
    assert_check(&pending, "required_tools", "pending", None);
    assert_check(&pending, "tool_round_trip", "pending", None);

    let read_only_tools = fixture.run_managed_tools_list_names(&connection_id)?;
    assert!(read_only_tools.contains(&"volicord.list_projects".to_owned()));
    assert!(!read_only_tools.contains(&"volicord.intake".to_owned()));
    fixture.run_current_guard_phases(&read_only_manifest, reused_native_session)?;
    let read_only_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(read_only_manifest.guard_installation_id.as_str()),
        &host_session_correlation(reused_native_session),
    )?
    .session_id;
    assert_ne!(read_only_session_id, workflow_session_id);
    assert_unbound_agent_session(&fixture, &read_only_session_id)?;
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
        &read_only_manifest,
    )?;
    assert_connection_report(
        &fixture.run_connection("verify", FUTURE_VERSION, true)?,
        0,
        "verify",
        "complete",
    )?;

    let repository_before_workflow = fixture.repository_snapshot()?;
    let workflow = fixture.run_connection_mode("workflow", FUTURE_VERSION, true)?;
    assert_eq!(
        workflow.status.code(),
        Some(0),
        "mode transition failed: stdout={} stderr={}",
        String::from_utf8_lossy(&workflow.stdout),
        String::from_utf8_lossy(&workflow.stderr)
    );
    assert!(workflow.stderr.is_empty());
    let workflow_report: Value = serde_json::from_slice(&workflow.stdout)?;
    assert_eq!(workflow_report["status"], "action_required");
    assert_eq!(workflow_report["connection"]["mode"], "workflow");
    assert_eq!(fixture.repository_snapshot()?, repository_before_workflow);
    let workflow_snapshot = fixture.registry_snapshot();
    let current_workflow_manifest =
        guard_manifest_from_json(&workflow_snapshot.guard_installations[0].manifest_json)?;
    assert_manifest_rebound_only(&read_only_manifest, &current_workflow_manifest);
    assert!(
        latest_current_managed_runtime_session(&fixture.runtime_home, &connection_id)?.is_none()
    );
    let pending = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let pending = assert_connection_report(&pending, 0, "status", "action_required")?;
    assert_check(&pending, "guard_observation", "pending", None);

    let workflow_tools = fixture.run_managed_tools_list_names(&connection_id)?;
    assert!(workflow_tools.contains(&"volicord.intake".to_owned()));
    fixture.run_current_guard_phases(&current_workflow_manifest, reused_native_session)?;
    let current_workflow_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(current_workflow_manifest.guard_installation_id.as_str()),
        &host_session_correlation(reused_native_session),
    )?
    .session_id;
    assert_ne!(current_workflow_session_id, read_only_session_id);
    assert_ne!(current_workflow_session_id, workflow_session_id);
    assert_unbound_agent_session(&fixture, &current_workflow_session_id)?;
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
        &current_workflow_manifest,
    )?;
    assert_connection_report(
        &fixture.run_connection("verify", FUTURE_VERSION, true)?,
        0,
        "verify",
        "complete",
    )?;
    let project_state = rusqlite::Connection::open(fixture.project_state_db_path())?;
    let revision_scoped_rows: i64 = project_state.query_row(
        "SELECT COUNT(*) FROM host_sessions WHERE host_session_id = ?1",
        [reused_native_session],
        |row| row.get(0),
    )?;
    assert_eq!(revision_scoped_rows, 3);

    let removed = fixture.run_connection("remove", FUTURE_VERSION, true)?;
    assert_eq!(removed.status.code(), Some(0));
    assert!(removed.stderr.is_empty());
    let removed: Value = serde_json::from_slice(&removed.stdout)?;
    assert_eq!(
        removed["operation_details"]["result"]["connection_removed"],
        true
    );
    Ok(())
}

fn connection_mode_preflight_failure_preserves_connection() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-connection-mode-preflight-failure")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&init, 0, "init", "action_required")?;
    let before = fixture.registry_snapshot().agent_connections[0].clone();
    let registry = rusqlite::Connection::open(fixture.runtime_home.join("registry.sqlite"))?;
    registry.execute("DELETE FROM guard_installations", [])?;
    drop(registry);

    let failed = fixture.run_connection_mode("read-only", FUTURE_VERSION, true)?;
    assert_ne!(failed.status.code(), Some(0));
    assert!(failed.stdout.is_empty());
    let error = String::from_utf8(failed.stderr)?;
    assert!(error.contains("exactly one current Guard Installation"));
    assert!(error.contains("volicord init"));
    assert!(error.contains(&fixture.runtime_home.display().to_string()));
    assert!(error.contains(&fixture.repo_root.display().to_string()));
    assert!(error.contains("record"));
    assert!(!error.contains("'\\''"));
    assert!(!error.contains(&format!("'{}'", fixture.runtime_home.display())));
    let after = fixture.registry_snapshot().agent_connections[0].clone();
    assert_eq!(after, before);
    Ok(())
}

fn assert_manifest_rebound_only(before: &GuardManifest, after: &GuardManifest) {
    let mut expected = before.clone();
    expected.integration_revision = after.integration_revision.clone();
    assert_eq!(after, &expected);
    assert_ne!(before.integration_revision, after.integration_revision);
}

fn assert_unbound_agent_session(
    fixture: &OperationalFixture,
    session_id: &str,
) -> Result<(), Box<dyn Error>> {
    assert!(agent_session(&fixture.runtime_home, &fixture.project_id(), session_id)?.is_none());
    let project_state = rusqlite::Connection::open(fixture.project_state_db_path())?;
    let host_session_count: i64 = project_state.query_row(
        "SELECT COUNT(*) FROM host_sessions WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?;
    assert_eq!(host_session_count, 1);
    Ok(())
}

fn connection_removal_after_operational_observations() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-connection-removal")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&init, 0, "init", "action_required")?;
    let before = fixture.registry_snapshot();
    let connection_id = before.agent_connections[0].connection_internal_id.clone();
    let project_id = before.projects[0].project_id.clone();
    let config_target = PathBuf::from(&before.agent_connections[0].config_target);
    let manifest = guard_manifest_from_json(&before.guard_installations[0].manifest_json)?;
    let reused_native_session = "session.same";
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
        &manifest,
    )?;
    let historical_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(manifest.guard_installation_id.as_str()),
        &host_session_correlation(reused_native_session),
    )?
    .session_id;
    let repository_before = fixture.repository_snapshot()?;
    let project_state_path = fixture.project_state_db_path();
    let project_state = rusqlite::Connection::open(&project_state_path)?;
    let agent_sessions_before: i64 =
        project_state.query_row("SELECT COUNT(*) FROM host_sessions", [], |row| row.get(0))?;
    let guard_events_before: i64 =
        project_state.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?;
    assert!(agent_sessions_before > 0);
    assert!(guard_events_before > 0);
    drop(project_state);

    let output = fixture.run_connection("remove", FUTURE_VERSION, true)?;

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        report["operation_details"]["result"]["membership_removed"],
        true
    );
    assert_eq!(
        report["operation_details"]["result"]["connection_removed"],
        true
    );
    assert_eq!(
        report["operation_details"]["result"]["remaining_project_count"],
        0
    );
    let after = fixture.registry_snapshot();
    assert!(after.agent_connections.is_empty());
    assert!(after.connection_projects.is_empty());
    assert!(after.guard_installations.is_empty());
    let registry = rusqlite::Connection::open(&after.path)?;
    for table in [
        "mcp_runtime_project_session_bindings",
        "mcp_runtime_sessions",
    ] {
        let count: i64 = registry.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE connection_internal_id = ?1"),
            [&connection_id],
            |row| row.get(0),
        )?;
        assert_eq!(count, 0, "{table} retained removed Connection rows");
    }
    let project_state = rusqlite::Connection::open(project_state_path)?;
    let agent_sessions_after: i64 =
        project_state.query_row("SELECT COUNT(*) FROM host_sessions", [], |row| row.get(0))?;
    let guard_events_after: i64 =
        project_state.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?;
    assert_eq!(agent_sessions_after, agent_sessions_before);
    assert_eq!(guard_events_after, guard_events_before);
    assert_eq!(fixture.repository_snapshot()?, repository_before);
    assert!(!fs::read_to_string(config_target)
        .unwrap_or_default()
        .contains("mcp_servers.volicord"));

    let recreated = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&recreated, 0, "init", "action_required")?;
    let recreated_snapshot = fixture.registry_snapshot();
    let recreated_connection_id = recreated_snapshot.agent_connections[0]
        .connection_internal_id
        .clone();
    let recreated_manifest =
        guard_manifest_from_json(&recreated_snapshot.guard_installations[0].manifest_json)?;
    fixture.run_successful_managed_mcp_with_guard(
        &recreated_connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
        &recreated_manifest,
    )?;
    let recreated_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &recreated_connection_id,
        Some(recreated_manifest.guard_installation_id.as_str()),
        &host_session_correlation(reused_native_session),
    )?
    .session_id;
    assert_ne!(recreated_session_id, historical_session_id);
    assert!(agent_session(&fixture.runtime_home, &project_id, &historical_session_id,)?.is_some());
    let project_state = rusqlite::Connection::open(fixture.project_state_db_path())?;
    let recreated_rows: i64 = project_state.query_row(
        "SELECT COUNT(*) FROM host_sessions WHERE host_session_id = ?1",
        [reused_native_session],
        |row| row.get(0),
    )?;
    assert_eq!(recreated_rows, agent_sessions_before + 1);
    Ok(())
}

fn drift_verification_preserves_owned_configuration_and_removal() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-verify-configuration-drift")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    assert_connection_report(&init, 0, "init", "action_required")?;
    let initial = fixture.registry_snapshot();
    let initial_revision = connection_integration_revision(
        &fixture.agent_connection_record(&initial.agent_connections[0]),
    )?;
    let applied_mcp_dir = fixture._temporary_root.root_path().join("applied-mcp");
    fs::create_dir_all(&applied_mcp_dir)?;
    let applied_mcp_command = applied_mcp_dir.join(if cfg!(windows) {
        "volicord.exe"
    } else {
        "volicord"
    });
    fs::copy(env!("CARGO_BIN_EXE_volicord"), &applied_mcp_command)?;
    let repair = fixture.run_init(
        FUTURE_VERSION,
        Some((&applied_mcp_command, "normal")),
        false,
    )?;
    let repair = assert_connection_report(&repair, 0, "init", "action_required")?;
    assert_eq!(
        repair["operation_details"]["result"],
        json!({
            "kind": "setup",
            "disposition": "committed",
            "setup_lease": "acquired",
            "runtime_home_publication": "existing_ready"
        })
    );
    let initialized = fixture.registry_snapshot();
    assert_ne!(
        initialized.agent_connections[0].managed_fingerprint,
        initial.agent_connections[0].managed_fingerprint
    );
    assert_ne!(
        connection_integration_revision(
            &fixture.agent_connection_record(&initialized.agent_connections[0])
        )?,
        initial_revision
    );
    assert!(initialized.agent_connections[0]
        .verification_report_json
        .is_some());
    let connection_id = initialized.agent_connections[0]
        .connection_internal_id
        .clone();
    let project_id = initialized.projects[0].project_id.clone();
    let config_target = PathBuf::from(&initialized.agent_connections[0].config_target);
    let config_f_old = fs::read(&config_target)?;
    let fingerprint_f_old = initialized.agent_connections[0].managed_fingerprint.clone();
    let manifest = guard_manifest_from_json(&initialized.guard_installations[0].manifest_json)?;
    let native_session = "future.session.verify.drift";
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        native_session,
        &manifest,
    )?;
    let current_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(manifest.guard_installation_id.as_str()),
        &host_session_correlation(native_session),
    )?
    .session_id;
    let agent_session_before =
        agent_session(&fixture.runtime_home, &project_id, &current_session_id)?
            .expect("current Agent Session before drift verification");
    assert!(agent_session_matches_current_integration(
        &fixture.runtime_home,
        &agent_session_before,
        Some(manifest.guard_installation_id.as_str()),
    )?);

    let alternate_mcp_dir = fixture._temporary_root.root_path().join("desired-mcp");
    fs::create_dir_all(&alternate_mcp_dir)?;
    let alternate_mcp_command = alternate_mcp_dir.join(if cfg!(windows) {
        "volicord.exe"
    } else {
        "volicord"
    });
    fs::copy(env!("CARGO_BIN_EXE_volicord"), &alternate_mcp_command)?;
    let mut metadata: Value =
        serde_json::from_str(&initialized.agent_connections[0].metadata_json)?;
    metadata["mcp_command"] = Value::String(alternate_mcp_command.display().to_string());
    let metadata_json = serde_json::to_string(&metadata)?;
    rusqlite::Connection::open(&initialized.path)?.execute(
        "UPDATE agent_connections
            SET metadata_json = ?2
          WHERE connection_internal_id = ?1",
        (&connection_id, &metadata_json),
    )?;
    let before_verify = fixture.registry_snapshot();
    let revision_before_verify = connection_integration_revision(
        &fixture.agent_connection_record(&before_verify.agent_connections[0]),
    )?;
    assert_eq!(
        before_verify.agent_connections[0].managed_fingerprint,
        fingerprint_f_old
    );
    assert_eq!(fs::read(&config_target)?, config_f_old);

    let verification = fixture.run_connection("verify", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&verification, 1, "verify", "failed")?;
    assert_check(
        &report,
        "managed_config",
        "failed",
        Some("managed_config_mismatch"),
    );
    assert_check(&report, "ambient_hook_coverage", "blocked", None);
    let after_verify = fixture.registry_snapshot();
    assert_eq!(fs::read(&config_target)?, config_f_old);
    assert_eq!(
        after_verify.agent_connections[0].managed_fingerprint,
        fingerprint_f_old
    );
    assert_eq!(
        connection_integration_revision(
            &fixture.agent_connection_record(&after_verify.agent_connections[0])
        )?,
        revision_before_verify
    );
    assert!(after_verify.agent_connections[0]
        .verification_report_json
        .is_some());
    assert_eq!(
        after_verify.guard_installations[0].manifest_json,
        initialized.guard_installations[0].manifest_json
    );
    assert_eq!(
        guard_manifest_from_json(&after_verify.guard_installations[0].manifest_json)?
            .integration_revision,
        revision_before_verify
    );
    assert_eq!(
        latest_current_managed_runtime_session(&fixture.runtime_home, &connection_id)?
            .expect("verification must leave a current runtime revision")
            .connection_integration_revision,
        revision_before_verify.as_str()
    );
    let agent_session_after =
        agent_session(&fixture.runtime_home, &project_id, &current_session_id)?
            .expect("current Agent Session after drift verification");
    assert_eq!(agent_session_after, agent_session_before);
    assert!(agent_session_matches_current_integration(
        &fixture.runtime_home,
        &agent_session_after,
        Some(manifest.guard_installation_id.as_str()),
    )?);

    let removed = fixture.run_connection("remove", FUTURE_VERSION, true)?;
    assert_eq!(removed.status.code(), Some(0));
    assert!(removed.stderr.is_empty());
    let removed: Value = serde_json::from_slice(&removed.stdout)?;
    assert_eq!(
        removed["operation_details"]["result"]["membership_removed"],
        true
    );
    assert_eq!(
        removed["operation_details"]["result"]["connection_removed"],
        true
    );
    assert!(fixture.registry_snapshot().agent_connections.is_empty());
    assert!(!fs::read_to_string(config_target)
        .unwrap_or_default()
        .contains("mcp_servers.volicord"));
    Ok(())
}

fn complete_managed_activation_journey_and_read_only_status() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-host-complete")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    let init_report = assert_connection_report(&init, 0, "init", "action_required")?;
    assert_eq!(
        init_report["operation_details"]["result"],
        json!({
            "kind": "setup",
            "disposition": "committed",
            "setup_lease": "acquired",
            "runtime_home_publication": "published_by_this_invocation"
        })
    );
    assert_check(&init_report, "managed_config", "passed", None);
    assert_check(&init_report, "host_executable", "passed", None);
    assert_check(&init_report, "mcp_server", "passed", None);
    assert_check(&init_report, "host_session", "pending", None);
    assert_check(&init_report, "required_tools", "pending", None);
    assert_check(&init_report, "tool_round_trip", "pending", None);
    assert_check(&init_report, "guard_observation", "pending", None);
    assert_check(&init_report, "ambient_hook_coverage", "pending", None);
    assert_check(
        &init_report,
        "correlated_guard_verification",
        "pending",
        None,
    );
    let initial_correlated = init_report["checks"]
        .as_array()
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["id"] == "correlated_guard_verification")
        })
        .ok_or("initial correlated_guard_verification check")?;
    assert!(initial_correlated["details"]["latest_attempt"].is_null());
    assert!(initial_correlated.get("observed_at").is_none());
    assert_eq!(init_report["activation_state"], "host_reload_required");
    assert_eq!(
        init_report["hook_activation_state"],
        "review_required_by_setup"
    );
    let initial_steps = init_report["activation_plan"]["required_steps"]
        .as_array()
        .expect("initial activation steps");
    assert_eq!(initial_steps.len(), 4);
    assert_eq!(
        initial_steps
            .iter()
            .map(|step| step["id"].as_str().expect("activation step ID"))
            .collect::<Vec<_>>(),
        vec![
            "reload_codex",
            "review_project_hooks",
            "request_integration_verification",
            "read_connection_status",
        ]
    );
    assert_eq!(initial_steps[0]["prerequisites"], json!([]));
    assert_eq!(initial_steps[1]["prerequisites"], json!(["reload_codex"]));
    assert_eq!(
        initial_steps[2]["prerequisites"],
        json!(["review_project_hooks"])
    );
    assert_eq!(
        initial_steps[3]["prerequisites"],
        json!(["request_integration_verification"])
    );
    for (id, initiator, executor, channel) in [
        ("reload_codex", "user", "host", "codex_ui"),
        ("review_project_hooks", "user", "user", "codex_ui"),
        (
            "request_integration_verification",
            "user",
            "agent",
            "codex_chat",
        ),
        ("read_connection_status", "user", "volicord", "cli"),
    ] {
        let step = initial_steps
            .iter()
            .find(|step| step["id"] == id)
            .unwrap_or_else(|| panic!("missing initial activation step {id}: {init_report}"));
        assert_eq!(step["initiator"], initiator);
        assert_eq!(step["executor"], executor);
        assert_eq!(step["execution_channel"], channel);
    }
    let request = initial_steps
        .iter()
        .find(|step| step["id"] == "request_integration_verification")
        .expect("request integration verification step");
    assert_eq!(
        request["agent_sequence"]
            .as_array()
            .expect("nested agent sequence")
            .iter()
            .map(|step| step["tool"].as_str().expect("nested tool"))
            .collect::<Vec<_>>(),
        vec![
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        ]
    );
    assert!(!initial_steps.iter().any(|step| step["id"] == "guard_probe"));
    assert_eq!(
        init_report["activation_plan"]["optional_diagnostics"],
        json!([{
            "id": "run_optional_active_diagnostics",
            "initiator": "user",
            "executor": "volicord",
            "execution_channel": "cli",
            "prerequisites": [],
            "completes_checks": [
                "host_executable",
                "managed_config",
                "mcp_server",
                "process_startup",
                "required_tools",
                "tool_round_trip"
            ],
            "root_finding_ids": [],
            "instruction": "Run `volicord connection verify` only when optional active diagnostics are needed",
            "diagnostic_only": true,
            "agent_sequence": []
        }])
    );

    let snapshot = fixture.registry_snapshot();
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.agent_connections.len(), 1);
    assert_eq!(snapshot.connection_projects.len(), 1);
    assert_eq!(snapshot.guard_installations.len(), 1);
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let project_id = snapshot.projects[0].project_id.clone();
    let setup_report = fixture
        .agent_connection_record(&snapshot.agent_connections[0])
        .verification_report()?
        .ok_or("setup must persist active-verification evidence")?;
    assert_eq!(
        setup_report.status(),
        volicord_types::connection_verification::ConnectionStatus::ActionRequired
    );
    let setup_status = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let setup_status_report =
        assert_connection_report(&setup_status, 0, "status", "action_required")?;
    assert_eq!(
        setup_status_report["activation_state"],
        "host_reload_required"
    );
    let setup_list = fixture.run_connection_list(FUTURE_VERSION, true, false)?;
    let setup_list_report = assert_connection_list_membership(&setup_list, "action_required")?;
    assert_eq!(
        setup_list_report["connections"][0]["memberships"][0]["current_state"]["activation"],
        setup_status_report["activation_state"]
    );
    let manifest = guard_manifest_from_json(&snapshot.guard_installations[0].manifest_json)?;
    assert_current_guard_projection(&fixture, &manifest)?;
    fixture.run_successful_managed_mcp(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        "future.session.managed-only",
    )?;
    let managed_only = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let managed_only_report =
        assert_connection_report(&managed_only, 0, "status", "action_required")?;
    let managed_only_list = fixture.run_connection_list(FUTURE_VERSION, true, false)?;
    let managed_only_list_report =
        assert_connection_list_membership(&managed_only_list, "action_required")?;
    assert_eq!(
        managed_only_list_report["connections"][0]["memberships"][0]["current_state"]["activation"],
        managed_only_report["activation_state"]
    );
    for check_id in [
        "host_reload",
        "managed_session_health",
        "managed_capability_proof",
    ] {
        assert_check(&managed_only_report, check_id, "passed", None);
    }
    assert_check(
        &managed_only_report,
        "correlated_guard_verification",
        "pending",
        None,
    );
    assert_ne!(
        managed_only_report["checks"],
        serde_json::to_value(setup_report.checks())?,
        "current managed-session evidence must replace the earlier setup aggregate"
    );
    assert_eq!(
        fixture
            .agent_connection_record(&fixture.registry_snapshot().agent_connections[0])
            .verification_report()?
            .ok_or("persisted setup report after current status")?
            .status(),
        volicord_types::connection_verification::ConnectionStatus::ActionRequired,
        "read-only current status must not replace the persisted setup report"
    );
    let managed_guidance = manifest
        .managed_files
        .iter()
        .find(|file| file.artifact() == GuardManagedArtifact::AgentsManagedBlock)
        .ok_or("managed repository guidance expectation")?;
    let managed_guidance_path = managed_guidance.path();
    let managed_guidance_relative = managed_guidance_path.strip_prefix(&fixture.repo_root)?;
    let managed_guidance_after_setup = fs::read(managed_guidance_path)?;
    let managed_guidance_status_after_setup = fixture.git_output(
        &["status", "--porcelain", "--"],
        &[managed_guidance_relative.as_os_str()],
    )?;
    assert!(
        !managed_guidance_status_after_setup.trim().is_empty(),
        "setup must leave its tracked managed guidance update uncommitted"
    );
    fixture
        .git
        .write(TRANSFORMED_TRACKED_PATH, b"record=preexisting\r\n")?;
    let transformed_bytes_before = fixture.git.worktree_bytes(TRANSFORMED_TRACKED_PATH)?;
    let transformed_identity_before = fixture
        .git
        .canonical_worktree_blob_identity(TRANSFORMED_TRACKED_PATH)?;
    let transformed_status_before = fixture.git_output(
        &["status", "--porcelain", "--"],
        &[OsStr::new(TRANSFORMED_TRACKED_PATH)],
    )?;
    assert!(
        !transformed_status_before.trim().is_empty(),
        "the transformed tracked fixture must remain dirty before verification"
    );

    let abandoned = volicord_test_support::start_test_mcp_runtime_session(
        &fixture.runtime_home,
        McpRuntimeSessionStart {
            connection_internal_id: connection_id.clone(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: Some(FUTURE_VERSION.to_owned()),
            process_id: 4242,
            process_started_at: "2000-01-01T00:00:00Z".to_owned(),
        },
    )?;
    assert!(abandoned.terminal_finding_id.is_none());
    assert!(abandoned.graceful_close_at.is_none());

    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        NATIVE_SESSION_999,
        &manifest,
    )?;

    assert_eq!(
        fixture
            .agent_connection_record(&fixture.registry_snapshot().agent_connections[0])
            .verification_report()?
            .ok_or("stale persisted setup report")?
            .status(),
        volicord_types::connection_verification::ConnectionStatus::ActionRequired
    );
    let before_complete_list = fixture.content_snapshot()?;
    let complete_list = fixture.run_connection_list(FUTURE_VERSION, true, false)?;
    let complete_list_report = assert_connection_list_membership(&complete_list, "complete")?;
    let complete_list_state =
        &complete_list_report["connections"][0]["memberships"][0]["current_state"];
    assert_eq!(complete_list_state["activation"], "complete");
    assert_eq!(
        complete_list_state["hook_activation"],
        "effective_by_observation"
    );
    assert_eq!(complete_list_state["required_step_count"], 0);
    assert_eq!(complete_list_state["required_steps"], json!([]));
    assert_eq!(
        complete_list_state["evaluated_at"],
        complete_list_report["generated_at"]
    );
    assert_eq!(
        fixture.content_snapshot()?,
        before_complete_list,
        "connection list wrote state"
    );
    let human_list = fixture.run_connection_list(FUTURE_VERSION, false, false)?;
    assert_eq!(human_list.status.code(), Some(0));
    assert!(human_list.stderr.is_empty());
    let human_list = String::from_utf8(human_list.stdout)?;
    assert!(human_list.starts_with("Connections (1)\n\ncodex\n"));
    assert!(human_list.contains(&format!("Repository: {}", fixture.repo_root.display())));
    assert!(human_list.contains("Status: ready\n"));
    assert!(human_list.contains("Activation: complete\n"));
    assert!(!human_list.contains('\t'));

    let verbose_list = fixture.run_connection_list(FUTURE_VERSION, false, true)?;
    assert_eq!(verbose_list.status.code(), Some(0));
    assert!(verbose_list.stderr.is_empty());
    let verbose_list = String::from_utf8(verbose_list.stdout)?;
    assert!(verbose_list.contains(&format!("Connection ID: {connection_id}\n")));
    assert!(verbose_list.contains(&format!("Project ID: {project_id}\n")));
    assert!(verbose_list.contains("Configuration target: "));
    assert!(verbose_list.contains("Integration revision: "));
    assert!(verbose_list.contains("Evaluated at: "));
    assert!(verbose_list.contains("Not applicable checks: "));
    assert!(!verbose_list.contains('\t'));
    let complete = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let complete_report = assert_connection_report(&complete, 0, "status", "complete")?;
    assert_eq!(complete_report["activation_state"], "complete");
    assert_eq!(
        complete_report["hook_activation_state"],
        "effective_by_observation"
    );
    assert_eq!(complete_report["root_cause_ids"], json!([]));
    let complete_status_counts = complete_report["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .fold(BTreeMap::<&str, usize>::new(), |mut counts, check| {
            *counts
                .entry(check["status"].as_str().expect("check status"))
                .or_default() += 1;
            counts
        });
    assert_eq!(complete_status_counts.get("blocked"), None);
    assert_eq!(complete_status_counts.get("pending"), None);
    assert_eq!(complete_status_counts.get("failed"), None);
    for check in complete_report["checks"].as_array().expect("checks") {
        assert!(
            matches!(check["status"].as_str(), Some("passed" | "not_applicable")),
            "complete activation retained a nonterminal check: {check}"
        );
    }
    for check_id in [
        "guard_observation",
        "correlated_guard_verification",
        "host_session",
        "required_tools",
        "tool_round_trip",
    ] {
        assert_check(&complete_report, check_id, "passed", None);
    }
    let active_verification = complete_report["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|check| check["id"] == "mcp_server"))
        .and_then(|check| check.pointer("/details/last_active_verification"))
        .ok_or("last active verification evidence")?;
    let active_verification_observed_at = active_verification["observed_at"]
        .as_str()
        .ok_or("last active verification evidence timestamp")?
        .to_owned();
    assert_eq!(active_verification["source"], "connection_verify");
    let guard_verification = complete_report["checks"]
        .as_array()
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["id"] == "correlated_guard_verification")
        })
        .ok_or("correlated_guard_verification check")?;
    let guard_evidence_observed_at = guard_verification["observed_at"]
        .as_str()
        .ok_or("correlated Guard evidence timestamp")?
        .to_owned();
    let guard_evidence_details = guard_verification["details"].clone();
    let complete_generated_at = complete_report["generated_at"].clone();
    assert_ne!(
        guard_verification["observed_at"], complete_report["generated_at"],
        "proof evidence time must not be replaced by report evaluation time"
    );
    let latest_attempt = &guard_verification["details"]["latest_attempt"];
    assert!(latest_attempt["verification_id"].is_string());
    assert!(latest_attempt["runtime_session_id"].is_string());
    assert!(latest_attempt["host_session_id"].is_string());
    assert!(latest_attempt["host_turn_id"].is_string());
    assert!(latest_attempt["prompt_event_id"].is_string());
    assert!(latest_attempt["pre_tool_event_id"].is_string());
    assert!(latest_attempt["post_tool_event_id"].is_string());
    assert_eq!(latest_attempt["attempt_state"], "complete");
    let latest_proof = &guard_verification["details"]["latest_completed_proof"];
    assert_eq!(
        latest_proof["verification_id"],
        latest_attempt["verification_id"]
    );
    assert_eq!(
        latest_proof["runtime_session_id"],
        latest_attempt["runtime_session_id"]
    );
    assert_eq!(
        complete_report["connection"]["verification_ids"],
        json!([latest_attempt["verification_id"].clone()])
    );
    assert!(latest_attempt["observed_host_callable_identity"].is_string());
    assert_eq!(
        latest_proof["observed_host_callable_identity"],
        latest_attempt["observed_host_callable_identity"]
    );
    let round_trip = complete_report["checks"]
        .as_array()
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["id"] == "managed_capability_proof")
        })
        .ok_or("managed_capability_proof check")?;
    assert_eq!(
        round_trip["details"]["verification_tool"]["expected_tool_identity"],
        managed_host_round_trip_tool().wire_name()
    );
    assert_eq!(
        round_trip["details"]["verification_tool"]["observed_tool_identity"],
        managed_host_round_trip_tool().wire_name()
    );
    assert!(round_trip["details"]["verification_tool"]["observed_at"].is_string());
    assert_eq!(
        complete_report["activation_plan"]["required_steps"],
        json!([])
    );
    assert_eq!(
        complete_report["activation_plan"]["optional_diagnostics"],
        json!([])
    );
    let runtime_sessions = complete_report["connection"]["runtime_sessions"]
        .as_array()
        .expect("role-bearing runtime sessions");
    assert_eq!(runtime_sessions.len(), 1);
    assert_eq!(
        runtime_sessions[0]["roles"],
        json!([
            "latest_managed_attempt",
            "latest_managed_capability_proof",
            "guard_verification_attempt",
            "guard_verification_proof"
        ])
    );
    let complete_runtime_session_id = runtime_sessions[0]["id"]
        .as_str()
        .ok_or("complete runtime-session ID")?;
    let registry = rusqlite::Connection::open(fixture.runtime_home.join("registry.sqlite"))?;
    let persisted_guard_completed_at: String = registry.query_row(
        "SELECT completed_at
           FROM guard_integration_verification_runs
          WHERE verification_id = ?1",
        [latest_attempt["verification_id"]
            .as_str()
            .ok_or("complete verification ID")?],
        |row| row.get(0),
    )?;
    assert_eq!(guard_evidence_observed_at, persisted_guard_completed_at);
    assert_eq!(latest_proof["completed_at"], persisted_guard_completed_at);
    assert_ne!(
        complete_list_state["evaluated_at"], persisted_guard_completed_at,
        "list evaluated_at must remain the batch evaluation time"
    );
    let complete_session_source: String = registry.query_row(
        "SELECT session_source FROM mcp_runtime_sessions WHERE runtime_session_id = ?1",
        [complete_runtime_session_id],
        |row| row.get(0),
    )?;
    assert_eq!(complete_session_source, "managed_host");
    let non_managed_session_count: i64 = registry.query_row(
        "SELECT COUNT(*)
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
            AND session_source IN ('manual_cli', 'cli_preflight', 'integration_probe')",
        [&connection_id],
        |row| row.get(0),
    )?;
    assert_eq!(non_managed_session_count, 0);
    let passed_guard_verification_count: i64 = registry.query_row(
        "SELECT COUNT(*)
           FROM guard_integration_verification_runs
          WHERE connection_internal_id = ?1
            AND status = 'complete'",
        [&connection_id],
        |row| row.get(0),
    )?;
    assert_eq!(passed_guard_verification_count, 1);
    let capability_proof_count: i64 = registry.query_row(
        "SELECT COUNT(*)
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND required_tools_present = 1
            AND verification_tool_name = ?2
            AND verification_tool_observed_at IS NOT NULL",
        [&connection_id, managed_host_round_trip_tool().wire_name()],
        |row| row.get(0),
    )?;
    assert_eq!(capability_proof_count, 2);
    drop(registry);
    assert_canonical_connection_command_shape(&complete_report);

    let before_status = fixture.content_snapshot()?;
    let repeated = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let repeated_report = assert_connection_report(&repeated, 0, "status", "complete")?;
    let repeated_guard = repeated_report["checks"]
        .as_array()
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["id"] == "correlated_guard_verification")
        })
        .ok_or("repeated correlated_guard_verification check")?;
    let repeated_active_verification = repeated_report["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|check| check["id"] == "mcp_server"))
        .and_then(|check| check.pointer("/details/last_active_verification"))
        .ok_or("repeated last active verification evidence")?;
    assert_ne!(repeated_report["generated_at"], complete_generated_at);
    assert_eq!(
        repeated_active_verification["observed_at"], active_verification_observed_at,
        "read-only status changed the active-verification evidence time"
    );
    assert_eq!(repeated_active_verification["source"], "connection_verify");
    assert_eq!(
        repeated_guard["observed_at"], guard_evidence_observed_at,
        "read-only status changed the persisted evidence time"
    );
    assert_eq!(
        repeated_guard["details"], guard_evidence_details,
        "read-only status changed the persisted evidence details"
    );
    let human = fixture.run_connection("status", FUTURE_VERSION, false)?;
    assert_eq!(human.status.code(), Some(0));
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout)?;
    assert!(human.starts_with("Codex connection is ready.\n\n"));
    assert!(human.contains(&format!("Repository: {}\n", fixture.repo_root.display())));
    assert!(human.contains("Mode: workflow\n"));
    assert!(human.contains("Activation: complete\n"));
    assert!(human.contains("Hook activation: effective by observation\n"));
    assert!(human.contains("\nChecks\n"));
    assert!(human.contains("\n  Passed: "));
    assert!(human.contains("\n  Blocked: 0\n"));
    assert!(human.contains("\n  Pending: 0\n"));
    assert!(human.contains("\n  Failed: 0\n"));
    let active_snapshot = format!(
        "Last active verification: passed\n  Observed at: {active_verification_observed_at}\n  Source: connection verify\nLast verified storage writeability: passed"
    );
    assert!(human.contains(&active_snapshot));
    assert!(!human.contains(&project_id));
    for check in complete_report["checks"].as_array().expect("checks") {
        assert!(!human.contains(check["id"].as_str().expect("check id")));
    }

    let repeated_human = fixture.run_connection("status", FUTURE_VERSION, false)?;
    assert_eq!(repeated_human.status.code(), Some(0));
    assert!(repeated_human.stderr.is_empty());
    assert!(String::from_utf8(repeated_human.stdout)?.contains(&active_snapshot));

    let verbose = fixture.run_connection_verbose("status", FUTURE_VERSION)?;
    assert_eq!(verbose.status.code(), Some(0));
    assert!(verbose.stderr.is_empty());
    let verbose = String::from_utf8(verbose.stdout)?;
    assert!(verbose.starts_with("Codex connection is ready.\n\nConnection\n"));
    assert!(verbose.contains("\n\nSummary\n  Status: ready\n"));
    for check in complete_report["checks"].as_array().expect("checks") {
        assert!(verbose.contains(check["summary"].as_str().expect("check summary")));
    }
    assert!(verbose.contains("    Protocol conformance: 5/5 revisions passed"));
    for revision in [
        "2024-10-07",
        "2024-11-05",
        "2025-03-26",
        "2025-06-18",
        "2025-11-25",
    ] {
        assert!(verbose.contains(&format!("      {revision}: passed")));
    }
    assert!(verbose.contains("    Host compatibility: 1/1 profiles passed"));
    assert!(verbose.contains("      codex: passed (codex-mcp-turn-metadata, protocol 2025-06-18)"));
    assert!(!verbose.contains("        Initialize:"));
    assert!(!verbose.contains("        Required tools:"));
    assert!(verbose.contains("    Expected verification tool: volicord.list_projects"));
    assert!(verbose.contains("    Observed verification tool: volicord.list_projects"));
    assert!(verbose.contains("    Verification tool observed at:"));
    assert!(verbose.contains("    Last active verification: passed"));
    assert!(verbose.contains(&format!(
        "    Observed at: {active_verification_observed_at}"
    )));
    assert!(verbose.contains("    Source: connection verify"));
    assert!(verbose.contains("    Store writeability: passed (Registry and 1 project)"));
    assert!(verbose.contains("    Hook path safety: verified"));
    assert!(verbose.contains("    CWD independence: verified"));
    assert!(verbose.contains("    Subdirectory safety: verified"));
    assert!(verbose.contains("    Evidence: 6 current managed artifacts verified"));
    assert!(verbose.contains(&format!("    Evidence time: {guard_evidence_observed_at}")));
    let guard_block = verbose
        .split("  [passed] Correlated Guard verification\n")
        .nth(1)
        .ok_or("verbose correlated Guard check")?;
    let guard_block = guard_block
        .split("\n\n  [")
        .next()
        .ok_or("verbose correlated Guard check boundary")?;
    for expected in [
        "    Correlation\n",
        "      Verification ID: ",
        "      Runtime session: ",
        "      Acquisition stage: post tool matched\n",
        "    Attempt\n      State: complete\n",
        "    Completed proof\n      Completed at: ",
    ] {
        assert!(guard_block.contains(expected), "{guard_block}");
    }
    assert_eq!(guard_block.matches("Verification ID:").count(), 1);
    assert_eq!(guard_block.matches("Guard installation ID:").count(), 1);
    let after_status = fixture.content_snapshot()?;
    assert_eq!(after_status, before_status, "connection status wrote state");
    assert_eq!(guard_block.matches("Policy digest:").count(), 1);
    assert_eq!(guard_block.matches("Hook definition digest:").count(), 1);
    for (field, label) in [
        ("guard_installation_id", "Guard installation ID"),
        ("policy_digest", "Policy digest"),
        ("hook_definition_digest", "Hook definition digest"),
    ] {
        let value = latest_attempt[field]
            .as_str()
            .ok_or("complete Guard correlation value")?;
        assert!(
            guard_block.contains(&format!("      {label}: {value}\n")),
            "{guard_block}"
        );
    }
    assert!(!verbose.contains("Details: {"));
    assert!(!verbose.contains("\":["));
    assert_eq!(
        fs::read(managed_guidance_path)?,
        managed_guidance_after_setup,
        "managed verification changed the setup-created guidance update"
    );
    assert_eq!(
        fixture.git_output(
            &["status", "--porcelain", "--"],
            &[managed_guidance_relative.as_os_str()],
        )?,
        managed_guidance_status_after_setup,
        "managed verification changed the preexisting guidance worktree state"
    );
    assert_eq!(
        fixture.git.worktree_bytes(TRANSFORMED_TRACKED_PATH)?,
        transformed_bytes_before,
        "managed verification changed transformed worktree bytes"
    );
    assert_eq!(
        fixture
            .git
            .canonical_worktree_blob_identity(TRANSFORMED_TRACKED_PATH)?,
        transformed_identity_before,
        "managed verification changed transformed canonical content"
    );
    assert_eq!(
        fixture.git_output(
            &["status", "--porcelain", "--"],
            &[OsStr::new(TRANSFORMED_TRACKED_PATH)],
        )?,
        transformed_status_before,
        "managed verification inherited or changed the transformed dirty state"
    );
    assert_database_integrity(&fixture.runtime_home.join("registry.sqlite"))?;
    assert_database_integrity(&fixture.project_state_db_path())?;

    let changed_version = fixture.run_connection("verify", NEXT_FUTURE_VERSION, true)?;
    let changed_report = assert_connection_report(&changed_version, 0, "verify", "complete")?;
    for check_id in ["host_session", "required_tools", "tool_round_trip"] {
        assert_check(&changed_report, check_id, "passed", None);
    }
    let runtime_session_id = changed_report["checks"]
        .as_array()
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["id"] == "managed_session_health")
        })
        .and_then(|check| check.pointer("/details/runtime_session_id"))
        .and_then(Value::as_str)
        .ok_or("host-session runtime ID")?;
    let mismatch =
        diagnostic_occurrences_for_runtime_session(&fixture.runtime_home, runtime_session_id)?
            .into_iter()
            .map(|finding| finding.to_diagnostic_finding())
            .find(|finding| {
                finding.code().as_str() == "host.codex.peer_version_differs_from_path_probe"
            })
            .ok_or("peer/PATH mismatch finding")?;
    assert_eq!(
        mismatch.code().as_str(),
        "host.codex.peer_version_differs_from_path_probe"
    );
    assert_eq!(
        mismatch.severity(),
        volicord_types::diagnostics::DiagnosticSeverity::Warning
    );
    assert_eq!(
        mismatch.facts().data()["actual_mcp_peer_client_info"]["version"],
        FUTURE_VERSION
    );
    assert_eq!(
        mismatch.facts().data()["path_executable_probe"]["version"],
        NEXT_FUTURE_VERSION
    );

    fixture.run_successful_managed_mcp(
        &connection_id,
        &project_id,
        NEXT_FUTURE_VERSION,
        NATIVE_SESSION_1000,
    )?;
    fixture.run_current_guard_phases(&manifest, NATIVE_SESSION_1000)?;
    let current_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(manifest.guard_installation_id.as_str()),
        &host_session_correlation(NATIVE_SESSION_1000),
    )?
    .session_id;
    assert!(
        agent_session(&fixture.runtime_home, &project_id, &current_session_id,)?
            .is_some_and(|session| session.runtime_session_id.is_some())
    );
    let completed_again = fixture.run_connection("status", NEXT_FUTURE_VERSION, true)?;
    assert_connection_report(&completed_again, 0, "status", "complete")?;

    let wrapper = fixture.repo_root.join(".codex/hooks/volicord-pre-tool.sh");
    fs::write(&wrapper, "malformed current wrapper\n")?;
    let tampered = fixture.run_connection("status", NEXT_FUTURE_VERSION, true)?;
    let tampered_report = assert_connection_report(&tampered, 1, "status", "failed")?;
    assert_check(
        &tampered_report,
        "ambient_hook_coverage",
        "failed",
        Some("ambient_hook_coverage_failed"),
    );
    Ok(())
}

fn dry_run_has_no_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-host-dry-run")?;
    let repo_before = fixture.repository_snapshot()?;
    assert!(!fixture.runtime_home.exists());
    let output = fixture.run_init(FUTURE_VERSION, None, true)?;
    let report = assert_connection_report(&output, 0, "init", "action_required")?;
    assert_eq!(report["operation_details"]["dry_run"], true);
    assert_eq!(
        report["operation_details"]["result"],
        json!({
            "kind": "setup",
            "disposition": "planned",
            "setup_lease": "acquired",
            "runtime_home_publication": "not_published"
        })
    );
    assert!(report["operation_details"]["planned_changes"].is_array());
    assert!(!fixture.runtime_home.exists());
    assert_eq!(fixture.repository_snapshot()?, repo_before);
    Ok(())
}

fn protocol_failures_are_authoritative() -> Result<(), Box<dyn Error>> {
    let initialize = OperationalFixture::initialized("operational-initialize-failure")?;
    initialize.run_managed_mcp_messages(
        &initialize.connection_id(),
        json_lines(&[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": {"name": "future-client", "version": FUTURE_VERSION}
            }
        })])?,
    )?;
    initialize.assert_failed_status("host_session", "host_session_current_attempt_failed")?;

    let tools_list = OperationalFixture::initialized("operational-tools-list-failure")?;
    tools_list.run_managed_mcp_messages(
        &tools_list.connection_id(),
        json_lines(&[
            initialize_request(FUTURE_VERSION),
            initialized_notification(),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": []
            }),
        ])?,
    )?;
    tools_list.assert_failed_status("host_session", "host_session_current_attempt_failed")?;
    tools_list.assert_latest_runtime_finding("mcp.tools.protocol_error")?;

    let safe_call = OperationalFixture::initialized("operational-safe-call-failure")?;
    safe_call.run_safe_tool_storage_failure()?;
    safe_call.assert_failed_status("host_session", "host_session_current_attempt_failed")?;
    safe_call.assert_latest_runtime_finding("mcp.tool_call.safe_read_only_failed")?;

    let missing_tools = OperationalFixture::initialized("operational-missing-tools")?;
    let state_db = missing_tools.project_state_db_path();
    let displaced = state_db.with_extension("sqlite.displaced");
    fs::rename(&state_db, &displaced)?;
    let result = missing_tools.run_managed_mcp_messages(
        &missing_tools.connection_id(),
        json_lines(&[
            initialize_request(FUTURE_VERSION),
            initialized_notification(),
            tools_list_request(),
        ])?,
    );
    fs::rename(&displaced, &state_db)?;
    result?;
    missing_tools.assert_failed_status("host_session", "host_session_current_attempt_failed")?;
    Ok(())
}

fn local_process_and_configuration_failures_are_structured() -> Result<(), Box<dyn Error>> {
    let unavailable = OperationalFixture::initialized("operational-host-unavailable")?;
    let output =
        unavailable.run_connection_with_path("verify", FUTURE_VERSION, true, Path::new(""))?;
    let report = assert_connection_report(&output, 1, "verify", "failed")?;
    assert_check(
        &report,
        "host_executable",
        "failed",
        Some("host_executable_not_found"),
    );

    let malformed = OperationalFixture::initialized("operational-config-malformed")?;
    let snapshot = malformed.registry_snapshot();
    fs::write(
        &snapshot.agent_connections[0].config_target,
        "[mcp_servers.volicord\n",
    )?;
    let output = malformed.run_connection("status", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&output, 1, "status", "failed")?;
    assert_check(
        &report,
        "managed_config",
        "failed",
        Some("managed_config_malformed"),
    );

    let startup = OperationalFixture::new("operational-mcp-startup-failure")?;
    let fixture_executable = startup.install_mcp_fixture_executable()?;
    let output = startup.run_init(
        FUTURE_VERSION,
        Some((&fixture_executable, "startup_failure")),
        false,
    )?;
    let report = assert_connection_report(&output, 1, "init", "failed")?;
    assert_eq!(
        report["operation_details"]["result"],
        json!({
            "kind": "setup",
            "disposition": "committed",
            "setup_lease": "acquired",
            "runtime_home_publication": "published_by_this_invocation"
        })
    );
    assert_check(
        &report,
        "mcp_server",
        "failed",
        Some("mcp_server_preflight_failed"),
    );

    let early_exit = OperationalFixture::new("operational-mcp-early-stdio-exit")?;
    let fixture_executable = early_exit.install_mcp_fixture_executable()?;
    let output = early_exit.run_init(
        FUTURE_VERSION,
        Some((&fixture_executable, "early_stdio_exit")),
        false,
    )?;
    let report = assert_connection_report(&output, 1, "init", "failed")?;
    assert_eq!(
        report["operation_details"]["result"],
        json!({
            "kind": "setup",
            "disposition": "committed",
            "setup_lease": "acquired",
            "runtime_home_publication": "published_by_this_invocation"
        })
    );
    assert_check(
        &report,
        "mcp_server",
        "failed",
        Some("mcp_server_initialize_failed"),
    );
    let active_probe = report["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|check| check["id"] == "mcp_server"))
        .and_then(|check| check.pointer("/details/last_active_verification/protocol_conformance/0"))
        .ok_or("MCP early-exit diagnostic projection should be present")?;
    assert_eq!(active_probe["diagnostic_code"], "process.child.exited");
    assert_eq!(active_probe["failure_stage"], "initialize");
    let finding_id = DiagnosticFindingId::parse(
        active_probe["finding_id"]
            .as_str()
            .ok_or("MCP early-exit finding ID")?,
    )?;
    let finding = stored_diagnostic_findings_by_ids(
        &early_exit.runtime_home,
        std::slice::from_ref(&finding_id),
    )?
    .into_iter()
    .next()
    .ok_or("persisted MCP early-exit finding")?;
    let finding = finding.to_diagnostic_finding();
    let facts = finding.facts().data();
    assert_eq!(facts.get("exit_code"), Some(&json!(23)));
    assert_eq!(facts.get("bounded_stderr_truncated"), Some(&json!(true)));
    assert_eq!(
        facts.get("bounded_stderr_omitted_bytes"),
        Some(&json!(1024))
    );
    assert!(facts["bounded_stderr_excerpt"].as_str().is_some_and(
        |text| text.len() <= volicord_types::diagnostics::MAX_DIAGNOSTIC_FACT_STRING_BYTES
    ));
    Ok(())
}

fn guard_failures_are_current_and_structured() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::initialized("operational-guard-contract-failure")?;
    let snapshot = fixture.registry_snapshot();
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let project_id = snapshot.projects[0].project_id.clone();
    let manifest = guard_manifest_from_json(&snapshot.guard_installations[0].manifest_json)?;
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        "future.session.guard.failure",
        &manifest,
    )?;

    for (phase, malformed_event) in [
        (
            GuardHookPhase::PromptCapture,
            json!({
                "session_id": "future.session.guard.failure",
                "turn_id": "future.turn.guard.malformed.prompt",
                "prompt": "not persisted in diagnostics"
            }),
        ),
        (
            GuardHookPhase::PreTool,
            json!({
                "session_id": "future.session.guard.failure",
                "turn_id": "future.turn.guard.malformed.pre",
                "tool_use_id": "future.tool-use.malformed.pre",
                "tool_name": "Read",
                "tool_input": {"path": fixture.repo_root.join("README.md")}
            }),
        ),
        (
            GuardHookPhase::PostTool,
            json!({
                "session_id": "future.session.guard.failure",
                "turn_id": "future.turn.guard.malformed.post",
                "tool_use_id": "future.tool-use.malformed.post",
                "tool_name": "Read",
                "tool_input": {"path": fixture.repo_root.join("README.md")},
                "tool_response": {"success": true, "stdout": "not persisted in diagnostics"}
            }),
        ),
    ] {
        let output =
            fixture.run_guard_command(manifest.runtime_commands.get(phase), &malformed_event)?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let host_output: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            host_output.pointer("/hookSpecificOutput/hookEventName"),
            Some(&json!(match phase {
                GuardHookPhase::PromptCapture => "UserPromptSubmit",
                GuardHookPhase::PreTool => "PreToolUse",
                GuardHookPhase::PostTool => "PostToolUse",
            }))
        );
        assert!(host_output
            .pointer("/hookSpecificOutput/additionalContext")
            .and_then(Value::as_str)
            .is_some_and(|message| {
                message.contains("guard.observation.incompatible")
                    && (message.contains("continues") || message.contains("completed"))
            }));
        assert!(host_output
            .pointer("/hookSpecificOutput/permissionDecision")
            .is_none());
    }

    let invalid_json = fixture.run_guard_command_raw(
        manifest.runtime_commands.get(GuardHookPhase::PreTool),
        "{not-json\n".to_owned(),
    )?;
    assert_eq!(invalid_json.status.code(), Some(0));
    assert!(invalid_json.stderr.is_empty());
    let invalid_json_output: Value = serde_json::from_slice(&invalid_json.stdout)?;
    assert!(invalid_json_output
        .pointer("/hookSpecificOutput/additionalContext")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("incompatible")));

    let denied = fixture.run_guard_command(
        manifest.runtime_commands.get(GuardHookPhase::PreTool),
        &json!({
            "hook_event_name": "PreToolUse",
            "session_id": "future.session.guard.failure",
            "turn_id": "future.turn.guard.denied",
            "tool_use_id": "future.tool-use.denied",
            "tool_name": "Write",
            "tool_input": {"path": fixture.repo_root.join("README.md"), "content": "denied"}
        }),
    )?;
    assert_eq!(denied.status.code(), Some(0));
    assert!(denied.stderr.is_empty());
    let denied_output: Value = serde_json::from_slice(&denied.stdout)?;
    assert_eq!(
        denied_output.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("deny"))
    );

    let registry = rusqlite::Connection::open(&snapshot.path)?;
    for code in ["guard.observation.incompatible", "guard.policy.denied"] {
        let count: i64 = registry.query_row(
            "SELECT COUNT(*) FROM diagnostic_findings WHERE lifecycle = 'occurrence' AND code = ?1",
            [code],
            |row| row.get(0),
        )?;
        assert!(count > 0, "missing typed Guard finding {code}");
    }
    let incompatible_facts: String = registry.query_row(
        "SELECT facts_json FROM diagnostic_findings WHERE code = 'guard.observation.incompatible' ORDER BY observed_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    assert!(incompatible_facts.contains("field_category"));
    assert!(!incompatible_facts.contains("not persisted in diagnostics"));
    drop(registry);

    let state_db = fixture.project_state_db_path();
    let displaced = state_db.with_extension("sqlite.guard-displaced");
    fs::rename(&state_db, &displaced)?;
    let unavailable = fixture.run_guard_command(
        manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
        &json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "future.session.guard.failure",
            "turn_id": "future.turn.guard.persistence-unavailable",
            "prompt": "persistence probe"
        }),
    );
    if state_db.exists() {
        fs::remove_file(&state_db)?;
    }
    fs::rename(&displaced, &state_db)?;
    let unavailable = unavailable?;
    assert_eq!(unavailable.status.code(), Some(0));
    assert!(unavailable.stderr.is_empty());
    let unavailable_output: Value = serde_json::from_slice(&unavailable.stdout)?;
    assert!(unavailable_output
        .pointer("/hookSpecificOutput/additionalContext")
        .and_then(Value::as_str)
        .is_some_and(|message| {
            message.contains("guard.event.persistence_unavailable")
                && message.contains("could not persist")
        }));
    let registry = rusqlite::Connection::open(&snapshot.path)?;
    let persistence_findings: i64 = registry.query_row(
        "SELECT COUNT(*) FROM diagnostic_findings WHERE lifecycle = 'occurrence' AND code = 'guard.event.persistence_unavailable'",
        [],
        |row| row.get(0),
    )?;
    assert!(persistence_findings > 0);
    drop(registry);

    let content_before_status = fixture.content_snapshot()?;
    let diagnostics_before_status = fixture.diagnostic_registry_snapshot()?;
    let status = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&status, 1, "status", "failed")?;
    assert_check(
        &report,
        "ambient_hook_coverage",
        "failed",
        Some("ambient_hook_coverage_failed"),
    );
    let guard_finding = report["findings"]
        .as_array()
        .and_then(|findings| {
            findings
                .iter()
                .find(|finding| finding["code"] == "guard.observation.incompatible")
        })
        .ok_or("inline Guard incompatibility finding")?;
    let guard_finding_id = guard_finding["id"]
        .as_str()
        .ok_or("inline Guard finding ID")?
        .to_owned();
    assert!(report["root_cause_ids"]
        .as_array()
        .is_some_and(|roots| roots
            .iter()
            .any(|root| root.as_str() == Some(guard_finding_id.as_str()))));
    assert!(!report["findings"]
        .as_array()
        .is_some_and(|findings| findings
            .iter()
            .any(|finding| finding["code"] == "diagnostics.finding_record_missing")));
    assert!(!serde_json::to_string(&report)?
        .contains("action.diagnostics.rebuild_current_observations"));

    let concise = fixture.run_connection("status", FUTURE_VERSION, false)?;
    assert_eq!(concise.status.code(), Some(1));
    assert!(concise.stderr.is_empty());
    let concise = String::from_utf8(concise.stdout)?;
    assert!(concise.contains("guard.observation.incompatible"));
    assert!(concise.contains(&format!("Finding: {guard_finding_id}")));

    let verbose = fixture.run_connection_verbose("status", FUTURE_VERSION)?;
    assert_eq!(verbose.status.code(), Some(1));
    assert!(verbose.stderr.is_empty());
    let verbose = String::from_utf8(verbose.stdout)?;
    assert!(verbose.contains("Code: guard.observation.incompatible"));
    assert!(verbose.contains(&format!("[root] {guard_finding_id}")));

    assert_status_reads_read_only_registry(&fixture, FUTURE_VERSION)?;
    assert_eq!(fixture.content_snapshot()?, content_before_status);
    assert_eq!(
        fixture.diagnostic_registry_snapshot()?,
        diagnostics_before_status,
        "status changed diagnostic counts or current snapshot timestamps"
    );

    let cli_preflight_before_verify = fixture.cli_preflight_session_count()?;
    let verify = fixture.run_connection("verify", FUTURE_VERSION, true)?;
    let verify_report = assert_connection_report(&verify, 1, "verify", "failed")?;
    let verify_guard_finding = verify_report["findings"]
        .as_array()
        .and_then(|findings| {
            findings
                .iter()
                .find(|finding| finding["code"] == "guard.observation.incompatible")
        })
        .ok_or("verified Guard incompatibility finding")?;
    assert_eq!(verify_guard_finding["id"], guard_finding_id);
    assert_eq!(
        fixture.cli_preflight_session_count()?,
        cli_preflight_before_verify
    );
    Ok(())
}

struct OperationalFixture {
    _temporary_root: TempRuntimeHome,
    git: IsolatedGitRepository,
    runtime_home: PathBuf,
    codex_home: PathBuf,
    user_home: PathBuf,
    path_dir: PathBuf,
    repo_root: PathBuf,
    shared: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct DiagnosticRegistrySnapshot {
    occurrence_count: i64,
    current_count: i64,
    current_timestamps: Vec<(String, String, Option<String>)>,
}

impl OperationalFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        Self::with_scope(prefix, false)
    }

    fn with_scope(prefix: &str, shared: bool) -> Result<Self, Box<dyn Error>> {
        Self::with_scope_and_planning_product(prefix, shared, false)
    }

    fn planning_product(prefix: &str) -> Result<Self, Box<dyn Error>> {
        Self::with_scope_and_planning_product(prefix, false, true)
    }

    fn with_scope_and_planning_product(
        prefix: &str,
        shared: bool,
        planning_product: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let temporary_root = TempRuntimeHome::new(prefix)?;
        let runtime_home = temporary_root.root_path().join("runtime-home");
        let codex_home = temporary_root.root_path().join("codex-home");
        let user_home = temporary_root.root_path().join("user-home");
        let path_dir = temporary_root.root_path().join("path");
        let repo_root = temporary_root.root_path().join("product-repository");
        for directory in [&codex_home, &user_home, &path_dir, &repo_root] {
            fs::create_dir_all(directory)?;
        }
        let managed_guidance_relative =
            GuardManagedArtifact::AgentsManagedBlock.repository_relative_path()?;
        let managed_guidance_path = repo_root.join(managed_guidance_relative);
        if let Some(parent) = managed_guidance_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            managed_guidance_path,
            "# Product repository guidance\n\nPreserve this tracked baseline.\n",
        )?;
        let git = IsolatedGitRepository::initialize_at(&repo_root)?;
        if planning_product {
            git.write(
                "plans/product.md",
                b"# Product plan\n\nPrepare one bounded first release from the approved recommendations.\n",
            )?;
            git.write(
                "plans/experience.md",
                b"# Experience plan\n\nKeep the first release small, readable, and reversible.\n",
            )?;
            git.write(
                "plans/technical.md",
                b"# Technical plan\n\nChoose the smallest local implementation boundary before writing.\n",
            )?;
        }
        git.write(
            ".gitattributes",
            format!("{TRANSFORMED_TRACKED_PATH} text eol=crlf\n").as_bytes(),
        )?;
        git.write(TRANSFORMED_TRACKED_PATH, b"record=baseline\r\n")?;
        git.commit_all("operational fixture baseline")?;
        let codex_name = if cfg!(windows) { "codex.exe" } else { "codex" };
        fs::copy(env::current_exe()?, path_dir.join(codex_name))?;
        let volicord_name = if cfg!(windows) {
            "volicord.exe"
        } else {
            "volicord"
        };
        fs::copy(env!("CARGO_BIN_EXE_volicord"), path_dir.join(volicord_name))?;
        Ok(Self {
            _temporary_root: temporary_root,
            git,
            runtime_home,
            codex_home,
            user_home,
            path_dir,
            repo_root,
            shared,
        })
    }

    fn initialized(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let fixture = Self::new(prefix)?;
        let output = fixture.run_init(FUTURE_VERSION, None, false)?;
        assert_connection_report(&output, 0, "init", "action_required")?;
        Ok(fixture)
    }

    fn install_mcp_fixture_executable(&self) -> Result<PathBuf, Box<dyn Error>> {
        let directory = self._temporary_root.root_path().join("mcp-fixture");
        fs::create_dir_all(&directory)?;
        let name = if cfg!(windows) {
            "volicord.exe"
        } else {
            "volicord"
        };
        let path = directory.join(name);
        fs::copy(env::current_exe()?, &path)?;
        Ok(path)
    }

    fn base_command(&self, program: impl AsRef<OsStr>, version: &str) -> Command {
        let mut command = Command::new(program);
        let mut search_paths = vec![self.path_dir.clone()];
        if let Some(ambient_path) = env::var_os("PATH") {
            search_paths.extend(env::split_paths(&ambient_path));
        }
        let search_path =
            env::join_paths(search_paths).expect("operational fixture PATH should join");
        command
            .env_clear()
            .env_remove("WSL_DISTRO_NAME")
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env("PATH", search_path)
            .env(CODEX_VERSION_ENV, version)
            .current_dir(&self.repo_root);
        self.git.apply_environment(&mut command);
        #[cfg(windows)]
        copy_required_windows_environment(&mut command);
        command
    }

    fn git_output(&self, arguments: &[&str], paths: &[&OsStr]) -> Result<String, Box<dyn Error>> {
        let mut all_arguments = arguments.iter().map(OsStr::new).collect::<Vec<_>>();
        all_arguments.extend_from_slice(paths);
        Ok(String::from_utf8(self.git.git_os(&all_arguments)?.stdout)?)
    }

    fn run_init(
        &self,
        version: &str,
        mcp_fixture: Option<(&Path, &str)>,
        dry_run: bool,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .env(
                "VOLICORD_HOME",
                self._temporary_root
                    .root_path()
                    .join("ambient-decoy-runtime-home"),
            )
            .env_remove("WSL_DISTRO_NAME")
            .arg("init")
            .arg("--host")
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--profile")
            .arg("record")
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--json");
        if self.shared {
            command.arg("--shared");
        }
        if let Some((path, mode)) = mcp_fixture {
            command
                .arg("--mcp-command")
                .arg(path)
                .env(MCP_FIXTURE_MODE, mode);
        }
        if dry_run {
            command.arg("--dry-run");
        }
        Ok(command.output()?)
    }

    fn run_connection(
        &self,
        operation: &str,
        version: &str,
        json: bool,
    ) -> Result<Output, Box<dyn Error>> {
        self.run_connection_with_path(operation, version, json, &self.path_dir)
    }

    fn run_inbox(&self, arguments: &[&str]) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), FUTURE_VERSION);
        command.arg("inbox").args(arguments);
        Ok(command.output()?)
    }

    fn run_connection_verbose(
        &self,
        operation: &str,
        version: &str,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .arg("connection")
            .arg(operation)
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--verbose");
        Ok(command.output()?)
    }

    fn run_connection_list(
        &self,
        version: &str,
        json: bool,
        verbose: bool,
    ) -> Result<Output, Box<dyn Error>> {
        self.run_connection_list_for_repo(version, json, verbose, &self.repo_root)
    }

    fn run_connection_list_for_repo(
        &self,
        version: &str,
        json: bool,
        verbose: bool,
        repo: &Path,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .arg("connection")
            .arg("list")
            .arg("--repo")
            .arg(repo);
        if json {
            command.arg("--json");
        }
        if verbose {
            command.arg("--verbose");
        }
        Ok(command.output()?)
    }

    fn run_connection_list_all(
        &self,
        version: &str,
        json: bool,
        verbose: bool,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command.arg("connection").arg("list");
        if json {
            command.arg("--json");
        }
        if verbose {
            command.arg("--verbose");
        }
        Ok(command.output()?)
    }

    fn run_connection_add_for_repo(
        &self,
        version: &str,
        repo: &Path,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .arg("connection")
            .arg("add")
            .arg("codex")
            .arg("--repo")
            .arg(repo)
            .arg("--json");
        Ok(command.output()?)
    }

    fn run_connection_with_path(
        &self,
        operation: &str,
        version: &str,
        json: bool,
        path: &Path,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .env("PATH", path)
            .arg("connection")
            .arg(operation)
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root);
        if json {
            command.arg("--json");
        }
        Ok(command.output()?)
    }

    fn run_connection_mode(
        &self,
        mode: &str,
        version: &str,
        json: bool,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .arg("connection")
            .arg("mode")
            .arg("codex")
            .arg(mode)
            .arg("--repo")
            .arg(&self.repo_root);
        if json {
            command.arg("--json");
        }
        Ok(command.output()?)
    }

    fn run_managed_tools_list_names(
        &self,
        connection_id: &str,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let output = self.run_managed_mcp_messages(
            connection_id,
            json_lines(&[
                initialize_request(FUTURE_VERSION),
                initialized_notification(),
                tools_list_request(),
            ])?,
        )?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let responses = json_rpc_responses(&output.stdout)?;
        Ok(responses[1]["result"]["tools"]
            .as_array()
            .expect("tools/list array")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name").to_owned())
            .collect())
    }

    fn run_successful_managed_mcp(
        &self,
        connection_id: &str,
        project_id: &str,
        version: &str,
        native_session: &str,
    ) -> Result<(), Box<dyn Error>> {
        let output = self.run_managed_mcp_messages(
            connection_id,
            json_lines(&[
                initialize_request(version),
                initialized_notification(),
                tools_list_request(),
                managed_tool_call(
                    3,
                    managed_host_round_trip_tool().wire_name(),
                    json!({}),
                    native_session,
                ),
            ])?,
        )?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let responses = json_rpc_responses(&output.stdout)?;
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[2]["result"]["isError"], false);
        let connection = agent_connection_record(&self.runtime_home, connection_id)?
            .ok_or("managed MCP acceptance Connection should exist")?;
        let expected_tools = match connection.mode.as_str() {
            "workflow" => AgentToolId::ALL
                .iter()
                .map(|tool| tool.wire_name())
                .collect::<Vec<_>>(),
            "read_only" => AgentToolId::ALL
                .iter()
                .copied()
                .filter(|tool| tool.available_in(AgentConnectionMode::ReadOnly))
                .map(AgentToolId::wire_name)
                .collect::<Vec<_>>(),
            mode => return Err(format!("unexpected Connection mode {mode}").into()),
        };
        let actual_tools = responses[1]["result"]["tools"]
            .as_array()
            .ok_or("tools/list should return an array")?
            .iter()
            .map(|tool| tool["name"].as_str().ok_or("tool name should be a string"))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(actual_tools, expected_tools);
        let projects = adapter_tool_response(&responses[2])?;
        let project = projects["projects"]
            .as_array()
            .ok_or("list_projects should return projects")?
            .iter()
            .find(|project| project["project_selector"] == project_id)
            .ok_or("list_projects should return the registered disposable project")?;
        assert_eq!(project["available"], true);
        assert_eq!(project["repo_root"].as_str(), self.repo_root.to_str());
        Ok(())
    }

    fn run_successful_managed_mcp_with_guard(
        &self,
        connection_id: &str,
        project_id: &str,
        version: &str,
        native_session: &str,
        manifest: &GuardManifest,
    ) -> Result<(), Box<dyn Error>> {
        let mut command = self.managed_mcp_command(connection_id)?;
        let mut child = LiveMcpChild::spawn(&mut command)?;
        let process_id = child.id();
        child.write(&json_lines(&[
            initialize_request(version),
            initialized_notification(),
            tools_list_request(),
        ])?)?;
        let startup_responses = child
            .read_responses(2)
            .map_err(|error| format!("managed startup response failed: {error}"))?;
        let schema_invalid_tool_name = startup_responses[1]["result"]["tools"]
            .as_array()
            .ok_or("tools/list result must carry tools")?
            .iter()
            .find(|tool| {
                tool["inputSchema"]["required"]
                    .as_array()
                    .is_some_and(|required| {
                        required.iter().any(|field| {
                            field
                                .as_str()
                                .is_some_and(|field| field != "project_selector")
                        })
                    })
                    && tool["annotations"]["readOnlyHint"] == true
            })
            .and_then(|tool| tool["name"].as_str())
            .ok_or("generated tool catalog must expose a required-argument schema")?
            .to_owned();
        let schema_invalid_tool = AgentToolId::ALL
            .iter()
            .copied()
            .find(|tool| tool.wire_name() == schema_invalid_tool_name)
            .ok_or("generated required-argument tool must have a semantic identity")?;
        let runtime =
            mcp_runtime_session_for_process(&self.runtime_home, connection_id, process_id)?
                .ok_or("managed MCP runtime was not recorded before its initialize response")?;
        assert!(
            runtime.tools_list_observed_at.is_some(),
            "tools/list response preceded its runtime milestone"
        );
        let runtime_session_id = runtime.runtime_session_id;

        let project_state = rusqlite::Connection::open(self.project_state_db_path())?;
        let guard_history_before: (i64, i64) = (
            project_state.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?,
            project_state
                .query_row("SELECT COUNT(*) FROM prompt_captures", [], |row| row.get(0))?,
        );
        drop(project_state);

        let prompt = json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "prompt": "Verify current MCP and Guard integration."
        });
        let prompt_output = self.run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
            &prompt,
        )?;
        assert!(prompt_output.status.success());

        let connection = agent_connection_record(&self.runtime_home, connection_id)?
            .ok_or("managed Guard Connection should exist")?;
        assert_eq!(connection.server_name, "volicord");
        let server = McpServerKey::parse(&connection.server_name)?;
        let list_callable = project_mcp_tool(&server, AgentToolId::LIST_PROJECTS)?;
        let begin_callable =
            project_mcp_tool(&server, AgentToolId::BEGIN_INTEGRATION_VERIFICATION)?;
        let probe_callable = project_mcp_tool(&server, AgentToolId::GUARD_PROBE)?;
        let status_callable = project_mcp_tool(&server, AgentToolId::GET_INTEGRATION_VERIFICATION)?;
        let schema_invalid_callable = project_mcp_tool(&server, schema_invalid_tool)?;

        let list_pre = json!({
            "hook_event_name": "PreToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": "future.tool-use.integration-verification-list",
            "tool_name": list_callable.callable_name().as_str(),
            "tool_input": {"project_selector": project_id},
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PreTool),
                &list_pre,
            )?
            .status
            .success());
        child.write(&json_lines(&[managed_tool_call_in_turn(
            3,
            managed_host_round_trip_tool().wire_name(),
            json!({}),
            native_session,
            INTEGRATION_VERIFICATION_TURN_ID,
        )])?)?;
        child
            .read_responses(1)
            .map_err(|error| format!("managed list-projects response failed: {error}"))?;
        let list_post = json!({
            "hook_event_name": "PostToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": "future.tool-use.integration-verification-list",
            "tool_name": list_callable.callable_name().as_str(),
            "tool_input": {"project_selector": project_id},
            "tool_response": {"success": true},
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PostTool),
                &list_post,
            )?
            .status
            .success());

        let schema_invalid_turn = "future.turn.schema-invalid";
        let schema_invalid_tool_use_id = "future.tool-use.schema-invalid";
        let schema_invalid_pre = json!({
            "hook_event_name": "PreToolUse",
            "session_id": native_session,
            "turn_id": schema_invalid_turn,
            "tool_use_id": schema_invalid_tool_use_id,
            "tool_name": schema_invalid_callable.callable_name().as_str(),
            "tool_input": {"project_selector": project_id},
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PreTool),
                &schema_invalid_pre,
            )?
            .status
            .success());
        let project_state = rusqlite::Connection::open(self.project_state_db_path())?;
        let schema_invalid_session_id = current_project_agent_session_coordinates(
            &self.runtime_home,
            project_id,
            connection_id,
            Some(manifest.guard_installation_id.as_str()),
            &host_session_correlation(native_session),
        )?
        .session_id;
        let schema_invalid_observation_id: String = project_state.query_row(
            "SELECT repository_observation_id
               FROM repository_observations
              WHERE host_tool_use_id = ?1 AND session_id = ?2",
            [
                schema_invalid_tool_use_id,
                schema_invalid_session_id.as_str(),
            ],
            |row| row.get(0),
        )?;
        let schema_invalid_user_actions_before: i64 =
            project_state.query_row("SELECT COUNT(*) FROM user_action_requests", [], |row| {
                row.get(0)
            })?;
        drop(project_state);
        let schema_invalid_observation = repository_observation(
            &self.runtime_home,
            project_id,
            &schema_invalid_observation_id,
        )?
        .ok_or("schema-invalid PreTool observation")?;
        assert_eq!(
            schema_invalid_observation.state,
            RepositoryObservationState::Open,
            "generated candidate {} ended before MCP validation with {:?}",
            schema_invalid_tool.wire_name(),
            schema_invalid_observation.unavailable_reason
        );
        child.write(&json_lines(&[managed_tool_call_in_turn(
            30,
            schema_invalid_tool.wire_name(),
            json!({"project_selector": project_id}),
            native_session,
            schema_invalid_turn,
        )])?)?;
        let schema_invalid_response = child
            .read_responses(1)
            .map_err(|error| format!("schema-invalid response failed: {error}"))?
            .remove(0);
        assert!(
            schema_invalid_response.get("error").is_some()
                || schema_invalid_response["result"]["isError"] == true,
            "schema-invalid generated tool call unexpectedly succeeded: {schema_invalid_response}"
        );
        let next_prompt = json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": native_session,
            "turn_id": "future.turn.after-schema-invalid",
            "prompt": "Continue after the rejected tool invocation."
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PromptCapture),
                &next_prompt,
            )?
            .status
            .success());
        let terminalized_schema_invalid = repository_observation(
            &self.runtime_home,
            project_id,
            &schema_invalid_observation_id,
        )?
        .ok_or("terminalized schema-invalid observation")?;
        assert_eq!(
            terminalized_schema_invalid.state,
            RepositoryObservationState::Unavailable
        );
        assert_eq!(
            terminalized_schema_invalid.unavailable_reason,
            Some(RepositoryObservationUnavailableReason::PostToolNotObserved)
        );
        assert!(terminalized_schema_invalid.delta.is_none());
        let project_state = rusqlite::Connection::open(self.project_state_db_path())?;
        assert_eq!(
            project_state.query_row("SELECT COUNT(*) FROM user_action_requests", [], |row| {
                row.get::<_, i64>(0)
            })?,
            schema_invalid_user_actions_before,
            "schema validation failure must not create a UserAction request"
        );
        assert_eq!(
            project_state.query_row(
                "SELECT COUNT(*) FROM repository_observations
                  WHERE repository_observation_id = ?1 AND state = 'open'",
                [&schema_invalid_observation_id],
                |row| row.get::<_, i64>(0),
            )?,
            0,
            "the missing PostToolUse observation must not remain orphaned"
        );
        drop(project_state);

        let begin_input = json!({"project_selector": project_id});
        let begin_pre = json!({
            "hook_event_name": "PreToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": "future.tool-use.integration-verification-begin",
            "tool_name": begin_callable.callable_name().as_str(),
            "tool_input": begin_input,
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PreTool),
                &begin_pre,
            )?
            .status
            .success());
        child.write(&json_lines(&[managed_tool_call_in_turn(
            4,
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            json!({"project_selector": project_id}),
            native_session,
            INTEGRATION_VERIFICATION_TURN_ID,
        )])?)?;
        let registry_path = self.runtime_home.join("registry.sqlite");
        let begin_responses = child
            .read_responses(1)
            .map_err(|error| format!("verification begin response failed: {error}"))?;
        let begin = adapter_tool_response(&begin_responses[0])?;
        let verification_id = begin["verification_id"]
            .as_str()
            .ok_or("begin response verification ID")?
            .to_owned();
        assert_eq!(
            begin["workflow"]["kind"],
            IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND
        );
        assert_eq!(
            begin["workflow"]["tool"],
            AgentToolId::GUARD_PROBE.wire_name()
        );
        let begin_post = json!({
            "hook_event_name": "PostToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": "future.tool-use.integration-verification-begin",
            "tool_name": begin_callable.callable_name().as_str(),
            "tool_input": {"project_selector": project_id},
            "tool_response": {"success": true},
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PostTool),
                &begin_post,
            )?
            .status
            .success());
        let probe_input = json!({"verification_id": verification_id});
        let pre_tool = json!({
            "hook_event_name": "PreToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": INTEGRATION_VERIFICATION_TOOL_USE_ID,
            "tool_name": probe_callable.callable_name().as_str(),
            "tool_input": probe_input,
        });
        let pre_output = self.run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PreTool),
            &pre_tool,
        )?;
        assert!(pre_output.status.success());

        child.write(&json_lines(&[managed_tool_call_in_turn(
            5,
            AgentToolId::GUARD_PROBE.wire_name(),
            json!({"verification_id": verification_id}),
            native_session,
            INTEGRATION_VERIFICATION_TURN_ID,
        )])?)?;
        let probe_responses = child
            .read_responses(1)
            .map_err(|error| format!("Guard probe response failed: {error}"))?;
        let probe = adapter_tool_response(&probe_responses[0])?;
        assert_eq!(probe["verification_id"], verification_id);
        assert_eq!(
            probe["workflow"]["kind"],
            IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND
        );
        assert_eq!(
            probe["workflow"]["tool"],
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name()
        );

        let post_tool = json!({
            "hook_event_name": "PostToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": INTEGRATION_VERIFICATION_TOOL_USE_ID,
            "tool_name": probe_callable.callable_name().as_str(),
            "tool_input": {"verification_id": verification_id},
            "tool_response": {"success": true},
        });
        let post_output = self.run_guard_command(
            manifest.runtime_commands.get(GuardHookPhase::PostTool),
            &post_tool,
        )?;
        assert!(post_output.status.success());
        let registry = rusqlite::Connection::open(&registry_path)?;
        let completed_before_status: GuardAttemptTerminalSnapshot = registry.query_row(
            "SELECT status, status_read_count, probe_acknowledged_at, completed_at,
                    matched_prompt_event_id, matched_pre_tool_event_id,
                    matched_post_tool_event_id
               FROM guard_integration_verification_runs
              WHERE verification_id = ?1",
            [&verification_id],
            |row| {
                Ok(GuardAttemptTerminalSnapshot {
                    status: row.get(0)?,
                    status_read_count: row.get(1)?,
                    probe_acknowledged_at: row.get(2)?,
                    completed_at: row.get(3)?,
                    matched_prompt_event_id: row.get(4)?,
                    matched_pre_tool_event_id: row.get(5)?,
                    matched_post_tool_event_id: row.get(6)?,
                })
            },
        )?;
        assert_eq!(completed_before_status.status, "complete");
        assert_eq!(
            completed_before_status.status_read_count, 0,
            "event correlation should complete without consuming the one status-read budget"
        );
        drop(registry);

        let status_pre = json!({
            "hook_event_name": "PreToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": "future.tool-use.integration-verification-status",
            "tool_name": status_callable.callable_name().as_str(),
            "tool_input": {"verification_id": verification_id},
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PreTool),
                &status_pre,
            )?
            .status
            .success());
        child.write(&json_lines(&[managed_tool_call_in_turn(
            6,
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
            json!({"verification_id": verification_id}),
            native_session,
            INTEGRATION_VERIFICATION_TURN_ID,
        )])?)?;
        let status_responses = child
            .read_responses(1)
            .map_err(|error| format!("verification status response failed: {error}"))?;
        let verification = adapter_tool_response(&status_responses[0])?;
        assert_eq!(verification["verification_id"], verification_id);
        assert_eq!(
            verification["workflow"]["kind"],
            IntegrationVerificationWorkflowState::COMPLETE_KIND
        );
        assert!(verification["workflow"].get("tool").is_none());
        assert_eq!(verification["guard_phases"]["prompt_capture"], "matched");
        assert_eq!(verification["guard_phases"]["pre_tool"], "matched");
        assert_eq!(verification["guard_phases"]["post_tool"], "matched");
        let status_post = json!({
            "hook_event_name": "PostToolUse",
            "session_id": native_session,
            "turn_id": INTEGRATION_VERIFICATION_TURN_ID,
            "tool_use_id": "future.tool-use.integration-verification-status",
            "tool_name": status_callable.callable_name().as_str(),
            "tool_input": {"verification_id": verification_id},
            "tool_response": {"success": true},
        });
        assert!(self
            .run_guard_command(
                manifest.runtime_commands.get(GuardHookPhase::PostTool),
                &status_post,
            )?
            .status
            .success());

        let current_session_id = current_project_agent_session_coordinates(
            &self.runtime_home,
            project_id,
            connection_id,
            Some(manifest.guard_installation_id.as_str()),
            &host_session_correlation(native_session),
        )?
        .session_id;
        let registry = rusqlite::Connection::open(&registry_path)?;
        let round_trip_observed: i64 = registry.query_row(
            "SELECT COUNT(*)
               FROM mcp_runtime_sessions
              WHERE runtime_session_id = ?1
                AND verification_tool_name = ?2
                AND verification_tool_observed_at IS NOT NULL",
            [
                runtime_session_id.as_str(),
                managed_host_round_trip_tool().wire_name(),
            ],
            |row| row.get(0),
        )?;
        assert_eq!(round_trip_observed, 1);
        let completed_after_status: GuardAttemptTerminalSnapshot = registry.query_row(
            "SELECT status, status_read_count, probe_acknowledged_at, completed_at,
                    matched_prompt_event_id, matched_pre_tool_event_id,
                    matched_post_tool_event_id
               FROM guard_integration_verification_runs
              WHERE verification_id = ?1",
            [&verification_id],
            |row| {
                Ok(GuardAttemptTerminalSnapshot {
                    status: row.get(0)?,
                    status_read_count: row.get(1)?,
                    probe_acknowledged_at: row.get(2)?,
                    completed_at: row.get(3)?,
                    matched_prompt_event_id: row.get(4)?,
                    matched_pre_tool_event_id: row.get(5)?,
                    matched_post_tool_event_id: row.get(6)?,
                })
            },
        )?;
        assert_eq!(completed_after_status, completed_before_status);
        let attempts = registry
            .prepare(
                "SELECT verification_id, runtime_session_id, status,
                        matched_prompt_event_id, created_at
                   FROM guard_integration_verification_runs
                  WHERE connection_internal_id = ?1
                    AND runtime_session_id = ?2
                    AND host_session_id = ?3
                    AND host_turn_id = ?4
                  ORDER BY created_at, verification_id",
            )?
            .query_map(
                [
                    connection_id,
                    runtime_session_id.as_str(),
                    native_session,
                    INTEGRATION_VERIFICATION_TURN_ID,
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            attempts.len(),
            1,
            "the journey must create exactly one attempt for its semantic runtime/turn coordinate: {attempts:?}"
        );
        assert_eq!(attempts[0].0, verification_id);
        drop(registry);

        let output = child.finish()?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let responses = json_rpc_responses(&output.stdout)?;
        assert_eq!(responses.len(), 7);
        for response in [&responses[2], &responses[4], &responses[5], &responses[6]] {
            assert_eq!(response["result"]["isError"], false, "{response}");
        }
        let projects = adapter_tool_response(&responses[2]).map_err(|error| {
            format!(
                "list-projects response was invalid: {error}; {}",
                responses[2]
            )
        })?;
        assert!(projects["projects"]
            .as_array()
            .is_some_and(|projects| projects.iter().any(|project| {
                project["project_selector"] == project_id && project["available"] == true
            })));
        let begin = adapter_tool_response(&responses[4]).map_err(|error| {
            format!(
                "begin integration-verification response was invalid: {error}; {}",
                responses[4]
            )
        })?;
        assert_eq!(begin["verification_id"], verification_id);
        assert_eq!(
            begin["workflow"]["kind"],
            IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND
        );
        assert_eq!(
            begin["workflow"]["tool"],
            AgentToolId::GUARD_PROBE.wire_name()
        );
        let probe = adapter_tool_response(&responses[5]).map_err(|error| {
            format!(
                "Guard probe response was invalid: {error}; {}",
                responses[5]
            )
        })?;
        assert_eq!(probe["verification_id"], verification_id);
        assert_eq!(
            probe["workflow"]["kind"],
            IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND
        );
        assert_eq!(
            probe["workflow"]["tool"],
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name()
        );
        let verification = adapter_tool_response(&responses[6]).map_err(|error| {
            format!(
                "integration-verification lookup response was invalid: {error}; {}",
                responses[6]
            )
        })?;
        assert_eq!(verification["verification_id"], verification_id);
        assert_eq!(
            verification["workflow"]["kind"],
            IntegrationVerificationWorkflowState::COMPLETE_KIND
        );
        assert!(verification["workflow"].get("tool").is_none());
        assert_eq!(verification["guard_phases"]["prompt_capture"], "matched");
        assert_eq!(verification["guard_phases"]["pre_tool"], "matched");
        assert_eq!(verification["guard_phases"]["post_tool"], "matched");
        let project_state = rusqlite::Connection::open(self.project_state_db_path())?;
        let bound_runtime: Option<String> = project_state.query_row(
            "SELECT runtime_session_id FROM managed_mcp_sessions WHERE session_id = ?1",
            [&current_session_id],
            |row| row.get(0),
        )?;
        assert!(
            bound_runtime.is_some(),
            "successful managed tool response did not attach Agent Session: {responses:?}"
        );
        assert_eq!(
            project_state.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row
                .get::<_, i64>(0))?,
            guard_history_before.0 + 11
        );
        assert_eq!(
            project_state.query_row("SELECT COUNT(*) FROM prompt_captures", [], |row| row
                .get::<_, i64>(0))?,
            guard_history_before.1 + 2
        );
        let observation_rows = project_state
            .prepare(
                "SELECT repository_observation_id, host_tool_use_id, host_tool_name, state,
                        unavailable_reason, session_id
                   FROM repository_observations
                  WHERE host_turn_id = ?1
                    AND session_id = ?2
                  ORDER BY host_tool_use_id, started_at",
            )?
            .query_map(
                [
                    INTEGRATION_VERIFICATION_TURN_ID,
                    current_session_id.as_str(),
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            observation_rows.len(),
            4,
            "the four managed verification tools must each own one observation: {observation_rows:?}"
        );
        for (observation_id, _, _, _, _, _) in observation_rows {
            let observation =
                repository_observation(&self.runtime_home, project_id, &observation_id)?
                    .ok_or("managed verification repository observation")?;
            assert_eq!(
                observation.state,
                RepositoryObservationState::Complete,
                "managed tool observation {} ended {:?}: {:?}",
                observation_id,
                observation.state,
                observation.unavailable_reason
            );
            assert!(observation
                .delta
                .as_ref()
                .is_some_and(|delta| delta.is_empty()));
            let result = observation
                .terminal_result
                .as_ref()
                .ok_or("managed verification terminal repository result")?;
            assert_eq!(
                result.observation_state,
                RepositoryObservationState::Complete
            );
            assert_eq!(
                result.delta.as_ref().map(|delta| delta.transition_count),
                Some(0)
            );
            assert!(result.expected_write_matches.is_empty());
            assert!(result.unrecorded_changes.is_empty());
        }
        assert_eq!(
            project_state.query_row("SELECT COUNT(*) FROM expected_writes", [], |row| {
                row.get::<_, i64>(0)
            })?,
            0
        );
        assert_eq!(
            project_state.query_row("SELECT COUNT(*) FROM unrecorded_changes", [], |row| {
                row.get::<_, i64>(0)
            })?,
            0
        );
        Ok(())
    }

    fn run_safe_tool_storage_failure(&self) -> Result<(), Box<dyn Error>> {
        let connection_id = self.connection_id();
        let git_dir = self.repo_root.join(".git");
        let displaced_git_dir = self.repo_root.join(".git-safe-call-displaced");
        fs::rename(&git_dir, &displaced_git_dir)?;
        fs::create_dir(&git_dir)?;
        let output = self.run_managed_mcp_messages(
            &connection_id,
            json_lines(&[
                initialize_request(FUTURE_VERSION),
                initialized_notification(),
                tools_list_request(),
                managed_tool_call(
                    3,
                    "volicord.status",
                    json!({"detail": "workflow", "task_id": "task_missing"}),
                    "future.session.safe.failure",
                ),
            ])?,
        )?;
        fs::remove_dir(&git_dir)?;
        fs::rename(&displaced_git_dir, &git_dir)?;
        assert!(
            !output.status.success()
                || json_rpc_responses(&output.stdout)?.iter().any(|response| {
                    response.pointer("/result/isError").and_then(Value::as_bool) == Some(true)
                        || response.get("error").is_some()
                }),
            "safe call unexpectedly succeeded"
        );
        Ok(())
    }

    fn run_managed_mcp_messages(
        &self,
        connection_id: &str,
        input: String,
    ) -> Result<support::binary_fixture::CapturedChildOutput, Box<dyn Error>> {
        let command = self.managed_mcp_command(connection_id)?;
        run_child(command, ChildStdin::WriteAndClose(input))
    }

    fn managed_mcp_command(&self, connection_id: &str) -> Result<Command, Box<dyn Error>> {
        let launch = self.managed_launch_spec(connection_id)?;
        let forwarded_environment = if launch
            .environment()
            .forwarded_names()
            .contains(VOLICORD_HOME_ENV)
        {
            BTreeMap::from([(
                VOLICORD_HOME_ENV.to_owned(),
                self.runtime_home.clone().into_os_string(),
            )])
        } else {
            BTreeMap::new()
        };
        let working_directory = if self.shared {
            ManagedMcpWorkingDirectory::ProductRepository(self.repo_root.clone())
        } else {
            ManagedMcpWorkingDirectory::Inherited
        };
        let materialized = launch.materialize(ManagedMcpMaterializationInput::new(
            ManagedMcpInvocationPurpose::ManagedStdio,
            forwarded_environment,
            working_directory,
        ))?;
        let mut command = materialized.process_command();
        command
            .env("PATH", &self.path_dir)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env_remove("WSL_DISTRO_NAME");
        self.git.apply_environment(&mut command);
        #[cfg(windows)]
        copy_required_windows_environment(&mut command);
        Ok(command)
    }

    fn managed_launch_spec(
        &self,
        connection_id: &str,
    ) -> Result<ManagedMcpLaunchSpec, Box<dyn Error>> {
        let snapshot = self.registry_snapshot();
        let connection = snapshot
            .agent_connections
            .iter()
            .find(|connection| connection.connection_internal_id == connection_id)
            .ok_or("managed launch Connection should exist")?;
        let document = fs::read_to_string(&connection.config_target)?.parse::<DocumentMut>()?;
        let entry = document["mcp_servers"]["volicord"]
            .as_table()
            .ok_or("managed Codex entry should be a table")?;
        let command = entry["command"]
            .as_str()
            .ok_or("managed Codex command should be a string")?
            .to_owned();
        let args = toml_entry_string_array(entry, "args")?;
        let static_environment = entry
            .get("env")
            .map(|item| {
                item.as_table()
                    .ok_or("managed Codex env should be a table")?
                    .iter()
                    .map(|(name, value)| {
                        value
                            .as_str()
                            .map(|value| (name.to_owned(), value.to_owned()))
                            .ok_or("managed Codex env values should be strings")
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let forwarded_environment = entry
            .get("env_vars")
            .map(|_| toml_entry_string_array(entry, "env_vars"))
            .transpose()?
            .unwrap_or_default();
        Ok(ManagedMcpLaunchSpec::try_from_host_projection(
            command,
            args,
            static_environment,
            forwarded_environment,
        )?)
    }

    fn assert_cli_verification_observations_are_isolated(
        &self,
        connection_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let snapshot = self.registry_snapshot();
        let registry = rusqlite::Connection::open(&snapshot.path)?;
        let managed_count: i64 = registry.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE connection_internal_id = ?1 AND session_source = 'managed_host'",
            [connection_id],
            |row| row.get(0),
        )?;
        let cli_count: i64 = registry.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE connection_internal_id = ?1 AND session_source = 'cli_preflight'",
            [connection_id],
            |row| row.get(0),
        )?;
        let cli_verification_evidence_count: i64 = registry.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE connection_internal_id = ?1 AND session_source = 'cli_preflight' AND (verification_tool_name IS NOT NULL OR verification_tool_observed_at IS NOT NULL)",
            [connection_id],
            |row| row.get(0),
        )?;
        assert_eq!(managed_count, 0);
        assert_eq!(cli_count, 0);
        assert_eq!(cli_verification_evidence_count, 0);
        Ok(())
    }

    fn run_current_guard_phases(
        &self,
        manifest: &GuardManifest,
        native_session: &str,
    ) -> Result<(), Box<dyn Error>> {
        for (phase, event) in [
            (
                GuardHookPhase::PreTool,
                json!({
                    "hook_event_name": "PreToolUse",
                    "session_id": native_session,
                    "turn_id": "future.turn.tool",
                    "tool_use_id": "future.tool-use.read",
                    "tool_name": "Read",
                    "tool_input": {"path": self.repo_root.join("README.md")}
                }),
            ),
            (
                GuardHookPhase::PostTool,
                json!({
                    "hook_event_name": "PostToolUse",
                    "session_id": native_session,
                    "turn_id": "future.turn.tool",
                    "tool_use_id": "future.tool-use.read",
                    "tool_name": "Read",
                    "tool_input": {"path": self.repo_root.join("README.md")},
                    "tool_response": {"success": true}
                }),
            ),
            (
                GuardHookPhase::PromptCapture,
                json!({
                    "hook_event_name": "UserPromptSubmit",
                    "session_id": native_session,
                    "turn_id": "future.turn.prompt",
                    "prompt": "Observe current Guard behavior."
                }),
            ),
        ] {
            let output = self.run_guard_command(manifest.runtime_commands.get(phase), &event)?;
            assert!(
                output.status.success(),
                "Guard phase {} failed: {}",
                phase.as_str(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    fn run_guard_command(
        &self,
        command_spec: &volicord_types::guard_manifest::GuardCommand,
        event: &Value,
    ) -> Result<support::binary_fixture::CapturedChildOutput, Box<dyn Error>> {
        self.run_guard_command_raw(command_spec, format!("{}\n", serde_json::to_string(event)?))
    }

    fn run_guard_command_raw(
        &self,
        command_spec: &volicord_types::guard_manifest::GuardCommand,
        input: String,
    ) -> Result<support::binary_fixture::CapturedChildOutput, Box<dyn Error>> {
        let mut command = self.base_command(&command_spec.command, FUTURE_VERSION);
        command
            .env("VOLICORD_MANAGED_WRAPPER", "codex-record")
            .args(&command_spec.args);
        run_child(command, ChildStdin::WriteAndClose(input))
    }

    fn assert_failed_status(&self, check_id: &str, code: &str) -> Result<(), Box<dyn Error>> {
        let output = self.run_connection("status", FUTURE_VERSION, true)?;
        let report = assert_connection_report(&output, 1, "status", "failed")?;
        assert_check(&report, check_id, "failed", Some(code));
        assert!(!serde_json::to_string(&report)?.contains("unsupported_artifact"));
        Ok(())
    }

    fn assert_latest_runtime_finding(&self, code: &str) -> Result<(), Box<dyn Error>> {
        let connection_id = self.connection_id();
        let runtime = latest_current_managed_runtime_session(&self.runtime_home, &connection_id)?
            .ok_or("latest managed runtime session")?;
        let findings = diagnostic_occurrences_for_runtime_session(
            &self.runtime_home,
            &runtime.runtime_session_id,
        )?;
        assert!(
            findings
                .iter()
                .any(|finding| finding.data().code().as_str() == code),
            "missing {code} finding in {findings:?}"
        );
        Ok(())
    }

    fn registry_snapshot(&self) -> RegistryInspectionSnapshot {
        match inspect_runtime_home(&self.runtime_home).registry {
            DatabaseInspection::Present(snapshot) => snapshot,
            other => panic!("expected registry snapshot, got {other:?}"),
        }
    }

    fn agent_connection_record(
        &self,
        connection: &AgentConnectionInspectionRecord,
    ) -> AgentConnectionRecord {
        AgentConnectionRecord {
            connection_internal_id: connection.connection_internal_id.clone(),
            integration_instance_id: connection.integration_instance_id.clone(),
            host_kind: connection.host_kind.clone(),
            intent: connection.intent.clone(),
            host_scope: connection.host_scope.clone(),
            project_internal_id: connection.project_internal_id.clone(),
            server_name: connection.server_name.clone(),
            config_target: connection.config_target.clone(),
            mode: connection.mode.clone(),
            enabled: connection.enabled,
            managed_fingerprint: connection.managed_fingerprint.clone(),
            integration_generation: connection.integration_generation,
            verification_report_json: connection.verification_report_json.clone(),
            created_at: connection.created_at.clone(),
            updated_at: connection.updated_at.clone(),
            metadata_json: connection.metadata_json.clone(),
        }
    }

    fn connection_id(&self) -> String {
        self.registry_snapshot().agent_connections[0]
            .connection_internal_id
            .clone()
    }

    fn project_id(&self) -> String {
        self.registry_snapshot().projects[0].project_id.clone()
    }

    fn project_state_db_path(&self) -> PathBuf {
        self.runtime_home
            .join("projects")
            .join(self.project_id())
            .join("state.sqlite")
    }

    fn repository_snapshot(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        directory_contents(&self.repo_root)
    }

    fn content_snapshot(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        let mut snapshot = BTreeMap::new();
        for (prefix, root) in [
            (Path::new("runtime"), &self.runtime_home),
            (Path::new("repository"), &self.repo_root),
            (Path::new("codex"), &self.codex_home),
        ] {
            for (path, bytes) in directory_contents(root)? {
                snapshot.insert(prefix.join(path), bytes);
            }
        }
        Ok(snapshot)
    }

    fn diagnostic_registry_snapshot(&self) -> Result<DiagnosticRegistrySnapshot, Box<dyn Error>> {
        let registry = rusqlite::Connection::open(self.runtime_home.join("registry.sqlite"))?;
        let occurrence_count = registry.query_row(
            "SELECT COUNT(*) FROM diagnostic_findings WHERE lifecycle = 'occurrence'",
            [],
            |row| row.get(0),
        )?;
        let current_count = registry.query_row(
            "SELECT COUNT(*) FROM diagnostic_findings WHERE lifecycle = 'current_state'",
            [],
            |row| row.get(0),
        )?;
        let mut statement = registry.prepare(
            "SELECT finding_id, observed_at, resolved_at FROM diagnostic_findings WHERE lifecycle = 'current_state' ORDER BY finding_id",
        )?;
        let current_timestamps = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DiagnosticRegistrySnapshot {
            occurrence_count,
            current_count,
            current_timestamps,
        })
    }

    fn cli_preflight_session_count(&self) -> Result<i64, Box<dyn Error>> {
        let registry = rusqlite::Connection::open(self.runtime_home.join("registry.sqlite"))?;
        Ok(registry.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE session_source = 'cli_preflight'",
            [],
            |row| row.get(0),
        )?)
    }
}

struct LiveMcpChild {
    child: Child,
    stdin: Option<ProcessStdin>,
    stdout_lines: Receiver<io::Result<Vec<u8>>>,
    stdout: JoinHandle<io::Result<Vec<u8>>>,
    stderr: JoinHandle<io::Result<Vec<u8>>>,
    transcript: ExactToolCallTranscript,
}

impl LiveMcpChild {
    fn spawn(command: &mut Command) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("managed MCP stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("managed MCP stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("managed MCP stderr was not piped"))?;
        let (stdout_sender, stdout_lines) = mpsc::channel();
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout_lines,
            stdout: thread::spawn(move || {
                let mut reader = BufReader::new(stdout);
                let mut captured = Vec::new();
                loop {
                    let mut line = Vec::new();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => return Ok(captured),
                        Ok(_) => {
                            captured.extend_from_slice(&line);
                            if stdout_sender.send(Ok(line)).is_err() {
                                return Ok(captured);
                            }
                        }
                        Err(error) => {
                            let forwarded = io::Error::new(error.kind(), error.to_string());
                            let _ = stdout_sender.send(Err(forwarded));
                            return Err(error);
                        }
                    }
                }
            }),
            stderr: thread::spawn(move || read_to_end(stderr)),
            transcript: ExactToolCallTranscript::default(),
        })
    }

    fn write(&mut self, input: &str) -> io::Result<()> {
        self.transcript
            .capture_json_lines(input)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::other("managed MCP stdin is closed"))?;
        stdin.write_all(input.as_bytes())?;
        stdin.flush()
    }

    fn transcript(&self) -> &ExactToolCallTranscript {
        &self.transcript
    }

    fn read_responses(&mut self, expected: usize) -> io::Result<Vec<Value>> {
        let mut responses = Vec::with_capacity(expected);
        for _ in 0..expected {
            let line = match self.stdout_lines.recv_timeout(MCP_RESPONSE_TIMEOUT) {
                Ok(Ok(line)) => line,
                Ok(Err(error)) => {
                    self.stop_after_response_failure();
                    return Err(error);
                }
                Err(error) => {
                    self.stop_after_response_failure();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("managed MCP response was not received: {error}"),
                    ));
                }
            };
            match serde_json::from_slice(&line) {
                Ok(response) => responses.push(response),
                Err(error) => {
                    self.stop_after_response_failure();
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("managed MCP emitted invalid response JSON: {error}"),
                    ));
                }
            }
        }
        Ok(responses)
    }

    fn stop_after_response_failure(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn id(&self) -> u32 {
        self.child.id()
    }

    fn finish(mut self) -> io::Result<support::binary_fixture::CapturedChildOutput> {
        self.stdin.take();
        let status = self.child.wait()?;
        Ok(support::binary_fixture::CapturedChildOutput {
            status,
            stdout: join_reader(self.stdout)?,
            stderr: join_reader(self.stderr)?,
        })
    }
}

fn read_to_end(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(reader: JoinHandle<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    reader
        .join()
        .map_err(|_| io::Error::other("managed MCP reader thread panicked"))?
}

fn assert_current_guard_projection(
    fixture: &OperationalFixture,
    manifest: &GuardManifest,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(manifest.required_hook_phases, GuardHookPhase::REQUIRED);
    for phase in GuardHookPhase::REQUIRED {
        let runtime = manifest.runtime_commands.get(phase);
        let hash_index = runtime
            .args
            .iter()
            .position(|arg| arg == "--policy-hash")
            .expect("runtime policy hash argument");
        assert_eq!(
            runtime.args.get(hash_index + 1).map(String::as_str),
            Some(manifest.policy_hash.as_str())
        );
    }
    let policy: Value =
        serde_json::from_slice(&fs::read(fixture.repo_root.join(".volicord/policy.json"))?)?;
    for command in policy["host_hook"]["commands"]
        .as_object()
        .expect("policy commands")
        .values()
    {
        assert!(command["args"]
            .as_array()
            .expect("policy args")
            .iter()
            .all(|arg| arg != "--policy-hash"));
    }
    for file in &manifest.managed_files {
        assert!(file.path().is_file(), "missing {}", file.path().display());
        if file.ownership() == GuardManagedOwnership::ManagedScript {
            assert_eq!(file.executable_required(), Some(true));
        } else {
            assert_eq!(file.executable_required(), None);
        }
    }
    assert_platform_script_permissions(manifest);
    Ok(())
}

#[cfg(unix)]
fn assert_platform_script_permissions(manifest: &GuardManifest) {
    use std::os::unix::fs::PermissionsExt;

    for file in manifest
        .managed_files
        .iter()
        .filter(|file| file.ownership() == GuardManagedOwnership::ManagedScript)
    {
        let mode = fs::metadata(file.path())
            .unwrap_or_else(|error| panic!("failed to inspect {}: {error}", file.path().display()))
            .permissions()
            .mode();
        assert_ne!(
            mode & 0o100,
            0,
            "script is not executable: {}",
            file.path().display()
        );
    }
}

#[cfg(not(unix))]
fn assert_platform_script_permissions(_manifest: &GuardManifest) {}

#[cfg(unix)]
fn assert_status_reads_read_only_registry(
    fixture: &OperationalFixture,
    version: &str,
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    fn collect_permissions(
        path: &Path,
        output: &mut Vec<(PathBuf, fs::Permissions, bool)>,
    ) -> Result<(), Box<dyn Error>> {
        let metadata = fs::metadata(path)?;
        let is_dir = metadata.is_dir();
        output.push((path.to_path_buf(), metadata.permissions(), is_dir));
        if is_dir {
            for entry in fs::read_dir(path)? {
                collect_permissions(&entry?.path(), output)?;
            }
        }
        Ok(())
    }

    let mut original_permissions = Vec::new();
    collect_permissions(&fixture.runtime_home, &mut original_permissions)?;
    for (path, permissions, is_dir) in &original_permissions {
        let mut read_only = permissions.clone();
        read_only.set_mode(if *is_dir { 0o555 } else { 0o444 });
        fs::set_permissions(path, read_only)?;
    }
    let status_result = fixture.run_connection("status", version, true);
    for (path, permissions, _) in &original_permissions {
        fs::set_permissions(path, permissions.clone())?;
    }
    let status = status_result?;
    assert_connection_report(&status, 1, "status", "failed")?;
    Ok(())
}

#[cfg(not(unix))]
fn assert_status_reads_read_only_registry(
    fixture: &OperationalFixture,
    version: &str,
) -> Result<(), Box<dyn Error>> {
    let status = fixture.run_connection("status", version, true)?;
    assert_connection_report(&status, 1, "status", "failed")?;
    Ok(())
}

fn assert_connection_report(
    output: &Output,
    expected_exit: i32,
    operation: &str,
    status: &str,
) -> Result<Value, Box<dyn Error>> {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "unexpected exit; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["operation"], operation);
    assert_eq!(
        report["status"],
        status,
        "unexpected report status: {}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    assert_canonical_connection_command_shape(&report);
    Ok(report)
}

fn assert_connection_list_membership(
    output: &Output,
    status: &str,
) -> Result<Value, Box<dyn Error>> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "unexpected list exit; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected list stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        report
            .as_object()
            .expect("connection list report object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["connections", "generated_at", "limits"])
    );
    assert_eq!(
        report["connections"][0]["memberships"][0]["current_state"]["state"],
        "available"
    );
    assert_eq!(
        report["connections"][0]["memberships"][0]["current_state"]["status"],
        status
    );
    assert_eq!(
        report["connections"][0]["memberships"][0]["current_state"]["evaluated_at"],
        report["generated_at"]
    );
    Ok(report)
}

fn assert_canonical_connection_command_shape(report: &Value) {
    let object = report.as_object().expect("connection report object");
    let expected = BTreeSet::from([
        "activation_plan",
        "activation_state",
        "checks",
        "connection",
        "findings",
        "generated_at",
        "hook_activation_state",
        "limits",
        "operation",
        "operation_details",
        "root_cause_ids",
        "schema_version",
        "status",
    ]);
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected
    );
    for noncanonical_field in [
        "states",
        "verification",
        "verification_report",
        "verification_status",
        "host_hook",
        "summary_card",
        "primary_next_action",
        "host_gate",
        "approval",
        "configuration_health",
        "observation_health",
        "effective_health",
        "generated_config_verified",
        "disclosure",
    ] {
        assert!(
            !json_key_exists(report, noncanonical_field),
            "noncanonical connection-command field {noncanonical_field}"
        );
    }
    assert_eq!(report["schema_version"], 2);
    assert!(report["activation_state"].is_string());
    assert!(report["hook_activation_state"].is_string());
    assert!(report["operation_details"]["dry_run"].is_boolean());
    assert_eq!(report["limits"].as_array().map(Vec::len), Some(3));
}

fn assert_check(report: &Value, id: &str, status: &str, expected_code: Option<&str>) {
    let current_id = match id {
        "host_session" => "managed_session_health",
        "required_tools" | "tool_round_trip" => "managed_capability_proof",
        "guard_files" | "guard_observation" => "ambient_hook_coverage",
        _ => id,
    };
    let check = report["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == current_id)
        .unwrap_or_else(|| panic!("missing check {current_id}: {report}"));
    assert_eq!(check["status"], status, "unexpected check {id}: {check}");
    if let Some(expected) = expected_code {
        assert_eq!(
            check["code"], expected,
            "unexpected check code for {id}: {check}"
        );
    }
}

fn initialize_request(version: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "arbitrary-future-client", "version": version}
        }
    })
}

fn codex_compatibility_initialize_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": CODEX_COMPATIBILITY_REVISION,
            "capabilities": {},
            "clientInfo": {
                "name": "codex-mcp-client",
                "title": "Codex",
                "version": CODEX_COMPATIBILITY_VERSION,
            }
        }
    })
}

fn initialized_notification() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    })
}

fn tools_list_request() -> Value {
    json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
}

fn managed_tool_call(id: u64, name: &str, arguments: Value, session_id: &str) -> Value {
    managed_tool_call_in_turn(
        id,
        name,
        arguments,
        session_id,
        &format!("future.turn.{id}"),
    )
}

fn managed_tool_call_in_turn(
    id: u64,
    name: &str,
    arguments: Value,
    session_id: &str,
    turn_id: &str,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
            "_meta": {
                "threadId": NATIVE_THREAD,
                "x-codex-turn-metadata": {
                    "session_id": session_id,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": turn_id
                }
            }
        }
    })
}

fn live_mcp_call(
    child: &mut LiveMcpChild,
    id: u64,
    tool: AgentToolId,
    arguments: Value,
    session_id: &str,
    turn_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let response = live_mcp_raw_call(child, id, tool, arguments, session_id, turn_id)?;
    if response["result"]["isError"] != false {
        return Err(format!("{} returned an MCP error: {response}", tool.wire_name()).into());
    }
    response["result"]["structuredContent"]
        .as_object()
        .map(|_| response["result"]["structuredContent"].clone())
        .ok_or_else(|| {
            format!(
                "{} response omitted structured content: {response}",
                tool.wire_name()
            )
            .into()
        })
}

fn live_mcp_error(
    child: &mut LiveMcpChild,
    id: u64,
    tool: AgentToolId,
    arguments: Value,
    session_id: &str,
    turn_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let response = live_mcp_raw_call(child, id, tool, arguments, session_id, turn_id)?;
    if response["result"]["isError"] != true {
        return Err(format!(
            "{} unexpectedly returned MCP success: {response}",
            tool.wire_name()
        )
        .into());
    }
    response["result"]["structuredContent"]
        .as_object()
        .map(|_| response["result"]["structuredContent"].clone())
        .ok_or_else(|| {
            format!(
                "{} error response omitted structured content: {response}",
                tool.wire_name()
            )
            .into()
        })
}

fn live_mcp_raw_call(
    child: &mut LiveMcpChild,
    id: u64,
    tool: AgentToolId,
    arguments: Value,
    session_id: &str,
    turn_id: &str,
) -> Result<Value, Box<dyn Error>> {
    child.write(&json_lines(&[managed_tool_call_in_turn(
        id,
        tool.wire_name(),
        arguments,
        session_id,
        turn_id,
    )])?)?;
    Ok(child.read_responses(1)?.remove(0))
}

fn method_result(structured: &Value) -> &Value {
    structured.get("method_result").unwrap_or(structured)
}

fn required_transition_method(workflow: &Value) -> Option<&str> {
    workflow["transition_catalog"]["transitions"]
        .as_array()?
        .iter()
        .find(|transition| transition["role"] == "required")?["action_key"]["method"]
        .as_str()
}

fn required_transition_form_ref(structured: &Value) -> Result<String, Box<dyn Error>> {
    ["/action_form_catalog", "/presentation/action_form_catalog"]
        .into_iter()
        .find_map(|pointer| {
            let catalog = structured.pointer(pointer)?;
            let required_method = catalog["required_method"].as_str()?;
            catalog["forms"].as_array()?.iter().find_map(|form| {
                if form["method"].as_str() == Some(required_method) {
                    form["form_ref"].as_str().map(str::to_owned)
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| {
            format!(
                "required transition action-form catalog entry should include a form_ref in {structured}"
            )
                .into()
        })
}

fn method_action_form(structured: &Value, tool: AgentToolId) -> Result<&Value, Box<dyn Error>> {
    ["/action_form_catalog", "/presentation/action_form_catalog"]
        .into_iter()
        .find_map(|pointer| {
            structured
                .pointer(pointer)?
                .get("forms")?
                .as_array()?
                .iter()
                .find(|form| form["method"].as_str() == Some(tool.wire_name()))
        })
        .ok_or_else(|| {
            format!(
                "action-form catalog should include {} in {structured}",
                tool.wire_name()
            )
            .into()
        })
}

fn bound_action_arguments(
    structured: &Value,
    tool: AgentToolId,
    agent_arguments: Value,
) -> Result<Value, Box<dyn Error>> {
    let form = method_action_form(structured, tool)?;
    let mut arguments = form["fixed_arguments"].clone();
    merge_json(&mut arguments, agent_arguments);
    arguments
        .as_object_mut()
        .ok_or("action-form fixed arguments should be an object")?
        .insert("action_form_ref".to_owned(), form["form_ref"].clone());
    Ok(arguments)
}

fn merge_json(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                if let Some(current) = target.get_mut(&key) {
                    merge_json(current, value);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (target, source) => *target = source,
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, Box<dyn Error>> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} should be a string in {value}").into())
}

fn assert_typed_mutation_state(
    result: &Value,
    state_version: u64,
    mode: &str,
    work_phase: Option<&str>,
    workflow: &str,
) {
    assert_eq!(result["base"]["response_kind"], "result", "{result:#}");
    assert_eq!(result["base"]["state_version"], state_version, "{result:#}");
    assert_eq!(
        result["state"]["state_version"], state_version,
        "{result:#}"
    );
    assert_eq!(result["state"]["mode"], mode, "{result:#}");
    assert_eq!(
        result["state"]["work_phase"].as_str(),
        work_phase,
        "{result:#}"
    );
    assert_eq!(result["state"]["workflow"]["kind"], workflow, "{result:#}");
}

fn assert_compact_mutation_state(
    structured: &Value,
    state_version: u64,
    mode: &str,
    work_phase: Option<&str>,
    workflow: &str,
) {
    let compact = method_result(structured);
    let effect = compact.get("effect").unwrap_or(compact);
    assert_eq!(effect["effect_kind"], "core_committed", "{structured:#}");
    assert_eq!(effect["state_version"], state_version, "{structured:#}");
    assert_eq!(
        structured["authority_receipt"]["state_version"], state_version,
        "{structured:#}"
    );
    assert_eq!(structured["workflow"]["kind"], workflow, "{structured:#}");
    assert_eq!(
        structured["presentation"]["task_phase"]["mode"], mode,
        "{structured:#}"
    );
    assert_eq!(
        structured["presentation"]["task_phase"]["work_phase"].as_str(),
        work_phase,
        "{structured:#}"
    );
}

fn table_count(connection: &rusqlite::Connection, table: &str) -> Result<i64, Box<dyn Error>> {
    assert!(matches!(
        table,
        "change_units"
            | "authority_events"
            | "tool_invocations"
            | "runs"
            | "shaping_checkpoints"
            | "write_tickets"
            | "user_action_requests"
            | "user_action_resolutions"
            | "repository_observations"
            | "unrecorded_changes"
    ));
    Ok(
        connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

fn json_lines(messages: &[Value]) -> Result<String, serde_json::Error> {
    let mut input = String::new();
    for message in messages {
        input.push_str(&serde_json::to_string(message)?);
        input.push('\n');
    }
    Ok(input)
}

fn json_rpc_responses(bytes: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    String::from_utf8(bytes.to_vec())?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

fn assert_database_integrity(path: &Path) -> Result<(), Box<dyn Error>> {
    let connection = rusqlite::Connection::open(path)?;
    assert_eq!(
        connection.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))?,
        "ok",
        "SQLite integrity check failed for {}",
        path.display()
    );
    let foreign_key_failures = connection
        .prepare("PRAGMA foreign_key_check")?
        .query_map([], |_| Ok(()))?
        .collect::<Result<Vec<_>, _>>()?;
    assert!(
        foreign_key_failures.is_empty(),
        "SQLite foreign-key check failed for {}",
        path.display()
    );
    Ok(())
}

fn directory_contents(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    fn visit(
        root: &Path,
        current: &Path,
        output: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        if !current.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                visit(root, &path, output)?;
            } else {
                output.insert(path.strip_prefix(root)?.to_path_buf(), fs::read(path)?);
            }
        }
        Ok(())
    }

    let mut output = BTreeMap::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn json_key_exists(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_key_exists(value, key))
        }
        Value::Array(values) => values.iter().any(|value| json_key_exists(value, key)),
        _ => false,
    }
}

fn json_values_for_key<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    let mut values = Vec::new();
    match value {
        Value::Object(object) => {
            if let Some(value) = object.get(key) {
                values.push(value);
            }
            for value in object.values() {
                values.extend(json_values_for_key(value, key));
            }
        }
        Value::Array(items) => {
            for item in items {
                values.extend(json_values_for_key(item, key));
            }
        }
        _ => {}
    }
    values
}

fn schema_accepts_json_null(schema: &Value) -> bool {
    json_values_for_key(schema, "type")
        .into_iter()
        .any(|value| match value {
            Value::String(value_type) => value_type == "null",
            Value::Array(value_types) => value_types.iter().any(|value_type| value_type == "null"),
            _ => false,
        })
}

fn toml_entry_string_array(
    table: &toml_edit::Table,
    key: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let values = table[key]
        .as_array()
        .ok_or_else(|| format!("managed Codex {key} should be an array"))?;
    Ok(values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("managed Codex {key} should contain strings"))
        })
        .collect::<Result<Vec<_>, _>>()?)
}

#[cfg(windows)]
fn copy_required_windows_environment(command: &mut Command) {
    for name in ["SystemRoot", "WINDIR", "PATHEXT", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}
