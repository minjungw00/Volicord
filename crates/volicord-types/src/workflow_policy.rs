//! Dependency-safe typed project workflow-policy schema.

use std::{collections::BTreeMap, error::Error, fmt};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    guard_manifest::GuardCommandSet,
    host_configuration::ConnectionIntent,
    product_path::ProductRelativePath,
    values::{AcceptancePolicy, HostKind, IntegrationProfile, TaskControlLevel},
};

/// Exact manager identity carried by the current workflow-policy document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WorkflowPolicyManager {
    #[serde(rename = "volicord")]
    Volicord,
}

/// Exact storage scope carried by the current workflow-policy document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WorkflowPolicyStorageScope {
    #[serde(rename = "local_overlay")]
    LocalOverlay,
}

/// Exact schema identity carried by the current workflow-policy document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WorkflowPolicySchema {
    #[serde(rename = "volicord.workflow_policy")]
    Current,
}

/// Store-facing provenance of the authoritative workflow-policy copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectWorkflowPolicySource {
    ProjectDatabase,
    VolicordInit,
}

impl ProjectWorkflowPolicySource {
    /// Returns the canonical persisted value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectDatabase => "project_database",
            Self::VolicordInit => "volicord_init",
        }
    }
}

/// MCP launch facts embedded in the current workflow-policy document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicyMcp {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

/// Host-hook facts embedded in the current workflow-policy document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicyHostHook {
    pub enabled: bool,
    pub commands: GuardCommandSet,
}

/// Light-control settings embedded in the current workflow policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LightWorkflowPolicy {
    pub enabled: bool,
    pub max_intended_paths: u64,
    pub allowed_path_patterns: Vec<ProductRelativePath>,
    pub denied_path_patterns: Vec<ProductRelativePath>,
    pub final_acceptance: AcceptancePolicy,
}

/// Write-ticket settings embedded in the current workflow policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicyWriteTicket {
    pub idle_timeout_minutes: Option<u64>,
}

/// Adapter-neutral workflow settings used by Core policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicySettings {
    pub default_direct_control: TaskControlLevel,
    pub default_work_control: TaskControlLevel,
    pub light: LightWorkflowPolicy,
    pub write_ticket: WorkflowPolicyWriteTicket,
}

/// Complete current project workflow-policy document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectWorkflowPolicy {
    pub schema: WorkflowPolicySchema,
    pub managed_by: WorkflowPolicyManager,
    pub storage_scope: WorkflowPolicyStorageScope,
    pub connection_intent: ConnectionIntent,
    pub host: HostKind,
    pub repo_root: String,
    pub connection_id: String,
    pub guard_installation_id: String,
    pub selected_profile: IntegrationProfile,
    pub mcp: WorkflowPolicyMcp,
    pub host_hook: WorkflowPolicyHostHook,
    pub workflow: WorkflowPolicySettings,
}

impl ProjectWorkflowPolicy {
    /// Validates non-closed structural requirements that serde cannot express.
    pub fn validate(&self) -> Result<(), WorkflowPolicyShapeError> {
        for (field, value) in [
            ("repo_root", self.repo_root.as_str()),
            ("connection_id", self.connection_id.as_str()),
            ("guard_installation_id", self.guard_installation_id.as_str()),
            ("mcp.command", self.mcp.command.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(WorkflowPolicyShapeError { field });
            }
        }
        if !self.host_hook.enabled {
            return Err(WorkflowPolicyShapeError {
                field: "host_hook.enabled",
            });
        }
        for phase in crate::values::GuardHookPhase::REQUIRED {
            if self.host_hook.commands.get(phase).command.trim().is_empty() {
                return Err(WorkflowPolicyShapeError {
                    field: "host_hook.commands.command",
                });
            }
        }
        if self.workflow.light.max_intended_paths == 0
            || usize::try_from(self.workflow.light.max_intended_paths).is_err()
        {
            return Err(WorkflowPolicyShapeError {
                field: "workflow.light.max_intended_paths",
            });
        }
        if self
            .workflow
            .write_ticket
            .idle_timeout_minutes
            .is_some_and(|minutes| minutes == 0)
        {
            return Err(WorkflowPolicyShapeError {
                field: "workflow.write_ticket.idle_timeout_minutes",
            });
        }
        Ok(())
    }
}

/// Structural validation failure for a decoded workflow policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowPolicyShapeError {
    field: &'static str,
}

impl WorkflowPolicyShapeError {
    /// Returns the semantic field that failed validation.
    pub const fn field(self) -> &'static str {
        self.field
    }
}

impl fmt::Display for WorkflowPolicyShapeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "workflow policy field `{}` is not valid",
            self.field
        )
    }
}

impl Error for WorkflowPolicyShapeError {}
