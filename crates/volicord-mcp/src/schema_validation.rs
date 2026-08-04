use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError};
use serde_json::{Map, Value};
use volicord_mcp_wire::{
    mcp_tool_contract, McpToolErrorIssue, McpToolIssueCode, SemanticSchemaDescriptor,
    SemanticValidationIssueCode,
};
use volicord_types::schema::RequiredNullable;
use volicord_types::tool_names::AgentToolId;

pub(crate) fn validate_mcp_tool_arguments(
    tool_name: &str,
    arguments: &Value,
) -> Result<(), McpAdapterError> {
    let tool = AgentToolId::from_wire_name(tool_name)
        .map_err(|_| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let contract = mcp_tool_contract(tool)
        .ok_or_else(|| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let validation = contract.input_descriptor().validate(arguments);
    if validation.issues.is_empty() {
        return Ok(());
    }

    let minimal_example = contract
        .canonical_examples()
        .first()
        .and_then(|example| example.value().as_object())
        .cloned();
    let issues = validation
        .issues
        .into_iter()
        .map(|issue| {
            let mut issue =
                McpToolErrorIssue::new(issue.path, issue_code(issue.code), issue.message);
            enrich_issue(
                contract.input_descriptor(),
                minimal_example.as_ref(),
                &mut issue,
            );
            bound_mcp_tool_error_issue(issue).0
        })
        .collect();
    Err(McpAdapterError::InvalidParams {
        tool_name: tool_name.to_owned(),
        issues,
        truncated: validation.truncated,
        source: None,
    })
}

pub(crate) fn decode_failure_issue(
    tool_name: &str,
    source: &serde_json::Error,
) -> McpToolErrorIssue {
    let mut issue = McpToolErrorIssue::new(
        String::new(),
        McpToolIssueCode::ArgumentDecodeFailed,
        format!("Arguments matched the public input schema but could not be decoded: {source}."),
    );
    if let Ok(tool) = AgentToolId::from_wire_name(tool_name) {
        if let Some(contract) = mcp_tool_contract(tool) {
            let minimal_example = contract
                .canonical_examples()
                .first()
                .and_then(|example| example.value().as_object());
            enrich_issue(contract.input_descriptor(), minimal_example, &mut issue);
        }
    }
    bound_mcp_tool_error_issue(issue).0
}

pub(crate) fn validate_mcp_tool_output(
    tool_name: &str,
    output: &Value,
) -> Result<(), McpAdapterError> {
    let tool = AgentToolId::from_wire_name(tool_name)
        .map_err(|_| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let contract = mcp_tool_contract(tool)
        .ok_or_else(|| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let validation = contract.output_descriptor().validate(output);
    if validation.issues.is_empty() {
        Ok(())
    } else {
        Err(McpAdapterError::ToolOutputSchema {
            tool_name: tool_name.to_owned(),
        })
    }
}

fn issue_code(code: SemanticValidationIssueCode) -> McpToolIssueCode {
    match code {
        SemanticValidationIssueCode::Required => McpToolIssueCode::ArgumentRequired,
        SemanticValidationIssueCode::Unknown => McpToolIssueCode::ArgumentUnknown,
        SemanticValidationIssueCode::TypeMismatch | SemanticValidationIssueCode::AmbiguousUnion => {
            McpToolIssueCode::ArgumentTypeMismatch
        }
        SemanticValidationIssueCode::EnumValue => McpToolIssueCode::ArgumentEnumValue,
    }
}

fn enrich_issue(
    descriptor: &SemanticSchemaDescriptor,
    minimal_example: Option<&Map<String, Value>>,
    issue: &mut McpToolErrorIssue,
) {
    let metadata = descriptor.metadata_at_instance_path(&issue.path);
    if let Some(metadata) = metadata.as_ref() {
        issue.expected_semantic_type = RequiredNullable::some(metadata.semantic_type.clone());
        issue.required_fields.clone_from(&metadata.required_fields);
        issue
            .allowed_enum_values
            .clone_from(&metadata.allowed_enum_values);
        issue.owner_hint = RequiredNullable::new(metadata.description.clone());
    }
    if pointer_last_segment(&issue.path).is_some_and(|segment| segment.parse::<usize>().is_ok()) {
        if let Some(parent) =
            parent_pointer(&issue.path).and_then(|path| descriptor.metadata_at_instance_path(path))
        {
            issue.owner_hint = RequiredNullable::new(parent.description);
        }
    }
    if issue.code == McpToolIssueCode::ArgumentUnknown {
        if let Some(field) = pointer_last_segment(&issue.path) {
            issue.unknown_fields = vec![field];
        }
    }
    issue.minimal_example = RequiredNullable::new(minimal_example.cloned());
}

fn pointer_last_segment(path: &str) -> Option<String> {
    path.rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .map(|value| value.replace("~1", "/").replace("~0", "~"))
}

fn parent_pointer(path: &str) -> Option<&str> {
    path.rfind('/').map(|index| &path[..index])
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Map, Value};
    use volicord_mcp_wire::SemanticValidationResult;

    use super::*;

    #[test]
    fn descriptor_validation_reports_the_bounded_issue_cap() {
        let mut instance = Map::new();
        for index in 0..(SemanticValidationResult::MAX_ISSUES + 20) {
            instance.insert(format!("unknown_{index}"), Value::Bool(true));
        }
        let contract = mcp_tool_contract(AgentToolId::STATUS).expect("status contract");
        let validation = contract
            .input_descriptor()
            .validate(&Value::Object(instance));

        assert_eq!(
            validation.issues.len(),
            SemanticValidationResult::MAX_ISSUES
        );
        assert!(validation.truncated);
    }

    #[test]
    fn descriptor_validation_rejects_string_null_for_required_nullable() {
        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping contract");
        let mut value = contract.canonical_examples()[0].value().clone();
        value["baseline_ref"] = json!("null");

        assert!(!contract
            .input_descriptor()
            .validate(&value)
            .issues
            .is_empty());
    }
}
