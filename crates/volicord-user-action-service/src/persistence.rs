use crate::error::UserActionServiceError;
use volicord_store::core_pipeline::{
    CoreStorageMutation, EffectiveUserActionRecord, UserActionMutation, UserActionRequestInsert,
    UserActionRequestRecord, UserActionResolutionInsert, UserActionResolutionRecord,
};
use volicord_types::{
    schema::{PersistedUserActionRequest, PersistedUserActionRequestMetadata, UserActionBasis},
    values::{
        ActorSource, MethodName, UserActionBasisStatus, UserActionKind, UserActionRequiredFor,
        UserActionStatus, UtcTimestamp,
    },
};

pub(super) struct UserActionRequestPersistenceInput {
    pub project_id: String,
    pub user_action_request_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub action_kind: UserActionKind,
    pub request: PersistedUserActionRequest,
    pub basis: UserActionBasis,
    pub required_for: Vec<UserActionRequiredFor>,
    pub requested_by_actor_source: ActorSource,
    pub source_method: MethodName,
    pub source_idempotency_key: String,
    pub requested_at: UtcTimestamp,
    pub expires_at: Option<UtcTimestamp>,
    pub metadata: PersistedUserActionRequestMetadata,
}

/// Maps one validated immutable resolution record into its Store mutation input.
pub(super) fn materialize_user_action_resolution_mutation(
    record: UserActionResolutionRecord,
) -> Result<CoreStorageMutation, UserActionServiceError> {
    Ok(CoreStorageMutation::UserAction(
        UserActionMutation::InsertResolution(UserActionResolutionInsert {
            user_action_resolution_id: record.user_action_resolution_id,
            user_action_request_id: record.user_action_request_id,
            action_kind: record.action_kind,
            channel_kind: record.channel_kind,
            channel_submission_id: record.channel_submission_id,
            resolution: record.resolution,
            resolved_by_actor_source: record.resolved_by_actor_source,
            resolved_verification_basis: record.resolved_verification_basis,
            resolved_assurance_level: record.resolved_assurance_level,
            resolved_at: record.resolved_at,
        }),
    ))
}

/// Maps one canonical request into the Store record projection and mutation input.
pub(super) fn map_user_action_request_persistence(
    input: UserActionRequestPersistenceInput,
) -> Result<(EffectiveUserActionRecord, CoreStorageMutation), UserActionServiceError> {
    let request_record = UserActionRequestRecord {
        project_id: input.project_id,
        user_action_request_id: input.user_action_request_id.clone(),
        task_id: input.task_id.clone(),
        change_unit_id: input.change_unit_id.clone(),
        action_kind: input.action_kind,
        request: input.request.clone(),
        basis: input.basis.clone(),
        basis_status: UserActionBasisStatus::Current,
        required_for: input.required_for.clone(),
        requested_by_actor_source: input.requested_by_actor_source.clone(),
        source_method: input.source_method,
        source_idempotency_key: input.source_idempotency_key.clone(),
        requested_at: input.requested_at.clone(),
        expires_at: input.expires_at.clone(),
        metadata: input.metadata.clone(),
    };
    let mutation = CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
        UserActionRequestInsert {
            user_action_request_id: input.user_action_request_id,
            task_id: input.task_id,
            change_unit_id: input.change_unit_id,
            action_kind: input.action_kind,
            request: input.request,
            basis: input.basis,
            basis_status: UserActionBasisStatus::Current,
            required_for: input.required_for,
            requested_by_actor_source: input.requested_by_actor_source,
            source_method: input.source_method,
            source_idempotency_key: input.source_idempotency_key,
            requested_at: input.requested_at,
            expires_at: input.expires_at,
            metadata: input.metadata,
        },
    ));
    Ok((
        EffectiveUserActionRecord {
            request: request_record,
            resolution: None,
            status: UserActionStatus::Pending,
        },
        mutation,
    ))
}
