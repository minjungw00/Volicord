use serde::Serialize;

use super::UserAction;

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
            Self::NotVerified => "not_verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedConfigStatus {
    Match,
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
    pub managed_config: ManagedConfigStatus,
    pub host_executable: HostExecutableStatus,
    pub host_gate: HostGateStatus,
    pub host_configuration: HostConfigurationStatus,
    pub project_trust: Option<ProjectTrustDiagnostic>,
    pub host_runtime: Option<HostRuntimeDiagnostic>,
    pub host_mcp_command: Option<HostMcpCommandDiagnostic>,
    pub mcp_handshake_allowed: bool,
    pub details: String,
    pub diagnostic: Option<String>,
    pub user_actions: Vec<UserAction>,
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
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn configured_ready(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Complete,
            host_state: HostVerificationState::ConfiguredReady,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::NotRequired,
            host_gate: HostGateStatus::Ready,
            host_configuration: HostConfigurationStatus::Discovered,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: true,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn action_required(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::ActionRequired,
            host_state: HostVerificationState::ConfiguredActionRequired,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::ActionRequired,
            host_configuration: HostConfigurationStatus::Discovered,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: true,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn missing(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Missing,
            host_state: HostVerificationState::Missing,
            managed_config: ManagedConfigStatus::Missing,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Missing,
            host_configuration: HostConfigurationStatus::Missing,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn changed(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Changed,
            host_state: HostVerificationState::Changed,
            managed_config: ManagedConfigStatus::Changed,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Changed,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn rejected(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Rejected,
            host_state: HostVerificationState::Rejected,
            managed_config: ManagedConfigStatus::Match,
            host_executable: HostExecutableStatus::Available,
            host_gate: HostGateStatus::Rejected,
            host_configuration: HostConfigurationStatus::Discovered,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn unavailable(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Unavailable,
            host_state: HostVerificationState::Unavailable,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::Unavailable,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn unknown(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Unknown,
            host_state: HostVerificationState::Unknown,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn failed(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Failed,
            host_state: HostVerificationState::Failed,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            project_trust: None,
            host_runtime: None,
            host_mcp_command: None,
            mcp_handshake_allowed: false,
            details: details.into(),
            diagnostic: None,
            user_actions: Vec::new(),
        }
    }

    pub fn with_managed_config(mut self, managed_config: ManagedConfigStatus) -> Self {
        self.managed_config = managed_config;
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
}
