//! Store-aware orchestration for the UserAction domain.

use crate::{
    authority::user_action_authority_from_record,
    body::construct_canonical_body,
    error::{
        UserActionInvariantError, UserActionServiceError, UserActionUnavailable,
        UserActionValidationError,
    },
    model::{
        UserActionBodyFacts, UserActionConstructionInput, UserActionValidationInput,
        ValidatedUserAction,
    },
    relevance::{user_action_blocks_operation, UserActionOperationContext},
    validation::validate_user_action,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use volicord_store::{
    artifacts::PersistentArtifactVerificationStatus,
    core_pipeline::{CoreProjectStore, StoredArtifactRecord},
    StoreError,
};
use volicord_types::{
    ids::{ArtifactId, ChangeUnitId, RiskId, StorageRef, TaskId},
    schema::{ArtifactRef, EvidenceTarget, StateRecordRef, UserActionDraft},
    values::{
        ArtifactAvailability, ArtifactIntegrityStatus, JudgmentKind, RedactionState,
        StateRecordKind, UserActionStatus, UtcTimestamp,
    },
};

/// Validates semantic intent and current facts, then constructs one canonical typed action.
pub fn construct_user_action(
    input: UserActionConstructionInput<'_>,
) -> Result<ValidatedUserAction, UserActionServiceError> {
    let UserActionConstructionInput {
        store,
        task,
        current_change_unit,
        context,
        intent,
    } = input;
    if task.task_id != intent.task_id.as_str() {
        return Err(UserActionServiceError::Invariant(
            UserActionInvariantError::TaskIdentityMismatch,
        ));
    }
    let baseline_ref = task_baseline_ref(task)?;
    let requested_change_unit_exists = match intent.change_unit_id.as_ref() {
        Some(change_unit_id) => store
            .change_unit_record(&intent.task_id, change_unit_id.as_str())
            .map_err(UserActionServiceError::from_store)?
            .is_some(),
        None => true,
    };
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let validated = validate_user_action(UserActionValidationInput {
        project_id: context.project_id.clone(),
        actual_task_id: task.task_id.clone(),
        task_scope_revision: task.scope_revision,
        baseline_ref,
        current_change_unit_id,
        requested_change_unit_exists,
        state_version: context.observed_state_version,
        operation_now: context.observed_at,
        intent,
    })
    .map_err(UserActionServiceError::Validation)?;
    let body_facts = match &validated.action {
        UserActionDraft::Choice(choice) => {
            let close = choice_close_coordinates(store, &validated.task_id, choice.judgment_kind)?;
            UserActionBodyFacts::Choice {
                close_basis_revision: close.close_basis_revision,
                result_refs: close.result_refs,
                residual_risk_ids: close.residual_risk_ids,
            }
        }
        UserActionDraft::EvidenceObservation(observation) => {
            for target in &observation.target_candidates {
                validate_user_action_target(
                    store,
                    &validated.task_id,
                    target,
                    "action.target_candidates",
                )?;
            }
            UserActionBodyFacts::EvidenceObservation {
                artifact_candidates: canonical_user_action_artifacts(
                    store,
                    &context.project_id,
                    context.observed_state_version,
                    &validated.task_id,
                    &observation.artifact_candidate_ids,
                    "action.artifact_candidate_ids",
                )?,
            }
        }
    };
    construct_canonical_body(validated, body_facts, context.locale.as_deref())
        .map_err(UserActionServiceError::Validation)
}

struct ChoiceCloseCoordinates {
    close_basis_revision: Option<u64>,
    result_refs: Vec<StateRecordRef>,
    residual_risk_ids: Vec<RiskId>,
}

fn choice_close_coordinates(
    store: &CoreProjectStore,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
) -> Result<ChoiceCloseCoordinates, UserActionServiceError> {
    if !matches!(
        judgment_kind,
        JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance
    ) {
        return Ok(ChoiceCloseCoordinates {
            close_basis_revision: None,
            result_refs: Vec::new(),
            residual_risk_ids: Vec::new(),
        });
    }
    let close_basis = store
        .task_revision_record(task_id)
        .map_err(UserActionServiceError::from_store)?
        .and_then(|record| record.current_close_basis)
        .ok_or(UserActionServiceError::Unavailable(
            UserActionUnavailable::CurrentCloseBasisRequired,
        ))?;
    Ok(ChoiceCloseCoordinates {
        close_basis_revision: Some(close_basis.close_basis_revision),
        result_refs: close_basis.result_refs.clone(),
        residual_risk_ids: close_basis
            .residual_risks
            .iter()
            .filter(|risk| risk.acceptance_required)
            .map(|risk| risk.risk_id.clone())
            .collect(),
    })
}

pub(crate) fn validate_user_action_target(
    store: &CoreProjectStore,
    task_id: &TaskId,
    target: &EvidenceTarget,
    field: &'static str,
) -> Result<(), UserActionServiceError> {
    let current = match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => store
            .acceptance_criterion_record(acceptance_criterion_id.as_str())
            .map_err(UserActionServiceError::from_store)?
            .is_some_and(|record| record.task_id == task_id.as_str() && record.status == "active"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => store
            .evidence_claim_record(task_id, evidence_claim_id.as_str())
            .map_err(UserActionServiceError::from_store)?
            .is_some_and(|record| record.statement == normalize_display_text(statement)),
    };
    current.then_some(()).ok_or_else(|| {
        UserActionServiceError::Validation(UserActionValidationError::new(
            field,
            "target must identify a current acceptance criterion or supplemental claim",
        ))
    })
}

pub(crate) fn canonical_user_action_artifacts(
    store: &CoreProjectStore,
    project_id: &volicord_types::ids::ProjectId,
    observed_state_version: u64,
    task_id: &TaskId,
    artifact_ids: &[ArtifactId],
    field: &'static str,
) -> Result<Vec<ArtifactRef>, UserActionServiceError> {
    let mut canonical = BTreeMap::new();
    for artifact_id in artifact_ids {
        let record = store
            .artifact_record(artifact_id.as_str())
            .map_err(UserActionServiceError::from_store)?;
        let owner_link = store
            .artifact_has_task_owner_link(artifact_id.as_str(), task_id.as_str())
            .map_err(UserActionServiceError::from_store)?;
        let Some(record) = record else {
            return Err(artifact_validation_error(field));
        };
        let verification = store
            .verify_persistent_artifact_body(&record)
            .map_err(UserActionServiceError::from_store)?;
        if record.project_id != project_id.as_str()
            || record.task_id != task_id.as_str()
            || !owner_link
            || verification.status != PersistentArtifactVerificationStatus::VerifiedCurrent
        {
            return Err(artifact_validation_error(field));
        }
        canonical.insert(
            artifact_id.as_str().to_owned(),
            verified_artifact_ref(&record, observed_state_version)?,
        );
    }
    Ok(canonical.into_values().collect())
}

fn verified_artifact_ref(
    record: &StoredArtifactRecord,
    observed_state_version: u64,
) -> Result<ArtifactRef, UserActionServiceError> {
    let task_id = TaskId::new(record.task_id.clone());
    let redaction_state =
        serde_json::from_value::<RedactionState>(Value::String(record.redaction_state.clone()))
            .map_err(|_| {
                UserActionServiceError::CorruptStoredState(StoreError::corrupt_owner_state_value(
                    "artifacts",
                    &record.artifact_id,
                    "redaction_state",
                ))
            })?;
    Ok(ArtifactRef {
        artifact_id: ArtifactId::new(record.artifact_id.clone()),
        project_id: volicord_types::ids::ProjectId::new(record.project_id.clone()),
        task_id: task_id.clone(),
        display_name: record
            .producer
            .display_name
            .clone()
            .unwrap_or_else(|| record.artifact_id.clone()),
        content_type: record.content_type.clone().into(),
        sha256: record.sha256.clone().into(),
        size_bytes: record.size_bytes.into(),
        integrity_status: ArtifactIntegrityStatus::Verified,
        redaction_state,
        availability: ArtifactAvailability::Available,
        created_by_run_ref: Some(StateRecordRef::new(
            StateRecordKind::Run,
            record.provenance.producer_run_id.as_str(),
            volicord_types::ids::ProjectId::new(record.project_id.clone()),
            Some(task_id),
            Some(observed_state_version),
        ))
        .into(),
        created_by_actor_source: Some(record.producer.created_by_actor_source.clone()).into(),
        storage_ref: Some(StorageRef::new(record.uri.clone())).into(),
    })
}

fn artifact_validation_error(field: &'static str) -> UserActionServiceError {
    UserActionServiceError::Validation(UserActionValidationError::new(
        field,
        "artifact candidates must be verified current artifacts owned by this Task",
    ))
}

/// Loads resolved authority facts for one judgment kind.
pub fn resolved_user_action_authorities(
    store: &CoreProjectStore,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
    now: &UtcTimestamp,
) -> Result<Vec<crate::model::UserActionAuthority>, UserActionServiceError> {
    store
        .resolved_user_action_records(task_id, judgment_kind.into(), now)
        .map_err(UserActionServiceError::from_store)?
        .iter()
        .map(user_action_authority_from_record)
        .collect()
}

/// Loads all current pending UserAction authority facts for a Task.
pub fn pending_user_action_authorities(
    store: &CoreProjectStore,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<crate::model::UserActionAuthority>, UserActionServiceError> {
    store
        .pending_user_action_records(task_id, now)
        .map_err(UserActionServiceError::from_store)?
        .iter()
        .map(user_action_authority_from_record)
        .collect()
}

/// Projects pending refs that block one typed operation.
pub fn pending_user_action_refs_for_operation(
    store: &CoreProjectStore,
    project_id: &volicord_types::ids::ProjectId,
    state_version: u64,
    now: &UtcTimestamp,
    context: &UserActionOperationContext<'_>,
) -> Result<Vec<StateRecordRef>, UserActionServiceError> {
    Ok(
        pending_user_action_authorities(store, context.task_id, now)?
            .iter()
            .filter(|authority| user_action_blocks_operation(authority, context))
            .map(|authority| {
                StateRecordRef::new(
                    StateRecordKind::UserActionRequest,
                    authority.user_action_request_id.as_str(),
                    project_id.clone(),
                    Some(context.task_id.clone()),
                    Some(state_version),
                )
            })
            .collect(),
    )
}

/// Loads every resolved UserAction authority for a Task.
pub fn resolved_user_action_authorities_for_all_kinds(
    store: &CoreProjectStore,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<crate::model::UserActionAuthority>, UserActionServiceError> {
    store
        .user_action_records_for_task(task_id, now)
        .map_err(UserActionServiceError::from_store)?
        .into_iter()
        .filter(|record| record.status == UserActionStatus::Resolved)
        .map(|record| user_action_authority_from_record(&record))
        .collect()
}

/// Reads current pending refs for a projected Task state.
pub fn projected_pending_user_action_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, UserActionServiceError> {
    store
        .pending_user_action_refs(task_id, state_version, now)
        .map_err(UserActionServiceError::from_store)
        .map(|records| {
            records
                .into_iter()
                .map(|record| {
                    StateRecordRef::new(
                        stored_record_kind(&record.record_kind),
                        record.record_id,
                        volicord_types::ids::ProjectId::new(record.project_id),
                        record.task_id.map(TaskId::new),
                        record.state_version,
                    )
                })
                .collect()
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedTaskShaping {
    #[serde(default)]
    goal_summary: Option<String>,
    #[serde(default)]
    scope_summary: Option<String>,
    #[serde(default)]
    non_goals: Vec<String>,
    #[serde(default)]
    baseline_ref: Option<String>,
    #[serde(default)]
    autonomy_boundary: Option<String>,
    #[serde(default)]
    initial_context_refs: Option<Value>,
    #[serde(default)]
    initial_source_refs: Option<Value>,
}

fn task_baseline_ref(
    task: &volicord_store::core_pipeline::TaskRecord,
) -> Result<Option<String>, UserActionServiceError> {
    let shaping: PersistedTaskShaping =
        serde_json::from_str(&task.shaping_summary_json).map_err(|_| {
            UserActionServiceError::CorruptStoredState(StoreError::corrupt_owner_state_json(
                "tasks",
                &task.task_id,
                "shaping_summary_json",
            ))
        })?;
    let _ = (
        shaping.goal_summary,
        shaping.scope_summary,
        shaping.non_goals,
        shaping.autonomy_boundary,
        shaping.initial_context_refs,
        shaping.initial_source_refs,
    );
    Ok(shaping.baseline_ref)
}

fn stored_record_kind(value: &str) -> StateRecordKind {
    match value {
        "user_action_request" => StateRecordKind::UserActionRequest,
        "user_action_resolution" => StateRecordKind::UserActionResolution,
        "blocker" => StateRecordKind::Blocker,
        "write_ticket" => StateRecordKind::WriteTicket,
        "change_unit" => StateRecordKind::ChangeUnit,
        "task" => StateRecordKind::Task,
        "evidence_observation" => StateRecordKind::EvidenceObservation,
        "unrecorded_change" => StateRecordKind::UnrecordedChange,
        "project_continuity_record" => StateRecordKind::ProjectContinuityRecord,
        _ => StateRecordKind::ProjectState,
    }
}

fn normalize_display_text(value: &str) -> String {
    value.trim().to_owned()
}
