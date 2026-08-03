use crate::routing::{
    effective_tool_mode_for_mode_and_storage, list_projects_output_schema, McpEffectiveToolMode,
    McpStorageCapability,
};
use schemars::schema_for;
use serde::Serialize;
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use volicord_host_contract::{HostContractError, McpServerKey, McpToolCatalog};
use volicord_mcp_protocol::{McpProtocolCapabilities, ToolResultCarrier};
use volicord_mcp_wire::{
    mcp_request_schema, mcp_response_schema, McpToolAnnotations, McpToolDefinitionEnvelope,
    McpToolResultEnvelope, McpToolStructuredContent,
};
use volicord_types::integration_verification::{
    BeginIntegrationVerificationArguments, BeginIntegrationVerificationResult,
    GetIntegrationVerificationResult, GuardProbeResult, IntegrationVerificationIdArguments,
};
use volicord_types::tool_names::{AgentToolCategory, AgentToolId, AgentToolOwner};
use volicord_types::values::{AgentConnectionMode, MethodName};

pub(crate) fn method_name_for_tool(tool_name: &str) -> Option<MethodName> {
    AgentToolId::from_wire_name(tool_name).ok()?.method()
}

#[cfg(test)]
pub(crate) const MAX_RUNTIME_TOOLS_LIST_BYTES: usize = 48_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CanonicalToolDefinition {
    #[serde(rename = "name")]
    pub id: AgentToolId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'static str>,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Value,
    pub annotations: McpToolAnnotations,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalContent {
    Text(String),
}

impl CanonicalContent {
    fn to_wire_value(&self) -> Value {
        match self {
            Self::Text(text) => json!({
                "type": "text",
                "text": text,
            }),
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Text(text) => text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalToolResult {
    pub metadata: Option<Map<String, Value>>,
    pub content: Vec<CanonicalContent>,
    pub structured_content: Value,
    pub is_error: bool,
}

impl CanonicalToolDefinition {
    pub(crate) fn project(
        &self,
        capabilities: McpProtocolCapabilities,
    ) -> McpToolDefinitionEnvelope {
        let tool_capabilities = capabilities.tools();
        let mut projected = Map::from_iter([
            (
                "description".to_owned(),
                Value::String(self.description.to_owned()),
            ),
            ("inputSchema".to_owned(), self.input_schema.clone()),
            (
                "name".to_owned(),
                Value::String(self.id.wire_name().to_owned()),
            ),
        ]);
        if tool_capabilities.definition_metadata() {
            if let Some(metadata) = &self.metadata {
                projected.insert("_meta".to_owned(), Value::Object(metadata.clone()));
            }
        }
        if tool_capabilities.annotations() {
            projected.insert(
                "annotations".to_owned(),
                serde_json::to_value(self.annotations)
                    .expect("canonical tool annotations should serialize"),
            );
        }
        if tool_capabilities.output_schema() {
            projected.insert("outputSchema".to_owned(), self.output_schema.clone());
        }
        if tool_capabilities.title() {
            if let Some(title) = self.title {
                projected.insert("title".to_owned(), Value::String(title.to_owned()));
            }
        }
        McpToolDefinitionEnvelope::new(Value::Object(projected))
    }
}

/// Builds the collision-checked host catalog for the complete canonical MCP registry.
pub fn canonical_mcp_tool_catalog(
    server: &McpServerKey,
) -> Result<McpToolCatalog, HostContractError> {
    McpToolCatalog::for_server(server, AgentToolId::ALL)
}

/// Builds the collision-checked host catalog for an effective `tools/list` projection.
pub fn effective_mcp_tool_catalog(
    server: &McpServerKey,
    tools: &[CanonicalToolDefinition],
) -> Result<McpToolCatalog, HostContractError> {
    McpToolCatalog::for_server(server, tools.iter().map(|tool| tool.id))
}

impl CanonicalToolResult {
    pub(crate) fn project(
        &self,
        capabilities: McpProtocolCapabilities,
    ) -> Result<McpToolResultEnvelope, serde_json::Error> {
        let tool_capabilities = capabilities.tools();
        let mut projected = Map::new();

        if tool_capabilities.result_metadata() {
            if let Some(metadata) = &self.metadata {
                projected.insert("_meta".to_owned(), Value::Object(metadata.clone()));
            }
        }

        match tool_capabilities.result_carrier() {
            ToolResultCarrier::DirectToolResult => {
                projected.insert("toolResult".to_owned(), self.structured_content.clone());
            }
            ToolResultCarrier::JsonTextContent => {
                let authoritative_text = serde_json::to_string(&self.structured_content)?;
                let mut content = vec![json!({
                    "type": "text",
                    "text": authoritative_text,
                })];
                content.extend(
                    self.content
                        .iter()
                        .filter(|item| item.text() != authoritative_text)
                        .map(CanonicalContent::to_wire_value),
                );
                projected.insert("content".to_owned(), Value::Array(content));
            }
            ToolResultCarrier::StructuredContentWithText => {
                projected.insert(
                    "content".to_owned(),
                    Value::Array(
                        self.content
                            .iter()
                            .map(CanonicalContent::to_wire_value)
                            .collect(),
                    ),
                );
                projected.insert(
                    "structuredContent".to_owned(),
                    self.structured_content.clone(),
                );
            }
        }
        if tool_capabilities.is_error() {
            projected.insert("isError".to_owned(), Value::Bool(self.is_error));
        }

        Ok(McpToolResultEnvelope::new(Value::Object(projected)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolSchemaDetail {
    RuntimeCompact,
    Documentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct McpToolExample {
    pub id: &'static str,
    pub description: &'static str,
    pub arguments_json: &'static str,
}

const INTAKE_CREATE_NEW_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Create an onboarding checklist.","requested_mode":"work","resume_policy":"create_new","acceptance_policy":null,"lineage":null,"initial_scope":{"boundary":"Onboarding checklist setup.","non_goals":[],"acceptance_criteria":[{"statement":"The checklist is available to new workspace users.","evidence_requirement":"required"}]}}"#;
const INTAKE_RESUME_ACTIVE_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Continue the active onboarding checklist work.","requested_mode":"auto","resume_policy":"resume_active","acceptance_policy":null,"lineage":null,"initial_scope":{"boundary":"Continue the current onboarding checklist scope.","non_goals":[],"acceptance_criteria":[]}}"#;
const INTAKE_SUPERSEDE_ACTIVE_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Replace the active onboarding work with the revised checklist.","requested_mode":"work","resume_policy":"supersede_active","acceptance_policy":null,"lineage":null,"initial_scope":{"boundary":"Revised onboarding checklist setup.","non_goals":["Changing account creation."],"acceptance_criteria":[{"statement":"The revised checklist replaces the active work.","evidence_requirement":"required"}]}}"#;
const INTAKE_REJECT_IF_ACTIVE_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Start an onboarding checklist only when no Task is active.","requested_mode":"advisor","resume_policy":"reject_if_active","acceptance_policy":null,"lineage":null,"initial_scope":{"boundary":"Onboarding checklist guidance.","non_goals":[],"acceptance_criteria":[{"statement":"Provide onboarding checklist guidance.","evidence_requirement":"not_required"}]}}"#;

pub(crate) const UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID: &str = "keep_current_change_unit";
pub(crate) const UPDATE_SCOPE_KEEP_CURRENT_ARGUMENTS_JSON: &str =
    r#"{"task_id":"task_filter_001","change_unit":{"operation":"keep_current"}}"#;
const UPDATE_SCOPE_CREATE_CURRENT_ARGUMENTS_JSON: &str = r#"{"task_id":"task_filter_002","goal_summary":"Limit saved search filters.","scope_update":{"include":["Saved-filter owner and label edits."],"exclude":[]},"scope_boundary":"Saved-filter owner and label edits.","acceptance_criteria":[{"acceptance_criterion_id":null,"statement":"Saved filters reject out-of-scope edits.","evidence_requirement":"required"}],"baseline_ref":"baseline_filter_002","change_unit":{"operation":"create_current","scope_summary":"Saved-filter validation.","affected_paths":["src/search/saved-filters.ts"]}}"#;
const UPDATE_SCOPE_REPLACE_CURRENT_ARGUMENTS_JSON: &str = r#"{"task_id":"task_filter_003","scope_boundary":"Saved-filter owner, label, and visibility edits.","baseline_ref":"baseline_filter_003","change_unit":{"operation":"replace_current","scope_summary":"Expanded saved-filter validation.","affected_paths":["src/search/saved-filters.ts"]}}"#;

const RECORD_SHAPING_ARGUMENTS_JSON: &str = r#"{"task_id":"task_shape_001","checkpoint_operation":{"operation":"create_initial"},"scope_revision":4,"baseline_ref":"baseline_shape_001","summary":"The implementation boundary and open decisions are recorded.","implementation_boundary":"Implement only the current saved-filter scope.","gaps":[],"source_refs":[],"evidence_refs":[],"close_assessment":null}"#;
const ADVANCE_TASK_ARGUMENTS_JSON: &str = r#"{"task_id":"task_shape_001","shaping_checkpoint_id":"shaping_checkpoint_001","change_unit_id":"change_unit_001","scope_revision":4,"baseline_ref":"baseline_shape_001","user_action_resolution_ids":[]}"#;

pub(crate) const STATUS_READ_ONLY_EXAMPLE_ID: &str = "read_only_status";
const STATUS_SUMMARY_ARGUMENTS_JSON: &str = r#"{"detail":"summary"}"#;
pub(crate) const STATUS_READ_ONLY_ARGUMENTS_JSON: &str = r#"{"detail":"workflow"}"#;
const STATUS_FULL_ARGUMENTS_JSON: &str = r#"{"detail":"full"}"#;

pub(crate) const GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID: &str = "first_operation_result_page";
const GET_OPERATION_RESULT_FIRST_PAGE_ARGUMENTS_JSON: &str = r#"{"operation_result_ref":{"project_id":"proj_history_001","source_method":"volicord.record_run","source_idempotency_key":"idem_run_history_001","committed_state_version":42,"response_sha256":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","response_size_bytes":32768}}"#;

pub(crate) const PREPARE_WRITE_SIMPLE_EXAMPLE_ID: &str = "simple_prepare_write";
pub(crate) const PREPARE_WRITE_SIMPLE_ARGUMENTS_JSON: &str = r#"{"detail":"full","intended_operation":"Update the profile preference save flow.","intended_paths":["src/preferences/profile-save.ts"],"product_file_write_intended":true,"baseline_ref":"baseline_pref_001"}"#;

pub(crate) const PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID: &str =
    "verified_command_capture";
pub(crate) const PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_ARGUMENTS_JSON: &str = r#"{"task_id":"task_capture_001","change_unit_id":"cu_capture_001","baseline_ref":"baseline_capture_001","target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_capture_001"},"capture":{"capture_kind":"verified_command_execution","command_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","command_label":"Focused validation"}}"#;
pub(crate) const PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID: &str = "verified_tool_capture";
const PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_ARGUMENTS_JSON: &str = r#"{"task_id":"task_capture_001","change_unit_id":"cu_capture_001","baseline_ref":"baseline_capture_001","target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_capture_001"},"capture":{"capture_kind":"verified_tool_invocation","tool_name":"example.validate","tool_input_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}"#;

const STAGE_ARTIFACT_SAFE_TEXT_ARGUMENTS_JSON: &str = r#"{"detail":"full","task_id":"task_trace_001","display_name":"diagnostic_trace.log","content_type":"text/plain","redaction_state":"none","safe_bytes_or_notice":"Local trace sample captured for debugging."}"#;

pub(crate) const RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID: &str = "evidence_bearing_record_run";
pub(crate) const RECORD_RUN_EVIDENCE_BEARING_ARGUMENTS_JSON: &str = r#"{"task_id":"task_run_002","change_unit_id":"cu_run_002","kind":"implementation","baseline_ref":"baseline_run_002","summary":"Saved-filter validation reviewed.","observed_changes":{"changed_paths":[],"product_file_write_observed":false,"sensitive_categories":[],"baseline_ref":"baseline_run_002"},"evidence_updates":[{"target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_saved_filter_001"},"coverage_state":"supported"}],"evidence_observations":[{"target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_saved_filter_001"},"source_kind":"agent_report","assurance_level":"cooperative_report","observed_at":"2026-07-12T00:00:00Z"}],"close_assessment":{"result_summary":"Saved-filter validation reviewed.","result_refs":[],"residual_risks":[],"sensitive_categories":[],"recovery_constraints":[]}}"#;

pub(crate) const REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID: &str = "final_acceptance_request";
pub(crate) const REQUEST_USER_ACTION_FINAL_ACCEPTANCE_ARGUMENTS_JSON: &str = r#"{"request":{"operation":"create","task_id":"task_close_001","change_unit_id":null,"action":{"action_type":"choice","judgment_kind":"final_acceptance","presentation":"short","question":"Do you accept this result as complete?","options":null,"context":{"summary":"Review the current close basis and decide final acceptance.","related_refs":[],"artifact_refs":[],"visible_risks":[],"constraints":["Only final acceptance for the current close basis is in scope."]},"affected_refs":[],"sensitive_action_scope":null},"required_for":["close_complete"],"expires_at":null}}"#;
const REQUEST_USER_ACTION_RESUME_ARGUMENTS_JSON: &str =
    r#"{"request":{"operation":"resume","user_action_request_id":"uact_existing_001"}}"#;

const RECONCILE_CHANGES_ARGUMENTS_JSON: &str =
    r#"{"detail":"full","task_id":"task_reconcile_001"}"#;

pub(crate) const CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID: &str =
    "check_close_missing_final_acceptance";
pub(crate) const CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_ARGUMENTS_JSON: &str =
    r#"{"task_id":"task_close_001"}"#;

const CLOSE_TASK_COMPLETE_ARGUMENTS_JSON: &str =
    r#"{"task_id":"task_close_001","intent":"complete","close_reason":"completed_self_checked"}"#;
const CLOSE_TASK_CANCEL_ARGUMENTS_JSON: &str =
    r#"{"task_id":"task_cancel_001","intent":"cancel","close_reason":"cancelled"}"#;
const CLOSE_TASK_SUPERSEDE_ARGUMENTS_JSON: &str = r#"{"task_id":"task_supersede_001","intent":"supersede","close_reason":"superseded","superseding_task_id":"task_replacement_001"}"#;

const INTAKE_EXAMPLES: [McpToolExample; 4] = [
    McpToolExample {
        id: "create_new",
        description: "Create a new Task when no active Task should be resumed.",
        arguments_json: INTAKE_CREATE_NEW_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "resume_active",
        description: "Resume the active Task.",
        arguments_json: INTAKE_RESUME_ACTIVE_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "supersede_active",
        description: "Supersede the active Task with revised work.",
        arguments_json: INTAKE_SUPERSEDE_ACTIVE_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "reject_if_active",
        description: "Reject intake when a Task is already active.",
        arguments_json: INTAKE_REJECT_IF_ACTIVE_ARGUMENTS_JSON,
    },
];

const UPDATE_SCOPE_EXAMPLES: [McpToolExample; 3] = [
    McpToolExample {
        id: UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
        description: "Keep the current Change Unit and leave omitted scope fields unchanged.",
        arguments_json: UPDATE_SCOPE_KEEP_CURRENT_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "create_current_change_unit",
        description: "Create a current Change Unit for the updated scope.",
        arguments_json: UPDATE_SCOPE_CREATE_CURRENT_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "replace_current_change_unit",
        description: "Replace the current Change Unit for revised scope.",
        arguments_json: UPDATE_SCOPE_REPLACE_CURRENT_ARGUMENTS_JSON,
    },
];

const RECORD_SHAPING_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: "record_current_shaping",
    description: "Record the current shaping checkpoint.",
    arguments_json: RECORD_SHAPING_ARGUMENTS_JSON,
}];

const ADVANCE_TASK_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: "enter_implementation",
    description: "Advance one ready work Task into implementation.",
    arguments_json: ADVANCE_TASK_ARGUMENTS_JSON,
}];

const STATUS_EXAMPLES: [McpToolExample; 3] = [
    McpToolExample {
        id: "summary_status",
        description: "Read the compact status summary.",
        arguments_json: STATUS_SUMMARY_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: STATUS_READ_ONLY_EXAMPLE_ID,
        description: "Read the normal workflow status view.",
        arguments_json: STATUS_READ_ONLY_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "full_status",
        description: "Read the full status view including continuity detail.",
        arguments_json: STATUS_FULL_ARGUMENTS_JSON,
    },
];

const GET_OPERATION_RESULT_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID,
    description: "Read the first bounded page of one immutable historical mutation response.",
    arguments_json: GET_OPERATION_RESULT_FIRST_PAGE_ARGUMENTS_JSON,
}];

const PREPARE_WRITE_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
    description: "Check one Product Repository write intent.",
    arguments_json: PREPARE_WRITE_SIMPLE_ARGUMENTS_JSON,
}];

const PREPARE_EVIDENCE_CAPTURE_EXAMPLES: [McpToolExample; 2] = [
    McpToolExample {
        id: PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
        description: "Create an intent for a registered command evidence source.",
        arguments_json: PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID,
        description: "Create an intent for an exact registered tool invocation.",
        arguments_json: PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_ARGUMENTS_JSON,
    },
];

const STAGE_ARTIFACT_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: "stage_safe_text",
    description: "Stage a text attachment input.",
    arguments_json: STAGE_ARTIFACT_SAFE_TEXT_ARGUMENTS_JSON,
}];

const RECORD_RUN_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
    description: "Record target-scoped evidence and a close assessment.",
    arguments_json: RECORD_RUN_EVIDENCE_BEARING_ARGUMENTS_JSON,
}];

const REQUEST_USER_ACTION_EXAMPLES: [McpToolExample; 2] = [
    McpToolExample {
        id: REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
        description: "Create final acceptance through the common user-action model.",
        arguments_json: REQUEST_USER_ACTION_FINAL_ACCEPTANCE_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "resume_user_action",
        description:
            "Resume the original exact Agent Connection result after a later CLI inbox resolution.",
        arguments_json: REQUEST_USER_ACTION_RESUME_ARGUMENTS_JSON,
    },
];

const RECONCILE_CHANGES_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: "reconcile_current_task",
    description: "Reconcile the current Task without an agent-supplied resolution request.",
    arguments_json: RECONCILE_CHANGES_ARGUMENTS_JSON,
}];

const CHECK_CLOSE_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID,
    description: "Read current close readiness for one Task.",
    arguments_json: CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_ARGUMENTS_JSON,
}];

const CLOSE_TASK_EXAMPLES: [McpToolExample; 3] = [
    McpToolExample {
        id: "close_complete",
        description: "Request the completion close path.",
        arguments_json: CLOSE_TASK_COMPLETE_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "close_cancel",
        description: "Request the cancellation close path.",
        arguments_json: CLOSE_TASK_CANCEL_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "close_supersede",
        description: "Request the supersession close path.",
        arguments_json: CLOSE_TASK_SUPERSEDE_ARGUMENTS_JSON,
    },
];

pub(crate) fn canonical_tool_examples(tool: AgentToolId) -> &'static [McpToolExample] {
    match tool.method() {
        Some(MethodName::Intake) => &INTAKE_EXAMPLES,
        Some(MethodName::UpdateScope) => &UPDATE_SCOPE_EXAMPLES,
        Some(MethodName::RecordShaping) => &RECORD_SHAPING_EXAMPLES,
        Some(MethodName::AdvanceTask) => &ADVANCE_TASK_EXAMPLES,
        Some(MethodName::Status) => &STATUS_EXAMPLES,
        Some(MethodName::GetOperationResult) => &GET_OPERATION_RESULT_EXAMPLES,
        Some(MethodName::PrepareEvidenceCapture) => &PREPARE_EVIDENCE_CAPTURE_EXAMPLES,
        Some(MethodName::PrepareWrite) => &PREPARE_WRITE_EXAMPLES,
        Some(MethodName::StageArtifact) => &STAGE_ARTIFACT_EXAMPLES,
        Some(MethodName::RecordRun) => &RECORD_RUN_EXAMPLES,
        Some(MethodName::RequestUserAction) => &REQUEST_USER_ACTION_EXAMPLES,
        Some(MethodName::ReconcileChanges) => &RECONCILE_CHANGES_EXAMPLES,
        Some(MethodName::CheckClose) => &CHECK_CLOSE_EXAMPLES,
        Some(MethodName::CloseTask) => &CLOSE_TASK_EXAMPLES,
        None | Some(MethodName::ResolveUserAction) => &[],
    }
}

pub fn public_method_tools() -> Vec<CanonicalToolDefinition> {
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| matches!(tool.owner(), AgentToolOwner::CoreMethod(_))),
        ToolSchemaDetail::Documentation,
    )
}

/// Returns adapter utility tool definitions.
pub fn adapter_utility_tools() -> Vec<CanonicalToolDefinition> {
    adapter_utility_tools_with_detail(ToolSchemaDetail::Documentation)
}

fn adapter_utility_tools_with_detail(detail: ToolSchemaDetail) -> Vec<CanonicalToolDefinition> {
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| matches!(tool.owner(), AgentToolOwner::AdapterUtility)),
        detail,
    )
}

/// Returns workflow-mode MCP-visible tools.
pub fn mcp_tools() -> Vec<CanonicalToolDefinition> {
    mcp_tools_for_mode(AgentConnectionMode::Workflow)
}

/// Returns MCP-visible tools for the supplied Agent Connection mode.
pub fn mcp_tools_for_mode(mode: AgentConnectionMode) -> Vec<CanonicalToolDefinition> {
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| tool.available_in(mode)),
        ToolSchemaDetail::Documentation,
    )
}

/// Returns MCP-visible tools for the effective connection and storage capability.
#[cfg(test)]
pub(crate) fn mcp_tools_for_mode_and_storage(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
) -> Vec<CanonicalToolDefinition> {
    mcp_tools_for_mode_and_storage_with_detail(
        mode,
        storage_capability,
        ToolSchemaDetail::Documentation,
    )
}

pub(crate) fn mcp_tools_for_mode_and_storage_with_detail(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
    detail: ToolSchemaDetail,
) -> Vec<CanonicalToolDefinition> {
    let effective_mode = effective_tool_mode_for_mode_and_storage(mode, storage_capability);
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| match effective_mode {
                McpEffectiveToolMode::Unavailable => {
                    matches!(tool.owner(), AgentToolOwner::AdapterUtility)
                }
                McpEffectiveToolMode::ReadOnly => tool.available_in(AgentConnectionMode::ReadOnly),
                McpEffectiveToolMode::ReadOnlyDegraded => {
                    matches!(tool.category(), AgentToolCategory::ReadOnly)
                        || matches!(tool.owner(), AgentToolOwner::ConnectionIntegration)
                        || *tool == AgentToolId::REQUEST_USER_ACTION
                }
                McpEffectiveToolMode::Workflow => tool.available_in(AgentConnectionMode::Workflow),
            }),
        detail,
    )
}

pub(crate) fn tools_list_schema_validation_status(
    tools: &[CanonicalToolDefinition],
) -> &'static str {
    if validate_tools_list_schema_compatibility(tools).is_ok() {
        "passed"
    } else {
        "failed"
    }
}

pub(crate) fn mcp_tool_naming_style(tools: &[CanonicalToolDefinition]) -> &'static str {
    if tools.is_empty() {
        return "empty";
    }
    if tools.iter().all(|tool| tool.id.wire_name().contains('.')) {
        "dotted_namespace"
    } else if tools.iter().all(|tool| !tool.id.wire_name().contains('.')) {
        "plain"
    } else {
        "mixed"
    }
}

pub(crate) fn validate_tools_list_schema_compatibility(
    tools: &[CanonicalToolDefinition],
) -> Result<(), Vec<String>> {
    let values = tools
        .iter()
        .map(|tool| serde_json::to_value(tool).expect("tool definition should serialize"))
        .collect::<Vec<_>>();
    validate_tools_list_json_compatibility(&values)
}

pub(crate) fn validate_tools_list_json_compatibility(tools: &[Value]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut names = BTreeSet::new();

    for (index, tool) in tools.iter().enumerate() {
        let Some(object) = tool.as_object() else {
            errors.push(format!("tool[{index}] is not an object"));
            continue;
        };

        let Some(name) = object.get("name").and_then(Value::as_str) else {
            errors.push(format!("tool[{index}].name is not a string"));
            continue;
        };
        if !valid_mcp_tool_name(name) {
            errors.push(format!("tool `{name}` has an MCP-incompatible name"));
        }
        if !names.insert(name.to_owned()) {
            errors.push(format!("tool `{name}` is duplicated"));
        }
        if object
            .get("description")
            .is_none_or(|description| description.as_str().is_none_or(|text| text.is_empty()))
        {
            errors.push(format!("tool `{name}` description is missing or empty"));
        }

        match object.get("inputSchema") {
            Some(input_schema) => {
                validate_root_object_schema(name, "inputSchema", input_schema, &mut errors)
            }
            None => errors.push(format!("tool `{name}` is missing inputSchema")),
        }
        match object.get("outputSchema") {
            Some(output_schema) => {
                validate_root_object_schema(name, "outputSchema", output_schema, &mut errors)
            }
            None => errors.push(format!("tool `{name}` is missing outputSchema")),
        }
        match object.get("annotations") {
            Some(annotations) => validate_annotations(name, annotations, &mut errors),
            None => errors.push(format!("tool `{name}` is missing annotations")),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn tool_definitions(
    tools: impl IntoIterator<Item = AgentToolId>,
    detail: ToolSchemaDetail,
) -> Vec<CanonicalToolDefinition> {
    tools
        .into_iter()
        .map(|id| CanonicalToolDefinition {
            id,
            title: None,
            description: tool_description(id, detail),
            input_schema: mcp_tool_input_schema_with_detail(id, detail)
                .expect("MCP tool schema should exist"),
            output_schema: match detail {
                ToolSchemaDetail::RuntimeCompact => compact_output_schema(),
                ToolSchemaDetail::Documentation => match id.owner() {
                    AgentToolOwner::CoreMethod(_) => {
                        mcp_response_schema(id).expect("MCP tool response schema should exist")
                    }
                    AgentToolOwner::AdapterUtility => list_projects_output_schema(),
                    AgentToolOwner::ConnectionIntegration => {
                        integration_verification_output_schema(id)
                    }
                },
            },
            annotations: tool_annotations(id),
            metadata: None,
        })
        .collect()
}

pub(crate) fn compact_output_schema() -> Value {
    json!({ "type": "object" })
}

pub(crate) fn mcp_tool_output_schema(name: &str) -> Option<Value> {
    AgentToolId::from_wire_name(name)
        .ok()
        .map(|_| compact_output_schema())
}

pub(crate) fn mcp_tool_input_schema(name: &str) -> Option<Value> {
    let tool = AgentToolId::from_wire_name(name).ok()?;
    mcp_tool_input_schema_with_detail(tool, ToolSchemaDetail::Documentation)
}

fn mcp_tool_input_schema_with_detail(tool: AgentToolId, detail: ToolSchemaDetail) -> Option<Value> {
    let mut schema = match tool.owner() {
        AgentToolOwner::AdapterUtility => json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        AgentToolOwner::ConnectionIntegration => integration_verification_input_schema(tool),
        AgentToolOwner::CoreMethod(_) => mcp_request_schema(tool)?,
    };
    match detail {
        ToolSchemaDetail::RuntimeCompact => compact_runtime_schema(&mut schema),
        ToolSchemaDetail::Documentation => {
            let examples = canonical_tool_examples(tool)
                .iter()
                .map(|example| {
                    serde_json::from_str(example.arguments_json)
                        .expect("canonical MCP tool example should be valid JSON")
                })
                .collect::<Vec<Value>>();
            if !examples.is_empty() {
                schema
                    .as_object_mut()
                    .expect("MCP tool input schema should be an object")
                    .insert("examples".to_owned(), Value::Array(examples));
            }
        }
    }
    Some(schema)
}

pub(crate) fn compact_runtime_schema(schema: &mut Value) {
    // Keep the draft marker and validation semantics. Runtime compaction
    // removes annotations and redundant constraints, drops unreachable
    // definitions, and rewrites only local definition references.
    strip_schema_presentation_annotations(schema);
    prune_unreferenced_definitions(schema);
    inline_single_use_definitions(schema);
    compact_definition_names(schema);
}

fn strip_schema_presentation_annotations(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    for annotation in [
        "$comment",
        "default",
        "deprecated",
        "description",
        "examples",
        "readOnly",
        "title",
        "writeOnly",
    ] {
        object.remove(annotation);
    }
    if enum_makes_type_redundant(object) {
        object.remove("type");
    }

    for keyword in [
        "additionalItems",
        "additionalProperties",
        "contains",
        "contentSchema",
        "else",
        "if",
        "not",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    ] {
        if let Some(child) = object.get_mut(keyword) {
            strip_schema_presentation_annotations(child);
        }
    }
    if let Some(items) = object.get_mut("items") {
        match items {
            Value::Array(items) => {
                for item in items {
                    strip_schema_presentation_annotations(item);
                }
            }
            item => strip_schema_presentation_annotations(item),
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(items) = object.get_mut(keyword).and_then(Value::as_array_mut) {
            for item in items {
                strip_schema_presentation_annotations(item);
            }
        }
    }
    for keyword in [
        "$defs",
        "definitions",
        "dependentSchemas",
        "patternProperties",
        "properties",
    ] {
        if let Some(children) = object.get_mut(keyword).and_then(Value::as_object_mut) {
            for child in children.values_mut() {
                strip_schema_presentation_annotations(child);
            }
        }
    }
    if let Some(dependencies) = object
        .get_mut("dependencies")
        .and_then(Value::as_object_mut)
    {
        for dependency in dependencies.values_mut() {
            if dependency.is_object() {
                strip_schema_presentation_annotations(dependency);
            }
        }
    }
}

fn enum_makes_type_redundant(schema: &Map<String, Value>) -> bool {
    let Some(values) = schema.get("enum").and_then(Value::as_array) else {
        return false;
    };
    if values.is_empty() {
        return false;
    }
    let schema_types = match schema.get("type") {
        Some(Value::String(schema_type)) if recognized_schema_type(schema_type) => {
            vec![schema_type.as_str()]
        }
        Some(Value::Array(schema_types)) => {
            let schema_type_names = schema_types
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            if schema_type_names.len() != schema_types.len() {
                return false;
            }
            schema_type_names
        }
        _ => return false,
    };
    !schema_types.is_empty()
        && schema_types
            .iter()
            .all(|schema_type| recognized_schema_type(schema_type))
        && values.iter().all(|value| {
            schema_types
                .iter()
                .any(|schema_type| value_matches_schema_type(value, schema_type))
        })
}

fn recognized_schema_type(schema_type: &str) -> bool {
    matches!(
        schema_type,
        "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
    )
}

fn value_matches_schema_type(value: &Value, schema_type: &str) -> bool {
    match schema_type {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => false,
    }
}

fn prune_unreferenced_definitions(schema: &mut Value) {
    let Some(definitions) = schema
        .get("definitions")
        .and_then(Value::as_object)
        .cloned()
    else {
        return;
    };
    let mut pending = Vec::new();
    if let Some(root) = schema.as_object() {
        for (keyword, child) in root {
            if keyword != "definitions" {
                collect_definition_refs(child, &mut pending);
            }
        }
    }
    let mut reachable = BTreeSet::new();
    let mut index = 0;
    while index < pending.len() {
        let name = pending[index].clone();
        index += 1;
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(definition) = definitions.get(&name) {
            collect_definition_refs(definition, &mut pending);
        }
    }

    if let Some(definitions) = schema
        .as_object_mut()
        .and_then(|object| object.get_mut("definitions"))
        .and_then(Value::as_object_mut)
    {
        definitions.retain(|name, _| reachable.contains(name));
    }
    remove_empty_definitions(schema);
}

fn collect_definition_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
            {
                refs.push(name.to_owned());
            }
            for child in object.values() {
                collect_definition_refs(child, refs);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_definition_refs(child, refs);
            }
        }
        _ => {}
    }
}

fn inline_single_use_definitions(schema: &mut Value) {
    loop {
        let Some(definitions) = schema
            .get("definitions")
            .and_then(Value::as_object)
            .cloned()
        else {
            return;
        };
        let mut counts = BTreeMap::<String, usize>::new();
        count_definition_refs(schema, &mut counts);
        let candidate = definitions
            .iter()
            .find(|(name, definition)| {
                counts.get(*name).copied() == Some(1)
                    && !value_references_definition(definition, name)
            })
            .map(|(name, definition)| (name.clone(), definition.clone()));
        let Some((name, definition)) = candidate else {
            break;
        };

        let replaced = replace_one_definition_ref(schema, &name, &definition);
        debug_assert!(replaced);
        if !replaced {
            break;
        }
        if let Some(definitions) = schema
            .as_object_mut()
            .and_then(|object| object.get_mut("definitions"))
            .and_then(Value::as_object_mut)
        {
            definitions.remove(&name);
        }
    }
    remove_empty_definitions(schema);
}

fn count_definition_refs(value: &Value, counts: &mut BTreeMap<String, usize>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
            {
                *counts.entry(name.to_owned()).or_default() += 1;
            }
            for child in object.values() {
                count_definition_refs(child, counts);
            }
        }
        Value::Array(items) => {
            for child in items {
                count_definition_refs(child, counts);
            }
        }
        _ => {}
    }
}

fn value_references_definition(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
                == Some(name)
                || object
                    .values()
                    .any(|child| value_references_definition(child, name))
        }
        Value::Array(items) => items
            .iter()
            .any(|child| value_references_definition(child, name)),
        _ => false,
    }
}

fn replace_one_definition_ref(value: &mut Value, name: &str, definition: &Value) -> bool {
    match value {
        Value::Object(object) => {
            if object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
                == Some(name)
            {
                *value = definition.clone();
                return true;
            }
            for child in object.values_mut() {
                if replace_one_definition_ref(child, name, definition) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => {
            for child in items {
                if replace_one_definition_ref(child, name, definition) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn compact_definition_names(schema: &mut Value) {
    let names = schema
        .get("definitions")
        .and_then(Value::as_object)
        .map(|definitions| definitions.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let aliases = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, base36(index)))
        .collect::<BTreeMap<_, _>>();
    replace_definition_refs(schema, &aliases);

    let Some(definitions) = schema
        .as_object_mut()
        .and_then(|object| object.get_mut("definitions"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let original = std::mem::take(definitions);
    for (name, definition) in original {
        definitions.insert(
            aliases
                .get(&name)
                .expect("every retained definition should have a compact alias")
                .clone(),
            definition,
        );
    }
}

fn replace_definition_refs(value: &mut Value, aliases: &BTreeMap<String, String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                if let Some(name) = definition_name_from_ref(reference) {
                    if let Some(alias) = aliases.get(name) {
                        *reference = format!("#/definitions/{alias}");
                    }
                }
            }
            for child in object.values_mut() {
                replace_definition_refs(child, aliases);
            }
        }
        Value::Array(items) => {
            for child in items {
                replace_definition_refs(child, aliases);
            }
        }
        _ => {}
    }
}

fn definition_name_from_ref(reference: &str) -> Option<&str> {
    reference.strip_prefix("#/definitions/")
}

fn remove_empty_definitions(schema: &mut Value) {
    if schema
        .get("definitions")
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty)
    {
        schema
            .as_object_mut()
            .expect("generated schema should be an object")
            .remove("definitions");
    }
}

fn base36(mut value: usize) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let mut digits = Vec::new();
    while value > 0 {
        let digit = value % 36;
        digits.push(if digit < 10 {
            char::from(b'0' + digit as u8)
        } else {
            char::from(b'a' + (digit - 10) as u8)
        });
        value /= 36;
    }
    digits.iter().rev().collect()
}

fn tool_annotations(tool: AgentToolId) -> McpToolAnnotations {
    let mut annotations = match tool.category() {
        AgentToolCategory::ReadOnly => McpToolAnnotations::read_only(),
        AgentToolCategory::NonDestructiveMutation => McpToolAnnotations::non_destructive_mutation(),
        AgentToolCategory::DestructiveMutation => McpToolAnnotations::destructive_mutation(),
    };
    if tool.is_idempotent() {
        annotations.idempotent_hint = true;
    }
    annotations
}

pub(crate) fn tool_description(tool: AgentToolId, detail: ToolSchemaDetail) -> &'static str {
    match (detail, tool) {
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::INTAKE) => {
            "Start or resume work and return its authority state."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::UPDATE_SCOPE) => {
            "Update Task scope and Change Unit before more work."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::RECORD_SHAPING) => {
            "Record a shaping checkpoint and any linked user decisions."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::ADVANCE_TASK) => {
            "Explicitly advance ready work from shaping to implementation."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::STATUS) => {
            "Refresh unknown Task authority, blockers, and next action."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::GET_OPERATION_RESULT) => {
            "Read one bounded immutable mutation-result page."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::PREPARE_EVIDENCE_CAPTURE) => {
            "Before capture, register an evidence intent; this records no Evidence."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::PREPARE_WRITE) => {
            "Before editing, check Product Repository paths and get a write decision."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::STAGE_ARTIFACT) => {
            "Stage an Evidence attachment; staging records no Evidence."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::RECORD_RUN) => {
            "After work, record its Run, changes, and evidence."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::REQUEST_USER_ACTION) => {
            "Create or resume one user action; complete pending requests through `volicord inbox`."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::RECONCILE_CHANGES) => {
            "Reconcile unresolved Product Repository changes with current authority."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::CHECK_CLOSE) => {
            "Read close readiness without requesting a terminal change."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::CLOSE_TASK) => {
            "Request completion, cancellation, or supersession to end the Task."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::LIST_PROJECTS) => {
            "List projects available to this MCP connection."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::BEGIN_INTEGRATION_VERIFICATION) => {
            "Begin or resume an in-chat MCP and Guard verification for this managed turn."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::GUARD_PROBE) => {
            "Acknowledge the exact probe observed by Guard pre-tool and post-tool hooks."
        }
        (ToolSchemaDetail::RuntimeCompact, AgentToolId::GET_INTEGRATION_VERIFICATION) => {
            "Read the exact correlated in-chat integration verification."
        }
        (_, AgentToolId::INTAKE) => {
            "Start, resume, supersede, or reject an ordinary user work loop."
        }
        (_, AgentToolId::UPDATE_SCOPE) => {
            "Update the current Task scope and keep, create, or replace its current Change Unit."
        }
        (_, AgentToolId::RECORD_SHAPING) => {
            "Atomically record the current shaping checkpoint, typed gaps, and linked UserAction requests."
        }
        (_, AgentToolId::ADVANCE_TASK) => {
            "Advance an exact ready work Task checkpoint and current Change Unit into implementation."
        }
        (_, AgentToolId::STATUS) => {
            "Read the current Core status view without creating Core authority state."
        }
        (_, AgentToolId::GET_OPERATION_RESULT) => {
            "Read one bounded page of an immutable historical mutation response; read current status separately."
        }
        (_, AgentToolId::PREPARE_EVIDENCE_CAPTURE) => {
            "Create a short-lived, current-basis intent for a registered evidence source. This does not execute the source or record Evidence."
        }
        (_, AgentToolId::PREPARE_WRITE) => {
            "Check a proposed Product Repository write against current Core scope. The default result includes the decision and any issued write ticket."
        }
        (_, AgentToolId::STAGE_ARTIFACT) => {
            "Prepare an Evidence attachment input; staging alone is not recorded Evidence. The default compact result includes the staged handle and expiry."
        }
        (_, AgentToolId::RECORD_RUN) => {
            "Record execution and evidence. Mode/kind: direct/direct or work/implementation."
        }
        (_, AgentToolId::REQUEST_USER_ACTION) => {
            "Create or resume one focused user action. MCP returns only a bounded pending summary; user-owned delivery and resolution use `volicord inbox`."
        }
        (_, AgentToolId::RECONCILE_CHANGES) => {
            "Reconcile unresolved Product Repository changes without agent-only dismissal. The default result includes per-finding outcomes."
        }
        (_, AgentToolId::CHECK_CLOSE) => {
            "Read current close readiness without requesting a terminal mutation."
        }
        (_, AgentToolId::CLOSE_TASK) => {
            "Request the complete, cancel, or supersede terminal path for one Task."
        }
        (_, AgentToolId::LIST_PROJECTS) => {
            "List projects explicitly allowed for this MCP connection."
        }
        (_, AgentToolId::BEGIN_INTEGRATION_VERIFICATION) => {
            "Create or resume the one immutable integration-verification attempt for the current semantic coordinate; returns the authoritative tagged workflow state and its exact typed operation."
        }
        (_, AgentToolId::GUARD_PROBE) => {
            "Record or replay a first-write-wins MCP probe acknowledgement and return the authoritative tagged workflow state without changing Product Repository workflow state; this exact call is observed by Guard PreToolUse and PostToolUse."
        }
        (_, AgentToolId::GET_INTEGRATION_VERIFICATION) => {
            "Observe the authoritative tagged workflow state under the semantic host policy; the bounded read may persist a typed terminal repair reason when expected same-turn Guard correlation is absent or incompatible."
        }
        _ => unreachable!("AgentToolId cannot contain a non-MCP MethodName"),
    }
}

fn integration_verification_input_schema(tool: AgentToolId) -> Value {
    match tool {
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION => {
            serde_json::to_value(schema_for!(BeginIntegrationVerificationArguments))
                .expect("begin integration-verification schema serializes")
        }
        AgentToolId::GUARD_PROBE | AgentToolId::GET_INTEGRATION_VERIFICATION => {
            serde_json::to_value(schema_for!(IntegrationVerificationIdArguments))
                .expect("integration-verification ID schema serializes")
        }
        _ => unreachable!("connection-integration owner has an exact input schema"),
    }
}

fn integration_verification_output_schema(tool: AgentToolId) -> Value {
    let mut schema = match tool {
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION => serde_json::to_value(schema_for!(
            McpToolStructuredContent<BeginIntegrationVerificationResult>
        ))
        .expect("begin integration-verification result schema serializes"),
        AgentToolId::GUARD_PROBE => {
            serde_json::to_value(schema_for!(McpToolStructuredContent<GuardProbeResult>))
                .expect("Guard probe result schema serializes")
        }
        AgentToolId::GET_INTEGRATION_VERIFICATION => serde_json::to_value(schema_for!(
            McpToolStructuredContent<GetIntegrationVerificationResult>
        ))
        .expect("get integration-verification result schema serializes"),
        _ => unreachable!("connection-integration owner has an exact output schema"),
    };
    schema
        .as_object_mut()
        .expect("integration-verification output schema has an object root")
        .insert("type".to_owned(), Value::String("object".to_owned()));
    schema
}

fn valid_mcp_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn validate_root_object_schema(
    tool_name: &str,
    schema_name: &str,
    schema: &Value,
    errors: &mut Vec<String>,
) {
    let Some(object) = schema.as_object() else {
        errors.push(format!("tool `{tool_name}` {schema_name} is not an object"));
        return;
    };

    match object.get("type") {
        Some(Value::String(schema_type)) if schema_type == "object" => {}
        Some(_) => errors.push(format!(
            "tool `{tool_name}` {schema_name} root type is not object"
        )),
        None => errors.push(format!(
            "tool `{tool_name}` {schema_name} root type is missing"
        )),
    }

    validate_schema_fragment(tool_name, schema_name, schema, errors);
}

fn validate_annotations(tool_name: &str, annotations: &Value, errors: &mut Vec<String>) {
    let Some(object) = annotations.as_object() else {
        errors.push(format!("tool `{tool_name}` annotations is not an object"));
        return;
    };
    for field in [
        "readOnlyHint",
        "destructiveHint",
        "idempotentHint",
        "openWorldHint",
    ] {
        if object.get(field).is_none_or(|value| !value.is_boolean()) {
            errors.push(format!(
                "tool `{tool_name}` annotations.{field} is not a boolean"
            ));
        }
    }
    for field in object.keys() {
        if !matches!(
            field.as_str(),
            "readOnlyHint" | "destructiveHint" | "idempotentHint" | "openWorldHint"
        ) {
            errors.push(format!(
                "tool `{tool_name}` annotations contains unsupported field `{field}`"
            ));
        }
    }
}

fn validate_schema_fragment(tool_name: &str, path: &str, schema: &Value, errors: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        errors.push(format!("tool `{tool_name}` {path} schema is not an object"));
        return;
    };

    for keyword in [
        "patternProperties",
        "unevaluatedProperties",
        "dependentSchemas",
        "$dynamicRef",
        "contains",
    ] {
        if object.contains_key(keyword) {
            errors.push(format!(
                "tool `{tool_name}` {path} uses unsupported schema keyword `{keyword}`"
            ));
        }
    }

    if let Some(schema_uri) = object.get("$schema") {
        if schema_uri.as_str().is_none_or(|uri| uri.is_empty()) {
            errors.push(format!("tool `{tool_name}` {path} has invalid $schema"));
        }
    }
    if let Some(reference) = object.get("$ref") {
        match reference.as_str() {
            Some(value) if value.starts_with("#/") => {}
            _ => errors.push(format!("tool `{tool_name}` {path} uses a non-local $ref")),
        }
    }
    if let Some(schema_type) = object.get("type") {
        validate_schema_type(tool_name, path, schema_type, errors);
    }
    if let Some(enum_values) = object.get("enum") {
        if enum_values
            .as_array()
            .is_none_or(|values| values.is_empty())
        {
            errors.push(format!(
                "tool `{tool_name}` {path} enum is not a non-empty array"
            ));
        }
    }
    if let Some(format) = object.get("format") {
        if format.as_str().is_none_or(|value| value.is_empty()) {
            errors.push(format!("tool `{tool_name}` {path} format is not a string"));
        }
    }
    if let Some(required) = object.get("required") {
        validate_required_fields(tool_name, path, object, required, errors);
    }
    if let Some(properties) = object.get("properties") {
        validate_properties(tool_name, path, properties, errors);
    }
    if let Some(items) = object.get("items") {
        validate_items(tool_name, path, items, errors);
    }
    if let Some(additional_properties) = object.get("additionalProperties") {
        validate_additional_properties(tool_name, path, additional_properties, errors);
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(combinator) {
            validate_schema_branches(tool_name, path, combinator, branches, errors);
        }
    }
    for definitions_key in ["definitions", "$defs"] {
        if let Some(definitions) = object.get(definitions_key) {
            validate_definitions(tool_name, path, definitions_key, definitions, errors);
        }
    }
}

fn validate_schema_branches(
    tool_name: &str,
    path: &str,
    combinator: &str,
    branches: &Value,
    errors: &mut Vec<String>,
) {
    let Some(branches) = branches.as_array() else {
        errors.push(format!(
            "tool `{tool_name}` {path}.{combinator} is not an array"
        ));
        return;
    };
    if branches.is_empty() {
        errors.push(format!("tool `{tool_name}` {path}.{combinator} is empty"));
    }
    for (index, branch) in branches.iter().enumerate() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.{combinator}[{index}]"),
            branch,
            errors,
        );
    }
}

fn validate_schema_type(
    tool_name: &str,
    path: &str,
    schema_type: &Value,
    errors: &mut Vec<String>,
) {
    match schema_type {
        Value::String(value) if valid_json_schema_type(value) => {}
        Value::Array(values)
            if !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(valid_json_schema_type)) => {}
        _ => errors.push(format!("tool `{tool_name}` {path} has invalid type")),
    }
}

fn valid_json_schema_type(value: &str) -> bool {
    matches!(
        value,
        "null" | "boolean" | "object" | "array" | "number" | "string" | "integer"
    )
}

fn validate_required_fields(
    tool_name: &str,
    path: &str,
    object: &Map<String, Value>,
    required: &Value,
    errors: &mut Vec<String>,
) {
    let Some(required_values) = required.as_array() else {
        errors.push(format!(
            "tool `{tool_name}` {path} required is not an array"
        ));
        return;
    };
    let properties = object.get("properties").and_then(Value::as_object);
    let mut seen = BTreeSet::new();
    for value in required_values {
        let Some(field) = value.as_str() else {
            errors.push(format!(
                "tool `{tool_name}` {path} required contains a non-string value"
            ));
            continue;
        };
        if !seen.insert(field.to_owned()) {
            errors.push(format!(
                "tool `{tool_name}` {path} required duplicates `{field}`"
            ));
        }
        if !properties.is_some_and(|properties| properties.contains_key(field)) {
            errors.push(format!(
                "tool `{tool_name}` {path} requires unknown property `{field}`"
            ));
        }
    }
}

fn validate_properties(tool_name: &str, path: &str, properties: &Value, errors: &mut Vec<String>) {
    let Some(properties) = properties.as_object() else {
        errors.push(format!(
            "tool `{tool_name}` {path} properties is not an object"
        ));
        return;
    };
    for (property_name, property_schema) in properties {
        if property_name.is_empty() {
            errors.push(format!(
                "tool `{tool_name}` {path} has an empty property name"
            ));
        }
        validate_schema_fragment(
            tool_name,
            &format!("{path}.properties.{property_name}"),
            property_schema,
            errors,
        );
    }
}

fn validate_items(tool_name: &str, path: &str, items: &Value, errors: &mut Vec<String>) {
    if let Some(item_schema) = items.as_object() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.items"),
            &Value::Object(item_schema.clone()),
            errors,
        );
    } else if let Some(item_schemas) = items.as_array() {
        for (index, item_schema) in item_schemas.iter().enumerate() {
            validate_schema_fragment(
                tool_name,
                &format!("{path}.items[{index}]"),
                item_schema,
                errors,
            );
        }
    } else {
        errors.push(format!("tool `{tool_name}` {path} items is not a schema"));
    }
}

fn validate_additional_properties(
    tool_name: &str,
    path: &str,
    additional_properties: &Value,
    errors: &mut Vec<String>,
) {
    if additional_properties.is_boolean() {
        return;
    }
    if additional_properties.is_object() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.additionalProperties"),
            additional_properties,
            errors,
        );
        return;
    }
    errors.push(format!(
        "tool `{tool_name}` {path} additionalProperties is not boolean or schema"
    ));
}

fn validate_definitions(
    tool_name: &str,
    path: &str,
    definitions_key: &str,
    definitions: &Value,
    errors: &mut Vec<String>,
) {
    let Some(definitions) = definitions.as_object() else {
        errors.push(format!(
            "tool `{tool_name}` {path}.{definitions_key} is not an object"
        ));
        return;
    };
    for (definition_name, definition_schema) in definitions {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.{definitions_key}.{definition_name}"),
            definition_schema,
            errors,
        );
    }
}
