use super::canonical_choice;
use crate::authority::user_action_authority_from_state;
use crate::lifecycle::projected_user_action_lifecycle_phase;
use volicord_store::core_pipeline::{ChangeUnitRecord, ProjectStateHeader, TaskRecord};
use volicord_types::ids::{ProjectId, UserActionRequestId};
use volicord_types::schema::{RequiredNullable, UserActionRequest};
use volicord_types::values::{UserActionKind, UserActionStatus};

fn task(lifecycle_phase: &str) -> TaskRecord {
    TaskRecord {
        project_id: "project-test".to_owned(),
        task_id: "task-test".to_owned(),
        mode: "deliverable".to_owned(),
        requested_control_level: "collaborative".to_owned(),
        effective_control_level: "collaborative".to_owned(),
        control_level_reason: String::new(),
        work_phase: "implementation".to_owned(),
        acceptance_policy: "evidence_required".to_owned(),
        acceptance_policy_reason: String::new(),
        predecessor_task_id: None,
        lineage_relation: None,
        lineage_reason: None,
        carry_forward_json: "{}".to_owned(),
        lifecycle_phase: lifecycle_phase.to_owned(),
        result: None,
        title: None,
        summary: None,
        shaping_summary_json: "{}".to_owned(),
        bounded_context_json: "{}".to_owned(),
        autonomy_boundary_json: "{}".to_owned(),
        scope_revision: 3,
        close_basis_revision: 0,
        close_basis_json: None,
        close_summary_json: "{}".to_owned(),
        current_change_unit_id: Some("change-test".to_owned()),
        closed_at: None,
        metadata_json: "{}".to_owned(),
    }
}

fn change_unit() -> ChangeUnitRecord {
    ChangeUnitRecord {
        project_id: "project-test".to_owned(),
        change_unit_id: "change-test".to_owned(),
        task_id: "task-test".to_owned(),
        status: "current".to_owned(),
        is_current: true,
        basis_state_version: 11,
        scope_summary_json: "{}".to_owned(),
        bounded_paths_json: "[]".to_owned(),
        write_basis_json: "{}".to_owned(),
        effect_contract_json: "{}".to_owned(),
        lifecycle_json: "{}".to_owned(),
    }
}

fn pending_authority() -> crate::model::UserActionAuthority {
    let constructed = canonical_choice();
    user_action_authority_from_state(&UserActionRequest {
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
    })
}

#[test]
fn lifecycle_enters_and_leaves_waiting_user_from_current_authority() {
    let project_state = ProjectStateHeader {
        project_id: "project-test".to_owned(),
        state_version: 11,
        active_task_id: Some("task-test".to_owned()),
        updated_at: "2026-07-27T00:00:00Z".to_owned(),
    };
    let change_unit = change_unit();

    assert_eq!(
        projected_user_action_lifecycle_phase(
            &project_state,
            &task("ready"),
            Some(&change_unit),
            &[pending_authority()],
        ),
        Some("waiting_user")
    );
    assert_eq!(
        projected_user_action_lifecycle_phase(
            &project_state,
            &task("waiting_user"),
            Some(&change_unit),
            &[],
        ),
        Some("ready")
    );
}
