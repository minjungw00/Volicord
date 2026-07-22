#![forbid(unsafe_code)]

mod support;

use std::{
    collections::BTreeMap,
    error::Error,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use support::binary_fixture::create_git_repo;
use toml_edit::DocumentMut;
use volicord_cli::{
    cli::{
        CodexHost, ConnectionAddArgs, ConnectionArgs, ConnectionCommand, ConnectionMode,
        ConnectionModeArgs, ConnectionReportOutputArgs, InitArgs, PolicyArgs, PolicyCommand,
        PolicyValidateArgs, RecordProfile, RuntimeHomeArgs,
    },
    connection_command::{
        run_connection_command, run_init_command, ConnectionCommandError, ConnectionProcess,
        ConnectionProcessOutput, McpExchangeOutcome, McpExchangeProgress, McpProcessFailure,
        McpStage,
    },
    policy_command::run_policy_command,
};
use volicord_host_contract::{CodexMcpCorrelation, HostSessionId, HostThreadId, HostTurnId};
use volicord_mcp::{ManagedMcpInvocationPurpose, MaterializedManagedMcpLaunch};
use volicord_store::{
    agent_connections::{
        agent_connection_record, connection_metadata_contains_pending_host_cleanup_key,
        replace_agent_connection_verification_report_if_revision, AgentConnectionRecord,
        CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW,
    },
    core_pipeline::CoreProjectStore,
    guards::{agent_session, bind_agent_session_runtime, AgentSessionRuntimeBinding},
    inspection::{inspect_runtime_home, DatabaseInspection, RegistryInspectionSnapshot},
    operational_sessions::{
        connection_integration_revision, mcp_runtime_session, McpRuntimeSessionStart,
    },
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    canonical_json_sha256, guard_manifest_has_exact_current_shape,
    guard_manifest_managed_artifacts, guard_manifest_matches_owner_binding,
    GuardManifestOwnerBinding, McpRuntimeSessionSource, ProjectId,
};

const GENERATED_SHAPE_ERROR: &str =
    "generated Guard manifest does not match the current exact shape";

#[derive(Debug)]
struct FakeConnectionProcess {
    runtime_home: PathBuf,
    codex_home: PathBuf,
    isolated_path: PathBuf,
    current_exe: PathBuf,
    preflight_modes: Vec<String>,
    verification_modes: Vec<String>,
}

impl FakeConnectionProcess {
    fn new(fixture: &TempRuntimeHome) -> Result<Self, Box<dyn Error>> {
        Self::named(fixture, "fake")
    }

    fn named(fixture: &TempRuntimeHome, name: &str) -> Result<Self, Box<dyn Error>> {
        let codex_home = fixture.path().join(format!("{name}-codex-home"));
        let isolated_path = fixture.path().join(format!("{name}-isolated-path"));
        fs::create_dir_all(&codex_home)?;
        fs::create_dir_all(&isolated_path)?;
        Ok(Self {
            runtime_home: fixture.path().to_path_buf(),
            codex_home,
            isolated_path,
            current_exe: PathBuf::from(env!("CARGO_BIN_EXE_volicord")),
            preflight_modes: Vec::new(),
            verification_modes: Vec::new(),
        })
    }
}

impl ConnectionProcess for FakeConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        match name {
            "VOLICORD_HOME" => Some(self.runtime_home.clone().into_os_string()),
            "CODEX_HOME" => Some(self.codex_home.clone().into_os_string()),
            "HOME" => Some(
                self.codex_home
                    .parent()
                    .expect("fake Codex home has a parent")
                    .as_os_str()
                    .to_owned(),
            ),
            "PATH" => Some(self.isolated_path.clone().into_os_string()),
            _ => None,
        }
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        Ok(self.current_exe.clone())
    }

    fn run_preflight(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
    ) -> Result<ConnectionProcessOutput, McpProcessFailure> {
        let ManagedMcpInvocationPurpose::CliPreflightCheck { connection_id, .. } = launch.purpose()
        else {
            return Err(McpProcessFailure::protocol(
                McpStage::Startup,
                "fixture expected a CLI preflight invocation",
            ));
        };
        let runtime_home = &self.runtime_home;
        let mode = agent_connection_record(runtime_home, connection_id)
            .map_err(|error| McpProcessFailure::protocol(McpStage::Startup, error.to_string()))?
            .ok_or_else(|| {
                McpProcessFailure::protocol(
                    McpStage::Startup,
                    format!("missing Agent Connection {connection_id}"),
                )
            })?
            .mode;
        self.preflight_modes.push(mode.clone());
        Ok(ConnectionProcessOutput {
            process_id: 0,
            success: true,
            status_code: Some(0),
            stdout: format!(
                "configuration: valid\ntransport: stdio\nconnection_id: {connection_id}\nmode: {mode}\nenabled: true\nregistry_read: passed\nproject_state_read: passed\nproject_state_write: passed\neffective_tool_mode: {mode}\ntools_list_schema_validation: passed\n"
            ),
            stderr: String::new(),
        })
    }

    fn verify_mcp_stdio(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
        mode: &str,
    ) -> McpExchangeOutcome {
        if launch.purpose() != &ManagedMcpInvocationPurpose::CliStdioHandshake {
            return McpExchangeOutcome::failed(
                McpExchangeProgress::not_started(),
                McpProcessFailure::protocol(
                    McpStage::Startup,
                    "fixture expected a CLI stdio handshake invocation",
                ),
            );
        }
        self.verification_modes.push(mode.to_owned());
        McpExchangeOutcome::failed(
            McpExchangeProgress::not_started(),
            McpProcessFailure::protocol(
                McpStage::Initialize,
                "live MCP verification is intentionally unavailable in this fixture",
            ),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedIds {
    installation_id: String,
    project_id: String,
    project_internal_id: String,
    connection_id: String,
    guard_installation_id: String,
}

#[test]
fn fresh_record_init_persists_exact_owner_bound_manifest_and_managed_artifacts(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-fresh")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;

    assert_minimal_git_worktree(&repo_root)?;
    let output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&output);

    let snapshot = registry_snapshot(fixture.path());
    let ids = assert_single_owned_records(&snapshot);
    assert_eq!(snapshot.agent_connections[0].mode, CONNECTION_MODE_WORKFLOW);
    assert_eq!(snapshot.agent_connections[0].integration_generation, 0);
    assert_unavailable_codex_verification(&snapshot)?;
    assert_eq!(snapshot.projects[0].repo_root, repo_root);
    assert_eq!(snapshot.connection_projects[0].project_id, ids.project_id);

    let manifest = assert_exact_manifest_and_artifacts(&snapshot, &repo_root)?;
    assert_manifest_commands_bind_policy_hash(&manifest);
    assert_valid_policy_without_policy_hash_args(&repo_root)?;
    assert_codex_config_is_user_owned(&snapshot, &process, &repo_root)?;
    assert_authoritative_policy_matches_file(fixture.path(), &ids.project_id, &repo_root)?;
    Ok(())
}

#[test]
fn connection_add_new_targets_select_requested_mode_and_dry_run_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-connection-add-new-modes")?;
    let profile_repo = create_git_repo(&fixture, "profile-repo")?;
    let workflow_repo = create_git_repo(&fixture, "workflow-repo")?;
    let read_only_repo = create_git_repo(&fixture, "read-only-repo")?;
    let dry_run_repo = create_git_repo(&fixture, "dry-run-repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;

    assert_failed_init_with_recorded_guard(&run_record_init(&profile_repo, &mut process)?);

    let workflow = run_connection_add(&workflow_repo, true, false, false, &mut process)?;
    assert_eq!(workflow["operation"], "add");
    assert_eq!(workflow["connection"]["mode"], CONNECTION_MODE_WORKFLOW);
    assert_eq!(
        workflow["operation_details"]["result"],
        json!({"kind": "setup", "applied": true})
    );
    let workflow_id = workflow["connection"]["connection_id"]
        .as_str()
        .expect("workflow connection id");
    let after_workflow = registry_snapshot(fixture.path());
    let workflow_project_id = project_id_for_repo(&after_workflow, &workflow_repo)?;
    assert_eq!(
        after_workflow
            .agent_connections
            .iter()
            .find(|connection| connection.connection_internal_id == workflow_id)
            .expect("new workflow Connection")
            .mode,
        CONNECTION_MODE_WORKFLOW
    );
    assert_codex_config_is_shared(
        fixture.path(),
        &workflow_repo,
        workflow_id,
        &workflow_project_id,
    )?;

    let read_only = run_connection_add(&read_only_repo, true, true, false, &mut process)?;
    assert_eq!(read_only["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    let read_only_id = read_only["connection"]["connection_id"]
        .as_str()
        .expect("read-only connection id");
    let after_read_only = registry_snapshot(fixture.path());
    let read_only_connection = after_read_only
        .agent_connections
        .iter()
        .find(|connection| connection.connection_internal_id == read_only_id)
        .expect("new read-only Connection");
    assert_eq!(read_only_connection.mode, CONNECTION_MODE_READ_ONLY);
    assert_eq!(read_only_connection.integration_generation, 0);

    let runtime_before = directory_contents(fixture.path())?;
    let repo_before = directory_contents(&dry_run_repo)?;
    let codex_before = directory_contents(&process.codex_home)?;
    let dry_run = run_connection_add(&dry_run_repo, true, true, true, &mut process)?;
    assert_eq!(dry_run["operation_details"]["dry_run"], true);
    assert_eq!(dry_run["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    assert_eq!(directory_contents(fixture.path())?, runtime_before);
    assert_eq!(directory_contents(&dry_run_repo)?, repo_before);
    assert_eq!(directory_contents(&process.codex_home)?, codex_before);
    Ok(())
}

#[test]
fn record_init_repairs_missing_guard_installation_and_replays_exactly() -> Result<(), Box<dyn Error>>
{
    let fixture = TempRuntimeHome::new("cli-record-init-partial")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let unrelated_path = repo_root.join("product-notes.txt");
    let unrelated_content = "user-owned repository content\n";
    fs::write(&unrelated_path, unrelated_content)?;
    let mut process = FakeConnectionProcess::new(&fixture)?;

    let seeded_output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&seeded_output);
    let seeded_snapshot = registry_snapshot(fixture.path());
    let seeded_ids = assert_single_owned_records(&seeded_snapshot);
    assert_unavailable_codex_verification(&seeded_snapshot)?;
    let managed_bytes = managed_artifact_bytes(&seeded_snapshot)?;

    replace_agent_connection_verification_report_if_revision(
        fixture.path(),
        &seeded_ids.connection_id,
        &connection_integration_revision(
            &agent_connection_record(fixture.path(), &seeded_ids.connection_id)?
                .expect("seeded Agent Connection"),
        )?,
        None,
    )?;
    delete_guard_installation(fixture.path(), &seeded_ids.guard_installation_id)?;

    let partial = registry_snapshot(fixture.path());
    assert_eq!(partial.projects.len(), 1);
    assert_eq!(partial.agent_connections.len(), 1);
    assert_eq!(partial.connection_projects.len(), 1);
    assert!(partial.guard_installations.is_empty());
    assert!(partial.agent_connections[0]
        .verification_report_json
        .is_none());
    assert_authoritative_policy_matches_file(fixture.path(), &seeded_ids.project_id, &repo_root)?;
    assert_eq!(fs::read_to_string(&unrelated_path)?, unrelated_content);

    let repair_output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&repair_output);
    assert!(repair_output.get("planned_changes").is_none());
    let repaired = registry_snapshot(fixture.path());
    let repaired_ids = assert_single_owned_records(&repaired);
    assert_eq!(repaired_ids, seeded_ids);
    assert_unavailable_codex_verification(&repaired)?;
    let repaired_manifest = assert_exact_manifest_and_artifacts(&repaired, &repo_root)?;
    assert_manifest_commands_bind_policy_hash(&repaired_manifest);
    assert_eq!(managed_artifact_bytes(&repaired)?, managed_bytes);
    assert_eq!(fs::read_to_string(&unrelated_path)?, unrelated_content);

    let replay_output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&replay_output);
    assert!(replay_output.get("planned_changes").is_none());
    let replayed = registry_snapshot(fixture.path());
    let replayed_ids = assert_single_owned_records(&replayed);
    assert_eq!(replayed_ids, seeded_ids);
    assert_unavailable_codex_verification(&replayed)?;
    assert_eq!(
        replayed.guard_installations[0].manifest_json,
        repaired.guard_installations[0].manifest_json
    );
    let replayed_manifest = assert_exact_manifest_and_artifacts(&replayed, &repo_root)?;
    assert_manifest_commands_bind_policy_hash(&replayed_manifest);
    assert_valid_policy_without_policy_hash_args(&repo_root)?;
    assert_authoritative_policy_matches_file(fixture.path(), &seeded_ids.project_id, &repo_root)?;
    assert_eq!(managed_artifact_bytes(&replayed)?, managed_bytes);
    assert_eq!(fs::read_to_string(&unrelated_path)?, unrelated_content);
    Ok(())
}

#[test]
fn read_only_init_replay_dry_run_and_repairs_preserve_mode_generation_and_revision(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-read-only-repair")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let unrelated_repo_path = repo_root.join("product-notes.txt");
    let unrelated_repo_content = "user-owned repository content\n";
    fs::write(&unrelated_repo_path, unrelated_repo_content)?;
    let mut process = FakeConnectionProcess::new(&fixture)?;
    let unrelated_codex_content = "[unrelated]\nvalue = \"preserved\"\n";
    fs::write(
        process.codex_home.join("config.toml"),
        unrelated_codex_content,
    )?;

    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut process)?);
    let workflow = registry_snapshot(fixture.path());
    let ids = assert_single_owned_records(&workflow);
    assert_eq!(workflow.agent_connections[0].mode, CONNECTION_MODE_WORKFLOW);
    let transition = run_read_only_mode(&repo_root, &mut process)?;
    assert_eq!(transition["operation"], "mode");
    assert_eq!(
        transition["operation_details"]["result"]["kind"],
        "mode_transition"
    );
    assert_eq!(transition["operation_details"]["result"]["changed"], true);
    assert_eq!(transition["connection"]["mode"], CONNECTION_MODE_READ_ONLY);

    let read_only = registry_snapshot(fixture.path());
    let connection = &read_only.agent_connections[0];
    let generation = connection.integration_generation;
    let revision = inspected_connection_revision(connection)?;
    let manifest_json = read_only.guard_installations[0].manifest_json.clone();
    let managed_bytes = managed_artifact_bytes(&read_only)?;
    let config_target = PathBuf::from(&connection.config_target);
    let config_bytes = fs::read(&config_target)?;
    assert_eq!(connection.mode, CONNECTION_MODE_READ_ONLY);
    assert_eq!(generation, 1);
    assert_manifest_revision(&read_only, &revision)?;
    assert!(String::from_utf8_lossy(&config_bytes).contains(unrelated_codex_content.trim()));

    process.preflight_modes.clear();
    process.verification_modes.clear();
    let replay = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&replay);
    assert_eq!(replay["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    let replayed = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&replayed, generation, &revision)?;
    assert_eq!(replayed.guard_installations[0].manifest_json, manifest_json);
    assert_eq!(managed_artifact_bytes(&replayed)?, managed_bytes);
    assert_eq!(fs::read(&config_target)?, config_bytes);
    assert_eq!(
        fs::read_to_string(&unrelated_repo_path)?,
        unrelated_repo_content
    );
    assert_mode_expectations(&process);

    let runtime_before_dry_run = directory_contents(fixture.path())?;
    let repo_before_dry_run = directory_contents(&repo_root)?;
    let codex_before_dry_run = directory_contents(&process.codex_home)?;
    let dry_run = run_record_init_dry_run(&repo_root, &mut process)?;
    assert_eq!(dry_run["operation_details"]["dry_run"], true);
    assert_eq!(dry_run["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    assert_eq!(directory_contents(fixture.path())?, runtime_before_dry_run);
    assert_eq!(directory_contents(&repo_root)?, repo_before_dry_run);
    assert_eq!(
        directory_contents(&process.codex_home)?,
        codex_before_dry_run
    );

    delete_guard_installation(fixture.path(), &ids.guard_installation_id)?;
    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut process)?);
    let repaired_guard = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&repaired_guard, generation, &revision)?;
    assert_manifest_revision(&repaired_guard, &revision)?;
    assert_eq!(managed_artifact_bytes(&repaired_guard)?, managed_bytes);
    assert_mode_expectations(&process);

    let damaged_file = managed_script_path(&repaired_guard)?;
    fs::remove_file(&damaged_file)?;
    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut process)?);
    let repaired_file = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&repaired_file, generation, &revision)?;
    assert_manifest_revision(&repaired_file, &revision)?;
    assert_eq!(managed_artifact_bytes(&repaired_file)?, managed_bytes);
    assert_mode_expectations(&process);

    fs::write(&config_target, unrelated_codex_content)?;
    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut process)?);
    let repaired_config = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&repaired_config, generation, &revision)?;
    assert_manifest_revision(&repaired_config, &revision)?;
    let repaired_config_text = fs::read_to_string(&config_target)?;
    assert!(repaired_config_text.contains(unrelated_codex_content.trim()));
    assert!(repaired_config_text.contains(&ids.connection_id));
    assert_eq!(
        fs::read_to_string(&unrelated_repo_path)?,
        unrelated_repo_content
    );
    assert_mode_expectations(&process);
    Ok(())
}

#[test]
fn read_only_connection_add_replay_and_repairs_preserve_owner_revision(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-connection-add-read-only-repair")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let unrelated_repo_path = repo_root.join("product-notes.txt");
    let unrelated_repo_content = "user-owned repository content\n";
    fs::write(&unrelated_repo_path, unrelated_repo_content)?;
    let unrelated_codex_content = "[unrelated]\nvalue = \"preserved\"\n";
    let mut process = FakeConnectionProcess::new(&fixture)?;
    fs::write(
        process.codex_home.join("config.toml"),
        unrelated_codex_content,
    )?;

    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut process)?);
    let ids = assert_single_owned_records(&registry_snapshot(fixture.path()));
    run_read_only_mode(&repo_root, &mut process)?;
    let read_only = registry_snapshot(fixture.path());
    let connection = &read_only.agent_connections[0];
    let integration_instance_id = connection.integration_instance_id.clone();
    let generation = connection.integration_generation;
    let revision = inspected_connection_revision(connection)?;
    let manifest_json = read_only.guard_installations[0].manifest_json.clone();
    let managed_bytes = managed_artifact_bytes(&read_only)?;
    let config_target = PathBuf::from(&connection.config_target);
    let config_bytes = fs::read(&config_target)?;
    process.preflight_modes.clear();
    process.verification_modes.clear();

    let replay = run_connection_add(&repo_root, false, false, false, &mut process)?;
    assert_eq!(replay["operation"], "add");
    assert_eq!(replay["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    let replayed = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&replayed, generation, &revision)?;
    assert_eq!(
        replayed.agent_connections[0].integration_instance_id,
        integration_instance_id
    );
    assert_eq!(replayed.guard_installations[0].manifest_json, manifest_json);
    assert_eq!(managed_artifact_bytes(&replayed)?, managed_bytes);
    assert_eq!(fs::read(&config_target)?, config_bytes);
    assert_eq!(
        fs::read_to_string(&unrelated_repo_path)?,
        unrelated_repo_content
    );

    let explicit = run_connection_add(&repo_root, false, true, false, &mut process)?;
    assert_eq!(explicit["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    let explicit_replay = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&explicit_replay, generation, &revision)?;
    assert_eq!(
        explicit_replay.guard_installations[0].manifest_json,
        manifest_json
    );
    assert_eq!(managed_artifact_bytes(&explicit_replay)?, managed_bytes);

    let runtime_before_dry_run = directory_contents(fixture.path())?;
    let repo_before_dry_run = directory_contents(&repo_root)?;
    let codex_before_dry_run = directory_contents(&process.codex_home)?;
    let dry_run = run_connection_add(&repo_root, false, false, true, &mut process)?;
    assert_eq!(dry_run["operation_details"]["dry_run"], true);
    assert_eq!(dry_run["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    assert!(dry_run["operation_details"]["planned_changes"]
        .as_array()
        .is_some_and(Vec::is_empty));
    assert_eq!(directory_contents(fixture.path())?, runtime_before_dry_run);
    assert_eq!(directory_contents(&repo_root)?, repo_before_dry_run);
    assert_eq!(
        directory_contents(&process.codex_home)?,
        codex_before_dry_run
    );

    fs::write(&config_target, unrelated_codex_content)?;
    run_connection_add(&repo_root, false, false, false, &mut process)?;
    let repaired_config = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&repaired_config, generation, &revision)?;
    assert_manifest_revision(&repaired_config, &revision)?;
    let repaired_config_text = fs::read_to_string(&config_target)?;
    assert!(repaired_config_text.contains(unrelated_codex_content.trim()));
    assert!(repaired_config_text.contains(&ids.connection_id));

    delete_guard_installation(fixture.path(), &ids.guard_installation_id)?;
    run_connection_add(&repo_root, false, false, false, &mut process)?;
    let repaired_guard = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&repaired_guard, generation, &revision)?;
    assert_manifest_revision(&repaired_guard, &revision)?;
    assert_eq!(managed_artifact_bytes(&repaired_guard)?, managed_bytes);

    let damaged_file = managed_script_path(&repaired_guard)?;
    fs::remove_file(&damaged_file)?;
    run_connection_add(&repo_root, false, false, false, &mut process)?;
    let repaired_file = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&repaired_file, generation, &revision)?;
    assert_manifest_revision(&repaired_file, &revision)?;
    assert_eq!(managed_artifact_bytes(&repaired_file)?, managed_bytes);
    assert_eq!(
        fs::read_to_string(&unrelated_repo_path)?,
        unrelated_repo_content
    );
    assert_mode_expectations(&process);
    Ok(())
}

#[test]
fn connection_add_explicit_read_only_rejects_workflow_before_mutation() -> Result<(), Box<dyn Error>>
{
    let fixture = TempRuntimeHome::new("cli-connection-add-explicit-mode-conflict")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;

    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut process)?);
    let runtime_before = directory_contents(fixture.path())?;
    let repo_before = directory_contents(&repo_root)?;
    let codex_before = directory_contents(&process.codex_home)?;
    let error = run_connection_command(
        ConnectionArgs {
            command: ConnectionCommand::Add(ConnectionAddArgs {
                host: Some(CodexHost::Codex),
                repo: Some(repo_root.clone()),
                runtime_home: RuntimeHomeArgs::default(),
                shared: false,
                read_only: true,
                dry_run: false,
                output: ConnectionReportOutputArgs {
                    json: true,
                    verbose: false,
                },
            }),
        },
        &repo_root,
        &mut process,
    )
    .expect_err("workflow Connection must reject implicit add transition");
    let message = error.to_string();
    assert!(message.contains("CONNECTION_MODE_REQUEST_CONFLICT"));
    assert!(message.contains("matching current Agent Connection is `workflow`"));
    assert!(message.contains("use `volicord connection mode`"));
    assert_eq!(directory_contents(fixture.path())?, runtime_before);
    assert_eq!(directory_contents(&repo_root)?, repo_before);
    assert_eq!(directory_contents(&process.codex_home)?, codex_before);
    let after = registry_snapshot(fixture.path());
    assert_eq!(after.agent_connections[0].mode, CONNECTION_MODE_WORKFLOW);
    assert_eq!(after.agent_connections[0].integration_generation, 0);
    Ok(())
}

#[test]
fn multi_project_connection_add_replay_preserves_selected_mode_and_other_membership(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-read-only-multi-project")?;
    let first_repo = create_git_repo(&fixture, "repo-one")?;
    let second_repo = create_git_repo(&fixture, "repo-two")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;

    assert_failed_init_with_recorded_guard(&run_record_init(&first_repo, &mut process)?);
    assert_failed_init_with_recorded_guard(&run_record_init(&second_repo, &mut process)?);
    let workflow = registry_snapshot(fixture.path());
    assert_eq!(workflow.agent_connections.len(), 1);
    assert_eq!(workflow.connection_projects.len(), 2);
    assert_eq!(workflow.guard_installations.len(), 2);

    run_read_only_mode(&first_repo, &mut process)?;
    let before = registry_snapshot(fixture.path());
    let generation = before.agent_connections[0].integration_generation;
    let revision = inspected_connection_revision(&before.agent_connections[0])?;
    let memberships = before.connection_projects.clone();
    let manifests = before
        .guard_installations
        .iter()
        .map(|guard| (guard.project_id.clone(), guard.manifest_json.clone()))
        .collect::<BTreeMap<_, _>>();

    let replay = run_connection_add(&second_repo, false, false, false, &mut process)?;
    assert_eq!(replay["operation"], "add");
    assert_eq!(replay["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    let after = registry_snapshot(fixture.path());
    assert_read_only_revision_unchanged(&after, generation, &revision)?;
    assert_eq!(after.connection_projects, memberships);
    assert_eq!(
        after
            .guard_installations
            .iter()
            .map(|guard| (guard.project_id.clone(), guard.manifest_json.clone()))
            .collect::<BTreeMap<_, _>>(),
        manifests
    );
    Ok(())
}

#[test]
fn init_migration_retires_bound_project_state_from_a_multi_project_connection(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-bound-multi-project-migration")?;
    let selected_repo = create_git_repo(&fixture, "repo-selected")?;
    let retained_repo = create_git_repo(&fixture, "repo-retained")?;
    let mut prior_process = FakeConnectionProcess::named(&fixture, "prior")?;

    assert_failed_init_with_recorded_guard(&run_record_init(&selected_repo, &mut prior_process)?);
    assert_failed_init_with_recorded_guard(&run_record_init(&retained_repo, &mut prior_process)?);
    let before = registry_snapshot(fixture.path());
    assert_eq!(before.agent_connections.len(), 1);
    assert_eq!(before.connection_projects.len(), 2);
    let prior_connection = before.agent_connections[0].clone();
    let selected_project_id = project_id_for_repo(&before, &selected_repo)?;
    let retained_project_id = project_id_for_repo(&before, &retained_repo)?;
    let selected_guard_id = guard_id_for_project(
        &before,
        &prior_connection.connection_internal_id,
        &selected_project_id,
    )?;
    let retained_guard_id = guard_id_for_project(
        &before,
        &prior_connection.connection_internal_id,
        &retained_project_id,
    )?;
    let (runtime_id, selected_session_id) = seed_managed_project_session(
        fixture.path(),
        &prior_connection.connection_internal_id,
        &selected_project_id,
        &selected_guard_id,
        5001,
        "init.migration.selected",
    )?;
    seed_project_session_on_runtime(
        fixture.path(),
        &runtime_id,
        &prior_connection.connection_internal_id,
        &retained_project_id,
        &retained_guard_id,
        "init.migration.retained",
        "2026-07-19T00:00:02Z",
    )?;

    let mut replacement_process = FakeConnectionProcess::named(&fixture, "replacement")?;
    let migration = run_record_init_outcome(&selected_repo, &mut replacement_process)?;
    assert_failed_init_with_recorded_guard(&migration);
    assert!(migration["migration"].is_null());
    assert!(!migration["error"]
        .as_str()
        .unwrap_or_default()
        .contains("FOREIGN KEY"));

    let after = registry_snapshot(fixture.path());
    assert_eq!(after.agent_connections.len(), 2);
    assert_eq!(after.connection_projects.len(), 2);
    assert_eq!(after.guard_installations.len(), 2);
    assert_eq!(after.runtime_project_session_bindings.len(), 1);
    assert_eq!(
        after.runtime_project_session_bindings[0].connection_internal_id,
        prior_connection.connection_internal_id
    );
    assert_eq!(
        after.runtime_project_session_bindings[0].project_internal_id,
        retained_project_id
    );
    let retained_prior = after
        .agent_connections
        .iter()
        .find(|connection| {
            connection.connection_internal_id == prior_connection.connection_internal_id
        })
        .expect("prior multi-project Connection remains");
    assert!(retained_prior.enabled);
    assert_eq!(
        retained_prior.integration_instance_id,
        prior_connection.integration_instance_id
    );
    assert_eq!(
        retained_prior.integration_generation,
        prior_connection.integration_generation
    );
    assert!(mcp_runtime_session(fixture.path(), &runtime_id)?.is_some());
    assert!(agent_session(fixture.path(), &selected_project_id, &selected_session_id,)?.is_some());

    let replay = run_record_init(&selected_repo, &mut replacement_process)?;
    assert_failed_init_with_recorded_guard(&replay);
    let replayed = registry_snapshot(fixture.path());
    assert_eq!(replayed.agent_connections.len(), 2);
    assert_eq!(replayed.connection_projects.len(), 2);
    assert_eq!(replayed.guard_installations.len(), 2);
    assert_eq!(replayed.runtime_project_session_bindings.len(), 1);
    Ok(())
}

#[test]
fn init_migration_retains_bound_cleanup_inventory_until_host_cleanup_replay(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-bound-pending-cleanup")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut prior_process = FakeConnectionProcess::named(&fixture, "prior")?;
    assert_failed_init_with_recorded_guard(&run_record_init(&repo_root, &mut prior_process)?);
    let before = registry_snapshot(fixture.path());
    let prior_connection = before.agent_connections[0].clone();
    let project_id = project_id_for_repo(&before, &repo_root)?;
    let guard_id = guard_id_for_project(
        &before,
        &prior_connection.connection_internal_id,
        &project_id,
    )?;
    let (runtime_id, project_session_id) = seed_managed_project_session(
        fixture.path(),
        &prior_connection.connection_internal_id,
        &project_id,
        &guard_id,
        5002,
        "init.cleanup.pending",
    )?;
    let prior_config_target = PathBuf::from(&prior_connection.config_target);
    fs::write(&prior_config_target, "malformed = [\n")?;

    let mut replacement_process = FakeConnectionProcess::named(&fixture, "replacement")?;
    let failed_cleanup = run_record_init_outcome(&repo_root, &mut replacement_process)?;
    assert_eq!(failed_cleanup["operation"], "init");
    assert_eq!(failed_cleanup["status"], "failed");
    assert_eq!(
        failed_cleanup["operation_details"]["result"],
        json!({"kind": "setup", "applied": false}),
        "unexpected migration output: {failed_cleanup}"
    );
    let failure_details = &failed_cleanup["checks"][0]["details"];
    assert_eq!(failure_details["registry_transition_applied"], true);
    assert_eq!(failure_details["prior_host_cleanup_completed"], false);
    assert!(failure_details["prior_connections"]
        .as_array()
        .is_some_and(|connections| connections
            .iter()
            .any(|connection| { connection["disposition"] == "disabled_pending_host_cleanup" })));
    assert!(!failure_details["failure"]
        .as_str()
        .unwrap_or_default()
        .contains("FOREIGN KEY"));

    let pending = registry_snapshot(fixture.path());
    assert_eq!(pending.agent_connections.len(), 2);
    assert_eq!(pending.connection_projects.len(), 2);
    assert_eq!(pending.guard_installations.len(), 2);
    assert_eq!(pending.runtime_project_session_bindings.len(), 1);
    let pending_prior = pending
        .agent_connections
        .iter()
        .find(|connection| {
            connection.connection_internal_id == prior_connection.connection_internal_id
        })
        .expect("disabled prior Connection remains");
    assert!(!pending_prior.enabled);
    assert!(connection_metadata_contains_pending_host_cleanup_key(
        &pending_prior.metadata_json
    ));
    assert!(mcp_runtime_session(fixture.path(), &runtime_id)?.is_some());
    assert!(agent_session(fixture.path(), &project_id, &project_session_id)?.is_some());

    fs::remove_file(&prior_config_target)?;
    let cleanup_replay = run_record_init_outcome(&repo_root, &mut replacement_process)?;
    assert_failed_init_with_recorded_guard(&cleanup_replay);
    assert!(cleanup_replay["migration"].is_null());
    assert!(!cleanup_replay["error"]
        .as_str()
        .unwrap_or_default()
        .contains("FOREIGN KEY"));

    let cleaned = registry_snapshot(fixture.path());
    assert_eq!(cleaned.agent_connections.len(), 2);
    assert_eq!(
        cleaned.connection_projects.len(),
        1,
        "cleanup replay did not retire inventory: {cleanup_replay}"
    );
    assert_eq!(cleaned.guard_installations.len(), 1);
    assert!(cleaned.runtime_project_session_bindings.is_empty());
    let historical_prior = cleaned
        .agent_connections
        .iter()
        .find(|connection| {
            connection.connection_internal_id == prior_connection.connection_internal_id
        })
        .expect("zero-membership prior Connection remains as history");
    assert!(!historical_prior.enabled);
    assert!(!connection_metadata_contains_pending_host_cleanup_key(
        &historical_prior.metadata_json
    ));
    assert!(mcp_runtime_session(fixture.path(), &runtime_id)?.is_some());
    assert!(agent_session(fixture.path(), &project_id, &project_session_id)?.is_some());

    let replay = run_record_init(&repo_root, &mut replacement_process)?;
    assert_failed_init_with_recorded_guard(&replay);
    let replayed = registry_snapshot(fixture.path());
    assert_eq!(replayed.agent_connections.len(), 2);
    assert_eq!(replayed.connection_projects.len(), 1);
    assert_eq!(replayed.guard_installations.len(), 1);
    assert!(replayed.runtime_project_session_bindings.is_empty());
    Ok(())
}

fn run_record_init(
    repo_root: &Path,
    process: &mut FakeConnectionProcess,
) -> Result<Value, Box<dyn Error>> {
    let result = run_init_command(
        InitArgs {
            host: CodexHost::Codex,
            repo: repo_root.to_path_buf(),
            shared: false,
            profile: RecordProfile::Record,
            runtime_home: RuntimeHomeArgs::default(),
            mcp_command: None,
            dry_run: false,
            output: ConnectionReportOutputArgs {
                json: true,
                verbose: false,
            },
        },
        repo_root,
        process,
    );
    let output = match result {
        Err(ConnectionCommandError::FailureOutput(output)) => output,
        Ok(output) => {
            assert!(
                !output.contains(GENERATED_SHAPE_ERROR),
                "init returned the generated exact-shape regression: {output}"
            );
            return Err("failed init unexpectedly used the success return channel".into());
        }
        Err(ConnectionCommandError::Usage(message)) => {
            return Err(
                format!("failed init unexpectedly returned a usage error: {message}").into(),
            );
        }
        Err(ConnectionCommandError::Runtime(message)) => {
            assert!(
                !message.contains(GENERATED_SHAPE_ERROR),
                "init returned the generated exact-shape regression: {message}"
            );
            return Err(
                format!("failed init unexpectedly returned a runtime error: {message}").into(),
            );
        }
    };
    assert!(!output.contains(GENERATED_SHAPE_ERROR));
    Ok(serde_json::from_str(&output)?)
}

fn run_record_init_outcome(
    repo_root: &Path,
    process: &mut FakeConnectionProcess,
) -> Result<Value, Box<dyn Error>> {
    let output = match run_init_command(
        InitArgs {
            host: CodexHost::Codex,
            repo: repo_root.to_path_buf(),
            shared: false,
            profile: RecordProfile::Record,
            runtime_home: RuntimeHomeArgs::default(),
            mcp_command: None,
            dry_run: false,
            output: ConnectionReportOutputArgs {
                json: true,
                verbose: false,
            },
        },
        repo_root,
        process,
    ) {
        Ok(output) | Err(ConnectionCommandError::FailureOutput(output)) => output,
        Err(error) => return Err(error.into()),
    };
    assert!(!output.contains(GENERATED_SHAPE_ERROR));
    Ok(serde_json::from_str(&output)?)
}

fn run_connection_add(
    repo_root: &Path,
    shared: bool,
    read_only: bool,
    dry_run: bool,
    process: &mut FakeConnectionProcess,
) -> Result<Value, Box<dyn Error>> {
    let output = match run_connection_command(
        ConnectionArgs {
            command: ConnectionCommand::Add(ConnectionAddArgs {
                host: Some(CodexHost::Codex),
                repo: Some(repo_root.to_path_buf()),
                runtime_home: RuntimeHomeArgs::default(),
                shared,
                read_only,
                dry_run,
                output: ConnectionReportOutputArgs {
                    json: true,
                    verbose: false,
                },
            }),
        },
        repo_root,
        process,
    ) {
        Ok(output) | Err(ConnectionCommandError::FailureOutput(output)) => output,
        Err(error) => return Err(error.into()),
    };
    assert!(!output.contains(GENERATED_SHAPE_ERROR));
    Ok(serde_json::from_str(&output)?)
}

fn project_id_for_repo(
    snapshot: &RegistryInspectionSnapshot,
    repo_root: &Path,
) -> Result<String, Box<dyn Error>> {
    snapshot
        .projects
        .iter()
        .find(|project| project.repo_root == repo_root)
        .map(|project| project.project_id.clone())
        .ok_or_else(|| format!("missing project for {}", repo_root.display()).into())
}

fn guard_id_for_project(
    snapshot: &RegistryInspectionSnapshot,
    connection_internal_id: &str,
    project_id: &str,
) -> Result<String, Box<dyn Error>> {
    snapshot
        .guard_installations
        .iter()
        .find(|guard| {
            guard.connection_internal_id == connection_internal_id && guard.project_id == project_id
        })
        .map(|guard| guard.guard_installation_id.clone())
        .ok_or_else(|| {
            format!("missing Guard Installation for {connection_internal_id}/{project_id}").into()
        })
}

fn seed_managed_project_session(
    runtime_home: &Path,
    connection_internal_id: &str,
    project_id: &str,
    guard_installation_id: &str,
    process_id: u32,
    host_session_id: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let runtime = volicord_test_support::start_test_mcp_runtime_session(
        runtime_home,
        McpRuntimeSessionStart {
            connection_internal_id: connection_internal_id.to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: Some("999.0.0".to_owned()),
            process_id,
            process_started_at: "2026-07-19T00:00:00Z".to_owned(),
        },
    )?;
    let session = seed_project_session_on_runtime(
        runtime_home,
        &runtime.runtime_session_id,
        connection_internal_id,
        project_id,
        guard_installation_id,
        host_session_id,
        "2026-07-19T00:00:01Z",
    )?;
    Ok((runtime.runtime_session_id, session))
}

fn seed_project_session_on_runtime(
    runtime_home: &Path,
    runtime_session_id: &str,
    connection_internal_id: &str,
    project_id: &str,
    guard_installation_id: &str,
    host_session_id: &str,
    observed_at: &str,
) -> Result<String, Box<dyn Error>> {
    Ok(bind_agent_session_runtime(
        runtime_home,
        project_id,
        AgentSessionRuntimeBinding {
            runtime_session_id: runtime_session_id.to_owned(),
            connection_internal_id: connection_internal_id.to_owned(),
            guard_installation_id: Some(guard_installation_id.to_owned()),
            correlation: CodexMcpCorrelation {
                session_id: HostSessionId::parse(host_session_id)?,
                thread_id: HostThreadId::parse("native.thread.fixture")?,
                turn_id: HostTurnId::parse("native.turn.fixture")?,
            },
            observed_at: observed_at.to_owned(),
        },
    )?
    .session_id)
}

fn run_record_init_dry_run(
    repo_root: &Path,
    process: &mut FakeConnectionProcess,
) -> Result<Value, Box<dyn Error>> {
    let result = run_init_command(
        InitArgs {
            host: CodexHost::Codex,
            repo: repo_root.to_path_buf(),
            shared: false,
            profile: RecordProfile::Record,
            runtime_home: RuntimeHomeArgs::default(),
            mcp_command: None,
            dry_run: true,
            output: ConnectionReportOutputArgs {
                json: true,
                verbose: false,
            },
        },
        repo_root,
        process,
    );
    let output = match result {
        Ok(output) | Err(ConnectionCommandError::FailureOutput(output)) => output,
        Err(error) => return Err(error.into()),
    };
    Ok(serde_json::from_str(&output)?)
}

fn run_read_only_mode(
    repo_root: &Path,
    process: &mut FakeConnectionProcess,
) -> Result<Value, Box<dyn Error>> {
    let output = run_connection_command(
        ConnectionArgs {
            command: ConnectionCommand::Mode(ConnectionModeArgs {
                host: Some(CodexHost::Codex),
                mode: ConnectionMode::ReadOnly,
                repo: Some(repo_root.to_path_buf()),
                runtime_home: RuntimeHomeArgs::default(),
                shared: false,
                output: ConnectionReportOutputArgs {
                    json: true,
                    verbose: false,
                },
            }),
        },
        repo_root,
        process,
    )?;
    Ok(serde_json::from_str(&output)?)
}

fn assert_failed_init_with_recorded_guard(output: &Value) {
    assert_eq!(
        output["status"], "failed",
        "unexpected init result: {output}"
    );
    assert_eq!(output["operation"], "init");
    assert_eq!(output["operation_details"]["dry_run"], false);
    assert_eq!(
        output["operation_details"]["result"],
        json!({"kind": "setup", "applied": true})
    );
}

fn assert_unavailable_codex_verification(
    snapshot: &RegistryInspectionSnapshot,
) -> Result<(), Box<dyn Error>> {
    let connection = &snapshot.agent_connections[0];
    let report: Value = serde_json::from_str(
        connection
            .verification_report_json
            .as_deref()
            .expect("failed init stores a canonical report"),
    )?;
    assert_eq!(report["status"], "failed");
    let checks = report["checks"].as_array().expect("report checks");
    let check = |id: &str| {
        checks
            .iter()
            .find(|check| check["id"] == id)
            .unwrap_or_else(|| panic!("missing check {id}"))
    };
    assert_eq!(check("managed_config")["status"], "passed");
    assert_eq!(
        check("managed_config")["details"]["observed_state"],
        "match"
    );
    assert_eq!(check("host_executable")["status"], "failed");
    assert_eq!(
        check("host_executable")["code"],
        "host_executable_not_found"
    );
    assert_eq!(check("mcp_server")["status"], "failed");
    assert_eq!(check("host_session")["status"], "pending");
    assert_eq!(check("required_tools")["status"], "pending");
    assert_eq!(check("tool_round_trip")["status"], "pending");
    Ok(())
}

fn assert_minimal_git_worktree(repo_root: &Path) -> Result<(), Box<dyn Error>> {
    let entries = fs::read_dir(repo_root)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_name(), ".git");
    assert!(fs::read_dir(repo_root.join(".git"))?.next().is_none());
    Ok(())
}

fn registry_snapshot(runtime_home: &Path) -> RegistryInspectionSnapshot {
    match inspect_runtime_home(runtime_home).registry {
        DatabaseInspection::Present(snapshot) => snapshot,
        other => panic!("expected current registry snapshot, got {other:?}"),
    }
}

fn assert_single_owned_records(snapshot: &RegistryInspectionSnapshot) -> OwnedIds {
    let installation = snapshot
        .installation_profile
        .as_ref()
        .expect("one installation profile");
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.agent_connections.len(), 1);
    assert_eq!(snapshot.connection_projects.len(), 1);
    assert_eq!(snapshot.guard_installations.len(), 1);
    let project = &snapshot.projects[0];
    let connection = &snapshot.agent_connections[0];
    let binding = &snapshot.connection_projects[0];
    let guard = &snapshot.guard_installations[0];
    assert_eq!(
        binding.connection_internal_id,
        connection.connection_internal_id
    );
    assert_eq!(binding.project_internal_id, project.project_internal_id);
    assert_eq!(
        guard.connection_internal_id,
        connection.connection_internal_id
    );
    assert_eq!(guard.project_internal_id, project.project_internal_id);
    assert_eq!(guard.project_id, project.project_id);
    OwnedIds {
        installation_id: installation.installation_id.clone(),
        project_id: project.project_id.clone(),
        project_internal_id: project.project_internal_id.clone(),
        connection_id: connection.connection_internal_id.clone(),
        guard_installation_id: guard.guard_installation_id.clone(),
    }
}

fn assert_exact_manifest_and_artifacts(
    snapshot: &RegistryInspectionSnapshot,
    repo_root: &Path,
) -> Result<Value, Box<dyn Error>> {
    let connection = &snapshot.agent_connections[0];
    let guard = &snapshot.guard_installations[0];
    let manifest: Value = serde_json::from_str(&guard.manifest_json)?;
    assert!(guard_manifest_has_exact_current_shape(&manifest));
    let connection_record = AgentConnectionRecord {
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
    };
    let integration_revision = connection_integration_revision(&connection_record)?;
    let exclude_path = repo_root.join(".git/info/exclude");
    assert!(guard_manifest_matches_owner_binding(
        &manifest,
        GuardManifestOwnerBinding {
            row_guard_installation_id: &guard.guard_installation_id,
            row_connection_id: &guard.connection_internal_id,
            row_project_id: &guard.project_id,
            connection_host_kind: &connection.host_kind,
            connection_integration_revision: integration_revision.as_str(),
            project_repo_root: repo_root,
            project_git_info_exclude_path: Some(&exclude_path),
        }
    ));

    let expected = BTreeMap::from([
        (
            repo_root.join("AGENTS.md"),
            ("agents_managed_block", "managed_block"),
        ),
        (
            repo_root.join(".volicord/policy.json"),
            ("volicord_policy", "managed_json"),
        ),
        (
            repo_root.join(".codex/hooks.json"),
            ("host_hook_config", "managed_json"),
        ),
        (
            repo_root.join(".codex/hooks/volicord-dispatch.sh"),
            ("host_hook_dispatch", "managed_script"),
        ),
        (
            repo_root.join(".codex/hooks/volicord-pre-tool.sh"),
            ("host_hook_wrapper", "managed_script"),
        ),
        (
            repo_root.join(".codex/hooks/volicord-post-tool.sh"),
            ("host_hook_wrapper", "managed_script"),
        ),
        (
            repo_root.join(".codex/hooks/volicord-prompt-capture.sh"),
            ("host_hook_wrapper", "managed_script"),
        ),
        (
            repo_root.join(".codex/rules/volicord.rules"),
            ("host_rule_instruction", "managed_block"),
        ),
        (
            repo_root.join(".git/info/exclude"),
            ("git_info_exclude", "managed_block"),
        ),
    ]);
    let files = manifest["managed_files"]
        .as_array()
        .expect("manifest managed files");
    assert_eq!(files.len(), expected.len());
    for file in files {
        let path = PathBuf::from(file["path"].as_str().expect("managed path"));
        let (kind, ownership) = expected
            .get(&path)
            .unwrap_or_else(|| panic!("unexpected managed artifact path: {}", path.display()));
        assert_eq!(file["kind"], *kind);
        assert_eq!(file["ownership"], *ownership);
        assert!(
            path.is_file(),
            "managed artifact is missing: {}",
            path.display()
        );
    }
    for artifact in
        guard_manifest_managed_artifacts(&manifest).expect("exact manifest artifact inventory")
    {
        let bytes = fs::read(&artifact.path)?;
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), artifact.digest);
    }
    Ok(manifest)
}

fn assert_manifest_commands_bind_policy_hash(manifest: &Value) {
    let policy_hash = manifest["policy_hash"].as_str().expect("policy hash");
    let commands = manifest["runtime_commands"]
        .as_object()
        .expect("manifest commands");
    assert_eq!(commands.len(), 3);
    for command in commands.values() {
        let args = command["args"]
            .as_array()
            .expect("manifest command args")
            .iter()
            .map(|value| value.as_str().expect("string arg"))
            .collect::<Vec<_>>();
        let index = args
            .iter()
            .position(|arg| *arg == "--policy-hash")
            .expect("manifest command policy hash flag");
        assert_eq!(args.get(index + 1), Some(&policy_hash));
    }
}

fn assert_valid_policy_without_policy_hash_args(repo_root: &Path) -> Result<(), Box<dyn Error>> {
    let policy_path = repo_root.join(".volicord/policy.json");
    let report = run_policy_command(
        PolicyArgs {
            command: PolicyCommand::Validate(PolicyValidateArgs {
                file: policy_path.clone(),
            }),
        },
        |_| None,
        repo_root,
    )?;
    assert!(report.starts_with("Policy schema: "));
    let policy: Value = serde_json::from_slice(&fs::read(policy_path)?)?;
    let commands = policy["host_hook"]["commands"]
        .as_object()
        .expect("policy host-hook commands");
    assert_eq!(commands.len(), 3);
    for command in commands.values() {
        assert!(command["args"]
            .as_array()
            .expect("policy command args")
            .iter()
            .all(|arg| arg.as_str() != Some("--policy-hash")));
    }
    Ok(())
}

fn assert_codex_config_is_user_owned(
    snapshot: &RegistryInspectionSnapshot,
    process: &FakeConnectionProcess,
    repo_root: &Path,
) -> Result<(), Box<dyn Error>> {
    let expected = process.codex_home.join("config.toml");
    let connection = &snapshot.agent_connections[0];
    assert_eq!(Path::new(&connection.config_target), expected);
    assert!(!expected.starts_with(repo_root));
    let document = fs::read_to_string(expected)?.parse::<DocumentMut>()?;
    let entry = document["mcp_servers"]["volicord"]
        .as_table()
        .ok_or("personal Codex entry should be a table")?;
    let command = entry["command"]
        .as_str()
        .ok_or("personal Codex command should be a string")?;
    assert!(Path::new(command).is_absolute());
    assert_eq!(Path::new(command), process.current_exe);
    assert_eq!(
        toml_string_array(entry, "args")?,
        [
            "_host-launch",
            "codex",
            "--connection",
            connection.connection_internal_id.as_str(),
        ]
    );
    assert!(entry.get("env_vars").is_none());
    let environment = entry["env"]
        .as_table()
        .ok_or("personal Codex environment should be a table")?;
    assert_eq!(environment.len(), 1);
    assert_eq!(
        environment["VOLICORD_HOME"].as_str(),
        process.runtime_home.to_str()
    );
    Ok(())
}

fn assert_codex_config_is_shared(
    runtime_home: &Path,
    repo_root: &Path,
    connection_id: &str,
    project_id: &str,
) -> Result<(), Box<dyn Error>> {
    let config_path = repo_root.join(".codex/config.toml");
    let config = fs::read_to_string(&config_path)?;
    let document = config.parse::<DocumentMut>()?;
    let entry = document["mcp_servers"]["volicord"]
        .as_table()
        .ok_or("shared Codex entry should be a table")?;
    assert_eq!(entry["command"].as_str(), Some("volicord"));
    assert_eq!(
        toml_string_array(entry, "args")?,
        ["_host-launch", "codex", "--discover-repository"]
    );
    assert_eq!(toml_string_array(entry, "env_vars")?, ["VOLICORD_HOME"]);
    assert!(entry.get("env").is_none());
    for local_coordinate in [
        runtime_home
            .to_str()
            .ok_or("Runtime Home should be UTF-8")?,
        connection_id,
        project_id,
    ] {
        assert!(
            !config.contains(local_coordinate),
            "shared Codex configuration embedded local coordinate {local_coordinate}"
        );
    }
    Ok(())
}

fn toml_string_array<'a>(
    table: &'a toml_edit::Table,
    key: &str,
) -> Result<Vec<&'a str>, Box<dyn Error>> {
    let values = table[key]
        .as_array()
        .ok_or_else(|| format!("Codex {key} should be an array"))?;
    Ok(values
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("Codex {key} should contain only strings"))
        })
        .collect::<Result<Vec<_>, _>>()?)
}

fn assert_authoritative_policy_matches_file(
    runtime_home: &Path,
    project_id: &str,
    repo_root: &Path,
) -> Result<(), Box<dyn Error>> {
    let store = CoreProjectStore::open_read_only(runtime_home, &ProjectId::new(project_id))?;
    let authority = store
        .project_workflow_policy()?
        .expect("authoritative workflow policy");
    let policy: Value =
        serde_json::from_slice(&fs::read(repo_root.join(".volicord/policy.json"))?)?;
    assert_eq!(
        authority.policy_fingerprint,
        canonical_json_sha256(&policy)?.into_inner()
    );
    Ok(())
}

fn managed_artifact_bytes(
    snapshot: &RegistryInspectionSnapshot,
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let manifest: Value = serde_json::from_str(&snapshot.guard_installations[0].manifest_json)?;
    guard_manifest_managed_artifacts(&manifest)
        .expect("exact manifest artifact inventory")
        .into_iter()
        .map(|artifact| Ok((artifact.path.clone(), fs::read(artifact.path)?)))
        .collect()
}

fn inspected_connection_revision(
    connection: &volicord_store::inspection::AgentConnectionInspectionRecord,
) -> Result<String, Box<dyn Error>> {
    let connection = AgentConnectionRecord {
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
    };
    Ok(connection_integration_revision(&connection)?.into_inner())
}

fn assert_read_only_revision_unchanged(
    snapshot: &RegistryInspectionSnapshot,
    generation: i64,
    revision: &str,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(snapshot.agent_connections.len(), 1);
    let connection = &snapshot.agent_connections[0];
    assert_eq!(connection.mode, CONNECTION_MODE_READ_ONLY);
    assert_eq!(connection.integration_generation, generation);
    assert_eq!(inspected_connection_revision(connection)?, revision);
    Ok(())
}

fn assert_manifest_revision(
    snapshot: &RegistryInspectionSnapshot,
    revision: &str,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(snapshot.guard_installations.len(), 1);
    let manifest: Value = serde_json::from_str(&snapshot.guard_installations[0].manifest_json)?;
    assert_eq!(manifest["integration_revision"], revision);
    assert_exact_manifest_and_artifacts(snapshot, Path::new(&snapshot.projects[0].repo_root))?;
    Ok(())
}

fn managed_script_path(snapshot: &RegistryInspectionSnapshot) -> Result<PathBuf, Box<dyn Error>> {
    let manifest: Value = serde_json::from_str(&snapshot.guard_installations[0].manifest_json)?;
    manifest["managed_files"]
        .as_array()
        .and_then(|files| {
            files.iter().find_map(|file| {
                (file["ownership"] == "managed_script")
                    .then(|| file["path"].as_str().map(PathBuf::from))
                    .flatten()
            })
        })
        .ok_or_else(|| "Guard manifest has no managed script".into())
}

fn assert_mode_expectations(process: &FakeConnectionProcess) {
    assert_eq!(
        process.preflight_modes.last().map(String::as_str),
        Some(CONNECTION_MODE_READ_ONLY)
    );
    assert_eq!(
        process.verification_modes.last().map(String::as_str),
        Some(CONNECTION_MODE_READ_ONLY)
    );
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

fn delete_guard_installation(
    runtime_home: &Path,
    guard_installation_id: &str,
) -> Result<(), Box<dyn Error>> {
    let connection = Connection::open(runtime_home.join("registry.sqlite"))?;
    let deleted = connection.execute(
        "DELETE FROM guard_installations WHERE guard_installation_id = ?1",
        params![guard_installation_id],
    )?;
    assert_eq!(deleted, 1);
    Ok(())
}
