use serde::Serialize;
use volicord_types::FailureCategory;

use super::{codex::ManagedHostEvidence, UserAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Complete,
    ActionRequired,
    Missing,
    Changed,
    Rejected,
    Unavailable,
    Unknown,
    Failed,
    UnsupportedContract,
    NotVerified,
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Missing => "missing",
            Self::Changed => "changed",
            Self::Rejected => "rejected",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
            Self::Failed => "failed",
            Self::UnsupportedContract => "unsupported_contract",
            Self::NotVerified => "not_verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostVerificationState {
    ConfiguredReady,
    ConfiguredActionRequired,
    Missing,
    Changed,
    Rejected,
    Unavailable,
    Unknown,
    Failed,
    UnsupportedContract,
    NotVerified,
}

impl HostVerificationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfiguredReady => "configured_ready",
            Self::ConfiguredActionRequired => "configured_action_required",
            Self::Missing => "missing",
            Self::Changed => "changed",
            Self::Rejected => "rejected",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
            Self::Failed => "failed",
            Self::UnsupportedContract => "unsupported_contract",
            Self::NotVerified => "not_verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedConfigStatus {
    Match,
    Unmanaged,
    Missing,
    Changed,
    Malformed,
    NotApplicable,
    Unknown,
}

impl ManagedConfigStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Unmanaged => "unmanaged",
            Self::Missing => "missing",
            Self::Changed => "changed",
            Self::Malformed => "malformed",
            Self::NotApplicable => "not_applicable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostConfigState {
    Missing,
    Match,
    MatchWithHostPolicyOverlay,
    Changed,
    Unmanaged,
    Malformed,
    Unknown,
    NotApplicable,
}

impl HostConfigState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Match => "match",
            Self::MatchWithHostPolicyOverlay => "match_with_host_policy_overlay",
            Self::Changed => "changed",
            Self::Unmanaged => "unmanaged",
            Self::Malformed => "malformed",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }

    pub fn from_managed_config(
        managed_config: ManagedConfigStatus,
        policy_overlay: HostPolicyOverlayState,
    ) -> Self {
        match managed_config {
            ManagedConfigStatus::Match
                if matches!(
                    policy_overlay,
                    HostPolicyOverlayState::Accepted | HostPolicyOverlayState::Warning
                ) =>
            {
                Self::MatchWithHostPolicyOverlay
            }
            ManagedConfigStatus::Match => Self::Match,
            ManagedConfigStatus::Unmanaged => Self::Unmanaged,
            ManagedConfigStatus::Missing => Self::Missing,
            ManagedConfigStatus::Changed => Self::Changed,
            ManagedConfigStatus::Malformed => Self::Malformed,
            ManagedConfigStatus::NotApplicable => Self::NotApplicable,
            ManagedConfigStatus::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostExecutableStatus {
    Available,
    Unavailable,
    NotRequired,
    NotChecked,
}

impl HostExecutableStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::NotRequired => "not_required",
            Self::NotChecked => "not_checked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostGateStatus {
    Ready,
    ActionRequired,
    Rejected,
    Missing,
    Unknown,
    NotApplicable,
}

impl HostGateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::ActionRequired => "action_required",
            Self::Rejected => "rejected",
            Self::Missing => "missing",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostApprovalState {
    Trusted,
    Approved,
    Pending,
    Rejected,
    Unknown,
    NotApplicable,
}

impl HostApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Approved => "approved",
            Self::Pending => "pending",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }

    pub fn from_host_gate(host_gate: HostGateStatus) -> Self {
        match host_gate {
            HostGateStatus::Ready => Self::Approved,
            HostGateStatus::ActionRequired => Self::Pending,
            HostGateStatus::Rejected => Self::Rejected,
            HostGateStatus::Missing | HostGateStatus::Unknown => Self::Unknown,
            HostGateStatus::NotApplicable => Self::NotApplicable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostConfigurationStatus {
    Discovered,
    Missing,
    Changed,
    Malformed,
    Unknown,
    NotApplicable,
}

impl HostConfigurationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Missing => "missing",
            Self::Changed => "changed",
            Self::Malformed => "malformed",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verification {
    pub status: VerificationStatus,
    pub host_state: HostVerificationState,
    pub host_version: Option<String>,
    pub managed_config: ManagedConfigStatus,
    pub host_executable: HostExecutableStatus,
    pub host_gate: HostGateStatus,
    pub host_configuration: HostConfigurationStatus,
    pub host_policy_overlay: Option<HostPolicyOverlayDiagnostic>,
    pub project_trust: Option<ProjectTrustDiagnostic>,
    pub host_runtime: Option<HostRuntimeDiagnostic>,
    pub host_mcp_command: Option<HostMcpCommandDiagnostic>,
    pub mcp_handshake_allowed: bool,
    pub details: String,
    pub diagnostic: Option<String>,
    pub failure_category: Option<FailureCategory>,
    pub failure_reason: Option<String>,
    pub(crate) managed_host_evidence: Option<ManagedHostEvidence>,
    pub user_actions: Vec<UserAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPolicyOverlayState {
    Absent,
    Accepted,
    Warning,
    Conflict,
}

impl HostPolicyOverlayState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Accepted => "accepted",
            Self::Warning => "warning",
            Self::Conflict => "conflict",
        }
    }

    pub fn from_diagnostic(overlay: Option<&HostPolicyOverlayDiagnostic>) -> Self {
        match overlay {
            Some(overlay) if overlay.accepted => Self::Accepted,
            Some(_) => Self::Conflict,
            None => Self::Absent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostPolicyOverlayDiagnostic {
    pub present: bool,
    pub accepted: bool,
    pub kind: String,
    pub tool_count: usize,
    pub tools: Vec<String>,
    pub entries: Vec<HostPolicyOverlayEntryDiagnostic>,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostPolicyOverlayEntryDiagnostic {
    pub tool: String,
    pub approval_mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTrustStatus {
    Trusted,
    Untrusted,
    Missing,
    Unknown,
    Unreadable,
    Malformed,
}

impl ProjectTrustStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Untrusted => "untrusted",
            Self::Missing => "missing",
            Self::Unknown => "unknown",
            Self::Unreadable => "unreadable",
            Self::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectTrustDiagnostic {
    pub status: ProjectTrustStatus,
    pub config_path: String,
    pub repo_root: String,
    pub details: String,
}

impl HostApprovalState {
    pub fn from_project_trust(project_trust: Option<&ProjectTrustDiagnostic>) -> Self {
        let Some(project_trust) = project_trust else {
            return Self::NotApplicable;
        };
        match project_trust.status {
            ProjectTrustStatus::Trusted => Self::Trusted,
            ProjectTrustStatus::Untrusted => Self::Pending,
            ProjectTrustStatus::Missing | ProjectTrustStatus::Unknown => Self::Unknown,
            ProjectTrustStatus::Unreadable | ProjectTrustStatus::Malformed => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRuntimeObservationStatus {
    Observed,
    NotObserved,
    Unknown,
}

impl HostRuntimeObservationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::NotObserved => "not_observed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostRuntimeDiagnostic {
    pub status: HostRuntimeObservationStatus,
    pub managed_host_startup: HostRuntimeObservationStatus,
    pub managed_host_tools_list: HostRuntimeObservationStatus,
    pub managed_host_tool_call: HostRuntimeObservationStatus,
    pub active_tool_exposure: ActiveToolExposureStatus,
    pub managed_host_storage: Option<ManagedHostStorageDiagnostic>,
    pub details: String,
    pub last_observed_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ManagedHostLifecycle {
    pub managed_host_startup: HostRuntimeObservationStatus,
    pub managed_host_tools_list: HostRuntimeObservationStatus,
    pub managed_host_tool_call: HostRuntimeObservationStatus,
}

impl ManagedHostLifecycle {
    pub fn from_runtime(runtime: &HostRuntimeDiagnostic) -> Self {
        Self {
            managed_host_startup: runtime.managed_host_startup,
            managed_host_tools_list: runtime.managed_host_tools_list,
            managed_host_tool_call: runtime.managed_host_tool_call,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveToolExposureStatus {
    Confirmed,
    Unconfirmed,
    Unknown,
}

impl ActiveToolExposureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Unconfirmed => "unconfirmed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedHostStorageDiagnostic {
    pub storage_read: String,
    pub storage_write: String,
    pub effective_tool_mode: String,
    pub source_lifecycle_event: String,
    pub observed_at: Option<String>,
}

impl ManagedHostStorageDiagnostic {
    pub fn storage_capability(&self) -> StorageCapability {
        StorageCapability::from_read_write_status(
            &self.storage_read,
            &self.storage_write,
            &self.effective_tool_mode,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCapability {
    ReadWrite,
    ReadOnly,
    Unavailable,
    Unknown,
}

impl StorageCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadWrite => "read_write",
            Self::ReadOnly => "read_only",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_mcp_storage_capability(value: &str) -> Self {
        match value {
            "read_write" => Self::ReadWrite,
            "read_only" => Self::ReadOnly,
            "unavailable" => Self::Unavailable,
            "unknown" => Self::Unknown,
            _ => Self::Unknown,
        }
    }

    pub fn from_read_write_status(
        storage_read: &str,
        storage_write: &str,
        effective_tool_mode: &str,
    ) -> Self {
        match (storage_read, storage_write, effective_tool_mode) {
            ("passed", "passed", _) => Self::ReadWrite,
            ("passed", "readonly", _) => Self::ReadOnly,
            (_, _, "unavailable") | ("failed", _, _) | (_, "skipped", _) => Self::Unavailable,
            _ => Self::Unknown,
        }
    }

    pub fn storage_read_status(self) -> &'static str {
        match self {
            Self::ReadWrite | Self::ReadOnly => "passed",
            Self::Unavailable => "failed",
            Self::Unknown => "unknown",
        }
    }

    pub fn storage_write_status(self) -> &'static str {
        match self {
            Self::ReadWrite => "passed",
            Self::ReadOnly => "readonly",
            Self::Unavailable => "skipped",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CliMcpStepStatus {
    Passed,
    Failed,
    Skipped,
    Unknown,
}

impl CliMcpStepStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CliMcpVerification {
    pub preflight: CliMcpStepStatus,
    pub handshake: CliMcpStepStatus,
    pub tools_list: CliMcpStepStatus,
    pub storage_capability: StorageCapability,
    pub effective_tool_mode: Option<String>,
}

impl CliMcpVerification {
    pub fn new(
        preflight: CliMcpStepStatus,
        handshake: CliMcpStepStatus,
        tools_list: CliMcpStepStatus,
        storage_capability: StorageCapability,
        effective_tool_mode: Option<String>,
    ) -> Self {
        Self {
            preflight,
            handshake,
            tools_list,
            storage_capability,
            effective_tool_mode,
        }
    }

    pub fn has_failed_step(&self) -> bool {
        matches!(self.preflight, CliMcpStepStatus::Failed)
            || matches!(self.handshake, CliMcpStepStatus::Failed)
            || matches!(self.tools_list, CliMcpStepStatus::Failed)
    }

    pub fn handshake_passed(&self) -> bool {
        matches!(self.handshake, CliMcpStepStatus::Passed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationGuaranteeDisclosure {
    CooperativeHostObservation,
}

impl VerificationGuaranteeDisclosure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CooperativeHostObservation => "cooperative_host_observation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostVerificationContract {
    pub host_config: HostConfigState,
    pub managed_identity: ManagedConfigStatus,
    pub host_policy_overlay: HostPolicyOverlayState,
    pub host_approval: HostApprovalState,
    pub managed_lifecycle: Option<ManagedHostLifecycle>,
    pub active_tool_exposure: ActiveToolExposureStatus,
    pub storage_capability: StorageCapability,
    pub guarantee_disclosure: VerificationGuaranteeDisclosure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostMcpCommandLaunchMode {
    AbsolutePath,
    PathResolved,
    RemoteExecutor,
    Unknown,
    Malformed,
}

impl HostMcpCommandLaunchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AbsolutePath => "absolute_path",
            Self::PathResolved => "path_resolved",
            Self::RemoteExecutor => "remote_executor",
            Self::Unknown => "unknown",
            Self::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostMcpCommandDiagnostic {
    pub mode: HostMcpCommandLaunchMode,
    pub command: Option<String>,
    pub risk: Option<String>,
    pub details: String,
}

impl Verification {
    pub fn new(status: VerificationStatus, details: impl Into<String>) -> Self {
        Self {
            status,
            host_state: host_state_from_status(status),
            host_version: None,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn configured_ready(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Complete,
            host_state: HostVerificationState::ConfiguredReady,
            host_version: None,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::NotRequired,
            host_gate: HostGateStatus::Ready,
            host_configuration: HostConfigurationStatus::Discovered,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: true,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn action_required(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::ActionRequired,
            host_state: HostVerificationState::ConfiguredActionRequired,
            host_version: None,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::ActionRequired,
            host_configuration: HostConfigurationStatus::Discovered,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: true,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn missing(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Missing,
            host_state: HostVerificationState::Missing,
            host_version: None,
            managed_config: ManagedConfigStatus::Missing,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Missing,
            host_configuration: HostConfigurationStatus::Missing,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn changed(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Changed,
            host_state: HostVerificationState::Changed,
            host_version: None,
            managed_config: ManagedConfigStatus::Changed,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Changed,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn rejected(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Rejected,
            host_state: HostVerificationState::Rejected,
            host_version: None,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::Available,
            host_gate: HostGateStatus::Rejected,
            host_configuration: HostConfigurationStatus::Discovered,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn unavailable(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Unavailable,
            host_state: HostVerificationState::Unavailable,
            host_version: None,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::Unavailable,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn unknown(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Unknown,
            host_state: HostVerificationState::Unknown,
            host_version: None,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn failed(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Failed,
            host_state: HostVerificationState::Failed,
            host_version: None,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: None,
            failure_reason: None,
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn unsupported_contract(reason: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::UnsupportedContract,
            host_state: HostVerificationState::UnsupportedContract,
            host_version: None,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::Available,
            host_gate: HostGateStatus::Rejected,
            host_configuration: HostConfigurationStatus::Discovered,
            host_policy_overlay: None,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            failure_category: Some(FailureCategory::UnsupportedContract),
            failure_reason: Some(reason.into()),
            managed_host_evidence: None,
            user_actions: Vec::new(),
        }
    }

    pub fn with_managed_config(mut self, managed_config: ManagedConfigStatus) -> Self {
        self.managed_config = managed_config;
        self
    }

    pub fn with_host_version(mut self, host_version: Option<String>) -> Self {
        self.host_version = host_version;
        self
    }

    pub fn with_host_executable(mut self, host_executable: HostExecutableStatus) -> Self {
        self.host_executable = host_executable;
        self
    }

    pub fn with_host_gate(mut self, host_gate: HostGateStatus) -> Self {
        self.host_gate = host_gate;
        self
    }

    pub fn with_host_configuration(mut self, host_configuration: HostConfigurationStatus) -> Self {
        self.host_configuration = host_configuration;
        self
    }

    pub fn with_host_policy_overlay(mut self, overlay: HostPolicyOverlayDiagnostic) -> Self {
        self.host_policy_overlay = Some(overlay);
        self
    }

    pub fn with_project_trust(mut self, project_trust: ProjectTrustDiagnostic) -> Self {
        self.project_trust = Some(project_trust);
        self
    }

    pub fn with_host_runtime(mut self, host_runtime: HostRuntimeDiagnostic) -> Self {
        self.host_runtime = Some(host_runtime);
        self
    }

    pub fn with_host_mcp_command(mut self, host_mcp_command: HostMcpCommandDiagnostic) -> Self {
        self.host_mcp_command = Some(host_mcp_command);
        self
    }

    pub fn with_mcp_handshake_allowed(mut self, allowed: bool) -> Self {
        self.mcp_handshake_allowed = allowed;
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }

    pub fn with_failure(mut self, category: FailureCategory, reason: impl Into<String>) -> Self {
        self.failure_category = Some(category);
        self.failure_reason = Some(reason.into());
        self
    }

    pub fn with_terminal_failure(
        mut self,
        category: FailureCategory,
        reason: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        self.status = if category == FailureCategory::UnsupportedContract {
            VerificationStatus::UnsupportedContract
        } else {
            VerificationStatus::Failed
        };
        self.host_state = if category == FailureCategory::UnsupportedContract {
            HostVerificationState::UnsupportedContract
        } else {
            HostVerificationState::Failed
        };
        self.host_gate = HostGateStatus::Rejected;
        self.mcp_handshake_allowed = false;
        self.details = details.into();
        self.failure_category = Some(category);
        self.failure_reason = Some(reason.into());
        self
    }

    pub fn with_user_actions(mut self, user_actions: Vec<UserAction>) -> Self {
        self.user_actions = user_actions;
        self
    }

    pub fn merge_user_actions(mut self, user_actions: &[UserAction]) -> Self {
        for action in user_actions {
            if !self.user_actions.contains(action) {
                self.user_actions.push(action.clone());
            }
        }
        self
    }

    pub fn host_policy_overlay_state(&self) -> HostPolicyOverlayState {
        HostPolicyOverlayState::from_diagnostic(self.host_policy_overlay.as_ref())
    }

    pub fn host_config_state(&self) -> HostConfigState {
        HostConfigState::from_managed_config(self.managed_config, self.host_policy_overlay_state())
    }

    pub fn host_approval_state(&self) -> HostApprovalState {
        if self.project_trust.is_some() {
            HostApprovalState::from_project_trust(self.project_trust.as_ref())
        } else {
            HostApprovalState::from_host_gate(self.host_gate)
        }
    }

    pub fn managed_lifecycle(&self) -> Option<ManagedHostLifecycle> {
        self.host_runtime
            .as_ref()
            .map(ManagedHostLifecycle::from_runtime)
    }

    pub fn active_tool_exposure_state(&self) -> ActiveToolExposureStatus {
        self.host_runtime
            .as_ref()
            .map(|runtime| runtime.active_tool_exposure)
            .unwrap_or(ActiveToolExposureStatus::Unknown)
    }

    pub fn storage_capability(&self) -> StorageCapability {
        self.host_runtime
            .as_ref()
            .and_then(|runtime| runtime.managed_host_storage.as_ref())
            .map(ManagedHostStorageDiagnostic::storage_capability)
            .unwrap_or(StorageCapability::Unknown)
    }

    pub fn common_contract(&self) -> HostVerificationContract {
        HostVerificationContract {
            host_config: self.host_config_state(),
            managed_identity: self.managed_config,
            host_policy_overlay: self.host_policy_overlay_state(),
            host_approval: self.host_approval_state(),
            managed_lifecycle: self.managed_lifecycle(),
            active_tool_exposure: self.active_tool_exposure_state(),
            storage_capability: self.storage_capability(),
            guarantee_disclosure: VerificationGuaranteeDisclosure::CooperativeHostObservation,
        }
    }
}

fn host_state_from_status(status: VerificationStatus) -> HostVerificationState {
    match status {
        VerificationStatus::Complete => HostVerificationState::ConfiguredReady,
        VerificationStatus::ActionRequired => HostVerificationState::ConfiguredActionRequired,
        VerificationStatus::Missing => HostVerificationState::Missing,
        VerificationStatus::Changed => HostVerificationState::Changed,
        VerificationStatus::Rejected => HostVerificationState::Rejected,
        VerificationStatus::Unavailable => HostVerificationState::Unavailable,
        VerificationStatus::Unknown => HostVerificationState::Unknown,
        VerificationStatus::Failed => HostVerificationState::Failed,
        VerificationStatus::UnsupportedContract => HostVerificationState::UnsupportedContract,
        VerificationStatus::NotVerified => HostVerificationState::NotVerified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_have_stable_text() {
        assert_eq!(
            HostVerificationState::ConfiguredActionRequired.as_str(),
            "configured_action_required"
        );
        assert_eq!(
            ManagedConfigStatus::NotApplicable.as_str(),
            "not_applicable"
        );
        assert_eq!(ProjectTrustStatus::Unreadable.as_str(), "unreadable");
        assert_eq!(
            HostRuntimeObservationStatus::NotObserved.as_str(),
            "not_observed"
        );
        assert_eq!(
            HostMcpCommandLaunchMode::PathResolved.as_str(),
            "path_resolved"
        );
        assert_eq!(Verification::changed("changed").status.as_str(), "changed");
    }

    #[test]
    fn common_contract_maps_managed_codex_parts() {
        let verification = Verification::configured_ready("ready")
            .with_host_policy_overlay(HostPolicyOverlayDiagnostic {
                present: true,
                accepted: true,
                kind: "codex_tool_approval".to_owned(),
                tool_count: 1,
                tools: vec!["volicord.status".to_owned()],
                entries: vec![HostPolicyOverlayEntryDiagnostic {
                    tool: "volicord.status".to_owned(),
                    approval_mode: "approve".to_owned(),
                }],
                details: "accepted".to_owned(),
            })
            .with_project_trust(ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Trusted,
                config_path: "/codex/config.toml".to_owned(),
                repo_root: "/repo".to_owned(),
                details: "trusted".to_owned(),
            })
            .with_host_runtime(HostRuntimeDiagnostic {
                status: HostRuntimeObservationStatus::Observed,
                managed_host_startup: HostRuntimeObservationStatus::Observed,
                managed_host_tools_list: HostRuntimeObservationStatus::Observed,
                managed_host_tool_call: HostRuntimeObservationStatus::Observed,
                active_tool_exposure: ActiveToolExposureStatus::Confirmed,
                managed_host_storage: Some(ManagedHostStorageDiagnostic {
                    storage_read: "passed".to_owned(),
                    storage_write: "passed".to_owned(),
                    effective_tool_mode: "workflow".to_owned(),
                    source_lifecycle_event: "managed_host_tool_call".to_owned(),
                    observed_at: Some("2026-07-01T00:00:00Z".to_owned()),
                }),
                details: "observed".to_owned(),
                last_observed_at: Some("2026-07-01T00:00:00Z".to_owned()),
            });

        let contract = verification.common_contract();

        assert_eq!(
            contract.host_config,
            HostConfigState::MatchWithHostPolicyOverlay
        );
        assert_eq!(contract.managed_identity, ManagedConfigStatus::Match);
        assert_eq!(
            contract.host_policy_overlay,
            HostPolicyOverlayState::Accepted
        );
        assert_eq!(contract.host_approval, HostApprovalState::Trusted);
        assert_eq!(
            contract
                .managed_lifecycle
                .expect("managed lifecycle should map")
                .managed_host_tool_call,
            HostRuntimeObservationStatus::Observed
        );
        assert_eq!(
            contract.active_tool_exposure,
            ActiveToolExposureStatus::Confirmed
        );
        assert_eq!(contract.storage_capability, StorageCapability::ReadWrite);
        assert_eq!(
            contract.guarantee_disclosure,
            VerificationGuaranteeDisclosure::CooperativeHostObservation
        );
    }

    #[test]
    fn storage_capability_maps_preflight_and_lifecycle_values() {
        assert_eq!(
            StorageCapability::from_mcp_storage_capability("read_write"),
            StorageCapability::ReadWrite
        );
        assert_eq!(
            StorageCapability::from_mcp_storage_capability("read_only"),
            StorageCapability::ReadOnly
        );
        assert_eq!(
            StorageCapability::from_read_write_status("passed", "readonly", "read_only"),
            StorageCapability::ReadOnly
        );
        assert_eq!(
            StorageCapability::from_read_write_status("failed", "skipped", "unavailable"),
            StorageCapability::Unavailable
        );
    }
}
