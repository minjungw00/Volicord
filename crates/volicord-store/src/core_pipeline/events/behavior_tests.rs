use std::error::Error;

use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash};
use volicord_types::values::MethodName;

use crate::core_pipeline::test_support::{
    local_user_replay_context as user_replay_context, pending_event_for_task, replay_context,
    response_json, task_insert, StoreFixture as StoreHarness, ACTOR_SOURCE, CONNECTION_ID,
    PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, CoreStorageMutation, MutationCommitOutcome, TaskMutation, TaskScopeUpdate,
};

#[test]
fn committed_mutations_append_authority_events_with_context_and_hash_chain(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_authority_events";

    let first = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_authority_event_first")),
            &RequestHash::new("sha256:authority-first"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("authority_first", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

    let user_context = user_replay_context();
    let second = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_authority_event_second")),
            &RequestHash::new("sha256:authority-second"),
            Some(user_context),
            Some(1),
            vec![pending_event_for_task("authority_second", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::UpdateScope(TaskScopeUpdate {
                task_id: task_id.to_owned(),
                work_phase: None,
                lifecycle_phase: None,
                result: None,
                title: Some("Authority event projection".to_owned()),
                summary: None,
                shaping_summary_json: None,
                bounded_context_json: None,
                autonomy_boundary_json: None,
                close_summary_json: None,
            }))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(second, MutationCommitOutcome::Committed { .. }));

    let mut stmt = store.conn.prepare(
        "SELECT
                event_seq,
                event_id,
                state_version,
                event_type,
                actor_source,
                operation_category,
                payload_json,
                request_hash,
                previous_event_hash,
                event_hash
             FROM authority_events
             WHERE project_id = ?1
             ORDER BY event_seq",
    )?;
    let rows = stmt
        .query_map([PROJECT_ID], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[0].2, 1);
    assert_eq!(rows[0].3, "store_test_event");
    assert_eq!(rows[0].4, ACTOR_SOURCE);
    assert_eq!(rows[0].5, "agent_workflow");
    assert_eq!(rows[0].6, "{}");
    assert_eq!(rows[0].7, "sha256:authority-first");
    assert!(rows[0].8.is_none());
    assert!(rows[0].9.starts_with("sha256:"));
    assert_eq!(rows[0].9.len(), 71);

    assert_eq!(rows[1].0, 2);
    assert_eq!(rows[1].2, 2);
    assert_eq!(rows[1].4, "local_user");
    assert_eq!(rows[1].5, "user_only");
    assert_eq!(rows[1].7, "sha256:authority-second");
    assert_eq!(rows[1].8.as_deref(), Some(rows[0].9.as_str()));
    assert!(rows[1].9.starts_with("sha256:"));
    assert_eq!(rows[1].9.len(), 71);
    assert_ne!(rows[0].9, rows[1].9);

    let task_scoped_event_count: i64 = store.conn.query_row(
        "SELECT COUNT(*)
               FROM authority_events
              WHERE project_id = ?1
                AND task_id IS NOT NULL
                AND event_type = 'store_test_event'",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(task_scoped_event_count, 2);
    Ok(())
}
