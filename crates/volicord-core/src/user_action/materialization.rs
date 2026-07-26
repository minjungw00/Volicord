use super::identity::{allocate_user_action_request_id, UserActionOrigin};
use super::model::ValidatedUserAction;
use super::persistence::{
    map_user_action_request_persistence, materialize_user_action_resolution_mutation,
    UserActionRequestPersistenceInput,
};
use super::service::user_action_validation_error;
use crate::methods::{state_ref, PlanError};
use crate::pipeline::{
    CorePipelineError, CoreResult, CoreService, VerifiedActorContext, VerifiedInvocationContext,
};
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, EffectiveUserActionRecord, ProjectStateHeader,
    UserActionResolutionRecord,
};
use volicord_types::ids::{ProjectId, UserActionRequestId, UserActionResolutionId};
use volicord_types::schema::{
    PersistedUserActionRequest, RequiredNullable, StateRecordRef, ToolEnvelope, UserActionRequest,
    UserActionResolutionBody,
};
use volicord_types::values::{
    StateRecordKind, UserActionChannelKind, UserActionKind, UserActionStatus, UtcTimestamp,
};

/// Inputs that add operation identity and Store ownership to a validated UserAction.
pub(crate) struct UserActionMaterializationInput<'a> {
    pub(crate) service: &'a CoreService,
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) project_state: &'a ProjectStateHeader,
    pub(crate) verified_invocation: &'a VerifiedInvocationContext,
    pub(crate) envelope: &'a ToolEnvelope,
    pub(crate) origin: UserActionOrigin,
    pub(crate) constructed: ValidatedUserAction,
}

/// One typed public request plus the exact Store mutation that persists it.
#[derive(Debug, Clone)]
pub(crate) struct MaterializedUserActionRequest {
    pub(crate) request_ref: StateRecordRef,
    pub(crate) public_request: UserActionRequest,
    pub(crate) effective: EffectiveUserActionRecord,
    pub(crate) mutation: CoreStorageMutation,
}

/// Typed values needed to materialize one immutable UserAction resolution.
pub(crate) struct UserActionResolutionMaterializationInput<'a> {
    pub(crate) project_id: &'a ProjectId,
    pub(crate) user_action_resolution_id: UserActionResolutionId,
    pub(crate) user_action_request_id: &'a UserActionRequestId,
    pub(crate) action_kind: UserActionKind,
    pub(crate) channel_kind: UserActionChannelKind,
    pub(crate) channel_submission_id: &'a str,
    pub(crate) resolution: UserActionResolutionBody,
    pub(crate) verified_actor: &'a VerifiedActorContext,
    pub(crate) resolved_at: &'a UtcTimestamp,
}

/// One immutable resolution record plus its exact Store mutation.
pub(crate) struct MaterializedUserActionResolution {
    pub(crate) record: UserActionResolutionRecord,
    pub(crate) mutation: CoreStorageMutation,
}

/// Adds canonical identity and serializes typed action values at the Store boundary.
pub(crate) fn materialize_user_action_request(
    input: UserActionMaterializationInput<'_>,
) -> Result<MaterializedUserActionRequest, PlanError> {
    let UserActionMaterializationInput {
        service,
        store,
        project_state,
        verified_invocation,
        envelope,
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
    let action_kind = body.action_kind();
    let Some(source_idempotency_key) = envelope.idempotency_key.as_ref() else {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "envelope.idempotency_key",
            "a committed user-action request requires an idempotency key",
        );
    };
    let persistence_identity = origin
        .persistence_identity(source_idempotency_key)
        .map_err(PlanError::Core)?;
    let request_id = allocate_user_action_request_id(service, |candidate| {
        store
            .user_action_request_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map_err(PlanError::Core)?;
    let request_ref = state_ref(
        StateRecordKind::UserActionRequest,
        request_id.as_str(),
        &envelope.project_id,
        Some(&task_id),
        Some(project_state.state_version + 1),
    );
    let persisted = PersistedUserActionRequest {
        body: body.clone(),
        required_for: required_for.clone(),
        expires_at: expires_at.clone(),
    };
    let request_json = serde_json::to_string(&persisted)?;
    let basis_json = serde_json::to_string(&basis)?;
    let required_for_json = serde_json::to_string(&required_for)?;
    let requested_by_actor_source = verified_invocation.actor_source.to_canonical_string();
    let requested_at = created_at.to_string();
    let stored_expires_at = expires_at.as_ref().map(ToString::to_string);
    let public_request = UserActionRequest {
        user_action_request_id: request_id.clone(),
        project_id: envelope.project_id.clone(),
        task_id: task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        action_kind,
        status: UserActionStatus::Pending,
        body,
        basis,
        required_for,
        user_action_resolution_ref: RequiredNullable::null(),
        expires_at,
        created_at,
    };
    let (effective, mutation) =
        map_user_action_request_persistence(UserActionRequestPersistenceInput {
            project_id: envelope.project_id.as_str().to_owned(),
            user_action_request_id: request_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: coordinate_change_unit_id.map(|id| id.into_inner()),
            action_kind,
            request_json,
            basis_json,
            required_for_json,
            requested_by_actor_source,
            source_method: persistence_identity.source_method.as_str().to_owned(),
            source_idempotency_key: persistence_identity.source_idempotency_key,
            requested_at,
            expires_at: stored_expires_at,
            metadata_json: persistence_identity.metadata_json,
        });
    Ok(MaterializedUserActionRequest {
        request_ref,
        public_request,
        effective,
        mutation,
    })
}

/// Serializes one canonical typed resolution at the Store boundary.
pub(crate) fn materialize_user_action_resolution(
    input: UserActionResolutionMaterializationInput<'_>,
) -> CoreResult<MaterializedUserActionResolution> {
    let record = UserActionResolutionRecord {
        project_id: input.project_id.as_str().to_owned(),
        user_action_resolution_id: input.user_action_resolution_id.as_str().to_owned(),
        user_action_request_id: input.user_action_request_id.as_str().to_owned(),
        action_kind: input.action_kind,
        channel_kind: input.channel_kind,
        channel_submission_id: input.channel_submission_id.to_owned(),
        resolution_json: serde_json::to_string(&input.resolution)?,
        resolved_by_actor_source: input.verified_actor.actor_source.to_canonical_string(),
        resolved_verification_basis: input.verified_actor.verification_basis.clone(),
        resolved_assurance_level: input.verified_actor.assurance_level.clone(),
        resolved_at: input.resolved_at.to_string(),
    };
    let mutation = materialize_user_action_resolution_mutation(record.clone());
    Ok(MaterializedUserActionResolution { record, mutation })
}
