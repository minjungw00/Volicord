//! Typed action selection for operational conditions.

use volicord_types::{DiagnosticAction, DiagnosticCode};

use super::{
    definitions::{
        GuardDiagnostic, InstallationDiagnostic, OperationalDiagnostic,
        OperationalDiagnosticDefinition, RevisionDiagnostic, TrustDiagnostic,
    },
    facts::TypedOperationalFacts,
};

/// Current owner check state used when selecting finding actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalCheckState {
    Passed,
    Pending,
    Failed,
    Blocked,
    NotApplicable,
}

/// Closed actions attached to CLI-owned findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalRecommendedAction {
    ReinstallCurrentBuild,
    RepairManagedConfiguration,
    ReloadHostAfterConfigurationChange,
    RepairGuard,
    TriggerGuardPhase,
    RetryGuardVerification,
    InspectRuntimeSession,
    TriggerPromptCapture,
    UseSupportedPromptCaptureHost,
    TrustRepository,
    RepairTrustConfiguration,
    ReobserveCurrentRevision,
    VerifyDesignatedTool,
}

impl OperationalRecommendedAction {
    pub const fn code(self) -> &'static str {
        match self {
            Self::ReinstallCurrentBuild => "action.installation.reinstall_current_build",
            Self::RepairManagedConfiguration => "action.managed_config.repair",
            Self::ReloadHostAfterConfigurationChange => {
                "action.host.reload_after_configuration_change"
            }
            Self::RepairGuard => "action.guard.repair",
            Self::TriggerGuardPhase => "action.guard.trigger_phase",
            Self::RetryGuardVerification => "action.guard.retry_verification",
            Self::InspectRuntimeSession => "action.mcp.inspect_runtime_session",
            Self::TriggerPromptCapture => "action.guard.trigger_prompt_capture",
            Self::UseSupportedPromptCaptureHost => "action.guard.use_supported_prompt_capture_host",
            Self::TrustRepository => "action.trust.approve_repository",
            Self::RepairTrustConfiguration => "action.trust.repair_configuration",
            Self::ReobserveCurrentRevision => "action.revision.reobserve_current",
            Self::VerifyDesignatedTool => "action.mcp.verify_designated_tool",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::ReinstallCurrentBuild => "Reinstall the current Volicord build",
            Self::RepairManagedConfiguration => "Repair the managed host configuration",
            Self::ReloadHostAfterConfigurationChange => {
                "Reload the host after applying the configuration change"
            }
            Self::RepairGuard => "Repair the current Guard installation",
            Self::TriggerGuardPhase => "Trigger the required unobserved Guard phase",
            Self::RetryGuardVerification => {
                "Start a later Guard verification attempt after satisfying its retry policy"
            }
            Self::InspectRuntimeSession => {
                "Inspect the managed runtime session for the failed Guard verification attempt"
            }
            Self::TriggerPromptCapture => "Trigger one configured prompt-capture observation",
            Self::UseSupportedPromptCaptureHost => {
                "Use a host boundary that supports prompt capture"
            }
            Self::TrustRepository => "Approve the Product Repository in the current host",
            Self::RepairTrustConfiguration => "Repair repository trust configuration",
            Self::ReobserveCurrentRevision => "Run verification against the current revision",
            Self::VerifyDesignatedTool => {
                "Run the canonical verification tool through the current managed host"
            }
        }
    }
}

pub(crate) fn actions_for<T: TypedOperationalFacts>(
    definition: &OperationalDiagnosticDefinition,
    facts: &T,
    check_state: OperationalCheckState,
) -> Vec<DiagnosticAction> {
    assert!(
        facts.supports(definition.diagnostic()),
        "typed operational facts do not match their immutable diagnostic definition"
    );
    if !matches!(
        check_state,
        OperationalCheckState::Pending | OperationalCheckState::Failed
    ) {
        return Vec::new();
    }

    recommended_actions(definition.diagnostic())
        .iter()
        .map(|action| {
            DiagnosticAction::try_new(
                DiagnosticCode::parse(action.code())
                    .expect("operational action codes are immutable valid definitions"),
                action.summary(),
            )
            .expect("operational action summaries are immutable bounded definitions")
        })
        .collect()
}

const fn recommended_actions(
    diagnostic: OperationalDiagnostic,
) -> &'static [OperationalRecommendedAction] {
    use OperationalRecommendedAction as Action;
    match diagnostic {
        OperationalDiagnostic::Installation(InstallationDiagnostic::ExecutableMissing)
        | OperationalDiagnostic::Installation(InstallationDiagnostic::ExecutableNotRunnable)
        | OperationalDiagnostic::Installation(InstallationDiagnostic::BuildIdentityUnavailable) => {
            &[Action::ReinstallCurrentBuild]
        }
        OperationalDiagnostic::Installation(
            InstallationDiagnostic::ManagedConfigurationInconsistent,
        ) => &[
            Action::ReinstallCurrentBuild,
            Action::RepairManagedConfiguration,
        ],
        OperationalDiagnostic::ManagedConfig(_) => &[Action::RepairManagedConfiguration],
        OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved) => {
            &[Action::TriggerGuardPhase]
        }
        OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnobserved) => {
            &[Action::TriggerPromptCapture]
        }
        OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnsupported) => {
            &[Action::UseSupportedPromptCaptureHost]
        }
        OperationalDiagnostic::Guard(
            GuardDiagnostic::ProbeHookEventNotObserved
            | GuardDiagnostic::ProbeVerificationIdMismatch
            | GuardDiagnostic::ProbeTurnMismatch
            | GuardDiagnostic::ProbeToolUseMismatch,
        ) => &[Action::RetryGuardVerification],
        OperationalDiagnostic::Guard(GuardDiagnostic::ProbeSessionMismatch) => {
            &[Action::InspectRuntimeSession]
        }
        OperationalDiagnostic::Guard(GuardDiagnostic::ProbeCurrentContractChanged) => {
            &[Action::RepairManagedConfiguration]
        }
        OperationalDiagnostic::Guard(_) => &[Action::RepairGuard],
        OperationalDiagnostic::Trust(TrustDiagnostic::RepositoryNotTrusted) => {
            &[Action::TrustRepository]
        }
        OperationalDiagnostic::Trust(
            TrustDiagnostic::ObservationUnavailable | TrustDiagnostic::ConfigurationMalformed,
        ) => &[Action::RepairTrustConfiguration],
        OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale) => {
            &[Action::ReloadHostAfterConfigurationChange]
        }
        OperationalDiagnostic::Revision(RevisionDiagnostic::ObservationMismatch) => {
            &[Action::ReobserveCurrentRevision]
        }
        OperationalDiagnostic::ToolVerification(_) => &[Action::VerifyDesignatedTool],
    }
}

#[cfg(test)]
mod tests {
    use crate::host_integration::verification::{ManagedConfigDiagnostic, ManagedConfigStatus};

    use super::*;
    use crate::operational_diagnostics::facts::ManagedConfigurationFacts;

    #[test]
    fn action_derivation_uses_definition_typed_facts_and_state_not_prose() {
        let definition = ManagedConfigDiagnostic::StaticEnvironmentDrift.definition();
        let facts = ManagedConfigurationFacts::from_status(ManagedConfigStatus::Changed);
        let failed = actions_for(definition, &facts, OperationalCheckState::Failed);
        let passed = actions_for(definition, &facts, OperationalCheckState::Passed);

        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].code().as_str(), "action.managed_config.repair");
        assert!(passed.is_empty());
        assert!(!definition.summary().contains("restart"));
    }
}
