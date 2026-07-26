use std::error::Error;

use rusqlite::params;
use serde_json::json;
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash, TaskId};
use volicord_types::values::MethodName;

use super::{EvidenceMutation, EvidenceSummaryUpsert};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, ACTOR_SOURCE, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, ChangeUnitInsert, ChangeUnitMutation, CoreStorageMutation,
    EvidenceCaptureIntentInsert, TaskMutation,
};
use crate::StoreError;

#[test]
fn latest_evidence_summary_uses_state_version_when_time_and_ids_disagree(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_evidence_summary_authority_order";
    let fixed_time = "2999-07-13T12:34:56.789123456Z";

    let mut first_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordRun,
        Some(&IdempotencyKey::new("idem_summary_authority_old")),
        &RequestHash::new("sha256:summary-authority-old"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("summary_authority_old", task_id)],
    );
    first_input.clock_floor = Some(fixed_time.to_owned());
    first_input.include_live_storage_time = false;
    store.commit_with(
        first_input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::Evidence(EvidenceMutation::UpsertSummary(evidence_summary_upsert(
                "summary_z_old",
                task_id,
                "run_summary_old",
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let mut second_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordRun,
        Some(&IdempotencyKey::new("idem_summary_authority_new")),
        &RequestHash::new("sha256:summary-authority-new"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(1),
        vec![pending_event_for_task("summary_authority_new", task_id)],
    );
    second_input.clock_floor = Some(fixed_time.to_owned());
    second_input.include_live_storage_time = false;
    store.commit_with(
        second_input,
        |mutation, facts| {
            CoreStorageMutation::Evidence(EvidenceMutation::UpsertSummary(evidence_summary_upsert(
                "summary_a_new",
                task_id,
                "run_summary_new",
            )))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let latest = store
        .latest_evidence_summary(&TaskId::new(task_id))?
        .expect("latest evidence summary should exist");
    assert_eq!(latest.evidence_summary_id, "summary_a_new");
    assert_eq!(latest.produced_at_state_version, 2);
    let timestamps = store
        .conn
        .prepare(
            "SELECT created_at
                   FROM evidence_summaries
                  WHERE project_id = ?1 AND task_id = ?2
                  ORDER BY evidence_summary_id",
        )?
        .query_map(params![PROJECT_ID, task_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        timestamps,
        vec![fixed_time.to_owned(), fixed_time.to_owned()]
    );
    assert_eq!(store.project_state()?.updated_at, fixed_time);

    let before_counts = store.effect_counts()?;
    let before_state = store.project_state()?;
    let mut duplicate_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordRun,
        Some(&IdempotencyKey::new("idem_summary_authority_duplicate")),
        &RequestHash::new("sha256:summary-authority-duplicate"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(2),
        vec![pending_event_for_task(
            "summary_authority_duplicate",
            task_id,
        )],
    );
    duplicate_input.clock_floor = Some(fixed_time.to_owned());
    duplicate_input.include_live_storage_time = false;
    let error = store
        .commit_with(
            duplicate_input,
            |mutation, facts| {
                for summary_id in ["summary_duplicate_first", "summary_duplicate_second"] {
                    CoreStorageMutation::Evidence(EvidenceMutation::UpsertSummary(
                        evidence_summary_upsert(summary_id, task_id, "run_summary_duplicate"),
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )
        .expect_err("one Task cannot have two summaries produced by one commit");
    assert!(matches!(error, StoreError::Sqlite(_)));
    assert_eq!(store.effect_counts()?, before_counts);
    assert_eq!(store.project_state()?, before_state);
    assert_eq!(
        store
            .latest_evidence_summary(&TaskId::new(task_id))?
            .expect("rolled-back duplicate must preserve current summary")
            .evidence_summary_id,
        "summary_a_new"
    );
    Ok(())
}

#[test]
fn evidence_capture_intent_insert_rejects_extended_ttl_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_capture_intent_extended_ttl";
    let change_unit_id = "cu_capture_intent_extended_ttl";
    let before_state = store.project_state()?;
    let before_effects = store.effect_counts()?;
    let capture_intent = EvidenceCaptureIntentInsert {
        evidence_capture_intent_id: "capture_intent_extended_ttl".to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: change_unit_id.to_owned(),
        scope_revision: 0,
        baseline_ref: "baseline_capture_intent_extended_ttl".to_owned(),
        target_json: json!({
            "target_kind": "supplemental_claim",
            "evidence_claim_id": "claim_capture_intent_extended_ttl",
            "statement": "A fixed capture-intent TTL is required."
        })
        .to_string(),
        capture_kind: "verified_command_execution".to_owned(),
        capture_spec_json: json!({
            "capture_type": "verified_command_execution",
            "command_summary": "Run a bounded local verification."
        })
        .to_string(),
        input_sha256: "a".repeat(64),
        expected_outcome_json: "{}".to_owned(),
        requested_by_actor_source: ACTOR_SOURCE.to_owned(),
        requesting_connection_internal_id: CONNECTION_ID.to_owned(),
        session_context_json: "{}".to_owned(),
        workspace_context_json: "{}".to_owned(),
        created_at: "2026-01-01T00:00:00Z".to_owned(),
        expires_at: "2026-01-01T00:16:00Z".to_owned(),
        metadata_json: "{}".to_owned(),
    };

    let mutations = [
        CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
        CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
            change_unit_id,
            task_id,
            "null".to_owned(),
        ))),
        CoreStorageMutation::Evidence(EvidenceMutation::InsertCaptureIntent(capture_intent)),
    ];
    let error = store
        .commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::PrepareEvidenceCapture,
                Some(&IdempotencyKey::new("idem_capture_intent_extended_ttl")),
                &RequestHash::new("sha256:capture-intent-extended-ttl"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "capture_intent_extended_ttl",
                    task_id,
                )],
            ),
            &mutations,
            response_json,
        )
        .expect_err("a 16-minute evidence-capture intent TTL must reject atomically");

    assert!(matches!(error, StoreError::SchemaInvariant { .. }));
    assert_eq!(store.project_state()?, before_state);
    assert_eq!(store.effect_counts()?, before_effects);
    Ok(())
}

fn evidence_summary_upsert(
    evidence_summary_id: &str,
    task_id: &str,
    updated_by_run_id: &str,
) -> EvidenceSummaryUpsert {
    EvidenceSummaryUpsert {
        evidence_summary_id: evidence_summary_id.to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: None,
        status: "unknown".to_owned(),
        coverage_json: "[]".to_owned(),
        supporting_refs_json: "[]".to_owned(),
        gap_refs_json: "[]".to_owned(),
        metadata_json: json!({ "updated_by_run_id": updated_by_run_id }).to_string(),
    }
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
