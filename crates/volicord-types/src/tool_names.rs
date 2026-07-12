use crate::values::MethodName;

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

/// MCP-visible tool name for `volicord.stage_artifact`.
pub const STAGE_ARTIFACT_TOOL_NAME: &str = MethodName::StageArtifact.as_str();

/// MCP-visible tool name for `volicord.record_run`.
pub const RECORD_RUN_TOOL_NAME: &str = MethodName::RecordRun.as_str();

/// MCP-visible tool name for `volicord.request_user_judgment`.
pub const REQUEST_USER_JUDGMENT_TOOL_NAME: &str = MethodName::RequestUserJudgment.as_str();

/// Public User Channel method name not exposed through Agent Connection MCP tool lists.
pub const RECORD_USER_JUDGMENT_TOOL_NAME: &str = MethodName::RecordUserJudgment.as_str();

/// MCP-visible tool name for `volicord.reconcile_changes`.
pub const RECONCILE_CHANGES_TOOL_NAME: &str = MethodName::ReconcileChanges.as_str();

/// MCP-visible tool name for `volicord.check_close`.
pub const CHECK_CLOSE_TOOL_NAME: &str = MethodName::CheckClose.as_str();

/// MCP-visible tool name for `volicord.close_task`.
pub const CLOSE_TASK_TOOL_NAME: &str = MethodName::CloseTask.as_str();

/// Adapter-owned project-list utility tool name.
pub const LIST_PROJECTS_TOOL_NAME: &str = "volicord.list_projects";

/// MCP-visible method tools exposed through workflow connections.
pub const WORKFLOW_METHOD_TOOL_NAMES: [&str; 11] = [
    INTAKE_TOOL_NAME,
    UPDATE_SCOPE_TOOL_NAME,
    STATUS_TOOL_NAME,
    GET_OPERATION_RESULT_TOOL_NAME,
    PREPARE_WRITE_TOOL_NAME,
    STAGE_ARTIFACT_TOOL_NAME,
    RECORD_RUN_TOOL_NAME,
    REQUEST_USER_JUDGMENT_TOOL_NAME,
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
