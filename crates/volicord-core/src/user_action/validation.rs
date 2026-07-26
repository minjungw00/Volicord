use super::model::{UserActionValidationInput, ValidatedUserActionIntent};
use crate::policy::write_ticket::normalize_sensitive_action_scope;
use chrono::Duration;
use std::collections::BTreeSet;
use volicord_types::ids::BaselineRef;
use volicord_types::schema::{
    RequiredNullable, UserActionBasisCoordinates, UserActionDraft,
    USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES,
};
use volicord_types::values::{
    JudgmentKind, StateRecordKind, UserActionBasisStatus, UserActionKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UserActionDomainError {
    field: &'static str,
    message: &'static str,
}

impl UserActionDomainError {
    pub(super) const fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }

    pub(super) const fn field(self) -> &'static str {
        self.field
    }

    pub(super) const fn message(self) -> &'static str {
        self.message
    }
}

pub(super) fn validate_user_action(
    input: UserActionValidationInput,
) -> Result<ValidatedUserActionIntent, UserActionDomainError> {
    let UserActionValidationInput {
        project_id,
        repository_root,
        actual_task_id,
        task_scope_revision,
        baseline_ref,
        current_change_unit_id,
        requested_change_unit_exists,
        state_version,
        operation_now,
        intent,
    } = input;
    let mut action = intent.action;

    action
        .validate_bounds()
        .map_err(|error| UserActionDomainError::new(error.field(), error.message()))?;
    if intent.required_for.is_empty() {
        return Err(UserActionDomainError::new(
            "required_for",
            "required_for must contain at least one bounded operation",
        ));
    }
    if intent
        .required_for
        .iter()
        .enumerate()
        .any(|(index, target)| intent.required_for[..index].contains(target))
    {
        return Err(UserActionDomainError::new(
            "required_for",
            "required_for must not contain duplicate operation targets",
        ));
    }
    if actual_task_id != intent.task_id.as_str() {
        return Err(UserActionDomainError::new(
            "task_id",
            "Task facts must match the semantic user-action intent",
        ));
    }
    validate_choice_affected_refs(&action, &project_id, &intent.task_id)?;
    if !intent
        .required_for
        .iter()
        .copied()
        .all(|target| action.action_kind().is_compatible_with_required_for(target))
    {
        return Err(UserActionDomainError::new(
            "required_for",
            "required_for contains an operation incompatible with the action kind",
        ));
    }

    let expires_at = if matches!(&action, UserActionDraft::EvidenceObservation(_)) {
        if intent.expires_at.is_some() {
            return Err(UserActionDomainError::new(
                "expires_at",
                "evidence-observation actions require caller expires_at to be null",
            ));
        }
        let derived = operation_now
            .checked_add(Duration::minutes(
                USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES,
            ))
            .map_err(|_| {
                UserActionDomainError::new(
                    "expires_at",
                    "derived expiration exceeds the supported canonical RFC 3339 range",
                )
            })?;
        RequiredNullable::some(derived)
    } else {
        intent.expires_at
    };
    if expires_at
        .as_ref()
        .is_some_and(|value| value.ensure_canonical_rfc3339_representable().is_err())
    {
        return Err(UserActionDomainError::new(
            "expires_at",
            "expires_at must be representable as a canonical four-digit RFC 3339 timestamp",
        ));
    }
    if expires_at
        .as_ref()
        .is_some_and(|value| value <= &operation_now)
    {
        return Err(UserActionDomainError::new(
            "expires_at",
            "expires_at must be later than the request timestamp",
        ));
    }

    if matches!(&action, UserActionDraft::EvidenceObservation(_))
        && (current_change_unit_id.is_none() || baseline_ref.is_none())
    {
        return Err(UserActionDomainError::new(
            "action",
            "evidence-observation actions require a current Change Unit and baseline",
        ));
    }
    if intent.change_unit_id.is_some() && !requested_change_unit_exists {
        return Err(UserActionDomainError::new(
            "change_unit_id",
            "change_unit_id must identify a Change Unit owned by the Task",
        ));
    }
    let action_needs_current_change_unit = matches!(
        action.action_kind(),
        UserActionKind::SensitiveApproval
            | UserActionKind::FinalAcceptance
            | UserActionKind::ResidualRiskAcceptance
            | UserActionKind::EvidenceObservation
    );
    if action_needs_current_change_unit {
        let Some(current) = current_change_unit_id.as_ref() else {
            return Err(UserActionDomainError::new(
                "change_unit_id",
                "this action kind requires the current active Change Unit",
            ));
        };
        if intent
            .change_unit_id
            .as_ref()
            .is_some_and(|requested| requested != current)
        {
            return Err(UserActionDomainError::new(
                "change_unit_id",
                "change_unit_id must match the current active Change Unit",
            ));
        }
    }

    validate_and_normalize_body_input(&mut action, &repository_root)?;

    let coordinate_change_unit_id = intent
        .change_unit_id
        .or_else(|| current_change_unit_id.clone());
    let coordinates = UserActionBasisCoordinates {
        task_id: intent.task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        scope_revision: task_scope_revision,
        baseline_ref: baseline_ref.map(BaselineRef::new).into(),
        created_at_state_version: state_version,
        compatibility_status: UserActionBasisStatus::Current,
    };
    Ok(ValidatedUserActionIntent {
        task_id: intent.task_id,
        coordinate_change_unit_id,
        action,
        coordinates,
        required_for: intent.required_for,
        expires_at,
        created_at: operation_now,
    })
}

fn validate_and_normalize_body_input(
    action: &mut UserActionDraft,
    repository_root: &std::path::Path,
) -> Result<(), UserActionDomainError> {
    match action {
        UserActionDraft::Choice(choice) => {
            if choice.question.trim().is_empty() || choice.context.summary.trim().is_empty() {
                return Err(UserActionDomainError::new(
                    "action.question",
                    "choice question and context summary must be non-empty",
                ));
            }
            if choice.judgment_kind != JudgmentKind::SensitiveApproval
                && choice.sensitive_action_scope.is_some()
            {
                return Err(UserActionDomainError::new(
                    "action.sensitive_action_scope",
                    "sensitive_action_scope is only valid for sensitive approval",
                ));
            }
            if choice.judgment_kind == JudgmentKind::SensitiveApproval
                && choice.sensitive_action_scope.is_none()
            {
                return Err(UserActionDomainError::new(
                    "action.sensitive_action_scope",
                    "sensitive approval requires a bounded sensitive action scope",
                ));
            }
            if let Some(scope) = choice.sensitive_action_scope.as_ref() {
                choice.sensitive_action_scope = Some(
                    normalize_sensitive_action_scope(repository_root, scope).map_err(|_| {
                        UserActionDomainError::new(
                            "action.sensitive_action_scope.intended_paths",
                            "sensitive action paths must stay within the Product Repository",
                        )
                    })?,
                )
                .into();
            }

            let authority_bearing = matches!(
                choice.judgment_kind,
                JudgmentKind::ScopeDecision
                    | JudgmentKind::SensitiveApproval
                    | JudgmentKind::FinalAcceptance
                    | JudgmentKind::ResidualRiskAcceptance
                    | JudgmentKind::Cancellation
            );
            let caller_options = choice
                .options
                .as_ref()
                .map(Vec::as_slice)
                .unwrap_or_default();
            if authority_bearing && !caller_options.is_empty() {
                return Err(UserActionDomainError::new(
                    "action.options",
                    "authority-bearing actions use only Core-owned options",
                ));
            }
            if !authority_bearing && caller_options.is_empty() {
                return Err(UserActionDomainError::new(
                    "action.options",
                    "product and technical choices require at least one caller-authored option",
                ));
            }
            let mut ids = BTreeSet::new();
            if caller_options
                .iter()
                .any(|option| !ids.insert(option.option_id.as_str()))
            {
                return Err(UserActionDomainError::new(
                    "action.options",
                    "choice option IDs must be unique",
                ));
            }
            if caller_options
                .iter()
                .filter(|option| option.is_default)
                .count()
                > 1
            {
                return Err(UserActionDomainError::new(
                    "action.options",
                    "choice options may contain at most one default",
                ));
            }
        }
        UserActionDraft::EvidenceObservation(observation) => {
            if observation.question.trim().is_empty()
                || observation.context_summary.trim().is_empty()
            {
                return Err(UserActionDomainError::new(
                    "action.question",
                    "observation question and context summary must be non-empty",
                ));
            }
            if observation
                .target_candidates
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != observation.target_candidates.len()
            {
                return Err(UserActionDomainError::new(
                    "action.target_candidates",
                    "target candidates must not contain duplicates",
                ));
            }
            if observation
                .artifact_candidate_ids
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != observation.artifact_candidate_ids.len()
            {
                return Err(UserActionDomainError::new(
                    "action.artifact_candidate_ids",
                    "artifact candidates must not contain duplicates",
                ));
            }
        }
    }
    Ok(())
}

fn validate_choice_affected_refs(
    action: &UserActionDraft,
    project_id: &volicord_types::ids::ProjectId,
    task_id: &volicord_types::ids::TaskId,
) -> Result<(), UserActionDomainError> {
    let UserActionDraft::Choice(choice) = action else {
        return Ok(());
    };
    for affected_ref in &choice.affected_refs {
        if affected_ref.project_id != *project_id {
            return Err(UserActionDomainError::new(
                "action.affected_refs.project_id",
                "affected_refs must belong to the request project",
            ));
        }
        let task_record_mismatch = affected_ref.record_kind == StateRecordKind::Task
            && affected_ref.record_id.as_str() != task_id.as_str();
        let task_context_mismatch = affected_ref
            .task_id
            .as_ref()
            .is_some_and(|affected_task_id| affected_task_id != task_id);
        if task_record_mismatch || task_context_mismatch {
            return Err(UserActionDomainError::new(
                "action.affected_refs.task_id",
                "task-scoped affected_refs must belong to the request Task",
            ));
        }
    }
    Ok(())
}
