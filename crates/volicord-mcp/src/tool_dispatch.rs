//! Public tool-call decoding, adapter dispatch, and shared tool-result carrier.

use crate::adapter::{McpAdapter, OwnedAgentSessionCoordinates};
use crate::authority_refresh::MutationRefreshContext;
use crate::binding::{
    bind_codex_managed_tool_call, managed_agent_session_binding,
    validate_managed_stdio_session_ownership_admitted,
};
use crate::committed_result_recovery::bounded_mutation_compatibility_text;
#[cfg(test)]
use crate::committed_result_recovery::{
    authoritative_refresh_failure_output, mutation_post_effect_failure_output,
    mutation_response_budget_exceeded_output, CanonicalMcpMutationOutcome,
    MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES,
};
use crate::diagnostics::{McpDiagnostic, McpToolCallDiagnostic};
use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError};
use crate::json_rpc::{
    invalid_params_response, json_rpc_error, json_rpc_error_for_adapter, required_object_params,
};
use crate::lifecycle::SessionRuntime;
#[cfg(test)]
use crate::mutation_projection::{
    compact_mutation_method_result, finalize_mutation_output_with_refresh,
    MAX_MCP_FULL_MUTATION_RESULT_BYTES,
};
use crate::mutation_projection::{
    finalize_mutation_output, mutation_detail_for_tool, mutation_effect_anchor,
    response_kind_from_structured_content, MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
};
#[cfg(test)]
use crate::prelude::*;
#[cfg(test)]
use crate::routing::McpConnectionContext;
use crate::schema_validation::validate_mcp_tool_output;
use crate::session_metrics::{
    record_tools_list_metric_best_effort, start_transport_diagnostic_session,
};
use crate::telemetry::{
    authoritative_observation_timestamp, record_tool_diagnostic_best_effort, ToolDiagnosticFacts,
};
use crate::tool_registry::{CanonicalContent, CanonicalToolResult};
use crate::user_action_projection::{user_action_tool_output, UserActionFallback};
use serde_json::{json, Value};
use std::{collections::BTreeSet, time::Instant};
use volicord_core::pipeline::{
    CoreOperationalOperation, CoreOperationalResource, CoreOperationalUnavailable,
    CorePipelineError, PipelineResponse,
};
use volicord_mcp_protocol::McpProtocolCapabilities;
#[cfg(test)]
use volicord_mcp_protocol::ProtocolRegistry;
use volicord_mcp_wire::{
    McpOperationalErrorCode, McpOperationalFailure, McpOperationalOperation,
    McpOperationalResource, McpPostEffectFailureCode, McpToolErrorCode, McpToolErrorIssue,
    McpToolErrorResponse, McpToolIssueCode, MAX_MCP_TOOL_ERROR_RESULT_BYTES, MAX_VALIDATION_ISSUES,
};
use volicord_store::agent_connections::{
    agent_connection_record_read_only, CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW,
};
use volicord_store::diagnostics::DiagnosticOutcome;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_store::operational_sessions::{
    record_mcp_tools_list, record_mcp_verification_tool_observation,
};
use volicord_types::methods::OperationResultRef;
use volicord_types::tool_names::{AgentToolId, AgentToolOwner, ToolVerificationRole};
use volicord_types::values::MethodName;
use volicord_types::values::{AgentConnectionMode, EffectKind};

pub(crate) fn list_tools_result(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
    runtime: &mut SessionRuntime,
    capabilities: McpProtocolCapabilities,
) -> Result<Result<Value, Value>, McpAdapterError> {
    if let Err(error) = crate::json_rpc::validate_optional_object_params(id, params, "tools/list") {
        runtime.pending_finding = Some(McpDiagnostic::ToolDiscovery(
            crate::diagnostics::McpToolDiscoveryDiagnostic::ProtocolError,
        ));
        return Ok(Err(error));
    }
    let canonical_tools = match adapter.tools_for_context(context) {
        Ok(tools) => tools,
        Err(error) => {
            runtime.pending_finding = Some(McpDiagnostic::from(&error));
            return Ok(Err(json_rpc_error_for_adapter(id.clone(), error)));
        }
    };
    let required_tools_present = required_tool_set_present(context, adapter, &canonical_tools)?;
    let returned_tool_identities = canonical_tools
        .iter()
        .map(|tool| tool.id.wire_name().to_owned())
        .collect::<Vec<_>>();
    if !required_tools_present {
        runtime.pending_finding = Some(McpDiagnostic::ToolDiscovery(
            crate::diagnostics::McpToolDiscoveryDiagnostic::RequiredToolMissing,
        ));
    }
    let tools = canonical_tools
        .iter()
        .map(|tool| tool.project(capabilities))
        .collect::<Vec<_>>();
    let result = json!({ "tools": tools });
    if !runtime.runtime_session_id.is_empty() {
        record_mcp_tools_list(
            context,
            &runtime.runtime_session_id,
            &returned_tool_identities,
            required_tools_present,
            &authoritative_observation_timestamp(),
        )
        .map_err(McpAdapterError::Store)?;
    }
    let serialized_bytes = serde_json::to_vec(&result)
        .ok()
        .and_then(|bytes| u64::try_from(bytes.len()).ok());
    if runtime.codex_binding.is_pending() {
        if runtime.deferred_tools_list_serialized_bytes.is_none() {
            runtime.deferred_tools_list_serialized_bytes = serialized_bytes;
        }
    } else if let Some(serialized_bytes) = serialized_bytes {
        record_tools_list_metric_best_effort(context, adapter, runtime, serialized_bytes);
    }
    Ok(Ok(result))
}

fn required_tool_set_present(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    tools: &[crate::tool_registry::CanonicalToolDefinition],
) -> Result<bool, McpAdapterError> {
    let connection = agent_connection_record_read_only(
        adapter.admitted_runtime_home(context)?,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| {
        McpAdapterError::Environment("tools/list Agent Connection disappeared".to_owned())
    })?;
    let mode = match connection.mode.as_str() {
        CONNECTION_MODE_WORKFLOW => AgentConnectionMode::Workflow,
        CONNECTION_MODE_READ_ONLY => AgentConnectionMode::ReadOnly,
        _ => {
            return Err(McpAdapterError::Environment(
                "tools/list Agent Connection has an invalid mode".to_owned(),
            ));
        }
    };
    let actual = tools
        .iter()
        .map(|tool| tool.id.wire_name())
        .collect::<BTreeSet<_>>();
    Ok(AgentToolId::ALL
        .iter()
        .filter(|tool| tool.available_in(mode))
        .all(|tool| actual.contains(tool.wire_name())))
}

pub(crate) fn safe_tool_call_response_failed(response: &Value) -> bool {
    if response.get("error").is_some() {
        return true;
    }
    let Some(result) = response.get("result") else {
        return false;
    };
    let error_code = projected_tool_error_code(result);
    let is_error = result.get("isError").and_then(Value::as_bool) == Some(true)
        || result
            .pointer("/toolResult/code")
            .and_then(Value::as_str)
            .is_some_and(|code| code.starts_with("MCP_"));
    is_error && error_code.as_deref() != Some("MCP_INVALID_ARGUMENTS")
}

pub(crate) fn projected_tool_error_code(result: &Value) -> Option<String> {
    if let Some(code) = result
        .pointer("/structuredContent/code")
        .or_else(|| result.pointer("/toolResult/code"))
        .and_then(Value::as_str)
    {
        return Some(code.to_owned());
    }
    result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|value| value.get("code").and_then(Value::as_str).map(str::to_owned))
}

pub(crate) fn call_tool_result(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
    state: &mut SessionRuntime,
    capabilities: McpProtocolCapabilities,
) -> Result<Result<Value, Value>, McpAdapterError> {
    let diagnostic_started = Instant::now();
    let diagnostic_request_bytes = params
        .as_ref()
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let diagnostic_tool_name = params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|object| object.get("name"))
        .and_then(Value::as_str)
        .and_then(|tool_name| AgentToolId::from_wire_name(tool_name).ok())
        .map(|tool| tool.wire_name().to_owned());
    let object = match required_object_params(id, params, "tools/call") {
        Ok(object) => object,
        Err(error) => {
            state.pending_finding = Some(McpDiagnostic::ToolCall(
                McpToolCallDiagnostic::InvalidArguments,
            ));
            record_tool_diagnostic_best_effort(
                context,
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                diagnostic_tool_name.as_deref(),
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    if object.contains_key("task") {
        let error = invalid_params_response(id, "tools/call task augmentation is not supported");
        state.pending_finding = Some(McpDiagnostic::ToolCall(
            McpToolCallDiagnostic::InvalidArguments,
        ));
        record_tool_diagnostic_best_effort(
            context,
            adapter,
            state,
            diagnostic_started,
            diagnostic_request_bytes,
            diagnostic_tool_name.as_deref(),
            Some(&error),
            ToolDiagnosticFacts::default(),
            true,
            DiagnosticOutcome::ValidationFailure,
        );
        return Ok(Err(error));
    }

    let requested_tool_name = match object.get("name").and_then(Value::as_str) {
        Some(tool_name) => tool_name,
        None => {
            let error = invalid_params_response(id, "tools/call params.name must be a string");
            state.pending_finding = Some(McpDiagnostic::ToolCall(
                McpToolCallDiagnostic::InvalidArguments,
            ));
            record_tool_diagnostic_best_effort(
                context,
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                None,
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    let tool = match AgentToolId::from_wire_name(requested_tool_name) {
        Ok(tool) => tool,
        Err(_) => {
            let error = json_rpc_error(
                id.clone(),
                -32602,
                "Invalid params",
                Some(format!("unknown MCP tool: {requested_tool_name}")),
            );
            state.pending_finding =
                Some(McpDiagnostic::ToolCall(McpToolCallDiagnostic::UnknownTool));
            record_tool_diagnostic_best_effort(
                context,
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                None,
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    let tool_name = tool.wire_name();
    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            let error =
                invalid_params_response(id, "tools/call params.arguments must be an object");
            state.pending_finding = Some(McpDiagnostic::ToolCall(
                McpToolCallDiagnostic::InvalidArguments,
            ));
            record_tool_diagnostic_best_effort(
                context,
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                Some(tool_name),
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    let binding_became_active =
        match bind_codex_managed_tool_call(context, adapter, &mut state.codex_binding, &object) {
            Ok(binding_became_active) => binding_became_active,
            Err(error) => {
                state.pending_finding = Some(McpDiagnostic::Host(error));
                return Ok(Err(invalid_params_response(id, error.to_string())));
            }
        };
    if binding_became_active {
        let _ = start_transport_diagnostic_session(context, adapter, state);
        if let Some(serialized_bytes) = state.deferred_tools_list_serialized_bytes.take() {
            record_tools_list_metric_best_effort(context, adapter, state, serialized_bytes);
        }
    }
    if tool == AgentToolId::STATUS {
        state.status_method_call_count = state.status_method_call_count.saturating_add(1);
    }
    let mutation_detail = mutation_detail_for_tool(tool, &arguments);

    let binding = managed_agent_session_binding(&state.codex_binding, &state.runtime_session_id);
    let coordinates = binding
        .as_ref()
        .map(|binding| {
            adapter.ensure_agent_session_binding_for_tool(context, tool, &arguments, binding)
        })
        .transpose()?
        .flatten();

    let output = if matches!(tool.owner(), AgentToolOwner::CoreMethod(_)) {
        let session = coordinates
            .as_ref()
            .map(OwnedAgentSessionCoordinates::borrowed);
        let call_result = adapter.call_tool_for_session(context, tool, arguments, session);
        match call_result {
            Ok(response) if tool == AgentToolId::REQUEST_USER_ACTION => {
                let pending_response = response.clone();
                match user_action_tool_output(context, adapter, response) {
                    Ok(output) => output,
                    Err(_) => ToolCallOutput::from_pipeline_response(&pending_response)?
                        .with_post_effect_failure(
                            McpPostEffectFailureCode::McpPostEffectAdapterFailed,
                        ),
                }
            }
            Ok(response) if tool == AgentToolId::GET_OPERATION_RESULT => {
                ToolCallOutput::from_operation_result_response(&response, capabilities)?
            }
            Ok(response) => ToolCallOutput::from_pipeline_response(&response)?,
            Err(McpAdapterError::Core(CorePipelineError::OperationalUnavailable(failure))) => {
                ToolCallOutput::from_core_operational_unavailable(
                    tool.method()
                        .expect("Core-owned MCP tools must have a MethodName"),
                    &failure,
                )?
            }
            Err(McpAdapterError::OperationalUnavailable {
                retryable,
                reached_core,
            }) => ToolCallOutput::operational_unavailable(
                tool.method()
                    .expect("Core-owned MCP tools must have a MethodName"),
                retryable,
                reached_core,
                McpOperationalOperation::StoreAccess,
                McpOperationalResource::ProjectStore,
            )?,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_capabilities(tool_name, &error, capabilities);
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    true,
                    DiagnosticOutcome::ValidationFailure,
                );
                return Ok(Ok(response));
            }
            Err(error @ McpAdapterError::ToolExecution { .. }) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_capabilities(tool_name, &error, capabilities);
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::ToolError,
                );
                return Ok(Ok(response));
            }
            Err(error) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response = json_rpc_error_for_adapter(id.clone(), error);
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(response));
            }
        }
    } else {
        let response = match adapter.call_adapter_tool(
            context,
            tool,
            arguments,
            binding.as_ref(),
            coordinates.as_ref(),
        ) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_capabilities(tool_name, &error, capabilities);
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    true,
                    DiagnosticOutcome::ValidationFailure,
                );
                return Ok(Ok(response));
            }
            Err(error @ McpAdapterError::ToolExecution { .. }) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_capabilities(tool_name, &error, capabilities);
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::ToolError,
                );
                return Ok(Ok(response));
            }
            Err(error) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response = json_rpc_error_for_adapter(id.clone(), error);
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(response));
            }
        };
        let text = serde_json::to_string(&response)
            .map_err(McpAdapterError::Json)
            .map_err(|error| json_rpc_error_for_adapter(id.clone(), error));
        match text {
            Ok(text) => ToolCallOutput::success(text)?,
            Err(error) => {
                record_tool_diagnostic_best_effort(
                    context,
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&error),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(error));
            }
        }
    };
    let output = finalize_mutation_output(
        context,
        adapter,
        state,
        capabilities,
        tool_name,
        mutation_detail,
        output,
    )?;

    let diagnostic_facts = output.diagnostic_facts();
    let diagnostic_outcome =
        if response_kind_from_structured_content(&output.structured_content) == Some("rejected") {
            DiagnosticOutcome::Rejected
        } else if output.is_error {
            DiagnosticOutcome::ToolError
        } else {
            DiagnosticOutcome::Success
        };
    let verification_tool = ToolVerificationRole::ManagedHostRoundTrip.tool();
    if diagnostic_outcome == DiagnosticOutcome::Success
        && tool == verification_tool
        && state.codex_binding.is_bound()
        && !state.runtime_session_id.is_empty()
    {
        validate_managed_stdio_session_ownership_admitted(context, adapter, &state.codex_binding)?;
        record_mcp_verification_tool_observation(
            context,
            &state.runtime_session_id,
            &authoritative_observation_timestamp(),
        )
        .map_err(McpAdapterError::Store)?;
    }
    let response = tool_call_result_from_output_for_capabilities(tool_name, output, capabilities)?;
    record_tool_diagnostic_best_effort(
        context,
        adapter,
        state,
        diagnostic_started,
        diagnostic_request_bytes,
        Some(tool_name),
        Some(&response),
        diagnostic_facts,
        false,
        diagnostic_outcome,
    );
    Ok(Ok(response))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCallOutput {
    pub(crate) primary_text: String,
    pub(crate) structured_content: Value,
    pub(crate) extra_texts: Vec<String>,
    pub(crate) is_error: bool,
    pub(crate) diagnostic_facts: ToolDiagnosticFacts,
    pub(crate) operation_result_ref: Option<OperationResultRef>,
    pub(crate) mutation_refresh_context: Option<MutationRefreshContext>,
    pub(crate) post_effect_failure: Option<McpPostEffectFailureCode>,
}

impl ToolCallOutput {
    pub(crate) fn success(primary_text: String) -> Result<Self, McpAdapterError> {
        let structured_content: Value =
            serde_json::from_str(&primary_text).map_err(McpAdapterError::Json)?;
        if !structured_content.is_object() {
            return Err(McpAdapterError::Protocol(
                "successful MCP tool output must be a JSON object".to_owned(),
            ));
        }
        Ok(Self {
            primary_text,
            structured_content,
            extra_texts: Vec::new(),
            is_error: false,
            diagnostic_facts: ToolDiagnosticFacts::default(),
            operation_result_ref: None,
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    }

    fn operational_unavailable(
        tool_name: MethodName,
        retryable: bool,
        reached_core: bool,
        operation: McpOperationalOperation,
        resource: McpOperationalResource,
    ) -> Result<Self, McpAdapterError> {
        let structured_content = serde_json::to_value(McpOperationalFailure {
            code: McpOperationalErrorCode::Unavailable,
            tool_name,
            operation,
            resource,
            retryable,
            reached_core,
            committed: false,
        })
        .map_err(McpAdapterError::Json)?;
        Ok(Self {
            primary_text:
                "Volicord could not produce a tool result because an operational dependency is unavailable."
                    .to_owned(),
            structured_content,
            extra_texts: Vec::new(),
            is_error: true,
            diagnostic_facts: ToolDiagnosticFacts {
                core_reached: reached_core,
                ..ToolDiagnosticFacts::default()
            },
            operation_result_ref: None,
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    }

    fn from_core_operational_unavailable(
        tool_name: MethodName,
        failure: &CoreOperationalUnavailable,
    ) -> Result<Self, McpAdapterError> {
        Self::operational_unavailable(
            tool_name,
            failure.retryable(),
            true,
            mcp_operational_operation(failure.operation()),
            mcp_operational_resource(failure.resource()),
        )
    }

    pub(crate) fn from_pipeline_response(
        response: &PipelineResponse,
    ) -> Result<Self, McpAdapterError> {
        let mut output = Self::success(response.response_json.clone())?;
        output.operation_result_ref = response.operation_result_ref.clone();
        output.apply_pipeline_diagnostics(response);
        Ok(output)
    }

    #[cfg(test)]
    fn from_operation_result_response_for_test(
        response: &PipelineResponse,
    ) -> Result<Self, McpAdapterError> {
        Self::from_operation_result_response(
            response,
            ProtocolRegistry::production()
                .preferred_server_profile()
                .capabilities(),
        )
    }

    fn from_operation_result_response(
        response: &PipelineResponse,
        capabilities: McpProtocolCapabilities,
    ) -> Result<Self, McpAdapterError> {
        let mut output = Self::from_pipeline_response(response)?;
        if output.structured_content["base"]["response_kind"].as_str() == Some("result") {
            let start = output.structured_content["start_offset_bytes"]
                .as_u64()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include start_offset_bytes".to_owned(),
                    )
                })?;
            let end = output.structured_content["end_offset_bytes"]
                .as_u64()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include end_offset_bytes".to_owned(),
                    )
                })?;
            let complete = output.structured_content["complete"]
                .as_bool()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include complete".to_owned(),
                    )
                })?;
            output.primary_text = bounded_mutation_compatibility_text(format!(
                "Volicord returned historical operation-result bytes [{start}, {end}); complete={complete}. Inspect chunk_utf8 in the authoritative result and do not treat historical bytes as current authority."
            ));
            if rendered_tool_call_output_size_for_capabilities(&output, capabilities)?
                > MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            {
                return Err(McpAdapterError::Protocol(
                    "operation-result page exceeded its fixed MCP output budget".to_owned(),
                ));
            }
        }
        Ok(output)
    }

    pub(crate) fn with_operation_result_ref(
        mut self,
        operation_result_ref: Option<OperationResultRef>,
    ) -> Self {
        self.operation_result_ref = operation_result_ref;
        self
    }

    pub(crate) fn with_pipeline_diagnostics(mut self, response: &PipelineResponse) -> Self {
        self.operation_result_ref = response.operation_result_ref.clone();
        self.apply_pipeline_diagnostics(response);
        self
    }

    fn apply_pipeline_diagnostics(&mut self, response: &PipelineResponse) {
        self.diagnostic_facts.core_reached = response.verified_invocation.is_some();
        self.diagnostic_facts.effect_kind = response
            .response_value
            .pointer("/base/effect_kind")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        self.diagnostic_facts.core_committed = !response.replayed
            && self.diagnostic_facts.effect_kind == Some(EffectKind::CoreCommitted);
        self.diagnostic_facts.effect_applied = matches!(
            self.diagnostic_facts.effect_kind,
            Some(EffectKind::CoreCommitted | EffectKind::StagingCreated)
        );
        self.diagnostic_facts.effect_anchor = mutation_effect_anchor(response);
        self.diagnostic_facts.replayed = response.replayed;
        self.diagnostic_facts.product_file_write_count = response
            .response_value
            .pointer("/run_summary/observed_changes/product_file_write_observed")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            as u64;
        self.mutation_refresh_context = MutationRefreshContext::from_pipeline_response(response);
    }

    fn with_post_effect_failure(mut self, code: McpPostEffectFailureCode) -> Self {
        self.post_effect_failure = Some(code);
        self
    }

    pub(crate) fn with_user_action_fallback(mut self, fallback: UserActionFallback) -> Self {
        self.diagnostic_facts.fallback_kind = Some(fallback.kind);
        self.extra_texts.extend(fallback.texts);
        self
    }

    fn diagnostic_facts(&self) -> ToolDiagnosticFacts {
        self.diagnostic_facts.clone()
    }
}

fn mcp_operational_operation(operation: CoreOperationalOperation) -> McpOperationalOperation {
    match operation {
        CoreOperationalOperation::ProductPathObservation => {
            McpOperationalOperation::ProductPathObservation
        }
        CoreOperationalOperation::StoreAccess => McpOperationalOperation::StoreAccess,
    }
}

fn mcp_operational_resource(resource: CoreOperationalResource) -> McpOperationalResource {
    match resource {
        CoreOperationalResource::ProductRepository => McpOperationalResource::ProductRepository,
        CoreOperationalResource::Store => McpOperationalResource::Store,
        CoreOperationalResource::RegistryStore => McpOperationalResource::RegistryStore,
        CoreOperationalResource::ProjectStore => McpOperationalResource::ProjectStore,
        CoreOperationalResource::RuntimeHome => McpOperationalResource::RuntimeHome,
        CoreOperationalResource::PlatformEnvironment => McpOperationalResource::PlatformEnvironment,
    }
}

#[cfg(test)]
fn rendered_tool_call_output_size(output: &ToolCallOutput) -> Result<usize, McpAdapterError> {
    rendered_tool_call_output_size_for_capabilities(
        output,
        ProtocolRegistry::production()
            .preferred_server_profile()
            .capabilities(),
    )
}

pub(crate) fn rendered_tool_call_output_size_for_capabilities(
    output: &ToolCallOutput,
    capabilities: McpProtocolCapabilities,
) -> Result<usize, McpAdapterError> {
    let canonical = canonical_tool_result_from_output(output);
    let projected = canonical
        .project(capabilities)
        .map_err(McpAdapterError::Json)?;
    serde_json::to_vec(projected.as_value())
        .map(|rendered| rendered.len())
        .map_err(McpAdapterError::Json)
}

#[cfg(test)]
pub(crate) fn tool_call_result_from_output(output: ToolCallOutput) -> Value {
    tool_call_result_from_output_for_capabilities(
        "volicord.test",
        output,
        ProtocolRegistry::production()
            .preferred_server_profile()
            .capabilities(),
    )
    .expect("canonical test tool result should project")
}

fn canonical_tool_result_from_output(output: &ToolCallOutput) -> CanonicalToolResult {
    let mut content = Vec::with_capacity(1 + output.extra_texts.len());
    content.push(CanonicalContent::Text(output.primary_text.clone()));
    content.extend(
        output
            .extra_texts
            .iter()
            .cloned()
            .map(CanonicalContent::Text),
    );
    CanonicalToolResult {
        metadata: None,
        content,
        structured_content: output.structured_content.clone(),
        is_error: output.is_error,
    }
}

pub(crate) fn tool_call_result_from_output_for_capabilities(
    tool_name: &str,
    output: ToolCallOutput,
    capabilities: McpProtocolCapabilities,
) -> Result<Value, McpAdapterError> {
    if capabilities.tools().structured_content() && is_known_mcp_tool(tool_name) {
        validate_mcp_tool_output(tool_name, &output.structured_content)?;
    }
    canonical_tool_result_from_output(&output)
        .project(capabilities)
        .map(|projected| projected.into_value())
        .map_err(McpAdapterError::Json)
}

pub(crate) fn is_known_mcp_tool(tool_name: &str) -> bool {
    AgentToolId::from_wire_name(tool_name).is_ok()
}

#[cfg(test)]
pub(crate) fn tool_execution_error_result(
    requested_tool_name: &str,
    error: &McpAdapterError,
) -> Value {
    tool_execution_error_result_for_capabilities(
        requested_tool_name,
        error,
        ProtocolRegistry::production()
            .preferred_server_profile()
            .capabilities(),
    )
}

fn tool_execution_error_result_for_capabilities(
    requested_tool_name: &str,
    error: &McpAdapterError,
    capabilities: McpProtocolCapabilities,
) -> Value {
    let structured = match error {
        McpAdapterError::InvalidParams {
            issues, truncated, ..
        } => McpToolErrorResponse {
            code: McpToolErrorCode::InvalidArguments,
            tool_name: requested_tool_name.to_owned(),
            retryable: true,
            reached_core: false,
            committed: false,
            reported_issue_count: issues.len(),
            truncated: *truncated,
            issues: issues.clone(),
        },
        McpAdapterError::ToolExecution { tool_name, message } => {
            let (path, message) = if tool_name == "project routing" {
                (
                    "/project_selector".to_owned(),
                    format!(
                        "{message}. Use {} when project selection is unclear.",
                        AgentToolId::LIST_PROJECTS.wire_name()
                    ),
                )
            } else {
                (
                    String::new(),
                    format!("{tool_name} failed before reaching Core: {message}"),
                )
            };
            McpToolErrorResponse {
                code: McpToolErrorCode::AdapterPreconditionFailed,
                tool_name: requested_tool_name.to_owned(),
                retryable: false,
                reached_core: false,
                committed: false,
                reported_issue_count: 1,
                truncated: false,
                issues: vec![McpToolErrorIssue::new(
                    path,
                    McpToolIssueCode::AdapterPreconditionFailed,
                    message,
                )],
            }
        }
        McpAdapterError::MutationAdmission(condition) => McpToolErrorResponse {
            code: McpToolErrorCode::AdapterPreconditionFailed,
            tool_name: requested_tool_name.to_owned(),
            retryable: condition.retryable(),
            reached_core: false,
            committed: false,
            reported_issue_count: 1,
            truncated: false,
            issues: vec![McpToolErrorIssue::new(
                String::new(),
                McpToolIssueCode::AdapterPreconditionFailed,
                condition.to_string(),
            )],
        },
        _ => McpToolErrorResponse {
            code: McpToolErrorCode::AdapterPreconditionFailed,
            tool_name: requested_tool_name.to_owned(),
            retryable: false,
            reached_core: false,
            committed: false,
            reported_issue_count: 1,
            truncated: false,
            issues: vec![McpToolErrorIssue::new(
                String::new(),
                McpToolIssueCode::AdapterPreconditionFailed,
                "Tool execution failed before reaching Core.",
            )],
        },
    };
    bounded_tool_error_result(structured, capabilities)
}

fn bounded_tool_error_result(
    mut structured: McpToolErrorResponse,
    capabilities: McpProtocolCapabilities,
) -> Value {
    let mut truncated = structured.truncated;
    if structured.issues.len() > MAX_VALIDATION_ISSUES {
        structured.issues.truncate(MAX_VALIDATION_ISSUES);
        truncated = true;
    }
    structured.issues = structured
        .issues
        .into_iter()
        .map(|issue| {
            let (issue, issue_truncated) = bound_mcp_tool_error_issue(issue);
            truncated |= issue_truncated;
            issue
        })
        .collect();
    if structured.issues.is_empty() {
        structured.issues.push(McpToolErrorIssue::new(
            String::new(),
            McpToolIssueCode::AdapterPreconditionFailed,
            "Tool execution failed before reaching Core.",
        ));
        truncated = true;
    }

    loop {
        structured.reported_issue_count = structured.issues.len();
        structured.truncated = truncated;
        let result = serialize_tool_error_result(&structured, capabilities);
        let result_bytes = serde_json::to_vec(&result)
            .expect("MCP tool error result should serialize")
            .len();
        if result_bytes <= MAX_MCP_TOOL_ERROR_RESULT_BYTES {
            return result;
        }
        if structured.issues.len() > 1 {
            structured.issues.pop();
            truncated = true;
            continue;
        }

        // Individual field limits and known tool names make this fallback
        // unreachable in normal operation, but keep the byte contract closed
        // if surrounding JSON overhead changes later.
        structured.issues[0].path.clear();
        structured.issues[0].message = "Validation failed before reaching Core.".to_owned();
        structured.truncated = true;
        let fallback = serialize_tool_error_result(&structured, capabilities);
        assert!(
            serde_json::to_vec(&fallback)
                .expect("fallback MCP tool error result should serialize")
                .len()
                <= MAX_MCP_TOOL_ERROR_RESULT_BYTES,
            "known-tool MCP error fallback exceeded its response byte limit"
        );
        return fallback;
    }
}

fn serialize_tool_error_result(
    structured: &McpToolErrorResponse,
    capabilities: McpProtocolCapabilities,
) -> Value {
    let structured_content =
        serde_json::to_value(structured).expect("MCP tool error should serialize");
    let text = serde_json::to_string(&structured_content)
        .expect("MCP tool error compatibility text should serialize");
    if capabilities.tools().structured_content() {
        validate_mcp_tool_output(&structured.tool_name, &structured_content)
            .expect("bounded MCP tool error should match advertised output schema");
    }
    CanonicalToolResult {
        metadata: None,
        content: vec![CanonicalContent::Text(text)],
        structured_content,
        is_error: true,
    }
    .project(capabilities)
    .expect("bounded MCP tool error should project")
    .into_value()
}

#[cfg(test)]
mod mutation_projection_and_recovery_tests {
    use super::*;
    use volicord_mcp_protocol::ToolResultCarrier;
    use volicord_store::evidence_capture::{
        EvidenceCaptureReceiptInsert, StoredEvidenceCaptureReceiptMetadata,
    };
    use volicord_test_support::core_fixtures::{
        CoreFixture, UpdateScopeFixture, UserActionFixture,
    };
    use volicord_types::canonical::canonical_json_bare_sha256;
    use volicord_types::ids::{
        AcceptanceCriterionId, BaselineRef, ChangeUnitId, EvidenceCaptureIntentId, RecordId,
        ShapingCheckpointId,
    };
    use volicord_types::methods::{
        AdvanceTaskRequest, RecordShapingRequest, MAX_OPERATION_RESULT_PAGE_BYTES,
    };
    use volicord_types::schema::{
        EvidenceCaptureSpec, EvidenceObservationInput, EvidenceProducer, EvidenceTarget,
        JsonObject, PersistedEvidenceCaptureReceiptBody, PersistedEvidenceCaptureReceiptSource,
        EVIDENCE_CAPTURE_COMMAND_LIMITATION,
    };
    use volicord_types::values::{
        ActorSource, ChangeUnitOperation, EvidenceAssuranceLevel, EvidenceProducerKind,
        EvidenceSourceKind, JudgmentKind, RedactionState, StateRecordKind, UtcTimestamp,
    };

    fn pad_valid_intake_result(result: &mut Value, bytes: usize) {
        let label = result
            .pointer_mut("/state/scope_summary")
            .expect("committed intake result should expose a scope summary");
        *label = Value::String("x".repeat(bytes));
    }

    #[test]
    fn compact_projection_rejects_an_effect_not_owned_by_the_source_method(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-exact-result-effect")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let committed = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_exact_result_effect",
                "idem_mcp_exact_result_effect",
                false,
                Some(0),
            ),
            test_agent_invocation(&fixture, OperationCategory::AgentWorkflow),
        )?;
        let mut tampered = committed.response_value;
        tampered["base"]["effect_kind"] = Value::String("staging_created".to_owned());
        tampered["base"]["events"] = Value::Array(Vec::new());

        assert!(matches!(
            compact_mutation_method_result(AgentToolId::INTAKE.wire_name(), &tampered),
            Err(McpAdapterError::Json(_))
        ));
        Ok(())
    }

    #[test]
    fn operational_unavailability_projects_through_every_supported_profile() {
        for profile in ProtocolRegistry::production().oldest_to_newest() {
            let capabilities = profile.capabilities();
            let core_error = CorePipelineError::from(volicord_store::StoreError::NotFound {
                entity: "project_state_database",
                id: "bounded-fixture-identity".to_owned(),
            });
            let CorePipelineError::OperationalUnavailable(failure) = core_error else {
                panic!("Store unavailability should remain a neutral Core error");
            };
            let output =
                ToolCallOutput::from_core_operational_unavailable(MethodName::Status, &failure)
                    .expect("operational failure should serialize");
            let projected = tool_call_result_from_output_for_capabilities(
                MethodName::Status.as_str(),
                output,
                capabilities,
            )
            .expect("supported profile should project operational failure");
            let structured = match capabilities.tools().result_carrier() {
                ToolResultCarrier::DirectToolResult => projected["toolResult"].clone(),
                ToolResultCarrier::JsonTextContent => serde_json::from_str(
                    projected["content"][0]["text"]
                        .as_str()
                        .expect("JSON text carrier must contain text"),
                )
                .expect("JSON text carrier must contain the structured failure"),
                ToolResultCarrier::StructuredContentWithText => {
                    projected["structuredContent"].clone()
                }
            };

            assert_eq!(structured["code"], "MCP_UNAVAILABLE");
            assert_eq!(structured["tool_name"], MethodName::Status.as_str());
            assert_eq!(structured["operation"], "store_access");
            assert_eq!(structured["resource"], "project_store");
            assert_eq!(structured["retryable"], true);
            assert_eq!(structured["reached_core"], true);
            assert_eq!(structured["committed"], false);
            assert_eq!(
                projected.get("isError").and_then(Value::as_bool),
                capabilities.tools().is_error().then_some(true)
            );
        }
    }

    #[test]
    fn product_path_operational_identity_projects_without_adapter_fallback() {
        let projected = serde_json::to_value(McpOperationalFailure {
            code: McpOperationalErrorCode::Unavailable,
            tool_name: MethodName::PrepareWrite,
            operation: mcp_operational_operation(CoreOperationalOperation::ProductPathObservation),
            resource: mcp_operational_resource(CoreOperationalResource::ProductRepository),
            retryable: true,
            reached_core: true,
            committed: false,
        })
        .expect("typed MCP operational failure");

        assert_eq!(projected["code"], "MCP_UNAVAILABLE");
        assert_eq!(projected["operation"], "product_path_observation");
        assert_eq!(projected["resource"], "product_repository");
        assert_eq!(projected["reached_core"], true);
        assert_eq!(projected["committed"], false);
    }

    fn test_agent_invocation(
        fixture: &CoreFixture,
        operation_category: OperationCategory,
    ) -> InvocationContext {
        let guard = guard_health_record(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
        )
        .expect("guard authority fixture must load");
        let session = volicord_test_support::seed_test_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
            guard
                .guard_installation
                .as_ref()
                .map(|installation| installation.guard_installation_id.as_str()),
        )
        .expect("managed Agent Session fixture must seed");
        let validated = CoreService::for_read_only(fixture.runtime_home_path())
            .validate_agent_session(
                AgentConnectionId::new(fixture.connection_id()),
                ProjectId::new(fixture.project_id()),
                session.runtime_session_id,
                session.project_session_id,
                operation_category,
            )
            .expect("managed Agent Session fixture must validate");
        InvocationContext::agent_connection(operation_category, validated)
    }

    fn committed_intake_with_receipt(
        prefix: &str,
    ) -> Result<(CoreFixture, PipelineResponse, AuthorityReceipt), Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let invocation = || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let committed = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_recovery_order",
                "idem_mcp_recovery_order",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .as_ref()
            .expect("committed intake resolves a Task");
        let status = core.status(
            fixture.status_request("req_mcp_recovery_order_status", Some(task_id.as_str())),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let receipt = serde_json::from_value(status.response_value["authority_receipt"].clone())?;
        Ok((fixture, committed, receipt))
    }

    fn committed_record_run_with_capture_producer(
        prefix: &str,
    ) -> Result<
        (
            CoreFixture,
            PipelineResponse,
            PipelineResponse,
            StateRecordRef,
        ),
        Box<dyn Error>,
    > {
        let fixture = CoreFixture::new(prefix)?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workspace = GitWorkspaceContext {
            git_common_dir: fixture
                .product_repo_path()
                .join(".git")
                .to_string_lossy()
                .into_owned(),
            worktree_id: format!("sha256:{}", "1".repeat(64)),
            branch_ref: Some("refs/heads/mcp-producer-recovery".to_owned()),
            head_sha: Some("2".repeat(40)),
            workspace_fingerprint: format!("sha256:{}", "3".repeat(64)),
        };
        let workflow_invocation = || {
            test_agent_invocation(&fixture, OperationCategory::AgentWorkflow)
                .with_git_workspace_context(workspace.clone())
        };
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_producer_recovery_intake",
                "idem_mcp_producer_recovery_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let scope = core.update_scope(
            &fixture.mutation_context()?,
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_producer_recovery_scope",
                idempotency_key: "idem_mcp_producer_recovery_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Bind an actual evidence producer to compact recovery.",
            }),
            workflow_invocation(),
        )?;
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or("scope should expose the current Change Unit")?;
        let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
            ["acceptance_criterion_id"]
            .as_str()
            .ok_or("scope should expose the current acceptance criterion")?;
        let shaped = core.record_shaping(
            &fixture.mutation_context()?,
            RecordShapingRequest {
                envelope: fixture.envelope(
                    "req_mcp_producer_recovery_shaping",
                    Some("idem_mcp_producer_recovery_shaping"),
                    false,
                    Some(2),
                    Some(task_id.as_str()),
                ),
                task_id: task_id.clone(),
                operation: volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
                    checkpoint_operation:
                        volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
                    scope_revision: 1,
                    baseline_ref: RequiredNullable::some(BaselineRef::new(
                        volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                    )),
                    summary: "The evidence-producer recovery boundary is ready.".to_owned(),
                    implementation_boundary: RequiredNullable::some(
                        "Record only the scoped evidence producer.".to_owned(),
                    ),
                    gaps: Vec::new(),
                    source_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                },
            },
            workflow_invocation(),
        )?;
        let shaping_checkpoint_id = shaped.response_value["shaping_checkpoint"]
            ["shaping_checkpoint_id"]
            .as_str()
            .ok_or("record_shaping should expose its checkpoint")?;
        core.advance_task(
            &fixture.mutation_context()?,
            AdvanceTaskRequest {
                envelope: fixture.envelope(
                    "req_mcp_producer_recovery_advance",
                    Some("idem_mcp_producer_recovery_advance"),
                    false,
                    Some(3),
                    Some(task_id.as_str()),
                ),
                task_id: task_id.clone(),
                shaping_checkpoint_id: ShapingCheckpointId::new(shaping_checkpoint_id),
                change_unit_id: ChangeUnitId::new(change_unit_id),
                scope_revision: 1,
                baseline_ref: BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                ),
                user_action_resolution_ids: Vec::new(),
            },
            workflow_invocation(),
        )?;
        let target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(criterion_id),
        };
        let prepared = core.prepare_evidence_capture(
            &fixture.mutation_context()?,
            PrepareEvidenceCaptureRequest {
                envelope: fixture.envelope(
                    "req_mcp_producer_recovery_prepare",
                    Some("idem_mcp_producer_recovery_prepare"),
                    false,
                    Some(4),
                    Some(task_id.as_str()),
                ),
                task_id: task_id.clone(),
                change_unit_id: ChangeUnitId::new(change_unit_id),
                baseline_ref: BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                ),
                target: target.clone(),
                capture: EvidenceCaptureSpec::VerifiedCommandExecution {
                    command_sha256: "4".repeat(64),
                    command_label: "actual compact producer fixture".to_owned(),
                    expected_exit_code: RequiredNullable::null(),
                },
            },
            workflow_invocation(),
        )?;
        let capture_intent_ref: StateRecordRef =
            serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;

        let mutation_context = fixture.mutation_context()?;
        let mut store = CoreProjectStore::open_for_mutation(
            &mutation_context,
            &ProjectId::new(fixture.project_id()),
        )?;
        let intent = store
            .evidence_capture_intent_record(capture_intent_ref.record_id.as_str())?
            .expect("committed capture intent should be readable");
        let observed_outcome: JsonObject = serde_json::from_value(json!({
            "exit_code": 0,
            "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "stdout_size_bytes": 0,
            "stderr_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "stderr_size_bytes": 0
        }))?;
        let result_sha256 = canonical_json_bare_sha256(&observed_outcome)?;
        let source: PersistedEvidenceCaptureReceiptSource = serde_json::from_value(json!({
            "connection_id": fixture.connection_id(),
            "host_invocation_id": "host_invocation_mcp_producer_recovery"
        }))?;
        let limitations = vec![EVIDENCE_CAPTURE_COMMAND_LIMITATION.to_owned()];
        let safe_receipt = PersistedEvidenceCaptureReceiptBody {
            contract_id: volicord_types::schema::EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID.to_owned(),
            capture_kind: EvidenceProducerKind::VerifiedCommandExecution,
            capture_intent_id: EvidenceCaptureIntentId::new(capture_intent_ref.record_id.as_str()),
            input_sha256: intent.input_sha256.clone(),
            result_sha256: result_sha256.clone(),
            expected_outcome: intent.expected_outcome.clone(),
            observed_outcome: observed_outcome.clone(),
            source: source.clone(),
            complete: true,
            limitations: limitations.clone(),
            redaction_state: RedactionState::Redacted,
            observed_by_actor_source: fixture.actor_source().parse::<ActorSource>()?,
            observed_at: intent.created_at.clone(),
        };
        store.fulfill_evidence_capture_source(EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: "evidence_capture_receipt_mcp_producer_recovery"
                .to_owned(),
            evidence_capture_intent_id: capture_intent_ref.record_id.as_str().to_owned(),
            staging_handle_id: "staged_capture_receipt_mcp_producer_recovery".to_owned(),
            task_id: intent.task_id.clone(),
            input_sha256: intent.input_sha256.clone(),
            result_sha256: result_sha256.clone(),
            expected_outcome: intent.expected_outcome.clone(),
            observed_outcome,
            source_refs: Vec::new(),
            observed_by_actor_source: fixture.actor_source().parse::<ActorSource>()?,
            observed_at: intent.created_at.clone(),
            limitations,
            safe_receipt,
            created_at: intent.created_at.clone(),
            staging_expires_at: intent.expires_at.clone(),
            metadata: StoredEvidenceCaptureReceiptMetadata { source },
        })?;
        drop(store);

        let mut record_request = fixture.record_run_request(
            "req_mcp_producer_recovery_record",
            "idem_mcp_producer_recovery_record",
            false,
            Some(5),
            task_id.as_str(),
            change_unit_id,
        );
        record_request.evidence_observations = vec![EvidenceObservationInput {
            target,
            source_kind: EvidenceSourceKind::ExternalTool,
            assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
            observed_by_actor_source: RequiredNullable::null(),
            tool_name: RequiredNullable::null(),
            tool_invocation_id: RequiredNullable::null(),
            tool_metadata: Map::new(),
            input_refs: vec![capture_intent_ref.clone()],
            source_refs: Vec::new(),
            output_artifact_refs: Vec::new(),
            limitations: Vec::new(),
            observed_at: UtcTimestamp::parse("2000-01-01T00:00:00Z")?,
        }];
        let recorded = core.record_run(
            &fixture.mutation_context()?,
            record_request,
            workflow_invocation(),
        )?;
        let producer: EvidenceProducer =
            serde_json::from_value(recorded.response_value["evidence_producers"][0].clone())?;
        let producer_id = producer.evidence_producer_id.as_str().to_owned();
        let producer_row = fixture
            .store()?
            .evidence_producer_record(&producer_id)?
            .expect("record_run producer should be immediately readable");
        assert_eq!(
            producer_row.evidence_capture_intent_id,
            capture_intent_ref.record_id.as_str()
        );
        let state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .ok_or("record_run should expose its committed state version")?;
        let producer_ref = StateRecordRef {
            record_kind: StateRecordKind::EvidenceProducer,
            record_id: RecordId::new(producer_id),
            project_id: producer.project_id,
            task_id: Some(producer.task_id).into(),
            produced_at_state_version: Some(state_version).into(),
        };

        let refreshed = core.status(
            fixture.status_request("req_mcp_producer_recovery_status", Some(task_id.as_str())),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        Ok((fixture, recorded, refreshed, producer_ref))
    }

    fn receipt_with_message_padding(
        receipt: &AuthorityReceipt,
        padding_bytes: usize,
    ) -> AuthorityReceipt {
        let mut value = serde_json::to_value(receipt).expect("receipt should serialize");
        value["close_blockers"][0]["message"] = Value::String("x".repeat(padding_bytes));
        serde_json::from_value(value).expect("padded receipt should remain valid")
    }

    fn recovery_facts() -> ToolDiagnosticFacts {
        ToolDiagnosticFacts {
            core_reached: true,
            core_committed: true,
            effect_kind: Some(EffectKind::CoreCommitted),
            effect_applied: true,
            effect_anchor: Some("authority_event:event_recovery_order".to_owned()),
            ..ToolDiagnosticFacts::default()
        }
    }

    fn recovery_operation_result_ref() -> OperationResultRef {
        OperationResultRef {
            project_id: ProjectId::new("project_mcp_recovery_order"),
            source_method: MethodName::Intake,
            source_idempotency_key: IdempotencyKey::new("idem_mcp_recovery_order"),
            committed_state_version: 1,
            response_sha256: format!("sha256:{}", "a".repeat(64)),
            response_size_bytes: 1_024,
        }
    }

    fn recovery_outcome(
        tool_name: &str,
        requested_detail: MutationDetailLevel,
        authority_receipt: Option<AuthorityReceipt>,
        exact_method_result: Option<Value>,
        compact_method_result: Option<Value>,
    ) -> CanonicalMcpMutationOutcome {
        CanonicalMcpMutationOutcome {
            tool_name: tool_name.to_owned(),
            capabilities: ProtocolRegistry::production()
                .preferred_server_profile()
                .capabilities(),
            requested_detail,
            facts: recovery_facts(),
            exact_method_result,
            compact_method_result,
            operation_result_ref: Some(recovery_operation_result_ref()),
            authority_receipt,
            workflow: None,
        }
    }

    fn assert_compact_budget(output: ToolCallOutput) -> Result<(), Box<dyn Error>> {
        assert!(!output.is_error);
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["effect_anchor"],
            "authority_event:event_recovery_order"
        );
        assert_eq!(
            output.structured_content["operation_result_ref"],
            serde_json::to_value(recovery_operation_result_ref())?
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn operation_result_page_keeps_escape_heavy_chunk_out_of_bounded_compatibility_text(
    ) -> Result<(), Box<dyn Error>> {
        let chunk_utf8 = "\\\"".repeat(MAX_OPERATION_RESULT_PAGE_BYTES / 2);
        assert_eq!(chunk_utf8.len(), MAX_OPERATION_RESULT_PAGE_BYTES);
        let response_value = json!({
            "base": { "response_kind": "result" },
            "start_offset_bytes": 0,
            "end_offset_bytes": MAX_OPERATION_RESULT_PAGE_BYTES,
            "chunk_utf8": chunk_utf8,
            "complete": false
        });
        let response = PipelineResponse {
            response_json: serde_json::to_string(&response_value)?,
            response_value,
            operation_result_ref: None,
            verified_invocation: None,
            resolved_task_id: None,
            replayed: false,
        };

        let output = ToolCallOutput::from_operation_result_response_for_test(&response)?;

        assert!(output.primary_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
        assert!(!output.primary_text.contains(&chunk_utf8));
        assert!(rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        Ok(())
    }

    #[test]
    fn idempotent_mutation_replay_default_summary_returns_refreshed_authority_receipt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-mutation-replay-summary")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let request = fixture.intake_request(
            "req_mcp_mutation_replay_summary",
            "idem_mcp_mutation_replay_summary",
            false,
            Some(0),
        );
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);

        let committed = core.intake(
            &fixture.mutation_context()?,
            request.clone(),
            workflow_invocation(),
        )?;
        assert!(!committed.replayed);
        let replayed = core.intake(&fixture.mutation_context()?, request, workflow_invocation())?;
        assert!(replayed.replayed);
        let task_id = replayed
            .resolved_task_id
            .clone()
            .expect("replay preserves the resolved Task identity");

        let detail = mutation_detail_for_tool(AgentToolId::INTAKE, &json!({}));
        assert_eq!(detail, Some(MutationDetailLevel::Summary));
        assert_eq!(
            mutation_detail_for_tool(AgentToolId::BEGIN_INTEGRATION_VERIFICATION, &json!({})),
            None,
            "Connection-integration writes do not use Core mutation projection"
        );
        assert_eq!(
            mutation_detail_for_tool(AgentToolId::GUARD_PROBE, &json!({})),
            None,
            "the bounded Guard probe does not use Core mutation projection"
        );
        let output = ToolCallOutput::from_pipeline_response(&replayed)?;
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            detail,
            output,
            |context| {
                assert_eq!(context.project_id.as_str(), fixture.project_id());
                assert_eq!(context.task_id, task_id);
                core.status(
                    fixture.status_request(
                        "req_mcp_mutation_replay_summary_refresh",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert!(output.diagnostic_facts.replayed);
        assert!(output.diagnostic_facts.core_reached);
        assert!(!output.diagnostic_facts.core_committed);
        assert_eq!(
            output.structured_content["authority_receipt"]["project_id"],
            fixture.project_id()
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert!(output.structured_content["authority_receipt"]["state_version"].is_u64());
        assert_eq!(
            output.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_eq!(
            output.structured_content["presentation"]["state_change"],
            "read_only_resume"
        );
        assert!(output
            .primary_text
            .contains("resumed current authority without mutation"));
        assert!(!output.primary_text.contains("committed Core authority"));
        assert!(output.structured_content.get("code").is_none());
        assert!(output
            .structured_content
            .get("completion_claim_withheld")
            .is_none());
        Ok(())
    }

    #[test]
    fn full_projection_pairs_exact_method_result_with_newer_fresh_receipt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-full-fresh-receipt")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_full_fresh_receipt",
                "idem_mcp_full_fresh_receipt",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .as_ref()
            .expect("intake should resolve a Task")
            .clone();
        let original_method_result = intake.response_value.clone();
        core.update_scope(
            &fixture.mutation_context()?,
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_full_fresh_receipt_scope",
                idempotency_key: "idem_mcp_full_fresh_receipt_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance authority after the original method result.",
            }),
            workflow_invocation(),
        )?;

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&intake)?,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_full_fresh_receipt_status",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["method_result"],
            original_method_result
        );
        assert_eq!(
            output.structured_content["method_result"]["base"]["state_version"],
            1
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            2
        );
        Ok(())
    }

    #[test]
    fn refresh_failure_withholds_success_and_does_not_return_private_error_body() {
        let private_error = "private-refresh-owner-body-must-not-escape";
        let mut output = ToolCallOutput::success(
            json!({
                "base": {
                    "response_kind": "result",
                    "effect_kind": "core_committed"
                }
            })
            .to_string(),
        )
        .expect("tool output");
        output.diagnostic_facts.core_reached = true;
        output.diagnostic_facts.core_committed = true;
        output.diagnostic_facts.effect_kind = Some(EffectKind::CoreCommitted);
        output.diagnostic_facts.effect_applied = true;
        output.diagnostic_facts.effect_anchor =
            Some("authority_event:event_refresh_failure".to_owned());
        output.mutation_refresh_context = Some(MutationRefreshContext {
            project_id: ProjectId::new("project_refresh_failure"),
            task_id: TaskId::new("task_refresh_failure"),
        });

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |_| Err(McpAdapterError::Environment(private_error.to_owned())),
        )
        .expect("fail-closed output");

        assert!(!output.is_error);
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["effect_anchor"],
            "authority_event:event_refresh_failure"
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        assert!(output.diagnostic_facts.authoritative_refresh_failure);
        let rendered =
            serde_json::to_string(&tool_call_result_from_output(output)).expect("rendered result");
        assert!(!rendered.contains(private_error));
        assert!(!rendered.contains("response_kind\":\"result"));
    }

    #[test]
    fn refresh_freshness_mismatch_uses_same_non_retryable_failure_boundary(
    ) -> Result<(), Box<dyn Error>> {
        let (fixture, committed, _) =
            committed_intake_with_receipt("mcp-refresh-freshness-mismatch")?;
        let core = CoreService::for_read_only(fixture.runtime_home_path());
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let expected_compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &committed.response_value,
        )?;
        let mut mismatched_refresh = core.status(
            fixture.status_request(
                "req_mcp_refresh_freshness_mismatch_status",
                Some(task_id.as_str()),
            ),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        mismatched_refresh.response_value["authority_receipt"]["state_version"] = json!(999);

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Summary),
            ToolCallOutput::from_pipeline_response(&committed)?,
            |_| Ok(mismatched_refresh),
        )?;

        assert!(!output.is_error);
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["method_result"], expected_compact);
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        assert!(output.diagnostic_facts.authoritative_refresh_failure);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn post_effect_adapter_failure_refreshes_authority_without_recommending_replay(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-adapter-failure")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let invocation = || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let committed = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_post_effect_adapter_failure",
                "idem_mcp_post_effect_adapter_failure",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let refreshed = core.status(
            fixture.status_request(
                "req_mcp_post_effect_adapter_failure_status",
                Some(task_id.as_str()),
            ),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let expected_compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &committed.response_value,
        )?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        pad_valid_intake_result(
            &mut output.structured_content,
            MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
        );
        let output =
            output.with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["method_result"], expected_compact);
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert_eq!(
            output.structured_content["authoritative_refresh_succeeded"],
            true
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn superseded_user_action_projects_latest_state_and_preserves_the_origin_effect(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-user-action-record-race")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_user_action_record_race_intake",
                "idem_mcp_user_action_record_race_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let pending_response = core.request_user_action(
            &fixture.mutation_context()?,
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_user_action_record_race_pending",
                idempotency_key: "idem_mcp_user_action_record_race_pending",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        core.update_scope(
            &fixture.mutation_context()?,
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_user_action_record_race_scope",
                idempotency_key: "idem_mcp_user_action_record_race_scope",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance state while the user action remains pending.",
            }),
            workflow_invocation(),
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);

        let output =
            user_action_tool_output(&fixture.mutation_context()?, &adapter, pending_response)?;
        assert_eq!(output.post_effect_failure, None);
        assert_eq!(
            output.structured_content["agent_workflow_result"]["user_action_request_summary"]
                ["status"],
            "pending"
        );
        assert_eq!(output.structured_content["current_status"], "superseded");
        assert!(output.structured_content["user_channel_resolution"].is_null());
        assert_eq!(output.structured_content["derived_refs"], json!([]));
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_user_action_record_race_status",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert!(output.structured_content.get("code").is_none());
        assert_eq!(
            output.structured_content["operation_result_ref"]["source_method"],
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
        assert_eq!(
            output.structured_content["method_result"]["status"],
            "superseded"
        );
        assert_eq!(
            output.structured_content["method_result"]["agent_workflow_result_replayed"],
            false
        );
        assert_eq!(
            output.structured_content["method_result"]["derived_refs"],
            json!([])
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            3
        );
        assert!(output
            .structured_content
            .get("completion_claim_withheld")
            .is_none());
        Ok(())
    }

    #[test]
    fn mismatched_safe_summary_routes_to_closed_post_effect_recovery() -> Result<(), Box<dyn Error>>
    {
        let fixture = CoreFixture::new("mcp-noncanonical-pending-post-effect")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_noncanonical_pending_intake",
                "idem_mcp_noncanonical_pending_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let mut pending_response = core.request_user_action(
            &fixture.mutation_context()?,
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_noncanonical_pending_action",
                idempotency_key: "idem_mcp_noncanonical_pending_action",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        let before = fixture.counts()?;
        pending_response.response_value["user_action_request_summary"]["user_action_request_id"] =
            json!("uar_not_in_trusted_projection");
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
        let error = user_action_tool_output(
            &fixture.mutation_context()?,
            &adapter,
            pending_response.clone(),
        )
        .expect_err("mismatched public summary must fail during current-state reread");
        assert!(matches!(error, McpAdapterError::Protocol(_)));
        assert_eq!(fixture.counts()?, before);

        let output = ToolCallOutput::from_pipeline_response(&pending_response)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_noncanonical_pending_status",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;
        let recovery: McpMutationPostEffectFailure =
            serde_json::from_value(output.structured_content.clone())?;
        assert_eq!(
            recovery.code,
            McpPostEffectFailureCode::McpPostEffectAdapterFailed
        );
        assert!(!recovery.retryable);
        assert!(recovery.reached_core);
        assert!(recovery.committed);
        assert!(recovery.effect_applied);
        assert!(recovery.response_projection_omitted);
        assert!(recovery.status_read_required);
        assert!(recovery.completion_claim_withheld);
        assert_eq!(
            output.structured_content["method_result"]["user_action_request_summary"]["status"],
            "pending"
        );
        assert!(output
            .structured_content
            .get("agent_workflow_result")
            .is_none());
        assert_eq!(fixture.counts()?, before);
        Ok(())
    }

    #[test]
    fn projection_failure_preserves_effect_facts_and_exact_method_result(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-projection-failure")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let invocation = || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let committed = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_post_effect_projection_failure",
                "idem_mcp_post_effect_projection_failure",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let refreshed = core.status(
            fixture.status_request(
                "req_mcp_post_effect_projection_failure_status",
                Some(task_id.as_str()),
            ),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        output.structured_content["base"]["effect_kind"] = json!("invalid_projection_fixture");
        let exact_unprojectable_result = output.structured_content.clone();
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_PROJECTION_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["method_result"],
            exact_unprojectable_result
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn staging_refresh_failure_reports_applied_handle_as_non_retryable_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-staging-refresh-failure")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_staging_refresh_failure",
                "idem_mcp_staging_refresh_failure",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .as_ref()
            .expect("intake should resolve a Task")
            .clone();
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake should report state version");
        let staged = core.stage_artifact(
            &fixture.mutation_context()?,
            fixture.stage_artifact_request(
                "req_mcp_staging_refresh_failure_stage",
                None,
                false,
                Some(state_version),
                task_id.as_str(),
            ),
            workflow_invocation(),
        )?;
        let handle_id = staged.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("stage result should include a handle")
            .to_owned();
        let output = ToolCallOutput::from_pipeline_response(&staged)?;
        assert_eq!(
            output.diagnostic_facts.effect_kind,
            Some(EffectKind::StagingCreated)
        );
        assert!(output.diagnostic_facts.effect_applied);
        assert!(!output.diagnostic_facts.core_committed);
        let effect_anchor = format!("staged_artifact:{handle_id}");
        assert_eq!(
            output.diagnostic_facts.effect_anchor.as_deref(),
            Some(effect_anchor.as_str())
        );

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |_| {
                Err(McpAdapterError::Environment(
                    "refresh unavailable".to_owned(),
                ))
            },
        )?;

        assert!(!output.is_error);
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["committed"], false);
        assert_eq!(output.structured_content["effect_kind"], "staging_created");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["effect_anchor"], effect_anchor);
        assert_eq!(
            output.structured_content["method_result"]["staged_artifact_handle"]["handle_id"],
            handle_id
        );
        assert!(output.structured_content["method_result"]["expires_at"].is_string());
        assert_eq!(output.structured_content["status_read_required"], true);
        Ok(())
    }

    #[test]
    fn oversized_valid_projection_preserves_effect_and_refresh_truth_within_each_budget(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-mutation-oversized-fresh-receipt")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let committed = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_mutation_oversized_fresh_receipt",
                "idem_mcp_mutation_oversized_fresh_receipt",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves the Task");
        let mut refreshed = core.status(
            fixture.status_request(
                "req_mcp_mutation_oversized_fresh_receipt_status",
                Some(task_id.as_str()),
            ),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("fresh intake status should expose a close blocker");
        let omitted_marker = "oversized-valid-criterion-blocker-must-not-escape";
        blocker["message"] = Value::String(format!(
            "{omitted_marker}{}",
            "x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2)
        ));
        let oversized_blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] =
            oversized_blockers.clone();
        refreshed.response_value["active_task"]["close_blockers"] = oversized_blockers.clone();
        refreshed.response_value["close_blockers"] = oversized_blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        for detail in [
            MutationDetailLevel::Summary,
            MutationDetailLevel::Workflow,
            MutationDetailLevel::Full,
        ] {
            let output = ToolCallOutput::from_pipeline_response(&committed)?;
            let refreshed = refreshed.clone();
            let output = finalize_mutation_output_with_refresh(
                AgentToolId::INTAKE.wire_name(),
                Some(detail),
                output,
                |_| Ok(refreshed),
            )?;

            assert!(!output.is_error);
            assert_eq!(
                output.structured_content["code"],
                "MCP_RESPONSE_BUDGET_EXCEEDED"
            );
            assert_eq!(
                output.structured_content["requested_detail"],
                serde_json::to_value(detail)?
            );
            assert_eq!(output.structured_content["retryable"], false);
            assert_eq!(output.structured_content["reached_core"], true);
            assert_eq!(output.structured_content["committed"], true);
            assert_eq!(output.structured_content["effect_kind"], "core_committed");
            assert_eq!(output.structured_content["effect_applied"], true);
            assert!(output.structured_content["effect_anchor"]
                .as_str()
                .is_some_and(|token| token.starts_with("authority_event:")));
            assert!(output.structured_content["operation_result_ref"].is_object());
            assert_eq!(
                output.structured_content["authoritative_refresh_succeeded"],
                true
            );
            assert_eq!(
                output.structured_content["response_projection_omitted"],
                true
            );
            assert_eq!(output.structured_content["status_read_required"], true);
            assert_eq!(output.structured_content["completion_claim_withheld"], true);
            assert_eq!(
                output.structured_content["method_result"]["effect_kind"],
                "core_committed"
            );
            assert!(!output.diagnostic_facts.authoritative_refresh_failure);

            let rendered = serde_json::to_vec(&tool_call_result_from_output(output))?;
            assert!(rendered.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
            assert!(!String::from_utf8(rendered)?.contains(omitted_marker));
        }

        let output = ToolCallOutput::from_pipeline_response(&committed)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;
        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["authority_receipt"], Value::Null);
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn post_effect_recovery_preserves_receipt_then_compact_result_then_effect_facts(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, committed, receipt) =
            committed_intake_with_receipt("mcp-post-effect-recovery-order")?;

        let compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &committed.response_value,
        )?;
        let both_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt.clone()),
            Some(committed.response_value.clone()),
            Some(compact),
        );
        let both = mutation_post_effect_failure_output(
            &both_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert!(both.structured_content["authority_receipt"].is_object());
        assert!(both.structured_content["method_result"].is_object());
        assert_compact_budget(both)?;

        let mut oversized_exact_result = committed.response_value.clone();
        pad_valid_intake_result(
            &mut oversized_exact_result,
            MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
        );
        let expected_compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &oversized_exact_result,
        )?;
        let receipt_and_compact_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt.clone()),
            Some(oversized_exact_result.clone()),
            Some(expected_compact.clone()),
        );
        let receipt_and_compact = mutation_post_effect_failure_output(
            &receipt_and_compact_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert!(receipt_and_compact.structured_content["authority_receipt"].is_object());
        assert_eq!(
            receipt_and_compact.structured_content["method_result"],
            expected_compact
        );
        assert_compact_budget(receipt_and_compact)?;

        let compact_only_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(oversized_exact_result),
            Some(expected_compact),
        );
        let compact_only = mutation_post_effect_failure_output(
            &compact_only_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert_eq!(
            compact_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            compact_only.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(compact_only)?;

        let mut unprojectable_result = committed.response_value;
        unprojectable_result["base"] = Value::String("invalid".to_owned());
        unprojectable_result["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let effect_facts_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(unprojectable_result),
            None,
        );
        let effect_facts_only = mutation_post_effect_failure_output(
            &effect_facts_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert_eq!(
            effect_facts_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            effect_facts_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(effect_facts_only)?;
        Ok(())
    }

    #[test]
    fn post_effect_recovery_budget_table_uses_canonical_candidate_priority(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-post-effect-recovery-budget-table")?;
        let small_exact = json!({"projection_marker": "exact"});
        let large_exact = json!({
            "projection_marker": "exact",
            "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
        });
        let small_compact = json!({"projection_marker": "compact"});
        let large_compact = json!({
            "projection_marker": "compact",
            "padding": "한".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
        });
        let oversized_receipt =
            receipt_with_message_padding(&receipt, MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        let cases = vec![
            (
                "receipt_exact",
                receipt.clone(),
                small_exact.clone(),
                small_compact.clone(),
                true,
                Some("exact"),
            ),
            (
                "receipt_compact",
                receipt.clone(),
                large_exact.clone(),
                small_compact.clone(),
                true,
                Some("compact"),
            ),
            (
                "receipt_only",
                receipt,
                large_exact.clone(),
                large_compact.clone(),
                true,
                None,
            ),
            (
                "compact_only",
                oversized_receipt.clone(),
                large_exact.clone(),
                small_compact,
                false,
                Some("compact"),
            ),
            (
                "effect_facts_only",
                oversized_receipt,
                large_exact,
                large_compact,
                false,
                None,
            ),
        ];

        for (name, receipt, exact, compact, expect_receipt, expected_marker) in cases {
            let outcome = recovery_outcome(
                AgentToolId::INTAKE.wire_name(),
                MutationDetailLevel::Summary,
                Some(receipt),
                Some(exact),
                Some(compact),
            );
            let output = mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )?;
            assert_eq!(
                output.structured_content["authority_receipt"].is_object(),
                expect_receipt,
                "{name} receipt preservation"
            );
            assert_eq!(
                output.structured_content["method_result"]["projection_marker"].as_str(),
                expected_marker,
                "{name} method-result preservation"
            );
            if expected_marker.is_none() {
                assert!(
                    output.structured_content["method_result"].is_null(),
                    "{name} must omit the complete method result"
                );
            }
            assert_compact_budget(output)?;
        }
        Ok(())
    }

    #[test]
    fn response_budget_recovery_preserves_receipt_then_compact_result_then_effect_facts(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-response-budget-recovery-order")?;
        let small_result = json!({"effect_kind": "core_committed"});

        let both_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Full,
            Some(receipt.clone()),
            None,
            Some(small_result.clone()),
        );
        let both = mutation_response_budget_exceeded_output(&both_outcome)?;
        assert!(both.structured_content["authority_receipt"].is_object());
        assert_eq!(
            both.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(both)?;

        let receipt_only_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(&receipt, 36 * 1024)),
            None,
            Some(json!({
                "effect_kind": "core_committed",
                "padding": "x".repeat(36 * 1024),
            })),
        );
        let receipt_only = mutation_response_budget_exceeded_output(&receipt_only_outcome)?;
        assert!(receipt_only.structured_content["authority_receipt"].is_object());
        assert_eq!(
            receipt_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(receipt_only)?;

        let compact_only_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            None,
            Some(small_result),
        );
        let compact_only = mutation_response_budget_exceeded_output(&compact_only_outcome)?;
        assert_eq!(
            compact_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            compact_only.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(compact_only)?;

        let effect_facts_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            None,
            Some(json!({
                "effect_kind": "core_committed",
                "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
            })),
        );
        let effect_facts_only = mutation_response_budget_exceeded_output(&effect_facts_outcome)?;
        assert_eq!(
            effect_facts_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            effect_facts_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(effect_facts_only)?;
        Ok(())
    }

    #[test]
    fn record_run_actual_producer_ref_survives_default_compact_and_bounded_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, recorded, refreshed, producer_ref) =
            committed_record_run_with_capture_producer("mcp-record-run-producer-recovery")?;
        let default_detail = MutationDetailLevel::default();
        assert_eq!(default_detail, MutationDetailLevel::Summary);
        let decode_producer_refs =
            |value: Value, label: &str| -> Result<Vec<StateRecordRef>, Box<dyn Error>> {
                if !value.is_array() {
                    return Err(format!("{label} did not preserve producer refs: {value}").into());
                }
                Ok(serde_json::from_value(value)?)
            };

        let normal = finalize_mutation_output_with_refresh(
            AgentToolId::RECORD_RUN.wire_name(),
            Some(default_detail),
            ToolCallOutput::from_pipeline_response(&recorded)?,
            |_| Ok(refreshed.clone()),
        )?;
        let default_refs = decode_producer_refs(
            normal.structured_content["method_result"]["evidence_producer_refs"].clone(),
            "default compact finalizer",
        )?;
        assert_eq!(default_refs, vec![producer_ref.clone()]);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(normal))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );

        let mut oversized_recorded = recorded.clone();
        oversized_recorded.response_value["run_summary"]["summary"] =
            Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES));
        oversized_recorded.response_json =
            serde_json::to_string(&oversized_recorded.response_value)?;

        let mut projection_output = ToolCallOutput::from_pipeline_response(&oversized_recorded)?;
        projection_output.post_effect_failure =
            Some(McpPostEffectFailureCode::McpResponseProjectionFailed);
        let projection_recovery = finalize_mutation_output_with_refresh(
            AgentToolId::RECORD_RUN.wire_name(),
            Some(default_detail),
            projection_output,
            |_| Ok(refreshed.clone()),
        )?;

        let budget_recovery = finalize_mutation_output_with_refresh(
            AgentToolId::RECORD_RUN.wire_name(),
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&oversized_recorded)?,
            |_| Ok(refreshed.clone()),
        )?;

        let recoveries = [
            (projection_recovery, "MCP_RESPONSE_PROJECTION_FAILED"),
            (budget_recovery, "MCP_RESPONSE_BUDGET_EXCEEDED"),
        ];
        for (recovery, expected_code) in recoveries {
            assert_eq!(recovery.structured_content["code"], expected_code);
            assert!(recovery.structured_content["authority_receipt"].is_object());
            let producer_refs = decode_producer_refs(
                recovery.structured_content["method_result"]["evidence_producer_refs"].clone(),
                expected_code,
            )?;
            assert_eq!(producer_refs, vec![producer_ref.clone()]);
            assert_eq!(recovery.structured_content["effect_applied"], true);
            assert_eq!(recovery.structured_content["committed"], true);
            assert_eq!(recovery.structured_content["retryable"], false);
            assert!(
                serde_json::to_vec(&tool_call_result_from_output(recovery))?.len()
                    <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            );
        }
        Ok(())
    }

    #[test]
    fn user_action_derived_refs_survive_compact_only_recovery_paths() -> Result<(), Box<dyn Error>>
    {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-user-action-derived-ref-recovery")?;
        let derived_ref = json!({
            "record_kind": "project_continuity_record",
            "record_id": "continuity_user_action_derived",
            "project_id": "project_mcp_recovery_order",
            "task_id": "task_mcp_recovery_order",
            "produced_at_state_version": 3
        });
        let compact = json!({
            "effect": {
                "effect_kind": "core_committed",
                "state_version": 3,
                "events": []
            },
            "agent_workflow_result_replayed": true,
            "user_action_request_summary": {
                "user_action_request_id": "user_action_request_recovery",
                "status": "pending",
                "next_actor": "user"
            },
            "user_action_resolution_ref": {
                "record_kind": "user_action_resolution",
                "record_id": "user_action_resolution_recovery",
                "project_id": "project_mcp_recovery_order",
                "task_id": "task_mcp_recovery_order",
                "produced_at_state_version": 3
            },
            "current_projection_state_version": 4,
            "current_projection_observed_at": "2026-07-13T12:00:00Z",
            "status": "resolved",
            "resolution_summary": {
                "resolution_type": "choice",
                "selected_option_id": "accept",
                "selected_option_label": "Accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted"
            },
            "derived_refs": [derived_ref.clone()]
        });
        let outcome = recovery_outcome(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(json!({
                "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES)
            })),
            Some(compact),
        );

        let outputs = [
            mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )?,
            mutation_response_budget_exceeded_output(&outcome)?,
            authoritative_refresh_failure_output(&outcome)?,
        ];
        for output in outputs {
            assert_eq!(output.structured_content["authority_receipt"], Value::Null);
            assert_eq!(
                output.structured_content["method_result"]["derived_refs"][0],
                derived_ref
            );
            assert_eq!(
                output.structured_content["method_result"]["agent_workflow_result_replayed"],
                true
            );
            assert_eq!(
                output.structured_content["method_result"]["current_projection_state_version"],
                4
            );
            assert_eq!(
                output.structured_content["method_result"]["current_projection_observed_at"],
                "2026-07-13T12:00:00Z"
            );
            assert_eq!(
                output.structured_content["method_result"]["status"],
                "resolved"
            );
            assert!(
                serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                    <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            );
        }
        Ok(())
    }

    #[test]
    fn oversized_compact_method_result_is_omitted_from_authoritative_refresh_failure(
    ) -> Result<(), Box<dyn Error>> {
        let facts = recovery_facts();
        let oversized_method_result = json!({
            "effect_kind": "core_committed",
            "oversized": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES * 2),
        });
        let outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            None,
            None,
            Some(oversized_method_result),
        );
        let mut outcome = outcome;
        outcome.facts = facts;
        let output = authoritative_refresh_failure_output(&outcome)?;
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["method_result"], Value::Null);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn oversized_stage_projection_preserves_the_staging_handle_in_bounded_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-stage-oversized-fresh-receipt")?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_mcp_stage_oversized_intake",
                "idem_mcp_stage_oversized_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake state version");
        let staged = core.stage_artifact(
            &fixture.mutation_context()?,
            fixture.stage_artifact_request(
                "req_mcp_stage_oversized_stage",
                None,
                false,
                Some(state_version),
                task_id.as_str(),
            ),
            workflow_invocation(),
        )?;
        let expected_handle = staged.response_value["staged_artifact_handle"].clone();
        let mut refreshed = core.status(
            fixture.status_request("req_mcp_stage_oversized_status", Some(task_id.as_str())),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("status exposes a close blocker");
        blocker["message"] = Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2));
        let blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] = blockers.clone();
        refreshed.response_value["active_task"]["close_blockers"] = blockers.clone();
        refreshed.response_value["close_blockers"] = blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            Some(MutationDetailLevel::Summary),
            ToolCallOutput::from_pipeline_response(&staged)?,
            |_| Ok(refreshed),
        )?;

        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_BUDGET_EXCEEDED"
        );
        assert_eq!(
            output.structured_content["method_result"]["effect"]["effect_kind"],
            "staging_created"
        );
        assert_eq!(
            output.structured_content["method_result"]["staged_artifact_handle"],
            expected_handle
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }
}
