use crate::artifact::{artifact_ref_from_verified_record, persistent_artifact_is_verified_current};
use crate::identity::allocate_artifact_id;
use crate::json_object::object_from_value;
use crate::pipeline::{CorePipelineError, CoreResult, CoreService, VerifiedInvocationContext};
use crate::recording::{recording_store_error, RecordRunInput, RecordingError, RecordingRejection};
use serde_json::json;
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    ArtifactLinkInsert, ArtifactMutation, ArtifactPromotion, ArtifactStagingStatus,
    StoredArtifactStagingRecord,
};
use volicord_types::ids::StorageRef;
use volicord_types::schema::{
    ArtifactInput, ArtifactRef, JsonObject, PersistedArtifactProducer,
    PersistedArtifactProvenanceMetadata, StagedArtifactHandle,
};
use volicord_types::values::{
    ArtifactAvailability, ArtifactInputSourceKind, ArtifactIntegrityStatus, StateRecordKind,
    UtcTimestamp,
};

use super::model::{RecordRunArtifactContext, RecordRunArtifactPlan};

pub(super) fn artifact_input_validation_plan_error<T>(
    input: &ArtifactInput,
    reason: &'static str,
    message: &'static str,
) -> Result<T, RecordingError> {
    Err(RecordingError::Rejected(
        RecordingRejection::ArtifactInput {
            artifact_input_id: input.artifact_input_id.as_str().to_owned(),
            reason,
            message,
        },
    ))
}

fn artifact_input_error(
    input: &ArtifactInput,
    reason: &'static str,
    message: &'static str,
) -> RecordingError {
    RecordingError::Rejected(RecordingRejection::ArtifactInput {
        artifact_input_id: input.artifact_input_id.as_str().to_owned(),
        reason,
        message,
    })
}

fn artifact_missing_error(message: &'static str) -> RecordingError {
    RecordingError::Rejected(RecordingRejection::ArtifactMissing { message })
}

pub(super) fn plan_record_run_artifacts(
    service: &CoreService,
    context: RecordRunArtifactContext<'_>,
) -> Result<Vec<RecordRunArtifactPlan>, RecordingError> {
    let request = context.request;
    let mut input_ids = BTreeSet::new();
    let mut staged_handles = BTreeSet::new();
    let mut plans = Vec::new();
    for input in &request.artifact_inputs {
        if input.artifact_input_id.as_str().trim().is_empty() {
            return artifact_input_validation_plan_error(
                input,
                "staged_handle_not_found",
                "artifact_input_id must not be empty",
            );
        }
        if !input_ids.insert(input.artifact_input_id.as_str()) {
            return artifact_input_validation_plan_error(
                input,
                "staged_handle_not_found",
                "artifact_input_id values must be unique within one request",
            );
        }
        match input.source_kind {
            ArtifactInputSourceKind::StagedArtifact => {
                if input.staged_artifact_handle.is_none() || input.existing_artifact_ref.is_some() {
                    return artifact_input_validation_plan_error(
                        input,
                        "staged_handle_not_found",
                        "staged_artifact inputs must populate only staged_artifact_handle",
                    );
                }
                let handle = input
                    .staged_artifact_handle
                    .as_ref()
                    .expect("checked staged_artifact_handle above");
                if !staged_handles.insert(handle.handle_id.as_str()) {
                    return artifact_input_validation_plan_error(
                        input,
                        "staged_handle_consumed",
                        "a staged artifact handle can be consumed at most once",
                    );
                }
                plans.push(plan_staged_artifact_input(
                    service, &context, input, handle,
                )?);
            }
            ArtifactInputSourceKind::ExistingArtifact => {
                if input.existing_artifact_ref.is_none() || input.staged_artifact_handle.is_some() {
                    return artifact_input_validation_plan_error(
                        input,
                        "staged_handle_not_found",
                        "existing_artifact inputs must populate only existing_artifact_ref",
                    );
                }
                plans.push(plan_existing_artifact_input(
                    &context,
                    input,
                    input
                        .existing_artifact_ref
                        .as_ref()
                        .expect("checked existing_artifact_ref above"),
                )?);
            }
        }
    }
    Ok(plans)
}

pub(super) fn plan_staged_artifact_input(
    service: &CoreService,
    context: &RecordRunArtifactContext<'_>,
    input: &ArtifactInput,
    handle: &StagedArtifactHandle,
) -> Result<RecordRunArtifactPlan, RecordingError> {
    let store = context.store;
    let request = context.request;
    let verified_invocation = context.verified_invocation;
    let run_id = context.run_id;
    let run_ref = context.run_ref;
    if handle.project_id != request.project_id {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_project_mismatch",
            "staged artifact handle belongs to a different project",
        );
    }
    if handle.task_id != request.task_id {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_task_mismatch",
            "staged artifact handle belongs to a different Task",
        );
    }
    if handle.consumed {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_consumed",
            "staged artifact handle is already consumed",
        );
    }

    let record = store
        .artifact_staging_record(handle.handle_id.as_str())
        .map_err(recording_store_error)?
        .ok_or_else(|| {
            artifact_input_error(
                input,
                "staged_handle_not_found",
                "staged artifact handle cannot be found",
            )
        })?;
    validate_staged_artifact_record(
        request,
        verified_invocation,
        input,
        handle,
        &record,
        context.now,
    )?;

    let artifact_id = allocate_artifact_id(service.durable_id_generator(), store)
        .map_err(RecordingError::Core)?;
    let uri = format!(
        "volicord-artifact://{}/{}",
        request.project_id.as_str(),
        artifact_id.as_str()
    );
    let display_name = staged_artifact_display_name(&record)?;
    let content_type = record
        .content_type
        .clone()
        .unwrap_or_else(|| handle.content_type.clone());
    let sha256 = record
        .sha256
        .clone()
        .expect("staged artifact validation ensures sha256 is present");
    let size_bytes = record
        .size_bytes
        .expect("staged artifact validation ensures size_bytes is present");
    let redaction_state = record.redaction_state;
    let artifact_ref = ArtifactRef {
        artifact_id: artifact_id.clone(),
        project_id: request.project_id.clone(),
        task_id: request.task_id.clone(),
        display_name: display_name.clone(),
        content_type: Some(content_type.clone()).into(),
        sha256: Some(sha256.clone()).into(),
        size_bytes: Some(size_bytes).into(),
        integrity_status: ArtifactIntegrityStatus::Verified,
        redaction_state,
        availability: ArtifactAvailability::Available,
        created_by_run_ref: Some(run_ref.clone()).into(),
        created_by_actor_source: Some(record.created_by_actor_source.clone()).into(),
        storage_ref: Some(StorageRef::new(uri.clone())).into(),
    };
    let source_mutation = Some(ArtifactMutation::PromoteStaged(ArtifactPromotion {
        handle_id: handle.handle_id.as_str().to_owned(),
        artifact_id: artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        run_id: run_id.as_str().to_owned(),
        expected_created_by_actor_source: verified_invocation.actor_source.clone(),
        expected_sha256: sha256,
        expected_size_bytes: size_bytes,
        expected_redaction_state: record.redaction_state,
        expected_created_at: record.created_at.clone(),
        expected_expires_at: record.expires_at.clone(),
        uri,
        retention: JsonObject::new(),
        producer: PersistedArtifactProducer {
            display_name: Some(display_name),
            content_type: Some(content_type),
            created_by_actor_source: verified_invocation.actor_source.clone(),
            artifact_input_id: input.artifact_input_id.clone(),
            relation_hint: input.relation_hint.clone(),
            evidence_target: input.evidence_target.clone(),
        },
        metadata: PersistedArtifactProvenanceMetadata {
            source_kind: ArtifactInputSourceKind::StagedArtifact,
        },
    }));
    let run_link = ArtifactMutation::Link(ArtifactLinkInsert {
        artifact_id: artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        owner_record_kind: StateRecordKind::Run,
        owner_record_id: run_id.as_str().to_owned(),
        created_by_run_id: run_id.as_str().to_owned(),
        metadata: artifact_link_metadata(input)?,
    });

    Ok(RecordRunArtifactPlan {
        artifact_ref,
        evidence_target: input.evidence_target.as_ref().cloned(),
        source_mutation,
        run_link,
    })
}

pub(super) fn validate_staged_artifact_record(
    request: &RecordRunInput,
    verified_invocation: &VerifiedInvocationContext,
    input: &ArtifactInput,
    handle: &StagedArtifactHandle,
    record: &StoredArtifactStagingRecord,
    now: &UtcTimestamp,
) -> Result<(), RecordingError> {
    if record.project_id != request.project_id.as_str() {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_project_mismatch",
            "stored staged artifact belongs to a different project",
        );
    }
    if record.task_id != request.task_id.as_str() {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_task_mismatch",
            "stored staged artifact belongs to a different Task",
        );
    }
    if record.created_by_actor_source != verified_invocation.actor_source
        || handle.created_by_actor_source != record.created_by_actor_source
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_actor_source_mismatch",
            "staged artifact provenance does not match the verified actor source",
        );
    }
    if record.status == ArtifactStagingStatus::Consumed {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_consumed",
            "staged artifact handle is already consumed",
        );
    }
    let stored_created_at = &record.created_at;
    let stored_expires_at = &record.expires_at;
    if stored_expires_at <= stored_created_at {
        return Err(RecordingError::Core(CorePipelineError::Invariant {
            detail: format!(
                "typed staged artifact `{}` expires no later than its creation time",
                record.handle_id
            ),
        }));
    }
    if now < stored_created_at {
        return Err(RecordingError::Core(CorePipelineError::Invariant {
            detail: format!(
                "typed staged artifact `{}` was created after the Core observation time",
                record.handle_id
            ),
        }));
    }
    if record.status == ArtifactStagingStatus::Expired || now >= stored_expires_at {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_expired",
            "staged artifact handle is expired",
        );
    }
    if stored_expires_at != &handle.expires_at {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact expiration does not match the submitted handle",
        );
    }
    if record.status != ArtifactStagingStatus::Staged {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_not_found",
            "staged artifact handle is not consumable",
        );
    }
    if record.sha256.as_deref() != Some(handle.sha256.as_str())
        || input
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| record.sha256.as_deref() != Some(expected))
        || record.sha256.is_none()
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact checksum does not match the submitted handle or expectation",
        );
    }
    if record.size_bytes != Some(handle.size_bytes)
        || input
            .expected_size_bytes
            .is_some_and(|expected| record.size_bytes != Some(expected))
        || record.size_bytes.is_none()
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_size_mismatch",
            "staged artifact size does not match the submitted handle or expectation",
        );
    }
    let expected_redaction = input.redaction_state.unwrap_or(handle.redaction_state);
    if record.redaction_state != handle.redaction_state
        || record.redaction_state != expected_redaction
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact redaction_state does not match the submitted handle or expectation",
        );
    }
    if record.content_type.as_deref() != Some(handle.content_type.as_str()) {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact content_type does not match the submitted handle",
        );
    }
    Ok(())
}

pub(super) fn plan_existing_artifact_input(
    context: &RecordRunArtifactContext<'_>,
    input: &ArtifactInput,
    existing_ref: &ArtifactRef,
) -> Result<RecordRunArtifactPlan, RecordingError> {
    let store = context.store;
    let request = context.request;
    let run_id = context.run_id;
    if existing_ref.project_id != request.project_id || existing_ref.task_id != request.task_id {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_project_mismatch",
            "existing artifact ref must belong to the request project and Task",
        );
    }
    let record = store
        .artifact_record(existing_ref.artifact_id.as_str())
        .map_err(recording_store_error)?
        .ok_or_else(|| artifact_missing_error("existing artifact cannot be found"))?;
    let artifact_available = persistent_artifact_is_verified_current(store, &record)?;
    if record.task_id != request.task_id.as_str()
        || record.project_id != request.project_id.as_str()
        || !artifact_available
        || !store
            .artifact_has_task_owner_link(
                existing_ref.artifact_id.as_str(),
                request.task_id.as_str(),
            )
            .map_err(recording_store_error)?
    {
        return Err(artifact_missing_error(
            "existing artifact is not available for this Task",
        ));
    }
    if existing_ref.integrity_status != ArtifactIntegrityStatus::Verified {
        return Err(artifact_missing_error(
            "existing artifact does not have verified integrity facts",
        ));
    }
    let Some(existing_sha256) = existing_ref.sha256.as_ref() else {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "verified existing artifact refs must include sha256",
        );
    };
    let Some(existing_size_bytes) = existing_ref.size_bytes.as_ref().copied() else {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_size_mismatch",
            "verified existing artifact refs must include size_bytes",
        );
    };
    let Some(existing_content_type) = existing_ref.content_type.as_ref() else {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "verified existing artifact refs must include content_type",
        );
    };
    if record.sha256.as_deref() != Some(existing_sha256.as_str())
        || input
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| record.sha256.as_deref() != Some(expected))
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact checksum does not match the stored artifact",
        );
    }
    if record.size_bytes != Some(existing_size_bytes)
        || input
            .expected_size_bytes
            .is_some_and(|expected| record.size_bytes != Some(expected))
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_size_mismatch",
            "existing artifact size does not match the stored artifact",
        );
    }
    if record.content_type.as_deref() != Some(existing_content_type.as_str()) {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact content_type does not match the stored artifact",
        );
    }
    let stored_redaction_state = record.redaction_state;
    let expected_redaction = input
        .redaction_state
        .unwrap_or(existing_ref.redaction_state);
    if stored_redaction_state != existing_ref.redaction_state
        || stored_redaction_state != expected_redaction
    {
        return artifact_input_validation_plan_error(
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact redaction_state does not match the stored artifact",
        );
    }
    let artifact_ref = artifact_ref_from_verified_record(
        store,
        &record,
        Some(existing_ref.display_name.clone()),
        None,
    )?;
    let run_link = ArtifactMutation::Link(ArtifactLinkInsert {
        artifact_id: existing_ref.artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        owner_record_kind: StateRecordKind::Run,
        owner_record_id: run_id.as_str().to_owned(),
        created_by_run_id: run_id.as_str().to_owned(),
        metadata: artifact_link_metadata(input)?,
    });
    Ok(RecordRunArtifactPlan {
        artifact_ref,
        evidence_target: input.evidence_target.as_ref().cloned(),
        source_mutation: None,
        run_link,
    })
}

pub(super) fn staged_artifact_display_name(
    record: &StoredArtifactStagingRecord,
) -> CoreResult<String> {
    Ok(record.display_name.clone())
}

pub(super) fn artifact_link_metadata(input: &ArtifactInput) -> CoreResult<JsonObject> {
    object_from_value(json!({
        "artifact_input_id": input.artifact_input_id.as_str(),
        "source_kind": input.source_kind,
        "relation_hint": input.relation_hint,
        "evidence_target": input.evidence_target
    }))
}
