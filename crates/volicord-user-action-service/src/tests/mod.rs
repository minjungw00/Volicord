mod authority;
mod body;
mod continuity;
mod identity;
mod lifecycle;
mod materialization;
mod persistence;
mod validation;

use crate::body::construct_canonical_body;
use crate::model::{
    UserActionBodyFacts, UserActionIntent, UserActionValidationInput, ValidatedUserAction,
    ValidatedUserActionIntent,
};
use std::path::PathBuf;
use volicord_types::ids::{ChangeUnitId, ProjectId, TaskId, UserActionOptionId};
use volicord_types::schema::{
    RequiredNullable, UserActionBasisCoordinates, UserActionChoiceDraft, UserActionContext,
    UserActionDraft, UserActionOptionInput,
};
use volicord_types::values::{
    JudgmentKind, JudgmentPresentation, UserActionBasisStatus, UserActionRequiredFor, UtcTimestamp,
};

pub(super) fn product_choice_draft() -> UserActionDraft {
    UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
        judgment_kind: JudgmentKind::ProductDecision,
        presentation: JudgmentPresentation::Short,
        question: "  Choose the current product direction.  ".to_owned(),
        options: Some(vec![
            UserActionOptionInput {
                option_id: UserActionOptionId::new("keep"),
                label: "Keep".to_owned(),
                description: "Keep the current direction.".to_owned(),
                consequence: "No product-direction change is made.".to_owned(),
                is_default: true,
            },
            UserActionOptionInput {
                option_id: UserActionOptionId::new("change"),
                label: "Change".to_owned(),
                description: "Change the current direction.".to_owned(),
                consequence: "The product direction changes.".to_owned(),
                is_default: false,
            },
        ])
        .into(),
        context: UserActionContext {
            summary: "A current user-owned decision is required.".to_owned(),
            related_refs: Vec::new(),
            artifact_refs: Vec::new(),
            visible_risks: Vec::new(),
            constraints: Vec::new(),
        },
        affected_refs: Vec::new(),
        sensitive_action_scope: RequiredNullable::null(),
    }))
}

pub(super) fn validation_input() -> UserActionValidationInput {
    UserActionValidationInput {
        project_id: ProjectId::new("project-test"),
        repository_root: PathBuf::from("/product"),
        actual_task_id: "task-test".to_owned(),
        task_scope_revision: 3,
        baseline_ref: Some("baseline-test".to_owned()),
        current_change_unit_id: Some(ChangeUnitId::new("change-test")),
        requested_change_unit_exists: true,
        state_version: 11,
        operation_now: UtcTimestamp::parse("2026-07-27T00:00:00Z")
            .expect("test timestamp must parse"),
        intent: UserActionIntent {
            task_id: TaskId::new("task-test"),
            change_unit_id: Some(ChangeUnitId::new("change-test")),
            action: product_choice_draft(),
            required_for: vec![UserActionRequiredFor::CloseComplete],
            expires_at: RequiredNullable::null(),
        },
    }
}

pub(super) fn validated_choice_intent() -> ValidatedUserActionIntent {
    let input = validation_input();
    ValidatedUserActionIntent {
        task_id: input.intent.task_id,
        coordinate_change_unit_id: input.intent.change_unit_id,
        action: input.intent.action,
        coordinates: UserActionBasisCoordinates {
            task_id: TaskId::new("task-test"),
            change_unit_id: Some(ChangeUnitId::new("change-test")).into(),
            scope_revision: 3,
            baseline_ref: Some(volicord_types::ids::BaselineRef::new("baseline-test")).into(),
            created_at_state_version: 11,
            compatibility_status: UserActionBasisStatus::Current,
        },
        required_for: input.intent.required_for,
        expires_at: input.intent.expires_at,
        created_at: input.operation_now,
    }
}

pub(super) fn canonical_choice() -> ValidatedUserAction {
    construct_canonical_body(
        validated_choice_intent(),
        UserActionBodyFacts::Choice {
            close_basis_revision: None,
            result_refs: Vec::new(),
            residual_risk_ids: Vec::new(),
        },
        None,
    )
    .expect("valid test intent must construct")
}
