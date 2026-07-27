use super::canonical_choice;
use crate::authority::user_action_authority_from_state;
use volicord_types::ids::{ProjectId, UserActionRequestId};
use volicord_types::schema::{RequiredNullable, UserActionRequest};
use volicord_types::values::{UserActionKind, UserActionStatus};

#[test]
fn pending_public_request_projects_to_neutral_current_authority() {
    let constructed = canonical_choice();
    let request = UserActionRequest {
        user_action_request_id: UserActionRequestId::new("action-test"),
        project_id: ProjectId::new("project-test"),
        task_id: constructed.task_id,
        change_unit_id: constructed.coordinate_change_unit_id.into(),
        action_kind: UserActionKind::ProductDecision,
        status: UserActionStatus::Pending,
        body: constructed.body,
        basis: constructed.basis,
        required_for: constructed.required_for,
        user_action_resolution_ref: RequiredNullable::null(),
        expires_at: constructed.expires_at,
        created_at: constructed.created_at,
    };

    let authority = user_action_authority_from_state(&request);

    assert_eq!(authority.user_action_request_id, "action-test");
    assert_eq!(authority.task_id.as_str(), "task-test");
    assert_eq!(authority.status, UserActionStatus::Pending);
    assert_eq!(authority.action_kind, UserActionKind::ProductDecision);
    assert!(authority.machine_action.is_none());
    assert!(authority.resolution_outcome.is_none());
    assert_eq!(
        authority
            .basis
            .as_ref()
            .expect("authority must retain canonical basis")
            .coordinates()
            .scope_revision,
        3
    );
}
