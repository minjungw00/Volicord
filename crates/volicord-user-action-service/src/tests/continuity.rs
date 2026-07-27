use crate::{derive_user_action_continuity, UserActionContinuityInput, UserActionUnavailable};
use volicord_types::{
    ids::{ProjectId, UserActionOptionId},
    schema::{RequiredNullable, StateRecordRef, UserActionResolutionBody},
    values::{
        JudgmentResolutionOutcome, ProjectContinuityKind, StateRecordKind, UserActionOptionAction,
    },
};

#[test]
fn accepted_product_decision_derives_semantic_continuity() {
    let constructed = super::canonical_choice();
    let resolution_ref = StateRecordRef::new(
        StateRecordKind::UserActionResolution,
        "resolution-test",
        ProjectId::new("project-test"),
        Some(constructed.task_id.clone()),
        Some(12),
    );
    let drafts = derive_user_action_continuity(UserActionContinuityInput {
        request_body: &constructed.body,
        basis: &constructed.basis,
        resolution: &UserActionResolutionBody::Choice {
            selected_option_id: UserActionOptionId::new("keep"),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            note: RequiredNullable::null(),
            accepted_risk_ids: Vec::new(),
        },
        resolution_ref: &resolution_ref,
        applies_to_paths: vec!["src".to_owned()],
        current_close_basis: None,
    })
    .expect("accepted decision must derive continuity");

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].kind, ProjectContinuityKind::Decision);
    assert_eq!(drafts[0].title, "Product decision: Keep");
    assert_eq!(drafts[0].rationale, None);
    assert_eq!(drafts[0].source_refs.first(), Some(&resolution_ref));
}

#[test]
fn missing_selected_option_is_a_typed_unavailable_result() {
    let constructed = super::canonical_choice();
    let resolution_ref = StateRecordRef::new(
        StateRecordKind::UserActionResolution,
        "resolution-test",
        ProjectId::new("project-test"),
        Some(constructed.task_id.clone()),
        Some(12),
    );
    let error = derive_user_action_continuity(UserActionContinuityInput {
        request_body: &constructed.body,
        basis: &constructed.basis,
        resolution: &UserActionResolutionBody::Choice {
            selected_option_id: UserActionOptionId::new("missing"),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            note: RequiredNullable::null(),
            accepted_risk_ids: Vec::new(),
        },
        resolution_ref: &resolution_ref,
        applies_to_paths: Vec::new(),
        current_close_basis: None,
    })
    .expect_err("missing option must be unavailable");

    assert!(matches!(
        error,
        crate::UserActionServiceError::Unavailable(
            UserActionUnavailable::StoredResolutionOptionMissing
        )
    ));
}
