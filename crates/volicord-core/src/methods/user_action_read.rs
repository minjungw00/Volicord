//! Adapter-neutral UserAction reads and originating-result replay.

use crate::methods::{decode_semantic_replay_result, state_ref};
use crate::pipeline::{
    operation_result_ref, CorePipelineError, CoreResult, CoreService, FreshnessPolicy,
    InvocationContext, MethodEffectPolicy, MethodPolicy, PipelineResponse, ReplayPolicy,
    TaskRequirement,
};
use crate::policy::evidence::state_record_ref_identity_key;
use volicord_store::core_pipeline::{
    CoreProjectStore, ProjectContinuityRecordRecord, StoredUserActionResolution,
    ToolInvocationRecord,
};
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestId, TaskId, UserActionRequestId};
use volicord_types::methods::{MethodResultBase, RequestUserActionResult, ResolveUserActionResult};
use volicord_types::schema::{
    AgentSafeUserActionRequestSummary, RequiredNullable, StateRecordRef, ToolEnvelope,
    UserActionResolution,
};
use volicord_types::values::{
    ActorSource, EffectKind, MethodName, OperationCategory, StateRecordKind, UserActionChannelKind,
    UserActionStatus,
};
use volicord_user_action_service::{
    pending_user_action_facts_from_records, user_action_from_record, user_action_resolution_facts,
    user_action_resolution_from_record, CurrentUserActionFacts, CurrentUserActionRead,
    CurrentUserActionUnavailableReason, PendingUserActionFacts, PendingUserActionFactsRequest,
    PendingUserActionResolutionSnapshot, UserActionResolutionAvailability,
};

fn user_channel_projection_is_authorized(
    service: &CoreService,
    store: &CoreProjectStore,
    project_id: &ProjectId,
    invocation: &InvocationContext,
) -> bool {
    let same_runtime_home = match (
        store.canonical_runtime_home(),
        service.admitted_runtime_home(),
    ) {
        (Some(store_runtime_home), Some(service_runtime_home)) => {
            store_runtime_home == service_runtime_home
        }
        (None, None) => store.runtime_home() == service.runtime_home(),
        _ => false,
    };
    same_runtime_home
        && store.project_record().project_internal_id == project_id.as_str()
        && user_channel_projection_invocation_is_authorized(project_id, invocation)
}

fn user_channel_projection_invocation_is_authorized(
    project_id: &ProjectId,
    invocation: &InvocationContext,
) -> bool {
    project_id == &invocation.project_id
        && invocation.operation_category == OperationCategory::Read
        && invocation.actor_source() == ActorSource::LocalUser
        && invocation.user_channel() == Some(UserActionChannelKind::Cli)
}

fn user_action_resolution_replay_projection(
    replay: Option<&ToolInvocationRecord>,
    continuity_records: &[ProjectContinuityRecordRecord],
    resolution: &StoredUserActionResolution,
    public_resolution: &UserActionResolution,
    resolution_ref: &StateRecordRef,
) -> CoreResult<(StateRecordRef, Vec<StateRecordRef>)> {
    let replay = replay.ok_or_else(|| CorePipelineError::Invariant {
        detail: format!(
            "resolved user action `{}` has no originating semantic replay result",
            resolution.user_action_resolution_id()
        ),
    })?;
    let replay_ref = format!(
        "{}/{}/{}",
        replay.project_id,
        replay.tool_name.as_str(),
        replay.idempotency_key.as_str()
    );
    let result: ResolveUserActionResult =
        decode_semantic_replay_result(&replay_ref, &replay.response_json)?;
    let exact_replay_context = replay.project_id == resolution.project_id()
        && replay.tool_name == MethodName::ResolveUserAction
        && replay.idempotency_key.as_str() == resolution.channel_submission_id()
        && &replay.actor_source == resolution.resolved_by_actor_source()
        && replay.actor_source == ActorSource::LocalUser
        && replay.operation_category == OperationCategory::UserOnly
        && replay.verification_basis.as_deref()
            == Some(resolution.resolved_verification_basis().as_str())
        && replay.git_workspace_context.is_none();
    let exact_resolution = exact_replay_context
        && result.user_action_resolution == *public_resolution
        && result.user_action_resolution_ref.record_kind == StateRecordKind::UserActionResolution
        && result.user_action_resolution_ref.record_id == resolution_ref.record_id
        && result.user_action_resolution_ref.project_id == resolution_ref.project_id
        && result.user_action_resolution_ref.task_id == resolution_ref.task_id
        && result.base.effect_kind() == EffectKind::CoreCommitted
        && result.base.dry_run_intent().is_not_requested()
        && result.base.state_version() == replay.committed_state_version
        && result
            .user_action_resolution_ref
            .produced_at_state_version
            .as_ref()
            == Some(&replay.committed_state_version);
    if !exact_resolution {
        return Err(CorePipelineError::Invariant {
            detail: format!(
                "resolve_user_action replay `{replay_ref}` contradicts its typed authority facts"
            ),
        });
    }
    let resolution_source_ref = state_ref(
        StateRecordKind::UserActionResolution,
        resolution.user_action_resolution_id(),
        &ProjectId::new(resolution.project_id()),
        Some(&public_resolution.task_id),
        Some(replay.committed_state_version),
    );
    let mut expected_derived_refs = Vec::new();
    for record in continuity_records {
        if record.project_id != resolution.project_id()
            || record.source_task_id != public_resolution.task_id.as_str()
        {
            return Err(CorePipelineError::Invariant {
                detail: format!(
                    "continuity record `{}` contradicts its resolved user-action source",
                    record.continuity_record_id
                ),
            });
        }
        if record.source_refs.first() == Some(&resolution_source_ref) {
            expected_derived_refs.push(state_ref(
                StateRecordKind::ProjectContinuityRecord,
                &record.continuity_record_id,
                &ProjectId::new(record.project_id.clone()),
                Some(&TaskId::new(record.source_task_id.clone())),
                Some(replay.committed_state_version),
            ));
        }
    }
    let mut actual_derived_refs = result.derived_refs.clone();
    expected_derived_refs.sort_by_key(state_record_ref_identity_key);
    actual_derived_refs.sort_by_key(state_record_ref_identity_key);
    if actual_derived_refs != expected_derived_refs {
        return Err(CorePipelineError::Invariant {
            detail: format!(
                "resolve_user_action replay `{replay_ref}` has inconsistent derived references"
            ),
        });
    }
    Ok((result.user_action_resolution_ref, result.derived_refs))
}

impl CoreService {
    /// Reads pending user-action semantic facts for an authenticated User
    /// Channel consumer.
    ///
    /// This is an internal, nonserialized boundary. Serialized public outputs
    /// use their separate safe summary shape.
    pub fn pending_user_action_facts(
        &self,
        request: PendingUserActionFactsRequest,
        invocation: InvocationContext,
    ) -> CoreResult<Option<PendingUserActionFacts>> {
        if !user_channel_projection_invocation_is_authorized(&request.project_id, &invocation) {
            return Ok(None);
        }
        let store = CoreProjectStore::open_read_only(self.runtime_home(), &request.project_id)?;
        self.pending_user_action_facts_from_store(&store, request, invocation)
    }

    /// Reads pending user-action semantic facts from an already-open project Store.
    ///
    /// The Store owns the complete SQLite read snapshot. This lets an admitted
    /// local User Channel operation reuse its mutation-capable handle without
    /// opening an independent read-only connection.
    pub fn pending_user_action_facts_from_store(
        &self,
        store: &CoreProjectStore,
        request: PendingUserActionFactsRequest,
        invocation: InvocationContext,
    ) -> CoreResult<Option<PendingUserActionFacts>> {
        if !user_channel_projection_is_authorized(self, store, &request.project_id, &invocation) {
            return Ok(None);
        }

        let Some((project_state, observed_at, records)) = store.with_read_snapshot(|snapshot| {
            if !snapshot.task_exists(&request.task_id)? {
                return Ok(None);
            }
            let project_state = snapshot.project_state()?;
            let observed_at = self.project_store_now(snapshot)?;
            let records = snapshot.pending_user_action_records(&request.task_id, &observed_at)?;
            Ok(Some((project_state, observed_at, records)))
        })?
        else {
            return Ok(None);
        };
        Ok(Some(pending_user_action_facts_from_records(
            request.project_id,
            request.task_id,
            project_state,
            observed_at,
            records,
        )?))
    }

    /// Reads the exact effective request and pending semantic facts from one Store
    /// snapshot for an admitted local User Channel resolution.
    pub fn pending_user_action_resolution_snapshot_from_store(
        &self,
        store: &CoreProjectStore,
        user_action_request_id: &UserActionRequestId,
        invocation: InvocationContext,
    ) -> CoreResult<Option<PendingUserActionResolutionSnapshot>> {
        let project_id = invocation.project_id.clone();
        if !user_channel_projection_is_authorized(self, store, &project_id, &invocation) {
            return Ok(None);
        }

        let Some((project_state, observed_at, record, pending_records)) = store
            .with_read_snapshot(|snapshot| {
                let project_state = snapshot.project_state()?;
                let observed_at = self.project_store_now(snapshot)?;
                let Some(record) =
                    snapshot.user_action_record(user_action_request_id.as_str(), &observed_at)?
                else {
                    return Ok(None);
                };
                let pending_records = if record.status() == UserActionStatus::Pending
                    && snapshot.task_exists(&TaskId::new(record.request().task_id()))?
                {
                    Some(snapshot.pending_user_action_records(
                        &TaskId::new(record.request().task_id()),
                        &observed_at,
                    )?)
                } else {
                    None
                };
                Ok(Some((project_state, observed_at, record, pending_records)))
            })?
        else {
            return Ok(None);
        };
        let pending_actions = pending_records
            .map(|records| {
                pending_user_action_facts_from_records(
                    project_id.clone(),
                    TaskId::new(record.request().task_id()),
                    project_state.clone(),
                    observed_at.clone(),
                    records,
                )
            })
            .transpose()?;
        Ok(Some(PendingUserActionResolutionSnapshot {
            project_id,
            observed_state_version: project_state.state_version,
            observed_at,
            resolution_availability: UserActionResolutionAvailability::from_status(record.status()),
            record,
            pending_actions,
        }))
    }

    /// Reads current effective lifecycle and safe resolution facts for one
    /// user-action request without replaying either mutation.
    pub fn current_user_action_facts(
        &self,
        project_id: &ProjectId,
        user_action_request_id: &UserActionRequestId,
    ) -> CoreResult<CurrentUserActionRead> {
        let store = CoreProjectStore::open_read_only(self.runtime_home(), project_id)?;
        let (project_state, observed_at, record, resolution_replay, continuity_records) = store
            .with_read_snapshot(|store| {
                let project_state = store.project_state()?;
                let observed_at = self.project_store_now(store)?;
                let record =
                    store.user_action_record(user_action_request_id.as_str(), &observed_at)?;
                let resolution_replay = record
                    .as_ref()
                    .and_then(|record| record.resolution())
                    .map(|resolution| {
                        store.tool_invocation(
                            MethodName::ResolveUserAction,
                            &IdempotencyKey::new(resolution.channel_submission_id()),
                        )
                    })
                    .transpose()?
                    .flatten();
                let continuity_records = match record.as_ref() {
                    Some(record) if record.resolution().is_some() => {
                        store.project_continuity_records_for_task(record.request().task_id())?
                    }
                    _ => Vec::new(),
                };
                Ok((
                    project_state,
                    observed_at.clone(),
                    record,
                    resolution_replay,
                    continuity_records,
                ))
            })?;
        let Some(record) = record else {
            return Ok(CurrentUserActionRead::Unavailable(
                CurrentUserActionUnavailableReason::NotFound,
            ));
        };
        let request = user_action_from_record(&record, project_state.state_version)?;
        let (resolution_ref, resolution, derived_refs) = match record.resolution() {
            Some(stored_resolution) => {
                let public_resolution =
                    user_action_resolution_from_record(stored_resolution, &request.task_id)?;
                let resolution_ref = request
                    .user_action_resolution_ref
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| CorePipelineError::Invariant {
                        detail: format!(
                            "resolved user action `{}` has no typed resolution reference",
                            user_action_request_id.as_str()
                        ),
                    })?;
                let safe = user_action_resolution_facts(&request, &public_resolution)?;
                let (resolution_ref, derived_refs) = user_action_resolution_replay_projection(
                    resolution_replay.as_ref(),
                    &continuity_records,
                    stored_resolution,
                    &public_resolution,
                    &resolution_ref,
                )?;
                (Some(resolution_ref), Some(safe), derived_refs)
            }
            None => (None, None, Vec::new()),
        };
        Ok(CurrentUserActionRead::Available(Box::new(
            CurrentUserActionFacts {
                project_id: project_id.clone(),
                user_action_request_id: user_action_request_id.clone(),
                action_kind: record.request().action_kind(),
                observed_state_version: project_state.state_version,
                observed_at,
                status: record.status(),
                resolution_availability: UserActionResolutionAvailability::from_status(
                    record.status(),
                ),
                user_action_resolution_ref: resolution_ref,
                user_action_resolution: resolution,
                derived_refs,
            },
        )))
    }

    /// Replays the exact originating `request_user_action` result for an
    /// access-matching Agent Connection without creating a second request.
    pub fn resume_user_action_request(
        &self,
        project_id: ProjectId,
        user_action_request_id: UserActionRequestId,
        invocation: InvocationContext,
    ) -> CoreResult<Option<PipelineResponse>> {
        let store = CoreProjectStore::open_read_only(self.runtime_home(), &project_id)?;
        let (project_state, record, origin_replay) = store.with_read_snapshot(|store| {
            let project_state = store.project_state()?;
            let now = self.project_store_now(store)?;
            let record = store.user_action_record(user_action_request_id.as_str(), &now)?;
            let origin_replay = record
                .as_ref()
                .filter(|record| record.request().source_method() == MethodName::RequestUserAction)
                .map(|record| {
                    store.tool_invocation(
                        MethodName::RequestUserAction,
                        &IdempotencyKey::new(record.request().source_idempotency_key()),
                    )
                })
                .transpose()?
                .flatten();
            Ok((project_state, record, origin_replay))
        })?;
        let Some(record) = record else {
            return Ok(None);
        };
        if record.request().source_method() != MethodName::RequestUserAction {
            return Ok(None);
        }
        let task_id = TaskId::new(record.request().task_id());
        let envelope = ToolEnvelope {
            project_id: project_id.clone(),
            task_id: Some(task_id.clone()).into(),
            request_id: RequestId::new("req_internal_user_action_resume"),
            idempotency_key: RequiredNullable::null(),
            expected_state_version: RequiredNullable::null(),
            dry_run: volicord_types::schema::DryRunIntent::NotRequested,
            locale: RequiredNullable::null(),
        };
        let policy = MethodPolicy::exact(
            OperationCategory::AgentWorkflow,
            TaskRequirement::Exact(task_id.clone()),
            ReplayPolicy::None,
            FreshnessPolicy::None,
            MethodEffectPolicy::ReadOnly,
        );
        let Ok(verified_invocation) = crate::policy::access::derive_verified_invocation(
            &project_state,
            &envelope,
            &invocation,
            &policy,
        ) else {
            return Ok(None);
        };
        let source_idempotency_key = IdempotencyKey::new(record.request().source_idempotency_key());
        let replay = origin_replay.ok_or_else(|| CorePipelineError::Invariant {
            detail: format!(
                "user action `{}` has no originating semantic replay result",
                user_action_request_id.as_str()
            ),
        })?;
        if replay.operation_category != OperationCategory::AgentWorkflow
            || replay.actor_source != verified_invocation.actor_source
        {
            return Ok(None);
        }
        if &replay.actor_source != record.request().requested_by_actor_source() {
            return Err(CorePipelineError::Invariant {
                detail: format!(
                    "user action `{}` contradicts its originating actor authority",
                    user_action_request_id.as_str()
                ),
            });
        }
        if !crate::pipeline::stored_public_response_is_current(
            MethodName::RequestUserAction,
            &replay.response_json,
            replay.committed_state_version,
        ) {
            return crate::pipeline::stored_response_corrupt_response(
                envelope.dry_run,
                project_state.state_version,
                Some(verified_invocation),
                Some(task_id),
            )
            .map(Some);
        }
        let replay_identity = format!(
            "{}/{}/{}",
            replay.project_id,
            replay.tool_name.as_str(),
            replay.idempotency_key.as_str()
        );
        let result: RequestUserActionResult =
            decode_semantic_replay_result(&replay_identity, &replay.response_json)?;
        let exact_origin = replay.project_id == project_id.as_str()
            && replay.tool_name == MethodName::RequestUserAction
            && replay.idempotency_key == source_idempotency_key
            && replay.operation_category == OperationCategory::AgentWorkflow
            && &replay.actor_source == record.request().requested_by_actor_source()
            && replay.committed_state_version > replay.basis_state_version
            && result.base.effect_kind() == EffectKind::CoreCommitted
            && result.base.dry_run_intent().is_not_requested()
            && result.base.state_version() == replay.committed_state_version
            && result.user_action_request_summary
                == AgentSafeUserActionRequestSummary::pending(user_action_request_id.clone());
        if !exact_origin {
            return Err(CorePipelineError::Invariant {
                detail: format!(
                    "request_user_action replay `{replay_identity}` contradicts its typed authority facts"
                ),
            });
        }
        let response_value = serde_json::from_str(&replay.response_json)?;
        let result_ref = operation_result_ref(
            &replay.response_json,
            &project_id,
            MethodName::RequestUserAction,
            Some(&source_idempotency_key),
            replay.committed_state_version,
            &verified_invocation,
        );
        Ok(Some(PipelineResponse {
            response_json: replay.response_json,
            response_value,
            operation_result_ref: result_ref,
            verified_invocation: Some(verified_invocation),
            resolved_task_id: Some(task_id),
            replayed: true,
        }))
    }
}
