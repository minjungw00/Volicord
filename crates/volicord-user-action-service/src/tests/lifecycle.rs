use super::canonical_choice;
use crate::authority::user_action_authority_from_state;
use crate::lifecycle::projected_user_action_lifecycle_phase;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, ChangeUnitStatus, ProjectStateHeader, StoredChangeUnitLifecycle,
    StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis, TaskAutonomyBoundary, TaskRecord,
    TaskShapingFacts,
};
use volicord_types::ids::{ProjectId, UserActionRequestId};
use volicord_types::schema::{JsonObject, RequiredNullable, UserActionRequest};
use volicord_types::values::{
    AcceptancePolicy, PersistedCloseSummary, RequestedControlLevel, TaskControlLevel,
    TaskLifecyclePhase, TaskMode, UserActionKind, UserActionStatus, UtcTimestamp, WorkPhase,
};

fn task(lifecycle_phase: TaskLifecyclePhase) -> TaskRecord {
    TaskRecord {
        project_id: "project-test".to_owned(),
        task_id: "task-test".to_owned(),
        mode: TaskMode::Work,
        requested_control_level: RequestedControlLevel::Tracked,
        effective_control_level: TaskControlLevel::Tracked,
        control_level_reason: String::new(),
        work_phase: WorkPhase::Implementation,
        acceptance_policy: AcceptancePolicy::Required,
        acceptance_policy_reason: String::new(),
        predecessor_task_id: None,
        lineage_relation: None,
        lineage_reason: None,
        carry_forward: Vec::new(),
        lifecycle_phase,
        result: None,
        title: None,
        summary: None,
        shaping: TaskShapingFacts {
            goal_summary: None,
            scope_summary: None,
            non_goals: Vec::new(),
            baseline_ref: None,
            autonomy_boundary: None,
            initial_context_refs: Vec::new(),
            initial_source_refs: Vec::new(),
        },
        bounded_context: JsonObject::new(),
        autonomy_boundary: TaskAutonomyBoundary {
            autonomy_boundary: None,
        },
        scope_revision: 3,
        close_basis_revision: 0,
        close_basis: None,
        close_summary: PersistedCloseSummary::default(),
        current_change_unit_id: Some("change-test".to_owned()),
        closed_at: None,
        metadata: JsonObject::new(),
    }
}

fn change_unit() -> ChangeUnitRecord {
    ChangeUnitRecord {
        project_id: "project-test".to_owned(),
        change_unit_id: "change-test".to_owned(),
        task_id: "task-test".to_owned(),
        status: ChangeUnitStatus::Active,
        is_current: true,
        basis_state_version: 11,
        scope_summary: StoredChangeUnitScopeSummary {
            scope_summary: None,
            affected_areas: Vec::new(),
            constraints: Vec::new(),
        },
        bounded_paths: Vec::new(),
        write_basis: StoredChangeUnitWriteBasis {
            baseline_ref: None,
            git_workspace_context: None,
        },
        effect_contract: None,
        lifecycle: StoredChangeUnitLifecycle {
            recovery_required: false,
        },
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
        updated_at: UtcTimestamp::parse("2026-07-27T00:00:00Z").expect("valid timestamp"),
    };
    let change_unit = change_unit();

    assert_eq!(
        projected_user_action_lifecycle_phase(
            &project_state,
            &task(TaskLifecyclePhase::Ready),
            Some(&change_unit),
            &[pending_authority()],
        ),
        Some(TaskLifecyclePhase::WaitingUser)
    );
    assert_eq!(
        projected_user_action_lifecycle_phase(
            &project_state,
            &task(TaskLifecyclePhase::WaitingUser),
            Some(&change_unit),
            &[],
        ),
        Some(TaskLifecyclePhase::Ready)
    );
}
