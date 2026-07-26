use crate::methods::{decode_required_json, parse_owner_storage_value, state_ref};
use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::close_readiness::UserActionAuthority;
use volicord_store::core_pipeline::EffectiveUserActionRecord;
use volicord_store::StoreError;
use volicord_types::ids::{ChangeUnitId, ProjectId, TaskId, UserActionRequestId};
use volicord_types::schema::{
    PersistedUserActionRequest, PersistedUserActionResolution, UserActionBasis, UserActionRequest,
    UserActionResolutionBody,
};
use volicord_types::values::{StateRecordKind, UserActionStatus};

/// Decodes one effective stored record into normalized Core authority facts.
pub(crate) fn user_action_authority_from_record(
    record: &EffectiveUserActionRecord,
) -> CoreResult<UserActionAuthority> {
    let request: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "request_json",
        Some(&record.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "basis_json",
        Some(&record.request.basis_json),
    )?;
    if request.body.action_kind() != record.request.action_kind
        || basis.compatibility_status() != record.request.basis_status
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "request_json",
            ),
        ));
    }
    let resolution = record
        .resolution
        .as_ref()
        .map(|resolution| {
            let body: PersistedUserActionResolution = decode_required_json(
                "user_action_resolutions",
                resolution.user_action_resolution_id.clone(),
                "resolution_json",
                Some(&resolution.resolution_json),
            )?;
            body.validate().map_err(|_| {
                CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                    "user_action_resolutions",
                    resolution.user_action_resolution_id.clone(),
                    "resolution_json",
                ))
            })?;
            if resolution.action_kind != record.request.action_kind {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.clone(),
                        "action_kind",
                    ),
                ));
            }
            Ok(body)
        })
        .transpose()?;
    if record.status == UserActionStatus::Resolved && record.resolution.is_none() {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "resolution",
            ),
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
    let resolved_by_actor_source = record
        .resolution
        .as_ref()
        .map(|resolution| {
            parse_owner_storage_value(
                "user_action_resolutions",
                resolution.user_action_resolution_id.clone(),
                "resolved_by_actor_source",
                &resolution.resolved_by_actor_source,
            )
        })
        .transpose()?;
    Ok(UserActionAuthority {
        user_action_request_id: record.request.user_action_request_id.clone(),
        user_action_resolution_id: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.user_action_resolution_id.clone()),
        task_id: TaskId::new(record.request.task_id.clone()),
        action_kind: record.request.action_kind,
        status: record.status,
        required_for: request.required_for,
        affected_refs: request.body.affected_refs().to_vec(),
        machine_action,
        resolution_outcome,
        resolved_by_actor_source,
        resolved_verification_basis: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_verification_basis.clone()),
        resolved_assurance_level: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_assurance_level.clone()),
        basis_status: record.request.basis_status,
        basis: Some(basis),
        resolution,
        expires_at: request.expires_at.into_option(),
    })
}

/// Projects a newly constructed pending request into method-neutral authority facts.
pub(crate) fn user_action_authority_from_state(request: &UserActionRequest) -> UserActionAuthority {
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

/// Strictly decodes one effective stored record into its public typed request.
pub(crate) fn user_action_from_record(
    record: &EffectiveUserActionRecord,
    state_version: u64,
) -> CoreResult<UserActionRequest> {
    let persisted: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "request_json",
        Some(&record.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "basis_json",
        Some(&record.request.basis_json),
    )?;
    if persisted.body.action_kind() != record.request.action_kind
        || basis.compatibility_status() != record.request.basis_status
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "request_json",
            ),
        ));
    }
    let project_id = ProjectId::new(record.request.project_id.clone());
    let task_id = TaskId::new(record.request.task_id.clone());
    let resolution_ref = record.resolution.as_ref().map(|resolution| {
        state_ref(
            StateRecordKind::UserActionResolution,
            &resolution.user_action_resolution_id,
            &project_id,
            Some(&task_id),
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
        body: persisted.body,
        basis,
        required_for: persisted.required_for,
        user_action_resolution_ref: resolution_ref.into(),
        expires_at: persisted.expires_at,
        created_at: parse_owner_storage_value(
            "user_action_requests",
            record.request.user_action_request_id.clone(),
            "requested_at",
            &record.request.requested_at,
        )?,
    })
}
