use std::{collections::BTreeMap, fmt, path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Map, Value};
use volicord_store::{
    agent_connections::{agent_connection_record, list_connection_projects_for_diagnostics},
    bootstrap::project_record_by_repo_root_read_only,
    diagnostic_findings::{
        bounded_diagnostic_graph_from_seeds, diagnostic_findings_by_ids,
        diagnostic_occurrences_for_runtime_session,
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
    AgentRuntimeSessionId, ConnectionCheck, ConnectionCheckDetails, ConnectionCheckKind,
    ConnectionCheckStatus, ConnectionStatus, DiagnosticAction, DiagnosticCode,
    DiagnosticConnectionContext, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts,
    DiagnosticFinding, DiagnosticFindingId, DiagnosticOperation, DiagnosticReport,
    DiagnosticReportAction, DiagnosticSeverity, DiagnosticSource, DiagnosticStage,
    DiagnosticSubject, IntegrationProfile, IntegrationRevision, UtcTimestamp,
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
    FailureOutput(String),
}

impl fmt::Display for DiagnosticsCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::FailureOutput(message) => {
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
    let report = if let Some(finding) =
        diagnostic_findings_by_ids(&runtime_home, std::slice::from_ref(&finding_id))?
            .into_iter()
            .next()
    {
        let findings = cause_chain_findings(&runtime_home, std::slice::from_ref(&finding))?;
        let roots = volicord_types::diagnostic_root_cause_ids(
            &findings,
            std::slice::from_ref(&finding_id),
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
        )
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?;
        let check = diagnostic_check(
            ConnectionCheckKind::DiagnosticLookup,
            ConnectionCheckStatus::Failed,
            "diagnostic_finding_found",
            "The requested diagnostic finding and bounded cause chain were loaded",
            roots,
            json!({"finding_id": finding_id.as_str(), "observation_state": "observed"}),
        )?;
        let context = finding_context(&runtime_home, &finding, &findings)?;
        build_report(
            DiagnosticOperation::DiagnosticsShow,
            ConnectionStatus::Failed,
            context,
            vec![check],
            findings,
            None,
        )?
    } else {
        missing_lookup_report(
            DiagnosticOperation::DiagnosticsShow,
            ConnectionCheckKind::DiagnosticLookup,
            "finding",
            &options.finding_id,
        )?
    };
    render_lookup_report(&report, options.json)
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
            diagnostic_occurrences_for_runtime_session(&runtime_home, &options.runtime_session_id)?
                .into_iter()
                .map(|finding| finding.to_diagnostic_finding())
                .collect::<Vec<_>>();
        let findings = cause_chain_findings(&runtime_home, &direct)?;
        let terminal_id = session
            .terminal_finding_id
            .as_deref()
            .map(DiagnosticFindingId::parse)
            .transpose()
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?;
        let (status, check_status, code, summary, causes) = if let Some(terminal_id) = terminal_id {
            (
                ConnectionStatus::Failed,
                ConnectionCheckStatus::Failed,
                "runtime_session_terminal_failure",
                "The runtime session ended with a typed terminal finding",
                vec![terminal_id],
            )
        } else if session.graceful_close_at.is_some() {
            (
                ConnectionStatus::Complete,
                ConnectionCheckStatus::Passed,
                "runtime_session_complete",
                "The runtime session completed without a terminal finding",
                Vec::new(),
            )
        } else {
            (
                ConnectionStatus::ActionRequired,
                ConnectionCheckStatus::Pending,
                "runtime_session_observation_incomplete",
                "The runtime session has not reached a terminal observation",
                Vec::new(),
            )
        };
        let check = diagnostic_check(
            ConnectionCheckKind::RuntimeSessionLookup,
            check_status,
            code,
            summary,
            causes,
            json!({
                "runtime_session_id": session.runtime_session_id,
                "session_source": session.session_source,
                "process_id": session.process_id,
                "process_started_at": session.process_started_at,
                "attempted_client_info": {
                    "name": session.attempted_client_name,
                    "version": session.attempted_client_version,
                },
                "requested_protocol_version": session.requested_protocol_version,
                "selected_protocol_version": session.selected_protocol_version,
                "negotiated_protocol_version": session.negotiated_protocol_version,
                "initialize_completed_at": session.initialize_completed_at,
                "initialized_notification_at": session.initialized_notification_at,
                "tools_list_observed_at": session.tools_list_observed_at,
                "required_tools_present": session.required_tools_present,
                "verification_tool_name": session.verification_tool_name,
                "verification_tool_observed_at": session.verification_tool_observed_at,
                "last_observed_at": session.last_observed_at,
                "terminal_finding_id": session.terminal_finding_id,
                "graceful_close_at": session.graceful_close_at,
            }),
        )?;
        let context = runtime_session_context(&runtime_home, &session, &findings)?;
        build_report(
            DiagnosticOperation::DiagnosticsSession,
            status,
            context,
            vec![check],
            findings,
            None,
        )?
    } else {
        missing_lookup_report(
            DiagnosticOperation::DiagnosticsSession,
            ConnectionCheckKind::RuntimeSessionLookup,
            "runtime_session",
            &options.runtime_session_id,
        )?
    };
    render_lookup_report(&report, options.json)
}

fn cause_chain_findings(
    runtime_home: &Path,
    direct: &[DiagnosticFinding],
) -> Result<Vec<DiagnosticFinding>, DiagnosticsCommandError> {
    let mut findings = BTreeMap::new();
    for finding in direct {
        let chain = bounded_diagnostic_graph_from_seeds(
            runtime_home,
            std::slice::from_ref(finding.id()),
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
        )?;
        for entry in chain.entries {
            findings.insert(entry.finding.id().clone(), entry.finding);
            if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
                return Err(DiagnosticsCommandError::Runtime(
                    "diagnostic lookup exceeded the shared finding bound".to_owned(),
                ));
            }
        }
    }
    Ok(findings.into_values().collect())
}

fn diagnostic_check(
    kind: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    causes: Vec<DiagnosticFindingId>,
    details: Value,
) -> Result<ConnectionCheck, DiagnosticsCommandError> {
    let Value::Object(details) = details else {
        return Err(DiagnosticsCommandError::Runtime(
            "diagnostic lookup details must be an object".to_owned(),
        ));
    };
    ConnectionCheck::try_new(
        kind,
        status,
        causes,
        Some(code.to_owned()),
        summary,
        Some(
            ConnectionCheckDetails::try_new(details)
                .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        ),
        Some(current_timestamp()),
    )
    .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))
}

#[derive(Serialize)]
struct MissingLookupFacts<'a> {
    summary: &'static str,
    observation_state: &'static str,
    requested_kind: &'static str,
    requested_id: &'a str,
}

impl DiagnosticFactSource for MissingLookupFacts<'_> {}

fn missing_lookup_report(
    operation: DiagnosticOperation,
    check_kind: ConnectionCheckKind,
    requested_kind: &'static str,
    requested_id: &str,
) -> Result<DiagnosticReport, DiagnosticsCommandError> {
    let (finding_id, code, summary) = match requested_kind {
        "finding" => (
            "finding.diagnostics.lookup.finding_missing",
            "diagnostics.lookup.finding_missing",
            "The requested diagnostic finding does not exist",
        ),
        _ => (
            "finding.diagnostics.lookup.runtime_session_missing",
            "diagnostics.lookup.runtime_session_missing",
            "The requested runtime session does not exist",
        ),
    };
    let finding = DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(finding_id)
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        DiagnosticCode::parse(code)
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        DiagnosticDomain::parse("diagnostics")
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        DiagnosticStage::parse("lookup")
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("administrative_cli")
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        DiagnosticSubject::try_new(requested_kind, requested_id)
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        DiagnosticFacts::project(&MissingLookupFacts {
            summary,
            observation_state: "absent",
            requested_kind,
            requested_id,
        })
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?,
        current_timestamp(),
    )
    .and_then(|finding| {
        finding.with_actions(vec![DiagnosticAction::try_new(
            DiagnosticCode::parse("action.diagnostics.check_identifier")?,
            "Check the exact diagnostic identifier and Runtime Home",
        )?])
    })
    .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?;
    let check = diagnostic_check(
        check_kind,
        ConnectionCheckStatus::Failed,
        "diagnostic_lookup_missing",
        summary,
        vec![finding.id().clone()],
        json!({
            "requested_kind": requested_kind,
            "requested_id": requested_id,
            "observation_state": "absent",
        }),
    )?;
    build_report(
        operation,
        ConnectionStatus::Failed,
        None,
        vec![check],
        vec![finding],
        None,
    )
}

fn finding_context(
    runtime_home: &Path,
    finding: &DiagnosticFinding,
    findings: &[DiagnosticFinding],
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
        findings,
        Vec::new(),
    )
    .map(Some)
}

fn runtime_session_context(
    runtime_home: &Path,
    session: &volicord_store::operational_sessions::McpRuntimeSessionRecord,
    findings: &[DiagnosticFinding],
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
        findings,
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
    findings: &[DiagnosticFinding],
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
    runtime_session_ids.extend(
        findings
            .iter()
            .filter_map(|finding| finding.runtime_session_id().cloned()),
    );
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

fn build_report(
    operation: DiagnosticOperation,
    status: ConnectionStatus,
    connection: Option<DiagnosticConnectionContext>,
    checks: Vec<ConnectionCheck>,
    findings: Vec<DiagnosticFinding>,
    operation_details: Option<Map<String, Value>>,
) -> Result<DiagnosticReport, DiagnosticsCommandError> {
    let selected = checks
        .iter()
        .flat_map(|check| check.cause_finding_ids().iter().cloned())
        .collect::<Vec<_>>();
    let roots = if selected.is_empty() {
        Vec::new()
    } else {
        volicord_types::diagnostic_root_cause_ids(
            &findings,
            &selected,
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
        )
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    };
    let mut actions = BTreeMap::<String, (String, Vec<DiagnosticFindingId>)>::new();
    for root in &roots {
        let Some(action) = findings
            .iter()
            .find(|finding| finding.id() == root)
            .and_then(|finding| finding.actions().first())
        else {
            continue;
        };
        let entry = actions
            .entry(action.code().to_string())
            .or_insert_with(|| (action.summary().to_owned(), Vec::new()));
        entry.1.push(root.clone());
    }
    let actions = actions
        .into_iter()
        .map(|(code, (summary, roots))| {
            DiagnosticReportAction::try_new(DiagnosticCode::parse(code)?, summary, roots)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?;
    DiagnosticReport::try_new(
        operation,
        status,
        current_timestamp(),
        connection,
        checks,
        findings,
        actions,
        operation_details,
        diagnostic_report_limits(),
    )
    .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))
}

fn diagnostic_report_limits() -> Vec<String> {
    vec![
        format!(
            "Diagnostic cause traversal is bounded to {} edges and {} findings.",
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH, MAX_DIAGNOSTIC_FINDINGS
        ),
        "Only bounded typed diagnostic facts are rendered; sensitive fields remain redacted."
            .to_owned(),
    ]
}

fn render_lookup_report(
    report: &DiagnosticReport,
    json: bool,
) -> Result<String, DiagnosticsCommandError> {
    let output = if json {
        serde_json::to_string_pretty(report)
            .map(|output| format!("{output}\n"))
            .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))?
    } else {
        render_lookup_text(report)
    };
    if report.status() == ConnectionStatus::Failed {
        Err(DiagnosticsCommandError::FailureOutput(output))
    } else {
        Ok(output)
    }
}

fn render_lookup_text(report: &DiagnosticReport) -> String {
    let mut lines = vec![
        format!("Diagnostic report: {}", report.status().as_str()),
        format!("Operation: {}", report.operation().as_str()),
    ];
    for check in report.checks() {
        lines.push(format!(
            "Check {}: {} — {}",
            check.id().as_str(),
            check.status().as_str(),
            check.summary()
        ));
        if let Some(details) = check.details() {
            lines.push(format!(
                "  Bounded check facts: {}",
                serde_json::to_string(details.as_object()).unwrap_or_else(|_| "{}".to_owned())
            ));
        }
    }
    if let Some(connection) = report.connection() {
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
    for finding in report.findings() {
        let role = if report.root_cause_ids().contains(finding.id()) {
            "root"
        } else {
            "related"
        };
        lines.push(format!("Finding ({role}): {}", finding.id()));
        lines.push(format!("  Code: {}", finding.code()));
        if let Some(summary) = finding
            .facts()
            .data()
            .get("summary")
            .and_then(Value::as_str)
        {
            lines.push(format!("  Summary: {summary}"));
        }
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
    }
    for action in report.actions() {
        lines.push(format!("Next: {} — {}", action.code(), action.summary()));
    }
    for limit in report.limits() {
        lines.push(format!("Limit: {limit}"));
    }
    format!("{}\n", lines.join("\n"))
}

fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
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
        diagnostic_findings::upsert_current_snapshot,
        operational_sessions::connection_integration_revision,
    };
    use volicord_test_support::core_fixtures::{CoreFixture, UserActionFixture};
    use volicord_types::{
        ActorSource, AgentConnectionId, CurrentDiagnosticFinding, CurrentDiagnosticKey,
        CurrentDiagnosticSnapshot, DiagnosticScope, DiagnosticScopeKind, DiagnosticSubjectIdentity,
        IntegrationProfile, JudgmentKind, MethodName, ObservationConfidence, OperationCategory,
        ProjectId,
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
                "diagnostic_lookup",
                "finding.diagnostics.lookup.finding_missing",
                "diagnostics.lookup.finding_missing",
            ),
            (
                DiagnosticsCommand::Session(DiagnosticsSessionArgs {
                    runtime_session_id: "runtime_session_does_not_exist".to_owned(),
                    json: true,
                }),
                "diagnostics_session",
                "runtime_session_lookup",
                "finding.diagnostics.lookup.runtime_session_missing",
                "diagnostics.lookup.runtime_session_missing",
            ),
        ];
        for (command, operation, check_id, finding_id, finding_code) in cases {
            let error = run_diagnostics_command(
                DiagnosticsArgs { command },
                env_for(fixture.runtime_home_path()),
                fixture.product_repo_path().as_path(),
            )
            .expect_err("missing lookup must use the typed failure output channel");
            let DiagnosticsCommandError::FailureOutput(output) = error else {
                panic!("missing lookup returned an ad hoc error: {error}");
            };
            let report: Value = serde_json::from_str(&output).expect("diagnostic report JSON");
            assert_eq!(report["schema_version"], 2);
            assert_eq!(report["operation"], operation);
            assert_eq!(report["status"], "failed");
            assert_eq!(report["checks"][0]["id"], check_id);
            assert_eq!(report["checks"][0]["status"], "failed");
            assert_eq!(
                report["checks"][0]["details"]["observation_state"],
                "absent"
            );
            assert_eq!(report["root_cause_ids"], json!([finding_id]));
            assert_eq!(report["findings"][0]["id"], finding_id);
            assert_eq!(report["findings"][0]["code"], finding_code);
            assert_eq!(
                report["findings"][0]["facts"]["data"]["observation_state"],
                "absent"
            );
            assert_eq!(
                report["actions"],
                json!([{
                    "code": "action.diagnostics.check_identifier",
                    "summary": "Check the exact diagnostic identifier and Runtime Home",
                    "root_cause_ids": [finding_id],
                }])
            );
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

        let error = run_diagnostics_command(
            DiagnosticsArgs {
                command: DiagnosticsCommand::Show(DiagnosticsShowArgs {
                    finding_id: finding_id.to_string(),
                    json: true,
                }),
            },
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect_err("finding lookup uses the typed failure output channel");
        let DiagnosticsCommandError::FailureOutput(output) = error else {
            panic!("finding lookup returned an ad hoc error: {error}");
        };
        let report: Value = serde_json::from_str(&output).expect("diagnostic report JSON");
        assert_eq!(report["findings"][0]["id"], finding_id.as_str());
        assert_eq!(
            report["findings"][0]["subject"]["reference"],
            "/bounded/current/config.toml"
        );
        assert_eq!(
            report["findings"][0]["facts"]["data"]["observed_state"],
            "drift"
        );
        assert_eq!(report["findings"][0]["observed_at"], "2026-07-22T02:03:04Z");
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
