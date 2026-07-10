use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_INTENT_SHARED,
    CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT, VERIFIED_STATUS_COMPLETE,
};
use volicord_store::guards::{
    guard_installation, list_guard_installations, upsert_guard_installation,
    GuardInstallationUpsert,
};
use volicord_store::{bootstrap::list_projects, core_pipeline::CoreProjectStore};
pub(crate) use volicord_test_support::core_fixtures::DEFAULT_PRODUCT_PATH;
use volicord_test_support::{
    core_fixtures::{
        answer_payload, supported_evidence_update, CoreFixture, UpdateScopeFixture,
        UserJudgmentFixture, DEFAULT_BASELINE_REF,
    },
    TempRuntimeHome,
};
use volicord_types::{
    chat_judgment_verification_code, ActorSource, BaselineRef, ChangeUnitId, ChangeUnitOperation,
    ChangeUnitUpdate, CheckCloseRequest, CloseAssessmentInput, CloseMutationIntent, CloseReason,
    CloseTaskRequest, IdempotencyKey, InitialScope, IntakeRequest, JudgmentKind,
    JudgmentPresentation, JudgmentRationale, JudgmentRequiredFor, ObservedChanges,
    OperationCategory, PrepareWriteRequest, ProjectId, ReconcileChangesRequest, RecordId,
    RecordRunRequest, RecordUserJudgmentRequest, RequestId, RequestUserJudgmentRequest,
    RequestedMode, ResumePolicy, RunKind, ScopeUpdate, StateRecordKind, StateRecordRef, TaskId,
    ToolEnvelope, UpdateScopeRequest, UserJudgmentContext, UserJudgmentOptionId, WriteTicketId,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING, VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

use super::{
    assertions::{assert_non_guarantees, assert_success, json_stdout, stderr, stdout},
    binary_fixture::volicord_bin,
    json::record_id,
};

#[cfg(unix)]
use super::{
    fake_hosts::{path_env, write_fake_codex},
    fake_mcp::write_basic_fake_mcp,
};

pub(crate) const PROMPT_CAPTURE_TEST_HOST_KIND: &str = "prompt_capture_test_host";
pub(crate) const CODEX_SESSION_START_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/session_start.json");
pub(crate) const CODEX_PRE_TOOL_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/pre_tool_write.json");
pub(crate) const CODEX_PRE_TOOL_BASH_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/pre_tool_bash_write.json");
pub(crate) const CODEX_POST_TOOL_BASH_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/post_tool_bash_write.json");
pub(crate) const CODEX_USER_PROMPT_JUDGMENT_EVENT: &str = include_str!(
    "../fixtures/host_contracts/codex/events/user_prompt_submit_judgment_command.json"
);
pub(crate) const CODEX_STOP_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/stop.json");
pub(crate) const CLAUDE_SESSION_START_EVENT: &str =
    include_str!("../fixtures/host_contracts/claude_code/events/session_start.json");
pub(crate) const CLAUDE_PRE_TOOL_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/claude_code/events/pre_tool_write.json");
pub(crate) const CLAUDE_PRE_TOOL_BASH_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/claude_code/events/pre_tool_bash_write.json");
pub(crate) const CLAUDE_POST_TOOL_BASH_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/claude_code/events/post_tool_bash_write.json");
pub(crate) const CLAUDE_USER_PROMPT_JUDGMENT_EVENT: &str = include_str!(
    "../fixtures/host_contracts/claude_code/events/user_prompt_submit_judgment_command.json"
);
pub(crate) const CLAUDE_STOP_EVENT: &str =
    include_str!("../fixtures/host_contracts/claude_code/events/stop.json");

pub(crate) struct GuardCliFixture {
    inner: CoreFixture,
    repo_root: PathBuf,
    repo_arg: String,
}

impl GuardCliFixture {
    pub(crate) fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let inner = CoreFixture::new(prefix)?;
        let repo_root = inner.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let repo_arg = repo_root.display().to_string();
        Ok(Self {
            inner,
            repo_root,
            repo_arg,
        })
    }

    pub(crate) fn with_prompt_capture(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let fixture = Self::new(prefix)?;
        fixture.install_guard_policy()?;
        Ok(fixture)
    }

    pub(crate) fn runtime_home(&self) -> &Path {
        self.inner.runtime_home_path()
    }

    pub(crate) fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub(crate) fn repo_arg(&self) -> &str {
        &self.repo_arg
    }

    pub(crate) fn project_id(&self) -> &str {
        self.inner.project_id()
    }

    pub(crate) fn connection_id(&self) -> &str {
        self.inner.connection_id()
    }

    pub(crate) fn guard_installation_id(&self) -> String {
        format!("guard_installation_cli_activation_{}", self.connection_id())
    }

    pub(crate) fn create_active_task(&self) -> Result<String, Box<dyn Error>> {
        let service = CoreService::new(self.runtime_home());
        let response = service.intake(
            self.inner
                .intake_request("req_guard_intake", "idem_guard_intake", false, Some(0)),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let task_id = record_id(&response.response_value["task_ref"])?;
        service.update_scope(
            self.inner.update_scope_request(UpdateScopeFixture {
                request_id: "req_guard_scope",
                idempotency_key: "idem_guard_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: &task_id,
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Guard fixture scope for src/export.rs.",
            }),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        Ok(task_id)
    }

    pub(crate) fn prepare_write(&self, task_id: &str) -> Result<(), Box<dyn Error>> {
        let service = CoreService::new(self.runtime_home());
        let state_version = self.inner.store()?.project_state()?.state_version;
        let response = service.prepare_write(
            self.inner.prepare_write_request(
                "req_guard_prepare_write",
                "idem_guard_prepare_write",
                Some(state_version),
                Some(task_id),
                None,
            ),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(response.response_value["decision"], "allowed");
        Ok(())
    }

    pub(crate) fn create_pending_authority_judgment(
        &self,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let task_id = self.create_active_task()?;
        let state_version = self.inner.store()?.project_state()?.state_version;
        let service = CoreService::new(self.runtime_home());
        let request_id = format!("req_guard_chat_judgment_{suffix}");
        let idempotency_key = format!("idem_guard_chat_judgment_{suffix}");
        let response = service.request_user_judgment(
            self.inner.user_judgment_request(UserJudgmentFixture {
                request_id: &request_id,
                idempotency_key: &idempotency_key,
                dry_run: false,
                expected_state_version: Some(state_version),
                task_id: &task_id,
                change_unit_id: None,
                judgment_kind: JudgmentKind::Cancellation,
            }),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        record_id(&response.response_value["user_judgment_ref"])
    }

    pub(crate) fn prompt_verification_code(
        &self,
        judgment_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let record = self
            .inner
            .store()?
            .user_judgment_record(judgment_id)?
            .expect("judgment should be stored");
        Ok(chat_judgment_verification_code(
            &record.project_id,
            &record.task_id,
            &record.judgment_id,
            &record.requested_at,
            self.connection_id(),
        ))
    }

    pub(crate) fn assert_recorded_prompt_judgment(
        &self,
        judgment_id: &str,
        expected_outcome: &str,
        expected_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        let record = self
            .inner
            .store()?
            .user_judgment_record(judgment_id)?
            .expect("judgment should be stored");
        assert_eq!(record.status, "resolved");
        assert_eq!(record.resolution_outcome.as_deref(), Some(expected_outcome));
        assert_eq!(
            record.resolution_machine_action.as_deref(),
            Some(expected_action)
        );
        assert_eq!(
            record.resolved_by_actor_source.as_deref(),
            Some("local_user")
        );
        assert_eq!(
            record.resolved_verification_basis.as_deref(),
            Some(VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK)
        );
        assert_eq!(
            record.resolved_assurance_level.as_deref(),
            Some("local_user_channel")
        );
        Ok(())
    }

    pub(crate) fn judgment_status(&self, judgment_id: &str) -> Result<String, Box<dyn Error>> {
        Ok(self.inner.user_judgment_status(judgment_id)?)
    }

    pub(crate) fn judgment_resolution(&self, judgment_id: &str) -> Result<Value, Box<dyn Error>> {
        self.inner.user_judgment_resolution(judgment_id)
    }

    pub(crate) fn set_judgment_basis_status(
        &self,
        judgment_id: &str,
        basis_status: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.inner.conn()?.execute(
            "UPDATE user_judgments
                SET basis_status = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![self.project_id(), judgment_id, basis_status],
        )?;
        Ok(())
    }

    pub(crate) fn set_judgment_expires_at(
        &self,
        judgment_id: &str,
        expires_at: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut request_json: Value = serde_json::from_str(
            &self
                .inner
                .store()?
                .user_judgment_record(judgment_id)?
                .expect("judgment should be stored")
                .request_json,
        )?;
        request_json["expires_at"] = json!(expires_at);
        self.inner.conn()?.execute(
            "UPDATE user_judgments
                SET request_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![self.project_id(), judgment_id, request_json.to_string()],
        )?;
        Ok(())
    }

    pub(crate) fn register_extra_connection(
        &self,
        connection_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        ensure_agent_connection(
            self.runtime_home(),
            AgentConnectionRegistration {
                connection_internal_id: connection_id.to_owned(),
                host_kind: HOST_KIND_CODEX.to_owned(),
                intent: CONNECTION_INTENT_SHARED.to_owned(),
                host_scope: HOST_SCOPE_PROJECT.to_owned(),
                server_name: format!("volicord-test-{connection_id}"),
                config_target: self
                    .runtime_home()
                    .join("agent-connections")
                    .join(connection_id)
                    .to_string_lossy()
                    .into_owned(),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: format!("fixture:{connection_id}"),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_connection_project(
            self.runtime_home(),
            ConnectionProjectRegistration {
                connection_internal_id: connection_id.to_owned(),
                project_id: self.project_id().to_owned(),
            },
        )?;
        Ok(())
    }

    pub(crate) fn install_guard_policy(&self) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_with(true, true, "configured")
    }

    pub(crate) fn install_guard_policy_for_host(
        &self,
        host_kind: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_for_connection_with_host(
            self.connection_id(),
            host_kind,
            true,
            true,
            "configured",
        )
    }

    pub(crate) fn install_guard_policy_with(
        &self,
        host_supports_prompt_capture: bool,
        prompt_capture_configured: bool,
        installation_status: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_for_connection(
            self.connection_id(),
            host_supports_prompt_capture,
            prompt_capture_configured,
            installation_status,
        )
    }

    pub(crate) fn install_guard_policy_for_connection(
        &self,
        connection_id: &str,
        host_supports_prompt_capture: bool,
        prompt_capture_configured: bool,
        installation_status: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_for_connection_with_host(
            connection_id,
            PROMPT_CAPTURE_TEST_HOST_KIND,
            host_supports_prompt_capture,
            prompt_capture_configured,
            installation_status,
        )
    }

    fn install_guard_policy_for_connection_with_host(
        &self,
        connection_id: &str,
        host_kind: &str,
        host_supports_prompt_capture: bool,
        prompt_capture_configured: bool,
        installation_status: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let guard_installation_id = format!("guard_installation_cli_activation_{connection_id}");
        let policy = json!({
            "schema": "volicord-policy-v1",
            "managed_by": "volicord",
            "host": host_kind,
            "selected_profile": "detective",
            "connection_id": connection_id,
            "guard_installation_id": guard_installation_id
        });
        let policy_hash = sha256_text(&serde_json::to_string(&policy)?);
        let policy_dir = self.repo_root.join(".volicord");
        fs::create_dir_all(&policy_dir)?;
        fs::write(
            policy_dir.join("policy.json"),
            serde_json::to_string_pretty(&policy)?,
        )?;
        upsert_guard_installation(
            self.runtime_home(),
            GuardInstallationUpsert {
                guard_installation_id: guard_installation_id.clone(),
                connection_internal_id: connection_id.to_owned(),
                project_id: Some(self.project_id().to_owned()),
                host_kind: host_kind.to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: json!({
                    "schema": "volicord-host-hook-capability-v1",
                    "policy_hash": policy_hash.clone(),
                    "host_capabilities": {
                        "user_prompt_submit_hook": host_supports_prompt_capture
                    },
                    "required_hook_phases": [
                        "session_start_hook",
                        "pre_tool_hook",
                        "post_tool_hook",
                        "user_prompt_submit_hook",
                        "stop_hook"
                    ],
                    "missing_required_hooks": [],
                    "prompt_capture": prompt_capture_configured
                })
                .to_string(),
                installation_status: installation_status.to_owned(),
                installed_at: Some("2026-06-30T03:59:00Z".to_owned()),
                last_checked_at: "2026-06-30T03:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok((guard_installation_id, policy_hash))
    }

    fn invocation(&self, operation_category: OperationCategory) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(self.project_id()),
            ActorSource::agent_connection(self.connection_id().to_owned()),
            operation_category,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
    }
}

#[cfg(unix)]
pub(crate) struct GuardedLifecycleFixture {
    _runtime_home: TempRuntimeHome,
    repo_root: PathBuf,
    repo_arg: String,
    project_id: String,
    connection_id: String,
    guard_installation_id: String,
    pub(crate) init_output: Value,
}

#[cfg(unix)]
impl GuardedLifecycleFixture {
    pub(crate) fn init(prefix: &str, mode: &str) -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let repo_root = runtime_home.create_product_repo("product-repo")?;
        fs::create_dir_all(repo_root.join(".git"))?;
        let repo_arg = repo_root
            .to_str()
            .ok_or("fixture product repo path should be UTF-8")?
            .to_owned();
        let bin_dir = runtime_home.path().join("bin");
        write_fake_codex(&bin_dir)?;
        write_basic_fake_mcp(&bin_dir)?;

        let mut args = vec![
            "init",
            "--host",
            "codex",
            "--repo",
            repo_arg.as_str(),
            "--profile",
            mode,
        ];
        args.push("--json");

        let output = Command::new(volicord_bin())
            .args(args)
            .env("VOLICORD_HOME", runtime_home.path())
            .env("PATH", path_env(&[bin_dir.as_path()]))
            .env("VOLICORD_TEST_CONNECTION_MODE", "workflow")
            .output()?;
        assert_success(&output);
        let init_output = json_stdout(&output)?;
        let connection_id = init_output["connection"]["connection_id"]
            .as_str()
            .expect("init should report connection_id")
            .to_owned();
        let projects = list_projects(runtime_home.path())?;
        assert_eq!(projects.len(), 1);
        let project_id = projects[0].project_id.clone();
        let guard_installations =
            list_guard_installations(runtime_home.path(), &connection_id, Some(&project_id))?;
        assert_eq!(guard_installations.len(), 1);
        let guard_installation_id = guard_installations[0].guard_installation_id.clone();
        mark_connection_verified(runtime_home.path(), &connection_id)?;

        Ok(Self {
            _runtime_home: runtime_home,
            repo_root,
            repo_arg,
            project_id,
            connection_id,
            guard_installation_id,
            init_output,
        })
    }

    pub(crate) fn runtime_home(&self) -> &Path {
        self._runtime_home.path()
    }

    pub(crate) fn repo_arg(&self) -> &str {
        &self.repo_arg
    }

    pub(crate) fn project_id(&self) -> &str {
        &self.project_id
    }

    pub(crate) fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub(crate) fn guard_installation_id(&self) -> &str {
        &self.guard_installation_id
    }

    pub(crate) fn session_id(&self) -> &str {
        "guard_lifecycle_session"
    }

    fn store(&self) -> Result<CoreProjectStore, Box<dyn Error>> {
        Ok(CoreProjectStore::open(
            self.runtime_home(),
            &ProjectId::new(&self.project_id),
        )?)
    }

    fn state_version(&self) -> Result<u64, Box<dyn Error>> {
        Ok(self.store()?.project_state()?.state_version)
    }

    fn service(&self) -> CoreService {
        CoreService::new(self.runtime_home())
    }

    fn invocation(&self, operation_category: OperationCategory) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(&self.project_id),
            ActorSource::agent_connection(self.connection_id.clone()),
            operation_category,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
        .with_session_id(self.session_id().to_owned())
    }

    fn user_invocation(&self) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(&self.project_id),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
    }

    fn envelope(
        &self,
        request_id: &str,
        idempotency_key: Option<&str>,
        expected_state_version: Option<u64>,
        task_id: Option<&str>,
    ) -> ToolEnvelope {
        ToolEnvelope {
            project_id: ProjectId::new(&self.project_id),
            task_id: task_id.map(TaskId::new).into(),
            request_id: RequestId::new(request_id),
            idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
            expected_state_version: expected_state_version.into(),
            dry_run: false,
            locale: Some("en-US".to_owned()).into(),
        }
    }

    pub(crate) fn activate_guard(&self, event_id: &str) -> Result<(), Box<dyn Error>> {
        let event = json!({
            "event_id": event_id,
            "session_id": self.session_id(),
            "connection_id": self.connection_id(),
            "guard_installation_id": self.guard_installation_id(),
            "host_kind": "codex",
            "timestamp": "2026-06-30T06:00:00Z"
        });
        let output = self.run_guard_event("session-start", &event)?;
        assert_success(&output);
        let value = json_stdout(&output)?;
        assert_eq!(value["decision"], "inject_context");
        let stored = guard_installation(self.runtime_home(), self.guard_installation_id())?
            .expect("guard installation should be stored");
        assert_eq!(stored.last_seen_phase.as_deref(), Some("session_start"));
        Ok(())
    }

    pub(crate) fn mark_required_hooks_supported(&self) -> Result<(), Box<dyn Error>> {
        let stored = guard_installation(self.runtime_home(), self.guard_installation_id())?
            .expect("guard installation should be stored");
        let mut capability = serde_json::from_str::<Value>(&stored.host_capability_json)?;
        capability["required_hook_phases"] = json!([
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ]);
        capability["missing_required_hooks"] = json!([]);
        capability["host_capabilities"] = json!({
            "stdio_mcp": true,
            "http_mcp": false,
            "session_start_hook": true,
            "pre_tool_hook": true,
            "post_tool_hook": true,
            "user_prompt_submit_hook": true,
            "stop_hook": true,
            "rule_file_support": true,
            "project_local_configuration": true
        });
        capability["prompt_capture"] = json!(true);
        upsert_guard_installation(
            self.runtime_home(),
            GuardInstallationUpsert {
                guard_installation_id: stored.guard_installation_id,
                connection_internal_id: stored.connection_internal_id,
                project_id: Some(self.project_id.clone()),
                host_kind: stored.host_kind,
                guard_mode: stored.guard_mode,
                host_capability_json: capability.to_string(),
                installation_status: "reload_required".to_owned(),
                installed_at: stored.installed_at,
                last_checked_at: "2026-06-30T05:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: stored.metadata_json,
            },
        )?;
        Ok(())
    }

    pub(crate) fn mark_required_hooks_missing(&self) -> Result<(), Box<dyn Error>> {
        let stored = guard_installation(self.runtime_home(), self.guard_installation_id())?
            .expect("guard installation should be stored");
        let mut capability = serde_json::from_str::<Value>(&stored.host_capability_json)?;
        capability["required_hook_phases"] = json!([
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ]);
        capability["missing_required_hooks"] = json!(["pre_tool_hook"]);
        capability["host_capabilities"] = json!({
            "stdio_mcp": true,
            "http_mcp": false,
            "session_start_hook": true,
            "pre_tool_hook": false,
            "post_tool_hook": true,
            "user_prompt_submit_hook": true,
            "stop_hook": true,
            "rule_file_support": true,
            "project_local_configuration": true
        });
        capability["prompt_capture"] = json!(true);
        upsert_guard_installation(
            self.runtime_home(),
            GuardInstallationUpsert {
                guard_installation_id: stored.guard_installation_id,
                connection_internal_id: stored.connection_internal_id,
                project_id: Some(self.project_id.clone()),
                host_kind: stored.host_kind,
                guard_mode: stored.guard_mode,
                host_capability_json: capability.to_string(),
                installation_status: "degraded".to_owned(),
                installed_at: stored.installed_at,
                last_checked_at: "2026-06-30T05:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: stored.metadata_json,
            },
        )?;
        Ok(())
    }

    pub(crate) fn run_guard_event(
        &self,
        phase: &str,
        event: &Value,
    ) -> Result<Output, Box<dyn Error>> {
        run_guard(
            self.runtime_home(),
            &self.repo_root,
            ["_hook", phase, "--repo", self.repo_arg()],
            event,
        )
    }

    pub(crate) fn create_task_with_change_unit(
        &self,
        suffix: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let service = self.service();
        let intake = service.intake(
            IntakeRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_intake"),
                    Some(&format!("idem_{suffix}_intake")),
                    Some(0),
                    None,
                ),
                plain_language_request: "Create a guarded lifecycle fixture task.".to_owned(),
                requested_mode: RequestedMode::Work,
                resume_policy: ResumePolicy::CreateNew,
                initial_scope: InitialScope {
                    boundary: "Exercise guarded lifecycle behavior in a temp repository."
                        .to_owned(),
                    non_goals: vec!["Changing unrelated files.".to_owned()],
                    acceptance_criteria: vec![
                        "The guarded lifecycle reaches the expected close state.".to_owned(),
                    ],
                },
                initial_context_refs: Vec::new(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let task_id = record_id(&intake.response_value["task_ref"])?;
        let after_intake = self.state_version()?;
        let mut fields = serde_json::Map::new();
        fields.insert(
            "scope_summary".to_owned(),
            Value::String("Guarded lifecycle scope for src/export.rs.".to_owned()),
        );
        fields.insert("affected_paths".to_owned(), json!([DEFAULT_PRODUCT_PATH]));
        let scope = service.update_scope(
            UpdateScopeRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_scope"),
                    Some(&format!("idem_{suffix}_scope")),
                    Some(after_intake),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                goal_summary: Some("Guarded lifecycle task.".to_owned()).into(),
                scope_update: Some(ScopeUpdate {
                    include: vec!["Guarded lifecycle fixture behavior.".to_owned()],
                    exclude: vec!["Unrelated repository behavior.".to_owned()],
                })
                .into(),
                scope_boundary: Some("Stay within the temp Product Repository.".to_owned()).into(),
                non_goals: Some(vec!["Do not touch external user files.".to_owned()]).into(),
                acceptance_criteria: Some(vec![
                    "The fixture close check reports the expected state.".to_owned(),
                ])
                .into(),
                autonomy_boundary: Some("Use only fixture inputs.".to_owned()).into(),
                baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
                change_unit: ChangeUnitUpdate {
                    operation: ChangeUnitOperation::CreateCurrent,
                    effect_contract: None,
                    fields,
                },
                related_scope_decision_refs: Vec::new(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let change_unit_id = record_id(&scope.response_value["change_unit_ref"])?;
        Ok((task_id, change_unit_id))
    }

    pub(crate) fn prepare_write(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let response = self.service().prepare_write(
            PrepareWriteRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_prepare"),
                    Some(&format!("idem_{suffix}_prepare")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                task_id: Some(TaskId::new(task_id)).into(),
                change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
                intended_operation: "local_product_file_update".to_owned(),
                intended_paths: vec![DEFAULT_PRODUCT_PATH.to_owned()],
                product_file_write_intended: true,
                sensitive_categories: Vec::new(),
                baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(response.response_value["decision"], "allowed");
        Ok(response.response_value["write_ticket_ref"]["record_id"]
            .as_str()
            .expect("write ticket id should be present")
            .to_owned())
    }

    pub(crate) fn apply_product_change(&self, contents: &str) -> Result<(), Box<dyn Error>> {
        let path = self.repo_root.join(DEFAULT_PRODUCT_PATH);
        fs::create_dir_all(path.parent().expect("fixture path should have a parent"))?;
        fs::write(path, format!("{contents}\n"))?;
        Ok(())
    }

    pub(crate) fn record_product_write_close_basis(
        &self,
        task_id: &str,
        change_unit_id: &str,
        write_ticket_id: &str,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        self.record_close_basis(task_id, change_unit_id, Some(write_ticket_id), true, suffix)
    }

    pub(crate) fn record_non_write_close_basis(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        self.record_close_basis(task_id, change_unit_id, None, false, suffix)
    }

    fn record_close_basis(
        &self,
        task_id: &str,
        change_unit_id: &str,
        write_ticket_id: Option<&str>,
        product_write_observed: bool,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let request = RecordRunRequest {
            envelope: self.envelope(
                &format!("req_{suffix}_run"),
                Some(&format!("idem_{suffix}_run")),
                Some(self.state_version()?),
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            change_unit_id: ChangeUnitId::new(change_unit_id),
            kind: RunKind::Implementation,
            run_id: None.into(),
            baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
            write_ticket_id: write_ticket_id.map(WriteTicketId::new).into(),
            summary: "Recorded guarded lifecycle fixture run.".to_owned(),
            observed_changes: ObservedChanges {
                changed_paths: if product_write_observed {
                    vec![DEFAULT_PRODUCT_PATH.to_owned()]
                } else {
                    Vec::new()
                },
                product_file_write_observed: product_write_observed,
                sensitive_categories: Vec::new(),
                baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
            },
            artifact_inputs: Vec::new(),
            evidence_updates: vec![supported_evidence_update(
                "Lifecycle close claim supported.",
            )],
            evidence_observations: Vec::new(),
            close_assessment: Some(CloseAssessmentInput {
                result_summary: "Lifecycle close claim supported.".to_owned(),
                result_refs: Vec::new(),
                residual_risks: Vec::new(),
                sensitive_categories: Vec::new(),
                recovery_constraints: Vec::new(),
            })
            .into(),
        };
        let response = self
            .service()
            .record_run(request, self.invocation(OperationCategory::AgentWorkflow))?;
        Ok(response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"))
    }

    pub(crate) fn request_final_acceptance(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let state_version = self.state_version()?;
        let response = self.service().request_user_judgment(
            RequestUserJudgmentRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_final"),
                    Some(&format!("idem_{suffix}_final")),
                    Some(state_version),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
                judgment_kind: JudgmentKind::FinalAcceptance,
                presentation: JudgmentPresentation::Short,
                question: "Does the user accept the current close basis?".to_owned(),
                options: None.into(),
                context: UserJudgmentContext {
                    summary: "The guarded lifecycle fixture is ready for final acceptance."
                        .to_owned(),
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                    visible_risks: Vec::new(),
                    constraints: vec![
                        "This answer applies only to the current fixture close basis.".to_owned(),
                    ],
                },
                affected_refs: vec![self.state_ref(
                    StateRecordKind::Task,
                    task_id,
                    Some(task_id),
                    Some(state_version),
                )],
                sensitive_action_scope: None.into(),
                required_for: vec![JudgmentRequiredFor::CloseComplete],
                expires_at: None.into(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        record_id(&response.response_value["user_judgment_ref"])
    }

    pub(crate) fn answer_pending_judgment_through_prompt(
        &self,
        task_id: &str,
        judgment_id: &str,
        event_id: &str,
        capture_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let records = self
            .store()?
            .user_judgment_records_for_task(&TaskId::new(task_id))?;
        let (index, record) = records
            .iter()
            .enumerate()
            .find(|(_, record)| record.judgment_id == judgment_id)
            .ok_or("pending judgment should be stored for task")?;
        let verification_code = chat_judgment_verification_code(
            &record.project_id,
            &record.task_id,
            &record.judgment_id,
            &record.requested_at,
            self.connection_id(),
        );
        let message = format!("Volicord: answer J-{} 1 {verification_code}", index + 1);
        let event = json!({
            "event_id": event_id,
            "prompt_capture_id": capture_id,
            "session_id": self.session_id(),
            "connection_id": self.connection_id(),
            "guard_installation_id": self.guard_installation_id(),
            "host_kind": "codex",
            "message": message,
            "timestamp": "2026-06-30T06:10:00Z"
        });
        let output = self.run_guard_event("prompt-capture", &event)?;
        assert_success(&output);
        let value = json_stdout(&output)?;
        assert_eq!(value["decision"], "inject_context");
        assert_eq!(
            value["result"]["recognized_judgment_command"]["resolution_outcome"],
            "accepted"
        );
        Ok(())
    }

    pub(crate) fn record_judgment_direct(
        &self,
        task_id: &str,
        judgment_id: &str,
        judgment_kind: JudgmentKind,
    ) -> Result<u64, Box<dyn Error>> {
        let response = self.service().record_user_judgment(
            RecordUserJudgmentRequest {
                envelope: self.envelope(
                    &format!("req_direct_record_{judgment_id}"),
                    Some(&format!("idem_direct_record_{judgment_id}")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                user_judgment_id: volicord_types::UserJudgmentId::new(judgment_id),
                judgment_kind,
                selected_option_id: UserJudgmentOptionId::new("accept"),
                answer: answer_payload(judgment_kind),
                rationale: JudgmentRationale {
                    summary: "The local user accepted the fixture judgment.".to_owned(),
                    selected_reason: Some(
                        "The fixture close basis was visible to the test user channel.".to_owned(),
                    )
                    .into(),
                    considered_alternatives: Vec::new(),
                    rejected_alternatives: Vec::new(),
                    assumptions: vec![
                        "This direct fixture answer covers only the pending judgment.".to_owned(),
                    ],
                    tradeoffs: vec![
                        "The fixture records acceptance only after the close basis is current."
                            .to_owned(),
                    ],
                    uncertainties: Vec::new(),
                    review_triggers: vec![
                        "Review if the fixture close basis changes before close.".to_owned(),
                    ],
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                },
                note: Some("Recorded by guarded lifecycle fixture.".to_owned()).into(),
                accepted_risks: Vec::new(),
            },
            self.user_invocation(),
        )?;
        assert_eq!(
            response.response_value["base"]["response_kind"], "result",
            "{}",
            response.response_value
        );
        Ok(response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"))
    }

    pub(crate) fn check_close(
        &self,
        task_id: &str,
    ) -> Result<volicord_core::PipelineResponse, Box<dyn Error>> {
        Ok(self.service().check_close(
            CheckCloseRequest {
                envelope: self.envelope(&format!("req_check_{task_id}"), None, None, Some(task_id)),
                task_id: TaskId::new(task_id),
            },
            self.invocation(OperationCategory::Read),
        )?)
    }

    pub(crate) fn close_task(
        &self,
        task_id: &str,
        suffix: &str,
    ) -> Result<volicord_core::PipelineResponse, Box<dyn Error>> {
        Ok(self.service().close_task(
            CloseTaskRequest {
                envelope: self.envelope(
                    &format!("req_close_{suffix}"),
                    Some(&format!("idem_close_{suffix}")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                intent: CloseMutationIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked).into(),
                superseding_task_id: None.into(),
                user_note: Some("Guarded lifecycle close.".to_owned()).into(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?)
    }

    pub(crate) fn reconcile_changes(
        &self,
        task_id: &str,
        suffix: &str,
    ) -> Result<volicord_core::PipelineResponse, Box<dyn Error>> {
        Ok(self.service().reconcile_changes(
            ReconcileChangesRequest {
                envelope: self.envelope(
                    &format!("req_reconcile_{suffix}"),
                    Some(&format!("idem_reconcile_{suffix}")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                resolution_requests: Vec::new(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?)
    }

    fn state_ref(
        &self,
        record_kind: StateRecordKind,
        record_id: &str,
        task_id: Option<&str>,
        state_version: Option<u64>,
    ) -> StateRecordRef {
        StateRecordRef {
            record_kind,
            record_id: RecordId::new(record_id),
            project_id: ProjectId::new(&self.project_id),
            task_id: task_id.map(TaskId::new).into(),
            state_version: state_version.into(),
        }
    }
}

pub(crate) fn prompt_event(
    fixture: &GuardCliFixture,
    event_id: &str,
    capture_id: &str,
    message: &str,
) -> Value {
    json!({
        "event_id": event_id,
        "prompt_capture_id": capture_id,
        "session_id": "guard_session_chat",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
        "message": message
    })
}

pub(crate) fn host_fixture_event(
    fixture: &GuardCliFixture,
    text: &str,
    event_id: &str,
    host_kind: &str,
) -> Result<Value, Box<dyn Error>> {
    let mut event: Value = serde_json::from_str(text)?;
    replace_repo_placeholder(&mut event, fixture.repo_arg());
    event["event_id"] = json!(event_id);
    event["connection_id"] = json!(fixture.connection_id());
    event["host_kind"] = json!(host_kind);
    Ok(event)
}

pub(crate) fn replace_repo_placeholder(value: &mut Value, repo_root: &str) {
    match value {
        Value::String(text) if text == "/repo" => {
            *text = repo_root.to_owned();
        }
        Value::String(text) => {
            if let Some(suffix) = text.strip_prefix("/repo/") {
                *text = format!("{repo_root}/{suffix}");
            }
        }
        Value::Array(items) => {
            for item in items {
                replace_repo_placeholder(item, repo_root);
            }
        }
        Value::Object(object) => {
            for item in object.values_mut() {
                replace_repo_placeholder(item, repo_root);
            }
        }
        _ => {}
    }
}

pub(crate) fn replace_prompt_verification_code(event: &mut Value, verification_code: &str) {
    if let Some(Value::String(prompt)) = event.get_mut("prompt") {
        *prompt = prompt.replace("#VOLICORD_VERIFICATION_CODE", verification_code);
    }
}

pub(crate) fn assert_host_native_json_stdout(
    output: &Output,
    expected_exit_code: i32,
) -> Result<Value, Box<dyn Error>> {
    assert_eq!(output.status.code(), Some(expected_exit_code));
    assert!(
        stderr(output).is_empty(),
        "host-native policy output should not write stderr: {}",
        stderr(output)
    );
    let text = stdout(output);
    assert!(
        text.trim_start().starts_with('{'),
        "host-native stdout should start with a JSON object, got {text:?}"
    );
    assert!(
        !text.contains("schema_version") && !text.contains("\"result\""),
        "host-native stdout must not contain Volicord wrapper JSON: {text}"
    );
    let value = serde_json::from_str::<Value>(&text)?;
    assert_no_volicord_wrapper_fields(&value);
    Ok(value)
}

fn assert_no_volicord_wrapper_fields(value: &Value) {
    let object = value
        .as_object()
        .expect("host-native stdout should be a JSON object");
    for key in [
        "schema_version",
        "phase",
        "allowed",
        "guard_event_id",
        "session_id",
        "result",
    ] {
        assert!(
            !object.contains_key(key),
            "host-native stdout must not expose Volicord wrapper field `{key}`: {value}"
        );
    }
}

pub(crate) fn assert_context_output(value: &Value, event_name: &str, expected_context: &str) {
    let object = value
        .as_object()
        .expect("context output should be an object");
    assert_eq!(object.len(), 1);
    let hook = value["hookSpecificOutput"]
        .as_object()
        .expect("context output should use hookSpecificOutput");
    assert_eq!(
        hook.get("hookEventName").and_then(Value::as_str),
        Some(event_name)
    );
    let context = hook
        .get("additionalContext")
        .and_then(Value::as_str)
        .expect("context output should include additionalContext");
    assert!(
        context.contains(expected_context),
        "expected context to contain {expected_context:?}, got {context:?}"
    );
}

pub(crate) fn assert_pre_tool_deny_output(value: &Value, expected_reason: &str) {
    assert!(is_host_native_pre_tool_deny(value));
    let hook = value["hookSpecificOutput"]
        .as_object()
        .expect("deny output should use hookSpecificOutput");
    let reason = hook
        .get("permissionDecisionReason")
        .and_then(Value::as_str)
        .expect("deny output should include permissionDecisionReason");
    assert!(
        reason.contains(expected_reason),
        "expected deny reason to contain {expected_reason:?}, got {reason:?}"
    );
    assert!(
        reason.contains("Does not prove:")
            && reason.contains("OS sandboxing")
            && reason.contains("actor identity proof"),
        "expected deny reason to disclose cooperative decision limits, got {reason:?}"
    );
}

pub(crate) fn is_host_native_pre_tool_deny(value: &Value) -> bool {
    let Some(hook) = value.get("hookSpecificOutput").and_then(Value::as_object) else {
        return false;
    };
    hook.get("hookEventName").and_then(Value::as_str) == Some("PreToolUse")
        && hook.get("permissionDecision").and_then(Value::as_str) == Some("deny")
        && hook
            .get("permissionDecisionReason")
            .and_then(Value::as_str)
            .is_some()
}

pub(crate) fn assert_cooperative_disclosure(value: &Value) {
    let disclosure = value
        .get("disclosure")
        .expect("host-hook output should include disclosure");
    assert_eq!(disclosure["guarantee_class"], "cooperative_host_decision");
    assert_non_guarantees(
        disclosure,
        &[
            "NotOsSandbox",
            "NotActorAttributionProof",
            "NotFullWritePrevention",
        ],
    );
}

pub(crate) fn assert_block_output(value: &Value, expected_reason: &str) {
    let object = value.as_object().expect("block output should be an object");
    assert_eq!(
        object.get("decision").and_then(Value::as_str),
        Some("block")
    );
    let reason = object
        .get("reason")
        .and_then(Value::as_str)
        .expect("block output should include reason");
    assert!(
        reason.contains(expected_reason),
        "expected block reason to contain {expected_reason:?}, got {reason:?}"
    );
}

pub(crate) fn run_host_guard(
    fixture: &GuardCliFixture,
    phase: &str,
    host_output: &str,
    event: &Value,
    extra_env: &[(&str, &str)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command
        .args([
            "_hook",
            phase,
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            host_output,
        ])
        .env("VOLICORD_HOME", fixture.runtime_home())
        .current_dir(fixture.repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(event.to_string().as_bytes())?;
    Ok(child.wait_with_output()?)
}

pub(crate) fn run_guard<const N: usize>(
    runtime_home: &Path,
    current_dir: &Path,
    args: [&str; N],
    event: &Value,
) -> Result<Output, Box<dyn Error>> {
    let mut child = Command::new(volicord_bin())
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(event.to_string().as_bytes())?;
    Ok(child.wait_with_output()?)
}

pub(crate) fn run_guard_file<const N: usize>(
    runtime_home: &Path,
    current_dir: &Path,
    args: [&str; N],
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin())
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .output()?)
}

pub(crate) fn sha256_text(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("sha256:{digest:x}")
}

#[cfg(unix)]
fn mark_connection_verified(
    runtime_home: &Path,
    connection_id: &str,
) -> Result<(), Box<dyn Error>> {
    let existing = agent_connection_record(runtime_home, connection_id)?
        .ok_or("initialized Agent Connection should be stored")?;
    ensure_agent_connection(
        runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: existing.connection_internal_id,
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: existing.config_target,
            mode: existing.mode,
            enabled: existing.enabled,
            managed_fingerprint: existing.managed_fingerprint,
            last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn assert_guard_init_state_is_installed_or_degraded(value: &Value) {
    let state = value["states"]["guard_installation"]
        .as_str()
        .expect("init output should include detective installation state");
    assert!(
        matches!(state, "configured" | "reload_required" | "degraded"),
        "unexpected guarded init state: {state}"
    );
}

pub(crate) fn assert_reason(value: &Value, code: &str) {
    assert!(
        value["result"]["reasons"]
            .as_array()
            .expect("reasons should be an array")
            .iter()
            .any(|reason| reason["code"] == code),
        "expected reason {code}, got {}",
        value["result"]["reasons"]
    );
}

#[cfg(unix)]
pub(crate) fn pre_tool_write_event(event_id: &str) -> Value {
    json!({
        "event_id": event_id,
        "session_id": "generated_hook_session",
        "tool_name": "Bash",
        "tool_call_id": format!("{event_id}_tool"),
        "command": "touch src/lib.rs",
        "paths": ["src/lib.rs"],
        "timestamp": "2026-07-01T00:00:00Z"
    })
}

#[cfg(unix)]
pub(crate) fn run_shell_hook_command(
    command_text: &str,
    runtime_home: &Path,
    current_dir: &Path,
    event: &Value,
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(command_text)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in envs {
        command.env(name, value);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().expect("hook stdin should be piped");
    stdin.write_all(event.to_string().as_bytes())?;
    drop(stdin);
    Ok(child.wait_with_output()?)
}

#[cfg(unix)]
pub(crate) fn run_executable_hook_command(
    executable: &Path,
    args: Vec<String>,
    runtime_home: &Path,
    current_dir: &Path,
    event: &Value,
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in envs {
        command.env(name, value);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().expect("hook stdin should be piped");
    stdin.write_all(event.to_string().as_bytes())?;
    drop(stdin);
    Ok(child.wait_with_output()?)
}

#[cfg(unix)]
pub(crate) fn expand_claude_project_command(
    command: &str,
    repo_root: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let relative = command
        .strip_prefix("${CLAUDE_PROJECT_DIR}/")
        .ok_or("Claude Code hook command must start with ${CLAUDE_PROJECT_DIR}/")?;
    Ok(repo_root.join(relative))
}
