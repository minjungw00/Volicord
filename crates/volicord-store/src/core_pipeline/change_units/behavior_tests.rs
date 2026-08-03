use std::error::Error;

use rusqlite::params;
use volicord_types::ids::{BaselineRef, IdempotencyKey, ProjectId, RequestHash, TaskId};
use volicord_types::schema::ChangeUnitEffectContract;
use volicord_types::values::{ChangeUnitEffectKind, MethodName, TaskMode};

use super::{
    ChangeUnitInsert, ChangeUnitMutation, StoredChangeUnitLifecycle, StoredChangeUnitScopeSummary,
    StoredChangeUnitWriteBasis,
};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{commit_input, CoreStorageMutation, TaskMutation};
use crate::StoreError;

#[test]
fn change_unit_effect_contract_round_trips() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_effect_contract";
    let contract = ChangeUnitEffectContract {
        allowed_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
        forbidden_effects: vec![ChangeUnitEffectKind::ExternalNetwork],
        allowed_paths: vec!["src/export.rs".to_owned()],
        expected_outputs: vec!["Updated export behavior.".to_owned()],
        invariants: vec!["Keep unrelated behavior unchanged.".to_owned()],
        evidence_expectations: vec!["Record a focused test run.".to_owned()],
        sensitive_action_expectations: vec!["No secret access is expected.".to_owned()],
    };

    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_effect_contract")),
        &RequestHash::new("sha256:effect-contract"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("effect_contract", task_id)],
    );
    store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
                "cu_effect_contract",
                task_id,
                Some(contract.clone()),
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let record = store
        .current_change_unit(&TaskId::new(task_id))?
        .expect("current Change Unit should be readable");
    assert_eq!(record.effect_contract, Some(contract));
    Ok(())
}

#[test]
fn malformed_effect_contract_json_fails_closed_on_read() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_bad_effect_contract";
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_bad_effect_contract")),
        &RequestHash::new("sha256:bad-effect-contract"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("bad_effect_contract", task_id)],
    );
    store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
                "cu_bad_effect_contract",
                task_id,
                None,
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    store.conn.execute(
        "UPDATE change_units
            SET effect_contract_json = '{\"allowed_effects\":[\"not_an_effect\"]}'
          WHERE project_id = ?1 AND change_unit_id = ?2",
        params![PROJECT_ID, "cu_bad_effect_contract"],
    )?;

    let error = store
        .current_change_unit(&TaskId::new(task_id))
        .expect_err("unsupported effect contract values must fail closed");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateJson {
            table: "change_units",
            logical_column: "effect_contract_json",
            ..
        }
    ));
    Ok(())
}

#[test]
fn advisor_current_change_unit_is_observe_only_at_write_and_read_boundaries(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_advisor_change_unit";
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_advisor_change_unit")),
        &RequestHash::new("sha256:advisor-change-unit"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("advisor_change_unit", task_id)],
    );
    let invalid = store.commit_with(
        input.clone(),
        |mutation, facts| {
            let mut task = task_insert(task_id);
            task.mode = TaskMode::Advisor;
            CoreStorageMutation::Task(TaskMutation::insert(task))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
                "cu_advisor_invalid",
                task_id,
                Some(advisor_effect_contract()),
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    );
    assert!(matches!(invalid, Err(StoreError::InvalidInput { .. })));

    let valid_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_advisor_change_unit_valid")),
        &RequestHash::new("sha256:advisor-change-unit-valid"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("advisor_change_unit_valid", task_id)],
    );
    store.commit_with(
        valid_input,
        |mutation, facts| {
            let mut task = task_insert(task_id);
            task.mode = TaskMode::Advisor;
            CoreStorageMutation::Task(TaskMutation::insert(task))
                .apply(mutation, facts)
                .map(|_| ())?;
            let mut change_unit =
                change_unit_insert("cu_advisor_valid", task_id, Some(advisor_effect_contract()));
            change_unit.bounded_paths.clear();
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(store.current_change_unit(&TaskId::new(task_id))?.is_some());
    store.conn.execute(
        "UPDATE change_units SET bounded_paths_json = '[\"src/write.rs\"]'
          WHERE project_id = ?1 AND change_unit_id = ?2",
        params![PROJECT_ID, "cu_advisor_valid"],
    )?;
    assert!(matches!(
        store.current_change_unit(&TaskId::new(task_id)),
        Err(StoreError::SchemaInvariant { .. })
    ));
    Ok(())
}

fn advisor_effect_contract() -> ChangeUnitEffectContract {
    ChangeUnitEffectContract {
        allowed_effects: vec![ChangeUnitEffectKind::ArtifactRegistration],
        forbidden_effects: vec![
            ChangeUnitEffectKind::ProductFileWrite,
            ChangeUnitEffectKind::RunRecording,
            ChangeUnitEffectKind::SensitiveAction,
            ChangeUnitEffectKind::ExternalNetwork,
            ChangeUnitEffectKind::SecretAccess,
        ],
        allowed_paths: Vec::new(),
        expected_outputs: Vec::new(),
        invariants: Vec::new(),
        evidence_expectations: Vec::new(),
        sensitive_action_expectations: Vec::new(),
    }
}

fn change_unit_insert(
    change_unit_id: &str,
    task_id: &str,
    effect_contract: Option<ChangeUnitEffectContract>,
) -> ChangeUnitInsert {
    ChangeUnitInsert {
        change_unit_id: change_unit_id.to_owned(),
        task_id: task_id.to_owned(),
        scope_summary: StoredChangeUnitScopeSummary {
            scope_summary: Some("Store effect contract scope.".to_owned()),
            affected_areas: Vec::new(),
            constraints: Vec::new(),
        },
        bounded_paths: vec!["src/export.rs".to_owned()],
        write_basis: StoredChangeUnitWriteBasis {
            baseline_ref: Some(BaselineRef::new("baseline_store")),
            git_workspace_context: None,
        },
        effect_contract,
        lifecycle: StoredChangeUnitLifecycle {
            recovery_required: false,
        },
    }
}
