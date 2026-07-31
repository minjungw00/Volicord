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

impl WorkflowPolicySchema {
    /// Returns the exact current workflow-policy schema identity.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "volicord.workflow_policy",
        }
    }
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

/// Exact schema identity carried by the current policy-show report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum PolicyShowReportSchema {
    #[serde(rename = "volicord.policy_show_report")]
    Current,
}

impl PolicyShowReportSchema {
    /// Returns the exact current policy-show report identity.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "volicord.policy_show_report",
        }
    }
}

/// Closed status of a successfully inspected authoritative workflow policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyShowStatus {
    Active,
}

/// Closed synchronization state of the managed workflow-policy file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManagedPolicyFileStatus {
    Matches,
    Missing,
    Malformed,
    PermissionFailure,
    BindingMismatch,
    FingerprintMismatch,
}

impl ManagedPolicyFileStatus {
    /// Returns the exact current machine value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Matches => "matches",
            Self::Missing => "missing",
            Self::Malformed => "malformed",
            Self::PermissionFailure => "permission_failure",
            Self::BindingMismatch => "binding_mismatch",
            Self::FingerprintMismatch => "fingerprint_mismatch",
        }
    }
}

/// Closed action kind exposed by a policy-show report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyShowActionKind {
    RepairManagedPolicy,
}

/// Closed administrative command exposed by a policy-show action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum PolicyShowActionCommand {
    #[serde(rename = "volicord policy apply")]
    PolicyApply,
}

impl PolicyShowActionCommand {
    /// Returns the exact current command path.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PolicyApply => "volicord policy apply",
        }
    }
}

/// One typed repair action exposed when the managed file is not synchronized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyShowAction {
    pub action: PolicyShowActionKind,
    pub command: PolicyShowActionCommand,
    pub arguments: Vec<String>,
}

/// Complete authoritative workflow-policy facts in a policy-show report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyShowAuthority {
    pub source: ProjectWorkflowPolicySource,
    pub policy_version: u64,
    pub policy_fingerprint: String,
    pub policy: ProjectWorkflowPolicy,
}

/// Managed-file synchronization facts in a policy-show report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyShowManagedFile {
    pub path: String,
    pub status: ManagedPolicyFileStatus,
    pub schema: Option<WorkflowPolicySchema>,
    pub fingerprint: Option<String>,
    pub matches_authority: bool,
}

/// Lossless current result of authoritative workflow-policy inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyShowReport {
    pub schema: PolicyShowReportSchema,
    pub status: PolicyShowStatus,
    pub repository: String,
    pub authority: PolicyShowAuthority,
    pub managed_file: PolicyShowManagedFile,
    pub active_task_requires_escalation: bool,
    pub actions: Vec<PolicyShowAction>,
}

/// Closed status of a successful workflow-policy file validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyValidationStatus {
    Valid,
}

/// Typed result of validating one workflow-policy file without effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyValidationReport {
    pub status: PolicyValidationStatus,
    pub file: String,
    pub policy_schema: WorkflowPolicySchema,
    pub policy_fingerprint: String,
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

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn current_policy() -> ProjectWorkflowPolicy {
        serde_json::from_value(json!({
            "schema": "volicord.workflow_policy",
            "managed_by": "volicord",
            "storage_scope": "local_overlay",
            "connection_intent": "personal",
            "host": "codex",
            "repo_root": "/workspace/product",
            "connection_id": "connection_example",
            "guard_installation_id": "guard_example",
            "selected_profile": "record",
            "mcp": {
                "command": "/opt/volicord/bin/volicord",
                "args": ["_host-launch", "codex", "--connection", "connection_example"],
                "env": {"VOLICORD_HOME": "/srv/volicord"}
            },
            "host_hook": {
                "enabled": true,
                "commands": {
                    "pre_tool": {"command": "volicord", "args": ["guard", "pre-tool"]},
                    "post_tool": {"command": "volicord", "args": ["guard", "post-tool"]},
                    "prompt_capture": {
                        "command": "volicord",
                        "args": ["guard", "prompt-capture"]
                    }
                }
            },
            "workflow": {
                "default_direct_control": "tracked",
                "default_work_control": "tracked",
                "light": {
                    "enabled": false,
                    "max_intended_paths": 3,
                    "allowed_path_patterns": [],
                    "denied_path_patterns": [],
                    "final_acceptance": "policy_dependent"
                },
                "write_ticket": {"idle_timeout_minutes": null}
            }
        }))
        .expect("current workflow policy")
    }

    #[test]
    fn policy_show_report_keeps_report_and_nested_policy_identities_distinct() {
        let report = PolicyShowReport {
            schema: PolicyShowReportSchema::Current,
            status: PolicyShowStatus::Active,
            repository: "/workspace/product".to_owned(),
            authority: PolicyShowAuthority {
                source: ProjectWorkflowPolicySource::ProjectDatabase,
                policy_version: 1,
                policy_fingerprint: format!("sha256:{}", "a".repeat(64)),
                policy: current_policy(),
            },
            managed_file: PolicyShowManagedFile {
                path: "/workspace/product/.volicord/policy.json".to_owned(),
                status: ManagedPolicyFileStatus::Matches,
                schema: Some(WorkflowPolicySchema::Current),
                fingerprint: Some(format!("sha256:{}", "a".repeat(64))),
                matches_authority: true,
            },
            active_task_requires_escalation: false,
            actions: Vec::new(),
        };

        let value = serde_json::to_value(&report).expect("report serializes");
        assert_eq!(value["schema"], "volicord.policy_show_report");
        assert_eq!(
            value["authority"]["policy"]["schema"],
            "volicord.workflow_policy"
        );
        assert_eq!(
            value["authority"]["policy"]["workflow"]["default_work_control"],
            "tracked"
        );
        assert_eq!(
            serde_json::from_value::<PolicyShowReport>(value).expect("report decodes"),
            report
        );
    }

    #[test]
    fn policy_report_results_reject_unknown_fields_and_closed_values() {
        let mut validation = serde_json::to_value(PolicyValidationReport {
            status: PolicyValidationStatus::Valid,
            file: "/workspace/product/.volicord/policy.json".to_owned(),
            policy_schema: WorkflowPolicySchema::Current,
            policy_fingerprint: format!("sha256:{}", "b".repeat(64)),
        })
        .expect("validation report serializes");
        validation
            .as_object_mut()
            .expect("validation object")
            .insert("unexpected".to_owned(), Value::Bool(true));
        assert!(serde_json::from_value::<PolicyValidationReport>(validation).is_err());

        assert!(serde_json::from_value::<PolicyShowReportSchema>(json!(
            "volicord.workflow_policy"
        ))
        .is_err());
        assert!(serde_json::from_value::<ManagedPolicyFileStatus>(json!("stale")).is_err());
    }
}
