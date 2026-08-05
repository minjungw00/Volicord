use crate::{
    error::{UserActionServiceError, UserActionUnavailable, UserActionValidationError},
    model::UserActionConstructionContext,
    service::{canonical_user_action_artifacts, validate_user_action_target},
};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, StoredUserActionResolution, TaskRecord, UserActionStoreReader,
};
use volicord_types::{
    ids::{
        BaselineRef, ChangeUnitId, ProjectId, TaskId, UserActionRequestId, UserActionResolutionId,
    },
    schema::{
        ArtifactRef, StateRecordRef, UserActionBasis, UserActionEvidenceObservation,
        UserActionRequestBody, UserActionResolution, UserActionResolutionBody,
        UserActionResolutionInput,
    },
    values::{
        EvidenceRelevanceStatus, JudgmentKind, UserActionBasisStatus, UserActionOptionAction,
    },
};

pub fn validate_current_resolution_basis(
    store: &dyn UserActionStoreReader,
    observed_state_version: u64,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    basis: &UserActionBasis,
) -> Result<(), UserActionServiceError> {
    let coordinates = basis.coordinates();
    let baseline_ref = task_baseline_ref(task)?;
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    if basis.compatibility_status() != UserActionBasisStatus::Current
        || coordinates.task_id.as_str() != task.task_id
        || coordinates.scope_revision != task.scope_revision
        || coordinates.created_at_state_version > observed_state_version
        || coordinates.baseline_ref.as_ref() != baseline_ref.as_ref()
        || coordinates.change_unit_id.as_ref() != current_change_unit_id.as_ref()
    {
        return Err(UserActionServiceError::Unavailable(
            UserActionUnavailable::BasisNotCurrent,
        ));
    }
    if let Some(close_basis_revision) = basis.close_basis_revision() {
        let current = store
            .task_revision_record(&TaskId::new(task.task_id.clone()))
            .map_err(UserActionServiceError::from_store)?
            .is_some_and(|record| record.close_basis_revision == close_basis_revision);
        if !current {
            return Err(UserActionServiceError::Unavailable(
                UserActionUnavailable::CloseBasisNotCurrent,
            ));
        }
    }
    Ok(())
}

pub fn construct_user_action_resolution(
    store: &dyn UserActionStoreReader,
    context: &UserActionConstructionContext,
    resolution_input: &UserActionResolutionInput,
    request_body: &UserActionRequestBody,
    basis: &UserActionBasis,
    task_id: &TaskId,
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<(UserActionResolutionBody, Vec<StateRecordRef>), UserActionServiceError> {
    match (request_body, resolution_input) {
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
                    validation(
                        "resolution.selected_option_id",
                        "selected option must belong to the stored user-action request",
                    )
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
                return Err(validation(
                    "resolution.relevance_status",
                    "user observation relevance must be supported or contradicted",
                ));
            }
            let normalized_summary = normalize_display_text(summary);
            if normalized_summary.is_empty() {
                return Err(validation(
                    "resolution.summary",
                    "user observation summary must be non-empty",
                ));
            }
            if !observation_request.target_candidates.contains(target) {
                return Err(validation(
                    "resolution.target",
                    "observation target must be one of the stored candidates",
                ));
            }
            if artifact_ids.iter().collect::<BTreeSet<_>>().len() != artifact_ids.len() {
                return Err(validation(
                    "resolution.artifact_ids",
                    "observation artifact IDs must not contain duplicates",
                ));
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
                return Err(validation(
                    "resolution.artifact_ids",
                    "observation artifacts must be selected from the stored candidates",
                ));
            }
            validate_user_action_target(store, task_id, target, "resolution.target")?;
            let output_artifact_refs = canonical_user_action_artifacts(
                store,
                &context.project_id,
                context.observed_state_version,
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
                return Err(UserActionServiceError::Unavailable(
                    UserActionUnavailable::SelectedArtifactChanged,
                ));
            }
            if current_change_unit.is_none() {
                return Err(UserActionServiceError::Unavailable(
                    UserActionUnavailable::CurrentChangeUnitRequired,
                ));
            }
            if basis.coordinates().baseline_ref.is_none() {
                return Err(UserActionServiceError::Unavailable(
                    UserActionUnavailable::CurrentBaselineRequired,
                ));
            }
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
        _ => Err(validation(
            "resolution.resolution_type",
            "resolution type must match the stored user-action request",
        )),
    }
}

pub fn user_action_resolution_from_record(
    record: &StoredUserActionResolution,
    task_id: &TaskId,
) -> Result<UserActionResolution, UserActionServiceError> {
    Ok(UserActionResolution {
        user_action_resolution_id: UserActionResolutionId::new(record.user_action_resolution_id()),
        user_action_request_id: UserActionRequestId::new(record.user_action_request_id()),
        project_id: ProjectId::new(record.project_id()),
        task_id: task_id.clone(),
        action_kind: record.action_kind(),
        body: record.resolution().clone(),
        resolved_by_actor_source: record.resolved_by_actor_source().clone(),
        resolved_verification_basis: record.resolved_verification_basis(),
        resolved_assurance_level: record.resolved_assurance_level().to_owned(),
        channel_kind: record.channel_kind(),
        channel_submission_id: record.channel_submission_id().to_owned(),
        resolved_at: record.resolved_at().clone(),
    })
}

pub fn resolution_input_matches_body(
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

fn task_baseline_ref(task: &TaskRecord) -> Result<Option<BaselineRef>, UserActionServiceError> {
    Ok(task.shaping.baseline_ref.clone())
}

fn validation(field: &'static str, message: &'static str) -> UserActionServiceError {
    UserActionServiceError::Validation(UserActionValidationError::new(field, message))
}

fn normalize_display_text(value: &str) -> String {
    value.trim().to_owned()
}
