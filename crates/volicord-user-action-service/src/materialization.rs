use super::{
    identity::UserActionOrigin,
    model::{UserActionPersistenceContext, ValidatedUserAction},
    persistence::{
        map_user_action_request_persistence, materialize_user_action_resolution_mutation,
        UserActionRequestPersistenceInput,
    },
};
use crate::error::{UserActionInvariantError, UserActionServiceError};
use volicord_store::core_pipeline::{
    CoreStorageMutation, EffectiveUserActionRecord, UserActionResolutionRecord,
};
use volicord_types::{
    ids::{ProjectId, UserActionRequestId, UserActionResolutionId},
    schema::{
        PersistedUserActionRequest, RequiredNullable, StateRecordRef, UserActionRequest,
        UserActionResolutionBody,
    },
    values::{
        ActorSource, StateRecordKind, UserActionChannelKind, UserActionKind, UserActionStatus,
        UserActionVerificationBasis, UtcTimestamp,
    },
};

/// Inputs that add durable operation identity to a validated UserAction.
pub struct UserActionMaterializationInput {
    pub context: UserActionPersistenceContext,
    pub origin: UserActionOrigin,
    pub constructed: ValidatedUserAction,
}

/// One typed public request plus the exact Store mutation that persists it.
#[derive(Debug, Clone)]
pub struct MaterializedUserActionRequest {
    pub request_ref: StateRecordRef,
    pub public_request: UserActionRequest,
    pub effective: EffectiveUserActionRecord,
    pub mutation: CoreStorageMutation,
}

/// Typed semantic values needed to materialize one immutable resolution.
pub struct UserActionResolutionMaterializationInput<'a> {
    pub project_id: &'a ProjectId,
    pub user_action_resolution_id: UserActionResolutionId,
    pub user_action_request_id: &'a UserActionRequestId,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub channel_submission_id: &'a str,
    pub resolution: UserActionResolutionBody,
    pub actor_source: ActorSource,
    pub verification_basis: UserActionVerificationBasis,
    pub assurance_level: String,
    pub resolved_at: &'a UtcTimestamp,
}

/// One immutable resolution record plus its exact Store mutation.
pub struct MaterializedUserActionResolution {
    pub record: UserActionResolutionRecord,
    pub mutation: CoreStorageMutation,
}

/// Adds canonical identity and serializes typed action values at the Store boundary.
pub fn materialize_user_action_request(
    input: UserActionMaterializationInput,
) -> Result<MaterializedUserActionRequest, UserActionServiceError> {
    let UserActionMaterializationInput {
        context,
        origin,
        constructed,
    } = input;
    let ValidatedUserAction {
        task_id,
        coordinate_change_unit_id,
        body,
        basis,
        required_for,
        expires_at,
        created_at,
    } = constructed;
    if context.planned_state_version == 0 {
        return Err(UserActionServiceError::Invariant(
            UserActionInvariantError::ActionFactsMismatch,
        ));
    }
    let action_kind = body.action_kind();
    let persistence_identity = origin.persistence_identity(&context.operation_identity)?;
    let request_id = context.user_action_request_id;
    let request_ref = StateRecordRef::new(
        StateRecordKind::UserActionRequest,
        request_id.as_str(),
        context.project_id.clone(),
        Some(task_id.clone()),
        Some(context.planned_state_version),
    );
    let persisted = PersistedUserActionRequest {
        body: body.clone(),
        required_for: required_for.clone(),
        expires_at: expires_at.clone(),
    };
    let public_request = UserActionRequest {
        user_action_request_id: request_id.clone(),
        project_id: context.project_id.clone(),
        task_id: task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        action_kind,
        status: UserActionStatus::Pending,
        body,
        basis: basis.clone(),
        required_for: required_for.clone(),
        user_action_resolution_ref: RequiredNullable::null(),
        expires_at: expires_at.clone(),
        created_at: created_at.clone(),
    };
    let (effective, mutation) =
        map_user_action_request_persistence(UserActionRequestPersistenceInput {
            project_id: context.project_id.as_str().to_owned(),
            user_action_request_id: request_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: coordinate_change_unit_id.map(|id| id.into_inner()),
            action_kind,
            request: persisted,
            basis,
            required_for,
            requested_by_actor_source: context.actor_source,
            source_method: persistence_identity.source_method,
            source_idempotency_key: persistence_identity.source_idempotency_key,
            requested_at: created_at,
            expires_at: expires_at.into_option(),
            metadata: persistence_identity.metadata,
        })?;
    Ok(MaterializedUserActionRequest {
        request_ref,
        public_request,
        effective,
        mutation,
    })
}

/// Serializes one canonical typed resolution at the Store boundary.
pub fn materialize_user_action_resolution(
    input: UserActionResolutionMaterializationInput<'_>,
) -> Result<MaterializedUserActionResolution, UserActionServiceError> {
    let record = UserActionResolutionRecord {
        project_id: input.project_id.as_str().to_owned(),
        user_action_resolution_id: input.user_action_resolution_id.as_str().to_owned(),
        user_action_request_id: input.user_action_request_id.as_str().to_owned(),
        action_kind: input.action_kind,
        channel_kind: input.channel_kind,
        channel_submission_id: input.channel_submission_id.to_owned(),
        resolution: input.resolution,
        resolved_by_actor_source: input.actor_source,
        resolved_verification_basis: input.verification_basis,
        resolved_assurance_level: input.assurance_level,
        resolved_at: input.resolved_at.clone(),
    };
    let mutation = materialize_user_action_resolution_mutation(record.clone())?;
    Ok(MaterializedUserActionResolution { record, mutation })
}
