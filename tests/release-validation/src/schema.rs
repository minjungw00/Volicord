use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformEnvironment {
    Linux,
    Macos,
    NativeWindows,
    Wsl2,
}

impl PlatformEnvironment {
    pub const ALL: [Self; 4] = [Self::Linux, Self::Macos, Self::NativeWindows, Self::Wsl2];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::NativeWindows => "native_windows",
            Self::Wsl2 => "wsl2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexCapability {
    ManagedStdioMcp,
    RecordWorkflow,
    PersonalManagedBinding,
    SharedManagedBinding,
}

impl CodexCapability {
    pub const FIRST_RELEASE: [Self; 4] = [
        Self::ManagedStdioMcp,
        Self::PersonalManagedBinding,
        Self::RecordWorkflow,
        Self::SharedManagedBinding,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManagedStdioMcp => "managed_stdio_mcp",
            Self::RecordWorkflow => "record_workflow",
            Self::PersonalManagedBinding => "personal_managed_binding",
            Self::SharedManagedBinding => "shared_managed_binding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationProfile {
    Record,
}

impl IntegrationProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEvidenceStatus {
    Passed,
    Failed,
    Unavailable,
}

impl ValidationEvidenceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioStatus {
    Passed,
    Failed,
    Unavailable,
    NotRun,
}

impl ScenarioStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
            Self::NotRun => "not_run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerArchitecture {
    X86_64,
    Aarch64,
}

impl RunnerArchitecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexReleaseScenarioId {
    FreshInstall,
    RuntimeHomeCreation,
    PersonalManagedBinding,
    SharedManagedBinding,
    ReceiptCreateAndValidate,
    ConfigurationDriftDetection,
    RepairAfterDrift,
    SafeUninstall,
    SymlinkAndCanonicalPath,
    CodexRestart,
    ProjectMove,
    RecordWriteWorkflow,
    SuppressionUnavailable,
    UnsupportedHost,
    UnsupportedHostArtifact,
    WslShutdownRestart,
    Wsl2Ext4Project,
    Wsl2DrvfsRejection,
    Wsl2CrossTopologyRejection,
    Wsl1Rejection,
    Wsl2NativeWindowsReceiptReuseRejection,
}

impl CodexReleaseScenarioId {
    pub const BASE: [Self; 15] = [
        Self::FreshInstall,
        Self::RuntimeHomeCreation,
        Self::PersonalManagedBinding,
        Self::SharedManagedBinding,
        Self::ReceiptCreateAndValidate,
        Self::ConfigurationDriftDetection,
        Self::RepairAfterDrift,
        Self::SafeUninstall,
        Self::SymlinkAndCanonicalPath,
        Self::CodexRestart,
        Self::ProjectMove,
        Self::RecordWriteWorkflow,
        Self::SuppressionUnavailable,
        Self::UnsupportedHost,
        Self::UnsupportedHostArtifact,
    ];

    pub const WSL2_ADDITIONAL: [Self; 6] = [
        Self::WslShutdownRestart,
        Self::Wsl2Ext4Project,
        Self::Wsl2DrvfsRejection,
        Self::Wsl2CrossTopologyRejection,
        Self::Wsl1Rejection,
        Self::Wsl2NativeWindowsReceiptReuseRejection,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FreshInstall => "fresh_install",
            Self::RuntimeHomeCreation => "runtime_home_creation",
            Self::PersonalManagedBinding => "personal_managed_binding",
            Self::SharedManagedBinding => "shared_managed_binding",
            Self::ReceiptCreateAndValidate => "receipt_create_and_validate",
            Self::ConfigurationDriftDetection => "configuration_drift_detection",
            Self::RepairAfterDrift => "repair_after_drift",
            Self::SafeUninstall => "safe_uninstall",
            Self::SymlinkAndCanonicalPath => "symlink_and_canonical_path",
            Self::CodexRestart => "codex_restart",
            Self::ProjectMove => "project_move",
            Self::RecordWriteWorkflow => "record_write_workflow",
            Self::SuppressionUnavailable => "suppression_unavailable",
            Self::UnsupportedHost => "unsupported_host",
            Self::UnsupportedHostArtifact => "unsupported_host_artifact",
            Self::WslShutdownRestart => "wsl_shutdown_restart",
            Self::Wsl2Ext4Project => "wsl2_ext4_project",
            Self::Wsl2DrvfsRejection => "wsl2_drvfs_rejection",
            Self::Wsl2CrossTopologyRejection => "wsl2_cross_topology_rejection",
            Self::Wsl1Rejection => "wsl1_rejection",
            Self::Wsl2NativeWindowsReceiptReuseRejection => {
                "wsl2_native_windows_receipt_reuse_rejection"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequiredNullable<T>(pub Option<T>);

impl<T> RequiredNullable<T> {
    pub const fn null() -> Self {
        Self(None)
    }

    pub const fn some(value: T) -> Self {
        Self(Some(value))
    }

    pub const fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseRunnerCoordinate {
    pub runner_id: String,
    pub target_triple: String,
    pub architecture: RunnerArchitecture,
    pub os_release: String,
    pub environment_image: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseScenarioResult {
    pub scenario_id: CodexReleaseScenarioId,
    pub status: ScenarioStatus,
    pub reason: RequiredNullable<String>,
    pub evidence_digest: RequiredNullable<String>,
    pub observed_at: RequiredNullable<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseValidationEvidence {
    pub status: ValidationEvidenceStatus,
    pub artifact_digest: String,
    pub platform: PlatformEnvironment,
    pub observed_capabilities: Vec<CodexCapability>,
    pub integration_profile: IntegrationProfile,
    pub volicord_artifact_digest: String,
    pub runner: CodexReleaseRunnerCoordinate,
    pub scenario_results: Vec<CodexReleaseScenarioResult>,
    pub evidence_digest: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseCell {
    pub artifact_digest: String,
    pub platform: PlatformEnvironment,
    pub observed_capabilities: Vec<CodexCapability>,
    pub integration_profile: IntegrationProfile,
    pub validation_evidence: CodexReleaseValidationEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestOnlyCodexDescriptor {
    pub test_only: bool,
    pub fixture_id: String,
    pub artifact_digest: String,
    pub platform: PlatformEnvironment,
    pub observed_capabilities: Vec<CodexCapability>,
}
