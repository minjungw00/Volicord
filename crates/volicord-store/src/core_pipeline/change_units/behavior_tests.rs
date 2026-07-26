use std::error::Error;

use serde_json::{json, Value};
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash, TaskId};
use volicord_types::values::MethodName;

use super::{ChangeUnitInsert, ChangeUnitMutation};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{commit_input, CoreStorageMutation, TaskMutation};
use crate::StoreError;

#[test]
fn change_unit_effect_contract_json_round_trips() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_effect_contract";
    let contract = json!({
        "allowed_effects": ["product_file_write"],
        "forbidden_effects": ["external_network"],
        "allowed_paths": ["src/export.rs"],
        "expected_outputs": ["Updated export behavior."],
        "invariants": ["Keep unrelated behavior unchanged."],
        "evidence_expectations": ["Record a focused test run."],
        "sensitive_action_expectations": ["No secret access is expected."]
    });

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
                contract.to_string(),
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let record = store
        .current_change_unit(&TaskId::new(task_id))?
        .expect("current Change Unit should be readable");
    assert_eq!(
        serde_json::from_str::<Value>(&record.effect_contract_json)?,
        contract
    );
    Ok(())
}

#[test]
fn malformed_effect_contract_json_rejects_commit_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_bad_effect_contract";
    let before = store.effect_counts()?;

    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_bad_effect_contract")),
        &RequestHash::new("sha256:bad-effect-contract"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("bad_effect_contract", task_id)],
    );
    let error = store
        .commit_with(
            input,
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(
                    change_unit_insert(
                        "cu_bad_effect_contract",
                        task_id,
                        r#"{"allowed_effects":["not_an_effect"]}"#.to_owned(),
                    ),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )
        .expect_err("unsupported effect contract values should reject");

    assert!(matches!(error, StoreError::InvalidInput { .. }));
    assert_eq!(store.effect_counts()?, before);
    Ok(())
}

fn change_unit_insert(
    change_unit_id: &str,
    task_id: &str,
    effect_contract_json: String,
) -> ChangeUnitInsert {
    ChangeUnitInsert {
        change_unit_id: change_unit_id.to_owned(),
        task_id: task_id.to_owned(),
        scope_summary_json: json!({
            "scope_summary": "Store effect contract scope."
        })
        .to_string(),
        bounded_paths_json: json!(["src/export.rs"]).to_string(),
        write_basis_json: json!({
            "baseline_ref": "baseline_store"
        })
        .to_string(),
        effect_contract_json,
        lifecycle_json: "{}".to_owned(),
    }
}
