//! Immutable definitions for CLI-owned operational diagnostics.

use volicord_types::diagnostics::DiagnosticSeverity;
use volicord_types::guard_manifest::GuardManagedArtifact;
use volicord_types::integration_verification::GuardVerificationRepairReason;

use crate::{
    guard_integration::audit::{
        GuardArtifactIssue, GuardManifestIssue, HookWrapperResolutionStatus,
    },
    host_integration::verification::{ManagedConfigDiagnostic, ProjectTrustStatus},
};

const ADMINISTRATIVE_CLI: &str = "administrative_cli";

/// One immutable operational-diagnostic definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationalDiagnosticDefinition {
    diagnostic: OperationalDiagnostic,
    code: &'static str,
    domain: &'static str,
    stage: &'static str,
    source: &'static str,
    summary: &'static str,
    severity: DiagnosticSeverity,
}

impl OperationalDiagnosticDefinition {
    const fn new(
        diagnostic: OperationalDiagnostic,
        code: &'static str,
        domain: &'static str,
        stage: &'static str,
        summary: &'static str,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self {
            diagnostic,
            code,
            domain,
            stage,
            source: ADMINISTRATIVE_CLI,
            summary,
            severity,
        }
    }

    pub const fn diagnostic(self) -> OperationalDiagnostic {
        self.diagnostic
    }

    pub const fn code(self) -> &'static str {
        self.code
    }

    pub const fn domain(self) -> &'static str {
        self.domain
    }

    pub const fn stage(self) -> &'static str {
        self.stage
    }

    pub const fn source(self) -> &'static str {
        self.source
    }

    pub const fn summary(self) -> &'static str {
        self.summary
    }

    pub const fn severity(self) -> DiagnosticSeverity {
        self.severity
    }
}

/// Closed installation diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstallationDiagnostic {
    ExecutableMissing,
    ExecutableNotRunnable,
    BuildIdentityUnavailable,
    BuildSourceNotReproducible,
    ManagedConfigurationInconsistent,
}

impl InstallationDiagnostic {
    pub const ALL: [Self; 5] = [
        Self::ExecutableMissing,
        Self::ExecutableNotRunnable,
        Self::BuildIdentityUnavailable,
        Self::BuildSourceNotReproducible,
        Self::ManagedConfigurationInconsistent,
    ];

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::ExecutableMissing => &INSTALLATION_EXECUTABLE_MISSING,
            Self::ExecutableNotRunnable => &INSTALLATION_EXECUTABLE_NOT_RUNNABLE,
            Self::BuildIdentityUnavailable => &INSTALLATION_BUILD_IDENTITY_UNAVAILABLE,
            Self::BuildSourceNotReproducible => &INSTALLATION_BUILD_SOURCE_NOT_REPRODUCIBLE,
            Self::ManagedConfigurationInconsistent => {
                &INSTALLATION_MANAGED_CONFIGURATION_INCONSISTENT
            }
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
    }
}

/// Closed Guard diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    ProbeHookEventNotObserved,
    ProbePayloadIncompatible,
    ProbeCallableMismatch,
    ProbeVerificationIdMismatch,
    ProbeSessionMismatch,
    ProbeTurnMismatch,
    ProbeToolUseMismatch,
    ProbeCurrentContractChanged,
}

impl GuardDiagnostic {
    pub const ALL: [Self; 18] = [
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
        Self::ProbeHookEventNotObserved,
        Self::ProbePayloadIncompatible,
        Self::ProbeCallableMismatch,
        Self::ProbeVerificationIdMismatch,
        Self::ProbeSessionMismatch,
        Self::ProbeTurnMismatch,
        Self::ProbeToolUseMismatch,
        Self::ProbeCurrentContractChanged,
    ];

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::ManagedFileMissing => &GUARD_MANAGED_FILE_MISSING,
            Self::ManagedFileIntegrityFailure => &GUARD_MANAGED_FILE_INTEGRITY_FAILURE,
            Self::ManifestMismatch => &GUARD_MANIFEST_MISMATCH,
            Self::HookWrapperMissing => &GUARD_HOOK_WRAPPER_MISSING,
            Self::HookWrapperNotExecutable => &GUARD_HOOK_WRAPPER_NOT_EXECUTABLE,
            Self::HookProcessFailure => &GUARD_HOOK_PROCESS_FAILURE,
            Self::RequiredPhaseNotObserved => &GUARD_REQUIRED_PHASE_NOT_OBSERVED,
            Self::IncompatibleObservation => &GUARD_INCOMPATIBLE_OBSERVATION,
            Self::PromptCaptureUnsupported => &GUARD_PROMPT_CAPTURE_UNSUPPORTED,
            Self::PromptCaptureUnobserved => &GUARD_PROMPT_CAPTURE_UNOBSERVED,
            Self::ProbeHookEventNotObserved => &GUARD_PROBE_HOOK_EVENT_NOT_OBSERVED,
            Self::ProbePayloadIncompatible => &GUARD_PROBE_PAYLOAD_INCOMPATIBLE,
            Self::ProbeCallableMismatch => &GUARD_PROBE_CALLABLE_MISMATCH,
            Self::ProbeVerificationIdMismatch => &GUARD_PROBE_VERIFICATION_ID_MISMATCH,
            Self::ProbeSessionMismatch => &GUARD_PROBE_SESSION_MISMATCH,
            Self::ProbeTurnMismatch => &GUARD_PROBE_TURN_MISMATCH,
            Self::ProbeToolUseMismatch => &GUARD_PROBE_TOOL_USE_MISMATCH,
            Self::ProbeCurrentContractChanged => &GUARD_PROBE_CURRENT_CONTRACT_CHANGED,
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
    }

    pub const fn from_verification_repair_reason(reason: GuardVerificationRepairReason) -> Self {
        match reason {
            GuardVerificationRepairReason::HookEventNotObserved
            | GuardVerificationRepairReason::ObservationDeadlineExceeded => {
                Self::ProbeHookEventNotObserved
            }
            GuardVerificationRepairReason::HookPayloadIncompatible => {
                Self::ProbePayloadIncompatible
            }
            GuardVerificationRepairReason::CallableIdentityMismatch => Self::ProbeCallableMismatch,
            GuardVerificationRepairReason::VerificationIdMismatch => {
                Self::ProbeVerificationIdMismatch
            }
            GuardVerificationRepairReason::SessionMismatch => Self::ProbeSessionMismatch,
            GuardVerificationRepairReason::TurnMismatch => Self::ProbeTurnMismatch,
            GuardVerificationRepairReason::ToolUseMismatch => Self::ProbeToolUseMismatch,
            GuardVerificationRepairReason::IntegrationRevisionChanged
            | GuardVerificationRepairReason::HookDefinitionChanged
            | GuardVerificationRepairReason::PolicyChanged => Self::ProbeCurrentContractChanged,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::RepositoryNotTrusted => &TRUST_REPOSITORY_NOT_TRUSTED,
            Self::ObservationUnavailable => &TRUST_OBSERVATION_UNAVAILABLE,
            Self::ConfigurationMalformed => &TRUST_CONFIGURATION_MALFORMED,
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RevisionDiagnostic {
    IntegrationStale,
    ObservationMismatch,
}

impl RevisionDiagnostic {
    pub const ALL: [Self; 2] = [Self::IntegrationStale, Self::ObservationMismatch];

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::IntegrationStale => &REVISION_INTEGRATION_STALE,
            Self::ObservationMismatch => &REVISION_OBSERVATION_MISMATCH,
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
    }
}

/// Closed MCP verification-tool diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolVerificationDiagnostic {
    DesignationMismatch,
}

impl ToolVerificationDiagnostic {
    pub const ALL: [Self; 1] = [Self::DesignationMismatch];

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::DesignationMismatch => &TOOL_VERIFICATION_DESIGNATION_MISMATCH,
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
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
    pub const ALL: [Self; 39] = [
        Self::Installation(InstallationDiagnostic::ExecutableMissing),
        Self::Installation(InstallationDiagnostic::ExecutableNotRunnable),
        Self::Installation(InstallationDiagnostic::BuildIdentityUnavailable),
        Self::Installation(InstallationDiagnostic::BuildSourceNotReproducible),
        Self::Installation(InstallationDiagnostic::ManagedConfigurationInconsistent),
        Self::ManagedConfig(ManagedConfigDiagnostic::TomlParseFailure),
        Self::ManagedConfig(ManagedConfigDiagnostic::EntryMissing),
        Self::ManagedConfig(ManagedConfigDiagnostic::EntryDisabled),
        Self::ManagedConfig(ManagedConfigDiagnostic::CommandDrift),
        Self::ManagedConfig(ManagedConfigDiagnostic::ArgumentDrift),
        Self::ManagedConfig(ManagedConfigDiagnostic::StaticEnvironmentDrift),
        Self::ManagedConfig(ManagedConfigDiagnostic::ForwardedEnvironmentDrift),
        Self::ManagedConfig(ManagedConfigDiagnostic::FingerprintMismatch),
        Self::ManagedConfig(ManagedConfigDiagnostic::MalformedApprovalOverlay),
        Self::ManagedConfig(ManagedConfigDiagnostic::Unavailable),
        Self::Guard(GuardDiagnostic::ManagedFileMissing),
        Self::Guard(GuardDiagnostic::ManagedFileIntegrityFailure),
        Self::Guard(GuardDiagnostic::ManifestMismatch),
        Self::Guard(GuardDiagnostic::HookWrapperMissing),
        Self::Guard(GuardDiagnostic::HookWrapperNotExecutable),
        Self::Guard(GuardDiagnostic::HookProcessFailure),
        Self::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
        Self::Guard(GuardDiagnostic::IncompatibleObservation),
        Self::Guard(GuardDiagnostic::PromptCaptureUnsupported),
        Self::Guard(GuardDiagnostic::PromptCaptureUnobserved),
        Self::Guard(GuardDiagnostic::ProbeHookEventNotObserved),
        Self::Guard(GuardDiagnostic::ProbePayloadIncompatible),
        Self::Guard(GuardDiagnostic::ProbeCallableMismatch),
        Self::Guard(GuardDiagnostic::ProbeVerificationIdMismatch),
        Self::Guard(GuardDiagnostic::ProbeSessionMismatch),
        Self::Guard(GuardDiagnostic::ProbeTurnMismatch),
        Self::Guard(GuardDiagnostic::ProbeToolUseMismatch),
        Self::Guard(GuardDiagnostic::ProbeCurrentContractChanged),
        Self::Trust(TrustDiagnostic::RepositoryNotTrusted),
        Self::Trust(TrustDiagnostic::ObservationUnavailable),
        Self::Trust(TrustDiagnostic::ConfigurationMalformed),
        Self::Revision(RevisionDiagnostic::IntegrationStale),
        Self::Revision(RevisionDiagnostic::ObservationMismatch),
        Self::ToolVerification(ToolVerificationDiagnostic::DesignationMismatch),
    ];

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::Installation(value) => value.definition(),
            Self::ManagedConfig(value) => value.definition(),
            Self::Guard(value) => value.definition(),
            Self::Trust(value) => value.definition(),
            Self::Revision(value) => value.definition(),
            Self::ToolVerification(value) => value.definition(),
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
    }
}

impl ManagedConfigDiagnostic {
    pub const ALL: [Self; 10] = [
        Self::TomlParseFailure,
        Self::EntryMissing,
        Self::EntryDisabled,
        Self::CommandDrift,
        Self::ArgumentDrift,
        Self::StaticEnvironmentDrift,
        Self::ForwardedEnvironmentDrift,
        Self::FingerprintMismatch,
        Self::MalformedApprovalOverlay,
        Self::Unavailable,
    ];

    pub const fn definition(self) -> &'static OperationalDiagnosticDefinition {
        match self {
            Self::TomlParseFailure => &MANAGED_CONFIG_TOML_PARSE_FAILURE,
            Self::EntryMissing => &MANAGED_CONFIG_ENTRY_MISSING,
            Self::EntryDisabled => &MANAGED_CONFIG_ENTRY_DISABLED,
            Self::CommandDrift => &MANAGED_CONFIG_COMMAND_DRIFT,
            Self::ArgumentDrift => &MANAGED_CONFIG_ARGUMENT_DRIFT,
            Self::StaticEnvironmentDrift => &MANAGED_CONFIG_STATIC_ENVIRONMENT_DRIFT,
            Self::ForwardedEnvironmentDrift => &MANAGED_CONFIG_FORWARDED_ENVIRONMENT_DRIFT,
            Self::FingerprintMismatch => &MANAGED_CONFIG_FINGERPRINT_MISMATCH,
            Self::MalformedApprovalOverlay => &MANAGED_CONFIG_APPROVAL_OVERLAY_MALFORMED,
            Self::Unavailable => &MANAGED_CONFIG_OBSERVATION_UNAVAILABLE,
        }
    }

    pub const fn code(self) -> &'static str {
        self.definition().code()
    }
}

macro_rules! definition {
    ($name:ident, $diagnostic:expr, $code:literal, $domain:literal, $stage:literal, $summary:literal, $severity:expr) => {
        const $name: OperationalDiagnosticDefinition = OperationalDiagnosticDefinition::new(
            $diagnostic,
            $code,
            $domain,
            $stage,
            $summary,
            $severity,
        );
    };
}

definition!(
    INSTALLATION_EXECUTABLE_MISSING,
    OperationalDiagnostic::Installation(InstallationDiagnostic::ExecutableMissing),
    "installation.executable.missing",
    "installation",
    "installation_verification",
    "The configured Volicord executable is missing",
    DiagnosticSeverity::Error
);
definition!(
    INSTALLATION_EXECUTABLE_NOT_RUNNABLE,
    OperationalDiagnostic::Installation(InstallationDiagnostic::ExecutableNotRunnable),
    "installation.executable.not_runnable",
    "installation",
    "installation_verification",
    "The configured Volicord executable is not runnable",
    DiagnosticSeverity::Error
);
definition!(
    INSTALLATION_BUILD_IDENTITY_UNAVAILABLE,
    OperationalDiagnostic::Installation(InstallationDiagnostic::BuildIdentityUnavailable),
    "installation.build_identity.unavailable",
    "installation",
    "installation_verification",
    "The installed Volicord build identity is unavailable",
    DiagnosticSeverity::Warning
);
definition!(
    INSTALLATION_BUILD_SOURCE_NOT_REPRODUCIBLE,
    OperationalDiagnostic::Installation(InstallationDiagnostic::BuildSourceNotReproducible),
    "installation.build_source.not_reproducible",
    "installation",
    "installation_verification",
    "The installed Volicord build source is not reproducible from its recorded commit",
    DiagnosticSeverity::Warning
);
definition!(
    INSTALLATION_MANAGED_CONFIGURATION_INCONSISTENT,
    OperationalDiagnostic::Installation(InstallationDiagnostic::ManagedConfigurationInconsistent),
    "installation.managed_config.inconsistent",
    "installation",
    "installation_verification",
    "The installed Volicord build and managed configuration are inconsistent",
    DiagnosticSeverity::Error
);

definition!(
    MANAGED_CONFIG_TOML_PARSE_FAILURE,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::TomlParseFailure),
    "managed_config.toml.parse_failed",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_ENTRY_MISSING,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::EntryMissing),
    "managed_config.entry.missing",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_ENTRY_DISABLED,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::EntryDisabled),
    "managed_config.entry.disabled",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_COMMAND_DRIFT,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::CommandDrift),
    "managed_config.command.drift",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_ARGUMENT_DRIFT,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::ArgumentDrift),
    "managed_config.arguments.drift",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_STATIC_ENVIRONMENT_DRIFT,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::StaticEnvironmentDrift),
    "managed_config.static_environment.drift",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_FORWARDED_ENVIRONMENT_DRIFT,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::ForwardedEnvironmentDrift),
    "managed_config.forwarded_environment.drift",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_FINGERPRINT_MISMATCH,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::FingerprintMismatch),
    "managed_config.fingerprint.mismatch",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_APPROVAL_OVERLAY_MALFORMED,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::MalformedApprovalOverlay),
    "managed_config.approval_overlay.malformed",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);
definition!(
    MANAGED_CONFIG_OBSERVATION_UNAVAILABLE,
    OperationalDiagnostic::ManagedConfig(ManagedConfigDiagnostic::Unavailable),
    "managed_config.observation.unavailable",
    "configuration",
    "managed_configuration",
    "The managed host configuration is not canonical",
    DiagnosticSeverity::Error
);

definition!(
    GUARD_MANAGED_FILE_MISSING,
    OperationalDiagnostic::Guard(GuardDiagnostic::ManagedFileMissing),
    "guard.managed_file.missing",
    "guard",
    "guard_files",
    "A required Guard managed file is missing",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_MANAGED_FILE_INTEGRITY_FAILURE,
    OperationalDiagnostic::Guard(GuardDiagnostic::ManagedFileIntegrityFailure),
    "guard.managed_file.integrity_failed",
    "guard",
    "guard_files",
    "A Guard managed file failed integrity validation",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_MANIFEST_MISMATCH,
    OperationalDiagnostic::Guard(GuardDiagnostic::ManifestMismatch),
    "guard.manifest.mismatch",
    "guard",
    "guard_files",
    "The Guard manifest does not match current authority",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_HOOK_WRAPPER_MISSING,
    OperationalDiagnostic::Guard(GuardDiagnostic::HookWrapperMissing),
    "guard.hook_wrapper.missing",
    "guard",
    "guard_files",
    "A required Guard hook wrapper is missing",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_HOOK_WRAPPER_NOT_EXECUTABLE,
    OperationalDiagnostic::Guard(GuardDiagnostic::HookWrapperNotExecutable),
    "guard.hook_wrapper.not_executable",
    "guard",
    "guard_files",
    "A required Guard hook wrapper is not executable",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_HOOK_PROCESS_FAILURE,
    OperationalDiagnostic::Guard(GuardDiagnostic::HookProcessFailure),
    "guard.hook_process.failed",
    "guard",
    "guard_files",
    "A Guard hook process failed",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_REQUIRED_PHASE_NOT_OBSERVED,
    OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
    "guard.phase.required_not_observed",
    "guard",
    "guard_observation",
    "A required Guard phase has not been observed",
    DiagnosticSeverity::Warning
);
definition!(
    GUARD_INCOMPATIBLE_OBSERVATION,
    OperationalDiagnostic::Guard(GuardDiagnostic::IncompatibleObservation),
    "guard.observation.incompatible",
    "guard",
    "guard_observation",
    "A Guard observation is incompatible with the current contract",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROMPT_CAPTURE_UNSUPPORTED,
    OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnsupported),
    "guard.prompt_capture.unsupported",
    "guard",
    "guard_observation",
    "Prompt capture is unsupported by the current host boundary",
    DiagnosticSeverity::Warning
);
definition!(
    GUARD_PROMPT_CAPTURE_UNOBSERVED,
    OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnobserved),
    "guard.prompt_capture.unobserved",
    "guard",
    "guard_observation",
    "Configured prompt capture has not been observed",
    DiagnosticSeverity::Warning
);
definition!(
    GUARD_PROBE_HOOK_EVENT_NOT_OBSERVED,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeHookEventNotObserved),
    "guard.probe.hook_event_not_observed",
    "guard",
    "guard_probe",
    "The expected Guard probe hook event was not observed",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_PAYLOAD_INCOMPATIBLE,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbePayloadIncompatible),
    "guard.probe.payload_incompatible",
    "guard",
    "guard_probe",
    "The Guard probe hook payload is incompatible with the current contract",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_CALLABLE_MISMATCH,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeCallableMismatch),
    "guard.probe.callable_mismatch",
    "guard",
    "guard_probe",
    "The observed Guard probe callable does not match the expected identity",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_VERIFICATION_ID_MISMATCH,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeVerificationIdMismatch),
    "guard.probe.verification_id_mismatch",
    "guard",
    "guard_probe",
    "The Guard probe event has a different verification ID",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_SESSION_MISMATCH,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeSessionMismatch),
    "guard.probe.session_mismatch",
    "guard",
    "guard_probe",
    "The Guard probe event does not match the managed runtime session",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_TURN_MISMATCH,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeTurnMismatch),
    "guard.probe.turn_mismatch",
    "guard",
    "guard_probe",
    "The Guard probe event does not match the verification turn",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_TOOL_USE_MISMATCH,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeToolUseMismatch),
    "guard.probe.tool_use_mismatch",
    "guard",
    "guard_probe",
    "The Guard probe pre-tool and post-tool events have different tool-use identities",
    DiagnosticSeverity::Error
);
definition!(
    GUARD_PROBE_CURRENT_CONTRACT_CHANGED,
    OperationalDiagnostic::Guard(GuardDiagnostic::ProbeCurrentContractChanged),
    "guard.probe.current_contract_changed",
    "guard",
    "guard_probe",
    "The current integration, policy, or hook contract changed during verification",
    DiagnosticSeverity::Error
);

definition!(
    TRUST_REPOSITORY_NOT_TRUSTED,
    OperationalDiagnostic::Trust(TrustDiagnostic::RepositoryNotTrusted),
    "trust.repository.not_trusted",
    "trust",
    "repository_trust",
    "The Product Repository is not trusted by the host",
    DiagnosticSeverity::Warning
);
definition!(
    TRUST_OBSERVATION_UNAVAILABLE,
    OperationalDiagnostic::Trust(TrustDiagnostic::ObservationUnavailable),
    "trust.observation.unavailable",
    "trust",
    "repository_trust",
    "Repository trust could not be observed",
    DiagnosticSeverity::Warning
);
definition!(
    TRUST_CONFIGURATION_MALFORMED,
    OperationalDiagnostic::Trust(TrustDiagnostic::ConfigurationMalformed),
    "trust.configuration.malformed",
    "trust",
    "repository_trust",
    "Repository trust configuration is malformed",
    DiagnosticSeverity::Error
);

definition!(
    REVISION_INTEGRATION_STALE,
    OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale),
    "revision.integration.stale",
    "revision",
    "revision_observation",
    "The host is using a stale integration revision",
    DiagnosticSeverity::Warning
);
definition!(
    REVISION_OBSERVATION_MISMATCH,
    OperationalDiagnostic::Revision(RevisionDiagnostic::ObservationMismatch),
    "revision.observation.mismatch",
    "revision",
    "revision_observation",
    "The observed revision does not match the current revision",
    DiagnosticSeverity::Warning
);
definition!(
    TOOL_VERIFICATION_DESIGNATION_MISMATCH,
    OperationalDiagnostic::ToolVerification(ToolVerificationDiagnostic::DesignationMismatch),
    "mcp.tool_verification.designation_mismatch",
    "mcp",
    "tool_round_trip",
    "The observed verification tool does not match the canonical role owner",
    DiagnosticSeverity::Error
);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_owner_variant_has_exactly_one_complete_definition() {
        let expected = InstallationDiagnostic::ALL
            .into_iter()
            .map(OperationalDiagnostic::Installation)
            .chain(
                ManagedConfigDiagnostic::ALL
                    .into_iter()
                    .map(OperationalDiagnostic::ManagedConfig),
            )
            .chain(
                GuardDiagnostic::ALL
                    .into_iter()
                    .map(OperationalDiagnostic::Guard),
            )
            .chain(
                TrustDiagnostic::ALL
                    .into_iter()
                    .map(OperationalDiagnostic::Trust),
            )
            .chain(
                RevisionDiagnostic::ALL
                    .into_iter()
                    .map(OperationalDiagnostic::Revision),
            )
            .chain(
                ToolVerificationDiagnostic::ALL
                    .into_iter()
                    .map(OperationalDiagnostic::ToolVerification),
            )
            .collect::<Vec<_>>();
        assert_eq!(OperationalDiagnostic::ALL.as_slice(), expected.as_slice());

        let mut codes = BTreeSet::new();
        for diagnostic in OperationalDiagnostic::ALL {
            let definition = diagnostic.definition();
            assert_eq!(definition.diagnostic(), diagnostic);
            assert!(definition.code().contains('.'));
            assert!(!definition.domain().is_empty());
            assert!(!definition.stage().is_empty());
            assert_eq!(definition.source(), ADMINISTRATIVE_CLI);
            assert!(codes.insert(definition.code()), "duplicate definition code");
        }
    }

    #[test]
    fn every_guard_repair_reason_maps_directly_to_one_stable_finding_code() {
        let expected = [
            "guard.probe.hook_event_not_observed",
            "guard.probe.payload_incompatible",
            "guard.probe.callable_mismatch",
            "guard.probe.verification_id_mismatch",
            "guard.probe.session_mismatch",
            "guard.probe.turn_mismatch",
            "guard.probe.tool_use_mismatch",
            "guard.probe.current_contract_changed",
            "guard.probe.current_contract_changed",
            "guard.probe.current_contract_changed",
            "guard.probe.hook_event_not_observed",
        ];
        assert_eq!(GuardVerificationRepairReason::ALL.len(), expected.len());
        for (reason, expected_code) in GuardVerificationRepairReason::ALL.into_iter().zip(expected)
        {
            assert_eq!(
                GuardDiagnostic::from_verification_repair_reason(reason).code(),
                expected_code,
                "{reason:?}"
            );
        }
    }
}
