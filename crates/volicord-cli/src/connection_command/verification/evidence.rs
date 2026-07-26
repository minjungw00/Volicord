//! Typed evidence projected into MCP-related connection-check details.

use serde::Serialize;
use serde_json::Value;
use volicord_store::{
    integration_verification::{
        GuardIntegrationVerificationRunRecord, GuardProbeObservationRecord,
    },
    operational_sessions::{ManagedCapabilityProof, ManagedPeerObservation, McpSessionMilestones},
};
use volicord_types::connection_verification::ActivationStepId;
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::integration_verification::{
    GuardIntegrationVerificationStatus, GuardProbeObservationStage,
    GuardVerificationRecoverability, GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    IntegrationVerificationWorkflowState,
};
use volicord_types::tool_names::{AgentToolId, ToolVerificationRole};
use volicord_types::values::UtcTimestamp;

use super::{HostExecutableStatus, Verification};
use crate::host_integration::verification::HostExecutableProbe;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct AmbientGuardCoverageEvidence {
    current_hook_definition_executed: bool,
    configured_phases_observed: bool,
    installation_ids: Vec<String>,
    affected_paths: Vec<String>,
    artifact_issues: Vec<Value>,
    manifest_issues: Vec<&'static str>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CorrelatedGuardAttemptEvidence {
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
        let terminal_timestamp = run
            .completed_at
            .as_deref()
            .map(parse_guard_timestamp)
            .transpose()?;
        Ok(Self {
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
            created_at: parse_guard_timestamp(&run.created_at)?,
            acknowledged_at: run
                .probe_acknowledged_at
                .as_deref()
                .map(parse_guard_timestamp)
                .transpose()?,
            completed_at: (attempt_state == GuardIntegrationVerificationStatus::Complete)
                .then(|| terminal_timestamp.clone())
                .flatten(),
            terminal_at: terminal_timestamp,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CorrelatedGuardProof {
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
        let required = |value: &Option<String>, field: &str| {
            value
                .clone()
                .ok_or_else(|| format!("completed Guard proof is missing {field}"))
        };
        let matched_observation = observations
            .iter()
            .rev()
            .find(|observation| observation.stage == GuardProbeObservationStage::PostToolMatched)
            .ok_or_else(|| {
                "completed Guard proof is missing its post-tool matched observation".to_owned()
            })?;
        Ok(Self {
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
            completed_at: parse_guard_timestamp(
                run.completed_at
                    .as_deref()
                    .ok_or_else(|| "completed Guard proof is missing completed_at".to_owned())?,
            )?,
        })
    }
}

fn parse_guard_timestamp(value: &str) -> Result<UtcTimestamp, String> {
    UtcTimestamp::parse(value)
        .map_err(|_| format!("Guard verification timestamp is not canonical: {value}"))
}

pub(super) fn recovery_action_for_retry_policy(
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
