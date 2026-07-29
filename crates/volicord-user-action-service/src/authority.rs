use crate::{error::UserActionServiceError, model::UserActionAuthority};
use volicord_store::core_pipeline::StoredUserActionRecordSet;
use volicord_types::{
    ids::{ChangeUnitId, ProjectId, TaskId, UserActionRequestId, UserActionResolutionId},
    schema::{StateRecordRef, UserActionRequest, UserActionResolutionBody},
    values::StateRecordKind,
};

/// Decodes one effective stored record into normalized UserAction authority facts.
pub fn user_action_authority_from_record(
    record: &StoredUserActionRecordSet,
) -> Result<UserActionAuthority, UserActionServiceError> {
    let request = record.request();
    let resolution = record
        .resolution()
        .map(|resolution| resolution.resolution().clone());
    let (machine_action, resolution_outcome) = match resolution.as_ref() {
        Some(UserActionResolutionBody::Choice {
            machine_action,
            resolution_outcome,
            ..
        }) => (Some(*machine_action), Some(*resolution_outcome)),
        _ => (None, None),
    };
    Ok(UserActionAuthority {
        project_id: ProjectId::new(request.project_id()),
        user_action_request_id: UserActionRequestId::new(request.user_action_request_id()),
        user_action_resolution_id: record
            .resolution()
            .map(|resolution| UserActionResolutionId::new(resolution.user_action_resolution_id())),
        task_id: TaskId::new(request.task_id()),
        action_kind: request.action_kind(),
        status: record.status(),
        required_for: request.request().required_for.clone(),
        affected_refs: request.request().body.affected_refs().to_vec(),
        machine_action,
        resolution_outcome,
        resolved_by_actor_source: record
            .resolution()
            .map(|resolution| resolution.resolved_by_actor_source().clone()),
        resolved_verification_basis: record
            .resolution()
            .map(|resolution| resolution.resolved_verification_basis()),
        resolved_assurance_level: record
            .resolution()
            .map(|resolution| resolution.resolved_assurance_level().to_owned()),
        basis_status: request.basis_status(),
        basis: Some(request.basis().clone()),
        resolution,
        expires_at: request.request().expires_at.as_ref().cloned(),
    })
}

/// Projects a newly constructed pending request into method-neutral authority facts.
pub fn user_action_authority_from_state(request: &UserActionRequest) -> UserActionAuthority {
    UserActionAuthority {
        project_id: request.project_id.clone(),
        user_action_request_id: request.user_action_request_id.clone(),
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
    record: &StoredUserActionRecordSet,
    state_version: u64,
) -> Result<UserActionRequest, UserActionServiceError> {
    let stored_request = record.request();
    let project_id = ProjectId::new(stored_request.project_id());
    let task_id = TaskId::new(stored_request.task_id());
    let resolution_ref = record.resolution().map(|resolution| {
        StateRecordRef::new(
            StateRecordKind::UserActionResolution,
            resolution.user_action_resolution_id(),
            project_id.clone(),
            Some(task_id.clone()),
            Some(state_version),
        )
    });
    Ok(UserActionRequest {
        user_action_request_id: UserActionRequestId::new(stored_request.user_action_request_id()),
        project_id,
        task_id,
        change_unit_id: stored_request
            .change_unit_id()
            .map(ChangeUnitId::new)
            .into(),
        action_kind: stored_request.action_kind(),
        status: record.status(),
        body: stored_request.request().body.clone(),
        basis: stored_request.basis().clone(),
        required_for: stored_request.request().required_for.clone(),
        user_action_resolution_ref: resolution_ref.into(),
        expires_at: stored_request.request().expires_at.clone(),
        created_at: stored_request.requested_at().clone(),
    })
}
