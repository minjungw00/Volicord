//! Typed human projection for correlated Guard verification evidence.

use serde::Deserialize;
use serde_json::Value;
use volicord_types::{
    connection_verification::{ActivationStepId, ConnectionCheckDetails, ConnectionCheckStatus},
    integration_verification::{
        GuardIntegrationVerificationStatus, GuardProbeObservationStage,
        GuardVerificationRecoverability, GuardVerificationRepairReason,
        GuardVerificationRetryPolicy,
    },
    tool_names::AgentToolId,
    values::UtcTimestamp,
};

use crate::connection_command::verification::evidence::recovery_action_for_retry_policy;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrelatedGuardEvidenceDetails {
    recoverability: Option<GuardVerificationRecoverability>,
    latest_attempt: Option<CorrelatedGuardAttemptEvidence>,
    latest_completed_proof: Option<CorrelatedGuardProof>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CorrelatedGuardEvidenceRole {
    GuardVerificationAttempt,
    GuardVerificationProof,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrelatedGuardAttemptEvidence {
    evidence_role: CorrelatedGuardEvidenceRole,
    verification_id: String,
    runtime_session_id: String,
    host_session_id: String,
    host_turn_id: String,
    attempt_state: GuardIntegrationVerificationStatus,
    prompt_event_id: Option<String>,
    pre_tool_event_id: Option<String>,
    post_tool_event_id: Option<String>,
    expected_agent_tool_id: AgentToolId,
    expected_host_callable_identity: String,
    observed_host_callable_identity: Option<String>,
    acquisition_stage: Option<GuardProbeObservationStage>,
    repair_reason: Option<GuardVerificationRepairReason>,
    retry_policy: Option<GuardVerificationRetryPolicy>,
    recoverability: Option<GuardVerificationRecoverability>,
    recovery_action: Option<ActivationStepId>,
    integration_revision: String,
    guard_installation_id: String,
    policy_digest: String,
    hook_definition_digest: String,
    created_at: UtcTimestamp,
    acknowledged_at: Option<UtcTimestamp>,
    completed_at: Option<UtcTimestamp>,
    terminal_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrelatedGuardProof {
    evidence_role: CorrelatedGuardEvidenceRole,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorrelatedGuardHumanProjection {
    pub(super) shared_correlation: Vec<CorrelationCoordinate>,
    pub(super) attempt: Option<CorrelatedGuardAttemptLifecycle>,
    pub(super) completed_proof: Option<CorrelatedGuardCompletedProof>,
    pub(super) divergence: Vec<CorrelatedGuardCoordinateDivergence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorrelatedGuardAttemptLifecycle {
    state: GuardIntegrationVerificationStatus,
    repair_reason: Option<GuardVerificationRepairReason>,
    retry_policy: Option<GuardVerificationRetryPolicy>,
    recoverability: Option<GuardVerificationRecoverability>,
    recovery_action: Option<ActivationStepId>,
    created_at: UtcTimestamp,
    acknowledged_at: Option<UtcTimestamp>,
    completed_at: Option<UtcTimestamp>,
    terminal_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorrelatedGuardCompletedProof {
    completed_at: UtcTimestamp,
    earlier_than_attempt: bool,
    historical_without_attempt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorrelatedGuardCoordinateDivergence {
    kind: CorrelationCoordinateKind,
    attempt: Option<CorrelationCoordinateValue>,
    proof: Option<CorrelationCoordinateValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorrelationCoordinate {
    kind: CorrelationCoordinateKind,
    value: CorrelationCoordinateValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CorrelationCoordinateKind {
    VerificationId,
    RuntimeSession,
    HostSession,
    HostTurn,
    PromptEvent,
    PreToolEvent,
    PostToolEvent,
    ExpectedAgentToolId,
    ExpectedHostCallable,
    ObservedHostCallable,
    AcquisitionStage,
    IntegrationRevision,
    GuardInstallationId,
    PolicyDigest,
    HookDefinitionDigest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CorrelationCoordinateValue {
    Text(String),
    AgentTool(AgentToolId),
    AcquisitionStage(GuardProbeObservationStage),
}

impl CorrelatedGuardHumanProjection {
    pub(super) fn try_from_details(
        details: &ConnectionCheckDetails,
        status: ConnectionCheckStatus,
    ) -> Result<Self, String> {
        let source: CorrelatedGuardEvidenceDetails =
            serde_json::from_value(Value::Object(details.as_object().clone()))
                .map_err(|error| format!("invalid correlated Guard evidence details: {error}"))?;
        source.validate(status)?;

        let attempt_correlation = source
            .latest_attempt
            .as_ref()
            .map(CorrelatedGuardAttemptEvidence::correlation);
        let proof_correlation = source
            .latest_completed_proof
            .as_ref()
            .map(CorrelatedGuardProof::correlation);
        let mut shared_correlation = Vec::new();
        let mut divergence = Vec::new();
        compare_correlations(
            attempt_correlation.as_ref(),
            proof_correlation.as_ref(),
            &mut shared_correlation,
            &mut divergence,
        );

        let earlier_than_attempt = !divergence.is_empty();
        let historical_without_attempt = source.latest_attempt.is_none();
        Ok(Self {
            shared_correlation,
            attempt: source.latest_attempt.map(Into::into),
            completed_proof: source.latest_completed_proof.map(|proof| {
                Self::proof_projection(proof, earlier_than_attempt, historical_without_attempt)
            }),
            divergence,
        })
    }

    fn proof_projection(
        proof: CorrelatedGuardProof,
        earlier_than_attempt: bool,
        historical_without_attempt: bool,
    ) -> CorrelatedGuardCompletedProof {
        CorrelatedGuardCompletedProof {
            completed_at: proof.completed_at,
            earlier_than_attempt,
            historical_without_attempt,
        }
    }
}

impl CorrelatedGuardEvidenceDetails {
    fn validate(&self, status: ConnectionCheckStatus) -> Result<(), String> {
        if self.recoverability
            != self
                .latest_attempt
                .as_ref()
                .and_then(|attempt| attempt.recoverability)
        {
            return Err(
                "correlated Guard recoverability does not match the latest attempt".to_owned(),
            );
        }
        if let Some(attempt) = self.latest_attempt.as_ref() {
            attempt.validate()?;
        }
        if let Some(proof) = self.latest_completed_proof.as_ref() {
            proof.validate()?;
        }

        match status {
            ConnectionCheckStatus::Passed => {
                let attempt = self.latest_attempt.as_ref().ok_or_else(|| {
                    "passed correlated Guard check is missing its latest attempt".to_owned()
                })?;
                let proof = self.latest_completed_proof.as_ref().ok_or_else(|| {
                    "passed correlated Guard check is missing its completed proof".to_owned()
                })?;
                if attempt.attempt_state != GuardIntegrationVerificationStatus::Complete {
                    return Err(
                        "passed correlated Guard check does not have a complete attempt".to_owned(),
                    );
                }
                validate_attempt_proof_relationship(attempt, proof)?;
            }
            ConnectionCheckStatus::Pending => {
                if self.latest_attempt.as_ref().is_some_and(|attempt| {
                    !matches!(
                        attempt.attempt_state,
                        GuardIntegrationVerificationStatus::AwaitingProbe
                            | GuardIntegrationVerificationStatus::AwaitingObservation
                    )
                }) {
                    return Err(
                        "pending correlated Guard check has a terminal latest attempt".to_owned(),
                    );
                }
                if let (Some(attempt), Some(proof)) =
                    (&self.latest_attempt, &self.latest_completed_proof)
                {
                    validate_attempt_proof_relationship(attempt, proof)?;
                }
            }
            ConnectionCheckStatus::Failed => {
                let attempt = self.latest_attempt.as_ref().ok_or_else(|| {
                    "failed correlated Guard check is missing its latest attempt".to_owned()
                })?;
                if attempt.attempt_state != GuardIntegrationVerificationStatus::RepairRequired {
                    return Err(
                        "failed correlated Guard check does not have a repair-required attempt"
                            .to_owned(),
                    );
                }
                if let Some(proof) = self.latest_completed_proof.as_ref() {
                    validate_attempt_proof_relationship(attempt, proof)?;
                }
            }
            ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                if let (Some(attempt), Some(proof)) =
                    (&self.latest_attempt, &self.latest_completed_proof)
                {
                    validate_attempt_proof_relationship(attempt, proof)?;
                }
            }
        }
        Ok(())
    }
}

impl CorrelatedGuardAttemptEvidence {
    fn validate(&self) -> Result<(), String> {
        if self.evidence_role != CorrelatedGuardEvidenceRole::GuardVerificationAttempt {
            return Err("latest Guard attempt has the wrong evidence role".to_owned());
        }
        if self.expected_agent_tool_id != AgentToolId::GUARD_PROBE {
            return Err("latest Guard attempt has the wrong expected AgentToolId".to_owned());
        }
        validate_lifecycle_order(
            &self.created_at,
            self.acknowledged_at.as_ref(),
            self.terminal_at.as_ref(),
        )?;

        let repair_fields = (
            self.repair_reason,
            self.retry_policy,
            self.recoverability,
            self.recovery_action,
        );
        match self.attempt_state {
            GuardIntegrationVerificationStatus::AwaitingProbe => {
                if self.acknowledged_at.is_some()
                    || self.completed_at.is_some()
                    || self.terminal_at.is_some()
                    || repair_fields != (None, None, None, None)
                {
                    return Err(
                        "awaiting-probe Guard attempt has terminal lifecycle facts".to_owned()
                    );
                }
            }
            GuardIntegrationVerificationStatus::AwaitingObservation => {
                if self.acknowledged_at.is_none()
                    || self.completed_at.is_some()
                    || self.terminal_at.is_some()
                    || repair_fields != (None, None, None, None)
                {
                    return Err(
                        "awaiting-observation Guard attempt has inconsistent lifecycle facts"
                            .to_owned(),
                    );
                }
            }
            GuardIntegrationVerificationStatus::Complete => {
                if self.prompt_event_id.is_none()
                    || self.pre_tool_event_id.is_none()
                    || self.post_tool_event_id.is_none()
                    || self.acknowledged_at.is_none()
                    || self.observed_host_callable_identity.is_none()
                    || self.acquisition_stage != Some(GuardProbeObservationStage::PostToolMatched)
                    || self.completed_at.is_none()
                    || self.completed_at != self.terminal_at
                    || repair_fields != (None, None, None, None)
                {
                    return Err("complete Guard attempt has inconsistent proof facts".to_owned());
                }
            }
            GuardIntegrationVerificationStatus::RepairRequired => {
                let (Some(_), Some(retry_policy), Some(recoverability), Some(recovery_action)) =
                    repair_fields
                else {
                    return Err(
                        "repair-required Guard attempt is missing recovery facts".to_owned()
                    );
                };
                if self.completed_at.is_some()
                    || self.terminal_at.is_none()
                    || recoverability
                        != GuardVerificationRecoverability::from_retry_policy(retry_policy)
                    || recovery_action != recovery_action_for_retry_policy(retry_policy)
                {
                    return Err(
                        "repair-required Guard attempt has inconsistent recovery facts".to_owned(),
                    );
                }
            }
        }
        Ok(())
    }

    fn correlation(&self) -> CorrelatedGuardCorrelation {
        CorrelatedGuardCorrelation {
            verification_id: self.verification_id.clone(),
            runtime_session_id: self.runtime_session_id.clone(),
            host_session_id: self.host_session_id.clone(),
            host_turn_id: self.host_turn_id.clone(),
            prompt_event_id: self.prompt_event_id.clone(),
            pre_tool_event_id: self.pre_tool_event_id.clone(),
            post_tool_event_id: self.post_tool_event_id.clone(),
            expected_agent_tool_id: self.expected_agent_tool_id,
            expected_host_callable_identity: self.expected_host_callable_identity.clone(),
            observed_host_callable_identity: self.observed_host_callable_identity.clone(),
            acquisition_stage: self.acquisition_stage,
            integration_revision: self.integration_revision.clone(),
            guard_installation_id: self.guard_installation_id.clone(),
            policy_digest: self.policy_digest.clone(),
            hook_definition_digest: self.hook_definition_digest.clone(),
        }
    }
}

impl CorrelatedGuardProof {
    fn validate(&self) -> Result<(), String> {
        if self.evidence_role != CorrelatedGuardEvidenceRole::GuardVerificationProof {
            return Err("completed Guard proof has the wrong evidence role".to_owned());
        }
        if self.expected_agent_tool_id != AgentToolId::GUARD_PROBE
            || self.acquisition_stage != GuardProbeObservationStage::PostToolMatched
        {
            return Err("completed Guard proof has inconsistent acquisition facts".to_owned());
        }
        Ok(())
    }

    fn correlation(&self) -> CorrelatedGuardCorrelation {
        CorrelatedGuardCorrelation {
            verification_id: self.verification_id.clone(),
            runtime_session_id: self.runtime_session_id.clone(),
            host_session_id: self.host_session_id.clone(),
            host_turn_id: self.host_turn_id.clone(),
            prompt_event_id: Some(self.prompt_event_id.clone()),
            pre_tool_event_id: Some(self.pre_tool_event_id.clone()),
            post_tool_event_id: Some(self.post_tool_event_id.clone()),
            expected_agent_tool_id: self.expected_agent_tool_id,
            expected_host_callable_identity: self.expected_host_callable_identity.clone(),
            observed_host_callable_identity: Some(self.observed_host_callable_identity.clone()),
            acquisition_stage: Some(self.acquisition_stage),
            integration_revision: self.integration_revision.clone(),
            guard_installation_id: self.guard_installation_id.clone(),
            policy_digest: self.policy_digest.clone(),
            hook_definition_digest: self.hook_definition_digest.clone(),
        }
    }
}

fn validate_attempt_proof_relationship(
    attempt: &CorrelatedGuardAttemptEvidence,
    proof: &CorrelatedGuardProof,
) -> Result<(), String> {
    let attempt_correlation = attempt.correlation();
    let proof_correlation = proof.correlation();
    if attempt.verification_id == proof.verification_id {
        if attempt.attempt_state != GuardIntegrationVerificationStatus::Complete
            || attempt_correlation != proof_correlation
            || attempt.completed_at.as_ref() != Some(&proof.completed_at)
            || attempt.terminal_at.as_ref() != Some(&proof.completed_at)
        {
            return Err(
                "attempt and proof for one Guard verification have inconsistent identity or completion"
                    .to_owned(),
            );
        }
        return Ok(());
    }

    if attempt.attempt_state == GuardIntegrationVerificationStatus::Complete
        || proof.completed_at > attempt.created_at
    {
        return Err(
            "completed Guard proof is not an earlier proof for the latest attempt".to_owned(),
        );
    }
    Ok(())
}

fn validate_lifecycle_order(
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CorrelatedGuardCorrelation {
    verification_id: String,
    runtime_session_id: String,
    host_session_id: String,
    host_turn_id: String,
    prompt_event_id: Option<String>,
    pre_tool_event_id: Option<String>,
    post_tool_event_id: Option<String>,
    expected_agent_tool_id: AgentToolId,
    expected_host_callable_identity: String,
    observed_host_callable_identity: Option<String>,
    acquisition_stage: Option<GuardProbeObservationStage>,
    integration_revision: String,
    guard_installation_id: String,
    policy_digest: String,
    hook_definition_digest: String,
}

impl CorrelatedGuardCorrelation {
    fn coordinates(
        &self,
    ) -> Vec<(
        CorrelationCoordinateKind,
        Option<CorrelationCoordinateValue>,
    )> {
        use CorrelationCoordinateKind as Kind;
        use CorrelationCoordinateValue as Coordinate;

        vec![
            (
                Kind::VerificationId,
                Some(Coordinate::Text(self.verification_id.clone())),
            ),
            (
                Kind::RuntimeSession,
                Some(Coordinate::Text(self.runtime_session_id.clone())),
            ),
            (
                Kind::HostSession,
                Some(Coordinate::Text(self.host_session_id.clone())),
            ),
            (
                Kind::HostTurn,
                Some(Coordinate::Text(self.host_turn_id.clone())),
            ),
            (
                Kind::PromptEvent,
                self.prompt_event_id.clone().map(Coordinate::Text),
            ),
            (
                Kind::PreToolEvent,
                self.pre_tool_event_id.clone().map(Coordinate::Text),
            ),
            (
                Kind::PostToolEvent,
                self.post_tool_event_id.clone().map(Coordinate::Text),
            ),
            (
                Kind::ExpectedAgentToolId,
                Some(Coordinate::AgentTool(self.expected_agent_tool_id)),
            ),
            (
                Kind::ExpectedHostCallable,
                Some(Coordinate::Text(
                    self.expected_host_callable_identity.clone(),
                )),
            ),
            (
                Kind::ObservedHostCallable,
                self.observed_host_callable_identity
                    .clone()
                    .map(Coordinate::Text),
            ),
            (
                Kind::AcquisitionStage,
                self.acquisition_stage.map(Coordinate::AcquisitionStage),
            ),
            (
                Kind::IntegrationRevision,
                Some(Coordinate::Text(self.integration_revision.clone())),
            ),
            (
                Kind::GuardInstallationId,
                Some(Coordinate::Text(self.guard_installation_id.clone())),
            ),
            (
                Kind::PolicyDigest,
                Some(Coordinate::Text(self.policy_digest.clone())),
            ),
            (
                Kind::HookDefinitionDigest,
                Some(Coordinate::Text(self.hook_definition_digest.clone())),
            ),
        ]
    }
}

fn compare_correlations(
    attempt: Option<&CorrelatedGuardCorrelation>,
    proof: Option<&CorrelatedGuardCorrelation>,
    shared: &mut Vec<CorrelationCoordinate>,
    divergence: &mut Vec<CorrelatedGuardCoordinateDivergence>,
) {
    match (attempt, proof) {
        (Some(attempt), Some(proof)) => {
            for ((kind, attempt), (proof_kind, proof)) in
                attempt.coordinates().into_iter().zip(proof.coordinates())
            {
                debug_assert_eq!(kind, proof_kind);
                if attempt == proof {
                    if let Some(value) = attempt {
                        shared.push(CorrelationCoordinate { kind, value });
                    }
                } else {
                    divergence.push(CorrelatedGuardCoordinateDivergence {
                        kind,
                        attempt,
                        proof,
                    });
                }
            }
        }
        (Some(correlation), None) | (None, Some(correlation)) => {
            shared.extend(
                correlation
                    .coordinates()
                    .into_iter()
                    .filter_map(|(kind, value)| {
                        value.map(|value| CorrelationCoordinate { kind, value })
                    }),
            );
        }
        (None, None) => {}
    }
}

impl From<CorrelatedGuardAttemptEvidence> for CorrelatedGuardAttemptLifecycle {
    fn from(attempt: CorrelatedGuardAttemptEvidence) -> Self {
        Self {
            state: attempt.attempt_state,
            repair_reason: attempt.repair_reason,
            retry_policy: attempt.retry_policy,
            recoverability: attempt.recoverability,
            recovery_action: attempt.recovery_action,
            created_at: attempt.created_at,
            acknowledged_at: attempt.acknowledged_at,
            completed_at: attempt.completed_at,
            terminal_at: attempt.terminal_at,
        }
    }
}

impl CorrelationCoordinate {
    pub(super) fn label(&self) -> &'static str {
        self.kind.label()
    }

    pub(super) fn value(&self) -> String {
        self.value.render()
    }
}

impl CorrelatedGuardCoordinateDivergence {
    pub(super) fn label(&self) -> &'static str {
        self.kind.label()
    }

    pub(super) fn attempt_value(&self) -> String {
        render_optional_coordinate(self.attempt.as_ref())
    }

    pub(super) fn proof_value(&self) -> String {
        render_optional_coordinate(self.proof.as_ref())
    }
}

impl CorrelationCoordinateKind {
    const fn label(self) -> &'static str {
        match self {
            Self::VerificationId => "Verification ID",
            Self::RuntimeSession => "Runtime session",
            Self::HostSession => "Host session",
            Self::HostTurn => "Host turn",
            Self::PromptEvent => "Prompt event",
            Self::PreToolEvent => "Pre-tool event",
            Self::PostToolEvent => "Post-tool event",
            Self::ExpectedAgentToolId => "Expected AgentToolId",
            Self::ExpectedHostCallable => "Expected host callable",
            Self::ObservedHostCallable => "Observed host callable",
            Self::AcquisitionStage => "Acquisition stage",
            Self::IntegrationRevision => "Integration revision",
            Self::GuardInstallationId => "Guard installation ID",
            Self::PolicyDigest => "Policy digest",
            Self::HookDefinitionDigest => "Hook definition digest",
        }
    }
}

impl CorrelationCoordinateValue {
    fn render(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::AgentTool(value) => value.wire_name().to_owned(),
            Self::AcquisitionStage(value) => humanize_enum(value.as_str()),
        }
    }
}

impl CorrelatedGuardAttemptLifecycle {
    pub(super) fn state(&self) -> String {
        humanize_enum(self.state.as_str())
    }

    pub(super) fn repair_reason(&self) -> Option<String> {
        self.repair_reason
            .map(|value| humanize_enum(value.as_str()))
    }

    pub(super) fn retry_policy(&self) -> Option<String> {
        self.retry_policy.map(|value| humanize_enum(value.as_str()))
    }

    pub(super) fn recoverability(&self) -> Option<&'static str> {
        self.recoverability.map(|value| match value {
            GuardVerificationRecoverability::Recoverable => "recoverable",
            GuardVerificationRecoverability::NotRecoverable => "not recoverable",
        })
    }

    pub(super) fn recovery_action(&self) -> Option<String> {
        self.recovery_action
            .map(|value| humanize_enum(value.as_str()))
    }

    pub(super) fn created_at(&self) -> &UtcTimestamp {
        &self.created_at
    }

    pub(super) fn acknowledged_at(&self) -> Option<&UtcTimestamp> {
        self.acknowledged_at.as_ref()
    }

    pub(super) fn completed_at(&self) -> Option<&UtcTimestamp> {
        self.completed_at.as_ref()
    }

    pub(super) fn terminal_at(&self) -> Option<&UtcTimestamp> {
        self.terminal_at.as_ref()
    }
}

impl CorrelatedGuardCompletedProof {
    pub(super) fn completed_at(&self) -> &UtcTimestamp {
        &self.completed_at
    }

    pub(super) fn is_earlier_than_attempt(&self) -> bool {
        self.earlier_than_attempt
    }

    pub(super) fn is_historical_without_attempt(&self) -> bool {
        self.historical_without_attempt
    }
}

fn render_optional_coordinate(value: Option<&CorrelationCoordinateValue>) -> String {
    value
        .map(CorrelationCoordinateValue::render)
        .unwrap_or_else(|| "not acquired".to_owned())
}

fn humanize_enum(value: &str) -> String {
    value.replace('_', " ")
}
