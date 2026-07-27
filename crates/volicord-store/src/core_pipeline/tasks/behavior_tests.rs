use std::error::Error;

use rusqlite::params;
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash, TaskId};
use volicord_types::values::MethodName;

use super::TaskMutation;
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{commit_input, CoreStorageMutation};
use crate::StoreError;

#[test]
fn task_close_summary_requires_an_explicit_close_reason_on_read() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
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
fn task_decoder_rejects_invalid_closed_at() -> Result<(), Box<dyn Error>> {
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
    store.conn.execute(
        "UPDATE tasks SET closed_at = 'tomorrow' WHERE project_id = ?1 AND task_id = ?2",
        params![PROJECT_ID, task_id],
    )?;
    let read = store.task_record(&TaskId::new(task_id));
    assert!(matches!(
        read,
        Err(StoreError::CorruptOwnerStateValue {
            table: "tasks",
            logical_column: "closed_at",
            ..
        })
    ));
    Ok(())
}
