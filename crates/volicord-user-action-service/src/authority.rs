use crate::{error::UserActionServiceError, model::UserActionAuthority};
use volicord_store::{core_pipeline::EffectiveUserActionRecord, StoreError};
use volicord_types::{
    ids::{ChangeUnitId, ProjectId, TaskId, UserActionRequestId},
    schema::{StateRecordRef, UserActionRequest, UserActionResolutionBody},
    values::{StateRecordKind, UserActionStatus},
};

/// Decodes one effective stored record into normalized UserAction authority facts.
pub fn user_action_authority_from_record(
    record: &EffectiveUserActionRecord,
) -> Result<UserActionAuthority, UserActionServiceError> {
    validate_request_record(record)?;
    let resolution = record
        .resolution
        .as_ref()
        .map(|resolution| resolution.resolution.clone());
    if record.status == UserActionStatus::Resolved && resolution.is_none() {
        return Err(corrupt_value(
            "user_action_requests",
            &record.request.user_action_request_id,
            "resolution",
        ));
    }
    let (machine_action, resolution_outcome) = match resolution.as_ref() {
        Some(UserActionResolutionBody::Choice {
            machine_action,
            resolution_outcome,
            ..
        }) => (Some(*machine_action), Some(*resolution_outcome)),
        _ => (None, None),
    };
    Ok(UserActionAuthority {
        user_action_request_id: record.request.user_action_request_id.clone(),
        user_action_resolution_id: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.user_action_resolution_id.clone()),
        task_id: TaskId::new(record.request.task_id.clone()),
        action_kind: record.request.action_kind,
        status: record.status,
        required_for: record.request.request.required_for.clone(),
        affected_refs: record.request.request.body.affected_refs().to_vec(),
        machine_action,
        resolution_outcome,
        resolved_by_actor_source: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_by_actor_source.clone()),
        resolved_verification_basis: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_verification_basis),
        resolved_assurance_level: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_assurance_level.clone()),
        basis_status: record.request.basis_status,
        basis: Some(record.request.basis.clone()),
        resolution,
        expires_at: record.request.request.expires_at.as_ref().cloned(),
    })
}

/// Projects a newly constructed pending request into method-neutral authority facts.
pub fn user_action_authority_from_state(request: &UserActionRequest) -> UserActionAuthority {
    UserActionAuthority {
        user_action_request_id: request.user_action_request_id.as_str().to_owned(),
        user_action_resolution_id: None,
        task_id: request.task_id.clone(),
        action_kind: request.action_kind,
        status: request.status,
        required_for: request.required_for.clone(),
        affected_refs: request.body.affected_refs().to_vec(),
        machine_action: None,
        resolution_outcome: None,
        resolved_by_actor_source: None,
        resolved_verification_basis: None,
        resolved_assurance_level: None,
        basis_status: request.basis.compatibility_status(),
        basis: Some(request.basis.clone()),
        resolution: None,
        expires_at: request.expires_at.as_ref().cloned(),
    }
}

/// Projects one typed effective Store record into its public request shape.
pub fn user_action_from_record(
    record: &EffectiveUserActionRecord,
    state_version: u64,
) -> Result<UserActionRequest, UserActionServiceError> {
    validate_request_record(record)?;
    let project_id = ProjectId::new(record.request.project_id.clone());
    let task_id = TaskId::new(record.request.task_id.clone());
    let resolution_ref = record.resolution.as_ref().map(|resolution| {
        StateRecordRef::new(
            StateRecordKind::UserActionResolution,
            resolution.user_action_resolution_id.clone(),
            project_id.clone(),
            Some(task_id.clone()),
            Some(state_version),
        )
    });
    Ok(UserActionRequest {
        user_action_request_id: UserActionRequestId::new(
            record.request.user_action_request_id.clone(),
        ),
        project_id,
        task_id,
        change_unit_id: record
            .request
            .change_unit_id
            .clone()
            .map(ChangeUnitId::new)
            .into(),
        action_kind: record.request.action_kind,
        status: record.status,
        body: record.request.request.body.clone(),
        basis: record.request.basis.clone(),
        required_for: record.request.request.required_for.clone(),
        user_action_resolution_ref: resolution_ref.into(),
        expires_at: record.request.request.expires_at.clone(),
        created_at: record.request.requested_at.clone(),
    })
}

fn validate_request_record(
    record: &EffectiveUserActionRecord,
) -> Result<(), UserActionServiceError> {
    if record.request.request.body.action_kind() != record.request.action_kind
        || record.request.basis.compatibility_status() != record.request.basis_status
        || record.request.request.required_for != record.request.required_for
        || record.request.request.expires_at.as_ref() != record.request.expires_at.as_ref()
    {
        return Err(corrupt_json(
            "user_action_requests",
            &record.request.user_action_request_id,
            "request_json",
        ));
    }
    if record
        .resolution
        .as_ref()
        .is_some_and(|resolution| resolution.action_kind != record.request.action_kind)
    {
        return Err(corrupt_value(
            "user_action_resolutions",
            record
                .resolution
                .as_ref()
                .map_or("", |resolution| &resolution.user_action_resolution_id),
            "action_kind",
        ));
    }
    Ok(())
}

fn corrupt_json(
    table: &'static str,
    record_ref: &str,
    logical_column: &'static str,
) -> UserActionServiceError {
    UserActionServiceError::CorruptStoredState(StoreError::corrupt_owner_state_json(
        table,
        record_ref,
        logical_column,
    ))
}

fn corrupt_value(
    table: &'static str,
    record_ref: &str,
    logical_column: &'static str,
) -> UserActionServiceError {
    UserActionServiceError::CorruptStoredState(StoreError::corrupt_owner_state_value(
        table,
        record_ref,
        logical_column,
    ))
}
