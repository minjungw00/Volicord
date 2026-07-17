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
use volicord_cli::{
    cli::{CodexHost, InitArgs, PolicyArgs, PolicyCommand, PolicyValidateArgs, RecordProfile},
    connection_command::{
        run_init_command, ConnectionProcess, ConnectionProcessOutput, McpLaunch, McpVerification,
    },
    policy_command::run_policy_command,
};
use volicord_store::{
    agent_connections::{
        update_agent_connection_verification_report, VERIFIED_STATUS_FAILED,
        VERIFIED_STATUS_NOT_VERIFIED,
    },
    core_pipeline::CoreProjectStore,
    inspection::{inspect_runtime_home, DatabaseInspection, RegistryInspectionSnapshot},
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    canonical_json_sha256, host_hook_capability_has_exact_current_shape,
    host_hook_capability_managed_artifacts, host_hook_capability_matches_owner_binding,
    HostHookCapabilityOwnerBinding, ProjectId,
};

const GENERATED_SHAPE_ERROR: &str =
    "generated host-hook capability does not match the current exact shape";

#[derive(Debug)]
struct FakeConnectionProcess {
    runtime_home: PathBuf,
    codex_home: PathBuf,
    isolated_path: PathBuf,
    current_exe: PathBuf,
}

impl FakeConnectionProcess {
    fn new(fixture: &TempRuntimeHome) -> Result<Self, Box<dyn Error>> {
        let codex_home = fixture.path().join("fake-codex-home");
        let isolated_path = fixture.path().join("isolated-path");
        fs::create_dir_all(&codex_home)?;
        fs::create_dir_all(&isolated_path)?;
        Ok(Self {
            runtime_home: fixture.path().to_path_buf(),
            codex_home,
            isolated_path,
            current_exe: PathBuf::from(env!("CARGO_BIN_EXE_volicord")),
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
        _launch: &McpLaunch,
        _runtime_home: &Path,
        connection_id: &str,
        _project_id: Option<&str>,
    ) -> Result<ConnectionProcessOutput, String> {
        Ok(ConnectionProcessOutput {
            success: true,
            status_code: Some(0),
            stdout: format!(
                "configuration: valid\ntransport: stdio\nconnection_id: {connection_id}\nmode: workflow\nenabled: true\nproject_state_read: passed\nproject_state_write: passed\neffective_tool_mode: workflow\n"
            ),
            stderr: String::new(),
        })
    }

    fn verify_mcp_stdio(
        &mut self,
        _launch: &McpLaunch,
        _runtime_home: &Path,
        _connection_id: &str,
        _mode: &str,
    ) -> Result<McpVerification, String> {
        Err("live MCP verification is intentionally unavailable in this fixture".to_owned())
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
fn fresh_record_init_persists_exact_owner_bound_capability_and_managed_artifacts(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-fresh")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;

    assert_minimal_git_worktree(&repo_root)?;
    let output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&output);

    let snapshot = registry_snapshot(fixture.path());
    let ids = assert_single_owned_records(&snapshot);
    assert_unavailable_codex_verification(&snapshot)?;
    assert_eq!(snapshot.projects[0].repo_root, repo_root);
    assert_eq!(snapshot.connection_projects[0].project_id, ids.project_id);

    let capability = assert_exact_capability_and_artifacts(&snapshot, &repo_root)?;
    assert_capability_commands_bind_policy_hash(&capability);
    assert_valid_policy_without_policy_hash_args(&repo_root)?;
    assert_codex_config_is_user_owned(&snapshot, &process, &repo_root)?;
    assert_authoritative_policy_matches_file(fixture.path(), &ids.project_id, &repo_root)?;
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
    let seeded_connection = &seeded_snapshot.agent_connections[0];
    let managed_bytes = managed_artifact_bytes(&seeded_snapshot)?;

    update_agent_connection_verification_report(
        fixture.path(),
        &seeded_ids.connection_id,
        VERIFIED_STATUS_NOT_VERIFIED,
        &seeded_connection.managed_fingerprint,
        "{}",
        "[]",
    )?;
    delete_guard_installation(fixture.path(), &seeded_ids.guard_installation_id)?;

    let partial = registry_snapshot(fixture.path());
    assert_eq!(partial.projects.len(), 1);
    assert_eq!(partial.agent_connections.len(), 1);
    assert_eq!(partial.connection_projects.len(), 1);
    assert!(partial.guard_installations.is_empty());
    assert_eq!(
        partial.agent_connections[0].last_verification_status,
        VERIFIED_STATUS_NOT_VERIFIED
    );
    assert_authoritative_policy_matches_file(fixture.path(), &seeded_ids.project_id, &repo_root)?;
    assert_eq!(fs::read_to_string(&unrelated_path)?, unrelated_content);

    let repair_output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&repair_output);
    assert_eq!(repair_output["changed_repo_files"], json!([]));
    let repaired = registry_snapshot(fixture.path());
    let repaired_ids = assert_single_owned_records(&repaired);
    assert_eq!(repaired_ids, seeded_ids);
    assert_unavailable_codex_verification(&repaired)?;
    let repaired_capability = assert_exact_capability_and_artifacts(&repaired, &repo_root)?;
    assert_capability_commands_bind_policy_hash(&repaired_capability);
    assert_eq!(managed_artifact_bytes(&repaired)?, managed_bytes);
    assert_eq!(fs::read_to_string(&unrelated_path)?, unrelated_content);

    let replay_output = run_record_init(&repo_root, &mut process)?;
    assert_failed_init_with_recorded_guard(&replay_output);
    assert_eq!(replay_output["changed_repo_files"], json!([]));
    let replayed = registry_snapshot(fixture.path());
    let replayed_ids = assert_single_owned_records(&replayed);
    assert_eq!(replayed_ids, seeded_ids);
    assert_unavailable_codex_verification(&replayed)?;
    assert_eq!(
        replayed.guard_installations[0].host_capability_json,
        repaired.guard_installations[0].host_capability_json
    );
    let replayed_capability = assert_exact_capability_and_artifacts(&replayed, &repo_root)?;
    assert_capability_commands_bind_policy_hash(&replayed_capability);
    assert_valid_policy_without_policy_hash_args(&repo_root)?;
    assert_authoritative_policy_matches_file(fixture.path(), &seeded_ids.project_id, &repo_root)?;
    assert_eq!(managed_artifact_bytes(&replayed)?, managed_bytes);
    assert_eq!(fs::read_to_string(&unrelated_path)?, unrelated_content);
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
            home: None,
            mcp_command: None,
            dry_run: false,
            json: true,
        },
        repo_root,
        process,
    );
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            let message = error.to_string();
            assert!(
                !message.contains(GENERATED_SHAPE_ERROR),
                "init returned the generated exact-shape regression: {message}"
            );
            return Err(message.into());
        }
    };
    assert!(!output.contains(GENERATED_SHAPE_ERROR));
    Ok(serde_json::from_str(&output)?)
}

fn assert_failed_init_with_recorded_guard(output: &Value) {
    assert_eq!(
        output["status"], "failed",
        "unexpected init result: {output}"
    );
    assert_eq!(output["guard_installation"]["recorded"], true);
}

fn assert_unavailable_codex_verification(
    snapshot: &RegistryInspectionSnapshot,
) -> Result<(), Box<dyn Error>> {
    let connection = &snapshot.agent_connections[0];
    assert_eq!(connection.last_verification_status, VERIFIED_STATUS_FAILED);
    let report: Value = serde_json::from_str(&connection.last_verification_report_json)?;
    assert_eq!(report["status"], "failed");
    assert_eq!(report["host"]["managed_config"], "match");
    assert_eq!(report["host"]["host_executable"], "unavailable");
    assert_eq!(report["host"]["mcp_handshake_allowed"], false);
    assert_eq!(
        report["host"]["diagnostic"],
        "Codex executable `codex` was not found on PATH"
    );
    assert_eq!(report["cli_mcp_preflight"]["status"], "passed");
    assert_eq!(report["cli_mcp_handshake"]["status"], "skipped");
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
    assert_eq!(
        guard.project_internal_id.as_deref(),
        Some(project.project_internal_id.as_str())
    );
    assert_eq!(
        guard.project_id.as_deref(),
        Some(project.project_id.as_str())
    );
    OwnedIds {
        installation_id: installation.installation_id.clone(),
        project_id: project.project_id.clone(),
        project_internal_id: project.project_internal_id.clone(),
        connection_id: connection.connection_internal_id.clone(),
        guard_installation_id: guard.guard_installation_id.clone(),
    }
}

fn assert_exact_capability_and_artifacts(
    snapshot: &RegistryInspectionSnapshot,
    repo_root: &Path,
) -> Result<Value, Box<dyn Error>> {
    let connection = &snapshot.agent_connections[0];
    let guard = &snapshot.guard_installations[0];
    let capability: Value = serde_json::from_str(&guard.host_capability_json)?;
    assert!(host_hook_capability_has_exact_current_shape(&capability));
    assert!(host_hook_capability_matches_owner_binding(
        &capability,
        HostHookCapabilityOwnerBinding {
            row_host_kind: &guard.host_kind,
            row_guard_mode: &guard.guard_mode,
            row_guard_installation_id: &guard.guard_installation_id,
            connection_internal_id: &connection.connection_internal_id,
            connection_host_kind: &connection.host_kind,
            connection_intent: &connection.intent,
            project_repo_root: Some(repo_root),
            project_git_info_exclude_path: Some(&repo_root.join(".git/info/exclude")),
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
    let files = capability["files"].as_array().expect("capability files");
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
    for artifact in host_hook_capability_managed_artifacts(&capability)
        .expect("exact capability artifact inventory")
    {
        let bytes = fs::read(&artifact.path)?;
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), artifact.digest);
    }
    Ok(capability)
}

fn assert_capability_commands_bind_policy_hash(capability: &Value) {
    let policy_hash = capability["policy_hash"].as_str().expect("policy hash");
    let commands = capability["commands"]
        .as_object()
        .expect("capability commands");
    assert_eq!(commands.len(), 3);
    for command in commands.values() {
        let args = command["args"]
            .as_array()
            .expect("capability command args")
            .iter()
            .map(|value| value.as_str().expect("string arg"))
            .collect::<Vec<_>>();
        let index = args
            .iter()
            .position(|arg| *arg == "--policy-hash")
            .expect("capability command policy hash flag");
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
    let config = fs::read_to_string(expected)?;
    assert!(config.contains(&connection.connection_internal_id));
    Ok(())
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
    let capability: Value =
        serde_json::from_str(&snapshot.guard_installations[0].host_capability_json)?;
    host_hook_capability_managed_artifacts(&capability)
        .expect("exact capability artifact inventory")
        .into_iter()
        .map(|artifact| Ok((artifact.path.clone(), fs::read(artifact.path)?)))
        .collect()
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
