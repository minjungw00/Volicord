use super::canonical_choice;
use crate::model::{UserActionAuthority, UserActionResolutionAvailability};
use crate::relevance::{
    current_cancellation_authority, user_action_blocks_operation, CancellationAuthorityRequirement,
    UserActionOperation, UserActionOperationContext,
};
use volicord_types::ids::{
    ChangeUnitId, ProjectId, TaskId, UserActionOptionId, UserActionRequestId,
    UserActionResolutionId,
};
use volicord_types::schema::{RequiredNullable, UserActionBasis, UserActionResolutionBody};
use volicord_types::values::{
    ActorSource, JudgmentResolutionOutcome, UserActionBasisStatus, UserActionKind,
    UserActionOptionAction, UserActionRequiredFor, UserActionStatus, UserActionVerificationBasis,
};

fn authority(
    action_kind: UserActionKind,
    status: UserActionStatus,
    required_for: UserActionRequiredFor,
) -> UserActionAuthority {
    UserActionAuthority {
        project_id: ProjectId::new("project-test"),
        user_action_request_id: UserActionRequestId::new("action-test"),
        user_action_resolution_id: None,
        task_id: TaskId::new("task-test"),
        action_kind,
        status,
        required_for: vec![required_for],
        affected_refs: Vec::new(),
        machine_action: None,
        resolution_outcome: None,
        resolved_by_actor_source: None,
        resolved_verification_basis: None,
        resolved_assurance_level: None,
        basis_status: UserActionBasisStatus::Current,
        basis: Some(canonical_choice().basis),
        resolution: None,
        expires_at: None,
    }
}

fn accepted_cancellation() -> UserActionAuthority {
    let mut authority = authority(
        UserActionKind::Cancellation,
        UserActionStatus::Resolved,
        UserActionRequiredFor::CloseCancel,
    );
    authority.user_action_resolution_id = Some(UserActionResolutionId::new("resolution-test"));
    authority.machine_action = Some(UserActionOptionAction::Accept);
    authority.resolution_outcome = Some(JudgmentResolutionOutcome::Accepted);
    authority.resolved_by_actor_source = Some(ActorSource::LocalUser);
    authority.resolved_verification_basis = Some(UserActionVerificationBasis::CliDirectUserChannel);
    authority.resolved_assurance_level = Some("direct_user_input".to_owned());
    authority.resolution = Some(UserActionResolutionBody::Choice {
        selected_option_id: UserActionOptionId::new("accept"),
        machine_action: UserActionOptionAction::Accept,
        resolution_outcome: JudgmentResolutionOutcome::Accepted,
        note: RequiredNullable::null(),
        accepted_risk_ids: Vec::new(),
    });
    authority
}

#[test]
fn pending_operation_relevance_matrix_uses_typed_authority_facts() {
    let task_id = TaskId::new("task-test");
    let change_unit_id = ChangeUnitId::new("change-test");
    let context = UserActionOperationContext {
        operation: UserActionOperation::CloseComplete,
        task_id: &task_id,
        change_unit_id: Some(&change_unit_id),
        scope_revision: 3,
        close_basis: None,
        operation_refs: &[],
        sensitive_approval: None,
    };

    let current = authority(
        UserActionKind::ProductDecision,
        UserActionStatus::Pending,
        UserActionRequiredFor::CloseComplete,
    );
    assert!(user_action_blocks_operation(&current, &context));

    let prepare_write_only = authority(
        UserActionKind::TechnicalDecision,
        UserActionStatus::Pending,
        UserActionRequiredFor::PrepareWrite,
    );
    assert!(!user_action_blocks_operation(&prepare_write_only, &context));

    let cancellation = authority(
        UserActionKind::Cancellation,
        UserActionStatus::Pending,
        UserActionRequiredFor::CloseCancel,
    );
    assert!(!user_action_blocks_operation(&cancellation, &context));

    for status in [
        UserActionStatus::Resolved,
        UserActionStatus::Stale,
        UserActionStatus::Superseded,
        UserActionStatus::Expired,
    ] {
        let non_pending = authority(
            UserActionKind::ProductDecision,
            status,
            UserActionRequiredFor::CloseComplete,
        );
        assert!(!user_action_blocks_operation(&non_pending, &context));
    }
}

#[test]
fn cancellation_authority_matrix_rejects_noncurrent_or_unverified_resolution() {
    let task_id = TaskId::new("task-test");
    let change_unit_id = ChangeUnitId::new("change-test");
    let requirement = CancellationAuthorityRequirement {
        task_id: &task_id,
        change_unit_id: Some(&change_unit_id),
        scope_revision: 3,
    };
    let current = accepted_cancellation();
    assert!(current_cancellation_authority(&current, &requirement));

    let mut rejected = current.clone();
    rejected.machine_action = Some(UserActionOptionAction::Reject);
    rejected.resolution_outcome = Some(JudgmentResolutionOutcome::Rejected);
    rejected.resolution = Some(UserActionResolutionBody::Choice {
        selected_option_id: UserActionOptionId::new("reject"),
        machine_action: UserActionOptionAction::Reject,
        resolution_outcome: JudgmentResolutionOutcome::Rejected,
        note: RequiredNullable::null(),
        accepted_risk_ids: Vec::new(),
    });
    assert!(!current_cancellation_authority(&rejected, &requirement));

    let mut stale_scope = current.clone();
    let Some(UserActionBasis::Choice(basis)) = stale_scope.basis.as_mut() else {
        panic!("test authority must use a choice basis");
    };
    basis.coordinates.scope_revision = 2;
    assert!(!current_cancellation_authority(&stale_scope, &requirement));

    let mut non_user = current.clone();
    non_user.resolved_by_actor_source = Some(ActorSource::System);
    assert!(!current_cancellation_authority(&non_user, &requirement));

    let mut unverified = current;
    unverified.resolved_verification_basis = None;
    assert!(!current_cancellation_authority(&unverified, &requirement));
}

#[test]
fn resolution_availability_is_owned_by_effective_lifecycle_status() {
    let cases = [
        (UserActionStatus::Pending, true),
        (UserActionStatus::Resolved, false),
        (UserActionStatus::Stale, false),
        (UserActionStatus::Superseded, false),
        (UserActionStatus::Expired, false),
    ];

    for (status, available) in cases {
        assert_eq!(
            UserActionResolutionAvailability::from_status(status).is_available(),
            available
        );
    }
}
