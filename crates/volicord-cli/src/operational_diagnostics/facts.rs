//! Typed, bounded fact inputs for operational-diagnostic families.

use serde::Serialize;
use volicord_types::connection_verification::ActivationStepId;
use volicord_types::diagnostics::{DiagnosticFactSource, DiagnosticFacts};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::integration_verification::{
    GuardProbeObservationStage, GuardVerificationRecoverability, GuardVerificationRepairReason,
    GuardVerificationRetryPolicy,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::GuardHookPhase;

use crate::host_integration::verification::{ManagedConfigStatus, ProjectTrustStatus};

use super::definitions::{
    GuardDiagnostic, OperationalDiagnostic, OperationalDiagnosticDefinition, RevisionDiagnostic,
};

pub(crate) trait TypedOperationalFacts: Serialize {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool;
}

#[derive(Debug, Default, Serialize)]
pub struct InstallationFacts {}

impl TypedOperationalFacts for InstallationFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(diagnostic, OperationalDiagnostic::Installation(_))
    }
}

#[derive(Debug, Serialize)]
pub struct ManagedConfigurationFacts {
    observed_state: &'static str,
}

impl ManagedConfigurationFacts {
    pub fn from_status(status: ManagedConfigStatus) -> Self {
        Self {
            observed_state: status.as_str(),
        }
    }
}

impl TypedOperationalFacts for ManagedConfigurationFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(diagnostic, OperationalDiagnostic::ManagedConfig(_))
    }
}

#[derive(Debug, Serialize)]
pub struct GuardArtifactFacts {
    artifact_kind: String,
}

impl GuardArtifactFacts {
    pub fn new(artifact_kind: impl Into<String>) -> Self {
        Self {
            artifact_kind: artifact_kind.into(),
        }
    }
}

impl TypedOperationalFacts for GuardArtifactFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(
            diagnostic,
            OperationalDiagnostic::Guard(
                GuardDiagnostic::ManagedFileMissing
                    | GuardDiagnostic::ManagedFileIntegrityFailure
                    | GuardDiagnostic::HookWrapperMissing
                    | GuardDiagnostic::HookWrapperNotExecutable
                    | GuardDiagnostic::HookProcessFailure
            )
        )
    }
}

#[derive(Debug, Default, Serialize)]
pub struct GuardInstallationFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_state: Option<&'static str>,
}

impl TypedOperationalFacts for GuardInstallationFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(
            diagnostic,
            OperationalDiagnostic::Guard(
                GuardDiagnostic::ManifestMismatch
                    | GuardDiagnostic::HookWrapperMissing
                    | GuardDiagnostic::HookWrapperNotExecutable
                    | GuardDiagnostic::HookProcessFailure
            )
        )
    }
}

#[derive(Debug, Serialize)]
pub struct GuardPhaseFacts {
    guard_phase: &'static str,
}

impl GuardPhaseFacts {
    pub const fn new(phase: GuardHookPhase) -> Self {
        Self {
            guard_phase: phase.as_str(),
        }
    }
}

impl TypedOperationalFacts for GuardPhaseFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(
            diagnostic,
            OperationalDiagnostic::Guard(
                GuardDiagnostic::RequiredPhaseNotObserved
                    | GuardDiagnostic::PromptCaptureUnsupported
                    | GuardDiagnostic::PromptCaptureUnobserved
            )
        )
    }
}

#[derive(Debug, Default, Serialize)]
pub struct GuardEventFacts {}

impl TypedOperationalFacts for GuardEventFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(
            diagnostic,
            OperationalDiagnostic::Guard(GuardDiagnostic::IncompatibleObservation)
        )
    }
}

#[derive(Debug, Serialize)]
pub struct GuardProbeFacts {
    repair_reason: GuardVerificationRepairReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    acquisition_stage: Option<GuardProbeObservationStage>,
    retry_policy: GuardVerificationRetryPolicy,
    recoverability: GuardVerificationRecoverability,
    recovery_action: ActivationStepId,
    expected_agent_tool_id: AgentToolId,
    expected_host_callable_identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_host_callable_identity: Option<String>,
}

impl GuardProbeFacts {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repair_reason: GuardVerificationRepairReason,
        acquisition_stage: Option<GuardProbeObservationStage>,
        retry_policy: GuardVerificationRetryPolicy,
        recoverability: GuardVerificationRecoverability,
        recovery_action: ActivationStepId,
        expected_host_callable_identity: impl Into<String>,
        observed_host_callable_identity: Option<String>,
    ) -> Self {
        Self {
            repair_reason,
            acquisition_stage,
            retry_policy,
            recoverability,
            recovery_action,
            expected_agent_tool_id: AgentToolId::GUARD_PROBE,
            expected_host_callable_identity: expected_host_callable_identity.into(),
            observed_host_callable_identity,
        }
    }
}

impl TypedOperationalFacts for GuardProbeFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(
            diagnostic,
            OperationalDiagnostic::Guard(
                GuardDiagnostic::ProbeHookEventNotObserved
                    | GuardDiagnostic::ProbePayloadIncompatible
                    | GuardDiagnostic::ProbeCallableMismatch
                    | GuardDiagnostic::ProbeVerificationIdMismatch
                    | GuardDiagnostic::ProbeSessionMismatch
                    | GuardDiagnostic::ProbeTurnMismatch
                    | GuardDiagnostic::ProbeToolUseMismatch
                    | GuardDiagnostic::ProbeCurrentContractChanged
            )
        )
    }
}

#[derive(Debug, Serialize)]
pub struct IntegrationRevisionFacts {
    expected_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_revision: Option<String>,
}

impl IntegrationRevisionFacts {
    pub fn new(
        expected_revision: &IntegrationRevision,
        observed_revision: Option<&IntegrationRevision>,
    ) -> Self {
        Self {
            expected_revision: expected_revision.as_str().to_owned(),
            observed_revision: observed_revision.map(|value| value.as_str().to_owned()),
        }
    }
}

impl TypedOperationalFacts for IntegrationRevisionFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(
            diagnostic,
            OperationalDiagnostic::Revision(
                RevisionDiagnostic::IntegrationStale | RevisionDiagnostic::ObservationMismatch
            )
        )
    }
}

#[derive(Debug, Serialize)]
pub struct VerificationToolFacts {
    expected_tool_name: String,
    observed_tool_name: String,
}

impl VerificationToolFacts {
    pub fn new(
        expected_tool_name: impl Into<String>,
        observed_tool_name: impl Into<String>,
    ) -> Self {
        Self {
            expected_tool_name: expected_tool_name.into(),
            observed_tool_name: observed_tool_name.into(),
        }
    }
}

impl TypedOperationalFacts for VerificationToolFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(diagnostic, OperationalDiagnostic::ToolVerification(_))
    }
}

#[derive(Debug, Serialize)]
pub struct TrustFacts {
    observed_state: &'static str,
}

impl TrustFacts {
    pub fn from_status(status: ProjectTrustStatus) -> Self {
        Self {
            observed_state: status.as_str(),
        }
    }
}

impl TypedOperationalFacts for TrustFacts {
    fn supports(&self, diagnostic: OperationalDiagnostic) -> bool {
        matches!(diagnostic, OperationalDiagnostic::Trust(_))
    }
}

#[derive(Serialize)]
struct ProjectedFacts<'a, T> {
    summary: &'static str,
    #[serde(flatten)]
    facts: &'a T,
}

impl<T: Serialize> DiagnosticFactSource for ProjectedFacts<'_, T> {}

pub(crate) fn project_facts<T: TypedOperationalFacts>(
    definition: &OperationalDiagnosticDefinition,
    facts: &T,
) -> Result<DiagnosticFacts, volicord_types::diagnostics::DiagnosticError> {
    assert!(
        facts.supports(definition.diagnostic()),
        "typed operational facts do not match their immutable diagnostic definition"
    );
    DiagnosticFacts::project(&ProjectedFacts {
        summary: definition.summary(),
        facts,
    })
}
