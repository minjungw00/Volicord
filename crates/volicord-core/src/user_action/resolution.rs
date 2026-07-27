use super::service::{
    canonical_user_action_artifacts, user_action_validation_error, validate_user_action_target,
};
use crate::methods::{
    decision_rejected_response, decode_required_json, no_active_change_unit_response,
    normalize_display_text, parse_owner_storage_value, validation_rejected, PlanError, StoredScope,
};
use crate::pipeline::{CorePipelineError, CoreResult, VerifiedInvocationContext};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ProjectStateHeader, TaskRecord, UserActionResolutionRecord,
};
use volicord_store::StoreError;
use volicord_types::ids::{
    BaselineRef, ChangeUnitId, ProjectId, TaskId, UserActionRequestId, UserActionResolutionId,
};
use volicord_types::methods::ResolveUserActionRequest;
use volicord_types::schema::{
    ArtifactRef, PersistedUserActionResolution, StateRecordRef, UserActionBasis,
    UserActionEvidenceObservation, UserActionRequestBody, UserActionResolution,
    UserActionResolutionBody, UserActionResolutionInput,
};
use volicord_types::values::{
    EvidenceRelevanceStatus, JudgmentKind, UserActionBasisStatus, UserActionChannelKind,
    UserActionOptionAction, UserActionVerificationBasis,
};

pub(crate) fn channel_kind_from_verified_invocation(
    invocation: &VerifiedInvocationContext,
) -> Option<UserActionChannelKind> {
    UserActionVerificationBasis::parse(&invocation.verification_basis)
        .map(UserActionChannelKind::from_verification_basis)
}

pub(crate) fn validate_current_resolution_basis(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ResolveUserActionRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    basis: &UserActionBasis,
) -> Result<(), PlanError> {
    let coordinates = basis.coordinates();
    let current_scope = StoredScope::from_task(task)?;
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    if basis.compatibility_status() != UserActionBasisStatus::Current
        || coordinates.task_id.as_str() != task.task_id
        || coordinates.scope_revision != task.scope_revision
        || coordinates.created_at_state_version > project_state.state_version
        || coordinates.baseline_ref.as_ref().map(BaselineRef::as_str)
            != current_scope.baseline_ref.as_deref()
        || coordinates.change_unit_id.as_ref() != current_change_unit_id.as_ref()
    {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "user-action basis is not current for this resolution",
        ))));
    }
    if let Some(close_basis_revision) = basis.close_basis_revision() {
        let current = store
            .task_revision_record(&TaskId::new(task.task_id.clone()))
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.close_basis_revision == close_basis_revision);
        if !current {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "user-action close basis is no longer current",
            ))));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn construct_user_action_resolution(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ResolveUserActionRequest,
    request_body: &UserActionRequestBody,
    basis: &UserActionBasis,
    task_id: &TaskId,
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<(UserActionResolutionBody, Vec<StateRecordRef>), PlanError> {
    match (request_body, &request.resolution) {
        (
            UserActionRequestBody::Choice(choice),
            UserActionResolutionInput::Choice {
                selected_option_id,
                note,
            },
        ) => {
            let selected = choice
                .options
                .iter()
                .find(|option| option.option_id == *selected_option_id)
                .ok_or_else(|| {
                    PlanError::Response(Box::new(
                        validation_rejected(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            "resolution.selected_option_id",
                            "selected option must belong to the stored user-action request",
                        )
                        .expect("validation response should serialize"),
                    ))
                })?;
            let accepted_risk_ids = if choice.judgment_kind == JudgmentKind::ResidualRiskAcceptance
                && selected.machine_action == UserActionOptionAction::Accept
            {
                basis.residual_risk_ids().to_vec()
            } else {
                Vec::new()
            };
            Ok((
                UserActionResolutionBody::Choice {
                    selected_option_id: selected.option_id.clone(),
                    machine_action: selected.machine_action,
                    resolution_outcome: selected.resolution_outcome,
                    note: note.clone(),
                    accepted_risk_ids,
                },
                Vec::new(),
            ))
        }
        (
            UserActionRequestBody::EvidenceObservation(observation_request),
            UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids,
                relevance_status,
                summary,
            },
        ) => {
            if !matches!(
                relevance_status,
                EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
            ) {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.relevance_status",
                    "user observation relevance must be supported or contradicted",
                );
            }
            let normalized_summary = normalize_display_text(summary);
            if normalized_summary.is_empty() {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.summary",
                    "user observation summary must be non-empty",
                );
            }
            if !observation_request.target_candidates.contains(target) {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.target",
                    "observation target must be one of the stored candidates",
                );
            }
            if artifact_ids.iter().collect::<BTreeSet<_>>().len() != artifact_ids.len() {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.artifact_ids",
                    "observation artifact IDs must not contain duplicates",
                );
            }
            let candidate_ids = observation_request
                .artifact_candidates
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect::<BTreeSet<_>>();
            if artifact_ids
                .iter()
                .any(|artifact_id| !candidate_ids.contains(artifact_id))
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.artifact_ids",
                    "observation artifacts must be selected from the stored candidates",
                );
            }
            validate_user_action_target(
                store,
                project_state,
                &request.envelope,
                task_id,
                target,
                "resolution.target",
            )?;
            let output_artifact_refs = canonical_user_action_artifacts(
                store,
                project_state,
                &request.envelope,
                task_id,
                artifact_ids,
                "resolution.artifact_ids",
            )?;
            let selected_ids = artifact_ids.iter().collect::<BTreeSet<_>>();
            let stored_selected_refs = observation_request
                .artifact_candidates
                .iter()
                .filter(|artifact| selected_ids.contains(&artifact.artifact_id))
                .cloned()
                .collect::<Vec<_>>();
            if !current_artifact_refs_preserve_candidates(
                &stored_selected_refs,
                &output_artifact_refs,
            ) {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "selected observation artifact changed after the request was created",
                ))));
            }
            let coordinates = basis.coordinates();
            let _change_unit_id = current_change_unit
                .map(|record| ChangeUnitId::new(record.change_unit_id.clone()))
                .ok_or_else(|| {
                    PlanError::Response(Box::new(no_active_change_unit_response(
                        &request.envelope,
                        Some(project_state.state_version),
                        "evidence observation resolution requires the current Change Unit",
                    )))
                })?;
            let _baseline_ref = coordinates.baseline_ref.as_ref().cloned().ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "evidence observation resolution requires a current baseline",
                )))
            })?;
            Ok((
                UserActionResolutionBody::EvidenceObservation {
                    observation: UserActionEvidenceObservation {
                        target: target.clone(),
                        relevance_status: *relevance_status,
                        output_artifact_refs: stored_selected_refs,
                        summary: normalized_summary,
                    },
                },
                Vec::new(),
            ))
        }
        _ => user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "resolution.resolution_type",
            "resolution type must match the stored user-action request",
        ),
    }
}

pub(crate) fn user_action_resolution_from_record(
    record: &UserActionResolutionRecord,
    task_id: &TaskId,
) -> CoreResult<UserActionResolution> {
    let body: PersistedUserActionResolution = decode_required_json(
        "user_action_resolutions",
        record.user_action_resolution_id.clone(),
        "resolution_json",
        Some(&record.resolution_json),
    )?;
    body.validate().map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
            "user_action_resolutions",
            record.user_action_resolution_id.clone(),
            "resolution_json",
        ))
    })?;
    Ok(UserActionResolution {
        user_action_resolution_id: UserActionResolutionId::new(
            record.user_action_resolution_id.clone(),
        ),
        user_action_request_id: UserActionRequestId::new(record.user_action_request_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: task_id.clone(),
        action_kind: record.action_kind,
        body,
        resolved_by_actor_source: parse_owner_storage_value(
            "user_action_resolutions",
            record.user_action_resolution_id.clone(),
            "resolved_by_actor_source",
            &record.resolved_by_actor_source,
        )?,
        resolved_verification_basis: record.resolved_verification_basis,
        resolved_assurance_level: record.resolved_assurance_level.clone(),
        channel_kind: record.channel_kind,
        channel_submission_id: record.channel_submission_id.clone(),
        resolved_at: parse_owner_storage_value(
            "user_action_resolutions",
            record.user_action_resolution_id.clone(),
            "resolved_at",
            &record.resolved_at,
        )?,
    })
}

pub(crate) fn resolution_input_matches_body(
    input: &UserActionResolutionInput,
    body: &UserActionResolutionBody,
) -> bool {
    match (input, body) {
        (
            UserActionResolutionInput::Choice {
                selected_option_id,
                note,
            },
            UserActionResolutionBody::Choice {
                selected_option_id: stored_id,
                note: stored_note,
                ..
            },
        ) => selected_option_id == stored_id && note == stored_note,
        (
            UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids,
                relevance_status,
                summary,
            },
            UserActionResolutionBody::EvidenceObservation { observation },
        ) => {
            let mut input_ids = artifact_ids.clone();
            let mut stored_ids = observation
                .output_artifact_refs
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect::<Vec<_>>();
            input_ids.sort();
            stored_ids.sort();
            target == &observation.target
                && relevance_status == &observation.relevance_status
                && normalize_display_text(summary) == observation.summary
                && input_ids == stored_ids
        }
        _ => false,
    }
}

/// Compares immutable request candidates with their current projection.
fn current_artifact_refs_preserve_candidates(left: &[ArtifactRef], right: &[ArtifactRef]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut candidates = left.iter().collect::<Vec<_>>();
    let mut current = right.iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    current.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    candidates
        .into_iter()
        .zip(current)
        .all(|(candidate, current)| {
            let mut normalized_current = current.clone();
            match (
                candidate.created_by_run_ref.as_ref(),
                normalized_current.created_by_run_ref.as_mut(),
            ) {
                (Some(candidate_run), Some(current_run)) => {
                    current_run.produced_at_state_version = candidate_run
                        .produced_at_state_version
                        .as_ref()
                        .copied()
                        .into();
                }
                (None, None) => {}
                _ => return false,
            }
            candidate == &normalized_current
        })
}
