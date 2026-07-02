use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt, fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_core::{CorePipelineError, CoreService, InvocationContext};
use volicord_store::{
    bootstrap::{project_record_for_execution, ProjectRecord},
    core_pipeline::{CoreProjectStore, UserJudgmentRecord},
    guards::{
        agent_session, guard_event, guard_health_record, insert_agent_session,
        insert_expected_write, insert_guard_event, insert_prompt_capture, insert_unrecorded_change,
        list_expected_writes_matched_by_post_event, list_pending_expected_writes,
        list_unresolved_unrecorded_changes, mark_expected_write_matched,
        observe_guard_installation, prompt_capture, prompt_capture_availability, unrecorded_change,
        AgentSessionInsert, ExpectedWriteInsert, ExpectedWriteMatch, ExpectedWriteRecord,
        GuardEventInsert, GuardInstallationObservation, PromptCaptureAvailability,
        PromptCaptureInsert, UnrecordedChangeInsert,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    session_watch::{
        create_watch_baseline, latest_watch_baseline_for_session, snapshot_product_repository,
        SessionWatchStatus, WatchBaselineCreate, WatchSnapshotOptions,
    },
    StoreError,
};
use volicord_types::{
    chat_judgment_verification_code, ActorSource, GuardDecision, HostKind, IntegrationProfile,
    JudgmentResolutionOutcome, OperationCategory, PersistedJudgmentBasis,
    PersistedUserJudgmentRequest, ProjectId, PromptCaptureStatus, RequestId,
    SessionWatchCoverageBasis, StatusInclude, StatusRequest, TaskId, ToolEnvelope,
    UserJudgmentOption, UserJudgmentOptionAction, UtcTimestamp,
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING, VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
use crate::user_command::{
    decode_options, record_user_judgment_from_record, select_option, JudgmentRecordingInput,
    UserCommandError,
};

const GUARD_SCHEMA_VERSION: u64 = 1;
const DEFAULT_INTEGRATION_PROFILE: &str = "observe";
const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
const EXPECTED_WRITE_TTL_MINUTES: i64 = 15;
const SESSION_WATCH_METADATA_SOURCE: &str = "volicord_session_watch";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuardPhase {
    SessionStart,
    PreTool,
    PostTool,
    PromptCapture,
    Stop,
}

impl GuardPhase {
    fn event_kind(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::PreTool => "pre_tool",
            Self::PostTool => "post_tool",
            Self::PromptCapture => "prompt_capture",
            Self::Stop => "stop",
        }
    }

    fn command_name(self) -> &'static str {
        match self {
            Self::SessionStart => "session-start",
            Self::PreTool => "pre-tool",
            Self::PostTool => "post-tool",
            Self::PromptCapture => "prompt-capture",
            Self::Stop => "stop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    VolicordJson,
    Text,
    HostNative(HostOutputMode),
}

impl OutputFormat {
    fn default_host_kind(self) -> Option<&'static str> {
        match self {
            Self::HostNative(HostOutputMode::Codex) => Some("codex"),
            Self::HostNative(HostOutputMode::ClaudeCode) => Some("claude_code"),
            Self::VolicordJson | Self::Text => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostOutputMode {
    Codex,
    ClaudeCode,
}

impl HostOutputMode {
    fn from_cli(value: &str) -> Result<Self, GuardCommandError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude-code" | "claude_code" => Ok(Self::ClaudeCode),
            _ => Err(GuardCommandError::Usage(
                "--host-output must be codex or claude-code".to_owned(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardOptions {
    event_file: Option<PathBuf>,
    repo: Option<PathBuf>,
    connection_id: Option<String>,
    session_id: Option<String>,
    guard_installation_id: Option<String>,
    host_kind: Option<String>,
    guard_mode: Option<String>,
    policy_hash: Option<String>,
    output: OutputFormat,
}

impl Default for GuardOptions {
    fn default() -> Self {
        Self {
            event_file: None,
            repo: None,
            connection_id: None,
            session_id: None,
            guard_installation_id: None,
            host_kind: None,
            guard_mode: None,
            policy_hash: None,
            output: OutputFormat::VolicordJson,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedGuardOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Debug, Clone)]
struct GuardInput {
    raw_text: String,
    raw_value: Value,
    raw_sha256: String,
    redacted_value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardEnvelope {
    event_id: String,
    session_id: Option<String>,
    connection_id: String,
    guard_installation_id: Option<String>,
    host_kind: String,
    guard_mode: String,
    occurred_at: String,
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
    current_write_check_ids: Vec<String>,
    stale_write_check_ids: Vec<String>,
    pending_user_judgment_count: usize,
    pending_user_judgments: Vec<GuardPendingJudgmentSummary>,
    active_blocker_count: usize,
    unresolved_unrecorded_change_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardPendingJudgmentSummary {
    chat_id: String,
    verification_code: String,
    judgment_kind: String,
    question: Option<String>,
    answer_instruction: String,
    note_instruction: String,
    options: Vec<GuardPendingJudgmentOptionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardPendingJudgmentOptionSummary {
    selector: String,
    option_id: String,
    label: String,
    machine_action: String,
    resolution_outcome: String,
    instruction: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolObservation {
    tool_name: Option<String>,
    host_invocation_id: Option<String>,
    command: Option<String>,
    classification: ToolClassification,
    paths: Vec<PathAssessment>,
    changed_paths: Vec<PathAssessment>,
    explicit_write_attempt: bool,
    exit_code: Option<i64>,
    success: Option<bool>,
    status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolClassification {
    ReadOnly,
    Mutating,
    UnknownMutationRisk,
    Unknown,
}

impl ToolClassification {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Mutating => "mutating",
            Self::UnknownMutationRisk => "unknown_mutation_risk",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathAssessment {
    raw: String,
    normalized: Option<String>,
    inside_repo: bool,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostToolCorrelation {
    matched_expected_writes: Vec<Value>,
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

pub fn guard_usage() -> String {
    concat!(
        "volicord guard session-start [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|observe] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord guard pre-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|observe] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord guard post-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|observe] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord guard prompt-capture [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|observe] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord guard stop [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|observe] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
    )
    .to_owned()
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
                "unknown guard command: {other}\n\n{}",
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

    let (decision, result, expected_write) = match phase {
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

fn parse_guard_options(args: &[String]) -> Result<GuardOptions, GuardCommandError> {
    let mut options = GuardOptions::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if matches!(token.as_str(), "-h" | "--help" | "help") {
            return Err(GuardCommandError::Usage(guard_usage()));
        }
        if let Some(value) = token.strip_prefix("--file=") {
            set_path_option(&mut options.event_file, "--file", value)?;
        } else if token == "--file" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--file requires a value".to_owned()))?;
            set_path_option(&mut options.event_file, "--file", value)?;
        } else if let Some(value) = token.strip_prefix("--repo=") {
            set_path_option(&mut options.repo, "--repo", value)?;
        } else if token == "--repo" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--repo requires a value".to_owned()))?;
            set_path_option(&mut options.repo, "--repo", value)?;
        } else if let Some(value) = token.strip_prefix("--connection=") {
            set_string_option(&mut options.connection_id, "--connection", value)?;
        } else if token == "--connection" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--connection requires a value".to_owned())
            })?;
            set_string_option(&mut options.connection_id, "--connection", value)?;
        } else if let Some(value) = token.strip_prefix("--session=") {
            set_string_option(&mut options.session_id, "--session", value)?;
        } else if token == "--session" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--session requires a value".to_owned()))?;
            set_string_option(&mut options.session_id, "--session", value)?;
        } else if let Some(value) = token.strip_prefix("--guard-installation=") {
            set_string_option(
                &mut options.guard_installation_id,
                "--guard-installation",
                value,
            )?;
        } else if token == "--guard-installation" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--guard-installation requires a value".to_owned())
            })?;
            set_string_option(
                &mut options.guard_installation_id,
                "--guard-installation",
                value,
            )?;
        } else if let Some(value) = token.strip_prefix("--host=") {
            set_string_option(&mut options.host_kind, "--host", value)?;
        } else if token == "--host" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--host requires a value".to_owned()))?;
            set_string_option(&mut options.host_kind, "--host", value)?;
        } else if let Some(value) = token.strip_prefix("--integration-profile=") {
            set_string_option(&mut options.guard_mode, "--integration-profile", value)?;
        } else if token == "--integration-profile" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--integration-profile requires a value".to_owned())
            })?;
            set_string_option(&mut options.guard_mode, "--integration-profile", value)?;
        } else if let Some(value) = token.strip_prefix("--policy-hash=") {
            set_string_option(&mut options.policy_hash, "--policy-hash", value)?;
        } else if token == "--policy-hash" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--policy-hash requires a value".to_owned())
            })?;
            set_string_option(&mut options.policy_hash, "--policy-hash", value)?;
        } else if token == "--text" {
            options.output = OutputFormat::Text;
        } else if token == "--json" {
            options.output = OutputFormat::VolicordJson;
        } else if let Some(value) = token.strip_prefix("--output=") {
            options.output = parse_output_format(value)?;
        } else if token == "--output" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--output requires a value".to_owned()))?;
            options.output = parse_output_format(value)?;
        } else if let Some(value) = token.strip_prefix("--host-output=") {
            options.output = OutputFormat::HostNative(HostOutputMode::from_cli(value)?);
        } else if token == "--host-output" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--host-output requires a value".to_owned())
            })?;
            options.output = OutputFormat::HostNative(HostOutputMode::from_cli(value)?);
        } else if token.starts_with('-') {
            return Err(GuardCommandError::Usage(format!("unknown option: {token}")));
        } else {
            return Err(GuardCommandError::Usage(format!(
                "unexpected argument: {token}"
            )));
        }
        index += 1;
    }
    Ok(options)
}

fn parse_output_format(value: &str) -> Result<OutputFormat, GuardCommandError> {
    match value {
        "volicord-json" | "volicord_json" | "json" => Ok(OutputFormat::VolicordJson),
        "text" => Ok(OutputFormat::Text),
        other => Err(GuardCommandError::Usage(format!(
            "unsupported --output value: {other}"
        ))),
    }
}

fn set_path_option(
    slot: &mut Option<PathBuf>,
    option: &'static str,
    value: &str,
) -> Result<(), GuardCommandError> {
    if slot.is_some() {
        return Err(GuardCommandError::Usage(format!(
            "{option} was supplied more than once"
        )));
    }
    if value.trim().is_empty() {
        return Err(GuardCommandError::Usage(format!(
            "{option} requires a non-empty value"
        )));
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_string_option(
    slot: &mut Option<String>,
    option: &'static str,
    value: &str,
) -> Result<(), GuardCommandError> {
    if slot.is_some() {
        return Err(GuardCommandError::Usage(format!(
            "{option} was supplied more than once"
        )));
    }
    if value.trim().is_empty() {
        return Err(GuardCommandError::Usage(format!(
            "{option} requires a non-empty value"
        )));
    }
    *slot = Some(value.to_owned());
    Ok(())
}

fn read_guard_input(path: Option<&Path>) -> Result<GuardInput, GuardCommandError> {
    let raw_text = match path {
        Some(path) => fs::read_to_string(path).map_err(|error| {
            GuardCommandError::Runtime(format!(
                "failed to read guard event file {}: {error}",
                path.display()
            ))
        })?,
        None => {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text).map_err(|error| {
                GuardCommandError::Runtime(format!("failed to read guard event stdin: {error}"))
            })?;
            text
        }
    };
    if raw_text.trim().is_empty() {
        return Err(GuardCommandError::Usage(
            "guard event JSON must not be empty".to_owned(),
        ));
    }
    let raw_value = serde_json::from_str::<Value>(&raw_text)
        .map_err(|error| GuardCommandError::Usage(format!("guard event must be JSON: {error}")))?;
    let raw_sha256 = sha256_text(&raw_text);
    let redacted_value = redact_event_value(&raw_value);
    Ok(GuardInput {
        raw_text,
        raw_value,
        raw_sha256,
        redacted_value,
    })
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

fn event_path_field<'a>(event: &'a Value, paths: &[&[&str]]) -> Option<&'a Path> {
    for path in paths {
        if let Some(value) = value_at(event, path).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return Some(Path::new(value));
            }
        }
    }
    None
}

fn guard_envelope(
    phase: GuardPhase,
    options: &GuardOptions,
    input: &GuardInput,
    project: &ProjectRecord,
) -> Result<GuardEnvelope, GuardCommandError> {
    let connection_id = options
        .connection_id
        .clone()
        .or_else(|| {
            event_string(
                &input.raw_value,
                &[
                    &["connection_id"],
                    &["connection_internal_id"],
                    &["connection", "id"],
                    &["volicord", "connection_id"],
                ],
            )
        })
        .ok_or_else(|| {
            GuardCommandError::Usage(
                "guard command requires --connection or connection_id in the event".to_owned(),
            )
        })?;
    let host_kind = normalize_host_kind(
        options
            .host_kind
            .clone()
            .or_else(|| {
                event_string(
                    &input.raw_value,
                    &[
                        &["host_kind"],
                        &["host", "kind"],
                        &["source", "host_kind"],
                        &["source", "host"],
                    ],
                )
            })
            .or_else(|| options.output.default_host_kind().map(str::to_owned))
            .unwrap_or_else(|| "generic".to_owned()),
    )?;
    let guard_mode = normalize_guard_mode(
        options
            .guard_mode
            .clone()
            .or_else(|| {
                event_string(
                    &input.raw_value,
                    &[
                        &["integration_profile"],
                        &["profile"],
                        &["guard", "profile"],
                    ],
                )
            })
            .unwrap_or_else(|| DEFAULT_INTEGRATION_PROFILE.to_owned()),
    )?;
    let session_id = options.session_id.clone().or_else(|| {
        event_string(
            &input.raw_value,
            &[
                &["session_id"],
                &["session", "id"],
                &["conversation_id"],
                &["transcript_id"],
            ],
        )
    });
    let session_id = match (phase, session_id) {
        (GuardPhase::SessionStart | GuardPhase::PromptCapture, None) => Some(stable_id(
            "agent_session",
            &[
                phase.command_name(),
                &connection_id,
                &project.project_id,
                &input.raw_sha256,
            ],
        )),
        (_, value) => value,
    };
    let event_id = event_string(
        &input.raw_value,
        &[
            &["guard_event_id"],
            &["event_id"],
            &["hook_event_id"],
            &["tool_call_id"],
            &["id"],
        ],
    )
    .unwrap_or_else(|| {
        stable_id(
            "guard_event",
            &[
                phase.command_name(),
                &connection_id,
                session_id.as_deref().unwrap_or(""),
                &project.project_id,
                &input.raw_sha256,
            ],
        )
    });
    let occurred_at = event_string(
        &input.raw_value,
        &[&["occurred_at"], &["timestamp"], &["time"]],
    )
    .unwrap_or_else(current_timestamp);
    Ok(GuardEnvelope {
        event_id,
        session_id,
        connection_id,
        guard_installation_id: options.guard_installation_id.clone().or_else(|| {
            event_string(
                &input.raw_value,
                &[
                    &["guard_installation_id"],
                    &["guard", "installation_id"],
                    &["volicord", "guard_installation_id"],
                ],
            )
        }),
        host_kind,
        guard_mode,
        occurred_at,
    })
}

fn normalize_host_kind(value: String) -> Result<String, GuardCommandError> {
    let normalized = match value.as_str() {
        "claude-code" => "claude_code".to_owned(),
        other => other.to_owned(),
    };
    HostKind::from_str(&normalized).map_err(|error| GuardCommandError::Usage(error.to_string()))?;
    Ok(normalized)
}

fn normalize_guard_mode(value: String) -> Result<String, GuardCommandError> {
    if matches!(
        value.as_str(),
        profile if profile == IntegrationProfile::Record.as_str()
            || profile == IntegrationProfile::Observe.as_str()
    ) {
        Ok(value)
    } else {
        Err(GuardCommandError::Usage(
            "integration profile must be record or observe".to_owned(),
        ))
    }
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
                    "source": "volicord_guard_cli",
                    "schema_version": GUARD_SCHEMA_VERSION
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
        || envelope.guard_mode != IntegrationProfile::Observe.as_str()
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
            "failed to start observe session watcher for {}: {error}",
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
                "schema_version": 1,
                "source": SESSION_WATCH_METADATA_SOURCE,
                "status_detail": "active",
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true,
                "coverage_start_at": started_at,
                "coverage_basis": SessionWatchCoverageBasis::McpStart.as_str(),
                "coverage_started_by": "session_start_hook"
            })
            .to_string(),
        },
    )?;
    Ok(())
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
                "failed to read guard policy {}: {error}",
                policy_path.display()
            )));
        }
    };
    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
        GuardCommandError::Runtime(format!(
            "guard policy is not valid JSON: {} ({error})",
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
    let mut current_write_check_ids = Vec::new();
    let mut stale_write_check_ids = Vec::new();
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
        for record in store.active_write_checks(&task_id)? {
            let current_basis = record.basis_state_version == project_state.state_version;
            let not_expired = UtcTimestamp::parse(&record.expires_at)
                .map(|expires_at| now_timestamp < expires_at)
                .unwrap_or(false);
            if current_basis && not_expired {
                current_write_check_ids.push(record.write_check_id);
            } else {
                stale_write_check_ids.push(record.write_check_id);
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
        current_write_check_ids,
        stale_write_check_ids,
        pending_user_judgment_count,
        pending_user_judgments,
        active_blocker_count,
        unresolved_unrecorded_change_count,
    })
}

fn tool_observation(event: &Value, repo_root: &Path) -> ToolObservation {
    let tool_name = event_string(
        event,
        &[
            &["tool_name"],
            &["tool", "name"],
            &["tool_use", "name"],
            &["tool"],
        ],
    );
    let command = event_string(
        event,
        &[
            &["command"],
            &["tool_input", "command"],
            &["input", "command"],
            &["tool", "input", "command"],
            &["tool", "arguments", "command"],
            &["tool_use", "input", "command"],
        ],
    );
    let classification = classify_tool(tool_name.as_deref(), command.as_deref());
    let paths = collect_path_assessments(event, repo_root, false);
    let changed_paths = collect_path_assessments(event, repo_root, true);
    let explicit_write_attempt = event_bool(
        event,
        &[
            &["product_file_write_intended"],
            &["write_attempt"],
            &["mutates_files"],
            &["tool_input", "product_file_write_intended"],
            &["tool_input", "write_attempt"],
            &["input", "product_file_write_intended"],
            &["input", "write_attempt"],
        ],
    )
    .unwrap_or(false);
    ToolObservation {
        tool_name,
        host_invocation_id: host_invocation_id(event),
        command,
        classification,
        paths,
        changed_paths,
        explicit_write_attempt,
        exit_code: event_i64(
            event,
            &[
                &["exit_code"],
                &["tool_result", "exit_code"],
                &["result", "exit_code"],
                &["output", "exit_code"],
            ],
        ),
        success: event_bool(
            event,
            &[
                &["success"],
                &["tool_result", "success"],
                &["result", "success"],
                &["output", "success"],
            ],
        ),
        status: event_string(
            event,
            &[
                &["status"],
                &["tool_result", "status"],
                &["result", "status"],
                &["output", "status"],
            ],
        ),
    }
}

fn classify_tool(tool_name: Option<&str>, command: Option<&str>) -> ToolClassification {
    let normalized_tool = tool_name.unwrap_or("").trim().to_ascii_lowercase();
    if matches!(
        normalized_tool.as_str(),
        "read" | "view" | "grep" | "search" | "list" | "glob"
    ) {
        return ToolClassification::ReadOnly;
    }
    if matches!(
        normalized_tool.as_str(),
        "edit" | "write" | "write_file" | "apply_patch" | "patch" | "notebook_edit"
    ) {
        return ToolClassification::Mutating;
    }
    let Some(command) = command.map(str::trim).filter(|value| !value.is_empty()) else {
        return if normalized_tool.is_empty() {
            ToolClassification::Unknown
        } else {
            ToolClassification::UnknownMutationRisk
        };
    };
    if shell_command_is_clearly_mutating(command) {
        return ToolClassification::Mutating;
    }
    if shell_command_is_read_only(command) {
        return ToolClassification::ReadOnly;
    }
    ToolClassification::UnknownMutationRisk
}

fn shell_command_is_clearly_mutating(command: &str) -> bool {
    let compact = format!(" {command} ");
    if compact.contains(" > ") || compact.contains(" >> ") || compact.contains(" tee ") {
        return true;
    }
    if compact.contains(" sed -i ")
        || compact.contains(" perl -pi ")
        || compact.contains(" git add ")
        || compact.contains(" git commit ")
        || compact.contains(" git reset ")
        || compact.contains(" git clean ")
        || compact.contains(" git checkout ")
        || compact.contains(" git switch ")
    {
        return true;
    }
    command_segments(command).iter().any(|segment| {
        let first = first_command_word(segment);
        matches!(
            first.as_deref(),
            Some(
                "rm" | "mv"
                    | "cp"
                    | "touch"
                    | "mkdir"
                    | "rmdir"
                    | "ln"
                    | "chmod"
                    | "chown"
                    | "truncate"
                    | "install"
                    | "cargo-fmt"
            )
        ) || segment.trim_start().starts_with("cargo fmt")
            || segment.trim_start().starts_with("npm install")
            || segment.trim_start().starts_with("pnpm install")
            || segment.trim_start().starts_with("yarn install")
    })
}

fn shell_command_is_read_only(command: &str) -> bool {
    command_segments(command).iter().all(|segment| {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            return true;
        }
        if trimmed.contains(" -delete") {
            return false;
        }
        let first = first_command_word(trimmed);
        matches!(
            first.as_deref(),
            Some(
                "pwd"
                    | "ls"
                    | "cat"
                    | "rg"
                    | "grep"
                    | "find"
                    | "wc"
                    | "head"
                    | "tail"
                    | "sed"
                    | "awk"
                    | "git"
                    | "cargo"
                    | "npm"
                    | "pnpm"
                    | "yarn"
                    | "node"
                    | "rustc"
            )
        ) && !trimmed.starts_with("cargo fmt")
            && !trimmed.starts_with("npm install")
            && !trimmed.starts_with("pnpm install")
            && !trimmed.starts_with("yarn install")
            && !trimmed.starts_with("git add")
            && !trimmed.starts_with("git commit")
            && !trimmed.starts_with("git reset")
            && !trimmed.starts_with("git clean")
            && !trimmed.starts_with("git checkout")
            && !trimmed.starts_with("git switch")
    })
}

fn command_segments(command: &str) -> Vec<&str> {
    command
        .split([';', '\n'])
        .flat_map(|part| part.split("&&"))
        .flat_map(|part| part.split("||"))
        .collect()
}

fn first_command_word(segment: &str) -> Option<String> {
    let mut words = segment.split_whitespace();
    let first = words.next()?;
    if first == "sudo" || first == "command" {
        words.next().map(str::to_owned)
    } else {
        Some(first.to_owned())
    }
}

fn collect_path_assessments(
    event: &Value,
    repo_root: &Path,
    changed_only: bool,
) -> Vec<PathAssessment> {
    let mut raw_paths = BTreeSet::new();
    collect_paths_recursive(event, changed_only, &mut raw_paths);
    if !changed_only {
        if let Some(command) = event_string(
            event,
            &[
                &["command"],
                &["tool_input", "command"],
                &["input", "command"],
                &["tool", "input", "command"],
            ],
        ) {
            raw_paths.extend(paths_from_redirection(&command));
        }
    }
    raw_paths
        .into_iter()
        .map(|raw| assess_path(repo_root, &raw))
        .collect()
}

fn collect_paths_recursive(value: &Value, changed_only: bool, paths: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let path_key = if changed_only {
                    matches!(
                        key.as_str(),
                        "changed_paths" | "observed_paths" | "modified_paths"
                    )
                } else {
                    matches!(
                        key.as_str(),
                        "paths"
                            | "path"
                            | "file_path"
                            | "target_path"
                            | "changed_paths"
                            | "observed_paths"
                            | "modified_paths"
                    )
                };
                if path_key {
                    collect_string_values(value, paths);
                }
                collect_paths_recursive(value, changed_only, paths);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_paths_recursive(value, changed_only, paths);
            }
        }
        _ => {}
    }
}

fn collect_string_values(value: &Value, values: &mut BTreeSet<String>) {
    match value {
        Value::String(text) if !text.trim().is_empty() => {
            values.insert(text.to_owned());
        }
        Value::Array(items) => {
            for item in items {
                collect_string_values(item, values);
            }
        }
        _ => {}
    }
}

fn paths_from_redirection(command: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let words = command.split_whitespace().collect::<Vec<_>>();
    for (index, word) in words.iter().enumerate() {
        if matches!(*word, ">" | ">>") {
            if let Some(path) = words.get(index + 1) {
                paths.push(path.trim_matches('"').trim_matches('\'').to_owned());
            }
        }
    }
    paths
}

fn assess_path(repo_root: &Path, raw: &str) -> PathAssessment {
    let path = Path::new(raw);
    let (inside_repo, normalized) = if path.is_absolute() {
        match path.strip_prefix(repo_root) {
            Ok(relative) => normalized_relative_path(relative)
                .map(|path| (true, Some(path)))
                .unwrap_or((false, None)),
            Err(_) => (false, None),
        }
    } else {
        normalized_relative_path(path)
            .map(|path| (true, Some(path)))
            .unwrap_or((false, None))
    };
    PathAssessment {
        raw: raw.to_owned(),
        normalized,
        inside_repo,
    }
}

fn normalized_relative_path(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
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
        } else if summary.current_write_check_ids.is_empty() {
            reasons.push(GuardReason {
                code: "write_readiness_missing",
                message: "The current task does not have a current active Write Check.".to_owned(),
                severity: "deny",
            });
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
    if summary.current_write_check_ids.is_empty() {
        return Ok(None);
    }
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
            &summary.current_write_check_ids.join("|"),
        ],
    );
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
            write_check_ids_json: serde_json::to_string(&summary.current_write_check_ids)
                .map_err(json_error)?,
            basis_state_version: summary.state_version,
            created_at: format_timestamp(created_at),
            expires_at: format_timestamp(expires_at),
            metadata_json: json!({
                "source": "volicord_guard_pre_tool",
                "schema_version": GUARD_SCHEMA_VERSION,
                "raw_event_sha256": input.raw_sha256
            })
            .to_string(),
        },
        expected_paths,
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
        "write_check_ids": candidate.insert.write_check_ids_json
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

fn host_invocation_id(event: &Value) -> Option<String> {
    event_string(
        event,
        &[
            &["tool_call_id"],
            &["tool_use_id"],
            &["tool_invocation_id"],
            &["invocation_id"],
            &["call_id"],
            &["tool", "call_id"],
            &["tool", "id"],
            &["tool_use", "id"],
            &["tool_result", "tool_call_id"],
            &["result", "tool_call_id"],
        ],
    )
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
            unrecorded_changes: Vec::new(),
        });
    }
    let changed = normalized_observed_paths(observation.changed_paths.iter());
    if changed.is_empty() {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
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
                unrecorded_changes: Vec::new(),
            })
        }
        ExpectedWriteMatchOutcome::AlreadyMatched(record) => Ok(PostToolCorrelation {
            matched_expected_writes: vec![matched_expected_write_json(&record, &changed)],
            unrecorded_changes: Vec::new(),
        }),
        ExpectedWriteMatchOutcome::NoCandidates => Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
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
        ExpectedWriteMatchOutcome::OutOfScope(candidate_ids) => Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
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
                "candidate_expected_write_ids": context.candidate_expected_write_ids
            })
            .to_string(),
            detected_at: context.envelope.occurred_at.clone(),
            metadata_json: json!({
                "guard_event_id": context.envelope.event_id,
                "schema_version": GUARD_SCHEMA_VERSION
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

    let host_invocation_id = host_invocation_id_from_observation(observation);
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

fn host_invocation_id_from_observation(observation: &ToolObservation) -> Option<String> {
    observation.host_invocation_id.clone()
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
        "write_check_ids": serde_json::from_str::<Value>(&record.write_check_ids_json)
            .unwrap_or_else(|_| json!([]))
    })
}

fn prompt_capture_availability_for_event(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> Result<PromptCaptureAvailability, GuardCommandError> {
    let record = guard_health_record(runtime_home, &project.project_id, &envelope.connection_id)?;
    let mut availability = prompt_capture_availability(&record)?;
    let Some(installation) = record.guard_installation.as_ref() else {
        return Ok(availability);
    };
    if envelope
        .guard_installation_id
        .as_deref()
        .is_some_and(|id| id != installation.guard_installation_id)
        || installation.connection_internal_id != envelope.connection_id
        || installation.host_kind != envelope.host_kind
        || installation.guard_mode != envelope.guard_mode
        || installation.project_id.as_deref() != Some(project.project_id.as_str())
    {
        availability.status = PromptCaptureStatus::Unavailable;
        return Ok(availability);
    }
    let expected_policy_hash = expected_policy_hash(&installation.host_capability_json)?;
    match (
        current_policy_hash(project)?,
        expected_policy_hash.as_deref(),
    ) {
        (Some(current), Some(expected)) if current == expected => {}
        (Some(_), Some(_)) => availability.status = PromptCaptureStatus::ReloadRequired,
        (None, Some(_)) => availability.status = PromptCaptureStatus::NotConfigured,
        _ => {}
    }
    Ok(availability)
}

fn expected_policy_hash(host_capability_json: &str) -> Result<Option<String>, GuardCommandError> {
    let value = serde_json::from_str::<Value>(host_capability_json).map_err(json_error)?;
    Ok(value
        .get("policy_hash")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned))
}

fn prompt_capture_unavailable_result(
    availability: &PromptCaptureAvailability,
) -> (GuardDecision, Value, bool) {
    let (code, message, next_action) = prompt_capture_unavailable_reason(availability.status);
    (
        GuardDecision::Deny,
        json!({
            "decision": GuardDecision::Deny.as_str(),
            "allowed": false,
            "prompt_capture": {
                "captured": false,
                "reason": code,
                "prompt_capture_status": availability.status.as_str(),
                "host_supports_prompt_capture": availability.host_supports_prompt_capture,
                "prompt_capture_configured": availability.prompt_capture_configured,
                "next_action": next_action
            },
            "recognized_judgment_command": null,
            "reasons": [{
                "code": code,
                "message": message,
                "severity": "deny",
                "next_action": next_action
            }],
            "next_action": next_action,
            "model_context": format!("Volicord did not record a user judgment: {message}"),
            "enforcement_level": "cooperative_detective"
        }),
        true,
    )
}

fn prompt_capture_unavailable_reason(
    status: PromptCaptureStatus,
) -> (&'static str, String, &'static str) {
    match status {
        PromptCaptureStatus::UnsupportedByHost => (
            "prompt_capture_unsupported",
            "This host does not support user prompt-submit hooks.".to_owned(),
            "Use MCP elicitation if available; otherwise use the local volicord user command as the recovery path.",
        ),
        PromptCaptureStatus::NotConfigured => (
            "prompt_capture_not_configured",
            "Prompt capture is not configured for this host, project, and connection.".to_owned(),
            "Configure a host prompt-capture hook, or use the local volicord user command as the recovery path.",
        ),
        PromptCaptureStatus::ReloadRequired => (
            "prompt_capture_reload_required",
            "Prompt capture configuration is installed but the host must reload the current policy.".to_owned(),
            "Restart or reload the host before using prompt-capture chat commands.",
        ),
        PromptCaptureStatus::Degraded => (
            "prompt_capture_degraded",
            "Prompt capture is degraded for this host, project, and connection.".to_owned(),
            "Repair the guard integration before using prompt-capture chat commands.",
        ),
        _ => (
            "prompt_capture_unavailable",
            "Prompt capture is unavailable for this host, project, and connection.".to_owned(),
            "Use MCP elicitation if available; otherwise use the local volicord user command as the recovery path.",
        ),
    }
}

fn record_prompt_capture(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<Value, GuardCommandError> {
    let Some(prompt) = extract_prompt_text(&input.raw_value) else {
        return Ok(json!({
            "captured": false,
            "reason": "no_prompt_text"
        }));
    };
    let session_id = envelope.session_id.as_ref().ok_or_else(|| {
        GuardCommandError::Runtime("prompt capture requires a session id".to_owned())
    })?;
    let prompt_sha256 = sha256_text(&prompt);
    let capture_id = event_string(
        &input.raw_value,
        &[&["prompt_capture_id"], &["capture_id"], &["id"]],
    )
    .unwrap_or_else(|| stable_id("prompt_capture", &[session_id, &prompt_sha256]));
    if prompt_capture(runtime_home, &project.project_id, &capture_id)?.is_none() {
        insert_prompt_capture(
            runtime_home,
            &project.project_id,
            PromptCaptureInsert {
                prompt_capture_id: capture_id.clone(),
                session_id: session_id.clone(),
                connection_internal_id: envelope.connection_id.clone(),
                capture_kind: event_string(&input.raw_value, &[&["capture_kind"]])
                    .unwrap_or_else(|| "user_prompt".to_owned()),
                prompt_sha256: prompt_sha256.clone(),
                prompt_text: None,
                captured_at: envelope.occurred_at.clone(),
                metadata_json: json!({
                    "source": "volicord_guard_prompt_capture",
                    "raw_event_sha256": input.raw_sha256,
                    "prompt_size_bytes": prompt.len(),
                    "prompt_text_omitted": true,
                    "schema_version": GUARD_SCHEMA_VERSION
                })
                .to_string(),
            },
        )?;
    }
    Ok(json!({
        "captured": true,
        "prompt_capture_id": capture_id,
        "prompt_sha256": prompt_sha256,
        "prompt_text_omitted": true
    }))
}

fn handle_prompt_capture(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<(GuardDecision, Value, bool), GuardCommandError> {
    let availability = prompt_capture_availability_for_event(runtime_home, project, envelope)?;
    if !availability.can_use_chat_commands() {
        return Ok(prompt_capture_unavailable_result(&availability));
    }
    let capture = record_prompt_capture(runtime_home, project, envelope, input)?;
    let command = extract_prompt_text(&input.raw_value)
        .map(|prompt| parse_prompt_judgment_command(&prompt))
        .unwrap_or(PromptCommandDetection::NoCommand);

    match command {
        PromptCommandDetection::NoCommand => Ok((
            GuardDecision::Allow,
            json!({
                "decision": GuardDecision::Allow.as_str(),
                "allowed": true,
                "prompt_capture": capture,
                "recognized_judgment_command": null,
                "model_context": null,
                "enforcement_level": "cooperative_detective"
            }),
            false,
        )),
        PromptCommandDetection::Blocked(block) => Ok(prompt_capture_blocked_result(capture, block)),
        PromptCommandDetection::Command(command) => {
            if let Some(event_project_id) = event_project_id(&input.raw_value) {
                if event_project_id != project.project_id {
                    return Ok(prompt_capture_blocked_result(
                        capture,
                        PromptCommandBlock {
                            code: "project_mismatch",
                            message: format!(
                                "Volicord judgment command targeted project `{event_project_id}`, but this prompt hook is bound to `{}`.",
                                project.project_id
                            ),
                        },
                    ));
                }
            }
            match record_prompt_judgment_command(runtime_home, project, envelope, command) {
                Ok(recorded) => Ok((
                    GuardDecision::InjectContext,
                    json!({
                        "decision": GuardDecision::InjectContext.as_str(),
                        "allowed": true,
                        "prompt_capture": capture,
                        "recognized_judgment_command": {
                            "command_kind": recorded.command_kind,
                            "chat_id": recorded.chat_id,
                            "verification_code": recorded.verification_code,
                            "selected_option_id": recorded.selected_option_id,
                            "machine_action": recorded.machine_action,
                            "resolution_outcome": recorded.resolution_outcome,
                            "note_text_omitted": recorded.note_text_omitted
                        },
                        "model_context": recorded.model_context,
                        "enforcement_level": "cooperative_detective"
                    }),
                    false,
                )),
                Err(block) => Ok(prompt_capture_blocked_result(capture, block)),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PromptCommandDetection {
    NoCommand,
    Command(PromptJudgmentCommand),
    Blocked(PromptCommandBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PromptJudgmentCommand {
    Answer {
        chat_id: String,
        answer_selector: String,
        verification_code: String,
    },
    Note {
        chat_id: String,
        note: String,
        verification_code: String,
    },
}

impl PromptJudgmentCommand {
    fn chat_id(&self) -> &str {
        match self {
            Self::Answer { chat_id, .. } | Self::Note { chat_id, .. } => chat_id,
        }
    }

    fn verification_code(&self) -> &str {
        match self {
            Self::Answer {
                verification_code, ..
            }
            | Self::Note {
                verification_code, ..
            } => verification_code,
        }
    }

    fn command_kind(&self) -> &'static str {
        match self {
            Self::Answer { .. } => "answer",
            Self::Note { .. } => "note",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptCommandBlock {
    code: &'static str,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedPromptJudgment {
    command_kind: &'static str,
    chat_id: String,
    verification_code: String,
    selected_option_id: String,
    machine_action: String,
    resolution_outcome: String,
    note_text_omitted: bool,
    model_context: String,
}

fn parse_prompt_judgment_command(prompt: &str) -> PromptCommandDetection {
    let command_lines = prompt
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("Volicord:").map(str::trim))
        .collect::<Vec<_>>();
    if command_lines.is_empty() {
        return PromptCommandDetection::NoCommand;
    }

    let mut parsed = Vec::new();
    for line in command_lines {
        match parse_prompt_judgment_command_line(line) {
            Ok(command) => parsed.push(command),
            Err(message) => {
                return PromptCommandDetection::Blocked(PromptCommandBlock {
                    code: "malformed_judgment_command",
                    message,
                })
            }
        }
    }

    let Some(first) = parsed.first().cloned() else {
        return PromptCommandDetection::NoCommand;
    };
    if parsed.len() > 1 {
        return PromptCommandDetection::Blocked(PromptCommandBlock {
            code: "ambiguous_judgment_command",
            message: "Multiple Volicord judgment commands were found; send exactly one command."
                .to_owned(),
        });
    }
    PromptCommandDetection::Command(first)
}

fn parse_prompt_judgment_command_line(line: &str) -> Result<PromptJudgmentCommand, String> {
    let Some((action, rest)) = split_once_whitespace(line) else {
        return Err(
            "Volicord judgment commands must be `answer J-N OPTION #CODE` or `note J-N \"text\" #CODE`."
                .to_owned(),
        );
    };
    match action {
        "answer" => {
            let parts = rest.split_whitespace().collect::<Vec<_>>();
            if parts.len() == 2 {
                return Err(
                    "Volicord answer commands must include the displayed verification code."
                        .to_owned(),
                );
            }
            if parts.len() != 3 {
                return Err(
                    "Volicord answer commands must be exactly `Volicord: answer J-N OPTION #CODE`."
                        .to_owned(),
                );
            }
            validate_chat_id(parts[0])?;
            if parts[1].trim().is_empty() || parts[1].starts_with('"') {
                return Err("Volicord answer option must be a number or option id.".to_owned());
            }
            let verification_code = normalize_verification_code(parts[2])?;
            Ok(PromptJudgmentCommand::Answer {
                chat_id: parts[0].to_owned(),
                answer_selector: parts[1].to_owned(),
                verification_code,
            })
        }
        "note" => {
            let Some((chat_id, note_text)) = split_once_whitespace(rest) else {
                return Err(
                    "Volicord note commands must be exactly `Volicord: note J-N \"text\" #CODE`."
                        .to_owned(),
                );
            };
            validate_chat_id(chat_id)?;
            let (note, verification_code) = parse_quoted_note_and_code(note_text)?;
            Ok(PromptJudgmentCommand::Note {
                chat_id: chat_id.to_owned(),
                note,
                verification_code,
            })
        }
        _ => Err(
            "Volicord judgment commands must start with `answer` or `note` after `Volicord:`."
                .to_owned(),
        ),
    }
}

fn split_once_whitespace(value: &str) -> Option<(&str, &str)> {
    let trimmed = value.trim();
    let split_at = trimmed.find(char::is_whitespace)?;
    let (first, rest) = trimmed.split_at(split_at);
    Some((first, rest.trim_start()))
}

fn validate_chat_id(value: &str) -> Result<(), String> {
    parse_chat_id(value)
        .map(|_| ())
        .map_err(|message| message.message)
}

fn normalize_verification_code(value: &str) -> Result<String, String> {
    let Some(raw) = value.strip_prefix('#') else {
        return Err("Volicord verification code must start with `#`.".to_owned());
    };
    if raw.len() < 4 || raw.len() > 16 || !raw.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err("Volicord verification code must be the displayed `#CODE` token.".to_owned());
    }
    Ok(format!("#{}", raw.to_ascii_uppercase()))
}

fn parse_quoted_note_and_code(value: &str) -> Result<(String, String), String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('"') {
        return Err("Volicord note text must be a double-quoted string.".to_owned());
    }
    let mut output = String::new();
    let mut chars = trimmed[1..].chars();
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                '"' | '\\' => output.push(ch),
                'n' => output.push('\n'),
                't' => output.push('\t'),
                other => {
                    output.push('\\');
                    output.push(other);
                }
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                let rest = chars.as_str().trim();
                if rest.is_empty() {
                    return Err(
                        "Volicord note commands must include the displayed verification code."
                            .to_owned(),
                    );
                }
                if rest.split_whitespace().count() == 1 {
                    let verification_code = normalize_verification_code(rest)?;
                    return Ok((output, verification_code));
                }
                return Err(
                    "Volicord note commands accept only the verification code after the closing quote."
                        .to_owned(),
                );
            }
            other => output.push(other),
        }
    }
    Err("Volicord note text is missing a closing double quote.".to_owned())
}

fn prompt_capture_blocked_result(
    capture: Value,
    block: PromptCommandBlock,
) -> (GuardDecision, Value, bool) {
    (
        GuardDecision::Deny,
        json!({
            "decision": GuardDecision::Deny.as_str(),
            "allowed": false,
            "prompt_capture": capture,
            "recognized_judgment_command": null,
            "reasons": [{
                "code": block.code,
                "message": block.message,
                "severity": "deny"
            }],
            "model_context": format!("Volicord did not record a user judgment: {}", block.message),
            "enforcement_level": "cooperative_detective"
        }),
        true,
    )
}

fn record_prompt_judgment_command(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    command: PromptJudgmentCommand,
) -> Result<RecordedPromptJudgment, PromptCommandBlock> {
    let store = CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id))
        .map_err(prompt_block_from_store_error)?;
    let project_state = store
        .project_state()
        .map_err(prompt_block_from_store_error)?;
    let Some(active_task_id) = project_state.active_task_id.as_deref() else {
        return Err(PromptCommandBlock {
            code: "no_active_task",
            message: "No active Volicord task is selected for this prompt-capture session."
                .to_owned(),
        });
    };
    let task_id = TaskId::new(active_task_id);
    let (record, chat_index) = select_chat_judgment(&store, &task_id, command.chat_id(), envelope)?;
    let expected_code = judgment_verification_code(&record, envelope);
    if command.verification_code() != expected_code {
        return Err(PromptCommandBlock {
            code: "wrong_verification_code",
            message: format!(
                "Volicord judgment `{}` requires the current displayed verification code.",
                command.chat_id()
            ),
        });
    }
    if record.status == "pending" && judgment_code_is_expired(&record, envelope)? {
        return Err(PromptCommandBlock {
            code: "expired_verification_code",
            message: format!(
                "Volicord judgment `{}` has an expired verification code; refresh the pending judgment instructions.",
                command.chat_id()
            ),
        });
    }
    let options = decode_options(&record).map_err(prompt_block_from_user_error)?;
    let selected_option = match &command {
        PromptJudgmentCommand::Answer {
            answer_selector, ..
        } => select_option(&options, answer_selector).map_err(prompt_block_from_user_error)?,
        PromptJudgmentCommand::Note { .. } => select_defer_option(&options)?,
    };
    let note = match &command {
        PromptJudgmentCommand::Answer { .. } => None,
        PromptJudgmentCommand::Note { note, .. } => Some(note.clone()),
    };
    let replay_id = prompt_judgment_replay_id(&record, envelope);
    let expected_state_version = judgment_expected_state_version(&record)?;
    let response = record_user_judgment_from_record(JudgmentRecordingInput {
        runtime_home,
        project_id: &project.project_id,
        expected_state_version: Some(expected_state_version),
        record: &record,
        selected_option: &selected_option,
        note,
        verification_basis: VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
        request_id: Some(format!("req_{replay_id}")),
        idempotency_key: Some(format!("idem_{replay_id}")),
    })
    .map_err(prompt_block_from_user_error)?;
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return Err(prompt_block_from_record_response(&response.response_value));
    }
    let chat_id = chat_id_for_index(chat_index);
    let resolution_outcome = outcome_value(selected_option.resolution_outcome).to_owned();
    Ok(RecordedPromptJudgment {
        command_kind: command.command_kind(),
        chat_id: chat_id.clone(),
        verification_code: expected_code,
        selected_option_id: selected_option.option_id.as_str().to_owned(),
        machine_action: machine_action_value(selected_option.machine_action).to_owned(),
        resolution_outcome: resolution_outcome.clone(),
        note_text_omitted: matches!(command, PromptJudgmentCommand::Note { .. }),
        model_context: format!(
            "Volicord recorded the user-owned judgment for {chat_id} as {resolution_outcome} through the local User Channel. Treat this as recorded context, not as an agent-authored judgment."
        ),
    })
}

fn select_chat_judgment(
    store: &CoreProjectStore,
    task_id: &TaskId,
    chat_id: &str,
    envelope: &GuardEnvelope,
) -> Result<(UserJudgmentRecord, usize), PromptCommandBlock> {
    let chat_index = parse_chat_id(chat_id)?;
    let records = store
        .user_judgment_records_for_task(task_id)
        .map_err(prompt_block_from_store_error)?;
    let Some(record) = records.get(chat_index - 1).cloned() else {
        return Err(PromptCommandBlock {
            code: "unknown_judgment_id",
            message: format!(
                "Volicord judgment id `{chat_id}` does not match a judgment for the active task."
            ),
        });
    };
    let expected_actor =
        ActorSource::agent_connection(envelope.connection_id.clone()).to_canonical_string();
    if record.requested_by_actor_source != expected_actor {
        return Err(PromptCommandBlock {
            code: "connection_mismatch",
            message: format!(
                "Volicord judgment `{chat_id}` belongs to a different Agent Connection."
            ),
        });
    }
    if record.status != "pending" {
        if record.status == "resolved" {
            return Ok((record, chat_index));
        }
        return Err(PromptCommandBlock {
            code: "judgment_not_pending",
            message: format!(
                "Volicord judgment `{chat_id}` is not pending (status: {}).",
                record.status
            ),
        });
    }
    if record.basis_status != "current" {
        return Err(PromptCommandBlock {
            code: "stale_judgment",
            message: format!(
                "Volicord judgment `{chat_id}` has a stale or superseded basis (basis_status: {}).",
                record.basis_status
            ),
        });
    }
    Ok((record, chat_index))
}

fn judgment_code_is_expired(
    record: &UserJudgmentRecord,
    envelope: &GuardEnvelope,
) -> Result<bool, PromptCommandBlock> {
    let request = serde_json::from_str::<PersistedUserJudgmentRequest>(&record.request_json)
        .map_err(|error| PromptCommandBlock {
            code: "invalid_judgment_command",
            message: format!("Failed to decode pending judgment request metadata: {error}"),
        })?;
    let Some(expires_at) = request.expires_at.as_ref() else {
        return Ok(false);
    };
    let occurred_at =
        UtcTimestamp::parse(&envelope.occurred_at).map_err(|error| PromptCommandBlock {
            code: "invalid_judgment_command",
            message: format!("Prompt capture timestamp is invalid: {error}"),
        })?;
    Ok(&occurred_at >= expires_at)
}

fn judgment_expected_state_version(record: &UserJudgmentRecord) -> Result<u64, PromptCommandBlock> {
    let basis =
        serde_json::from_str::<PersistedJudgmentBasis>(&record.basis_json).map_err(|error| {
            PromptCommandBlock {
                code: "invalid_judgment_command",
                message: format!("Failed to decode pending judgment basis metadata: {error}"),
            }
        })?;
    basis
        .created_at_state_version
        .checked_add(1)
        .ok_or_else(|| PromptCommandBlock {
            code: "invalid_judgment_command",
            message: "Pending judgment state version is too large.".to_owned(),
        })
}

fn prompt_block_from_record_response(response: &Value) -> PromptCommandBlock {
    let message = core_rejection_message(response);
    if message.contains("idempotency_key was reused with a different request hash") {
        PromptCommandBlock {
            code: "conflicting_judgment_command",
            message: "Volicord already recorded a different answer for this verification code."
                .to_owned(),
        }
    } else {
        PromptCommandBlock {
            code: "judgment_record_rejected",
            message,
        }
    }
}

fn parse_chat_id(chat_id: &str) -> Result<usize, PromptCommandBlock> {
    let Some(raw_index) = chat_id.strip_prefix("J-") else {
        return Err(PromptCommandBlock {
            code: "invalid_judgment_id",
            message: format!("Volicord judgment id `{chat_id}` must use the chat form `J-N`."),
        });
    };
    if raw_index.is_empty() || !raw_index.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(PromptCommandBlock {
            code: "invalid_judgment_id",
            message: format!(
                "Volicord judgment id `{chat_id}` must use a positive numeric suffix."
            ),
        });
    }
    let index = raw_index.parse::<usize>().map_err(|_| PromptCommandBlock {
        code: "invalid_judgment_id",
        message: format!("Volicord judgment id `{chat_id}` is too large."),
    })?;
    if index == 0 {
        return Err(PromptCommandBlock {
            code: "invalid_judgment_id",
            message: "Volicord judgment ids start at J-1.".to_owned(),
        });
    }
    Ok(index)
}

fn select_defer_option(
    options: &[UserJudgmentOption],
) -> Result<UserJudgmentOption, PromptCommandBlock> {
    options
        .iter()
        .find(|option| option.machine_action == UserJudgmentOptionAction::Defer)
        .cloned()
        .ok_or_else(|| PromptCommandBlock {
            code: "defer_unavailable",
            message: "The addressed judgment does not offer a defer option.".to_owned(),
        })
}

fn prompt_block_from_user_error(error: UserCommandError) -> PromptCommandBlock {
    PromptCommandBlock {
        code: "invalid_judgment_command",
        message: error.to_string(),
    }
}

fn prompt_block_from_store_error(error: StoreError) -> PromptCommandBlock {
    PromptCommandBlock {
        code: "store_error",
        message: error.to_string(),
    }
}

fn core_rejection_message(response: &Value) -> String {
    response["errors"]
        .as_array()
        .and_then(|errors| errors.first())
        .and_then(|error| error["message"].as_str())
        .unwrap_or("Core rejected the user judgment command.")
        .to_owned()
}

fn pending_chat_judgment_summaries(
    store: &CoreProjectStore,
    task_id: &TaskId,
    envelope: &GuardEnvelope,
) -> Result<Vec<GuardPendingJudgmentSummary>, GuardCommandError> {
    let occurred_at = UtcTimestamp::parse(&envelope.occurred_at)
        .map_err(|error| GuardCommandError::Runtime(format!("invalid guard timestamp: {error}")))?;
    let expected_actor =
        ActorSource::agent_connection(envelope.connection_id.clone()).to_canonical_string();
    let records = store.user_judgment_records_for_task(task_id)?;
    let mut summaries = Vec::new();
    for (index, record) in records.iter().enumerate() {
        if record.status != "pending" || record.requested_by_actor_source != expected_actor {
            continue;
        }
        if record.basis_status != "current" {
            continue;
        }
        let chat_id = chat_id_for_index(index + 1);
        let request = serde_json::from_str::<PersistedUserJudgmentRequest>(&record.request_json)
            .map_err(|error| {
                GuardCommandError::Runtime(format!(
                    "failed to decode user_judgments.request_json: {error}"
                ))
            })?;
        if request
            .expires_at
            .as_ref()
            .is_some_and(|expires_at| &occurred_at >= expires_at)
        {
            continue;
        }
        let options = decode_options(record).map_err(guard_error_from_user_error)?;
        let option_summaries = options
            .iter()
            .enumerate()
            .map(|(option_index, option)| {
                let selector = chat_option_selector(option_index + 1, option);
                GuardPendingJudgmentOptionSummary {
                    instruction: format!(
                        "Volicord: answer {chat_id} {selector} {}",
                        judgment_verification_code(record, envelope)
                    ),
                    selector,
                    option_id: option.option_id.as_str().to_owned(),
                    label: option.label.clone(),
                    machine_action: machine_action_value(option.machine_action).to_owned(),
                    resolution_outcome: outcome_value(option.resolution_outcome).to_owned(),
                }
            })
            .collect::<Vec<_>>();
        let default_selector = option_summaries
            .first()
            .map(|option| option.selector.clone())
            .unwrap_or_else(|| "1".to_owned());
        let verification_code = judgment_verification_code(record, envelope);
        summaries.push(GuardPendingJudgmentSummary {
            chat_id: chat_id.clone(),
            verification_code: verification_code.clone(),
            judgment_kind: record.judgment_kind.clone(),
            question: Some(request.question),
            answer_instruction: format!(
                "Volicord: answer {chat_id} {default_selector} {verification_code}"
            ),
            note_instruction: format!("Volicord: note {chat_id} \"text\" {verification_code}"),
            options: option_summaries,
        });
    }
    Ok(summaries)
}

fn guard_error_from_user_error(error: UserCommandError) -> GuardCommandError {
    match error {
        UserCommandError::Usage(message) => GuardCommandError::Usage(message),
        UserCommandError::Runtime(message) => GuardCommandError::Runtime(message),
    }
}

fn chat_option_selector(index: usize, option: &UserJudgmentOption) -> String {
    match option.machine_action {
        UserJudgmentOptionAction::Reject => "reject".to_owned(),
        UserJudgmentOptionAction::Defer => "defer".to_owned(),
        UserJudgmentOptionAction::Accept => index.to_string(),
    }
}

fn chat_id_for_index(index: usize) -> String {
    format!("J-{index}")
}

fn judgment_verification_code(record: &UserJudgmentRecord, envelope: &GuardEnvelope) -> String {
    chat_judgment_verification_code(
        &record.project_id,
        &record.task_id,
        &record.judgment_id,
        &record.requested_at,
        &envelope.connection_id,
    )
}

fn prompt_judgment_replay_id(record: &UserJudgmentRecord, envelope: &GuardEnvelope) -> String {
    let digest = short_digest(&[
        "prompt_judgment_record",
        &record.project_id,
        &record.task_id,
        &record.judgment_id,
        &record.requested_at,
        &envelope.connection_id,
    ]);
    format!("prompt_judgment_{digest}")
}

fn short_digest(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hex_bytes(&hasher.finalize());
    digest[..10].to_owned()
}

fn machine_action_value(value: UserJudgmentOptionAction) -> &'static str {
    match value {
        UserJudgmentOptionAction::Accept => "accept",
        UserJudgmentOptionAction::Reject => "reject",
        UserJudgmentOptionAction::Defer => "defer",
    }
}

fn outcome_value(value: JudgmentResolutionOutcome) -> &'static str {
    match value {
        JudgmentResolutionOutcome::Accepted => "accepted",
        JudgmentResolutionOutcome::Rejected => "rejected",
        JudgmentResolutionOutcome::Deferred => "deferred",
    }
}

fn event_project_id(event: &Value) -> Option<String> {
    event_string(event, &[&["project_id"], &["project", "id"]])
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
                write_check: true,
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
                "schema_version": GUARD_SCHEMA_VERSION,
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
        "schema_version": GUARD_SCHEMA_VERSION,
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
                    "schema_version": GUARD_SCHEMA_VERSION,
                    "phase": phase.event_kind(),
                    "decision": decision.as_str(),
                    "allowed": decision != GuardDecision::Deny,
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
            Ok(RenderedGuardOutput {
                stdout: format!(
                    "Volicord guard {}: {} ({})\n",
                    phase.command_name(),
                    decision.as_str(),
                    allowed
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
            "additionalContext": message
        }
    }))
}

fn blocking_reason(phase: GuardPhase, result: &Value) -> String {
    first_reason_message(result).unwrap_or_else(|| match phase {
        GuardPhase::SessionStart => "Volicord session context could not be prepared.".to_owned(),
        GuardPhase::PreTool => "Volicord blocked this tool call.".to_owned(),
        GuardPhase::PostTool => "Volicord blocked normal handling of this tool result.".to_owned(),
        GuardPhase::PromptCapture => "Volicord blocked this user prompt.".to_owned(),
        GuardPhase::Stop => "Volicord needs more work before this session stops.".to_owned(),
    })
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
    let write_checks = context
        .get("current_write_check_ids")
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
        "Volicord context: project `{project_name}`, state_version {state_version}, active_task {active_task}, current_write_checks {write_checks}, pending_user_judgments {pending_judgments}, unresolved_unrecorded_changes {unresolved_changes}."
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
        "current_write_check_ids": summary.current_write_check_ids,
        "stale_write_check_ids": summary.stale_write_check_ids,
        "pending_user_judgment_count": summary.pending_user_judgment_count,
        "pending_user_judgments": summary.pending_user_judgments
            .iter()
            .map(pending_judgment_summary_json)
            .collect::<Vec<_>>(),
        "active_blocker_count": summary.active_blocker_count,
        "unresolved_unrecorded_change_count": summary.unresolved_unrecorded_change_count
    })
}

fn pending_judgment_summary_json(summary: &GuardPendingJudgmentSummary) -> Value {
    json!({
        "chat_id": summary.chat_id,
        "verification_code": summary.verification_code,
        "judgment_kind": summary.judgment_kind,
        "question": summary.question,
        "answer_instruction": summary.answer_instruction,
        "note_instruction": summary.note_instruction,
        "options": summary.options.iter().map(|option| {
            json!({
                "selector": option.selector,
                "option_id": option.option_id,
                "label": option.label,
                "machine_action": option.machine_action,
                "resolution_outcome": option.resolution_outcome,
                "instruction": option.instruction
            })
        }).collect::<Vec<_>>()
    })
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

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cursor = value;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    Some(cursor)
}

fn event_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        if let Some(text) = value_at(value, path).and_then(Value::as_str) {
            if !text.trim().is_empty() {
                return Some(text.to_owned());
            }
        }
    }
    None
}

fn event_bool(value: &Value, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_bool))
}

fn event_i64(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_i64))
}

fn extract_prompt_text(value: &Value) -> Option<String> {
    event_string(
        value,
        &[
            &["prompt"],
            &["user_prompt"],
            &["message"],
            &["input", "prompt"],
            &["input", "message"],
            &["event", "prompt"],
        ],
    )
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

fn current_timestamp() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn event_time_or_now(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::from(SystemTime::now()))
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
    GuardCommandError::Runtime(format!("failed to serialize guard output: {error}"))
}
