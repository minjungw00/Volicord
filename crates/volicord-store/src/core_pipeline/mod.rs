#[cfg(test)]
use rusqlite::params;
#[cfg(test)]
use volicord_types::values::{
    UserActionBasisStatus, UserActionChannelKind, UserActionKind, UtcTimestamp,
};

pub use crate::evidence_capture::{
    derive_evidence_capture_source_claims, EvidenceCaptureIntentInsert,
    EvidenceCaptureIntentRecord, EvidenceCaptureReceiptInsert, EvidenceCaptureReceiptRecord,
    EvidenceCaptureSourceClaimIdentity, EvidenceCaptureSourceClaimKind,
    EvidenceCaptureSourceClaimRecord, EvidenceProducerInsert, EvidenceProducerRecord,
};

pub use self::commit::commit_input;

/// Pending event supplied by a method-specific commit branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTaskEvent {
    pub event_id: String,
    pub task_id: Option<String>,
    pub change_unit_id: Option<String>,
    pub event_kind: String,
    pub event_payload_json: String,
}

/// Event reference facts created by an atomic mutation commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedEventRef {
    pub event_id: String,
    pub event_kind: String,
}

/// Facts available to build the exact committed response before replay storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedMutationFacts {
    pub basis_state_version: u64,
    pub committed_state_version: u64,
    pub events: Vec<CommittedEventRef>,
}

/// Input for an atomic Core mutation commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMutationInput {
    pub project_id: String,
    pub tool_name: String,
    pub idempotency_key: Option<String>,
    pub request_hash: String,
    pub replay_context: Option<VerifiedReplayContext>,
    pub expected_state_version: Option<u64>,
    pub clock_floor: Option<String>,
    /// Whether commit time must also sample SQLite's live UTC clock.
    pub include_live_storage_time: bool,
    pub events: Vec<PendingTaskEvent>,
}

/// Result of attempting a mutating commit through the replay/freshness gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationCommitOutcome {
    Replayed {
        response_json: String,
        basis_state_version: u64,
        committed_state_version: u64,
    },
    ReplayContextMismatch {
        current_state_version: u64,
        idempotency_key: String,
    },
    IdempotencyConflict {
        current_state_version: u64,
        idempotency_key: String,
        stored_request_hash: String,
        attempted_request_hash: String,
    },
    StaleExpectedState {
        current_state_version: u64,
        expected_state_version: u64,
    },
    Committed {
        response_json: String,
        basis_state_version: u64,
        committed_state_version: u64,
        events: Vec<CommittedEventRef>,
    },
}

mod agent_sessions;
mod artifacts;
mod blockers;
mod change_units;
pub(crate) mod clock;
mod commit;
mod continuity;
mod enforcement_profile;
mod events;
mod evidence;
mod facade;
mod inspection;
pub(crate) mod mutations;
mod open;
mod project_state;
mod reconciliation;
mod record_refs;
mod replay;
mod runs;
mod tasks;
mod user_actions;
pub(crate) mod validation;
mod write_tickets;

pub use crate::workflow_records::{ProjectWorkflowPolicyMutation, WorkflowPolicyMutation};
pub use artifacts::{
    ArtifactLinkInsert, ArtifactMutation, ArtifactPromotion, StoredArtifactRecord,
    StoredArtifactStagingRecord,
};
pub use change_units::{ChangeUnitInsert, ChangeUnitMutation, ChangeUnitRecord};
pub use continuity::{
    ActiveProjectContinuityPage, ContinuityMutation, ProjectContinuityRecordInsert,
    ProjectContinuityRecordRecord, UnrecordedChangeResolutionUpdate,
};
pub use enforcement_profile::ProjectEnforcementProfileRecord;
pub use evidence::{
    EvidenceClaimInsert, EvidenceMutation, EvidenceObservationInsert, EvidenceObservationRecord,
    EvidenceSummaryRecord, EvidenceSummaryUpsert,
};
pub use facade::CoreProjectStore;
pub use inspection::StorageEffectCounts;
pub use mutations::CoreStorageMutation;
pub use project_state::ProjectStateHeader;
pub use reconciliation::ProductWriteObservationCandidate;
pub use record_refs::StoredRecordRef;
pub use replay::{StoredOperationResult, ToolInvocationRecord, VerifiedReplayContext};
pub use runs::{RunInsert, RunMutation, RunObservedChangesRecord, RunRecord};
pub use tasks::{
    AcceptanceCriteriaReplace, AcceptanceCriterionRecord, AcceptanceCriterionUpsert,
    EvidenceClaimRecord, TaskCloseBasisUpdate, TaskCloseUpdate, TaskControlLevelUpdate, TaskInsert,
    TaskMutation, TaskRecord, TaskRevisionRecord, TaskScopeRevisionUpdate, TaskScopeUpdate,
};
pub use user_actions::{
    effective_user_action_status, EffectiveUserActionRecord, UserActionBasisStatusMark,
    UserActionBasisUpdate, UserActionInvalidation, UserActionMutation, UserActionRequestInsert,
    UserActionRequestRecord, UserActionResolutionInsert, UserActionResolutionRecord,
};
pub use write_tickets::{
    WriteTicketByIdInvalidation, WriteTicketConsumption, WriteTicketInsert,
    WriteTicketInvalidation, WriteTicketMutation, WriteTicketRecord,
};

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::ids::{
        IdempotencyKey, ProjectContinuityRecordId, ProjectId, RecordId, RequestHash, TaskId,
    };
    use volicord_types::schema::{
        ContinuityCursor, RequiredNullable, StateRecordRef, UserActionBasis,
        MAX_CONTINUITY_PAGE_SIZE,
    };
    use volicord_types::values::{
        JudgmentResolutionOutcome, MethodName, StateRecordKind, UserActionOptionAction,
        UserActionStatus,
    };

    use super::clock::advance_project_utc_floor_tx;
    use super::*;
    use crate::bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };
    use crate::mutation::TestRuntimeHomeAdmission;
    use crate::sqlite::open_project_state_database_for_test;
    use crate::{StoreError, StoreResult};

    const PROJECT_ID: &str = "project_store";
    const CONNECTION_ID: &str = "conn_store";
    const ACTOR_SOURCE: &str = "agent_connection:conn_store";

    struct StoreHarness {
        _runtime_home: TempRuntimeHome,
        mutation: TestRuntimeHomeAdmission,
        runtime_home_path: PathBuf,
    }

    impl StoreHarness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("store-replay-context")?;
            let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
            let setup_context = setup.context()?;
            initialize_runtime_home(&setup_context, "runtime_home_store", "{}")?;
            register_project(
                &setup_context,
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root: runtime_home.create_product_repo("repo")?,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            drop(setup_context);
            drop(setup);
            let mutation = TestRuntimeHomeAdmission::shared(runtime_home.path())?;

            Ok(Self {
                runtime_home_path: runtime_home.path().to_path_buf(),
                mutation,
                _runtime_home: runtime_home,
            })
        }

        fn store(&self) -> StoreResult<CoreProjectStore<'_>> {
            CoreProjectStore::open_for_mutation(
                &self.mutation.context()?,
                &ProjectId::new(PROJECT_ID),
            )
        }
    }

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
        input.clock_floor = Some(configured_floor.to_owned());

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
        let committed_at = UtcTimestamp::parse(&store.project_state()?.updated_at)?;
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
            params![PROJECT_ID, original_floor],
        )?;
        assert_eq!(store.effect_counts()?, before);
        drop(store);
        let mut conn = open_project_state_database_for_test(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
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
        assert_eq!(after_floor, original_floor);
        drop(conn);
        assert_eq!(harness.store()?.effect_counts()?, before);
        Ok(())
    }

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
                CoreStorageMutation::Evidence(EvidenceMutation::UpsertSummary(
                    evidence_summary_upsert("summary_z_old", task_id, "run_summary_old"),
                ))
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
                CoreStorageMutation::Evidence(EvidenceMutation::UpsertSummary(
                    evidence_summary_upsert("summary_a_new", task_id, "run_summary_new"),
                ))
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
    fn prepared_artifact_eligibility_uses_exact_submillisecond_expiry() -> Result<(), Box<dyn Error>>
    {
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

    #[test]
    fn explicit_future_clock_floor_survives_active_task_commit_and_reopen(
    ) -> Result<(), Box<dyn Error>> {
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
        clock_floor_input.clock_floor = Some(future_floor.to_string());
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
                    lifecycle_phase: "completed".to_owned(),
                    result: "completed".to_owned(),
                    close_summary_json: "{\"close_reason\":\"completed_self_checked\"}".to_owned(),
                    closed_at: "2999-07-13T12:00:00Z".to_owned(),
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
        assert_eq!(state.updated_at, expected);
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
        replay_input.clock_floor = Some(future_attempt_floor.to_owned());
        let replay = store.commit_with(
            replay_input,
            |_, _| panic!("replay must not invoke the mutation closure"),
            response_json,
        )?;
        assert!(matches!(replay, MutationCommitOutcome::Replayed { .. }));
        assert_eq!(store.project_state()?.updated_at, expected);
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
        stale_input.clock_floor = Some(future_attempt_floor.to_owned());
        let stale = store.commit_with(
            stale_input,
            |_, _| panic!("stale expected state must not invoke the mutation closure"),
            response_json,
        )?;
        assert!(matches!(
            stale,
            MutationCommitOutcome::StaleExpectedState { .. }
        ));
        assert_eq!(store.project_state()?.updated_at, expected);
        assert_eq!(store.effect_counts()?, before_noncommitting);

        let before_invalid = store.effect_counts()?;
        let mut invalid_floor = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_invalid_clock_floor")),
            &RequestHash::new("sha256:invalid-clock-floor"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(2),
            vec![pending_event_for_task("invalid_clock_floor", task_id)],
        );
        invalid_floor.clock_floor = Some("not-a-timestamp".to_owned());
        let error = store
            .commit_with(invalid_floor, |_, _| Ok(()), response_json)
            .expect_err("invalid explicit clock floor must fail before effects");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before_invalid);

        let remembered_floor = UtcTimestamp::parse("3000-01-01T00:00:00Z")?;
        store.remember_clock_sample(&remembered_floor);
        assert!(UtcTimestamp::parse(&store.current_timestamp()?)? >= remembered_floor);
        drop(store);
        let reopened = harness.store()?;
        assert_eq!(reopened.current_timestamp()?, expected);
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

    #[test]
    fn semantic_timestamp_inputs_reject_atomically_before_durable_rows(
    ) -> Result<(), Box<dyn Error>> {
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

        let write_ticket = store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::PrepareWrite,
                Some(&IdempotencyKey::new("idem_invalid_write_ticket_expiry")),
                &RequestHash::new("sha256:invalid-write-ticket-expiry"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(1),
                vec![pending_event_for_task(
                    "invalid_write_ticket_expiry",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::WriteTicket(WriteTicketMutation::insert(WriteTicketInsert {
                    write_ticket_id: "write_ticket_invalid_expiry".to_owned(),
                    task_id: task_id.to_owned(),
                    change_unit_id: "change_unit_missing".to_owned(),
                    validity_basis_json: "{}".to_owned(),
                    allowed_path_prefixes_json: "[]".to_owned(),
                    denied_path_prefixes_json: "[]".to_owned(),
                    attempt_scope_json: "{}".to_owned(),
                    created_by_actor_source: ACTOR_SOURCE.to_owned(),
                    created_by_user_action_resolution_id: None,
                    idle_expires_at: Some("tomorrow".to_owned()),
                    created_at: "2026-07-13T00:00:00Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                }))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        );
        assert!(matches!(write_ticket, Err(StoreError::InvalidInput { .. })));
        assert_eq!(store.effect_counts()?, before);
        Ok(())
    }

    #[test]
    fn transaction_replay_context_mismatch_precedes_request_hash_conflict(
    ) -> Result<(), Box<dyn Error>> {
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
        first_context.git_workspace_context_json =
            Some(volicord_types::canonical::canonical_json_string(&json!({
                "git_common_dir": "/tmp/repo/.git",
                "worktree_id": format!("sha256:{}", "1".repeat(64)),
                "branch_ref": "refs/heads/original",
                "head_sha": "1111111111111111111111111111111111111111",
                "workspace_fingerprint": format!("sha256:{}", "2".repeat(64))
            }))?);
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
        changed_context.git_workspace_context_json =
            Some(volicord_types::canonical::canonical_json_string(&json!({
                "git_common_dir": "/tmp/repo/.git",
                "worktree_id": format!("sha256:{}", "3".repeat(64)),
                "branch_ref": "refs/heads/other",
                "head_sha": "2222222222222222222222222222222222222222",
                "workspace_fingerprint": format!("sha256:{}", "4".repeat(64))
            }))?);
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
        context.git_workspace_context_json =
            Some(volicord_types::canonical::canonical_json_string(&json!({
                "git_common_dir": "/tmp/repo/.git",
                "worktree_id": format!("sha256:{}", "1".repeat(64)),
                "branch_ref": "refs/heads/original",
                "head_sha": "1111111111111111111111111111111111111111",
                "workspace_fingerprint": format!("sha256:{}", "2".repeat(64))
            }))?);
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
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(
                    "task_workspace_corrupt",
                )))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
        drop(store);

        let conn = open_project_state_database_for_test(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
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
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(
                    "task_replay_stale_first",
                )))
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
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(
                    "task_operation_result",
                )))
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
        assert_eq!(stored.source_method, MethodName::UpdateScope.as_str());
        assert_eq!(stored.source_idempotency_key, idempotency_key.as_str());
        assert_eq!(stored.committed_state_version, 1);
        assert_eq!(stored.actor_source, ACTOR_SOURCE);
        assert_eq!(stored.operation_category, "agent_workflow");
        assert_eq!(stored.response_json, response_json);
        assert_eq!(stored.response_size_bytes, response_json.len() as u64);
        assert_eq!(
            stored.response_sha256,
            format!("sha256:{:x}", Sha256::digest(response_json.as_bytes()))
        );
        Ok(())
    }

    #[test]
    fn invalid_replay_identity_is_rejected_before_transaction_and_effects(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;

        let mut invalid_actor = replay_context(CONNECTION_ID, "agent_workflow");
        invalid_actor.actor_source = "agent_connection:".to_owned();
        let mut invalid_category = replay_context(CONNECTION_ID, "agent_workflow");
        invalid_category.operation_category = "agent-workflow".to_owned();
        let mut blank_basis = replay_context(CONNECTION_ID, "agent_workflow");
        blank_basis.verification_basis = Some(" \t ".to_owned());
        let mut invalid_git_context = replay_context(CONNECTION_ID, "agent_workflow");
        invalid_git_context.git_workspace_context_json = Some("{}".to_owned());

        for (case, context, expected_field) in [
            ("actor", invalid_actor, "actor_source"),
            ("category", invalid_category, "operation_category"),
            ("basis", blank_basis, "verification_basis"),
            (
                "git_context",
                invalid_git_context,
                "tool_invocations.git_workspace_context_json",
            ),
        ] {
            let idempotency_key =
                IdempotencyKey::new(format!("idem_invalid_replay_identity_{case}"));
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

    #[test]
    fn write_ticket_consumption_revalidates_policy_authority_inside_transaction(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_ticket_policy_transaction";
        let write_ticket_id = "ticket_policy_transaction";
        let run_id = "run_policy_transaction";
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_ticket_policy_transaction_setup")),
                &RequestHash::new("sha256:ticket-policy-transaction-setup"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "ticket_policy_transaction_setup",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )?;

        let change_unit_id = "change_unit_ticket_policy_transaction";
        store.conn.execute(
            "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'active', 1, 1,
                       '2026-07-17T00:00:00Z', '2026-07-17T00:00:00Z')",
            params![PROJECT_ID, change_unit_id, task_id],
        )?;
        store.conn.execute(
            "UPDATE tasks
                SET current_change_unit_id = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
            params![PROJECT_ID, task_id, change_unit_id],
        )?;

        let issued_fingerprint =
            crate::workflow_records::project_write_authority_fingerprint(None)?;
        let validity_basis_json = volicord_types::canonical::canonical_json_string(&json!({
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "scope_revision": 0,
            "baseline_ref": null,
            "workspace_context_sha256": null,
            "write_authority_fingerprint": issued_fingerprint,
            "approval_basis_refs": []
        }))?;
        store.conn.execute(
            "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, validity_basis_json,
                allowed_path_prefixes_json, denied_path_prefixes_json,
                attempt_scope_json, created_by_actor_source, created_at,
                metadata_json
             ) VALUES (?1, ?2, ?3, ?4, 1, 'active', ?5,
                       '[\"src/export.rs\"]', '[]', '{}', ?6,
                       '2026-07-17T00:00:00Z', '{}')",
            params![
                PROJECT_ID,
                write_ticket_id,
                task_id,
                change_unit_id,
                validity_basis_json,
                ACTOR_SOURCE
            ],
        )?;
        let tightened_policy = json!({
            "schema": volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID,
            "workflow": {
                "default_direct_control": "tracked",
                "default_work_control": "tracked",
                "light": {
                    "enabled": false,
                    "max_intended_paths": 3,
                    "allowed_path_patterns": [],
                    "denied_path_patterns": ["src/**"],
                    "final_acceptance": "policy_dependent"
                },
                "write_ticket": {
                    "idle_timeout_minutes": null
                }
            }
        });
        let policy_json = volicord_types::canonical::canonical_json_string(&tightened_policy)?;
        let policy_fingerprint =
            volicord_types::canonical::canonical_json_sha256(&tightened_policy)?.into_inner();
        let current_fingerprint =
            crate::workflow_records::project_write_authority_fingerprint(Some(&policy_json))?;
        assert_ne!(issued_fingerprint, current_fingerprint);
        store.conn.execute(
            "INSERT INTO project_workflow_policies (
                project_id, policy_schema, policy_version, policy_json,
                policy_fingerprint, source, applied_at, created_at
             ) VALUES (?1, ?2, 1, ?3, ?4, 'store_test',
                       '2026-07-17T00:00:00Z', '2026-07-17T00:00:00Z')",
            params![
                PROJECT_ID,
                volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID,
                policy_json,
                policy_fingerprint
            ],
        )?;
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;

        let error = store
            .commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RecordRun,
                    Some(&IdempotencyKey::new(
                        "idem_ticket_policy_transaction_consume",
                    )),
                    &RequestHash::new("sha256:ticket-policy-transaction-consume"),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(1),
                    vec![pending_event_for_task(
                        "ticket_policy_transaction_consume",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Run(RunMutation::Insert(RunInsert {
                        run_id: run_id.to_owned(),
                        task_id: task_id.to_owned(),
                        change_unit_id: None,
                        scope_revision: 0,
                        write_ticket_id: Some(write_ticket_id.to_owned()),
                        kind: "implementation".to_owned(),
                        status: "recorded".to_owned(),
                        summary_json: "{}".to_owned(),
                        observed_changes_json: "{}".to_owned(),
                        evidence_updates_json: "[]".to_owned(),
                        write_ticket_effect_json: "{}".to_owned(),
                        created_by_actor_source: ACTOR_SOURCE.to_owned(),
                        metadata_json: "{}".to_owned(),
                    }))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                    CoreStorageMutation::WriteTicket(WriteTicketMutation::Consume(
                        WriteTicketConsumption {
                            write_ticket_id: write_ticket_id.to_owned(),
                            run_id: run_id.to_owned(),
                            expected_basis_state_version: 1,
                            expected_write_authority_fingerprint: issued_fingerprint.clone(),
                        },
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )
            .expect_err("changed policy authority must reject ticket consumption");

        assert!(matches!(
            error,
            StoreError::Conflict {
                entity: "write_ticket",
                ..
            }
        ));
        let (status, consumed_by_run_id): (String, Option<String>) = store.conn.query_row(
            "SELECT status, consumed_by_run_id
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
            params![PROJECT_ID, write_ticket_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(status, "active");
        assert_eq!(consumed_by_run_id, None);
        let run_count: i64 = store.conn.query_row(
            "SELECT COUNT(*)
               FROM runs
              WHERE project_id = ?1
                AND run_id = ?2",
            params![PROJECT_ID, run_id],
            |row| row.get(0),
        )?;
        assert_eq!(run_count, 0);
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(store.effect_counts()?, before_effects);
        Ok(())
    }

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

    #[test]
    fn user_action_request_and_basis_store_apis_round_trip() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_basis_round_trip";
        let request_id = "action_basis_round_trip";
        let now = UtcTimestamp::parse("2026-01-01T00:10:00Z")?;

        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_basis_initial")),
            &RequestHash::new("sha256:basis-initial"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("basis_initial", task_id)],
        );
        let first = store.commit_with(
            first_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(request_id, task_id, None),
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

        let current = store
            .user_action_record(request_id, &now)?
            .expect("user-action request should be readable");
        assert_eq!(current.status, UserActionStatus::Pending);
        assert_eq!(current.request.user_action_request_id, request_id);
        assert_eq!(current.request.task_id, task_id);
        assert_eq!(current.request.action_kind, UserActionKind::ProductDecision);
        assert_eq!(current.request.basis_status, UserActionBasisStatus::Current);
        assert_eq!(current.request.required_for_json, r#"["informational"]"#);
        assert_eq!(current.request.requested_by_actor_source, ACTOR_SOURCE);
        assert!(current.resolution.is_none());
        let basis: UserActionBasis = serde_json::from_str(&current.request.basis_json)?;
        assert_eq!(basis.compatibility_status(), UserActionBasisStatus::Current);
        assert_eq!(basis.coordinates().task_id.as_str(), task_id);

        let stale_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_basis_stale")),
            &RequestHash::new("sha256:basis-stale"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("basis_stale", task_id)],
        );
        let stale = store.commit_with(
            stale_input,
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::MarkBasesStatus(
                    UserActionBasisStatusMark {
                        user_action_request_ids: vec![request_id.to_owned()],
                        basis_status: UserActionBasisStatus::Stale,
                    },
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        assert!(matches!(stale, MutationCommitOutcome::Committed { .. }));
        let stale = store
            .user_action_record(request_id, &now)?
            .expect("stale request should remain readable");
        assert_eq!(stale.status, UserActionStatus::Stale);
        assert_eq!(stale.request.basis_status, UserActionBasisStatus::Stale);
        let stale_basis: UserActionBasis = serde_json::from_str(&stale.request.basis_json)?;
        assert_eq!(
            stale_basis.compatibility_status(),
            UserActionBasisStatus::Stale
        );

        let superseded_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_basis_superseded")),
            &RequestHash::new("sha256:basis-superseded"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(2),
            vec![pending_event_for_task("basis_superseded", task_id)],
        );
        let superseded = store.commit_with(
            superseded_input,
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::MarkBasesStatus(
                    UserActionBasisStatusMark {
                        user_action_request_ids: vec![request_id.to_owned()],
                        basis_status: UserActionBasisStatus::Superseded,
                    },
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        assert!(matches!(
            superseded,
            MutationCommitOutcome::Committed { .. }
        ));
        assert_eq!(
            store
                .user_action_record(request_id, &now)?
                .expect("superseded request should remain readable")
                .status,
            UserActionStatus::Superseded
        );
        Ok(())
    }

    #[test]
    fn user_action_request_store_rejects_empty_duplicate_and_mismatched_owner_facts(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_invalid_user_action_owner_facts";

        for (marker, mut action) in [
            (
                "empty_required_for",
                user_action_request_insert("action_empty_required_for", task_id, None),
            ),
            (
                "duplicate_required_for",
                user_action_request_insert("action_duplicate_required_for", task_id, None),
            ),
            (
                "mismatched_sensitive_scope",
                user_action_request_insert("action_mismatched_sensitive_scope", task_id, None),
            ),
            (
                "incompatible_required_for",
                user_action_request_insert("action_incompatible_required_for", task_id, None),
            ),
        ] {
            match marker {
                "empty_required_for" => {
                    let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                    request["required_for"] = json!([]);
                    action.request_json = request.to_string();
                    action.required_for_json = "[]".to_owned();
                }
                "duplicate_required_for" => {
                    let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                    request["required_for"] = json!(["informational", "informational"]);
                    action.request_json = request.to_string();
                    action.required_for_json = r#"["informational","informational"]"#.to_owned();
                }
                "mismatched_sensitive_scope" => {
                    let mut basis = serde_json::from_str::<Value>(&action.basis_json)?;
                    basis["sensitive_action_scope"] = json!({
                        "action_kind": "write_files",
                        "description": "Bounded write.",
                        "intended_paths": ["src/lib.rs"],
                        "sensitive_categories": ["product_file_write"],
                        "command_or_tool_summary": null,
                        "network_or_host_summary": null,
                        "secret_or_credential_summary": null,
                        "capability_claim": "Local file write only.",
                        "expires_at": null
                    });
                    action.basis_json = basis.to_string();
                }
                "incompatible_required_for" => {
                    let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                    request["required_for"] = json!(["close_cancel"]);
                    action.request_json = request.to_string();
                    action.required_for_json = r#"["close_cancel"]"#.to_owned();
                }
                _ => unreachable!("test table contains only declared invalid cases"),
            }
            let error = store
                .commit_with(
                    commit_input(
                        &ProjectId::new(PROJECT_ID),
                        MethodName::RequestUserAction,
                        Some(&IdempotencyKey::new(format!("idem_store_{marker}"))),
                        &RequestHash::new(format!("sha256:{marker}")),
                        Some(replay_context(CONNECTION_ID, "agent_workflow")),
                        Some(0),
                        vec![pending_event_for_task(marker, task_id)],
                    ),
                    |mutation, facts| {
                        CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                            .apply(mutation, facts)
                            .map(|_| ())?;
                        CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                            .apply(mutation, facts)
                            .map(|_| ())
                    },
                    response_json,
                )
                .expect_err("invalid user-action owner facts must fail closed");
            assert!(matches!(&error, StoreError::InvalidInput { .. }));
            if marker == "incompatible_required_for" {
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail }
                        if detail == "user_action_requests.request_json required_for contains an operation incompatible with its action kind"
                ));
            }
            assert_eq!(store.effect_counts()?.tasks, 0);
        }
        Ok(())
    }

    #[test]
    fn user_action_request_timestamp_order_is_strict_at_insert_boundaries(
    ) -> Result<(), Box<dyn Error>> {
        for (suffix, expires_at, should_commit) in [
            ("before", "2025-12-31T23:59:59.999Z", false),
            ("equal", "2026-01-01T00:00:00Z", false),
            ("after", "2026-01-01T00:00:00.001Z", true),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_request_timestamp_{suffix}");
            let request_id = format!("action_request_timestamp_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, expires_at);
            let outcome = store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_request_timestamp_{suffix}"
                    ))),
                    &RequestHash::new(format!("sha256:request-timestamp-{suffix}")),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(&task_id)))
                        .apply(mutation, facts)
                        .map(|_| ())?;
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                        .apply(mutation, facts)
                        .map(|_| ())
                },
                response_json,
            );

            if should_commit {
                assert!(matches!(outcome?, MutationCommitOutcome::Committed { .. }));
                assert_eq!(
                    store
                        .user_action_record(
                            &request_id,
                            &UtcTimestamp::parse("2026-01-01T00:00:00Z")?,
                        )?
                        .expect("strictly later expiry should remain readable")
                        .status,
                    UserActionStatus::Pending
                );
            } else {
                let error = outcome.expect_err("non-later expiry must reject atomically");
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail }
                        if detail == "user_action_requests.expires_at must be later than user_action_requests.requested_at"
                ));
                assert_eq!(store.effect_counts()?.tasks, 0);
            }
        }
        Ok(())
    }

    #[test]
    fn evidence_observation_request_insert_rejects_extended_ttl_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_evidence_action_extended_ttl";
        let request_id = "action_evidence_action_extended_ttl";
        let mut action = evidence_user_action_request_insert(request_id, task_id, 1);
        set_user_action_request_expiry(&mut action, "2026-01-01T00:16:00Z");
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;

        let error = store
            .commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new("idem_evidence_action_extended_ttl")),
                    &RequestHash::new("sha256:evidence-action-extended-ttl"),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        "evidence_action_extended_ttl",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                        .apply(mutation, facts)
                        .map(|_| ())?;
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                        .apply(mutation, facts)
                        .map(|_| ())
                },
                response_json,
            )
            .expect_err("a 16-minute evidence-observation request TTL must reject atomically");

        assert!(matches!(
            error,
            StoreError::InvalidInput { detail }
                if detail == "evidence-observation user_action_requests.expires_at must be exactly 15 minutes after user_action_requests.requested_at"
        ));
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(store.effect_counts()?, before_effects);
        Ok(())
    }

    #[test]
    fn evidence_capture_intent_insert_rejects_extended_ttl_without_effect(
    ) -> Result<(), Box<dyn Error>> {
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

    #[test]
    fn ordered_multi_aggregate_commit_is_versioned_replayable_and_durable(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_evidence_observation";
        let run_id = "run_evidence_observation";
        let observation_id = "evidence_observation_store";
        let idempotency_key = IdempotencyKey::new("idem_store_evidence_observation");

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&idempotency_key),
            &RequestHash::new("sha256:evidence-observation"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("evidence_observation", task_id)],
        );
        let mutations = [
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
            CoreStorageMutation::Run(RunMutation::Insert(run_insert(run_id, task_id))),
            CoreStorageMutation::Evidence(EvidenceMutation::EnsureClaim(EvidenceClaimInsert {
                task_id: task_id.to_owned(),
                evidence_claim_id: "claim_search_result_count".to_owned(),
                statement: "Search result count was verified.".to_owned(),
            })),
            CoreStorageMutation::Evidence(EvidenceMutation::InsertObservation(
                EvidenceObservationInsert {
                    evidence_observation_id: observation_id.to_owned(),
                    task_id: task_id.to_owned(),
                    change_unit_id: None,
                    run_id: Some(run_id.to_owned()),
                    acceptance_criterion_id: None,
                    evidence_claim_id: Some("claim_search_result_count".to_owned()),
                    source_kind: "external_tool".to_owned(),
                    assurance_level: "external_tool_result".to_owned(),
                    observed_by_actor_source: Some(ACTOR_SOURCE.to_owned()),
                    tool_name: Some("local-test-runner".to_owned()),
                    tool_invocation_id: Some("tool_invocation_001".to_owned()),
                    tool_metadata_json: json!({"exit_code": 0}).to_string(),
                    input_refs_json: "[]".to_owned(),
                    source_refs_json: json!([{
                        "source_kind": "user_context",
                        "source": {"context_id": "message_store_evidence"}
                    }])
                    .to_string(),
                    output_artifact_refs_json: "[]".to_owned(),
                    limitations_json: json!(["External tool result is not a proof."]).to_string(),
                    observed_at: "2026-06-18T00:00:00Z".to_owned(),
                    recorded_at: "2026-06-18T00:00:01Z".to_owned(),
                    metadata_json: json!({
                        "recorded_by_run_id": run_id,
                        "invocation_verification_basis": "store_test_boundary",
                        "producer_anchor": {
                            "producer_kind": "unverified_caller",
                            "producer_ref": null,
                            "output_artifact_refs": [],
                            "verification_basis": null
                        },
                        "relevance_assessment": {
                            "status": "unassessed",
                            "assessment_ref": null,
                            "assessed_by_actor_source": null
                        }
                    })
                    .to_string(),
                },
            )),
        ];
        let committed = store.commit_mutation(input.clone(), &mutations, response_json)?;
        let MutationCommitOutcome::Committed {
            response_json: committed_response,
            basis_state_version,
            committed_state_version,
            events,
        } = committed
        else {
            panic!("ordered aggregate batch must commit");
        };
        assert_eq!(basis_state_version, 0);
        assert_eq!(committed_state_version, 1);
        assert_eq!(events.len(), 1);

        let record = store
            .evidence_observation_record(observation_id)?
            .expect("evidence observation should be readable");
        assert_eq!(record.run_id.as_deref(), Some(run_id));
        assert_eq!(record.source_kind, "external_tool");
        assert_eq!(record.assurance_level, "external_tool_result");
        assert_eq!(
            serde_json::from_str::<Value>(&record.source_refs_json)?,
            json!([{
                "source_kind": "user_context",
                "source": {"context_id": "message_store_evidence"}
            }])
        );
        assert_eq!(
            serde_json::from_str::<Vec<String>>(&record.limitations_json)?,
            vec!["External tool result is not a proof."]
        );
        let committed_counts = store.effect_counts()?;
        assert_eq!(committed_counts.state_version, 1);
        assert_eq!(committed_counts.tasks, 1);
        assert_eq!(committed_counts.runs, 1);
        assert_eq!(committed_counts.evidence_claims, 1);
        assert_eq!(committed_counts.evidence_observations, 1);
        assert_eq!(committed_counts.authority_events, 1);
        assert_eq!(committed_counts.tool_invocations, 1);

        let replay = store.commit_mutation(input, &mutations, |_| {
            panic!("eligible replay must reuse the stored response")
        })?;
        assert!(matches!(
            replay,
            MutationCommitOutcome::Replayed {
                response_json,
                basis_state_version: 0,
                committed_state_version: 1,
            } if response_json == committed_response
        ));
        assert_eq!(store.effect_counts()?, committed_counts);

        drop(store);
        let reopened = harness.store()?;
        assert_eq!(reopened.project_state()?.state_version, 1);
        assert!(reopened.task_record(&TaskId::new(task_id))?.is_some());
        assert!(reopened.run_record(run_id)?.is_some());
        assert!(reopened
            .evidence_claim_record(&TaskId::new(task_id), "claim_search_result_count")?
            .is_some());
        assert!(reopened
            .evidence_observation_record(observation_id)?
            .is_some());
        assert_eq!(
            reopened
                .tool_invocation(MethodName::RecordRun, &idempotency_key)?
                .expect("replay row must survive reopen")
                .response_json,
            committed_response
        );
        assert_eq!(reopened.effect_counts()?, committed_counts);
        Ok(())
    }

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
                CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(
                    change_unit_insert("cu_effect_contract", task_id, contract.to_string()),
                ))
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
    fn malformed_effect_contract_json_rejects_commit_without_effect() -> Result<(), Box<dyn Error>>
    {
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

    #[test]
    fn user_action_store_derives_expiry_resolution_and_stale_status() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_user_action_status";

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_action_expiring")),
                &RequestHash::new("sha256:action-expiring"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("action_expiring", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(
                        "action_expiring",
                        task_id,
                        Some("2026-01-01T00:15:00Z"),
                    ),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let before_expiry = UtcTimestamp::parse("2026-01-01T00:14:59Z")?;
        let at_expiry = UtcTimestamp::parse("2026-01-01T00:15:00Z")?;
        assert_eq!(
            store
                .user_action_record("action_expiring", &before_expiry)?
                .expect("expiring action should be readable")
                .status,
            UserActionStatus::Pending
        );
        assert_eq!(
            store
                .user_action_record("action_expiring", &at_expiry)?
                .expect("expired action should remain readable")
                .status,
            UserActionStatus::Expired
        );

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_action_current")),
                &RequestHash::new("sha256:action-current"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(1),
                vec![pending_event_for_task("action_current", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert("action_current", task_id, None),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_action_resolve")),
                &RequestHash::new("sha256:action-resolve"),
                Some(VerifiedReplayContext {
                    actor_source: "local_user".to_owned(),
                    operation_category: "user_only".to_owned(),
                    verification_basis: Some("store_test_user_channel".to_owned()),
                    git_workspace_context_json: None,
                }),
                Some(2),
                vec![pending_event_for_task("action_resolve", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    user_action_resolution_insert("resolution_current", "action_current"),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        assert_eq!(
            store
                .user_action_record("action_current", &at_expiry)?
                .expect("resolved action should be readable")
                .status,
            UserActionStatus::Resolved
        );

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::UpdateScope,
                Some(&IdempotencyKey::new("idem_store_action_stale")),
                &RequestHash::new("sha256:action-stale"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(3),
                vec![pending_event_for_task("action_stale", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::MarkBasesStatus(
                    UserActionBasisStatusMark {
                        user_action_request_ids: vec!["action_current".to_owned()],
                        basis_status: UserActionBasisStatus::Stale,
                    },
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        let stale = store
            .user_action_record("action_current", &at_expiry)?
            .expect("stale action should be readable");
        assert_eq!(stale.status, UserActionStatus::Stale);
        assert_eq!(
            serde_json::from_str::<Value>(&stale.request.basis_json)?["coordinates"]
                ["compatibility_status"],
            "stale"
        );
        Ok(())
    }

    #[test]
    fn user_action_resolution_round_trips_choice_and_channel_provenance(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_deferred_action";
        let request_id = "action_deferred_pair";
        let resolution_id = "resolution_deferred_pair";
        let mut deferred_request = user_action_request_insert(request_id, task_id, None);
        let mut deferred_request_json =
            serde_json::from_str::<Value>(&deferred_request.request_json)?;
        deferred_request_json["body"]["options"]
            .as_array_mut()
            .expect("choice options should be an array")
            .push(json!({
                "option_id": "defer",
                "label": "Defer",
                "description": "Defer this bounded decision.",
                "consequence": "The request remains resolved as deferred.",
                "machine_action": "defer",
                "resolution_outcome": "deferred",
                "is_default": false
            }));
        deferred_request.request_json = deferred_request_json.to_string();

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_defer_insert")),
            &RequestHash::new("sha256:defer-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("defer_insert", task_id)],
        );
        let inserted = store.commit_with(
            insert_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        deferred_request,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));

        let mut resolution = user_action_resolution_insert(resolution_id, request_id);
        resolution.channel_submission_id = "submission_deferred_pair".to_owned();
        resolution.resolution_json = choice_resolution_json(
            "defer",
            UserActionOptionAction::Defer,
            JudgmentResolutionOutcome::Deferred,
        );
        resolution.resolved_assurance_level = "verified_local_user_channel".to_owned();
        let resolve_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_defer_resolve")),
            &RequestHash::new("sha256:defer-resolve"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("defer_resolve", task_id)],
        );
        let resolved = store.commit_with(
            resolve_input,
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(resolution))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )?;
        assert!(matches!(resolved, MutationCommitOutcome::Committed { .. }));

        let record = store
            .user_action_resolution_record(resolution_id)?
            .expect("resolved user action should be readable");
        assert_eq!(record.user_action_request_id, request_id);
        assert_eq!(record.channel_kind, UserActionChannelKind::Cli);
        assert_eq!(record.channel_submission_id, "submission_deferred_pair");
        assert_eq!(record.resolved_by_actor_source, "local_user");
        assert_eq!(
            serde_json::from_str::<Value>(&record.resolution_json)?["machine_action"],
            "defer"
        );
        assert_eq!(
            store
                .user_action_resolution_for_channel_submission(
                    UserActionChannelKind::Cli,
                    "submission_deferred_pair",
                )?
                .expect("channel submission lookup should return the immutable resolution"),
            record
        );
        assert_eq!(
            store
                .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:11:00Z")?,)?
                .expect("resolved request should remain readable")
                .status,
            UserActionStatus::Resolved
        );
        let before_tamper = store.effect_counts()?;
        store
            .conn
            .execute_batch("PRAGMA ignore_check_constraints = ON")?;
        store.conn.execute(
            "UPDATE user_action_resolutions
                SET channel_submission_id = ?3
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
            params![PROJECT_ID, resolution_id, "x".repeat(257)],
        )?;
        store
            .conn
            .execute_batch("PRAGMA ignore_check_constraints = OFF")?;
        assert!(matches!(
            store.user_action_resolution_record(resolution_id),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
        assert_eq!(store.effect_counts()?, before_tamper);
        Ok(())
    }

    #[test]
    fn user_action_resolution_timestamp_order_enforces_half_open_boundaries(
    ) -> Result<(), Box<dyn Error>> {
        for (suffix, resolved_at, expected_error) in [
            (
                "before_request",
                "2025-12-31T23:59:59.999Z",
                Some(
                    "user_action_resolutions.resolved_at must be at or after user_action_requests.requested_at",
                ),
            ),
            ("at_request", "2026-01-01T00:00:00Z", None),
            ("before_expiry", "2026-01-01T00:00:09.999Z", None),
            (
                "at_expiry",
                "2026-01-01T00:00:10Z",
                Some(
                    "user_action_resolutions.resolved_at must be before user_action_requests.expires_at",
                ),
            ),
            (
                "after_expiry",
                "2026-01-01T00:00:10.001Z",
                Some(
                    "user_action_resolutions.resolved_at must be before user_action_requests.expires_at",
                ),
            ),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_resolution_timestamp_{suffix}");
            let request_id = format!("action_resolution_timestamp_{suffix}");
            let resolution_id = format!("resolution_timestamp_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
            store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_request_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-request-{suffix}"
                    )),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(&task_id)))
                        .apply(mutation, facts).map(|_| ())?;
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                        .apply(mutation, facts).map(|_| ())
                },
                response_json,
            )?;

            let mut resolution = user_action_resolution_insert(&resolution_id, &request_id);
            resolution.resolved_at = resolved_at.to_owned();
            let outcome = store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_resolve_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-resolve-{suffix}"
                    )),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(
                        &format!("{suffix}_resolve"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(resolution))
                        .apply(mutation, facts).map(|_| ())
                },
                response_json,
            );

            if let Some(expected_error) = expected_error {
                let error = outcome.expect_err("out-of-window resolution must reject atomically");
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail } if detail == expected_error
                ));
                assert_eq!(store.effect_counts()?.user_action_resolutions, 0);
                assert_eq!(store.project_state()?.state_version, 1);
            } else {
                assert!(matches!(outcome?, MutationCommitOutcome::Committed { .. }));
                assert_eq!(
                    store
                        .user_action_resolution_record(&resolution_id)?
                        .expect("in-window resolution should remain readable")
                        .resolved_at,
                    resolved_at
                );
            }
        }
        Ok(())
    }

    #[test]
    fn evidence_observation_resolution_preserves_exact_candidate_after_projection_advances(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_observation_resolution_reread";
        let request_id = "action_observation_resolution_reread";
        let resolution_id = "resolution_observation_reread";

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_observation_request")),
                &RequestHash::new("sha256:observation-request"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("observation_request", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    evidence_user_action_request_insert(request_id, task_id, 3),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let before_mismatch = store.effect_counts()?;
        let mismatch = store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_observation_resolution")),
                &RequestHash::new("sha256:observation-resolution"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("observation_resolution", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    evidence_user_action_resolution_insert(resolution_id, request_id, task_id, 4),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        );
        assert!(matches!(mismatch, Err(StoreError::InvalidInput { .. })));
        assert_eq!(store.effect_counts()?, before_mismatch);
        assert!(store
            .user_action_resolution_record(resolution_id)?
            .is_none());

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_observation_resolution")),
                &RequestHash::new("sha256:observation-resolution"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("observation_resolution", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    evidence_user_action_resolution_insert(resolution_id, request_id, task_id, 3),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let resolved = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("resolved evidence-observation action should remain readable");
        assert_eq!(resolved.status, UserActionStatus::Resolved);
        let resolution = store
            .user_action_resolution_record(resolution_id)?
            .expect("the immutable resolution should be readable by id");
        assert_eq!(
            serde_json::from_str::<Value>(&resolution.resolution_json)?["observation"]
                ["output_artifact_refs"][0]["created_by_run_ref"]["produced_at_state_version"],
            3
        );

        let mut tampered: Value = serde_json::from_str(&resolution.resolution_json)?;
        tampered["observation"]["output_artifact_refs"][0]["sha256"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        store.conn.execute(
            "UPDATE user_action_resolutions
                SET resolution_json = ?2
              WHERE project_id = ?1
                AND user_action_resolution_id = ?3",
            params![PROJECT_ID, tampered.to_string(), resolution_id],
        )?;
        assert!(matches!(
            store.user_action_resolution_record(resolution_id),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
        Ok(())
    }

    #[test]
    fn user_action_resolution_is_one_to_one_and_channel_submission_is_unique(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_uniqueness";
        let first_request_id = "action_resolution_unique_first";
        let second_request_id = "action_resolution_unique_second";

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_resolution_unique_insert")),
                &RequestHash::new("sha256:resolution-unique-insert"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("resolution_unique_insert", task_id)],
            ),
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(first_request_id, task_id, None),
                    )),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(second_request_id, task_id, None),
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;

        let mut first_resolution =
            user_action_resolution_insert("resolution_unique_first", first_request_id);
        first_resolution.channel_submission_id = "submission_unique".to_owned();
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_resolution_unique_first")),
                &RequestHash::new("sha256:resolution-unique-first"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("resolution_unique_first", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    first_resolution,
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        let before_conflicts = store.effect_counts()?;

        let second_for_same_request = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_resolution_same_request")),
            &RequestHash::new("sha256:resolution-same-request"),
            Some(user_replay_context()),
            Some(2),
            vec![pending_event_for_task("resolution_same_request", task_id)],
        );
        let error = store
            .commit_with(
                second_for_same_request,
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                        user_action_resolution_insert(
                            "resolution_unique_duplicate_request",
                            first_request_id,
                        ),
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )
            .expect_err("one request must not accept a second immutable resolution");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(store.effect_counts()?, before_conflicts);

        let mut reused_submission =
            user_action_resolution_insert("resolution_unique_submission", second_request_id);
        reused_submission.channel_submission_id = "submission_unique".to_owned();
        let error = store
            .commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(
                        "idem_store_resolution_same_submission",
                    )),
                    &RequestHash::new("sha256:resolution-same-submission"),
                    Some(user_replay_context()),
                    Some(2),
                    vec![pending_event_for_task(
                        "resolution_same_submission",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                        reused_submission,
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )
            .expect_err("one channel submission must not resolve two requests");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(store.effect_counts()?, before_conflicts);
        assert_eq!(
            store
                .user_action_resolution_for_channel_submission(
                    UserActionChannelKind::Cli,
                    "submission_unique",
                )?
                .expect("the first resolution must remain canonical")
                .user_action_request_id,
            first_request_id
        );
        Ok(())
    }

    #[test]
    fn user_action_resolution_rejects_request_action_kind_mismatch() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_kind_mismatch";
        let request_id = "action_resolution_kind_mismatch";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_missing_action_insert")),
            &RequestHash::new("sha256:missing-action-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("missing_action_insert", task_id)],
        );
        let inserted = store.commit_with(
            insert_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(request_id, task_id, None),
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;

        let resolve_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_missing_action_resolve")),
            &RequestHash::new("sha256:missing-action-resolve"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("missing_action_resolve", task_id)],
        );
        let mut resolution = user_action_resolution_insert("resolution_kind_mismatch", request_id);
        resolution.action_kind = UserActionKind::TechnicalDecision;

        let error = store
            .commit_with(
                resolve_input,
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                        resolution,
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )
            .expect_err("resolution action kind must match its request");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
        let record = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("pending user action should remain readable");
        assert_eq!(record.status, UserActionStatus::Pending);
        assert!(record.resolution.is_none());
        Ok(())
    }

    #[test]
    fn user_action_resolution_read_fails_closed_on_tampered_choice_authority(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_tampered_choice_authority";
        let request_id = "action_tampered_choice_authority";
        let resolution_id = "resolution_tampered_choice_authority";
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_tampered_choice_insert")),
                &RequestHash::new("sha256:tampered-choice-insert"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("tampered_choice_insert", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_tampered_choice_resolve")),
                &RequestHash::new("sha256:tampered-choice-resolve"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("tampered_choice_resolve", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    user_action_resolution_insert(resolution_id, request_id),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        for tampered_resolution in [
            choice_resolution_json(
                "not_a_request_option",
                UserActionOptionAction::Accept,
                JudgmentResolutionOutcome::Accepted,
            ),
            choice_resolution_json(
                "accept",
                UserActionOptionAction::Reject,
                JudgmentResolutionOutcome::Rejected,
            ),
        ] {
            store.conn.execute(
                "UPDATE user_action_resolutions
                    SET resolution_json = ?2
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?3",
                params![PROJECT_ID, tampered_resolution, resolution_id],
            )?;
            assert!(matches!(
                store.user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?),
                Err(StoreError::CorruptOwnerStateValue { .. })
            ));
            assert!(matches!(
                store.user_action_resolution_record(resolution_id),
                Err(StoreError::CorruptOwnerStateValue { .. })
            ));
        }
        Ok(())
    }

    #[test]
    fn user_action_resolution_requires_local_user_and_verified_provenance(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_provenance";
        let request_id = "action_resolution_provenance";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_blocked_resolution_insert")),
            &RequestHash::new("sha256:blocked-resolution-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("blocked_resolution_insert", task_id)],
        );
        let inserted = store.commit_with(
            insert_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(request_id, task_id, None),
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;

        let mut invalid_resolutions = Vec::new();
        let mut wrong_actor = user_action_resolution_insert("resolution_wrong_actor", request_id);
        wrong_actor.resolved_by_actor_source = ACTOR_SOURCE.to_owned();
        invalid_resolutions.push(("wrong_actor", wrong_actor));
        let mut missing_basis =
            user_action_resolution_insert("resolution_missing_basis", request_id);
        missing_basis.resolved_verification_basis.clear();
        invalid_resolutions.push(("missing_basis", missing_basis));
        let mut missing_assurance =
            user_action_resolution_insert("resolution_missing_assurance", request_id);
        missing_assurance.resolved_assurance_level.clear();
        invalid_resolutions.push(("missing_assurance", missing_assurance));
        let mut mismatched_channel_basis =
            user_action_resolution_insert("resolution_mismatched_channel_basis", request_id);
        mismatched_channel_basis.resolved_verification_basis =
            "unsupported_user_action_channel".to_owned();
        invalid_resolutions.push(("mismatched_channel_basis", mismatched_channel_basis));

        for (marker, resolution) in invalid_resolutions {
            let error = store
                .commit_with(
                    commit_input(
                        &ProjectId::new(PROJECT_ID),
                        MethodName::ResolveUserAction,
                        Some(&IdempotencyKey::new(format!(
                            "idem_store_resolution_{marker}"
                        ))),
                        &RequestHash::new(format!("sha256:resolution-{marker}")),
                        Some(user_replay_context()),
                        Some(1),
                        vec![pending_event_for_task(marker, task_id)],
                    ),
                    |mutation, facts| {
                        CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                            resolution,
                        ))
                        .apply(mutation, facts)
                        .map(|_| ())
                    },
                    response_json,
                )
                .expect_err("invalid user actor or provenance must reject");
            assert!(matches!(error, StoreError::InvalidInput { .. }));
            assert_eq!(store.effect_counts()?, before);
        }
        let record = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("pending request should remain readable");
        assert_eq!(record.status, UserActionStatus::Pending);
        assert!(record.resolution.is_none());
        Ok(())
    }

    #[test]
    fn user_action_resolution_rejects_unknown_fields_and_invalid_outcomes(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_invalid_resolution_json";
        let request_id = "action_invalid_resolution_json";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_unknown_rationale_insert")),
            &RequestHash::new("sha256:unknown-rationale-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("unknown_rationale_insert", task_id)],
        );
        let inserted = store.commit_with(
            insert_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(request_id, task_id, None),
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;

        let mut unknown_field =
            user_action_resolution_insert("resolution_unknown_field", request_id);
        let mut unknown_value: Value = serde_json::from_str(&unknown_field.resolution_json)?;
        unknown_value["unknown_resolution_field"] = json!(true);
        unknown_field.resolution_json = unknown_value.to_string();
        let mut invalid_outcome =
            user_action_resolution_insert("resolution_invalid_outcome", request_id);
        let mut invalid_outcome_value: Value =
            serde_json::from_str(&invalid_outcome.resolution_json)?;
        invalid_outcome_value["resolution_outcome"] = json!("blocked");
        invalid_outcome.resolution_json = invalid_outcome_value.to_string();

        for (marker, resolution) in [
            ("unknown_field", unknown_field),
            ("invalid_outcome", invalid_outcome),
        ] {
            let error = store
                .commit_with(
                    commit_input(
                        &ProjectId::new(PROJECT_ID),
                        MethodName::ResolveUserAction,
                        Some(&IdempotencyKey::new(format!(
                            "idem_store_resolution_{marker}"
                        ))),
                        &RequestHash::new(format!("sha256:resolution-{marker}")),
                        Some(user_replay_context()),
                        Some(1),
                        vec![pending_event_for_task(marker, task_id)],
                    ),
                    |mutation, facts| {
                        CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                            resolution,
                        ))
                        .apply(mutation, facts)
                        .map(|_| ())
                    },
                    response_json,
                )
                .expect_err("unsupported closed resolution shapes must reject");
            assert!(matches!(error, StoreError::InvalidInput { .. }));
            assert_eq!(store.effect_counts()?, before);
        }
        let record = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("pending request should remain readable");
        assert_eq!(record.status, UserActionStatus::Pending);
        assert!(record.resolution.is_none());
        Ok(())
    }

    #[test]
    fn malformed_stored_user_action_basis_json_is_store_data_error() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_malformed_basis";
        let request_id = "action_malformed_basis";

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_basis_malformed")),
            &RequestHash::new("sha256:basis-malformed"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("basis_malformed", task_id)],
        );
        store.commit_with(
            input,
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let conn = open_project_state_database_for_test(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
        conn.execute(
            "UPDATE user_action_requests
                SET basis_json = 'not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id],
        )?;
        drop(conn);

        let store = harness.store()?;
        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
            .expect_err("malformed persisted basis JSON should be corruption");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "basis_json",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn stored_user_action_request_errors_preserve_request_and_required_for_columns(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_request_owner_columns";
        let malformed_request_id = "action_malformed_request_column";
        let mismatched_required_for_id = "action_mismatched_required_for_column";

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_request_owner_columns")),
                &RequestHash::new("sha256:request-owner-columns"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("request_owner_columns", task_id)],
            ),
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(malformed_request_id, task_id, None),
                    )),
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                        user_action_request_insert(mismatched_required_for_id, task_id, None),
                    )),
                ] {
                    storage_mutation.apply(mutation, facts).map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;
        store.conn.execute(
            "UPDATE user_action_requests
                SET request_json = 'not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, malformed_request_id],
        )?;
        store.conn.execute(
            "UPDATE user_action_requests
                SET required_for_json = '[\"close_complete\"]'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, mismatched_required_for_id],
        )?;

        for (request_id, expected_column) in [
            (malformed_request_id, "request_json"),
            (mismatched_required_for_id, "required_for_json"),
        ] {
            let error = store
                .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
                .expect_err("invalid owner JSON should fail closed on its canonical column");
            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateValue {
                    table: "user_action_requests",
                    logical_column,
                    ..
                } if logical_column == expected_column
            ));
        }
        Ok(())
    }

    #[test]
    fn stored_user_action_request_fails_closed_on_incompatible_required_for(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_incompatible_required_for_reread";
        let request_id = "action_incompatible_required_for_reread";

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new(
                    "idem_store_incompatible_required_for_reread",
                )),
                &RequestHash::new("sha256:incompatible-required-for-reread"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "incompatible_required_for_reread",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let stored_request_json: String = store.conn.query_row(
            "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id],
            |row| row.get(0),
        )?;
        let mut request_json = serde_json::from_str::<Value>(&stored_request_json)?;
        request_json["required_for"] = json!(["close_cancel"]);
        store.conn.execute(
            "UPDATE user_action_requests
                SET request_json = ?3,
                    required_for_json = '[\"close_cancel\"]'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id, request_json.to_string()],
        )?;

        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
            .expect_err("incompatible persisted required_for must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "request_json",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn stored_user_action_request_fails_closed_on_invalid_timestamp_order(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_request_timestamp_reread";
        let request_id = "action_request_timestamp_reread";
        let mut action = user_action_request_insert(request_id, task_id, None);
        set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_request_timestamp_reread")),
                &RequestHash::new("sha256:request-timestamp-reread"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("request_timestamp_reread", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )?;

        let stored_request_json: String = store.conn.query_row(
            "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id],
            |row| row.get(0),
        )?;
        let mut request_json = serde_json::from_str::<Value>(&stored_request_json)?;
        request_json["expires_at"] = json!("2026-01-01T00:00:00Z");
        store.conn.execute(
            "UPDATE user_action_requests
                SET request_json = ?3,
                    expires_at = '2026-01-01T00:00:00Z'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id, request_json.to_string()],
        )?;

        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:00:00Z")?)
            .expect_err("invalid stored request timestamp order must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "expires_at",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn stored_user_action_resolution_fails_closed_on_invalid_timestamp_order(
    ) -> Result<(), Box<dyn Error>> {
        for (suffix, corrupted_resolved_at) in [
            ("before_request", "2025-12-31T23:59:59.999Z"),
            ("at_expiry", "2026-01-01T00:00:10Z"),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_resolution_timestamp_reread_{suffix}");
            let request_id = format!("action_resolution_timestamp_reread_{suffix}");
            let resolution_id = format!("resolution_timestamp_reread_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
            store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_reread_request_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-reread-request-{suffix}"
                    )),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(&task_id)))
                        .apply(mutation, facts)
                        .map(|_| ())?;
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                        .apply(mutation, facts)
                        .map(|_| ())
                },
                response_json,
            )?;
            let mut resolution = user_action_resolution_insert(&resolution_id, &request_id);
            resolution.resolved_at = "2026-01-01T00:00:05Z".to_owned();
            store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_reread_resolve_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-reread-resolve-{suffix}"
                    )),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(
                        &format!("{suffix}_resolve"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                        resolution,
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )?;

            store.conn.execute(
                "UPDATE user_action_resolutions
                    SET resolved_at = ?3
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
                params![PROJECT_ID, resolution_id, corrupted_resolved_at],
            )?;
            let error = store
                .user_action_resolution_record(&resolution_id)
                .expect_err("invalid stored resolution timestamp order must fail closed");
            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateValue {
                    table: "user_action_resolutions",
                    logical_column: "resolved_at",
                    ..
                }
            ));
        }
        Ok(())
    }

    #[test]
    fn effective_user_action_rejects_resolution_from_future_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_future_reread";
        let request_id = "action_resolution_future_reread";
        let resolution_id = "resolution_future_reread";
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_resolution_future_request")),
                &RequestHash::new("sha256:resolution-future-request"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("resolution_future_request", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_resolution_future_resolve")),
                &RequestHash::new("sha256:resolution-future-resolve"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("resolution_future_resolve", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    user_action_resolution_insert(resolution_id, request_id),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;
        store.conn.execute(
            "UPDATE user_action_resolutions
                SET resolved_at = '2999-07-13T00:00:00Z'
              WHERE project_id = ?1 AND user_action_resolution_id = ?2",
            params![PROJECT_ID, resolution_id],
        )?;
        let before = (store.effect_counts()?, store.project_state()?);
        let now = UtcTimestamp::parse(&store.current_timestamp()?)?;

        let error = store
            .user_action_record(request_id, &now)
            .expect_err("a future stored resolution cannot be current authority");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_resolutions",
                logical_column: "resolved_at",
                ..
            }
        ));
        assert_eq!((store.effect_counts()?, store.project_state()?), before);
        Ok(())
    }

    #[test]
    fn effective_user_action_read_enforces_requested_at_lower_bound() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_requested_at_lower_bound";
        let request_id = "action_requested_at_lower_bound";

        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_requested_at_lower_bound")),
                &RequestHash::new("sha256:requested-at-lower-bound"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("requested_at_lower_bound", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let error = store
            .user_action_record(
                request_id,
                &UtcTimestamp::parse("2025-12-31T23:59:59.999Z")?,
            )
            .expect_err("time before requested_at must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "requested_at",
                ..
            }
        ));

        assert_eq!(
            store
                .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:00:00Z")?,)?
                .expect("requested_at boundary is inclusive")
                .status,
            UserActionStatus::Pending
        );
        Ok(())
    }

    #[test]
    fn project_continuity_record_mutation_persists_and_reads_active_rows(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_continuity_store";
        let change_unit_id = "cu_continuity_store";
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_continuity")),
            &RequestHash::new("sha256:store-continuity"),
            Some(user_replay_context()),
            Some(0),
            vec![pending_event_for_task("continuity", task_id)],
        );

        store.commit_with(
            input,
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(
                    change_unit_insert(change_unit_id, task_id, "null".to_owned()),
                ))
                .apply(mutation, facts)
                .map(|_| ())?;
                CoreStorageMutation::Continuity(ContinuityMutation::insert_record(
                    project_continuity_record_insert(
                        "continuity_store_001",
                        task_id,
                        change_unit_id,
                        "2026-01-01T00:00:00Z",
                    ),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )?;

        let active = store.active_project_continuity_page(10, None)?;
        assert_eq!(store.effect_counts()?.project_continuity_records, 1);
        assert_eq!(active.total_count, 1);
        assert!(!active.truncated);
        assert_eq!(active.records.len(), 1);
        assert_eq!(
            active.records[0].continuity_record_id,
            "continuity_store_001"
        );
        assert_eq!(active.records[0].kind, "decision");
        assert_eq!(active.records[0].status, "active");
        assert_eq!(active.records[0].source_task_id, task_id);
        assert_eq!(
            active.records[0].source_change_unit_id.as_deref(),
            Some(change_unit_id)
        );

        let task_records = store.project_continuity_records_for_task(task_id)?;
        assert_eq!(task_records.len(), 1);
        assert!(store.project_continuity_record_exists("continuity_store_001")?);
        Ok(())
    }

    #[test]
    fn project_continuity_pages_are_exclusive_totalled_and_tie_broken_by_id(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_continuity_page";
        let change_unit_id = "cu_continuity_page";
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_continuity_page")),
            &RequestHash::new("sha256:store-continuity-page"),
            Some(user_replay_context()),
            Some(0),
            vec![pending_event_for_task("continuity_page", task_id)],
        );

        store.commit_with(
            input,
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(
                    change_unit_insert(change_unit_id, task_id, "null".to_owned()),
                ))
                .apply(mutation, facts)
                .map(|_| ())?;
                for (record_id, updated_at) in [
                    ("continuity_a", "2026-01-02T00:00:00Z"),
                    ("continuity_c", "2026-01-02T00:00:00Z"),
                    ("continuity_b", "2026-01-02T00:00:00Z"),
                    ("continuity_d", "2026-01-01T23:59:59Z"),
                ] {
                    CoreStorageMutation::Continuity(ContinuityMutation::insert_record(
                        project_continuity_record_insert(
                            record_id,
                            task_id,
                            change_unit_id,
                            updated_at,
                        ),
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                }
                Ok(())
            },
            response_json,
        )?;

        let first = store.active_project_continuity_page(2, None)?;
        assert_eq!(first.total_count, 4);
        assert!(first.truncated);
        assert_eq!(
            first
                .records
                .iter()
                .map(|record| record.continuity_record_id.as_str())
                .collect::<Vec<_>>(),
            vec!["continuity_c", "continuity_b"]
        );
        let last = first.records.last().expect("first page cursor source");
        let cursor = ContinuityCursor {
            updated_at: UtcTimestamp::parse(&last.updated_at)?,
            continuity_record_id: ProjectContinuityRecordId::new(last.continuity_record_id.clone()),
        };
        let second = store.active_project_continuity_page(2, Some(&cursor))?;
        assert_eq!(second.total_count, 4);
        assert!(!second.truncated);
        assert_eq!(
            second
                .records
                .iter()
                .map(|record| record.continuity_record_id.as_str())
                .collect::<Vec<_>>(),
            vec!["continuity_a", "continuity_d"]
        );

        for invalid_page_size in [0, MAX_CONTINUITY_PAGE_SIZE + 1] {
            assert!(matches!(
                store.active_project_continuity_page(invalid_page_size, None),
                Err(StoreError::InvalidInput { .. })
            ));
        }
        let malformed_cursor = ContinuityCursor {
            updated_at: UtcTimestamp::parse("2026-01-02T00:00:00Z")?,
            continuity_record_id: ProjectContinuityRecordId::new("   "),
        };
        assert!(matches!(
            store.active_project_continuity_page(2, Some(&malformed_cursor)),
            Err(StoreError::InvalidInput { .. })
        ));
        Ok(())
    }

    #[test]
    fn intermediate_aggregate_failure_rolls_back_every_commit_effect() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let before = store.effect_counts()?;
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&IdempotencyKey::new("idem_store_foreign_key")),
            &RequestHash::new("sha256:foreign-key"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event("foreign_key")],
        );
        let mutations = [
            CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_before_failure"))),
            CoreStorageMutation::Run(RunMutation::Insert(run_insert_with_missing_task())),
        ];

        let error = store
            .commit_mutation(input, &mutations, response_json)
            .expect_err("missing run task should fail a foreign-key constraint");
        let classification = error.classification();

        assert_eq!(classification.category, "constraint_foreign_key");
        assert!(matches!(
            classification.route,
            crate::StoreFailureRoute::OperationalUnavailable
        ));
        assert_eq!(store.effect_counts()?, before);
        assert!(store
            .task_record(&TaskId::new("task_before_failure"))?
            .is_none());
        assert!(store
            .tool_invocation(
                MethodName::RecordRun,
                &IdempotencyKey::new("idem_store_foreign_key")
            )?
            .is_none());
        Ok(())
    }

    fn replay_context(connection_id: &str, operation_category: &str) -> VerifiedReplayContext {
        VerifiedReplayContext {
            actor_source: format!("agent_connection:{connection_id}"),
            operation_category: operation_category.to_owned(),
            verification_basis: Some("store_test_registration".to_owned()),
            git_workspace_context_json: None,
        }
    }

    fn user_replay_context() -> VerifiedReplayContext {
        VerifiedReplayContext {
            actor_source: "local_user".to_owned(),
            operation_category: "user_only".to_owned(),
            verification_basis: Some("store_test_user_channel".to_owned()),
            git_workspace_context_json: None,
        }
    }

    fn pending_event(marker: &str) -> PendingTaskEvent {
        pending_event_for_task(marker, &format!("task_{marker}"))
    }

    fn pending_event_for_task(marker: &str, task_id: &str) -> PendingTaskEvent {
        PendingTaskEvent {
            event_id: format!("evt_{marker}"),
            task_id: Some(task_id.to_owned()),
            change_unit_id: None,
            event_kind: "store_test_event".to_owned(),
            event_payload_json: "{}".to_owned(),
        }
    }

    fn task_insert(task_id: &str) -> TaskInsert {
        TaskInsert {
            task_id: task_id.to_owned(),
            created_by_actor_source: ACTOR_SOURCE.to_owned(),
            mode: "work".to_owned(),
            requested_control_level: "tracked".to_owned(),
            effective_control_level: "tracked".to_owned(),
            control_level_reason: "Store test control.".to_owned(),
            work_phase: "shaping".to_owned(),
            acceptance_policy: "required".to_owned(),
            acceptance_policy_reason: "Store test policy.".to_owned(),
            predecessor_task_id: None,
            lineage_relation: None,
            lineage_reason: None,
            carry_forward_json: "[]".to_owned(),
            lifecycle_phase: "shaping".to_owned(),
            result: None,
            title: None,
            summary: None,
            shaping_summary_json: "{}".to_owned(),
            bounded_context_json: "[]".to_owned(),
            autonomy_boundary_json: "{}".to_owned(),
            close_summary_json: "{\"close_reason\":\"none\"}".to_owned(),
            current_change_unit_id: None,
        }
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

    fn user_action_request_insert(
        request_id: &str,
        task_id: &str,
        expires_at: Option<&str>,
    ) -> UserActionRequestInsert {
        let request_json = json!({
            "body": {
                "action_type": "choice",
                "judgment_kind": "product_decision",
                "presentation": "short",
                "question": "Choose the current product direction.",
                "options": [{
                    "option_id": "accept",
                    "label": "Accept",
                    "description": "Accept the current direction.",
                    "consequence": "The work may continue.",
                    "machine_action": "accept",
                    "resolution_outcome": "accepted",
                    "is_default": true
                }],
                "context": {
                    "summary": "A bounded choice is required.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": []
                },
                "affected_refs": [],
                "sensitive_action_scope": null
            },
            "required_for": ["informational"],
            "expires_at": expires_at
        })
        .to_string();
        let basis_json = json!({
            "action_type": "choice",
            "coordinates": {
                "task_id": task_id,
                "change_unit_id": null,
                "scope_revision": 0,
                "baseline_ref": null,
                "created_at_state_version": 0,
                "compatibility_status": "current"
            },
            "close_basis_revision": null,
            "result_refs": [],
            "residual_risk_ids": [],
            "sensitive_action_scope": null
        })
        .to_string();
        UserActionRequestInsert {
            user_action_request_id: request_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            action_kind: UserActionKind::ProductDecision,
            request_json,
            basis_json,
            basis_status: UserActionBasisStatus::Current,
            required_for_json: r#"["informational"]"#.to_owned(),
            requested_by_actor_source: ACTOR_SOURCE.to_owned(),
            source_method: MethodName::RequestUserAction.as_str().to_owned(),
            source_idempotency_key: format!("idem_{request_id}"),
            requested_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: expires_at.map(str::to_owned),
            metadata_json: "{}".to_owned(),
        }
    }

    fn set_user_action_request_expiry(input: &mut UserActionRequestInsert, expires_at: &str) {
        let mut request_json = serde_json::from_str::<Value>(&input.request_json)
            .expect("test user-action request JSON should decode");
        request_json["expires_at"] = json!(expires_at);
        input.request_json = request_json.to_string();
        input.expires_at = Some(expires_at.to_owned());
    }

    fn evidence_user_action_request_insert(
        request_id: &str,
        task_id: &str,
        produced_at_state_version: u64,
    ) -> UserActionRequestInsert {
        let target = json!({
            "target_kind": "acceptance_criterion",
            "acceptance_criterion_id": "criterion_observation_reread"
        });
        let artifact = user_action_artifact_ref_json(task_id, produced_at_state_version);
        UserActionRequestInsert {
            user_action_request_id: request_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            action_kind: UserActionKind::EvidenceObservation,
            request_json: json!({
                "body": {
                    "action_type": "evidence_observation",
                    "question": "Does this artifact support the criterion?",
                    "context_summary": "Review the exact stored artifact bytes.",
                    "target_candidates": [target.clone()],
                    "artifact_candidates": [artifact.clone()]
                },
                "required_for": ["record_run"],
                "expires_at": "2026-01-01T00:15:00Z"
            })
            .to_string(),
            basis_json: json!({
                "action_type": "evidence_observation",
                "coordinates": {
                    "task_id": task_id,
                    "change_unit_id": null,
                    "scope_revision": 0,
                    "baseline_ref": null,
                    "created_at_state_version": 0,
                    "compatibility_status": "current"
                },
                "target_candidates": [target],
                "artifact_candidates": [artifact]
            })
            .to_string(),
            basis_status: UserActionBasisStatus::Current,
            required_for_json: r#"["record_run"]"#.to_owned(),
            requested_by_actor_source: ACTOR_SOURCE.to_owned(),
            source_method: MethodName::RequestUserAction.as_str().to_owned(),
            source_idempotency_key: format!("idem_{request_id}"),
            requested_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: Some("2026-01-01T00:15:00Z".to_owned()),
            metadata_json: "{}".to_owned(),
        }
    }

    fn evidence_user_action_resolution_insert(
        resolution_id: &str,
        request_id: &str,
        task_id: &str,
        produced_at_state_version: u64,
    ) -> UserActionResolutionInsert {
        UserActionResolutionInsert {
            user_action_resolution_id: resolution_id.to_owned(),
            user_action_request_id: request_id.to_owned(),
            action_kind: UserActionKind::EvidenceObservation,
            channel_kind: UserActionChannelKind::Cli,
            channel_submission_id: format!("submission_{resolution_id}"),
            resolution_json: json!({
                "resolution_type": "evidence_observation",
                "observation": {
                    "target": {
                        "target_kind": "acceptance_criterion",
                        "acceptance_criterion_id": "criterion_observation_reread"
                    },
                    "relevance_status": "supported",
                    "output_artifact_refs": [user_action_artifact_ref_json(
                        task_id,
                        produced_at_state_version
                    )],
                    "summary": "The exact artifact bytes support the criterion."
                }
            })
            .to_string(),
            resolved_by_actor_source: "local_user".to_owned(),
            resolved_verification_basis: "cli_direct_user_channel".to_owned(),
            resolved_assurance_level: "local_user_channel".to_owned(),
            resolved_at: "2026-01-01T00:10:00Z".to_owned(),
        }
    }

    fn user_action_artifact_ref_json(task_id: &str, produced_at_state_version: u64) -> Value {
        json!({
            "artifact_id": "artifact_observation_reread",
            "project_id": PROJECT_ID,
            "task_id": task_id,
            "display_name": "observation.json",
            "content_type": "application/json",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size_bytes": 64,
            "integrity_status": "verified",
            "redaction_state": "none",
            "availability": "available",
            "created_by_run_ref": {
                "record_kind": "run",
                "record_id": "run_observation_reread",
                "project_id": PROJECT_ID,
                "task_id": task_id,
                "produced_at_state_version": produced_at_state_version
            },
            "created_by_actor_source": ACTOR_SOURCE,
            "storage_ref": "artifact-storage://observation-reread"
        })
    }

    fn user_action_resolution_insert(
        resolution_id: &str,
        request_id: &str,
    ) -> UserActionResolutionInsert {
        UserActionResolutionInsert {
            user_action_resolution_id: resolution_id.to_owned(),
            user_action_request_id: request_id.to_owned(),
            action_kind: UserActionKind::ProductDecision,
            channel_kind: UserActionChannelKind::Cli,
            channel_submission_id: format!("submission_{resolution_id}"),
            resolution_json: json!({
                "resolution_type": "choice",
                "selected_option_id": "accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "note": null,
                "accepted_risk_ids": []
            })
            .to_string(),
            resolved_by_actor_source: "local_user".to_owned(),
            resolved_verification_basis: "cli_direct_user_channel".to_owned(),
            resolved_assurance_level: "local_user_channel".to_owned(),
            resolved_at: "2026-01-01T00:10:00Z".to_owned(),
        }
    }

    fn choice_resolution_json(
        selected_option_id: &str,
        machine_action: UserActionOptionAction,
        resolution_outcome: JudgmentResolutionOutcome,
    ) -> String {
        json!({
            "resolution_type": "choice",
            "selected_option_id": selected_option_id,
            "machine_action": machine_action,
            "resolution_outcome": resolution_outcome,
            "note": null,
            "accepted_risk_ids": []
        })
        .to_string()
    }

    fn project_continuity_record_insert(
        continuity_record_id: &str,
        task_id: &str,
        change_unit_id: &str,
        updated_at: &str,
    ) -> ProjectContinuityRecordInsert {
        ProjectContinuityRecordInsert {
            continuity_record_id: continuity_record_id.to_owned(),
            source_task_id: task_id.to_owned(),
            source_change_unit_id: Some(change_unit_id.to_owned()),
            kind: "decision".to_owned(),
            title: "Store continuity decision".to_owned(),
            summary: "A durable store-level continuity decision.".to_owned(),
            rationale: Some("The test records a traceable decision.".to_owned()),
            applies_to_paths_json: json!(["src/export.rs"]).to_string(),
            applies_to_refs_json: serde_json::to_string(&vec![state_ref(
                StateRecordKind::ChangeUnit,
                change_unit_id,
                task_id,
                1,
            )])
            .expect("state ref JSON should serialize"),
            source_refs_json: serde_json::to_string(&vec![state_ref(
                StateRecordKind::Task,
                task_id,
                task_id,
                1,
            )])
            .expect("state ref JSON should serialize"),
            artifact_refs_json: "[]".to_owned(),
            status: "active".to_owned(),
            supersedes_refs_json: "[]".to_owned(),
            review_triggers_json: json!(["Review if the source Task changes."]).to_string(),
            created_at: updated_at.to_owned(),
            updated_at: updated_at.to_owned(),
            metadata_json: json!({"source": "store_test"}).to_string(),
        }
    }

    fn state_ref(
        record_kind: StateRecordKind,
        record_id: &str,
        task_id: &str,
        state_version: u64,
    ) -> StateRecordRef {
        StateRecordRef {
            record_kind,
            record_id: RecordId::new(record_id),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: RequiredNullable::some(TaskId::new(task_id)),
            produced_at_state_version: RequiredNullable::some(state_version),
        }
    }

    fn run_insert_with_missing_task() -> RunInsert {
        RunInsert {
            run_id: "run_missing_task".to_owned(),
            task_id: "missing_task".to_owned(),
            change_unit_id: None,
            scope_revision: 0,
            write_ticket_id: None,
            kind: "implementation".to_owned(),
            status: "completed".to_owned(),
            summary_json: "{}".to_owned(),
            observed_changes_json: json!({
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": null
            })
            .to_string(),
            evidence_updates_json: "[]".to_owned(),
            write_ticket_effect_json: "{}".to_owned(),
            created_by_actor_source: ACTOR_SOURCE.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn run_insert(run_id: &str, task_id: &str) -> RunInsert {
        RunInsert {
            run_id: run_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            scope_revision: 0,
            write_ticket_id: None,
            kind: "implementation".to_owned(),
            status: "recorded".to_owned(),
            summary_json: "{}".to_owned(),
            observed_changes_json: json!({
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": null
            })
            .to_string(),
            evidence_updates_json: "[]".to_owned(),
            write_ticket_effect_json: "{}".to_owned(),
            created_by_actor_source: ACTOR_SOURCE.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn response_json(facts: CommittedMutationFacts) -> StoreResult<String> {
        Ok(json!({
            "base": {
                "state_version": facts.committed_state_version
            },
            "stored_response": "must_not_leak_on_mismatch"
        })
        .to_string())
    }
}
