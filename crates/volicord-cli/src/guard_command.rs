use std::{ffi::OsString, fmt, fs, path::Path};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_core::CorePipelineError;
use volicord_store::{
    bootstrap::{project_record_for_execution, ProjectRecord},
    guards::{
        agent_session, guard_event, insert_agent_session, insert_guard_event,
        observe_guard_installation, AgentSessionInsert, GuardEventInsert,
        GuardInstallationObservation,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};
use volicord_types::{GuardDecision, IntegrationProfile};

use crate::disclosure::cooperative_host_decision_disclosure_json;
use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
const DEFAULT_INTEGRATION_PROFILE: &str = "detective";
const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
const EXPECTED_WRITE_TTL_MINUTES: i64 = 15;
const SESSION_WATCH_METADATA_SOURCE: &str = "volicord_session_watch";

mod args;
mod context;
mod envelope;
mod mutation;
mod phase;
mod prompt_capture;
mod prompt_command;
mod render;
mod tool_observation;
mod write_ticket;

pub use args::guard_usage;
use args::{parse_guard_options, read_guard_input, GuardInput, GuardOptions, GuardPhase};
use envelope::{event_path_field, event_string, guard_envelope, GuardEnvelope};
use phase::{pre_tool::persist_expected_write, GuardPhaseResult};
use prompt_capture::handle_prompt_capture;
use render::render_guard_output;

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
    let mut phase_result = match phase {
        GuardPhase::SessionStart => {
            phase::session_start::handle_session_start(&runtime_home, &project, &envelope, &input)?
        }
        GuardPhase::PreTool => {
            phase::pre_tool::handle_pre_tool(&runtime_home, &project, &envelope, &input)?
        }
        GuardPhase::PostTool => {
            phase::post_tool::handle_post_tool(&runtime_home, &project, &envelope, &input)?
        }
        GuardPhase::PromptCapture => {
            let (decision, result, _exits_failure) =
                handle_prompt_capture(&runtime_home, &project, &envelope, &input)?;
            GuardPhaseResult::new(decision, result)
        }
        GuardPhase::Stop => phase::stop::handle_stop(&runtime_home, &project, &envelope, &input)?,
    };
    attach_guard_disclosure(&mut phase_result.result);

    let subject = guard_subject(phase, &input, &envelope, &project);
    persist_guard_event(
        &runtime_home,
        &project,
        &envelope,
        phase,
        phase_result.decision,
        subject,
        phase_result.result.clone(),
    )?;
    if let Some(expected_write) = phase_result.expected_write {
        persist_expected_write(&runtime_home, &project, expected_write)?;
    }
    let rendered = render_guard_output(
        phase,
        phase_result.decision,
        &envelope,
        phase_result.result,
        options.output,
    )?;
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
