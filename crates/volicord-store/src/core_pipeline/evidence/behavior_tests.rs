use std::error::Error;

use rusqlite::params;
use volicord_types::ids::{
    AgentConnectionId, BaselineRef, EvidenceClaimId, IdempotencyKey, ProjectId, RequestHash, RunId,
    TaskId,
};
use volicord_types::schema::{
    evidence_capture_expected_outcome, EvidenceCaptureSpec, EvidenceTarget,
    PersistedEvidenceMetadata,
};
use volicord_types::values::{ActorSource, EvidenceStatus, MethodName, UtcTimestamp};

use super::{EvidenceMutation, EvidenceSummaryUpsert};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, ACTOR_SOURCE, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, ChangeUnitInsert, ChangeUnitMutation, CoreStorageMutation,
    EvidenceCaptureIntentInsert, StoredChangeUnitLifecycle, StoredChangeUnitScopeSummary,
    StoredChangeUnitWriteBasis, TaskMutation,
};
use crate::evidence_capture::{
    StoredEvidenceCaptureIntentMetadata, StoredEvidenceCaptureIntentSessionContext,
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
    first_input.clock_floor = Some(UtcTimestamp::parse(fixed_time)?);
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
    second_input.clock_floor = Some(UtcTimestamp::parse(fixed_time)?);
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
    assert_eq!(
        store.project_state()?.updated_at,
        UtcTimestamp::parse(fixed_time)?
    );

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
    duplicate_input.clock_floor = Some(UtcTimestamp::parse(fixed_time)?);
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
    let capture = EvidenceCaptureSpec::VerifiedCommandExecution {
        command_sha256: "a".repeat(64),
        command_label: "Run a bounded local verification.".to_owned(),
        expected_exit_code: Some(0).into(),
    };
    let capture_intent = EvidenceCaptureIntentInsert {
        evidence_capture_intent_id: "capture_intent_extended_ttl".to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: change_unit_id.to_owned(),
        scope_revision: 0,
        baseline_ref: BaselineRef::new("baseline_capture_intent_extended_ttl"),
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: EvidenceClaimId::new("claim_capture_intent_extended_ttl"),
            statement: "A fixed capture-intent TTL is required.".to_owned(),
        },
        capture: capture.clone(),
        input_sha256: "a".repeat(64),
        expected_outcome: evidence_capture_expected_outcome(&capture),
        requested_by_actor_source: ACTOR_SOURCE.parse::<ActorSource>()?,
        requesting_connection_internal_id: AgentConnectionId::new(CONNECTION_ID),
        session_context: StoredEvidenceCaptureIntentSessionContext { session_id: None },
        workspace_context: Default::default(),
        created_at: UtcTimestamp::parse("2026-01-01T00:00:00Z")?,
        expires_at: UtcTimestamp::parse("2026-01-01T00:16:00Z")?,
        metadata: StoredEvidenceCaptureIntentMetadata {
            verification_basis: "test".to_owned(),
        },
    };

    let mutations = [
        CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
        CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(change_unit_insert(
            change_unit_id,
            task_id,
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
        status: EvidenceStatus::Unknown,
        coverage: Vec::new(),
        supporting_refs: Vec::new(),
        gap_refs: Vec::new(),
        metadata: PersistedEvidenceMetadata {
            updated_by_run_id: RunId::new(updated_by_run_id),
        },
    }
}

fn change_unit_insert(change_unit_id: &str, task_id: &str) -> ChangeUnitInsert {
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
        effect_contract: None,
        lifecycle: StoredChangeUnitLifecycle {
            recovery_required: false,
        },
    }
}
