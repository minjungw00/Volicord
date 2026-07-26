use volicord_store::core_pipeline::{
    CoreStorageMutation, EffectiveUserActionRecord, UserActionMutation, UserActionRequestInsert,
    UserActionRequestRecord, UserActionResolutionInsert, UserActionResolutionRecord,
};
use volicord_types::values::{UserActionBasisStatus, UserActionKind, UserActionStatus};

pub(super) struct UserActionRequestPersistenceInput {
    pub(super) project_id: String,
    pub(super) user_action_request_id: String,
    pub(super) task_id: String,
    pub(super) change_unit_id: Option<String>,
    pub(super) action_kind: UserActionKind,
    pub(super) request_json: String,
    pub(super) basis_json: String,
    pub(super) required_for_json: String,
    pub(super) requested_by_actor_source: String,
    pub(super) source_method: String,
    pub(super) source_idempotency_key: String,
    pub(super) requested_at: String,
    pub(super) expires_at: Option<String>,
    pub(super) metadata_json: String,
}

/// Maps one validated immutable resolution record into its Store mutation input.
pub(super) fn materialize_user_action_resolution_mutation(
    record: UserActionResolutionRecord,
) -> CoreStorageMutation {
    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
        UserActionResolutionInsert {
            user_action_resolution_id: record.user_action_resolution_id,
            user_action_request_id: record.user_action_request_id,
            action_kind: record.action_kind,
            channel_kind: record.channel_kind,
            channel_submission_id: record.channel_submission_id,
            resolution_json: record.resolution_json,
            resolved_by_actor_source: record.resolved_by_actor_source,
            resolved_verification_basis: record.resolved_verification_basis,
            resolved_assurance_level: record.resolved_assurance_level,
            resolved_at: record.resolved_at,
        },
    ))
}

/// Maps one canonical request into the Store record projection and mutation input.
pub(super) fn map_user_action_request_persistence(
    input: UserActionRequestPersistenceInput,
) -> (EffectiveUserActionRecord, CoreStorageMutation) {
    let request_record = UserActionRequestRecord {
        project_id: input.project_id,
        user_action_request_id: input.user_action_request_id.clone(),
        task_id: input.task_id.clone(),
        change_unit_id: input.change_unit_id.clone(),
        action_kind: input.action_kind,
        request_json: input.request_json.clone(),
        basis_json: input.basis_json.clone(),
        basis_status: UserActionBasisStatus::Current,
        required_for_json: input.required_for_json.clone(),
        requested_by_actor_source: input.requested_by_actor_source.clone(),
        source_method: input.source_method.clone(),
        source_idempotency_key: input.source_idempotency_key.clone(),
        requested_at: input.requested_at.clone(),
        expires_at: input.expires_at.clone(),
        metadata_json: input.metadata_json.clone(),
    };
    let mutation = CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
        UserActionRequestInsert {
            user_action_request_id: input.user_action_request_id,
            task_id: input.task_id,
            change_unit_id: input.change_unit_id,
            action_kind: input.action_kind,
            request_json: input.request_json,
            basis_json: input.basis_json,
            basis_status: UserActionBasisStatus::Current,
            required_for_json: input.required_for_json,
            requested_by_actor_source: input.requested_by_actor_source,
            source_method: input.source_method,
            source_idempotency_key: input.source_idempotency_key,
            requested_at: input.requested_at,
            expires_at: input.expires_at,
            metadata_json: input.metadata_json,
        },
    ));
    (
        EffectiveUserActionRecord {
            request: request_record,
            resolution: None,
            status: UserActionStatus::Pending,
        },
        mutation,
    )
}
