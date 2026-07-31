use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedConfigStatus {
    Match,
    Unmanaged,
    Missing,
    Changed,
    Malformed,
    Unavailable,
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
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
        }
    }
}

/// Closed diagnostic vocabulary for one managed Codex MCP entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedConfigDiagnostic {
    TomlParseFailure,
    EntryMissing,
    EntryDisabled,
    CommandDrift,
    ArgumentDrift,
    StaticEnvironmentDrift,
    ForwardedEnvironmentDrift,
    FingerprintMismatch,
    MalformedApprovalOverlay,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostExecutableStatus {
    Available,
    Unavailable,
    NotChecked,
}

/// Installation and manual-invocation aid observed from the current PATH.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostExecutableProbe {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovered_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl HostExecutableStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::NotChecked => "not_checked",
        }
    }
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
    pub code: String,
    pub config_path: String,
    pub repo_root: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub config_target: String,
    pub managed_config: ManagedConfigStatus,
    pub managed_config_diagnostic: Option<ManagedConfigDiagnostic>,
    pub managed_config_details: String,
    pub host_executable: HostExecutableStatus,
    pub executable_path: Option<String>,
    pub host_version: Option<String>,
    pub host_executable_code: String,
    pub host_executable_details: String,
    pub project_trust: Option<ProjectTrustDiagnostic>,
}

impl Verification {
    pub fn unobserved(config_target: impl Into<String>) -> Self {
        Self {
            config_target: config_target.into(),
            managed_config: ManagedConfigStatus::Unknown,
            managed_config_diagnostic: Some(ManagedConfigDiagnostic::Unavailable),
            managed_config_details: "Managed Codex configuration was not inspected".to_owned(),
            host_executable: HostExecutableStatus::NotChecked,
            executable_path: None,
            host_version: None,
            host_executable_code: "host_executable_not_checked".to_owned(),
            host_executable_details: "Codex executable was not probed".to_owned(),
            project_trust: None,
        }
    }

    pub fn host_executable_probe(&self) -> HostExecutableProbe {
        HostExecutableProbe {
            discovered_path: self.executable_path.clone(),
            version: self.host_version.clone(),
        }
    }
}
