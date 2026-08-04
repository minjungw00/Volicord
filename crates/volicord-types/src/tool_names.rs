use std::{error::Error, fmt, str::FromStr};

use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
    JsonSchema,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::values::{AgentConnectionMode, MethodName, OperationCategory};

/// Prospective effect of one tool invocation on Product Repository files.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ProductRepositoryEffect {
    /// The tool contract cannot write Product Repository files.
    NoProductWrite,
    /// The invocation may write Product Repository files.
    MayWriteProduct,
    /// The invocation contract cannot determine its Product Repository effect.
    UnknownProductEffect,
}

impl ProductRepositoryEffect {
    /// The complete closed effect vocabulary.
    pub const ALL: [Self; 3] = [
        Self::NoProductWrite,
        Self::MayWriteProduct,
        Self::UnknownProductEffect,
    ];

    /// Returns the stable semantic spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoProductWrite => "no_product_write",
            Self::MayWriteProduct => "may_write_product",
            Self::UnknownProductEffect => "unknown_product_effect",
        }
    }
}

/// Closed semantic roles used to select a tool for operational verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolVerificationRole {
    /// The tool whose successful managed-host call proves an MCP round trip.
    ManagedHostRoundTrip,
}

/// Semantic relevance of one canonical Agent tool to Guard integration verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegrationVerificationToolRole {
    /// The only tool whose hook events can provide correlated Guard-probe proof.
    ProbeTarget,
    /// A begin or status operation that controls the verification workflow.
    WorkflowControl,
    /// A known Volicord tool unrelated to the correlated Guard-probe target.
    UnrelatedKnownTool,
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
    ConnectionIntegration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AgentToolKind {
    Method(MethodName),
    ListProjects,
    BeginIntegrationVerification,
    GuardProbe,
    GetIntegrationVerification,
}

/// Canonical typed identity for every Agent Connection MCP tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentToolId(AgentToolKind);

impl JsonSchema for AgentToolId {
    fn schema_name() -> String {
        "AgentToolId".to_owned()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            enum_values: Some(
                Self::ALL
                    .into_iter()
                    .map(|tool| serde_json::Value::String(tool.wire_name().to_owned()))
                    .collect(),
            ),
            ..Default::default()
        })
    }
}

impl AgentToolId {
    pub const INTAKE: Self = Self(AgentToolKind::Method(MethodName::Intake));
    pub const UPDATE_SCOPE: Self = Self(AgentToolKind::Method(MethodName::UpdateScope));
    pub const RECORD_SHAPING_CHECKPOINT: Self =
        Self(AgentToolKind::Method(MethodName::RecordShapingCheckpoint));
    pub const FINALIZE_ADVICE: Self = Self(AgentToolKind::Method(MethodName::FinalizeAdvice));
    pub const ADVANCE_TASK: Self = Self(AgentToolKind::Method(MethodName::AdvanceTask));
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
    pub const BEGIN_INTEGRATION_VERIFICATION: Self =
        Self(AgentToolKind::BeginIntegrationVerification);
    pub const GUARD_PROBE: Self = Self(AgentToolKind::GuardProbe);
    pub const GET_INTEGRATION_VERIFICATION: Self = Self(AgentToolKind::GetIntegrationVerification);

    /// The complete Agent Connection MCP tool catalog in stable discovery order.
    pub const ALL: [Self; 19] = [
        Self::INTAKE,
        Self::UPDATE_SCOPE,
        Self::RECORD_SHAPING_CHECKPOINT,
        Self::FINALIZE_ADVICE,
        Self::ADVANCE_TASK,
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
        Self::BEGIN_INTEGRATION_VERIFICATION,
        Self::GUARD_PROBE,
        Self::GET_INTEGRATION_VERIFICATION,
    ];

    /// Returns the canonical Agent Connection identity for a public Core method.
    pub const fn from_method(method: MethodName) -> Option<Self> {
        match method {
            MethodName::Intake => Some(Self::INTAKE),
            MethodName::UpdateScope => Some(Self::UPDATE_SCOPE),
            MethodName::RecordShapingCheckpoint => Some(Self::RECORD_SHAPING_CHECKPOINT),
            MethodName::FinalizeAdvice => Some(Self::FINALIZE_ADVICE),
            MethodName::AdvanceTask => Some(Self::ADVANCE_TASK),
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
            AgentToolKind::BeginIntegrationVerification => {
                "volicord.begin_integration_verification"
            }
            AgentToolKind::GuardProbe => "volicord.guard_probe",
            AgentToolKind::GetIntegrationVerification => "volicord.get_integration_verification",
        }
    }

    /// Returns this tool's behavioral category.
    pub const fn category(self) -> AgentToolCategory {
        match self.0 {
            AgentToolKind::Method(
                MethodName::Status | MethodName::GetOperationResult | MethodName::CheckClose,
            )
            | AgentToolKind::ListProjects => AgentToolCategory::ReadOnly,
            AgentToolKind::BeginIntegrationVerification
            | AgentToolKind::GuardProbe
            | AgentToolKind::GetIntegrationVerification => {
                AgentToolCategory::NonDestructiveMutation
            }
            AgentToolKind::Method(
                MethodName::PrepareEvidenceCapture
                | MethodName::PrepareWrite
                | MethodName::StageArtifact,
            ) => AgentToolCategory::NonDestructiveMutation,
            AgentToolKind::Method(_) => AgentToolCategory::DestructiveMutation,
        }
    }

    /// Returns this tool's prospective Product Repository effect.
    ///
    /// Runtime Home, Core, and adapter-state mutation is independent of this
    /// Product Repository boundary.
    pub const fn product_repository_effect(self) -> ProductRepositoryEffect {
        match self.0 {
            AgentToolKind::Method(_)
            | AgentToolKind::ListProjects
            | AgentToolKind::BeginIntegrationVerification
            | AgentToolKind::GuardProbe
            | AgentToolKind::GetIntegrationVerification => ProductRepositoryEffect::NoProductWrite,
        }
    }

    /// Returns this tool's implementation owner.
    pub const fn owner(self) -> AgentToolOwner {
        match self.0 {
            AgentToolKind::Method(method) => AgentToolOwner::CoreMethod(method),
            AgentToolKind::ListProjects => AgentToolOwner::AdapterUtility,
            AgentToolKind::BeginIntegrationVerification
            | AgentToolKind::GuardProbe
            | AgentToolKind::GetIntegrationVerification => AgentToolOwner::ConnectionIntegration,
        }
    }

    /// Returns the public Core method owned by this tool, when applicable.
    pub const fn method(self) -> Option<MethodName> {
        match self.owner() {
            AgentToolOwner::CoreMethod(method) => Some(method),
            AgentToolOwner::AdapterUtility | AgentToolOwner::ConnectionIntegration => None,
        }
    }

    /// Returns whether this tool is exposed in the supplied Connection mode.
    pub const fn available_in(self, mode: AgentConnectionMode) -> bool {
        match mode {
            AgentConnectionMode::Workflow => true,
            AgentConnectionMode::ReadOnly => {
                matches!(self.owner(), AgentToolOwner::ConnectionIntegration)
                    || matches!(self.category(), AgentToolCategory::ReadOnly)
            }
        }
    }

    /// Returns the operational verification role assigned to this tool.
    pub const fn verification_role(self) -> Option<ToolVerificationRole> {
        match self.0 {
            AgentToolKind::ListProjects => Some(ToolVerificationRole::ManagedHostRoundTrip),
            AgentToolKind::Method(_)
            | AgentToolKind::BeginIntegrationVerification
            | AgentToolKind::GuardProbe
            | AgentToolKind::GetIntegrationVerification => None,
        }
    }

    /// Returns this tool's semantic role in Guard integration verification.
    pub const fn integration_verification_role(self) -> IntegrationVerificationToolRole {
        match self.0 {
            AgentToolKind::GuardProbe => IntegrationVerificationToolRole::ProbeTarget,
            AgentToolKind::BeginIntegrationVerification
            | AgentToolKind::GetIntegrationVerification => {
                IntegrationVerificationToolRole::WorkflowControl
            }
            AgentToolKind::Method(_) | AgentToolKind::ListProjects => {
                IntegrationVerificationToolRole::UnrelatedKnownTool
            }
        }
    }

    /// Returns whether retrying this tool with the same integration coordinate is idempotent.
    pub const fn is_idempotent(self) -> bool {
        matches!(
            self.0,
            AgentToolKind::ListProjects
                | AgentToolKind::BeginIntegrationVerification
                | AgentToolKind::GuardProbe
                | AgentToolKind::GetIntegrationVerification
                | AgentToolKind::Method(
                    MethodName::Status | MethodName::GetOperationResult | MethodName::CheckClose
                )
        )
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
    fn integration_verification_tools_share_one_non_core_owner_and_all_modes() {
        for tool in [
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
            AgentToolId::GUARD_PROBE,
            AgentToolId::GET_INTEGRATION_VERIFICATION,
        ] {
            assert!(matches!(
                tool.owner(),
                AgentToolOwner::ConnectionIntegration
            ));
            assert!(tool.available_in(AgentConnectionMode::ReadOnly));
            assert!(tool.available_in(AgentConnectionMode::Workflow));
            assert!(tool.is_idempotent());
            assert_eq!(tool.method(), None);
        }
    }

    #[test]
    fn every_agent_tool_has_one_no_product_write_effect() {
        for tool in AgentToolId::ALL {
            assert_eq!(
                tool.product_repository_effect(),
                ProductRepositoryEffect::NoProductWrite,
                "{tool}"
            );
        }
    }

    #[test]
    fn unknown_agent_tool_wire_names_are_rejected() {
        for unknown in [
            "",
            "volicord.unknown",
            "volicord/list_projects",
            "volicord.list-projects",
            "VOLICORD.list_projects",
            " volicord.list_projects",
            "volicord.list_projects ",
            "volicord.list_projects\0ignored",
            MethodName::ResolveUserAction.as_str(),
        ] {
            assert_eq!(
                AgentToolId::from_wire_name(unknown),
                Err(AgentToolIdParseError),
                "{unknown:?}"
            );
        }
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

    #[test]
    fn integration_verification_roles_cover_the_complete_canonical_catalog() {
        for tool in AgentToolId::ALL {
            let expected = if tool == AgentToolId::GUARD_PROBE {
                IntegrationVerificationToolRole::ProbeTarget
            } else if matches!(
                tool,
                AgentToolId::BEGIN_INTEGRATION_VERIFICATION
                    | AgentToolId::GET_INTEGRATION_VERIFICATION
            ) {
                IntegrationVerificationToolRole::WorkflowControl
            } else {
                IntegrationVerificationToolRole::UnrelatedKnownTool
            };
            assert_eq!(tool.integration_verification_role(), expected);
        }
    }
}
