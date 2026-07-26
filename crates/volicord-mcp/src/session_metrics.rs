//! MCP diagnostic-session setup and workflow metric persistence.
//!
//! The diagnostic event carrier remains in `telemetry`; this module owns the
//! session-scoped metric lifecycle used by transport and tool dispatch.

use crate::adapter::McpAdapter;
use crate::errors::McpAdapterError;
use crate::lifecycle::SessionRuntime;
use crate::tool_registry::method_name_for_tool;
use volicord_store::agent_connections::{
    agent_connection_record_read_only, list_connection_projects_read_only,
};
use volicord_store::diagnostics::{
    record_workflow_metric_event, start_diagnostic_session, DiagnosticHostKind, DiagnosticOutcome,
    DiagnosticSessionStart, DiagnosticTransport, WorkflowMetricEvent, WorkflowMetricKind,
    WorkflowMetricOutcome,
};
use volicord_store::error::StoreError;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::values::MethodName;

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

pub(crate) fn record_public_method_metrics_best_effort(
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
