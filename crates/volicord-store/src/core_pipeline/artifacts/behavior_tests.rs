use std::error::Error;

use rusqlite::params;
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash, TaskId};
use volicord_types::values::{MethodName, UtcTimestamp};

use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, ACTOR_SOURCE, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{commit_input, CoreStorageMutation, TaskMutation};

#[test]
fn prepared_artifact_eligibility_uses_exact_submillisecond_expiry() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_staged_exact_expiry";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_staged_exact_expiry")),
            &RequestHash::new("sha256:staged-exact-expiry"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("staged_exact_expiry", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    store.conn.execute(
        "INSERT INTO artifact_staging (
                project_id, handle_id, task_id, created_by_actor_source,
                redaction_state, status, expires_at, created_at
             ) VALUES (
                ?1, 'stage_exact_expiry', ?2, ?3,
                'none', 'staged', '2026-07-13T00:10:00.000000501Z',
                '2026-07-13T00:00:00Z'
             )",
        params![PROJECT_ID, task_id, ACTOR_SOURCE],
    )?;
    let now = UtcTimestamp::parse("2026-07-13T00:10:00.000000500Z")?;
    let before_state = store.project_state()?;

    assert!(store.has_prepared_artifact_input(&TaskId::new(task_id), &now)?);
    store.conn.execute(
        "UPDATE artifact_staging
                SET expires_at = '2026-07-13T00:10:00.000000500Z'
              WHERE project_id = ?1 AND handle_id = 'stage_exact_expiry'",
        [PROJECT_ID],
    )?;
    assert!(!store.has_prepared_artifact_input(&TaskId::new(task_id), &now)?);
    store.conn.execute(
        "UPDATE artifact_staging
                SET expires_at = '2026-07-13T00:10:00.000000499Z'
              WHERE project_id = ?1 AND handle_id = 'stage_exact_expiry'",
        [PROJECT_ID],
    )?;
    assert!(!store.has_prepared_artifact_input(&TaskId::new(task_id), &now)?);
    assert_eq!(store.project_state()?, before_state);
    Ok(())
}
