use super::authority::user_action_from_record;
use super::model::{
    PendingUserAction, PendingUserActionFacts, UserActionResolutionAvailability,
    UserActionResolutionFacts, UserActionResolutionFactsBody,
};
use crate::methods::{decode_required_json, state_ref};
use crate::pipeline::{CorePipelineError, CoreResult, CoreService, InvocationContext};
use crate::policy::evidence::state_record_ref_identity_key;
use volicord_store::core_pipeline::{
    CoreProjectStore, EffectiveUserActionRecord, ProjectContinuityRecordRecord, ProjectStateHeader,
    ToolInvocationRecord, UserActionResolutionRecord,
};
use volicord_store::error::{StoreError, StoreResult};
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::methods::ResolveUserActionResult;
use volicord_types::schema::{
    StateRecordRef, UserActionRequest, UserActionRequestBody, UserActionResolution,
    UserActionResolutionBody,
};
use volicord_types::values::{
    ActorSource, EffectKind, MethodName, OperationCategory, ResponseKind, StateRecordKind,
    UserActionChannelKind, UtcTimestamp,
};

pub(crate) fn user_channel_projection_is_authorized(
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

pub(crate) fn user_channel_projection_invocation_is_authorized(
    project_id: &ProjectId,
    invocation: &InvocationContext,
) -> bool {
    project_id == &invocation.project_id
        && invocation.operation_category == OperationCategory::Read
        && invocation.actor_source() == ActorSource::LocalUser
        && invocation.user_channel() == Some(UserActionChannelKind::Cli)
}

pub(crate) fn pending_user_action_fact_records(
    service: &CoreService,
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> StoreResult<
    Option<(
        ProjectStateHeader,
        UtcTimestamp,
        Vec<EffectiveUserActionRecord>,
    )>,
> {
    if !store.task_exists(task_id)? {
        return Ok(None);
    }
    let project_state = store.project_state()?;
    let observed_at = service.project_store_now(store)?;
    let records = store.pending_user_action_records(task_id, &observed_at)?;
    Ok(Some((project_state, observed_at, records)))
}

pub(crate) fn pending_user_action_facts_from_records(
    project_id: ProjectId,
    task_id: TaskId,
    project_state: ProjectStateHeader,
    observed_at: UtcTimestamp,
    records: Vec<EffectiveUserActionRecord>,
) -> CoreResult<PendingUserActionFacts> {
    let actions = records
        .iter()
        .map(|record| {
            let request = user_action_from_record(record, project_state.state_version)?;
            Ok(PendingUserAction {
                request_ref: state_ref(
                    StateRecordKind::UserActionRequest,
                    request.user_action_request_id.as_str(),
                    &request.project_id,
                    Some(&request.task_id),
                    Some(project_state.state_version),
                ),
                request,
                resolution_availability: UserActionResolutionAvailability::from_status(
                    record.status,
                ),
            })
        })
        .collect::<CoreResult<Vec<_>>>()?;
    Ok(PendingUserActionFacts {
        project_id,
        task_id,
        observed_state_version: project_state.state_version,
        observed_at,
        actions,
    })
}

pub(crate) fn user_action_resolution_facts(
    request: &UserActionRequest,
    resolution: &UserActionResolution,
) -> CoreResult<UserActionResolutionFacts> {
    let resolution_summary = match &resolution.body {
        UserActionResolutionBody::Choice {
            selected_option_id,
            machine_action,
            resolution_outcome,
            ..
        } => {
            let UserActionRequestBody::Choice(choice) = &request.body else {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.as_str(),
                        "resolution_json",
                    ),
                ));
            };
            let selected_option_label = choice
                .options
                .iter()
                .find(|option| option.option_id == *selected_option_id)
                .map(|option| option.label.clone())
                .ok_or_else(|| {
                    CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.as_str(),
                        "resolution_json",
                    ))
                })?;
            UserActionResolutionFactsBody::Choice {
                selected_option_id: selected_option_id.clone(),
                selected_option_label,
                machine_action: *machine_action,
                resolution_outcome: *resolution_outcome,
            }
        }
        UserActionResolutionBody::EvidenceObservation { observation } => {
            if !matches!(request.body, UserActionRequestBody::EvidenceObservation(_)) {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.as_str(),
                        "resolution_json",
                    ),
                ));
            }
            UserActionResolutionFactsBody::EvidenceObservation {
                target: observation.target.clone(),
                artifact_refs: observation.output_artifact_refs.clone(),
                relevance_status: observation.relevance_status,
            }
        }
    };
    Ok(UserActionResolutionFacts {
        user_action_resolution_id: resolution.user_action_resolution_id.clone(),
        user_action_request_id: resolution.user_action_request_id.clone(),
        action_kind: resolution.action_kind,
        channel_kind: resolution.channel_kind,
        resolved_at: resolution.resolved_at.clone(),
        resolution: resolution_summary,
    })
}

pub(crate) fn user_action_resolution_replay_projection(
    replay: Option<&ToolInvocationRecord>,
    continuity_records: &[ProjectContinuityRecordRecord],
    resolution: &UserActionResolutionRecord,
    public_resolution: &UserActionResolution,
    resolution_ref: &StateRecordRef,
) -> CoreResult<(StateRecordRef, Vec<StateRecordRef>)> {
    let replay = replay.ok_or_else(|| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            resolution.user_action_resolution_id.clone(),
            "channel_submission_id",
        ))
    })?;
    let replay_ref = format!(
        "{}/{}/{}",
        replay.project_id, replay.tool_name, replay.idempotency_key
    );
    let result: ResolveUserActionResult = decode_required_json(
        "tool_invocations",
        replay_ref.clone(),
        "response_json",
        Some(&replay.response_json),
    )?;
    let exact_replay_context = replay.project_id == resolution.project_id
        && replay.tool_name == MethodName::ResolveUserAction.as_str()
        && replay.idempotency_key == resolution.channel_submission_id
        && replay.actor_source == resolution.resolved_by_actor_source
        && replay.actor_source == ActorSource::LocalUser.to_canonical_string()
        && replay.operation_category == OperationCategory::UserOnly.as_str()
        && replay.verification_basis.as_deref()
            == Some(resolution.resolved_verification_basis.as_str())
        && replay.git_workspace_context_json.is_none();
    let exact_resolution = exact_replay_context
        && result.user_action_resolution == *public_resolution
        && result.user_action_resolution_ref.record_kind == StateRecordKind::UserActionResolution
        && result.user_action_resolution_ref.record_id == resolution_ref.record_id
        && result.user_action_resolution_ref.project_id == resolution_ref.project_id
        && result.user_action_resolution_ref.task_id == resolution_ref.task_id
        && result.base.response_kind == ResponseKind::Result
        && result.base.effect_kind == EffectKind::CoreCommitted
        && !result.base.dry_run
        && result.base.state_version == Some(replay.committed_state_version)
        && result
            .user_action_resolution_ref
            .produced_at_state_version
            .as_ref()
            == Some(&replay.committed_state_version);
    if !exact_resolution {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json("tool_invocations", replay_ref, "response_json"),
        ));
    }
    let resolution_source_ref = state_ref(
        StateRecordKind::UserActionResolution,
        &resolution.user_action_resolution_id,
        &ProjectId::new(resolution.project_id.clone()),
        Some(&public_resolution.task_id),
        Some(replay.committed_state_version),
    );
    let mut expected_derived_refs = Vec::new();
    for record in continuity_records {
        if record.project_id != resolution.project_id
            || record.source_task_id != public_resolution.task_id.as_str()
        {
            return Err(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "project_continuity_records",
                    &record.continuity_record_id,
                    "source_task_id",
                ),
            ));
        }
        let source_refs = decode_required_json::<Vec<StateRecordRef>>(
            "project_continuity_records",
            record.continuity_record_id.clone(),
            "source_refs_json",
            Some(&record.source_refs_json),
        )?;
        if source_refs.first() == Some(&resolution_source_ref) {
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
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json("tool_invocations", replay_ref, "response_json"),
        ));
    }
    Ok((result.user_action_resolution_ref, result.derived_refs))
}
