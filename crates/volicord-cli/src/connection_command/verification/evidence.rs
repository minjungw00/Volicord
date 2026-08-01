//! Typed evidence projected into MCP-related connection-check details.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use volicord_store::{
    integration_verification::{
        GuardIntegrationVerificationRunRecord, GuardProbeObservationRecord,
    },
    operational_sessions::{ManagedCapabilityProof, ManagedPeerObservation, McpSessionMilestones},
};
use volicord_types::connection_verification::{ActivationStepId, ConnectionCheckStatus};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::integration_verification::{
    GuardIntegrationVerificationStatus, GuardProbeObservationStage,
    GuardVerificationRecoverability, GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    IntegrationVerificationWorkflowState,
};
use volicord_types::tool_names::{AgentToolId, ToolVerificationRole};
use volicord_types::values::UtcTimestamp;

use super::{HostExecutableStatus, Verification};
use crate::guard_integration::audit::HookPathSafetyAssessment;
use crate::host_integration::verification::HostExecutableProbe;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostExecutableProbeDetails {
    status: HostExecutableStatus,
    probe: HostExecutableProbe,
    diagnostic: String,
}

impl HostExecutableProbeDetails {
    pub(super) fn from_verification(host: &Verification) -> Self {
        Self {
            status: host.host_executable,
            probe: host.host_executable_probe(),
            diagnostic: host.host_executable_details.clone(),
        }
    }

    pub(super) const fn status(&self) -> HostExecutableStatus {
        self.status
    }

    pub(super) fn probe(&self) -> &HostExecutableProbe {
        &self.probe
    }

    pub(super) fn diagnostic(&self) -> &str {
        &self.diagnostic
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct AmbientGuardCoverageEvidence {
    current_hook_definition_executed: bool,
    configured_phases_observed: bool,
    installation_ids: Vec<String>,
    affected_paths: Vec<String>,
    artifact_issues: Vec<Value>,
    manifest_issues: Vec<&'static str>,
    hook_path_safety: HookPathSafetyAssessment,
    configured_missing_phases: Vec<String>,
    required_phases: Vec<String>,
    observed_phases: Vec<String>,
    missing_required_phases: Vec<String>,
    incompatible_event_ids: Vec<String>,
    prompt_capture: AmbientPromptCaptureEvidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_current_observation_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct AmbientPromptCaptureEvidence {
    host_supported: bool,
    configured: bool,
    observed: bool,
}

impl AmbientGuardCoverageEvidence {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        current_hook_definition_executed: bool,
        configured_phases_observed: bool,
        installation_ids: Vec<String>,
        affected_paths: Vec<String>,
        artifact_issues: Vec<Value>,
        manifest_issues: Vec<&'static str>,
        hook_path_safety: HookPathSafetyAssessment,
        configured_missing_phases: Vec<String>,
        required_phases: Vec<String>,
        observed_phases: Vec<String>,
        missing_required_phases: Vec<String>,
        incompatible_event_ids: Vec<String>,
        prompt_capture: AmbientPromptCaptureEvidence,
        last_current_observation_at: Option<String>,
    ) -> Self {
        Self {
            current_hook_definition_executed,
            configured_phases_observed,
            installation_ids,
            affected_paths,
            artifact_issues,
            manifest_issues,
            hook_path_safety,
            configured_missing_phases,
            required_phases,
            observed_phases,
            missing_required_phases,
            incompatible_event_ids,
            prompt_capture,
            last_current_observation_at,
        }
    }
}

impl AmbientPromptCaptureEvidence {
    pub(super) const fn new(host_supported: bool, configured: bool, observed: bool) -> Self {
        Self {
            host_supported,
            configured,
            observed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CorrelatedGuardVerificationEvidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    recoverability: Option<GuardVerificationRecoverability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_attempt: Option<CorrelatedGuardAttemptEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_completed_proof: Option<CorrelatedGuardProof>,
}

impl CorrelatedGuardVerificationEvidence {
    pub(super) fn new(
        latest_attempt: Option<CorrelatedGuardAttemptEvidence>,
        latest_completed_proof: Option<CorrelatedGuardProof>,
    ) -> Self {
        let recoverability = latest_attempt
            .as_ref()
            .and_then(|attempt| attempt.recoverability);
        Self {
            recoverability,
            latest_attempt,
            latest_completed_proof,
        }
    }

    pub(super) fn decisive_evidence_timestamp(
        &self,
        status: ConnectionCheckStatus,
    ) -> Result<Option<UtcTimestamp>, String> {
        match status {
            ConnectionCheckStatus::Passed => {
                let attempt = self.latest_attempt.as_ref().ok_or_else(|| {
                    "passed Guard verification check is missing its current attempt".to_owned()
                })?;
                if attempt.attempt_state != GuardIntegrationVerificationStatus::Complete {
                    return Err(
                        "passed Guard verification check is not backed by a completed current attempt"
                            .to_owned(),
                    );
                }
                let proof = self.latest_completed_proof.as_ref().ok_or_else(|| {
                    "passed Guard verification check is missing its completed proof".to_owned()
                })?;
                attempt.require_same_completed_verification(proof)?;
                Ok(Some(proof.evidence_timestamp().clone()))
            }
            ConnectionCheckStatus::Pending => match self.latest_attempt.as_ref() {
                None => Ok(None),
                Some(attempt)
                    if matches!(
                        attempt.attempt_state,
                        GuardIntegrationVerificationStatus::AwaitingProbe
                            | GuardIntegrationVerificationStatus::AwaitingObservation
                    ) =>
                {
                    Ok(Some(attempt.evidence_timestamp()?.clone()))
                }
                Some(_) => Err(
                    "pending Guard verification check is backed by a terminal current attempt"
                        .to_owned(),
                ),
            },
            ConnectionCheckStatus::Failed => {
                let attempt = self.latest_attempt.as_ref().ok_or_else(|| {
                    "failed Guard verification check is missing its current attempt".to_owned()
                })?;
                if attempt.attempt_state != GuardIntegrationVerificationStatus::RepairRequired {
                    return Err(
                        "failed Guard verification check is not backed by a typed repair state"
                            .to_owned(),
                    );
                }
                Ok(Some(attempt.evidence_timestamp()?.clone()))
            }
            ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => Err(
                "correlated Guard verification cannot select evidence time for this check status"
                    .to_owned(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CorrelatedGuardEvidenceIdentity {
    verification_id: String,
    connection_internal_id: String,
    project_internal_id: String,
    project_id: String,
    runtime_session_id: String,
    host_session_id: String,
    host_turn_id: String,
    integration_revision: String,
    guard_installation_id: String,
    host_contract_profile: String,
    expected_probe_tool: String,
    expected_host_callable_name: String,
    policy_digest: String,
    hook_definition_digest: String,
}

impl CorrelatedGuardEvidenceIdentity {
    fn from_run(run: &GuardIntegrationVerificationRunRecord) -> Self {
        Self {
            verification_id: run.verification_id.clone(),
            connection_internal_id: run.connection_internal_id.clone(),
            project_internal_id: run.project_internal_id.clone(),
            project_id: run.project_id.clone(),
            runtime_session_id: run.runtime_session_id.clone(),
            host_session_id: run.host_session_id.clone(),
            host_turn_id: run.host_turn_id.clone(),
            integration_revision: run.integration_revision.clone(),
            guard_installation_id: run.guard_installation_id.clone(),
            host_contract_profile: run.host_contract_profile.clone(),
            expected_probe_tool: run.expected_probe_tool.clone(),
            expected_host_callable_name: run.expected_host_callable_name.clone(),
            policy_digest: run.policy_digest.clone(),
            hook_definition_digest: run.hook_definition_digest.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CorrelatedGuardAttemptEvidence {
    #[serde(skip)]
    identity: CorrelatedGuardEvidenceIdentity,
    #[serde(skip)]
    applicable_observation_at: Option<UtcTimestamp>,
    evidence_role: &'static str,
    verification_id: String,
    runtime_session_id: String,
    host_session_id: String,
    host_turn_id: String,
    attempt_state: GuardIntegrationVerificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pre_tool_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    post_tool_event_id: Option<String>,
    expected_agent_tool_id: AgentToolId,
    expected_host_callable_identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_host_callable_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acquisition_stage: Option<GuardProbeObservationStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repair_reason: Option<GuardVerificationRepairReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_policy: Option<GuardVerificationRetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recoverability: Option<GuardVerificationRecoverability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_action: Option<ActivationStepId>,
    integration_revision: String,
    guard_installation_id: String,
    policy_digest: String,
    hook_definition_digest: String,
    created_at: UtcTimestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    acknowledged_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_at: Option<UtcTimestamp>,
}

impl CorrelatedGuardAttemptEvidence {
    pub(super) fn try_new(
        run: &GuardIntegrationVerificationRunRecord,
        workflow: &IntegrationVerificationWorkflowState,
        observations: &[GuardProbeObservationRecord],
    ) -> Result<Self, String> {
        let (attempt_state, repair_reason, retry_policy) = match workflow {
            IntegrationVerificationWorkflowState::AwaitingProbe { .. } => (
                GuardIntegrationVerificationStatus::AwaitingProbe,
                None,
                None,
            ),
            IntegrationVerificationWorkflowState::AwaitingObservation { .. } => (
                GuardIntegrationVerificationStatus::AwaitingObservation,
                None,
                None,
            ),
            IntegrationVerificationWorkflowState::Complete { .. } => {
                (GuardIntegrationVerificationStatus::Complete, None, None)
            }
            IntegrationVerificationWorkflowState::RepairRequired {
                reason,
                retry_policy,
                ..
            } => (
                GuardIntegrationVerificationStatus::RepairRequired,
                Some(*reason),
                Some(*retry_policy),
            ),
        };
        let selected_observation =
            selected_guard_observation(attempt_state, repair_reason, observations);
        let recoverability = retry_policy.map(GuardVerificationRecoverability::from_retry_policy);
        let recovery_action = retry_policy.map(recovery_action_for_retry_policy);
        if run.status != attempt_state.as_str() {
            return Err(
                "Guard verification workflow does not match its persisted status".to_owned(),
            );
        }
        let created_at = parse_guard_timestamp(&run.created_at)?;
        let acknowledged_at = run
            .probe_acknowledged_at
            .as_deref()
            .map(parse_guard_timestamp)
            .transpose()?;
        let terminal_timestamp = run
            .completed_at
            .as_deref()
            .map(parse_guard_timestamp)
            .transpose()?;
        validate_guard_lifecycle_timestamps(
            &created_at,
            acknowledged_at.as_ref(),
            terminal_timestamp.as_ref(),
        )?;
        let observation_timestamps = validate_guard_observations(
            run,
            observations,
            &created_at,
            terminal_timestamp.as_ref(),
        )?;
        let applicable_observation_at = observations
            .iter()
            .zip(observation_timestamps)
            .filter(|(observation, _)| {
                observation.stage != GuardProbeObservationStage::UnrelatedRoutedTool
            })
            .map(|(_, timestamp)| timestamp)
            .max();
        let workflow_completed_at = match workflow {
            IntegrationVerificationWorkflowState::Complete { completed_at } => Some(completed_at),
            _ => None,
        };
        if workflow_completed_at != terminal_timestamp.as_ref()
            && attempt_state == GuardIntegrationVerificationStatus::Complete
        {
            return Err(
                "completed Guard verification workflow does not match its persisted completion time"
                    .to_owned(),
            );
        }
        Ok(Self {
            identity: CorrelatedGuardEvidenceIdentity::from_run(run),
            applicable_observation_at,
            evidence_role: "guard_verification_attempt",
            verification_id: run.verification_id.clone(),
            runtime_session_id: run.runtime_session_id.clone(),
            host_session_id: run.host_session_id.clone(),
            host_turn_id: run.host_turn_id.clone(),
            attempt_state,
            prompt_event_id: run.matched_prompt_event_id.clone(),
            pre_tool_event_id: run.matched_pre_tool_event_id.clone(),
            post_tool_event_id: run.matched_post_tool_event_id.clone(),
            expected_agent_tool_id: AgentToolId::GUARD_PROBE,
            expected_host_callable_identity: run.expected_host_callable_name.clone(),
            observed_host_callable_identity: selected_observation
                .and_then(|observation| observation.observed_callable_name.clone()),
            acquisition_stage: selected_observation.map(|observation| observation.stage),
            repair_reason,
            retry_policy,
            recoverability,
            recovery_action,
            integration_revision: run.integration_revision.clone(),
            guard_installation_id: run.guard_installation_id.clone(),
            policy_digest: run.policy_digest.clone(),
            hook_definition_digest: run.hook_definition_digest.clone(),
            created_at,
            acknowledged_at,
            completed_at: (attempt_state == GuardIntegrationVerificationStatus::Complete)
                .then(|| terminal_timestamp.clone())
                .flatten(),
            terminal_at: terminal_timestamp,
        })
    }

    pub(super) fn evidence_timestamp(&self) -> Result<&UtcTimestamp, String> {
        match self.attempt_state {
            GuardIntegrationVerificationStatus::AwaitingProbe => Ok(&self.created_at),
            GuardIntegrationVerificationStatus::AwaitingObservation => self
                .acknowledged_at
                .iter()
                .chain(self.applicable_observation_at.iter())
                .chain(std::iter::once(&self.created_at))
                .max()
                .ok_or_else(|| {
                    "awaiting-observation Guard attempt has no evidence timestamp".to_owned()
                }),
            GuardIntegrationVerificationStatus::Complete => self
                .completed_at
                .as_ref()
                .ok_or_else(|| "completed Guard attempt is missing completed_at".to_owned()),
            GuardIntegrationVerificationStatus::RepairRequired => self
                .terminal_at
                .as_ref()
                .ok_or_else(|| "repair-required Guard attempt is missing terminal_at".to_owned()),
        }
    }

    fn require_same_completed_verification(
        &self,
        proof: &CorrelatedGuardProof,
    ) -> Result<(), String> {
        if self.identity != proof.identity {
            return Err(
                "completed Guard attempt and proof have different verification identities"
                    .to_owned(),
            );
        }
        if self.completed_at.as_ref() != Some(proof.evidence_timestamp())
            || self.terminal_at.as_ref() != Some(proof.evidence_timestamp())
        {
            return Err(
                "completed Guard attempt and proof have different completion times".to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CorrelatedGuardProof {
    #[serde(skip)]
    identity: CorrelatedGuardEvidenceIdentity,
    evidence_role: &'static str,
    verification_id: String,
    runtime_session_id: String,
    host_session_id: String,
    host_turn_id: String,
    prompt_event_id: String,
    pre_tool_event_id: String,
    post_tool_event_id: String,
    expected_agent_tool_id: AgentToolId,
    expected_host_callable_identity: String,
    observed_host_callable_identity: String,
    acquisition_stage: GuardProbeObservationStage,
    integration_revision: String,
    guard_installation_id: String,
    policy_digest: String,
    hook_definition_digest: String,
    completed_at: UtcTimestamp,
}

impl CorrelatedGuardProof {
    pub(super) fn try_new(
        run: &GuardIntegrationVerificationRunRecord,
        observations: &[GuardProbeObservationRecord],
    ) -> Result<Self, String> {
        if run.status != GuardIntegrationVerificationStatus::Complete.as_str() {
            return Err("Guard proof does not belong to a completed verification".to_owned());
        }
        let required = |value: &Option<String>, field: &str| {
            value
                .clone()
                .ok_or_else(|| format!("completed Guard proof is missing {field}"))
        };
        let matched_observation = observations
            .iter()
            .find(|observation| {
                observation.stage == GuardProbeObservationStage::PostToolMatched
                    && observation.guard_event_id.as_deref()
                        == run.matched_post_tool_event_id.as_deref()
            })
            .ok_or_else(|| {
                "completed Guard proof is missing its post-tool matched observation".to_owned()
            })?;
        let created_at = parse_guard_timestamp(&run.created_at)?;
        let acknowledged_at = run
            .probe_acknowledged_at
            .as_deref()
            .map(parse_guard_timestamp)
            .transpose()?
            .ok_or_else(|| "completed Guard proof is missing acknowledged_at".to_owned())?;
        let completed_at = parse_guard_timestamp(
            run.completed_at
                .as_deref()
                .ok_or_else(|| "completed Guard proof is missing completed_at".to_owned())?,
        )?;
        validate_guard_lifecycle_timestamps(
            &created_at,
            Some(&acknowledged_at),
            Some(&completed_at),
        )?;
        validate_guard_observations(run, observations, &created_at, Some(&completed_at))?;
        if matched_observation.guard_event_id.as_deref()
            != run.matched_post_tool_event_id.as_deref()
            || parse_guard_timestamp(&matched_observation.observed_at)? != completed_at
        {
            return Err(
                "completed Guard proof does not correspond to its persisted completion".to_owned(),
            );
        }
        Ok(Self {
            identity: CorrelatedGuardEvidenceIdentity::from_run(run),
            evidence_role: "guard_verification_proof",
            verification_id: run.verification_id.clone(),
            runtime_session_id: run.runtime_session_id.clone(),
            host_session_id: run.host_session_id.clone(),
            host_turn_id: run.host_turn_id.clone(),
            prompt_event_id: required(&run.matched_prompt_event_id, "prompt_event_id")?,
            pre_tool_event_id: required(&run.matched_pre_tool_event_id, "pre_tool_event_id")?,
            post_tool_event_id: required(&run.matched_post_tool_event_id, "post_tool_event_id")?,
            expected_agent_tool_id: AgentToolId::GUARD_PROBE,
            expected_host_callable_identity: run.expected_host_callable_name.clone(),
            observed_host_callable_identity: matched_observation
                .observed_callable_name
                .clone()
                .ok_or_else(|| {
                    "completed Guard proof is missing its observed host callable identity"
                        .to_owned()
                })?,
            acquisition_stage: GuardProbeObservationStage::PostToolMatched,
            integration_revision: run.integration_revision.clone(),
            guard_installation_id: run.guard_installation_id.clone(),
            policy_digest: run.policy_digest.clone(),
            hook_definition_digest: run.hook_definition_digest.clone(),
            completed_at,
        })
    }

    pub(super) fn evidence_timestamp(&self) -> &UtcTimestamp {
        &self.completed_at
    }
}

fn validate_guard_lifecycle_timestamps(
    created_at: &UtcTimestamp,
    acknowledged_at: Option<&UtcTimestamp>,
    terminal_at: Option<&UtcTimestamp>,
) -> Result<(), String> {
    if acknowledged_at.is_some_and(|value| value < created_at)
        || terminal_at.is_some_and(|value| value < created_at)
        || acknowledged_at
            .zip(terminal_at)
            .is_some_and(|(acknowledged, terminal)| acknowledged > terminal)
    {
        return Err("Guard verification lifecycle timestamp order is corrupt".to_owned());
    }
    Ok(())
}

fn validate_guard_observations(
    run: &GuardIntegrationVerificationRunRecord,
    observations: &[GuardProbeObservationRecord],
    created_at: &UtcTimestamp,
    terminal_at: Option<&UtcTimestamp>,
) -> Result<Vec<UtcTimestamp>, String> {
    observations
        .iter()
        .map(|observation| {
            if observation.verification_id != run.verification_id
                || observation.guard_installation_id != run.guard_installation_id
                || observation.integration_revision != run.integration_revision
                || observation.expected_agent_tool_id != run.expected_probe_tool
                || observation.expected_host_callable_name != run.expected_host_callable_name
            {
                return Err(
                    "Guard probe observation does not match its verification identity".to_owned(),
                );
            }
            let observed_at = parse_guard_timestamp(&observation.observed_at)?;
            if &observed_at < created_at
                || terminal_at.is_some_and(|terminal| &observed_at > terminal)
            {
                return Err(
                    "Guard probe observation timestamp is outside its verification lifecycle"
                        .to_owned(),
                );
            }
            Ok(observed_at)
        })
        .collect()
}

fn parse_guard_timestamp(value: &str) -> Result<UtcTimestamp, String> {
    UtcTimestamp::parse(value)
        .map_err(|_| format!("Guard verification timestamp is invalid: {value}"))
}

pub(in crate::connection_command) fn recovery_action_for_retry_policy(
    policy: GuardVerificationRetryPolicy,
) -> ActivationStepId {
    match policy {
        GuardVerificationRetryPolicy::NoAutomaticRetry => ActivationStepId::ReadConnectionStatus,
        GuardVerificationRetryPolicy::NewTurnRequired => {
            ActivationStepId::RequestIntegrationVerification
        }
        GuardVerificationRetryPolicy::HostReloadRequired => ActivationStepId::ReloadCodex,
        GuardVerificationRetryPolicy::HookReviewRequired => ActivationStepId::RepairHookContract,
        GuardVerificationRetryPolicy::RepairRequired => {
            ActivationStepId::RepairManagedConfiguration
        }
    }
}

pub(super) fn selected_guard_observation(
    status: GuardIntegrationVerificationStatus,
    reason: Option<GuardVerificationRepairReason>,
    observations: &[GuardProbeObservationRecord],
) -> Option<&GuardProbeObservationRecord> {
    let matches_reason = |stage: GuardProbeObservationStage| match reason {
        Some(GuardVerificationRepairReason::HookEventNotObserved)
        | Some(GuardVerificationRepairReason::ObservationDeadlineExceeded) => {
            stage == GuardProbeObservationStage::HookEventNotObserved
        }
        Some(GuardVerificationRepairReason::HookPayloadIncompatible) => {
            stage == GuardProbeObservationStage::HookPayloadIncompatible
        }
        Some(GuardVerificationRepairReason::CallableIdentityMismatch) => matches!(
            stage,
            GuardProbeObservationStage::CallableIdentityUnknown
                | GuardProbeObservationStage::CallableIdentityMismatch
        ),
        Some(GuardVerificationRepairReason::VerificationIdMismatch) => {
            stage == GuardProbeObservationStage::VerificationIdMismatch
        }
        Some(GuardVerificationRepairReason::SessionMismatch) => {
            stage == GuardProbeObservationStage::SessionMismatch
        }
        Some(GuardVerificationRepairReason::TurnMismatch) => {
            stage == GuardProbeObservationStage::TurnMismatch
        }
        Some(GuardVerificationRepairReason::ToolUseMismatch) => {
            stage == GuardProbeObservationStage::ToolUseMismatch
        }
        Some(
            GuardVerificationRepairReason::IntegrationRevisionChanged
            | GuardVerificationRepairReason::HookDefinitionChanged
            | GuardVerificationRepairReason::PolicyChanged,
        )
        | None => false,
    };
    if status == GuardIntegrationVerificationStatus::Complete {
        return observations
            .iter()
            .rev()
            .find(|observation| observation.stage == GuardProbeObservationStage::PostToolMatched);
    }
    match reason {
        Some(_) => observations
            .iter()
            .rev()
            .find(|observation| matches_reason(observation.stage)),
        None => observations.last(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ManagedPeerObservationDetails {
    client_info: ManagedPeerClientInfoDetails,
    requested_protocol_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_protocol_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negotiated_protocol_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ManagedPeerClientInfoDetails {
    name: String,
    version: String,
}

impl From<&ManagedPeerObservation> for ManagedPeerObservationDetails {
    fn from(peer: &ManagedPeerObservation) -> Self {
        Self {
            client_info: ManagedPeerClientInfoDetails {
                name: peer.client_info.name().to_owned(),
                version: peer.client_info.version().to_owned(),
            },
            requested_protocol_revision: peer.requested_protocol_revision.clone(),
            selected_protocol_revision: peer.selected_protocol_revision.clone(),
            negotiated_protocol_revision: peer.negotiated_protocol_revision.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ManagedSessionAttemptDetails {
    evidence_role: &'static str,
    current_integration_revision: IntegrationRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<volicord_types::integration_revision::McpRuntimeSessionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_integration_revision: Option<IntegrationRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_started_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initialize_completed_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initialized_notification_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    managed_peer: Option<ManagedPeerObservationDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_tools: Option<RequiredToolsAttemptEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_tool: Option<VerificationToolEvidence>,
    host_executable_probe: HostExecutableProbe,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_finding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_observed_at: Option<UtcTimestamp>,
}

impl ManagedSessionAttemptDetails {
    pub(super) fn new(
        current_revision: &IntegrationRevision,
        attempt: Option<&McpSessionMilestones>,
        host: &Verification,
    ) -> Self {
        Self {
            evidence_role: "latest_managed_attempt",
            current_integration_revision: current_revision.clone(),
            runtime_session_id: attempt.map(|value| value.runtime_session_id.as_str().to_owned()),
            source: attempt.map(|value| value.source),
            observed_integration_revision: attempt.map(|value| value.integration_revision.clone()),
            process_started_at: attempt.map(|value| value.process_started_at.clone()),
            initialize_completed_at: attempt
                .and_then(|value| value.initialize_completed_at.clone()),
            initialized_notification_at: attempt
                .and_then(|value| value.initialized_notification_at.clone()),
            managed_peer: attempt
                .and_then(|value| value.managed_peer.as_ref())
                .map(ManagedPeerObservationDetails::from),
            required_tools: attempt.and_then(|value| {
                Some(RequiredToolsAttemptEvidence {
                    tools_list_observed_at: value.tools_list_observed_at.clone()?,
                    returned_tool_identities: value.returned_tool_identities.clone()?,
                    required_tools_present: value.required_tools_present?,
                    required_tools_validated_at: value.required_tools_validated_at.clone(),
                })
            }),
            verification_tool: attempt.and_then(|value| {
                Some(VerificationToolEvidence {
                    expected_tool_identity: expected_verification_tool_name().to_owned(),
                    observed_tool_identity: value.verification_tool_name.clone()?,
                    observed_at: value.verification_tool_observed_at.clone()?,
                })
            }),
            host_executable_probe: host.host_executable_probe(),
            terminal_finding_id: attempt
                .and_then(|value| value.terminal_finding.as_ref())
                .map(|value| value.as_str().to_owned()),
            last_observed_at: attempt.map(|value| value.last_observed_at.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RequiredToolsAttemptEvidence {
    tools_list_observed_at: UtcTimestamp,
    returned_tool_identities: Vec<String>,
    required_tools_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_tools_validated_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct RequiredToolsEvidence {
    tools_list_observed_at: UtcTimestamp,
    returned_tool_identities: Vec<String>,
    required_tools_validated_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct VerificationToolEvidence {
    expected_tool_identity: String,
    observed_tool_identity: String,
    observed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ManagedCapabilityProofDetails {
    evidence_role: &'static str,
    current_integration_revision: IntegrationRevision,
    runtime_session_id: String,
    source: volicord_types::integration_revision::McpRuntimeSessionSource,
    observed_integration_revision: IntegrationRevision,
    process_started_at: UtcTimestamp,
    initialize_completed_at: UtcTimestamp,
    initialized_notification_at: UtcTimestamp,
    managed_peer: ManagedPeerObservationDetails,
    required_tools: RequiredToolsEvidence,
    verification_tool: VerificationToolEvidence,
    host_executable_probe: HostExecutableProbe,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_finding_id: Option<String>,
    last_observed_at: UtcTimestamp,
}

impl ManagedCapabilityProofDetails {
    pub(super) fn new(
        current_revision: &IntegrationRevision,
        proof: &ManagedCapabilityProof,
        host: &Verification,
    ) -> Self {
        let milestones = proof.milestones();
        Self {
            evidence_role: "latest_managed_capability_proof",
            current_integration_revision: current_revision.clone(),
            runtime_session_id: milestones.runtime_session_id.as_str().to_owned(),
            source: milestones.source,
            observed_integration_revision: milestones.integration_revision.clone(),
            process_started_at: milestones.process_started_at.clone(),
            initialize_completed_at: milestones
                .initialize_completed_at
                .clone()
                .expect("complete proof has initialize completion"),
            initialized_notification_at: milestones
                .initialized_notification_at
                .clone()
                .expect("complete proof has initialized notification"),
            managed_peer: ManagedPeerObservationDetails::from(
                milestones
                    .managed_peer
                    .as_ref()
                    .expect("complete proof has managed peer observation"),
            ),
            required_tools: RequiredToolsEvidence {
                tools_list_observed_at: milestones
                    .tools_list_observed_at
                    .clone()
                    .expect("complete proof has tools/list observation"),
                returned_tool_identities: milestones
                    .returned_tool_identities
                    .clone()
                    .expect("complete proof has returned tool identities"),
                required_tools_validated_at: milestones
                    .required_tools_validated_at
                    .clone()
                    .expect("complete proof has required-tool validation"),
            },
            verification_tool: VerificationToolEvidence {
                expected_tool_identity: expected_verification_tool_name().to_owned(),
                observed_tool_identity: milestones
                    .verification_tool_name
                    .clone()
                    .expect("complete proof has verification-tool identity"),
                observed_at: milestones
                    .verification_tool_observed_at
                    .clone()
                    .expect("complete proof has verification-tool observation"),
            },
            host_executable_probe: host.host_executable_probe(),
            terminal_finding_id: milestones
                .terminal_finding
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            last_observed_at: milestones.last_observed_at.clone(),
        }
    }
}

pub(super) fn expected_verification_tool_name() -> &'static str {
    ToolVerificationRole::ManagedHostRoundTrip
        .tool()
        .wire_name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::integration_verification::GuardIntegrationVerificationFinding;

    const HASH: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    const CREATED_AT: &str = "2026-07-23T00:00:01Z";
    const ACKNOWLEDGED_AT: &str = "2026-07-23T00:00:02Z";
    const OBSERVED_AT: &str = "2026-07-23T00:00:03Z";
    const COMPLETED_AT: &str = "2026-07-23T00:00:04Z";

    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::parse(value).expect("test timestamp")
    }

    fn run(
        verification_id: &str,
        status: GuardIntegrationVerificationStatus,
    ) -> GuardIntegrationVerificationRunRecord {
        let terminal = matches!(
            status,
            GuardIntegrationVerificationStatus::Complete
                | GuardIntegrationVerificationStatus::RepairRequired
        );
        let acknowledged = status != GuardIntegrationVerificationStatus::AwaitingProbe;
        GuardIntegrationVerificationRunRecord {
            verification_id: verification_id.to_owned(),
            connection_internal_id: "connection_internal".to_owned(),
            project_internal_id: "project_internal".to_owned(),
            project_id: "project".to_owned(),
            runtime_session_id: "runtime_session".to_owned(),
            host_session_id: "host_session".to_owned(),
            host_turn_id: "host_turn".to_owned(),
            integration_revision: HASH.to_owned(),
            guard_installation_id: "guard_installation".to_owned(),
            host_contract_profile: "codex-command-hooks".to_owned(),
            hook_definition_digest: HASH.to_owned(),
            policy_digest: HASH.to_owned(),
            expected_probe_tool: AgentToolId::GUARD_PROBE.wire_name().to_owned(),
            expected_host_callable_name: "mcp__volicord__volicord_guard_probe".to_owned(),
            observation_policy_kind: "synchronous".to_owned(),
            observation_deadline_at: None,
            allowed_status_reads: 1,
            status_read_count: 0,
            created_at: CREATED_AT.to_owned(),
            cleanup_after: "2026-07-23T00:05:00Z".to_owned(),
            status: status.as_str().to_owned(),
            probe_acknowledged_at: acknowledged.then(|| ACKNOWLEDGED_AT.to_owned()),
            completed_at: terminal.then(|| COMPLETED_AT.to_owned()),
            matched_prompt_event_id: Some("guard_event_prompt".to_owned()),
            matched_pre_tool_event_id: (status == GuardIntegrationVerificationStatus::Complete)
                .then(|| "guard_event_pre".to_owned()),
            matched_post_tool_event_id: (status == GuardIntegrationVerificationStatus::Complete)
                .then(|| "guard_event_post".to_owned()),
            repair_reason: (status == GuardIntegrationVerificationStatus::RepairRequired).then(
                || {
                    GuardVerificationRepairReason::HookEventNotObserved
                        .as_str()
                        .to_owned()
                },
            ),
            retry_policy: (status == GuardIntegrationVerificationStatus::RepairRequired).then(
                || {
                    GuardVerificationRetryPolicy::HostReloadRequired
                        .as_str()
                        .to_owned()
                },
            ),
            terminal_finding_code: (status == GuardIntegrationVerificationStatus::RepairRequired)
                .then(|| "hook_event_not_observed".to_owned()),
            terminal_finding_summary: (status
                == GuardIntegrationVerificationStatus::RepairRequired)
                .then(|| "No matching Guard event was observed.".to_owned()),
        }
    }

    fn observation(
        run: &GuardIntegrationVerificationRunRecord,
        stage: GuardProbeObservationStage,
        observed_at: &str,
    ) -> GuardProbeObservationRecord {
        GuardProbeObservationRecord {
            observation_id: format!("observation_{}", stage.as_str()),
            verification_id: run.verification_id.clone(),
            guard_event_id: (stage == GuardProbeObservationStage::PostToolMatched)
                .then(|| "guard_event_post".to_owned()),
            stage,
            expected_agent_tool_id: run.expected_probe_tool.clone(),
            expected_host_callable_name: run.expected_host_callable_name.clone(),
            observed_callable_name: (stage == GuardProbeObservationStage::PostToolMatched)
                .then(|| run.expected_host_callable_name.clone()),
            hook_event_kind: (stage == GuardProbeObservationStage::PostToolMatched)
                .then(|| "post_tool".to_owned()),
            verification_id_present: true,
            verification_id_matches: true,
            guard_installation_id: run.guard_installation_id.clone(),
            integration_revision: run.integration_revision.clone(),
            observed_at: observed_at.to_owned(),
        }
    }

    fn workflow(
        status: GuardIntegrationVerificationStatus,
    ) -> IntegrationVerificationWorkflowState {
        match status {
            GuardIntegrationVerificationStatus::AwaitingProbe => {
                IntegrationVerificationWorkflowState::AwaitingProbe {
                    tool: volicord_types::integration_verification::GuardProbeToolReference::new(),
                }
            }
            GuardIntegrationVerificationStatus::AwaitingObservation => {
                IntegrationVerificationWorkflowState::AwaitingObservation {
                    tool: volicord_types::integration_verification::IntegrationVerificationStatusToolReference::new(),
                    acknowledged_at: timestamp(ACKNOWLEDGED_AT),
                    remaining_status_reads: 1,
                }
            }
            GuardIntegrationVerificationStatus::Complete => {
                IntegrationVerificationWorkflowState::Complete {
                    completed_at: timestamp(COMPLETED_AT),
                }
            }
            GuardIntegrationVerificationStatus::RepairRequired => {
                IntegrationVerificationWorkflowState::RepairRequired {
                    reason: GuardVerificationRepairReason::HookEventNotObserved,
                    retry_policy: GuardVerificationRetryPolicy::HostReloadRequired,
                    finding: GuardIntegrationVerificationFinding {
                        code: "hook_event_not_observed".to_owned(),
                        summary: "No matching Guard event was observed.".to_owned(),
                    },
                }
            }
        }
    }

    fn attempt(
        run: &GuardIntegrationVerificationRunRecord,
        observations: &[GuardProbeObservationRecord],
    ) -> CorrelatedGuardAttemptEvidence {
        CorrelatedGuardAttemptEvidence::try_new(run, &workflow_status(run), observations)
            .expect("attempt evidence")
    }

    fn workflow_status(
        run: &GuardIntegrationVerificationRunRecord,
    ) -> IntegrationVerificationWorkflowState {
        let status = [
            GuardIntegrationVerificationStatus::AwaitingProbe,
            GuardIntegrationVerificationStatus::AwaitingObservation,
            GuardIntegrationVerificationStatus::Complete,
            GuardIntegrationVerificationStatus::RepairRequired,
        ]
        .into_iter()
        .find(|status| status.as_str() == run.status)
        .expect("test status");
        workflow(status)
    }

    #[test]
    fn awaiting_probe_uses_attempt_creation_and_no_run_has_no_evidence_time() {
        let run = run(
            "guard_verification_awaiting_probe",
            GuardIntegrationVerificationStatus::AwaitingProbe,
        );
        let evidence = CorrelatedGuardVerificationEvidence::new(Some(attempt(&run, &[])), None);
        assert_eq!(
            evidence
                .decisive_evidence_timestamp(ConnectionCheckStatus::Pending)
                .expect("evidence timestamp"),
            Some(timestamp(CREATED_AT))
        );

        let no_run = CorrelatedGuardVerificationEvidence::new(None, None);
        assert_eq!(
            no_run
                .decisive_evidence_timestamp(ConnectionCheckStatus::Pending)
                .expect("absent evidence timestamp"),
            None
        );
    }

    #[test]
    fn awaiting_observation_uses_latest_applicable_persisted_evidence() {
        let run = run(
            "guard_verification_awaiting_observation",
            GuardIntegrationVerificationStatus::AwaitingObservation,
        );
        let observations = vec![
            observation(
                &run,
                GuardProbeObservationStage::ProbeAcknowledged,
                ACKNOWLEDGED_AT,
            ),
            observation(
                &run,
                GuardProbeObservationStage::PreToolMatched,
                OBSERVED_AT,
            ),
            observation(
                &run,
                GuardProbeObservationStage::UnrelatedRoutedTool,
                COMPLETED_AT,
            ),
        ];
        let evidence =
            CorrelatedGuardVerificationEvidence::new(Some(attempt(&run, &observations)), None);
        assert_eq!(
            evidence
                .decisive_evidence_timestamp(ConnectionCheckStatus::Pending)
                .expect("evidence timestamp"),
            Some(timestamp(OBSERVED_AT))
        );
    }

    #[test]
    fn repair_required_uses_the_terminal_transition_time() {
        let run = run(
            "guard_verification_repair_required",
            GuardIntegrationVerificationStatus::RepairRequired,
        );
        let evidence = CorrelatedGuardVerificationEvidence::new(Some(attempt(&run, &[])), None);
        assert_eq!(
            evidence
                .decisive_evidence_timestamp(ConnectionCheckStatus::Failed)
                .expect("evidence timestamp"),
            Some(timestamp(COMPLETED_AT))
        );
    }

    #[test]
    fn completed_check_uses_its_matching_proof_completion_time() {
        let run = run(
            "guard_verification_complete",
            GuardIntegrationVerificationStatus::Complete,
        );
        let observations = vec![observation(
            &run,
            GuardProbeObservationStage::PostToolMatched,
            COMPLETED_AT,
        )];
        let evidence = CorrelatedGuardVerificationEvidence::new(
            Some(attempt(&run, &observations)),
            Some(
                CorrelatedGuardProof::try_new(&run, &observations)
                    .expect("completed proof evidence"),
            ),
        );
        assert_eq!(
            evidence
                .decisive_evidence_timestamp(ConnectionCheckStatus::Passed)
                .expect("evidence timestamp"),
            Some(timestamp(COMPLETED_AT))
        );
        assert_ne!(timestamp(COMPLETED_AT), timestamp("2026-07-23T00:00:05Z"));
    }

    #[test]
    fn newer_pending_attempt_keeps_older_completed_proof_historical() {
        let completed = run(
            "guard_verification_historical_complete",
            GuardIntegrationVerificationStatus::Complete,
        );
        let completed_observations = vec![observation(
            &completed,
            GuardProbeObservationStage::PostToolMatched,
            COMPLETED_AT,
        )];
        let mut pending = run(
            "guard_verification_newer_pending",
            GuardIntegrationVerificationStatus::AwaitingProbe,
        );
        pending.created_at = "2026-07-23T00:01:00Z".to_owned();
        let evidence = CorrelatedGuardVerificationEvidence::new(
            Some(attempt(&pending, &[])),
            Some(
                CorrelatedGuardProof::try_new(&completed, &completed_observations)
                    .expect("historical proof"),
            ),
        );
        assert_eq!(
            evidence
                .decisive_evidence_timestamp(ConnectionCheckStatus::Pending)
                .expect("latest attempt timestamp"),
            Some(timestamp("2026-07-23T00:01:00Z"))
        );
        assert!(serde_json::to_value(evidence).expect("serialized evidence")
            ["latest_completed_proof"]
            .is_object());
    }

    #[test]
    fn passed_check_rejects_a_proof_from_another_identity() {
        let attempt_run = run(
            "guard_verification_current_complete",
            GuardIntegrationVerificationStatus::Complete,
        );
        let attempt_observations = vec![observation(
            &attempt_run,
            GuardProbeObservationStage::PostToolMatched,
            COMPLETED_AT,
        )];
        let mut proof_run = attempt_run.clone();
        proof_run.verification_id = "guard_verification_different".to_owned();
        proof_run.connection_internal_id = "different_connection".to_owned();
        let proof_observations = vec![observation(
            &proof_run,
            GuardProbeObservationStage::PostToolMatched,
            COMPLETED_AT,
        )];
        let evidence = CorrelatedGuardVerificationEvidence::new(
            Some(attempt(&attempt_run, &attempt_observations)),
            Some(
                CorrelatedGuardProof::try_new(&proof_run, &proof_observations)
                    .expect("mismatched proof can be decoded independently"),
            ),
        );
        assert!(evidence
            .decisive_evidence_timestamp(ConnectionCheckStatus::Passed)
            .expect_err("identity mismatch must fail")
            .contains("different verification identities"));
    }

    #[test]
    fn invalid_lifecycle_and_observation_chronology_fail_strictly() {
        let mut invalid_ack = run(
            "guard_verification_invalid_ack",
            GuardIntegrationVerificationStatus::AwaitingObservation,
        );
        invalid_ack.probe_acknowledged_at = Some("2026-07-23T00:00:00Z".to_owned());
        let invalid_workflow = IntegrationVerificationWorkflowState::AwaitingObservation {
            tool: volicord_types::integration_verification::IntegrationVerificationStatusToolReference::new(),
            acknowledged_at: timestamp("2026-07-23T00:00:00Z"),
            remaining_status_reads: 1,
        };
        assert!(
            CorrelatedGuardAttemptEvidence::try_new(&invalid_ack, &invalid_workflow, &[])
                .expect_err("acknowledgement before creation must fail")
                .contains("timestamp order")
        );

        let complete = run(
            "guard_verification_late_observation",
            GuardIntegrationVerificationStatus::Complete,
        );
        let observations = vec![observation(
            &complete,
            GuardProbeObservationStage::PostToolMatched,
            "2026-07-23T00:00:05Z",
        )];
        assert!(CorrelatedGuardProof::try_new(&complete, &observations)
            .expect_err("observation after completion must fail")
            .contains("outside its verification lifecycle"));
    }
}
