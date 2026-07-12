use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    bootstrap::ProjectRecord,
    core_pipeline::{CoreProjectStore, UserJudgmentRecord},
    guards::{
        guard_health_record, insert_prompt_capture, prompt_capture, prompt_capture_availability,
        PromptCaptureAvailability, PromptCaptureInsert,
    },
    StoreError,
};
use volicord_types::{
    chat_judgment_verification_code, ActorSource, GuardDecision, JudgmentResolutionOutcome,
    PersistedJudgmentBasis, PersistedUserJudgmentRequest, ProjectId, PromptCaptureStatus, TaskId,
    UserJudgmentOption, UserJudgmentOptionAction, UtcTimestamp,
    VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

use crate::user_command::{
    decode_options, record_user_judgment_from_record, select_option, JudgmentRecordingInput,
    UserCommandError,
};

use super::prompt_command::{
    parse_chat_id, parse_prompt_judgment_command, PromptCommandBlock, PromptCommandDetection,
    PromptJudgmentCommand, RecordedPromptJudgment,
};
use super::{
    args::GuardInput,
    current_policy_hash,
    envelope::{event_string, GuardEnvelope},
    hex_bytes, json_error, sha256_text, stable_id, GuardCommandError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardPendingJudgmentSummary {
    pub(super) chat_id: String,
    pub(super) verification_code: String,
    pub(super) judgment_kind: String,
    pub(super) question: Option<String>,
    pub(super) answer_instruction: String,
    pub(super) note_instruction: String,
    pub(super) options: Vec<GuardPendingJudgmentOptionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardPendingJudgmentOptionSummary {
    pub(super) selector: String,
    pub(super) option_id: String,
    pub(super) label: String,
    pub(super) machine_action: String,
    pub(super) resolution_outcome: String,
    pub(super) instruction: String,
}

pub(super) fn prompt_capture_availability_for_event(
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
            "Use host prompt input if available; otherwise use the local volicord inbox command as the CLI inbox path.",
        ),
        PromptCaptureStatus::NotConfigured => (
            "prompt_capture_not_configured",
            "Chat command capture is not configured for this host, project, and connection.".to_owned(),
            "Configure chat command capture, or use the local volicord inbox command as the CLI inbox path.",
        ),
        PromptCaptureStatus::ReloadRequired => (
            "prompt_capture_reload_required",
            "Chat command capture configuration is installed but the host must reload the current policy.".to_owned(),
            "Restart or reload the host before using chat commands.",
        ),
        PromptCaptureStatus::Degraded => (
            "prompt_capture_degraded",
            "Chat command capture is degraded for this host, project, and connection.".to_owned(),
            "Repair the detective host hook integration before using chat commands.",
        ),
        _ => (
            "prompt_capture_unavailable",
            "Chat command capture is unavailable for this host, project, and connection.".to_owned(),
            "Use host prompt input if available; otherwise use the local volicord inbox command as the CLI inbox path.",
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
                    "prompt_text_omitted": true
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

pub(super) fn handle_prompt_capture(
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
                            "note_text_omitted": recorded.note_text_omitted,
                            "replayed": recorded.replayed
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
        replayed: response.replayed,
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

pub(super) fn pending_chat_judgment_summaries(
    store: &CoreProjectStore,
    task_id: &TaskId,
    envelope: &GuardEnvelope,
) -> Result<Vec<GuardPendingJudgmentSummary>, GuardCommandError> {
    let occurred_at = UtcTimestamp::parse(&envelope.occurred_at).map_err(|error| {
        GuardCommandError::Runtime(format!("invalid host-hook timestamp: {error}"))
    })?;
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
