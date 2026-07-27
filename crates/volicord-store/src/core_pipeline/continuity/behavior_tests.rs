use std::error::Error;

use volicord_types::ids::{
    BaselineRef, IdempotencyKey, ProjectContinuityRecordId, ProjectId, RecordId, RequestHash,
    TaskId, UserActionOptionId,
};
use volicord_types::schema::{
    ContinuityCursor, PersistedProjectContinuityMetadata, PersistedProjectContinuitySource,
    RequiredNullable, StateRecordRef, MAX_CONTINUITY_PAGE_SIZE,
};
use volicord_types::values::{
    JudgmentResolutionOutcome, MethodName, ProjectContinuityKind, ProjectContinuityStatus,
    StateRecordKind, UserActionKind, UtcTimestamp,
};

use super::{ContinuityMutation, ProjectContinuityRecordInsert};
use crate::core_pipeline::test_support::{
    local_user_replay_context as user_replay_context, pending_event_for_task, response_json,
    task_insert, StoreFixture as StoreHarness, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, ChangeUnitInsert, ChangeUnitMutation, CoreStorageMutation, TaskMutation,
};
use crate::core_pipeline::{
    StoredChangeUnitLifecycle, StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis,
};
use crate::StoreError;

#[test]
fn project_continuity_record_mutation_persists_and_reads_active_rows() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_continuity_store";
    let change_unit_id = "cu_continuity_store";
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::ResolveUserAction,
        Some(&IdempotencyKey::new("idem_store_continuity")),
        &RequestHash::new("sha256:store-continuity"),
        Some(user_replay_context()),
        Some(0),
        vec![pending_event_for_task("continuity", task_id)],
    );

    store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
                change_unit_id,
                task_id,
            )))
            .apply(mutation, facts)
            .map(|_| ())?;
            CoreStorageMutation::Continuity(ContinuityMutation::insert_record(
                project_continuity_record_insert(
                    "continuity_store_001",
                    task_id,
                    change_unit_id,
                    "2026-01-01T00:00:00Z",
                ),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let active = store.active_project_continuity_page(10, None)?;
    assert_eq!(store.effect_counts()?.project_continuity_records, 1);
    assert_eq!(active.total_count, 1);
    assert!(!active.truncated);
    assert_eq!(active.records.len(), 1);
    assert_eq!(
        active.records[0].continuity_record_id,
        "continuity_store_001"
    );
    assert_eq!(active.records[0].kind, ProjectContinuityKind::Decision);
    assert_eq!(active.records[0].status, ProjectContinuityStatus::Active);
    assert_eq!(active.records[0].source_task_id, task_id);
    assert_eq!(
        active.records[0].source_change_unit_id.as_deref(),
        Some(change_unit_id)
    );

    let task_records = store.project_continuity_records_for_task(task_id)?;
    assert_eq!(task_records.len(), 1);
    assert!(store.project_continuity_record_exists("continuity_store_001")?);
    Ok(())
}

#[test]
fn project_continuity_pages_are_exclusive_totalled_and_tie_broken_by_id(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_continuity_page";
    let change_unit_id = "cu_continuity_page";
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::ResolveUserAction,
        Some(&IdempotencyKey::new("idem_store_continuity_page")),
        &RequestHash::new("sha256:store-continuity-page"),
        Some(user_replay_context()),
        Some(0),
        vec![pending_event_for_task("continuity_page", task_id)],
    );

    store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
                change_unit_id,
                task_id,
            )))
            .apply(mutation, facts)
            .map(|_| ())?;
            for (record_id, updated_at) in [
                ("continuity_a", "2026-01-02T00:00:00Z"),
                ("continuity_c", "2026-01-02T00:00:00Z"),
                ("continuity_b", "2026-01-02T00:00:00Z"),
                ("continuity_d", "2026-01-01T23:59:59Z"),
            ] {
                CoreStorageMutation::Continuity(ContinuityMutation::insert_record(
                    project_continuity_record_insert(
                        record_id,
                        task_id,
                        change_unit_id,
                        updated_at,
                    ),
                ))
                .apply(mutation, facts)
                .map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;

    let first = store.active_project_continuity_page(2, None)?;
    assert_eq!(first.total_count, 4);
    assert!(first.truncated);
    assert_eq!(
        first
            .records
            .iter()
            .map(|record| record.continuity_record_id.as_str())
            .collect::<Vec<_>>(),
        vec!["continuity_c", "continuity_b"]
    );
    let last = first.records.last().expect("first page cursor source");
    let cursor = ContinuityCursor {
        updated_at: last.updated_at.clone(),
        continuity_record_id: ProjectContinuityRecordId::new(last.continuity_record_id.clone()),
    };
    let second = store.active_project_continuity_page(2, Some(&cursor))?;
    assert_eq!(second.total_count, 4);
    assert!(!second.truncated);
    assert_eq!(
        second
            .records
            .iter()
            .map(|record| record.continuity_record_id.as_str())
            .collect::<Vec<_>>(),
        vec!["continuity_a", "continuity_d"]
    );

    for invalid_page_size in [0, MAX_CONTINUITY_PAGE_SIZE + 1] {
        assert!(matches!(
            store.active_project_continuity_page(invalid_page_size, None),
            Err(StoreError::InvalidInput { .. })
        ));
    }
    let malformed_cursor = ContinuityCursor {
        updated_at: UtcTimestamp::parse("2026-01-02T00:00:00Z")?,
        continuity_record_id: ProjectContinuityRecordId::new("   "),
    };
    assert!(matches!(
        store.active_project_continuity_page(2, Some(&malformed_cursor)),
        Err(StoreError::InvalidInput { .. })
    ));
    Ok(())
}

fn project_continuity_record_insert(
    continuity_record_id: &str,
    task_id: &str,
    change_unit_id: &str,
    updated_at: &str,
) -> ProjectContinuityRecordInsert {
    ProjectContinuityRecordInsert {
        continuity_record_id: continuity_record_id.to_owned(),
        source_task_id: task_id.to_owned(),
        source_change_unit_id: Some(change_unit_id.to_owned()),
        kind: ProjectContinuityKind::Decision,
        title: "Store continuity decision".to_owned(),
        summary: "A durable store-level continuity decision.".to_owned(),
        rationale: Some("The test records a traceable decision.".to_owned()),
        applies_to_paths: vec!["src/export.rs".to_owned()],
        applies_to_refs: vec![state_ref(
            StateRecordKind::ChangeUnit,
            change_unit_id,
            task_id,
            1,
        )],
        source_refs: vec![state_ref(StateRecordKind::Task, task_id, task_id, 1)],
        artifact_refs: Vec::new(),
        status: ProjectContinuityStatus::Active,
        supersedes_refs: Vec::new(),
        review_triggers: vec!["Review if the source Task changes.".to_owned()],
        created_at: UtcTimestamp::parse(updated_at).expect("timestamp must parse"),
        updated_at: UtcTimestamp::parse(updated_at).expect("timestamp must parse"),
        metadata: PersistedProjectContinuityMetadata::ResolveUserActionDecision {
            source: PersistedProjectContinuitySource::ResolveUserAction,
            action_kind: UserActionKind::ProductDecision,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            selected_option_id: UserActionOptionId::new("option_store_test"),
        },
    }
}

fn state_ref(
    record_kind: StateRecordKind,
    record_id: &str,
    task_id: &str,
    state_version: u64,
) -> StateRecordRef {
    StateRecordRef {
        record_kind,
        record_id: RecordId::new(record_id),
        project_id: ProjectId::new(PROJECT_ID),
        task_id: RequiredNullable::some(TaskId::new(task_id)),
        produced_at_state_version: RequiredNullable::some(state_version),
    }
}

fn change_unit_insert(change_unit_id: &str, task_id: &str) -> ChangeUnitInsert {
    ChangeUnitInsert {
        change_unit_id: change_unit_id.to_owned(),
        task_id: task_id.to_owned(),
        scope_summary: StoredChangeUnitScopeSummary {
            scope_summary: Some("Store continuity scope.".to_owned()),
            affected_areas: Vec::new(),
            constraints: Vec::new(),
        },
        bounded_paths: vec!["src/export.rs".to_owned()],
        write_basis: StoredChangeUnitWriteBasis {
            baseline_ref: Some(BaselineRef::new("baseline_store")),
            git_workspace_context: None,
        },
        effect_contract: None,
        lifecycle: StoredChangeUnitLifecycle {
            recovery_required: false,
        },
    }
}
