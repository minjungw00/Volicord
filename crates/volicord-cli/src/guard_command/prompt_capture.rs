use std::path::Path;

use serde_json::{json, Value};
use volicord_store::{
    bootstrap::ProjectRecord,
    core_pipeline::CoreProjectStore,
    guards::{
        guard_health_record, insert_prompt_capture, prompt_capture, prompt_capture_availability,
        PromptCaptureAvailability, PromptCaptureInsert,
    },
};
use volicord_types::{
    ActorSource, AgentSafeUserActionRequestSummary, GuardDecision, PromptCaptureStatus, TaskId,
    UserActionRequestId, UserActionStatus, UtcTimestamp,
};

use crate::guard_integration::host_hook_capability_has_exact_current_shape;

use super::{
    args::GuardInput,
    current_policy_hash,
    envelope::{event_string, GuardEnvelope},
    json_error, sha256_text, stable_id, GuardCommandError,
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
    if !host_hook_capability_has_exact_current_shape(&value) {
        return Err(GuardCommandError::Runtime(
            "stored host-hook capability is not current input".to_owned(),
        ));
    }
    Ok(value
        .get("policy_hash")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned))
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
                    "prompt_text_omitted": true,
                    "resolution_channel": "cli_inbox_only"
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
    let capture = record_prompt_capture(runtime_home, project, envelope, input)?;
    let available = availability.is_operational();
    let decision = if available {
        GuardDecision::Allow
    } else {
        GuardDecision::Warn
    };
    let model_context = (!available).then(|| {
        format!(
            "Volicord recorded this prompt observation, but the prompt-capture hook is {}. User-owned actions can be resolved only through the CLI inbox.",
            availability.status.as_str()
        )
    });
    Ok((
        decision,
        json!({
            "decision": decision.as_str(),
            "allowed": true,
            "prompt_capture": capture,
            "prompt_capture_status": availability.status.as_str(),
            "host_supports_prompt_capture": availability.host_supports_prompt_capture,
            "prompt_capture_configured": availability.prompt_capture_configured,
            "resolution_channel": "cli_inbox_only",
            "model_context": model_context,
            "enforcement_level": "cooperative_guard"
        }),
        false,
    ))
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
