use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_cli::host_integration::{MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1};
use volicord_core::{Clock, CoreService, InvocationContext, SystemClock};
use volicord_store::agent_connections::{
    add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
    ConnectionProjectRegistration, CONNECTION_INTENT_SHARED, CONNECTION_MODE_WORKFLOW,
    HOST_KIND_CODEX, HOST_SCOPE_PROJECT, VERIFIED_STATUS_COMPLETE,
};
use volicord_store::core_pipeline::StorageEffectCounts;
use volicord_store::guards::{upsert_guard_installation, GuardInstallationUpsert};
use volicord_test_support::core_fixtures::{
    artifact_input_for_handle, choice_user_action_resolution, CoreFixture,
    ObservationUserActionFixture, TaskOwnerJsonColumn, UpdateScopeFixture, UserActionFixture,
};
use volicord_types::{
    chat_user_action_verification_code, managed_host_session_id, ActorSource, ChangeUnitOperation,
    JudgmentKind, OperationCategory, ProjectId, UtcTimestamp,
    VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

use super::{
    assertions::{assert_non_guarantees, stderr, stdout},
    binary_fixture::{prepare_runtime_home, volicord_bin},
    json::record_id,
};

#[cfg(unix)]
pub(crate) use volicord_test_support::core_fixtures::DEFAULT_PRODUCT_PATH;

#[cfg(unix)]
use volicord_store::{
    agent_connections::agent_connection_record,
    bootstrap::list_projects,
    core_pipeline::CoreProjectStore,
    guards::{guard_installation, list_guard_installations},
};

#[cfg(unix)]
use volicord_test_support::{
    core_fixtures::{supported_evidence_update, DEFAULT_BASELINE_REF},
    TempRuntimeHome,
};

#[cfg(unix)]
use volicord_types::{
    AcceptanceCriterionId, AcceptanceCriterionInput, AcceptanceCriterionReplacement, ArtifactInput,
    ArtifactInputId, ArtifactInputSourceKind, BaselineRef, ChangeUnitId, ChangeUnitUpdate,
    CheckCloseRequest, CloseAssessmentInput, CloseMutationIntent, CloseReason, CloseTaskRequest,
    EvidenceRequirement, EvidenceTarget, IdempotencyKey, InitialScope, IntakeRequest,
    ObservedChanges, PrepareWriteRequest, ReconcileChangesRequest, RecordId, RecordRunRequest,
    RedactionState, RequestId, RequestedMode, ResumePolicy, RunKind, ScopeUpdate,
    StageArtifactRequest, StagedArtifactHandle, StateRecordKind, StateRecordRef, TaskId,
    ToolEnvelope, UpdateScopeRequest, UserActionChoiceDraft, UserActionContext, UserActionDraft,
    UserActionRequestId, UserActionRequiredFor, WriteTicketId,
};

#[cfg(unix)]
use super::{
    assertions::{assert_success, json_stdout},
    fake_hosts::{path_env, write_fake_codex},
    fake_mcp::write_basic_fake_mcp,
};

pub(crate) const PROMPT_CAPTURE_TEST_HOST_KIND: &str = "codex";
pub(crate) const CODEX_SESSION_START_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/session_start.json");
pub(crate) const CODEX_PRE_TOOL_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/pre_tool_write.json");
pub(crate) const CODEX_PRE_TOOL_BASH_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/pre_tool_bash_write.json");
pub(crate) const CODEX_POST_TOOL_BASH_WRITE_EVENT: &str =
    include_str!("../fixtures/host_contracts/codex/events/post_tool_bash_write.json");
pub(crate) const CODEX_USER_PROMPT_ACTION_EVENT: &str = include_str!(
    "../fixtures/host_contracts/codex/events/user_prompt_submit_user_action_command.json"
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
pub(crate) const CLAUDE_USER_PROMPT_ACTION_EVENT: &str = include_str!(
    "../fixtures/host_contracts/claude_code/events/user_prompt_submit_user_action_command.json"
);
pub(crate) const CLAUDE_STOP_EVENT: &str =
    include_str!("../fixtures/host_contracts/claude_code/events/stop.json");

#[cfg(unix)]
fn guard_fixture_command_args(
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host_output: &str,
    command_name: &str,
    policy_hash: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "_hook".to_owned(),
        command_name.to_owned(),
        "--repo".to_owned(),
        repo_root.display().to_string(),
        "--connection".to_owned(),
        connection_id.to_owned(),
        "--guard-installation".to_owned(),
        guard_installation_id.to_owned(),
        "--host".to_owned(),
        host_output.to_owned(),
        "--integration-profile".to_owned(),
        "detective".to_owned(),
    ];
    if let Some(policy_hash) = policy_hash {
        args.extend(["--policy-hash".to_owned(), policy_hash.to_owned()]);
    }
    args.extend(["--host-output".to_owned(), host_output.to_owned()]);
    args
}

#[cfg(unix)]
fn guard_fixture_command_line(command: &str, args: &[String]) -> String {
    std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(guard_fixture_shell_word)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(unix)]
fn guard_fixture_shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) struct GuardCliFixture {
    inner: CoreFixture,
    repo_root: PathBuf,
    repo_arg: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PromptEvidenceAction {
    pub(crate) task_id: String,
    pub(crate) user_action_request_id: String,
    pub(crate) target: volicord_types::EvidenceTarget,
    pub(crate) artifact_candidates: Vec<volicord_types::ArtifactRef>,
}

impl GuardCliFixture {
    pub(crate) fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let inner = CoreFixture::new(prefix)?;
        let selected_volicord = fs::canonicalize(volicord_bin())?;
        prepare_runtime_home(inner.runtime_home_path(), &selected_volicord)?;
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

    pub(crate) fn core_effect_counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        Ok(self.inner.counts()?)
    }

    pub(crate) fn replay_effect_snapshot(
        &self,
    ) -> Result<(StorageEffectCounts, String, Option<String>), Box<dyn Error>> {
        let (updated_at, active_task_id) = self.inner.conn()?.query_row(
            "SELECT updated_at, active_task_id
               FROM project_state
              WHERE project_id = ?1",
            [self.project_id()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((self.inner.counts()?, updated_at, active_task_id))
    }

    pub(crate) fn only_guard_event_id(&self, event_kind: &str) -> Result<String, Box<dyn Error>> {
        let connection = self.inner.conn()?;
        let mut statement = connection.prepare(
            "SELECT guard_event_id FROM guard_events WHERE event_kind = ?1 ORDER BY guard_event_id",
        )?;
        let ids = statement
            .query_map([event_kind], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        match ids.as_slice() {
            [id] => Ok(id.clone()),
            _ => Err(format!(
                "expected exactly one {event_kind} GuardEvent, found {}",
                ids.len()
            )
            .into()),
        }
    }

    pub(crate) fn corrupt_current_close_basis(
        &self,
        task_id: &str,
        raw_json: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.inner.set_task_owner_json_raw(
            task_id,
            TaskOwnerJsonColumn::CurrentCloseBasis,
            raw_json,
        )?;
        Ok(())
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

    pub(crate) fn create_additional_active_task(
        &self,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let service = CoreService::new(self.runtime_home());
        let state_version = self.inner.store()?.project_state()?.state_version;
        let request_id = format!("req_guard_additional_intake_{suffix}");
        let idempotency_key = format!("idem_guard_additional_intake_{suffix}");
        let response = service.intake(
            self.inner
                .intake_request(&request_id, &idempotency_key, false, Some(state_version)),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        record_id(&response.response_value["task_ref"])
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

    pub(crate) fn create_pending_user_action(
        &self,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let task_id = self.create_active_task()?;
        let state_version = self.inner.store()?.project_state()?.state_version;
        let service = CoreService::new(self.runtime_home());
        let request_id = format!("req_guard_chat_user_action_{suffix}");
        let idempotency_key = format!("idem_guard_chat_user_action_{suffix}");
        let response = service.request_user_action(
            self.inner.user_action_request(UserActionFixture {
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
        response.response_value["user_action_request_summary"]["user_action_request_id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "safe user-action request summary should identify the request".into())
    }

    pub(crate) fn create_pending_evidence_observation(
        &self,
        suffix: &str,
    ) -> Result<PromptEvidenceAction, Box<dyn Error>> {
        self.create_pending_evidence_observation_with_marker(suffix, None)
    }

    pub(crate) fn create_pending_sensitive_evidence_observation(
        &self,
        suffix: &str,
        marker: &str,
    ) -> Result<PromptEvidenceAction, Box<dyn Error>> {
        self.create_pending_evidence_observation_with_marker(suffix, Some(marker))
    }

    fn create_pending_evidence_observation_with_marker(
        &self,
        suffix: &str,
        sensitive_marker: Option<&str>,
    ) -> Result<PromptEvidenceAction, Box<dyn Error>> {
        let task_id = self.create_active_task()?;
        let change_unit_id = self
            .inner
            .current_change_unit_id(&task_id)?
            .ok_or("guard evidence fixture should have a current Change Unit")?;
        let criteria = self
            .inner
            .store()?
            .active_acceptance_criteria(&volicord_types::TaskId::new(&task_id))?;
        let [criterion] = criteria.as_slice() else {
            return Err(format!(
                "guard evidence fixture expected one acceptance criterion, found {}",
                criteria.len()
            )
            .into());
        };
        let target = if let Some(marker) = sensitive_marker {
            volicord_types::EvidenceTarget::SupplementalClaim {
                evidence_claim_id: volicord_types::EvidenceClaimId::new(format!(
                    "claim_guard_sensitive_{suffix}"
                )),
                statement: format!(
                    "An API key must be handled only in a user-only channel: {marker}"
                ),
            }
        } else {
            volicord_types::EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id: volicord_types::AcceptanceCriterionId::new(
                    &criterion.acceptance_criterion_id,
                ),
            }
        };
        let service = CoreService::new(self.runtime_home());
        let mut staged_handles = Vec::new();
        let display_names = if let Some(marker) = sensitive_marker {
            [
                format!("credential-material-{marker}.txt"),
                "ordinary-observation-candidate-b.txt".to_owned(),
            ]
        } else {
            [
                "prompt-observation-candidate-a.txt".to_owned(),
                "prompt-observation-candidate-b.txt".to_owned(),
            ]
        };
        for (index, (display_name, contents)) in display_names
            .into_iter()
            .zip([
                "First prompt-capture observation candidate.",
                "Selected prompt-capture observation candidate.",
            ])
            .enumerate()
        {
            let state_version = self.inner.store()?.project_state()?.state_version;
            let request_id = format!("req_guard_prompt_evidence_stage_{suffix}_{index}");
            let idempotency_key = format!("idem_guard_prompt_evidence_stage_{suffix}_{index}");
            let mut request = self.inner.stage_artifact_request(
                &request_id,
                Some(&idempotency_key),
                false,
                Some(state_version),
                &task_id,
            );
            request.display_name = display_name;
            request.safe_bytes_or_notice = contents.to_owned();
            request.relation_hint = Some("prompt_capture_observation_candidate".to_owned()).into();
            let response = service
                .stage_artifact(request, self.invocation(OperationCategory::AgentWorkflow))?;
            staged_handles.push(serde_json::from_value::<
                volicord_types::StagedArtifactHandle,
            >(
                response.response_value["staged_artifact_handle"].clone()
            )?);
        }

        let state_version = self.inner.store()?.project_state()?.state_version;
        let run_request_id = format!("req_guard_prompt_evidence_run_{suffix}");
        let run_idempotency_key = format!("idem_guard_prompt_evidence_run_{suffix}");
        let mut run = self.inner.record_run_request(
            &run_request_id,
            &run_idempotency_key,
            false,
            Some(state_version),
            &task_id,
            &change_unit_id,
        );
        run.summary = "Register canonical prompt-capture observation candidates.".to_owned();
        run.artifact_inputs = staged_handles
            .into_iter()
            .enumerate()
            .map(|(index, handle)| {
                let mut input = artifact_input_for_handle(
                    &format!("artifact_input_guard_prompt_evidence_{suffix}_{index}"),
                    handle,
                    Some("prompt_capture_observation_candidate"),
                    None,
                );
                input.evidence_target = Some(target.clone()).into();
                input
            })
            .collect();
        let recorded =
            service.record_run(run, self.invocation(OperationCategory::AgentWorkflow))?;
        let artifact_candidates = recorded.response_value["registered_artifacts"]
            .as_array()
            .ok_or("record_run should expose registered artifacts")?
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<Vec<volicord_types::ArtifactRef>, _>>()?;
        assert_eq!(artifact_candidates.len(), 2);

        let state_version = self.inner.store()?.project_state()?.state_version;
        let request_id = format!("req_guard_prompt_evidence_action_{suffix}");
        let idempotency_key = format!("idem_guard_prompt_evidence_action_{suffix}");
        let requested = service.request_user_action(
            self.inner
                .observation_user_action_request(ObservationUserActionFixture {
                    request_id: &request_id,
                    idempotency_key: &idempotency_key,
                    dry_run: false,
                    expected_state_version: Some(state_version),
                    task_id: &task_id,
                    change_unit_id: &change_unit_id,
                    target_candidates: vec![target.clone()],
                    artifact_candidate_ids: artifact_candidates
                        .iter()
                        .map(|artifact| artifact.artifact_id.clone())
                        .collect(),
                }),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        Ok(PromptEvidenceAction {
            task_id,
            user_action_request_id: requested.response_value["user_action_request_summary"]
                ["user_action_request_id"]
                .as_str()
                .ok_or("safe user-action request summary should identify the request")?
                .to_owned(),
            target,
            artifact_candidates,
        })
    }

    pub(crate) fn prompt_verification_code(
        &self,
        user_action_request_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let store = self.inner.store()?;
        let now = SystemClock.project_now(&store)?;
        let record = store
            .user_action_record(user_action_request_id, &now)?
            .expect("user action should be stored");
        Ok(chat_user_action_verification_code(
            &record.request.project_id,
            &record.request.task_id,
            &record.request.user_action_request_id,
            &record.request.requested_at,
            self.connection_id(),
        ))
    }

    pub(crate) fn assert_resolved_prompt_user_action(
        &self,
        user_action_request_id: &str,
        expected_outcome: &str,
        expected_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        assert_eq!(
            self.inner.user_action_status(user_action_request_id)?,
            "resolved"
        );
        assert_eq!(
            self.inner
                .user_action_resolution_outcome(user_action_request_id)?,
            Some(expected_outcome.to_owned())
        );
        assert_eq!(
            self.inner
                .user_action_resolution_machine_action(user_action_request_id)?,
            Some(expected_action.to_owned())
        );
        let store = self.inner.store()?;
        let now = SystemClock.project_now(&store)?;
        let record = store
            .user_action_record(user_action_request_id, &now)?
            .expect("resolved user action should be stored")
            .resolution
            .expect("user action resolution should be stored");
        assert_eq!(record.resolved_by_actor_source, "local_user");
        assert_eq!(
            record.resolved_verification_basis,
            VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK
        );
        assert_eq!(record.resolved_assurance_level, "local_user_channel");
        Ok(())
    }

    pub(crate) fn assert_resolved_prompt_evidence_action(
        &self,
        action: &PromptEvidenceAction,
        expected_artifact: &volicord_types::ArtifactRef,
        expected_relevance: volicord_types::EvidenceRelevanceStatus,
        expected_summary: &str,
    ) -> Result<(), Box<dyn Error>> {
        let store = self.inner.store()?;
        let now = SystemClock.project_now(&store)?;
        let records = store
            .user_action_records_for_task(&volicord_types::TaskId::new(&action.task_id), &now)?;
        assert_eq!(
            records.len(),
            1,
            "prompt capture must not duplicate requests"
        );
        let record = records
            .iter()
            .find(|record| record.request.user_action_request_id == action.user_action_request_id)
            .ok_or("prompt evidence action should remain stored")?;
        assert_eq!(record.status, volicord_types::UserActionStatus::Resolved);
        let resolution = record
            .resolution
            .as_ref()
            .ok_or("prompt evidence action should have a stored resolution")?;
        assert_eq!(resolution.resolved_by_actor_source, "local_user");
        assert_eq!(
            resolution.channel_kind,
            volicord_types::UserActionChannelKind::PromptCapture
        );
        assert_eq!(
            resolution.resolved_verification_basis,
            VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK
        );
        assert_eq!(resolution.resolved_assurance_level, "local_user_channel");
        let body: volicord_types::UserActionResolutionBody =
            serde_json::from_str(&resolution.resolution_json)?;
        let volicord_types::UserActionResolutionBody::EvidenceObservation { observation } = body
        else {
            return Err("prompt resolution should be an evidence observation".into());
        };
        assert_eq!(observation.target, action.target);
        assert_eq!(observation.relevance_status, expected_relevance);
        assert_eq!(observation.summary, expected_summary);
        assert_eq!(
            observation.output_artifact_refs,
            vec![expected_artifact.clone()],
            "prompt capture must preserve the exact historical artifact ref"
        );
        Ok(())
    }

    pub(crate) fn user_action_status(
        &self,
        user_action_request_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        Ok(self.inner.user_action_status(user_action_request_id)?)
    }

    pub(crate) fn user_action_resolution(
        &self,
        user_action_request_id: &str,
    ) -> Result<Value, Box<dyn Error>> {
        self.inner.user_action_resolution(user_action_request_id)
    }

    pub(crate) fn set_user_action_basis_status(
        &self,
        user_action_request_id: &str,
        basis_status: &str,
    ) -> Result<(), Box<dyn Error>> {
        let basis_json: String = self.inner.conn()?.query_row(
            "SELECT basis_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            rusqlite::params![self.project_id(), user_action_request_id],
            |row| row.get(0),
        )?;
        let mut basis: Value = serde_json::from_str(&basis_json)?;
        basis["coordinates"]["compatibility_status"] = json!(basis_status);
        self.inner.conn()?.execute(
            "UPDATE user_action_requests
                SET basis_status = ?3,
                    basis_json = ?4
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            rusqlite::params![
                self.project_id(),
                user_action_request_id,
                basis_status,
                basis.to_string()
            ],
        )?;
        Ok(())
    }

    pub(crate) fn expire_user_action_at_core_clock(
        &self,
        user_action_request_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let store = self.inner.store()?;
        let current_core_now = SystemClock.project_now(&store)?;
        let mut conn = self.inner.conn()?;
        let tx = conn.transaction()?;
        let (requested_at, request_json): (String, String) = tx.query_row(
            "SELECT requested_at
                    , request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            rusqlite::params![self.project_id(), user_action_request_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let requested_at = UtcTimestamp::parse(&requested_at)?;
        let minimum_expiry = UtcTimestamp::from_datetime(
            *requested_at.as_datetime() + chrono::Duration::milliseconds(1),
        );
        let expires_at = std::cmp::max(current_core_now, minimum_expiry);
        let persisted_floor: String = tx.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            rusqlite::params![self.project_id()],
            |row| row.get(0),
        )?;
        let persisted_floor = UtcTimestamp::parse(&persisted_floor)?;
        let clock_floor = std::cmp::max(persisted_floor, expires_at.clone());
        let mut request_json: Value = serde_json::from_str(&request_json)?;
        request_json["expires_at"] = json!(expires_at.to_string());
        let request_changed = tx.execute(
            "UPDATE user_action_requests
                SET request_json = ?3,
                    expires_at = ?4
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            rusqlite::params![
                self.project_id(),
                user_action_request_id,
                request_json.to_string(),
                expires_at.to_string()
            ],
        )?;
        if request_changed != 1 {
            return Err("test fixture failed to expire exactly one user-action request".into());
        }
        let project_changed = tx.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            rusqlite::params![self.project_id(), clock_floor.to_string()],
        )?;
        if project_changed != 1 {
            return Err("test fixture failed to advance exactly one project clock floor".into());
        }
        tx.commit()?;
        Ok(expires_at.to_string())
    }

    pub(crate) fn register_extra_connection(
        &self,
        connection_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.register_extra_connection_for_host(connection_id, HOST_KIND_CODEX)
    }

    pub(crate) fn register_extra_connection_for_host(
        &self,
        connection_id: &str,
        host_kind: &str,
    ) -> Result<(), Box<dyn Error>> {
        ensure_agent_connection(
            self.runtime_home(),
            AgentConnectionRegistration {
                connection_internal_id: connection_id.to_owned(),
                host_kind: host_kind.to_owned(),
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

    pub(crate) fn install_guard_policy_for_connection_and_host(
        &self,
        connection_id: &str,
        host_kind: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_for_connection_with_host(
            connection_id,
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
        let registry = rusqlite::Connection::open(volicord_store::sqlite::registry_db_path(
            self.runtime_home(),
        ))?;
        let updated = registry.execute(
            "UPDATE agent_connections
                SET host_kind = ?1
              WHERE connection_internal_id = ?2",
            rusqlite::params![host_kind, connection_id],
        )?;
        if updated != 1 {
            return Err(format!("fixture connection {connection_id} should exist").into());
        }
        let policy_host = match host_kind {
            "claude_code" => "claude-code",
            other => other,
        };
        let root_resolution_basis = if host_kind == "claude_code" {
            "claude_project_dir"
        } else {
            "git_work_tree"
        };
        let hook_command_path_basis = if host_kind == "claude_code" {
            "claude_project_dir"
        } else {
            "git_root_runtime"
        };
        let phases = [
            ("session_start_hook", "session_start", "session-start"),
            ("pre_tool_hook", "pre_tool", "pre-tool"),
            ("post_tool_hook", "post_tool", "post-tool"),
            (
                "user_prompt_submit_hook",
                "prompt_capture",
                "prompt-capture",
            ),
            ("stop_hook", "stop", "stop"),
        ];
        let host_hook_commands = phases
            .iter()
            .filter(|(phase, _, _)| {
                *phase != "user_prompt_submit_hook" || host_supports_prompt_capture
            })
            .map(|(phase, policy_key, command_name)| {
                let hook_directory = if host_kind == "claude_code" {
                    ".claude/hooks"
                } else {
                    ".codex/hooks"
                };
                let expected_phase_wrapper_path = self
                    .repo_root
                    .join(hook_directory)
                    .join(format!("volicord-{command_name}.sh"));
                let expected_wrapper_path = if host_kind == "codex" {
                    self.repo_root.join(".codex/hooks/volicord-dispatch.sh")
                } else {
                    expected_phase_wrapper_path.clone()
                };
                let (command_shape, command, args) = if host_kind == "claude_code" {
                    (
                        "exec_form",
                        format!(
                            "${{CLAUDE_PROJECT_DIR}}/.claude/hooks/volicord-{command_name}.sh"
                        ),
                        json!([]),
                    )
                } else {
                    (
                        "shell_command_string",
                        format!(
                            "sh -c '{}'",
                            format!(
                                "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" {command_name}"
                            )
                            .replace('\'', "'\\''")
                        ),
                        Value::Null,
                    )
                };
                json!({
                    "host_kind": host_kind,
                    "phase": phase,
                    "purpose": "detective_guard",
                    "policy_key": policy_key,
                    "command_shape": command_shape,
                    "command": command,
                    "args": args,
                    "expected_wrapper_path": expected_wrapper_path,
                    "expected_phase_wrapper_path": expected_phase_wrapper_path,
                    "root_resolution_basis": root_resolution_basis,
                    "hook_command_path_basis": hook_command_path_basis,
                    "cwd_independent": true,
                    "subdirectory_safe": true,
                    "wrapper_resolution_status": "ok",
                    "verification": {
                        "basis_verified_by": "test_fixture",
                        "host_contract_source": "test_fixture",
                    },
                })
            })
            .collect::<Vec<_>>();
        let root_phases = host_hook_commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command["phase"],
                    "root_resolution_basis": command["root_resolution_basis"],
                    "hook_command_path_basis": command["hook_command_path_basis"],
                    "cwd_independent": command["cwd_independent"],
                    "subdirectory_safe": command["subdirectory_safe"],
                    "wrapper_resolution_status": command["wrapper_resolution_status"],
                })
            })
            .collect::<Vec<_>>();
        let safety_commands = host_hook_commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command["phase"],
                    "hook_command_path_basis": command["hook_command_path_basis"],
                    "cwd_independent": command["cwd_independent"],
                    "subdirectory_safe": command["subdirectory_safe"],
                    "wrapper_resolution_status": command["wrapper_resolution_status"],
                })
            })
            .collect::<Vec<_>>();
        let commands = json!({
            "session_start": {"command": "volicord", "args": guard_fixture_command_args(&self.repo_root, connection_id, &guard_installation_id, policy_host, "session-start", None)},
            "pre_tool": {"command": "volicord", "args": guard_fixture_command_args(&self.repo_root, connection_id, &guard_installation_id, policy_host, "pre-tool", None)},
            "post_tool": {"command": "volicord", "args": guard_fixture_command_args(&self.repo_root, connection_id, &guard_installation_id, policy_host, "post-tool", None)},
            "prompt_capture": {"command": "volicord", "args": guard_fixture_command_args(&self.repo_root, connection_id, &guard_installation_id, policy_host, "prompt-capture", None)},
            "stop": {"command": "volicord", "args": guard_fixture_command_args(&self.repo_root, connection_id, &guard_installation_id, policy_host, "stop", None)}
        });
        let policy = json!({
            "schema": "volicord-policy-v1",
            "managed_by": "volicord",
            "storage_scope": "local_overlay",
            "connection_intent": "shared",
            "host": policy_host,
            "repo_root": self.repo_root.display().to_string(),
            "selected_profile": "detective",
            "connection_id": connection_id,
            "guard_installation_id": guard_installation_id,
            "mcp": {"command": "volicord", "args": ["mcp", "--stdio"], "env": {}},
            "host_hook": {"enabled": true, "commands": commands}
        });
        let policy_hash = sha256_text(&serde_json::to_string(&policy)?);
        let policy_dir = self.repo_root.join(".volicord");
        fs::create_dir_all(&policy_dir)?;
        fs::write(
            policy_dir.join("policy.json"),
            serde_json::to_string_pretty(&policy)?,
        )?;
        let mut files = host_hook_commands
            .iter()
            .map(|command| {
                let command_name = phases
                    .iter()
                    .find_map(|(_, policy_key, command_name)| {
                        (command["policy_key"].as_str() == Some(*policy_key))
                            .then_some(*command_name)
                    })
                    .expect("fixture command policy key must be canonical");
                let wrapper_args = guard_fixture_command_args(
                    &self.repo_root,
                    connection_id,
                    &guard_installation_id,
                    policy_host,
                    command_name,
                    Some(&policy_hash),
                );
                json!({
                    "kind": "host_hook_wrapper",
                    "path": command["expected_phase_wrapper_path"],
                    "status": "unchanged",
                    "content_hash": "wrapper-hash",
                    "ownership": "managed_script",
                    "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                    "executable_required": true,
                    "managed_script_command": guard_fixture_command_line("volicord", &wrapper_args),
                    "host_kind": policy_host,
                    "phase": command["policy_key"],
                    "purpose": "detective_guard",
                    "connection_id": connection_id,
                    "guard_installation_id": guard_installation_id.as_str(),
                    "policy_hash": policy_hash.as_str(),
                    "host_output": policy_host,
                })
            })
            .collect::<Vec<_>>();
        files.push(json!({
            "kind": "volicord_policy",
            "path": policy_dir.join("policy.json"),
            "status": "unchanged",
            "content_hash": "policy-file-hash",
            "ownership": "managed_json",
        }));
        if host_kind == "codex" {
            files.extend([
                json!({
                    "kind": "host_hook_dispatch",
                    "path": self.repo_root.join(".codex/hooks/volicord-dispatch.sh"),
                    "status": "unchanged",
                    "content_hash": "dispatch-hash",
                    "ownership": "managed_script",
                    "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                    "executable_required": true,
                    "managed_script_role": "codex_dispatch",
                    "host_kind": "codex",
                    "phase": "dispatch",
                }),
                json!({
                    "kind": "host_hook_config",
                    "path": self.repo_root.join(".codex/hooks.json"),
                    "status": "unchanged",
                    "content_hash": "config-hash",
                    "ownership": "managed_json",
                }),
                json!({
                    "kind": "host_rule_instruction",
                    "path": self.repo_root.join(".codex/rules/volicord.rules"),
                    "status": "unchanged",
                    "content_hash": "rule-hash",
                    "ownership": "managed_block",
                    "managed_marker_start": "# BEGIN VOLICORD MANAGED CODEX RULES",
                    "managed_marker_end": "# END VOLICORD MANAGED CODEX RULES",
                }),
            ]);
        } else if host_kind == "claude_code" {
            files.extend([
                json!({
                    "kind": "host_hook_config",
                    "path": self.repo_root.join(".claude/settings.json"),
                    "status": "unchanged",
                    "content_hash": "config-hash",
                    "ownership": "managed_json_projection",
                    "managed_projection": "claude_code_settings_hooks",
                    "managed_projection_json": "{}",
                }),
                json!({
                    "kind": "host_rule_instruction",
                    "path": self.repo_root.join(".claude/rules/volicord.md"),
                    "status": "unchanged",
                    "content_hash": "rule-hash",
                    "ownership": "managed_block",
                    "managed_marker_start": "<!-- BEGIN VOLICORD MANAGED GUIDANCE -->",
                    "managed_marker_end": "<!-- END VOLICORD MANAGED GUIDANCE -->",
                }),
            ]);
        }
        upsert_guard_installation(
            self.runtime_home(),
            GuardInstallationUpsert {
                guard_installation_id: guard_installation_id.clone(),
                connection_internal_id: connection_id.to_owned(),
                project_id: Some(self.project_id().to_owned()),
                host_kind: host_kind.to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: json!({
                    "schema": "volicord-host-hook-capability-v2",
                    "policy_hash": policy_hash.clone(),
                    "selected_profile": "detective",
                    "connection_intent": "shared",
                    "native_host_output_adapter": if matches!(host_kind, "codex" | "claude_code") { policy_host } else { "none" },
                    "native_host_output_adapter_config_verified": matches!(host_kind, "codex" | "claude_code"),
                    "final_output_authority_disclosure_implementation_available": matches!(host_kind, "codex" | "claude_code"),
                    "bash_shell_mutation_coverage": true,
                    "direct_file_write_matcher_coverage": true,
                    "host_capabilities": {
                        "stdio_mcp": true,
                        "http_mcp": false,
                        "session_start_hook": true,
                        "pre_tool_hook": true,
                        "post_tool_hook": true,
                        "user_prompt_submit_hook": host_supports_prompt_capture,
                        "stop_hook": true,
                        "rule_file_support": true,
                        "project_local_configuration": true,
                    },
                    "required_hook_phases": [
                        "session_start_hook",
                        "pre_tool_hook",
                        "post_tool_hook",
                        "user_prompt_submit_hook",
                        "stop_hook"
                    ],
                    "missing_required_hooks": if host_supports_prompt_capture { json!([]) } else { json!(["user_prompt_submit_hook"]) },
                    "prompt_capture": prompt_capture_configured,
                    "files": files,
                    "host_hook_commands": host_hook_commands,
                    "hook_root_resolution": {
                        "basis": root_resolution_basis,
                        "all_cwd_independent": true,
                        "all_subdirectory_safe": true,
                        "overall_status": "ok",
                        "phases": root_phases,
                    },
                    "hook_path_safety": {
                        "overall_status": "ok",
                        "all_cwd_independent": true,
                        "all_subdirectory_safe": true,
                        "commands": safety_commands,
                    },
                    "commands": commands,
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
            "--shared",
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
        let managed_session_id =
            managed_host_session_id("codex", self.connection_id(), self.session_id())
                .expect("guarded lifecycle fixture session coordinates should be valid");
        InvocationContext::new(
            ProjectId::new(&self.project_id),
            ActorSource::agent_connection(self.connection_id.clone()),
            operation_category,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
        .with_session_id(managed_session_id)
    }

    fn user_invocation(&self) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(&self.project_id),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
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
            "host_kind": "codex"
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
        capability["host_hook_commands"]
            .as_array_mut()
            .ok_or("fixture capability should contain host_hook_commands")?
            .retain(|command| command["phase"] != "pre_tool_hook");
        capability["hook_root_resolution"]["phases"]
            .as_array_mut()
            .ok_or("fixture capability should contain root-resolution phases")?
            .retain(|phase| phase["phase"] != "pre_tool_hook");
        capability["hook_path_safety"]["commands"]
            .as_array_mut()
            .ok_or("fixture capability should contain path-safety commands")?
            .retain(|command| command["phase"] != "pre_tool_hook");
        capability["files"]
            .as_array_mut()
            .ok_or("fixture capability should contain managed files")?
            .retain(|file| file["kind"] != "host_hook_wrapper" || file["phase"] != "pre_tool");
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
                acceptance_policy: volicord_types::RequiredNullable::null(),
                lineage: volicord_types::RequiredNullable::null(),
                initial_scope: InitialScope {
                    boundary: "Exercise guarded lifecycle behavior in a temp repository."
                        .to_owned(),
                    non_goals: vec!["Changing unrelated files.".to_owned()],
                    acceptance_criteria: vec![AcceptanceCriterionInput {
                        statement: "The guarded lifecycle reaches the expected close state."
                            .to_owned(),
                        evidence_requirement: EvidenceRequirement::Required,
                    }],
                },
                initial_context_refs: Vec::new(),
                initial_source_refs: Vec::new(),
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
                acceptance_criteria: Some(vec![AcceptanceCriterionReplacement {
                    acceptance_criterion_id: None.into(),
                    statement: "The fixture close check reports the expected state.".to_owned(),
                    evidence_requirement: EvidenceRequirement::Required,
                }])
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
        let store = CoreProjectStore::open(
            self.runtime_home(),
            &ProjectId::new(self.project_id.clone()),
        )?;
        let criteria = store.active_acceptance_criteria(&TaskId::new(task_id))?;
        let [criterion] = criteria.as_slice() else {
            return Err(format!(
                "guarded lifecycle fixture expected exactly one active acceptance criterion, found {}",
                criteria.len()
            )
            .into());
        };
        let target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(&criterion.acceptance_criterion_id),
        };
        let mut evidence_update = supported_evidence_update("Lifecycle close claim supported.");
        evidence_update.target = target.clone();
        let current_state_version = self.state_version()?;
        let staged = self.service().stage_artifact(
            StageArtifactRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_stage_evidence"),
                    Some(&format!("idem_{suffix}_stage_evidence")),
                    Some(current_state_version),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                display_name: format!("{suffix}-evidence.json"),
                content_type: "application/json".to_owned(),
                redaction_state: RedactionState::None,
                safe_bytes_or_notice: "{\"fixture\":\"lifecycle-close-evidence\"}".to_owned(),
                expected_sha256: None.into(),
                expected_size_bytes: None.into(),
                relation_hint: Some("evidence observation output".to_owned()).into(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let handle: StagedArtifactHandle =
            serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;
        let artifact_input = ArtifactInput {
            artifact_input_id: ArtifactInputId::new(format!("artifact_input_{suffix}_evidence")),
            source_kind: ArtifactInputSourceKind::StagedArtifact,
            staged_artifact_handle: Some(handle.clone()).into(),
            existing_artifact_ref: None.into(),
            relation_hint: Some("evidence observation output".to_owned()).into(),
            evidence_target: Some(target).into(),
            expected_sha256: Some(handle.sha256).into(),
            expected_size_bytes: Some(handle.size_bytes).into(),
            redaction_state: Some(handle.redaction_state).into(),
        };
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
            artifact_inputs: vec![artifact_input],
            evidence_updates: vec![evidence_update],
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

    pub(crate) fn request_final_acceptance_action(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let state_version = self.state_version()?;
        let response = self.service().request_user_action(
            volicord_types::RequestUserActionRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_final"),
                    Some(&format!("idem_{suffix}_final")),
                    Some(state_version),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
                action: UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
                    judgment_kind: JudgmentKind::FinalAcceptance,
                    presentation: volicord_types::JudgmentPresentation::Short,
                    question: "Does the user accept the current close basis?".to_owned(),
                    options: None.into(),
                    context: UserActionContext {
                        summary: "The guarded lifecycle fixture is ready for final acceptance."
                            .to_owned(),
                        related_refs: Vec::new(),
                        artifact_refs: Vec::new(),
                        visible_risks: Vec::new(),
                        constraints: vec![
                            "This answer applies only to the current fixture close basis."
                                .to_owned(),
                        ],
                    },
                    affected_refs: vec![self.state_ref(
                        StateRecordKind::Task,
                        task_id,
                        Some(task_id),
                        Some(state_version),
                    )],
                    sensitive_action_scope: None.into(),
                })),
                required_for: vec![UserActionRequiredFor::CloseComplete],
                expires_at: None.into(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        response.response_value["user_action_request_summary"]["user_action_request_id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "safe user-action request summary should identify the request".into())
    }

    pub(crate) fn resolve_pending_user_action_through_prompt(
        &self,
        task_id: &str,
        user_action_request_id: &str,
        event_id: &str,
        capture_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let store = self.store()?;
        let now = SystemClock.project_now(&store)?;
        let records = store.user_action_records_for_task(&TaskId::new(task_id), &now)?;
        let (index, record) = records
            .iter()
            .enumerate()
            .find(|(_, record)| record.request.user_action_request_id == user_action_request_id)
            .ok_or("pending user action should be stored for task")?;
        let verification_code = chat_user_action_verification_code(
            &record.request.project_id,
            &record.request.task_id,
            &record.request.user_action_request_id,
            &record.request.requested_at,
            self.connection_id(),
        );
        let message = format!(
            "Volicord: resolve A-{} --request {} --choice 1 {verification_code}",
            index + 1,
            record.request.user_action_request_id
        );
        let event = json!({
            "event_id": event_id,
            "prompt_capture_id": capture_id,
            "session_id": self.session_id(),
            "connection_id": self.connection_id(),
            "guard_installation_id": self.guard_installation_id(),
            "host_kind": "codex",
            "message": message,
            "timestamp": now.to_string()
        });
        let output = self.run_guard_event("prompt-capture", &event)?;
        assert_success(&output);
        let value = json_stdout(&output)?;
        assert_eq!(value["decision"], "inject_context");
        assert_eq!(
            value["result"]["recognized_user_action_command"]["action_type"],
            "choice"
        );
        Ok(())
    }

    pub(crate) fn resolve_user_action_direct(
        &self,
        task_id: &str,
        user_action_request_id: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let response = self.service().resolve_user_action(
            volicord_types::ResolveUserActionRequest {
                envelope: self.envelope(
                    &format!("req_direct_resolve_{user_action_request_id}"),
                    Some(&format!("submission_direct_{user_action_request_id}")),
                    None,
                    Some(task_id),
                ),
                user_action_request_id: UserActionRequestId::new(user_action_request_id),
                channel_submission_id: format!("submission_direct_{user_action_request_id}"),
                resolution: choice_user_action_resolution("accept"),
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
            produced_at_state_version: state_version.into(),
        }
    }
}

pub(crate) fn prompt_event(
    fixture: &GuardCliFixture,
    event_id: &str,
    capture_id: &str,
    message: &str,
) -> Value {
    let timestamp = UtcTimestamp::from_datetime(SystemClock.now()).to_string();
    json!({
        "event_id": event_id,
        "prompt_capture_id": capture_id,
        "session_id": "guard_session_chat",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
        "message": message,
        "timestamp": timestamp
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

pub(crate) fn expected_managed_session_id(
    fixture: &GuardCliFixture,
    event: &Value,
) -> Result<String, Box<dyn Error>> {
    let host_kind = event["host_kind"]
        .as_str()
        .ok_or("managed fixture event should contain host_kind")?;
    let native_session_id = event
        .get("session_id")
        .or_else(|| event.get("thread_id"))
        .and_then(Value::as_str)
        .ok_or("managed fixture event should contain a native session id")?;
    Ok(managed_host_session_id(
        host_kind,
        fixture.connection_id(),
        native_session_id,
    )?)
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

pub(crate) fn replace_prompt_user_action_binding(
    event: &mut Value,
    user_action_request_id: &str,
    verification_code: &str,
) {
    if let Some(Value::String(prompt)) = event.get_mut("prompt") {
        *prompt = prompt
            .replace("VOLICORD_REQUEST_ID", user_action_request_id)
            .replace("#VOLICORD_VERIFICATION_CODE", verification_code);
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
    let event = managed_test_event(event, Some(host_output));
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
    command.env(MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1);
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
    let configured_host = args
        .windows(2)
        .find_map(|pair| matches!(pair[0], "--host" | "--host-output").then_some(pair[1]));
    let event = managed_test_event(event, configured_host);
    let mut child = Command::new(volicord_bin())
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .env(MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1)
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

fn managed_test_event(event: &Value, configured_host: Option<&str>) -> Value {
    let mut event = event.clone();
    let event_host = event
        .get("host_kind")
        .and_then(Value::as_str)
        .or_else(|| event.pointer("/host/kind").and_then(Value::as_str))
        .or(configured_host)
        .map(|host| match host {
            "claude-code" => "claude_code",
            other => other,
        });
    if !matches!(event_host, Some("codex" | "claude_code")) {
        return event;
    }
    let Some(object) = event.as_object_mut() else {
        return event;
    };
    if !object.contains_key("session_id") && !object.contains_key("thread_id") {
        let native_session_id = object
            .get("event_id")
            .and_then(Value::as_str)
            .filter(|value| {
                !value.is_empty()
                    && value.as_bytes().iter().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b':' | b'-')
                    })
            })
            .unwrap_or("managed-test-session")
            .to_owned();
        object.insert("session_id".to_owned(), Value::String(native_session_id));
    }
    event
}

pub(crate) fn run_guard_file<const N: usize>(
    runtime_home: &Path,
    current_dir: &Path,
    args: [&str; N],
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin())
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .env(MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1)
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
