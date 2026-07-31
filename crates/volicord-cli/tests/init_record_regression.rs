#![forbid(unsafe_code)]

mod support;

use std::{
    collections::BTreeMap,
    error::Error,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Barrier, Condvar, Mutex},
};

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use support::binary_fixture::create_git_repo;
use toml_edit::DocumentMut;
use volicord_cli::{
    connection_command::{
        run_connection_command, run_init_command, ConnectionCommandError, ConnectionProcess,
        ConnectionProcessOutput, McpExchangeOutcome, McpExchangeProgress, McpProcessFailure,
        McpStage,
    },
    policy_command::run_policy_command,
};
use volicord_command_model::{
    CodexHost, ConnectionAddArgs, ConnectionArgs, ConnectionCommand, ConnectionMode,
    ConnectionModeArgs, InitArgs, PolicyArgs, PolicyCommand, PolicyValidateArgs, RecordProfile,
    ReportOutputArgs, RuntimeHomeArgs,
};
use volicord_host_contract::{CodexMcpCorrelation, HostSessionId, HostThreadId, HostTurnId};
use volicord_mcp::{ManagedMcpInvocationPurpose, MaterializedManagedMcpLaunch};
use volicord_platform_fs::directory_tree_removal_test_support::{
    fail_next_directory_tree_removal, DirectoryTreeRemovalFault,
};
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
use volicord_test_support::{TempRuntimeHome, TestRuntimeHomeMutation, TestRuntimeHomeSetup};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::guard_manifest::{
    guard_manifest_has_exact_current_shape, guard_manifest_managed_artifacts,
    guard_manifest_matches_owner_binding, GuardManifestOwnerBinding,
};
use volicord_types::ids::ProjectId;
use volicord_types::integration_revision::McpRuntimeSessionSource;

const GENERATED_SHAPE_ERROR: &str =
    "generated Guard manifest does not match the current exact shape";

type RuntimeHomeFileSnapshot = BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)>;

#[derive(Debug)]
struct FakeConnectionProcess {
    runtime_home: PathBuf,
    codex_home: PathBuf,
    isolated_path: PathBuf,
    current_exe: PathBuf,
    preflight_modes: Vec<String>,
    verification_modes: Vec<String>,
    setup_points: Vec<String>,
    fail_setup_call: Option<usize>,
    fail_setup_point: Option<String>,
    fail_during_rollback: bool,
    directory_removal_fault: Option<DirectoryTreeRemovalFault>,
    concurrent_codex_bytes: Option<Vec<u8>>,
    post_commit_codex_bytes: Option<Vec<u8>>,
    setup_pause_call: Option<usize>,
    setup_pause_point: Option<String>,
    setup_pause_occurrence: Option<usize>,
    setup_barrier: Option<Arc<Barrier>>,
    setup_release: Option<Arc<(Mutex<bool>, Condvar)>>,
}

impl FakeConnectionProcess {
    fn new(fixture: &TempRuntimeHome) -> Result<Self, Box<dyn Error>> {
        Self::named(fixture, "fake")
    }

    fn named(fixture: &TempRuntimeHome, name: &str) -> Result<Self, Box<dyn Error>> {
        let codex_home = fixture.root_path().join(format!("{name}-codex-home"));
        let isolated_path = fixture.root_path().join(format!("{name}-isolated-path"));
        fs::create_dir_all(&codex_home)?;
        fs::create_dir_all(&isolated_path)?;
        Ok(Self {
            runtime_home: fixture.path().to_path_buf(),
            codex_home,
            isolated_path,
            current_exe: PathBuf::from(env!("CARGO_BIN_EXE_volicord")),
            preflight_modes: Vec::new(),
            verification_modes: Vec::new(),
            setup_points: Vec::new(),
            fail_setup_call: None,
            fail_setup_point: None,
            fail_during_rollback: false,
            directory_removal_fault: None,
            concurrent_codex_bytes: None,
            post_commit_codex_bytes: None,
            setup_pause_call: None,
            setup_pause_point: None,
            setup_pause_occurrence: None,
            setup_barrier: None,
            setup_release: None,
        })
    }

    fn fail_setup_call(&mut self, call: usize) {
        self.fail_setup_call = Some(call);
    }

    fn fail_setup_point(&mut self, point: &str) {
        self.fail_setup_point = Some(point.to_owned());
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

    fn setup_fault(&mut self, point: &str) -> Result<(), String> {
        let call = self.setup_points.len();
        self.setup_points.push(point.to_owned());
        let point_occurrence = self
            .setup_points
            .iter()
            .filter(|observed| observed.as_str() == point)
            .count();
        let pause_at_call = self.setup_pause_call == Some(call);
        let pause_at_point = self.setup_pause_call.is_none()
            && self.setup_pause_point.as_deref() == Some(point)
            && self
                .setup_pause_occurrence
                .is_none_or(|expected| expected == point_occurrence);
        if pause_at_call || pause_at_point {
            if let Some(barrier) = &self.setup_barrier {
                barrier.wait();
            }
            if let Some(release) = &self.setup_release {
                let (lock, ready) = &**release;
                let mut released = lock
                    .lock()
                    .map_err(|_| "fixture setup release lock poisoned".to_owned())?;
                while !*released {
                    released = ready
                        .wait(released)
                        .map_err(|_| "fixture setup release lock poisoned".to_owned())?;
                }
            }
        }
        if point == "before_codex_config_replace" {
            if let Some(bytes) = self.concurrent_codex_bytes.take() {
                fs::write(self.codex_home.join("config.toml"), bytes)
                    .map_err(|error| error.to_string())?;
            }
        }
        if point == "after_codex_config_replace" {
            if let Some(bytes) = self.post_commit_codex_bytes.take() {
                fs::write(self.codex_home.join("config.toml"), bytes)
                    .map_err(|error| error.to_string())?;
            }
        }
        if point == "during_rollback" {
            if let Some(fault) = self.directory_removal_fault.take() {
                fail_next_directory_tree_removal(fault);
            }
        }
        if (point == "during_rollback" && self.fail_during_rollback)
            || self.fail_setup_call == Some(call)
            || self.fail_setup_point.as_deref() == Some(point)
        {
            return Err(format!("fixture setup fault at {point}"));
        }
        Ok(())
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
                "{{\"operation\":\"mcp_preflight\",\"status\":\"passed\",\"side_effects\":[],\"evidence_class\":\"read_only_preflight\",\"configuration\":\"valid\",\"canonical_managed_entry\":\"passed\",\"transport\":\"stdio\",\"connection_id\":\"{connection_id}\",\"mode\":\"{mode}\",\"enabled\":true,\"registry_read\":\"passed\",\"project_state_read\":\"passed\",\"writeability\":{{\"status\":\"not_checked\",\"requirement\":\"requires_active_verification\"}},\"effective_tool_mode\":\"requires_active_verification\",\"tools_list_schema_validation\":\"passed\",\"protocol_profiles\":[\"2025-11-25\"],\"host_contracts\":[{{\"profile\":\"codex\",\"digest\":\"sha256:fixture\"}}],\"projects\":[]}}\n"
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
fn fresh_init_fault_matrix_restores_every_transactional_target() -> Result<(), Box<dyn Error>> {
    let control = TempRuntimeHome::new("cli-record-init-transaction-points")?;
    let control_repo = create_git_repo(&control, "repo")?;
    let mut control_process = FakeConnectionProcess::new(&control)?;
    let control_output = run_record_init_outcome(&control_repo, &mut control_process)?;
    assert_eq!(
        control_output["operation_details"]["result"]["disposition"],
        "committed"
    );
    let setup_points = control_process.setup_points.clone();
    assert_eq!(
        setup_points.first().map(String::as_str),
        Some("after_runtime_home_preparation")
    );
    assert!(setup_points
        .iter()
        .any(|point| point == "after_registry_mutation_preparation"));
    assert!(setup_points
        .iter()
        .any(|point| point == "runtime_home_parent_directory_sync"));
    assert!(setup_points
        .iter()
        .any(|point| point == "runtime_home_publication_read_back"));
    assert!(setup_points
        .iter()
        .any(|point| point == "runtime_home_publication_manifest_validation"));
    assert!(setup_points
        .iter()
        .any(|point| point == "before_codex_config_replace"));
    assert!(setup_points
        .iter()
        .any(|point| point == "after_codex_config_replace"));
    assert!(setup_points
        .iter()
        .any(|point| point == "before_integration_revision_commit"));
    assert!(setup_points
        .iter()
        .any(|point| point == "after_store_commit_before_checkpoint"));
    assert_eq!(
        setup_points.last().map(String::as_str),
        Some("after_store_checkpoint")
    );
    assert!(
        setup_points
            .iter()
            .filter(|point| point.starts_with("after_managed_file:"))
            .count()
            >= 9,
        "fault coverage did not include every managed hook/rule/guidance file: {setup_points:?}"
    );

    for (call, point) in setup_points.iter().enumerate() {
        let fixture = TempRuntimeHome::new("cli-record-init-transaction-fault")?;
        let repo_root = create_git_repo(&fixture, "repo")?;
        let writer_repo = create_git_repo(&fixture, "writer-repo")?;
        fs::write(repo_root.join("user-notes.txt"), b"user-owned bytes\n")?;
        let mut process = FakeConnectionProcess::new(&fixture)?;
        let runtime_before = directory_contents(fixture.path())?;
        let repo_before = directory_contents(&repo_root)?;
        let codex_before = directory_contents(&process.codex_home)?;
        let codex_home = process.codex_home.clone();
        let barrier = Arc::new(Barrier::new(2));
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        process.setup_pause_call = Some(call);
        process.setup_barrier = Some(Arc::clone(&barrier));
        process.setup_release = Some(Arc::clone(&release));
        process.fail_setup_call(call);

        let runtime_home = fixture.path().to_path_buf();
        let repo_root_for_setup = repo_root.clone();
        let (failure, busy, before_busy, after_busy) = std::thread::scope(|scope| {
            let setup = scope.spawn(move || {
                run_record_init_outcome(&repo_root_for_setup, &mut process)
                    .map_err(|error| error.to_string())
            });
            barrier.wait();
            let before_busy = directory_contents(&runtime_home);
            let busy = run_project_use_binary(&runtime_home, &writer_repo);
            let after_busy = directory_contents(&runtime_home);
            release_setup_pause(&release);
            (
                setup.join().expect("faulted init thread panicked"),
                busy,
                before_busy,
                after_busy,
            )
        });
        let failure = failure.map_err(|error| error.to_string())?;
        let busy = busy?;
        assert_eq!(busy.status.code(), Some(1), "{point}");
        let busy_stderr = String::from_utf8(busy.stderr)?;
        if matches!(
            point.as_str(),
            "after_runtime_home_preparation" | "after_registry_mutation_preparation"
        ) {
            assert!(
                busy_stderr.contains("RUNTIME_HOME_MISSING"),
                "{point}: {busy_stderr}"
            );
        } else {
            assert!(
                busy_stderr.contains("runtime_home.mutation.setup_in_progress"),
                "{point}: {busy_stderr}"
            );
            assert!(busy_stderr.contains("cli.project.use"), "{point}");
        }
        assert_eq!(after_busy?, before_busy?, "{point}");
        let expected_disposition = if matches!(
            point.as_str(),
            "after_runtime_home_preparation" | "after_registry_mutation_preparation"
        ) {
            "preserved"
        } else {
            "rolled_back"
        };
        assert_eq!(
            failure["operation_details"]["result"]["disposition"], expected_disposition,
            "unexpected disposition for fault point {point}: {failure}"
        );
        let expected_publication = if matches!(
            point.as_str(),
            "after_runtime_home_preparation" | "after_registry_mutation_preparation"
        ) {
            "not_published"
        } else {
            "owned_publication_rolled_back"
        };
        assert_eq!(
            failure["operation_details"]["result"]["runtime_home_publication"],
            expected_publication,
            "unexpected publication state for fault point {point}: {failure}"
        );
        assert!(failure["activation_plan"]["required_steps"]
            .as_array()
            .is_some_and(Vec::is_empty));
        assert_eq!(
            directory_contents(fixture.path())?,
            runtime_before,
            "{point}"
        );
        assert_eq!(directory_contents(&repo_root)?, repo_before, "{point}");
        assert_eq!(directory_contents(&codex_home)?, codex_before, "{point}");
    }
    Ok(())
}

#[test]
fn competing_inits_are_busy_until_success_releases_the_lease_for_either_order(
) -> Result<(), Box<dyn Error>> {
    for first_name in ["a", "b"] {
        let fixture = TempRuntimeHome::new(&format!("cli-record-init-setup-lease-{first_name}"))?;
        let repo_a = create_git_repo(&fixture, "repo-a")?;
        let repo_b = create_git_repo(&fixture, "repo-b")?;
        let second_name = if first_name == "a" { "b" } else { "a" };
        let (first_repo, second_repo) = if first_name == "a" {
            (repo_a.clone(), repo_b.clone())
        } else {
            (repo_b.clone(), repo_a.clone())
        };
        let second_repo_before = directory_contents(&second_repo)?;
        let barrier = Arc::new(Barrier::new(2));
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let mut first_process = FakeConnectionProcess::named(&fixture, first_name)?;
        first_process.setup_pause_point =
            Some("runtime_home_publication_manifest_validation".to_owned());
        first_process.setup_barrier = Some(Arc::clone(&barrier));
        first_process.setup_release = Some(Arc::clone(&release));
        let mut second_process = FakeConnectionProcess::named(&fixture, second_name)?;
        let second_codex_before = directory_contents(&second_process.codex_home)?;
        let runtime_home = fixture.path().to_path_buf();
        let first_repo_for_run = first_repo.clone();

        let (first_result, dry_run_busy, full_busy, runtime_while_busy) =
            std::thread::scope(|scope| {
                let first = scope.spawn(move || {
                    run_record_init_outcome(&first_repo_for_run, &mut first_process)
                        .map(|output| (output, first_process))
                        .map_err(|error| error.to_string())
                });
                barrier.wait();
                let runtime_before_busy = directory_contents(&runtime_home);
                let dry_run_busy = run_record_init_dry_run(&second_repo, &mut second_process);
                let full_busy = run_record_init_outcome(&second_repo, &mut second_process);
                let runtime_after_busy = directory_contents(&runtime_home);
                release_setup_pause(&release);
                let first_result = first.join().expect("first init thread panicked");
                (
                    first_result,
                    dry_run_busy,
                    full_busy,
                    runtime_before_busy
                        .and_then(|before| runtime_after_busy.map(|after| (before, after))),
                )
            });
        let (first_output, first_process) =
            first_result.map_err(|error| format!("first {first_name} failed: {error}"))?;
        let dry_run_busy = dry_run_busy?;
        let full_busy = full_busy?;
        let (runtime_before_busy, runtime_after_busy) = runtime_while_busy?;

        assert_setup_lease_busy(&dry_run_busy, "init");
        assert_setup_lease_busy(&full_busy, "init");
        assert_eq!(dry_run_busy["operation_details"]["dry_run"], true);
        assert_eq!(full_busy["operation_details"]["dry_run"], false);
        assert_eq!(runtime_after_busy, runtime_before_busy);
        assert_eq!(directory_contents(&second_repo)?, second_repo_before);
        assert_eq!(
            directory_contents(&second_process.codex_home)?,
            second_codex_before
        );
        assert!(
            second_process.setup_points.is_empty(),
            "the busy invocation reached setup planning: {:?}",
            second_process.setup_points
        );
        assert_eq!(
            first_output["operation_details"]["result"]["runtime_home_publication"],
            "published_by_this_invocation"
        );
        assert_eq!(
            first_output["operation_details"]["result"]["setup_lease"],
            "acquired"
        );
        assert!(!first_process.setup_points.is_empty());

        let second_output = run_record_init_outcome(&second_repo, &mut second_process)?;
        assert_eq!(
            second_output["operation_details"]["result"]["disposition"],
            "committed"
        );
        assert_eq!(
            second_output["operation_details"]["result"]["runtime_home_publication"],
            "existing_ready"
        );
        let after_second = registry_snapshot(&runtime_home);
        assert!(after_second
            .projects
            .iter()
            .any(|project| project.repo_root == second_repo));
        let project_count = after_second.projects.len();
        let connection_count = after_second.agent_connections.len();
        let membership_count = after_second.connection_projects.len();

        let replay = run_record_init_outcome(&second_repo, &mut second_process)?;
        assert_eq!(
            replay["operation_details"]["result"]["runtime_home_publication"],
            "existing_ready"
        );
        let after_replay = registry_snapshot(&runtime_home);
        assert_eq!(after_replay.projects.len(), project_count);
        assert_eq!(after_replay.agent_connections.len(), connection_count);
        assert_eq!(after_replay.connection_projects.len(), membership_count);
    }
    Ok(())
}

#[test]
fn publisher_rollback_finishes_before_the_waiting_init_can_create_fresh_state(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-setup-lease-rollback")?;
    let first_repo = create_git_repo(&fixture, "repo-a")?;
    let second_repo = create_git_repo(&fixture, "repo-b")?;
    let second_repo_before = directory_contents(&second_repo)?;
    let barrier = Arc::new(Barrier::new(2));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let mut first_process = FakeConnectionProcess::named(&fixture, "a")?;
    first_process.setup_pause_point =
        Some("runtime_home_publication_manifest_validation".to_owned());
    first_process.setup_barrier = Some(Arc::clone(&barrier));
    first_process.setup_release = Some(Arc::clone(&release));
    first_process.fail_setup_point("before_integration_revision_commit");
    let mut second_process = FakeConnectionProcess::named(&fixture, "b")?;
    let second_codex_before = directory_contents(&second_process.codex_home)?;
    let runtime_home = fixture.path().to_path_buf();
    let runtime_home_for_thread = runtime_home.clone();

    let (first_result, busy, runtime_while_busy) = std::thread::scope(|scope| {
        let first = scope.spawn(move || {
            run_record_init_outcome(&first_repo, &mut first_process)
                .map(|output| (output, first_process))
                .map_err(|error| error.to_string())
        });
        barrier.wait();
        let runtime_before_busy = directory_contents(&runtime_home_for_thread);
        let busy = run_record_init_outcome(&second_repo, &mut second_process);
        let runtime_after_busy = directory_contents(&runtime_home_for_thread);
        release_setup_pause(&release);
        (
            first.join().expect("first init thread panicked"),
            busy,
            runtime_before_busy.and_then(|before| runtime_after_busy.map(|after| (before, after))),
        )
    });
    let (first_output, _first_process) = first_result.map_err(|error| error.to_string())?;
    let busy = busy?;
    let (runtime_before_busy, runtime_after_busy) = runtime_while_busy?;

    assert_setup_lease_busy(&busy, "init");
    assert_eq!(busy["operation_details"]["dry_run"], false);
    assert_eq!(runtime_after_busy, runtime_before_busy);
    assert_eq!(directory_contents(&second_repo)?, second_repo_before);
    assert_eq!(
        directory_contents(&second_process.codex_home)?,
        second_codex_before
    );
    assert!(second_process.setup_points.is_empty());
    assert_eq!(
        first_output["operation_details"]["result"]["disposition"],
        "rolled_back"
    );
    assert_eq!(
        first_output["operation_details"]["result"]["runtime_home_publication"],
        "owned_publication_rolled_back"
    );
    assert!(
        !runtime_home.exists(),
        "the first transaction's rollback must complete before lease release"
    );

    let second_output = run_record_init_outcome(&second_repo, &mut second_process)?;
    assert_eq!(
        second_output["operation_details"]["result"]["disposition"],
        "committed"
    );
    assert_eq!(
        second_output["operation_details"]["result"]["runtime_home_publication"],
        "published_by_this_invocation"
    );
    let snapshot = registry_snapshot(&runtime_home);
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.projects[0].repo_root, second_repo);
    assert_eq!(snapshot.agent_connections.len(), 1);
    assert_eq!(snapshot.guard_installations.len(), 1);
    Ok(())
}

#[test]
fn setup_admission_remains_exclusive_through_rollback_and_fresh_retry() -> Result<(), Box<dyn Error>>
{
    let fixture = TempRuntimeHome::new("cli-record-init-rollback-admission")?;
    let setup_repo = create_git_repo(&fixture, "repo-setup")?;
    let writer_repo = create_git_repo(&fixture, "repo-writer")?;
    let barrier = Arc::new(Barrier::new(2));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let mut setup_process = FakeConnectionProcess::named(&fixture, "setup")?;
    setup_process.fail_setup_point("before_integration_revision_commit");
    setup_process.setup_pause_point = Some("during_rollback".to_owned());
    setup_process.setup_barrier = Some(Arc::clone(&barrier));
    setup_process.setup_release = Some(Arc::clone(&release));
    let runtime_home = fixture.path().to_path_buf();

    let (setup_result, busy, before_busy, after_busy) = std::thread::scope(|scope| {
        let setup = scope.spawn(move || {
            run_record_init_outcome(&setup_repo, &mut setup_process)
                .map_err(|error| error.to_string())
        });
        barrier.wait();
        let before_busy = directory_contents(&runtime_home);
        let busy = run_project_use_binary(&runtime_home, &writer_repo);
        let after_busy = directory_contents(&runtime_home);
        release_setup_pause(&release);
        (
            setup.join().expect("rollback setup thread panicked"),
            busy,
            before_busy,
            after_busy,
        )
    });
    let setup_result = setup_result.map_err(|error| error.to_string())?;
    let busy = busy?;
    assert_eq!(busy.status.code(), Some(1));
    let busy_stderr = String::from_utf8(busy.stderr)?;
    assert!(busy_stderr.contains("runtime_home.mutation.setup_in_progress"));
    assert!(busy_stderr.contains("cli.project.use"));
    assert_eq!(after_busy?, before_busy?);
    assert_eq!(
        setup_result["operation_details"]["result"]["disposition"],
        "rolled_back"
    );
    assert!(!runtime_home.exists());

    let absent_retry = run_project_use_binary(&runtime_home, &writer_repo)?;
    assert_eq!(absent_retry.status.code(), Some(1));
    assert!(String::from_utf8(absent_retry.stderr)?.contains("RUNTIME_HOME_MISSING"));

    let mut writer_process = FakeConnectionProcess::named(&fixture, "writer")?;
    let accepted = run_record_init_outcome(&writer_repo, &mut writer_process)?;
    assert_eq!(
        accepted["operation_details"]["result"]["disposition"],
        "committed"
    );
    let snapshot = registry_snapshot(&runtime_home);
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.projects[0].repo_root, writer_repo);
    Ok(())
}

#[test]
fn existing_checkpoint_excludes_blocked_external_writer_and_retry_persists_after_rollback(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-checkpoint-admission")?;
    let base_repo = create_git_repo(&fixture, "repo-base")?;
    let setup_repo = create_git_repo(&fixture, "repo-setup")?;
    let writer_repo = create_git_repo(&fixture, "repo-writer")?;
    let mut process = FakeConnectionProcess::named(&fixture, "checkpoint")?;
    let initial = run_record_init_outcome(&base_repo, &mut process)?;
    assert_eq!(
        initial["operation_details"]["result"]["disposition"],
        "committed"
    );
    let original = registry_snapshot(fixture.path());
    assert_eq!(original.projects.len(), 1);
    process.setup_points.clear();
    let barrier = Arc::new(Barrier::new(2));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    process.setup_pause_point = Some("after_store_commit_before_checkpoint".to_owned());
    process.setup_pause_occurrence = Some(2);
    process.setup_barrier = Some(Arc::clone(&barrier));
    process.setup_release = Some(Arc::clone(&release));
    process.fail_setup_point("before_integration_revision_commit");
    let runtime_home = fixture.path().to_path_buf();
    let runtime_home_for_setup = runtime_home.clone();
    let setup_repo_for_thread = setup_repo.clone();

    let (setup_result, busy_output, during_before, during_after) = std::thread::scope(|scope| {
        let setup = scope.spawn(move || {
            run_record_init_outcome(&setup_repo_for_thread, &mut process)
                .map_err(|error| error.to_string())
        });
        barrier.wait();
        let before = directory_contents(&runtime_home_for_setup);
        let busy = run_project_use_binary(&runtime_home_for_setup, &writer_repo);
        let after = directory_contents(&runtime_home_for_setup);
        release_setup_pause(&release);
        (
            setup.join().expect("setup thread panicked"),
            busy,
            before,
            after,
        )
    });
    let setup_result = setup_result.map_err(|error| error.to_string())?;
    let busy_output = busy_output?;
    assert_eq!(busy_output.status.code(), Some(1));
    assert!(busy_output.stdout.is_empty());
    let busy_stderr = String::from_utf8(busy_output.stderr)?;
    assert!(busy_stderr.contains("runtime_home.mutation.setup_in_progress"));
    assert!(busy_stderr.contains("cli.project.use"));
    assert_eq!(during_after?, during_before?);
    assert_eq!(
        setup_result["operation_details"]["result"]["disposition"],
        "rolled_back"
    );
    let rolled_back = registry_snapshot(&runtime_home);
    assert_eq!(rolled_back.projects, original.projects);
    assert_eq!(rolled_back.agent_connections, original.agent_connections);
    assert_eq!(
        rolled_back.connection_projects,
        original.connection_projects
    );

    let accepted = run_project_use_binary(&runtime_home, &writer_repo)?;
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    let final_snapshot = registry_snapshot(&runtime_home);
    assert_eq!(final_snapshot.projects.len(), 2);
    assert!(final_snapshot
        .projects
        .iter()
        .any(|project| project.repo_root == base_repo));
    assert!(final_snapshot
        .projects
        .iter()
        .any(|project| project.repo_root == writer_repo));
    assert!(!final_snapshot
        .projects
        .iter()
        .any(|project| project.repo_root == setup_repo));
    Ok(())
}

#[test]
fn owner_defined_read_only_commands_remain_no_effect_while_setup_is_exclusive(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-read-only-during-setup")?;
    let repo = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::named(&fixture, "read-only")?;
    let initialized = run_record_init_outcome(&repo, &mut process)?;
    assert_eq!(
        initialized["operation_details"]["result"]["disposition"],
        "committed"
    );
    let export_path = fixture.root_path().join("authority-bundle.json");
    let setup = TestRuntimeHomeSetup::acquire(fixture.path())?;
    let before = runtime_home_file_snapshot(fixture.path())?;

    let cases = [
        (
            "connection status",
            vec![
                "connection".into(),
                "status".into(),
                "codex".into(),
                "--repo".into(),
                repo.as_os_str().to_owned(),
                "--json".into(),
            ],
            Some(1),
        ),
        (
            "project list",
            vec!["project".into(), "list".into(), "--json".into()],
            Some(0),
        ),
        (
            "project current",
            vec!["project".into(), "current".into(), "--json".into()],
            Some(0),
        ),
        (
            "diagnostics lookup",
            vec![
                "diagnostics".into(),
                "show".into(),
                "finding_missing_during_setup".into(),
                "--json".into(),
            ],
            Some(1),
        ),
        (
            "authority export",
            vec![
                "export".into(),
                "authority-bundle".into(),
                "--output".into(),
                export_path.as_os_str().to_owned(),
                "--repo".into(),
                repo.as_os_str().to_owned(),
                "--json".into(),
            ],
            Some(0),
        ),
    ];
    for (name, arguments, expected_exit) in cases {
        let output = run_binary_with_fake_environment(&process, &repo, arguments)?;
        assert_eq!(
            output.status.code(),
            expected_exit,
            "{name}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout)
                .contains("runtime_home.mutation.setup_in_progress"),
            "{name} was incorrectly writer-gated"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr)
                .contains("runtime_home.mutation.setup_in_progress"),
            "{name} was incorrectly writer-gated"
        );
    }

    assert!(export_path.is_dir());
    assert_eq!(runtime_home_file_snapshot(fixture.path())?, before);
    drop(setup);
    Ok(())
}

#[test]
fn connection_mode_is_no_effect_while_setup_is_exclusive_and_commits_after_release(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-connection-mode-during-setup")?;
    let repo = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::named(&fixture, "connection-mode")?;
    let initialized = run_record_init_outcome(&repo, &mut process)?;
    assert_eq!(
        initialized["operation_details"]["result"]["disposition"],
        "committed"
    );
    let arguments = vec![
        "connection".into(),
        "mode".into(),
        "codex".into(),
        "read-only".into(),
        "--repo".into(),
        repo.as_os_str().to_owned(),
        "--json".into(),
    ];
    let setup = TestRuntimeHomeSetup::acquire(fixture.path())?;
    let before = runtime_home_file_snapshot(fixture.path())?;

    let busy = run_binary_with_fake_environment(&process, &repo, arguments.clone())?;
    assert_eq!(busy.status.code(), Some(1));
    let busy_text = format!(
        "{}{}",
        String::from_utf8_lossy(&busy.stdout),
        String::from_utf8_lossy(&busy.stderr)
    );
    assert!(busy_text.contains("runtime_home.mutation.setup_in_progress"));
    assert!(busy_text.contains("cli.connection.mode"));
    assert_eq!(runtime_home_file_snapshot(fixture.path())?, before);
    drop(setup);

    let accepted = run_binary_with_fake_environment(&process, &repo, arguments)?;
    assert!(
        accepted.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&accepted.stdout),
        String::from_utf8_lossy(&accepted.stderr)
    );
    let connection = agent_connection_record(
        fixture.path(),
        initialized["connection"]["connection_id"]
            .as_str()
            .expect("initialized connection ID"),
    )?
    .expect("initialized connection");
    assert_eq!(connection.mode, CONNECTION_MODE_READ_ONLY);
    Ok(())
}

#[test]
fn init_concurrent_codex_change_is_preserved_and_other_targets_roll_back(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-concurrent-config")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;
    let runtime_before = directory_contents(fixture.path())?;
    let repo_before = directory_contents(&repo_root)?;
    let external_bytes = b"# written by another process\n".to_vec();
    process.concurrent_codex_bytes = Some(external_bytes.clone());

    let failure = run_record_init_outcome(&repo_root, &mut process)?;
    assert_eq!(
        failure["operation_details"]["result"]["disposition"],
        "rolled_back"
    );
    assert!(failure["checks"][0]["details"]["failure"]
        .as_str()
        .is_some_and(|failure| failure.contains("SETUP_CONCURRENT_MODIFICATION")));
    assert_eq!(
        failure["findings"][0]["code"],
        "setup.concurrent_modification"
    );
    assert_eq!(directory_contents(fixture.path())?, runtime_before);
    assert_eq!(directory_contents(&repo_root)?, repo_before);
    assert_eq!(
        fs::read(process.codex_home.join("config.toml"))?,
        external_bytes
    );
    Ok(())
}

#[test]
fn existing_init_failure_restores_registry_repository_and_codex_bytes() -> Result<(), Box<dyn Error>>
{
    let fixture = TempRuntimeHome::new("cli-record-init-existing-rollback")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;
    let committed = run_record_init_outcome(&repo_root, &mut process)?;
    assert_eq!(
        committed["operation_details"]["result"]["disposition"],
        "committed"
    );
    let runtime_before = directory_contents(fixture.path())?;
    let repo_before = directory_contents(&repo_root)?;
    let codex_before = directory_contents(&process.codex_home)?;
    process.setup_points.clear();
    process.fail_setup_point("before_integration_revision_commit");

    let failure = run_record_init_outcome(&repo_root, &mut process)?;
    assert_eq!(
        failure["operation_details"]["result"]["disposition"],
        "rolled_back"
    );
    assert_eq!(directory_contents(fixture.path())?, runtime_before);
    assert_eq!(directory_contents(&repo_root)?, repo_before);
    assert_eq!(directory_contents(&process.codex_home)?, codex_before);
    Ok(())
}

#[test]
fn init_reports_partial_rollback_fault_but_continues_best_effort_restoration(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-rollback-fault")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;
    let runtime_before = directory_contents(fixture.path())?;
    let repo_before = directory_contents(&repo_root)?;
    let codex_before = directory_contents(&process.codex_home)?;
    let external_bytes = b"# external edit after setup replacement\n".to_vec();
    process.fail_setup_point("after_codex_config_replace");
    process.fail_during_rollback = true;
    process.post_commit_codex_bytes = Some(external_bytes.clone());

    let failure = run_record_init_outcome(&repo_root, &mut process)?;
    assert_eq!(
        failure["operation_details"]["result"]["disposition"],
        "partially_rolled_back"
    );
    assert_eq!(failure["findings"][0]["code"], "setup.partial_rollback");
    assert_eq!(directory_contents(fixture.path())?, runtime_before);
    assert_eq!(directory_contents(&repo_root)?, repo_before);
    assert_ne!(directory_contents(&process.codex_home)?, codex_before);
    assert_eq!(
        fs::read(process.codex_home.join("config.toml"))?,
        external_bytes
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_reports_removed_runtime_home_with_failed_parent_sync_without_claiming_preservation(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-removal-parent-sync")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;
    process.fail_setup_point("runtime_home_publication_read_back");
    process.directory_removal_fault = Some(DirectoryTreeRemovalFault::ParentDirectorySyncFailure);

    let failure = run_record_init_outcome(&repo_root, &mut process)?;

    assert_eq!(
        failure["operation_details"]["result"]["disposition"],
        "partially_rolled_back"
    );
    assert_eq!(
        failure["operation_details"]["result"]["runtime_home_publication"],
        "owned_publication_rolled_back"
    );
    assert_eq!(
        failure["operation_details"]["result"]["runtime_home_rollback"]["outcome"],
        "removed"
    );
    assert_eq!(
        failure["operation_details"]["result"]["runtime_home_rollback"]["durability"],
        "parent_synchronization_failed"
    );
    assert_eq!(
        failure["operation_details"]["result"]["runtime_home_rollback"]["failure_phase"],
        "parent_directory_synchronization"
    );
    assert!(!fixture.path().exists());
    let output = serde_json::to_string(&failure)?;
    assert!(!output.contains("publication remains"));
    assert!(!output.contains("final path was preserved"));
    Ok(())
}

#[test]
fn init_reports_typed_incomplete_runtime_home_removal() -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("cli-record-init-removal-incomplete")?;
    let repo_root = create_git_repo(&fixture, "repo")?;
    let mut process = FakeConnectionProcess::new(&fixture)?;
    process.fail_setup_point("runtime_home_publication_read_back");
    process.directory_removal_fault = Some(DirectoryTreeRemovalFault::BeforeRecursiveRemoval);

    let failure = run_record_init_outcome(&repo_root, &mut process)?;

    assert_eq!(
        failure["operation_details"]["result"]["disposition"],
        "partially_rolled_back"
    );
    assert_eq!(
        failure["operation_details"]["result"]["runtime_home_publication"],
        "owned_publication_removal_incomplete"
    );
    assert_eq!(
        failure["operation_details"]["result"]["runtime_home_rollback"],
        json!({
            "outcome": "removal_incomplete",
            "effect": "not_removed",
            "phase": "recursive_removal",
            "final_path": "present"
        })
    );
    assert!(fixture.path().is_dir());
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
        json!({
            "kind": "setup",
            "disposition": "committed",
            "setup_lease": "acquired",
            "runtime_home_publication": "existing_ready"
        })
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
    let mutation = TestRuntimeHomeMutation::acquire(fixture.path())?;
    let context = mutation.context()?;

    replace_agent_connection_verification_report_if_revision(
        &context,
        &seeded_ids.connection_id,
        &connection_integration_revision(
            &agent_connection_record(fixture.path(), &seeded_ids.connection_id)?
                .expect("seeded Agent Connection"),
        )?,
        None,
    )?;
    delete_guard_installation(fixture.path(), &seeded_ids.guard_installation_id)?;
    drop(context);
    drop(mutation);

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
                output: ReportOutputArgs {
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
fn init_migration_rolls_back_bound_cleanup_inventory_until_clean_replay(
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
    let prior_config_bytes = fs::read(&prior_config_target)?;
    let before_failed_migration = registry_snapshot(fixture.path());

    let mut replacement_process = FakeConnectionProcess::named(&fixture, "replacement")?;
    replacement_process.fail_setup_point("after_codex_config_replace");
    let failed_cleanup = run_record_init_outcome(&repo_root, &mut replacement_process)?;
    assert_eq!(failed_cleanup["operation"], "init");
    assert_eq!(failed_cleanup["status"], "failed");
    assert_eq!(
        failed_cleanup["operation_details"]["result"],
        json!({
            "kind": "setup",
            "disposition": "rolled_back",
            "setup_lease": "acquired",
            "runtime_home_publication": "existing_ready"
        }),
        "unexpected migration output: {failed_cleanup}"
    );
    let failure_details = &failed_cleanup["checks"][0]["details"];
    assert_eq!(failure_details["disposition"], "rolled_back");
    assert_eq!(failure_details["rollback"]["partially_rolled_back"], 0);
    assert!(!failure_details["failure"]
        .as_str()
        .unwrap_or_default()
        .contains("FOREIGN KEY"));

    let rolled_back = registry_snapshot(fixture.path());
    assert_eq!(rolled_back, before_failed_migration);
    let retained_prior = rolled_back
        .agent_connections
        .iter()
        .find(|connection| {
            connection.connection_internal_id == prior_connection.connection_internal_id
        })
        .expect("prior Connection remains");
    assert!(retained_prior.enabled);
    assert!(!connection_metadata_contains_pending_host_cleanup_key(
        &retained_prior.metadata_json
    ));
    assert!(mcp_runtime_session(fixture.path(), &runtime_id)?.is_some());
    assert!(agent_session(fixture.path(), &project_id, &project_session_id)?.is_some());
    assert_eq!(fs::read(&prior_config_target)?, prior_config_bytes);

    replacement_process.fail_setup_point = None;
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
            output: ReportOutputArgs {
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
        Err(ConnectionCommandError::MutationAdmission(error)) => {
            return Err(format!("init mutation admission unexpectedly failed: {error}").into());
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
        Err(ConnectionCommandError::ConcurrentModification(message)) => {
            assert!(
                !message.contains(GENERATED_SHAPE_ERROR),
                "init returned the generated exact-shape regression: {message}"
            );
            return Err(format!(
                "failed init unexpectedly returned an unrendered concurrent-modification error: {message}"
            )
            .into());
        }
    };
    assert!(!output.contains(GENERATED_SHAPE_ERROR));
    Ok(serde_json::from_str(&output)?)
}

fn run_project_use_binary(
    runtime_home: &Path,
    repo_root: &Path,
) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(["project", "use"])
        .arg(repo_root)
        .arg("--json")
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(repo_root)
        .output()?)
}

fn run_binary_with_fake_environment(
    process: &FakeConnectionProcess,
    current_dir: &Path,
    arguments: Vec<OsString>,
) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(arguments)
        .env("VOLICORD_HOME", &process.runtime_home)
        .env("CODEX_HOME", &process.codex_home)
        .env(
            "HOME",
            process
                .codex_home
                .parent()
                .expect("fake Codex home has a parent"),
        )
        .env("PATH", &process.isolated_path)
        .current_dir(current_dir)
        .output()?)
}

fn release_setup_pause(release: &Arc<(Mutex<bool>, Condvar)>) {
    let (lock, ready) = &**release;
    let mut released = lock.lock().expect("fixture setup release lock");
    *released = true;
    ready.notify_all();
}

fn assert_setup_lease_busy(output: &Value, operation: &str) {
    assert_eq!(output["schema_version"], 2);
    assert_eq!(output["operation"], operation);
    assert_eq!(output["status"], "failed");
    assert_eq!(output["checks"][0]["id"], "setup_plan");
    assert_eq!(output["checks"][0]["code"], "setup_lease_busy");
    assert_eq!(
        output["operation_details"]["setup_lease"]["outcome"],
        "busy"
    );
    assert_eq!(
        output["operation_details"]["setup_lease"]["requested_operation"],
        operation
    );
    assert_eq!(
        output["operation_details"]["setup_lease"]["wait_policy"],
        "immediate"
    );
    assert_eq!(output["findings"][0]["code"], "setup.lease_busy");
    assert_eq!(
        output["findings"][0]["actions"][0]["code"],
        "action.setup.wait_for_current_transaction"
    );
    let rendered = serde_json::to_string(output).expect("busy report serializes");
    assert!(!rendered.contains(".lock"));
    assert!(!rendered.contains("delete"));
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
            output: ReportOutputArgs {
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
                output: ReportOutputArgs {
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
    let mutation = TestRuntimeHomeMutation::acquire(runtime_home)?;
    let context = mutation.context()?;
    Ok(bind_agent_session_runtime(
        &context,
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
            output: ReportOutputArgs {
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
                output: ReportOutputArgs {
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
        output["operation_details"]["result"]["kind"], "setup",
        "unexpected init result details: {output}"
    );
    assert_eq!(
        output["operation_details"]["result"]["disposition"],
        "committed"
    );
    assert!(
        matches!(
            output["operation_details"]["result"]["runtime_home_publication"].as_str(),
            Some("published_by_this_invocation" | "existing_ready")
        ),
        "unexpected Runtime Home publication result: {output}"
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
    assert_eq!(check("host_reload")["status"], "pending");
    assert_eq!(check("managed_session_health")["status"], "pending");
    assert_eq!(check("managed_capability_proof")["status"], "pending");
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

fn runtime_home_file_snapshot(root: &Path) -> Result<RuntimeHomeFileSnapshot, Box<dyn Error>> {
    fn visit(
        root: &Path,
        current: &Path,
        output: &mut BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)>,
    ) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                visit(root, &path, output)?;
            } else {
                output.insert(
                    path.strip_prefix(root)?.to_path_buf(),
                    (fs::read(&path)?, entry.metadata()?.modified()?),
                );
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
