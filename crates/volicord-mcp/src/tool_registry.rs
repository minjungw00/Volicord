use crate::prelude::*;
use crate::routing::{
    effective_tool_mode_for_mode_and_storage, list_projects_output_schema, McpEffectiveToolMode,
    McpStorageCapability,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolAnnotations {
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

impl McpToolAnnotations {
    const fn read_only() -> Self {
        Self {
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }
    }

    const fn non_destructive_mutation() -> Self {
        Self {
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }
    }

    const fn destructive_mutation() -> Self {
        Self {
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Value,
    pub annotations: McpToolAnnotations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct McpToolExample {
    pub id: &'static str,
    pub description: &'static str,
    pub arguments_json: &'static str,
}

const INTAKE_CREATE_NEW_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Create an onboarding checklist.","requested_mode":"work","resume_policy":"create_new","initial_scope":{"boundary":"Onboarding checklist setup.","non_goals":[],"acceptance_criteria":[{"statement":"The checklist is available to new workspace users.","evidence_requirement":"required"}]}}"#;
const INTAKE_RESUME_ACTIVE_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Continue the active onboarding checklist work.","requested_mode":"auto","resume_policy":"resume_active","initial_scope":{"boundary":"Continue the current onboarding checklist scope.","non_goals":[],"acceptance_criteria":[]}}"#;
const INTAKE_SUPERSEDE_ACTIVE_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Replace the active onboarding work with the revised checklist.","requested_mode":"work","resume_policy":"supersede_active","initial_scope":{"boundary":"Revised onboarding checklist setup.","non_goals":["Changing account creation."],"acceptance_criteria":[{"statement":"The revised checklist replaces the active work.","evidence_requirement":"required"}]}}"#;
const INTAKE_REJECT_IF_ACTIVE_ARGUMENTS_JSON: &str = r#"{"plain_language_request":"Start an onboarding checklist only when no Task is active.","requested_mode":"advisor","resume_policy":"reject_if_active","initial_scope":{"boundary":"Onboarding checklist guidance.","non_goals":[],"acceptance_criteria":[{"statement":"Provide onboarding checklist guidance.","evidence_requirement":"not_required"}]}}"#;

pub(crate) const UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID: &str = "keep_current_change_unit";
pub(crate) const UPDATE_SCOPE_KEEP_CURRENT_ARGUMENTS_JSON: &str =
    r#"{"task_id":"task_filter_001","change_unit":{"operation":"keep_current"}}"#;
const UPDATE_SCOPE_CREATE_CURRENT_ARGUMENTS_JSON: &str = r#"{"task_id":"task_filter_002","goal_summary":"Limit saved search filters.","scope_update":{"include":["Saved-filter owner and label edits."],"exclude":[]},"scope_boundary":"Saved-filter owner and label edits.","acceptance_criteria":[{"acceptance_criterion_id":null,"statement":"Saved filters reject out-of-scope edits.","evidence_requirement":"required"}],"baseline_ref":"baseline_filter_002","change_unit":{"operation":"create_current","scope_summary":"Saved-filter validation.","affected_paths":["src/search/saved-filters.ts"]}}"#;
const UPDATE_SCOPE_REPLACE_CURRENT_ARGUMENTS_JSON: &str = r#"{"task_id":"task_filter_003","scope_boundary":"Saved-filter owner, label, and visibility edits.","baseline_ref":"baseline_filter_003","change_unit":{"operation":"replace_current","scope_summary":"Expanded saved-filter validation.","affected_paths":["src/search/saved-filters.ts"]}}"#;

pub(crate) const STATUS_READ_ONLY_EXAMPLE_ID: &str = "read_only_status";
const STATUS_SUMMARY_ARGUMENTS_JSON: &str = r#"{"detail":"summary"}"#;
pub(crate) const STATUS_READ_ONLY_ARGUMENTS_JSON: &str = r#"{"detail":"workflow"}"#;
const STATUS_FULL_ARGUMENTS_JSON: &str = r#"{"detail":"full"}"#;

pub(crate) const PREPARE_WRITE_SIMPLE_EXAMPLE_ID: &str = "simple_prepare_write";
pub(crate) const PREPARE_WRITE_SIMPLE_ARGUMENTS_JSON: &str = r#"{"intended_operation":"Update the profile preference save flow.","intended_paths":["src/preferences/profile-save.ts"],"product_file_write_intended":true,"baseline_ref":"baseline_pref_001"}"#;

const STAGE_ARTIFACT_SAFE_TEXT_ARGUMENTS_JSON: &str = r#"{"task_id":"task_trace_001","display_name":"diagnostic_trace.log","content_type":"text/plain","redaction_state":"none","safe_bytes_or_notice":"Local trace sample captured for debugging."}"#;

pub(crate) const RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID: &str =
    "advisor_no_product_write_record_run";
pub(crate) const RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_ARGUMENTS_JSON: &str = r#"{"task_id":"task_advisor_analysis_001","change_unit_id":"cu_advisor_analysis_001","kind":"shaping_update","baseline_ref":"baseline_advisor_analysis_001","summary":"Advisor analysis completed without Product Repository file writes.","observed_changes":{"changed_paths":[],"product_file_write_observed":false,"sensitive_categories":[],"baseline_ref":"baseline_advisor_analysis_001"}}"#;
const RECORD_RUN_EVIDENCE_BEARING_ARGUMENTS_JSON: &str = r#"{"task_id":"task_run_002","change_unit_id":"cu_run_002","kind":"implementation","baseline_ref":"baseline_run_002","summary":"Saved-filter validation passed.","observed_changes":{"changed_paths":[],"product_file_write_observed":false,"sensitive_categories":[],"baseline_ref":"baseline_run_002"},"evidence_updates":[{"target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_saved_filter_001"},"coverage_state":"supported"}],"evidence_observations":[{"target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_saved_filter_001"},"source_kind":"external_tool","assurance_level":"external_tool_result","observed_at":"2026-07-12T00:00:00Z"}],"close_assessment":{"result_summary":"Saved-filter validation passed.","result_refs":[],"residual_risks":[],"sensitive_categories":[],"recovery_constraints":[]}}"#;

pub(crate) const REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID: &str =
    "final_acceptance_request";
pub(crate) const REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_ARGUMENTS_JSON: &str = r#"{"task_id":"task_close_001","judgment_kind":"final_acceptance","presentation":"short","question":"Do you accept this result as complete?","context":{"summary":"Review the current close basis and decide final acceptance.","related_refs":[],"artifact_refs":[],"visible_risks":[],"constraints":["Only final acceptance for the current close basis is in scope."]},"required_for":["close_complete"]}"#;

const RECONCILE_CHANGES_ARGUMENTS_JSON: &str = r#"{"task_id":"task_reconcile_001"}"#;

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

const PREPARE_WRITE_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
    description: "Check one Product Repository write intent.",
    arguments_json: PREPARE_WRITE_SIMPLE_ARGUMENTS_JSON,
}];

const STAGE_ARTIFACT_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: "stage_safe_text",
    description: "Stage a text attachment input.",
    arguments_json: STAGE_ARTIFACT_SAFE_TEXT_ARGUMENTS_JSON,
}];

const RECORD_RUN_EXAMPLES: [McpToolExample; 2] = [
    McpToolExample {
        id: RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
        description: "Record an advisor shaping update with no Product Repository write.",
        arguments_json: RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_ARGUMENTS_JSON,
    },
    McpToolExample {
        id: "evidence_bearing_record_run",
        description: "Record target-scoped evidence and a close assessment.",
        arguments_json: RECORD_RUN_EVIDENCE_BEARING_ARGUMENTS_JSON,
    },
];

const REQUEST_USER_JUDGMENT_EXAMPLES: [McpToolExample; 1] = [McpToolExample {
    id: REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID,
    description: "Request final acceptance with Core-owned authority options.",
    arguments_json: REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_ARGUMENTS_JSON,
}];

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

pub(crate) fn canonical_tool_examples(tool_name: &str) -> &'static [McpToolExample] {
    match tool_name {
        INTAKE_TOOL_NAME => &INTAKE_EXAMPLES,
        UPDATE_SCOPE_TOOL_NAME => &UPDATE_SCOPE_EXAMPLES,
        STATUS_TOOL_NAME => &STATUS_EXAMPLES,
        PREPARE_WRITE_TOOL_NAME => &PREPARE_WRITE_EXAMPLES,
        STAGE_ARTIFACT_TOOL_NAME => &STAGE_ARTIFACT_EXAMPLES,
        RECORD_RUN_TOOL_NAME => &RECORD_RUN_EXAMPLES,
        REQUEST_USER_JUDGMENT_TOOL_NAME => &REQUEST_USER_JUDGMENT_EXAMPLES,
        RECONCILE_CHANGES_TOOL_NAME => &RECONCILE_CHANGES_EXAMPLES,
        CHECK_CLOSE_TOOL_NAME => &CHECK_CLOSE_EXAMPLES,
        CLOSE_TASK_TOOL_NAME => &CLOSE_TASK_EXAMPLES,
        _ => &[],
    }
}

pub fn public_method_tools() -> Vec<McpToolDefinition> {
    method_tools(PUBLIC_METHOD_TOOL_NAMES)
}

/// Returns adapter utility tool definitions.
pub fn adapter_utility_tools() -> Vec<McpToolDefinition> {
    ADAPTER_UTILITY_TOOL_NAMES
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: mcp_tool_input_schema(name)
                .expect("adapter utility tool input schema should exist"),
            output_schema: list_projects_output_schema(),
            annotations: McpToolAnnotations::read_only(),
        })
        .collect()
}

/// Returns workflow-mode MCP-visible tools.
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    mcp_tools_for_mode(AgentConnectionMode::Workflow)
}

/// Returns MCP-visible tools for the supplied Agent Connection mode.
pub fn mcp_tools_for_mode(mode: AgentConnectionMode) -> Vec<McpToolDefinition> {
    let mut tools = match mode {
        AgentConnectionMode::ReadOnly => method_tools(READ_ONLY_METHOD_TOOL_NAMES),
        AgentConnectionMode::Workflow => public_method_tools(),
    };
    tools.extend(adapter_utility_tools());
    tools
}

/// Returns MCP-visible tools for the effective connection and storage capability.
pub fn mcp_tools_for_mode_and_storage(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
) -> Vec<McpToolDefinition> {
    let mut tools = match effective_tool_mode_for_mode_and_storage(mode, storage_capability) {
        McpEffectiveToolMode::Unavailable => Vec::new(),
        McpEffectiveToolMode::ReadOnly | McpEffectiveToolMode::ReadOnlyDegraded => {
            method_tools(READ_ONLY_METHOD_TOOL_NAMES)
        }
        McpEffectiveToolMode::Workflow => public_method_tools(),
    };
    tools.extend(adapter_utility_tools());
    tools
}

pub(crate) fn tools_list_schema_validation_status(tools: &[McpToolDefinition]) -> &'static str {
    if validate_tools_list_schema_compatibility(tools).is_ok() {
        "passed"
    } else {
        "failed"
    }
}

pub(crate) fn mcp_tool_naming_style(tools: &[McpToolDefinition]) -> &'static str {
    if tools.is_empty() {
        return "empty";
    }
    if tools.iter().all(|tool| tool.name.contains('.')) {
        "dotted_namespace"
    } else if tools.iter().all(|tool| !tool.name.contains('.')) {
        "plain"
    } else {
        "mixed"
    }
}

pub(crate) fn validate_tools_list_schema_compatibility(
    tools: &[McpToolDefinition],
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

pub(crate) fn method_tools<const N: usize>(names: [&'static str; N]) -> Vec<McpToolDefinition> {
    names
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: mcp_tool_input_schema(name).expect("MCP tool schema should exist"),
            output_schema: mcp_response_schema(name)
                .expect("MCP tool response schema should exist"),
            annotations: tool_annotations(name),
        })
        .collect()
}

pub(crate) fn mcp_tool_input_schema(name: &str) -> Option<Value> {
    let mut schema = if name == LIST_PROJECTS_TOOL_NAME {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    } else {
        mcp_request_schema(name)?
    };
    let examples = canonical_tool_examples(name)
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
    Some(schema)
}

fn tool_annotations(name: &str) -> McpToolAnnotations {
    match name {
        STATUS_TOOL_NAME | CHECK_CLOSE_TOOL_NAME => McpToolAnnotations::read_only(),
        PREPARE_WRITE_TOOL_NAME
        | STAGE_ARTIFACT_TOOL_NAME
        | RECORD_RUN_TOOL_NAME
        | REQUEST_USER_JUDGMENT_TOOL_NAME => McpToolAnnotations::non_destructive_mutation(),
        INTAKE_TOOL_NAME
        | UPDATE_SCOPE_TOOL_NAME
        | RECONCILE_CHANGES_TOOL_NAME
        | CLOSE_TASK_TOOL_NAME => McpToolAnnotations::destructive_mutation(),
        _ => panic!("missing MCP annotation policy for tool `{name}`"),
    }
}

pub(crate) fn tool_description(name: &str) -> &'static str {
    match name {
        INTAKE_TOOL_NAME => "Start, resume, supersede, or reject an ordinary user work loop.",
        UPDATE_SCOPE_TOOL_NAME => {
            "Update the current Task scope and keep, create, or replace its current Change Unit."
        }
        STATUS_TOOL_NAME => "Read the current Core status view without creating Core authority state.",
        PREPARE_WRITE_TOOL_NAME => {
            "Check one proposed Product Repository write against current Core scope, authority, and freshness."
        }
        STAGE_ARTIFACT_TOOL_NAME => {
            "Prepare an Evidence attachment input; staging alone is not recorded Evidence."
        }
        RECORD_RUN_TOOL_NAME => {
            "Record a Run and evidence. Mode/kind: advisor/shaping_update; direct/direct; work/shaping_update or implementation. Advisor has no Product Repository writes."
        }
        REQUEST_USER_JUDGMENT_TOOL_NAME => {
            "Create one focused user-owned judgment; authority-bearing choices remain Core-owned."
        }
        RECONCILE_CHANGES_TOOL_NAME => {
            "Reconcile unresolved Product Repository changes without agent-only dismissal."
        }
        CHECK_CLOSE_TOOL_NAME => {
            "Read current close readiness without requesting a terminal mutation."
        }
        CLOSE_TASK_TOOL_NAME => {
            "Request the complete, cancel, or supersede terminal path for one Task."
        }
        LIST_PROJECTS_TOOL_NAME => "List projects explicitly allowed for this MCP connection.",
        _ => "Unsupported Volicord method.",
    }
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
