use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    bootstrap::ProjectRecord,
    core_pipeline::{CoreProjectStore, EffectiveUserActionRecord},
    guards::{
        guard_health_record, insert_prompt_capture, prompt_capture, prompt_capture_availability,
        PromptCaptureAvailability, PromptCaptureInsert,
    },
    StoreError,
};
use volicord_types::{
    chat_user_action_verification_code, ActorSource, AgentSafeUserActionRequestSummary, ArtifactId,
    EvidenceTarget, GuardDecision, PersistedUserActionRequest, ProjectId, PromptCaptureStatus,
    TaskId, UserActionInboxForm, UserActionPresentationPlan, UserActionPresentationSafety,
    UserActionRequestId, UserActionResolutionInput, UserActionStatus, UtcTimestamp,
    VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

use crate::guard_integration::host_hook_capability_has_exact_v2_shape;
use crate::user_command::{
    canonical_user_action_inbox_items, resolve_user_action_from_record, select_inbox_choice,
    UserActionResolutionRecordingInput, UserCommandError,
};

use super::prompt_command::{
    parse_chat_id, parse_prompt_user_action_command, PromptCommandBlock, PromptCommandDetection,
    PromptEvidenceTarget, PromptUserActionCommand, PromptUserActionResolution,
    RecordedPromptUserAction,
};
use super::{
    args::GuardInput,
    core_current_timestamp, current_policy_hash,
    envelope::{event_string, GuardEnvelope},
    hex_bytes, json_error, sha256_text, stable_id, GuardCommandError,
};

pub(super) type GuardPendingUserActionSummary = AgentSafeUserActionRequestSummary;

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
    if !host_hook_capability_has_exact_v2_shape(&value) {
        return Err(GuardCommandError::Runtime(
            "stored host-hook capability is not current v2 input".to_owned(),
        ));
    }
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
            "recognized_user_action_command": null,
            "reasons": [{
                "code": code,
                "message": message,
                "severity": "deny",
                "next_action": next_action
            }],
            "next_action": next_action,
            "model_context": format!("Volicord did not resolve a user action: {message}"),
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
            "Use host prompt input if available; otherwise use `volicord inbox resolve`.",
        ),
        PromptCaptureStatus::NotConfigured => (
            "prompt_capture_not_configured",
            "Chat command capture is not configured for this host, project, and connection."
                .to_owned(),
            "Configure chat command capture, or use `volicord inbox resolve`.",
        ),
        PromptCaptureStatus::ReloadRequired => (
            "prompt_capture_reload_required",
            "Chat command capture configuration is installed but the host must reload the current policy."
                .to_owned(),
            "Restart or reload the host before using chat commands.",
        ),
        PromptCaptureStatus::Degraded => (
            "prompt_capture_degraded",
            "Chat command capture is degraded for this host, project, and connection.".to_owned(),
            "Repair the detective host hook integration before using chat commands.",
        ),
        _ => (
            "prompt_capture_unavailable",
            "Chat command capture is unavailable for this host, project, and connection."
                .to_owned(),
            "Use host prompt input if available; otherwise use `volicord inbox resolve`.",
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
        return Ok(json!({ "captured": false, "reason": "no_prompt_text" }));
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
        .map(|prompt| parse_prompt_user_action_command(&prompt))
        .unwrap_or(PromptCommandDetection::NoCommand);
    match command {
        PromptCommandDetection::NoCommand => Ok((
            GuardDecision::Allow,
            json!({
                "decision": GuardDecision::Allow.as_str(),
                "allowed": true,
                "prompt_capture": capture,
                "recognized_user_action_command": null,
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
                                "Volicord user-action command targeted project `{event_project_id}`, but this prompt hook is bound to `{}`.",
                                project.project_id
                            ),
                        },
                    ));
                }
            }
            match record_prompt_user_action_command(runtime_home, project, envelope, command) {
                Ok(recorded) => Ok((
                    GuardDecision::InjectContext,
                    json!({
                        "decision": GuardDecision::InjectContext.as_str(),
                        "allowed": true,
                        "prompt_capture": capture,
                        "recognized_user_action_command": {
                            "command_kind": "resolve",
                            "chat_id": recorded.chat_id,
                            "verification_code": recorded.verification_code,
                            "action_type": recorded.action_type,
                            "selected_option_id": recorded.selected_option_id,
                            "selected_target": recorded.selected_target,
                            "artifact_ids": recorded.artifact_ids,
                            "relevance_status": recorded.relevance_status,
                            "note_text_omitted": recorded.note_text_omitted,
                            "summary_text_omitted": recorded.summary_text_omitted,
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
            "recognized_user_action_command": null,
            "reasons": [{ "code": block.code, "message": block.message, "severity": "deny" }],
            "model_context": format!("Volicord did not resolve a user action: {}", block.message),
            "enforcement_level": "cooperative_detective"
        }),
        true,
    )
}

fn record_prompt_user_action_command(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    command: PromptUserActionCommand,
) -> Result<RecordedPromptUserAction, PromptCommandBlock> {
    let store = CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id))
        .map_err(prompt_block_from_store_error)?;
    let now = core_current_timestamp(&store).map_err(prompt_block_from_store_error)?;
    let record = store
        .user_action_record(&command.user_action_request_id, &now)
        .map_err(prompt_block_from_store_error)?;
    let record = record.ok_or_else(|| PromptCommandBlock {
        code: "unknown_user_action_request",
        message: format!(
            "Volicord user-action request `{}` was not found in this project.",
            command.user_action_request_id
        ),
    })?;
    let task_id = TaskId::new(&record.request.task_id);
    let records = store
        .user_action_records_for_task(&task_id, &now)
        .map_err(prompt_block_from_store_error)?;
    let index = parse_chat_id(&command.chat_id)?;
    let indexed_record = records.get(index - 1).ok_or_else(|| PromptCommandBlock {
        code: "unknown_user_action_id",
        message: format!(
            "Volicord user-action id `{}` does not match an action for the stored request task.",
            command.chat_id
        ),
    })?;
    if indexed_record.request.user_action_request_id != record.request.user_action_request_id {
        return Err(PromptCommandBlock {
            code: "user_action_request_mismatch",
            message: format!(
                "Volicord user-action id `{}` does not match request `{}`.",
                command.chat_id, command.user_action_request_id
            ),
        });
    }
    validate_prompt_record(&record, envelope, &command)?;

    let (form, question, context_summary) = if record.status == UserActionStatus::Pending {
        let item = canonical_user_action_inbox_items(
            runtime_home,
            &project.project_id,
            &record.request.task_id,
            ActorSource::agent_connection(envelope.connection_id.clone()),
            VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
            envelope.session_id.as_deref(),
        )
        .map_err(prompt_block_from_user_error)?
        .into_iter()
        .find(|item| item.user_action_request_id.as_str() == record.request.user_action_request_id)
        .ok_or_else(|| PromptCommandBlock {
            code: "user_action_not_pending",
            message: "The addressed user action is no longer in the canonical pending inbox."
                .to_owned(),
        })?;
        (item.form, item.question, item.context_summary)
    } else {
        immutable_replay_presentation(&record)?
    };
    require_user_only_channel_when_presentation_is_sensitive(&form, &question, &context_summary)?;
    let (resolution, recorded) = prompt_resolution_from_form(&form, &command.resolution)?;
    let replay_id = prompt_user_action_replay_id(&record, envelope);
    let response = resolve_user_action_from_record(UserActionResolutionRecordingInput {
        runtime_home,
        project_id: &project.project_id,
        record: &record,
        resolution,
        verification_basis: VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
        request_id: Some(format!("req_{replay_id}")),
        channel_submission_id: Some(format!("submission_{replay_id}")),
        session_id: envelope.session_id.as_deref(),
    })
    .map_err(prompt_block_from_user_error)?;
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return Err(prompt_block_from_record_response(&response.response_value));
    }
    let mut recorded = recorded;
    recorded.chat_id = command.chat_id;
    recorded.verification_code = command.verification_code;
    recorded.replayed = response.replayed;
    recorded.model_context = format!(
        "Volicord resolved user action {} through the local User Channel. Treat this as user-owned recorded context, not as an agent-authored action.",
        record.request.user_action_request_id
    );
    Ok(recorded)
}

fn validate_prompt_record(
    record: &EffectiveUserActionRecord,
    envelope: &GuardEnvelope,
    command: &PromptUserActionCommand,
) -> Result<(), PromptCommandBlock> {
    let expected_actor =
        ActorSource::agent_connection(envelope.connection_id.clone()).to_canonical_string();
    if record.request.requested_by_actor_source != expected_actor {
        return Err(PromptCommandBlock {
            code: "connection_mismatch",
            message: format!(
                "Volicord user action `{}` belongs to a different Agent Connection.",
                command.chat_id
            ),
        });
    }
    let expected_code = user_action_verification_code(record, envelope);
    if command.verification_code != expected_code {
        return Err(PromptCommandBlock {
            code: "wrong_verification_code",
            message: format!(
                "Volicord user action `{}` requires the current displayed verification code.",
                command.chat_id
            ),
        });
    }
    if record.status != UserActionStatus::Pending && record.resolution.is_none() {
        return Err(PromptCommandBlock {
            code: "user_action_not_pending",
            message: format!(
                "Volicord user action `{}` is not pending (status: {}).",
                command.chat_id,
                enum_text(record.status)
            ),
        });
    }
    Ok(())
}

fn immutable_replay_presentation(
    record: &EffectiveUserActionRecord,
) -> Result<(UserActionInboxForm, String, String), PromptCommandBlock> {
    let request: PersistedUserActionRequest = serde_json::from_str(&record.request.request_json)
        .map_err(|error| PromptCommandBlock {
            code: "invalid_user_action_command",
            message: format!("Failed to decode immutable user-action request: {error}"),
        })?;
    let question = request.body.question().to_owned();
    let context_summary = request.body.context_summary().to_owned();
    let form = request
        .body
        .capture_form()
        .map_err(|error| PromptCommandBlock {
            code: "invalid_user_action_command",
            message: format!("Invalid immutable user-action form: {error}"),
        })?;
    Ok((form, question, context_summary))
}

fn require_user_only_channel_when_presentation_is_sensitive(
    form: &UserActionInboxForm,
    question: &str,
    context_summary: &str,
) -> Result<(), PromptCommandBlock> {
    let safety = UserActionPresentationPlan::from_form(form)
        .and_then(|presentation| {
            presentation.agent_facing_input_safety(question, context_summary)
        })
        .map_err(|_| PromptCommandBlock {
            code: "prompt_capture_presentation_unavailable",
            message: "This user action could not be safely presented through prompt capture. Use a user-only local consent or CLI inbox channel. No resolution was recorded."
                .to_owned(),
        })?;
    if safety == UserActionPresentationSafety::UserOnlyInputRequired {
        return Err(PromptCommandBlock {
            code: "prompt_capture_presentation_user_only",
            message: "This user action requires a user-only local consent or CLI inbox channel. Prompt capture did not show or resolve the action."
                .to_owned(),
        });
    }
    Ok(())
}

fn prompt_resolution_from_form(
    form: &UserActionInboxForm,
    input: &PromptUserActionResolution,
) -> Result<(UserActionResolutionInput, RecordedPromptUserAction), PromptCommandBlock> {
    match (form, input) {
        (
            UserActionInboxForm::Choice { choices, .. },
            PromptUserActionResolution::Choice { selector, note },
        ) => {
            let selected =
                select_inbox_choice(choices, selector).map_err(prompt_block_from_user_error)?;
            Ok((
                UserActionResolutionInput::Choice {
                    selected_option_id: selected.choice_id.clone(),
                    note: note.clone().into(),
                },
                RecordedPromptUserAction {
                    chat_id: String::new(),
                    verification_code: String::new(),
                    action_type: "choice",
                    selected_option_id: Some(selected.choice_id.as_str().to_owned()),
                    selected_target: None,
                    artifact_ids: Vec::new(),
                    relevance_status: None,
                    note_text_omitted: note.is_some(),
                    summary_text_omitted: false,
                    replayed: false,
                    model_context: String::new(),
                },
            ))
        }
        (
            UserActionInboxForm::EvidenceObservation {
                target_candidates,
                artifact_candidates,
                ..
            },
            PromptUserActionResolution::EvidenceObservation {
                target,
                artifact_ids,
                summary,
                relevance_status,
            },
        ) => {
            let selected_target = select_prompt_target(target_candidates, target)?;
            validate_prompt_artifacts(artifact_candidates, artifact_ids)?;
            let selected_target_text = target_text(&selected_target);
            Ok((
                UserActionResolutionInput::EvidenceObservation {
                    target: selected_target,
                    artifact_ids: artifact_ids.iter().map(ArtifactId::new).collect(),
                    relevance_status: *relevance_status,
                    summary: summary.clone(),
                },
                RecordedPromptUserAction {
                    chat_id: String::new(),
                    verification_code: String::new(),
                    action_type: "evidence_observation",
                    selected_option_id: None,
                    selected_target: Some(selected_target_text),
                    artifact_ids: artifact_ids.clone(),
                    relevance_status: Some(enum_text(*relevance_status)),
                    note_text_omitted: false,
                    summary_text_omitted: true,
                    replayed: false,
                    model_context: String::new(),
                },
            ))
        }
        _ => Err(PromptCommandBlock {
            code: "user_action_form_mismatch",
            message:
                "The submitted resolve flags do not match the stored canonical user-action form."
                    .to_owned(),
        }),
    }
}

fn select_prompt_target(
    candidates: &[EvidenceTarget],
    selector: &PromptEvidenceTarget,
) -> Result<EvidenceTarget, PromptCommandBlock> {
    candidates
        .iter()
        .find(|candidate| match (candidate, selector) {
            (
                EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id,
                },
                PromptEvidenceTarget::AcceptanceCriterion(id),
            ) => acceptance_criterion_id.as_str() == id,
            (
                EvidenceTarget::SupplementalClaim {
                    evidence_claim_id, ..
                },
                PromptEvidenceTarget::SupplementalClaim(id),
            ) => evidence_claim_id.as_str() == id,
            _ => false,
        })
        .cloned()
        .ok_or_else(|| PromptCommandBlock {
            code: "invalid_user_action_command",
            message: "The selected evidence target is not in the stored canonical form.".to_owned(),
        })
}

fn validate_prompt_artifacts(
    candidates: &[volicord_types::ArtifactRef],
    artifact_ids: &[String],
) -> Result<(), PromptCommandBlock> {
    if artifact_ids.iter().all(|id| {
        candidates
            .iter()
            .any(|candidate| candidate.artifact_id.as_str() == id)
    }) {
        Ok(())
    } else {
        Err(PromptCommandBlock {
            code: "invalid_user_action_command",
            message: "Every selected artifact must be in the stored canonical form.".to_owned(),
        })
    }
}

pub(super) fn pending_agent_user_action_summaries(
    store: &CoreProjectStore,
    task_id: &TaskId,
    envelope: &GuardEnvelope,
    now: &UtcTimestamp,
) -> Result<Vec<GuardPendingUserActionSummary>, GuardCommandError> {
    let expected_actor =
        ActorSource::agent_connection(envelope.connection_id.clone()).to_canonical_string();
    let records = store.user_action_records_for_task(task_id, now)?;
    let mut summaries = Vec::new();
    for record in records {
        if record.status != UserActionStatus::Pending
            || record.request.requested_by_actor_source != expected_actor
        {
            continue;
        }
        summaries.push(AgentSafeUserActionRequestSummary::pending(
            UserActionRequestId::new(record.request.user_action_request_id),
        ));
    }
    Ok(summaries)
}

fn target_text(target: &EvidenceTarget) -> String {
    match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => format!("--criterion {acceptance_criterion_id}"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id, ..
        } => format!("--claim {evidence_claim_id}"),
    }
}

fn prompt_block_from_record_response(response: &Value) -> PromptCommandBlock {
    let message = response["errors"]
        .as_array()
        .and_then(|errors| errors.first())
        .and_then(|error| error["message"].as_str())
        .unwrap_or("Core rejected the user-action command.")
        .to_owned();
    PromptCommandBlock {
        code: if message.contains("idempotency_key was reused") {
            "conflicting_user_action_command"
        } else {
            "user_action_resolution_rejected"
        },
        message,
    }
}

fn prompt_block_from_user_error(error: UserCommandError) -> PromptCommandBlock {
    PromptCommandBlock {
        code: "invalid_user_action_command",
        message: error.to_string(),
    }
}

fn prompt_block_from_store_error(error: StoreError) -> PromptCommandBlock {
    PromptCommandBlock {
        code: "store_error",
        message: error.to_string(),
    }
}

fn user_action_verification_code(
    record: &EffectiveUserActionRecord,
    envelope: &GuardEnvelope,
) -> String {
    chat_user_action_verification_code(
        &record.request.project_id,
        &record.request.task_id,
        &record.request.user_action_request_id,
        &record.request.requested_at,
        &envelope.connection_id,
    )
}

fn prompt_user_action_replay_id(
    record: &EffectiveUserActionRecord,
    envelope: &GuardEnvelope,
) -> String {
    let digest = short_digest(&[
        "prompt_user_action_resolve",
        &record.request.project_id,
        &record.request.task_id,
        &record.request.user_action_request_id,
        &record.request.requested_at,
        &envelope.connection_id,
    ]);
    format!("prompt_user_action_{digest}")
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

fn enum_text<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
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
