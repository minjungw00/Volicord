use std::{
    ffi::OsString,
    fmt,
    io::Read,
    path::Path,
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
    agent_connections::{agent_connection_project_access, agent_connection_record_read_only},
    bootstrap::{ProjectRecord, ACTIVE_PROJECT_STATUS},
    core_pipeline::{CoreProjectStore, EvidenceCaptureIntentRecord, EvidenceCaptureReceiptInsert},
    guards::{
        agent_session, guard_event, guard_installation, guard_observation_summary,
        validate_stored_guard_installation_manifest_binding, GuardEventRecord,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    CanonicalRuntimeHomePath, RuntimeHomeMutationContext, StoreError,
};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_string, evidence_capture_input_sha256, ActorSource,
    AgentConnectionId, DurableIdGenerator, DurableIdKind, EvidenceCaptureIntentId,
    EvidenceCaptureSpec, EvidenceProducerKind, EvidenceTarget, JsonObject,
    PersistedEvidenceCaptureReceiptBody, PersistedEvidenceCaptureReceiptSource, ProjectId,
    RandomDurableIdGenerator, RedactionState, TaskId, UtcTimestamp,
    EVIDENCE_CAPTURE_COMMAND_LIMITATION as COMMAND_LIMITATION,
    EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID,
};

use crate::cli::{EvidenceArgs, EvidenceCommand};
use crate::mutation_admission::{with_cli_runtime_home_mutation, CliMutationAdmissionError};
use crate::project_context::{
    registered_project_for_repo_admitted, resolve_repository_root, ProjectCommandError,
};

const MAX_CAPTURE_COMMAND_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const PROJECT_CLOCK_RESAMPLE_DELAY: Duration = Duration::from_millis(1);
#[cfg(any(target_os = "linux", target_os = "macos"))]
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceCommandError {
    Usage(String),
    Runtime(String),
    MutationAdmission(CliMutationAdmissionError),
}

impl EvidenceCommandError {
    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for EvidenceCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
            Self::MutationAdmission(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EvidenceCommandError {}

impl From<CliMutationAdmissionError> for EvidenceCommandError {
    fn from(error: CliMutationAdmissionError) -> Self {
        Self::MutationAdmission(error)
    }
}

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
            ProjectCommandError::MutationAdmission(error) => Self::MutationAdmission(error),
        }
    }
}

#[derive(Debug)]
struct EvidenceContext<'mutation> {
    runtime_home: CanonicalRuntimeHomePath,
    project: ProjectRecord,
    store: CoreProjectStore<'mutation>,
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
    args: EvidenceArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, EvidenceCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    with_cli_runtime_home_mutation(&runtime_home, "cli.evidence.capture", |mutation_context| {
        let result = (|| -> Result<String, EvidenceCommandError> {
            match args.command {
                EvidenceCommand::CaptureCommand(options) => {
                    let mut context =
                        resolve_context(mutation_context, current_dir, options.repo.as_deref())?;
                    let intent = load_and_validate_intent(&context, &options.intent)?;
                    let facts = fulfill_command(&context, &intent, &options.program)?;
                    persist_fulfillment(&mut context, &intent, facts, options.json)
                }
                EvidenceCommand::CaptureTool(options) => {
                    let mut context =
                        resolve_context(mutation_context, current_dir, options.repo.as_deref())?;
                    let intent = load_and_validate_intent(&context, &options.intent)?;
                    let facts =
                        fulfill_tool(&context, &intent, &options.pre_event, &options.post_event)?;
                    persist_fulfillment(&mut context, &intent, facts, options.json)
                }
            }
        })();
        result.map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
    })
    .map_err(Into::into)
}

fn resolve_context<'mutation>(
    mutation_context: &'mutation RuntimeHomeMutationContext<'mutation>,
    current_dir: &Path,
    selected_repo: Option<&Path>,
) -> Result<EvidenceContext<'mutation>, EvidenceCommandError> {
    let repo_root = resolve_repository_root(current_dir, selected_repo)?;
    let project = registered_project_for_repo_admitted(mutation_context, &repo_root)?;
    if project.status != ACTIVE_PROJECT_STATUS {
        return Err(EvidenceCommandError::runtime(
            "evidence capture requires an active registered project",
        ));
    }
    let store = CoreProjectStore::open_for_mutation(
        mutation_context,
        &ProjectId::new(&project.project_id),
    )?;
    Ok(EvidenceContext {
        runtime_home: mutation_context.runtime_home().clone(),
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
    if evidence_capture_input_sha256(&capture).map_err(json_runtime)? != record.input_sha256 {
        return Err(EvidenceCommandError::runtime(
            "evidence capture intent input digest is corrupt",
        ));
    }
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
    Ok(FulfillmentFacts {
        observed_outcome,
        source: source_object(&pre.connection_internal_id, Some(&pre_invocation)),
        observed_at: post.occurred_at.clone(),
        limitations: vec![COMMAND_LIMITATION.to_owned()],
    })
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
        contract_id: EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID.to_owned(),
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
        || pre.guard_installation_id != post.guard_installation_id
        || pre.policy_hash != post.policy_hash
        || pre.integration_revision != post.integration_revision
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
        &pre.guard_installation_id,
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
        || installation.project_id != context.project.project_id
    {
        return Err(EvidenceCommandError::runtime(
            "guard installation is not current for this connection and project",
        ));
    }
    let connection = agent_connection_record_read_only(&context.runtime_home, connection_id)?
        .ok_or_else(|| EvidenceCommandError::runtime("guard connection is not registered"))?;
    validate_stored_guard_installation_manifest_binding(
        &installation,
        &connection,
        &context.project.repo_root,
    )?;
    if !guard_observation_summary(
        &context.runtime_home,
        &context.project.project_id,
        &installation,
    )?
    .all_required_phases_observed()
    {
        return Err(EvidenceCommandError::runtime(
            "current Guard phases have not all been observed",
        ));
    }
    let session = agent_session(
        &context.runtime_home,
        &context.project.project_id,
        session_id,
    )?
    .ok_or_else(|| EvidenceCommandError::runtime("guard session is not registered"))?;
    if session.connection_internal_id != connection_id
        || !volicord_store::guards::agent_session_matches_current_integration(
            &context.runtime_home,
            &session,
            Some(installation_id),
        )?
    {
        return Err(EvidenceCommandError::runtime(
            "guard session does not match the registered connection and installation",
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

fn source_object(
    connection_id: &str,
    host_invocation_id: Option<&str>,
) -> PersistedEvidenceCaptureReceiptSource {
    PersistedEvidenceCaptureReceiptSource {
        connection_id: AgentConnectionId::new(connection_id),
        host_invocation_id: host_invocation_id.map(str::to_owned).into(),
    }
}

fn capture_kind(capture: &EvidenceCaptureSpec) -> &'static str {
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => "verified_command_execution",
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => "verified_tool_invocation",
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
    use clap::Parser;

    use crate::cli::{Cli, Command as CliCommand};

    use super::*;

    #[test]
    fn parses_capture_command_delimiter_without_reinterpreting_program_arguments() {
        let parsed = Cli::try_parse_from([
            "volicord",
            "evidence",
            "capture-command",
            "--intent",
            "intent_1",
            "--json",
            "--",
            "program",
            "--intent",
            "child-value",
        ])
        .expect("capture command should parse");
        let Some(CliCommand::Evidence(EvidenceArgs {
            command: EvidenceCommand::CaptureCommand(options),
        })) = parsed.command
        else {
            panic!("unexpected parsed command")
        };
        assert_eq!(options.intent, "intent_1");
        assert!(options.json);
        assert_eq!(options.program, ["program", "--intent", "child-value"]);
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
}
