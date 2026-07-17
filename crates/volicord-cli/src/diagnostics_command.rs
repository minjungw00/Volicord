use std::{fmt, path::Path};

use serde::Serialize;
use volicord_store::{
    bootstrap::project_record_by_repo_root_read_only,
    diagnostics::{
        current_diagnostics_storage_manifest, read_diagnostic_session,
        read_workflow_metric_aggregates, DiagnosticSessionAggregate, WorkflowMetricAggregateRow,
        DIAGNOSTICS_DB_FILE, DIAGNOSTICS_MAX_EVENTS_PER_SESSION, DIAGNOSTICS_MAX_SESSIONS,
        DIAGNOSTICS_RETENTION_DAYS,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

use crate::cli::{DiagnosticsArgs, DiagnosticsCommand, DiagnosticsWorkflowMetricsArgs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsCommandError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for DiagnosticsCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
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
        DiagnosticsCommand::Session(options) => {
            let runtime_home = resolve_runtime_home(env_var, current_dir)?;
            let aggregate = read_diagnostic_session(&runtime_home, options.session.as_deref())?;
            if options.json {
                render_json(aggregate)
            } else {
                render_text(aggregate)
            }
        }
        DiagnosticsCommand::WorkflowMetrics(options) => {
            run_workflow_metrics(options, env_var, current_dir)
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
struct DiagnosticsReport {
    status: &'static str,
    scope: &'static str,
    storage: DiagnosticsStorageReport,
    redaction: DiagnosticsRedactionReport,
    authority_isolation: AuthorityIsolationReport,
    current_build: CurrentBuildReport,
    session: Option<DiagnosticSessionReport>,
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
struct DiagnosticsRedactionReport {
    stores_prompt_text: bool,
    stores_file_content_or_paths: bool,
    stores_secret_text: bool,
    stores_user_action_form_or_resolution_text: bool,
    stored_detail: &'static str,
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
struct CurrentBuildReport {
    package_version: &'static str,
    build_id: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticSessionReport {
    session_id: String,
    connection_id: Option<String>,
    project_id: Option<String>,
    transport: String,
    host_kind: Option<String>,
    producer_build: ProducerBuildReport,
    started_at: String,
    updated_at: String,
    tools: Vec<DiagnosticToolReport>,
    totals: volicord_store::diagnostics::DiagnosticTotals,
    user_channel_counts: std::collections::BTreeMap<String, u64>,
    fallback_counts: std::collections::BTreeMap<String, u64>,
}

#[derive(Debug, Serialize)]
struct ProducerBuildReport {
    package_version: String,
    build_id: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticToolReport {
    tool_name: String,
    call_count: u64,
    latency_micros_total: u64,
    latency_micros_max: u64,
    latency_micros_average: u64,
    request_bytes: u64,
    response_bytes: u64,
    validation_failures: u64,
    retries_after_validation_failure: u64,
    core_reached_count: u64,
    core_committed_count: u64,
    replayed_count: u64,
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

fn diagnostics_report(
    aggregate: Option<DiagnosticSessionAggregate>,
) -> Result<DiagnosticsReport, DiagnosticsCommandError> {
    let build = volicord_mcp::build_info();
    Ok(DiagnosticsReport {
        status: if aggregate.is_some() { "available" } else { "no_data" },
        scope: "bounded_local_operability_only",
        storage: diagnostics_storage_report()?,
        redaction: DiagnosticsRedactionReport {
            stores_prompt_text: false,
            stores_file_content_or_paths: false,
            stores_secret_text: false,
            stores_user_action_form_or_resolution_text: false,
            stored_detail: "allowlisted identifiers, categorical outcomes, counters, byte sizes, and latency only",
        },
        authority_isolation: AuthorityIsolationReport {
            project_state_database_opened_by_report: false,
            changes_state_version: false,
            changes_evidence_or_assurance: false,
            changes_close_readiness: false,
            changes_user_actions: false,
        },
        current_build: CurrentBuildReport {
            package_version: build.package_version,
            build_id: build.build_id,
        },
        session: aggregate.map(session_report),
    })
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

fn session_report(aggregate: DiagnosticSessionAggregate) -> DiagnosticSessionReport {
    let tools = aggregate
        .tools
        .into_iter()
        .map(|tool| DiagnosticToolReport {
            tool_name: tool.tool_name,
            call_count: tool.call_count,
            latency_micros_total: tool.latency_micros_total,
            latency_micros_max: tool.latency_micros_max,
            latency_micros_average: tool
                .latency_micros_total
                .checked_div(tool.call_count)
                .unwrap_or(0),
            request_bytes: tool.request_bytes,
            response_bytes: tool.response_bytes,
            validation_failures: tool.validation_failures,
            retries_after_validation_failure: tool.retries_after_validation_failure,
            core_reached_count: tool.core_reached_count,
            core_committed_count: tool.core_committed_count,
            replayed_count: tool.replayed_count,
        })
        .collect();
    DiagnosticSessionReport {
        session_id: aggregate.session_id,
        connection_id: aggregate.connection_id,
        project_id: aggregate.project_id,
        transport: aggregate.transport,
        host_kind: aggregate.host_kind,
        producer_build: ProducerBuildReport {
            package_version: aggregate.package_version,
            build_id: aggregate.build_id,
        },
        started_at: aggregate.started_at,
        updated_at: aggregate.updated_at,
        tools,
        totals: aggregate.totals,
        user_channel_counts: aggregate.user_channel_counts,
        fallback_counts: aggregate.fallback_counts,
    }
}

fn render_json(
    aggregate: Option<DiagnosticSessionAggregate>,
) -> Result<String, DiagnosticsCommandError> {
    serde_json::to_string_pretty(&diagnostics_report(aggregate)?)
        .map(|output| format!("{output}\n"))
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))
}

fn render_text(
    aggregate: Option<DiagnosticSessionAggregate>,
) -> Result<String, DiagnosticsCommandError> {
    let report = diagnostics_report(aggregate)?;
    let Some(session) = report.session else {
        return Ok(concat!(
            "diagnostics session\n",
            "status: no_data\n",
            "scope: bounded local operability only\n",
            "authority_effect: none\n"
        )
        .to_owned());
    };
    let channels = serde_json::to_string(&session.user_channel_counts)
        .expect("diagnostic user-channel count maps are JSON-serializable");
    let fallbacks = serde_json::to_string(&session.fallback_counts)
        .expect("diagnostic fallback count maps are JSON-serializable");
    Ok(format!(
        concat!(
            "diagnostics session\n",
            "status: available\n",
            "session_id: {}\n",
            "transport: {}\n",
            "tool_calls: {}\n",
            "validation_failures: {}\n",
            "retries_after_validation_failure: {}\n",
            "core_reached: {}\n",
            "core_committed: {}\n",
            "product_file_writes_observed: {}\n",
            "authoritative_refresh_failures: {}\n",
            "user_channels: {}\n",
            "fallbacks: {}\n",
            "authority_effect: none\n"
        ),
        session.session_id,
        session.transport,
        session.totals.tool_call_count,
        session.totals.validation_failures,
        session.totals.retries_after_validation_failure,
        session.totals.core_reached_count,
        session.totals.core_committed_count,
        session.totals.product_file_write_count,
        session.totals.authoritative_refresh_failures,
        channels,
        fallbacks,
    ))
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs};

    use rusqlite::OptionalExtension;
    use serde_json::Value;
    use volicord_core::{validate_host_verification_receipt, CoreService, InvocationContext};
    use volicord_store::diagnostics::{
        diagnostics_db_path, record_diagnostic_event, record_workflow_metric_event,
        start_diagnostic_session, DiagnosticEvent, DiagnosticEventKind, DiagnosticFallbackKind,
        DiagnosticHostKind, DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport,
        WorkflowMetricEvent, WorkflowMetricKind, WorkflowMetricOutcome,
    };
    use volicord_test_support::{
        core_fixtures::{CoreFixture, UserActionFixture},
        test_host_receipt_fixture,
    };
    use volicord_types::{
        managed_stdio_session_id, ActorSource, IntegrationProfile, JudgmentKind, MethodName,
        ObservationConfidence, OperationCategory, ProjectId,
        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
    };

    use crate::cli::{DiagnosticsSessionArgs, DiagnosticsWorkflowMetricsArgs};

    use super::*;

    fn env_for(runtime_home: &Path) -> impl Fn(&str) -> Option<OsString> + '_ {
        move |name| (name == "VOLICORD_HOME").then(|| OsString::from(runtime_home))
    }

    fn session_args(json: bool) -> DiagnosticsArgs {
        DiagnosticsArgs {
            command: DiagnosticsCommand::Session(DiagnosticsSessionArgs {
                session: None,
                json,
            }),
        }
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
    fn json_report_exposes_bounded_operability_aggregates() {
        let fixture = CoreFixture::new("diagnostics-command-json").expect("fixture");
        let session_id = managed_stdio_session_id(fixture.connection_id(), "session_json")
            .expect("managed session coordinate");
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
        record_diagnostic_event(
            fixture.runtime_home_path(),
            DiagnosticEvent {
                session_id: &session_id,
                event_kind: DiagnosticEventKind::McpToolCall,
                tool_name: Some("volicord.status"),
                latency_micros: 90,
                request_bytes: 32,
                response_bytes: 64,
                validation_failure: false,
                core_reached: true,
                core_committed: false,
                replayed: false,
                user_channel_kind: None,
                fallback_kind: Some(DiagnosticFallbackKind::CliInbox),
                product_file_write_count: 0,
                authoritative_refresh_failure: false,
                outcome: DiagnosticOutcome::Success,
            },
        )
        .expect("event");

        let output = run_diagnostics_command(
            session_args(true),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("diagnostics output");
        let report: serde_json::Value = serde_json::from_str(&output).expect("JSON");
        assert_eq!(report["status"], "available");
        assert!(report.get("schema_version").is_none());
        assert_eq!(
            report["storage"]["contract_id"],
            "volicord.sqlite.diagnostics"
        );
        assert!(report["storage"]["canonical_schema_digest"]
            .as_str()
            .expect("schema digest")
            .starts_with("sha256:"));
        assert_eq!(report["session"]["totals"]["core_reached_count"], 1);
        assert_eq!(report["session"]["fallback_counts"]["cli_inbox"], 1);
        assert_eq!(
            report["authority_isolation"]["changes_state_version"],
            false
        );
        assert_eq!(report["redaction"]["stores_secret_text"], false);
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
        let session_id =
            managed_stdio_session_id(fixture.connection_id(), "session_workflow_metrics")
                .expect("managed session coordinate");
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

    #[derive(Debug, PartialEq, Eq)]
    struct AuthoritySnapshot {
        state_version: u64,
        enforcement_profile_json: String,
        evidence_claim_count: u64,
        evidence_summary_count: u64,
        evidence_observation_count: u64,
        assurance_levels: Option<String>,
        blocker_count: u64,
        close_state: Option<String>,
        user_action_count: u64,
        user_action_state: Option<String>,
    }

    fn authority_snapshot(fixture: &CoreFixture) -> AuthoritySnapshot {
        let conn = fixture.conn().expect("authority db");
        AuthoritySnapshot {
            state_version: conn
                .query_row("SELECT state_version FROM project_state", [], |row| row.get(0))
                .expect("state version"),
            enforcement_profile_json: conn
                .query_row("SELECT enforcement_profile_json FROM project_state", [], |row| {
                    row.get(0)
                })
                .expect("enforcement profile"),
            evidence_claim_count: count(&conn, "evidence_claims"),
            evidence_summary_count: count(&conn, "evidence_summaries"),
            evidence_observation_count: count(&conn, "evidence_observations"),
            assurance_levels: conn
                .query_row(
                    "SELECT group_concat(assurance_level, ',') FROM evidence_observations",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .expect("assurance levels")
                .flatten(),
            blocker_count: count(&conn, "blockers"),
            close_state: conn
                .query_row(
                    "SELECT group_concat(lifecycle_phase || ':' || close_basis_revision, ',') FROM tasks",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .expect("close state")
                .flatten(),
            user_action_count: count(&conn, "user_action_requests"),
            user_action_state: conn
                .query_row(
                    "SELECT group_concat(r.action_kind || ':' || COALESCE(s.resolved_verification_basis, 'pending'), ',') FROM user_action_requests r LEFT JOIN user_action_resolutions s ON s.user_action_request_id = r.user_action_request_id",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .expect("user-action state")
                .flatten(),
        }
    }

    fn count(conn: &rusqlite::Connection, table: &str) -> u64 {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        conn.query_row(&sql, [], |row| row.get(0)).expect("count")
    }

    #[test]
    fn diagnostics_cannot_change_authority_state_evidence_close_assurance_or_user_actions() {
        let fixture = CoreFixture::new("diagnostics-authority-isolation").expect("fixture");
        let core = CoreService::new(fixture.runtime_home_path());
        let host = test_host_receipt_fixture(fixture.project_id(), fixture.connection_id());
        let receipt =
            validate_host_verification_receipt(host.receipt, &host.current, &host.validation_time)
                .expect("typed host receipt fixture should validate");
        let invocation = InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::AgentWorkflow,
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
        )
        .with_validated_host_receipt(receipt);
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
        let before = authority_snapshot(&fixture);
        let session_id = managed_stdio_session_id(fixture.connection_id(), "session_isolation")
            .expect("managed session coordinate");

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
            session_args(true),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("diagnostics read");

        assert_eq!(authority_snapshot(&fixture), before);

        fs::write(
            volicord_store::diagnostics::diagnostics_db_path(fixture.runtime_home_path()),
            b"not a sqlite diagnostics database",
        )
        .expect("corrupt only diagnostics storage");
        let error = run_diagnostics_command(
            session_args(true),
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect_err("corrupt diagnostics should fail only its own report");
        assert!(matches!(error, DiagnosticsCommandError::Runtime(_)));
        assert_eq!(authority_snapshot(&fixture), before);
    }
}
