//! Store-aware orchestration for the shared UserAction domain.

use crate::methods::{
    artifact_ref_from_verified_record, decision_rejected_response, normalize_display_text,
    persistent_artifact_is_verified_current, state_ref, store_error_response,
    stored_refs_to_state_refs, validation_plan_error, PlanError, StoredScope,
};
use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::close_readiness::{current_acceptance_required_risk_ids, UserActionAuthority};
use crate::policy::user_action_relevance::{
    user_action_blocks_operation, UserActionOperationContext,
};
use std::collections::BTreeMap;
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_store::StoreError;
use volicord_types::ids::{ArtifactId, ChangeUnitId, RiskId, TaskId};
use volicord_types::schema::{
    ArtifactRef, EvidenceTarget, StateRecordRef, ToolEnvelope, UserActionDraft,
};
use volicord_types::values::{JudgmentKind, StateRecordKind, UserActionStatus, UtcTimestamp};

use super::authority::user_action_authority_from_record;
use super::body::construct_canonical_body;
use super::model::{
    UserActionBodyFacts, UserActionConstructionInput, UserActionValidationInput,
    ValidatedUserAction,
};
use super::validation::validate_user_action;

pub(crate) fn user_action_validation_error<T>(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    validation_plan_error(dry_run, state_version, field, message)
}

/// Validates semantic intent and current facts, then constructs the canonical typed action.
pub(crate) fn construct_user_action(
    input: UserActionConstructionInput<'_>,
) -> Result<ValidatedUserAction, PlanError> {
    let UserActionConstructionInput {
        store,
        project_state,
        envelope,
        task,
        current_change_unit,
        operation_now,
        intent,
    } = input;
    if task.task_id != intent.task_id.as_str() {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value("tasks", &task.task_id, "task_id"),
        )));
    }
    let scope = StoredScope::from_task(task)?;
    let requested_change_unit_exists = match intent.change_unit_id.as_ref() {
        Some(change_unit_id) => store
            .change_unit_record(&intent.task_id, change_unit_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some(),
        None => true,
    };
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let validated = match validate_user_action(UserActionValidationInput {
        project_id: envelope.project_id.clone(),
        repository_root: store.project_record().repo_root.clone(),
        actual_task_id: task.task_id.clone(),
        task_scope_revision: task.scope_revision,
        baseline_ref: scope.baseline_ref,
        current_change_unit_id,
        requested_change_unit_exists,
        state_version: project_state.state_version,
        operation_now: operation_now.clone(),
        intent,
    }) {
        Ok(validated) => validated,
        Err(error) => {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            );
        }
    };
    let body_facts = match &validated.action {
        UserActionDraft::Choice(choice) => {
            let close = choice_close_coordinates(
                store,
                project_state,
                envelope,
                &validated.task_id,
                choice.judgment_kind,
            )?;
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
                    project_state,
                    envelope,
                    &validated.task_id,
                    target,
                    "action.target_candidates",
                )?;
            }
            UserActionBodyFacts::EvidenceObservation {
                artifact_candidates: canonical_user_action_artifacts(
                    store,
                    project_state,
                    envelope,
                    &validated.task_id,
                    &observation.artifact_candidate_ids,
                    "action.artifact_candidate_ids",
                )?,
            }
        }
    };
    match construct_canonical_body(
        validated,
        body_facts,
        envelope.locale.as_ref().map(String::as_str),
    ) {
        Ok(action) => Ok(action),
        Err(error) => user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            error.field(),
            error.message(),
        ),
    }
}

struct ChoiceCloseCoordinates {
    close_basis_revision: Option<u64>,
    result_refs: Vec<StateRecordRef>,
    residual_risk_ids: Vec<RiskId>,
}

fn choice_close_coordinates(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
) -> Result<ChoiceCloseCoordinates, PlanError> {
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
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis)
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                envelope,
                Some(project_state.state_version),
                "a current close basis is required for this user action",
            )))
        })?;
    Ok(ChoiceCloseCoordinates {
        close_basis_revision: Some(close_basis.close_basis_revision),
        result_refs: close_basis.result_refs.clone(),
        residual_risk_ids: current_acceptance_required_risk_ids(&close_basis)
            .into_iter()
            .collect(),
    })
}

pub(crate) fn validate_user_action_target(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    target: &EvidenceTarget,
    field: &'static str,
) -> Result<(), PlanError> {
    let current = match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => store
            .acceptance_criterion_record(acceptance_criterion_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.task_id == task_id.as_str() && record.status == "active"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => store
            .evidence_claim_record(task_id, evidence_claim_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.statement == normalize_display_text(statement)),
    };
    if current {
        Ok(())
    } else {
        user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            field,
            "target must identify a current acceptance criterion or supplemental claim",
        )
    }
}

pub(crate) fn canonical_user_action_artifacts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    artifact_ids: &[ArtifactId],
    field: &'static str,
) -> Result<Vec<ArtifactRef>, PlanError> {
    let mut canonical = BTreeMap::new();
    for artifact_id in artifact_ids {
        let record = store
            .artifact_record(artifact_id.as_str())
            .map_err(CorePipelineError::from)?;
        let owner_link = store
            .artifact_has_task_owner_link(artifact_id.as_str(), task_id.as_str())
            .map_err(CorePipelineError::from)?;
        let Some(record) = record else {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                "artifact candidates must identify current persistent Task artifacts",
            );
        };
        if record.project_id != envelope.project_id.as_str()
            || record.task_id != task_id.as_str()
            || !owner_link
            || !persistent_artifact_is_verified_current(store, &record)?
        {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                "artifact candidates must be verified current artifacts owned by this Task",
            );
        }
        let artifact_ref = artifact_ref_from_verified_record(
            store,
            &record,
            None,
            Some(project_state.state_version),
        )?;
        canonical.insert(artifact_id.as_str().to_owned(), artifact_ref);
    }
    Ok(canonical.into_values().collect())
}

/// Loads resolved authority facts for one judgment kind.
pub(crate) fn resolved_user_action_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .resolved_user_action_records(task_id, judgment_kind.into(), now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

/// Loads all current pending UserAction authority facts for a Task.
pub(crate) fn pending_user_action_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .pending_user_action_records(task_id, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

/// Projects pending refs that block one typed Core operation.
pub(crate) fn pending_user_action_refs_for_operation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    now: &UtcTimestamp,
    context: &UserActionOperationContext<'_>,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(pending_user_action_authorities_for_plan(
        store,
        project_state,
        envelope,
        context.task_id,
        now,
    )?
    .iter()
    .filter(|authority| user_action_blocks_operation(authority, context))
    .map(|authority| {
        state_ref(
            StateRecordKind::UserActionRequest,
            &authority.user_action_request_id,
            &envelope.project_id,
            Some(context.task_id),
            Some(project_state.state_version),
        )
    })
    .collect())
}

/// Loads every current resolved UserAction authority for a Task.
pub(crate) fn resolved_user_action_authorities_for_all_kinds(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .user_action_records_for_task(task_id, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .filter(|record| record.status == UserActionStatus::Resolved)
        .map(|record| user_action_authority_from_record(&record))
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

/// Reads current pending refs for a projected Task state.
pub(crate) fn projected_pending_user_action_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(stored_refs_to_state_refs(
        store
            .pending_user_action_refs(task_id, state_version, now)
            .map_err(CorePipelineError::from)?,
    ))
}
