#![forbid(unsafe_code)]

use std::{
    error::Error,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_core::{CoreService, GitWorkspaceContext, InvocationContext};
use volicord_host_contract::{project_mcp_tool, McpServerKey};
use volicord_platform_fs::capture_git_workspace_snapshot;
use volicord_store::bootstrap::{write_installation_profile, InstallationProfileRegistration};
use volicord_store::guards::{
    repository_observation, upsert_guard_installation, GuardInstallationUpsert,
    RepositoryObservationRecord, RepositoryObservationState,
};
use volicord_test_support::{
    core_fixtures::{CoreFixture, UpdateScopeFixture},
    seed_test_agent_session, test_guard_manifest_json,
};
use volicord_types::{
    guard_manifest::{guard_manifest_from_json, GuardManifest},
    ids::{AgentConnectionId, ProjectId},
    tool_names::AgentToolId,
    values::{ChangeUnitOperation, GuardHookPhase, OperationCategory},
};

fn run(args: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(args)
        .output()?)
}

struct GuardRepositoryFixture {
    core: CoreFixture,
    manifest: GuardManifest,
}

impl GuardRepositoryFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let core = CoreFixture::new(prefix)?;
        let repository = core.product_repo_path();
        let managed_binary = core.runtime_home_path().join("bin/volicord");
        fs::create_dir_all(managed_binary.parent().ok_or("managed binary parent")?)?;
        fs::copy(env!("CARGO_BIN_EXE_volicord"), &managed_binary)?;
        write_installation_profile(
            &core.mutation_context()?,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: managed_binary.to_string_lossy().into_owned(),
                volicord_mcp_command: managed_binary.to_string_lossy().into_owned(),
                bin_dir: managed_binary
                    .parent()
                    .ok_or("managed binary parent")?
                    .to_path_buf(),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        run_git(&repository, &["init", "-q"])?;
        run_git(&repository, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
        run_git(&repository, &["config", "user.name", "Volicord Guard Test"])?;
        run_git(
            &repository,
            &["config", "user.email", "guard-test@volicord.invalid"],
        )?;
        run_git(&repository, &["config", "core.autocrlf", "false"])?;
        run_git(&repository, &["config", "commit.gpgsign", "false"])?;
        for (path, content) in [
            ("tracked-dirty.txt", "tracked baseline\n"),
            ("clean-modify.txt", "clean baseline\n"),
            ("delete.txt", "delete baseline\n"),
            ("restore.txt", "restore baseline\n"),
            ("commit.txt", "commit baseline\n"),
            ("src/export.rs", "pub fn export() {}\n"),
            ("src/second.rs", "pub fn second() {}\n"),
            ("tests/export.rs", "#[test]\nfn export_works() {}\n"),
            (".volicord/policy.json", "{}"),
        ] {
            write_repository_file(&repository, path, content.as_bytes())?;
        }
        run_git(&repository, &["add", "--all"])?;
        run_git(
            &repository,
            &["commit", "-q", "-m", "guard fixture baseline"],
        )?;

        let policy_hash = format!("sha256:{:x}", Sha256::digest(b"{}"));
        let guard_installation_id = format!("guard_{prefix}");
        let manifest_json = test_guard_manifest_json(
            core.runtime_home_path(),
            &repository,
            core.project_id(),
            core.connection_id(),
            &guard_installation_id,
            &policy_hash,
        );
        upsert_guard_installation(
            &core.mutation_context()?,
            GuardInstallationUpsert {
                guard_installation_id,
                connection_internal_id: core.connection_id().to_owned(),
                project_id: core.project_id().to_owned(),
                manifest_json: manifest_json.clone(),
            },
        )?;
        let manifest = guard_manifest_from_json(&manifest_json)?;
        Ok(Self { core, manifest })
    }

    fn repository(&self) -> PathBuf {
        self.core.product_repo_path()
    }

    fn mcp_callable(&self, tool: AgentToolId) -> Result<String, Box<dyn Error>> {
        Ok(
            project_mcp_tool(&McpServerKey::parse("volicord-test")?, tool)?
                .callable_name()
                .as_str()
                .to_owned(),
        )
    }

    fn invoke(&self, phase: GuardHookPhase, event: &Value) -> Result<Output, Box<dyn Error>> {
        let command_spec = self.manifest.runtime_commands.get(phase);
        let mut child = Command::new(&command_spec.command)
            .args(&command_spec.args)
            .current_dir(self.repository())
            .env("VOLICORD_HOME", self.core.runtime_home_path())
            .env("VOLICORD_MANAGED_WRAPPER", "codex-record")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        serde_json::to_writer(child.stdin.as_mut().ok_or("Guard stdin")?, event)?;
        drop(child.stdin.take());
        let output = child.wait_with_output()?;
        assert!(
            output.status.success(),
            "Guard hook failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(output)
    }

    fn pre(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        tool_input: Value,
    ) -> Result<Output, Box<dyn Error>> {
        self.invoke(
            GuardHookPhase::PreTool,
            &tool_event("PreToolUse", tool_use_id, tool_name, tool_input),
        )
    }

    fn post(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        tool_input: Value,
    ) -> Result<Output, Box<dyn Error>> {
        let mut event = tool_event("PostToolUse", tool_use_id, tool_name, tool_input);
        event
            .as_object_mut()
            .ok_or("tool event object")?
            .insert("tool_response".to_owned(), json!({"success": true}));
        self.invoke(GuardHookPhase::PostTool, &event)
    }

    fn observation(
        &self,
        tool_use_id: &str,
    ) -> Result<RepositoryObservationRecord, Box<dyn Error>> {
        let observation_id = self.core.conn()?.query_row(
            "SELECT repository_observation_id
               FROM repository_observations
              WHERE host_tool_use_id = ?1
              ORDER BY started_at DESC
              LIMIT 1",
            [tool_use_id],
            |row| row.get::<_, String>(0),
        )?;
        repository_observation(
            self.core.runtime_home_path(),
            self.core.project_id(),
            &observation_id,
        )?
        .ok_or_else(|| "repository observation should exist".into())
    }

    fn observation_for(
        &self,
        tool_use_id: &str,
        tool_name: &str,
    ) -> Result<RepositoryObservationRecord, Box<dyn Error>> {
        let observation_id = self.core.conn()?.query_row(
            "SELECT repository_observation_id
               FROM repository_observations
              WHERE host_tool_use_id = ?1
                AND host_tool_name = ?2",
            [tool_use_id, tool_name],
            |row| row.get::<_, String>(0),
        )?;
        repository_observation(
            self.core.runtime_home_path(),
            self.core.project_id(),
            &observation_id,
        )?
        .ok_or_else(|| "repository observation should exist".into())
    }

    fn count(&self, table: &str) -> Result<i64, Box<dyn Error>> {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        Ok(self
            .core
            .conn()?
            .query_row(&sql, [], |row| row.get::<_, i64>(0))?)
    }

    fn agent_invocation(&self) -> Result<InvocationContext, Box<dyn Error>> {
        let session = seed_test_agent_session(
            self.core.runtime_home_path(),
            self.core.project_id(),
            self.core.connection_id(),
            Some(self.manifest.guard_installation_id.as_str()),
        )?;
        let validated = CoreService::for_read_only(self.core.runtime_home_path())
            .validate_agent_session(
                AgentConnectionId::new(self.core.connection_id()),
                ProjectId::new(self.core.project_id()),
                session.runtime_session_id,
                session.project_session_id,
                OperationCategory::AgentWorkflow,
            )?;
        let snapshot = capture_git_workspace_snapshot(&self.repository())?
            .ok_or("fixture Git workspace snapshot")?;
        Ok(
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated)
                .with_git_workspace_context(GitWorkspaceContext {
                    git_common_dir: snapshot.layout.common_dir.display().to_string(),
                    worktree_id: snapshot.worktree_id,
                    branch_ref: snapshot.branch_ref,
                    head_sha: snapshot.head_sha,
                    workspace_fingerprint: snapshot.workspace_fingerprint,
                }),
        )
    }

    fn prepare_write_ticket(&self, prefix: &str, paths: &[&str]) -> Result<(), Box<dyn Error>> {
        let service = CoreService::for_mutation(&self.core.mutation_context()?);
        let intake = service.intake(
            &self.core.mutation_context()?,
            self.core.intake_request(
                &format!("req_{prefix}_intake"),
                &format!("idem_{prefix}_intake"),
                false,
                Some(0),
            ),
            self.agent_invocation()?,
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .ok_or("prepared task ID")?
            .to_owned();
        let scope = service.update_scope(
            &self.core.mutation_context()?,
            self.core.update_scope_request(UpdateScopeFixture {
                request_id: &format!("req_{prefix}_scope"),
                idempotency_key: &format!("idem_{prefix}_scope"),
                dry_run: false,
                expected_state_version: Some(1),
                task_id: &task_id,
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Exercise exact invocation write attribution.",
            }),
            self.agent_invocation()?,
        )?;
        let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
            .as_str()
            .ok_or("prepared Change Unit ID")?
            .to_owned();
        let mut request = self.core.prepare_write_request(
            &format!("req_{prefix}_prepare"),
            &format!("idem_{prefix}_prepare"),
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.intended_paths = paths.iter().map(|path| (*path).to_owned()).collect();
        let prepared = service.prepare_write(
            &self.core.mutation_context()?,
            request,
            self.agent_invocation()?,
        )?;
        assert_eq!(prepared.response_value["decision"], "allowed");
        Ok(())
    }

    fn expected_write_status(
        &self,
        tool_use_id: &str,
    ) -> Result<(String, Option<String>), Box<dyn Error>> {
        Ok(self.core.conn()?.query_row(
            "SELECT e.status, e.matched_paths_json
               FROM expected_writes e
               JOIN repository_observations o
                 ON o.project_id = e.project_id
                AND o.repository_observation_id = e.repository_observation_id
              WHERE o.host_tool_use_id = ?1",
            [tool_use_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }
}

fn tool_event(event_name: &str, tool_use_id: &str, tool_name: &str, tool_input: Value) -> Value {
    json!({
        "hook_event_name": event_name,
        "session_id": "guard-regression-session",
        "turn_id": "guard-regression-turn",
        "tool_use_id": tool_use_id,
        "tool_name": tool_name,
        "tool_input": tool_input,
    })
}

fn write_repository_file(
    repository: &Path,
    relative: &str,
    content: &[u8],
) -> Result<(), Box<dyn Error>> {
    let path = repository.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn run_git(repository: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    run_git_with_paths(repository, args, &[])
}

fn run_git_with_paths(
    repository: &Path,
    args: &[&str],
    paths: &[&OsStr],
) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .args(paths)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "Git command failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn assert_complete_paths(observation: &RepositoryObservationRecord, expected_paths: &[&str]) {
    assert_eq!(observation.state, RepositoryObservationState::Complete);
    let result = observation
        .terminal_result
        .as_ref()
        .expect("complete observation has a terminal result");
    assert_eq!(
        result
            .delta
            .as_ref()
            .expect("complete observation has a delta")
            .paths
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        expected_paths
    );
}

#[test]
fn hidden_guard_help_lists_only_codex_record_phases() -> Result<(), Box<dyn Error>> {
    let output = run(&["_hook", "--help"])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    for phase in ["pre-tool", "post-tool", "prompt-capture"] {
        assert!(stdout.contains(phase));
        let phase_help = run(&["_hook", phase, "--help"])?;
        assert!(phase_help.status.success());
        let phase_stdout = String::from_utf8(phase_help.stdout)?;
        assert!(phase_stdout.contains("--host <HOST>"));
        assert!(phase_stdout.contains("--integration-profile <INTEGRATION_PROFILE>"));
        assert!(phase_stdout.contains("--host-output <HOST_OUTPUT>"));
        assert!(phase_stdout.contains("possible values: codex"));
        assert!(phase_stdout.contains("possible values: record"));
    }
    Ok(())
}

#[test]
fn hidden_guard_rejects_unknown_guard_phase_before_setup() -> Result<(), Box<dyn Error>> {
    let output = run(&["_hook", "unknown-phase", "--help"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("unrecognized subcommand"));
    Ok(())
}

#[test]
fn preexisting_dirty_tracked_and_untracked_state_stays_unattributed_across_calls(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardRepositoryFixture::new("dirty_noop")?;
    let repository = fixture.repository();
    write_repository_file(
        &repository,
        "tracked-dirty.txt",
        b"preexisting tracked change\n",
    )?;
    write_repository_file(
        &repository,
        "preexisting-untracked.txt",
        b"preexisting untracked content\n",
    )?;
    write_repository_file(
        &repository,
        "clean-modify.txt",
        b"second preexisting tracked change\n",
    )?;
    write_repository_file(
        &repository,
        "second-preexisting-untracked.txt",
        b"second preexisting untracked content\n",
    )?;
    let status_before = run_git(&repository, &["status", "--porcelain"])?;
    let tracked_before = fs::read(repository.join("tracked-dirty.txt"))?;
    let untracked_before = fs::read(repository.join("preexisting-untracked.txt"))?;
    let second_tracked_before = fs::read(repository.join("clean-modify.txt"))?;
    let second_untracked_before = fs::read(repository.join("second-preexisting-untracked.txt"))?;
    let callable = fixture.mcp_callable(AgentToolId::STATUS)?;

    for tool_use_id in ["noop-dirty-first", "noop-dirty-second"] {
        fixture.pre(tool_use_id, &callable, json!({}))?;
        fixture.post(tool_use_id, &callable, json!({}))?;
        let observation = fixture.observation(tool_use_id)?;
        assert_complete_paths(&observation, &[]);
        assert!(observation
            .terminal_result
            .as_ref()
            .is_some_and(|result| result.unrecorded_changes.is_empty()
                && result.expected_write_matches.is_empty()));
    }

    assert_eq!(fixture.count("expected_writes")?, 0);
    assert_eq!(fixture.count("unrecorded_changes")?, 0);
    assert_eq!(
        run_git(&repository, &["status", "--porcelain"])?,
        status_before
    );
    assert_eq!(
        fs::read(repository.join("tracked-dirty.txt"))?,
        tracked_before
    );
    assert_eq!(
        fs::read(repository.join("preexisting-untracked.txt"))?,
        untracked_before
    );
    assert_eq!(
        fs::read(repository.join("clean-modify.txt"))?,
        second_tracked_before
    );
    assert_eq!(
        fs::read(repository.join("second-preexisting-untracked.txt"))?,
        second_untracked_before
    );
    Ok(())
}

#[test]
fn invocation_changes_are_attributed_once_even_for_dirty_restore_and_commit_cases(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardRepositoryFixture::new("actual_deltas")?;
    let repository = fixture.repository();
    let callable = fixture.mcp_callable(AgentToolId::STATUS)?;
    write_repository_file(
        &repository,
        "tracked-dirty.txt",
        b"first preexisting tracked change\n",
    )?;
    write_repository_file(
        &repository,
        "preexisting-untracked.txt",
        b"first preexisting untracked content\n",
    )?;
    fs::remove_file(repository.join("restore.txt"))?;

    let cases = [
        ("delta-dirty-again", "tracked-dirty.txt"),
        ("delta-create", "created.txt"),
        ("delta-clean-modify", "clean-modify.txt"),
        ("delta-untracked-again", "preexisting-untracked.txt"),
        ("delta-delete", "delete.txt"),
        ("delta-restore", "restore.txt"),
        ("delta-commit", "commit.txt"),
    ];
    for (tool_use_id, path) in cases {
        fixture.pre(tool_use_id, &callable, json!({}))?;
        match tool_use_id {
            "delta-dirty-again" => {
                write_repository_file(&repository, path, b"second invocation tracked change\n")?
            }
            "delta-create" => {
                write_repository_file(&repository, path, b"created during invocation\n")?
            }
            "delta-clean-modify" => {
                write_repository_file(&repository, path, b"modified during invocation\n")?
            }
            "delta-untracked-again" => {
                write_repository_file(&repository, path, b"second untracked content\n")?
            }
            "delta-delete" => fs::remove_file(repository.join(path))?,
            "delta-restore" => write_repository_file(&repository, path, b"restore baseline\n")?,
            "delta-commit" => {
                write_repository_file(&repository, path, b"committed during invocation\n")?;
                run_git_with_paths(&repository, &["add", "--"], &[OsStr::new(path)])?;
                run_git(
                    &repository,
                    &["commit", "-q", "-m", "commit during invocation"],
                )?;
            }
            _ => unreachable!("closed test case"),
        }
        fixture.post(tool_use_id, &callable, json!({}))?;
        let observation = fixture.observation(tool_use_id)?;
        assert_complete_paths(&observation, &[path]);
        assert_eq!(
            observation
                .terminal_result
                .as_ref()
                .expect("terminal result")
                .unrecorded_changes
                .len(),
            1
        );
        assert_eq!(
            observation
                .terminal_result
                .as_ref()
                .expect("terminal result")
                .unrecorded_changes[0]
                .observed_paths
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            [path]
        );
    }
    assert_eq!(fixture.count("unrecorded_changes")?, cases.len() as i64);

    fixture.post("delta-commit", &callable, json!({}))?;
    assert_eq!(
        fixture.count("unrecorded_changes")?,
        cases.len() as i64,
        "an exact PostToolUse replay must not duplicate a finding"
    );

    fixture.pre("delta-following-noop", &callable, json!({}))?;
    fixture.post("delta-following-noop", &callable, json!({}))?;
    assert_complete_paths(&fixture.observation("delta-following-noop")?, &[]);
    assert_eq!(fixture.count("unrecorded_changes")?, cases.len() as i64);
    Ok(())
}

#[test]
fn unavailable_observations_never_fabricate_paths_or_findings() -> Result<(), Box<dyn Error>> {
    let fixture = GuardRepositoryFixture::new("unavailable")?;
    let repository = fixture.repository();
    let status_callable = fixture.mcp_callable(AgentToolId::STATUS)?;
    let list_callable = fixture.mcp_callable(AgentToolId::LIST_PROJECTS)?;
    let oversized_path = repository.join("oversized-unavailable.bin");
    let oversized = fs::File::create(&oversized_path)?;
    oversized.set_len(512 * 1024 * 1024 + 1)?;
    drop(oversized);

    let denied = fixture.pre(
        "unavailable-write",
        "Write",
        json!({"file_path": repository.join("src/export.rs")}),
    )?;
    let denied_output: Value = serde_json::from_slice(&denied.stdout)?;
    assert_eq!(
        denied_output.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&Value::String("deny".to_owned())),
        "{denied_output}"
    );
    let denied_observation = fixture.observation("unavailable-write")?;
    assert_eq!(
        denied_observation.state,
        RepositoryObservationState::Unavailable
    );
    assert!(denied_observation.delta.is_none());
    assert!(denied_observation
        .terminal_result
        .as_ref()
        .is_some_and(|result| result.delta.is_none() && result.unrecorded_changes.is_empty()));

    let read_only = fixture.pre("unavailable-read", &status_callable, json!({}))?;
    assert!(!String::from_utf8(read_only.stdout)?.contains("permissionDecision"));
    fixture.post("unavailable-read", &status_callable, json!({}))?;
    let read_observation = fixture.observation("unavailable-read")?;
    assert_eq!(
        read_observation.state,
        RepositoryObservationState::Unavailable
    );
    assert!(read_observation.delta.is_none());

    fs::remove_file(oversized_path)?;
    fixture.post("missing-pre", &status_callable, json!({}))?;
    let missing = fixture.observation("missing-pre")?;
    assert_eq!(missing.state, RepositoryObservationState::Unavailable);
    assert_eq!(
        missing
            .terminal_result
            .as_ref()
            .and_then(|result| result.unavailable_reason.as_deref()),
        Some("missing_open_observation")
    );
    assert!(missing.delta.is_none());

    fixture.pre("conflicting-post", &status_callable, json!({}))?;
    let conflicting_output = fixture.post("conflicting-post", &list_callable, json!({}))?;
    assert!(
        String::from_utf8_lossy(&conflicting_output.stdout).contains("unavailable"),
        "{}",
        String::from_utf8_lossy(&conflicting_output.stdout)
    );
    assert_eq!(
        fixture
            .observation_for("conflicting-post", &status_callable)?
            .state,
        RepositoryObservationState::Open
    );
    assert_eq!(
        fixture.core.conn()?.query_row(
            "SELECT COUNT(*)
               FROM repository_observations
              WHERE host_tool_use_id = ?1
                AND host_tool_name = ?2",
            ["conflicting-post", list_callable.as_str()],
            |row| row.get::<_, i64>(0),
        )?,
        0,
        "a conflicting PostToolUse must not fabricate a terminal delta"
    );
    assert_eq!(fixture.count("unrecorded_changes")?, 0);
    Ok(())
}

#[test]
fn observer_resource_failure_is_explicit_not_an_empty_success() -> Result<(), Box<dyn Error>> {
    let fixture = GuardRepositoryFixture::new("resource_limit")?;
    let repository = fixture.repository();
    let oversized = fs::File::create(repository.join("oversized.bin"))?;
    oversized.set_len(512 * 1024 * 1024 + 1)?;
    let callable = fixture.mcp_callable(AgentToolId::STATUS)?;

    fixture.pre("resource-limit", &callable, json!({}))?;
    let observation = fixture.observation("resource-limit")?;
    assert_eq!(observation.state, RepositoryObservationState::Unavailable);
    assert!(observation.delta.is_none());
    assert!(observation
        .unavailable_reason
        .is_some_and(|reason| reason.as_str() != "missing_open_observation"));
    assert_eq!(fixture.count("unrecorded_changes")?, 0);
    Ok(())
}

#[test]
fn expected_writes_match_only_their_exact_observation_and_changed_paths(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardRepositoryFixture::new("expected_writes")?;
    let repository = fixture.repository();
    let path_a = "src/export.rs";
    let path_b = "tests/export.rs";
    fixture.prepare_write_ticket("expected", &[path_a, path_b])?;
    let write_input = |path: &str| json!({"file_path": repository.join(path)});

    let empty_pre = fixture.pre("expected-empty", "Write", write_input(path_a))?;
    assert!(
        !String::from_utf8_lossy(&empty_pre.stdout).contains("permissionDecision"),
        "{}",
        String::from_utf8_lossy(&empty_pre.stdout)
    );
    fixture.post("expected-empty", "Write", write_input(path_a))?;
    assert_complete_paths(&fixture.observation("expected-empty")?, &[]);
    assert_eq!(
        fixture.expected_write_status("expected-empty")?,
        ("pending".to_owned(), None)
    );

    fixture.pre("expected-unavailable", "Write", write_input(path_a))?;
    let oversized_path = repository.join("post-snapshot-oversized.bin");
    let oversized = fs::File::create(&oversized_path)?;
    oversized.set_len(512 * 1024 * 1024 + 1)?;
    drop(oversized);
    fixture.post("expected-unavailable", "Write", write_input(path_a))?;
    let unavailable = fixture.observation("expected-unavailable")?;
    assert_eq!(unavailable.state, RepositoryObservationState::Unavailable);
    assert!(unavailable.delta.is_none());
    assert_eq!(
        fixture.expected_write_status("expected-unavailable")?,
        ("pending".to_owned(), None)
    );
    fs::remove_file(oversized_path)?;

    fixture.pre("expected-exact", "Write", write_input(path_a))?;
    write_repository_file(
        &repository,
        path_a,
        b"pub fn export() { println!(\"exact\"); }\n",
    )?;
    fixture.post("expected-exact", "Write", write_input(path_a))?;
    let exact = fixture.observation("expected-exact")?;
    assert_complete_paths(&exact, &[path_a]);
    assert_eq!(
        exact
            .terminal_result
            .as_ref()
            .expect("exact terminal result")
            .expected_write_matches
            .len(),
        1
    );
    assert!(exact
        .terminal_result
        .as_ref()
        .expect("exact terminal result")
        .unrecorded_changes
        .is_empty());
    assert_eq!(
        fixture.expected_write_status("expected-exact")?,
        ("matched".to_owned(), Some("[\"src/export.rs\"]".to_owned()))
    );

    for (tool_use_id, path, content) in [
        (
            "expected-adjacent-a",
            path_a,
            b"pub fn export() { println!(\"adjacent-a\"); }\n".as_slice(),
        ),
        (
            "expected-adjacent-b",
            path_b,
            b"#[test]\nfn export_works() { assert!(true); }\n".as_slice(),
        ),
    ] {
        fixture.pre(tool_use_id, "Write", write_input(path))?;
        write_repository_file(&repository, path, content)?;
        fixture.post(tool_use_id, "Write", write_input(path))?;
        let observation = fixture.observation(tool_use_id)?;
        assert_complete_paths(&observation, &[path]);
        let result = observation
            .terminal_result
            .as_ref()
            .expect("terminal result");
        assert_eq!(result.expected_write_matches.len(), 1);
        assert!(result.unrecorded_changes.is_empty());
    }

    fixture.pre("expected-partial", "Write", write_input(path_a))?;
    write_repository_file(
        &repository,
        path_a,
        b"pub fn export() { println!(\"partial-a\"); }\n",
    )?;
    write_repository_file(
        &repository,
        path_b,
        b"#[test]\nfn export_works() { assert_eq!(2 + 2, 4); }\n",
    )?;
    fixture.post("expected-partial", "Write", write_input(path_a))?;
    let partial = fixture.observation("expected-partial")?;
    assert_complete_paths(&partial, &[path_a, path_b]);
    let partial_result = partial
        .terminal_result
        .as_ref()
        .expect("partial terminal result");
    assert_eq!(partial_result.expected_write_matches.len(), 1);
    assert_eq!(
        partial_result.expected_write_matches[0]
            .matched_paths
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        [path_a]
    );
    assert_eq!(partial_result.unrecorded_changes.len(), 1);
    assert_eq!(
        partial_result.unrecorded_changes[0]
            .observed_paths
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        [path_b]
    );
    assert_eq!(fixture.count("expected_writes")?, 6);
    assert_eq!(fixture.count("unrecorded_changes")?, 1);
    Ok(())
}

#[test]
fn identical_filenames_in_distinct_repositories_do_not_cross_match() -> Result<(), Box<dyn Error>> {
    let first = GuardRepositoryFixture::new("repo_isolation_first")?;
    let second = GuardRepositoryFixture::new("repo_isolation_second")?;
    let path = "src/export.rs";
    first.prepare_write_ticket("first", &[path])?;
    second.prepare_write_ticket("second", &[path])?;

    for (fixture, content) in [
        (
            &first,
            b"pub fn export() { println!(\"first\"); }\n".as_slice(),
        ),
        (
            &second,
            b"pub fn export() { println!(\"second\"); }\n".as_slice(),
        ),
    ] {
        let input = json!({"file_path": fixture.repository().join(path)});
        fixture.pre("same-coordinate", "Write", input.clone())?;
        write_repository_file(&fixture.repository(), path, content)?;
        fixture.post("same-coordinate", "Write", input)?;
        let observation = fixture.observation("same-coordinate")?;
        assert_complete_paths(&observation, &[path]);
        let result = observation
            .terminal_result
            .as_ref()
            .expect("terminal result");
        assert_eq!(result.expected_write_matches.len(), 1);
        assert!(result.unrecorded_changes.is_empty());
        assert_eq!(fixture.count("expected_writes")?, 1);
        assert_eq!(fixture.count("unrecorded_changes")?, 0);
    }
    Ok(())
}
