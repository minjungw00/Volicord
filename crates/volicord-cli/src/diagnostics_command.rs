use std::{fmt, path::Path};

use serde::Serialize;
use serde_json::json;
use volicord_store::{
    diagnostics::{
        read_diagnostic_session, DiagnosticSessionAggregate, DIAGNOSTICS_DB_FILE,
        DIAGNOSTICS_MAX_EVENTS_PER_SESSION, DIAGNOSTICS_MAX_SESSIONS, DIAGNOSTICS_RETENTION_DAYS,
        DIAGNOSTICS_SCHEMA_VERSION,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

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

pub fn diagnostics_usage() -> String {
    concat!(
        "volicord diagnostics session [--session ID] [--json]\n",
        "volicord diagnostics --help\n"
    )
    .to_owned()
}

/// Renders bounded local session diagnostics without opening an authority database.
pub fn run_diagnostics_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, DiagnosticsCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            if args.len() <= 1 {
                return Ok(diagnostics_usage());
            }
            return Err(DiagnosticsCommandError::Usage(format!(
                "unexpected argument: {}\n\n{}",
                args[1],
                diagnostics_usage()
            )));
        }
        Some("session") => {}
        Some(other) => {
            return Err(DiagnosticsCommandError::Usage(format!(
                "unknown diagnostics command: {other}\n\n{}",
                diagnostics_usage()
            )));
        }
    }

    let options = parse_session_options(&args[1..])?;
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let aggregate = read_diagnostic_session(&runtime_home, options.session_id.as_deref())?;
    if options.json {
        render_json(aggregate)
    } else {
        Ok(render_text(aggregate))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SessionOptions {
    session_id: Option<String>,
    json: bool,
}

fn parse_session_options(args: &[String]) -> Result<SessionOptions, DiagnosticsCommandError> {
    let mut options = SessionOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                if options.json {
                    return Err(usage_error("--json was supplied more than once"));
                }
                options.json = true;
                index += 1;
            }
            "--session" => {
                if options.session_id.is_some() {
                    return Err(usage_error("--session was supplied more than once"));
                }
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.starts_with('-'))
                    .ok_or_else(|| usage_error("--session requires a value"))?;
                options.session_id = Some(value.clone());
                index += 1;
            }
            "-h" | "--help" | "help" => {
                return Err(usage_error(
                    "help cannot be combined with diagnostics session options",
                ));
            }
            option if option.starts_with('-') => {
                return Err(usage_error(format!("unknown option: {option}")));
            }
            argument => {
                return Err(usage_error(format!("unexpected argument: {argument}")));
            }
        }
    }
    Ok(options)
}

fn usage_error(message: impl Into<String>) -> DiagnosticsCommandError {
    DiagnosticsCommandError::Usage(format!("{}\n\n{}", message.into(), diagnostics_usage()))
}

#[derive(Debug, Serialize)]
struct DiagnosticsReport {
    schema_version: u32,
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

fn diagnostics_report(aggregate: Option<DiagnosticSessionAggregate>) -> DiagnosticsReport {
    let build = volicord_mcp::build_info();
    DiagnosticsReport {
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        status: if aggregate.is_some() { "available" } else { "no_data" },
        scope: "bounded_local_operability_only",
        storage: DiagnosticsStorageReport {
            database_file: DIAGNOSTICS_DB_FILE,
            retention_days: DIAGNOSTICS_RETENTION_DAYS,
            max_sessions: DIAGNOSTICS_MAX_SESSIONS,
            max_events_per_session: DIAGNOSTICS_MAX_EVENTS_PER_SESSION,
        },
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
    }
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
    serde_json::to_string_pretty(&diagnostics_report(aggregate))
        .map(|output| format!("{output}\n"))
        .map_err(|error| DiagnosticsCommandError::Runtime(error.to_string()))
}

fn render_text(aggregate: Option<DiagnosticSessionAggregate>) -> String {
    let report = diagnostics_report(aggregate);
    let Some(session) = report.session else {
        return concat!(
            "diagnostics session\n",
            "status: no_data\n",
            "scope: bounded local operability only\n",
            "authority_effect: none\n"
        )
        .to_owned();
    };
    let channels = serde_json::to_string(&session.user_channel_counts)
        .unwrap_or_else(|_| json!({}).to_string());
    let fallbacks =
        serde_json::to_string(&session.fallback_counts).unwrap_or_else(|_| json!({}).to_string());
    format!(
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
    )
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs};

    use rusqlite::OptionalExtension;
    use volicord_core::{CoreService, InvocationContext};
    use volicord_store::diagnostics::{
        record_diagnostic_event, start_diagnostic_session, DiagnosticEvent, DiagnosticEventKind,
        DiagnosticFallbackKind, DiagnosticHostKind, DiagnosticOutcome, DiagnosticSessionStart,
        DiagnosticTransport,
    };
    use volicord_test_support::core_fixtures::{CoreFixture, UserActionFixture};
    use volicord_types::{ActorSource, JudgmentKind, OperationCategory, ProjectId};

    use super::*;

    fn env_for(runtime_home: &Path) -> impl Fn(&str) -> Option<OsString> + '_ {
        move |name| (name == "VOLICORD_HOME").then(|| OsString::from(runtime_home))
    }

    #[test]
    fn json_report_exposes_bounded_operability_aggregates() {
        let fixture = CoreFixture::new("diagnostics-command-json").expect("fixture");
        start_diagnostic_session(
            fixture.runtime_home_path(),
            DiagnosticSessionStart {
                session_id: "session_json",
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
                session_id: "session_json",
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
            &["session".to_owned(), "--json".to_owned()],
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect("diagnostics output");
        let report: serde_json::Value = serde_json::from_str(&output).expect("JSON");
        assert_eq!(report["status"], "available");
        assert_eq!(report["session"]["totals"]["core_reached_count"], 1);
        assert_eq!(report["session"]["fallback_counts"]["cli_inbox"], 1);
        assert_eq!(
            report["authority_isolation"]["changes_state_version"],
            false
        );
        assert_eq!(report["redaction"]["stores_secret_text"], false);
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
        let invocation = InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::AgentWorkflow,
            "mcp_stdio_connection_binding",
        );
        let intake = core
            .intake(
                fixture.intake_request("req_diag_intake", "idem_diag_intake", false, Some(0)),
                invocation.clone(),
            )
            .expect("intake");
        let task_id = intake.response_value["state"]["task_ref"]["record_id"]
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

        start_diagnostic_session(
            fixture.runtime_home_path(),
            DiagnosticSessionStart {
                session_id: "session_isolation",
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
                session_id: "session_isolation",
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
            &["session".to_owned(), "--json".to_owned()],
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
            &["session".to_owned(), "--json".to_owned()],
            env_for(fixture.runtime_home_path()),
            fixture.product_repo_path().as_path(),
        )
        .expect_err("corrupt diagnostics should fail only its own report");
        assert!(matches!(error, DiagnosticsCommandError::Runtime(_)));
        assert_eq!(authority_snapshot(&fixture), before);
    }
}
