use std::{ffi::OsString, fmt, fs, path::Path, time::Instant};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_core::{Clock, CorePipelineError, SystemClock};
use volicord_store::{
    bootstrap::{project_record_for_execution, ProjectRecord},
    core_pipeline::CoreProjectStore,
    diagnostics::{
        record_diagnostic_event, start_diagnostic_session, DiagnosticEvent, DiagnosticEventKind,
        DiagnosticHostKind, DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport,
        DiagnosticUserChannelKind,
    },
    guards::{
        agent_session, guard_event, insert_agent_session, insert_guard_event,
        observe_guard_installation, AgentSessionInsert, GuardEventInsert,
        GuardInstallationObservation,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError, StoreResult,
};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_bytes, GuardDecision, IntegrationProfile,
    UtcTimestamp,
};

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

fn core_current_timestamp(store: &CoreProjectStore) -> StoreResult<UtcTimestamp> {
    SystemClock.project_now(store)
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
    let diagnostic_started = Instant::now();
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
    record_guard_diagnostic_best_effort(
        &runtime_home,
        &project,
        &envelope,
        phase,
        diagnostic_started,
        input.raw_text.len() as u64,
        &phase_result.result,
    );
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

#[allow(clippy::too_many_arguments)]
fn record_guard_diagnostic_best_effort(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    started: Instant,
    request_bytes: u64,
    result: &Value,
) {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return;
    };
    let authoritative_refresh_failure = result
        .get("reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|reason| {
            reason.get("code").and_then(Value::as_str) == Some("authoritative_refresh_failed")
        });
    let prompt_capture_recorded = phase == GuardPhase::PromptCapture
        && result
            .get("recognized_user_action_command")
            .is_some_and(|value| !value.is_null());
    let prompt_capture_replayed = prompt_capture_recorded
        && result
            .pointer("/recognized_user_action_command/replayed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let product_file_write_count = (phase == GuardPhase::PostTool
        && result
            .pointer("/tool/changed_paths")
            .and_then(Value::as_array)
            .is_some_and(|paths| {
                paths
                    .iter()
                    .any(|path| path.get("inside_repo").and_then(Value::as_bool) == Some(true))
            })) as u64;
    let core_reached = prompt_capture_recorded
        || (phase == GuardPhase::Stop
            && result
                .pointer("/close_status/active_task")
                .is_some_and(|value| !value.is_null()));
    let core_committed = prompt_capture_recorded && !prompt_capture_replayed;
    let response_bytes = serde_json::to_vec(result)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let outcome = if authoritative_refresh_failure {
        DiagnosticOutcome::Unavailable
    } else if result.get("allowed").and_then(Value::as_bool) == Some(false) {
        DiagnosticOutcome::Rejected
    } else {
        DiagnosticOutcome::Success
    };
    let build = volicord_mcp::build_info();
    let host_kind = Some(DiagnosticHostKind::from_connection_host_kind(
        &envelope.host_kind,
    ));
    let _ = start_diagnostic_session(
        runtime_home,
        DiagnosticSessionStart {
            session_id,
            connection_id: Some(&envelope.connection_id),
            project_id: Some(&project.project_id),
            transport: DiagnosticTransport::GuardHook,
            host_kind,
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    );
    let _ = record_diagnostic_event(
        runtime_home,
        DiagnosticEvent {
            session_id,
            event_kind: DiagnosticEventKind::GuardHook,
            tool_name: None,
            latency_micros: elapsed,
            request_bytes,
            response_bytes,
            validation_failure: false,
            core_reached,
            core_committed,
            replayed: prompt_capture_replayed,
            user_channel_kind: prompt_capture_recorded
                .then_some(DiagnosticUserChannelKind::PromptCapture),
            fallback_kind: None,
            product_file_write_count,
            authoritative_refresh_failure,
            outcome,
        },
    );
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
    let input = GuardEventInsert {
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
    };
    if let Some(existing) = guard_event(runtime_home, &project.project_id, &envelope.event_id)? {
        if guard_event_record_payload_sha256(&existing)?
            == guard_event_insert_payload_sha256(&input)?
        {
            return Ok(());
        }
        return Err(GuardCommandError::Runtime(format!(
            "guard event {} conflicts with a different payload hash",
            envelope.event_id
        )));
    }
    insert_guard_event(runtime_home, &project.project_id, input)?;
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
        "tool_input_sha256": guard_event_tool_input(&input.raw_value).map(canonical_value_sha256),
        "tool_result_sha256": guard_event_tool_result(&input.raw_value).map(canonical_value_sha256),
        "tool_result_size_bytes": guard_event_tool_result(&input.raw_value)
            .and_then(|value| canonical_json_bytes(value).ok())
            .and_then(|bytes| u64::try_from(bytes.len()).ok()),
        "raw_event": input.redacted_value
    })
}

fn guard_event_tool_input(event: &Value) -> Option<&Value> {
    event
        .get("tool_input")
        .or_else(|| event.get("input"))
        .or_else(|| event.pointer("/tool/input"))
        .or_else(|| event.pointer("/tool/arguments"))
        .or_else(|| event.pointer("/tool_use/input"))
}

fn guard_event_tool_result(event: &Value) -> Option<&Value> {
    event
        .get("tool_response")
        .or_else(|| event.get("tool_result"))
        .or_else(|| event.get("result"))
        .or_else(|| event.get("output"))
}

fn canonical_value_sha256(value: &Value) -> String {
    canonical_json_bare_sha256(value).expect("serde_json::Value must serialize")
}

fn guard_event_insert_payload_sha256(
    input: &GuardEventInsert,
) -> Result<String, GuardCommandError> {
    guard_event_payload_sha256(
        input.session_id.as_deref(),
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.event_kind,
        &input.decision,
        &input.subject_json,
        &input.occurred_at,
    )
}

fn guard_event_record_payload_sha256(
    record: &volicord_store::guards::GuardEventRecord,
) -> Result<String, GuardCommandError> {
    guard_event_payload_sha256(
        record.session_id.as_deref(),
        &record.connection_internal_id,
        record.guard_installation_id.as_deref(),
        &record.event_kind,
        &record.decision,
        &record.subject_json,
        &record.occurred_at,
    )
}

fn guard_event_payload_sha256(
    session_id: Option<&str>,
    connection_id: &str,
    guard_installation_id: Option<&str>,
    event_kind: &str,
    decision: &str,
    subject_json: &str,
    occurred_at: &str,
) -> Result<String, GuardCommandError> {
    let subject: Value = serde_json::from_str(subject_json).map_err(json_error)?;
    let raw_event_sha256 = subject
        .get("raw_event_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GuardCommandError::Runtime(
                "guard event subject has no raw_event_sha256 replay coordinate".to_owned(),
            )
        })?;
    Ok(canonical_value_sha256(&json!({
        "session_id": session_id,
        "connection_id": connection_id,
        "guard_installation_id": guard_installation_id,
        "event_kind": event_kind,
        "decision": decision,
        "raw_event_sha256": raw_event_sha256,
        "occurred_at": occurred_at,
    })))
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

#[cfg(test)]
mod replay_tests {
    use super::*;

    #[test]
    fn guard_event_replay_hash_is_idempotent_for_same_source_and_conflicts_for_changed_payload() {
        let first = GuardEventInsert {
            guard_event_id: "guard_event_replay".to_owned(),
            session_id: Some("session_replay".to_owned()),
            connection_internal_id: "connection_replay".to_owned(),
            guard_installation_id: Some("guard_replay".to_owned()),
            event_kind: "post_tool".to_owned(),
            decision: "allow".to_owned(),
            subject_json: json!({
                "raw_event_sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "raw_event": {"tool_use_id": "same"}
            })
            .to_string(),
            result_json: json!({"state": "first-render"}).to_string(),
            occurred_at: "2026-07-13T00:00:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        };
        let mut same_source = first.clone();
        same_source.result_json = json!({"state": "later-render"}).to_string();
        assert_eq!(
            guard_event_insert_payload_sha256(&first).expect("first replay hash"),
            guard_event_insert_payload_sha256(&same_source).expect("same-source replay hash")
        );

        let mut changed_payload = first.clone();
        changed_payload.subject_json = json!({
            "raw_event_sha256": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "raw_event": {"tool_use_id": "changed"}
        })
        .to_string();
        assert_ne!(
            guard_event_insert_payload_sha256(&first).expect("first replay hash"),
            guard_event_insert_payload_sha256(&changed_payload)
                .expect("changed-payload replay hash")
        );

        let mut changed_decision = first.clone();
        changed_decision.decision = "deny".to_owned();
        assert_ne!(
            guard_event_insert_payload_sha256(&first).expect("first replay hash"),
            guard_event_insert_payload_sha256(&changed_decision)
                .expect("changed-decision replay hash")
        );
    }
}
