use std::error::Error;

use rusqlite::params;
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash};
use volicord_types::values::{
    CloseReason, MethodName, PersistedCloseSummary, TaskLifecyclePhase, TaskResult, UtcTimestamp,
};

use super::advance_project_utc_floor_tx;
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, CoreStorageMutation, EvidenceClaimInsert, EvidenceMutation,
    MutationCommitOutcome, TaskCloseUpdate, TaskMutation,
};
use crate::sqlite::open_project_state_database_for_test;
use crate::StoreError;

#[test]
fn default_commit_clock_includes_transaction_live_storage_time() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let configured_floor = "2000-01-01T00:00:00Z";
    store.conn.execute(
        "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
        params![PROJECT_ID, configured_floor],
    )?;
    let sqlite_before: String =
        store
            .conn
            .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
                row.get(0)
            })?;
    let task_id = "task_live_commit_clock";
    let mut input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::Intake,
        Some(&IdempotencyKey::new("idem_live_commit_clock")),
        &RequestHash::new("sha256:live-commit-clock"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("live_commit_clock", task_id)],
    );
    input.clock_floor = Some(UtcTimestamp::parse(configured_floor)?);

    let outcome = store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;

    assert!(matches!(outcome, MutationCommitOutcome::Committed { .. }));
    let committed_at = store.project_state()?.updated_at;
    assert!(committed_at >= UtcTimestamp::parse(&sqlite_before)?);
    assert!(committed_at > UtcTimestamp::parse(configured_floor)?);
    Ok(())
}

#[test]
fn canonical_clock_helpers_reject_corrupt_floor_and_extreme_sample_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let store = harness.store()?;
    let before = store.effect_counts()?;
    let original_floor = store.project_state()?.updated_at;
    let out_of_range = "9999-12-31T23:59:59-23:59";
    store.conn.execute(
        "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
        params![PROJECT_ID, out_of_range],
    )?;

    assert!(matches!(
        store.current_timestamp(),
        Err(StoreError::CorruptOwnerStateValue { .. })
    ));
    let persisted: String = store.conn.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(persisted, out_of_range);

    store.conn.execute(
        "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
        params![PROJECT_ID, original_floor.to_canonical_string()],
    )?;
    assert_eq!(store.effect_counts()?, before);
    drop(store);
    let mut conn = open_project_state_database_for_test(harness.state_database_path())?;
    let tx = conn.transaction()?;
    let extreme = UtcTimestamp::from_datetime(chrono::DateTime::<chrono::Utc>::MAX_UTC);
    assert!(matches!(
        advance_project_utc_floor_tx(&tx, PROJECT_ID, &extreme),
        Err(StoreError::SchemaInvariant { .. })
    ));
    drop(tx);
    let after_floor: String = conn.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_floor, original_floor.to_canonical_string());
    drop(conn);
    assert_eq!(harness.store()?.effect_counts()?, before);
    Ok(())
}

#[test]
fn explicit_future_clock_floor_survives_active_task_commit_and_reopen() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_clock_floor";
    let first = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_clock_floor_task")),
            &RequestHash::new("sha256:clock-floor-task"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("clock_floor_task", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

    let future_floor = UtcTimestamp::parse("2999-07-13T12:34:56.789Z")?;
    let future_task_id = "task_clock_floor_future";
    let mut clock_floor_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::Intake,
        Some(&IdempotencyKey::new("idem_clock_floor_activate")),
        &RequestHash::new("sha256:clock-floor-activate"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(1),
        vec![
            pending_event_for_task("clock_floor_activate", future_task_id),
            pending_event_for_task("clock_floor_activate_second", future_task_id),
        ],
    );
    clock_floor_input.clock_floor = Some(future_floor.clone());
    let second = store.commit_with(
        clock_floor_input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(future_task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::Evidence(EvidenceMutation::EnsureClaim(EvidenceClaimInsert {
                evidence_claim_id: "claim_clock_floor".to_owned(),
                task_id: future_task_id.to_owned(),
                statement: "The canonical commit clock is shared.".to_owned(),
            }))
            .apply(mutation, facts)
            .map(|_| ())?;
            CoreStorageMutation::Task(TaskMutation::Close(TaskCloseUpdate {
                task_id: task_id.to_owned(),
                lifecycle_phase: TaskLifecyclePhase::Completed,
                result: TaskResult::Completed,
                close_summary: PersistedCloseSummary {
                    close_reason: CloseReason::CompletedSelfChecked,
                    ..PersistedCloseSummary::default()
                },
                closed_at: UtcTimestamp::parse("2999-07-13T12:00:00Z")
                    .expect("test timestamp must parse"),
            }))
            .apply(mutation, facts)
            .map(|_| ())?;
            CoreStorageMutation::Task(TaskMutation::SetActive {
                task_id: future_task_id.to_owned(),
            })
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(second, MutationCommitOutcome::Committed { .. }));

    let expected = future_floor.to_string();
    let state = store.project_state()?;
    assert_eq!(state.active_task_id.as_deref(), Some(future_task_id));
    assert_eq!(state.updated_at, future_floor);
    let (task_created_at, task_updated_at) = store.conn.query_row(
        "SELECT created_at, updated_at
               FROM tasks
              WHERE project_id = ?1 AND task_id = ?2",
        params![PROJECT_ID, future_task_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    assert_eq!(task_created_at, expected);
    assert_eq!(task_updated_at, expected);
    let (closed_at, closed_task_updated_at) = store.conn.query_row(
        "SELECT closed_at, updated_at
               FROM tasks
              WHERE project_id = ?1 AND task_id = ?2",
        params![PROJECT_ID, task_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    assert_eq!(closed_at, "2999-07-13T12:00:00Z");
    assert_eq!(closed_task_updated_at, expected);
    let claim_created_at = store.conn.query_row(
        "SELECT created_at
               FROM evidence_claims
              WHERE project_id = ?1 AND evidence_claim_id = 'claim_clock_floor'",
        [PROJECT_ID],
        |row| row.get::<_, String>(0),
    )?;
    assert_eq!(claim_created_at, expected);
    let event_created_at = store.conn.query_row(
        "SELECT created_at FROM authority_events
              WHERE project_id = ?1 AND event_id = 'evt_clock_floor_activate'",
        [PROJECT_ID],
        |row| row.get::<_, String>(0),
    )?;
    let invocation_created_at = store.conn.query_row(
        "SELECT created_at FROM tool_invocations
              WHERE project_id = ?1 AND idempotency_key = 'idem_clock_floor_activate'",
        [PROJECT_ID],
        |row| row.get::<_, String>(0),
    )?;
    assert_eq!(event_created_at, expected);
    assert_eq!(invocation_created_at, expected);
    let (event_count, distinct_event_timestamps) = store.conn.query_row(
        "SELECT COUNT(*), COUNT(DISTINCT created_at)
               FROM authority_events
              WHERE project_id = ?1
                AND event_id IN ('evt_clock_floor_activate', 'evt_clock_floor_activate_second')",
        [PROJECT_ID],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;
    assert_eq!(event_count, 2);
    assert_eq!(distinct_event_timestamps, 1);

    let before_noncommitting = store.effect_counts()?;
    let future_attempt_floor = "4000-01-01T00:00:00Z";
    let mut replay_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::Intake,
        Some(&IdempotencyKey::new("idem_clock_floor_activate")),
        &RequestHash::new("sha256:clock-floor-activate"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(1),
        vec![
            pending_event_for_task("clock_floor_activate", future_task_id),
            pending_event_for_task("clock_floor_activate_second", future_task_id),
        ],
    );
    replay_input.clock_floor = Some(UtcTimestamp::parse(future_attempt_floor)?);
    let replay = store.commit_with(
        replay_input,
        |_, _| panic!("replay must not invoke the mutation closure"),
        response_json,
    )?;
    assert!(matches!(replay, MutationCommitOutcome::Replayed { .. }));
    assert_eq!(store.project_state()?.updated_at, future_floor);
    assert_eq!(store.effect_counts()?, before_noncommitting);

    let mut stale_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::Intake,
        Some(&IdempotencyKey::new("idem_clock_floor_stale")),
        &RequestHash::new("sha256:clock-floor-stale"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("clock_floor_stale", future_task_id)],
    );
    stale_input.clock_floor = Some(UtcTimestamp::parse(future_attempt_floor)?);
    let stale = store.commit_with(
        stale_input,
        |_, _| panic!("stale expected state must not invoke the mutation closure"),
        response_json,
    )?;
    assert!(matches!(
        stale,
        MutationCommitOutcome::StaleExpectedState { .. }
    ));
    assert_eq!(store.project_state()?.updated_at, future_floor);
    assert_eq!(store.effect_counts()?, before_noncommitting);

    let before_unrepresentable = store.effect_counts()?;
    let mut unrepresentable_floor = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::Intake,
        Some(&IdempotencyKey::new("idem_unrepresentable_clock_floor")),
        &RequestHash::new("sha256:unrepresentable-clock-floor"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(2),
        vec![pending_event_for_task(
            "unrepresentable_clock_floor",
            task_id,
        )],
    );
    unrepresentable_floor.clock_floor = Some(UtcTimestamp::parse("9999-12-31T23:59:59-23:59")?);
    let error = store
        .commit_with(unrepresentable_floor, |_, _| Ok(()), response_json)
        .expect_err("unrepresentable explicit clock floor must fail before effects");
    assert!(matches!(error, StoreError::InvalidInput { .. }));
    assert_eq!(store.effect_counts()?, before_unrepresentable);

    let remembered_floor = UtcTimestamp::parse("3000-01-01T00:00:00Z")?;
    store.remember_clock_sample(&remembered_floor);
    assert!(store.current_timestamp()? >= remembered_floor);
    drop(store);
    let reopened = harness.store()?;
    assert_eq!(reopened.current_timestamp()?, future_floor);
    Ok(())
}

#[test]
fn unrepresentable_remembered_clock_sample_rejects_commit_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let before_state = store.project_state()?;
    let before_effects = store.effect_counts()?;
    let unrepresentable = UtcTimestamp::parse("9999-12-31T23:59:59-23:59")?;
    assert!(unrepresentable
        .ensure_canonical_rfc3339_representable()
        .is_err());
    store.remember_clock_sample(&unrepresentable);

    let mut input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::Intake,
        Some(&IdempotencyKey::new(
            "idem_unrepresentable_remembered_clock",
        )),
        &RequestHash::new("sha256:unrepresentable-remembered-clock"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task(
            "unrepresentable_remembered_clock",
            "task_unrepresentable_remembered_clock",
        )],
    );
    input.include_live_storage_time = false;

    let error = store
        .commit_with(
            input,
            |_, _| panic!("invalid remembered sample must fail before mutation"),
            response_json,
        )
        .expect_err("unrepresentable remembered sample must fail closed");
    assert!(matches!(error, StoreError::SchemaInvariant { .. }));
    assert_eq!(store.project_state()?, before_state);
    assert_eq!(store.effect_counts()?, before_effects);
    Ok(())
}
