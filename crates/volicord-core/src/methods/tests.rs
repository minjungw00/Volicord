use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};
use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
        ConnectionProjectRegistration, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX,
        HOST_SCOPE_PROJECT, VERIFIED_STATUS_COMPLETE, VERIFIED_STATUS_FAILED,
    },
    bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts, TaskRevisionRecord},
    guards::{
        insert_agent_session, insert_expected_write, insert_guard_event, insert_unrecorded_change,
        list_unresolved_unrecorded_changes, observe_guard_installation, unrecorded_change,
        upsert_guard_installation, AgentSessionInsert, ExpectedWriteInsert, GuardEventInsert,
        GuardInstallationObservation, GuardInstallationUpsert, UnrecordedChangeInsert,
        UnrecordedChangeRecord,
    },
    sqlite::open_project_state_database,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    prefixed_durable_id, ActorSource, ChangeUnitEffectContract, ChangeUnitEffectKind,
    ChangeUnitUpdate, DurableIdError, DurableIdGenerator, DurableIdKind, EvidenceAssuranceLevel,
    EvidenceSourceKind, EvidenceUpdateProvenance, IdempotencyKey, InitialScope, OperationCategory,
    RequestId, ScopeUpdate, SequenceDurableIdGenerator, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

use super::*;

const PROJECT_ID: &str = "project_methods";
const CONNECTION_ID: &str = "connection_methods";
const AGENT_ACTOR_SOURCE: &str = "agent_connection:connection_methods";
const LOCAL_USER_ACTOR_SOURCE: &str = "local_user";

#[derive(Debug, Clone)]
struct ManualClock {
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl ManualClock {
    fn at(timestamp: &str) -> Self {
        let now = DateTime::parse_from_rfc3339(timestamp)
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);
        Self {
            now: Arc::new(Mutex::new(now)),
        }
    }

    fn advance(&self, duration: Duration) {
        let mut now = self
            .now
            .lock()
            .expect("manual clock mutex should not be poisoned");
        *now += duration;
    }
}

impl crate::pipeline::Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
            .lock()
            .expect("manual clock mutex should not be poisoned")
            .to_owned()
    }
}

#[derive(Debug, Clone)]
struct CountingDurableIdGenerator {
    suffixes: Arc<Mutex<Vec<String>>>,
    generated: Arc<Mutex<Vec<DurableIdKind>>>,
}

impl CountingDurableIdGenerator {
    fn new(suffixes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut suffixes = suffixes
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        suffixes.reverse();
        Self {
            suffixes: Arc::new(Mutex::new(suffixes)),
            generated: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn count(&self, kind: DurableIdKind) -> usize {
        self.generated
            .lock()
            .expect("generated id log mutex should not be poisoned")
            .iter()
            .filter(|candidate| **candidate == kind)
            .count()
    }
}

impl DurableIdGenerator for CountingDurableIdGenerator {
    fn generate(&self, kind: DurableIdKind) -> Result<String, DurableIdError> {
        self.generated
            .lock()
            .expect("generated id log mutex should not be poisoned")
            .push(kind);
        let suffix = self
            .suffixes
            .lock()
            .expect("deterministic durable id generator mutex should not be poisoned")
            .pop()
            .ok_or(DurableIdError::DeterministicSequenceExhausted)?;
        Ok(prefixed_durable_id(kind, &suffix))
    }
}

struct MethodHarness {
    _runtime_home: TempRuntimeHome,
    runtime_home_path: PathBuf,
    service: CoreService,
}

#[derive(Debug, Clone)]
struct ContinuityRecordRow {
    source_task_id: String,
    source_change_unit_id: Option<String>,
    kind: String,
    title: String,
    summary: String,
    status: String,
    source_refs_json: String,
}

impl MethodHarness {
    fn new() -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("core-methods")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_methods", "{}")?;
        register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: PROJECT_ID.to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        ensure_agent_connection(
            runtime_home.path(),
            AgentConnectionRegistration {
                connection_internal_id: CONNECTION_ID.to_owned(),
                host_kind: HOST_KIND_CODEX.to_owned(),
                intent: volicord_store::agent_connections::CONNECTION_INTENT_SHARED.to_owned(),
                host_scope: HOST_SCOPE_PROJECT.to_owned(),
                server_name: "volicord-method-test".to_owned(),
                config_target: runtime_home
                    .path()
                    .join("agent-connections")
                    .join(CONNECTION_ID)
                    .to_string_lossy()
                    .into_owned(),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: "fixture:methods".to_owned(),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_connection_project(
            runtime_home.path(),
            ConnectionProjectRegistration {
                connection_internal_id: CONNECTION_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;

        let runtime_home_path = runtime_home.path().to_path_buf();
        let service = CoreService::new(&runtime_home_path);
        Ok(Self {
            _runtime_home: runtime_home,
            runtime_home_path,
            service,
        })
    }

    fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        let store = CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
        Ok(store.effect_counts()?)
    }

    fn conn(&self) -> Result<rusqlite::Connection, Box<dyn Error>> {
        Ok(open_project_state_database(
            self.runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?)
    }

    fn project_enforcement_profile_json(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.conn()?.query_row(
            "SELECT enforcement_profile_json
               FROM project_state
              WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?)
    }

    fn continuity_records(&self) -> Result<Vec<ContinuityRecordRow>, Box<dyn Error>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
                source_task_id,
                source_change_unit_id,
                kind,
                title,
                summary,
                status,
                source_refs_json
             FROM project_continuity_records
             WHERE project_id = ?1
             ORDER BY created_at, continuity_record_id",
        )?;
        let rows = stmt.query_map([PROJECT_ID], |row| {
            Ok(ContinuityRecordRow {
                source_task_id: row.get(0)?,
                source_change_unit_id: row.get(1)?,
                kind: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                status: row.get(5)?,
                source_refs_json: row.get(6)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    fn set_project_enforcement_profile_json(
        &self,
        profile_json: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.conn()?.execute(
            "UPDATE project_state
                SET enforcement_profile_json = ?2
              WHERE project_id = ?1",
            rusqlite::params![PROJECT_ID, profile_json],
        )?;
        Ok(())
    }

    fn use_generator_and_clock(
        &mut self,
        generator: CountingDurableIdGenerator,
        clock: ManualClock,
    ) {
        self.service =
            CoreService::with_id_generator_and_clock(&self.runtime_home_path, generator, clock);
    }

    fn use_clock(&mut self, clock: ManualClock) {
        self.service = CoreService::with_clock(&self.runtime_home_path, clock);
    }
}

fn set_method_harness_connection_verification_status(
    harness: &MethodHarness,
    status: &str,
) -> Result<(), Box<dyn Error>> {
    ensure_agent_connection(
        &harness.runtime_home_path,
        AgentConnectionRegistration {
            connection_internal_id: CONNECTION_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            intent: volicord_store::agent_connections::CONNECTION_INTENT_SHARED.to_owned(),
            host_scope: HOST_SCOPE_PROJECT.to_owned(),
            server_name: "volicord-method-test".to_owned(),
            config_target: harness
                .runtime_home_path
                .join("agent-connections")
                .join(CONNECTION_ID)
                .to_string_lossy()
                .into_owned(),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: true,
            managed_fingerprint: "fixture:methods".to_owned(),
            last_verification_status: status.to_owned(),
            last_verification_report_json: "{}".to_owned(),
            last_user_actions_json: "[]".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct UserJudgmentActorProvenance {
    resolved_by_actor_source: Option<String>,
    resolved_verification_basis: Option<String>,
    resolved_assurance_level: Option<String>,
}

#[test]
fn reused_request_id_does_not_collide_for_core_generated_records() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request_id = "req_reused_for_generated_ids";

    let first_intake = harness.service.intake(
        intake_request(
            request_id,
            "idem_reused_intake_1",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_task_id = response_record_id(&first_intake.response_value, "task_ref");
    let first_event_id = response_event_id(&first_intake.response_value);

    let second_intake = harness.service.intake(
        intake_request(
            request_id,
            "idem_reused_intake_2",
            false,
            Some(1),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_task_id = response_record_id(&second_intake.response_value, "task_ref");
    let second_event_id = response_event_id(&second_intake.response_value);
    assert_ne!(first_task_id, second_task_id);
    assert_ne!(first_event_id, second_event_id);

    let first_scope = harness.service.update_scope(
        update_scope_request(
            request_id,
            "idem_reused_scope_1",
            false,
            Some(2),
            &second_task_id,
            ChangeUnitOperation::CreateCurrent,
            "First reused request scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_change_unit_id = response_record_id(&first_scope.response_value, "change_unit_ref");
    let first_scope_event_id = response_event_id(&first_scope.response_value);

    let second_scope = harness.service.update_scope(
        update_scope_request(
            request_id,
            "idem_reused_scope_2",
            false,
            Some(3),
            &second_task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Second reused request scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_change_unit_id = response_record_id(&second_scope.response_value, "change_unit_ref");
    let second_scope_event_id = response_event_id(&second_scope.response_value);
    assert_ne!(first_change_unit_id, second_change_unit_id);
    assert_ne!(first_scope_event_id, second_scope_event_id);

    let first_write = harness.service.prepare_write(
        prepare_write_request(
            request_id,
            "idem_reused_write_1",
            Some(4),
            Some(&second_task_id),
            Some(&second_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_write_id = response_record_id(&first_write.response_value, "write_check_ref");
    let first_write_event_id = response_event_id(&first_write.response_value);

    let second_write = harness.service.prepare_write(
        prepare_write_request(
            request_id,
            "idem_reused_write_2",
            Some(5),
            Some(&second_task_id),
            Some(&second_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_write_id = response_record_id(&second_write.response_value, "write_check_ref");
    let second_write_event_id = response_event_id(&second_write.response_value);
    assert_ne!(first_write_id, second_write_id);
    assert_ne!(first_write_event_id, second_write_event_id);

    let first_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            request_id,
            "idem_reused_judgment_1",
            false,
            Some(6),
            &second_task_id,
            Some(&second_change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_judgment_id = response_record_id(&first_judgment.response_value, "user_judgment_ref");
    let first_judgment_event_id = response_event_id(&first_judgment.response_value);

    let second_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            request_id,
            "idem_reused_judgment_2",
            false,
            Some(7),
            &second_task_id,
            Some(&second_change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_judgment_id =
        response_record_id(&second_judgment.response_value, "user_judgment_ref");
    let second_judgment_event_id = response_event_id(&second_judgment.response_value);
    assert_ne!(first_judgment_id, second_judgment_id);
    assert_ne!(first_judgment_event_id, second_judgment_event_id);

    Ok(())
}

#[test]
fn reused_request_id_stage_artifact_returns_distinct_handles() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_reused_request")?;

    let first = harness.service.stage_artifact(
        stage_artifact_request("req_stage_reused", None, false, None, &task_id),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second = harness.service.stage_artifact(
        stage_artifact_request("req_stage_reused", None, false, None, &task_id),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let first_handle = first.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("first handle should be present");
    let second_handle = second.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("second handle should be present");
    assert_ne!(first_handle, second_handle);
    assert_eq!(harness.counts()?.artifact_staging, 2);
    Ok(())
}

#[test]
fn idempotent_replay_returns_original_generated_ids() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request = intake_request(
        "req_replay_generated_ids",
        "idem_replay_generated_ids",
        false,
        Some(0),
        RequestedMode::Work,
    );

    let first = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert!(second.replayed);
    assert_eq!(
        response_record_id(&first.response_value, "task_ref"),
        response_record_id(&second.response_value, "task_ref")
    );
    assert_eq!(
        response_event_id(&first.response_value),
        response_event_id(&second.response_value)
    );
    assert_eq!(harness.counts()?.tasks, 1);
    assert_eq!(harness.counts()?.task_events, 1);
    Ok(())
}

#[test]
fn deterministic_generated_id_collision_retries_bounded_candidates() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    insert_superseding_task(&harness, "task_collision")?;
    harness.service = CoreService::with_id_generator(
        &harness.runtime_home_path,
        SequenceDurableIdGenerator::new(["collision", "fresh", "event"]),
    );

    let response = harness.service.intake(
        intake_request(
            "req_collision_retry",
            "idem_collision_retry",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response_record_id(&response.response_value, "task_ref"),
        "task_fresh"
    );
    assert_eq!(response_event_id(&response.response_value), "evt_event");
    assert_eq!(harness.counts()?.tasks, 2);
    Ok(())
}

fn response_record_id(response_value: &Value, field: &str) -> String {
    response_value[field]["record_id"]
        .as_str()
        .expect("record_id should be present")
        .to_owned()
}

fn response_event_id(response_value: &Value) -> String {
    response_value["base"]["events"][0]["event_id"]
        .as_str()
        .expect("event_id should be present")
        .to_owned()
}

fn test_state_record_ref(
    record_kind: StateRecordKind,
    record_id: &str,
    project_id: &str,
    task_id: &str,
    state_version: Option<u64>,
) -> StateRecordRef {
    StateRecordRef {
        record_kind,
        record_id: RecordId::new(record_id),
        project_id: ProjectId::new(project_id),
        task_id: Some(TaskId::new(task_id)).into(),
        state_version: state_version.into(),
    }
}

#[test]
fn status_is_read_only_including_dry_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status", None, false, None, None),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], false);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(harness.counts()?, before);

    let dry_run = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_dry",
                Some("idem_status_dry"),
                true,
                Some(0),
                None,
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(dry_run.response_value["base"]["response_kind"], "result");
    assert_eq!(dry_run.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(dry_run.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_renders_effective_write_check_expiration_without_mutating_row(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_auth_expired")?;
    insert_active_write_check_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_status_future",
        2,
        "2026-06-18T00:00:00.000Z",
        "2999-01-01T00:00:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_auth_expired", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["write_check_summary"]["status"],
        "expired"
    );
    assert_eq!(
        response.response_value["active_task"]["write_check_summary"]["status"],
        "expired"
    );
    assert_eq!(write_check_status(&harness, "wa_status_future")?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_include_evidence_returns_current_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_evidence")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_evidence",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_evidence", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_check: false,
                evidence: true,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["claim"],
        "Close claim supported."
    );
    assert_eq!(
        response.response_value["active_task"]["evidence_summary"],
        response.response_value["evidence_summary"]
    );
    assert_field_absent(&response.response_value, "current_close_basis");
    assert_field_absent(&response.response_value, "close_state");
    assert_field_absent(&response.response_value, "close_blockers");
    assert_field_absent(&response.response_value, "risk_acceptance_coverage");
    assert_field_absent(&response.response_value["active_task"], "close_state");
    assert_field_absent(&response.response_value["active_task"], "close_blockers");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_include_matches_close_task_check_blockers() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_close")?;
    record_close_evidence(&harness, &task_id, &change_unit_id, 2, "status_close", true)?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_close", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_status_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(status.response_value["close_state"], "blocked");
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(
        status.response_value["current_close_basis"],
        check.response_value["current_close_basis"]
    );
    assert_eq!(
        status.response_value["close_blockers"],
        check.response_value["blockers"]
    );
    assert_close_blocker(&status.response_value, "missing_final_acceptance");
    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_ne!(
        status.response_value["guarantee_display"]["level"],
        "detective"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_ready_close_uses_empty_blockers_only_after_computation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_ready_empty")?;
    let after_run = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_ready_empty",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_run,
        "status_ready_empty",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_ready_empty", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(status.response_value["close_state"], "ready");
    assert_eq!(status.response_value["close_blockers"], json!([]));
    assert_eq!(status.response_value["active_task"]["close_state"], "ready");
    assert_eq!(
        status.response_value["active_task"]["close_blockers"],
        json!([])
    );
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_include_false_omits_optional_sections_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_flags")?;
    record_close_evidence(&harness, &task_id, &change_unit_id, 2, "status_flags", true)?;
    let before = harness.counts()?;

    let none = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_flags_none", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert!(none.response_value["active_task"].is_null());
    assert!(none.response_value["write_check_summary"].is_null());
    assert_field_absent(&none.response_value, "evidence_summary");
    assert_field_absent(&none.response_value, "close_state");
    assert_field_absent(&none.response_value, "current_close_basis");
    assert_field_absent(&none.response_value, "risk_acceptance_coverage");
    assert_field_absent(&none.response_value, "close_blockers");
    assert_field_absent(&none.response_value, "guarantee_display");
    assert_no_close_next_actions(&none.response_value);

    let evidence_only = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_flags_evidence",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: true,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(evidence_only.response_value["active_task"].is_null());
    assert_eq!(
        evidence_only.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_field_absent(&evidence_only.response_value, "close_state");
    assert_field_absent(&evidence_only.response_value, "close_blockers");
    assert_field_absent(&evidence_only.response_value, "guarantee_display");
    assert_no_close_next_actions(&evidence_only.response_value);

    let close_only = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_flags_close", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(close_only.response_value["active_task"].is_null());
    assert_field_absent(&close_only.response_value, "evidence_summary");
    assert_field_absent(&close_only.response_value, "guarantee_display");
    assert_close_blocker(&close_only.response_value, "missing_final_acceptance");

    let guarantees_only = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_flags_guarantee",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(guarantees_only.response_value["active_task"].is_null());
    assert_field_absent(&guarantees_only.response_value, "evidence_summary");
    assert_field_absent(&guarantees_only.response_value, "close_state");
    assert_field_absent(&guarantees_only.response_value, "close_blockers");
    assert_eq!(
        guarantees_only.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_false_does_not_read_corrupt_close_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_close_not_read")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let excluded = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_close_not_read_excluded",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(excluded.response_value["base"]["response_kind"], "result");
    assert_field_absent(&excluded.response_value, "close_state");
    assert_field_absent(&excluded.response_value, "current_close_basis");
    assert_field_absent(&excluded.response_value, "close_blockers");
    assert_field_absent(&excluded.response_value["active_task"], "close_state");
    assert_field_absent(&excluded.response_value["active_task"], "close_blockers");
    assert_no_close_next_actions(&excluded.response_value);
    assert_eq!(harness.counts()?, before);

    let selected = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_close_not_read_selected",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &selected,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn fresh_project_registration_creates_baseline_enforcement_profile() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let profile_json = harness.project_enforcement_profile_json()?;
    assert_eq!(profile_json, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON);

    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let record = store.project_enforcement_profile()?;
    assert_eq!(record.project_id, PROJECT_ID);
    assert_eq!(record.profile.profile_id, "baseline_cooperative");
    assert_eq!(record.profile.enabled_mechanisms.len(), 0);
    Ok(())
}

#[test]
fn status_guarantee_include_false_does_not_read_corrupt_profile() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_profile_skip")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_profile_skip",
        true,
    )?;
    harness.set_project_enforcement_profile_json(corrupt_owner_json())?;
    let before = harness.counts()?;

    let excluded = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_profile_skip_excluded",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(excluded.response_value["base"]["response_kind"], "result");
    assert_field_absent(&excluded.response_value, "guarantee_display");
    assert_field_absent(&excluded.response_value["active_task"], "guarantee_display");
    assert_eq!(harness.counts()?, before);

    let selected = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_profile_skip_selected",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &selected,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_guarantee_include_true_rejects_unsupported_profile_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_project_enforcement_profile_json(
        &json!({
            "profile_id": "baseline_cooperative",
            "guarantee_level": "detective",
            "enabled_mechanisms": [],
            "source": "baseline_scope",
            "status": "active"
        })
        .to_string(),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_profile_detective", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_guarantee_include_true_rejects_missing_profile_fields() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_project_enforcement_profile_json(
        &json!({
            "profile_id": "baseline_cooperative",
            "enabled_mechanisms": [],
            "source": "baseline_scope",
            "status": "active"
        })
        .to_string(),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_profile_missing", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarantee_display_uses_verified_invocation_without_profile_elevation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_guarantee_invocation", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_check: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_ne!(
        status.response_value["guarantee_display"]["level"],
        "detective"
    );
    assert!(status.response_value["guarantee_display"]["basis"]
        .as_str()
        .is_some_and(|basis| {
            basis.contains(AGENT_ACTOR_SOURCE)
                && basis.contains("baseline_cooperative")
                && basis.contains("read")
                && basis.contains("enabled mechanisms: none")
                && basis.contains("no stronger enforcement")
        }));
    assert_eq!(
        status.response_value["guarantee_display"]["capability_refs"][0]["record_kind"],
        "agent_connection"
    );
    assert_eq!(
        status.response_value["guarantee_display"]["capability_refs"][0]["record_id"],
        CONNECTION_ID
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_reports_exact_missing_residual_risk_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_risk")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_risk",
        vec![
            residual_risk_input("First status risk."),
            residual_risk_input("Second status risk."),
        ],
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "status_risk",
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_risk", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let coverage = response.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    let projected_ids = coverage
        .iter()
        .map(|item| item["risk_id"].as_str().expect("risk_id").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(projected_ids, risk_ids);
    assert!(coverage.iter().all(|item| item["accepted"] == false));
    assert!(coverage
        .iter()
        .all(|item| item["missing_reason"] == "acceptance_required"));
    assert_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_shows_stale_final_acceptance_blocker_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_stale_final")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_stale_final_old",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "status_stale_final",
    )?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "status_stale_final_new",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_stale_final", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(user_judgment_status(&harness, &final_judgment_id)?, "stale");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    let final_blocker = response.response_value["close_blockers"]
        .as_array()
        .expect("close blockers")
        .iter()
        .find(|blocker| blocker["code"] == "stale_final_acceptance")
        .expect("final acceptance blocker");
    assert!(final_blocker["related_refs"]
        .as_array()
        .expect("related refs")
        .iter()
        .any(|record_ref| record_ref["record_id"] == final_judgment_id));
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn invalid_stored_method_owned_json_routes_to_structured_unavailability(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_method_json")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_method_json_judgment",
            "idem_bad_method_json_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    harness.conn()?.execute(
        "UPDATE user_judgments
                SET options_json = '{not-json'
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id],
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_bad_method_json_record",
            "idem_bad_method_json_record",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "options_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn authority_owner_json_decode_paths_do_not_reintroduce_fail_open_patterns() {
    let sources = [
        ("methods/mod.rs", include_str!("mod.rs")),
        ("methods/close_task.rs", include_str!("close_task.rs")),
        ("methods/prepare_write.rs", include_str!("prepare_write.rs")),
        ("methods/record_run.rs", include_str!("record_run.rs")),
        ("methods/update_scope.rs", include_str!("update_scope.rs")),
        ("methods/status.rs", include_str!("status.rs")),
    ];
    let forbidden = [
        "parse_json_object(&task.completion_policy_json)",
        "parse_json_object(&context.task.close_summary_json)",
        "parse_json_object(&record.close_basis_json)",
        "parse_json_object(&record.lifecycle_json)",
        "parse_json_object(&change_unit.write_basis_json)",
        "serde_json::from_str::<Vec<String>>(&change_unit.bounded_paths_json).unwrap_or_default()",
    ];

    for (path, source) in sources {
        for pattern in forbidden {
            assert!(
                !source.contains(pattern),
                "{path} reintroduced fail-open owner-state JSON decoding: {pattern}"
            );
        }
    }
}

#[test]
fn public_methods_use_same_verified_invocation_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "verified_context")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_verified_status", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_verified_invocation(&status, OperationCategory::Read);

    let intake = harness.service.intake(
        intake_request(
            "req_verified_intake",
            "idem_verified_intake",
            true,
            Some(2),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&intake, OperationCategory::AgentWorkflow);

    let update_scope = harness.service.update_scope(
        update_scope_request(
            "req_verified_scope",
            "idem_verified_scope",
            true,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&update_scope, OperationCategory::AgentWorkflow);

    let mut prepare_write = prepare_write_request(
        "req_verified_prepare",
        "idem_verified_prepare",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare_write.envelope.dry_run = true;
    let prepare_write = harness
        .service
        .prepare_write(prepare_write, invocation(OperationCategory::AgentWorkflow))?;
    assert_verified_invocation(&prepare_write, OperationCategory::AgentWorkflow);

    let stage_artifact = harness.service.stage_artifact(
        stage_artifact_request(
            "req_verified_stage",
            Some("idem_verified_stage"),
            true,
            Some(2),
            &task_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&stage_artifact, OperationCategory::AgentWorkflow);

    let record_run = harness.service.record_run(
        record_run_request(
            "req_verified_run",
            "idem_verified_run",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&record_run, OperationCategory::AgentWorkflow);

    let request_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_verified_judgment_preview",
            "idem_verified_judgment_preview",
            true,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&request_judgment, OperationCategory::AgentWorkflow);

    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_verified_judgment_pending",
            "idem_verified_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut record_judgment = record_judgment_request(
        "req_verified_record_judgment",
        "idem_verified_record_judgment",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_judgment.envelope.dry_run = true;
    let record_judgment = harness
        .service
        .record_user_judgment(record_judgment, invocation(OperationCategory::UserOnly))?;
    assert_verified_invocation(&record_judgment, OperationCategory::UserOnly);

    let close_check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_verified_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_verified_invocation(&close_check, OperationCategory::Read);

    Ok(())
}

#[test]
fn intake_commits_once_and_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let request = intake_request(
        "req_intake",
        "idem_intake",
        false,
        Some(0),
        RequestedMode::Auto,
    );

    let first = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;

    assert_eq!(first.response_value["base"]["response_kind"], "result");
    assert_eq!(
        first.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(first.response_value["base"]["state_version"], 1);
    assert_eq!(first.response_value["state"]["mode"], "work");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.tasks, before.tasks + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);

    let second = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn intake_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let response = harness.service.intake(
        intake_request(
            "req_intake_dry",
            "idem_intake_dry",
            true,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn update_scope_commits_once_and_creates_one_current_change_unit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_scope_task",
            "idem_scope_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_create",
            "idem_scope_create",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Create current export scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 2);
    assert!(response.response_value["change_unit_ref"].is_object());
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
    let revision = task_revision(&harness, &task_id)?;
    assert_eq!(revision.scope_revision, 1);
    assert_eq!(revision.close_basis_revision, 1);
    assert!(revision.current_close_basis.is_none());
    Ok(())
}

#[test]
fn update_scope_replaces_current_and_marks_write_check_stale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_replace_task",
            "idem_replace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let create = harness.service.update_scope(
        update_scope_request(
            "req_replace_create",
            "idem_replace_create",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = create.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    insert_active_write_check(&harness, &task_id, &change_unit_id)?;
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_replace_current",
            "idem_replace_current",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["stale_write_check_refs"]
            .as_array()
            .expect("stale refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
    assert_eq!(write_check_status(&harness, "wa_replace")?, "stale");
    Ok(())
}

#[test]
fn material_scope_change_increments_revision_and_invalidates_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_invalidates")?;
    let mut record = record_run_request(
        "req_scope_basis_run",
        "idem_scope_basis_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    record.close_assessment = Some(close_assessment_with_risks(
        "Established close basis.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(record, invocation(OperationCategory::AgentWorkflow))?;
    let before = task_revision(&harness, &task_id)?;
    assert!(before.current_close_basis.is_some());

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_material_change",
            "idem_scope_material_change",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Materially changed current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = task_revision(&harness, &task_id)?;
    let (_, event_payload, _) = latest_task_event(&harness)?;

    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(after.scope_revision, before.scope_revision + 1);
    assert_eq!(after.close_basis_revision, before.close_basis_revision + 1);
    assert!(after.current_close_basis.is_none());
    assert_eq!(response.response_value["state"]["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value["state"],
        "missing_current_close_basis",
    );
    assert_eq!(event_payload["scope_changed"], true);
    assert_eq!(event_payload["scope_revision"], after.scope_revision);
    assert_eq!(
        event_payload["close_basis_revision"],
        after.close_basis_revision
    );
    Ok(())
}

#[test]
fn semantic_noop_scope_update_does_not_increment_revisions() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "scope_noop")?;
    let before = task_revision(&harness, &task_id)?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_noop",
            "idem_scope_noop",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "  Initial current scope.  ",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = task_revision(&harness, &task_id)?;
    let (_, event_payload, _) = latest_task_event(&harness)?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(after.scope_revision, before.scope_revision);
    assert_eq!(after.close_basis_revision, before.close_basis_revision);
    assert_eq!(after.current_close_basis, before.current_close_basis);
    assert_eq!(event_payload["scope_changed"], false);
    Ok(())
}

#[test]
fn update_scope_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_dry_task",
            "idem_dry_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_dry",
            "idem_scope_dry",
            true,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Dry-run scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_ref_alone_does_not_change_current_scope() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_decision_task",
            "idem_decision_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let decision = harness.service.request_user_judgment(
        user_judgment_request(
            "req_scope_decision_ref_only",
            "idem_scope_decision_ref_only",
            false,
            Some(1),
            &task_id,
            None,
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let decision_ref: StateRecordRef =
        serde_json::from_value(decision.response_value["user_judgment_ref"].clone())?;
    let decision_id = decision_ref.record_id.as_str().to_owned();
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_scope_decision_ref_only_record",
            "idem_scope_decision_ref_only_record",
            Some(2),
            &task_id,
            &decision_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let response = harness.service.update_scope(
        UpdateScopeRequest {
            envelope: envelope(
                "req_decision_only",
                Some("idem_decision_only"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            goal_summary: None.into(),
            scope_update: None.into(),
            scope_boundary: None.into(),
            non_goals: None.into(),
            acceptance_criteria: None.into(),
            autonomy_boundary: None.into(),
            baseline_ref: None.into(),
            change_unit: ChangeUnitUpdate {
                operation: ChangeUnitOperation::KeepCurrent,
                effect_contract: None,
                fields: Map::new(),
            },
            related_scope_decision_refs: vec![decision_ref],
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["state"]["scope_summary"],
        "Initial test scope."
    );
    assert_eq!(
        response.response_value["linked_scope_decision_refs"]
            .as_array()
            .expect("linked refs should be an array")
            .len(),
        1
    );
    Ok(())
}

#[test]
fn accepted_current_user_scope_decision_links_scope_update() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_link_accept")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "link_accept",
        true,
    )?;

    let mut request = update_scope_request(
        "req_scope_link_accept_update",
        "idem_scope_link_accept_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Decision-backed material scope.",
    );
    request.related_scope_decision_refs = vec![decision_ref.clone()];
    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["linked_scope_decision_refs"],
        json!([decision_ref])
    );
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "stale");
    assert_eq!(user_judgment_basis_status(&harness, &decision_id)?, "stale");
    Ok(())
}

#[test]
fn rejected_scope_decision_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let suffix = "scope_link_rejected";
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
    let (state_version, decision_ref, decision_id) =
        record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, suffix, false)?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_link_rejected_update",
        "idem_scope_link_rejected_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Rejected scope decision must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &decision_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "resolved");
    Ok(())
}

#[test]
fn agent_or_unverified_scope_decision_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    for case in ["agent_actor", "agent_source", "missing_provenance"] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("scope_{case}"))?;
        let (state_version, decision_ref, decision_id) =
            record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, case, true)?;
        match case {
            "agent_actor" => {
                set_user_judgment_resolution_actor(&harness, &decision_id, AGENT_ACTOR_SOURCE)?
            }
            "agent_source" => {
                set_user_judgment_resolved_by_actor_source(&harness, &decision_id, "agent")?;
            }
            "missing_provenance" => {
                clear_user_judgment_actor_provenance(&harness, &decision_id)?;
            }
            _ => unreachable!("covered cases are exhaustive"),
        }
        let before = harness.counts()?;
        let mut request = update_scope_request(
            &format!("req_{case}_scope_link"),
            &format!("idem_{case}_scope_link"),
            false,
            Some(state_version),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Agent-recorded scope decision must not link.",
        );
        request.related_scope_decision_refs = vec![decision_ref];

        let response = harness
            .service
            .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, before);
        assert_eq!(user_judgment_status(&harness, &decision_id)?, "resolved");
    }
    Ok(())
}

#[test]
fn scope_decision_for_other_operation_cannot_authorize_scope_update() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_required_for")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "required_for",
        true,
    )?;
    set_user_judgment_required_for(
        &harness,
        &decision_id,
        &[volicord_types::JudgmentRequiredFor::PrepareWrite],
    )?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_required_for_update",
        "idem_scope_required_for_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Prepare-write decision must not authorize scope update.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "resolved");
    Ok(())
}

#[test]
fn old_revision_scope_decision_cannot_be_reused() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_old_revision")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "old_revision",
        true,
    )?;
    let autonomous = harness.service.update_scope(
        update_scope_request(
            "req_scope_old_revision_first",
            "idem_scope_old_revision_first",
            false,
            Some(state_version),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Autonomous material scope change before reuse.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let next_state_version = autonomous.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "stale");

    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_old_revision_reuse",
        "idem_scope_old_revision_reuse",
        false,
        Some(next_state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Attempt to reuse stale scope decision.",
    );
    request.related_scope_decision_refs = vec![decision_ref];
    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_for_another_change_unit_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_other_cu")?;
    let (state_version, decision_ref, decision_id) =
        record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, "other_cu", true)?;
    mutate_user_judgment_basis_json(&harness, &decision_id, |basis| {
        basis["change_unit_id"] = json!("cu_not_current");
    })?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_other_cu_update",
        "idem_scope_other_cu_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Other Change Unit decision must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_with_incompatible_affected_refs_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "scope_bad_affected_refs")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_affected_refs",
        true,
    )?;
    let incompatible_ref = test_state_record_ref(
        StateRecordKind::ChangeUnit,
        "cu_not_current",
        PROJECT_ID,
        &task_id,
        Some(2),
    );
    set_user_judgment_affected_refs(&harness, &decision_id, &[incompatible_ref])?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_bad_affected_refs_update",
        "idem_scope_bad_affected_refs_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Incompatible affected refs must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn expired_scope_decision_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_expired")?;
    let (state_version, decision_ref, decision_id) =
        record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, "expired", true)?;
    set_user_judgment_expires_at(&harness, &decision_id, "2000-01-01T00:00:00Z")?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_expired_update",
        "idem_scope_expired_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Expired scope decision must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn invalid_related_scope_decision_ref_has_no_update_scope_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "scope_invalid_ref")?;
    let original_scope = current_change_unit_scope(&harness, &task_id)?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_invalid_ref_update",
        "idem_scope_invalid_ref_update",
        false,
        Some(2),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Invalid ref must not update scope.",
    );
    request.related_scope_decision_refs = vec![test_state_record_ref(
        StateRecordKind::UserJudgment,
        "uj_missing_scope_decision",
        PROJECT_ID,
        &task_id,
        Some(2),
    )];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        current_change_unit_scope(&harness, &task_id)?,
        original_scope
    );
    Ok(())
}

#[test]
fn autonomous_scope_update_still_succeeds_without_scope_decision() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "scope_autonomous")?;
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_autonomous_update",
            "idem_scope_autonomous_update",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Autonomous scope update with no decision ref.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?.state_version, before.state_version + 1);
    assert_eq!(
        response.response_value["linked_scope_decision_refs"],
        json!([])
    );
    Ok(())
}

#[test]
fn material_scope_update_invalidates_scope_decisions_atomically() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "scope_atomic_invalidation")?;
    let (after_resolved, _, resolved_decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "atomic_resolved",
        true,
    )?;
    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_scope_atomic_pending",
            "idem_scope_atomic_pending",
            false,
            Some(after_resolved),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_decision_id = response_record_id(&pending.response_value, "user_judgment_ref");
    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_atomic_update",
            "idem_scope_atomic_update",
            false,
            Some(after_resolved + 1),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Material scope change invalidates scope decisions.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        user_judgment_status(&harness, &resolved_decision_id)?,
        "stale"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &resolved_decision_id)?,
        "stale"
    );
    assert_eq!(
        user_judgment_status(&harness, &pending_decision_id)?,
        "superseded"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &pending_decision_id)?,
        "superseded"
    );
    Ok(())
}

#[test]
fn prepare_write_allowed_creates_one_write_check_with_post_commit_basis(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_allowed")?;
    let sensitive_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_prepare_allowed_sensitive",
            "idem_prepare_allowed_sensitive",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let sensitive_judgment_id =
        response_record_id(&sensitive_judgment.response_value, "user_judgment_ref");
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_prepare_allowed_record",
            "idem_prepare_allowed_record",
            Some(3),
            &task_id,
            &sensitive_judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let id_generator =
        CountingDurableIdGenerator::new(["prepare_allowed_auth", "prepare_allowed_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_allowed",
        "idem_prepare_allowed",
        Some(4),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(response.response_value["write_check_effect"], "created");
    assert_eq!(response.response_value["base"]["state_version"], 5);
    assert_eq!(
        response.response_value["write_check"]["basis_state_version"],
        5
    );
    assert_eq!(
        response.response_value["write_check"]["attempt_scope"]["intended_paths"],
        json!(["src/export.rs"])
    );
    assert_eq!(
        response.response_value["active_user_judgment_refs"]
            .as_array()
            .expect("active judgment refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_checks, before.write_checks + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let write_check_id = response_record_id(&response.response_value, "write_check_ref");
    assert_eq!(write_check_basis(&harness, &write_check_id)?, 5);
    let (created_at, expires_at) = write_check_timestamps(&harness, &write_check_id)?;
    assert_eq!(created_at, "2026-06-18T00:00:00Z");
    assert_eq!(expires_at, "2026-06-18T00:15:00Z");
    assert_eq!(
        response.response_value["write_check"]["expires_at"],
        expires_at
    );
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_prepare_allowed_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["base"]["state_version"], 5);
    let mut response_state = response.response_value["state"].clone();
    let status_state = status.response_value["active_task"].clone();
    response_state["guarantee_display"] = status_state["guarantee_display"].clone();
    response_state["write_check_summary"]["guarantee_display"] =
        status_state["write_check_summary"]["guarantee_display"].clone();
    assert_eq!(response_state, status_state);
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 1);
    Ok(())
}

#[test]
fn change_unit_effect_contract_is_stored_and_returned() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_effect_contract_task",
            "idem_effect_contract_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let contract = ChangeUnitEffectContract {
        allowed_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
        forbidden_effects: vec![ChangeUnitEffectKind::ExternalNetwork],
        allowed_paths: vec!["src/export.rs".to_owned()],
        expected_outputs: vec!["Updated export behavior.".to_owned()],
        invariants: vec!["Do not alter unrelated exports.".to_owned()],
        evidence_expectations: vec!["Record a focused test run.".to_owned()],
        sensitive_action_expectations: vec!["No secret access is expected.".to_owned()],
    };
    let mut request = update_scope_request(
        "req_effect_contract_scope",
        "idem_effect_contract_scope",
        false,
        Some(1),
        &task_id,
        ChangeUnitOperation::CreateCurrent,
        "Effect-contract current scope.",
    );
    request.change_unit.effect_contract = Some(contract.clone());

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_effect_contract_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let expected = serde_json::to_value(contract)?;
    assert_eq!(
        response.response_value["state"]["effect_contract"],
        expected
    );
    assert_eq!(
        status.response_value["active_task"]["effect_contract"],
        expected
    );
    Ok(())
}

#[test]
fn state_summary_reports_absent_effect_contract_as_null() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "no_effect_contract")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_no_effect_contract_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_no_effect_contract_prepare",
            "idem_no_effect_contract_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert!(status.response_value["active_task"]["effect_contract"].is_null());
    assert_eq!(response.response_value["decision"], "allowed");
    assert!(response.response_value["state"]["effect_contract"].is_null());
    Ok(())
}

#[test]
fn prepare_write_rejects_product_write_forbidden_by_effect_contract() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_forbid_write",
        ChangeUnitEffectContract {
            forbidden_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_contract_forbid_write",
            "idem_contract_forbid_write",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(
        &response.response_value,
        "effect_contract_forbids_product_file_write",
    );
    assert_eq!(
        response.response_value["write_decision_reasons"][0]["category"],
        "effect_contract"
    );
    assert!(response.response_value["write_check"].is_null());
    assert_eq!(harness.counts()?.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn prepare_write_rejects_paths_outside_effect_contract_allowed_paths() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_path",
        ChangeUnitEffectContract {
            allowed_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
            allowed_paths: vec!["tests".to_owned()],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_contract_path",
            "idem_contract_path",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "effect_contract_path_not_allowed");
    assert!(response.response_value["write_check"].is_null());
    assert_eq!(harness.counts()?.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn effect_contract_does_not_create_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_effect_contract(
        &harness,
        "contract_no_final",
        ChangeUnitEffectContract {
            expected_outputs: vec!["Implementation output is expected.".to_owned()],
            evidence_expectations: vec!["Evidence is expected before close.".to_owned()],
            ..ChangeUnitEffectContract::default()
        },
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_contract_no_final_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    Ok(())
}

#[test]
fn effect_contract_does_not_replace_sensitive_approval() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_sensitive",
        ChangeUnitEffectContract {
            allowed_effects: vec![
                ChangeUnitEffectKind::ProductFileWrite,
                ChangeUnitEffectKind::SensitiveAction,
            ],
            sensitive_action_expectations: vec!["Network-sensitive step may be needed.".to_owned()],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_contract_sensitive",
        "idem_contract_sensitive",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_check"].is_null());
    assert_eq!(harness.counts()?.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn prepare_write_blocked_path_creates_no_write_check() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_path")?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_blocked_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_path",
        "idem_prepare_path",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "path_out_of_scope");
    assert!(response.response_value["write_check"].is_null());
    assert!(response.response_value["write_check_ref"].is_null());
    assert_eq!(response.response_value["write_check_effect"], "none");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(after.artifact_staging, before.artifact_staging);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.artifact_links, before.artifact_links);
    assert_eq!(after.evidence_summaries, before.evidence_summaries);
    assert_eq!(after.blockers, before.blockers);
    assert_eq!(after.runs, before.runs);
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "blocked",
        "path_out_of_scope",
    )?;
    assert_eq!(event_payload["task_id"], task_id);
    assert_eq!(event_payload["change_unit_id"], change_unit_id);
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "scope");
    assert_eq!(reason["code"], "path_out_of_scope");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("outside the current Change Unit path scope"));
    assert!(!reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_missing_change_unit_returns_decision_reason() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_prepare_no_cu_task",
            "idem_prepare_no_cu_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_no_cu",
        "idem_prepare_no_cu",
        Some(1),
        Some(&task_id),
        None,
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "no_current_change_unit");
    assert_eq!(after.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn prepare_write_unresolved_user_judgment_requires_decision() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_judgment")?;
    let mut judgment_request = user_judgment_request(
        "req_prepare_judgment_pending",
        "idem_prepare_judgment_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    judgment_request.required_for = vec![volicord_types::JudgmentRequiredFor::PrepareWrite];
    harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_decision_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_judgment",
        "idem_prepare_judgment",
        Some(3),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "decision_required");
    assert_prepare_reason(&response.response_value, "user_judgment_unresolved");
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "decision_required",
        "user_judgment_unresolved",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "user_judgment");
    assert_eq!(reason["code"], "user_judgment_unresolved");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("user-owned judgment"));
    assert!(!reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_ignores_pending_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_ignore_final")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "prepare_ignore_final",
        true,
    )?;
    harness.service.request_user_judgment(
        user_judgment_request(
            "req_prepare_ignore_final_pending",
            "idem_prepare_ignore_final_pending",
            false,
            Some(after_evidence),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_ignore_final",
            "idem_prepare_ignore_final",
            Some(after_evidence + 1),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert!(response.response_value["write_decision_reasons"]
        .as_array()
        .expect("write_decision_reasons should be an array")
        .is_empty());
    assert_eq!(harness.counts()?.write_checks, before.write_checks + 1);
    Ok(())
}

#[test]
fn informational_judgment_does_not_block_prepare_write_or_close_check() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "informational_judgment")?;
    let mut judgment_request = user_judgment_request(
        "req_info_pending",
        "idem_info_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    );
    judgment_request.required_for = vec![volicord_types::JudgmentRequiredFor::Informational];
    harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let prepare = harness.service.prepare_write(
        prepare_write_request(
            "req_info_prepare",
            "idem_info_prepare",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(prepare.response_value["decision"], "allowed");

    let close = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_info_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_no_close_blocker(&close.response_value, "pending_user_judgment");
    Ok(())
}

#[test]
fn prepare_write_ignores_another_change_unit_pending_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_other_cu")?;
    let mut judgment_request = user_judgment_request(
        "req_prepare_other_cu_pending",
        "idem_prepare_other_cu_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    judgment_request.required_for = vec![volicord_types::JudgmentRequiredFor::PrepareWrite];
    let judgment = harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    mutate_user_judgment_basis_json(&harness, &judgment_id, |basis| {
        basis["change_unit_id"] = json!("cu_unrelated");
    })?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_other_cu",
            "idem_prepare_other_cu",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_no_prepare_reason(&response.response_value, "user_judgment_unresolved");
    assert_eq!(harness.counts()?.write_checks, before.write_checks + 1);
    Ok(())
}

#[test]
fn malformed_stored_required_for_rejects_prepare_write_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_required_for")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_required_for_pending",
            "idem_bad_required_for_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(
            r#"{"presentation":"short","question":"Bad required_for","required_for":["not_a_target"],"expires_at":null}"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_required_for_prepare",
            "idem_bad_required_for_prepare",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_missing_sensitive_approval_requires_approval() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_sensitive")?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_approval_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_sensitive",
        "idem_prepare_sensitive",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "approval_required",
        "sensitive_approval_missing",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "sensitive_approval");
    assert_eq!(reason["code"], "sensitive_approval_missing");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("sensitive-action approval"));
    assert!(reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_baseline_mismatch_blocks_write_check() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_baseline")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_baseline",
        "idem_prepare_baseline",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.baseline_ref = BaselineRef::new("baseline_other");
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "baseline_mismatch");
    assert_eq!(after.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn prepare_write_user_only_category_is_invocation_context_rejection() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_binding")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_invocation_context",
        "idem_prepare_invocation_context",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::UserOnly))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(after.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn prepare_write_uses_agent_workflow_invocation_without_extra_binding() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_binding_ok")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_binding_ok",
        "idem_prepare_binding_ok",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(harness.counts()?.write_checks, before.write_checks + 1);
    Ok(())
}

#[test]
fn prepare_write_uses_agent_workflow_invocation_without_extra_profile() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_cap")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_capability",
        "idem_prepare_capability",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(after.write_checks, before.write_checks + 1);
    Ok(())
}

#[test]
fn prepare_write_product_write_flag_mismatch_blocks_write_check() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_flag")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_flag",
        "idem_prepare_flag",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.product_file_write_intended = false;
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "product_write_flag_mismatch");
    assert_eq!(after.write_checks, before.write_checks);
    Ok(())
}

#[test]
fn prepare_write_dry_run_has_no_write_check_effect() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_dry")?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let mut request = prepare_write_request(
        "req_prepare_dry",
        "idem_prepare_dry",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = true;
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(
        response.response_value["dry_run_summary"]["planned_effects"][0]["action"],
        "would_create"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 0);

    let mut blocked_preview = prepare_write_request(
        "req_prepare_dry_blocked",
        "idem_prepare_dry_blocked",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    blocked_preview.envelope.dry_run = true;
    blocked_preview.intended_paths = vec!["src/other.rs".to_owned()];
    let blocked_preview = harness.service.prepare_write(
        blocked_preview,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        blocked_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(
        blocked_preview.response_value["dry_run_summary"]["would_blockers"][0]["code"],
        "path_out_of_scope"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 0);
    Ok(())
}

#[test]
fn prepare_write_rejects_escaping_product_path_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_escape")?;
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let mut request = prepare_write_request(
        "req_prepare_escape",
        "idem_prepare_escape",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["../outside.rs".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    Ok(())
}

#[test]
fn prepare_write_stale_state_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_stale")?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let request = prepare_write_request(
        "req_prepare_stale",
        "idem_prepare_stale",
        Some(1),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert!(!response.response_json.contains("write_decision_reasons"));
    assert!(!response
        .response_json
        .contains("STATE_VERSION_CONFLICT\",\"category"));
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 0);
    Ok(())
}

#[test]
fn prepare_write_idempotency_replays_without_second_write_check() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_replay")?;
    let id_generator =
        CountingDurableIdGenerator::new(["prepare_replay_auth", "prepare_replay_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock.clone());
    let request = prepare_write_request(
        "req_prepare_replay",
        "idem_prepare_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );

    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    clock.advance(Duration::minutes(5));
    let second = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(first.response_value["decision"], "allowed");
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    assert_eq!(write_check_count(&harness)?, 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteCheck), 1);
    assert_eq!(
        second.response_value["write_check"]["expires_at"],
        first.response_value["write_check"]["expires_at"]
    );
    Ok(())
}

#[test]
fn prepare_write_non_allow_replay_returns_original_response_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_non_allow_replay")?;
    let mut request = prepare_write_request(
        "req_prepare_non_allow_replay",
        "idem_prepare_non_allow_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];
    let before = harness.counts()?;

    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let same_context = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let context_mismatch = harness.service.prepare_write(
        request,
        invocation_with_actor(
            ActorSource::agent_connection("connection_other"),
            OperationCategory::AgentWorkflow,
        ),
    )?;

    assert_eq!(first.response_value["decision"], "blocked");
    assert_prepare_reason(&first.response_value, "path_out_of_scope");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);
    assert_eq!(after_first.write_checks, before.write_checks);
    assert_latest_prepare_write_event(
        &harness,
        &first.response_value,
        "blocked",
        "path_out_of_scope",
    )?;
    assert!(same_context.replayed);
    assert_eq!(same_context.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    assert!(!context_mismatch.replayed);
    assert_eq!(
        context_mismatch.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        context_mismatch.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert!(!context_mismatch.response_json.contains("path_out_of_scope"));
    assert!(context_mismatch
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn prepare_write_replay_requires_current_invocation_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_replay_verify")?;
    let request = prepare_write_request(
        "req_prepare_replay_verify",
        "idem_prepare_replay_verify",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness.service.prepare_write(
        request,
        invocation_with_actor(
            ActorSource::agent_connection("connection_other"),
            OperationCategory::AgentWorkflow,
        ),
    )?;

    assert_eq!(first.response_value["decision"], "allowed");
    assert!(!second.replayed);
    assert_eq!(second.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        second.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_ne!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn stage_artifact_creates_transient_handle_without_core_commit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_valid")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_valid",
        Some("idem_stage_valid"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "trace.log".to_owned();
    request.content_type = "text/plain; charset=utf-8".to_owned();
    request.safe_bytes_or_notice = "Local trace sample captured for debugging.".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present")
        .to_owned();
    let row = staged_artifact_row(&harness, &handle_id)?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "staging_created"
    );
    assert_eq!(response.response_value["base"]["state_version"], 2);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(
        response.response_value["staged_artifact_handle"]["consumed"],
        false
    );
    assert_eq!(response.response_value.get("artifact_ref"), None);
    assert_eq!(after.state_version, before.state_version);
    assert_eq!(after.artifact_staging, before.artifact_staging + 1);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.task_events, before.task_events);
    assert_eq!(after.tool_invocations, before.tool_invocations);
    assert_eq!(row.status, "staged");
    assert_eq!(row.redaction_state, "none");
    assert_eq!(row.created_by_actor_source, AGENT_ACTOR_SOURCE);
    assert!(row.tmp_path.starts_with("artifacts/tmp/"));
    assert!(row.tmp_path.ends_with(".txt"));
    assert!(harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join(&row.tmp_path)
        .exists());
    assert!(
        (23.99..=24.01).contains(&row.ttl_hours),
        "expected 24h TTL, got {}",
        row.ttl_hours
    );
    Ok(())
}

#[test]
fn stage_artifact_rejects_checksum_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_sha")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_sha",
        Some("idem_stage_sha"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "checksum mismatch sample".to_owned();
    request.expected_sha256 =
        Some("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_invalid_checksum_format_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_sha_format")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_sha_format",
        Some("idem_stage_sha_format"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "checksum format sample".to_owned();
    request.expected_sha256 = Some("sha256:0000".to_owned()).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert!(response.response_json.contains("64-character SHA-256"));
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_size_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_size")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_size",
        Some("idem_stage_size"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "size mismatch sample".to_owned();
    request.expected_size_bytes = Some(999).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_oversized_input_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_big")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_big",
        Some("idem_stage_big"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "huge.log".to_owned();
    request.safe_bytes_or_notice = "x".repeat(MAX_STAGED_BODY_BYTES + 1);
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_unsafe_secret_input_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_secret")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_secret",
        Some("idem_stage_secret"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "secrets.log".to_owned();
    request.safe_bytes_or_notice = "password=hunter2".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_unsupported_redaction_state() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::to_value(stage_artifact_request(
        "req_stage_bad_redaction",
        Some("idem_stage_bad_redaction"),
        false,
        Some(2),
        "task_redaction",
    ))?;
    value["redaction_state"] = json!("unsupported");

    let error = serde_json::from_value::<StageArtifactRequest>(value)
        .expect_err("unsupported redaction_state should not deserialize");
    assert!(error.to_string().contains("unknown variant"));
    Ok(())
}

#[test]
fn stage_artifact_dry_run_creates_no_handle_or_storage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_dry",
        Some("idem_stage_dry"),
        true,
        Some(2),
        &task_id,
    );
    request.display_name = "trace.md".to_owned();
    request.content_type = "text/markdown".to_owned();
    request.redaction_state = RedactionState::Redacted;
    request.safe_bytes_or_notice = "Redacted diagnostic excerpt.".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert!(response
        .response_value
        .get("staged_artifact_handle")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_dry_run_still_checks_stale_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry_stale")?;
    let before = harness.counts()?;

    let request = stage_artifact_request(
        "req_stage_dry_stale",
        Some("idem_stage_dry_stale"),
        true,
        Some(1),
        &task_id,
    );
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_invalid_input_does_not_bypass_invocation_preflight() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_invocation_first")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_invocation_first",
        Some("idem_stage_invocation_first"),
        true,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = String::new();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::Read))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_uses_verified_invocation_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_provenance")?;

    let mut request = stage_artifact_request(
        "req_stage_provenance",
        Some("idem_stage_provenance"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "binary.bin".to_owned();
    request.content_type = "application/octet-stream".to_owned();
    request.redaction_state = RedactionState::Blocked;
    request.safe_bytes_or_notice = "Binary output omitted; see local run context.".to_owned();

    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["staged_artifact_handle"]["created_by_actor_source"],
        AGENT_ACTOR_SOURCE
    );
    assert_eq!(
        response.response_value["staged_artifact_handle"]["redaction_state"],
        "blocked"
    );
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present");
    let row = staged_artifact_row(&harness, handle_id)?;
    assert_eq!(row.created_by_actor_source, AGENT_ACTOR_SOURCE);
    Ok(())
}

#[test]
fn stage_artifact_rejects_caller_submitted_provenance_fields() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::to_value(stage_artifact_request(
        "req_stage_forged_provenance",
        Some("idem_stage_forged_provenance"),
        false,
        Some(2),
        "task_forged_provenance",
    ))?;
    value["created_by_actor_source"] = json!("forged_connection");
    value["created_by_actor_source"] = json!("forged_instance");

    let error = serde_json::from_value::<StageArtifactRequest>(value)
        .expect_err("caller-submitted provenance fields should be rejected");

    assert!(error.to_string().contains("created_by_actor_source"));
    Ok(())
}

#[test]
fn record_run_without_product_write_commits_run_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_no_write")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let response = harness.service.record_run(
        record_run_request(
            "req_run_no_write",
            "idem_run_no_write",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["run_summary"]["observed_changes"]["product_file_write_observed"],
        false
    );
    let run_id = run_id_from_record_run(&response.response_value);
    assert_eq!(run_scope_revision(&harness, &run_id)?, 1);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let after_revision = task_revision(&harness, &task_id)?;
    assert_eq!(
        after_revision.close_basis_revision,
        before_revision.close_basis_revision + 1
    );
    assert!(after_revision.current_close_basis.is_none());
    assert!(response.response_value["current_close_basis"].is_null());
    Ok(())
}

#[test]
fn record_run_non_null_close_assessment_creates_current_basis() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_basis")?;
    let generator = CountingDurableIdGenerator::new(["run_basis", "event_basis"]);
    let clock = ManualClock::at("2026-06-18T12:00:00Z");
    harness.use_generator_and_clock(generator, clock);

    let mut request = record_run_request(
        "req_run_basis",
        "idem_run_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(close_assessment_with_risks(
        "Recorded close basis.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;
    let basis = revision
        .current_close_basis
        .expect("current close basis should be stored");

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(basis.task_id.as_str(), task_id);
    assert_eq!(basis.change_unit_id.as_str(), change_unit_id);
    assert_eq!(basis.scope_revision, 1);
    assert_eq!(basis.close_basis_revision, revision.close_basis_revision);
    assert_eq!(basis.result_summary, "Recorded close basis.");
    assert!(basis.residual_risks.is_empty());
    assert_eq!(basis.updated_at.to_string(), "2026-06-18T12:00:00Z");
    assert_eq!(
        response.response_value["current_close_basis"]["residual_risks"],
        json!([])
    );
    assert!(
        response.response_value["current_close_basis"]["result_refs"]
            .as_array()
            .expect("result_refs should be present")
            .iter()
            .filter_map(|record_ref| record_ref["record_kind"].as_str())
            .any(|kind| kind == "run")
    );
    Ok(())
}

#[test]
fn current_compatible_run_ref_can_enter_close_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "current_run_ref")?;

    let mut first = record_run_request(
        "req_current_run_ref_first",
        "idem_current_run_ref_first",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    first.run_id = Some(RunId::new("run_current_ref_first")).into();
    let first_response = harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(first_response.response_value["base"]["state_version"], 3);

    let mut second = record_run_request(
        "req_current_run_ref_second",
        "idem_current_run_ref_second",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    second.run_id = Some(RunId::new("run_current_ref_second")).into();
    second.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Current prior Run can support this close basis.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_current_ref_first",
            PROJECT_ID,
            &task_id,
            Some(999),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(second, invocation(OperationCategory::AgentWorkflow))?;
    let basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current basis should be stored");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_current_ref_first"
            && record_ref.state_version.as_ref() == Some(&4)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_current_ref_second"
            && record_ref.state_version.as_ref() == Some(&4)
    }));
    Ok(())
}

#[test]
fn record_run_rejects_superseded_change_unit_run_ref_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "old_unit_run_ref")?;

    let mut old = record_run_request(
        "req_old_unit_run",
        "idem_old_unit_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    old.run_id = Some(RunId::new("run_old_unit")).into();
    harness
        .service
        .record_run(old, invocation(OperationCategory::AgentWorkflow))?;

    let replace = harness.service.update_scope(
        update_scope_request(
            "req_old_unit_replace",
            "idem_old_unit_replace",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_old_unit_rejected",
        "idem_old_unit_rejected",
        false,
        Some(4),
        &task_id,
        &replacement_change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Old unit Run must not become current.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_old_unit",
            PROJECT_ID,
            &task_id,
            Some(3),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_scope_revision_is_required_by_storage_constraint() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_scope_required")?;

    let mut request = record_run_request(
        "req_scope_required_run",
        "idem_scope_required_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_scope_required")).into();
    harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;

    let error = harness
        .conn()?
        .execute(
            "UPDATE runs
                SET scope_revision = NULL
              WHERE project_id = ?1
                AND run_id = 'run_scope_required'",
            rusqlite::params![PROJECT_ID],
        )
        .expect_err("runs.scope_revision is required");
    assert_constraint_error(error);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_baseline_incompatible_run_ref_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "baseline_run_ref")?;

    let mut baseline = record_run_request(
        "req_baseline_run",
        "idem_baseline_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    baseline.run_id = Some(RunId::new("run_baseline_mismatch")).into();
    harness
        .service
        .record_run(baseline, invocation(OperationCategory::AgentWorkflow))?;
    set_run_observed_baseline(&harness, "run_baseline_mismatch", "baseline_other")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_baseline_ref_rejected",
        "idem_baseline_ref_rejected",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Baseline-mismatched Run must not become current.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_baseline_mismatch",
            PROJECT_ID,
            &task_id,
            Some(3),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn historical_verified_artifact_reuse_requires_new_current_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "artifact_reuse")?;
    let (artifact_state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "artifact_reuse")?;
    let old_run_id = latest_run_id(&harness, &task_id)?;

    let replace = harness.service.update_scope(
        update_scope_request(
            "req_artifact_reuse_replace",
            "idem_artifact_reuse_replace",
            false,
            Some(artifact_state_version),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope for artifact reuse.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");

    let mut direct_old_run = record_run_request(
        "req_artifact_reuse_old_run",
        "idem_artifact_reuse_old_run",
        false,
        Some(artifact_state_version + 1),
        &task_id,
        &replacement_change_unit_id,
    );
    direct_old_run.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Old Run must not be reused directly.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            &old_run_id,
            PROJECT_ID,
            &task_id,
            Some(artifact_state_version),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_reject = harness.counts()?;
    let rejected = harness
        .service
        .record_run(direct_old_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before_reject);

    let mut current_reuse = record_run_request(
        "req_artifact_reuse_current",
        "idem_artifact_reuse_current",
        false,
        Some(artifact_state_version + 1),
        &task_id,
        &replacement_change_unit_id,
    );
    current_reuse.run_id = Some(RunId::new("run_artifact_reuse_current")).into();
    current_reuse.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_reuse_current",
        artifact_ref.clone(),
    )];
    current_reuse.evidence_updates = vec![supported_evidence_update(
        "Historical verified artifact reused by a current Run.",
    )];
    current_reuse.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Artifact reuse is recorded by a current Run.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Artifact,
            artifact_ref.artifact_id.as_str(),
            PROJECT_ID,
            &task_id,
            Some(artifact_state_version),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(current_reuse, invocation(OperationCategory::AgentWorkflow))?;
    let basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current basis should be stored");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        run_scope_revision(&harness, "run_artifact_reuse_current")?,
        2
    );
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_artifact_reuse_current"
    }));
    assert!(basis.result_refs.iter().all(|record_ref| {
        record_ref.record_kind != StateRecordKind::Run
            || record_ref.record_id.as_str() != old_run_id
    }));
    Ok(())
}

#[test]
fn record_run_state_includes_current_evidence_and_close_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_state_projection")?;
    let mut request = record_run_request(
        "req_run_state_projection",
        "idem_run_state_projection",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Close claim supported.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["state"]["evidence_summary"],
        response.response_value["evidence_summary"]
    );
    assert_eq!(response.response_value["state"]["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value["state"],
        "missing_final_acceptance",
    );
    assert!(response.response_value["state"]["close_blockers"]
        .as_array()
        .is_some_and(|blockers| !blockers.is_empty()));
    Ok(())
}

#[test]
fn record_run_generates_opaque_residual_risk_ids_on_commit() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_risks")?;
    let generator = CountingDurableIdGenerator::new(["risk_alpha", "risk_beta", "event_risks"]);
    let clock = ManualClock::at("2026-06-18T12:30:00Z");
    harness.use_generator_and_clock(generator.clone(), clock);

    let mut request = record_run_request(
        "req_run_risks",
        "idem_run_risks",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_risks_supplied")).into();
    request.close_assessment = Some(close_assessment_with_risks(
        "Recorded close basis with risks.",
        vec![
            residual_risk_input("First residual risk."),
            residual_risk_input("Second residual risk."),
        ],
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let risk_ids = response.response_value["current_close_basis"]["residual_risks"]
        .as_array()
        .expect("residual risks should be an array")
        .iter()
        .map(|risk| {
            risk["risk_id"]
                .as_str()
                .expect("risk id should be present")
                .to_owned()
        })
        .collect::<Vec<_>>();
    let (_, event_payload, _) = latest_task_event(&harness)?;

    assert_eq!(risk_ids, vec!["risk_risk_alpha", "risk_risk_beta"]);
    assert_eq!(generator.count(DurableIdKind::Risk), 2);
    assert_eq!(event_payload["residual_risk_ids"], json!(risk_ids));
    assert_eq!(
        event_payload["source_run_ref"]["record_id"],
        "run_risks_supplied"
    );
    assert_eq!(event_payload["scope_revision"], 1);
    assert_eq!(event_payload["close_basis_revision"], 2);
    Ok(())
}

#[test]
fn record_run_rejects_unsupported_close_basis_ref_kinds_without_effect(
) -> Result<(), Box<dyn Error>> {
    let unsupported = [
        (StateRecordKind::WriteCheck, "wa_fabricated"),
        (StateRecordKind::UserJudgment, "uj_fabricated"),
        (StateRecordKind::Blocker, "blocker_fabricated"),
        (StateRecordKind::TaskEvent, "evt_fabricated"),
        (StateRecordKind::ProjectState, "project_state_fabricated"),
        (StateRecordKind::Task, "task_fabricated"),
        (StateRecordKind::AgentConnection, "connection_fabricated"),
    ];

    for (index, (record_kind, record_id)) in unsupported.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("unsupported_ref_{index}"))?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_unsupported_ref_{index}"),
            &format!("idem_unsupported_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(volicord_types::CloseAssessmentInput {
            result_summary: "Unsupported refs must not enter close authority.".to_owned(),
            result_refs: vec![test_state_record_ref(
                record_kind,
                record_id,
                PROJECT_ID,
                &task_id,
                Some(999),
            )],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn record_run_rejects_nonexistent_allowed_close_basis_refs_without_effect(
) -> Result<(), Box<dyn Error>> {
    let allowed_but_missing = [
        (StateRecordKind::Run, "run_missing"),
        (StateRecordKind::Artifact, "artifact_missing"),
        (StateRecordKind::EvidenceSummary, "evidence_missing"),
        (StateRecordKind::ChangeUnit, "cu_missing"),
    ];

    for (index, (record_kind, record_id)) in allowed_but_missing.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("missing_ref_{index}"))?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_missing_ref_{index}"),
            &format!("idem_missing_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(volicord_types::CloseAssessmentInput {
            result_summary: "Missing allowed refs still need stored records.".to_owned(),
            result_refs: vec![test_state_record_ref(
                record_kind,
                record_id,
                PROJECT_ID,
                &task_id,
                Some(2),
            )],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn record_run_rejects_cross_project_artifact_and_cross_task_run_refs_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cross_refs")?;

    for (index, record_ref) in [
        test_state_record_ref(
            StateRecordKind::Artifact,
            "artifact_cross_project",
            "project_other",
            &task_id,
            Some(2),
        ),
        test_state_record_ref(
            StateRecordKind::Run,
            "run_cross_task",
            PROJECT_ID,
            "task_other",
            Some(2),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_cross_ref_{index}"),
            &format!("idem_cross_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.run_id = Some(RunId::new(format!("run_cross_ref_{index}"))).into();
        request.close_assessment = Some(volicord_types::CloseAssessmentInput {
            result_summary: "Cross-owner refs must not enter close authority.".to_owned(),
            result_refs: vec![record_ref],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn record_run_rejects_corrupt_artifact_close_basis_ref_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "corrupt_basis_artifact")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "corrupt_basis_artifact",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    set_artifact_integrity(
        &harness,
        &artifact_id,
        "corrupt",
        artifact_ref.content_type.as_deref(),
        artifact_ref.sha256.as_deref(),
        artifact_ref.size_bytes.as_ref().copied(),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_unverified_artifact_basis",
        "idem_unverified_artifact_basis",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Unverified artifact must not enter close authority.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Artifact,
            &artifact_id,
            PROJECT_ID,
            &task_id,
            Some(999),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_noncurrent_evidence_summary_close_basis_ref_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "noncurrent_evidence")?;
    let first_state =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "old_evidence", true)?;
    let old_evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let current_state = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        first_state,
        "new_evidence",
        true,
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_noncurrent_evidence_basis",
        "idem_noncurrent_evidence_basis",
        false,
        Some(current_state),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Old evidence summary must not enter current close authority.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::EvidenceSummary,
            &old_evidence_summary_id,
            PROJECT_ID,
            &task_id,
            Some(first_state),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_canonicalizes_deduplicates_and_adds_current_close_basis_refs(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_refs")?;
    let mut request = record_run_request(
        "req_canonical_refs",
        "idem_canonical_refs",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_canonical_refs")).into();
    request.evidence_updates = vec![supported_evidence_update("Canonical close basis claim.")];
    let future_run_ref = test_state_record_ref(
        StateRecordKind::Run,
        "run_canonical_refs",
        PROJECT_ID,
        &task_id,
        Some(999),
    );
    let past_run_ref = test_state_record_ref(
        StateRecordKind::Run,
        "run_canonical_refs",
        PROJECT_ID,
        &task_id,
        Some(1),
    );
    let mut risk = residual_risk_input("Caller-versioned risk source.");
    risk.acceptance_required = false;
    risk.source_refs = vec![future_run_ref.clone(), past_run_ref.clone()];
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Canonical refs are stored.".to_owned(),
        result_refs: vec![future_run_ref, past_run_ref],
        residual_risks: vec![risk],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;
    let basis = revision
        .current_close_basis
        .expect("current close basis should be stored");

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(basis.result_refs.len(), 3);
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_canonical_refs"
            && record_ref.state_version.as_ref() == Some(&3)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::ChangeUnit
            && record_ref.record_id.as_str() == change_unit_id
            && record_ref.state_version.as_ref() == Some(&3)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::EvidenceSummary
            && record_ref.state_version.as_ref() == Some(&3)
    }));
    assert_eq!(
        basis
            .evidence_summary_ref
            .as_ref()
            .and_then(|record_ref| record_ref.state_version.as_ref().copied()),
        Some(3)
    );
    assert_eq!(basis.residual_risks[0].source_refs.len(), 1);
    assert_eq!(
        basis.residual_risks[0].source_refs[0]
            .state_version
            .as_ref(),
        Some(&3)
    );
    Ok(())
}

#[test]
fn final_acceptance_judgment_basis_uses_canonical_close_basis_refs() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_final")?;
    let state_version = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "canonical_final",
        true,
    )?;
    let close_basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current close basis should be stored");

    let response = harness.service.request_user_judgment(
        user_judgment_request(
            "req_canonical_final",
            "idem_canonical_final",
            false,
            Some(state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["basis"]["result_refs"],
        serde_json::to_value(&close_basis.result_refs)?
    );
    assert!(close_basis
        .result_refs
        .iter()
        .all(|record_ref| record_ref.state_version.as_ref() == Some(&state_version)));
    Ok(())
}

#[test]
fn record_run_null_close_assessment_invalidates_existing_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_clear_basis")?;

    let mut establish = record_run_request(
        "req_run_establish_basis",
        "idem_run_establish_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    establish.close_assessment = Some(close_assessment_with_risks(
        "Established basis.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(establish, invocation(OperationCategory::AgentWorkflow))?;
    assert!(task_revision(&harness, &task_id)?
        .current_close_basis
        .is_some());

    let clear = record_run_request(
        "req_run_clear_basis",
        "idem_run_clear_basis",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    let response = harness
        .service
        .record_run(clear, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;

    assert!(response.response_value["current_close_basis"].is_null());
    assert_eq!(revision.close_basis_revision, 3);
    assert!(revision.current_close_basis.is_none());
    Ok(())
}

#[test]
fn record_run_dry_run_allocates_no_residual_risk_ids() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_dry_risk")?;
    let generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T13:00:00Z");
    harness.use_generator_and_clock(generator.clone(), clock);
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_dry_risk",
        "idem_run_dry_risk",
        true,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_dry_risk_supplied")).into();
    request.close_assessment = Some(close_assessment_with_risks(
        "Dry-run close basis.",
        vec![residual_risk_input("Dry-run residual risk.")],
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(generator.count(DurableIdKind::Risk), 0);
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    Ok(())
}

#[test]
fn record_run_product_write_consumes_valid_write_check_once() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_write")?;
    let write_check_id = prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_write")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_write",
        "idem_run_write",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_check_id = Some(WriteCheckId::new(&write_check_id)).into();
    request.evidence_updates = vec![supported_evidence_update(
        "Product write was reported with external tool output.",
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let run_id = run_id_from_record_run(&response.response_value);
    let observation_id = response.response_value["evidence_observations"][0]["observation_id"]
        .as_str()
        .expect("observation id should be present")
        .to_owned();
    let write_summary = &response.response_value["state"]["write_check_summary"];

    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(write_check_status(&harness, &write_check_id)?, "consumed");
    assert_eq!(write_summary["status"], "consumed");
    assert_eq!(write_summary["consumed_by_run_ref"]["record_id"], run_id);
    assert_eq!(
        write_summary["observation_refs"][0]["record_kind"],
        "evidence_observation"
    );
    assert_eq!(
        write_summary["observation_refs"][0]["record_id"],
        observation_id
    );
    assert_eq!(
        write_summary["guarantee_display"]["capability_refs"][0]["record_kind"],
        "agent_connection"
    );
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_run_write_status", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let mut response_write_summary =
        response.response_value["state"]["write_check_summary"].clone();
    let status_write_summary = status.response_value["write_check_summary"].clone();
    response_write_summary["guarantee_display"] = status_write_summary["guarantee_display"].clone();
    assert_eq!(status_write_summary, response_write_summary);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn record_run_consumes_write_check_at_fourteen_minutes_fifty_nine_seconds(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_1459")?;
    let id_generator =
        CountingDurableIdGenerator::new(["auth_1459", "prepare_event_1459", "record_event_1459"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator, clock.clone());
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_auth_1459")?;
    clock.advance(Duration::seconds(14 * 60 + 59));
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_1459",
            "idem_run_auth_1459",
            3,
            &task_id,
            &change_unit_id,
            &write_check_id,
            "run_auth_1459",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(write_check_status(&harness, &write_check_id)?, "consumed");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    Ok(())
}

#[test]
fn record_run_rejects_write_check_at_exactly_fifteen_minutes_without_effect(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_1500")?;
    let id_generator = CountingDurableIdGenerator::new(["auth_1500", "prepare_event_1500"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator, clock.clone());
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_auth_1500")?;
    clock.advance(Duration::minutes(15));
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_1500",
            "idem_run_auth_1500",
            3,
            &task_id,
            &change_unit_id,
            &write_check_id,
            "run_auth_1500",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_CHECK_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_check_reason"],
        "expired"
    );
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_limits_historical_far_future_write_check_to_fifteen_minutes(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_legacy")?;
    insert_active_write_check_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_legacy_future",
        2,
        "2026-06-18T00:00:00.000Z",
        "2999-01-01T00:00:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_legacy",
            "idem_run_auth_legacy",
            2,
            &task_id,
            &change_unit_id,
            "wa_legacy_future",
            "run_auth_legacy",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_check_reason"],
        "expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_honors_stored_expiration_earlier_than_fifteen_minutes() -> Result<(), Box<dyn Error>>
{
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_early_exp")?;
    insert_active_write_check_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_early_expiration",
        2,
        "2026-06-18T00:00:00.000Z",
        "2026-06-18T00:05:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:05:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_early_exp",
            "idem_run_auth_early_exp",
            2,
            &task_id,
            &change_unit_id,
            "wa_early_expiration",
            "run_auth_early_exp",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_check_reason"],
        "expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_treats_invalid_write_check_timestamp_as_corrupt_state() -> Result<(), Box<dyn Error>>
{
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_bad_time")?;
    insert_active_write_check_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_bad_timestamp",
        2,
        "not-a-timestamp",
        "2026-06-18T00:15:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_bad_time",
            "idem_run_auth_bad_time",
            2,
            &task_id,
            &change_unit_id,
            "wa_bad_timestamp",
            "run_auth_bad_time",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_store_rejection(&response, "MCP_UNAVAILABLE", "corrupt_stored_value");
    let details = &response.response_value["errors"][0]["details"]["owner_state_error"];
    assert_eq!(details["table"], "write_checks");
    assert_eq!(details["record_ref"], "wa_bad_timestamp");
    assert_eq!(details["logical_column"], "created_at");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_stale_basis_precedes_write_check_expiration() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_stale_exp")?;
    insert_active_write_check_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_stale_and_expired",
        2,
        "2026-06-18T00:00:00.000Z",
        "2999-01-01T00:00:00.000Z",
    )?;
    harness.service.update_scope(
        update_scope_request(
            "req_run_auth_stale_exp_touch",
            "idem_run_auth_stale_exp_touch",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_stale_exp",
            "idem_run_auth_stale_exp",
            3,
            &task_id,
            &change_unit_id,
            "wa_stale_and_expired",
            "run_auth_stale_exp",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_missing_write_check_rejects_product_write_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_missing_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_auth",
        "idem_run_missing_auth",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_CHECK_REQUIRED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_stale_write_check_basis_rejects_before_consumption() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stale_auth")?;
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_stale_auth")?;
    harness.service.update_scope(
        update_scope_request(
            "req_run_stale_auth_touch",
            "idem_run_stale_auth_touch",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stale_auth",
        "idem_run_stale_auth",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_check_id = Some(WriteCheckId::new(&write_check_id)).into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_consumed_write_check_reuse_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_reuse_auth")?;
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_reuse_auth")?;

    let mut first = record_run_request(
        "req_run_reuse_first",
        "idem_run_reuse_first",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    first.observed_changes.product_file_write_observed = true;
    first.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    first.write_check_id = Some(WriteCheckId::new(&write_check_id)).into();
    harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;

    let mut second = record_run_request(
        "req_run_reuse_second",
        "idem_run_reuse_second",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    second.observed_changes.product_file_write_observed = true;
    second.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    second.write_check_id = Some(WriteCheckId::new(&write_check_id)).into();
    let response = harness
        .service
        .record_run(second, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_CHECK_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_check_reason"],
        "consumed"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_path_mismatch_rejects_without_consuming_write_check() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_path_auth")?;
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_path_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_path_auth",
        "idem_run_path_auth",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["tests/export.rs".to_owned()];
    request.write_check_id = Some(WriteCheckId::new(&write_check_id)).into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_CHECK_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_check_reason"],
        "path_mismatch"
    );
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_check_baseline_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_baseline_auth")?;
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_baseline_auth")?;
    mutate_write_check_scope_json(&harness, &write_check_id, |scope| {
        scope["baseline_ref"] = json!("baseline_other");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_baseline_auth",
            "idem_run_baseline_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_check_id,
            "run_baseline_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_check_invalid_reason(&response, "baseline_mismatch");
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_check_task_mismatch_without_consumption() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_task_auth")?;
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_task_auth")?;
    mutate_write_check_scope_json(&harness, &write_check_id, |scope| {
        scope["task_id"] = json!("task_other");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_task_auth",
            "idem_run_task_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_check_id,
            "run_task_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_check_invalid_reason(&response, "task_mismatch");
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_check_change_unit_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_change_unit_auth")?;
    let write_check_id = prepare_write_check(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "run_change_unit_auth",
    )?;
    mutate_write_check_scope_json(&harness, &write_check_id, |scope| {
        scope["change_unit_id"] = json!("cu_other");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_change_unit_auth",
            "idem_run_change_unit_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_check_id,
            "run_change_unit_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_check_invalid_reason(&response, "change_unit_mismatch");
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_check_product_write_flag_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_flag_auth")?;
    let write_check_id =
        prepare_write_check(&harness, &task_id, &change_unit_id, 2, "run_flag_auth")?;
    mutate_write_check_scope_json(&harness, &write_check_id, |scope| {
        scope["product_file_write_intended"] = json!(false);
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_flag_auth",
            "idem_run_flag_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_check_id,
            "run_flag_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_check_invalid_reason(&response, "product_write_flag_mismatch");
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_check_sensitive_category_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_sensitive_auth")?;
    insert_active_write_check_with_scope(
        &harness,
        WriteCheckScopeFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            write_check_id: "wa_sensitive_mismatch",
            basis_state_version: 2,
            created_at: "2999-01-01T00:00:00.000Z",
            expires_at: "2999-01-01T00:15:00.000Z",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            sensitive_categories: &["network"],
        },
    )?;
    enable_record_run_capabilities(&harness)?;
    let before = harness.counts()?;
    let mut request = product_write_record_run_request(
        "req_run_sensitive_auth",
        "idem_run_sensitive_auth",
        2,
        &task_id,
        &change_unit_id,
        "wa_sensitive_mismatch",
        "run_sensitive_auth",
    );
    request.observed_changes.sensitive_categories = vec!["credential".to_owned()];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_write_check_invalid_reason(&response, "sensitive_category_mismatch");
    assert_eq!(
        write_check_status(&harness, "wa_sensitive_mismatch")?,
        "active"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_promotes_staged_artifact_and_updates_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_artifact", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let expected_content_type = handle.content_type.clone();
    let expected_sha256 = handle.sha256.clone();
    let expected_size_bytes = handle.size_bytes;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_artifact",
        "idem_run_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_report",
        handle,
        Some("validation_report"),
        Some("Search-result count validation passed."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Search-result count validation passed.",
    )];
    request.evidence_observations = vec![EvidenceObservationInput {
        claim: "Search-result count validation passed.".to_owned(),
        source_kind: EvidenceSourceKind::ExternalTool,
        assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
        observed_by_actor_source: None.into(),
        tool_name: Some("search-count-validator".to_owned()).into(),
        tool_invocation_id: None.into(),
        tool_metadata: Map::from_iter([("validator".to_owned(), json!("search-count"))]),
        input_refs: Vec::new(),
        output_artifact_refs: Vec::new(),
        limitations: vec!["External tool output is not product correctness proof.".to_owned()],
        observed_at: volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
    }];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();
    let observation = &response.response_value["evidence_observations"][0];
    let observation_id = observation["observation_id"]
        .as_str()
        .expect("observation id should be present")
        .to_owned();
    let artifact_row = persistent_artifact_row(&harness, &artifact_id)?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["content_type"],
        expected_content_type
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["sha256"],
        expected_sha256
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["size_bytes"],
        expected_size_bytes
    );
    assert_eq!(artifact_row.integrity_status, "verified");
    assert_eq!(
        artifact_row.content_type.as_deref(),
        Some(expected_content_type.as_str())
    );
    let body_path = artifact_row
        .body_path
        .as_deref()
        .expect("promoted artifact should store a body path");
    let staging_row = staged_artifact_row(&harness, &handle_id)?;
    assert!(
        body_path.starts_with("tmp/"),
        "persistent body_path should be artifact-store-relative: {body_path}"
    );
    assert!(
        !body_path.starts_with("artifacts/"),
        "persistent body_path must not include the project-home artifact prefix"
    );
    assert_eq!(staging_row.tmp_path, format!("artifacts/{body_path}"));
    assert_eq!(
        artifact_row.sha256.as_deref(),
        Some(expected_sha256.as_str())
    );
    assert_eq!(artifact_row.size_bytes, Some(expected_size_bytes));
    assert_eq!(artifact_row.status, "available");
    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["supporting_refs"][0]
            ["record_kind"],
        "run"
    );
    assert_eq!(observation["source_kind"], "external_tool");
    assert_eq!(observation["assurance_level"], "external_tool_result");
    assert_eq!(observation["observed_by_actor_source"], AGENT_ACTOR_SOURCE);
    assert_eq!(observation["tool_metadata"]["validator"], "search-count");
    assert_eq!(
        observation["output_artifact_refs"][0]["artifact_id"],
        artifact_id
    );
    assert!(
        observation_id.starts_with("evidence_observation_"),
        "generated observation id should use the durable prefix: {observation_id}"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            ["record_kind"],
        "evidence_observation"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            ["record_id"],
        observation_id
    );
    assert_eq!(
        response.response_value["evidence_summary"]["observation_refs"][0]["record_id"],
        observation_id
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.artifacts, before.artifacts + 1);
    assert_eq!(after.artifact_links, before.artifact_links + 3);
    assert_eq!(after.evidence_summaries, before.evidence_summaries + 1);
    assert_eq!(
        after.evidence_observations,
        before.evidence_observations + 1
    );
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "consumed");
    assert!(artifact_owner_link_exists(&harness, &artifact_id, "run")?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_summary"
    )?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_observation"
    )?);
    Ok(())
}

#[test]
fn record_run_observations_preserve_provenance_classification() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_observation_classes")?;
    let classes = [
        (
            "Agent cooperative report.",
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
            "agent_report",
            "cooperative_report",
        ),
        (
            "Registered connection observation.",
            EvidenceSourceKind::ConnectionObservation,
            EvidenceAssuranceLevel::RegisteredConnectionObserved,
            "connection_observation",
            "registered_connection_observed",
        ),
        (
            "External tool result.",
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
            "external_tool",
            "external_tool_result",
        ),
        (
            "User observation.",
            EvidenceSourceKind::UserObservation,
            EvidenceAssuranceLevel::UserObserved,
            "user_observation",
            "user_observed",
        ),
        (
            "Unverified claim.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
            "unverified_claim",
            "unverified",
        ),
    ];
    let mut request = record_run_request(
        "req_run_observation_classes",
        "idem_run_observation_classes",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = classes
        .iter()
        .map(|(claim, source_kind, assurance_level, _, _)| {
            supported_evidence_update_with_provenance(claim, *source_kind, *assurance_level)
        })
        .collect();
    request.evidence_observations = classes
        .iter()
        .map(
            |(claim, source_kind, assurance_level, _, _)| EvidenceObservationInput {
                claim: (*claim).to_owned(),
                source_kind: *source_kind,
                assurance_level: *assurance_level,
                observed_by_actor_source: None.into(),
                tool_name: Some("fixture-evidence-check".to_owned()).into(),
                tool_invocation_id: None.into(),
                tool_metadata: JsonObject::new(),
                input_refs: Vec::new(),
                output_artifact_refs: Vec::new(),
                limitations: Vec::new(),
                observed_at: volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")
                    .expect("fixture timestamp should parse"),
            },
        )
        .collect();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let observations = response.response_value["evidence_observations"]
        .as_array()
        .expect("evidence observations should be present");

    assert_eq!(observations.len(), classes.len());
    for (observation, (_, _, _, source_value, assurance_value)) in observations.iter().zip(classes)
    {
        assert_eq!(observation["source_kind"], source_value);
        assert_eq!(observation["assurance_level"], assurance_value);
        assert!(observation.get("guarantee_display").is_none());
    }
    Ok(())
}

#[test]
fn record_run_rejects_supported_evidence_without_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_missing_provenance")?;
    let mut request = record_run_request(
        "req_run_missing_provenance",
        "idem_run_missing_provenance",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let mut evidence_update = supported_evidence_update("Claim without provenance.");
    evidence_update.provenance = None;
    request.evidence_updates = vec![evidence_update];
    let before = harness.counts()?;

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].provenance"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_promotes_zero_byte_artifact_with_real_empty_sha256() -> Result<(), Box<dyn Error>> {
    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_zero_artifact")?;
    let mut stage_request = stage_artifact_request(
        "req_stage_zero_artifact",
        Some("idem_stage_zero_artifact"),
        false,
        Some(2),
        &task_id,
    );
    stage_request.safe_bytes_or_notice = String::new();
    stage_request.expected_sha256 = Some(EMPTY_SHA256.to_owned()).into();
    stage_request.expected_size_bytes = Some(0).into();
    let stage_response = harness
        .service
        .stage_artifact(stage_request, invocation(OperationCategory::AgentWorkflow))?;
    let handle: StagedArtifactHandle =
        serde_json::from_value(stage_response.response_value["staged_artifact_handle"].clone())?;

    let mut request = record_run_request(
        "req_run_zero_artifact",
        "idem_run_zero_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_zero",
        handle,
        Some("empty_report"),
        Some("Zero-byte artifact was registered."),
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present");
    let artifact_row = persistent_artifact_row(&harness, artifact_id)?;

    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["sha256"],
        EMPTY_SHA256
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["size_bytes"],
        0
    );
    assert_eq!(artifact_row.integrity_status, "verified");
    assert_eq!(artifact_row.sha256.as_deref(), Some(EMPTY_SHA256));
    assert_eq!(artifact_row.size_bytes, Some(0));
    Ok(())
}

#[test]
fn corrupt_artifact_blocks_evidence_and_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "corrupt_evidence_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "corrupt_evidence_artifact", 2)?;

    let mut request = record_run_request(
        "req_run_corrupt_evidence_artifact",
        "idem_run_corrupt_evidence_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_corrupt",
        handle,
        Some("validation_report"),
        Some("Corrupt integrity evidence."),
    )];
    request.evidence_updates = vec![supported_evidence_update("Corrupt integrity evidence.")];
    request.close_assessment = Some(close_assessment_with_risks(
        "Corrupt integrity evidence.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();

    set_artifact_integrity(&harness, &artifact_id, "corrupt", None, None, None)?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_corrupt_evidence_artifact",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let artifact_ref = &status.response_value["evidence_summary"]["coverage_items"][0]
        ["supporting_artifact_refs"][0];

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "blocked"
    );
    assert_eq!(artifact_ref["availability"], "integrity_failed");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert!(artifact_ref["content_type"].is_null());
    assert!(artifact_ref["sha256"].is_null());
    assert!(artifact_ref["size_bytes"].is_null());
    assert_close_blocker(&status.response_value, "artifact_unavailable");

    let check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_corrupt_evidence_artifact",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn corrupt_artifact_is_not_linkable_as_existing_artifact() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "corrupt_artifact")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "corrupt")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let before = harness.counts()?;
    set_artifact_integrity(
        &harness,
        &artifact_id,
        "corrupt",
        artifact_ref.content_type.as_ref().map(String::as_str),
        artifact_ref.sha256.as_ref().map(String::as_str),
        artifact_ref.size_bytes.as_ref().copied(),
    )?;

    let mut request = record_run_request(
        "req_run_corrupt_existing",
        "idem_run_corrupt_existing",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_corrupt_existing",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "ARTIFACT_MISSING"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn verified_existing_artifact_ref_missing_integrity_fact_is_rejected() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "missing_ref_fact")?;
    let (state_version, mut artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "missing_ref")?;
    artifact_ref.sha256 = RequiredNullable::null();
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_existing_ref_fact",
        "idem_run_missing_existing_ref_fact",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_missing_existing_ref_fact",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn missing_persistent_artifact_body_blocks_evidence_and_close_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "missing_body")?;
    let before_counts = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, fixture.artifact_id())?;

    fs::remove_file(&fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "blocked"
    );
    assert_eq!(artifact_ref["availability"], "missing");
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&status, &harness.runtime_home_path);

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&check, &harness.runtime_home_path);
    assert_eq!(harness.counts()?, before_counts);
    assert_eq!(
        persistent_artifact_row(&harness, fixture.artifact_id())?,
        before_row
    );
    Ok(())
}

#[test]
fn modified_persistent_artifact_body_blocks_existing_link_before_write_check(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "modified_existing")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "modified_existing",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let write_check_id = prepare_write_check(
        &harness,
        &task_id,
        &change_unit_id,
        state_version,
        "modified_existing",
    )?;
    let before = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, &artifact_id)?;
    let body_path = persistent_artifact_body_path(&harness, &artifact_id)?;
    fs::write(&body_path, b"{\"fixture\":\"changed_bytes\"}")?;

    let mut request = product_write_record_run_request(
        "req_run_modified_existing",
        "idem_run_modified_existing",
        state_version + 1,
        &task_id,
        &change_unit_id,
        &write_check_id,
        "run_modified_existing",
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_modified_existing",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "ARTIFACT_MISSING"
    );
    assert_eq!(write_check_status(&harness, &write_check_id)?, "active");
    assert_eq!(harness.counts()?, before);
    assert_eq!(persistent_artifact_row(&harness, &artifact_id)?, before_row);
    assert_public_response_has_no_internal_leak(&response, &harness.runtime_home_path);
    Ok(())
}

#[test]
fn changed_persistent_artifact_body_blocks_evidence_and_close_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "changed_body")?;
    let before_counts = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, fixture.artifact_id())?;

    fs::write(&fixture.body_path, b"{\"fixture\":\"changed\"}")?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "blocked"
    );
    assert_eq!(artifact_ref["availability"], "integrity_failed");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert_close_blocker(&status.response_value, "artifact_unavailable");

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_eq!(harness.counts()?, before_counts);
    assert_eq!(
        persistent_artifact_row(&harness, fixture.artifact_id())?,
        before_row
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_escape_persistent_artifact_body_is_unusable_without_path_leak(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "symlink_escape")?;
    let before_counts = harness.counts()?;
    let outside_path = harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join("outside-artifact-store.json");
    fs::write(&outside_path, b"{\"fixture\":\"symlink_escape\"}")?;
    fs::remove_file(&fixture.body_path)?;
    std::os::unix::fs::symlink(&outside_path, &fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(artifact_ref["availability"], "unusable");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&status, &harness.runtime_home_path);

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&check, &harness.runtime_home_path);
    assert_eq!(harness.counts()?, before_counts);
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_within_artifact_store_keeps_persistent_artifact_usable() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "symlink_inside")?;
    let original_bytes = fs::read(&fixture.body_path)?;
    let inside_target = fixture
        .body_path
        .parent()
        .expect("artifact body has parent")
        .join("symlink-inside-target.json");
    fs::write(&inside_target, original_bytes)?;
    fs::remove_file(&fixture.body_path)?;
    std::os::unix::fs::symlink(&inside_target, &fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(artifact_ref["availability"], "available");
    assert_eq!(artifact_ref["integrity_status"], "verified");
    assert_no_close_blocker(&status.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn record_run_staged_artifact_actor_source_mismatch_rejects_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_source")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_source", 2)?;
    handle.created_by_actor_source = ActorSource::agent_connection("forged_connection");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_source",
        "idem_run_stage_source",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_source",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_actor_source_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_expired_staged_artifact_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_expired")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_expired", 2)?;
    expire_staged_artifact(&harness, handle.handle_id.as_str())?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_expired",
        "idem_run_stage_expired",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_expired",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_staged_artifact_uses_semantic_expiry_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_boundary")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_boundary", 2)?;
    clock.advance(Duration::seconds(24 * 60 * 60 - 1));

    let mut request = record_run_request(
        "req_run_stage_boundary_before",
        "idem_run_stage_boundary_before",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_boundary_before",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");

    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_boundary_exact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_boundary_exact", 2)?;
    clock.advance(Duration::hours(24));
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_boundary_exact",
        "idem_run_stage_boundary_exact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_boundary_exact",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_staged_artifact_accepts_equivalent_offset_expiration() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_offset")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_offset", 2)?;
    handle.expires_at = volicord_types::UtcTimestamp::parse("2026-06-19T09:00:00+09:00")?;

    let mut request = record_run_request(
        "req_run_stage_offset",
        "idem_run_stage_offset",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_offset",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    Ok(())
}

#[test]
fn record_run_invalid_stored_staged_artifact_expiration_is_corrupt_state(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_bad_expires")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_bad_expires", 2)?;
    set_staged_artifact_expires_at(&harness, handle.handle_id.as_str(), "tomorrow")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_bad_expires",
        "idem_run_stage_bad_expires",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_bad_expires",
        handle.clone(),
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_value_rejection(
        &response,
        "artifact_staging",
        handle.handle_id.as_str(),
        "expires_at",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        artifact_staging_status(&harness, handle.handle_id.as_str())?,
        "staged"
    );
    Ok(())
}

#[test]
fn record_run_checksum_mismatch_rejects_and_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut input = artifact_input_for_handle("artifact_input_sha", handle, None, None);
    input.expected_sha256 =
        Some("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).into();
    let mut request = record_run_request(
        "req_run_stage_sha",
        "idem_run_stage_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![input];
    request.close_assessment = Some(close_assessment_with_risks(
        "Rejected close basis.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_checksum_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_body_checksum_mismatch_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_body_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    fs::write(
        staged_artifact_body_path(&harness, &handle_id)?,
        vec![b'x'; handle.size_bytes as usize],
    )?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_sha",
        "idem_run_body_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_sha",
        handle,
        Some("validation_report"),
        Some("Tampered body should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Tampered body should not promote.",
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Tampered body should not promote.",
        Vec::new(),
    ))
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_body_size_mismatch_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_body_size")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_size", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    fs::write(
        staged_artifact_body_path(&harness, &handle_id)?,
        vec![b'x'; handle.size_bytes as usize + 1],
    )?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_size",
        "idem_run_body_size",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_size",
        handle,
        Some("validation_report"),
        Some("Resized body should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Resized body should not promote.",
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Resized body should not promote.",
        Vec::new(),
    ))
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_staging_path_outside_artifact_store_rolls_back_all_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_body_path_outside")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_path_outside", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    set_artifact_staging_tmp_path(&harness, &handle_id, "tmp/not-under-artifacts.txt")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_path_outside",
        "idem_run_body_path_outside",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_path_outside",
        handle,
        Some("validation_report"),
        Some("Invalid staging path should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Invalid staging path should not promote.",
    )];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_dry_run_and_idempotency_replay_have_no_extra_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_replay")?;
    let before_dry = harness.counts()?;
    let dry_run = harness.service.record_run(
        record_run_request(
            "req_run_dry",
            "idem_run_dry",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(harness.counts()?, before_dry);

    let request = record_run_request(
        "req_run_replay",
        "idem_run_replay",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let first = harness.service.record_run(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn request_user_judgment_creates_pending_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "pending")?;
    let before = harness.counts()?;

    let response = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_pending",
            "idem_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let judgment_id = response_record_id(&response.response_value, "user_judgment_ref");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["user_judgment"]["status"],
        "pending"
    );
    assert_eq!(
        response.response_value["user_judgment"]["judgment_kind"],
        "product_decision"
    );
    assert_eq!(
        response.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments + 1);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn authority_bearing_judgment_generates_canonical_options() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_options")?;

    let response = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_canonical_options",
            "idem_judgment_canonical_options",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let options = response.response_value["user_judgment"]["options"]
        .as_array()
        .expect("options should be an array");
    assert_eq!(options.len(), 3);
    assert_eq!(options[0]["option_id"], "accept");
    assert_eq!(options[0]["machine_action"], "accept");
    assert_eq!(options[0]["resolution_outcome"], "accepted");
    assert_eq!(options[1]["option_id"], "reject");
    assert_eq!(options[1]["machine_action"], "reject");
    assert_eq!(options[1]["resolution_outcome"], "rejected");
    assert_eq!(options[2]["option_id"], "defer");
    assert_eq!(options[2]["machine_action"], "defer");
    assert_eq!(options[2]["resolution_outcome"], "deferred");
    Ok(())
}

#[test]
fn authority_option_locale_changes_display_only() -> Result<(), Box<dyn Error>> {
    let english_harness = MethodHarness::new()?;
    let (english_task_id, english_change_unit_id) =
        create_task_with_change_unit(&english_harness, "locale_en")?;
    let english = english_harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_locale_en",
            "idem_judgment_locale_en",
            false,
            Some(2),
            &english_task_id,
            Some(&english_change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let korean_harness = MethodHarness::new()?;
    let (korean_task_id, korean_change_unit_id) =
        create_task_with_change_unit(&korean_harness, "locale_ko")?;
    let mut korean_request = user_judgment_request(
        "req_judgment_locale_ko",
        "idem_judgment_locale_ko",
        false,
        Some(2),
        &korean_task_id,
        Some(&korean_change_unit_id),
        JudgmentKind::Cancellation,
    );
    korean_request.envelope.locale = Some("ko-KR".to_owned()).into();
    let korean = korean_harness
        .service
        .request_user_judgment(korean_request, invocation(OperationCategory::AgentWorkflow))?;

    let english_accept = &english.response_value["user_judgment"]["options"][0];
    let korean_accept = &korean.response_value["user_judgment"]["options"][0];
    assert_ne!(english_accept["label"], korean_accept["label"]);
    assert_eq!(english_accept["option_id"], korean_accept["option_id"]);
    assert_eq!(
        english_accept["machine_action"],
        korean_accept["machine_action"]
    );
    assert_eq!(
        english_accept["resolution_outcome"],
        korean_accept["resolution_outcome"]
    );

    let fallback_harness = MethodHarness::new()?;
    let (fallback_task_id, fallback_change_unit_id) =
        create_task_with_change_unit(&fallback_harness, "locale_fallback")?;
    let mut fallback_request = user_judgment_request(
        "req_judgment_locale_fallback",
        "idem_judgment_locale_fallback",
        false,
        Some(2),
        &fallback_task_id,
        Some(&fallback_change_unit_id),
        JudgmentKind::Cancellation,
    );
    fallback_request.envelope.locale = Some("zz-ZZ".to_owned()).into();
    let fallback = fallback_harness.service.request_user_judgment(
        fallback_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        english_accept["label"],
        fallback.response_value["user_judgment"]["options"][0]["label"]
    );
    Ok(())
}

#[test]
fn authority_bearing_judgment_request_rejects_caller_options() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "authority_options")?;
    let mut request = user_judgment_request(
        "req_judgment_authority_options",
        "idem_judgment_authority_options",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::Cancellation,
    );
    request.options = Some(vec![volicord_types::UserJudgmentOptionInput {
        option_id: volicord_types::UserJudgmentOptionId::new("reject_visible_accept"),
        label: "Reject".to_owned(),
        description: "Caller-authored authority options are not accepted.".to_owned(),
        consequence: "Core must generate the authority option set.".to_owned(),
        is_default: false,
    }])
    .into();
    let before = harness.counts()?;

    let response = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_user_judgment_resolves_pending_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "resolve")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_resolve",
            "idem_judgment_resolve",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_resolve",
            "idem_record_resolve",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(
        response.response_value["user_judgment"]["status"],
        "resolved"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolved_by_actor_source"],
        LOCAL_USER_ACTOR_SOURCE
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["rationale"]["summary"],
        "The user selected the focused judgment option."
    );
    assert_eq!(
        response.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        0
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    assert!(
        resolution_json(&harness, &pending_judgment_id)?["answer"]["product_decision"].is_object()
    );
    assert_eq!(
        resolution_json(&harness, &pending_judgment_id)?["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        resolution_rationale_json(&harness, &pending_judgment_id)?["summary"],
        response.response_value["user_judgment"]["resolution"]["rationale"]["summary"]
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    assert_eq!(
        user_judgment_actor_provenance(&harness, &pending_judgment_id)?,
        UserJudgmentActorProvenance {
            resolved_by_actor_source: Some(LOCAL_USER_ACTOR_SOURCE.to_owned()),
            resolved_verification_basis: Some(VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned()),
            resolved_assurance_level: Some("local_user_channel".to_owned()),
        }
    );
    let (event_kind, event_payload, _) = latest_task_event(&harness)?;
    assert_eq!(event_kind, "user_judgment_recorded");
    assert_eq!(event_payload["resolution_outcome"], "accepted");
    Ok(())
}

#[test]
fn record_user_judgment_persists_authority_accept_action() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "accept_action")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_accept_action",
            "idem_judgment_accept_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_accept_action",
            "idem_record_accept_action",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::Cancellation,
            answer_payload(JudgmentKind::Cancellation),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["machine_action"],
        "accept"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        resolution_json(&harness, &pending_judgment_id)?["machine_action"],
        "accept"
    );
    assert_eq!(
        user_judgment_resolution_machine_action(&harness, &pending_judgment_id)?,
        Some("accept".to_owned())
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn accepted_decision_judgments_create_project_continuity_records() -> Result<(), Box<dyn Error>> {
    for (suffix, judgment_kind, title_prefix) in [
        (
            "product_continuity",
            JudgmentKind::ProductDecision,
            "Product decision:",
        ),
        (
            "technical_continuity",
            JudgmentKind::TechnicalDecision,
            "Technical decision:",
        ),
        (
            "scope_continuity",
            JudgmentKind::ScopeDecision,
            "Scope decision:",
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                &format!("req_judgment_{suffix}"),
                &format!("idem_judgment_{suffix}"),
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                judgment_kind,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                &format!("req_record_{suffix}"),
                &format!("idem_record_{suffix}"),
                Some(3),
                &task_id,
                &pending_judgment_id,
                judgment_kind,
                answer_payload(judgment_kind),
            ),
            invocation(OperationCategory::UserOnly),
        )?;

        let after = harness.counts()?;
        let rows = harness.continuity_records()?;
        assert_eq!(
            after.project_continuity_records,
            before.project_continuity_records + 1
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "decision");
        assert_eq!(rows[0].status, "active");
        assert_eq!(rows[0].source_task_id, task_id);
        assert_eq!(
            rows[0].source_change_unit_id.as_deref(),
            Some(change_unit_id.as_str())
        );
        assert!(rows[0].title.starts_with(title_prefix));
        assert!(rows[0].source_refs_json.contains(&pending_judgment_id));
        assert!(response.response_value["updated_refs"]
            .as_array()
            .expect("updated_refs should be an array")
            .iter()
            .any(|record_ref| record_ref["record_kind"] == "project_continuity_record"));
    }
    Ok(())
}

#[test]
fn accepted_residual_risk_creates_project_continuity_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_continuity")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "continuity",
        vec![residual_risk_input(
            "Visible residual risk needing acceptance.",
        )],
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_continuity",
            "idem_risk_continuity",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_continuity_record",
            "idem_risk_continuity_record",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let after = harness.counts()?;
    let rows = harness.continuity_records()?;
    assert_eq!(
        after.project_continuity_records,
        before.project_continuity_records + 1
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "accepted_risk");
    assert_eq!(rows[0].summary, "Visible residual risk needing acceptance.");
    assert!(rows[0].title.starts_with("Accepted residual risk:"));
    assert!(rows[0].source_refs_json.contains(&pending_judgment_id));
    assert!(response.response_value["updated_refs"]
        .as_array()
        .expect("updated_refs should be an array")
        .iter()
        .any(|record_ref| record_ref["record_kind"] == "project_continuity_record"));
    Ok(())
}

#[test]
fn status_continuity_summary_is_include_gated() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_continuity")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_status_continuity_judgment",
            "idem_status_continuity_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_status_continuity_record",
            "idem_status_continuity_record",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::TechnicalDecision,
            answer_payload(JudgmentKind::TechnicalDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let hidden = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_continuity_hidden",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                continuity: false,
                ..status_include()
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_field_absent(&hidden.response_value, "continuity_summary");

    let shown = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_continuity_shown",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                continuity: true,
                ..status_include()
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let summary = shown.response_value["continuity_summary"]
        .as_array()
        .expect("continuity_summary should be an array");
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0]["kind"], "decision");
    assert_eq!(summary[0]["status"], "active");
    assert!(summary[0]["continuity_record_ref"].is_object());
    Ok(())
}

#[test]
fn stale_judgment_does_not_create_project_continuity_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "stale_continuity")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_stale_continuity_judgment",
            "idem_stale_continuity_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    harness.service.update_scope(
        update_scope_request(
            "req_stale_continuity_scope",
            "idem_stale_continuity_scope",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "stale_continuity_scope",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_stale_continuity_record",
            "idem_stale_continuity_record",
            Some(4),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert!(harness.continuity_records()?.is_empty());
    Ok(())
}

#[test]
fn close_completion_creates_known_limit_continuity_and_preserves_records(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_continuity")?;
    let (after_basis, _) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "known_limit",
        vec![volicord_types::ResidualRiskInput {
            summary: "Known limitation that does not require acceptance.".to_owned(),
            consequence: "Future related work should remember this limitation.".to_owned(),
            acceptance_required: false,
            source_refs: Vec::new(),
        }],
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "known_limit",
    )?;
    let before_close = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_continuity_complete",
            idempotency_key: Some("idem_close_continuity_complete"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let after_close = harness.counts()?;
    let rows = harness.continuity_records()?;
    assert_eq!(response.response_value["close_state"], "closed");
    let response_continuity = response.response_value["continuity_summary"]
        .as_array()
        .expect("continuity_summary should be an array");
    assert_eq!(response_continuity.len(), 1);
    assert_eq!(response_continuity[0]["kind"], "known_limit");
    assert_eq!(response_continuity[0]["status"], "active");
    assert_eq!(
        response_continuity[0]["source_task_ref"]["record_id"],
        task_id
    );
    assert_eq!(
        after_close.project_continuity_records,
        before_close.project_continuity_records + 1
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "known_limit");
    assert_eq!(rows[0].status, "active");
    assert_eq!(rows[0].source_task_id, task_id);
    assert_eq!(
        rows[0].source_change_unit_id.as_deref(),
        Some(change_unit_id.as_str())
    );
    assert_eq!(
        rows[0].summary,
        "Known limitation that does not require acceptance."
    );
    Ok(())
}

#[test]
fn accepted_authority_judgment_requires_structured_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "accepted_rationale_required")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_rationale_required",
            "idem_judgment_rationale_required",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_rationale_required",
        "idem_record_rationale_required",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::Cancellation,
        answer_payload(JudgmentKind::Cancellation),
    );
    request.rationale.selected_reason = None.into();
    request.rationale.tradeoffs.clear();
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "rationale.selected_reason"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn record_user_judgment_persists_rejected_option_outcome() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "reject_outcome")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_reject_outcome",
            "idem_judgment_reject_outcome",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_reject_outcome",
        "idem_record_reject_outcome",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::Cancellation,
        cancellation_payload_with_decision("rejected"),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_ne!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(
        user_judgment_resolution_machine_action(&harness, &pending_judgment_id)?,
        Some("reject".to_owned())
    );
    assert_eq!(
        resolution_json(&harness, &pending_judgment_id)?["resolution_outcome"],
        "rejected"
    );
    assert_eq!(response.response_value["state"]["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value["state"],
        "missing_current_close_basis",
    );
    let (event_kind, event_payload, _) = latest_task_event(&harness)?;
    assert_eq!(event_kind, "user_judgment_recorded");
    assert_eq!(event_payload["resolution_outcome"], "rejected");
    Ok(())
}

#[test]
fn rejected_authority_judgment_accepts_concise_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "rejected_concise_rationale")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_concise_rationale",
            "idem_judgment_concise_rationale",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_concise_rationale",
        "idem_record_concise_rationale",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::Cancellation,
        cancellation_payload_with_decision("rejected"),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    request.rationale = JudgmentRationale {
        summary: "The user declined cancellation for now.".to_owned(),
        selected_reason: None.into(),
        considered_alternatives: Vec::new(),
        rejected_alternatives: Vec::new(),
        assumptions: Vec::new(),
        tradeoffs: Vec::new(),
        uncertainties: Vec::new(),
        review_triggers: Vec::new(),
        related_refs: Vec::new(),
        artifact_refs: Vec::new(),
    };

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "rejected"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["rationale"]["summary"],
        "The user declined cancellation for now."
    );
    assert_eq!(
        resolution_rationale_json(&harness, &pending_judgment_id)?["summary"],
        "The user declined cancellation for now."
    );
    Ok(())
}

#[test]
fn resolved_judgment_without_machine_action_is_owner_state_corruption() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "legacy_no_action")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "legacy_no_action",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_legacy_no_action_judgment",
            "idem_legacy_no_action_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(
        &harness,
        &judgment_id,
        Some(
            &json!({
                "selected_option_id": "accept",
                "answer": {
                    "product_decision": null,
                    "technical_decision": null,
                    "scope_decision": null,
                    "sensitive_action_scope": null,
                    "final_acceptance": { "judgment": { "decision": "accepted" } },
                    "residual_risk_acceptance": null,
                    "cancellation": null
                },
                "note": null,
                "accepted_risks": [],
                "resolved_by_actor_source": "user"
            })
            .to_string(),
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_legacy_no_action_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_machine_action",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_judgment_null_action_column_with_json_action_is_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("null_action_column", |harness, judgment_id| {
        set_user_judgment_resolution_machine_action(harness, judgment_id, None)
    })
}

#[test]
fn stored_judgment_action_column_with_missing_json_action_is_corrupt() -> Result<(), Box<dyn Error>>
{
    assert_final_acceptance_action_corruption_with(
        "missing_json_action",
        "resolution_json",
        "corrupt_stored_json",
        |harness, judgment_id| {
            let mut resolution = resolution_json(harness, judgment_id)?;
            resolution
                .as_object_mut()
                .expect("resolution JSON should be an object")
                .remove("machine_action");
            set_user_judgment_resolution_json_only_value(harness, judgment_id, &resolution)
        },
    )
}

#[test]
fn stored_judgment_differing_action_values_are_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("differing_action", |harness, judgment_id| {
        let mut resolution = resolution_json(harness, judgment_id)?;
        resolution["machine_action"] = json!("reject");
        set_user_judgment_resolution_json_only_value(harness, judgment_id, &resolution)
    })
}

#[test]
fn stored_judgment_action_outcome_mismatch_is_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("action_outcome_mismatch", |harness, judgment_id| {
        set_user_judgment_resolution_machine_action(harness, judgment_id, Some("reject"))?;
        let mut resolution = resolution_json(harness, judgment_id)?;
        resolution["machine_action"] = json!("reject");
        set_user_judgment_resolution_json_value(harness, judgment_id, &resolution)
    })
}

#[test]
fn stored_judgment_unsupported_action_string_is_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("unsupported_action", |harness, judgment_id| {
        set_user_judgment_resolution_machine_action_raw(harness, judgment_id, Some("approve"))
    })
}

#[test]
fn non_authority_custom_options_remain_usable_without_outcome_input() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "custom_option")?;
    let mut request = user_judgment_request(
        "req_judgment_custom_option",
        "idem_judgment_custom_option",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.options = Some(vec![volicord_types::UserJudgmentOptionInput {
        option_id: volicord_types::UserJudgmentOptionId::new("reject_like_custom_id"),
        label: "Use the alternate copy".to_owned(),
        description: "Record the user's product choice without caller-defined authority."
            .to_owned(),
        consequence: "The selected custom option is recorded for this product decision.".to_owned(),
        is_default: true,
    }])
    .into();
    let pending_judgment = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    assert_eq!(
        pending_judgment.response_value["user_judgment"]["options"][0]["option_id"],
        "reject_like_custom_id"
    );
    assert_eq!(
        pending_judgment.response_value["user_judgment"]["options"][0]["machine_action"],
        "accept"
    );
    assert_eq!(
        pending_judgment.response_value["user_judgment"]["options"][0]["resolution_outcome"],
        "accepted"
    );

    let mut record_request = record_judgment_request(
        "req_record_custom_option",
        "idem_record_custom_option",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_request.selected_option_id =
        volicord_types::UserJudgmentOptionId::new("reject_like_custom_id");

    let response = harness
        .service
        .record_user_judgment(record_request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["machine_action"],
        "accept"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn record_user_judgment_rejects_answer_outcome_contradicting_option() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "outcome_conflict")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_outcome_conflict",
            "idem_judgment_outcome_conflict",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_outcome_conflict",
        "idem_record_outcome_conflict",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ScopeDecision,
        answer_payload(JudgmentKind::ScopeDecision),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        None
    );
    Ok(())
}

#[test]
fn record_user_judgment_rejects_blocked_answer_outcome() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "blocked_outcome")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_blocked_outcome",
            "idem_judgment_blocked_outcome",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_blocked_outcome",
        "idem_record_blocked_outcome",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ScopeDecision,
        scope_decision_payload("blocked"),
    );
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        None
    );
    Ok(())
}

#[test]
fn non_user_actor_cannot_resolve_authority_bearing_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "authority_actor")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "authority_actor",
        true,
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_authority_actor",
            "idem_judgment_authority_actor",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_authority_actor",
        "idem_record_authority_actor",
        Some(after_basis + 1),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::FinalAcceptance,
        answer_payload(JudgmentKind::FinalAcceptance),
    );
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn local_user_can_resolve_authority_bearing_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "authority_role")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_authority_role",
            "idem_judgment_authority_role",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_authority_role",
            "idem_record_authority_role",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?.state_version, before.state_version + 1);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    Ok(())
}

#[test]
fn local_user_can_resolve_non_authority_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "agent_non_authority")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_agent_non_authority",
            "idem_judgment_agent_non_authority",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_agent_non_authority",
            "idem_record_agent_non_authority",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        user_judgment_actor_provenance(&harness, &pending_judgment_id)?.resolved_by_actor_source,
        Some(LOCAL_USER_ACTOR_SOURCE.to_owned())
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn agent_actor_cannot_resolve_non_authority_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "agent_non_authority_reject")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_agent_non_authority_reject",
            "idem_judgment_agent_non_authority_reject",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_agent_non_authority_reject",
            "idem_record_agent_non_authority_reject",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation_with_actor(
            ActorSource::agent_connection("connection_agent_user_only"),
            OperationCategory::UserOnly,
        ),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "invocation.actor_source"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn stored_final_acceptance_without_actor_provenance_does_not_authorize_close(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "final_legacy_provenance")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_legacy_provenance",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "final_legacy_provenance",
    )?;
    clear_user_judgment_actor_provenance(&harness, &final_judgment_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_final_legacy_provenance",
            idempotency_key: Some("idem_close_final_legacy_provenance"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &final_judgment_id,
        "resolved_by_actor_source",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn rejected_final_acceptance_does_not_authorize_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let suffix = "final_negative_rejected";
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
    let after_basis = record_close_evidence(&harness, &task_id, &change_unit_id, 2, suffix, true)?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_final_negative_rejected",
            "idem_final_negative_rejected",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let final_judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let mut record = record_judgment_request(
        "req_final_negative_rejected_record",
        "idem_final_negative_rejected_record",
        Some(after_basis + 1),
        &task_id,
        &final_judgment_id,
        JudgmentKind::FinalAcceptance,
        rejected_final_acceptance_payload(),
    );
    record.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    let recorded = harness
        .service
        .record_user_judgment(record, invocation(OperationCategory::UserOnly))?;
    let after_final = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_final_negative_rejected",
            idempotency_key: Some("idem_close_final_negative_rejected"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &final_judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(
        user_judgment_status(&harness, &final_judgment_id)?,
        "resolved"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_final_acceptance_non_user_actor_does_not_authorize_close_or_status(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "final_non_user")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_non_user",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "final_non_user",
    )?;
    set_user_judgment_resolution_actor(&harness, &final_judgment_id, AGENT_ACTOR_SOURCE)?;
    let before = harness.counts()?;

    let close = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_final_non_user",
            idempotency_key: Some("idem_close_final_non_user"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_final_non_user",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(close.response_value["close_state"], "blocked");
    assert_close_blocker(&close.response_value, "missing_final_acceptance");
    assert_eq!(status.response_value["close_state"], "blocked");
    assert_close_blocker(&status.response_value, "missing_final_acceptance");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &final_judgment_id)?,
        Some("accepted".to_owned())
    );
    assert_eq!(
        user_judgment_status(&harness, &final_judgment_id)?,
        "resolved"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_residual_risk_acceptance_non_user_actor_covers_no_risks() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_non_user")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "risk_non_user",
        vec![residual_risk_input("Risk needing user acceptance.")],
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_non_user",
            "idem_risk_non_user",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let accepted = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_non_user_record",
            "idem_risk_non_user_record",
            Some(after_basis + 1),
            &task_id,
            &judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_risk = accepted.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    set_user_judgment_resolution_actor(&harness, &judgment_id, AGENT_ACTOR_SOURCE)?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_risk,
        "risk_non_user",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_risk_non_user",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let coverage = status.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["risk_id"], risk_ids[0]);
    assert_eq!(coverage[0]["accepted"], false);
    assert_eq!(coverage[0]["accepted_by_judgment_refs"], json!([]));
    assert_close_blocker(&status.response_value, "missing_residual_risk_acceptance");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &judgment_id)?,
        Some("accepted".to_owned())
    );
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_sensitive_approval_non_user_actor_does_not_authorize_write() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive_non_user")?;
    let (after_approval, judgment_id) =
        record_sensitive_approval(&harness, &task_id, &change_unit_id, 2, "sensitive_non_user")?;
    set_user_judgment_resolution_actor(&harness, &judgment_id, AGENT_ACTOR_SOURCE)?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_sensitive_non_user",
        "idem_prepare_sensitive_non_user",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert_eq!(
        response.response_value["active_user_judgment_refs"],
        json!([])
    );
    assert!(response.response_value["write_check"].is_null());
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn incompatible_judgment_kind_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "kind")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_kind",
            "idem_judgment_kind",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_wrong_kind",
            "idem_record_wrong_kind",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::TechnicalDecision,
            answer_payload(JudgmentKind::TechnicalDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn final_acceptance_does_not_substitute_for_residual_risk_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk")?;
    enable_record_run_capabilities(&harness)?;
    let mut basis_request = record_run_request(
        "req_judgment_risk_basis",
        "idem_judgment_risk_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    basis_request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    basis_request.close_assessment = Some(close_assessment_with_risks(
        "Close claim supported with a residual risk.",
        vec![residual_risk_input(
            "Risk that still needs user acceptance.",
        )],
    ))
    .into();
    let basis_response = harness
        .service
        .record_run(basis_request, invocation(OperationCategory::AgentWorkflow))?;
    let after_basis = basis_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_risk",
            "idem_judgment_risk",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_final_for_risk",
            "idem_record_final_for_risk",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            answer_payload(JudgmentKind::FinalAcceptance),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn final_acceptance_for_old_scope_revision_is_rejected_for_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "final_old_scope")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_old_scope_initial",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "old_scope",
    )?;

    let scope_response = harness.service.update_scope(
        update_scope_request(
            "req_final_old_scope_change",
            "idem_final_old_scope_change",
            false,
            Some(after_final),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Materially changed scope after final acceptance.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_scope = scope_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    assert_eq!(user_judgment_status(&harness, &final_judgment_id)?, "stale");
    assert_eq!(
        user_judgment_basis_status(&harness, &final_judgment_id)?,
        "stale"
    );

    let after_new_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_scope,
        "final_old_scope_new_basis",
        true,
    )?;
    let before_close = harness.counts()?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_final_old_scope_close",
            idempotency_key: Some("idem_final_old_scope_close"),
            dry_run: false,
            expected_state_version: Some(after_new_basis),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    assert_eq!(harness.counts()?, before_close);
    Ok(())
}

#[test]
fn final_acceptance_for_old_close_basis_revision_is_rejected_for_close(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "final_old_basis")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_old_basis_initial",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "old_basis",
    )?;
    let after_new_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "final_old_basis_new_run",
        true,
    )?;

    assert_eq!(user_judgment_status(&harness, &final_judgment_id)?, "stale");
    assert_eq!(
        user_judgment_basis_status(&harness, &final_judgment_id)?,
        "stale"
    );
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_final_old_basis_close",
            idempotency_key: Some("idem_final_old_basis_close"),
            dry_run: false,
            expected_state_version: Some(after_new_basis),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    Ok(())
}

#[test]
fn resolved_judgment_without_outcome_is_owner_state_corruption() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "missing_outcome")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_outcome",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_missing_outcome_judgment",
            "idem_missing_outcome_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(
        &harness,
        &judgment_id,
        Some(
            r#"{
                "selected_option_id":"accept",
                "answer":{
                    "product_decision":null,
                    "technical_decision":null,
                    "scope_decision":null,
                    "sensitive_action_scope":null,
                    "final_acceptance":{"judgment":{"decision":"accepted"}},
                    "residual_risk_acceptance":null,
                    "cancellation":null
                },
                "note":null,
                "accepted_risks":[],
                "resolved_by_actor_source":"user"
            }"#,
        ),
    )?;
    let before_close = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_missing_outcome_close",
            idempotency_key: Some("idem_missing_outcome_close"),
            dry_run: false,
            expected_state_version: Some(after_basis + 1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_machine_action",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before_close);
    Ok(())
}

#[test]
fn partial_residual_risk_acceptance_leaves_current_risk_blocker() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_partial")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "partial",
        vec![
            residual_risk_input("First risk needing acceptance."),
            residual_risk_input("Second risk needing acceptance."),
        ],
    )?;
    let accepted_risk_ids = vec![risk_ids[0].clone()];
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_partial",
            "idem_risk_partial",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let accepted = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_partial_record",
            "idem_risk_partial_record",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&accepted_risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_partial = accepted.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_partial,
        "risk_partial",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_risk_partial_close",
            idempotency_key: Some("idem_risk_partial_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    Ok(())
}

#[test]
fn stale_residual_risk_acceptance_is_distinct_from_missing_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_stale")?;
    let (after_old_basis, old_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "stale_old",
        vec![residual_risk_input(
            "Risk accepted against the old close basis.",
        )],
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_stale",
            "idem_risk_stale",
            false,
            Some(after_old_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let accepted = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_stale_record",
            "idem_risk_stale_record",
            Some(after_old_basis + 1),
            &task_id,
            &judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&old_risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_old_acceptance = accepted.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let (after_current_basis, current_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        after_old_acceptance,
        "stale_current",
        vec![residual_risk_input(
            "Risk accepted against the old close basis.",
        )],
    )?;
    assert_ne!(old_risk_ids[0], current_risk_ids[0]);
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_current_basis,
        "risk_stale",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_risk_stale_close",
            idempotency_key: Some("idem_risk_stale_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "stale");
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "stale_residual_risk_acceptance");
    assert_no_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    let coverage = response.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["risk_id"], current_risk_ids[0]);
    assert_eq!(coverage[0]["accepted"], false);
    assert_eq!(coverage[0]["missing_reason"], "stale_acceptance");
    let risk_blocker = response.response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .find(|blocker| blocker["code"] == "stale_residual_risk_acceptance")
        .expect("stale residual-risk blocker");
    assert!(risk_blocker["related_refs"]
        .as_array()
        .expect("related refs should be an array")
        .iter()
        .any(|record_ref| record_ref["record_id"] == judgment_id));
    Ok(())
}

#[test]
fn residual_risk_answer_rejects_identical_text_with_different_risk_id() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_identity")?;
    let (after_old_basis, old_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "old_identity",
        vec![residual_risk_input("Same visible risk text.")],
    )?;
    let (after_current_basis, current_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        after_old_basis,
        "current_identity",
        vec![residual_risk_input("Same visible risk text.")],
    )?;
    assert_ne!(old_risk_ids[0], current_risk_ids[0]);
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_wrong_id",
            "idem_risk_wrong_id",
            false,
            Some(after_current_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_wrong_id_record",
            "idem_risk_wrong_id_record",
            Some(after_current_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&old_risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn sensitive_approval_requires_exact_path_category_and_change_unit() -> Result<(), Box<dyn Error>> {
    let path_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&path_harness, "sensitive_path")?;
    let (after_approval, _) =
        record_sensitive_approval(&path_harness, &task_id, &change_unit_id, 2, "path")?;
    let mut request = prepare_write_request(
        "req_sensitive_path_prepare",
        "idem_sensitive_path_prepare",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["tests/export.rs".to_owned()];
    request.sensitive_categories = vec!["network".to_owned()];
    let response = path_harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_check"].is_null());

    let category_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&category_harness, "sensitive_category")?;
    let (after_approval, _) =
        record_sensitive_approval(&category_harness, &task_id, &change_unit_id, 2, "category")?;
    let mut request = prepare_write_request(
        "req_sensitive_category_prepare",
        "idem_sensitive_category_prepare",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["credential".to_owned()];
    let response = category_harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_check"].is_null());

    let cu_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&cu_harness, "sensitive_change_unit")?;
    let (after_approval, _) =
        record_sensitive_approval(&cu_harness, &task_id, &change_unit_id, 2, "change_unit")?;
    let replace = cu_harness.service.update_scope(
        update_scope_request(
            "req_sensitive_cu_replace",
            "idem_sensitive_cu_replace",
            false,
            Some(after_approval),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope for sensitive approval.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");
    let after_replace = replace.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let mut request = prepare_write_request(
        "req_sensitive_cu_prepare",
        "idem_sensitive_cu_prepare",
        Some(after_replace),
        Some(&task_id),
        Some(&replacement_change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = cu_harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_check"].is_null());
    Ok(())
}

#[test]
fn public_sensitive_lifecycle_derives_full_requirement_and_closes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "sensitive_public_lifecycle")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_sensitive_public_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["base"]["response_kind"], "result");

    let (after_sensitive, _) =
        record_sensitive_approval(&harness, &task_id, &change_unit_id, 2, "public_lifecycle")?;
    let mut prepare = prepare_write_request(
        "req_sensitive_public_prepare",
        "idem_sensitive_public_prepare",
        Some(after_sensitive),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.sensitive_categories = vec!["network".to_owned()];
    let prepared = harness
        .service
        .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(prepared.response_value["decision"], "allowed");
    let write_check_id = response_record_id(&prepared.response_value, "write_check_ref");
    let after_prepare = prepared.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    enable_record_run_capabilities(&harness)?;
    let staged = stage_artifact_for_record_run(
        &harness,
        &task_id,
        "sensitive_public_lifecycle",
        after_prepare,
    )?;
    let mut run = product_write_record_run_request(
        "req_sensitive_public_run",
        "idem_sensitive_public_run",
        after_prepare,
        &task_id,
        &change_unit_id,
        &write_check_id,
        "run_sensitive_public",
    );
    run.observed_changes.sensitive_categories = vec!["network".to_owned()];
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_sensitive_public",
        staged,
        Some("validation_report"),
        Some("Close claim supported."),
    )];
    run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    run.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Sensitive product write is ready for close.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: vec!["network".to_owned()],
        recovery_constraints: Vec::new(),
    })
    .into();
    let recorded = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");
    let requirement =
        &recorded.response_value["current_close_basis"]["sensitive_action_requirements"][0];
    assert_eq!(requirement["action_kind"], "local_sensitive_step");
    assert_eq!(requirement["normalized_paths"], json!(["src/export.rs"]));
    assert_eq!(requirement["sensitive_categories"], json!(["network"]));
    assert_eq!(requirement["change_unit_id"], change_unit_id);
    assert_eq!(
        requirement["source_write_check_ref"]["record_id"],
        write_check_id
    );
    assert!(requirement["action_kind"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(!requirement["normalized_paths"]
        .as_array()
        .expect("paths should be an array")
        .is_empty());
    let after_run = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_sensitive_public_status_after_run",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(
        status.response_value["current_close_basis"]["sensitive_action_requirements"][0]
            ["normalized_paths"],
        json!(["src/export.rs"])
    );

    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_run,
        "sensitive_public_lifecycle",
    )?;
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_public_close",
            idempotency_key: Some("idem_sensitive_public_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    assert_no_close_blocker(&closed.response_value, "missing_sensitive_approval");
    Ok(())
}

#[test]
fn close_sensitive_approval_coverage_rejects_mismatched_approvals() -> Result<(), Box<dyn Error>> {
    fn assert_mismatch(
        suffix: &str,
        requirement_categories: &[&str],
        approval_scope: volicord_types::SensitiveActionScope,
        mutate_basis: Option<fn(&mut Value)>,
        accepted: bool,
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
        let write_check_id = format!("wa_sensitive_{suffix}");
        let recorded = record_sensitive_product_write_close_basis(
            &harness,
            SensitiveProductWriteBasisFixture {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                expected_state_version: 2,
                suffix,
                write_check_id: &write_check_id,
                intended_operation: "local_sensitive_step",
                intended_paths: &["src/export.rs"],
                observed_categories: requirement_categories,
                assessment_categories: requirement_categories,
            },
        )?;
        assert_eq!(recorded.response_value["base"]["response_kind"], "result");
        let after_basis = recorded.response_value["base"]["state_version"]
            .as_u64()
            .expect("state_version should be present");
        let (after_approval, judgment_id) = record_sensitive_approval_with_scope(
            &harness,
            &task_id,
            &change_unit_id,
            after_basis,
            suffix,
            approval_scope,
            accepted,
        )?;
        if let Some(mutate_basis) = mutate_basis {
            mutate_user_judgment_basis_json(&harness, &judgment_id, mutate_basis)?;
        }
        let after_final =
            record_final_acceptance(&harness, &task_id, &change_unit_id, after_approval, suffix)?;
        let close_request_id = format!("req_close_sensitive_{suffix}");
        let close_idempotency_key = format!("idem_close_sensitive_{suffix}");
        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: &close_request_id,
                idempotency_key: Some(&close_idempotency_key),
                dry_run: false,
                expected_state_version: Some(after_final),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(response.response_value["close_state"], "blocked");
        assert_close_blocker(&response.response_value, "missing_sensitive_approval");
        Ok(())
    }

    assert_mismatch(
        "sensitive_wrong_operation",
        &["network"],
        sensitive_scope(
            "other_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        None,
        true,
    )?;
    assert_mismatch(
        "sensitive_wrong_path",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["tests/export.rs"],
            vec!["network"],
        ),
        None,
        true,
    )?;
    assert_mismatch(
        "sensitive_partial_category",
        &["network", "credential"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        None,
        true,
    )?;
    assert_mismatch(
        "sensitive_wrong_baseline",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        Some(|basis| basis["baseline_ref"] = json!("other_baseline")),
        true,
    )?;
    assert_mismatch(
        "sensitive_wrong_change_unit",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        Some(|basis| basis["change_unit_id"] = json!("other_change_unit")),
        true,
    )?;
    assert_mismatch(
        "sensitive_rejected",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        None,
        false,
    )?;
    Ok(())
}

#[test]
fn multiple_sensitive_requirements_require_complete_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive_multiple")?;
    let first = record_sensitive_product_write_close_basis(
        &harness,
        SensitiveProductWriteBasisFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: 2,
            suffix: "multiple_network",
            write_check_id: "wa_sensitive_multiple_network",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            observed_categories: &["network"],
            assessment_categories: &["network"],
        },
    )?;
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let second = record_sensitive_product_write_close_basis(
        &harness,
        SensitiveProductWriteBasisFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: after_first,
            suffix: "multiple_credential",
            write_check_id: "wa_sensitive_multiple_credential",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            observed_categories: &["credential"],
            assessment_categories: &["network", "credential"],
        },
    )?;
    let requirements = second.response_value["current_close_basis"]
        ["sensitive_action_requirements"]
        .as_array()
        .expect("requirements should be an array");
    assert_eq!(requirements.len(), 2);
    let after_second = second.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let (after_network, _) = record_sensitive_approval_with_scope(
        &harness,
        &task_id,
        &change_unit_id,
        after_second,
        "multiple_network_only",
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_network,
        "multiple_network_only",
    )?;
    let blocked = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_multiple_blocked",
            idempotency_key: Some("idem_sensitive_multiple_blocked"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(blocked.response_value["close_state"], "blocked");
    assert_close_blocker(&blocked.response_value, "missing_sensitive_approval");

    let (after_credential, _) = record_sensitive_approval_with_scope(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "multiple_credential",
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["credential"],
        ),
        true,
    )?;
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_multiple_closed",
            idempotency_key: Some("idem_sensitive_multiple_closed"),
            dry_run: false,
            expected_state_version: Some(after_credential),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    Ok(())
}

#[test]
fn close_assessment_cannot_invent_or_erase_sensitive_requirements() -> Result<(), Box<dyn Error>> {
    let invent_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&invent_harness, "sensitive_invent")?;
    enable_record_run_capabilities(&invent_harness)?;
    let mut invent = record_run_request(
        "req_sensitive_invent",
        "idem_sensitive_invent",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    invent.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    invent.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Caller tries to invent a sensitive category.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: vec!["network".to_owned()],
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_invent = invent_harness.counts()?;
    let invented = invent_harness
        .service
        .record_run(invent, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(invented.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        invented.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(invent_harness.counts()?, before_invent);

    let erase_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&erase_harness, "sensitive_erase")?;
    let first = record_sensitive_product_write_close_basis(
        &erase_harness,
        SensitiveProductWriteBasisFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: 2,
            suffix: "erase_initial",
            write_check_id: "wa_sensitive_erase_initial",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            observed_categories: &["network"],
            assessment_categories: &["network"],
        },
    )?;
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    enable_record_run_capabilities(&erase_harness)?;
    let mut erase = record_run_request(
        "req_sensitive_erase",
        "idem_sensitive_erase",
        false,
        Some(after_first),
        &task_id,
        &change_unit_id,
    );
    erase.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    erase.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Caller tries to erase the sensitive requirement.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_erase = erase_harness.counts()?;
    let erased = erase_harness
        .service
        .record_run(erase, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(erased.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        erased.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(erase_harness.counts()?, before_erase);
    Ok(())
}

#[test]
fn category_only_close_basis_is_corrupt_owner_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "category_only_basis")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "category_only_basis",
        true,
    )?;
    let revision = task_revision(&harness, &task_id)?;
    let mut category_only_basis = serde_json::to_value(
        revision
            .current_close_basis
            .expect("close basis should exist"),
    )?;
    category_only_basis["sensitive_categories"] = json!(["network"]);
    category_only_basis
        .as_object_mut()
        .expect("close basis should be an object")
        .remove("sensitive_action_requirements");
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(&category_only_basis.to_string()),
    )?;
    let before = harness.counts()?;

    let check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_category_only_basis_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_owner_state_rejection(
        &check,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_change_supersedes_pending_judgment_and_stale_pending_answer_has_no_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "pending_superseded")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_pending_superseded",
            "idem_pending_superseded",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    assert_eq!(
        user_judgment_basis_status(&harness, &pending_judgment_id)?,
        "current"
    );
    let scope_response = harness.service.update_scope(
        update_scope_request(
            "req_pending_superseded_material_scope",
            "idem_pending_superseded_material_scope",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Material scope change after pending judgment.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        scope_response.response_value["base"]["response_kind"], "result",
        "{:?}",
        scope_response.response_value
    );
    assert_eq!(scope_response.response_value["base"]["state_version"], 4);
    assert_eq!(
        scope_response.response_value["state"]["pending_user_judgment_refs"],
        json!([])
    );

    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "superseded"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &pending_judgment_id)?,
        "superseded"
    );
    let before = harness.counts()?;
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_pending_superseded_answer",
            "idem_pending_superseded_answer",
            Some(4),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn basisless_resolved_judgment_is_rejected_by_storage_constraint() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "basis_required")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "basis_required",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "basis_required",
    )?;
    let before = harness.counts()?;

    let error = harness
        .conn()?
        .execute(
            "UPDATE user_judgments
                SET basis_json = NULL
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![PROJECT_ID, final_judgment_id],
        )
        .expect_err("basis_json is required for stored judgments");
    assert_constraint_error(error);
    assert_eq!(harness.counts()?, before);
    assert_eq!(after_final, before.state_version);
    Ok(())
}

#[test]
fn bare_array_authority_options_are_owner_state_corruption() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "legacy_options")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_legacy_options",
            "idem_legacy_options",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    set_user_judgment_owner_json(
        &harness,
        &pending_judgment_id,
        "options_json",
        Some(
            r#"[{
                "option_id":"accept",
                "label":"Accept",
                "description":"Bare array option without machine action.",
                "consequence":"Ambiguity must not become current authority.",
                "is_default":true
            }]"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_legacy_options",
            "idem_record_legacy_options",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::Cancellation,
            answer_payload(JudgmentKind::Cancellation),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &pending_judgment_id,
        "options_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        None
    );
    Ok(())
}

#[test]
fn record_user_judgment_rejects_selected_option_outside_original_request(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "judgment_option")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_option",
            "idem_judgment_option",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_judgment_option_record",
        "idem_judgment_option_record",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("not_an_option");
    let before = harness.counts()?;
    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn sensitive_action_scope_does_not_create_write_check() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_sensitive",
            "idem_judgment_sensitive",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_sensitive",
            "idem_record_sensitive",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(after.write_checks, before.write_checks);
    assert_eq!(
        response.response_value["state"]["write_check_summary"],
        Value::Null
    );
    Ok(())
}

#[test]
fn recorded_scope_decision_does_not_change_scope_or_current_change_unit(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_judgment")?;
    let original_scope = current_change_unit_scope(&harness, &task_id)?;
    let original_current = current_change_unit_id(&harness, &task_id)?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_scope",
            "idem_judgment_scope",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_scope",
            "idem_record_scope",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["state"]["scope_summary"],
        "Initial current scope."
    );
    assert_eq!(
        current_change_unit_scope(&harness, &task_id)?,
        original_scope
    );
    assert_eq!(
        current_change_unit_id(&harness, &task_id)?,
        original_current
    );
    assert_eq!(after.change_units, before.change_units);
    Ok(())
}

#[test]
fn local_user_channel_records_authority_judgment_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "judgment_provenance")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_provenance",
            "idem_judgment_provenance",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_judgment_provenance",
            "idem_record_judgment_provenance",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    assert_eq!(
        user_judgment_resolution_machine_action(&harness, &pending_judgment_id)?,
        Some("accept".to_owned())
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    let provenance = user_judgment_actor_provenance(&harness, &pending_judgment_id)?;
    assert_eq!(
        provenance.resolved_by_actor_source,
        Some(LOCAL_USER_ACTOR_SOURCE.to_owned())
    );
    let verification_basis = provenance
        .resolved_verification_basis
        .expect("resolved verification basis should be present");
    assert_eq!(verification_basis, VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    assert_eq!(
        provenance.resolved_assurance_level,
        Some("local_user_channel".to_owned())
    );
    assert_eq!(harness.counts()?.user_judgments, before.user_judgments);
    Ok(())
}

#[test]
fn agent_connection_cannot_record_authority_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "agent_connection")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_agent_connection_judgment",
            "idem_agent_connection_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_agent_connection_record",
            "idem_agent_connection_record",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "invocation.operation_category"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn accepted_authority_judgments_require_structured_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "rationale_required")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_rationale_required",
            "idem_rationale_required",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_rationale_required",
        "idem_record_rationale_required",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ScopeDecision,
        answer_payload(JudgmentKind::ScopeDecision),
    );
    request.rationale.selected_reason = None.into();
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "rationale.selected_reason"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn project_continuity_and_rationale_do_not_replace_final_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "continuity_not_auth")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "continuity_not_auth",
        true,
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_continuity_product",
            "idem_continuity_product",
            false,
            Some(after_evidence),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let recorded = harness.service.record_user_judgment(
        record_judgment_request(
            "req_continuity_product_record",
            "idem_continuity_product_record",
            Some(after_evidence + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_judgment = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let continuity = harness.continuity_records()?;
    assert_eq!(continuity.len(), 1);
    assert_eq!(continuity[0].kind, "decision");
    let before_close = harness.counts()?;

    let close = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_continuity_not_auth_close",
            idempotency_key: Some("idem_continuity_not_auth_close"),
            dry_run: false,
            expected_state_version: Some(after_judgment),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(close.response_value["close_state"], "blocked");
    assert_close_blocker(&close.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before_close);
    Ok(())
}

#[test]
fn final_and_residual_risk_acceptance_are_non_substitutable() -> Result<(), Box<dyn Error>> {
    let final_only_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&final_only_harness, "final_only_risk")?;
    let (after_basis, _) = record_close_basis_with_risks(
        &final_only_harness,
        &task_id,
        &change_unit_id,
        2,
        "final_only_risk",
        vec![residual_risk_input("Risk still needs separate acceptance.")],
    )?;
    let after_final = record_final_acceptance(
        &final_only_harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "final_only_risk",
    )?;
    let final_only = final_only_harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_final_only_risk_close",
            idempotency_key: Some("idem_final_only_risk_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_close_blocker(
        &final_only.response_value,
        "missing_residual_risk_acceptance",
    );
    assert_no_close_blocker(&final_only.response_value, "missing_final_acceptance");

    let risk_only_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&risk_only_harness, "risk_only_final")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &risk_only_harness,
        &task_id,
        &change_unit_id,
        2,
        "risk_only_final",
        vec![residual_risk_input(
            "Risk is accepted but final acceptance is absent.",
        )],
    )?;
    let pending_judgment = risk_only_harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_only_judgment",
            "idem_risk_only_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let recorded = risk_only_harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_only_record",
            "idem_risk_only_record",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_risk = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let risk_only = risk_only_harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_risk_only_final_close",
            idempotency_key: Some("idem_risk_only_final_close"),
            dry_run: false,
            expected_state_version: Some(after_risk),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_close_blocker(&risk_only.response_value, "missing_final_acceptance");
    assert_no_close_blocker(
        &risk_only.response_value,
        "missing_residual_risk_acceptance",
    );
    Ok(())
}

#[test]
fn judgment_dry_runs_have_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_judgment")?;
    let before_request = harness.counts()?;

    let request_preview = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_dry",
            "idem_judgment_dry",
            true,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        request_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(harness.counts()?, before_request);

    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_dry_record",
            "idem_judgment_dry_record",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before_record = harness.counts()?;

    let mut record_preview_request = record_judgment_request(
        "req_record_dry",
        "idem_record_dry",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_preview_request.envelope.dry_run = true;
    let record_preview = harness.service.record_user_judgment(
        record_preview_request,
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(
        record_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(harness.counts()?, before_record);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn stale_state_rejects_record_user_judgment_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "stale_judgment")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_stale",
            "idem_judgment_stale",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_stale",
            "idem_record_stale",
            Some(2),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn record_user_judgment_idempotency_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "replay_judgment")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_replay",
            "idem_judgment_replay",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_replay",
        "idem_record_replay",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );

    let first = harness
        .service
        .record_user_judgment(request.clone(), invocation(OperationCategory::UserOnly))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn close_task_check_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_check_dry_run_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check_dry")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_check_dry",
            idempotency_key: Some("idem_close_check_dry"),
            dry_run: true,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_does_not_use_terminal_summary_as_current_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "terminal_summary_not_basis")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(
            r#"{"close_reason":"none","visible_risks":[{"risk_id":"risk_summary_only","summary":"Terminal summary risk."}]}"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_terminal_summary_not_basis",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert!(response.response_value["current_close_basis"].is_null());
    assert_close_blocker(&response.response_value, "missing_current_close_basis");
    assert_no_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_completion_policy_rejects_close_check_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_policy_check")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_policy_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "completion_policy_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_completion_policy_rejects_close_complete_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_policy_complete")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_policy_complete",
            idempotency_key: Some("idem_bad_policy_complete"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "completion_policy_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn schema_invalid_close_summary_rejects_instead_of_hiding_residual_risk(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_close_summary")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(r#"{"residual_risks":"known-but-wrong-shape"}"#),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_close_summary",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_summary_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_close_basis_stops_close_readiness_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_close_basis")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_close_basis",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_lifecycle_state_does_not_default_close_phase() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_lifecycle")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "lifecycle_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_lifecycle",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "lifecycle_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_write_basis_rejects_prepare_write_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_write_basis")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "write_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_write_basis",
            "idem_bad_write_basis",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "write_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_bounded_paths_rejects_prepare_write_without_empty_scope() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_paths")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "bounded_paths_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_paths",
            "idem_bad_paths",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "bounded_paths_json",
        &harness.runtime_home_path,
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_dry_run_with_corrupt_owner_state_is_rejected_no_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_bad_owner")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "write_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;
    let mut request = prepare_write_request(
        "req_dry_bad_owner",
        "idem_dry_bad_owner",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = true;

    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "write_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_read_only_rejects_corrupt_owner_state_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_bad_owner")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_bad_owner", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_summary_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn resolved_judgment_null_resolution_json_is_owner_state_corruption() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "null_resolution")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "null_resolution",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_null_resolution_judgment",
            "idem_null_resolution_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(&harness, &judgment_id, None)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_null_resolution_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_optional_resolution_json_rejects_close_readiness() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_resolution")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_resolution",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_resolution_judgment",
            "idem_bad_resolution_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(&harness, &judgment_id, Some(corrupt_owner_json()))?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_resolution_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_judgment_request_wrong_field_type_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_request_type")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_request_type",
            "idem_bad_request_type",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let corrupt_request_json = r#"{"presentation":17,"question":"must not leak secret-request-path","required_for":["close_complete"],"expires_at":null}"#;
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_request_type",
            "idem_record_bad_request_type",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, "secret-request-path");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn request_user_judgment_rejects_expiration_at_clock_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_request_exact")?;
    let before = harness.counts()?;
    let mut request = user_judgment_request(
        "req_judgment_expiry_request_exact",
        "idem_judgment_expiry_request_exact",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")?).into();

    let response = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "expires_at"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_user_judgment_uses_semantic_expiry_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_before")?;
    let mut request = user_judgment_request(
        "req_judgment_expiry_before",
        "idem_judgment_expiry_before",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::UtcTimestamp::parse(
        "2026-06-18T09:00:01+09:00",
    )?)
    .into();
    let judgment = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_judgment_expiry_before",
            "idem_record_judgment_expiry_before",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");

    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_exact")?;
    let mut request = user_judgment_request(
        "req_judgment_expiry_exact",
        "idem_judgment_expiry_exact",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::UtcTimestamp::parse("2026-06-18T00:00:01Z")?).into();
    let judgment = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    clock.advance(Duration::seconds(1));
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_judgment_expiry_exact",
            "idem_record_judgment_expiry_exact",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_request_invalid_expiration_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_request_expiration")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_request_expiration",
            "idem_bad_request_expiration",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let corrupt_request_json = r#"{"presentation":"short","question":"must not leak secret-expiry-path","required_for":["close_complete"],"expires_at":"tomorrow"}"#;
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_request_expiration",
            "idem_record_bad_request_expiration",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, "secret-expiry-path");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_request_missing_required_field_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_request_missing")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_request_missing",
            "idem_bad_request_missing",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let corrupt_request_json =
        r#"{"presentation":"short","required_for":["close_complete"],"expires_at":null}"#;
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_request_missing",
            "idem_record_bad_request_missing",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, corrupt_request_json);
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_resolution_incompatible_branches_rejects_close_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_resolution_branch")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_resolution_branch",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_resolution_branch_judgment",
            "idem_bad_resolution_branch_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(
        &harness,
        &judgment_id,
        Some(
            r#"{
                "selected_option_id":"accept",
                "machine_action":"accept",
                "resolution_outcome":"accepted",
                "answer":{
                    "product_decision":{"judgment":{"decision":"accepted"}},
                    "technical_decision":null,
                    "scope_decision":null,
                    "sensitive_action_scope":null,
                    "final_acceptance":{"judgment":{"decision":"accepted"}},
                    "residual_risk_acceptance":null,
                    "cancellation":null
                },
                "note":null,
                "accepted_risks":[],
                "resolved_by_actor_source":"user"
            }"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_resolution_branch",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_judgment_basis_invalid_revision_type_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_basis_revision")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_basis_revision",
            "idem_bad_basis_revision",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "basis_json",
        Some(
            &json!({
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "scope_revision": "not-a-revision",
                "close_basis_revision": null,
                "baseline_ref": null,
                "result_refs": [],
                "residual_risk_ids": [],
                "sensitive_action_scope": null,
                "created_at_state_version": 3,
                "compatibility_status": "current"
            })
            .to_string(),
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_basis_revision",
            "idem_record_bad_basis_revision",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_accepted_risk_missing_risk_id_rejects_close_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_accepted_risk")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_accepted_risk",
        vec![residual_risk_input("Risk requiring explicit acceptance.")],
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_accepted_risk_judgment",
            "idem_bad_accepted_risk_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(
        &harness,
        &judgment_id,
        Some(
            &json!({
                "selected_option_id": "accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "answer": {
                    "product_decision": null,
                    "technical_decision": null,
                    "scope_decision": null,
                    "sensitive_action_scope": null,
                    "final_acceptance": null,
                    "residual_risk_acceptance": { "risk_ids": risk_ids },
                    "cancellation": null
                },
                "note": null,
                "accepted_risks": [{
                    "summary": "Risk accepted without a persisted risk_id.",
                    "consequence": "The missing risk identity must fail closed.",
                    "related_refs": [],
                    "accepted_for_close": true
                }],
                "resolved_by_actor_source": "user"
            })
            .to_string(),
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_accepted_risk",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_artifact_producer_json_rejects_existing_artifact_run_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_producer")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "bad_producer")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    set_artifact_owner_json(
        &harness,
        &artifact_id,
        "producer_json",
        corrupt_owner_json(),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_reuse_bad_producer",
        "idem_reuse_bad_producer",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_bad_producer",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_rejection(
        &response,
        "artifacts",
        &artifact_id,
        "producer_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn artifact_provenance_missing_source_ref_rejects_close_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_provenance")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "bad_provenance")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let artifact_state_ref = StateRecordRef {
        record_kind: StateRecordKind::Artifact,
        record_id: RecordId::new(&artifact_id),
        project_id: ProjectId::new(PROJECT_ID),
        task_id: Some(TaskId::new(&task_id)).into(),
        state_version: Some(state_version).into(),
    };
    let mut basis_request = record_run_request(
        "req_basis_bad_provenance",
        "idem_basis_bad_provenance",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    basis_request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_bad_provenance_basis",
        artifact_ref,
    )];
    basis_request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Close basis references the registered artifact.".to_owned(),
        result_refs: vec![artifact_state_ref],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let basis_response = harness
        .service
        .record_run(basis_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        basis_response.response_value["base"]["response_kind"],
        "result"
    );
    clear_artifact_source_staging_handle(&harness, &artifact_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_provenance",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "artifacts",
        &artifact_id,
        "source_staging_handle_id",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_evidence_coverage_rejects_status_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_evidence_coverage")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence_coverage",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let corrupt_coverage_json =
        r#"{"claim":"secret-evidence-coverage-path","coverage_state":"supported"}"#;
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "coverage_json",
        corrupt_coverage_json,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_bad_evidence_coverage",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "evidence_summaries",
        &evidence_summary_id,
        "coverage_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, "secret-evidence-coverage-path");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_evidence_source_refs_rejects_close_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_evidence_refs")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence_refs",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "supporting_refs_json",
        r#"{"record_kind":"run"}"#,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_evidence_refs",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "evidence_summaries",
        &evidence_summary_id,
        "supporting_refs_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_evidence_metadata_rejects_status_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_evidence_metadata")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence_metadata",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "metadata_json",
        r#"{"updated_by_run_id":123}"#,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_bad_evidence_metadata",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "evidence_summaries",
        &evidence_summary_id,
        "metadata_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn display_only_staged_artifact_metadata_corruption_falls_back() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "display_only_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "display_only_artifact", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    set_artifact_staging_artifact_json(&harness, &handle_id, corrupt_owner_json())?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_display_only_artifact",
        "idem_display_only_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_display_only",
        handle,
        Some("display_only"),
        Some("Display-only artifact metadata may fall back."),
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["registered_artifacts"][0]["display_name"],
        handle_id
    );
    assert_public_response_omits(&response, corrupt_owner_json());
    assert_eq!(harness.counts()?.state_version, before.state_version + 1);
    Ok(())
}

#[test]
fn close_task_complete_blocks_missing_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_no_final")?;
    let state_version =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "no_final", true)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_no_final",
            idempotency_key: Some("idem_close_no_final"),
            dry_run: false,
            expected_state_version: Some(state_version),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_complete_blocks_only_relevant_pending_judgments() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_pending_kind")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "pending_kind", true)?;
    let mut product_request = user_judgment_request(
        "req_close_product_pending",
        "idem_close_product_pending",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for = vec![volicord_types::JudgmentRequiredFor::CloseComplete];
    harness.service.request_user_judgment(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "pending_kind",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_product_pending_attempt",
            idempotency_key: Some("idem_close_product_pending_attempt"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "pending_user_judgment");
    assert_close_blocker_category(
        &response.response_value,
        "pending_user_judgment",
        "pending_user_judgment",
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_complete_ignores_pending_cancellation_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_ignore_cancel")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "ignore_cancel",
        true,
    )?;
    harness.service.request_user_judgment(
        user_judgment_request(
            "req_close_cancel_pending",
            "idem_close_cancel_pending",
            false,
            Some(after_evidence),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "ignore_cancel",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_ignore_cancel",
            idempotency_key: Some("idem_close_ignore_cancel"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_no_close_blocker(&response.response_value, "pending_user_judgment");
    Ok(())
}

#[test]
fn close_task_complete_blocks_unsupported_evidence_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_bad_evidence")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence",
        false,
    )?;
    let after_final =
        record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "bad")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_evidence",
            idempotency_key: Some("idem_close_bad_evidence"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_unsupported");
    assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn unverified_claim_alone_cannot_satisfy_close_readiness() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_unverified_claim")?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "unverified_claim",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
        )],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "unverified_claim",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_unverified_claim",
            idempotency_key: Some("idem_close_unverified_claim"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn missing_evidence_and_insufficient_provenance_are_distinct_blockers() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_missing_and_weak_evidence")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(
            r#"{"evidence_required":true,"required_claims":["Close claim supported.","Missing close claim."]}"#,
        ),
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_and_weak_evidence",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
        )],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "missing_and_weak_evidence",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_missing_and_weak_evidence",
            idempotency_key: Some("idem_close_missing_and_weak_evidence"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_missing");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_claim_missing",
        "evidence_claim",
    );
    assert_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_provenance_insufficient",
        "evidence_provenance",
    );
    Ok(())
}

#[test]
fn cooperative_agent_report_only_blocks_when_stronger_evidence_is_required(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_agent_report_only")?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "agent_report_only",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
        )],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "agent_report_only",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_agent_report_only",
            idempotency_key: Some("idem_close_agent_report_only"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_agent_report_only");
    Ok(())
}

#[test]
fn external_tool_provenance_supports_the_attached_close_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_external_tool")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "external_tool",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "external_tool",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_external_tool",
            idempotency_key: Some("idem_close_external_tool"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_no_close_blocker(&response.response_value, "evidence_agent_report_only");
    Ok(())
}

#[test]
fn supported_evidence_without_provenance_cannot_satisfy_close_readiness(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_missing_provenance")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_provenance",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "missing_provenance",
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let coverage_json: String = harness.conn()?.query_row(
        "SELECT coverage_json
           FROM evidence_summaries
          WHERE project_id = ?1
            AND evidence_summary_id = ?2",
        rusqlite::params![PROJECT_ID, evidence_summary_id],
        |row| row.get(0),
    )?;
    let mut coverage: Value = serde_json::from_str(&coverage_json)?;
    coverage[0]["observation_refs"] = json!([]);
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "coverage_json",
        &serde_json::to_string(&coverage)?,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_missing_provenance",
            idempotency_key: Some("idem_close_missing_provenance"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_provenance_insufficient",
        "evidence_provenance",
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn external_tool_evidence_does_not_support_unattached_close_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_external_tool_scope")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(
            r#"{"evidence_required":true,"required_claims":["Close claim supported.","Other claim supported."]}"#,
        ),
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "external_tool_scope",
        vec![supported_evidence_update("Other claim supported.")],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "external_tool_scope",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_external_tool_scope",
            idempotency_key: Some("idem_close_external_tool_scope"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_missing");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_claim_missing",
        "evidence_claim",
    );
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn user_observation_evidence_does_not_replace_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_user_observation")?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_observation",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::UserObservation,
            EvidenceAssuranceLevel::UserObserved,
        )],
        "Close claim supported.",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_user_observation",
            idempotency_key: Some("idem_close_user_observation"),
            dry_run: false,
            expected_state_version: Some(after_evidence),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    Ok(())
}

#[test]
fn stale_evidence_provenance_is_not_current_close_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_stale_provenance")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "stale_provenance",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let coverage_json: String = harness.conn()?.query_row(
        "SELECT coverage_json
           FROM evidence_summaries
          WHERE project_id = ?1
            AND evidence_summary_id = ?2",
        rusqlite::params![PROJECT_ID, evidence_summary_id],
        |row| row.get(0),
    )?;
    let mut coverage: Value = serde_json::from_str(&coverage_json)?;
    coverage[0]["observation_refs"][0]["state_version"] = json!(after_evidence - 1);
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "coverage_json",
        &serde_json::to_string(&coverage)?,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "stale_provenance",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_stale_provenance",
            idempotency_key: Some("idem_close_stale_provenance"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_provenance_stale");
    Ok(())
}

#[test]
fn close_task_complete_success() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_success")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "success", true)?;
    let after_final =
        record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "ok")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_success",
            idempotency_key: Some("idem_close_success"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(
        response.response_value["base"]["state_version"],
        after_final + 1
    );
    assert_eq!(fields.lifecycle_phase, "completed");
    assert_eq!(fields.result.as_deref(), Some("completed"));
    assert_eq!(
        fields.close_summary["close_reason"],
        "completed_self_checked"
    );
    assert!(fields.closed_at.is_some());
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn guarded_close_complete_success_reports_guard_health() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "guarded_success", "guarded", "active", "{}")?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_close_success")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_success",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_success",
    )?;

    let before_status = harness.counts()?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_success",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(harness.counts()?, before_status);
    assert_eq!(
        status.response_value["guard_health"]["guard_mode"],
        "guarded"
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_installation_id"],
        guard_installation_id
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_installation_status"],
        "active"
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_strength"],
        "host_hook_guarded"
    );
    assert_eq!(
        status.response_value["guard_health"]["pre_tool_blocking_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["post_tool_correlation_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["local_web_consent_available"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["managed_distribution_verified"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_hook_observed"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["last_guard_observed_at"],
        "2026-06-30T00:02:00Z"
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_status"],
        "observed"
    );
    assert_eq!(
        status.response_value["guard_health"]["mcp_connection_healthy"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_eq!(
        status.response_value["active_task"]["guard_health"]["guard_mode"],
        "guarded"
    );
    let local_web_status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_success_local_web",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read).with_local_web_consent_available(true),
    )?;
    assert_eq!(
        local_web_status.response_value["guard_health"]["local_web_consent_available"],
        true
    );

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_success",
            idempotency_key: Some("idem_close_guarded_success"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        response.response_value["guard_health"]["guard_mode"],
        "guarded"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "host_hook_guarded"
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    Ok(())
}

#[test]
fn managed_guarded_strength_requires_verified_distribution() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let managed_capability = json!({
        "schema": "volicord-guard-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "guard_profile": "managed_guarded",
        "managed_source": "org_policy_bundle",
        "managed_bundle_hash": "sha256:managedfixture",
        "managed_verification_status": "verified",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_guard_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": [],
        "prompt_capture": true
    })
    .to_string();
    record_guard_installation(
        &harness,
        "managed_verified",
        "managed",
        "active",
        &managed_capability,
    )?;
    let (task_id, _, _) = create_close_ready_task(&harness, "managed_verified")?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_managed_verified",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "session_watch_unavailable");
    let blocker = close_blocker_by_code(&response.response_value, "session_watch_unavailable");
    assert_eq!(blocker["guard_strength"], "managed_guarded");
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "managed_guarded"
    );
    assert_eq!(
        response.response_value["guard_health"]["pre_tool_blocking_available"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["post_tool_correlation_available"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["managed_distribution_verified"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    Ok(())
}

#[test]
fn managed_mode_without_verified_bundle_reports_host_hook_strength() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let unverified_capability = json!({
        "schema": "volicord-guard-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "guard_profile": "managed_guarded",
        "managed_source": "org_policy_bundle",
        "managed_verification_status": "unverified",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_guard_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": [],
        "prompt_capture": true
    })
    .to_string();
    record_guard_installation(
        &harness,
        "managed_unverified",
        "managed",
        "active",
        &unverified_capability,
    )?;
    let (task_id, _, _) = create_close_ready_task(&harness, "managed_unverified")?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_managed_unverified",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "session_watch_unavailable");
    let blocker = close_blocker_by_code(&response.response_value, "session_watch_unavailable");
    assert_eq!(blocker["guard_strength"], "host_hook_guarded");
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "host_hook_guarded"
    );
    assert_eq!(
        response.response_value["guard_health"]["managed_distribution_verified"],
        false
    );
    Ok(())
}

#[test]
fn guarded_close_blocks_unhealthy_guard_installation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "guarded_unhealthy",
        "guarded",
        "reload_required",
        "{}",
    )?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_unhealthy")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_unhealthy",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_unhealthy",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_unhealthy",
            idempotency_key: Some("idem_close_guarded_unhealthy"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_reload_required");
    assert_close_blocker_category(
        &response.response_value,
        "guard_reload_required",
        "connection_capability",
    );
    assert_close_blocker_resolution(
        &response.response_value,
        "guard_reload_required",
        false,
        true,
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_status"],
        "reload_required"
    );
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_close_blocks_configured_guard_before_observation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "guarded_not_observed",
        "guarded",
        "configured",
        "{}",
    )?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_not_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_not_observed",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_not_observed",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_not_observed",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["close_state"], "blocked");
    assert_close_blocker(&status.response_value, "guard_not_observed");
    assert_eq!(
        status.response_value["guard_health"]["guard_installation_status"],
        "configured"
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_strength"],
        "authority_record_only"
    );
    assert_eq!(
        status.response_value["guard_health"]["pre_tool_blocking_available"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_hook_observed"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["last_guard_observed_at"],
        Value::Null
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_status"],
        "configured"
    );
    assert_eq!(harness.counts()?, before);

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_not_observed",
            idempotency_key: Some("idem_close_guarded_not_observed"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_not_observed");
    let blocker = close_blocker_by_code(&response.response_value, "guard_not_observed");
    assert_eq!(blocker["guard_strength"], "authority_record_only");
    assert_close_blocker_resolution(&response.response_value, "guard_not_observed", false, true);
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_configured_guard_becomes_effectively_active_after_valid_observation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id = record_guard_installation(
        &harness,
        "guarded_configured_observed",
        "guarded",
        "configured",
        "{}",
    )?;
    let observed = observe_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationObservation {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "guarded".to_owned(),
            observed_policy_hash: "sha256:guardedfixture".to_owned(),
            observed_binary_version: Some("0.0.0-test".to_owned()),
            observed_phase: "session_start".to_owned(),
            observed_at: "2026-06-30T00:03:00Z".to_owned(),
        },
    )?
    .expect("matching observation should record guard activation");
    assert_eq!(observed.installation_status, "active");

    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_configured_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_configured_observed",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_configured_observed",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_guarded_configured_observed",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["close_state"], "ready");
    assert_no_close_blocker(&response.response_value, "guard_not_observed");
    assert_eq!(
        response.response_value["guard_health"]["guard_configuration_status"],
        "configured"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_observation_status"],
        "observed"
    );
    assert_eq!(
        response.response_value["guard_health"]["effective_guard_status"],
        "active"
    );
    Ok(())
}

#[test]
fn guarded_degraded_installation_with_valid_event_still_blocks_missing_required_hooks(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let degraded_capability = json!({
        "schema": "volicord-guard-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_guard_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": ["pre_tool_hook"],
        "prompt_capture": true
    })
    .to_string();
    let guard_installation_id = record_guard_installation(
        &harness,
        "guarded_degraded_observed",
        "guarded",
        "degraded",
        &degraded_capability,
    )?;
    let observed = observe_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationObservation {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "guarded".to_owned(),
            observed_policy_hash: "sha256:guardedfixture".to_owned(),
            observed_binary_version: Some("0.0.0-test".to_owned()),
            observed_phase: "session_start".to_owned(),
            observed_at: "2026-06-30T00:03:00Z".to_owned(),
        },
    )?
    .expect("matching degraded observation should record metadata");
    assert_eq!(observed.installation_status, "degraded");
    assert_eq!(observed.last_seen_phase.as_deref(), Some("session_start"));

    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_degraded_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_degraded_observed",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_degraded_observed",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_guarded_degraded_observed",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_required_hooks_missing");
    assert_no_close_blocker(&response.response_value, "guard_not_observed");
    let blocker = close_blocker_by_code(&response.response_value, "guard_required_hooks_missing");
    assert!(blocker["message"]
        .as_str()
        .is_some_and(|message| message.contains("pre_tool_hook") && message.contains("codex")));
    assert_eq!(
        response.response_value["guard_health"]["guard_configuration_status"],
        "degraded"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_observation_status"],
        "observed"
    );
    assert_eq!(
        response.response_value["guard_health"]["effective_guard_status"],
        "degraded"
    );
    assert_eq!(
        response.response_value["guard_health"]["missing_required_hook_phases"],
        json!(["pre_tool_hook"])
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "authority_record_only"
    );
    assert_eq!(
        response.response_value["guard_health"]["pre_tool_blocking_available"],
        false
    );
    let blocker = close_blocker_by_code(&response.response_value, "guard_required_hooks_missing");
    assert_eq!(blocker["guard_strength"], "authority_record_only");
    Ok(())
}

#[test]
fn guarded_partial_required_phase_configuration_with_event_still_blocks_missing_required_hooks(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let partial_capability = json!({
        "schema": "volicord-guard-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_guard_phases": ["session_start_hook"],
        "missing_required_hooks": [],
        "prompt_capture": true
    })
    .to_string();
    let guard_installation_id = record_guard_installation(
        &harness,
        "guarded_partial_observed",
        "guarded",
        "configured",
        &partial_capability,
    )?;
    let observed = observe_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationObservation {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "guarded".to_owned(),
            observed_policy_hash: "sha256:guardedfixture".to_owned(),
            observed_binary_version: Some("0.0.0-test".to_owned()),
            observed_phase: "session_start".to_owned(),
            observed_at: "2026-06-30T00:03:00Z".to_owned(),
        },
    )?
    .expect("matching partial observation should record metadata");
    assert_eq!(observed.installation_status, "configured");

    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_partial_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_partial_observed",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_partial_observed",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_guarded_partial_observed",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_required_hooks_missing");
    assert_eq!(
        response.response_value["guard_health"]["guard_configuration_status"],
        "degraded"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_observation_status"],
        "observed"
    );
    assert_eq!(
        response.response_value["guard_health"]["effective_guard_status"],
        "degraded"
    );
    let missing = response.response_value["guard_health"]["missing_required_hook_phases"]
        .as_array()
        .expect("missing required hook phases should be an array");
    assert_eq!(missing.len(), 4);
    assert!(missing.iter().any(|phase| phase == "pre_tool_hook"));
    assert!(missing.iter().all(|phase| phase != "session_start_hook"));
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "authority_record_only"
    );
    Ok(())
}

#[test]
fn guarded_close_blocks_missing_guard_installation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    insert_guarded_agent_session(&harness, "guarded_missing_install", "guarded")?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_missing_install")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_missing_install",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_missing_install",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_missing_install",
            idempotency_key: Some("idem_close_guarded_missing_install"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_not_installed");
    assert_close_blocker_category(
        &response.response_value,
        "guard_not_installed",
        "connection_capability",
    );
    assert_close_blocker_resolution(&response.response_value, "guard_not_installed", false, true);
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_status"],
        "absent"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_id"],
        Value::Null
    );
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_close_blocks_stale_broken_and_degraded_guard_status() -> Result<(), Box<dyn Error>> {
    for (status, code) in [
        ("stale", "guard_stale"),
        ("broken", "guard_broken"),
        ("degraded", "guard_degraded"),
    ] {
        let harness = MethodHarness::new()?;
        record_guard_installation(
            &harness,
            &format!("guarded_{status}"),
            "guarded",
            status,
            "{}",
        )?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("guarded_{status}"))?;
        let after_evidence = record_close_evidence(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            &format!("guarded_{status}"),
            true,
        )?;
        let after_final = record_final_acceptance(
            &harness,
            &task_id,
            &change_unit_id,
            after_evidence,
            &format!("guarded_{status}"),
        )?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: &format!("req_close_guarded_{status}"),
                idempotency_key: Some(&format!("idem_close_guarded_{status}")),
                dry_run: false,
                expected_state_version: Some(after_final),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(OperationCategory::AgentWorkflow),
        )?;

        assert_eq!(response.response_value["close_state"], "blocked");
        assert_close_blocker(&response.response_value, code);
        assert_close_blocker_category(&response.response_value, code, "connection_capability");
        assert_close_blocker_resolution(&response.response_value, code, false, true);
        assert_eq!(
            response.response_value["guard_health"]["guard_installation_status"],
            status
        );
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(harness.counts()?, before);
    }
    Ok(())
}

#[test]
fn guarded_close_blocks_unresolved_unrecorded_changes_and_check_is_read_only(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "guarded_unrecorded", "guarded", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_unrecorded")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_unrecorded",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_unrecorded",
    )?;
    insert_guarded_unrecorded_change(&harness, &task_id, "guarded_unrecorded")?;
    let before = harness.counts()?;

    let check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_guarded_unrecorded",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(check.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_close_blocker(&check.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        check.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    assert!(!check.response_json.contains("src/export.rs"));
    assert_eq!(harness.counts()?, before);

    let complete = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_unrecorded",
            idempotency_key: Some("idem_close_guarded_unrecorded"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(complete.response_value["close_state"], "blocked");
    assert_close_blocker(&complete.response_value, "unresolved_unrecorded_changes");
    assert_eq!(complete.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn reconcile_changes_resolves_not_product_change_and_updates_close_blocker(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_not_product", "guarded", "active", "{}")?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "reconcile_not_product")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "reconcile_not_product",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "reconcile_not_product",
    )?;
    let unrecorded_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_not_product",
        "[]",
    )?;

    let before = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_reconcile_not_product_check_before",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_close_blocker(&before.response_value, "unresolved_unrecorded_changes");
    assert_close_blocker_resolution(
        &before.response_value,
        "unresolved_unrecorded_changes",
        true,
        false,
    );
    let blocker = close_blocker_by_code(&before.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        blocker["next_actions"][0]["owner_method"],
        "volicord.reconcile_changes"
    );
    assert_eq!(
        blocker["next_actions"][0]["action_kind"],
        "reconcile_changes"
    );

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_not_product",
            "idem_reconcile_not_product",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "not_product_change"
    );
    assert_eq!(
        response.response_value["resolved_changes"][0]["resolved_by_actor_source"],
        "system"
    );
    assert!(
        response.response_value["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .is_empty(),
        "deterministically resolved reconciliation should not leave stale next actions: {:?}",
        response.response_value["next_actions"]
    );
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    let resolution = row_resolution(&row);
    assert_eq!(resolution["resolution_basis"], "not_product_change");
    assert_eq!(
        resolution["capture_basis"],
        "core_deterministic_not_product_change"
    );

    let after = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_reconcile_not_product_check_after",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_no_close_blocker(&after.response_value, "unresolved_unrecorded_changes");
    Ok(())
}

#[test]
fn reconcile_changes_accepts_local_recovery_and_persists_replay_category(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_local", "guarded", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_local")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change_with_paths(&harness, &task_id, "reconcile_local", "[]")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local",
            "idem_reconcile_local",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "not_product_change"
    );
    assert_eq!(
        response
            .verified_invocation
            .as_ref()
            .expect("local recovery should verify invocation")
            .operation_category,
        OperationCategory::LocalRecovery
    );
    assert_eq!(
        response
            .verified_invocation
            .as_ref()
            .expect("local recovery should verify invocation")
            .actor_source,
        ActorSource::LocalUser
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "resolved"
    );

    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let replay = store
        .tool_invocation(
            MethodName::ReconcileChanges,
            &IdempotencyKey::new("idem_reconcile_local"),
        )?
        .expect("local recovery commit should persist replay row");
    assert_eq!(replay.actor_source, LOCAL_USER_ACTOR_SOURCE);
    assert_eq!(replay.operation_category, "local_recovery");
    assert_eq!(
        replay.verification_basis.as_deref(),
        Some(VERIFICATION_BASIS_TEST_FIXTURE_BINDING)
    );
    Ok(())
}

#[test]
fn reconcile_changes_local_recovery_reports_no_unresolved_findings_read_only(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_none")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_none",
            "idem_reconcile_none",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert!(response.response_value["unresolved_changes"]
        .as_array()
        .expect("unresolved_changes should be an array")
        .is_empty());
    assert!(response.response_value["resolved_changes"]
        .as_array()
        .expect("resolved_changes should be an array")
        .is_empty());
    assert!(response.response_value["pending_user_judgment_refs"]
        .as_array()
        .expect("pending refs should be an array")
        .is_empty());
    assert_eq!(
        response
            .verified_invocation
            .as_ref()
            .expect("local recovery should verify invocation")
            .actor_source,
        ActorSource::LocalUser
    );
    Ok(())
}

#[test]
fn reconcile_changes_creates_and_consumes_user_acceptance_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_accept", "guarded", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_accept")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_accept")?;

    let first = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_accept_first",
            "idem_reconcile_accept_first",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        first.response_value["unresolved_changes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        first.response_value["pending_user_judgment_refs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    let judgment_id = first.response_value["pending_user_judgment_refs"][0]["record_id"]
        .as_str()
        .expect("pending judgment ref should be present")
        .to_owned();
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let recorded = harness.service.record_user_judgment(
        record_judgment_request(
            "req_reconcile_accept_record",
            "idem_reconcile_accept_record",
            Some(after_first),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let second = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_accept_second",
            "idem_reconcile_accept_second",
            Some(after_record),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        second.response_value["resolved_changes"][0]["resolution_basis"],
        "accepted_by_user"
    );
    assert_eq!(
        second.response_value["resolved_changes"][0]["resolved_by_actor_source"],
        "local_user"
    );
    assert!(second.response_value["pending_user_judgment_refs"]
        .as_array()
        .expect("pending refs should be an array")
        .is_empty());
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    let resolution = row_resolution(&row);
    assert_eq!(resolution["resolution_basis"], "accepted_by_user");
    assert_eq!(
        resolution["capture_basis"],
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING
    );
    assert_eq!(
        resolution["user_judgment_ref"]["record_id"],
        judgment_id.as_str()
    );
    assert_eq!(
        row.resolved_by_actor_source.as_deref(),
        Some(LOCAL_USER_ACTOR_SOURCE)
    );
    Ok(())
}

#[test]
fn reconcile_changes_local_recovery_consumes_user_acceptance_and_removes_close_blocker(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "reconcile_local_accept",
        "guarded",
        "active",
        "{}",
    )?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "reconcile_local_accept")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "reconcile_local_accept",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "reconcile_local_accept",
    )?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_local_accept")?;

    let before = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_reconcile_local_accept_before",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_close_blocker(&before.response_value, "unresolved_unrecorded_changes");

    let first = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local_accept_first",
            "idem_reconcile_local_accept_first",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;
    let judgment_id = first.response_value["pending_user_judgment_refs"][0]["record_id"]
        .as_str()
        .expect("pending judgment ref should be present")
        .to_owned();
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let recorded = harness.service.record_user_judgment(
        record_judgment_request(
            "req_reconcile_local_accept_record",
            "idem_reconcile_local_accept_record",
            Some(after_first),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let second = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local_accept_second",
            "idem_reconcile_local_accept_second",
            Some(after_record),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_eq!(
        second.response_value["resolved_changes"][0]["resolution_basis"],
        "accepted_by_user"
    );
    assert_eq!(
        second.response_value["resolved_changes"][0]["resolved_by_actor_source"],
        LOCAL_USER_ACTOR_SOURCE
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    assert_eq!(
        row.resolved_by_actor_source.as_deref(),
        Some(LOCAL_USER_ACTOR_SOURCE)
    );
    let resolution = row_resolution(&row);
    assert_eq!(
        resolution["capture_basis"],
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING
    );

    let after = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_reconcile_local_accept_after",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_no_close_blocker(&after.response_value, "unresolved_unrecorded_changes");
    Ok(())
}

#[test]
fn reconcile_changes_rejects_agent_supplied_system_resolution_basis() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_reject", "guarded", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_reject")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_reject")?;

    let seed = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_reject_seed",
            "idem_reconcile_reject_seed",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_seed = seed.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_reject_basis",
            "idem_reconcile_reject_basis",
            Some(after_seed),
            &task_id,
            vec![UnrecordedChangeResolutionRequest {
                unrecorded_change_id: UnrecordedChangeId::new(unrecorded_change_id.clone()),
                basis: UnrecordedChangeResolutionBasis::InvalidObservation,
                user_judgment_id: RequiredNullable::null(),
            }],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["rejected_resolution_requests"][0]["code"],
        "system_resolution_basis_not_caller_owned"
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn reconcile_changes_rejects_agent_direct_accepted_by_user_without_judgment(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "reconcile_agent_accept",
        "guarded",
        "active",
        "{}",
    )?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_agent_accept")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_agent_accept")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_agent_accept",
            "idem_reconcile_agent_accept",
            Some(2),
            &task_id,
            vec![UnrecordedChangeResolutionRequest {
                unrecorded_change_id: UnrecordedChangeId::new(unrecorded_change_id.clone()),
                basis: UnrecordedChangeResolutionBasis::AcceptedByUser,
                user_judgment_id: Some(UserJudgmentId::new("judgment_missing_accept")).into(),
            }],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["rejected_resolution_requests"][0]["code"],
        "user_judgment_not_accepted"
    );
    assert_eq!(
        response.response_value["unresolved_changes"][0]["unrecorded_change_ref"]["record_id"],
        unrecorded_change_id
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn reconcile_changes_rejects_mismatched_invocation_project() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_project_mismatch")?;
    let other_project_id = register_additional_project(&harness, "project_methods_mismatch")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_project_mismatch",
            "idem_reconcile_project_mismatch",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        InvocationContext::new(
            ProjectId::new(other_project_id),
            ActorSource::LocalUser,
            OperationCategory::LocalRecovery,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        ),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "envelope.project_id"
    );
    Ok(())
}

#[test]
fn reconcile_changes_resolves_invalid_observation_deterministically() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_invalid", "guarded", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_invalid")?;
    let unrecorded_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_invalid",
        "[123]",
    )?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_invalid",
            "idem_reconcile_invalid",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "invalid_observation"
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    assert_eq!(
        row_resolution(&row)["capture_basis"],
        "core_deterministic_invalid_observation"
    );
    Ok(())
}

#[test]
fn reconcile_changes_isolates_other_projects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_cross", "guarded", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_cross")?;
    let main_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_cross_main",
        "[]",
    )?;
    let other_project_id = register_additional_project(&harness, "project_methods_other")?;
    let other_change_id = insert_project_unrecorded_change(
        &harness,
        &other_project_id,
        None,
        "reconcile_cross_other",
        "[]",
    )?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_cross",
            "idem_reconcile_cross",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["unrecorded_change_ref"]["record_id"],
        main_change_id
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &main_change_id)?.status,
        "resolved"
    );
    assert_eq!(
        unrecorded_change_row(&harness, &other_project_id, &other_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn guarded_close_blocks_write_readiness_issue_from_guard_event() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "guarded_write_ready", "guarded", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_write_ready")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_write_ready",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_write_ready",
    )?;
    insert_write_readiness_guard_event(&harness, &guard_installation_id, "guarded_write_ready")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_guarded_write_ready",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value,
        "guard_write_readiness_missing_or_stale",
    );
    assert_eq!(
        response.response_value["guard_health"]["missing_or_stale_write_readiness"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["last_guard_event_at"],
        "2026-06-30T00:06:00Z"
    );
    assert_eq!(harness.counts()?, before);

    let complete = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_write_ready",
            idempotency_key: Some("idem_close_guarded_write_ready"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(complete.response_value["close_state"], "blocked");
    assert_eq!(complete.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_pending_judgment_displays_user_answer_paths() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "guarded_pending", "guarded", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_pending")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_pending",
        true,
    )?;
    let mut product_request = user_judgment_request(
        "req_guarded_pending_judgment",
        "idem_guarded_pending_judgment",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for = vec![volicord_types::JudgmentRequiredFor::CloseComplete];
    harness.service.request_user_judgment(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "guarded_pending",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_pending",
            idempotency_key: Some("idem_close_guarded_pending"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "pending_user_judgment");
    let pending = response.response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .find(|blocker| blocker["code"] == "pending_user_judgment")
        .expect("pending judgment blocker should be present");
    let guidance = pending["next_actions"][0]["blocking_question"]
        .as_str()
        .expect("pending blocker should include answer-path guidance");
    assert!(guidance.contains("MCP elicitation"), "{guidance}");
    assert!(!guidance.contains("prompt-capture"), "{guidance}");
    assert!(!guidance.contains("volicord user"), "{guidance}");
    assert_eq!(
        response.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_pending_judgment_uses_prompt_capture_guidance_when_mcp_unhealthy(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    set_method_harness_connection_verification_status(&harness, VERIFIED_STATUS_FAILED)?;
    record_guard_installation(
        &harness,
        "guarded_pending_prompt_capture",
        "guarded",
        "active",
        "{}",
    )?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_pending_prompt_capture")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_pending_prompt_capture",
        true,
    )?;
    let mut product_request = user_judgment_request(
        "req_guarded_pending_prompt_capture_judgment",
        "idem_guarded_pending_prompt_capture_judgment",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for = vec![volicord_types::JudgmentRequiredFor::CloseComplete];
    harness.service.request_user_judgment(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "guarded_pending_prompt_capture",
    )?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_pending_prompt_capture",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["close_state"], "blocked");
    assert_pending_judgment_prompt_capture_guidance(&status.response_value);

    let check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_check_guarded_pending_prompt_capture",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_pending_judgment_prompt_capture_guidance(&check.response_value);
    assert_eq!(
        check.response_value["guard_health"]["mcp_connection_healthy"],
        false
    );
    assert_eq!(
        check.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(
        check.response_value["guard_health"]["prompt_capture_status"],
        "observed"
    );
    Ok(())
}

#[test]
fn mcp_only_close_does_not_receive_guarded_unrecorded_change_blocker() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "mcp_only_unrecorded",
        "mcp_only",
        "configured",
        "{}",
    )?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "mcp_only_unrecorded")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "mcp_only_unrecorded",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "mcp_only_unrecorded",
    )?;
    insert_guarded_unrecorded_change(&harness, &task_id, "mcp_only_unrecorded")?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_mcp_only_unrecorded",
            idempotency_key: Some("idem_close_mcp_only_unrecorded"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_no_close_blocker(&response.response_value, "guard_not_observed");
    assert_eq!(
        response.response_value["guard_health"]["guard_mode"],
        "mcp_only"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "authority_record_only"
    );
    assert_eq!(
        response.response_value["guard_health"]["pre_tool_blocking_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_status"],
        "configured"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_hook_observed"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    assert_eq!(
        response.response_value["guard_health"]["prompt_capture_available"],
        false
    );
    Ok(())
}

#[test]
fn mcp_only_watcher_detects_bypass_file_changes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "watch_mcp_only", "mcp_only", "configured", "{}")?;
    let (task_id, _, _) = create_close_ready_task(&harness, "watch_mcp_only")?;
    let session_id = "session_watch_mcp_only";
    initialize_watch_baseline(&harness, &task_id, session_id, "mcp_only_seed")?;

    write_product_file(&harness, "src/watch.txt", "changed outside guard\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_mcp_only_detect",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::Read, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["guard_mode"],
        "mcp_only"
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_status"],
        "active"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_strength"],
        "authority_record_only"
    );
    assert_eq!(
        response.response_value["guard_health"]["pre_tool_blocking_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_coverage_basis"],
        "method_boundary"
    );
    assert!(
        response.response_value["guard_health"]["session_watch_partial_coverage_warning"]
            .as_str()
            .unwrap_or_default()
            .contains("method boundary")
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    let blocker = close_blocker_by_code(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(blocker["guard_strength"], "authority_record_only");
    let changes = unresolved_changes_for_connection(&harness)?;
    assert_eq!(changes.len(), 1);
    let detection: Value = serde_json::from_str(&changes[0].detection_json)?;
    assert_eq!(detection["source"], "volicord_session_watch");
    assert_eq!(detection["does_not_prevent_writes"], true);
    assert_eq!(detection["does_not_identify_actor"], true);
    assert!(!changes[0].detection_json.contains("changed outside guard"));
    Ok(())
}

#[test]
fn guarded_expected_write_does_not_create_duplicate_watcher_blocker() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "watch_expected", "guarded", "active", "{}")?;
    let (task_id, change_unit_id, _) = create_close_ready_task(&harness, "watch_expected")?;
    let session_id = "session_watch_expected";
    initialize_watch_baseline(&harness, &task_id, session_id, "expected_seed")?;
    insert_expected_write_for_paths(
        &harness,
        &guard_installation_id,
        session_id,
        &task_id,
        &change_unit_id,
        "watch_expected",
        &["src/watch.txt"],
    )?;

    write_product_file(&harness, "src/watch.txt", "covered guarded write\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_expected_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::Read, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "ready");
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["session_watch_status"],
        "active"
    );
    assert!(unresolved_changes_for_connection(&harness)?.is_empty());
    Ok(())
}

#[test]
fn guarded_hook_missing_write_is_detected_by_watcher() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "watch_guarded_fallback",
        "guarded",
        "active",
        "{}",
    )?;
    let (task_id, _, _) = create_close_ready_task(&harness, "watch_guarded_fallback")?;
    let session_id = "session_watch_guarded_fallback";
    initialize_watch_baseline(&harness, &task_id, session_id, "guarded_fallback_seed")?;

    write_product_file(&harness, "src/watch.txt", "guard hook skipped this write\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_guarded_fallback",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::Read, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["guard_mode"],
        "guarded"
    );
    assert_eq!(unresolved_changes_for_connection(&harness)?.len(), 1);
    Ok(())
}

#[test]
fn watcher_reverted_change_auto_resolves() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "watch_revert", "mcp_only", "configured", "{}")?;
    let (task_id, _, after_final) = create_close_ready_task(&harness, "watch_revert")?;
    let session_id = "session_watch_revert";
    write_product_file(&harness, "src/watch.txt", "original\n")?;
    initialize_watch_baseline(&harness, &task_id, session_id, "revert_seed")?;
    write_product_file(&harness, "src/watch.txt", "changed\n")?;
    let blocked = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_revert_detect",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::Read, session_id),
    )?;
    assert_close_blocker(&blocked.response_value, "unresolved_unrecorded_changes");

    write_product_file(&harness, "src/watch.txt", "original\n")?;
    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_watch_revert_reconcile",
            "idem_watch_revert_reconcile",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "reverted"
    );
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    let changes = unresolved_changes_for_connection(&harness)?;
    assert!(changes.is_empty());
    Ok(())
}

#[test]
fn close_blocks_while_watcher_findings_remain_unresolved_and_unblocks_after_reconciliation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "watch_close_block",
        "mcp_only",
        "configured",
        "{}",
    )?;
    let (task_id, _, after_final) = create_close_ready_task(&harness, "watch_close_block")?;
    let session_id = "session_watch_close_block";
    write_product_file(&harness, "src/watch.txt", "original\n")?;
    initialize_watch_baseline(&harness, &task_id, session_id, "close_block_seed")?;
    write_product_file(&harness, "src/watch.txt", "changed\n")?;

    let blocked = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_close_block",
            idempotency_key: Some("idem_watch_close_block"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    assert_eq!(blocked.response_value["close_state"], "blocked");
    assert_close_blocker(&blocked.response_value, "unresolved_unrecorded_changes");

    write_product_file(&harness, "src/watch.txt", "original\n")?;
    let reconciled = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_watch_close_block_reconcile",
            "idem_watch_close_block_reconcile",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    let after_reconcile = reconciled.response_value["base"]["state_version"]
        .as_u64()
        .expect("reconcile should report state version");
    assert_no_close_blocker(&reconciled.response_value, "unresolved_unrecorded_changes");

    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_close_unblocked",
            idempotency_key: Some("idem_watch_close_unblocked"),
            dry_run: false,
            expected_state_version: Some(after_reconcile),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    assert_no_close_blocker(&closed.response_value, "unresolved_unrecorded_changes");
    Ok(())
}

#[test]
fn close_task_cancel_success_despite_missing_completion_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_cancel")?;
    let (after_authority, _) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "close_cancel",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_cancel",
            idempotency_key: Some("idem_close_cancel"),
            dry_run: false,
            expected_state_version: Some(after_authority),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "cancelled");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(fields.lifecycle_phase, "cancelled");
    assert_eq!(fields.result.as_deref(), Some("cancelled"));
    assert_eq!(fields.close_summary["close_reason"], "cancelled");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_cancel_requires_current_user_cancellation_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "cancel_missing_authority")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_missing_authority",
            idempotency_key: Some("idem_cancel_missing_authority"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_cancellation_authority");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "ready"
    );
    Ok(())
}

#[test]
fn rejected_cancellation_authority_does_not_cancel_task() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cancel_rejected")?;
    let (after_rejection, judgment_id) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "cancel_rejected",
        false,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_rejected",
            idempotency_key: Some("idem_cancel_rejected"),
            dry_run: false,
            expected_state_version: Some(after_rejection),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        user_judgment_resolution_outcome(&harness, &judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "cancellation_rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_change_stales_cancellation_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cancel_stale_scope")?;
    let (after_authority, judgment_id) =
        record_cancellation_authority(&harness, &task_id, &change_unit_id, 2, "stale_scope", true)?;
    let scope = harness.service.update_scope(
        update_scope_request(
            "req_cancel_stale_scope_update",
            "idem_cancel_stale_scope_update",
            false,
            Some(after_authority),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope after cancellation judgment.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_scope = scope.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "stale");
    assert_eq!(user_judgment_basis_status(&harness, &judgment_id)?, "stale");
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_stale_scope",
            idempotency_key: Some("idem_cancel_stale_scope"),
            dry_run: false,
            expected_state_version: Some(after_scope),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "cancellation_judgment_stale");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_supersede_success() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_supersede")?;
    let superseding_task_id = "task_close_superseding";
    insert_superseding_task(&harness, superseding_task_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_supersede",
            idempotency_key: Some("idem_close_supersede"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Supersede,
            close_reason: Some(CloseReason::Superseded),
            superseding_task_id: Some(superseding_task_id),
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "superseded");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(fields.lifecycle_phase, "superseded");
    assert_eq!(fields.result.as_deref(), Some("superseded"));
    assert_eq!(
        active_task_id(&harness)?.as_deref(),
        Some(superseding_task_id)
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_stale_state_rejected_without_blocker() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_stale")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_stale",
            idempotency_key: Some("idem_close_stale"),
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response.response_value.get("blockers").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_blocker_code_routing_uses_method_local_codes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_codes")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_codes",
            idempotency_key: Some("idem_close_codes"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
    assert!(response.response_value.get("errors").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_idempotency_replays_terminal_transition() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_replay")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "replay", true)?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "replay",
    )?;
    let request = close_task_request(CloseTaskFixture {
        request_id: "req_close_replay",
        idempotency_key: Some("idem_close_replay"),
        dry_run: false,
        expected_state_version: Some(after_final),
        task_id: &task_id,
        intent: CloseIntent::Complete,
        close_reason: Some(CloseReason::CompletedSelfChecked),
        superseding_task_id: None,
    });

    let first = harness.service.close_task(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .close_task(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(first.response_value["close_state"], "closed");
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

fn envelope(
    request_id: &str,
    idempotency_key: Option<&str>,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: Option<&str>,
) -> ToolEnvelope {
    ToolEnvelope {
        project_id: ProjectId::new(PROJECT_ID),
        task_id: task_id.map(TaskId::new).into(),
        request_id: RequestId::new(request_id),
        idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
        expected_state_version: expected_state_version.into(),
        dry_run,
        locale: None.into(),
    }
}

fn invocation(operation_category: OperationCategory) -> InvocationContext {
    invocation_with_actor(
        actor_source_for_operation_category(operation_category),
        operation_category,
    )
}

fn invocation_with_session(
    operation_category: OperationCategory,
    session_id: &str,
) -> InvocationContext {
    invocation(operation_category).with_session_id(session_id.to_owned())
}

fn actor_source_for_operation_category(operation_category: OperationCategory) -> ActorSource {
    match operation_category {
        OperationCategory::Read | OperationCategory::AgentWorkflow => {
            ActorSource::agent_connection(CONNECTION_ID)
        }
        OperationCategory::UserOnly
        | OperationCategory::AdminLocal
        | OperationCategory::LocalRecovery => ActorSource::LocalUser,
    }
}

fn invocation_with_actor(
    actor_source: ActorSource,
    operation_category: OperationCategory,
) -> InvocationContext {
    InvocationContext::new(
        ProjectId::new(PROJECT_ID),
        actor_source,
        operation_category,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    )
}

fn create_close_ready_task(
    harness: &MethodHarness,
    suffix: &str,
) -> Result<(String, String, u64), Box<dyn Error>> {
    let (task_id, change_unit_id) = create_task_with_change_unit(harness, suffix)?;
    let after_evidence =
        record_close_evidence(harness, &task_id, &change_unit_id, 2, suffix, true)?;
    let after_final =
        record_final_acceptance(harness, &task_id, &change_unit_id, after_evidence, suffix)?;
    Ok((task_id, change_unit_id, after_final))
}

fn initialize_watch_baseline(
    harness: &MethodHarness,
    task_id: &str,
    session_id: &str,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    let request_id = format!("req_watch_baseline_{suffix}");
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: &request_id,
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::Read, session_id),
    )?;
    assert_eq!(
        response.response_value["guard_health"]["session_watch_status"],
        "active"
    );
    Ok(())
}

fn product_repo_root(harness: &MethodHarness) -> Result<PathBuf, Box<dyn Error>> {
    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    Ok(store.project_record().repo_root.clone())
}

fn write_product_file(
    harness: &MethodHarness,
    path: &str,
    contents: &str,
) -> Result<(), Box<dyn Error>> {
    let absolute = product_repo_root(harness)?.join(path);
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(absolute, contents)?;
    Ok(())
}

fn unresolved_changes_for_connection(
    harness: &MethodHarness,
) -> Result<Vec<UnrecordedChangeRecord>, Box<dyn Error>> {
    Ok(list_unresolved_unrecorded_changes(
        &harness.runtime_home_path,
        PROJECT_ID,
        Some(CONNECTION_ID),
    )?)
}

fn insert_expected_write_for_paths(
    harness: &MethodHarness,
    guard_installation_id: &str,
    session_id: &str,
    task_id: &str,
    change_unit_id: &str,
    suffix: &str,
    expected_paths: &[&str],
) -> Result<String, Box<dyn Error>> {
    let expected_write_id = format!("expected_write_{suffix}");
    let expected_paths = expected_paths
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    insert_expected_write(
        &harness.runtime_home_path,
        PROJECT_ID,
        ExpectedWriteInsert {
            expected_write_id: expected_write_id.clone(),
            session_id: Some(session_id.to_owned()),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: Some(guard_installation_id.to_owned()),
            pre_tool_guard_event_id: format!("guard_event_pre_tool_{suffix}"),
            host_invocation_id: Some(format!("host_invocation_{suffix}")),
            tool_name: Some("fixture_tool".to_owned()),
            command_kind: "product_file_write".to_owned(),
            path_policy: "exact_paths".to_owned(),
            expected_paths_json: serde_json::to_string(&expected_paths)?,
            task_id: task_id.to_owned(),
            change_unit_id: Some(change_unit_id.to_owned()),
            write_check_ids_json: "[]".to_owned(),
            basis_state_version: 2,
            created_at: "2026-06-30T00:07:00Z".to_owned(),
            expires_at: "2026-06-30T01:07:00Z".to_owned(),
            metadata_json: serde_json::to_string(&json!({
                "source": "test_fixture"
            }))?,
        },
    )?;
    Ok(expected_write_id)
}

fn assert_verified_invocation(response: &PipelineResponse, operation_category: OperationCategory) {
    let verified = response
        .verified_invocation
        .as_ref()
        .expect("method response should carry verified invocation context");
    assert_eq!(verified.project_id.as_str(), PROJECT_ID);
    assert_eq!(
        verified.actor_source,
        actor_source_for_operation_category(operation_category)
    );
    assert_eq!(verified.operation_category, operation_category);
    assert_eq!(
        verified.verification_basis,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING
    );
}

fn assert_store_rejection(
    response: &PipelineResponse,
    expected_code: &str,
    expected_category: &str,
) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(response.response_value["errors"][0]["code"], expected_code);
    assert_eq!(
        response.response_value["errors"][0]["details"]["store_failure_category"],
        expected_category
    );
}

fn assert_owner_state_rejection(
    response: &PipelineResponse,
    table: &str,
    record_ref: &str,
    logical_column: &str,
    runtime_home_path: &Path,
) {
    assert_owner_state_rejection_with_category(
        response,
        table,
        record_ref,
        logical_column,
        "corrupt_stored_json",
        runtime_home_path,
    )
}

fn assert_owner_state_value_rejection(
    response: &PipelineResponse,
    table: &str,
    record_ref: &str,
    logical_column: &str,
    runtime_home_path: &Path,
) {
    assert_owner_state_rejection_with_category(
        response,
        table,
        record_ref,
        logical_column,
        "corrupt_stored_value",
        runtime_home_path,
    )
}

fn assert_owner_state_rejection_with_category(
    response: &PipelineResponse,
    table: &str,
    record_ref: &str,
    logical_column: &str,
    corruption_category: &str,
    runtime_home_path: &Path,
) {
    assert_store_rejection(response, "MCP_UNAVAILABLE", corruption_category);
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    let details = &response.response_value["errors"][0]["details"];
    assert_eq!(details["owner_state_error"]["table"], table);
    assert_eq!(details["owner_state_error"]["record_ref"], record_ref);
    assert_eq!(
        details["owner_state_error"]["logical_column"],
        logical_column
    );
    assert_eq!(
        details["owner_state_error"]["corruption_category"],
        corruption_category
    );
    assert!(!response.response_json.contains(corrupt_owner_json()));
    assert!(!response
        .response_json
        .contains("/tmp/volicord-redaction-secret"));
    assert_public_response_has_no_internal_leak(response, runtime_home_path);
}

fn assert_public_response_omits(response: &PipelineResponse, fragment: &str) {
    assert!(
        !response.response_json.contains(fragment),
        "public response leaked forbidden fragment {fragment}: {}",
        response.response_json
    );
}

fn assert_constraint_error(error: rusqlite::Error) {
    match error {
        rusqlite::Error::SqliteFailure(err, _) => assert_eq!(
            err.code,
            rusqlite::ErrorCode::ConstraintViolation,
            "expected SQLite constraint error, got {err:?}"
        ),
        other => panic!("expected SQLite constraint error, got {other:?}"),
    }
}

fn assert_public_response_has_no_internal_leak(
    response: &PipelineResponse,
    runtime_home_path: &Path,
) {
    let body = &response.response_json;
    let runtime_home = runtime_home_path.to_string_lossy();
    assert!(!body.contains(runtime_home.as_ref()));
    for fragment in [
        "SELECT ",
        "INSERT INTO",
        "UPDATE ",
        "DELETE ",
        "constraint failed",
        "state.sqlite",
    ] {
        assert!(
            !body.contains(fragment),
            "public response leaked internal fragment {fragment}: {body}"
        );
    }
}

fn assert_write_check_invalid_reason(response: &PipelineResponse, reason: &str) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_CHECK_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_check_reason"],
        reason
    );
}

fn corrupt_owner_json() -> &'static str {
    "{not-json /tmp/volicord-redaction-secret"
}

fn status_include() -> StatusInclude {
    StatusInclude {
        task: true,
        pending_user_judgments: true,
        write_check: true,
        evidence: true,
        close: true,
        guarantees: true,
        continuity: false,
    }
}

fn intake_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    requested_mode: RequestedMode,
) -> volicord_types::IntakeRequest {
    volicord_types::IntakeRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            None,
        ),
        plain_language_request: "Create a test export flow.".to_owned(),
        requested_mode,
        resume_policy: ResumePolicy::CreateNew,
        initial_scope: InitialScope {
            boundary: "Initial test scope.".to_owned(),
            non_goals: vec!["Changing unrelated flows.".to_owned()],
            acceptance_criteria: vec!["The test export flow is represented.".to_owned()],
        },
        initial_context_refs: Vec::new(),
    }
}

fn update_scope_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    operation: ChangeUnitOperation,
    scope_summary: &str,
) -> UpdateScopeRequest {
    let mut fields = Map::new();
    fields.insert(
        "scope_summary".to_owned(),
        Value::String(scope_summary.to_owned()),
    );
    fields.insert(
        "affected_paths".to_owned(),
        json!(["src/export.rs", "tests/export.rs"]),
    );
    UpdateScopeRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        goal_summary: Some(scope_summary.to_owned()).into(),
        scope_update: Some(ScopeUpdate {
            include: vec![scope_summary.to_owned()],
            exclude: vec!["Unrelated behavior.".to_owned()],
        })
        .into(),
        scope_boundary: Some(scope_summary.to_owned()).into(),
        non_goals: Some(vec!["Unrelated behavior.".to_owned()]).into(),
        acceptance_criteria: Some(vec!["The scoped behavior is represented.".to_owned()]).into(),
        autonomy_boundary: Some("Stay inside the scoped test behavior.".to_owned()).into(),
        baseline_ref: Some(BaselineRef::new("baseline_test")).into(),
        change_unit: ChangeUnitUpdate {
            operation,
            effect_contract: None,
            fields,
        },
        related_scope_decision_refs: Vec::new(),
    }
}

fn prepare_write_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: Option<&str>,
    change_unit_id: Option<&str>,
) -> PrepareWriteRequest {
    PrepareWriteRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            false,
            expected_state_version,
            task_id,
        ),
        task_id: task_id.map(TaskId::new).into(),
        change_unit_id: change_unit_id.map(ChangeUnitId::new).into(),
        intended_operation: "local_sensitive_step".to_owned(),
        intended_paths: vec!["src/export.rs".to_owned()],
        product_file_write_intended: true,
        sensitive_categories: Vec::new(),
        baseline_ref: BaselineRef::new("baseline_test"),
    }
}

fn stage_artifact_request(
    request_id: &str,
    idempotency_key: Option<&str>,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
) -> StageArtifactRequest {
    StageArtifactRequest {
        envelope: envelope(
            request_id,
            idempotency_key,
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        display_name: "trace.log".to_owned(),
        content_type: "text/plain".to_owned(),
        redaction_state: RedactionState::None,
        safe_bytes_or_notice: "staging sample".to_owned(),
        expected_sha256: None.into(),
        expected_size_bytes: None.into(),
        relation_hint: Some("diagnostic_log".to_owned()).into(),
    }
}

fn record_run_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    change_unit_id: &str,
) -> RecordRunRequest {
    RecordRunRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: ChangeUnitId::new(change_unit_id),
        kind: volicord_types::RunKind::Implementation,
        run_id: None.into(),
        baseline_ref: BaselineRef::new("baseline_test"),
        write_check_id: None.into(),
        summary: "Recorded implementation run.".to_owned(),
        observed_changes: ObservedChanges {
            changed_paths: Vec::new(),
            product_file_write_observed: false,
            sensitive_categories: Vec::new(),
            baseline_ref: Some(BaselineRef::new("baseline_test")).into(),
        },
        artifact_inputs: Vec::new(),
        evidence_updates: Vec::new(),
        evidence_observations: Vec::new(),
        close_assessment: None.into(),
    }
}

fn product_write_record_run_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: u64,
    task_id: &str,
    change_unit_id: &str,
    write_check_id: &str,
    run_id: &str,
) -> RecordRunRequest {
    let mut request = record_run_request(
        request_id,
        idempotency_key,
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.run_id = Some(RunId::new(run_id)).into();
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_check_id = Some(WriteCheckId::new(write_check_id)).into();
    request
}

struct CloseTaskFixture<'a> {
    request_id: &'a str,
    idempotency_key: Option<&'a str>,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &'a str,
    intent: CloseIntent,
    close_reason: Option<CloseReason>,
    superseding_task_id: Option<&'a str>,
}

fn close_task_request(input: CloseTaskFixture<'_>) -> CloseTaskRequest {
    CloseTaskRequest {
        envelope: envelope(
            input.request_id,
            input.idempotency_key,
            input.dry_run,
            input.expected_state_version,
            Some(input.task_id),
        ),
        task_id: TaskId::new(input.task_id),
        intent: input.intent,
        close_reason: input.close_reason.into(),
        superseding_task_id: input.superseding_task_id.map(TaskId::new).into(),
        user_note: Some("Focused close-task test.".to_owned()).into(),
    }
}

fn reconcile_changes_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: &str,
    resolution_requests: Vec<UnrecordedChangeResolutionRequest>,
) -> ReconcileChangesRequest {
    ReconcileChangesRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            false,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        resolution_requests,
    }
}

fn record_guard_installation(
    harness: &MethodHarness,
    suffix: &str,
    guard_mode: &str,
    installation_status: &str,
    host_capability_json: &str,
) -> Result<String, Box<dyn Error>> {
    let guard_installation_id = format!("guard_installation_{suffix}");
    let host_capability_json = if host_capability_json == "{}" {
        complete_guard_capability_json()
    } else {
        host_capability_json.to_owned()
    };
    upsert_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationUpsert {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: Some(PROJECT_ID.to_owned()),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: guard_mode.to_owned(),
            host_capability_json,
            installation_status: installation_status.to_owned(),
            installed_at: Some("2026-06-30T00:00:00Z".to_owned()),
            last_checked_at: "2026-06-30T00:01:00Z".to_owned(),
            first_seen_at: (installation_status == "active")
                .then(|| "2026-06-30T00:02:00Z".to_owned()),
            last_seen_at: (installation_status == "active")
                .then(|| "2026-06-30T00:02:00Z".to_owned()),
            last_seen_phase: (installation_status == "active").then(|| "session_start".to_owned()),
            observed_host_kind: (installation_status == "active")
                .then(|| HOST_KIND_CODEX.to_owned()),
            observed_policy_hash: (installation_status == "active")
                .then(|| "sha256:guardedfixture".to_owned()),
            observed_binary_version: (installation_status == "active")
                .then(|| "0.0.0-test".to_owned()),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(guard_installation_id)
}

fn complete_guard_capability_json() -> String {
    json!({
        "schema": "volicord-guard-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_guard_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": [],
        "prompt_capture": true
    })
    .to_string()
}

fn insert_guarded_agent_session(
    harness: &MethodHarness,
    suffix: &str,
    guard_mode: &str,
) -> Result<(), Box<dyn Error>> {
    insert_agent_session(
        &harness.runtime_home_path,
        PROJECT_ID,
        AgentSessionInsert {
            session_id: format!("agent_session_{suffix}"),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: None,
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: guard_mode.to_owned(),
            started_at: "2026-06-30T00:02:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn insert_guarded_unrecorded_change(
    harness: &MethodHarness,
    task_id: &str,
    suffix: &str,
) -> Result<String, Box<dyn Error>> {
    insert_guarded_unrecorded_change_with_paths(harness, task_id, suffix, r#"["src/export.rs"]"#)
}

fn insert_guarded_unrecorded_change_with_paths(
    harness: &MethodHarness,
    task_id: &str,
    suffix: &str,
    observed_paths_json: &str,
) -> Result<String, Box<dyn Error>> {
    insert_project_unrecorded_change(
        harness,
        PROJECT_ID,
        Some(task_id.to_owned()),
        suffix,
        observed_paths_json,
    )
}

fn insert_project_unrecorded_change(
    harness: &MethodHarness,
    project_id: &str,
    task_id: Option<String>,
    suffix: &str,
    observed_paths_json: &str,
) -> Result<String, Box<dyn Error>> {
    let unrecorded_change_id = format!("unrecorded_change_{suffix}");
    insert_unrecorded_change(
        &harness.runtime_home_path,
        project_id,
        UnrecordedChangeInsert {
            unrecorded_change_id: unrecorded_change_id.clone(),
            session_id: None,
            connection_internal_id: CONNECTION_ID.to_owned(),
            task_id,
            summary: "Product Repository change observed outside a recorded run.".to_owned(),
            observed_paths_json: observed_paths_json.to_owned(),
            detection_json: "{}".to_owned(),
            detected_at: "2026-06-30T00:05:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(unrecorded_change_id)
}

fn register_additional_project(
    harness: &MethodHarness,
    project_id: &str,
) -> Result<String, Box<dyn Error>> {
    let repo_root = harness
        ._runtime_home
        .create_product_repo(format!("repo-{project_id}"))?;
    register_project(
        &harness.runtime_home_path,
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        &harness.runtime_home_path,
        ConnectionProjectRegistration {
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: project_id.to_owned(),
        },
    )?;
    Ok(project_id.to_owned())
}

fn unrecorded_change_row(
    harness: &MethodHarness,
    project_id: &str,
    unrecorded_change_id: &str,
) -> Result<UnrecordedChangeRecord, Box<dyn Error>> {
    unrecorded_change(&harness.runtime_home_path, project_id, unrecorded_change_id)?
        .ok_or_else(|| format!("missing unrecorded change {unrecorded_change_id}").into())
}

fn row_resolution(row: &UnrecordedChangeRecord) -> Value {
    serde_json::from_str(
        row.resolution_json
            .as_deref()
            .expect("resolved row should carry resolution_json"),
    )
    .expect("resolution_json should be valid JSON")
}

fn insert_write_readiness_guard_event(
    harness: &MethodHarness,
    guard_installation_id: &str,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    insert_guard_event(
        &harness.runtime_home_path,
        PROJECT_ID,
        GuardEventInsert {
            guard_event_id: format!("guard_event_{suffix}"),
            session_id: None,
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: Some(guard_installation_id.to_owned()),
            event_kind: "prepare_write".to_owned(),
            decision: "deny".to_owned(),
            subject_json: "{}".to_owned(),
            result_json: r#"{"reasons":[{"code":"write_readiness_missing"}]}"#.to_owned(),
            occurred_at: "2026-06-30T00:06:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn record_close_evidence(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    supported: bool,
) -> Result<u64, Box<dyn Error>> {
    record_close_evidence_with_updates(
        harness,
        task_id,
        change_unit_id,
        expected_state_version,
        suffix,
        vec![if supported {
            supported_evidence_update("Close claim supported.")
        } else {
            unsupported_evidence_update("Close claim supported.")
        }],
        "Close claim supported.",
    )
}

fn record_close_evidence_with_updates(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    evidence_updates: Vec<EvidenceCoverageItem>,
    result_summary: &str,
) -> Result<u64, Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    let request_id = format!("req_close_evidence_{suffix}");
    let idempotency_key = format!("idem_close_evidence_{suffix}");
    let mut request = record_run_request(
        &request_id,
        &idempotency_key,
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.evidence_updates = evidence_updates;
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: result_summary.to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present"))
}

fn record_close_basis_with_risks(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    residual_risks: Vec<volicord_types::ResidualRiskInput>,
) -> Result<(u64, Vec<String>), Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    let request_id = format!("req_close_risk_basis_{suffix}");
    let idempotency_key = format!("idem_close_risk_basis_{suffix}");
    let mut request = record_run_request(
        &request_id,
        &idempotency_key,
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.run_id = Some(RunId::new(format!("run_close_risk_basis_{suffix}"))).into();
    request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    request.close_assessment = Some(close_assessment_with_risks(
        "Close claim supported with visible residual risks.",
        residual_risks,
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let risk_ids = response.response_value["current_close_basis"]["residual_risks"]
        .as_array()
        .expect("residual_risks should be present")
        .iter()
        .map(|risk| {
            risk["risk_id"]
                .as_str()
                .expect("risk_id should be present")
                .to_owned()
        })
        .collect();
    Ok((state_version, risk_ids))
}

fn record_final_acceptance(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<u64, Box<dyn Error>> {
    Ok(record_final_acceptance_with_id(
        harness,
        task_id,
        change_unit_id,
        expected_state_version,
        suffix,
    )?
    .0)
}

fn record_final_acceptance_with_id(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<(u64, String), Box<dyn Error>> {
    let request_id = format!("req_close_final_{suffix}");
    let idempotency_key = format!("idem_close_final_{suffix}");
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            &request_id,
            &idempotency_key,
            false,
            Some(expected_state_version),
            task_id,
            Some(change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("user judgment ref should be present")
        .to_owned();
    let record_request_id = format!("req_close_final_record_{suffix}");
    let record_idempotency_key = format!("idem_close_final_record_{suffix}");
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            &record_request_id,
            &record_idempotency_key,
            Some(expected_state_version + 1),
            task_id,
            &judgment_id,
            JudgmentKind::FinalAcceptance,
            answer_payload(JudgmentKind::FinalAcceptance),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, judgment_id))
}

fn assert_final_acceptance_action_corruption<F>(
    suffix: &str,
    mutate: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&MethodHarness, &str) -> Result<(), Box<dyn Error>>,
{
    assert_final_acceptance_action_corruption_with(
        suffix,
        "resolution_machine_action",
        "corrupt_stored_value",
        mutate,
    )
}

fn assert_final_acceptance_action_corruption_with<F>(
    suffix: &str,
    logical_column: &str,
    corruption_category: &str,
    mutate: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&MethodHarness, &str) -> Result<(), Box<dyn Error>>,
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, &format!("bad_action_{suffix}"))?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        &format!("bad_action_{suffix}"),
        true,
    )?;
    let (_, judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        &format!("bad_action_{suffix}"),
    )?;
    mutate(&harness, &judgment_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: &format!("req_close_bad_action_{suffix}"),
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection_with_category(
        &response,
        "user_judgments",
        &judgment_id,
        logical_column,
        corruption_category,
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");
    Ok(())
}

fn record_cancellation_authority(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    accepted: bool,
) -> Result<(u64, String), Box<dyn Error>> {
    let request_id = format!("req_cancel_authority_{suffix}");
    let idempotency_key = format!("idem_cancel_authority_{suffix}");
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            &request_id,
            &idempotency_key,
            false,
            Some(expected_state_version),
            task_id,
            Some(change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let record_request_id = format!("req_cancel_authority_record_{suffix}");
    let record_idempotency_key = format!("idem_cancel_authority_record_{suffix}");
    let mut request = record_judgment_request(
        &record_request_id,
        &record_idempotency_key,
        Some(expected_state_version + 1),
        task_id,
        &judgment_id,
        JudgmentKind::Cancellation,
        answer_payload(JudgmentKind::Cancellation),
    );
    if !accepted {
        request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
        request.answer.cancellation = Some(json_object(json!({
            "decision": "rejected",
            "reason": "The user chose not to cancel the Task."
        })))
        .into();
    }
    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, judgment_id))
}

fn record_scope_decision_authority(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    accepted: bool,
) -> Result<(u64, StateRecordRef, String), Box<dyn Error>> {
    let request_id = format!("req_scope_authority_{suffix}");
    let idempotency_key = format!("idem_scope_authority_{suffix}");
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            &request_id,
            &idempotency_key,
            false,
            Some(expected_state_version),
            task_id,
            Some(change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let decision_ref: StateRecordRef =
        serde_json::from_value(judgment.response_value["user_judgment_ref"].clone())?;
    let judgment_id = decision_ref.record_id.as_str().to_owned();
    let record_request_id = format!("req_scope_authority_record_{suffix}");
    let record_idempotency_key = format!("idem_scope_authority_record_{suffix}");
    let mut request = record_judgment_request(
        &record_request_id,
        &record_idempotency_key,
        Some(expected_state_version + 1),
        task_id,
        &judgment_id,
        JudgmentKind::ScopeDecision,
        scope_decision_payload(if accepted { "accepted" } else { "rejected" }),
    );
    if !accepted {
        request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    }
    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, decision_ref, judgment_id))
}

fn record_sensitive_approval(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<(u64, String), Box<dyn Error>> {
    let request_id = format!("req_sensitive_approval_{suffix}");
    let idempotency_key = format!("idem_sensitive_approval_{suffix}");
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            &request_id,
            &idempotency_key,
            false,
            Some(expected_state_version),
            task_id,
            Some(change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let record_request_id = format!("req_sensitive_approval_record_{suffix}");
    let record_idempotency_key = format!("idem_sensitive_approval_record_{suffix}");
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            &record_request_id,
            &record_idempotency_key,
            Some(expected_state_version + 1),
            task_id,
            &judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, judgment_id))
}

fn record_sensitive_approval_with_scope(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    scope: volicord_types::SensitiveActionScope,
    accepted: bool,
) -> Result<(u64, String), Box<dyn Error>> {
    let request_id = format!("req_sensitive_scope_{suffix}");
    let idempotency_key = format!("idem_sensitive_scope_{suffix}");
    let mut judgment_request = user_judgment_request(
        &request_id,
        &idempotency_key,
        false,
        Some(expected_state_version),
        task_id,
        Some(change_unit_id),
        JudgmentKind::SensitiveApproval,
    );
    judgment_request.sensitive_action_scope = Some(scope.clone()).into();
    let judgment = harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let record_request_id = format!("req_sensitive_scope_record_{suffix}");
    let record_idempotency_key = format!("idem_sensitive_scope_record_{suffix}");
    let mut record_request = record_judgment_request(
        &record_request_id,
        &record_idempotency_key,
        Some(expected_state_version + 1),
        task_id,
        &judgment_id,
        JudgmentKind::SensitiveApproval,
        sensitive_approval_payload(scope),
    );
    if !accepted {
        record_request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    }
    let response = harness
        .service
        .record_user_judgment(record_request, invocation(OperationCategory::UserOnly))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, judgment_id))
}

fn sensitive_approval_payload(
    scope: volicord_types::SensitiveActionScope,
) -> RecordUserJudgmentPayload {
    RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: Some(scope).into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    }
}

fn sensitive_scope(
    action_kind: &str,
    intended_paths: Vec<&str>,
    sensitive_categories: Vec<&str>,
) -> volicord_types::SensitiveActionScope {
    volicord_types::SensitiveActionScope {
        action_kind: action_kind.to_owned(),
        description: "Allow the named sensitive step only.".to_owned(),
        intended_paths: intended_paths.into_iter().map(str::to_owned).collect(),
        sensitive_categories: sensitive_categories
            .into_iter()
            .map(str::to_owned)
            .collect(),
        command_or_tool_summary: Some("Run a local diagnostic command.".to_owned()).into(),
        network_or_host_summary: Some("No remote host is authorized here.".to_owned()).into(),
        secret_or_credential_summary: None.into(),
        capability_claim: "This is not Write Check.".to_owned(),
        expires_at: None.into(),
    }
}

fn prepare_write_check(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<String, Box<dyn Error>> {
    let request_id = format!("req_prepare_{suffix}");
    let idempotency_key = format!("idem_prepare_{suffix}");
    let response = harness.service.prepare_write(
        prepare_write_request(
            &request_id,
            &idempotency_key,
            Some(expected_state_version),
            Some(task_id),
            Some(change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(response.response_value["decision"], "allowed");
    Ok(response.response_value["write_check_ref"]["record_id"]
        .as_str()
        .expect("write check ref should be present")
        .to_owned())
}

fn stage_artifact_for_record_run(
    harness: &MethodHarness,
    task_id: &str,
    suffix: &str,
    expected_state_version: u64,
) -> Result<StagedArtifactHandle, Box<dyn Error>> {
    let request_id = format!("req_stage_{suffix}");
    let idempotency_key = format!("idem_stage_{suffix}");
    let mut request = stage_artifact_request(
        &request_id,
        Some(&idempotency_key),
        false,
        Some(expected_state_version),
        task_id,
    );
    request.display_name = format!("{suffix}.json");
    request.content_type = "application/json".to_owned();
    request.safe_bytes_or_notice = format!("{{\"fixture\":\"{suffix}\"}}");
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;
    Ok(serde_json::from_value(
        response.response_value["staged_artifact_handle"].clone(),
    )?)
}

fn artifact_input_for_handle(
    artifact_input_id: &str,
    handle: StagedArtifactHandle,
    relation_hint: Option<&str>,
    claim: Option<&str>,
) -> ArtifactInput {
    ArtifactInput {
        artifact_input_id: volicord_types::ArtifactInputId::new(artifact_input_id),
        source_kind: ArtifactInputSourceKind::StagedArtifact,
        staged_artifact_handle: Some(handle.clone()).into(),
        existing_artifact_ref: None.into(),
        relation_hint: relation_hint.map(str::to_owned).into(),
        claim: claim.map(str::to_owned).into(),
        expected_sha256: Some(handle.sha256).into(),
        expected_size_bytes: Some(handle.size_bytes).into(),
        redaction_state: Some(handle.redaction_state).into(),
    }
}

fn supported_evidence_update(claim: &str) -> EvidenceCoverageItem {
    EvidenceCoverageItem {
        claim: claim.to_owned(),
        required_for_close: true,
        coverage_state: EvidenceCoverageState::Supported,
        provenance: Some(evidence_update_provenance(
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
        )),
        supporting_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn unsupported_evidence_update(claim: &str) -> EvidenceCoverageItem {
    EvidenceCoverageItem {
        claim: claim.to_owned(),
        required_for_close: true,
        coverage_state: EvidenceCoverageState::Unsupported,
        provenance: None,
        supporting_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn evidence_update_provenance(
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
) -> EvidenceUpdateProvenance {
    EvidenceUpdateProvenance {
        source_kind,
        assurance_level,
        observed_at: None.into(),
        tool_name: Some("fixture-evidence-check".to_owned()).into(),
        tool_invocation_id: None.into(),
        tool_metadata: JsonObject::new(),
        limitations: Vec::new(),
    }
}

fn supported_evidence_update_with_provenance(
    claim: &str,
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
) -> EvidenceCoverageItem {
    let mut update = supported_evidence_update(claim);
    update.provenance = Some(evidence_update_provenance(source_kind, assurance_level));
    update
}

fn close_assessment_with_risks(
    summary: &str,
    residual_risks: Vec<volicord_types::ResidualRiskInput>,
) -> volicord_types::CloseAssessmentInput {
    volicord_types::CloseAssessmentInput {
        result_summary: summary.to_owned(),
        result_refs: Vec::new(),
        residual_risks,
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    }
}

fn residual_risk_input(summary: &str) -> volicord_types::ResidualRiskInput {
    volicord_types::ResidualRiskInput {
        summary: summary.to_owned(),
        consequence: "The user must decide whether this remaining risk is acceptable.".to_owned(),
        acceptance_required: true,
        source_refs: Vec::new(),
    }
}

fn enable_record_run_capabilities(harness: &MethodHarness) -> Result<(), Box<dyn Error>> {
    let _ = harness;
    Ok(())
}

fn assert_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().any(|candidate| candidate == code),
        "expected close blocker code {code}, got {codes:?}"
    );
}

fn assert_close_blocker_category(response_value: &Value, code: &str, category: &str) {
    let blocker = close_blocker_by_code(response_value, code);
    assert_eq!(blocker["category"], category);
}

fn assert_pending_judgment_prompt_capture_guidance(response_value: &Value) {
    assert_close_blocker(response_value, "pending_user_judgment");
    let blocker = close_blocker_by_code(response_value, "pending_user_judgment");
    let guidance = blocker["next_actions"][0]["blocking_question"]
        .as_str()
        .expect("pending blocker should include answer-path guidance");
    assert!(guidance.contains("prompt-capture"), "{guidance}");
    assert!(guidance.contains("verification code"), "{guidance}");
    assert!(!guidance.contains("MCP elicitation"), "{guidance}");
}

fn close_blocker_by_code<'a>(response_value: &'a Value, code: &str) -> &'a Value {
    let blockers = response_value
        .get("blockers")
        .or_else(|| response_value.get("close_blockers"))
        .expect("blockers or close_blockers should be present")
        .as_array()
        .expect("blockers should be an array");
    blockers
        .iter()
        .find(|blocker| blocker["code"] == code)
        .unwrap_or_else(|| panic!("expected close blocker code {code}, got {blockers:?}"))
}

fn assert_close_blocker_resolution(
    response_value: &Value,
    code: &str,
    can_resolve_in_chat: bool,
    terminal_action_required: bool,
) {
    let blocker = close_blocker_by_code(response_value, code);
    assert_eq!(blocker["can_resolve_in_chat"], can_resolve_in_chat);
    assert_eq!(
        blocker["terminal_action_required"],
        terminal_action_required
    );
    assert!(
        !blocker["next_actions"]
            .as_array()
            .expect("guard blocker next_actions should be an array")
            .is_empty(),
        "guard blocker should include a next action: {blocker:?}"
    );
}

fn assert_no_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().all(|candidate| candidate != code),
        "did not expect close blocker code {code}, got {codes:?}"
    );
}

fn assert_field_absent(value: &Value, field: &str) {
    assert!(
        value.get(field).is_none(),
        "expected field {field} to be absent, got {value:?}"
    );
}

fn assert_no_close_next_actions(response_value: &Value) {
    let actions = response_value["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    assert!(
        actions.iter().all(|action| {
            action["owner_method"] != "volicord.close_task" && action["action_kind"] != "close_task"
        }),
        "close-only next actions should not be present when close is excluded: {actions:?}"
    );
}

fn close_blocker_codes(response_value: &Value) -> Vec<String> {
    response_value
        .get("blockers")
        .or_else(|| response_value.get("close_blockers"))
        .expect("blockers or close_blockers should be present")
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .filter_map(|blocker| blocker["code"].as_str().map(str::to_owned))
        .collect()
}

fn assert_prepare_reason(response_value: &Value, code: &str) {
    let reasons = response_value["write_decision_reasons"]
        .as_array()
        .expect("write_decision_reasons should be an array");
    assert!(
        reasons.iter().any(|reason| reason["code"] == code),
        "expected prepare_write reason code {code}, got {reasons:?}"
    );
}

fn assert_no_prepare_reason(response_value: &Value, code: &str) {
    let reasons = response_value["write_decision_reasons"]
        .as_array()
        .expect("write_decision_reasons should be an array");
    assert!(
        reasons.iter().all(|reason| reason["code"] != code),
        "did not expect prepare_write reason code {code}, got {reasons:?}"
    );
}

fn create_task_with_change_unit(
    harness: &MethodHarness,
    prefix: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let intake_request_id = format!("req_{prefix}_task");
    let intake_idempotency_key = format!("idem_{prefix}_task");
    let intake = harness.service.intake(
        intake_request(
            &intake_request_id,
            &intake_idempotency_key,
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();

    let scope_request_id = format!("req_{prefix}_scope");
    let scope_idempotency_key = format!("idem_{prefix}_scope");
    let scope = harness.service.update_scope(
        update_scope_request(
            &scope_request_id,
            &scope_idempotency_key,
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    Ok((task_id, change_unit_id))
}

fn create_task_with_effect_contract(
    harness: &MethodHarness,
    prefix: &str,
    contract: ChangeUnitEffectContract,
) -> Result<(String, String), Box<dyn Error>> {
    let intake_request_id = format!("req_{prefix}_task");
    let intake_idempotency_key = format!("idem_{prefix}_task");
    let intake = harness.service.intake(
        intake_request(
            &intake_request_id,
            &intake_idempotency_key,
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();

    let scope_request_id = format!("req_{prefix}_scope");
    let scope_idempotency_key = format!("idem_{prefix}_scope");
    let mut request = update_scope_request(
        &scope_request_id,
        &scope_idempotency_key,
        false,
        Some(1),
        &task_id,
        ChangeUnitOperation::CreateCurrent,
        "Initial current scope.",
    );
    request.change_unit.effect_contract = Some(contract);
    let scope = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    Ok((task_id, change_unit_id))
}

#[derive(Debug, PartialEq)]
struct TaskTerminalFields {
    lifecycle_phase: String,
    result: Option<String>,
    close_summary: Value,
    closed_at: Option<String>,
}

fn task_terminal_fields(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<TaskTerminalFields, Box<dyn Error>> {
    let conn = harness.conn()?;
    let (lifecycle_phase, result, close_summary_text, closed_at): (
        String,
        Option<String>,
        String,
        Option<String>,
    ) = conn.query_row(
        "SELECT lifecycle_phase, result, close_summary_json, closed_at
               FROM tasks
              WHERE project_id = ?1
                AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    Ok(TaskTerminalFields {
        lifecycle_phase,
        result,
        close_summary: serde_json::from_str(&close_summary_text)?,
        closed_at,
    })
}

fn insert_superseding_task(harness: &MethodHarness, task_id: &str) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_actor_source,
                mode,
                lifecycle_phase,
                result,
                title,
                summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                completion_policy_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'work',
                'ready',
                'none',
                'Superseding task',
                'Superseding task',
                '{\"goal_summary\":\"Superseding task\"}',
                '{}',
                '{}',
                '{\"close_reason\":\"none\"}',
                '{}',
                't0',
                't0'
            )",
        rusqlite::params![PROJECT_ID, task_id, AGENT_ACTOR_SOURCE],
    )?;
    Ok(())
}

fn active_task_id(harness: &MethodHarness) -> Result<Option<String>, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT active_task_id
               FROM project_state
              WHERE project_id = ?1",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?)
}

#[derive(Debug, PartialEq)]
struct StagedArtifactRow {
    created_by_actor_source: String,
    status: String,
    redaction_state: String,
    tmp_path: String,
    ttl_hours: f64,
}

#[derive(Debug, PartialEq)]
struct PersistentArtifactRow {
    body_path: Option<String>,
    content_type: Option<String>,
    sha256: Option<String>,
    size_bytes: Option<u64>,
    integrity_status: String,
    status: String,
}

fn enable_stage_artifact_capability(_harness: &MethodHarness) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn staged_artifact_row(
    harness: &MethodHarness,
    handle_id: &str,
) -> Result<StagedArtifactRow, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT
                created_by_actor_source,
                status,
                redaction_state,
                tmp_path,
                (julianday(expires_at) - julianday(created_at)) * 24.0
             FROM artifact_staging
             WHERE project_id = ?1
               AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id],
        |row| {
            Ok(StagedArtifactRow {
                created_by_actor_source: row.get(0)?,
                status: row.get(1)?,
                redaction_state: row.get(2)?,
                tmp_path: row.get(3)?,
                ttl_hours: row.get(4)?,
            })
        },
    )?)
}

fn persistent_artifact_row(
    harness: &MethodHarness,
    artifact_id: &str,
) -> Result<PersistentArtifactRow, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT
                body_path,
                content_type,
                sha256,
                size_bytes,
                integrity_status,
                status
             FROM artifacts
             WHERE project_id = ?1
               AND artifact_id = ?2",
        rusqlite::params![PROJECT_ID, artifact_id],
        |row| {
            let size_bytes = row.get::<_, Option<i64>>(3)?.map(|value| value as u64);
            Ok(PersistentArtifactRow {
                body_path: row.get(0)?,
                content_type: row.get(1)?,
                sha256: row.get(2)?,
                size_bytes,
                integrity_status: row.get(4)?,
                status: row.get(5)?,
            })
        },
    )?)
}

fn persistent_artifact_body_path(
    harness: &MethodHarness,
    artifact_id: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let conn = harness.conn()?;
    let body_path: String = conn.query_row(
        "SELECT body_path
             FROM artifacts
            WHERE project_id = ?1
              AND artifact_id = ?2",
        rusqlite::params![PROJECT_ID, artifact_id],
        |row| row.get(0),
    )?;
    Ok(harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join("artifacts")
        .join(body_path))
}

fn staged_artifact_body_path(
    harness: &MethodHarness,
    handle_id: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let row = staged_artifact_row(harness, handle_id)?;
    Ok(harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join(row.tmp_path))
}

fn user_judgment_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    change_unit_id: Option<&str>,
    judgment_kind: JudgmentKind,
) -> volicord_types::RequestUserJudgmentRequest {
    let options = if matches!(
        judgment_kind,
        JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision
    ) {
        vec![
            volicord_types::UserJudgmentOptionInput {
                option_id: volicord_types::UserJudgmentOptionId::new("accept"),
                label: "Accept".to_owned(),
                description: "Record the focused user-owned judgment.".to_owned(),
                consequence: "Only this judgment record is resolved.".to_owned(),
                is_default: true,
            },
            volicord_types::UserJudgmentOptionInput {
                option_id: volicord_types::UserJudgmentOptionId::new("decline"),
                label: "Decline".to_owned(),
                description: "Record that the focused judgment was not accepted.".to_owned(),
                consequence: "The Task remains unresolved for this question.".to_owned(),
                is_default: false,
            },
        ]
    } else {
        Vec::new()
    };

    volicord_types::RequestUserJudgmentRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: change_unit_id.map(ChangeUnitId::new).into(),
        judgment_kind,
        presentation: volicord_types::JudgmentPresentation::Short,
        question: "Choose the focused test judgment outcome.".to_owned(),
        options: Some(options).into(),
        context: UserJudgmentContext {
            summary: "A focused test judgment needs a user-owned answer.".to_owned(),
            related_refs: Vec::new(),
            artifact_refs: Vec::new(),
            visible_risks: Vec::new(),
            constraints: vec!["The answer covers only the requested judgment kind.".to_owned()],
        },
        affected_refs: vec![StateRecordRef {
            record_kind: StateRecordKind::Task,
            record_id: RecordId::new(task_id),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: Some(TaskId::new(task_id)).into(),
            state_version: expected_state_version.into(),
        }],
        sensitive_action_scope: sensitive_action_scope_for_kind(judgment_kind).into(),
        required_for: required_for_for_kind(judgment_kind),
        expires_at: None.into(),
    }
}

fn required_for_for_kind(judgment_kind: JudgmentKind) -> Vec<volicord_types::JudgmentRequiredFor> {
    match judgment_kind {
        JudgmentKind::ScopeDecision => vec![volicord_types::JudgmentRequiredFor::ScopeUpdate],
        JudgmentKind::SensitiveApproval => vec![
            volicord_types::JudgmentRequiredFor::PrepareWrite,
            volicord_types::JudgmentRequiredFor::CloseComplete,
        ],
        JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance => {
            vec![volicord_types::JudgmentRequiredFor::CloseComplete]
        }
        JudgmentKind::Cancellation => vec![volicord_types::JudgmentRequiredFor::CloseCancel],
        JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision => {
            vec![volicord_types::JudgmentRequiredFor::CloseComplete]
        }
    }
}

fn sensitive_action_scope_for_kind(
    judgment_kind: JudgmentKind,
) -> Option<volicord_types::SensitiveActionScope> {
    match judgment_kind {
        JudgmentKind::SensitiveApproval => Some(volicord_types::SensitiveActionScope {
            action_kind: "local_sensitive_step".to_owned(),
            description: "Allow the named sensitive step only.".to_owned(),
            intended_paths: vec!["src/export.rs".to_owned()],
            sensitive_categories: vec!["network".to_owned()],
            command_or_tool_summary: Some("Run a local diagnostic command.".to_owned()).into(),
            network_or_host_summary: Some("No remote host is authorized here.".to_owned()).into(),
            secret_or_credential_summary: None.into(),
            capability_claim: "This is not Write Check.".to_owned(),
            expires_at: None.into(),
        }),
        _ => None,
    }
}

fn record_judgment_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: &str,
    user_judgment_id: &str,
    judgment_kind: JudgmentKind,
    answer: RecordUserJudgmentPayload,
) -> RecordUserJudgmentRequest {
    let request_envelope = envelope(
        request_id,
        Some(idempotency_key),
        false,
        expected_state_version,
        Some(task_id),
    );
    RecordUserJudgmentRequest {
        envelope: request_envelope,
        user_judgment_id: volicord_types::UserJudgmentId::new(user_judgment_id),
        judgment_kind,
        selected_option_id: volicord_types::UserJudgmentOptionId::new("accept"),
        answer,
        rationale: default_judgment_rationale(),
        note: Some("Recorded by the focused judgment test.".to_owned()).into(),
        accepted_risks: Vec::new(),
    }
}

fn residual_risk_acceptance_payload(risk_ids: &[String]) -> RecordUserJudgmentPayload {
    let mut payload = RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    };
    payload.residual_risk_acceptance = Some(json_object(json!({ "risk_ids": risk_ids }))).into();
    payload
}

fn cancellation_payload_with_decision(decision: &str) -> RecordUserJudgmentPayload {
    let mut payload = RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    };
    payload.cancellation = Some(json_object(json!({
        "decision": decision,
        "reason": "The user selected this cancellation outcome."
    })))
    .into();
    payload
}

fn scope_decision_payload(decision: &str) -> RecordUserJudgmentPayload {
    let mut payload = RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    };
    payload.scope_decision = Some(json_object(json!({
        "requested_scope_summary": "Expanded scope that must not apply silently.",
        "decision": decision
    })))
    .into();
    payload
}

fn rejected_final_acceptance_payload() -> RecordUserJudgmentPayload {
    let mut payload = answer_payload(JudgmentKind::FinalAcceptance);
    payload.final_acceptance = Some(json_object(json!({
        "judgment": {
            "decision": "rejected",
            "basis": "The visible close basis is not accepted."
        }
    })))
    .into();
    payload
}

fn default_judgment_rationale() -> JudgmentRationale {
    JudgmentRationale {
        summary: "The user selected the focused judgment option.".to_owned(),
        selected_reason: Some("The selected option matches the visible prompt.".to_owned()).into(),
        considered_alternatives: vec!["Use another listed option.".to_owned()],
        rejected_alternatives: Vec::new(),
        assumptions: vec!["The pending judgment basis is current.".to_owned()],
        tradeoffs: vec![
            "The rationale preserves intent without changing the selected option.".to_owned(),
        ],
        uncertainties: Vec::new(),
        review_triggers: vec!["Review if the judgment basis changes.".to_owned()],
        related_refs: Vec::new(),
        artifact_refs: Vec::new(),
    }
}

fn default_judgment_rationale_json() -> String {
    serde_json::to_string(&default_judgment_rationale())
        .expect("default judgment rationale should serialize")
}

fn answer_payload(judgment_kind: JudgmentKind) -> RecordUserJudgmentPayload {
    let mut payload = RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    };
    match judgment_kind {
        JudgmentKind::ProductDecision => {
            payload.product_decision = Some(json_object(json!({
                "judgment": {
                    "decision": "accepted",
                    "rationale": "The product direction is accepted for this focused test."
                }
            })))
            .into();
        }
        JudgmentKind::TechnicalDecision => {
            payload.technical_decision = Some(json_object(json!({
                "judgment": {
                    "decision": "accepted",
                    "rationale": "The technical direction is accepted for this focused test."
                }
            })))
            .into();
        }
        JudgmentKind::ScopeDecision => {
            payload.scope_decision = Some(json_object(json!({
                "requested_scope_summary": "Expanded scope that must not apply silently.",
                "decision": "accepted"
            })))
            .into();
        }
        JudgmentKind::SensitiveApproval => {
            payload.sensitive_action_scope = sensitive_action_scope_for_kind(judgment_kind).into();
        }
        JudgmentKind::FinalAcceptance => {
            payload.final_acceptance = Some(json_object(json!({
                "judgment": {
                    "decision": "accepted",
                    "basis": "The visible close basis is acceptable."
                }
            })))
            .into();
        }
        JudgmentKind::ResidualRiskAcceptance => {
            payload.residual_risk_acceptance = Some(json_object(json!({
                "risk_id": "risk_visible_001",
                "decision": "accepted"
            })))
            .into();
        }
        JudgmentKind::Cancellation => {
            payload.cancellation = Some(json_object(json!({
                "decision": "cancel",
                "reason": "The user chose to stop the Task."
            })))
            .into();
        }
    }
    payload
}

fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => panic!("test helper expected a JSON object"),
    }
}

fn insert_active_write_check(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
) -> Result<(), Box<dyn Error>> {
    insert_active_write_check_with_timestamps(
        harness,
        task_id,
        change_unit_id,
        "wa_replace",
        2,
        "2026-06-18T00:00:00.000Z",
        "2026-06-18T00:15:00.000Z",
    )
}

fn insert_active_write_check_with_timestamps(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    write_check_id: &str,
    basis_state_version: u64,
    created_at: &str,
    expires_at: &str,
) -> Result<(), Box<dyn Error>> {
    insert_active_write_check_with_scope(
        harness,
        WriteCheckScopeFixture {
            task_id,
            change_unit_id,
            write_check_id,
            basis_state_version,
            created_at,
            expires_at,
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            sensitive_categories: &[],
        },
    )
}

struct WriteCheckScopeFixture<'a> {
    task_id: &'a str,
    change_unit_id: &'a str,
    write_check_id: &'a str,
    basis_state_version: u64,
    created_at: &'a str,
    expires_at: &'a str,
    intended_operation: &'a str,
    intended_paths: &'a [&'a str],
    sensitive_categories: &'a [&'a str],
}

fn insert_active_write_check_with_scope(
    harness: &MethodHarness,
    input: WriteCheckScopeFixture<'_>,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let attempt_scope_json = json!({
        "task_id": input.task_id,
        "change_unit_id": input.change_unit_id,
        "intended_operation": input.intended_operation,
        "intended_paths": input.intended_paths,
        "product_file_write_intended": true,
        "sensitive_categories": input.sensitive_categories,
        "baseline_ref": "baseline_test"
    })
    .to_string();
    conn.execute(
        "INSERT INTO write_checks (
                project_id,
                write_check_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                attempt_scope_json,
                created_by_actor_source,
                expires_at,
                created_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                'active',
                ?6,
                ?7,
                ?8,
                ?9
            )",
        rusqlite::params![
            PROJECT_ID,
            input.write_check_id,
            input.task_id,
            input.change_unit_id,
            i64::try_from(input.basis_state_version)?,
            attempt_scope_json,
            AGENT_ACTOR_SOURCE,
            input.expires_at,
            input.created_at
        ],
    )?;
    Ok(())
}

fn mutate_write_check_scope_json(
    harness: &MethodHarness,
    write_check_id: &str,
    mutate: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT attempt_scope_json
           FROM write_checks
          WHERE project_id = ?1
            AND write_check_id = ?2",
        rusqlite::params![PROJECT_ID, write_check_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    mutate(&mut value);
    conn.execute(
        "UPDATE write_checks
            SET attempt_scope_json = ?3
          WHERE project_id = ?1
            AND write_check_id = ?2",
        rusqlite::params![PROJECT_ID, write_check_id, value.to_string()],
    )?;
    Ok(())
}

struct SensitiveProductWriteBasisFixture<'a> {
    task_id: &'a str,
    change_unit_id: &'a str,
    expected_state_version: u64,
    suffix: &'a str,
    write_check_id: &'a str,
    intended_operation: &'a str,
    intended_paths: &'a [&'a str],
    observed_categories: &'a [&'a str],
    assessment_categories: &'a [&'a str],
}

fn record_sensitive_product_write_close_basis(
    harness: &MethodHarness,
    input: SensitiveProductWriteBasisFixture<'_>,
) -> Result<PipelineResponse, Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    insert_active_write_check_with_scope(
        harness,
        WriteCheckScopeFixture {
            task_id: input.task_id,
            change_unit_id: input.change_unit_id,
            write_check_id: input.write_check_id,
            basis_state_version: input.expected_state_version,
            created_at: "2999-01-01T00:00:00.000Z",
            expires_at: "2999-01-01T00:15:00.000Z",
            intended_operation: input.intended_operation,
            intended_paths: input.intended_paths,
            sensitive_categories: input.observed_categories,
        },
    )?;
    let mut request = product_write_record_run_request(
        &format!("req_sensitive_run_{}", input.suffix),
        &format!("idem_sensitive_run_{}", input.suffix),
        input.expected_state_version,
        input.task_id,
        input.change_unit_id,
        input.write_check_id,
        &format!("run_sensitive_{}", input.suffix),
    );
    request.observed_changes.changed_paths = input
        .intended_paths
        .iter()
        .map(|path| path.to_string())
        .collect();
    request.observed_changes.sensitive_categories = input
        .observed_categories
        .iter()
        .map(|category| category.to_string())
        .collect();
    request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Sensitive product write is ready for close.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: input
            .assessment_categories
            .iter()
            .map(|category| category.to_string())
            .collect(),
        recovery_constraints: Vec::new(),
    })
    .into();
    Ok(harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?)
}

fn write_check_count(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM write_checks
              WHERE project_id = ?1",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count)?)
}

fn write_decision_event_count(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM task_events
              WHERE project_id = ?1
                AND event_kind = 'write_decision_recorded'",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count)?)
}

fn latest_task_event(harness: &MethodHarness) -> Result<(String, Value, u64), Box<dyn Error>> {
    let conn = harness.conn()?;
    let (event_kind, event_payload_text, state_version): (String, String, i64) = conn.query_row(
        "SELECT event_kind, event_payload_json, state_version
                   FROM task_events
                  WHERE project_id = ?1
                  ORDER BY event_seq DESC
                  LIMIT 1",
        rusqlite::params![PROJECT_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    Ok((
        event_kind,
        serde_json::from_str(&event_payload_text)?,
        u64::try_from(state_version)?,
    ))
}

fn assert_latest_prepare_write_event(
    harness: &MethodHarness,
    response_value: &Value,
    expected_decision: &str,
    expected_reason_code: &str,
) -> Result<Value, Box<dyn Error>> {
    let (event_kind, payload, event_state_version) = latest_task_event(harness)?;
    assert_eq!(event_kind, "write_decision_recorded");
    assert_eq!(event_state_version, response_value["base"]["state_version"]);
    assert_eq!(payload["decision"], expected_decision);
    assert!(payload["write_check_id"].is_null());
    assert!(payload.get("reason_codes").is_none());
    assert!(payload.get("intended_paths").is_none());
    assert!(payload.get("intended_operation").is_none());
    assert!(payload.get("sensitive_categories").is_none());
    assert!(payload.get("baseline_ref").is_none());
    assert_eq!(
        payload["write_decision_reasons"],
        response_value["write_decision_reasons"]
    );
    assert_prepare_reason(&payload, expected_reason_code);
    Ok(payload)
}

fn write_check_basis(harness: &MethodHarness, write_check_id: &str) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let basis: i64 = conn.query_row(
        "SELECT basis_state_version
               FROM write_checks
              WHERE project_id = ?1
                AND write_check_id = ?2",
        rusqlite::params![PROJECT_ID, write_check_id],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(basis)?)
}

fn write_check_timestamps(
    harness: &MethodHarness,
    write_check_id: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT created_at, expires_at
               FROM write_checks
              WHERE project_id = ?1
                AND write_check_id = ?2",
        rusqlite::params![PROJECT_ID, write_check_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

fn user_judgment_status(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?)
}

fn user_judgment_basis_status(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT basis_status
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?)
}

fn user_judgment_resolution_outcome(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT resolution_outcome
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?)
}

fn user_judgment_resolution_machine_action(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT resolution_machine_action
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?)
}

fn user_judgment_actor_provenance(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<UserJudgmentActorProvenance, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level
           FROM user_judgments
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| {
            Ok(UserJudgmentActorProvenance {
                resolved_by_actor_source: row.get(0)?,
                resolved_verification_basis: row.get(1)?,
                resolved_assurance_level: row.get(2)?,
            })
        },
    )?)
}

fn clear_user_judgment_actor_provenance(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_judgments
            SET resolved_by_actor_source = NULL,
                resolved_verification_basis = NULL,
                resolved_assurance_level = NULL
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn resolution_json(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT resolution_json
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&text)?)
}

fn resolution_rationale_json(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT resolution_rationale_json
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&text)?)
}

fn current_change_unit_id(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT current_change_unit_id
               FROM tasks
              WHERE project_id = ?1
                AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn task_revision(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<TaskRevisionRecord, Box<dyn Error>> {
    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    store
        .task_revision_record(&TaskId::new(task_id))?
        .ok_or_else(|| format!("missing task revision for {task_id}").into())
}

fn run_id_from_record_run(response_value: &Value) -> String {
    response_value["run_summary"]["run_ref"]["record_id"]
        .as_str()
        .expect("run_ref.record_id should be present")
        .to_owned()
}

fn latest_run_id(harness: &MethodHarness, task_id: &str) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT run_id
               FROM runs
              WHERE project_id = ?1
                AND task_id = ?2
              ORDER BY rowid DESC
              LIMIT 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn run_scope_revision(harness: &MethodHarness, run_id: &str) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let scope_revision: i64 = conn.query_row(
        "SELECT scope_revision
               FROM runs
              WHERE project_id = ?1
                AND run_id = ?2",
        rusqlite::params![PROJECT_ID, run_id],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(scope_revision)?)
}

fn set_run_observed_baseline(
    harness: &MethodHarness,
    run_id: &str,
    baseline_ref: &str,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE runs
            SET observed_changes_json = ?3
          WHERE project_id = ?1
            AND run_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            run_id,
            json!({
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": baseline_ref
            })
            .to_string()
        ],
    )?;
    Ok(())
}

fn current_change_unit_scope(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT scope_summary_json
               FROM change_units
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    let value: Value = serde_json::from_str(&text)?;
    Ok(value["scope_summary"]
        .as_str()
        .expect("scope_summary should be a string")
        .to_owned())
}

fn set_task_owner_json(
    harness: &MethodHarness,
    task_id: &str,
    logical_column: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "shaping_summary_json" => {
            "UPDATE tasks
                SET shaping_summary_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "autonomy_boundary_json" => {
            "UPDATE tasks
                SET autonomy_boundary_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "close_basis_json" => {
            "UPDATE tasks
                SET close_basis_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "close_summary_json" => {
            "UPDATE tasks
                SET close_summary_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "completion_policy_json" => {
            "UPDATE tasks
                SET completion_policy_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        _ => panic!("unsupported task owner JSON column {logical_column}"),
    };
    harness
        .conn()?
        .execute(sql, rusqlite::params![PROJECT_ID, task_id, value])?;
    Ok(())
}

fn set_change_unit_owner_json(
    harness: &MethodHarness,
    change_unit_id: &str,
    logical_column: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "scope_summary_json" => {
            "UPDATE change_units
                SET scope_summary_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "bounded_paths_json" => {
            "UPDATE change_units
                SET bounded_paths_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "write_basis_json" => {
            "UPDATE change_units
                SET write_basis_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "lifecycle_json" => {
            "UPDATE change_units
                SET lifecycle_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        _ => panic!("unsupported change-unit owner JSON column {logical_column}"),
    };
    harness
        .conn()?
        .execute(sql, rusqlite::params![PROJECT_ID, change_unit_id, value])?;
    Ok(())
}

fn set_user_judgment_resolution_json(
    harness: &MethodHarness,
    judgment_id: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let (machine_action, resolution_outcome) = match value {
        Some(text) => match serde_json::from_str::<Value>(text) {
            Ok(value) => (
                value
                    .get("machine_action")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                value
                    .get("resolution_outcome")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
            Err(_) => (Some("accept".to_owned()), Some("accepted".to_owned())),
        },
        None => (None, None),
    };
    let conn = harness.conn()?;
    let rationale = value.map(|_| default_judgment_rationale_json());
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_judgments
            SET status = 'resolved',
                resolution_json = ?3,
                resolution_rationale_json = ?4,
                resolution_machine_action = ?5,
                resolution_outcome = ?6,
                resolved_at = 't1'
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            judgment_id,
            value,
            rationale,
            machine_action,
            resolution_outcome
        ],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn set_user_judgment_resolution_machine_action(
    harness: &MethodHarness,
    judgment_id: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_judgments
            SET resolution_machine_action = ?3
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id, value],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn set_user_judgment_resolution_machine_action_raw(
    harness: &MethodHarness,
    judgment_id: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_judgments
            SET resolution_machine_action = ?3
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id, value],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn set_user_judgment_resolution_json_value(
    harness: &MethodHarness,
    judgment_id: &str,
    value: &Value,
) -> Result<(), Box<dyn Error>> {
    let text = serde_json::to_string(value)?;
    set_user_judgment_resolution_json(harness, judgment_id, Some(&text))
}

fn set_user_judgment_resolution_json_only_value(
    harness: &MethodHarness,
    judgment_id: &str,
    value: &Value,
) -> Result<(), Box<dyn Error>> {
    let text = serde_json::to_string(value)?;
    harness.conn()?.execute(
        "UPDATE user_judgments
            SET resolution_json = ?3
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id, text],
    )?;
    Ok(())
}

fn set_user_judgment_resolution_actor(
    harness: &MethodHarness,
    judgment_id: &str,
    actor_kind: &str,
) -> Result<(), Box<dyn Error>> {
    let mut resolution = resolution_json(harness, judgment_id)?;
    resolution["resolved_by_actor_source"] = json!(actor_kind);
    harness.conn()?.execute(
        "UPDATE user_judgments
            SET resolution_json = ?3,
                resolved_by_actor_source = ?4
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id, resolution.to_string(), actor_kind],
    )?;
    Ok(())
}

fn set_user_judgment_resolved_by_actor_source(
    harness: &MethodHarness,
    judgment_id: &str,
    role: &str,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE user_judgments
            SET resolved_by_actor_source = ?3
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id, role],
    )?;
    Ok(())
}

fn set_user_judgment_required_for(
    harness: &MethodHarness,
    judgment_id: &str,
    required_for: &[volicord_types::JudgmentRequiredFor],
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT request_json
           FROM user_judgments
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    value["required_for"] = serde_json::to_value(required_for)?;
    set_user_judgment_owner_json(
        harness,
        judgment_id,
        "request_json",
        Some(&value.to_string()),
    )
}

fn set_user_judgment_affected_refs(
    harness: &MethodHarness,
    judgment_id: &str,
    affected_refs: &[StateRecordRef],
) -> Result<(), Box<dyn Error>> {
    let value = serde_json::to_string(affected_refs)?;
    set_user_judgment_owner_json(harness, judgment_id, "affected_refs_json", Some(&value))
}

fn set_user_judgment_expires_at(
    harness: &MethodHarness,
    judgment_id: &str,
    expires_at: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT request_json
           FROM user_judgments
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    value["expires_at"] = json!(expires_at);
    set_user_judgment_owner_json(
        harness,
        judgment_id,
        "request_json",
        Some(&value.to_string()),
    )
}

fn set_user_judgment_owner_json(
    harness: &MethodHarness,
    judgment_id: &str,
    logical_column: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "request_json" => {
            "UPDATE user_judgments
                SET request_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2"
        }
        "basis_json" => {
            "UPDATE user_judgments
                SET basis_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2"
        }
        "options_json" => {
            "UPDATE user_judgments
                SET options_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2"
        }
        "resolution_json" => {
            "UPDATE user_judgments
                SET resolution_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2"
        }
        "artifact_refs_json" => {
            "UPDATE user_judgments
                SET artifact_refs_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2"
        }
        "affected_refs_json" => {
            "UPDATE user_judgments
                SET affected_refs_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2"
        }
        _ => panic!("unsupported user-judgment owner JSON column {logical_column}"),
    };
    harness
        .conn()?
        .execute(sql, rusqlite::params![PROJECT_ID, judgment_id, value])?;
    Ok(())
}

fn mutate_user_judgment_basis_json(
    harness: &MethodHarness,
    judgment_id: &str,
    mutate: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT basis_json
           FROM user_judgments
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    mutate(&mut value);
    set_user_judgment_owner_json(harness, judgment_id, "basis_json", Some(&value.to_string()))
}

fn set_artifact_owner_json(
    harness: &MethodHarness,
    artifact_id: &str,
    logical_column: &str,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "producer_json" => {
            "UPDATE artifacts
                SET producer_json = ?3
              WHERE project_id = ?1
                AND artifact_id = ?2"
        }
        "metadata_json" => {
            "UPDATE artifacts
                SET metadata_json = ?3
              WHERE project_id = ?1
                AND artifact_id = ?2"
        }
        _ => panic!("unsupported artifact owner JSON column {logical_column}"),
    };
    harness
        .conn()?
        .execute(sql, rusqlite::params![PROJECT_ID, artifact_id, value])?;
    Ok(())
}

fn set_artifact_integrity(
    harness: &MethodHarness,
    artifact_id: &str,
    integrity_status: &str,
    content_type: Option<&str>,
    sha256: Option<&str>,
    size_bytes: Option<u64>,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "UPDATE artifacts
            SET integrity_status = ?3,
                content_type = ?4,
                sha256 = ?5,
                size_bytes = ?6
          WHERE project_id = ?1
            AND artifact_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            artifact_id,
            integrity_status,
            content_type,
            sha256,
            size_bytes.map(|value| value as i64)
        ],
    )?;
    Ok(())
}

fn clear_artifact_source_staging_handle(
    harness: &MethodHarness,
    artifact_id: &str,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE artifacts
            SET source_staging_handle_id = NULL
          WHERE project_id = ?1
            AND artifact_id = ?2",
        rusqlite::params![PROJECT_ID, artifact_id],
    )?;
    Ok(())
}

fn set_artifact_staging_artifact_json(
    harness: &MethodHarness,
    handle_id: &str,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE artifact_staging
            SET artifact_json = ?3
          WHERE project_id = ?1
            AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id, value],
    )?;
    Ok(())
}

fn set_artifact_staging_tmp_path(
    harness: &MethodHarness,
    handle_id: &str,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE artifact_staging
            SET tmp_path = ?3
          WHERE project_id = ?1
            AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id, value],
    )?;
    Ok(())
}

fn latest_evidence_summary_id(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT evidence_summary_id
               FROM evidence_summaries
              WHERE project_id = ?1
                AND task_id = ?2
              ORDER BY updated_at DESC, evidence_summary_id DESC
              LIMIT 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn set_evidence_summary_owner_json(
    harness: &MethodHarness,
    evidence_summary_id: &str,
    logical_column: &str,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "coverage_json" => {
            "UPDATE evidence_summaries
                SET coverage_json = ?3
              WHERE project_id = ?1
                AND evidence_summary_id = ?2"
        }
        "supporting_refs_json" => {
            "UPDATE evidence_summaries
                SET supporting_refs_json = ?3
              WHERE project_id = ?1
                AND evidence_summary_id = ?2"
        }
        "gap_refs_json" => {
            "UPDATE evidence_summaries
                SET gap_refs_json = ?3
              WHERE project_id = ?1
                AND evidence_summary_id = ?2"
        }
        "metadata_json" => {
            "UPDATE evidence_summaries
                SET metadata_json = ?3
              WHERE project_id = ?1
                AND evidence_summary_id = ?2"
        }
        _ => panic!("unsupported evidence summary owner JSON column {logical_column}"),
    };
    harness.conn()?.execute(
        sql,
        rusqlite::params![PROJECT_ID, evidence_summary_id, value],
    )?;
    Ok(())
}

fn promote_artifact_for_record_run(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<(u64, ArtifactRef), Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    let handle = stage_artifact_for_record_run(harness, task_id, suffix, expected_state_version)?;
    let mut request = record_run_request(
        &format!("req_promote_artifact_{suffix}"),
        &format!("idem_promote_artifact_{suffix}"),
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        &format!("artifact_input_{suffix}"),
        handle,
        Some("test_artifact"),
        Some("Artifact registered for corruption coverage."),
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let artifact_ref: ArtifactRef =
        serde_json::from_value(response.response_value["registered_artifacts"][0].clone())?;
    Ok((state_version, artifact_ref))
}

fn existing_artifact_input(artifact_input_id: &str, artifact_ref: ArtifactRef) -> ArtifactInput {
    ArtifactInput {
        artifact_input_id: volicord_types::ArtifactInputId::new(artifact_input_id),
        source_kind: ArtifactInputSourceKind::ExistingArtifact,
        staged_artifact_handle: None.into(),
        existing_artifact_ref: Some(artifact_ref.clone()).into(),
        relation_hint: Some("reuse_existing_artifact".to_owned()).into(),
        claim: Some("Reused artifact for corruption coverage.".to_owned()).into(),
        expected_sha256: artifact_ref.sha256.as_ref().cloned().into(),
        expected_size_bytes: artifact_ref.size_bytes.as_ref().copied().into(),
        redaction_state: Some(artifact_ref.redaction_state).into(),
    }
}

struct ArtifactAuthorityFixture {
    task_id: String,
    artifact_ref: ArtifactRef,
    body_path: PathBuf,
}

impl ArtifactAuthorityFixture {
    fn artifact_id(&self) -> &str {
        self.artifact_ref.artifact_id.as_str()
    }
}

fn current_artifact_evidence_and_close_fixture(
    harness: &MethodHarness,
    suffix: &str,
) -> Result<ArtifactAuthorityFixture, Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(harness, suffix)?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(harness, &task_id, &change_unit_id, 2, suffix)?;
    let mut request = record_run_request(
        &format!("req_artifact_authority_{suffix}"),
        &format!("idem_artifact_authority_{suffix}"),
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        &format!("artifact_input_authority_{suffix}"),
        artifact_ref.clone(),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Reused artifact for corruption coverage.",
    )];
    let mut close_assessment =
        close_assessment_with_risks("Reused artifact for corruption coverage.", Vec::new());
    close_assessment.result_refs = vec![state_ref(
        StateRecordKind::Artifact,
        artifact_ref.artifact_id.as_str(),
        &ProjectId::new(PROJECT_ID),
        Some(&TaskId::new(&task_id)),
        Some(state_version),
    )];
    request.close_assessment = Some(close_assessment).into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let body_path = persistent_artifact_body_path(harness, artifact_ref.artifact_id.as_str())?;
    Ok(ArtifactAuthorityFixture {
        task_id,
        artifact_ref,
        body_path,
    })
}

fn status_with_evidence_and_close(
    harness: &MethodHarness,
    task_id: &str,
) -> CoreResult<PipelineResponse> {
    harness.service.status(
        StatusRequest {
            envelope: envelope(
                &format!("req_status_artifact_authority_{task_id}"),
                None,
                false,
                None,
                Some(task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_check: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )
}

fn close_check(harness: &MethodHarness, task_id: &str) -> CoreResult<PipelineResponse> {
    harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: &format!("req_close_check_artifact_authority_{task_id}"),
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )
}

fn status_evidence_artifact_ref(response_value: &Value) -> &Value {
    &response_value["evidence_summary"]["coverage_items"][0]["supporting_artifact_refs"][0]
}

fn active_current_change_units(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<i64, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT COUNT(*)
               FROM change_units
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn write_check_status(
    harness: &MethodHarness,
    write_check_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM write_checks
              WHERE project_id = ?1
                AND write_check_id = ?2",
        rusqlite::params![PROJECT_ID, write_check_id],
        |row| row.get(0),
    )?)
}

fn artifact_staging_status(
    harness: &MethodHarness,
    handle_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM artifact_staging
              WHERE project_id = ?1
                AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id],
        |row| row.get(0),
    )?)
}

fn expire_staged_artifact(harness: &MethodHarness, handle_id: &str) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "UPDATE artifact_staging
                SET expires_at = '2000-01-01T00:00:00.000Z'
              WHERE project_id = ?1
                AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id],
    )?;
    Ok(())
}

fn set_staged_artifact_expires_at(
    harness: &MethodHarness,
    handle_id: &str,
    expires_at: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "UPDATE artifact_staging
                SET expires_at = ?3
              WHERE project_id = ?1
                AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id, expires_at],
    )?;
    Ok(())
}

fn artifact_owner_link_exists(
    harness: &MethodHarness,
    artifact_id: &str,
    owner_record_kind: &str,
) -> Result<bool, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM artifact_links
              WHERE project_id = ?1
                AND artifact_id = ?2
                AND owner_record_kind = ?3",
        rusqlite::params![PROJECT_ID, artifact_id, owner_record_kind],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
