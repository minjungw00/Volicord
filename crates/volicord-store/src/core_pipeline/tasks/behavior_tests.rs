use std::error::Error;

use rusqlite::params;
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash, TaskId};
use volicord_types::values::MethodName;

use super::{TaskCloseUpdate, TaskMutation};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{commit_input, CoreStorageMutation};
use crate::StoreError;

#[test]
fn task_close_summary_requires_an_explicit_close_reason_on_write_and_read(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let before = store.effect_counts()?;
    let mut invalid = task_insert("task_missing_close_reason_write");
    invalid.close_summary_json = "{}".to_owned();
    let write = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_missing_close_reason_write")),
            &RequestHash::new("sha256:missing-close-reason-write"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "missing_close_reason_write",
                "task_missing_close_reason_write",
            )],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(invalid))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    );
    assert!(matches!(write, Err(StoreError::InvalidInput { .. })));
    assert_eq!(store.effect_counts()?, before);

    let task_id = "task_missing_close_reason_read";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_missing_close_reason_read")),
            &RequestHash::new("sha256:missing-close-reason-read"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("missing_close_reason_read", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    store.conn.execute(
        "UPDATE tasks SET close_summary_json = '{}' WHERE project_id = ?1 AND task_id = ?2",
        params![PROJECT_ID, task_id],
    )?;
    let read = store.task_record(&TaskId::new(task_id));
    assert!(matches!(
        read,
        Err(StoreError::CorruptOwnerStateJson {
            table: "tasks",
            logical_column: "close_summary_json",
            ..
        })
    ));
    Ok(())
}

#[test]
fn task_close_rejects_invalid_closed_at_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_invalid_semantic_timestamp";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_invalid_timestamp_setup")),
            &RequestHash::new("sha256:invalid-timestamp-setup"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("invalid_timestamp_setup", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    let before = store.effect_counts()?;

    let close = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::CloseTask,
            Some(&IdempotencyKey::new("idem_invalid_closed_at")),
            &RequestHash::new("sha256:invalid-closed-at"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("invalid_closed_at", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::Close(TaskCloseUpdate {
                task_id: task_id.to_owned(),
                lifecycle_phase: "completed".to_owned(),
                result: "completed".to_owned(),
                close_summary_json: "{\"close_reason\":\"completed_self_checked\"}".to_owned(),
                closed_at: "tomorrow".to_owned(),
            }))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    );
    assert!(matches!(close, Err(StoreError::InvalidInput { .. })));
    assert_eq!(store.effect_counts()?, before);
    Ok(())
}
