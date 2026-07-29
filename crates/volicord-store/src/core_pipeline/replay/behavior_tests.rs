use std::error::Error;

use rusqlite::params;
use serde_json::json;
use sha2::{Digest, Sha256};
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash};
use volicord_types::values::{ActorSource, MethodName, OperationCategory};

use crate::core_pipeline::test_support::{
    pending_event, replay_context, response_json, task_insert, StoreFixture as StoreHarness,
    ACTOR_SOURCE, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, CoreStorageMutation, MutationCommitOutcome, TaskMutation,
};
use crate::sqlite::open_project_state_database_for_test;
use crate::StoreError;

#[test]
fn transaction_replay_context_mismatch_precedes_request_hash_conflict() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let first_context = replay_context(CONNECTION_ID, "agent_workflow");
    let first_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_context")),
        &RequestHash::new("sha256:first"),
        Some(first_context),
        Some(0),
        vec![pending_event("first")],
    );
    let first = store.commit_with(
        first_input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_first")))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
    let before = store.effect_counts()?;

    let mismatch_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_context")),
        &RequestHash::new("sha256:second"),
        Some(replay_context("conn_other", "agent_workflow")),
        Some(1),
        vec![pending_event("second")],
    );
    let mismatch = store.commit_with(mismatch_input, |_, _| Ok(()), response_json)?;

    assert!(matches!(
        mismatch,
        MutationCommitOutcome::ReplayContextMismatch { .. }
    ));
    assert_eq!(store.effect_counts()?, before);
    Ok(())
}

#[test]
fn transaction_replay_rejects_changed_git_workspace_context() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let mut first_context = replay_context(CONNECTION_ID, "agent_workflow");
    first_context.git_workspace_context = Some(
        json!({
            "git_common_dir": "/tmp/repo/.git",
            "worktree_id": format!("sha256:{}", "1".repeat(64)),
            "branch_ref": "refs/heads/original",
            "head_sha": "1111111111111111111111111111111111111111",
            "workspace_fingerprint": format!("sha256:{}", "2".repeat(64))
        })
        .as_object()
        .cloned()
        .expect("object"),
    );
    let first_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_workspace_context")),
        &RequestHash::new("sha256:same-request"),
        Some(first_context.clone()),
        Some(0),
        vec![pending_event("workspace_first")],
    );
    let first = store.commit_with(
        first_input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_workspace_first")))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
    let before = store.effect_counts()?;

    let mut changed_basis = first_context.clone();
    changed_basis.verification_basis = Some("different_verified_channel".to_owned());
    let basis_replay_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_workspace_context")),
        &RequestHash::new("sha256:same-request"),
        Some(changed_basis),
        Some(1),
        vec![pending_event("basis_second")],
    );
    let basis_replay = store.commit_with(basis_replay_input, |_, _| Ok(()), response_json)?;
    assert!(matches!(
        basis_replay,
        MutationCommitOutcome::ReplayContextMismatch { .. }
    ));
    assert_eq!(store.effect_counts()?, before);

    let mut changed_context = first_context;
    changed_context.git_workspace_context = Some(
        json!({
            "git_common_dir": "/tmp/repo/.git",
            "worktree_id": format!("sha256:{}", "3".repeat(64)),
            "branch_ref": "refs/heads/other",
            "head_sha": "2222222222222222222222222222222222222222",
            "workspace_fingerprint": format!("sha256:{}", "4".repeat(64))
        })
        .as_object()
        .cloned()
        .expect("object"),
    );
    let replay_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_workspace_context")),
        &RequestHash::new("sha256:same-request"),
        Some(changed_context),
        Some(1),
        vec![pending_event("workspace_second")],
    );
    let replay = store.commit_with(replay_input, |_, _| Ok(()), response_json)?;

    assert!(matches!(
        replay,
        MutationCommitOutcome::ReplayContextMismatch { .. }
    ));
    assert_eq!(store.effect_counts()?, before);
    Ok(())
}

#[test]
fn malformed_stored_git_workspace_replay_context_is_corruption() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let mut context = replay_context(CONNECTION_ID, "agent_workflow");
    context.git_workspace_context = Some(
        json!({
            "git_common_dir": "/tmp/repo/.git",
            "worktree_id": format!("sha256:{}", "1".repeat(64)),
            "branch_ref": "refs/heads/original",
            "head_sha": "1111111111111111111111111111111111111111",
            "workspace_fingerprint": format!("sha256:{}", "2".repeat(64))
        })
        .as_object()
        .cloned()
        .expect("object"),
    );
    let idempotency_key = IdempotencyKey::new("idem_store_workspace_corrupt");
    let first = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&idempotency_key),
            &RequestHash::new("sha256:workspace-corrupt"),
            Some(context),
            Some(0),
            vec![pending_event("workspace_corrupt")],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_workspace_corrupt")))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
    drop(store);

    let conn = open_project_state_database_for_test(harness.state_database_path())?;
    conn.execute(
        "UPDATE tool_invocations
                SET git_workspace_context_json = '{\"unexpected\":true}'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
        params![
            PROJECT_ID,
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str()
        ],
    )?;
    drop(conn);

    let store = harness.store()?;
    let error = store
        .tool_invocation(MethodName::UpdateScope, &idempotency_key)
        .expect_err("malformed replay workspace context must be corrupt owner state");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateJson {
            table: "tool_invocations",
            logical_column: "git_workspace_context_json",
            ..
        }
    ));
    Ok(())
}

#[test]
fn transaction_replay_returns_stored_response_before_stale_expected_state(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let context = replay_context(CONNECTION_ID, "agent_workflow");
    let first_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_replay_stale")),
        &RequestHash::new("sha256:replay"),
        Some(context.clone()),
        Some(0),
        vec![pending_event("replay_stale_first")],
    );
    let first = store.commit_with(
        first_input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_replay_stale_first")))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    let MutationCommitOutcome::Committed {
        response_json: stored_response,
        ..
    } = first
    else {
        panic!("first transaction should commit");
    };
    let before_replay = store.effect_counts()?;

    let replay_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_replay_stale")),
        &RequestHash::new("sha256:replay"),
        Some(context),
        Some(0),
        vec![pending_event("replay_stale_second")],
    );
    let replay = store.commit_with(
        replay_input,
        |_, _| panic!("eligible replay must not apply a second mutation"),
        |_| panic!("eligible replay must not build a fresh response"),
    )?;

    assert!(matches!(
        replay,
        MutationCommitOutcome::Replayed {
            response_json,
            ..
        } if response_json == stored_response
    ));
    assert_eq!(store.effect_counts()?, before_replay);
    Ok(())
}

#[test]
fn operation_result_reuses_exact_replay_bytes_and_metadata() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let idempotency_key = IdempotencyKey::new("idem_store_operation_result");
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&idempotency_key),
        &RequestHash::new("sha256:operation-result"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event("operation_result")],
    );
    let committed = store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_operation_result")))
                .apply(mutation, facts)
                .map(|_| ())
        },
        |facts| {
            Ok(format!(
                "{{\"base\":{{\"state_version\":{}}},\"unicode\":\"결과🙂\"}}",
                facts.committed_state_version
            ))
        },
    )?;
    let MutationCommitOutcome::Committed { response_json, .. } = committed else {
        panic!("operation-result fixture should commit");
    };

    let stored = store
        .operation_result(MethodName::UpdateScope, &idempotency_key)?
        .expect("committed replay response should be retrievable");
    assert_eq!(stored.project_id, PROJECT_ID);
    assert_eq!(stored.source_method, MethodName::UpdateScope);
    assert_eq!(stored.source_idempotency_key, idempotency_key);
    assert_eq!(stored.committed_state_version, 1);
    assert_eq!(
        stored.actor_source,
        ActorSource::agent_connection(CONNECTION_ID)
    );
    assert_eq!(stored.operation_category, OperationCategory::AgentWorkflow);
    assert_eq!(stored.response_json, response_json);
    assert_eq!(stored.response_size_bytes, response_json.len() as u64);
    assert_eq!(
        stored.response_sha256,
        format!("sha256:{:x}", Sha256::digest(response_json.as_bytes()))
    );
    Ok(())
}

#[test]
fn invalid_replay_identity_is_rejected_before_transaction_and_effects() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let before_state = store.project_state()?;
    let before_effects = store.effect_counts()?;

    let mut blank_basis = replay_context(CONNECTION_ID, "agent_workflow");
    blank_basis.verification_basis = Some(" \t ".to_owned());
    let mut invalid_git_context = replay_context(CONNECTION_ID, "agent_workflow");
    invalid_git_context.git_workspace_context = Some(serde_json::Map::new());

    for (case, context, expected_field) in [
        ("basis", blank_basis, "verification_basis"),
        (
            "git_context",
            invalid_git_context,
            "tool_invocations.git_workspace_context_json",
        ),
    ] {
        let idempotency_key = IdempotencyKey::new(format!("idem_invalid_replay_identity_{case}"));
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&idempotency_key),
            &RequestHash::new(format!("sha256:invalid-replay-identity-{case}")),
            Some(context),
            Some(before_state.state_version),
            vec![pending_event(&format!("invalid_replay_identity_{case}"))],
        );
        let error = store
            .commit_with(
                input,
                |_, _| panic!("invalid replay identity must not apply a mutation"),
                |_| panic!("invalid replay identity must not build a response"),
            )
            .expect_err("invalid replay identity must fail before commit");
        let StoreError::InvalidInput { detail } = error else {
            panic!("unexpected invalid replay identity error: {error}");
        };
        assert!(
            detail.starts_with(expected_field),
            "{case} reported unexpected detail: {detail}"
        );
        assert!(store.conn.is_autocommit());
        assert_eq!(store.project_state()?, before_state);
        let after_effects = store.effect_counts()?;
        assert_eq!(after_effects.state_version, before_effects.state_version);
        assert_eq!(
            after_effects.authority_events,
            before_effects.authority_events
        );
        assert_eq!(
            after_effects.tool_invocations,
            before_effects.tool_invocations
        );
        assert_eq!(after_effects, before_effects);
        assert!(store
            .tool_invocation(MethodName::UpdateScope, &idempotency_key)?
            .is_none());
    }
    Ok(())
}

#[test]
fn loaded_replay_context_rejects_corrupt_typed_identity_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let idempotency_key = IdempotencyKey::new("idem_store_loaded_replay_identity");
    let context = replay_context(CONNECTION_ID, "agent_workflow");
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&idempotency_key),
        &RequestHash::new("sha256:loaded-replay-identity"),
        Some(context.clone()),
        Some(0),
        vec![pending_event("loaded_replay_identity")],
    );
    let committed = store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(
                "task_loaded_replay_identity",
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(committed, MutationCommitOutcome::Committed { .. }));
    let before = store.effect_counts()?;
    let expected_record_ref = format!(
        "{PROJECT_ID}/{}/{}",
        MethodName::UpdateScope.as_str(),
        idempotency_key.as_str()
    );
    let assert_corrupt_value = |error: StoreError, expected_column: &str| match error {
        StoreError::CorruptOwnerStateValue {
            database_kind,
            table,
            record_ref,
            logical_column,
        } => {
            assert_eq!(database_kind, "project_state");
            assert_eq!(table, "tool_invocations");
            assert_eq!(record_ref, expected_record_ref);
            assert_eq!(logical_column, expected_column);
        }
        other => panic!("unexpected replay identity error: {other}"),
    };

    store.conn.execute(
        "UPDATE tool_invocations
                SET actor_source = 'not-an-actor'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
        params![
            PROJECT_ID,
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str()
        ],
    )?;
    let actor_error = store
        .operation_result(MethodName::UpdateScope, &idempotency_key)
        .expect_err("malformed stored actor source must fail closed");
    assert_corrupt_value(actor_error, "actor_source");
    store.conn.execute(
        "UPDATE tool_invocations
                SET actor_source = ?4
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
        params![
            PROJECT_ID,
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str(),
            ACTOR_SOURCE
        ],
    )?;

    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = ON")?;
    store.conn.execute(
        "UPDATE tool_invocations
                SET operation_category = 'unsupported'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
        params![
            PROJECT_ID,
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str()
        ],
    )?;
    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = OFF")?;
    let category_error = store
        .tool_invocation(MethodName::UpdateScope, &idempotency_key)
        .expect_err("unsupported stored operation category must fail closed");
    assert_corrupt_value(category_error, "operation_category");
    store.conn.execute(
        "UPDATE tool_invocations
                SET operation_category = 'agent_workflow'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
        params![
            PROJECT_ID,
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str()
        ],
    )?;

    store.conn.execute(
        "UPDATE tool_invocations
                SET verification_basis = ''
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
        params![
            PROJECT_ID,
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str()
        ],
    )?;
    let replay_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&idempotency_key),
        &RequestHash::new("sha256:loaded-replay-identity"),
        Some(context),
        Some(0),
        vec![pending_event("loaded_replay_identity")],
    );
    let basis_error = store
        .commit_with(
            replay_input,
            |_, _| panic!("corrupt replay identity must not apply a mutation"),
            |_| panic!("corrupt replay identity must not rebuild a response"),
        )
        .expect_err("empty stored verification basis must fail closed");
    assert_corrupt_value(basis_error, "verification_basis");
    assert_eq!(store.effect_counts()?, before);
    Ok(())
}

#[test]
fn transaction_replay_hash_conflict_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let context = replay_context(CONNECTION_ID, "agent_workflow");
    let first_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_hash_conflict")),
        &RequestHash::new("sha256:first"),
        Some(context.clone()),
        Some(0),
        vec![pending_event("hash_conflict_first")],
    );
    let first = store.commit_with(
        first_input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(
                "task_hash_conflict_first",
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
    let before_conflict = store.effect_counts()?;

    let conflict_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_hash_conflict")),
        &RequestHash::new("sha256:second"),
        Some(context),
        Some(1),
        vec![pending_event("hash_conflict_second")],
    );
    let conflict = store.commit_with(
        conflict_input,
        |_, _| panic!("hash conflict must not apply a second mutation"),
        |_| panic!("hash conflict must not build a fresh response"),
    )?;

    assert!(matches!(
        conflict,
        MutationCommitOutcome::IdempotencyConflict {
            stored_request_hash,
            attempted_request_hash,
            ..
        } if stored_request_hash == "sha256:first"
            && attempted_request_hash == "sha256:second"
    ));
    assert_eq!(store.effect_counts()?, before_conflict);
    Ok(())
}
