use super::*;
use volicord_types::ids::{ProjectId, RecordId, TaskId};
use volicord_types::schema::{RequiredNullable, StateRecordRef};
use volicord_types::values::{CloseReadinessBlockerCategory, StateRecordKind};

fn task_ref() -> StateRecordRef {
    StateRecordRef {
        record_kind: StateRecordKind::Task,
        record_id: RecordId::new("task_blockers"),
        project_id: ProjectId::new("project_blockers"),
        task_id: Some(TaskId::new("task_blockers")).into(),
        produced_at_state_version: Some(4).into(),
    }
}

#[test]
fn blocker_normalization_applies_freshness_to_each_local_action() {
    let task_ref = task_ref();
    let first = close_guidance(
        CloseGuidance::RecordCurrentCloseBasis,
        vec![task_ref.clone()],
    );
    let second = close_guidance(CloseGuidance::RequestFinalAcceptance, vec![task_ref]);
    let mut blockers = vec![
        close_blocker(
            CloseReadinessBlockerCategory::Task,
            "missing_current_close_basis",
            "missing basis",
            Vec::new(),
            vec![first],
        ),
        close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "missing acceptance",
            Vec::new(),
            vec![second],
        ),
    ];

    normalize_close_blockers(&mut blockers, 17);

    assert_eq!(
        blockers[0].next_actions[0].expected_state_version,
        RequiredNullable::some(17)
    );
    assert_eq!(
        blockers[1].next_actions[0].expected_state_version,
        RequiredNullable::some(17)
    );
}
