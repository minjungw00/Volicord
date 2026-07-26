//! MCP diagnostic persistence and bounded telemetry.
//!
//! Authoritative lifecycle milestones remain Store-owned. This module owns the
//! adapter-side persistence calls and keeps diagnostics-carrier failures
//! best-effort where the contract requires that boundary.

use crate::adapter::McpAdapter;
use crate::diagnostics::{
    data_for_diagnostic, production_supported_revisions, McpDiagnostic, McpDiagnosticContext,
};
use crate::errors::McpAdapterError;
use crate::lifecycle::SessionRuntime;
use crate::mutation_admission::with_mcp_runtime_home_mutation;
use crate::tool_registry::method_name_for_tool;
use serde_json::Value;
use std::time::{Instant, SystemTime};
use volicord_store::agent_connections::{
    agent_connection_record_read_only, list_connection_projects_read_only,
};
use volicord_store::diagnostic_findings::insert_occurrence_finding;
use volicord_store::diagnostics::{
    record_diagnostic_event, record_workflow_metric_event, start_diagnostic_session,
    DiagnosticEvent, DiagnosticEventKind, DiagnosticFallbackKind, DiagnosticHostKind,
    DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport, WorkflowMetricEvent,
    WorkflowMetricKind, WorkflowMetricOutcome,
};
use volicord_store::error::StoreError;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_store::operational_sessions::{mcp_runtime_session, record_mcp_terminal_finding};
use volicord_types::diagnostics::OccurrenceDiagnosticFinding;
use volicord_types::ids::AgentRuntimeSessionId;
use volicord_types::values::{EffectKind, MethodName, UtcTimestamp};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ToolDiagnosticFacts {
    pub(crate) core_reached: bool,
    pub(crate) core_committed: bool,
    pub(crate) replayed: bool,
    pub(crate) effect_kind: Option<EffectKind>,
    pub(crate) effect_applied: bool,
    pub(crate) effect_anchor: Option<String>,
    pub(crate) fallback_kind: Option<DiagnosticFallbackKind>,
    pub(crate) product_file_write_count: u64,
    pub(crate) authoritative_refresh_failure: bool,
}

pub(crate) fn authoritative_observation_timestamp() -> String {
    UtcTimestamp::from_datetime(chrono::DateTime::<chrono::Utc>::from(SystemTime::now()))
        .to_canonical_string()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_current_session_finding_with_admission(
    adapter: &McpAdapter,
    runtime: &mut SessionRuntime,
    diagnostic: McpDiagnostic,
    json_rpc_error_code: Option<i64>,
    safe_error_data: Option<String>,
    tool_name: Option<String>,
    missing_tools: Vec<String>,
    terminal: bool,
) -> Result<(), McpAdapterError> {
    with_mcp_runtime_home_mutation(&adapter.runtime_home, "mcp.terminal_finding", |context| {
        record_current_session_finding(
            context,
            adapter,
            runtime,
            diagnostic,
            json_rpc_error_code,
            safe_error_data,
            tool_name,
            missing_tools,
            terminal,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_current_session_finding(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    runtime: &mut SessionRuntime,
    diagnostic: McpDiagnostic,
    json_rpc_error_code: Option<i64>,
    safe_error_data: Option<String>,
    tool_name: Option<String>,
    missing_tools: Vec<String>,
    terminal: bool,
) -> Result<(), McpAdapterError> {
    if runtime.runtime_session_id.is_empty() || (terminal && runtime.terminal_finding_recorded) {
        return Ok(());
    }
    let persisted = mcp_runtime_session(
        adapter.admitted_runtime_home(context)?,
        &runtime.runtime_session_id,
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| McpAdapterError::Protocol("MCP runtime session disappeared".to_owned()))?;
    let data = data_for_diagnostic(
        diagnostic,
        &McpDiagnosticContext {
            observed_at: UtcTimestamp::parse(&authoritative_observation_timestamp()).map_err(
                |_| McpAdapterError::Protocol("diagnostic timestamp is invalid".to_owned()),
            )?,
            connection_id: Some(persisted.connection_internal_id),
            integration_revision: Some(persisted.connection_integration_revision),
            runtime_session_id: Some(runtime.runtime_session_id.clone()),
            requested_revision: persisted.requested_protocol_version,
            selected_revision: persisted.selected_protocol_version,
            negotiated_revision: persisted.negotiated_protocol_version,
            supported_revisions: production_supported_revisions(),
            attempted_client_name: persisted.attempted_client_name,
            attempted_client_version: persisted.attempted_client_version,
            json_rpc_error_code,
            safe_error_data,
            tool_name,
            missing_tools,
        },
    )
    .map_err(|error| {
        McpAdapterError::Protocol(format!("structured diagnostic finding is invalid: {error}"))
    })?;
    let finding = OccurrenceDiagnosticFinding::try_new(
        data,
        Some(AgentRuntimeSessionId::new(
            runtime.runtime_session_id.clone(),
        )),
    )
    .map_err(|error| {
        McpAdapterError::Protocol(format!(
            "structured diagnostic occurrence is invalid: {error}"
        ))
    })?;
    if terminal {
        record_mcp_terminal_finding(context, &finding).map_err(McpAdapterError::Store)?;
        runtime.terminal_finding_recorded = true;
    } else {
        insert_occurrence_finding(context, &finding).map_err(McpAdapterError::Store)?;
    }
    Ok(())
}

fn stdio_diagnostic_project_id(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
) -> Result<Option<String>, McpAdapterError> {
    let runtime_home = adapter.admitted_runtime_home(context)?;
    let project_id = adapter
        .context
        .project_allowlist
        .as_ref()
        .filter(|projects| projects.len() == 1)
        .and_then(|projects| projects.first())
        .map(|project| project.as_str().to_owned())
        .or_else(|| {
            list_connection_projects_read_only(
                runtime_home,
                adapter.context.connection_internal_id.as_str(),
            )
            .ok()
            .filter(|projects| projects.len() == 1)
            .and_then(|projects| projects.first().map(|project| project.project_id.clone()))
        });
    Ok(project_id)
}

pub(crate) fn start_transport_diagnostic_session(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    runtime: &SessionRuntime,
) -> Result<(), StoreError> {
    let runtime_home =
        adapter
            .admitted_runtime_home(context)
            .map_err(|error| StoreError::InvalidInput {
                detail: error.to_string(),
            })?;
    let connection = agent_connection_record_read_only(
        runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .ok()
    .flatten();
    let host_kind = connection
        .as_ref()
        .and_then(|record| DiagnosticHostKind::from_connection_host_kind(&record.host_kind));
    let project_id = stdio_diagnostic_project_id(context, adapter).map_err(|error| {
        StoreError::InvalidInput {
            detail: error.to_string(),
        }
    })?;
    let build = crate::build_info();
    start_diagnostic_session(
        context,
        DiagnosticSessionStart {
            session_id: &runtime.runtime_session_id,
            connection_id: Some(adapter.context.connection_internal_id.as_str()),
            project_id: project_id.as_deref(),
            transport: DiagnosticTransport::McpStdio,
            host_kind,
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    )
}

pub(crate) fn record_tools_list_metric_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    runtime: &SessionRuntime,
    serialized_bytes: u64,
) {
    if runtime.codex_binding.is_pending()
        || start_transport_diagnostic_session(context, adapter, runtime).is_err()
    {
        return;
    }
    let _ = record_workflow_metric_event(
        context,
        &WorkflowMetricEvent {
            session_id: runtime.runtime_session_id.clone(),
            metric_kind: WorkflowMetricKind::ToolsListSerializedBytes,
            value: serialized_bytes,
            method_name: None,
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: Some(WorkflowMetricOutcome::Success),
        },
    );
}

fn record_public_method_metrics_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    runtime: &SessionRuntime,
    tool_name: Option<&str>,
    outcome: DiagnosticOutcome,
) {
    let Some(method_name) = tool_name.and_then(method_name_for_tool) else {
        return;
    };
    let outcome = workflow_metric_outcome(outcome);
    let _ = record_workflow_metric_event(
        context,
        &WorkflowMetricEvent {
            session_id: runtime.runtime_session_id.clone(),
            metric_kind: WorkflowMetricKind::McpMethodCall,
            value: 1,
            method_name: Some(method_name),
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: Some(outcome),
        },
    );
    if method_name == MethodName::Status && runtime.status_method_call_count > 1 {
        let _ = record_workflow_metric_event(
            context,
            &WorkflowMetricEvent {
                session_id: runtime.runtime_session_id.clone(),
                metric_kind: WorkflowMetricKind::StatusReread,
                value: 1,
                method_name: None,
                integration_profile: None,
                decision: None,
                observation_confidence: None,
                outcome: Some(outcome),
            },
        );
    }
}

const fn workflow_metric_outcome(outcome: DiagnosticOutcome) -> WorkflowMetricOutcome {
    match outcome {
        DiagnosticOutcome::Success => WorkflowMetricOutcome::Success,
        DiagnosticOutcome::Rejected => WorkflowMetricOutcome::Rejected,
        DiagnosticOutcome::ValidationFailure => WorkflowMetricOutcome::ValidationFailure,
        DiagnosticOutcome::ToolError => WorkflowMetricOutcome::ToolError,
        DiagnosticOutcome::TransportError => WorkflowMetricOutcome::TransportError,
        DiagnosticOutcome::Unavailable => WorkflowMetricOutcome::Unavailable,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_tool_diagnostic_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    runtime: &SessionRuntime,
    started: Instant,
    request_bytes: u64,
    tool_name: Option<&str>,
    response: Option<&Value>,
    facts: ToolDiagnosticFacts,
    validation_failure: bool,
    outcome: DiagnosticOutcome,
) {
    if runtime.codex_binding.is_pending() {
        return;
    }
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let response_bytes = response
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    if start_transport_diagnostic_session(context, adapter, runtime).is_err() {
        return;
    }
    let _ = record_diagnostic_event(
        context,
        DiagnosticEvent {
            session_id: &runtime.runtime_session_id,
            event_kind: DiagnosticEventKind::McpToolCall,
            tool_name,
            latency_micros: elapsed,
            request_bytes,
            response_bytes,
            validation_failure,
            core_reached: facts.core_reached,
            core_committed: facts.core_committed,
            replayed: facts.replayed,
            user_channel_kind: None,
            fallback_kind: facts.fallback_kind,
            product_file_write_count: facts.product_file_write_count,
            authoritative_refresh_failure: facts.authoritative_refresh_failure,
            outcome,
        },
    );
    record_public_method_metrics_best_effort(context, runtime, tool_name, outcome);
}
