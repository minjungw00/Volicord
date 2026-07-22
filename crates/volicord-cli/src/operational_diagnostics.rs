//! Typed findings for Volicord-owned administrative and integration boundaries.

use serde::Serialize;
use volicord_store::{
    diagnostic_findings::{diagnostic_finding, insert_diagnostic_finding},
    operational_sessions::connection_integration_revision,
};
use volicord_types::{
    AgentConnectionId, DiagnosticAction, DiagnosticCode, DiagnosticDomain, DiagnosticError,
    DiagnosticFactSource, DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId,
    DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject, GuardManagedArtifact,
    IntegrationRevision, UtcTimestamp,
};

use crate::{
    connection_command::ConnectionCommandError,
    guard_integration::audit::{
        GuardArtifactIssue, GuardManifestIssue, HookWrapperResolutionStatus,
    },
    host_integration::verification::{ManagedConfigDiagnostic, ProjectTrustStatus},
};

/// Closed installation diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationDiagnostic {
    ExecutableMissing,
    ExecutableNotRunnable,
    BuildIdentityUnavailable,
    ManagedConfigurationInconsistent,
}

impl InstallationDiagnostic {
    pub const ALL: [Self; 4] = [
        Self::ExecutableMissing,
        Self::ExecutableNotRunnable,
        Self::BuildIdentityUnavailable,
        Self::ManagedConfigurationInconsistent,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::ExecutableMissing => "installation.executable.missing",
            Self::ExecutableNotRunnable => "installation.executable.not_runnable",
            Self::BuildIdentityUnavailable => "installation.build_identity.unavailable",
            Self::ManagedConfigurationInconsistent => "installation.managed_config.inconsistent",
        }
    }
}

/// Closed Guard diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardDiagnostic {
    ManagedFileMissing,
    ManagedFileIntegrityFailure,
    ManifestMismatch,
    HookWrapperMissing,
    HookWrapperNotExecutable,
    HookProcessFailure,
    RequiredPhaseNotObserved,
    IncompatibleObservation,
    PromptCaptureUnsupported,
    PromptCaptureUnobserved,
}

impl GuardDiagnostic {
    pub const ALL: [Self; 10] = [
        Self::ManagedFileMissing,
        Self::ManagedFileIntegrityFailure,
        Self::ManifestMismatch,
        Self::HookWrapperMissing,
        Self::HookWrapperNotExecutable,
        Self::HookProcessFailure,
        Self::RequiredPhaseNotObserved,
        Self::IncompatibleObservation,
        Self::PromptCaptureUnsupported,
        Self::PromptCaptureUnobserved,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::ManagedFileMissing => "guard.managed_file.missing",
            Self::ManagedFileIntegrityFailure => "guard.managed_file.integrity_failed",
            Self::ManifestMismatch => "guard.manifest.mismatch",
            Self::HookWrapperMissing => "guard.hook_wrapper.missing",
            Self::HookWrapperNotExecutable => "guard.hook_wrapper.not_executable",
            Self::HookProcessFailure => "guard.hook_process.failed",
            Self::RequiredPhaseNotObserved => "guard.phase.required_not_observed",
            Self::IncompatibleObservation => "guard.observation.incompatible",
            Self::PromptCaptureUnsupported => "guard.prompt_capture.unsupported",
            Self::PromptCaptureUnobserved => "guard.prompt_capture.unobserved",
        }
    }

    pub(crate) const fn from_artifact_issue(
        artifact: GuardManagedArtifact,
        issue: GuardArtifactIssue,
    ) -> Self {
        match (artifact, issue) {
            (GuardManagedArtifact::HostHookWrapper(_), GuardArtifactIssue::Missing) => {
                Self::HookWrapperMissing
            }
            (GuardManagedArtifact::HostHookWrapper(_), GuardArtifactIssue::PermissionMismatch) => {
                Self::HookWrapperNotExecutable
            }
            (_, GuardArtifactIssue::Missing) => Self::ManagedFileMissing,
            (
                _,
                GuardArtifactIssue::Malformed
                | GuardArtifactIssue::ContentMismatch
                | GuardArtifactIssue::OwnershipMismatch
                | GuardArtifactIssue::PermissionMismatch
                | GuardArtifactIssue::HookContractMismatch,
            ) => Self::ManagedFileIntegrityFailure,
        }
    }

    pub(crate) const fn from_manifest_issue(_issue: GuardManifestIssue) -> Self {
        Self::ManifestMismatch
    }

    pub(crate) const fn from_hook_wrapper_status(
        status: HookWrapperResolutionStatus,
    ) -> Option<Self> {
        match status {
            HookWrapperResolutionStatus::MetadataMissing => Some(Self::HookWrapperMissing),
            HookWrapperResolutionStatus::AuthorityMismatch
            | HookWrapperResolutionStatus::PolicyHashMismatch
            | HookWrapperResolutionStatus::HostOutputMismatch => Some(Self::ManifestMismatch),
            HookWrapperResolutionStatus::Ok => None,
        }
    }
}

/// Closed trust diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDiagnostic {
    RepositoryNotTrusted,
    ObservationUnavailable,
    ConfigurationMalformed,
}

impl TrustDiagnostic {
    pub const ALL: [Self; 3] = [
        Self::RepositoryNotTrusted,
        Self::ObservationUnavailable,
        Self::ConfigurationMalformed,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::RepositoryNotTrusted => "trust.repository.not_trusted",
            Self::ObservationUnavailable => "trust.observation.unavailable",
            Self::ConfigurationMalformed => "trust.configuration.malformed",
        }
    }

    pub const fn from_status(status: ProjectTrustStatus) -> Option<Self> {
        match status {
            ProjectTrustStatus::Trusted => None,
            ProjectTrustStatus::Untrusted | ProjectTrustStatus::Missing => {
                Some(Self::RepositoryNotTrusted)
            }
            ProjectTrustStatus::Unknown | ProjectTrustStatus::Unreadable => {
                Some(Self::ObservationUnavailable)
            }
            ProjectTrustStatus::Malformed => Some(Self::ConfigurationMalformed),
        }
    }
}

/// Closed revision-observation diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionDiagnostic {
    IntegrationStale,
    ObservationMismatch,
}

/// Closed MCP verification-tool diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolVerificationDiagnostic {
    DesignationMismatch,
}

impl ToolVerificationDiagnostic {
    pub const ALL: [Self; 1] = [Self::DesignationMismatch];

    pub const fn code(self) -> &'static str {
        match self {
            Self::DesignationMismatch => "mcp.tool_verification.designation_mismatch",
        }
    }
}

impl RevisionDiagnostic {
    pub const ALL: [Self; 2] = [Self::IntegrationStale, Self::ObservationMismatch];

    pub const fn code(self) -> &'static str {
        match self {
            Self::IntegrationStale => "revision.integration.stale",
            Self::ObservationMismatch => "revision.observation.mismatch",
        }
    }
}

/// Union of typed CLI-owned operational diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalDiagnostic {
    Installation(InstallationDiagnostic),
    ManagedConfig(ManagedConfigDiagnostic),
    Guard(GuardDiagnostic),
    Trust(TrustDiagnostic),
    Revision(RevisionDiagnostic),
    ToolVerification(ToolVerificationDiagnostic),
}

impl OperationalDiagnostic {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Installation(diagnostic) => diagnostic.code(),
            Self::ManagedConfig(diagnostic) => diagnostic.code(),
            Self::Guard(diagnostic) => diagnostic.code(),
            Self::Trust(diagnostic) => diagnostic.code(),
            Self::Revision(diagnostic) => diagnostic.code(),
            Self::ToolVerification(diagnostic) => diagnostic.code(),
        }
    }

    pub const fn domain(self) -> &'static str {
        match self {
            Self::Installation(_) => "installation",
            Self::ManagedConfig(_) => "configuration",
            Self::Guard(_) => "guard",
            Self::Trust(_) => "trust",
            Self::Revision(_) => "revision",
            Self::ToolVerification(_) => "mcp",
        }
    }

    pub const fn stage(self) -> &'static str {
        match self {
            Self::Installation(_) => "installation_verification",
            Self::ManagedConfig(_) => "managed_configuration",
            Self::Guard(GuardDiagnostic::RequiredPhaseNotObserved)
            | Self::Guard(GuardDiagnostic::IncompatibleObservation)
            | Self::Guard(GuardDiagnostic::PromptCaptureUnsupported)
            | Self::Guard(GuardDiagnostic::PromptCaptureUnobserved) => "guard_observation",
            Self::Guard(_) => "guard_files",
            Self::Trust(_) => "repository_trust",
            Self::Revision(_) => "revision_observation",
            Self::ToolVerification(_) => "tool_round_trip",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::Installation(InstallationDiagnostic::ExecutableMissing) => {
                "The configured Volicord executable is missing"
            }
            Self::Installation(InstallationDiagnostic::ExecutableNotRunnable) => {
                "The configured Volicord executable is not runnable"
            }
            Self::Installation(InstallationDiagnostic::BuildIdentityUnavailable) => {
                "The installed Volicord build identity is unavailable"
            }
            Self::Installation(InstallationDiagnostic::ManagedConfigurationInconsistent) => {
                "The installed Volicord build and managed configuration are inconsistent"
            }
            Self::ManagedConfig(_) => "The managed host configuration is not canonical",
            Self::Guard(GuardDiagnostic::ManagedFileMissing) => {
                "A required Guard managed file is missing"
            }
            Self::Guard(GuardDiagnostic::ManagedFileIntegrityFailure) => {
                "A Guard managed file failed integrity validation"
            }
            Self::Guard(GuardDiagnostic::ManifestMismatch) => {
                "The Guard manifest does not match current authority"
            }
            Self::Guard(GuardDiagnostic::HookWrapperMissing) => {
                "A required Guard hook wrapper is missing"
            }
            Self::Guard(GuardDiagnostic::HookWrapperNotExecutable) => {
                "A required Guard hook wrapper is not executable"
            }
            Self::Guard(GuardDiagnostic::HookProcessFailure) => "A Guard hook process failed",
            Self::Guard(GuardDiagnostic::RequiredPhaseNotObserved) => {
                "A required Guard phase has not been observed"
            }
            Self::Guard(GuardDiagnostic::IncompatibleObservation) => {
                "A Guard observation is incompatible with the current contract"
            }
            Self::Guard(GuardDiagnostic::PromptCaptureUnsupported) => {
                "Prompt capture is unsupported by the current host boundary"
            }
            Self::Guard(GuardDiagnostic::PromptCaptureUnobserved) => {
                "Configured prompt capture has not been observed"
            }
            Self::Trust(TrustDiagnostic::RepositoryNotTrusted) => {
                "The Product Repository is not trusted by the host"
            }
            Self::Trust(TrustDiagnostic::ObservationUnavailable) => {
                "Repository trust could not be observed"
            }
            Self::Trust(TrustDiagnostic::ConfigurationMalformed) => {
                "Repository trust configuration is malformed"
            }
            Self::Revision(RevisionDiagnostic::IntegrationStale) => {
                "The host is using a stale integration revision"
            }
            Self::Revision(RevisionDiagnostic::ObservationMismatch) => {
                "The observed revision does not match the current revision"
            }
            Self::ToolVerification(ToolVerificationDiagnostic::DesignationMismatch) => {
                "The observed verification tool does not match the canonical role owner"
            }
        }
    }

    pub const fn severity(self) -> DiagnosticSeverity {
        match self {
            Self::Installation(InstallationDiagnostic::BuildIdentityUnavailable)
            | Self::Guard(GuardDiagnostic::RequiredPhaseNotObserved)
            | Self::Guard(GuardDiagnostic::PromptCaptureUnsupported)
            | Self::Guard(GuardDiagnostic::PromptCaptureUnobserved)
            | Self::Trust(TrustDiagnostic::RepositoryNotTrusted)
            | Self::Trust(TrustDiagnostic::ObservationUnavailable)
            | Self::Revision(_) => DiagnosticSeverity::Warning,
            Self::Installation(_)
            | Self::ManagedConfig(_)
            | Self::Guard(_)
            | Self::Trust(TrustDiagnostic::ConfigurationMalformed)
            | Self::ToolVerification(_) => DiagnosticSeverity::Error,
        }
    }

    /// Recommended actions are selected only from the closed diagnostic variant.
    pub const fn actions(self) -> &'static [OperationalRecommendedAction] {
        use OperationalRecommendedAction as Action;
        match self {
            Self::Installation(InstallationDiagnostic::ExecutableMissing)
            | Self::Installation(InstallationDiagnostic::ExecutableNotRunnable)
            | Self::Installation(InstallationDiagnostic::BuildIdentityUnavailable) => {
                &[Action::ReinstallCurrentBuild]
            }
            Self::Installation(InstallationDiagnostic::ManagedConfigurationInconsistent) => &[
                Action::ReinstallCurrentBuild,
                Action::RepairManagedConfiguration,
            ],
            Self::ManagedConfig(_) => &[Action::RepairManagedConfiguration],
            Self::Guard(GuardDiagnostic::RequiredPhaseNotObserved) => &[Action::TriggerGuardPhase],
            Self::Guard(GuardDiagnostic::PromptCaptureUnobserved) => {
                &[Action::TriggerPromptCapture]
            }
            Self::Guard(GuardDiagnostic::PromptCaptureUnsupported) => {
                &[Action::UseSupportedPromptCaptureHost]
            }
            Self::Guard(_) => &[Action::RepairGuard],
            Self::Trust(TrustDiagnostic::RepositoryNotTrusted) => &[Action::TrustRepository],
            Self::Trust(TrustDiagnostic::ObservationUnavailable)
            | Self::Trust(TrustDiagnostic::ConfigurationMalformed) => {
                &[Action::RepairTrustConfiguration]
            }
            Self::Revision(RevisionDiagnostic::IntegrationStale) => {
                &[Action::ReloadHostAfterConfigurationChange]
            }
            Self::Revision(RevisionDiagnostic::ObservationMismatch) => {
                &[Action::ReobserveCurrentRevision]
            }
            Self::ToolVerification(_) => &[Action::VerifyDesignatedTool],
        }
    }
}

/// Closed actions attached to CLI-owned findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalRecommendedAction {
    ReinstallCurrentBuild,
    RepairManagedConfiguration,
    ReloadHostAfterConfigurationChange,
    RepairGuard,
    TriggerGuardPhase,
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

/// Safe bounded facts for one CLI-owned operational finding.
#[derive(Debug, Default, Serialize)]
pub struct OperationalDiagnosticFacts {
    pub observed_state: Option<&'static str>,
    pub artifact_kind: Option<String>,
    pub guard_phase: Option<String>,
    pub expected_revision: Option<String>,
    pub observed_revision: Option<String>,
    pub expected_tool_name: Option<String>,
    pub observed_tool_name: Option<String>,
}

impl DiagnosticFactSource for OperationalDiagnosticFacts {}

#[derive(Serialize)]
struct ProjectedOperationalDiagnosticFacts<'a> {
    summary: &'static str,
    observed_state: Option<&'static str>,
    artifact_kind: &'a Option<String>,
    guard_phase: &'a Option<String>,
    expected_revision: &'a Option<String>,
    observed_revision: &'a Option<String>,
    expected_tool_name: &'a Option<String>,
    observed_tool_name: &'a Option<String>,
}

impl DiagnosticFactSource for ProjectedOperationalDiagnosticFacts<'_> {}

/// Creates a shared finding without reading any display summary or error text.
pub fn operational_diagnostic_finding(
    diagnostic: OperationalDiagnostic,
    finding_id: impl Into<String>,
    subject_kind: &'static str,
    subject_reference: impl Into<String>,
    facts: &OperationalDiagnosticFacts,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DiagnosticError> {
    let actions = diagnostic
        .actions()
        .iter()
        .map(|action| {
            DiagnosticAction::try_new(DiagnosticCode::parse(action.code())?, action.summary())
        })
        .collect::<Result<Vec<_>, _>>()?;
    DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(finding_id)?,
        DiagnosticCode::parse(diagnostic.code())?,
        DiagnosticDomain::parse(diagnostic.domain())?,
        DiagnosticStage::parse(diagnostic.stage())?,
        diagnostic.severity(),
        DiagnosticSource::parse("administrative_cli")?,
        DiagnosticSubject::try_new(subject_kind, subject_reference)?,
        DiagnosticFacts::project(&ProjectedOperationalDiagnosticFacts {
            summary: diagnostic.summary(),
            observed_state: facts.observed_state,
            artifact_kind: &facts.artifact_kind,
            guard_phase: &facts.guard_phase,
            expected_revision: &facts.expected_revision,
            observed_revision: &facts.observed_revision,
            expected_tool_name: &facts.expected_tool_name,
            observed_tool_name: &facts.observed_tool_name,
        })?,
        observed_at,
    )?
    .with_actions(actions)
}

/// Persists one connection-correlated operational finding.
pub(crate) fn persist_connection_operational_finding(
    runtime_home: &std::path::Path,
    connection: &volicord_store::agent_connections::AgentConnectionRecord,
    diagnostic: OperationalDiagnostic,
    facts: &OperationalDiagnosticFacts,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFindingId, ConnectionCommandError> {
    let finding_id = connection_operational_finding_id(connection, diagnostic)?;
    if diagnostic_finding(runtime_home, &finding_id)?.is_some() {
        return Ok(finding_id);
    }
    let revision = connection_integration_revision(connection)?;
    let finding = operational_diagnostic_finding(
        diagnostic,
        finding_id.to_string(),
        "agent_connection",
        &connection.connection_internal_id,
        facts,
        observed_at,
    )
    .and_then(|finding| {
        finding.with_connection_id(AgentConnectionId::new(
            connection.connection_internal_id.clone(),
        ))
    })
    .map(|finding| {
        finding.with_integration_revision(
            IntegrationRevision::parse(revision.as_str().to_owned())
                .expect("stored connection revision is valid"),
        )
    })
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    insert_diagnostic_finding(runtime_home, &finding)?;
    Ok(finding_id)
}

pub(crate) fn connection_operational_finding_id(
    connection: &volicord_store::agent_connections::AgentConnectionRecord,
    diagnostic: OperationalDiagnostic,
) -> Result<DiagnosticFindingId, ConnectionCommandError> {
    let id_suffix = diagnostic.code().replace('.', "_");
    DiagnosticFindingId::parse(format!(
        "finding.{}.{}",
        connection.connection_internal_id, id_suffix
    ))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

pub(crate) fn guard_artifact_kind(artifact: GuardManagedArtifact) -> String {
    match artifact {
        GuardManagedArtifact::HostHookWrapper(phase) => {
            format!("host_hook_wrapper:{}", phase.as_str())
        }
        artifact => artifact.kind().as_str().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::GuardHookPhase;

    use super::*;

    #[test]
    fn every_owner_diagnostic_variant_maps_to_a_namespaced_code() {
        for code in InstallationDiagnostic::ALL
            .into_iter()
            .map(InstallationDiagnostic::code)
            .chain(
                ManagedConfigDiagnostic::ALL
                    .into_iter()
                    .map(ManagedConfigDiagnostic::code),
            )
            .chain(GuardDiagnostic::ALL.into_iter().map(GuardDiagnostic::code))
            .chain(TrustDiagnostic::ALL.into_iter().map(TrustDiagnostic::code))
            .chain(
                RevisionDiagnostic::ALL
                    .into_iter()
                    .map(RevisionDiagnostic::code),
            )
            .chain(
                ToolVerificationDiagnostic::ALL
                    .into_iter()
                    .map(ToolVerificationDiagnostic::code),
            )
        {
            assert!(
                code.contains('.'),
                "diagnostic code must be namespaced: {code}"
            );
        }
    }

    #[test]
    fn every_guard_source_enum_variant_maps_without_prose() {
        let artifact = GuardManagedArtifact::VolicordPolicy;
        for issue in [
            GuardArtifactIssue::Missing,
            GuardArtifactIssue::Malformed,
            GuardArtifactIssue::ContentMismatch,
            GuardArtifactIssue::OwnershipMismatch,
            GuardArtifactIssue::PermissionMismatch,
            GuardArtifactIssue::HookContractMismatch,
        ] {
            let diagnostic = GuardDiagnostic::from_artifact_issue(artifact, issue);
            assert!(diagnostic.code().starts_with("guard."));
        }
        for issue in [
            GuardManifestIssue::Malformed,
            GuardManifestIssue::OwnershipMismatch,
        ] {
            assert_eq!(
                GuardDiagnostic::from_manifest_issue(issue),
                GuardDiagnostic::ManifestMismatch
            );
        }
        for status in [
            HookWrapperResolutionStatus::MetadataMissing,
            HookWrapperResolutionStatus::AuthorityMismatch,
            HookWrapperResolutionStatus::PolicyHashMismatch,
            HookWrapperResolutionStatus::HostOutputMismatch,
            HookWrapperResolutionStatus::Ok,
        ] {
            assert_eq!(
                GuardDiagnostic::from_hook_wrapper_status(status).is_none(),
                status == HookWrapperResolutionStatus::Ok
            );
        }

        let wrapper = GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool);
        assert_eq!(
            GuardDiagnostic::from_artifact_issue(wrapper, GuardArtifactIssue::Missing),
            GuardDiagnostic::HookWrapperMissing
        );
        assert_eq!(
            GuardDiagnostic::from_artifact_issue(wrapper, GuardArtifactIssue::PermissionMismatch),
            GuardDiagnostic::HookWrapperNotExecutable
        );
    }

    #[test]
    fn trust_status_mapping_is_exhaustive() {
        for status in [
            ProjectTrustStatus::Trusted,
            ProjectTrustStatus::Untrusted,
            ProjectTrustStatus::Missing,
            ProjectTrustStatus::Unknown,
            ProjectTrustStatus::Unreadable,
            ProjectTrustStatus::Malformed,
        ] {
            assert_eq!(
                TrustDiagnostic::from_status(status).is_none(),
                status == ProjectTrustStatus::Trusted
            );
        }
    }

    #[test]
    fn actions_are_selected_from_codes_and_never_from_summary_text() {
        let managed =
            OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::StaticEnvironmentDrift);
        assert_eq!(
            managed.actions(),
            &[OperationalRecommendedAction::RepairManagedConfiguration]
        );
        assert!(!managed.actions().iter().any(|action| matches!(
            action,
            OperationalRecommendedAction::ReloadHostAfterConfigurationChange
        )));

        let stale = OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale);
        assert_eq!(
            stale.actions(),
            &[OperationalRecommendedAction::ReloadHostAfterConfigurationChange]
        );
        assert_ne!(managed.summary(), stale.summary());
    }

    #[test]
    fn guard_integrity_missing_phase_and_stale_revision_scenarios_are_typed() {
        assert_eq!(
            GuardDiagnostic::from_artifact_issue(
                GuardManagedArtifact::VolicordPolicy,
                GuardArtifactIssue::ContentMismatch,
            ),
            GuardDiagnostic::ManagedFileIntegrityFailure
        );
        assert_eq!(
            OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved).code(),
            "guard.phase.required_not_observed"
        );
        assert_eq!(
            OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale).code(),
            "revision.integration.stale"
        );
    }

    #[test]
    fn verification_tool_mismatch_projects_bounded_expected_and_observed_names() {
        let finding = operational_diagnostic_finding(
            OperationalDiagnostic::ToolVerification(
                ToolVerificationDiagnostic::DesignationMismatch,
            ),
            "finding.runtime.verification_tool_designation_mismatch",
            "runtime_session",
            "runtime_fixture",
            &OperationalDiagnosticFacts {
                expected_tool_name: Some("volicord.list_projects".to_owned()),
                observed_tool_name: Some("volicord.status".to_owned()),
                ..OperationalDiagnosticFacts::default()
            },
            UtcTimestamp::parse("2026-07-22T00:00:00Z").expect("timestamp"),
        )
        .expect("typed diagnostic");

        assert_eq!(
            finding.code().as_str(),
            "mcp.tool_verification.designation_mismatch"
        );
        assert_eq!(
            finding.facts().data()["expected_tool_name"],
            "volicord.list_projects"
        );
        assert_eq!(
            finding.facts().data()["observed_tool_name"],
            "volicord.status"
        );
    }
}
