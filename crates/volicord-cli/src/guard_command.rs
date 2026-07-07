use std::{collections::BTreeSet, ffi::OsString, fmt, fs, path::Path};

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_core::{CorePipelineError, CoreService, InvocationContext};
use volicord_store::{
    bootstrap::{project_record_for_execution, ProjectRecord},
    core_pipeline::CoreProjectStore,
    guards::{
        agent_session, guard_event, insert_agent_session, insert_expected_write,
        insert_guard_event, insert_unrecorded_change, list_expected_writes_matched_by_post_event,
        list_pending_expected_writes, list_unresolved_unrecorded_changes,
        mark_expected_write_matched, observe_guard_installation, unrecorded_change,
        AgentSessionInsert, ExpectedWriteInsert, ExpectedWriteMatch, ExpectedWriteRecord,
        GuardEventInsert, GuardInstallationObservation, UnrecordedChangeInsert,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    session_watch::{
        create_watch_baseline, latest_watch_baseline_for_session, snapshot_product_repository,
        watch_scan_summary_from_entries_json, SessionWatchStatus, WatchBaselineCreate,
        WatchSnapshotOptions,
    },
    StoreError,
};
use volicord_types::{
    ActorSource, GuardDecision, IntegrationProfile, OperationCategory, ProjectId,
    PromptCaptureStatus, RequestId, SessionWatchCoverageBasis, SessionWatchScanSummary,
    StatusInclude, StatusRequest, TaskId, ToolEnvelope, UtcTimestamp, WriteTicketAttemptScope,
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
};

use crate::disclosure::{
    cooperative_host_decision_disclosure_json, COOPERATIVE_DECISION_DISCLOSURE_TEXT,
};
use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
const DEFAULT_INTEGRATION_PROFILE: &str = "detective";
const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
const EXPECTED_WRITE_TTL_MINUTES: i64 = 15;
const SESSION_WATCH_METADATA_SOURCE: &str = "volicord_session_watch";

mod args;
mod envelope;
mod mutation;
mod prompt_capture;
mod prompt_command;
mod tool_observation;

pub use args::guard_usage;
use args::{
    parse_guard_options, read_guard_input, GuardInput, GuardOptions, GuardPhase, HostOutputMode,
    OutputFormat,
};
use envelope::{
    event_bool, event_path_field, event_string, event_time_or_now, guard_envelope, GuardEnvelope,
};
use mutation::{PathAssessment, ToolClassification};
use prompt_capture::{
    handle_prompt_capture, pending_chat_judgment_summaries, pending_judgment_summary_json,
    prompt_capture_availability_for_event, GuardPendingJudgmentSummary,
};
use tool_observation::{host_invocation_id, tool_observation, ToolObservation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardCommandOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardCommandError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for GuardCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GuardCommandError {}

impl From<StoreError> for GuardCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for GuardCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for GuardCommandError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<CorePipelineError> for GuardCommandError {
    fn from(error: CorePipelineError) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedGuardOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardStateSummary {
    project_id: String,
    project_name: String,
    repo_root: String,
    state_version: u64,
    active_task_id: Option<String>,
    active_change_unit_id: Option<String>,
    prompt_capture_status: PromptCaptureStatus,
    prompt_capture_enabled: bool,
    current_write_ticket_ids: Vec<String>,
    stale_write_ticket_ids: Vec<String>,
    active_write_tickets: Vec<ActiveWriteTicketSummary>,
    pending_user_judgment_count: usize,
    pending_user_judgments: Vec<GuardPendingJudgmentSummary>,
    active_blocker_count: usize,
    unresolved_unrecorded_change_count: usize,
    session_watch_scan_summary: Option<SessionWatchScanSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveWriteTicketSummary {
    write_ticket_id: String,
    change_unit_id: Option<String>,
    intended_paths: Vec<String>,
    expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardReason {
    code: &'static str,
    message: String,
    severity: &'static str,
}

#[derive(Debug, Clone)]
struct ExpectedWriteCandidate {
    insert: ExpectedWriteInsert,
    expected_paths: Vec<String>,
    write_ticket: ActiveWriteTicketSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostToolCorrelation {
    matched_expected_writes: Vec<Value>,
    ticket_backed_observations: Vec<Value>,
    unrecorded_changes: Vec<Value>,
}

struct UnrecordedChangeContext<'a> {
    runtime_home: &'a Path,
    project: &'a ProjectRecord,
    envelope: &'a GuardEnvelope,
    summary: &'a GuardStateSummary,
    observation: &'a ToolObservation,
    changed: Vec<String>,
    correlation_status: &'static str,
    candidate_expected_write_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedWriteMatchOutcome {
    Matched(ExpectedWriteRecord),
    AlreadyMatched(ExpectedWriteRecord),
    NoCandidates,
    OutOfScope(Vec<String>),
    Ambiguous(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActiveWriteTicketMatchOutcome {
    Matched(ActiveWriteTicketSummary),
    NoActiveTickets,
    OutOfScope(Vec<String>),
    Ambiguous(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteTicketCoverage {
    NotWriteLike,
    TicketBacked {
        ticket: ActiveWriteTicketSummary,
        observed_paths: Vec<String>,
    },
    NoObservedPaths,
    NoActiveTickets {
        observed_paths: Vec<String>,
    },
    OutOfScope {
        observed_paths: Vec<String>,
        active_ticket_ids: Vec<String>,
    },
    Ambiguous {
        observed_paths: Vec<String>,
        matching_ticket_ids: Vec<String>,
    },
}

pub fn run_guard_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<GuardCommandOutcome, GuardCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(GuardCommandOutcome {
            stdout: guard_usage(),
            stderr: String::new(),
            exit_code: 0,
        });
    };
    if matches!(subcommand, "-h" | "--help" | "help") {
        if args.len() == 1 {
            return Ok(GuardCommandOutcome {
                stdout: guard_usage(),
                stderr: String::new(),
                exit_code: 0,
            });
        }
        return Err(GuardCommandError::Usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            guard_usage()
        )));
    }

    let phase = match subcommand {
        "session-start" => GuardPhase::SessionStart,
        "pre-tool" => GuardPhase::PreTool,
        "post-tool" => GuardPhase::PostTool,
        "prompt-capture" => GuardPhase::PromptCapture,
        "stop" => GuardPhase::Stop,
        other => {
            return Err(GuardCommandError::Usage(format!(
                "unknown _hook command: {other}\n\n{}",
                guard_usage()
            )))
        }
    };
    let options = parse_guard_options(&args[1..])?;
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let input = read_guard_input(options.event_file.as_deref())?;
    let project = resolve_guard_project(&runtime_home, current_dir, &options, &input.raw_value)?;
    let envelope = guard_envelope(phase, &options, &input, &project)?;
    ensure_required_session(&runtime_home, &project, &envelope, phase)?;
    let _activation =
        observe_guard_installation_activation(&runtime_home, &project, &envelope, phase, &options)?;
    initialize_observe_session_watch(&runtime_home, &project, &envelope, phase)?;

    let (decision, mut result, expected_write) = match phase {
        GuardPhase::SessionStart => {
            let summary = guard_state_summary(&runtime_home, &project, &envelope, &input)?;
            (
                GuardDecision::InjectContext,
                json!({
                    "decision": GuardDecision::InjectContext.as_str(),
                    "message": "Volicord context is available for this host session.",
                    "context": context_json(&summary),
                    "enforcement_level": "cooperative_detective"
                }),
                None,
            )
        }
        GuardPhase::PreTool => {
            let summary = guard_state_summary(&runtime_home, &project, &envelope, &input)?;
            let observation = tool_observation(&input.raw_value, &project.repo_root);
            let (decision, reasons) = pre_tool_decision(&summary, &observation, &input.raw_value);
            let write_ticket_backing = if tool_attempts_product_write(&observation) {
                write_ticket_backing_json(write_ticket_coverage(&summary, &observation))
            } else {
                write_ticket_backing_json(WriteTicketCoverage::NotWriteLike)
            };
            let expected_write = expected_write_candidate(
                &project,
                &envelope,
                &summary,
                &observation,
                &input,
                decision,
            )?;
            let expected_write_json = expected_write
                .as_ref()
                .map(expected_write_candidate_json)
                .unwrap_or(Value::Null);
            (
                decision,
                json!({
                    "decision": decision.as_str(),
                    "allowed": decision != GuardDecision::Deny,
                    "reasons": reasons_json(&reasons),
                    "tool": tool_observation_json(&observation),
                    "write_ticket_backing": write_ticket_backing,
                    "expected_write": expected_write_json,
                    "context": context_json(&summary),
                    "enforcement_level": "cooperative_detective"
                }),
                expected_write,
            )
        }
        GuardPhase::PostTool => {
            let summary = guard_state_summary(&runtime_home, &project, &envelope, &input)?;
            let observation = tool_observation(&input.raw_value, &project.repo_root);
            let correlation = record_post_tool_correlation(
                &runtime_home,
                &project,
                &envelope,
                &summary,
                &observation,
            )?;
            let decision = if correlation.unrecorded_changes.is_empty() {
                GuardDecision::Allow
            } else {
                GuardDecision::Warn
            };
            (
                decision,
                json!({
                    "decision": decision.as_str(),
                    "allowed": true,
                    "tool": tool_observation_json(&observation),
                    "matched_expected_writes": correlation.matched_expected_writes,
                    "ticket_backed_observations": correlation.ticket_backed_observations,
                    "unrecorded_changes": correlation.unrecorded_changes,
                    "context": context_json(&summary),
                    "enforcement_level": "cooperative_detective"
                }),
                None,
            )
        }
        GuardPhase::PromptCapture => {
            let (decision, result, _exits_failure) =
                handle_prompt_capture(&runtime_home, &project, &envelope, &input)?;
            (decision, result, None)
        }
        GuardPhase::Stop => {
            let summary = guard_state_summary(&runtime_home, &project, &envelope, &input)?;
            let (decision, reasons, close_status) =
                stop_decision(&runtime_home, &project, &envelope, &summary)?;
            (
                decision,
                json!({
                    "decision": decision.as_str(),
                    "allowed": decision != GuardDecision::Deny,
                    "reasons": reasons_json(&reasons),
                    "close_status": close_status,
                    "context": context_json(&summary),
                    "enforcement_level": "cooperative_detective"
                }),
                None,
            )
        }
    };
    attach_guard_disclosure(&mut result);

    let subject = guard_subject(phase, &input, &envelope, &project);
    persist_guard_event(
        &runtime_home,
        &project,
        &envelope,
        phase,
        decision,
        subject,
        result.clone(),
    )?;
    if let Some(expected_write) = expected_write {
        persist_expected_write(&runtime_home, &project, expected_write)?;
    }
    let rendered = render_guard_output(phase, decision, &envelope, result, options.output)?;
    Ok(GuardCommandOutcome {
        stdout: rendered.stdout,
        stderr: rendered.stderr,
        exit_code: rendered.exit_code,
    })
}

fn attach_guard_disclosure(result: &mut Value) {
    if let Some(object) = result.as_object_mut() {
        object.insert(
            "disclosure".to_owned(),
            cooperative_host_decision_disclosure_json(),
        );
    }
}

fn resolve_guard_project(
    runtime_home: &Path,
    current_dir: &Path,
    options: &GuardOptions,
    event: &Value,
) -> Result<ProjectRecord, GuardCommandError> {
    if let Some(repo) = options
        .repo
        .as_deref()
        .or_else(|| event_path_field(event, &[&["repo_root"], &["repository_root"], &["cwd"]]))
    {
        let repo_root = resolve_repository_root(current_dir, Some(repo))?;
        return registered_project_for_repo(runtime_home, &repo_root).map_err(Into::into);
    }
    if let Some(project_id) = event_string(event, &[&["project_id"], &["project", "id"]]) {
        return project_record_for_execution(runtime_home, &project_id)?
            .ok_or_else(|| GuardCommandError::Runtime(format!("project not found: {project_id}")));
    }
    let repo_root = resolve_repository_root(current_dir, None)?;
    registered_project_for_repo(runtime_home, &repo_root).map_err(Into::into)
}

fn ensure_required_session(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
) -> Result<(), GuardCommandError> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(());
    };
    if agent_session(runtime_home, &project.project_id, session_id)?.is_some() {
        return Ok(());
    }
    if matches!(phase, GuardPhase::SessionStart | GuardPhase::PromptCapture)
        || envelope.session_id.is_some()
    {
        insert_agent_session(
            runtime_home,
            &project.project_id,
            AgentSessionInsert {
                session_id: session_id.to_owned(),
                connection_internal_id: envelope.connection_id.clone(),
                guard_installation_id: envelope.guard_installation_id.clone(),
                host_kind: envelope.host_kind.clone(),
                guard_mode: envelope.guard_mode.clone(),
                started_at: envelope.occurred_at.clone(),
                metadata_json: json!({
                    "source": "volicord_guard_cli"
                })
                .to_string(),
            },
        )?;
    }
    Ok(())
}

fn initialize_observe_session_watch(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
) -> Result<(), GuardCommandError> {
    if phase != GuardPhase::SessionStart
        || envelope.guard_mode != IntegrationProfile::Detective.as_str()
    {
        return Ok(());
    }
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(());
    };
    if latest_watch_baseline_for_session(runtime_home, &project.project_id, session_id)?.is_some() {
        return Ok(());
    }
    let snapshot = snapshot_product_repository(
        runtime_home,
        &project.repo_root,
        WatchSnapshotOptions::default(),
    )
    .map_err(|error| {
        GuardCommandError::Runtime(format!(
            "failed to start detective session watcher for {}: {error}",
            project.repo_root.display()
        ))
    })?;
    let started_at = format_timestamp(event_time_or_now(&envelope.occurred_at));
    let watch_baseline_id = stable_id(
        "watch_base",
        &[
            &project.project_id,
            session_id,
            &envelope.connection_id,
            &snapshot.digest,
        ],
    );
    let scan_summary = session_watch_scan_summary_from_snapshot(&snapshot);
    create_watch_baseline(
        runtime_home,
        &project.project_id,
        WatchBaselineCreate {
            watch_baseline_id,
            session_id: session_id.to_owned(),
            connection_internal_id: envelope.connection_id.clone(),
            guard_installation_id: envelope.guard_installation_id.clone(),
            status: SessionWatchStatus::Active,
            snapshot,
            created_at: started_at.clone(),
            metadata_json: json!({
                "source": SESSION_WATCH_METADATA_SOURCE,
                "status_detail": "active",
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true,
                "coverage_start_at": started_at,
                "coverage_basis": SessionWatchCoverageBasis::McpStart.as_str(),
                "coverage_started_by": "session_start_hook",
                "scan_summary": scan_summary
            })
            .to_string(),
        },
    )?;
    Ok(())
}

fn session_watch_scan_summary_from_snapshot(
    snapshot: &volicord_store::session_watch::WatchSnapshot,
) -> SessionWatchScanSummary {
    session_watch_scan_summary_from_store(&snapshot.scan_summary)
}

fn session_watch_scan_summary_from_store(
    summary: &volicord_store::session_watch::WatchScanSummary,
) -> SessionWatchScanSummary {
    SessionWatchScanSummary {
        files_scanned: summary.files_scanned,
        files_skipped: summary.files_skipped,
        unreadable_paths_count: summary.unreadable_paths_count,
        degraded_reasons: summary.degraded_reasons.clone(),
        degraded_reason_counts: summary.degraded_reason_counts.clone(),
        skipped_paths_sample: summary.skipped_paths_sample.clone(),
        skipped_paths_truncated: summary.skipped_paths_truncated,
        default_excluded_paths: volicord_store::session_watch::default_watch_excluded_paths(),
        max_file_size_bytes: volicord_store::session_watch::DEFAULT_MAX_FILE_HASH_BYTES,
        max_file_count: volicord_store::session_watch::DEFAULT_MAX_SCAN_FILE_COUNT,
        follows_symlinks: false,
        not_full_filesystem_monitoring: true,
    }
}

fn observe_guard_installation_activation(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    options: &GuardOptions,
) -> Result<Option<volicord_store::guards::GuardInstallationRecord>, GuardCommandError> {
    if envelope.guard_mode == IntegrationProfile::Record.as_str() {
        return Ok(None);
    }
    let Some(guard_installation_id) = envelope.guard_installation_id.clone() else {
        return Ok(None);
    };
    let Some(observed_policy_hash) = current_policy_hash(project)? else {
        return Ok(None);
    };
    if options
        .policy_hash
        .as_deref()
        .is_some_and(|expected| expected != observed_policy_hash)
    {
        return Ok(None);
    }
    observe_guard_installation(
        runtime_home,
        GuardInstallationObservation {
            guard_installation_id,
            connection_internal_id: envelope.connection_id.clone(),
            project_id: project.project_id.clone(),
            host_kind: envelope.host_kind.clone(),
            guard_mode: envelope.guard_mode.clone(),
            observed_policy_hash,
            observed_binary_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            observed_phase: phase.event_kind().to_owned(),
            observed_at: envelope.occurred_at.clone(),
        },
    )
    .map_err(Into::into)
}

fn current_policy_hash(project: &ProjectRecord) -> Result<Option<String>, GuardCommandError> {
    let policy_path = project.repo_root.join(VOLICORD_POLICY_FILE);
    let text = match fs::read_to_string(&policy_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(GuardCommandError::Runtime(format!(
                "failed to read detective host hook policy {}: {error}",
                policy_path.display()
            )));
        }
    };
    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
        GuardCommandError::Runtime(format!(
            "detective host hook policy is not valid JSON: {} ({error})",
            policy_path.display()
        ))
    })?;
    serde_json::to_string(&value)
        .map(|canonical| Some(sha256_text(&canonical)))
        .map_err(json_error)
}

fn guard_state_summary(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardStateSummary, GuardCommandError> {
    let store = CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id))?;
    let project_state = store.project_state()?;
    let now = event_time_or_now(&envelope.occurred_at);
    let now_timestamp = UtcTimestamp::from_datetime(now);
    let mut current_write_ticket_ids = Vec::new();
    let mut stale_write_ticket_ids = Vec::new();
    let mut active_write_tickets = Vec::new();
    let mut active_change_unit_id = None;
    let mut pending_user_judgment_count = 0;
    let mut pending_user_judgments = Vec::new();
    let mut active_blocker_count = 0;
    let prompt_capture_availability =
        prompt_capture_availability_for_event(runtime_home, project, envelope)?;
    let prompt_capture_status = prompt_capture_availability.status;
    let prompt_capture_enabled = prompt_capture_availability.can_use_chat_commands();
    if let Some(active_task_id) = project_state.active_task_id.as_deref() {
        let task_id = TaskId::new(active_task_id);
        active_change_unit_id = store
            .task_record(&task_id)?
            .and_then(|task| task.current_change_unit_id);
        for record in store.active_write_tickets(&task_id)? {
            let current_basis = record.basis_state_version == project_state.state_version;
            let not_expired = UtcTimestamp::parse(&record.expires_at)
                .map(|expires_at| now_timestamp < expires_at)
                .unwrap_or(false);
            if current_basis && not_expired {
                let write_ticket_id = record.write_ticket_id.clone();
                current_write_ticket_ids.push(write_ticket_id.clone());
                let attempt_scope: WriteTicketAttemptScope =
                    serde_json::from_str(&record.attempt_scope_json).map_err(json_error)?;
                if attempt_scope.product_file_write_intended {
                    active_write_tickets.push(ActiveWriteTicketSummary {
                        write_ticket_id,
                        change_unit_id: record.change_unit_id.clone(),
                        intended_paths: attempt_scope.intended_paths,
                        expires_at: record.expires_at,
                    });
                }
            } else {
                stale_write_ticket_ids.push(record.write_ticket_id);
            }
        }
        pending_user_judgment_count = store.pending_user_judgment_records(&task_id)?.len();
        if prompt_capture_enabled {
            pending_user_judgments = pending_chat_judgment_summaries(&store, &task_id, envelope)?;
        }
        active_blocker_count = store
            .active_blocker_refs(&task_id, project_state.state_version)?
            .len();
    }
    let unresolved_unrecorded_change_count = list_unresolved_unrecorded_changes(
        runtime_home,
        &project.project_id,
        Some(&envelope.connection_id),
    )?
    .len();
    let session_watch_scan_summary =
        guard_session_watch_scan_summary(runtime_home, project, envelope)?;
    let _ = input.raw_text.len();
    Ok(GuardStateSummary {
        project_id: project.project_id.clone(),
        project_name: project.project_name.clone(),
        repo_root: project.repo_root.display().to_string(),
        state_version: project_state.state_version,
        active_task_id: project_state.active_task_id,
        active_change_unit_id,
        prompt_capture_status,
        prompt_capture_enabled,
        current_write_ticket_ids,
        stale_write_ticket_ids,
        active_write_tickets,
        pending_user_judgment_count,
        pending_user_judgments,
        active_blocker_count,
        unresolved_unrecorded_change_count,
        session_watch_scan_summary,
    })
}

fn guard_session_watch_scan_summary(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> Result<Option<SessionWatchScanSummary>, GuardCommandError> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(None);
    };
    let Some(baseline) =
        latest_watch_baseline_for_session(runtime_home, &project.project_id, session_id)?
    else {
        return Ok(None);
    };
    if let Ok(metadata) = serde_json::from_str::<Value>(&baseline.metadata_json) {
        if let Some(raw_summary) = metadata.get("scan_summary") {
            if let Ok(summary) =
                serde_json::from_value::<SessionWatchScanSummary>(raw_summary.clone())
            {
                return Ok(Some(summary));
            }
        }
    }
    let summary = watch_scan_summary_from_entries_json(&baseline.snapshot_entries_json)?;
    Ok(Some(session_watch_scan_summary_from_store(&summary)))
}

fn pre_tool_decision(
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    event: &Value,
) -> (GuardDecision, Vec<GuardReason>) {
    let mut reasons = Vec::new();
    let product_file_write_attempt = tool_attempts_product_write(observation);
    if observation
        .paths
        .iter()
        .chain(observation.changed_paths.iter())
        .any(|path| !path.inside_repo)
    {
        reasons.push(GuardReason {
            code: "target_outside_project_allowlist",
            message: "One or more target paths are outside the selected Product Repository."
                .to_owned(),
            severity: "deny",
        });
    }
    if product_file_write_attempt {
        if summary.active_task_id.is_none() {
            reasons.push(GuardReason {
                code: "no_active_task",
                message: "Product-file writes require an active Volicord task.".to_owned(),
                severity: "deny",
            });
        } else {
            match write_ticket_coverage(summary, observation) {
                WriteTicketCoverage::NotWriteLike => {}
                WriteTicketCoverage::TicketBacked { .. } => {}
                WriteTicketCoverage::NoObservedPaths => reasons.push(GuardReason {
                    code: "write_ticket_scope_indeterminate",
                    message: "The host hook did not expose a deterministic Product Repository path for this write-like operation. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::NoActiveTickets { .. } => reasons.push(GuardReason {
                    code: "write_ticket_missing",
                    message: "No active write ticket covers this Product Repository write-like operation. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::OutOfScope { .. } => reasons.push(GuardReason {
                    code: "write_ticket_path_scope_violation",
                    message: "The observed Product Repository path is outside the active write ticket scope. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::Ambiguous { .. } => reasons.push(GuardReason {
                    code: "write_ticket_ambiguous",
                    message: "More than one active write ticket could cover this Product Repository path, so Volicord cannot deterministically link the operation. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
            }
        }
    }
    if observation.classification == ToolClassification::UnknownMutationRisk {
        let severity = event_string(
            event,
            &[
                &["policy", "unknown_mutation_decision"],
                &["guard_policy", "unknown_mutation_decision"],
            ],
        )
        .unwrap_or_else(|| "warn".to_owned());
        reasons.push(GuardReason {
            code: "unknown_mutation_risk",
            message: "Volicord could not confidently classify this tool invocation as read-only."
                .to_owned(),
            severity: if severity == "deny" { "deny" } else { "warn" },
        });
    }
    if observation.classification == ToolClassification::Mutating
        && event_bool(
            event,
            &[
                &["policy", "block_mutating_shell"],
                &["guard_policy", "block_mutating_shell"],
            ],
        )
        .unwrap_or(false)
    {
        reasons.push(GuardReason {
            code: "mutating_shell_blocked_by_policy",
            message: "Guard policy blocks clearly mutating shell commands.".to_owned(),
            severity: "deny",
        });
    }
    let decision = if reasons.iter().any(|reason| reason.severity == "deny") {
        GuardDecision::Deny
    } else if reasons.iter().any(|reason| reason.severity == "warn") {
        GuardDecision::Warn
    } else {
        GuardDecision::Allow
    };
    (decision, reasons)
}

fn write_ticket_coverage(
    summary: &GuardStateSummary,
    observation: &ToolObservation,
) -> WriteTicketCoverage {
    let observed_paths = normalized_observed_paths(
        observation
            .paths
            .iter()
            .chain(observation.changed_paths.iter()),
    );
    if observed_paths.is_empty() {
        return WriteTicketCoverage::NoObservedPaths;
    }
    if summary.active_write_tickets.is_empty() {
        return WriteTicketCoverage::NoActiveTickets { observed_paths };
    }
    let matching = summary
        .active_write_tickets
        .iter()
        .filter(|ticket| paths_are_authorized(&observed_paths, &ticket.intended_paths))
        .cloned()
        .collect::<Vec<_>>();
    if matching.len() == 1 {
        return WriteTicketCoverage::TicketBacked {
            ticket: matching.into_iter().next().expect("length checked"),
            observed_paths,
        };
    }
    if matching.len() > 1 {
        return WriteTicketCoverage::Ambiguous {
            observed_paths,
            matching_ticket_ids: matching
                .into_iter()
                .map(|ticket| ticket.write_ticket_id)
                .collect(),
        };
    }
    WriteTicketCoverage::OutOfScope {
        observed_paths,
        active_ticket_ids: summary
            .active_write_tickets
            .iter()
            .map(|ticket| ticket.write_ticket_id.clone())
            .collect(),
    }
}

fn tool_attempts_product_write(observation: &ToolObservation) -> bool {
    observation.explicit_write_attempt
        || observation.classification == ToolClassification::Mutating
        || tool_name_implies_write(observation.tool_name.as_deref())
}

fn confidently_expects_product_write(observation: &ToolObservation) -> bool {
    observation.classification == ToolClassification::Mutating
        || tool_name_implies_write(observation.tool_name.as_deref())
}

fn tool_name_implies_write(tool_name: Option<&str>) -> bool {
    tool_name
        .map(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "edit" | "write" | "write_file" | "apply_patch" | "patch" | "notebook_edit"
            )
        })
        .unwrap_or(false)
}

fn expected_write_candidate(
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    input: &GuardInput,
    decision: GuardDecision,
) -> Result<Option<ExpectedWriteCandidate>, GuardCommandError> {
    if decision == GuardDecision::Deny || !confidently_expects_product_write(observation) {
        return Ok(None);
    }
    let Some(task_id) = summary.active_task_id.clone() else {
        return Ok(None);
    };
    if observation
        .paths
        .iter()
        .chain(observation.changed_paths.iter())
        .any(|path| !path.inside_repo)
    {
        return Ok(None);
    }
    let expected_paths = normalized_observed_paths(
        observation
            .paths
            .iter()
            .chain(observation.changed_paths.iter()),
    );
    if expected_paths.is_empty() {
        return Ok(None);
    }
    let write_ticket = match write_ticket_coverage(summary, observation) {
        WriteTicketCoverage::TicketBacked { ticket, .. } => ticket,
        _ => return Ok(None),
    };
    let created_at = event_time_or_now(&envelope.occurred_at);
    let expires_at = created_at + ChronoDuration::minutes(EXPECTED_WRITE_TTL_MINUTES);
    let host_invocation_id = host_invocation_id(&input.raw_value);
    let expected_write_id = stable_id(
        "expected_write",
        &[
            &project.project_id,
            &envelope.connection_id,
            envelope.session_id.as_deref().unwrap_or(""),
            &envelope.event_id,
            host_invocation_id.as_deref().unwrap_or(""),
            &expected_paths.join("|"),
            &write_ticket.write_ticket_id,
        ],
    );
    let write_ticket_ids = vec![write_ticket.write_ticket_id.clone()];
    Ok(Some(ExpectedWriteCandidate {
        insert: ExpectedWriteInsert {
            expected_write_id,
            session_id: envelope.session_id.clone(),
            connection_internal_id: envelope.connection_id.clone(),
            guard_installation_id: envelope.guard_installation_id.clone(),
            pre_tool_guard_event_id: envelope.event_id.clone(),
            host_invocation_id,
            tool_name: observation.tool_name.clone(),
            command_kind: observation.classification.as_str().to_owned(),
            path_policy: "exact_paths".to_owned(),
            expected_paths_json: serde_json::to_string(&expected_paths).map_err(json_error)?,
            task_id,
            change_unit_id: summary.active_change_unit_id.clone(),
            write_ticket_ids_json: serde_json::to_string(&write_ticket_ids).map_err(json_error)?,
            basis_state_version: summary.state_version,
            created_at: format_timestamp(created_at),
            expires_at: format_timestamp(expires_at),
            metadata_json: json!({
                "source": "volicord_guard_pre_tool",
                "raw_event_sha256": input.raw_sha256,
                "ticket_backed": true,
                "write_ticket_ids": write_ticket_ids
            })
            .to_string(),
        },
        expected_paths,
        write_ticket,
    }))
}

fn persist_expected_write(
    runtime_home: &Path,
    project: &ProjectRecord,
    candidate: ExpectedWriteCandidate,
) -> Result<(), GuardCommandError> {
    insert_expected_write(runtime_home, &project.project_id, candidate.insert)?;
    Ok(())
}

fn expected_write_candidate_json(candidate: &ExpectedWriteCandidate) -> Value {
    json!({
        "expected_write_id": candidate.insert.expected_write_id,
        "host_invocation_id": candidate.insert.host_invocation_id,
        "tool_name": candidate.insert.tool_name,
        "command_kind": candidate.insert.command_kind,
        "path_policy": candidate.insert.path_policy,
        "expected_paths": candidate.expected_paths,
        "task_id": candidate.insert.task_id,
        "change_unit_id": candidate.insert.change_unit_id,
        "ticket_backed": true,
        "write_ticket_id": candidate.write_ticket.write_ticket_id,
        "write_ticket_ids": candidate.insert.write_ticket_ids_json
            .parse::<Value>()
            .unwrap_or_else(|_| json!([])),
        "basis_state_version": candidate.insert.basis_state_version,
        "expires_at": candidate.insert.expires_at
    })
}

fn normalized_observed_paths<'a>(paths: impl Iterator<Item = &'a PathAssessment>) -> Vec<String> {
    paths
        .filter(|path| path.inside_repo)
        .filter_map(|path| path.normalized.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn record_post_tool_correlation(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
) -> Result<PostToolCorrelation, GuardCommandError> {
    if observation.tool_name.as_deref() == Some("volicord.record_run") {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
        });
    }
    let changed = normalized_observed_paths(observation.changed_paths.iter());
    if changed.is_empty() {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
        });
    }
    let match_outcome =
        match_expected_write(runtime_home, project, envelope, observation, &changed)?;
    match match_outcome {
        ExpectedWriteMatchOutcome::Matched(record) => {
            mark_expected_write_matched(
                runtime_home,
                &project.project_id,
                &record.expected_write_id,
                ExpectedWriteMatch {
                    matched_post_tool_guard_event_id: envelope.event_id.clone(),
                    matched_paths_json: serde_json::to_string(&changed).map_err(json_error)?,
                    matched_at: envelope.occurred_at.clone(),
                },
            )?;
            Ok(PostToolCorrelation {
                matched_expected_writes: vec![matched_expected_write_json(&record, &changed)],
                ticket_backed_observations: Vec::new(),
                unrecorded_changes: Vec::new(),
            })
        }
        ExpectedWriteMatchOutcome::AlreadyMatched(record) => Ok(PostToolCorrelation {
            matched_expected_writes: vec![matched_expected_write_json(&record, &changed)],
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
        }),
        ExpectedWriteMatchOutcome::NoCandidates => {
            match active_write_ticket_match(summary, &changed) {
                ActiveWriteTicketMatchOutcome::Matched(ticket) => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: vec![ticket_backed_observation_json(
                        &ticket, &changed,
                    )],
                    unrecorded_changes: Vec::new(),
                }),
                ActiveWriteTicketMatchOutcome::NoActiveTickets => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: Vec::new(),
                    unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        changed,
                        correlation_status: "unmatched_expected_write",
                        candidate_expected_write_ids: Vec::new(),
                    })?,
                }),
                ActiveWriteTicketMatchOutcome::OutOfScope(ticket_ids) => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: Vec::new(),
                    unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        changed,
                        correlation_status: "out_of_scope_write_ticket",
                        candidate_expected_write_ids: ticket_ids,
                    })?,
                }),
                ActiveWriteTicketMatchOutcome::Ambiguous(ticket_ids) => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: Vec::new(),
                    unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        changed,
                        correlation_status: "ambiguous_write_ticket",
                        candidate_expected_write_ids: ticket_ids,
                    })?,
                }),
            }
        }
        ExpectedWriteMatchOutcome::OutOfScope(candidate_ids) => Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                changed,
                correlation_status: "out_of_scope_expected_write",
                candidate_expected_write_ids: candidate_ids,
            })?,
        }),
        ExpectedWriteMatchOutcome::Ambiguous(candidate_ids) => Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                changed,
                correlation_status: "ambiguous_expected_write",
                candidate_expected_write_ids: candidate_ids,
            })?,
        }),
    }
}

fn record_unrecorded_changes(
    context: UnrecordedChangeContext<'_>,
) -> Result<Vec<Value>, GuardCommandError> {
    if context.changed.is_empty() {
        return Ok(Vec::new());
    }
    let change_id = stable_id(
        "unrecorded_change",
        &[
            &context.envelope.event_id,
            &context.project.project_id,
            &context.changed.join("|"),
        ],
    );
    if unrecorded_change(
        context.runtime_home,
        &context.project.project_id,
        &change_id,
    )?
    .is_some()
    {
        return Ok(vec![json!({
            "unrecorded_change_id": change_id,
            "status": "already_recorded",
            "observed_paths": context.changed
        })]);
    }
    insert_unrecorded_change(
        context.runtime_home,
        &context.project.project_id,
        UnrecordedChangeInsert {
            unrecorded_change_id: change_id.clone(),
            session_id: context.envelope.session_id.clone(),
            connection_internal_id: context.envelope.connection_id.clone(),
            task_id: context.summary.active_task_id.clone(),
            summary: "Product file changes were observed after a host tool without a matching Volicord run record".to_owned(),
            observed_paths_json: serde_json::to_string(&context.changed).map_err(json_error)?,
            detection_json: json!({
                "source": "volicord_guard_post_tool",
                "tool_name": context.observation.tool_name,
                "exit_code": context.observation.exit_code,
                "success": context.observation.success,
                "status": context.observation.status,
                "correlation_status": context.correlation_status,
                "candidate_expected_write_ids": context.candidate_expected_write_ids,
                "active_write_ticket_ids": context.summary.active_write_tickets
                    .iter()
                    .map(|ticket| ticket.write_ticket_id.clone())
                    .collect::<Vec<_>>(),
                "ticket_scope_violation": matches!(
                    context.correlation_status,
                    "out_of_scope_expected_write" | "out_of_scope_write_ticket"
                ),
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true
            })
            .to_string(),
            detected_at: context.envelope.occurred_at.clone(),
            metadata_json: json!({
                "guard_event_id": context.envelope.event_id
            })
            .to_string(),
        },
    )?;
    Ok(vec![json!({
        "unrecorded_change_id": change_id,
        "status": "unresolved",
        "observed_paths": context.changed
    })])
}

fn match_expected_write(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    observation: &ToolObservation,
    changed: &[String],
) -> Result<ExpectedWriteMatchOutcome, GuardCommandError> {
    let already_matched = list_expected_writes_matched_by_post_event(
        runtime_home,
        &project.project_id,
        &envelope.connection_id,
        &envelope.event_id,
    )?;
    let changed_set = changed.iter().cloned().collect::<BTreeSet<_>>();
    let already_matched = already_matched
        .into_iter()
        .filter(|record| expected_write_session_matches(record, envelope))
        .filter(|record| matched_paths_cover_observed(record, &changed_set))
        .collect::<Vec<_>>();
    if already_matched.len() == 1 {
        return Ok(ExpectedWriteMatchOutcome::AlreadyMatched(
            already_matched.into_iter().next().expect("length checked"),
        ));
    }
    if already_matched.len() > 1 {
        return Ok(ExpectedWriteMatchOutcome::Ambiguous(
            already_matched
                .into_iter()
                .map(|record| record.expected_write_id)
                .collect(),
        ));
    }

    let host_invocation_id = observation.host_invocation_id.clone();
    let observed_at = event_time_or_now(&envelope.occurred_at);
    let pending =
        list_pending_expected_writes(runtime_home, &project.project_id, &envelope.connection_id)?;
    let time_scoped = pending
        .into_iter()
        .filter(|record| expected_write_time_contains(record, observed_at))
        .collect::<Vec<_>>();

    let candidates = if let Some(host_id) = host_invocation_id.as_deref() {
        let exact = time_scoped
            .iter()
            .filter(|record| record.host_invocation_id.as_deref() == Some(host_id))
            .filter(|record| expected_write_session_matches(record, envelope))
            .cloned()
            .collect::<Vec<_>>();
        if exact.is_empty() {
            fallback_expected_write_candidates(&time_scoped, envelope, true)
        } else {
            exact
        }
    } else {
        fallback_expected_write_candidates(&time_scoped, envelope, false)
    };
    if candidates.is_empty() {
        return Ok(ExpectedWriteMatchOutcome::NoCandidates);
    }

    let path_matched = candidates
        .iter()
        .filter(|record| expected_paths_cover_observed(record, &changed_set))
        .cloned()
        .collect::<Vec<_>>();
    if path_matched.len() == 1 {
        return Ok(ExpectedWriteMatchOutcome::Matched(
            path_matched.into_iter().next().expect("length checked"),
        ));
    }
    if path_matched.len() > 1 {
        return Ok(ExpectedWriteMatchOutcome::Ambiguous(
            path_matched
                .into_iter()
                .map(|record| record.expected_write_id)
                .collect(),
        ));
    }
    let candidate_ids = candidates
        .into_iter()
        .map(|record| record.expected_write_id)
        .collect::<Vec<_>>();
    if candidate_ids.len() == 1 {
        Ok(ExpectedWriteMatchOutcome::OutOfScope(candidate_ids))
    } else {
        Ok(ExpectedWriteMatchOutcome::Ambiguous(candidate_ids))
    }
}

fn active_write_ticket_match(
    summary: &GuardStateSummary,
    changed: &[String],
) -> ActiveWriteTicketMatchOutcome {
    if summary.active_write_tickets.is_empty() {
        return ActiveWriteTicketMatchOutcome::NoActiveTickets;
    }
    let matching = summary
        .active_write_tickets
        .iter()
        .filter(|ticket| paths_are_authorized(changed, &ticket.intended_paths))
        .cloned()
        .collect::<Vec<_>>();
    if matching.len() == 1 {
        return ActiveWriteTicketMatchOutcome::Matched(
            matching.into_iter().next().expect("length checked"),
        );
    }
    if matching.len() > 1 {
        return ActiveWriteTicketMatchOutcome::Ambiguous(
            matching
                .into_iter()
                .map(|ticket| ticket.write_ticket_id)
                .collect(),
        );
    }
    ActiveWriteTicketMatchOutcome::OutOfScope(
        summary
            .active_write_tickets
            .iter()
            .map(|ticket| ticket.write_ticket_id.clone())
            .collect(),
    )
}

fn fallback_expected_write_candidates(
    records: &[ExpectedWriteRecord],
    envelope: &GuardEnvelope,
    require_missing_host_invocation_id: bool,
) -> Vec<ExpectedWriteRecord> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Vec::new();
    };
    records
        .iter()
        .filter(|record| record.session_id.as_deref() == Some(session_id))
        .filter(|record| !require_missing_host_invocation_id || record.host_invocation_id.is_none())
        .cloned()
        .collect()
}

fn expected_write_session_matches(record: &ExpectedWriteRecord, envelope: &GuardEnvelope) -> bool {
    envelope
        .session_id
        .as_deref()
        .is_none_or(|session_id| record.session_id.as_deref() == Some(session_id))
}

fn expected_write_time_contains(record: &ExpectedWriteRecord, observed_at: DateTime<Utc>) -> bool {
    let Ok(created_at) = DateTime::parse_from_rfc3339(&record.created_at) else {
        return false;
    };
    let Ok(expires_at) = DateTime::parse_from_rfc3339(&record.expires_at) else {
        return false;
    };
    created_at.with_timezone(&Utc) <= observed_at && observed_at <= expires_at.with_timezone(&Utc)
}

fn expected_paths_cover_observed(
    record: &ExpectedWriteRecord,
    changed_set: &BTreeSet<String>,
) -> bool {
    if record.path_policy != "exact_paths" {
        return false;
    }
    let expected = json_string_set(&record.expected_paths_json);
    !changed_set.is_empty() && changed_set.is_subset(&expected)
}

fn matched_paths_cover_observed(
    record: &ExpectedWriteRecord,
    changed_set: &BTreeSet<String>,
) -> bool {
    let expected = record
        .matched_paths_json
        .as_deref()
        .map(json_string_set)
        .unwrap_or_default();
    !changed_set.is_empty() && changed_set.is_subset(&expected)
}

fn json_string_set(text: &str) -> BTreeSet<String> {
    serde_json::from_str::<Vec<String>>(text)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn paths_are_authorized(observed_paths: &[String], authorized_paths: &[String]) -> bool {
    !observed_paths.is_empty()
        && !authorized_paths.is_empty()
        && observed_paths.iter().all(|path| {
            authorized_paths
                .iter()
                .any(|authorized| path_is_within(path, authorized))
        })
}

fn path_is_within(path: &str, scope: &str) -> bool {
    path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn matched_expected_write_json(record: &ExpectedWriteRecord, changed: &[String]) -> Value {
    json!({
        "expected_write_id": record.expected_write_id,
        "status": "matched",
        "pre_tool_guard_event_id": record.pre_tool_guard_event_id,
        "host_invocation_id": record.host_invocation_id,
        "path_policy": record.path_policy,
        "observed_paths": changed,
        "task_id": record.task_id,
        "change_unit_id": record.change_unit_id,
        "ticket_backed": true,
        "write_ticket_ids": serde_json::from_str::<Value>(&record.write_ticket_ids_json)
            .unwrap_or_else(|_| json!([]))
    })
}

fn ticket_backed_observation_json(ticket: &ActiveWriteTicketSummary, changed: &[String]) -> Value {
    json!({
        "status": "ticket_backed",
        "ticket_backed": true,
        "write_ticket_id": ticket.write_ticket_id.clone(),
        "write_ticket_ids": [ticket.write_ticket_id.clone()],
        "observed_paths": changed,
        "change_unit_id": ticket.change_unit_id.clone(),
        "intended_paths": ticket.intended_paths.clone(),
        "expires_at": ticket.expires_at.clone()
    })
}

fn stop_decision(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
) -> Result<(GuardDecision, Vec<GuardReason>, Value), GuardCommandError> {
    let Some(task_id) = summary.active_task_id.as_deref() else {
        return Ok((
            GuardDecision::Allow,
            Vec::new(),
            json!({"active_task": null, "close_blockers": []}),
        ));
    };
    let response = CoreService::new(runtime_home).status(
        StatusRequest {
            envelope: ToolEnvelope {
                project_id: ProjectId::new(&project.project_id),
                task_id: Some(TaskId::new(task_id)).into(),
                request_id: RequestId::new(stable_id(
                    "req_guard_stop_status",
                    &[&envelope.event_id, task_id],
                )),
                idempotency_key: None.into(),
                expected_state_version: None.into(),
                dry_run: false,
                locale: None.into(),
            },
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        InvocationContext::new(
            ProjectId::new(&project.project_id),
            ActorSource::agent_connection(envelope.connection_id.clone()),
            OperationCategory::Read,
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
        ),
    )?;
    let close_blockers = response
        .response_value
        .get("close_blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut reasons = Vec::new();
    if !close_blockers.is_empty() {
        reasons.push(GuardReason {
            code: "close_readiness_blocked",
            message: "Close readiness has blockers for the active task.".to_owned(),
            severity: "deny",
        });
    }
    if summary.pending_user_judgment_count > 0 {
        reasons.push(GuardReason {
            code: "pending_user_judgments",
            message: "User-owned judgments are still pending for the active task.".to_owned(),
            severity: "deny",
        });
    }
    if summary.unresolved_unrecorded_change_count > 0 {
        reasons.push(GuardReason {
            code: "unresolved_unrecorded_changes",
            message: "Observed Product Repository changes still need reconciliation.".to_owned(),
            severity: "deny",
        });
    }
    let decision = if reasons.iter().any(|reason| reason.severity == "deny") {
        GuardDecision::Deny
    } else {
        GuardDecision::Allow
    };
    Ok((
        decision,
        reasons,
        json!({
            "active_task": task_id,
            "status_summary": response.response_value.get("status_summary").cloned().unwrap_or(Value::Null),
            "close_state": response.response_value.get("close_state").cloned().unwrap_or(Value::Null),
            "close_blockers": close_blockers
        }),
    ))
}

fn persist_guard_event(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    decision: GuardDecision,
    subject: Value,
    result: Value,
) -> Result<(), GuardCommandError> {
    if guard_event(runtime_home, &project.project_id, &envelope.event_id)?.is_some() {
        return Ok(());
    }
    insert_guard_event(
        runtime_home,
        &project.project_id,
        GuardEventInsert {
            guard_event_id: envelope.event_id.clone(),
            session_id: envelope.session_id.clone(),
            connection_internal_id: envelope.connection_id.clone(),
            guard_installation_id: envelope.guard_installation_id.clone(),
            event_kind: phase.event_kind().to_owned(),
            decision: decision.as_str().to_owned(),
            subject_json: object_text(subject)?,
            result_json: object_text(result)?,
            occurred_at: envelope.occurred_at.clone(),
            metadata_json: json!({
                "source": "volicord_guard_cli",
                "cooperative_detective": true
            })
            .to_string(),
        },
    )?;
    Ok(())
}

fn guard_subject(
    phase: GuardPhase,
    input: &GuardInput,
    envelope: &GuardEnvelope,
    project: &ProjectRecord,
) -> Value {
    json!({
        "lifecycle_phase": phase.event_kind(),
        "host_kind": envelope.host_kind,
        "connection_id": envelope.connection_id,
        "project_id": project.project_id,
        "repo_root": project.repo_root.display().to_string(),
        "raw_event_sha256": input.raw_sha256,
        "raw_event": input.redacted_value
    })
}

fn render_guard_output(
    phase: GuardPhase,
    decision: GuardDecision,
    envelope: &GuardEnvelope,
    result: Value,
    output: OutputFormat,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    match output {
        OutputFormat::VolicordJson => Ok(RenderedGuardOutput {
            stdout: format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({
                    "phase": phase.event_kind(),
                    "decision": decision.as_str(),
                    "allowed": decision != GuardDecision::Deny,
                    "disclosure": cooperative_host_decision_disclosure_json(),
                    "guard_event_id": envelope.event_id,
                    "session_id": envelope.session_id,
                    "result": result
                }))
                .map_err(json_error)?
            ),
            stderr: String::new(),
            exit_code: if decision == GuardDecision::Deny {
                1
            } else {
                0
            },
        }),
        OutputFormat::Text => {
            let allowed = if decision == GuardDecision::Deny {
                "blocked"
            } else {
                "allowed"
            };
            let watcher_text = guard_watcher_scan_text(&result);
            Ok(RenderedGuardOutput {
                stdout: format!(
                    "Volicord host-hook {}: {} ({})\n{}{}\n",
                    phase.command_name(),
                    decision.as_str(),
                    allowed,
                    watcher_text,
                    COOPERATIVE_DECISION_DISCLOSURE_TEXT
                ),
                stderr: String::new(),
                exit_code: if decision == GuardDecision::Deny {
                    1
                } else {
                    0
                },
            })
        }
        OutputFormat::HostNative(host) => render_host_native_output(host, phase, decision, result),
    }
}

fn guard_watcher_scan_text(result: &Value) -> String {
    let Some(summary) = result
        .get("context")
        .and_then(|context| context.get("session_watch_scan_summary"))
        .filter(|summary| summary.is_object())
    else {
        return String::new();
    };
    let degraded_reasons = summary
        .get("degraded_reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let degraded_reasons = if degraded_reasons.is_empty() {
        "none".to_owned()
    } else {
        degraded_reasons.join(",")
    };
    format!(
        "watcher_scan: files_scanned={}; files_skipped={}; unreadable_paths={}; degraded_reasons={}\nwatcher_note: not full filesystem monitoring\n",
        summary
            .get("files_scanned")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("files_skipped")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("unreadable_paths_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        degraded_reasons,
    )
}

fn render_host_native_output(
    host: HostOutputMode,
    phase: GuardPhase,
    decision: GuardDecision,
    result: Value,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    let event_name = host_hook_event_name(phase);
    let value = match phase {
        GuardPhase::SessionStart => context_output(event_name, guard_context_message(&result)),
        GuardPhase::PreTool => match decision {
            GuardDecision::Deny => Some(json!({
                "hookSpecificOutput": {
                    "hookEventName": event_name,
                    "permissionDecision": "deny",
                    "permissionDecisionReason": blocking_reason(phase, &result)
                }
            })),
            GuardDecision::Warn | GuardDecision::InjectContext => {
                context_output(event_name, guard_context_message(&result))
            }
            GuardDecision::Allow => None,
        },
        GuardPhase::PostTool => match decision {
            GuardDecision::Deny => Some(json!({
                "decision": "block",
                "reason": blocking_reason(phase, &result)
            })),
            GuardDecision::Warn | GuardDecision::InjectContext => {
                context_output(event_name, post_tool_context_message(&result))
            }
            GuardDecision::Allow => None,
        },
        GuardPhase::PromptCapture => match decision {
            GuardDecision::Deny => Some(json!({
                "decision": "block",
                "reason": blocking_reason(phase, &result)
            })),
            GuardDecision::InjectContext | GuardDecision::Warn => {
                context_output(event_name, prompt_context_message(&result))
            }
            GuardDecision::Allow => prompt_context_message(&result)
                .filter(|message| !message.trim().is_empty())
                .and_then(|message| context_output(event_name, Some(message))),
        },
        GuardPhase::Stop => match decision {
            GuardDecision::Deny => Some(json!({
                "decision": "block",
                "reason": blocking_reason(phase, &result)
            })),
            GuardDecision::Allow | GuardDecision::Warn | GuardDecision::InjectContext => {
                Some(json!({ "continue": true }))
            }
        },
    };
    let stdout = match value {
        Some(value) => format!("{}\n", serde_json::to_string(&value).map_err(json_error)?),
        None => String::new(),
    };
    Ok(RenderedGuardOutput {
        stdout,
        stderr: String::new(),
        exit_code: host_success_exit_code(host),
    })
}

fn host_success_exit_code(_host: HostOutputMode) -> i32 {
    0
}

fn host_hook_event_name(phase: GuardPhase) -> &'static str {
    match phase {
        GuardPhase::SessionStart => "SessionStart",
        GuardPhase::PreTool => "PreToolUse",
        GuardPhase::PostTool => "PostToolUse",
        GuardPhase::PromptCapture => "UserPromptSubmit",
        GuardPhase::Stop => "Stop",
    }
}

fn context_output(event_name: &str, message: Option<String>) -> Option<Value> {
    let message = message.filter(|message| !message.trim().is_empty())?;
    Some(json!({
        "hookSpecificOutput": {
            "hookEventName": event_name,
            "additionalContext": host_native_message_with_disclosure(&message)
        }
    }))
}

fn blocking_reason(phase: GuardPhase, result: &Value) -> String {
    let reason = first_reason_message(result).unwrap_or_else(|| match phase {
        GuardPhase::SessionStart => "Volicord session context could not be prepared.".to_owned(),
        GuardPhase::PreTool => "Volicord requested a host denial for this tool call.".to_owned(),
        GuardPhase::PostTool => {
            "Volicord requested a host denial for normal handling of this tool result.".to_owned()
        }
        GuardPhase::PromptCapture => {
            "Volicord requested a host denial for this user prompt.".to_owned()
        }
        GuardPhase::Stop => "Volicord needs more work before this session stops.".to_owned(),
    });
    host_native_message_with_disclosure(&reason)
}

fn host_native_message_with_disclosure(message: &str) -> String {
    format!("{message} {COOPERATIVE_DECISION_DISCLOSURE_TEXT}.")
}

fn first_reason_message(result: &Value) -> Option<String> {
    result
        .get("reasons")
        .and_then(Value::as_array)
        .and_then(|reasons| reasons.first())
        .and_then(|reason| {
            let message = reason.get("message").and_then(Value::as_str)?;
            let code = reason.get("code").and_then(Value::as_str);
            Some(match code {
                Some(code) if !code.trim().is_empty() => format!("{message} ({code})"),
                _ => message.to_owned(),
            })
        })
        .or_else(|| {
            result
                .get("model_context")
                .and_then(Value::as_str)
                .filter(|message| !message.trim().is_empty())
                .map(str::to_owned)
        })
}

fn guard_context_message(result: &Value) -> Option<String> {
    let context = result.get("context")?;
    let project_name = context.get("project_name").and_then(Value::as_str)?;
    let state_version = context.get("state_version").and_then(Value::as_u64)?;
    let active_task = context
        .get("active_task_id")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let write_tickets = context
        .get("current_write_ticket_ids")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let pending_judgments = context
        .get("pending_user_judgment_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let unresolved_changes = context
        .get("unresolved_unrecorded_change_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(format!(
        "Volicord context: project `{project_name}`, state_version {state_version}, active_task {active_task}, current_write_tickets {write_tickets}, pending_user_judgments {pending_judgments}, unresolved_unrecorded_changes {unresolved_changes}."
    ))
}

fn post_tool_context_message(result: &Value) -> Option<String> {
    let changes = result
        .get("unrecorded_changes")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if changes == 0 {
        return guard_context_message(result);
    }
    Some(format!(
        "Volicord observed {changes} unresolved Product Repository change finding(s) after this tool call. Reconcile them before close."
    ))
}

fn prompt_context_message(result: &Value) -> Option<String> {
    result
        .get("model_context")
        .and_then(Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| guard_context_message(result))
}

fn context_json(summary: &GuardStateSummary) -> Value {
    json!({
        "project_id": summary.project_id,
        "project_name": summary.project_name,
        "repo_root": summary.repo_root,
        "state_version": summary.state_version,
        "active_task_id": summary.active_task_id,
        "active_change_unit_id": summary.active_change_unit_id,
        "prompt_capture_status": summary.prompt_capture_status.as_str(),
        "prompt_capture_enabled": summary.prompt_capture_enabled,
        "current_write_ticket_ids": summary.current_write_ticket_ids,
        "stale_write_ticket_ids": summary.stale_write_ticket_ids,
        "active_write_tickets": summary.active_write_tickets
            .iter()
            .map(active_write_ticket_json)
            .collect::<Vec<_>>(),
        "pending_user_judgment_count": summary.pending_user_judgment_count,
        "pending_user_judgments": summary.pending_user_judgments
            .iter()
            .map(pending_judgment_summary_json)
            .collect::<Vec<_>>(),
        "active_blocker_count": summary.active_blocker_count,
        "unresolved_unrecorded_change_count": summary.unresolved_unrecorded_change_count,
        "session_watch_scan_summary": summary.session_watch_scan_summary
    })
}

fn active_write_ticket_json(ticket: &ActiveWriteTicketSummary) -> Value {
    json!({
        "write_ticket_id": ticket.write_ticket_id,
        "change_unit_id": ticket.change_unit_id,
        "intended_paths": ticket.intended_paths,
        "expires_at": ticket.expires_at
    })
}

fn write_ticket_backing_json(coverage: WriteTicketCoverage) -> Value {
    match coverage {
        WriteTicketCoverage::NotWriteLike => json!({
            "status": "not_write_like",
            "ticket_backed": false,
            "observed_paths": []
        }),
        WriteTicketCoverage::TicketBacked {
            ticket,
            observed_paths,
        } => json!({
            "status": "ticket_backed",
            "ticket_backed": true,
            "write_ticket_id": ticket.write_ticket_id.clone(),
            "write_ticket_ids": [ticket.write_ticket_id.clone()],
            "observed_paths": observed_paths,
            "scope": {
                "change_unit_id": ticket.change_unit_id,
                "intended_paths": ticket.intended_paths,
                "expires_at": ticket.expires_at
            },
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::NoObservedPaths => json!({
            "status": "scope_indeterminate",
            "ticket_backed": false,
            "observed_paths": [],
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::NoActiveTickets { observed_paths } => json!({
            "status": "missing_ticket",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::OutOfScope {
            observed_paths,
            active_ticket_ids,
        } => json!({
            "status": "out_of_scope",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "active_write_ticket_ids": active_ticket_ids,
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::Ambiguous {
            observed_paths,
            matching_ticket_ids,
        } => json!({
            "status": "ambiguous",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "matching_write_ticket_ids": matching_ticket_ids,
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
    }
}

fn tool_observation_json(observation: &ToolObservation) -> Value {
    json!({
        "tool_name": observation.tool_name,
        "host_invocation_id": observation.host_invocation_id,
        "command": observation.command,
        "classification": observation.classification.as_str(),
        "paths": path_assessments_json(&observation.paths),
        "changed_paths": path_assessments_json(&observation.changed_paths),
        "explicit_write_attempt": observation.explicit_write_attempt,
        "exit_code": observation.exit_code,
        "success": observation.success,
        "status": observation.status
    })
}

fn path_assessments_json(paths: &[PathAssessment]) -> Vec<Value> {
    paths
        .iter()
        .map(|path| {
            json!({
                "raw": path.raw,
                "normalized": path.normalized,
                "inside_repo": path.inside_repo
            })
        })
        .collect()
}

fn reasons_json(reasons: &[GuardReason]) -> Vec<Value> {
    reasons
        .iter()
        .map(|reason| {
            json!({
                "code": reason.code,
                "message": reason.message,
                "severity": reason.severity
            })
        })
        .collect()
}

fn object_text(value: Value) -> Result<String, GuardCommandError> {
    match value {
        Value::Object(_) => serde_json::to_string(&value).map_err(json_error),
        other => serde_json::to_string(&json!({ "value": other })).map_err(json_error),
    }
}

fn redact_event_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    if prompt_like_key(key) {
                        (key.clone(), redacted_prompt_value(value))
                    } else {
                        (key.clone(), redact_event_value(value))
                    }
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_event_value).collect()),
        other => other.clone(),
    }
}

fn prompt_like_key(key: &str) -> bool {
    matches!(
        key,
        "prompt" | "user_prompt" | "message" | "messages" | "content" | "transcript"
    )
}

fn redacted_prompt_value(value: &Value) -> Value {
    match value {
        Value::String(text) => json!({
            "omitted": true,
            "sha256": sha256_text(text),
            "size_bytes": text.len()
        }),
        Value::Array(values) => json!({
            "omitted": true,
            "sha256": sha256_text(&value.to_string()),
            "item_count": values.len()
        }),
        _ => json!({
            "omitted": true,
            "sha256": sha256_text(&value.to_string())
        }),
    }
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hex_bytes(&hasher.finalize());
    format!("{prefix}_{}", &digest[..16])
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn json_error(error: serde_json::Error) -> GuardCommandError {
    GuardCommandError::Runtime(format!("failed to serialize host-hook output: {error}"))
}
