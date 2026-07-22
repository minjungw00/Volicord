use crate::values::MethodName;
use std::{error::Error, fmt};

/// Closed semantic roles used to select a tool for operational verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolVerificationRole {
    /// The tool whose successful managed-host call proves an MCP round trip.
    ManagedHostRoundTrip,
}

/// A malformed canonical verification-role assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolVerificationRoleResolutionError {
    pub role: ToolVerificationRole,
    pub owner_count: usize,
}

impl fmt::Display for ToolVerificationRoleResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "verification role {:?} must have exactly one tool owner, found {}",
            self.role, self.owner_count
        )
    }
}

impl Error for ToolVerificationRoleResolutionError {}

/// MCP-visible tool name for `volicord.intake`.
pub const INTAKE_TOOL_NAME: &str = MethodName::Intake.as_str();

/// MCP-visible tool name for `volicord.update_scope`.
pub const UPDATE_SCOPE_TOOL_NAME: &str = MethodName::UpdateScope.as_str();

/// MCP-visible tool name for `volicord.status`.
pub const STATUS_TOOL_NAME: &str = MethodName::Status.as_str();

/// MCP-visible tool name for `volicord.get_operation_result`.
pub const GET_OPERATION_RESULT_TOOL_NAME: &str = MethodName::GetOperationResult.as_str();

/// MCP-visible tool name for `volicord.prepare_write`.
pub const PREPARE_WRITE_TOOL_NAME: &str = MethodName::PrepareWrite.as_str();

/// MCP-visible tool name for `volicord.prepare_evidence_capture`.
pub const PREPARE_EVIDENCE_CAPTURE_TOOL_NAME: &str = MethodName::PrepareEvidenceCapture.as_str();

/// MCP-visible tool name for `volicord.stage_artifact`.
pub const STAGE_ARTIFACT_TOOL_NAME: &str = MethodName::StageArtifact.as_str();

/// MCP-visible tool name for `volicord.record_run`.
pub const RECORD_RUN_TOOL_NAME: &str = MethodName::RecordRun.as_str();

/// MCP-visible tool name for `volicord.request_user_action`.
pub const REQUEST_USER_ACTION_TOOL_NAME: &str = MethodName::RequestUserAction.as_str();

/// Public User Channel method name not exposed through Agent Connection MCP tool lists.
pub const RESOLVE_USER_ACTION_TOOL_NAME: &str = MethodName::ResolveUserAction.as_str();

/// MCP-visible tool name for `volicord.reconcile_changes`.
pub const RECONCILE_CHANGES_TOOL_NAME: &str = MethodName::ReconcileChanges.as_str();

/// MCP-visible tool name for `volicord.check_close`.
pub const CHECK_CLOSE_TOOL_NAME: &str = MethodName::CheckClose.as_str();

/// MCP-visible tool name for `volicord.close_task`.
pub const CLOSE_TASK_TOOL_NAME: &str = MethodName::CloseTask.as_str();

/// Adapter-owned project-list utility tool name.
pub const LIST_PROJECTS_TOOL_NAME: &str = "volicord.list_projects";

const TOOL_VERIFICATION_ROLE_ASSIGNMENTS: [(ToolVerificationRole, &str); 1] = [(
    ToolVerificationRole::ManagedHostRoundTrip,
    LIST_PROJECTS_TOOL_NAME,
)];

/// Resolves the one canonical tool assigned to an operational verification role.
pub fn tool_name_for_verification_role(
    role: ToolVerificationRole,
) -> Result<&'static str, ToolVerificationRoleResolutionError> {
    resolve_tool_name_for_verification_role(role, &TOOL_VERIFICATION_ROLE_ASSIGNMENTS)
}

fn resolve_tool_name_for_verification_role<'a>(
    role: ToolVerificationRole,
    assignments: &'a [(ToolVerificationRole, &'a str)],
) -> Result<&'a str, ToolVerificationRoleResolutionError> {
    let mut owners = assignments
        .iter()
        .filter_map(|(candidate_role, tool_name)| (*candidate_role == role).then_some(*tool_name));
    let owner = owners.next();
    let owner_count = usize::from(owner.is_some()) + owners.count();
    match (owner, owner_count) {
        (Some(tool_name), 1) => Ok(tool_name),
        _ => Err(ToolVerificationRoleResolutionError { role, owner_count }),
    }
}

/// MCP-visible method tools exposed through workflow connections.
pub const WORKFLOW_METHOD_TOOL_NAMES: [&str; 12] = [
    INTAKE_TOOL_NAME,
    UPDATE_SCOPE_TOOL_NAME,
    STATUS_TOOL_NAME,
    GET_OPERATION_RESULT_TOOL_NAME,
    PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
    PREPARE_WRITE_TOOL_NAME,
    STAGE_ARTIFACT_TOOL_NAME,
    RECORD_RUN_TOOL_NAME,
    REQUEST_USER_ACTION_TOOL_NAME,
    RECONCILE_CHANGES_TOOL_NAME,
    CHECK_CLOSE_TOOL_NAME,
    CLOSE_TASK_TOOL_NAME,
];

/// MCP-visible method tools exposed through read-only connections.
pub const READ_ONLY_METHOD_TOOL_NAMES: [&str; 3] = [
    STATUS_TOOL_NAME,
    GET_OPERATION_RESULT_TOOL_NAME,
    CHECK_CLOSE_TOOL_NAME,
];

/// Adapter-owned MCP utility tools that are not public Core methods.
pub const ADAPTER_UTILITY_TOOL_NAMES: [&str; 1] = [LIST_PROJECTS_TOOL_NAME];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_host_round_trip_role_has_exactly_one_canonical_owner() {
        assert_eq!(
            tool_name_for_verification_role(ToolVerificationRole::ManagedHostRoundTrip),
            Ok(LIST_PROJECTS_TOOL_NAME)
        );
    }

    #[test]
    fn verification_role_resolution_rejects_zero_owners() {
        assert_eq!(
            resolve_tool_name_for_verification_role(
                ToolVerificationRole::ManagedHostRoundTrip,
                &[]
            ),
            Err(ToolVerificationRoleResolutionError {
                role: ToolVerificationRole::ManagedHostRoundTrip,
                owner_count: 0,
            })
        );
    }

    #[test]
    fn verification_role_resolution_rejects_multiple_owners() {
        let assignments = [
            (
                ToolVerificationRole::ManagedHostRoundTrip,
                "volicord.list_projects",
            ),
            (
                ToolVerificationRole::ManagedHostRoundTrip,
                "volicord.status",
            ),
        ];
        assert_eq!(
            resolve_tool_name_for_verification_role(
                ToolVerificationRole::ManagedHostRoundTrip,
                &assignments
            ),
            Err(ToolVerificationRoleResolutionError {
                role: ToolVerificationRole::ManagedHostRoundTrip,
                owner_count: 2,
            })
        );
    }
}
