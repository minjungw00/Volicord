use std::{
    ffi::OsString,
    fmt,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use rustix::process::{kill_process_group, Pid, Signal};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::process::CommandExt;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::{Child, Command, Stdio};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::time::Instant;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_platform_fs::capture_git_workspace_snapshot;
use volicord_store::{
    agent_connections::agent_connection_project_access,
    bootstrap::{ProjectRecord, ACTIVE_PROJECT_STATUS},
    core_pipeline::{CoreProjectStore, EvidenceCaptureIntentRecord, EvidenceCaptureReceiptInsert},
    guards::{agent_session, guard_event, guard_installation, GuardEventRecord},
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    session_watch::{
        validate_persisted_watch_observation, watch_baseline, watch_observation,
        WatchBaselineRecord, WatchObservationRecord, WatchScanSummary,
    },
    StoreError,
};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_string, ActorSource, AgentConnectionId,
    AgentSessionId, ConnectionObservationSourceKind, DurableIdGenerator, DurableIdKind,
    EvidenceCaptureIntentId, EvidenceCaptureSpec, EvidenceProducerKind, EvidenceTarget,
    GuardEventId, GuardInstallationId, JsonObject, PersistedEvidenceCaptureReceiptBody,
    PersistedEvidenceCaptureReceiptSource, ProjectId, RandomDurableIdGenerator, RedactionState,
    TaskId, UtcTimestamp, EVIDENCE_CAPTURE_COMMAND_LIMITATION as COMMAND_LIMITATION,
    EVIDENCE_CAPTURE_GUARD_LIMITATION as HOOK_LIMITATION,
    EVIDENCE_CAPTURE_WATCHER_LIMITATION as WATCH_LIMITATION,
};

use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};

const RECEIPT_SCHEMA_VERSION: &str = "volicord.evidence_capture_receipt.v1";
const MAX_CAPTURE_COMMAND_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const PROJECT_CLOCK_RESAMPLE_DELAY: Duration = Duration::from_millis(1);
#[cfg(any(target_os = "linux", target_os = "macos"))]
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceCommandError {
    Usage(String),
    Runtime(String),
}

impl EvidenceCommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for EvidenceCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for EvidenceCommandError {}

impl From<StoreError> for EvidenceCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for EvidenceCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for EvidenceCommandError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvidenceCommand {
    CaptureCommand {
        intent_id: String,
        repo: Option<PathBuf>,
        json: bool,
        argv: Vec<String>,
    },
    CaptureTool {
        intent_id: String,
        pre_event_id: String,
        post_event_id: String,
        repo: Option<PathBuf>,
        json: bool,
    },
    CaptureConnection {
        intent_id: String,
        source: ConnectionSource,
        repo: Option<PathBuf>,
        json: bool,
    },
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectionSource {
    GuardEvent(String),
    WatchObservation(String),
}

#[derive(Debug)]
struct EvidenceContext {
    runtime_home: PathBuf,
    project: ProjectRecord,
    store: CoreProjectStore,
}

#[derive(Debug)]
struct ValidatedIntent {
    record: EvidenceCaptureIntentRecord,
    capture: EvidenceCaptureSpec,
    expected_outcome: JsonObject,
    session_id: Option<String>,
}

#[derive(Debug)]
struct FulfillmentFacts {
    observed_outcome: JsonObject,
    source: PersistedEvidenceCaptureReceiptSource,
    observed_at: String,
    limitations: Vec<String>,
}

#[derive(Debug)]
struct CommandStreamDigest {
    sha256: String,
    size_bytes: u64,
}

pub fn run_evidence_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, EvidenceCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    match parse_evidence_command(args)? {
        EvidenceCommand::Help => Ok(evidence_usage()),
        EvidenceCommand::CaptureCommand {
            intent_id,
            repo,
            json,
            argv,
        } => {
            let mut context = resolve_context(env_var, current_dir, repo.as_deref())?;
            let intent = load_and_validate_intent(&context, &intent_id)?;
            let facts = fulfill_command(&context, &intent, &argv)?;
            persist_fulfillment(&mut context, &intent, facts, json)
        }
        EvidenceCommand::CaptureTool {
            intent_id,
            pre_event_id,
            post_event_id,
            repo,
            json,
        } => {
            let mut context = resolve_context(env_var, current_dir, repo.as_deref())?;
            let intent = load_and_validate_intent(&context, &intent_id)?;
            let facts = fulfill_tool(&context, &intent, &pre_event_id, &post_event_id)?;
            persist_fulfillment(&mut context, &intent, facts, json)
        }
        EvidenceCommand::CaptureConnection {
            intent_id,
            source,
            repo,
            json,
        } => {
            let mut context = resolve_context(env_var, current_dir, repo.as_deref())?;
            let intent = load_and_validate_intent(&context, &intent_id)?;
            let facts = fulfill_connection(&context, &intent, &source)?;
            persist_fulfillment(&mut context, &intent, facts, json)
        }
    }
}

pub fn evidence_usage() -> String {
    concat!(
        "volicord evidence capture-command --intent ID [--repo PATH] [--json] -- PROGRAM [ARG...]\n",
        "volicord evidence capture-tool --intent ID --pre-event ID --post-event ID [--repo PATH] [--json]\n",
        "volicord evidence capture-connection --intent ID (--guard-event ID | --watch-observation ID) [--repo PATH] [--json]\n",
    )
    .to_owned()
}

fn parse_evidence_command(args: &[String]) -> Result<EvidenceCommand, EvidenceCommandError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(EvidenceCommand::Help);
    };
    if matches!(subcommand, "-h" | "--help" | "help") {
        if args.len() == 1 {
            return Ok(EvidenceCommand::Help);
        }
        return Err(EvidenceCommandError::usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            evidence_usage()
        )));
    }
    match subcommand {
        "capture-command" => parse_capture_command(&args[1..]),
        "capture-tool" => parse_capture_tool(&args[1..]),
        "capture-connection" => parse_capture_connection(&args[1..]),
        other => Err(EvidenceCommandError::usage(format!(
            "unknown evidence command: {other}\n\n{}",
            evidence_usage()
        ))),
    }
}

fn parse_capture_command(args: &[String]) -> Result<EvidenceCommand, EvidenceCommandError> {
    let delimiter = args.iter().position(|arg| arg == "--").ok_or_else(|| {
        EvidenceCommandError::usage("capture-command requires `-- PROGRAM [ARG...]`")
    })?;
    let options = parse_options(&args[..delimiter], OptionPolicy::Command)?;
    let argv = args[delimiter + 1..].to_vec();
    if argv.is_empty() || argv[0].is_empty() {
        return Err(EvidenceCommandError::usage(
            "capture-command requires a non-empty PROGRAM after `--`",
        ));
    }
    Ok(EvidenceCommand::CaptureCommand {
        intent_id: require_option(options.intent_id, "--intent")?,
        repo: options.repo,
        json: options.json,
        argv,
    })
}

fn parse_capture_tool(args: &[String]) -> Result<EvidenceCommand, EvidenceCommandError> {
    let options = parse_options(args, OptionPolicy::Tool)?;
    Ok(EvidenceCommand::CaptureTool {
        intent_id: require_option(options.intent_id, "--intent")?,
        pre_event_id: require_option(options.pre_event_id, "--pre-event")?,
        post_event_id: require_option(options.post_event_id, "--post-event")?,
        repo: options.repo,
        json: options.json,
    })
}

fn parse_capture_connection(args: &[String]) -> Result<EvidenceCommand, EvidenceCommandError> {
    let options = parse_options(args, OptionPolicy::Connection)?;
    let source =
        match (options.guard_event_id, options.watch_observation_id) {
            (Some(id), None) => ConnectionSource::GuardEvent(id),
            (None, Some(id)) => ConnectionSource::WatchObservation(id),
            _ => return Err(EvidenceCommandError::usage(
                "capture-connection requires exactly one of --guard-event or --watch-observation",
            )),
        };
    Ok(EvidenceCommand::CaptureConnection {
        intent_id: require_option(options.intent_id, "--intent")?,
        source,
        repo: options.repo,
        json: options.json,
    })
}

#[derive(Debug, Clone, Copy)]
enum OptionPolicy {
    Command,
    Tool,
    Connection,
}

#[derive(Debug, Default)]
struct ParsedOptions {
    intent_id: Option<String>,
    pre_event_id: Option<String>,
    post_event_id: Option<String>,
    guard_event_id: Option<String>,
    watch_observation_id: Option<String>,
    repo: Option<PathBuf>,
    json: bool,
}

fn parse_options(
    args: &[String],
    policy: OptionPolicy,
) -> Result<ParsedOptions, EvidenceCommandError> {
    let mut parsed = ParsedOptions::default();
    let mut index = 0;
    while index < args.len() {
        let option = args[index].as_str();
        match option {
            "--intent" => set_string_option(args, &mut index, &mut parsed.intent_id, option)?,
            "--repo" => {
                let value = option_value(args, &mut index, option)?;
                if parsed.repo.replace(PathBuf::from(value)).is_some() {
                    return Err(EvidenceCommandError::usage("duplicate option: --repo"));
                }
            }
            "--json" => {
                if parsed.json {
                    return Err(EvidenceCommandError::usage("duplicate option: --json"));
                }
                parsed.json = true;
            }
            "--pre-event" if matches!(policy, OptionPolicy::Tool) => {
                set_string_option(args, &mut index, &mut parsed.pre_event_id, option)?
            }
            "--post-event" if matches!(policy, OptionPolicy::Tool) => {
                set_string_option(args, &mut index, &mut parsed.post_event_id, option)?
            }
            "--guard-event" if matches!(policy, OptionPolicy::Connection) => {
                set_string_option(args, &mut index, &mut parsed.guard_event_id, option)?
            }
            "--watch-observation" if matches!(policy, OptionPolicy::Connection) => {
                set_string_option(args, &mut index, &mut parsed.watch_observation_id, option)?
            }
            "-h" | "--help" | "help" => return Err(EvidenceCommandError::usage(evidence_usage())),
            unknown if unknown.starts_with('-') => {
                return Err(EvidenceCommandError::usage(format!(
                    "unknown option: {unknown}"
                )))
            }
            argument => {
                return Err(EvidenceCommandError::usage(format!(
                    "unexpected argument: {argument}"
                )))
            }
        }
        index += 1;
    }
    Ok(parsed)
}

fn set_string_option(
    args: &[String],
    index: &mut usize,
    slot: &mut Option<String>,
    option: &str,
) -> Result<(), EvidenceCommandError> {
    let value = option_value(args, index, option)?;
    if slot.replace(value.to_owned()).is_some() {
        return Err(EvidenceCommandError::usage(format!(
            "duplicate option: {option}"
        )));
    }
    Ok(())
}

fn option_value<'a>(
    args: &'a [String],
    index: &mut usize,
    option: &str,
) -> Result<&'a str, EvidenceCommandError> {
    *index += 1;
    args.get(*index)
        .map(String::as_str)
        .filter(|value| !value.is_empty() && !value.starts_with('-'))
        .ok_or_else(|| EvidenceCommandError::usage(format!("{option} requires a value")))
}

fn require_option(value: Option<String>, option: &str) -> Result<String, EvidenceCommandError> {
    value.ok_or_else(|| EvidenceCommandError::usage(format!("{option} is required")))
}

fn resolve_context<F>(
    env_var: F,
    current_dir: &Path,
    selected_repo: Option<&Path>,
) -> Result<EvidenceContext, EvidenceCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, selected_repo)?;
    let project = registered_project_for_repo(&runtime_home, &repo_root)?;
    if project.status != ACTIVE_PROJECT_STATUS {
        return Err(EvidenceCommandError::runtime(
            "evidence capture requires an active registered project",
        ));
    }
    let store = CoreProjectStore::open(&runtime_home, &ProjectId::new(&project.project_id))?;
    Ok(EvidenceContext {
        runtime_home,
        project,
        store,
    })
}

fn load_and_validate_intent(
    context: &EvidenceContext,
    intent_id: &str,
) -> Result<ValidatedIntent, EvidenceCommandError> {
    let record = context
        .store
        .evidence_capture_intent_record(intent_id)?
        .ok_or_else(|| {
            EvidenceCommandError::runtime(format!(
                "evidence capture intent was not found: {intent_id}"
            ))
        })?;
    if context
        .store
        .evidence_capture_receipt_for_intent(intent_id)?
        .is_some()
    {
        return Err(EvidenceCommandError::runtime(format!(
            "evidence capture intent was already fulfilled: {intent_id}"
        )));
    }
    if context
        .store
        .evidence_producer_for_intent(intent_id)?
        .is_some()
    {
        return Err(EvidenceCommandError::runtime(format!(
            "evidence capture intent was already consumed: {intent_id}"
        )));
    }
    if record.project_id != context.project.project_id {
        return Err(EvidenceCommandError::runtime(
            "evidence capture intent belongs to another project",
        ));
    }
    validate_connection_access(context, &record.requesting_connection_internal_id)?;
    validate_current_basis(context, &record)?;
    validate_not_expired(&context.store, &record)?;

    let capture: EvidenceCaptureSpec = strict_json(
        "evidence capture intent capture_spec_json",
        &record.capture_spec_json,
    )?;
    let target: EvidenceTarget =
        strict_json("evidence capture intent target_json", &record.target_json)?;
    validate_target(&context.store, &record, &target)?;
    validate_workspace(context, &record)?;
    if capture_kind(&capture) != record.capture_kind {
        return Err(EvidenceCommandError::runtime(
            "evidence capture intent kind is inconsistent",
        ));
    }
    let expected_outcome: JsonObject = strict_json(
        "evidence capture intent expected_outcome_json",
        &record.expected_outcome_json,
    )?;
    let session_context = json_object(
        "evidence capture intent session_context_json",
        &record.session_context_json,
    )?;
    let session_id = match session_context.get("session_id") {
        Some(Value::String(value)) if !value.is_empty() => Some(value.clone()),
        Some(Value::Null) | None => None,
        _ => {
            return Err(EvidenceCommandError::runtime(
                "evidence capture intent session context is corrupt",
            ))
        }
    };
    Ok(ValidatedIntent {
        record,
        capture,
        expected_outcome,
        session_id,
    })
}

fn validate_connection_access(
    context: &EvidenceContext,
    connection_id: &str,
) -> Result<(), EvidenceCommandError> {
    let access = agent_connection_project_access(
        &context.runtime_home,
        connection_id,
        &context.project.project_id,
    )?
    .ok_or_else(|| {
        EvidenceCommandError::runtime(
            "requesting Agent Connection is no longer registered for this project",
        )
    })?;
    if !access.connection_enabled || !access.project_allowed {
        return Err(EvidenceCommandError::runtime(
            "requesting Agent Connection is disabled or no longer allows this project",
        ));
    }
    Ok(())
}

fn validate_current_basis(
    context: &EvidenceContext,
    intent: &EvidenceCaptureIntentRecord,
) -> Result<(), EvidenceCommandError> {
    let state = context.store.project_state()?;
    if state.active_task_id.as_deref() != Some(intent.task_id.as_str()) {
        return stale_intent("Task is no longer active");
    }
    let task_id = TaskId::new(&intent.task_id);
    let task = context
        .store
        .task_record(&task_id)?
        .ok_or_else(|| EvidenceCommandError::runtime("capture-intent Task is missing"))?;
    if task.lifecycle_phase == "closed"
        || task.current_change_unit_id.as_deref() != Some(intent.change_unit_id.as_str())
        || task.scope_revision != intent.scope_revision
    {
        return stale_intent("Task, Change Unit, or scope revision changed");
    }
    let change_unit = context
        .store
        .change_unit_record(&task_id, &intent.change_unit_id)?
        .ok_or_else(|| EvidenceCommandError::runtime("capture-intent Change Unit is missing"))?;
    if !change_unit.is_current || change_unit.status != "active" {
        return stale_intent("Change Unit is no longer current and active");
    }
    let shaping = json_object("tasks.shaping_summary_json", &task.shaping_summary_json)?;
    let write_basis = json_object(
        "change_units.write_basis_json",
        &change_unit.write_basis_json,
    )?;
    if shaping.get("baseline_ref").and_then(Value::as_str) != Some(intent.baseline_ref.as_str())
        || write_basis.get("baseline_ref").and_then(Value::as_str)
            != Some(intent.baseline_ref.as_str())
    {
        return stale_intent("baseline changed");
    }
    Ok(())
}

fn validate_target(
    store: &CoreProjectStore,
    intent: &EvidenceCaptureIntentRecord,
    target: &EvidenceTarget,
) -> Result<(), EvidenceCommandError> {
    match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => {
            let criterion = store
                .acceptance_criterion_record(acceptance_criterion_id.as_str())?
                .ok_or_else(|| {
                    EvidenceCommandError::runtime("capture-intent acceptance criterion is missing")
                })?;
            if criterion.task_id != intent.task_id || criterion.status != "active" {
                return stale_intent("acceptance criterion is no longer current");
            }
        }
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => {
            if let Some(claim) = store
                .evidence_claim_record(&TaskId::new(&intent.task_id), evidence_claim_id.as_str())?
            {
                if claim.statement != *statement {
                    return stale_intent("supplemental claim changed");
                }
            }
        }
    }
    Ok(())
}

fn validate_workspace(
    context: &EvidenceContext,
    intent: &EvidenceCaptureIntentRecord,
) -> Result<(), EvidenceCommandError> {
    let snapshot = capture_git_workspace_snapshot(&context.project.repo_root)
        .map_err(|error| {
            EvidenceCommandError::runtime(format!(
                "failed to capture current Git workspace context: {error}"
            ))
        })?
        .ok_or_else(|| {
            EvidenceCommandError::runtime(
                "evidence capture requires a current Git workspace context",
            )
        })?;
    let current = json!({
        "git_common_dir": snapshot.layout.common_dir.display().to_string(),
        "worktree_id": snapshot.worktree_id,
        "branch_ref": snapshot.branch_ref,
        "head_sha": snapshot.head_sha,
        "workspace_fingerprint": snapshot.workspace_fingerprint,
    });
    let expected = json_object(
        "evidence capture intent workspace_context_json",
        &intent.workspace_context_json,
    )?;
    if canonical_json_string(&current).map_err(json_runtime)?
        != canonical_json_string(&expected).map_err(json_runtime)?
    {
        return stale_intent("Git workspace context changed");
    }
    Ok(())
}

fn project_current_timestamp(
    store: &CoreProjectStore,
) -> Result<UtcTimestamp, EvidenceCommandError> {
    let timestamp = store.current_timestamp()?;
    strict_stored_timestamp(
        &timestamp,
        "Store returned an invalid Core current UTC timestamp",
    )
}

fn project_time_before_intent_expiry(
    store: &CoreProjectStore,
    intent: &EvidenceCaptureIntentRecord,
) -> Result<(UtcTimestamp, UtcTimestamp), EvidenceCommandError> {
    let created_at = strict_stored_timestamp(
        &intent.created_at,
        "evidence capture intent creation timestamp is corrupt",
    )?;
    let expires_at = strict_stored_timestamp(
        &intent.expires_at,
        "evidence capture intent expiry is corrupt",
    )?;
    if expires_at <= created_at {
        return Err(EvidenceCommandError::runtime(
            "evidence capture intent time window is corrupt",
        ));
    }
    let now = project_current_timestamp(store)?;
    if now >= expires_at {
        return Err(EvidenceCommandError::runtime(
            "evidence capture intent has expired",
        ));
    }
    Ok((now, expires_at))
}

fn receipt_creation_timestamp(
    store: &CoreProjectStore,
    intent: &EvidenceCaptureIntentRecord,
    observed_at: &UtcTimestamp,
) -> Result<UtcTimestamp, EvidenceCommandError> {
    let (mut created_at, _) = project_time_before_intent_expiry(store, intent)?;
    if observed_at > &created_at {
        thread::sleep(PROJECT_CLOCK_RESAMPLE_DELAY);
        (created_at, _) = project_time_before_intent_expiry(store, intent)?;
    }
    if observed_at > &created_at {
        return Err(EvidenceCommandError::runtime(
            "source observation is later than the current Core clock",
        ));
    }
    Ok(created_at)
}

fn validate_not_expired(
    store: &CoreProjectStore,
    intent: &EvidenceCaptureIntentRecord,
) -> Result<(), EvidenceCommandError> {
    project_time_before_intent_expiry(store, intent)?;
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn remaining_intent_ttl(
    store: &CoreProjectStore,
    intent: &EvidenceCaptureIntentRecord,
) -> Result<Duration, EvidenceCommandError> {
    let (now, expires_at) = project_time_before_intent_expiry(store, intent)?;
    expires_at
        .as_datetime()
        .signed_duration_since(*now.as_datetime())
        .to_std()
        .map_err(|_| EvidenceCommandError::runtime("capture-intent TTL is out of range"))
}

fn stale_intent<T>(detail: &str) -> Result<T, EvidenceCommandError> {
    Err(EvidenceCommandError::runtime(format!(
        "evidence capture intent is stale: {detail}"
    )))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn fulfill_command(
    _context: &EvidenceContext,
    _intent: &ValidatedIntent,
    _argv: &[String],
) -> Result<FulfillmentFacts, EvidenceCommandError> {
    Err(EvidenceCommandError::runtime(
        "capture-command is supported only on Linux and macOS because bounded process-tree termination is unavailable on this platform",
    ))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn fulfill_command(
    context: &EvidenceContext,
    intent: &ValidatedIntent,
    argv: &[String],
) -> Result<FulfillmentFacts, EvidenceCommandError> {
    if !matches!(
        intent.capture,
        EvidenceCaptureSpec::VerifiedCommandExecution { .. }
    ) {
        return Err(EvidenceCommandError::runtime(
            "capture-command requires a verified_command_execution intent",
        ));
    }
    let input_sha256 = canonical_json_bare_sha256(&argv).map_err(json_runtime)?;
    if input_sha256 != intent.record.input_sha256 {
        return Err(EvidenceCommandError::runtime(
            "capture-command argument-vector digest does not match the intent",
        ));
    }
    let remaining_ttl = remaining_intent_ttl(&context.store, &intent.record)?;
    let (status, stdout, stderr) =
        run_bounded_capture_command(&context.project.repo_root, argv, remaining_ttl)?;
    let (observed_at, _) = project_time_before_intent_expiry(&context.store, &intent.record)?;
    let exit_code = status.code().ok_or_else(|| {
        EvidenceCommandError::runtime(
            "capture command ended without a numeric exit status; no receipt was created",
        )
    })?;
    let observed_outcome = object_from_value(json!({
        "exit_code": exit_code,
        "stdout_sha256": stdout.sha256,
        "stdout_size_bytes": stdout.size_bytes,
        "stderr_sha256": stderr.sha256,
        "stderr_size_bytes": stderr.size_bytes,
    }))?;
    let host_invocation_id = format!(
        "volicord_command:{}",
        intent.record.evidence_capture_intent_id
    );
    Ok(FulfillmentFacts {
        observed_outcome,
        source: source_object(
            &intent.record.requesting_connection_internal_id,
            intent.session_id.as_deref(),
            None,
            &[],
            &[],
            Some(&host_invocation_id),
        ),
        observed_at: observed_at.to_string(),
        limitations: vec![COMMAND_LIMITATION.to_owned()],
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run_bounded_capture_command(
    repo_root: &Path,
    argv: &[String],
    remaining_ttl: Duration,
) -> Result<
    (
        std::process::ExitStatus,
        CommandStreamDigest,
        CommandStreamDigest,
    ),
    EvidenceCommandError,
> {
    let deadline = Instant::now().checked_add(remaining_ttl).ok_or_else(|| {
        EvidenceCommandError::runtime("capture-intent TTL exceeds the monotonic timer range")
    })?;
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        EvidenceCommandError::runtime(format!("failed to execute capture command: {error}"))
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        EvidenceCommandError::runtime("capture command stdout pipe was not available")
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        EvidenceCommandError::runtime("capture command stderr pipe was not available")
    })?;
    let total_bytes = Arc::new(AtomicU64::new(0));
    let exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader =
        spawn_command_stream_reader(stdout, Arc::clone(&total_bytes), Arc::clone(&exceeded));
    let stderr_reader =
        spawn_command_stream_reader(stderr, Arc::clone(&total_bytes), Arc::clone(&exceeded));

    let status = loop {
        if exceeded.load(Ordering::Acquire) {
            terminate_capture_processes(&mut child);
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(EvidenceCommandError::runtime(format!(
                "capture command output exceeded the {MAX_CAPTURE_COMMAND_OUTPUT_BYTES}-byte bound; no receipt was created"
            )));
        }
        if Instant::now() >= deadline {
            terminate_capture_processes(&mut child);
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(EvidenceCommandError::runtime(
                "capture command did not finish before the capture intent expired; no receipt was created",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(COMMAND_POLL_INTERVAL),
            Err(error) => {
                terminate_capture_processes(&mut child);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(EvidenceCommandError::runtime(format!(
                    "failed while waiting for capture command: {error}"
                )));
            }
        }
    };
    terminate_remaining_capture_process_group(child.id());
    let stdout = join_command_stream_reader(stdout_reader, "stdout")?;
    let stderr = join_command_stream_reader(stderr_reader, "stderr")?;
    if exceeded.load(Ordering::Acquire) {
        return Err(EvidenceCommandError::runtime(format!(
            "capture command output exceeded the {MAX_CAPTURE_COMMAND_OUTPUT_BYTES}-byte bound; no receipt was created"
        )));
    }
    Ok((status, stdout, stderr))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn terminate_capture_processes(child: &mut Child) {
    terminate_remaining_capture_process_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn terminate_remaining_capture_process_group(child_id: u32) {
    if let Ok(raw_pid) = i32::try_from(child_id) {
        if let Some(pid) = Pid::from_raw(raw_pid) {
            let _ = kill_process_group(pid, Signal::KILL);
        }
    }
}

fn spawn_command_stream_reader<R>(
    mut reader: R,
    total_bytes: Arc<AtomicU64>,
    exceeded: Arc<AtomicBool>,
) -> thread::JoinHandle<std::io::Result<CommandStreamDigest>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut hasher = Sha256::new();
        let mut size_bytes = 0_u64;
        let mut buffer = [0_u8; 8192];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            let count = count as u64;
            let prior = total_bytes.fetch_add(count, Ordering::AcqRel);
            if prior
                .checked_add(count)
                .is_none_or(|total| total > MAX_CAPTURE_COMMAND_OUTPUT_BYTES)
            {
                exceeded.store(true, Ordering::Release);
                break;
            }
            size_bytes += count;
            hasher.update(&buffer[..count as usize]);
        }
        Ok(CommandStreamDigest {
            sha256: format!("{:x}", hasher.finalize()),
            size_bytes,
        })
    })
}

fn join_command_stream_reader(
    handle: thread::JoinHandle<std::io::Result<CommandStreamDigest>>,
    stream: &str,
) -> Result<CommandStreamDigest, EvidenceCommandError> {
    handle
        .join()
        .map_err(|_| {
            EvidenceCommandError::runtime(format!("capture command {stream} reader panicked"))
        })?
        .map_err(|error| {
            EvidenceCommandError::runtime(format!(
                "failed to read capture command {stream}: {error}"
            ))
        })
}

fn fulfill_tool(
    context: &EvidenceContext,
    intent: &ValidatedIntent,
    pre_event_id: &str,
    post_event_id: &str,
) -> Result<FulfillmentFacts, EvidenceCommandError> {
    let (expected_tool_name, expected_input_sha256) = match &intent.capture {
        EvidenceCaptureSpec::VerifiedToolInvocation {
            tool_name,
            tool_input_sha256,
            ..
        } => (tool_name.as_str(), tool_input_sha256.as_str()),
        _ => {
            return Err(EvidenceCommandError::runtime(
                "capture-tool requires a verified_tool_invocation intent",
            ))
        }
    };
    let pre = required_guard_event(context, pre_event_id)?;
    let post = required_guard_event(context, post_event_id)?;
    if pre.event_kind != "pre_tool" || post.event_kind != "post_tool" {
        return Err(EvidenceCommandError::runtime(
            "capture-tool requires exact pre_tool and post_tool events",
        ));
    }
    if pre.decision == "deny" {
        return Err(EvidenceCommandError::runtime(
            "a denied pre-tool event cannot prove a completed tool invocation",
        ));
    }
    validate_exact_guard_scope(context, intent, &pre, &post)?;
    validate_event_order(&pre, &post)?;
    validate_source_time_window(intent, &pre.occurred_at, "pre-tool event")?;
    validate_source_time_window(intent, &post.occurred_at, "post-tool event")?;

    let pre_subject = guard_subject_value(&pre)?;
    let post_subject = guard_subject_value(&post)?;
    let pre_raw = required_raw_event(&pre_subject)?;
    let post_raw = required_raw_event(&post_subject)?;
    let pre_invocation = required_host_invocation_id(pre_raw)?;
    let post_invocation = required_host_invocation_id(post_raw)?;
    if pre_invocation != post_invocation {
        return Err(EvidenceCommandError::runtime(
            "pre/post host invocation IDs do not match",
        ));
    }
    let pre_tool_name =
        required_event_string(pre_raw, &[&["tool_name"], &["tool", "name"]], "tool_name")?;
    let post_tool_name =
        required_event_string(post_raw, &[&["tool_name"], &["tool", "name"]], "tool_name")?;
    if pre_tool_name != post_tool_name || pre_tool_name != expected_tool_name {
        return Err(EvidenceCommandError::runtime(
            "pre/post tool names do not match the intent",
        ));
    }
    let pre_input_sha256 = guard_tool_input_sha256(&pre_subject, pre_raw)?;
    let post_input_sha256 = guard_tool_input_sha256(&post_subject, post_raw)?;
    if pre_input_sha256 != post_input_sha256 || pre_input_sha256 != expected_input_sha256 {
        return Err(EvidenceCommandError::runtime(
            "pre/post canonical tool input digest does not match the intent",
        ));
    }

    let tool_response = required_tool_response(post_raw)?;
    reject_incomplete_value(tool_response)?;
    let response_sha256 = safe_digest_from_subject(&post_subject, "tool_result_sha256")
        .ok_or_else(|| {
            EvidenceCommandError::runtime(
                "post-tool event has no complete canonical tool-result digest",
            )
        })?;
    let response_size_bytes = post_subject
        .get("tool_result_size_bytes")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            EvidenceCommandError::runtime(
                "post-tool event has no complete canonical tool-result byte count",
            )
        })?;
    let exit_code = value_i64(
        post_raw,
        &[
            &["tool_response", "exit_code"],
            &["tool_result", "exit_code"],
            &["result", "exit_code"],
            &["output", "exit_code"],
            &["exit_code"],
        ],
    );
    let success = value_bool(
        post_raw,
        &[
            &["tool_response", "success"],
            &["tool_result", "success"],
            &["result", "success"],
            &["output", "success"],
            &["success"],
        ],
    )
    .or_else(|| exit_code.map(|code| code == 0))
    .ok_or_else(|| {
        EvidenceCommandError::runtime(
            "post-tool event does not contain a complete success or exit-code result",
        )
    })?;
    let observed_outcome = object_from_value(json!({
        "success": success,
        "exit_code": exit_code,
        "tool_result_sha256": response_sha256,
        "tool_result_size_bytes": response_size_bytes,
    }))?;
    let session_id = pre.session_id.as_deref().expect("validated guard session");
    let installation_id = pre
        .guard_installation_id
        .as_deref()
        .expect("validated guard installation");
    Ok(FulfillmentFacts {
        observed_outcome,
        source: source_object(
            &pre.connection_internal_id,
            Some(session_id),
            Some(installation_id),
            &[pre.guard_event_id.clone(), post.guard_event_id.clone()],
            &[],
            Some(&pre_invocation),
        ),
        observed_at: post.occurred_at.clone(),
        limitations: vec![HOOK_LIMITATION.to_owned()],
    })
}

fn fulfill_connection(
    context: &EvidenceContext,
    intent: &ValidatedIntent,
    source: &ConnectionSource,
) -> Result<FulfillmentFacts, EvidenceCommandError> {
    let (expected_kind, expected_input_sha256) = match &intent.capture {
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind,
            observation_input_sha256,
            ..
        } => (*source_kind, observation_input_sha256.as_str()),
        _ => {
            return Err(EvidenceCommandError::runtime(
                "capture-connection requires a registered_connection_observation intent",
            ))
        }
    };
    match source {
        ConnectionSource::GuardEvent(event_id) => {
            if expected_kind != ConnectionObservationSourceKind::GuardEvent {
                return Err(EvidenceCommandError::runtime(
                    "capture-connection source kind does not match the intent",
                ));
            }
            let event = required_guard_event(context, event_id)?;
            validate_guard_source_scope(context, intent, &event)?;
            validate_source_time_window(intent, &event.occurred_at, "guard event")?;
            let subject = guard_subject_value(&event)?;
            let raw_event = required_raw_event(&subject)?;
            let input_sha256 = canonical_json_bare_sha256(raw_event).map_err(json_runtime)?;
            if input_sha256 != expected_input_sha256 {
                return Err(EvidenceCommandError::runtime(
                    "selected redacted guard-event digest does not match the intent",
                ));
            }
            let observed_outcome = object_from_value(json!({
                "complete": true,
                "guard_event_kind": event.event_kind,
                "guard_decision": event.decision,
                "observation_sha256": input_sha256,
            }))?;
            Ok(FulfillmentFacts {
                observed_outcome,
                source: source_object(
                    &event.connection_internal_id,
                    event.session_id.as_deref(),
                    event.guard_installation_id.as_deref(),
                    std::slice::from_ref(&event.guard_event_id),
                    &[],
                    None,
                ),
                observed_at: event.occurred_at,
                limitations: vec![HOOK_LIMITATION.to_owned()],
            })
        }
        ConnectionSource::WatchObservation(observation_id) => {
            if expected_kind != ConnectionObservationSourceKind::SessionWatcher {
                return Err(EvidenceCommandError::runtime(
                    "capture-connection source kind does not match the intent",
                ));
            }
            let observation = watch_observation(
                &context.runtime_home,
                &context.project.project_id,
                observation_id,
            )?
            .ok_or_else(|| {
                EvidenceCommandError::runtime(format!(
                    "session-watch observation was not found: {observation_id}"
                ))
            })?;
            validate_watch_source_scope(context, intent, &observation)?;
            validate_source_time_window(
                intent,
                &observation.observed_at,
                "session-watch observation",
            )?;
            let baseline = watch_baseline(
                &context.runtime_home,
                &context.project.project_id,
                &observation.watch_baseline_id,
            )?
            .ok_or_else(|| {
                EvidenceCommandError::runtime(
                    "session-watch observation baseline is not registered",
                )
            })?;
            validate_complete_watch_observation(&baseline, &observation)?;
            let selection = watch_observation_selection(&observation)?;
            let input_sha256 = canonical_json_bare_sha256(&selection).map_err(json_runtime)?;
            if input_sha256 != expected_input_sha256 {
                return Err(EvidenceCommandError::runtime(
                    "selected session-watch observation digest does not match the intent",
                ));
            }
            let observed_outcome = object_from_value(json!({
                "complete": true,
                "snapshot_algorithm": observation.snapshot_algorithm,
                "snapshot_digest": observation.snapshot_digest,
                "observation_sha256": input_sha256,
            }))?;
            Ok(FulfillmentFacts {
                observed_outcome,
                source: source_object(
                    &observation.connection_internal_id,
                    Some(&observation.session_id),
                    None,
                    &[],
                    std::slice::from_ref(&observation.watch_observation_id),
                    None,
                ),
                observed_at: observation.observed_at,
                limitations: vec![WATCH_LIMITATION.to_owned()],
            })
        }
    }
}

fn persist_fulfillment(
    context: &mut EvidenceContext,
    intent: &ValidatedIntent,
    facts: FulfillmentFacts,
    output_json: bool,
) -> Result<String, EvidenceCommandError> {
    validate_source_time_window(intent, &facts.observed_at, "source observation")?;
    let result_sha256 =
        canonical_json_bare_sha256(&facts.observed_outcome).map_err(json_runtime)?;
    let observed_at = UtcTimestamp::parse(&facts.observed_at)
        .map_err(|_| EvidenceCommandError::runtime("source observation timestamp is invalid"))?;
    let observed_by_actor_source = ActorSource::from_str(&intent.record.requested_by_actor_source)
        .map_err(|error| EvidenceCommandError::runtime(error.to_string()))?;
    let safe_receipt = PersistedEvidenceCaptureReceiptBody {
        schema_version: RECEIPT_SCHEMA_VERSION.to_owned(),
        capture_kind: capture_producer_kind(&intent.capture),
        capture_intent_id: EvidenceCaptureIntentId::new(&intent.record.evidence_capture_intent_id),
        input_sha256: intent.record.input_sha256.clone(),
        result_sha256: result_sha256.clone(),
        expected_outcome: intent.expected_outcome.clone(),
        observed_outcome: facts.observed_outcome.clone(),
        source: facts.source,
        complete: true,
        limitations: facts.limitations,
        redaction_state: RedactionState::Redacted,
        observed_by_actor_source,
        observed_at,
    };
    let safe_receipt_json = canonical_json_string(&safe_receipt).map_err(json_runtime)?;
    let generator = RandomDurableIdGenerator;
    let receipt_id = generator
        .generate(DurableIdKind::EvidenceCaptureReceipt)
        .map_err(|error| EvidenceCommandError::runtime(error.to_string()))?;
    let staging_handle_id = generator
        .generate(DurableIdKind::StagedArtifact)
        .map_err(|error| EvidenceCommandError::runtime(error.to_string()))?;
    let expected_outcome_json =
        canonical_json_string(&intent.expected_outcome).map_err(json_runtime)?;
    let observed_outcome_json =
        canonical_json_string(&safe_receipt.observed_outcome).map_err(json_runtime)?;
    let limitations_json =
        canonical_json_string(&safe_receipt.limitations).map_err(json_runtime)?;
    let observed_at = safe_receipt.observed_at.to_canonical_string();
    let metadata_json =
        canonical_json_string(&json!({ "source": &safe_receipt.source })).map_err(json_runtime)?;
    let created_at =
        receipt_creation_timestamp(&context.store, &intent.record, &safe_receipt.observed_at)?;
    let record = context
        .store
        .fulfill_evidence_capture_source(EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: receipt_id,
            evidence_capture_intent_id: intent.record.evidence_capture_intent_id.clone(),
            staging_handle_id,
            task_id: intent.record.task_id.clone(),
            capture_kind: intent.record.capture_kind.clone(),
            input_sha256: intent.record.input_sha256.clone(),
            result_sha256,
            expected_outcome_json,
            observed_outcome_json,
            source_refs_json: "[]".to_owned(),
            observed_by_actor_source: intent.record.requested_by_actor_source.clone(),
            observed_at,
            limitations_json,
            safe_receipt_json,
            created_at: created_at.to_string(),
            staging_expires_at: intent.record.expires_at.clone(),
            metadata_json,
        })?;
    render_receipt(&record, output_json)
}

fn render_receipt(
    record: &volicord_store::core_pipeline::EvidenceCaptureReceiptRecord,
    output_json: bool,
) -> Result<String, EvidenceCommandError> {
    let observed_outcome: Value = strict_json(
        "evidence capture receipt observed_outcome_json",
        &record.observed_outcome_json,
    )?;
    if output_json {
        return serde_json::to_string_pretty(&json!({
            "capture_intent_id": record.evidence_capture_intent_id,
            "capture_receipt_id": record.evidence_capture_receipt_id,
            "capture_kind": record.capture_kind,
            "staged_receipt_handle_id": record.staging_handle_id,
            "complete": record.completeness == "complete",
            "observed_at": record.observed_at,
            "observed_outcome": observed_outcome,
        }))
        .map(|text| format!("{text}\n"))
        .map_err(json_runtime);
    }
    let observed = canonical_json_string(&observed_outcome).map_err(json_runtime)?;
    Ok(format!(
        "Evidence capture receipt created\nintent: {}\nreceipt: {}\ncapture_kind: {}\nstaged_receipt_handle: {}\ncomplete: true\nobserved_at: {}\nobserved_outcome: {}\n",
        record.evidence_capture_intent_id,
        record.evidence_capture_receipt_id,
        record.capture_kind,
        record.staging_handle_id,
        record.observed_at,
        observed,
    ))
}

fn required_guard_event(
    context: &EvidenceContext,
    event_id: &str,
) -> Result<GuardEventRecord, EvidenceCommandError> {
    guard_event(&context.runtime_home, &context.project.project_id, event_id)?.ok_or_else(|| {
        EvidenceCommandError::runtime(format!("guard event was not found: {event_id}"))
    })
}

fn validate_exact_guard_scope(
    context: &EvidenceContext,
    intent: &ValidatedIntent,
    pre: &GuardEventRecord,
    post: &GuardEventRecord,
) -> Result<(), EvidenceCommandError> {
    if pre.connection_internal_id != post.connection_internal_id
        || pre.connection_internal_id != intent.record.requesting_connection_internal_id
        || pre.session_id.is_none()
        || pre.session_id != post.session_id
        || pre.guard_installation_id.is_none()
        || pre.guard_installation_id != post.guard_installation_id
    {
        return Err(EvidenceCommandError::runtime(
            "pre/post events do not have the exact registered connection, session, and guard installation",
        ));
    }
    if intent
        .session_id
        .as_deref()
        .is_some_and(|session_id| pre.session_id.as_deref() != Some(session_id))
    {
        return Err(EvidenceCommandError::runtime(
            "pre/post session does not match the capture-intent session",
        ));
    }
    validate_active_guard_installation(
        context,
        &pre.connection_internal_id,
        pre.session_id.as_deref().expect("checked session"),
        pre.guard_installation_id
            .as_deref()
            .expect("checked installation"),
    )
}

fn validate_guard_source_scope(
    context: &EvidenceContext,
    intent: &ValidatedIntent,
    event: &GuardEventRecord,
) -> Result<(), EvidenceCommandError> {
    if event.connection_internal_id != intent.record.requesting_connection_internal_id {
        return Err(EvidenceCommandError::runtime(
            "guard event belongs to another Agent Connection",
        ));
    }
    let session_id = event.session_id.as_deref().ok_or_else(|| {
        EvidenceCommandError::runtime("guard-event source requires an exact session")
    })?;
    let installation_id = event.guard_installation_id.as_deref().ok_or_else(|| {
        EvidenceCommandError::runtime("guard-event source requires an exact guard installation")
    })?;
    if intent
        .session_id
        .as_deref()
        .is_some_and(|expected| expected != session_id)
    {
        return Err(EvidenceCommandError::runtime(
            "guard-event source session does not match the intent",
        ));
    }
    validate_active_guard_installation(
        context,
        &event.connection_internal_id,
        session_id,
        installation_id,
    )
}

fn validate_active_guard_installation(
    context: &EvidenceContext,
    connection_id: &str,
    session_id: &str,
    installation_id: &str,
) -> Result<(), EvidenceCommandError> {
    let installation = guard_installation(&context.runtime_home, installation_id)?
        .ok_or_else(|| EvidenceCommandError::runtime("guard installation is not registered"))?;
    if installation.connection_internal_id != connection_id
        || installation.project_id.as_deref() != Some(context.project.project_id.as_str())
        || installation.installation_status != "active"
    {
        return Err(EvidenceCommandError::runtime(
            "guard installation is not active for this connection and project",
        ));
    }
    let session = agent_session(
        &context.runtime_home,
        &context.project.project_id,
        session_id,
    )?
    .ok_or_else(|| EvidenceCommandError::runtime("guard session is not registered"))?;
    if session.connection_internal_id != connection_id
        || session.guard_installation_id.as_deref() != Some(installation_id)
    {
        return Err(EvidenceCommandError::runtime(
            "guard session does not match the registered connection and installation",
        ));
    }
    Ok(())
}

fn validate_watch_source_scope(
    context: &EvidenceContext,
    intent: &ValidatedIntent,
    observation: &WatchObservationRecord,
) -> Result<(), EvidenceCommandError> {
    if observation.connection_internal_id != intent.record.requesting_connection_internal_id {
        return Err(EvidenceCommandError::runtime(
            "session-watch observation belongs to another Agent Connection",
        ));
    }
    if intent
        .session_id
        .as_deref()
        .is_some_and(|expected| expected != observation.session_id)
    {
        return Err(EvidenceCommandError::runtime(
            "session-watch observation belongs to another session",
        ));
    }
    let session = agent_session(
        &context.runtime_home,
        &context.project.project_id,
        &observation.session_id,
    )?
    .ok_or_else(|| EvidenceCommandError::runtime("watch source session is not registered"))?;
    if session.connection_internal_id != observation.connection_internal_id {
        return Err(EvidenceCommandError::runtime(
            "watch source session does not match its Agent Connection",
        ));
    }
    Ok(())
}

fn validate_event_order(
    pre: &GuardEventRecord,
    post: &GuardEventRecord,
) -> Result<(), EvidenceCommandError> {
    let pre_time = strict_stored_timestamp(&pre.occurred_at, "pre-tool timestamp is corrupt")?;
    let post_time = strict_stored_timestamp(&post.occurred_at, "post-tool timestamp is corrupt")?;
    if pre_time.as_datetime() > post_time.as_datetime() {
        return Err(EvidenceCommandError::runtime(
            "post-tool event precedes its pre-tool event",
        ));
    }
    Ok(())
}

fn validate_source_time_window(
    intent: &ValidatedIntent,
    observed_at: &str,
    source_label: &str,
) -> Result<(), EvidenceCommandError> {
    let created_at = strict_stored_timestamp(
        &intent.record.created_at,
        "evidence capture intent creation timestamp is corrupt",
    )?;
    let expires_at = strict_stored_timestamp(
        &intent.record.expires_at,
        "evidence capture intent expiry is corrupt",
    )?;
    if expires_at <= created_at {
        return Err(EvidenceCommandError::runtime(
            "evidence capture intent time window is corrupt",
        ));
    }
    let observed_at =
        strict_stored_timestamp(observed_at, &format!("{source_label} timestamp is corrupt"))?;
    if observed_at.as_datetime() < created_at.as_datetime()
        || observed_at.as_datetime() >= expires_at.as_datetime()
    {
        return Err(EvidenceCommandError::runtime(format!(
            "{source_label} is outside the capture-intent source window"
        )));
    }
    Ok(())
}

fn strict_stored_timestamp(
    raw: &str,
    corrupt_message: &str,
) -> Result<UtcTimestamp, EvidenceCommandError> {
    let timestamp =
        UtcTimestamp::parse(raw).map_err(|_| EvidenceCommandError::runtime(corrupt_message))?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| EvidenceCommandError::runtime(corrupt_message))?;
    Ok(timestamp)
}

fn guard_subject_value(event: &GuardEventRecord) -> Result<Value, EvidenceCommandError> {
    json_object("guard_events.subject_json", &event.subject_json)
}

fn required_raw_event(subject: &Value) -> Result<&Value, EvidenceCommandError> {
    subject
        .get("raw_event")
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            EvidenceCommandError::runtime("guard event has no redacted raw_event object")
        })
}

fn required_host_invocation_id(event: &Value) -> Result<String, EvidenceCommandError> {
    required_event_string(
        event,
        &[
            &["tool_use_id"],
            &["tool_invocation_id"],
            &["tool_call_id"],
            &["invocation_id"],
            &["call_id"],
            &["tool", "id"],
            &["tool_use", "id"],
        ],
        "host invocation ID",
    )
}

fn required_event_string(
    event: &Value,
    paths: &[&[&str]],
    label: &str,
) -> Result<String, EvidenceCommandError> {
    paths
        .iter()
        .find_map(|path| value_at(event, path).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| EvidenceCommandError::runtime(format!("guard event has no {label}")))
}

fn guard_tool_input_sha256(
    subject: &Value,
    raw_event: &Value,
) -> Result<String, EvidenceCommandError> {
    if let Some(digest) = safe_digest_from_subject(subject, "tool_input_sha256") {
        return Ok(digest);
    }
    let input = tool_input(raw_event)
        .ok_or_else(|| EvidenceCommandError::runtime("guard event has no canonical tool input"))?;
    canonical_json_bare_sha256(input).map_err(json_runtime)
}

fn safe_digest_from_subject(subject: &Value, field: &str) -> Option<String> {
    subject
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| lowercase_sha256(value))
        .map(str::to_owned)
}

fn tool_input(event: &Value) -> Option<&Value> {
    [
        &["tool_input"][..],
        &["input"][..],
        &["tool", "input"][..],
        &["tool", "arguments"][..],
        &["tool_use", "input"][..],
    ]
    .iter()
    .find_map(|path| value_at(event, path))
}

fn required_tool_response(event: &Value) -> Result<&Value, EvidenceCommandError> {
    [
        &["tool_response"][..],
        &["tool_result"][..],
        &["result"][..],
        &["output"][..],
    ]
    .iter()
    .find_map(|path| value_at(event, path))
    .filter(|value| !value.is_null())
    .ok_or_else(|| EvidenceCommandError::runtime("post-tool event has no complete tool response"))
}

fn reject_incomplete_value(value: &Value) -> Result<(), EvidenceCommandError> {
    match value {
        Value::Object(object) => {
            if object.get("truncated").and_then(Value::as_bool) == Some(true)
                || object.get("partial").and_then(Value::as_bool) == Some(true)
                || object.get("complete").and_then(Value::as_bool) == Some(false)
                || object.get("is_complete").and_then(Value::as_bool) == Some(false)
            {
                return Err(EvidenceCommandError::runtime(
                    "post-tool response is truncated or incomplete",
                ));
            }
            for child in object.values() {
                reject_incomplete_value(child)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                reject_incomplete_value(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_complete_watch_observation(
    baseline: &WatchBaselineRecord,
    observation: &WatchObservationRecord,
) -> Result<(), EvidenceCommandError> {
    if baseline.status != "active"
        || baseline.project_id != observation.project_id
        || baseline.watch_baseline_id != observation.watch_baseline_id
        || baseline.session_id != observation.session_id
        || baseline.connection_internal_id != observation.connection_internal_id
        || baseline.snapshot_algorithm != observation.snapshot_algorithm
    {
        return Err(EvidenceCommandError::runtime(
            "session-watch observation does not match an active registered baseline",
        ));
    }
    let validated =
        validate_persisted_watch_observation(baseline, observation).map_err(|error| {
            EvidenceCommandError::runtime(format!(
                "session-watch observation integrity validation failed: {error}"
            ))
        })?;
    require_complete_watch_scan(&validated.baseline_snapshot.scan_summary, "baseline")?;
    let derived = validated.observation_snapshot.scan_summary;
    validate_observation_scan_metadata(observation, &derived)?;
    require_complete_watch_scan(&derived, "observation")
}

fn validate_observation_scan_metadata(
    observation: &WatchObservationRecord,
    derived: &WatchScanSummary,
) -> Result<(), EvidenceCommandError> {
    let metadata = json_object(
        "session_watch_observations.metadata_json",
        &observation.metadata_json,
    )?;
    let explicit_value = metadata
        .get("scan_summary")
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            EvidenceCommandError::runtime(
                "session-watch observation metadata has no canonical scan_summary",
            )
        })?;
    let explicit_object = explicit_value.as_object().expect("checked object");
    const SUMMARY_FIELDS: [&str; 7] = [
        "files_scanned",
        "files_skipped",
        "unreadable_paths_count",
        "degraded_reasons",
        "degraded_reason_counts",
        "skipped_paths_sample",
        "skipped_paths_truncated",
    ];
    if explicit_object.len() != SUMMARY_FIELDS.len()
        || SUMMARY_FIELDS
            .iter()
            .any(|field| !explicit_object.contains_key(*field))
    {
        return Err(EvidenceCommandError::runtime(
            "session-watch observation scan_summary has invalid fields",
        ));
    }
    let explicit: WatchScanSummary =
        serde_json::from_value(explicit_value.clone()).map_err(|_| {
            EvidenceCommandError::runtime("session-watch observation scan_summary is corrupt")
        })?;
    if &explicit != derived {
        return Err(EvidenceCommandError::runtime(
            "session-watch observation scan_summary is inconsistent with its snapshot entries",
        ));
    }
    Ok(())
}

fn require_complete_watch_scan(
    summary: &WatchScanSummary,
    label: &str,
) -> Result<(), EvidenceCommandError> {
    if summary.files_skipped != 0
        || summary.unreadable_paths_count != 0
        || !summary.degraded_reasons.is_empty()
        || !summary.degraded_reason_counts.is_empty()
        || !summary.skipped_paths_sample.is_empty()
        || summary.skipped_paths_truncated
    {
        return Err(EvidenceCommandError::runtime(format!(
            "session-watch {label} is incomplete or degraded"
        )));
    }
    Ok(())
}

fn watch_observation_selection(
    observation: &WatchObservationRecord,
) -> Result<Value, EvidenceCommandError> {
    Ok(json!({
        "watch_observation_id": observation.watch_observation_id,
        "watch_baseline_id": observation.watch_baseline_id,
        "session_id": observation.session_id,
        "connection_id": observation.connection_internal_id,
        "snapshot_algorithm": observation.snapshot_algorithm,
        "snapshot_digest": observation.snapshot_digest,
        "snapshot_entries": strict_json::<Value>(
            "session_watch_observations.snapshot_entries_json",
            &observation.snapshot_entries_json,
        )?,
        "observed_paths": strict_json::<Value>(
            "session_watch_observations.observed_paths_json",
            &observation.observed_paths_json,
        )?,
        "change_summary": strict_json::<Value>(
            "session_watch_observations.change_summary_json",
            &observation.change_summary_json,
        )?,
        "observed_at": observation.observed_at,
    }))
}

fn source_object(
    connection_id: &str,
    session_id: Option<&str>,
    guard_installation_id: Option<&str>,
    guard_event_ids: &[String],
    watch_observation_refs: &[String],
    host_invocation_id: Option<&str>,
) -> PersistedEvidenceCaptureReceiptSource {
    PersistedEvidenceCaptureReceiptSource {
        connection_id: AgentConnectionId::new(connection_id),
        session_id: session_id.map(AgentSessionId::new).into(),
        guard_installation_id: guard_installation_id.map(GuardInstallationId::new).into(),
        guard_event_ids: guard_event_ids.iter().map(GuardEventId::new).collect(),
        watch_observation_refs: watch_observation_refs.to_vec(),
        host_invocation_id: host_invocation_id.map(str::to_owned).into(),
    }
}

fn capture_kind(capture: &EvidenceCaptureSpec) -> &'static str {
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => "verified_command_execution",
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => "verified_tool_invocation",
        EvidenceCaptureSpec::RegisteredConnectionObservation { .. } => {
            "registered_connection_observation"
        }
    }
}

fn capture_producer_kind(capture: &EvidenceCaptureSpec) -> EvidenceProducerKind {
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            EvidenceProducerKind::VerifiedCommandExecution
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => {
            EvidenceProducerKind::VerifiedToolInvocation
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation { .. } => {
            EvidenceProducerKind::RegisteredConnectionObservation
        }
    }
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn strict_json<T: serde::de::DeserializeOwned>(
    label: &str,
    text: &str,
) -> Result<T, EvidenceCommandError> {
    serde_json::from_str(text)
        .map_err(|error| EvidenceCommandError::runtime(format!("{label} is corrupt: {error}")))
}

fn json_object(label: &str, text: &str) -> Result<Value, EvidenceCommandError> {
    let value: Value = strict_json(label, text)?;
    if !value.is_object() {
        return Err(EvidenceCommandError::runtime(format!(
            "{label} is not an object"
        )));
    }
    Ok(value)
}

fn object_from_value(value: Value) -> Result<JsonObject, EvidenceCommandError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(EvidenceCommandError::runtime(
            "internal evidence-capture outcome must be an object",
        )),
    }
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn value_bool(value: &Value, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_bool))
}

fn value_i64(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_i64))
}

fn json_runtime(error: serde_json::Error) -> EvidenceCommandError {
    EvidenceCommandError::runtime(format!("failed to encode canonical JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_capture_command_delimiter_without_reinterpreting_program_arguments() {
        let parsed = parse_evidence_command(&[
            "capture-command".to_owned(),
            "--intent".to_owned(),
            "intent_1".to_owned(),
            "--json".to_owned(),
            "--".to_owned(),
            "program".to_owned(),
            "--intent".to_owned(),
            "child-value".to_owned(),
        ])
        .expect("capture command should parse");
        assert_eq!(
            parsed,
            EvidenceCommand::CaptureCommand {
                intent_id: "intent_1".to_owned(),
                repo: None,
                json: true,
                argv: vec![
                    "program".to_owned(),
                    "--intent".to_owned(),
                    "child-value".to_owned(),
                ],
            }
        );
    }

    #[test]
    fn canonical_capture_digests_are_bare_lowercase_sha256() {
        let digest = canonical_json_bare_sha256(&vec!["program", "한글", "--flag"])
            .expect("argv should hash");
        assert!(lowercase_sha256(&digest));
        assert!(!digest.starts_with("sha256:"));
        assert_eq!(
            digest,
            canonical_json_bare_sha256(&json!(["program", "한글", "--flag"]))
                .expect("equivalent JSON should hash identically")
        );
    }

    #[test]
    fn tool_response_paths_supply_success_and_exit_code() {
        let event = json!({
            "tool_response": {
                "success": false,
                "exit_code": 17,
                "stdout": "not persisted in a safe receipt"
            }
        });
        assert_eq!(
            value_bool(&event, &[&["tool_response", "success"]]),
            Some(false)
        );
        assert_eq!(
            value_i64(&event, &[&["tool_response", "exit_code"]]),
            Some(17)
        );
    }

    #[test]
    fn incomplete_tool_results_are_rejected() {
        for result in [
            json!({"truncated": true}),
            json!({"complete": false}),
            json!({"nested": {"partial": true}}),
        ] {
            assert!(reject_incomplete_value(&result).is_err());
        }
        assert!(reject_incomplete_value(&json!({"success": false})).is_ok());
    }

    #[test]
    fn command_stream_reader_caps_combined_output_without_buffering_raw_bytes() {
        let total = Arc::new(AtomicU64::new(0));
        let exceeded = Arc::new(AtomicBool::new(false));
        let reader = std::io::repeat(0).take(MAX_CAPTURE_COMMAND_OUTPUT_BYTES + 1);
        let result = spawn_command_stream_reader(reader, Arc::clone(&total), Arc::clone(&exceeded))
            .join()
            .expect("reader should not panic")
            .expect("repeat reader should be infallible");
        assert!(exceeded.load(Ordering::Acquire));
        assert!(result.size_bytes <= MAX_CAPTURE_COMMAND_OUTPUT_BYTES);
        assert!(total.load(Ordering::Acquire) > MAX_CAPTURE_COMMAND_OUTPUT_BYTES);
    }

    #[test]
    fn command_stream_readers_accept_the_exact_combined_output_boundary() {
        let total = Arc::new(AtomicU64::new(0));
        let exceeded = Arc::new(AtomicBool::new(false));
        let half = MAX_CAPTURE_COMMAND_OUTPUT_BYTES / 2;
        let stdout = spawn_command_stream_reader(
            std::io::repeat(0).take(half),
            Arc::clone(&total),
            Arc::clone(&exceeded),
        );
        let stderr = spawn_command_stream_reader(
            std::io::repeat(1).take(MAX_CAPTURE_COMMAND_OUTPUT_BYTES - half),
            Arc::clone(&total),
            Arc::clone(&exceeded),
        );
        let stdout = stdout
            .join()
            .expect("stdout reader should not panic")
            .expect("stdout reader should be infallible");
        let stderr = stderr
            .join()
            .expect("stderr reader should not panic")
            .expect("stderr reader should be infallible");
        assert!(!exceeded.load(Ordering::Acquire));
        assert_eq!(
            total.load(Ordering::Acquire),
            MAX_CAPTURE_COMMAND_OUTPUT_BYTES
        );
        assert_eq!(
            stdout.size_bytes + stderr.size_bytes,
            MAX_CAPTURE_COMMAND_OUTPUT_BYTES
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn command_runner_rejects_deadline_without_waiting_for_child_completion() {
        let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), "sleep 5".to_owned()];
        let started = Instant::now();
        let error = run_bounded_capture_command(Path::new("/tmp"), &argv, Duration::ZERO)
            .expect_err("expired deadline should kill the child");
        assert!(error.to_string().contains("did not finish before"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn command_runner_deadline_terminates_descendants_holding_output_pipes() {
        let argv = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "sleep 5 & wait".to_owned(),
        ];
        let started = Instant::now();
        let error =
            run_bounded_capture_command(Path::new("/tmp"), &argv, Duration::from_millis(100))
                .expect_err("deadline should terminate the isolated command process group");
        assert!(error.to_string().contains("did not finish before"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn watch_completeness_fails_closed_without_canonical_scan_metadata() {
        let observation = WatchObservationRecord {
            project_id: "project_test".to_owned(),
            watch_observation_id: "watch_observation_test".to_owned(),
            watch_baseline_id: "watch_baseline_test".to_owned(),
            session_id: "session_test".to_owned(),
            connection_internal_id: "connection_test".to_owned(),
            expected_write_id: None,
            unrecorded_change_id: None,
            observation_status: "observed".to_owned(),
            observed_paths_json: "[]".to_owned(),
            change_summary_json: "{}".to_owned(),
            snapshot_algorithm: "sha256-v1".to_owned(),
            snapshot_digest: "0".repeat(64),
            snapshot_entries_json: "[]".to_owned(),
            observed_at: "2026-07-13T00:00:00Z".to_owned(),
            linked_at: None,
            metadata_json: "{}".to_owned(),
        };

        let error = validate_observation_scan_metadata(
            &observation,
            &WatchScanSummary {
                files_scanned: 0,
                files_skipped: 0,
                unreadable_paths_count: 0,
                degraded_reasons: Vec::new(),
                degraded_reason_counts: Default::default(),
                skipped_paths_sample: Vec::new(),
                skipped_paths_truncated: false,
            },
        )
        .expect_err("missing explicit scan metadata must fail closed");
        assert!(error.to_string().contains("no canonical scan_summary"));
    }

    #[test]
    fn watch_completeness_rejects_baseline_only_degradation() {
        let summary = WatchScanSummary {
            files_scanned: 0,
            files_skipped: 1,
            unreadable_paths_count: 0,
            degraded_reasons: vec!["file_size_limit".to_owned()],
            degraded_reason_counts: [("file_size_limit".to_owned(), 1)].into(),
            skipped_paths_sample: vec!["large.bin".to_owned()],
            skipped_paths_truncated: false,
        };
        let error = require_complete_watch_scan(&summary, "baseline")
            .expect_err("baseline-only degradation must fail closed");
        assert!(error.to_string().contains("baseline is incomplete"));
    }
}
