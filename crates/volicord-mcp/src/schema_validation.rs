use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError};
use serde_json::Value;
use volicord_mcp_wire::{
    mcp_tool_contract, McpInputContractValidation, McpToolErrorIssue, McpToolIssueCode,
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
    let validation = match contract.validate_and_decode_input(arguments) {
        McpInputContractValidation::Valid => return Ok(()),
        McpInputContractValidation::Invalid(validation) => validation,
        McpInputContractValidation::SchemaContractFailure => {
            return Err(McpAdapterError::SchemaContractFailure {
                tool_name: tool_name.to_owned(),
            });
        }
    };

    let selected_variant = validation.selected_variant;
    let canonical_example = validation.canonical_example;
    let mut truncated = validation.truncated;
    let mut issues = Vec::with_capacity(validation.issues.len());
    for semantic_issue in validation.issues {
        let mut issue = McpToolErrorIssue::new(
            semantic_issue.path,
            issue_code(semantic_issue.code),
            semantic_issue.message,
        );
        issue.expected_semantic_type = RequiredNullable::new(semantic_issue.expected_semantic_type);
        issue.allowed_values = semantic_issue.allowed_values;
        issue.owner_hint = RequiredNullable::new(semantic_issue.field_description);
        let (issue, issue_truncated) = bound_mcp_tool_error_issue(issue);
        truncated |= issue_truncated;
        issues.push(issue);
    }
    Err(McpAdapterError::InvalidParams {
        tool_name: tool_name.to_owned(),
        issues,
        truncated,
        selected_variant,
        canonical_example,
    })
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
        SemanticValidationIssueCode::TypeMismatch => McpToolIssueCode::ArgumentTypeMismatch,
        SemanticValidationIssueCode::EnumValue => McpToolIssueCode::ArgumentEnumValue,
    }
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

        let validation = contract.input_descriptor().validate(&value);
        let issue = validation
            .issues
            .iter()
            .find(|issue| issue.path == "/baseline_ref")
            .expect("baseline mismatch");
        assert_eq!(
            issue.expected_semantic_type.as_deref(),
            Some("BaselineRef | null")
        );
    }

    #[test]
    fn invalid_checkpoint_discriminator_never_guesses_branch_fields() {
        let arguments = json!({
            "checkpoint_operation": {"operation": "create"},
            "baseline_ref": null
        });

        let error = validate_mcp_tool_arguments(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            &arguments,
        )
        .expect_err("invalid discriminator must fail before decoding");
        let McpAdapterError::InvalidParams {
            issues,
            selected_variant,
            canonical_example,
            ..
        } = error
        else {
            panic!("expected descriptor invalid-params error");
        };

        assert_eq!(selected_variant, None);
        assert_eq!(issues[0].path, "/checkpoint_operation/operation");
        assert_eq!(issues[0].code, McpToolIssueCode::ArgumentEnumValue);
        assert_eq!(
            issues[0].allowed_values,
            ["create_initial", "replace_current"]
        );
        assert!(canonical_example.as_ref().is_some_and(
            |summary| summary.contains_key("variants") && !summary.contains_key("task_id")
        ));
        for forbidden in [
            "expected_current_checkpoint_id",
            "retired_non_authorizing_request_refs",
            "carry_forward_application_refs",
            "stale_authority_actions",
        ] {
            assert!(issues.iter().all(|issue| !issue.path.contains(forbidden)));
        }
    }

    #[test]
    fn create_initial_with_required_null_baseline_passes_schema_and_exact_decode() {
        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping contract");
        let value = contract
            .canonical_examples()
            .iter()
            .find(|example| example.id() == "create_initial_null_baseline")
            .expect("null-baseline example")
            .value();

        validate_mcp_tool_arguments(AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(), value)
            .expect("valid selected branch must validate and decode exactly");
    }

    #[test]
    fn missing_checkpoint_discriminator_reports_variants_without_guessing() {
        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping contract");
        let mut value = contract
            .canonical_examples()
            .iter()
            .find(|example| example.id() == "create_initial_null_baseline")
            .expect("null-baseline example")
            .value()
            .clone();
        value["checkpoint_operation"]
            .as_object_mut()
            .expect("checkpoint operation")
            .remove("operation");

        let validation = contract.input_descriptor().validate(&value);
        let local = validation
            .issues
            .iter()
            .filter(|issue| issue.path.starts_with("/checkpoint_operation"))
            .collect::<Vec<_>>();
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].path, "/checkpoint_operation/operation");
        assert_eq!(local[0].code, SemanticValidationIssueCode::Required);
        assert!(local[0].message.contains("Allowed variants"));
        assert_eq!(
            local[0].allowed_values,
            ["create_initial", "replace_current"]
        );
        assert!(validation
            .canonical_example
            .as_ref()
            .is_some_and(|summary| summary.contains_key("variants")));
    }
}
