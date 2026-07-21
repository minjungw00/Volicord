#![forbid(unsafe_code)]

mod support;

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    ffi::OsStr,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin as ProcessStdin, Command, Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use support::binary_fixture::{run_child, ChildStdin};
use support::json::adapter_tool_response;
use toml_edit::DocumentMut;
use volicord_mcp::{
    ManagedMcpInvocationPurpose, ManagedMcpLaunchSpec, ManagedMcpMaterializationInput,
    ManagedMcpWorkingDirectory, ADAPTER_UTILITY_TOOL_NAMES, PUBLIC_METHOD_TOOL_NAMES,
    READ_ONLY_METHOD_TOOL_NAMES, VOLICORD_HOME_ENV,
};
use volicord_store::agent_connections::{agent_connection_record, AgentConnectionRecord};
use volicord_store::guards::{
    agent_session, agent_session_matches_current_integration,
    current_project_agent_session_coordinates,
};
use volicord_store::inspection::{
    inspect_runtime_home, AgentConnectionInspectionRecord, DatabaseInspection,
    RegistryInspectionSnapshot,
};
use volicord_store::operational_sessions::{
    connection_integration_revision, current_managed_runtime_sessions,
    latest_current_managed_runtime_session, start_mcp_runtime_session, McpRuntimeSessionStart,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    guard_manifest_from_json, GuardHookPhase, GuardManagedOwnership, GuardManifest,
    McpRuntimeSessionSource,
};

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
            Ok("early_stdio_exit") if args.iter().any(|arg| arg == "--check") => {
                let connection_id = args
                    .windows(2)
                    .find(|pair| pair[0] == "--connection")
                    .and_then(|pair| pair[1].to_str())
                    .expect("fixture preflight connection ID");
                println!(
                    "configuration: valid\ntransport: stdio\nconnection_id: {connection_id}\nmode: workflow\nenabled: true\nregistry_read: passed\nproject_state_read: passed\nproject_state_write: passed\neffective_tool_mode: workflow\ntools_list_schema_validation: passed"
                );
                return;
            }
            Ok("early_stdio_exit") if args.iter().any(|arg| arg == "--stdio") => {
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
    codex_2025_06_18_compatibility_records_managed_runtime_facts()?;
    managed_launch_contracts_survive_filtered_environments()?;
    fresh_operation_version_transition_and_read_only_status()?;
    connection_mode_transition_rebinds_guard_revision()?;
    connection_mode_preflight_failure_preserves_connection()?;
    connection_removal_after_operational_observations()?;
    drift_verification_preserves_owned_configuration_and_removal()?;
    dry_run_has_no_mutation()?;
    protocol_failures_are_authoritative()?;
    local_process_and_configuration_failures_are_structured()?;
    guard_failures_are_current_and_structured()?;
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
        mcp_details["self_test"]["production_supported_revisions"],
        json!([
            "2024-10-07",
            "2024-11-05",
            "2025-03-26",
            "2025-06-18",
            "2025-11-25"
        ])
    );
    assert!(mcp_details["self_test"]["conformance"]
        .as_array()
        .is_some_and(|probes| probes.len() == 5
            && probes.iter().all(|probe| {
                probe["status"] == "passed"
                    && probe["requested_revision"] == probe["negotiated_revision"]
                    && probe["pinned_schema_validated"] == true
                    && probe["safe_read_only_tool_completed"] == true
                    && probe["shutdown_completed"] == true
            })));
    assert_eq!(
        mcp_details["self_test"]["host_compatibility_profiles"],
        json!(["codex"])
    );
    let codex_probe = &mcp_details["self_test"]["host_compatibility"][0];
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
                "volicord.list_projects",
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
    let expected_tools = PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
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
    assert_eq!(session.client_name.as_deref(), Some("codex-mcp-client"));
    assert_eq!(
        session.client_version.as_deref(),
        Some(CODEX_COMPATIBILITY_VERSION)
    );
    assert_eq!(
        session.negotiated_protocol_version.as_deref(),
        Some(CODEX_COMPATIBILITY_REVISION)
    );
    assert!(session.initialize_completed_at.is_some());
    assert!(session.initialized_notification_at.is_some());
    assert!(session.tools_list_observed_at.is_some());
    assert_eq!(session.required_tools_present, Some(true));
    assert!(session.last_safe_read_only_tool_call_at.is_some());
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
        assert!(partial.last_safe_read_only_tool_call_at.is_none());

        let partial_status = fixture.run_connection("status", FUTURE_VERSION, true)?;
        let partial_report =
            assert_connection_report(&partial_status, 0, "status", "action_required")?;
        assert_check(&partial_report, "host_session", "pending", None);
        assert_check(&partial_report, "required_tools", "pending", None);
        assert_check(&partial_report, "tool_round_trip", "pending", None);

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
                        && session.last_safe_read_only_tool_call_at.is_some()
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
    assert_eq!(no_op["result"]["kind"], "mode_transition");
    assert_eq!(no_op["result"]["changed"], false);
    assert_eq!(no_op["actions"], json!([]));
    assert_eq!(
        no_op["result"]["previous_integration_revision"],
        no_op["result"]["current_integration_revision"]
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
        reused_native_session,
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
    assert_eq!(read_only_report["result"]["kind"], "mode_transition");
    assert_eq!(read_only_report["result"]["changed"], true);
    assert_ne!(
        read_only_report["result"]["previous_integration_revision"],
        read_only_report["result"]["current_integration_revision"]
    );
    assert_eq!(
        read_only_report["result"]["rebound_guard_installation_ids"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        read_only_report["actions"].as_array().map(Vec::len),
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
        read_only_report["result"]["current_integration_revision"]
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
    assert_eq!(dry_run["dry_run"], true);
    assert_eq!(dry_run["connection"]["mode"], "read_only");
    assert_eq!(fixture.registry_snapshot(), registry_before_dry_run);
    assert_eq!(fixture.repository_snapshot()?, repository_before_dry_run);
    assert_eq!(fs::read(&config_target)?, config_before_dry_run);

    let pending = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let pending = assert_connection_report(&pending, 0, "status", "action_required")?;
    assert_check(&pending, "guard_files", "passed", None);
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
        reused_native_session,
    )?
    .session_id;
    assert_ne!(read_only_session_id, workflow_session_id);
    assert_unbound_agent_session(&fixture, &read_only_session_id)?;
    fixture.run_successful_managed_mcp(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
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
    assert_check(&pending, "guard_files", "passed", None);
    assert_check(&pending, "guard_observation", "pending", None);

    let workflow_tools = fixture.run_managed_tools_list_names(&connection_id)?;
    assert!(workflow_tools.contains(&"volicord.intake".to_owned()));
    fixture.run_current_guard_phases(&current_workflow_manifest, reused_native_session)?;
    let current_workflow_session_id = current_project_agent_session_coordinates(
        &fixture.runtime_home,
        &project_id,
        &connection_id,
        Some(current_workflow_manifest.guard_installation_id.as_str()),
        reused_native_session,
    )?
    .session_id;
    assert_ne!(current_workflow_session_id, read_only_session_id);
    assert_ne!(current_workflow_session_id, workflow_session_id);
    assert_unbound_agent_session(&fixture, &current_workflow_session_id)?;
    fixture.run_successful_managed_mcp(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        reused_native_session,
    )?;
    assert_connection_report(
        &fixture.run_connection("verify", FUTURE_VERSION, true)?,
        0,
        "verify",
        "complete",
    )?;
    let project_state = rusqlite::Connection::open(fixture.project_state_db_path())?;
    let revision_scoped_rows: i64 = project_state.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE host_session_id = ?1",
        [reused_native_session],
        |row| row.get(0),
    )?;
    assert_eq!(revision_scoped_rows, 3);

    let removed = fixture.run_connection("remove", FUTURE_VERSION, true)?;
    assert_eq!(removed.status.code(), Some(0));
    assert!(removed.stderr.is_empty());
    let removed: Value = serde_json::from_slice(&removed.stdout)?;
    assert_eq!(removed["result"]["connection_removed"], true);
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
    let session = agent_session(&fixture.runtime_home, &fixture.project_id(), session_id)?
        .expect("current Guard observation must create its revision-scoped Agent Session");
    assert!(session.runtime_session_id.is_none());
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
        reused_native_session,
    )?
    .session_id;
    let repository_before = fixture.repository_snapshot()?;
    let project_state_path = fixture.project_state_db_path();
    let project_state = rusqlite::Connection::open(&project_state_path)?;
    let agent_sessions_before: i64 =
        project_state.query_row("SELECT COUNT(*) FROM agent_sessions", [], |row| row.get(0))?;
    let guard_events_before: i64 =
        project_state.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?;
    assert!(agent_sessions_before > 0);
    assert!(guard_events_before > 0);
    drop(project_state);

    let output = fixture.run_connection("remove", FUTURE_VERSION, true)?;

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["result"]["membership_removed"], true);
    assert_eq!(report["result"]["connection_removed"], true);
    assert_eq!(report["result"]["remaining_project_count"], 0);
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
        project_state.query_row("SELECT COUNT(*) FROM agent_sessions", [], |row| row.get(0))?;
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
        reused_native_session,
    )?
    .session_id;
    assert_ne!(recreated_session_id, historical_session_id);
    assert!(agent_session(&fixture.runtime_home, &project_id, &historical_session_id,)?.is_some());
    let project_state = rusqlite::Connection::open(fixture.project_state_db_path())?;
    let recreated_rows: i64 = project_state.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE host_session_id = ?1",
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
    let applied_mcp_dir = fixture._temporary_root.path().join("applied-mcp");
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
    assert_eq!(repair["result"], json!({"kind": "setup", "applied": true}));
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
        native_session,
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

    let alternate_mcp_dir = fixture._temporary_root.path().join("desired-mcp");
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
    assert_check(&report, "guard_files", "passed", None);
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
    assert_eq!(removed["result"]["membership_removed"], true);
    assert_eq!(removed["result"]["connection_removed"], true);
    assert!(fixture.registry_snapshot().agent_connections.is_empty());
    assert!(!fs::read_to_string(config_target)
        .unwrap_or_default()
        .contains("mcp_servers.volicord"));
    Ok(())
}

fn fresh_operation_version_transition_and_read_only_status() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-host-complete")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    let init_report = assert_connection_report(&init, 0, "init", "action_required")?;
    assert_eq!(
        init_report["result"],
        json!({"kind": "setup", "applied": true})
    );
    assert_check(&init_report, "managed_config", "passed", None);
    assert_check(&init_report, "host_executable", "passed", None);
    assert_check(&init_report, "mcp_server", "passed", None);
    assert_check(&init_report, "host_session", "pending", None);
    assert_check(&init_report, "required_tools", "pending", None);
    assert_check(&init_report, "tool_round_trip", "pending", None);
    assert_check(&init_report, "guard_observation", "pending", None);
    assert!(init_report["actions"]
        .as_array()
        .is_some_and(|actions| !actions.is_empty()));

    let snapshot = fixture.registry_snapshot();
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.agent_connections.len(), 1);
    assert_eq!(snapshot.connection_projects.len(), 1);
    assert_eq!(snapshot.guard_installations.len(), 1);
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let project_id = snapshot.projects[0].project_id.clone();
    let manifest = guard_manifest_from_json(&snapshot.guard_installations[0].manifest_json)?;
    assert_current_guard_projection(&fixture, &manifest)?;

    let abandoned = start_mcp_runtime_session(
        &fixture.runtime_home,
        McpRuntimeSessionStart {
            connection_internal_id: connection_id.clone(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: Some(FUTURE_VERSION.to_owned()),
            process_id: 4242,
            process_started_at: "2000-01-01T00:00:00Z".to_owned(),
        },
    )?;
    assert!(abandoned.terminal_protocol_failure_code.is_none());
    assert!(abandoned.graceful_close_at.is_none());

    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        NATIVE_SESSION_999,
        &manifest,
    )?;

    let complete = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let complete_report = assert_connection_report(&complete, 0, "status", "complete")?;
    for check_id in [
        "guard_files",
        "guard_observation",
        "host_session",
        "required_tools",
        "tool_round_trip",
    ] {
        assert_check(&complete_report, check_id, "passed", None);
    }
    assert_eq!(complete_report["actions"], json!([]));
    assert_canonical_connection_command_shape(&complete_report);

    let before_status = fixture.content_snapshot()?;
    let repeated = fixture.run_connection("status", FUTURE_VERSION, true)?;
    assert_connection_report(&repeated, 0, "status", "complete")?;
    let after_status = fixture.content_snapshot()?;
    assert_eq!(after_status, before_status, "connection status wrote state");

    let human = fixture.run_connection("status", FUTURE_VERSION, false)?;
    assert_eq!(human.status.code(), Some(0));
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout)?;
    assert!(human.starts_with("Codex connection is ready.\n\n"));
    assert!(human.contains(&format!("Repository: {}\n", fixture.repo_root.display())));
    assert!(human.contains("Mode: workflow\nChecks: "));
    for check in complete_report["checks"].as_array().expect("checks") {
        assert!(!human.contains(check["id"].as_str().expect("check id")));
    }

    let verbose = fixture.run_connection_verbose("status", FUTURE_VERSION)?;
    assert_eq!(verbose.status.code(), Some(0));
    assert!(verbose.stderr.is_empty());
    let verbose = String::from_utf8(verbose.stdout)?;
    assert!(verbose.starts_with("Codex connection is ready.\n\nConnection\n"));
    assert!(verbose.contains("\n\nSummary\n  Status: complete\n"));
    for check in complete_report["checks"].as_array().expect("checks") {
        assert!(verbose.contains(check["summary"].as_str().expect("check summary")));
    }
    assert!(verbose.contains("    Tools returned:"));
    assert!(verbose.contains("    Designated read-only tool: volicord.list_projects"));
    assert!(!verbose.contains("Details: {"));
    assert!(!verbose.contains("\":["));

    let changed_version = fixture.run_connection("verify", NEXT_FUTURE_VERSION, true)?;
    let changed_report =
        assert_connection_report(&changed_version, 0, "verify", "action_required")?;
    for (check_id, code) in [
        ("host_session", "host_version_observation_stale"),
        ("required_tools", "required_tools_observation_stale"),
        ("tool_round_trip", "tool_round_trip_observation_stale"),
    ] {
        assert_check(&changed_report, check_id, "pending", Some(code));
    }
    assert!(changed_report["actions"].as_array().is_some_and(|actions| {
        actions.iter().any(|action| {
            let instruction = action["instruction"].as_str().unwrap_or_default();
            instruction.contains("Codex") || instruction.contains("Volicord")
        })
    }));

    fixture.run_successful_managed_mcp(
        &connection_id,
        &project_id,
        NEXT_FUTURE_VERSION,
        NATIVE_SESSION_1000,
    )?;
    fixture.run_current_guard_phases(&manifest, NATIVE_SESSION_1000)?;
    let project_state = rusqlite::Connection::open(fixture.project_state_db_path())?;
    let runtime_after_guard: Option<String> = project_state.query_row(
        "SELECT runtime_session_id FROM agent_sessions WHERE host_session_id = ?1",
        [NATIVE_SESSION_1000],
        |row| row.get(0),
    )?;
    assert!(runtime_after_guard.is_some());
    drop(project_state);
    let completed_again = fixture.run_connection("status", NEXT_FUTURE_VERSION, true)?;
    assert_connection_report(&completed_again, 0, "status", "complete")?;

    let wrapper = fixture.repo_root.join(".codex/hooks/volicord-pre-tool.sh");
    fs::write(&wrapper, "malformed current wrapper\n")?;
    let tampered = fixture.run_connection("status", NEXT_FUTURE_VERSION, true)?;
    let tampered_report = assert_connection_report(&tampered, 1, "status", "failed")?;
    assert_check(
        &tampered_report,
        "guard_files",
        "failed",
        Some("guard_files_failed"),
    );
    Ok(())
}

fn dry_run_has_no_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-host-dry-run")?;
    let repo_before = fixture.repository_snapshot()?;
    assert!(!fixture.runtime_home.exists());
    let output = fixture.run_init(FUTURE_VERSION, None, true)?;
    let report = assert_connection_report(&output, 0, "init", "action_required")?;
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["result"], json!({"kind": "setup", "applied": false}));
    assert!(report["planned_changes"].is_array());
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
    initialize.assert_failed_status("host_session", "host_session_initialize_failed")?;

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
    tools_list.assert_failed_status("required_tools", "required_tools_invalid")?;

    let safe_call = OperationalFixture::initialized("operational-safe-call-failure")?;
    safe_call.run_safe_tool_storage_failure()?;
    safe_call.assert_failed_status("tool_round_trip", "tool_round_trip_failed")?;

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
    missing_tools.assert_failed_status("required_tools", "required_tools_missing")?;
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
    assert_eq!(report["result"], json!({"kind": "setup", "applied": true}));
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
    assert_eq!(report["result"], json!({"kind": "setup", "applied": true}));
    assert_check(
        &report,
        "mcp_server",
        "failed",
        Some("mcp_server_initialize_failed"),
    );
    let failure = report["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|check| check["id"] == "mcp_server"))
        .and_then(|check| check.pointer("/details/self_test/failure"))
        .ok_or("MCP early-exit diagnostic projection should be present")?;
    assert_eq!(failure["kind"], "exited_before_response");
    assert_eq!(failure["stage"], "initialize");
    assert_eq!(failure["exit_code"], 23);
    assert_eq!(failure["stderr"]["truncated"], true);
    assert_eq!(failure["stderr"]["omitted_bytes"], 1024);
    assert!(failure["stderr"]["text"]
        .as_str()
        .is_some_and(|text| text.ends_with("...[stderr truncated; 1024 bytes omitted]")));
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

    let command = manifest.runtime_commands.get(GuardHookPhase::PreTool);
    let malformed_event = json!({
        "session_id": "future.session.guard.failure",
        "turn_id": "future.turn.guard.malformed",
        "tool_name": "Read",
        "tool_input": {"path": fixture.repo_root.join("README.md")}
    });
    let failed_hook = fixture.run_guard_command(command, &malformed_event)?;
    assert!(!failed_hook.status.success());

    let status = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&status, 1, "status", "failed")?;
    assert_check(
        &report,
        "guard_observation",
        "failed",
        Some("guard_observation_failed"),
    );
    Ok(())
}

struct OperationalFixture {
    _temporary_root: TempRuntimeHome,
    runtime_home: PathBuf,
    codex_home: PathBuf,
    user_home: PathBuf,
    path_dir: PathBuf,
    repo_root: PathBuf,
    shared: bool,
}

impl OperationalFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        Self::with_scope(prefix, false)
    }

    fn with_scope(prefix: &str, shared: bool) -> Result<Self, Box<dyn Error>> {
        let temporary_root = TempRuntimeHome::new(prefix)?;
        let runtime_home = temporary_root.path().join("runtime-home");
        let codex_home = temporary_root.path().join("codex-home");
        let user_home = temporary_root.path().join("user-home");
        let path_dir = temporary_root.path().join("path");
        let repo_root = temporary_root.path().join("product-repository");
        for directory in [&codex_home, &user_home, &path_dir, &repo_root] {
            fs::create_dir_all(directory)?;
        }
        fs::create_dir(repo_root.join(".git"))?;
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
        let directory = self._temporary_root.path().join("mcp-fixture");
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
        command
            .env_clear()
            .env_remove("WSL_DISTRO_NAME")
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env("PATH", &self.path_dir)
            .env(CODEX_VERSION_ENV, version)
            .current_dir(&self.repo_root);
        #[cfg(windows)]
        copy_required_windows_environment(&mut command);
        command
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
                    .path()
                    .join("ambient-decoy-runtime-home"),
            )
            .env("VOLICORD_MCP_LAUNCH", "ambient-decoy-launch")
            .env("VOLICORD_MCP_HOST", "ambient-decoy-host")
            .env("VOLICORD_MCP_CONNECTION_ID", "ambient-decoy-connection")
            .env("VOLICORD_MCP_PROJECT_ID", "ambient-decoy-project")
            .env("VOLICORD_MCP_VERIFICATION", "ambient-decoy-verification")
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
                managed_tool_call(3, "volicord.list_projects", json!({}), native_session),
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
            "workflow" => PUBLIC_METHOD_TOOL_NAMES
                .iter()
                .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
                .copied()
                .collect::<Vec<_>>(),
            "read_only" => READ_ONLY_METHOD_TOOL_NAMES
                .iter()
                .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
                .copied()
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
        child.write(&json_lines(&[
            initialize_request(version),
            initialized_notification(),
            tools_list_request(),
        ])?)?;
        let started = Instant::now();
        loop {
            if latest_current_managed_runtime_session(&self.runtime_home, connection_id)?
                .is_some_and(|session| session.tools_list_observed_at.is_some())
            {
                break;
            }
            if started.elapsed() >= Duration::from_secs(10) {
                return Err("managed MCP tools/list was not recorded before timeout".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.run_current_guard_phases(manifest, native_session)?;
        let current_session_id = current_project_agent_session_coordinates(
            &self.runtime_home,
            project_id,
            connection_id,
            Some(manifest.guard_installation_id.as_str()),
            native_session,
        )?
        .session_id;
        let project_state = rusqlite::Connection::open(self.project_state_db_path())?;
        let unbound_runtime: Option<String> = project_state.query_row(
            "SELECT runtime_session_id FROM agent_sessions WHERE session_id = ?1",
            [&current_session_id],
            |row| row.get(0),
        )?;
        assert!(unbound_runtime.is_none());
        let guard_history_before: (i64, i64) = (
            project_state.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?,
            project_state
                .query_row("SELECT COUNT(*) FROM prompt_captures", [], |row| row.get(0))?,
        );
        assert!(guard_history_before.0 > 0);
        assert!(guard_history_before.1 > 0);
        drop(project_state);

        child.write(&json_lines(&[managed_tool_call(
            3,
            "volicord.list_projects",
            json!({}),
            native_session,
        )])?)?;
        let started = Instant::now();
        loop {
            if latest_current_managed_runtime_session(&self.runtime_home, connection_id)?
                .is_some_and(|session| session.last_safe_read_only_tool_call_at.is_some())
            {
                break;
            }
            if started.elapsed() >= Duration::from_secs(10) {
                return Err("managed MCP safe round trip was not recorded before timeout".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.finish()?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let responses = json_rpc_responses(&output.stdout)?;
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[2]["result"]["isError"], false);
        let project_state = rusqlite::Connection::open(self.project_state_db_path())?;
        let bound_runtime: Option<String> = project_state.query_row(
            "SELECT runtime_session_id FROM agent_sessions WHERE session_id = ?1",
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
            guard_history_before.0
        );
        assert_eq!(
            project_state.query_row("SELECT COUNT(*) FROM prompt_captures", [], |row| row
                .get::<_, i64>(0))?,
            guard_history_before.1
        );
        Ok(())
    }

    fn run_safe_tool_storage_failure(&self) -> Result<(), Box<dyn Error>> {
        let connection_id = self.connection_id();
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
        let complete_cli_count: i64 = registry.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE connection_internal_id = ?1 AND session_source = 'cli_preflight' AND initialize_completed_at IS NOT NULL AND initialized_notification_at IS NOT NULL AND tools_list_observed_at IS NOT NULL AND required_tools_present = 1 AND last_safe_read_only_tool_call_at IS NOT NULL",
            [connection_id],
            |row| row.get(0),
        )?;
        assert_eq!(managed_count, 0);
        assert!(cli_count >= 1);
        assert!(complete_cli_count >= 1);
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
                    "session_id": native_session,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": "future.turn.pre",
                    "tool_name": "Read",
                    "tool_input": {"path": self.repo_root.join("README.md")}
                }),
            ),
            (
                GuardHookPhase::PostTool,
                json!({
                    "session_id": native_session,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": "future.turn.post",
                    "tool_name": "Read",
                    "tool_input": {"path": self.repo_root.join("README.md")},
                    "tool_response": {"success": true}
                }),
            ),
            (
                GuardHookPhase::PromptCapture,
                json!({
                    "session_id": native_session,
                    "thread_id": NATIVE_THREAD,
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
        command_spec: &volicord_types::GuardCommand,
        event: &Value,
    ) -> Result<support::binary_fixture::CapturedChildOutput, Box<dyn Error>> {
        let mut command = self.base_command(&command_spec.command, FUTURE_VERSION);
        command
            .env("VOLICORD_MANAGED_WRAPPER", "codex-record")
            .args(&command_spec.args);
        run_child(
            command,
            ChildStdin::WriteAndClose(format!("{}\n", serde_json::to_string(event)?)),
        )
    }

    fn assert_failed_status(&self, check_id: &str, code: &str) -> Result<(), Box<dyn Error>> {
        let output = self.run_connection("status", FUTURE_VERSION, true)?;
        let report = assert_connection_report(&output, 1, "status", "failed")?;
        assert_check(&report, check_id, "failed", Some(code));
        assert!(!serde_json::to_string(&report)?.contains("unsupported_artifact"));
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
}

struct LiveMcpChild {
    child: Child,
    stdin: Option<ProcessStdin>,
    stdout: JoinHandle<io::Result<Vec<u8>>>,
    stderr: JoinHandle<io::Result<Vec<u8>>>,
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
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout: thread::spawn(move || read_to_end(stdout)),
            stderr: thread::spawn(move || read_to_end(stderr)),
        })
    }

    fn write(&mut self, input: &str) -> io::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::other("managed MCP stdin is closed"))?;
        stdin.write_all(input.as_bytes())?;
        stdin.flush()
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

fn assert_canonical_connection_command_shape(report: &Value) {
    let object = report.as_object().expect("connection report object");
    let mut expected = BTreeSet::from([
        "actions",
        "checks",
        "connection",
        "dry_run",
        "limits",
        "operation",
        "runtime_home",
        "status",
    ]);
    if report.get("result").is_some() {
        expected.insert("result");
    }
    if report["dry_run"] == true {
        expected.insert("planned_changes");
    }
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
    assert_eq!(report["limits"].as_array().map(Vec::len), Some(1));
}

fn assert_check(report: &Value, id: &str, status: &str, expected_code: Option<&str>) {
    let check = report["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == id)
        .unwrap_or_else(|| panic!("missing check {id}: {report}"));
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
                    "turn_id": format!("future.turn.{id}")
                }
            }
        }
    })
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
