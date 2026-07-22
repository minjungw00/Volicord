use std::{fmt, path::Path};

use serde::Serialize;
use volicord_store::{
    agent_connections::{agent_connection_record, list_connection_projects_for_diagnostics},
    bootstrap::project_record_by_repo_root_read_only,
    diagnostic_findings::{
        bounded_stored_diagnostic_graph_from_seeds, diagnostic_occurrences_for_runtime_session,
        stored_diagnostic_finding_by_id,
    },
    diagnostics::{
        current_diagnostics_storage_manifest, read_workflow_metric_aggregates,
        WorkflowMetricAggregateRow, DIAGNOSTICS_DB_FILE, DIAGNOSTICS_MAX_EVENTS_PER_SESSION,
        DIAGNOSTICS_MAX_SESSIONS, DIAGNOSTICS_RETENTION_DAYS,
    },
    operational_sessions::mcp_runtime_session,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

use volicord_types::{
    AgentRuntimeSessionId, DiagnosticConnectionContext, DiagnosticFinding, DiagnosticFindingId,
    DiagnosticLookupReport, DiagnosticLookupStatus, DiagnosticOperation, IntegrationProfile,
    IntegrationRevision, McpRuntimeSessionSource, StoredDiagnosticFinding, StoredDiagnosticGraph,
    MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH, MAX_DIAGNOSTIC_FINDINGS,
};

use crate::cli::{
    DiagnosticsArgs, DiagnosticsCommand, DiagnosticsSessionArgs, DiagnosticsShowArgs,
    DiagnosticsWorkflowMetricsArgs,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsCommandError {
    Usage(String),
    Runtime(String),
    NotFoundOutput(String),
}

impl fmt::Display for DiagnosticsCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::NotFoundOutput(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for DiagnosticsCommandError {}

impl From<StoreError> for DiagnosticsCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for DiagnosticsCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

/// Renders bounded local session diagnostics without opening an authority database.
pub fn run_diagnostics_command<F>(
    args: DiagnosticsArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, DiagnosticsCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match args.command {
        DiagnosticsCommand::Show(options) => run_show(options, env_var, current_dir),
        DiagnosticsCommand::Session(options) => run_runtime_session(options, env_var, current_dir),
        DiagnosticsCommand::WorkflowMetrics(options) => {
            run_workflow_metrics(options, env_var, current_dir)
        }
    }
}

fn run_show<F>(
    options: DiagnosticsShowArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, DiagnosticsCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let finding_id = DiagnosticFindingId::parse(options.finding_id.clone())
        .map_err(|error| DiagnosticsCommandError::Usage(error.to_string()))?;
    let report = if let Some(root) = stored_diagnostic_finding_by_id(&runtime_home, &finding_id)? {
        let cause_graph = bounded_stored_diagnostic_graph_from_seeds(
            &runtime_home,
            std::slice::from_ref(&finding_id),
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
        )?;
        let root_projection = root.to_diagnostic_finding();
        let context = finding_context(&runtime_home, &root_projection, &cause_graph)?;
        DiagnosticLookupReport::try_new(
            DiagnosticOperation::DiagnosticsShow,
            DiagnosticLookupStatus::Found,
            finding_id.as_str(),
            Some(root),
            cause_graph,
            context,
            diagnostic_lookup_limits(),
        )
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    } else {
        DiagnosticLookupReport::<StoredDiagnosticFinding>::try_new(
            DiagnosticOperation::DiagnosticsShow,
            DiagnosticLookupStatus::NotFound,
            finding_id.as_str(),
            None,
            StoredDiagnosticGraph::empty(),
            None,
            diagnostic_lookup_limits(),
        )
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    };
    render_finding_lookup_report(&report, options.json)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RuntimeSessionTerminalCondition {
    InProgress,
    GracefullyClosed,
    TerminatedWithFinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RuntimeSessionLookupRoot {
    runtime_session_id: String,
    connection_internal_id: String,
    session_source: McpRuntimeSessionSource,
    connection_integration_revision: String,
    observed_host_executable_version: Option<String>,
    attempted_client_name: Option<String>,
    attempted_client_version: Option<String>,
    requested_protocol_version: Option<String>,
    selected_protocol_version: Option<String>,
    negotiated_protocol_version: Option<String>,
    process_id: u32,
    process_started_at: String,
    initialize_completed_at: Option<String>,
    initialized_notification_at: Option<String>,
    tools_list_observed_at: Option<String>,
    required_tools_present: Option<bool>,
    verification_tool_name: Option<String>,
    verification_tool_observed_at: Option<String>,
    last_observed_at: String,
    terminal_condition: RuntimeSessionTerminalCondition,
    terminal_finding_id: Option<String>,
    graceful_close_at: Option<String>,
}

impl From<&volicord_store::operational_sessions::McpRuntimeSessionRecord>
    for RuntimeSessionLookupRoot
{
    fn from(session: &volicord_store::operational_sessions::McpRuntimeSessionRecord) -> Self {
        let terminal_condition = if session.terminal_finding_id.is_some() {
            RuntimeSessionTerminalCondition::TerminatedWithFinding
        } else if session.graceful_close_at.is_some() {
            RuntimeSessionTerminalCondition::GracefullyClosed
        } else {
            RuntimeSessionTerminalCondition::InProgress
        };
        Self {
            runtime_session_id: session.runtime_session_id.clone(),
            connection_internal_id: session.connection_internal_id.clone(),
            session_source: session.session_source,
            connection_integration_revision: session.connection_integration_revision.clone(),
            observed_host_executable_version: session.observed_host_executable_version.clone(),
            attempted_client_name: session.attempted_client_name.clone(),
            attempted_client_version: session.attempted_client_version.clone(),
            requested_protocol_version: session.requested_protocol_version.clone(),
            selected_protocol_version: session.selected_protocol_version.clone(),
            negotiated_protocol_version: session.negotiated_protocol_version.clone(),
            process_id: session.process_id,
            process_started_at: session.process_started_at.clone(),
            initialize_completed_at: session.initialize_completed_at.clone(),
            initialized_notification_at: session.initialized_notification_at.clone(),
            tools_list_observed_at: session.tools_list_observed_at.clone(),
            required_tools_present: session.required_tools_present,
            verification_tool_name: session.verification_tool_name.clone(),
            verification_tool_observed_at: session.verification_tool_observed_at.clone(),
            last_observed_at: session.last_observed_at.clone(),
            terminal_condition,
            terminal_finding_id: session.terminal_finding_id.clone(),
            graceful_close_at: session.graceful_close_at.clone(),
        }
    }
}

fn run_runtime_session<F>(
    options: DiagnosticsSessionArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, DiagnosticsCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let report = if let Some(session) =
        mcp_runtime_session(&runtime_home, &options.runtime_session_id)?
    {
        let direct =
            diagnostic_occurrences_for_runtime_session(&runtime_home, &options.runtime_session_id)?;
        let seed_ids = direct
            .iter()
            .map(|finding| finding.id())
            .collect::<Vec<_>>();
        let cause_graph = if seed_ids.is_empty() {
            StoredDiagnosticGraph::empty()
        } else {
            bounded_stored_diagnostic_graph_from_seeds(
                &runtime_home,
                &seed_ids,
                MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
            )?
        };
        let context = runtime_session_context(&runtime_home, &session, &cause_graph)?;
        DiagnosticLookupReport::try_new(
            DiagnosticOperation::DiagnosticsSession,
            DiagnosticLookupStatus::Found,
            options.runtime_session_id.as_str(),
            Some(RuntimeSessionLookupRoot::from(&session)),
            cause_graph,
            context,
            diagnostic_lookup_limits(),
        )
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    } else {
        DiagnosticLookupReport::<RuntimeSessionLookupRoot>::try_new(
            DiagnosticOperation::DiagnosticsSession,
            DiagnosticLookupStatus::NotFound,
            options.runtime_session_id.as_str(),
            None,
            StoredDiagnosticGraph::empty(),
            None,
            diagnostic_lookup_limits(),
        )
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    };
    render_session_lookup_report(&report, options.json)
}

fn finding_context(
    runtime_home: &Path,
    finding: &DiagnosticFinding,
    cause_graph: &StoredDiagnosticGraph,
) -> Result<Option<DiagnosticConnectionContext>, DiagnosticsCommandError> {
    let Some(connection_id) = finding.connection_id() else {
        return Ok(None);
    };
    let Some(connection) = agent_connection_record(runtime_home, connection_id.as_str())? else {
        return Ok(None);
    };
    connection_context(
        runtime_home,
        &connection,
        finding.project_id().map(|project_id| project_id.as_str()),
        finding.integration_revision().cloned(),
        cause_graph,
        Vec::new(),
    )
    .map(Some)
}

fn runtime_session_context(
    runtime_home: &Path,
    session: &volicord_store::operational_sessions::McpRuntimeSessionRecord,
    cause_graph: &StoredDiagnosticGraph,
) -> Result<Option<DiagnosticConnectionContext>, DiagnosticsCommandError> {
    let Some(connection) = agent_connection_record(runtime_home, &session.connection_internal_id)?
    else {
        return Ok(None);
    };
    let revision = IntegrationRevision::parse(session.connection_integration_revision.clone())
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?;
    connection_context(
        runtime_home,
        &connection,
        None,
        Some(revision),
        cause_graph,
        vec![AgentRuntimeSessionId::new(
            session.runtime_session_id.clone(),
        )],
    )
    .map(Some)
}

fn connection_context(
    runtime_home: &Path,
    connection: &volicord_store::agent_connections::AgentConnectionRecord,
    project_id: Option<&str>,
    revision: Option<IntegrationRevision>,
    cause_graph: &StoredDiagnosticGraph,
    mut runtime_session_ids: Vec<AgentRuntimeSessionId>,
) -> Result<DiagnosticConnectionContext, DiagnosticsCommandError> {
    let projects =
        list_connection_projects_for_diagnostics(runtime_home, &connection.connection_internal_id)?;
    let repository = project_id
        .and_then(|project_id| {
            projects
                .iter()
                .find(|project| project.project_id == project_id)
        })
        .or_else(|| (projects.len() == 1).then(|| &projects[0]))
        .map(|project| project.project.repo_root.to_string_lossy().into_owned());
    runtime_session_ids.extend(cause_graph.entries().iter().filter_map(|entry| {
        entry
            .finding()
            .occurrence()
            .and_then(|finding| finding.runtime_session_id().cloned())
    }));
    runtime_session_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    runtime_session_ids.dedup();
    DiagnosticConnectionContext::try_new(
        runtime_home.to_string_lossy().into_owned(),
        connection.connection_internal_id.clone(),
        connection.host_kind.clone(),
        connection.host_scope.clone(),
        IntegrationProfile::Record.as_str().to_owned(),
        connection.mode.clone(),
        repository,
        Some(connection.config_target.clone()),
        revision,
        runtime_session_ids,
    )
    .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))
}

fn diagnostic_lookup_limits() -> Vec<String> {
    vec![
        format!(
            "Diagnostic cause traversal is bounded to {} edges and {} findings.",
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH, MAX_DIAGNOSTIC_FINDINGS
        ),
        "Only bounded typed diagnostic facts are rendered; sensitive fields remain redacted."
            .to_owned(),
    ]
}

fn render_finding_lookup_report(
    report: &DiagnosticLookupReport<StoredDiagnosticFinding>,
    json: bool,
) -> Result<String, DiagnosticsCommandError> {
    let output = if json {
        serde_json::to_string_pretty(report)
            .map(|output| format!("{output}\n"))
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    } else {
        render_finding_lookup_text(report)
    };
    if report.lookup_status() == DiagnosticLookupStatus::NotFound {
        Err(DiagnosticsCommandError::NotFoundOutput(output))
    } else {
        Ok(output)
    }
}

fn render_session_lookup_report(
    report: &DiagnosticLookupReport<RuntimeSessionLookupRoot>,
    json: bool,
) -> Result<String, DiagnosticsCommandError> {
    let output = if json {
        serde_json::to_string_pretty(report)
            .map(|output| format!("{output}\n"))
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    } else {
        render_session_lookup_text(report)
    };
    if report.lookup_status() == DiagnosticLookupStatus::NotFound {
        Err(DiagnosticsCommandError::NotFoundOutput(output))
    } else {
        Ok(output)
    }
}

fn lookup_text_header<T>(report: &DiagnosticLookupReport<T>) -> Vec<String>
where
    T: Serialize,
{
    let mut lines = vec![
        format!("Diagnostic lookup: {}", report.lookup_status().as_str()),
        format!("Operation: {}", report.operation().as_str()),
        format!("Requested ID: {}", report.requested_id()),
    ];
    if let Some(connection) = report.context() {
        lines.push(format!("Runtime home: {}", connection.runtime_home()));
        lines.push(format!("Connection: {}", connection.connection_id()));
        if let Some(revision) = connection.integration_revision() {
            lines.push(format!("Integration revision: {}", revision.as_str()));
        }
        if !connection.runtime_session_ids().is_empty() {
            lines.push(format!(
                "Runtime sessions: {}",
                connection
                    .runtime_session_ids()
                    .iter()
                    .map(AgentRuntimeSessionId::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    lines
}

fn render_finding_lookup_text(report: &DiagnosticLookupReport<StoredDiagnosticFinding>) -> String {
    let mut lines = lookup_text_header(report);
    append_stored_graph(&mut lines, report.cause_graph());
    for limit in report.limits() {
        lines.push(format!("Limit: {limit}"));
    }
    format!("{}\n", lines.join("\n"))
}

fn render_session_lookup_text(report: &DiagnosticLookupReport<RuntimeSessionLookupRoot>) -> String {
    let mut lines = lookup_text_header(report);
    if let Some(session) = report.root() {
        lines.push(format!("Runtime session: {}", session.runtime_session_id));
        lines.push(format!(
            "  Session source: {}",
            session.session_source.as_str()
        ));
        lines.push(format!(
            "  Terminal condition: {}",
            match session.terminal_condition {
                RuntimeSessionTerminalCondition::InProgress => "in_progress",
                RuntimeSessionTerminalCondition::GracefullyClosed => "gracefully_closed",
                RuntimeSessionTerminalCondition::TerminatedWithFinding => {
                    "terminated_with_finding"
                }
            }
        ));
        if let Some(finding_id) = session.terminal_finding_id.as_deref() {
            lines.push(format!("  Terminal finding: {finding_id}"));
        }
        lines.push(format!("  Last observed: {}", session.last_observed_at));
    }
    append_stored_graph(&mut lines, report.cause_graph());
    for limit in report.limits() {
        lines.push(format!("Limit: {limit}"));
    }
    format!("{}\n", lines.join("\n"))
}

fn append_stored_graph(lines: &mut Vec<String>, graph: &StoredDiagnosticGraph) {
    for entry in graph.entries() {
        let stored = entry.finding();
        let finding = stored.to_diagnostic_finding();
        lines.push(format!(
            "Finding (depth {}): {}",
            entry.depth(),
            finding.id()
        ));
        lines.push(format!("  Lifecycle: {}", stored.lifecycle().as_str()));
        if let Some(current) = stored.current() {
            lines.push(format!(
                "  Current status: {}",
                current.snapshot().status().as_str()
            ));
            if let Some(resolved_at) = current.snapshot().resolved_at() {
                lines.push(format!("  Resolved at: {resolved_at}"));
            }
        }
        lines.push(format!("  Severity: {}", finding.severity().as_str()));
        lines.push(format!("  Code: {}", finding.code()));
        lines.push(format!("  Domain: {}", finding.domain()));
        lines.push(format!("  Stage: {}", finding.stage()));
        lines.push(format!("  Source: {}", finding.source()));
        lines.push(format!(
            "  Subject: {} {}",
            finding.subject().kind(),
            finding.subject().reference()
        ));
        lines.push(format!("  Observed at: {}", finding.observed_at()));
        if let Some(runtime_session_id) = finding.runtime_session_id() {
            lines.push(format!("  Runtime session: {runtime_session_id}"));
        }
        lines.push(format!(
            "  Bounded typed facts: {}",
            serde_json::to_string(finding.facts().data()).unwrap_or_else(|_| "{}".to_owned())
        ));
        if !finding.causes().is_empty() {
            lines.push(format!(
                "  Caused by: {}",
                finding
                    .causes()
                    .iter()
                    .map(|cause| cause.finding_id().as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for action in finding.actions() {
            lines.push(format!(
                "  Action: {} — {}",
                action.code(),
                action.summary()
            ));
        }
    }
}

fn run_workflow_metrics<F>(
    options: DiagnosticsWorkflowMetricsArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, DiagnosticsCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let repo_root = if options.repo.is_absolute() {
        options.repo
    } else {
        current_dir.join(options.repo)
    };
    let project =
        project_record_by_repo_root_read_only(&runtime_home, &repo_root)?.ok_or_else(|| {
            DiagnosticsCommandError::Runtime(format!(
                "project is not registered for repository {}; run `volicord project use`",
                repo_root.display()
            ))
        })?;
    let rows = read_workflow_metric_aggregates(&runtime_home, &project.project_id)?;
    serde_json::to_string_pretty(&workflow_metrics_report(rows)?)
        .map(|output| format!("{output}\n"))
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))
}

#[derive(Debug, Serialize)]
struct DiagnosticsStorageReport {
    database_file: &'static str,
    contract_id: String,
    canonical_schema_digest: String,
    retention_days: u32,
    max_sessions: u32,
    max_events_per_session: u32,
}

#[derive(Debug, Serialize)]
struct AuthorityIsolationReport {
    project_state_database_opened_by_report: bool,
    changes_state_version: bool,
    changes_evidence_or_assurance: bool,
    changes_close_readiness: bool,
    changes_user_actions: bool,
}

#[derive(Debug, Serialize)]
struct WorkflowMetricsReport {
    status: &'static str,
    scope: &'static str,
    storage: DiagnosticsStorageReport,
    duration_measurements: WorkflowDurationMeasurements,
    confirmed_unrecorded_false_positive_rate: WorkflowRateMeasurement,
    aggregates: Vec<WorkflowMetricAggregateRow>,
    redaction: WorkflowMetricsRedactionReport,
    authority_isolation: AuthorityIsolationReport,
}

#[derive(Debug, Serialize)]
struct WorkflowDurationMeasurements {
    task_duration_micros: WorkflowDistributionMeasurement,
    first_product_write_duration_micros: WorkflowDistributionMeasurement,
}

#[derive(Debug, Serialize)]
struct WorkflowDistributionMeasurement {
    status: &'static str,
    value: Option<WorkflowDistribution>,
}

#[derive(Debug, Serialize)]
struct WorkflowDistribution {
    sample_count: u64,
    total: u64,
    minimum: u64,
    maximum: u64,
    average: u64,
}

#[derive(Debug, Serialize)]
struct WorkflowRateMeasurement {
    status: &'static str,
    numerator: Option<u64>,
    denominator: Option<u64>,
    value: Option<f64>,
}

#[derive(Debug, Serialize)]
struct WorkflowMetricsRedactionReport {
    aggregate_only: bool,
    stores_or_returns_command_prompt_path_content_or_user_answer: bool,
}

fn workflow_metrics_report(
    rows: Vec<WorkflowMetricAggregateRow>,
) -> Result<WorkflowMetricsReport, DiagnosticsCommandError> {
    let task_duration = distribution_measurement(&rows, "task_duration_micros");
    let first_write_duration =
        distribution_measurement(&rows, "first_product_write_duration_micros");
    let false_positive_rows = rows
        .iter()
        .filter(|row| row.metric_kind == "confirmed_unrecorded_false_positive")
        .collect::<Vec<_>>();
    let false_positive_numerator = false_positive_rows
        .iter()
        .map(|row| row.value_total)
        .sum::<u64>();
    let classified_confirmed_findings = false_positive_rows
        .iter()
        .map(|row| row.sample_count)
        .sum::<u64>();
    let false_positive_rate = if classified_confirmed_findings > 0 {
        WorkflowRateMeasurement {
            status: "available",
            numerator: Some(false_positive_numerator),
            denominator: Some(classified_confirmed_findings),
            value: Some(false_positive_numerator as f64 / classified_confirmed_findings as f64),
        }
    } else {
        WorkflowRateMeasurement {
            status: "measurement_pending",
            numerator: None,
            denominator: None,
            value: None,
        }
    };
    Ok(WorkflowMetricsReport {
        status: if rows.is_empty() {
            "no_data"
        } else {
            "available"
        },
        scope: "bounded_local_operability_aggregates_only",
        storage: diagnostics_storage_report()?,
        duration_measurements: WorkflowDurationMeasurements {
            task_duration_micros: task_duration,
            first_product_write_duration_micros: first_write_duration,
        },
        confirmed_unrecorded_false_positive_rate: false_positive_rate,
        aggregates: rows,
        redaction: WorkflowMetricsRedactionReport {
            aggregate_only: true,
            stores_or_returns_command_prompt_path_content_or_user_answer: false,
        },
        authority_isolation: AuthorityIsolationReport {
            project_state_database_opened_by_report: false,
            changes_state_version: false,
            changes_evidence_or_assurance: false,
            changes_close_readiness: false,
            changes_user_actions: false,
        },
    })
}

fn distribution_measurement(
    rows: &[WorkflowMetricAggregateRow],
    metric_kind: &str,
) -> WorkflowDistributionMeasurement {
    let matching = rows
        .iter()
        .filter(|row| row.metric_kind == metric_kind)
        .collect::<Vec<_>>();
    let sample_count = matching.iter().map(|row| row.sample_count).sum::<u64>();
    if sample_count == 0 {
        return WorkflowDistributionMeasurement {
            status: "measurement_pending",
            value: None,
        };
    }
    let total = matching.iter().map(|row| row.value_total).sum::<u64>();
    WorkflowDistributionMeasurement {
        status: "available",
        value: Some(WorkflowDistribution {
            sample_count,
            total,
            minimum: matching.iter().map(|row| row.value_min).min().unwrap_or(0),
            maximum: matching.iter().map(|row| row.value_max).max().unwrap_or(0),
            average: total.checked_div(sample_count).unwrap_or(0),
        }),
    }
}

fn diagnostics_storage_report() -> Result<DiagnosticsStorageReport, DiagnosticsCommandError> {
    let manifest = current_diagnostics_storage_manifest()?;
    Ok(DiagnosticsStorageReport {
        database_file: DIAGNOSTICS_DB_FILE,
        contract_id: manifest.contract_id.clone(),
        canonical_schema_digest: manifest.canonical_schema_digest.clone(),
        retention_days: DIAGNOSTICS_RETENTION_DAYS,
        max_sessions: DIAGNOSTICS_MAX_SESSIONS,
        max_events_per_session: DIAGNOSTICS_MAX_EVENTS_PER_SESSION,
    })
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs};

    use serde::Serialize;
    use serde_json::Value;
    use volicord_core::{CoreService, InvocationContext};
    use volicord_store::diagnostics::{
        diagnostics_db_path, record_diagnostic_event, record_workflow_metric_event,
        start_diagnostic_session, DiagnosticEvent, DiagnosticEventKind, DiagnosticFallbackKind,
        DiagnosticHostKind, DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport,
        WorkflowMetricEvent, WorkflowMetricKind, WorkflowMetricOutcome,
    };
    use volicord_store::{
        diagnostic_findings::{
            insert_occurrence_finding, resolve_current_finding, upsert_current_snapshot,
        },
        operational_sessions::{
            connection_integration_revision, record_mcp_terminal_finding, McpRuntimeSessionStart,
        },
    };
    use volicord_test_support::core_fixtures::{CoreFixture, UserActionFixture};
    use volicord_types::{
        ActorSource, AgentConnectionId, CurrentDiagnosticFinding, CurrentDiagnosticKey,
        CurrentDiagnosticSnapshot, DiagnosticCause, DiagnosticCode, DiagnosticDomain,
        DiagnosticFactSource, DiagnosticFacts, DiagnosticFindingData, DiagnosticScope,
        DiagnosticScopeKind, DiagnosticSeverity, DiagnosticSource, DiagnosticStage,
        DiagnosticSubject, DiagnosticSubjectIdentity, IntegrationProfile, JudgmentKind, MethodName,
        ObservationConfidence, OccurrenceDiagnosticFinding, OperationCategory, ProjectId,
        UtcTimestamp,
    };

    use crate::cli::DiagnosticsWorkflowMetricsArgs;

    use super::*;

    fn env_for(runtime_home: &Path) -> impl Fn(&str) -> Option<OsString> + '_ {
        move |name| (name == "VOLICORD_HOME").then(|| OsString::from(runtime_home))
    }

    fn workflow_metrics_args(repo: &Path) -> DiagnosticsArgs {
        DiagnosticsArgs {
            command: DiagnosticsCommand::WorkflowMetrics(DiagnosticsWorkflowMetricsArgs {
                repo: repo.to_path_buf(),
                json: true,
            }),
        }
    }

    #[test]
    fn missing_finding_and_runtime_session_lookups_are_exact_typed_reports() {
        let fixture = CoreFixture::new("missing-diagnostic-lookups").expect("fixture");
        let cases = [
            (
                DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: "finding.does_not_exist".to_owned(),
                    json: true,
                }),
                "diagnostics_show",
                "finding.does_not_exist",
            ),
            (
                DiagnosticsCommand::Session(DiagnosticsSessionArgs {
                    runtime_session_id: "runtime_session_does_not_exist".to_owned(),
                    json: true,
                }),
                "diagnostics_session",
                "runtime_session_does_not_exist",
            ),
        ];
        for (command, operation, requested_id) in cases {
            let error = run_diagnostics_command(
                DiagnosticsArgs { command },
                env_for(fixture.runtime_home_path()),
                fixture.product_repo_path().as_path(),
            )
            .expect_err("missing lookup must use the typed failure output channel");
            let DiagnosticsCommandError::NotFoundOutput(output) = error else {
                panic!("missing lookup returned an ad hoc error: {error}");
            };
            let report: Value = serde_json::from_str(&output).expect("diagnostic lookup JSON");
            assert_eq!(report["schema_version"], 1);
            assert_eq!(report["operation"], operation);
            assert_eq!(report["lookup_status"], "not_found");
            assert_eq!(report["requested_id"], requested_id);
            assert!(report["root"].is_null());
            assert_eq!(report["cause_graph"]["entries"], serde_json::json!([]));
            assert!(report.get("status").is_none());
            assert!(report.get("checks").is_none());
            assert!(report.get("findings").is_none());
        }
    }

    #[derive(Serialize)]
    struct CurrentLookupFacts<'a> {
        observed_state: &'a str,
    }

    impl DiagnosticFactSource for CurrentLookupFacts<'_> {}

    #[test]
    fn diagnostics_show_returns_latest_current_state_for_stable_id() {
        let fixture = CoreFixture::new("diagnostics-show-current-state").expect("fixture");
        let connection =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())
                .expect("connection lookup")
                .expect("connection");
        let revision = connection_integration_revision(&connection).expect("revision");
        let key = CurrentDiagnosticKey::new(
            DiagnosticScope::try_new(DiagnosticScopeKind::Connection, fixture.connection_id())
                .expect("scope"),
            DiagnosticCode::parse("managed_config.command.drift").expect("code"),
            DiagnosticDomain::parse("configuration").expect("domain"),
            DiagnosticStage::parse("managed_configuration").expect("stage"),
            DiagnosticSource::parse("administrative_cli").expect("source"),
            DiagnosticSubjectIdentity::from_canonical_bytes(
                b"volicord.test.managed-config:/canonical/private/config.toml",
            ),
        );
        let finding_id = key.finding_id();
        let make_finding = |observed_state: &'static str, observed_at: &'static str| {
            CurrentDiagnosticSnapshot::try_new(
                DiagnosticSubject::try_new("managed_config_target", "/bounded/current/config.toml")
                    .expect("subject"),
                DiagnosticSeverity::Error,
                DiagnosticFacts::project(&CurrentLookupFacts { observed_state }).expect("facts"),
                UtcTimestamp::parse(observed_at).expect("time"),
            )
            .and_then(|snapshot| {
                snapshot.with_connection_id(AgentConnectionId::new(fixture.connection_id()))
            })
            .map(|snapshot| snapshot.with_integration_revision(revision.clone()))
            .and_then(|snapshot| CurrentDiagnosticFinding::try_new(key.clone(), snapshot))
            .expect("finding")
        };
        let original = make_finding("missing", "2026-07-22T01:02:03Z");
        let latest = make_finding("drift", "2026-07-22T02:03:04Z");
        upsert_current_snapshot(fixture.runtime_home_path(), &original).expect("initial snapshot");
        upsert_current_snapshot(fixture.runtime_home_path(), &latest)
            .expect("replacement snapshot");

        let output = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: finding_id.to_string(),
                    json: true,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("found finding lookup succeeds regardless of severity");
        let report: Value = serde_json::from_str(&output).expect("diagnostic lookup JSON");
        assert_eq!(report["lookup_status"], "found");
        assert_eq!(report["root"]["lifecycle"], "current_state");
        assert_eq!(report["root"]["current_state_status"], "active");
        assert!(report["root"]["resolved_at"].is_null());
        assert_eq!(report["root"]["finding"]["id"], finding_id.as_str());
        assert_eq!(
            report["root"]["finding"]["subject"]["reference"],
            "/bounded/current/config.toml"
        );
        assert_eq!(
            report["root"]["finding"]["facts"]["data"]["observed_state"],
            "drift"
        );
        assert_eq!(
            report["root"]["finding"]["observed_at"],
            "2026-07-22T02:03:04Z"
        );
        assert_eq!(
            report["cause_graph"]["entries"][0]["finding"],
            report["root"]
        );

        let active_text = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: finding_id.to_string(),
                    json: false,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("active finding human lookup succeeds");
        assert!(active_text.contains("Diagnostic lookup: found"));
        assert!(active_text.contains(&format!("Finding (depth 0): {finding_id}")));
        assert!(active_text.contains("Lifecycle: current_state"));
        assert!(active_text.contains("Current status: active"));
        assert!(active_text.contains("Severity: error"));

        let resolved_at = UtcTimestamp::parse("2026-07-22T03:04:05Z").expect("time");
        resolve_current_finding(fixture.runtime_home_path(), &key, resolved_at.clone())
            .expect("resolve current finding");
        let resolved_json = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: finding_id.to_string(),
                    json: true,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("resolved error finding lookup succeeds");
        let resolved_report: Value =
            serde_json::from_str(&resolved_json).expect("resolved lookup JSON");
        assert_eq!(resolved_report["lookup_status"], "found");
        assert_eq!(resolved_report["root"]["current_state_status"], "resolved");
        assert_eq!(
            resolved_report["root"]["resolved_at"],
            "2026-07-22T03:04:05Z"
        );
        assert_eq!(resolved_report["root"]["finding"]["severity"], "error");
        assert!(resolved_report.get("status").is_none());
        assert!(resolved_report.get("checks").is_none());

        let resolved_text = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: finding_id.to_string(),
                    json: false,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("resolved finding human lookup succeeds");
        assert!(resolved_text.contains("Diagnostic lookup: found"));
        assert!(resolved_text.contains("Lifecycle: current_state"));
        assert!(resolved_text.contains("Current status: resolved"));
        assert!(resolved_text.contains("Resolved at: 2026-07-22T03:04:05Z"));
        assert!(!resolved_text.contains("verification failed"));
    }

    fn test_occurrence(
        severity: DiagnosticSeverity,
        observed_at: &str,
    ) -> OccurrenceDiagnosticFinding {
        OccurrenceDiagnosticFinding::try_new(
            DiagnosticFindingData::try_new(
                DiagnosticCode::parse("diagnostics.test_severity").expect("code"),
                DiagnosticDomain::parse("diagnostics").expect("domain"),
                DiagnosticStage::parse("lookup").expect("stage"),
                severity,
                DiagnosticSource::parse("cli_test").expect("source"),
                DiagnosticSubject::try_new("test_record", severity.as_str()).expect("subject"),
                DiagnosticFacts::empty(),
                UtcTimestamp::parse(observed_at).expect("time"),
            )
            .expect("finding data"),
            None,
        )
        .expect("occurrence")
    }

    #[test]
    fn diagnostics_show_found_occurrences_succeed_for_every_severity() {
        let fixture = CoreFixture::new("diagnostics-show-severity").expect("fixture");
        for (index, severity) in [
            DiagnosticSeverity::Info,
            DiagnosticSeverity::Warning,
            DiagnosticSeverity::Error,
        ]
        .into_iter()
        .enumerate()
        {
            let finding = test_occurrence(severity, &format!("2026-07-22T04:05:0{}Z", index + 1));
            insert_occurrence_finding(fixture.runtime_home_path(), &finding).expect("insert");
            let output = run_diagnostics_command(
                DiagnosticsArgs {
                    command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                        finding_id: finding.id().to_string(),
                        json: true,
                    }),
                },
                env_for(fixture.runtime_home_path()),
                fixture.product_repo_path().as_path(),
            )
            .expect("found occurrence lookup succeeds for every severity");
            let report: Value = serde_json::from_str(&output).expect("lookup JSON");
            assert_eq!(report["lookup_status"], "found");
            assert_eq!(report["root"]["lifecycle"], "occurrence");
            assert_eq!(report["root"]["finding"]["severity"], severity.as_str());
            assert_eq!(
                report["root"]["finding"]["observed_at"],
                format!("2026-07-22T04:05:0{}Z", index + 1)
            );
        }
    }

    #[test]
    fn diagnostics_show_json_and_text_preserve_every_lifecycle_in_the_cause_graph() {
        let fixture = CoreFixture::new("diagnostics-show-mixed-graph").expect("fixture");
        let occurrence_cause = test_occurrence(DiagnosticSeverity::Info, "2026-07-22T04:10:01Z");
        insert_occurrence_finding(fixture.runtime_home_path(), &occurrence_cause)
            .expect("occurrence cause");

        let current_key = |subject: &str| {
            CurrentDiagnosticKey::new(
                DiagnosticScope::try_new(DiagnosticScopeKind::Connection, fixture.connection_id())
                    .expect("scope"),
                DiagnosticCode::parse("guard.test_condition").expect("code"),
                DiagnosticDomain::parse("guard").expect("domain"),
                DiagnosticStage::parse("guard_files").expect("stage"),
                DiagnosticSource::parse("cli_test").expect("source"),
                DiagnosticSubjectIdentity::from_canonical_bytes(subject.as_bytes()),
            )
        };
        let active_key = current_key("active-current");
        let active = CurrentDiagnosticFinding::try_new(
            active_key,
            CurrentDiagnosticSnapshot::try_new(
                DiagnosticSubject::try_new("guard_artifact", "active-safe-subject")
                    .expect("subject"),
                DiagnosticSeverity::Warning,
                DiagnosticFacts::empty(),
                UtcTimestamp::parse("2026-07-22T04:10:02Z").expect("time"),
            )
            .and_then(|snapshot| {
                snapshot.with_causes(vec![DiagnosticCause::new(occurrence_cause.id())])
            })
            .expect("active snapshot"),
        )
        .expect("active current");
        upsert_current_snapshot(fixture.runtime_home_path(), &active).expect("active upsert");

        let resolved_key = current_key("resolved-current");
        let resolved = CurrentDiagnosticFinding::try_new(
            resolved_key.clone(),
            CurrentDiagnosticSnapshot::try_new(
                DiagnosticSubject::try_new("guard_artifact", "resolved-safe-subject")
                    .expect("subject"),
                DiagnosticSeverity::Error,
                DiagnosticFacts::empty(),
                UtcTimestamp::parse("2026-07-22T04:10:03Z").expect("time"),
            )
            .expect("resolved snapshot"),
        )
        .expect("resolved current");
        upsert_current_snapshot(fixture.runtime_home_path(), &resolved).expect("resolved upsert");
        resolve_current_finding(
            fixture.runtime_home_path(),
            &resolved_key,
            UtcTimestamp::parse("2026-07-22T04:10:04Z").expect("time"),
        )
        .expect("resolve");

        let root = OccurrenceDiagnosticFinding::try_new(
            DiagnosticFindingData::try_new(
                DiagnosticCode::parse("diagnostics.mixed_root").expect("code"),
                DiagnosticDomain::parse("diagnostics").expect("domain"),
                DiagnosticStage::parse("lookup").expect("stage"),
                DiagnosticSeverity::Error,
                DiagnosticSource::parse("cli_test").expect("source"),
                DiagnosticSubject::try_new("test_record", "mixed-root").expect("subject"),
                DiagnosticFacts::empty(),
                UtcTimestamp::parse("2026-07-22T04:10:05Z").expect("time"),
            )
            .and_then(|data| {
                data.with_causes(vec![
                    DiagnosticCause::new(active.id().clone()),
                    DiagnosticCause::new(resolved.id().clone()),
                ])
            })
            .expect("root data"),
            None,
        )
        .expect("root occurrence");
        insert_occurrence_finding(fixture.runtime_home_path(), &root).expect("root insert");

        let args = |json| DiagnosticsArgs {
            command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                finding_id: root.id().to_string(),
                json,
            }),
        };
        let json_output = run_diagnostics_command(
            args(true),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("mixed graph JSON lookup");
        let report: Value = serde_json::from_str(&json_output).expect("lookup JSON");
        let entries = report["cause_graph"]["entries"]
            .as_array()
            .expect("entries");
        assert!(entries.iter().any(|entry| {
            entry["finding"]["lifecycle"] == "occurrence"
                && entry["finding"]["finding"]["id"] == occurrence_cause.id().as_str()
        }));
        assert!(entries.iter().any(|entry| {
            entry["finding"]["current_state_status"] == "active"
                && entry["finding"]["finding"]["id"] == active.id().as_str()
        }));
        assert!(entries.iter().any(|entry| {
            entry["finding"]["current_state_status"] == "resolved"
                && entry["finding"]["resolved_at"] == "2026-07-22T04:10:04Z"
                && entry["finding"]["finding"]["id"] == resolved.id().as_str()
        }));

        let human_output = run_diagnostics_command(
            args(false),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("mixed graph human lookup");
        assert!(human_output.contains(&format!("Finding (depth 0): {}", root.id())));
        assert!(human_output.contains(&format!("Finding (depth 1): {}", active.id())));
        assert!(human_output.contains(&format!("Finding (depth 1): {}", resolved.id())));
        assert!(human_output.contains("Lifecycle: occurrence"));
        assert!(human_output.contains("Current status: active"));
        assert!(human_output.contains("Current status: resolved"));
        assert!(human_output.contains("Resolved at: 2026-07-22T04:10:04Z"));
    }

    #[test]
    fn invalid_finding_id_is_usage_not_not_found() {
        let fixture = CoreFixture::new("diagnostics-show-invalid-id").expect("fixture");
        let error = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: "invalid finding id".to_owned(),
                    json: true,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect_err("invalid ID must be usage failure");
        assert!(matches!(error, DiagnosticsCommandError::Usage(_)));
    }

    #[test]
    fn diagnostics_session_lookup_success_is_independent_of_terminal_severity() {
        let fixture = CoreFixture::new("diagnostics-session-terminal-error").expect("fixture");
        let session = volicord_test_support::start_test_mcp_runtime_session(
            fixture.runtime_home_path(),
            McpRuntimeSessionStart {
                connection_internal_id: fixture.connection_id().to_owned(),
                session_source: McpRuntimeSessionSource::ManagedHost,
                observed_host_executable_version: Some("future-host-999".to_owned()),
                process_id: 42,
                process_started_at: "2026-07-22T05:00:00Z".to_owned(),
            },
        )
        .expect("runtime session");
        let terminal = OccurrenceDiagnosticFinding::try_new(
            DiagnosticFindingData::try_new(
                DiagnosticCode::parse("mcp.test_terminal_error").expect("code"),
                DiagnosticDomain::parse("mcp").expect("domain"),
                DiagnosticStage::parse("transport").expect("stage"),
                DiagnosticSeverity::Error,
                DiagnosticSource::parse("cli_test").expect("source"),
                DiagnosticSubject::try_new("runtime_session", &session.runtime_session_id)
                    .expect("subject"),
                DiagnosticFacts::empty(),
                UtcTimestamp::parse("2026-07-22T05:00:01Z").expect("time"),
            )
            .and_then(|data| {
                data.with_connection_id(AgentConnectionId::new(fixture.connection_id()))
            })
            .map(|data| {
                data.with_integration_revision(
                    IntegrationRevision::parse(session.connection_integration_revision.clone())
                        .expect("revision"),
                )
            })
            .expect("terminal finding data"),
            Some(AgentRuntimeSessionId::new(
                session.runtime_session_id.clone(),
            )),
        )
        .expect("terminal occurrence");
        record_mcp_terminal_finding(fixture.runtime_home_path(), &terminal)
            .expect("record terminal finding");

        let json_output = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Session(DiagnosticsSessionArgs {
                    runtime_session_id: session.runtime_session_id.clone(),
                    json: true,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("found terminal session lookup succeeds");
        let report: Value = serde_json::from_str(&json_output).expect("session lookup JSON");
        assert_eq!(report["lookup_status"], "found");
        assert_eq!(
            report["root"]["terminal_condition"],
            "terminated_with_finding"
        );
        assert_eq!(
            report["root"]["terminal_finding_id"],
            terminal.id().as_str()
        );
        assert_eq!(
            report["cause_graph"]["entries"][0]["finding"]["lifecycle"],
            "occurrence"
        );
        assert_eq!(
            report["cause_graph"]["entries"][0]["finding"]["finding"]["severity"],
            "error"
        );
        assert!(report.get("status").is_none());
        assert!(report.get("checks").is_none());

        let human_output = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Session(DiagnosticsSessionArgs {
                    runtime_session_id: session.runtime_session_id,
                    json: false,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("found terminal session human lookup succeeds");
        assert!(human_output.contains("Diagnostic lookup: found"));
        assert!(human_output.contains("Terminal condition: terminated_with_finding"));
        assert!(human_output.contains("Lifecycle: occurrence"));
        assert!(human_output.contains("Severity: error"));
    }

    #[test]
    fn workflow_metrics_no_data_is_read_only_and_reports_pending_measurements() {
        let fixture = CoreFixture::new("workflow-metrics-no-data").expect("fixture");
        let diagnostics_path = diagnostics_db_path(fixture.runtime_home_path());
        assert!(!diagnostics_path.exists());

        let output = run_diagnostics_command(
            workflow_metrics_args(&fixture.product_repo_path()),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("workflow metrics no-data report");
        let report: Value = serde_json::from_str(&output).expect("JSON");
        assert_eq!(report["status"], "no_data");
        assert!(report.get("schema_version").is_none());
        assert_eq!(
            report["storage"]["contract_id"],
            "volicord.sqlite.diagnostics"
        );
        assert_eq!(
            report["duration_measurements"]["task_duration_micros"]["status"],
            "measurement_pending"
        );
        assert_eq!(
            report["confirmed_unrecorded_false_positive_rate"]["status"],
            "measurement_pending"
        );
        assert!(report["aggregates"]
            .as_array()
            .expect("aggregates")
            .is_empty());
        assert!(!diagnostics_path.exists());
    }

    #[test]
    fn workflow_metrics_returns_only_bounded_project_aggregates() {
        let fixture = CoreFixture::new("workflow-metrics-json").expect("fixture");
        let session_id = "mcp_runtime_session_workflow_metrics".to_owned();
        start_diagnostic_session(
            fixture.runtime_home_path(),
            DiagnosticSessionStart {
                session_id: &session_id,
                connection_id: Some(fixture.connection_id()),
                project_id: Some(fixture.project_id()),
                transport: DiagnosticTransport::McpStdio,
                host_kind: Some(DiagnosticHostKind::Codex),
                package_version: "0.2.0",
                build_id: "0.2.0;git=unknown",
            },
        )
        .expect("session");
        for duration in [100_u64, 300] {
            record_workflow_metric_event(
                fixture.runtime_home_path(),
                &WorkflowMetricEvent {
                    session_id: session_id.clone(),
                    metric_kind: WorkflowMetricKind::TaskDurationMicros,
                    value: duration,
                    method_name: None,
                    integration_profile: Some(IntegrationProfile::Record),
                    decision: None,
                    observation_confidence: None,
                    outcome: None,
                },
            )
            .expect("task duration");
        }
        record_workflow_metric_event(
            fixture.runtime_home_path(),
            &WorkflowMetricEvent {
                session_id: session_id.clone(),
                metric_kind: WorkflowMetricKind::McpMethodCall,
                value: 1,
                method_name: Some(MethodName::Status),
                integration_profile: Some(IntegrationProfile::Record),
                decision: None,
                observation_confidence: None,
                outcome: Some(WorkflowMetricOutcome::Success),
            },
        )
        .expect("method call");
        record_workflow_metric_event(
            fixture.runtime_home_path(),
            &WorkflowMetricEvent {
                session_id: session_id.clone(),
                metric_kind: WorkflowMetricKind::ObservationAssessment,
                value: 1,
                method_name: None,
                integration_profile: Some(IntegrationProfile::Record),
                decision: None,
                observation_confidence: Some(ObservationConfidence::Confirmed),
                outcome: Some(WorkflowMetricOutcome::ProductFileWrite),
            },
        )
        .expect("observation");
        for sample in [0_u64, 1] {
            record_workflow_metric_event(
                fixture.runtime_home_path(),
                &WorkflowMetricEvent {
                    session_id: session_id.clone(),
                    metric_kind: WorkflowMetricKind::ConfirmedUnrecordedFalsePositive,
                    value: sample,
                    method_name: None,
                    integration_profile: Some(IntegrationProfile::Record),
                    decision: None,
                    observation_confidence: None,
                    outcome: None,
                },
            )
            .expect("binary false-positive sample");
        }

        let output = run_diagnostics_command(
            workflow_metrics_args(&fixture.product_repo_path()),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("workflow metrics report");
        let report: Value = serde_json::from_str(&output).expect("JSON");
        assert_eq!(report["status"], "available");
        assert_eq!(
            report["duration_measurements"]["task_duration_micros"]["value"]["sample_count"],
            2
        );
        assert_eq!(
            report["duration_measurements"]["task_duration_micros"]["value"]["average"],
            200
        );
        assert_eq!(
            report["duration_measurements"]["first_product_write_duration_micros"]["status"],
            "measurement_pending"
        );
        assert_eq!(
            report["confirmed_unrecorded_false_positive_rate"]["status"],
            "available"
        );
        assert_eq!(
            report["confirmed_unrecorded_false_positive_rate"]["numerator"],
            1
        );
        assert_eq!(
            report["confirmed_unrecorded_false_positive_rate"]["denominator"],
            2
        );
        assert_eq!(
            report["confirmed_unrecorded_false_positive_rate"]["value"],
            0.5
        );
        assert!(report["aggregates"]
            .as_array()
            .expect("aggregate rows")
            .iter()
            .any(|row| row["metric_kind"] == "mcp_method_call"
                && row["method_name"] == "volicord.status"));
        for forbidden in [
            "SENSITIVE_COMMAND_SENTINEL",
            "SENSITIVE_PROMPT_SENTINEL",
            "SENSITIVE_FILE_BODY_SENTINEL",
            "SENSITIVE_USER_ANSWER_SENTINEL",
        ] {
            assert!(!output.contains(forbidden));
        }
    }

    #[test]
    fn diagnostics_cannot_change_authority_state_evidence_close_assurance_or_user_actions() {
        let fixture = CoreFixture::new("diagnostics-authority-isolation").expect("fixture");
        let core = CoreService::new(fixture.runtime_home_path());
        let session = volicord_test_support::seed_test_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
            None,
        )
        .expect("managed Agent Session fixture should seed");
        let validated = core
            .validate_agent_session(
                AgentConnectionId::new(fixture.connection_id()),
                ProjectId::new(fixture.project_id()),
                session.runtime_session_id,
                session.project_session_id,
                OperationCategory::AgentWorkflow,
            )
            .expect("managed Agent Session fixture should validate");
        let invocation = InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::AgentWorkflow,
            "",
        )
        .with_validated_agent_session(validated);
        let intake = core
            .intake(
                fixture.intake_request("req_diag_intake", "idem_diag_intake", false, Some(0)),
                invocation.clone(),
            )
            .expect("intake");
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task id");
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version");
        core.request_user_action(
            fixture.user_action_request(UserActionFixture {
                request_id: "req_diag_user_action",
                idempotency_key: "idem_diag_user_action",
                dry_run: false,
                expected_state_version: Some(state_version),
                task_id,
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            invocation,
        )
        .expect("pending user action");
        let before = fixture.authority_snapshot().expect("authority snapshot");
        let session_id = "mcp_runtime_session_isolation".to_owned();

        start_diagnostic_session(
            fixture.runtime_home_path(),
            DiagnosticSessionStart {
                session_id: &session_id,
                connection_id: Some(fixture.connection_id()),
                project_id: Some(fixture.project_id()),
                transport: DiagnosticTransport::McpStdio,
                host_kind: Some(DiagnosticHostKind::Codex),
                package_version: "0.2.0",
                build_id: "0.2.0;git=unknown",
            },
        )
        .expect("diagnostic session");
        record_diagnostic_event(
            fixture.runtime_home_path(),
            DiagnosticEvent {
                session_id: &session_id,
                event_kind: DiagnosticEventKind::McpToolCall,
                tool_name: Some("volicord.request_user_action"),
                latency_micros: 120,
                request_bytes: 500,
                response_bytes: 900,
                validation_failure: false,
                core_reached: true,
                core_committed: true,
                replayed: false,
                user_channel_kind: None,
                fallback_kind: Some(DiagnosticFallbackKind::CliInbox),
                product_file_write_count: 0,
                authoritative_refresh_failure: true,
                outcome: DiagnosticOutcome::Success,
            },
        )
        .expect("diagnostic event");
        let _ = run_diagnostics_command(
            workflow_metrics_args(&fixture.product_repo_path()),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("diagnostics read");

        assert_eq!(
            fixture.authority_snapshot().expect("authority snapshot"),
            before
        );

        fs::write(
            volicord_store::diagnostics::diagnostics_db_path(fixture.runtime_home_path()),
            b"not a sqlite diagnostics database",
        )
        .expect("corrupt only diagnostics storage");
        let error = run_diagnostics_command(
            workflow_metrics_args(&fixture.product_repo_path()),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect_err("corrupt diagnostics should fail only its own report");
        assert!(matches!(error, DiagnosticsCommandError::Runtime(_)));
        assert_eq!(
            fixture.authority_snapshot().expect("authority snapshot"),
            before
        );
    }
}
