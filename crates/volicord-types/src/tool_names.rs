use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::values::{AgentConnectionMode, MethodName, OperationCategory};

/// Closed semantic roles used to select a tool for operational verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolVerificationRole {
    /// The tool whose successful managed-host call proves an MCP round trip.
    ManagedHostRoundTrip,
}

impl ToolVerificationRole {
    /// Returns the canonical tool bound to this role at compile time.
    pub const fn tool(self) -> AgentToolId {
        match self {
            Self::ManagedHostRoundTrip => AgentToolId::LIST_PROJECTS,
        }
    }
}

/// Stable behavioral category for an Agent Connection MCP tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentToolCategory {
    ReadOnly,
    NonDestructiveMutation,
    DestructiveMutation,
}

impl AgentToolCategory {
    /// Returns the Core-facing operation category for this tool category.
    pub const fn operation_category(self) -> OperationCategory {
        match self {
            Self::ReadOnly => OperationCategory::Read,
            Self::NonDestructiveMutation | Self::DestructiveMutation => {
                OperationCategory::AgentWorkflow
            }
        }
    }
}

/// Implementation owner for an Agent Connection MCP tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentToolOwner {
    CoreMethod(MethodName),
    AdapterUtility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AgentToolKind {
    Method(MethodName),
    ListProjects,
}

/// Canonical typed identity for every Agent Connection MCP tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentToolId(AgentToolKind);

impl AgentToolId {
    pub const INTAKE: Self = Self(AgentToolKind::Method(MethodName::Intake));
    pub const UPDATE_SCOPE: Self = Self(AgentToolKind::Method(MethodName::UpdateScope));
    pub const STATUS: Self = Self(AgentToolKind::Method(MethodName::Status));
    pub const GET_OPERATION_RESULT: Self =
        Self(AgentToolKind::Method(MethodName::GetOperationResult));
    pub const PREPARE_EVIDENCE_CAPTURE: Self =
        Self(AgentToolKind::Method(MethodName::PrepareEvidenceCapture));
    pub const PREPARE_WRITE: Self = Self(AgentToolKind::Method(MethodName::PrepareWrite));
    pub const STAGE_ARTIFACT: Self = Self(AgentToolKind::Method(MethodName::StageArtifact));
    pub const RECORD_RUN: Self = Self(AgentToolKind::Method(MethodName::RecordRun));
    pub const REQUEST_USER_ACTION: Self =
        Self(AgentToolKind::Method(MethodName::RequestUserAction));
    pub const RECONCILE_CHANGES: Self = Self(AgentToolKind::Method(MethodName::ReconcileChanges));
    pub const CHECK_CLOSE: Self = Self(AgentToolKind::Method(MethodName::CheckClose));
    pub const CLOSE_TASK: Self = Self(AgentToolKind::Method(MethodName::CloseTask));
    pub const LIST_PROJECTS: Self = Self(AgentToolKind::ListProjects);

    /// The complete Agent Connection MCP tool catalog in stable discovery order.
    pub const ALL: [Self; 13] = [
        Self::INTAKE,
        Self::UPDATE_SCOPE,
        Self::STATUS,
        Self::GET_OPERATION_RESULT,
        Self::PREPARE_EVIDENCE_CAPTURE,
        Self::PREPARE_WRITE,
        Self::STAGE_ARTIFACT,
        Self::RECORD_RUN,
        Self::REQUEST_USER_ACTION,
        Self::RECONCILE_CHANGES,
        Self::CHECK_CLOSE,
        Self::CLOSE_TASK,
        Self::LIST_PROJECTS,
    ];

    /// Returns the canonical Agent Connection identity for a public Core method.
    pub const fn from_method(method: MethodName) -> Option<Self> {
        match method {
            MethodName::Intake => Some(Self::INTAKE),
            MethodName::UpdateScope => Some(Self::UPDATE_SCOPE),
            MethodName::Status => Some(Self::STATUS),
            MethodName::GetOperationResult => Some(Self::GET_OPERATION_RESULT),
            MethodName::CheckClose => Some(Self::CHECK_CLOSE),
            MethodName::PrepareEvidenceCapture => Some(Self::PREPARE_EVIDENCE_CAPTURE),
            MethodName::PrepareWrite => Some(Self::PREPARE_WRITE),
            MethodName::StageArtifact => Some(Self::STAGE_ARTIFACT),
            MethodName::RecordRun => Some(Self::RECORD_RUN),
            MethodName::RequestUserAction => Some(Self::REQUEST_USER_ACTION),
            MethodName::ResolveUserAction => None,
            MethodName::ReconcileChanges => Some(Self::RECONCILE_CHANGES),
            MethodName::CloseTask => Some(Self::CLOSE_TASK),
        }
    }

    /// Parses one exact public MCP wire name into its canonical identity.
    pub fn from_wire_name(name: &str) -> Result<Self, AgentToolIdParseError> {
        Self::ALL
            .iter()
            .copied()
            .find(|tool| tool.wire_name() == name)
            .ok_or(AgentToolIdParseError)
    }

    /// Returns the stable MCP wire name.
    pub const fn wire_name(self) -> &'static str {
        match self.0 {
            AgentToolKind::Method(method) => method.as_str(),
            AgentToolKind::ListProjects => "volicord.list_projects",
        }
    }

    /// Returns this tool's behavioral category.
    pub const fn category(self) -> AgentToolCategory {
        match self.0 {
            AgentToolKind::Method(
                MethodName::Status | MethodName::GetOperationResult | MethodName::CheckClose,
            )
            | AgentToolKind::ListProjects => AgentToolCategory::ReadOnly,
            AgentToolKind::Method(
                MethodName::PrepareEvidenceCapture
                | MethodName::PrepareWrite
                | MethodName::StageArtifact,
            ) => AgentToolCategory::NonDestructiveMutation,
            AgentToolKind::Method(_) => AgentToolCategory::DestructiveMutation,
        }
    }

    /// Returns this tool's implementation owner.
    pub const fn owner(self) -> AgentToolOwner {
        match self.0 {
            AgentToolKind::Method(method) => AgentToolOwner::CoreMethod(method),
            AgentToolKind::ListProjects => AgentToolOwner::AdapterUtility,
        }
    }

    /// Returns the public Core method owned by this tool, when applicable.
    pub const fn method(self) -> Option<MethodName> {
        match self.owner() {
            AgentToolOwner::CoreMethod(method) => Some(method),
            AgentToolOwner::AdapterUtility => None,
        }
    }

    /// Returns whether this tool is exposed in the supplied Connection mode.
    pub const fn available_in(self, mode: AgentConnectionMode) -> bool {
        match mode {
            AgentConnectionMode::Workflow => true,
            AgentConnectionMode::ReadOnly => {
                matches!(self.category(), AgentToolCategory::ReadOnly)
            }
        }
    }

    /// Returns the operational verification role assigned to this tool.
    pub const fn verification_role(self) -> Option<ToolVerificationRole> {
        match self.0 {
            AgentToolKind::ListProjects => Some(ToolVerificationRole::ManagedHostRoundTrip),
            AgentToolKind::Method(_) => None,
        }
    }
}

impl fmt::Display for AgentToolId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

impl FromStr for AgentToolId {
    type Err = AgentToolIdParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::from_wire_name(raw)
    }
}

impl Serialize for AgentToolId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.wire_name())
    }
}

impl<'de> Deserialize<'de> for AgentToolId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_wire_name(&raw).map_err(serde::de::Error::custom)
    }
}

/// Error returned for a wire name outside the canonical Agent Connection catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentToolIdParseError;

impl fmt::Display for AgentToolIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown Agent Connection MCP tool name")
    }
}

impl Error for AgentToolIdParseError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_agent_tool_wire_name_is_unique_and_round_trips() {
        let mut names = BTreeSet::new();
        for tool in AgentToolId::ALL {
            assert!(names.insert(tool.wire_name()));
            assert_eq!(AgentToolId::from_wire_name(tool.wire_name()), Ok(tool));
            assert_eq!(tool.wire_name().parse::<AgentToolId>(), Ok(tool));
        }
    }

    #[test]
    fn unknown_agent_tool_wire_names_are_rejected() {
        assert_eq!(
            AgentToolId::from_wire_name("volicord.unknown"),
            Err(AgentToolIdParseError)
        );
        assert!(AgentToolId::from_wire_name(MethodName::ResolveUserAction.as_str()).is_err());
    }

    #[test]
    fn core_tool_identity_reuses_method_name_identity() {
        for tool in AgentToolId::ALL {
            if let AgentToolOwner::CoreMethod(method) = tool.owner() {
                assert_eq!(AgentToolId::from_method(method), Some(tool));
                assert_eq!(tool.wire_name(), method.as_str());
            }
        }
        assert_eq!(
            AgentToolId::from_method(MethodName::ResolveUserAction),
            None
        );
    }

    #[test]
    fn managed_host_round_trip_role_is_bound_to_exposed_list_projects_tool() {
        let tool = ToolVerificationRole::ManagedHostRoundTrip.tool();
        assert_eq!(tool, AgentToolId::LIST_PROJECTS);
        assert_eq!(
            tool.verification_role(),
            Some(ToolVerificationRole::ManagedHostRoundTrip)
        );
        assert!(matches!(tool.owner(), AgentToolOwner::AdapterUtility));
        assert!(tool.available_in(AgentConnectionMode::ReadOnly));
        assert!(tool.available_in(AgentConnectionMode::Workflow));
    }
}
