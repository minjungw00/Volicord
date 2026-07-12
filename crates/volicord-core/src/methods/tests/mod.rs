use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
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
        guard_health_record, insert_agent_session, insert_expected_write, insert_guard_event,
        insert_unrecorded_change, list_unresolved_unrecorded_changes, observe_guard_installation,
        unrecorded_change, upsert_guard_installation, AgentSessionInsert, ExpectedWriteInsert,
        GuardEventInsert, GuardInstallationObservation, GuardInstallationUpsert,
        UnrecordedChangeInsert, UnrecordedChangeRecord,
    },
    local_consent::{create_local_web_consent_token, LocalWebConsentTokenCreate},
    session_watch::{
        create_watch_baseline, snapshot_product_repository, SessionWatchStatus,
        WatchBaselineCreate, WatchSnapshotOptions,
    },
    sqlite::open_project_state_database,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::CloseMutationIntent;
use volicord_types::{
    prefixed_durable_id, ActorSource, ChangeUnitEffectContract, ChangeUnitEffectKind,
    ChangeUnitUpdate, DurableIdError, DurableIdGenerator, DurableIdKind, EvidenceAssuranceLevel,
    EvidenceSourceKind, EvidenceUpdateProvenance, IdempotencyKey, InitialScope, OperationCategory,
    RequestId, ScopeUpdate, SequenceDurableIdGenerator, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON,
    VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
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

type LocalWebTokenStatus = (String, Option<String>, Option<String>);

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
        produced_at_state_version: state_version.into(),
    }
}

mod close_task;
mod intake;
mod preflight;
mod prepare_write;
mod reconcile_changes;
mod record_run;
mod replay;
mod stage_artifact;
mod status;
mod update_scope;
mod user_judgment;

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

fn local_web_invocation(
    actor_source: ActorSource,
    operation_category: OperationCategory,
) -> InvocationContext {
    InvocationContext::new(
        ProjectId::new(PROJECT_ID),
        actor_source,
        operation_category,
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
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
    _task_id: &str,
    session_id: &str,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    let health = guard_health_record(&harness.runtime_home_path, PROJECT_ID, CONNECTION_ID)?;
    let guard_installation_id = health
        .guard_installation
        .as_ref()
        .map(|installation| installation.guard_installation_id.clone());
    let guard_mode = health
        .guard_installation
        .as_ref()
        .map(|installation| installation.guard_mode.clone())
        .unwrap_or_else(|| "detective".to_owned());
    insert_agent_session(
        &harness.runtime_home_path,
        PROJECT_ID,
        AgentSessionInsert {
            session_id: session_id.to_owned(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: guard_installation_id.clone(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode,
            started_at: "2026-06-30T00:03:00Z".to_owned(),
            metadata_json: serde_json::to_string(&json!({
                "source": "test_fixture",
                "session_watch_initialized": true
            }))?,
        },
    )?;
    let repo_root = product_repo_root(harness)?;
    let snapshot = snapshot_product_repository(
        &harness.runtime_home_path,
        &repo_root,
        WatchSnapshotOptions::default(),
    )?;
    create_watch_baseline(
        &harness.runtime_home_path,
        PROJECT_ID,
        WatchBaselineCreate {
            watch_baseline_id: format!("watch_base_{suffix}"),
            session_id: session_id.to_owned(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id,
            status: SessionWatchStatus::Active,
            snapshot,
            created_at: "2026-06-30T00:03:00Z".to_owned(),
            metadata_json: serde_json::to_string(&json!({
                "source": "volicord_session_watch",
                "status_detail": "active",
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true,
                "coverage_start_at": "2026-06-30T00:03:00Z",
                "coverage_basis": SessionWatchCoverageBasis::MethodBoundary.as_str(),
                "partial_coverage_warning": "Session-watch coverage starts at a method boundary; Product Repository changes before that boundary are outside watcher coverage."
            }))?,
        },
    )?;
    Ok(())
}

fn initialize_full_watch_baseline(
    harness: &MethodHarness,
    session_id: &str,
    guard_installation_id: &str,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    insert_agent_session(
        &harness.runtime_home_path,
        PROJECT_ID,
        AgentSessionInsert {
            session_id: session_id.to_owned(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: Some(guard_installation_id.to_owned()),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            started_at: "2026-06-30T00:03:00Z".to_owned(),
            metadata_json: serde_json::to_string(&json!({
                "source": "test_fixture",
                "session_watch_initialized": true
            }))?,
        },
    )?;
    let repo_root = product_repo_root(harness)?;
    let snapshot = snapshot_product_repository(
        &harness.runtime_home_path,
        &repo_root,
        WatchSnapshotOptions::default(),
    )?;
    create_watch_baseline(
        &harness.runtime_home_path,
        PROJECT_ID,
        WatchBaselineCreate {
            watch_baseline_id: format!("watch_base_full_{suffix}"),
            session_id: session_id.to_owned(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: Some(guard_installation_id.to_owned()),
            status: SessionWatchStatus::Active,
            snapshot,
            created_at: "2026-06-30T00:03:00Z".to_owned(),
            metadata_json: serde_json::to_string(&json!({
                "source": "volicord_session_watch",
                "status_detail": "active",
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true,
                "coverage_start_at": "2026-06-30T00:03:00Z",
                "coverage_basis": SessionWatchCoverageBasis::McpStart.as_str(),
                "coverage_started_by": "session_start_hook"
            }))?,
        },
    )?;
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
            write_ticket_ids_json: "[]".to_owned(),
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

fn assert_authority_disclosure(value: &Value) {
    let disclosure = &value["base"]["disclosure"];
    assert_eq!(disclosure["guarantee_class"], "authority_record");
    let values = disclosure["non_guarantees"]
        .as_array()
        .expect("authority disclosure should include non_guarantees")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("non_guarantees should contain strings")
        })
        .collect::<BTreeSet<_>>();
    for expected in [
        "NotCorrectnessProof",
        "NotTestSufficiencyProof",
        "NotHumanReviewReplacement",
        "NotFullWritePrevention",
        "NotFullFilesystemMonitoring",
        "NotActorAttributionProof",
        "NotIntentProof",
        "NotOsSandbox",
    ] {
        assert!(
            values.contains(expected),
            "missing non-guarantee {expected}: {disclosure}"
        );
    }
}

fn assert_coverage_non_guarantees(value: &Value) {
    let values = value["non_guarantees"]
        .as_array()
        .expect("coverage summary should include non_guarantees")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("coverage non_guarantees should contain strings")
        })
        .collect::<BTreeSet<_>>();
    for expected in [
        "NotActorAttributionProof",
        "NotFullFilesystemMonitoring",
        "NotFullWritePrevention",
    ] {
        assert!(
            values.contains(expected),
            "missing coverage non-guarantee {expected}: {value}"
        );
    }
}

fn assert_write_ticket_invalid_reason(response: &PipelineResponse, reason: &str) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_TICKET_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_ticket_reason"],
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
        write_ticket: true,
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
            acceptance_criteria: vec![volicord_types::AcceptanceCriterionInput {
                statement: "The test export flow is represented.".to_owned(),
                evidence_requirement: EvidenceRequirement::NotRequired,
            }],
        },
        initial_context_refs: Vec::new(),
        initial_source_refs: Vec::new(),
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
        acceptance_criteria: Some(vec![volicord_types::AcceptanceCriterionReplacement {
            acceptance_criterion_id: None.into(),
            statement: "The scoped behavior is represented.".to_owned(),
            evidence_requirement: EvidenceRequirement::NotRequired,
        }])
        .into(),
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
        write_ticket_id: None.into(),
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
    write_ticket_id: &str,
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
    request.write_ticket_id = Some(WriteTicketId::new(write_ticket_id)).into();
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
    let intent = match input.intent {
        CloseIntent::Complete => CloseMutationIntent::Complete,
        CloseIntent::Cancel => CloseMutationIntent::Cancel,
        CloseIntent::Supersede => CloseMutationIntent::Supersede,
        CloseIntent::Check => panic!("use check_close_request for close-readiness checks"),
    };
    CloseTaskRequest {
        envelope: envelope(
            input.request_id,
            input.idempotency_key,
            input.dry_run,
            input.expected_state_version,
            Some(input.task_id),
        ),
        task_id: TaskId::new(input.task_id),
        intent,
        close_reason: input.close_reason.into(),
        superseding_task_id: input.superseding_task_id.map(TaskId::new).into(),
        user_note: Some("Focused close-task test.".to_owned()).into(),
    }
}

fn check_close_request(input: CloseTaskFixture<'_>) -> CheckCloseRequest {
    CheckCloseRequest {
        envelope: envelope(
            input.request_id,
            input.idempotency_key,
            input.dry_run,
            input.expected_state_version,
            Some(input.task_id),
        ),
        task_id: TaskId::new(input.task_id),
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
    let host_capability_json = if host_capability_json == "{}" && guard_mode != "record" {
        complete_host_hook_capability_json(harness)?
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

fn complete_host_hook_capability_json(harness: &MethodHarness) -> Result<String, Box<dyn Error>> {
    Ok(complete_guard_capability_value(harness)?.to_string())
}

fn complete_guard_capability_value(harness: &MethodHarness) -> Result<Value, Box<dyn Error>> {
    let repo_root = product_repo_root(harness)?;
    let policy_path = repo_root.join(".volicord").join("policy.json");
    let hook_config_path = repo_root.join(".codex").join("hooks.json");
    let hooks_dir = repo_root.join(".codex").join("hooks");
    fs::create_dir_all(policy_path.parent().expect("policy path has parent"))?;
    fs::create_dir_all(
        hook_config_path
            .parent()
            .expect("hook config path has parent"),
    )?;
    fs::create_dir_all(&hooks_dir)?;
    let policy_text = r#"{"managed_by":"volicord","host_hook": {"commands":{}}}"#;
    let hook_config_text = r#"{"hooks":{"SessionStart":[{"matcher":"startup|resume","hooks":[{"type":"command","command":"sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" session-start'"}]}],"PreToolUse":[{"matcher":"Bash|apply_patch|Edit|Write|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*","hooks":[{"type":"command","command":"sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" pre-tool'"}]}],"PostToolUse":[{"matcher":"Bash|apply_patch|Edit|Write|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*","hooks":[{"type":"command","command":"sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" post-tool'"}]}],"UserPromptSubmit":[{"hooks":[{"type":"command","command":"sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" prompt-capture'"}]}],"Stop":[{"hooks":[{"type":"command","command":"sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" stop'"}]}]}}"#;
    fs::write(&policy_path, policy_text)?;
    fs::write(&hook_config_path, hook_config_text)?;
    let dispatch_path = hooks_dir.join("volicord-dispatch.sh");
    let dispatch_text = concat!(
        "#!/bin/sh\n",
        "# VOLICORD_MANAGED_HOOK_WRAPPER\n",
        "# host_kind=codex\n",
        "# phase=dispatch\n",
        "# script_role=codex_dispatch\n",
        "if [ \"$#\" -ne 1 ]; then\n",
        "    exit 64\n",
        "fi\n",
        "phase=$1\n",
        "case \"$phase\" in\n",
        "    session-start|pre-tool|post-tool|prompt-capture|stop) ;;\n",
        "    *) exit 64 ;;\n",
        "esac\n",
        "root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 70\n",
        "wrapper=\"$root/.codex/hooks/volicord-$phase.sh\"\n",
        "if [ ! -f \"$wrapper\" ] || [ ! -x \"$wrapper\" ]; then\n",
        "    exit 70\n",
        "fi\n",
        "exec \"$wrapper\"\n",
    );
    fs::write(&dispatch_path, dispatch_text)?;
    set_test_executable(&dispatch_path)?;
    let phases = [
        ("session_start_hook", "session-start", "session_start"),
        ("pre_tool_hook", "pre-tool", "pre_tool"),
        ("post_tool_hook", "post-tool", "post_tool"),
        (
            "user_prompt_submit_hook",
            "prompt-capture",
            "prompt_capture",
        ),
        ("stop_hook", "stop", "stop"),
    ];
    let mut wrapper_files = Vec::new();
    let mut host_hook_commands = Vec::new();
    for (capability_phase, command_name, policy_key) in phases {
        let wrapper_path = hooks_dir.join(format!("volicord-{command_name}.sh"));
        let wrapper_command = format!(
            "volicord _hook {command_name} --repo {} --connection {CONNECTION_ID} --guard-installation guard_installation --host codex --integration-profile detective --policy-hash sha256:guardedfixture --host-output codex",
            path_text(&repo_root),
        );
        let wrapper_text = format!(
            "#!/bin/sh\n# VOLICORD_MANAGED_HOOK_WRAPPER\n# host_kind=codex\n# phase={policy_key}\n# connection_id={CONNECTION_ID}\n# guard_installation_id=guard_installation\n# policy_hash=sha256:guardedfixture\n# host_output=codex\nexec {wrapper_command}\n"
        );
        fs::write(&wrapper_path, &wrapper_text)?;
        set_test_executable(&wrapper_path)?;
        wrapper_files.push(json!({
            "kind": "host_hook_wrapper",
            "path": path_text(&wrapper_path),
            "content_hash": sha256_text(&wrapper_text),
            "ownership": "managed_script",
            "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
            "executable_required": true,
            "managed_script_command": wrapper_command,
            "host_kind": "codex",
            "phase": policy_key,
            "connection_id": CONNECTION_ID,
            "guard_installation_id": "guard_installation",
            "policy_hash": "sha256:guardedfixture",
            "host_output": "codex"
        }));
        host_hook_commands.push(json!({
            "host_kind": "codex",
            "phase": capability_phase,
            "policy_key": policy_key,
            "command_shape": "shell_command_string",
            "command": format!("sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" {command_name}'"),
            "args": Value::Null,
            "expected_wrapper_path": path_text(&dispatch_path),
            "expected_phase_wrapper_path": path_text(&wrapper_path),
            "root_resolution_basis": "git_work_tree",
            "hook_command_path_basis": "git_root_runtime",
            "cwd_independent": true,
            "subdirectory_safe": true,
            "wrapper_resolution_status": "ok",
            "verification": {
                "basis_verified_by": "repo_root_git_marker",
                "host_contract_source": "codex_hook_command_string"
            }
        }));
    }
    let mut files = vec![
        json!({
            "kind": "volicord_policy",
            "path": path_text(&policy_path),
            "content_hash": sha256_text(policy_text),
            "ownership": "managed_json"
        }),
        json!({
            "kind": "host_hook_config",
            "path": path_text(&hook_config_path),
            "content_hash": sha256_text(hook_config_text),
            "ownership": "managed_json"
        }),
        json!({
            "kind": "host_hook_dispatch",
            "path": path_text(&dispatch_path),
            "content_hash": sha256_text(dispatch_text),
            "ownership": "managed_script",
            "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
            "executable_required": true,
            "managed_script_role": "codex_dispatch",
            "host_kind": "codex",
            "phase": "dispatch"
        }),
    ];
    files.extend(wrapper_files);
    Ok(json!({
        "schema": "volicord-host-hook-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "native_host_output_adapter": "codex",
        "native_host_output_adapter_verified": true,
        "bash_shell_mutation_coverage": true,
        "direct_file_write_matcher_coverage": true,
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_hook_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": [],
        "prompt_capture": true,
        "files": files,
        "host_hook_commands": host_hook_commands,
        "hook_path_safety": {
            "overall_status": "ok",
            "all_cwd_independent": true,
            "all_subdirectory_safe": true
        }
    }))
}

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(unix)]
fn set_test_executable(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_test_executable(_path: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
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

fn insert_write_ticket_guard_event(
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
            result_json: r#"{"reasons":[{"code":"write_ticket_missing"}]}"#.to_owned(),
            occurred_at: "2026-06-30T00:06:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn insert_write_ticket_path_scope_guard_event(
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
            event_kind: "pre_tool".to_owned(),
            decision: "deny".to_owned(),
            subject_json: "{}".to_owned(),
            result_json: serde_json::to_string(&json!({
                "reasons": [{
                    "code": "write_ticket_path_scope_violation",
                    "severity": "deny"
                }],
                "write_ticket_backing": {
                    "status": "out_of_scope",
                    "ticket_scope_violation": true,
                    "observed_paths": ["src/other.rs"],
                    "active_write_ticket_ids": ["wt_scope_fixture"]
                },
                "disclosure": {
                    "non_guarantees": ["NotFullWritePrevention", "NotActorAttributionProof", "NotOsSandbox"]
                }
            }))?,
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
    mut evidence_updates: Vec<EvidenceCoverageUpdate>,
    result_summary: &str,
) -> Result<u64, Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    if evidence_updates.len() == 1
        && matches!(
            evidence_updates[0].target,
            EvidenceTarget::SupplementalClaim { .. }
        )
    {
        let acceptance_criterion_id = active_acceptance_criterion_id(harness, task_id)?;
        set_active_acceptance_criterion_requirement(
            harness,
            task_id,
            EvidenceRequirement::Required,
        )?;
        evidence_updates[0].target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: volicord_types::AcceptanceCriterionId::new(
                acceptance_criterion_id,
            ),
        };
    }
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

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
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
        capability_claim: "This is not a write ticket.".to_owned(),
        expires_at: None.into(),
    }
}

fn prepare_write_ticket(
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
    Ok(response.response_value["write_ticket_ref"]["record_id"]
        .as_str()
        .expect("write ticket ref should be present")
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
        evidence_target: claim.map(supplemental_evidence_target).into(),
        expected_sha256: Some(handle.sha256).into(),
        expected_size_bytes: Some(handle.size_bytes).into(),
        redaction_state: Some(handle.redaction_state).into(),
    }
}

fn supplemental_evidence_target(statement: &str) -> EvidenceTarget {
    let mut hasher = Sha256::new();
    hasher.update(statement.as_bytes());
    EvidenceTarget::SupplementalClaim {
        evidence_claim_id: volicord_types::EvidenceClaimId::new(format!(
            "claim_{}",
            hex_bytes(&hasher.finalize())
        )),
        statement: statement.to_owned(),
    }
}

fn supported_evidence_update(claim: &str) -> EvidenceCoverageUpdate {
    EvidenceCoverageUpdate {
        target: supplemental_evidence_target(claim),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: Some(evidence_update_provenance(
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
        )),
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn unsupported_evidence_update(claim: &str) -> EvidenceCoverageUpdate {
    EvidenceCoverageUpdate {
        target: supplemental_evidence_target(claim),
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn evidence_update_for_acceptance_criterion(
    mut update: EvidenceCoverageUpdate,
    acceptance_criterion_id: &volicord_types::AcceptanceCriterionId,
) -> EvidenceCoverageUpdate {
    update.target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: acceptance_criterion_id.clone(),
    };
    update
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
        source_refs: Vec::new(),
        limitations: Vec::new(),
    }
}

fn supported_evidence_update_with_provenance(
    claim: &str,
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
) -> EvidenceCoverageUpdate {
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
    assert!(guidance.contains("chat command"), "{guidance}");
    assert!(guidance.contains("verification code"), "{guidance}");
    assert!(!guidance.contains("MCP elicitation"), "{guidance}");
}

fn channel_path<'a>(availability: &'a Value, kind: &str) -> &'a Value {
    let paths = availability["paths"]
        .as_array()
        .expect("user_channel_availability.paths should be an array");
    paths
        .iter()
        .find(|path| path["kind"] == kind)
        .unwrap_or_else(|| panic!("expected user channel path {kind}, got {paths:?}"))
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
    outside_chat_action_required: bool,
) {
    let blocker = close_blocker_by_code(response_value, code);
    assert_eq!(blocker["can_resolve_in_chat"], can_resolve_in_chat);
    assert_eq!(
        blocker["outside_chat_action_required"],
        outside_chat_action_required
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
    create_task_with_mode_and_change_unit(harness, prefix, RequestedMode::Work)
}

fn create_task_with_mode_and_change_unit(
    harness: &MethodHarness,
    prefix: &str,
    requested_mode: RequestedMode,
) -> Result<(String, String), Box<dyn Error>> {
    let intake_request_id = format!("req_{prefix}_task");
    let intake_idempotency_key = format!("idem_{prefix}_task");
    let intake = harness.service.intake(
        intake_request(
            &intake_request_id,
            &intake_idempotency_key,
            false,
            Some(0),
            requested_mode,
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

fn replace_acceptance_criteria_for_test(
    harness: &MethodHarness,
    task_id: &str,
    expected_state_version: u64,
    suffix: &str,
    criteria: &[(&str, EvidenceRequirement)],
) -> Result<(u64, Vec<AcceptanceCriterion>), Box<dyn Error>> {
    let current_id = active_acceptance_criterion_id(harness, task_id)?;
    let replacements = criteria
        .iter()
        .enumerate()
        .map(|(index, (statement, evidence_requirement))| {
            volicord_types::AcceptanceCriterionReplacement {
                acceptance_criterion_id: if index == 0 {
                    Some(volicord_types::AcceptanceCriterionId::new(&current_id)).into()
                } else {
                    None.into()
                },
                statement: (*statement).to_owned(),
                evidence_requirement: *evidence_requirement,
            }
        })
        .collect();
    let response = harness.service.update_scope(
        UpdateScopeRequest {
            envelope: envelope(
                &format!("req_replace_criteria_{suffix}"),
                Some(&format!("idem_replace_criteria_{suffix}")),
                false,
                Some(expected_state_version),
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            goal_summary: None.into(),
            scope_update: None.into(),
            scope_boundary: None.into(),
            non_goals: None.into(),
            acceptance_criteria: Some(replacements).into(),
            autonomy_boundary: None.into(),
            baseline_ref: None.into(),
            change_unit: ChangeUnitUpdate {
                operation: ChangeUnitOperation::KeepCurrent,
                effect_contract: None,
                fields: Map::new(),
            },
            related_scope_decision_refs: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let criteria =
        serde_json::from_value(response.response_value["state"]["acceptance_criteria"].clone())?;
    Ok((state_version, criteria))
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
                't0',
                't0'
            )",
        rusqlite::params![PROJECT_ID, task_id, AGENT_ACTOR_SOURCE],
    )?;
    Ok(())
}

fn active_acceptance_criterion_id(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<String, Box<dyn Error>> {
    Ok(harness.conn()?.query_row(
        "SELECT acceptance_criterion_id
           FROM acceptance_criteria
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
          ORDER BY position ASC
          LIMIT 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn set_active_acceptance_criterion_requirement(
    harness: &MethodHarness,
    task_id: &str,
    requirement: EvidenceRequirement,
) -> Result<(), Box<dyn Error>> {
    let value = serde_json::to_value(requirement)?;
    let value = value
        .as_str()
        .expect("evidence requirement should serialize as a string");
    harness.conn()?.execute(
        "UPDATE acceptance_criteria
            SET evidence_requirement = ?3
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'",
        rusqlite::params![PROJECT_ID, task_id, value],
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
            produced_at_state_version: expected_state_version.into(),
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
            capability_claim: "This is not a write ticket.".to_owned(),
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

fn insert_active_write_ticket(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
) -> Result<(), Box<dyn Error>> {
    insert_active_write_ticket_with_timestamps(
        harness,
        task_id,
        change_unit_id,
        "wa_replace",
        2,
        "2026-06-18T00:00:00.000Z",
        "2026-06-18T00:15:00.000Z",
    )
}

fn insert_active_write_ticket_with_timestamps(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    write_ticket_id: &str,
    basis_state_version: u64,
    created_at: &str,
    expires_at: &str,
) -> Result<(), Box<dyn Error>> {
    insert_active_write_ticket_with_scope(
        harness,
        WriteTicketScopeFixture {
            task_id,
            change_unit_id,
            write_ticket_id,
            basis_state_version,
            created_at,
            expires_at,
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            sensitive_categories: &[],
        },
    )
}

struct WriteTicketScopeFixture<'a> {
    task_id: &'a str,
    change_unit_id: &'a str,
    write_ticket_id: &'a str,
    basis_state_version: u64,
    created_at: &'a str,
    expires_at: &'a str,
    intended_operation: &'a str,
    intended_paths: &'a [&'a str],
    sensitive_categories: &'a [&'a str],
}

fn insert_active_write_ticket_with_scope(
    harness: &MethodHarness,
    input: WriteTicketScopeFixture<'_>,
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
        "INSERT INTO write_tickets (
                project_id,
                write_ticket_id,
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
            input.write_ticket_id,
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

fn mutate_write_ticket_scope_json(
    harness: &MethodHarness,
    write_ticket_id: &str,
    mutate: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT attempt_scope_json
           FROM write_tickets
          WHERE project_id = ?1
            AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    mutate(&mut value);
    conn.execute(
        "UPDATE write_tickets
            SET attempt_scope_json = ?3
          WHERE project_id = ?1
            AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id, value.to_string()],
    )?;
    Ok(())
}

struct SensitiveProductWriteBasisFixture<'a> {
    task_id: &'a str,
    change_unit_id: &'a str,
    expected_state_version: u64,
    suffix: &'a str,
    write_ticket_id: &'a str,
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
    insert_active_write_ticket_with_scope(
        harness,
        WriteTicketScopeFixture {
            task_id: input.task_id,
            change_unit_id: input.change_unit_id,
            write_ticket_id: input.write_ticket_id,
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
        input.write_ticket_id,
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

fn write_ticket_count(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM write_tickets
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
    assert!(payload["write_ticket_id"].is_null());
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

fn write_ticket_basis(
    harness: &MethodHarness,
    write_ticket_id: &str,
) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let basis: i64 = conn.query_row(
        "SELECT basis_state_version
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(basis)?)
}

fn write_ticket_timestamps(
    harness: &MethodHarness,
    write_ticket_id: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT created_at, expires_at
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id],
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

fn create_local_web_token_for_judgment(
    harness: &MethodHarness,
    token: &str,
    judgment_id: &str,
) -> Result<String, Box<dyn Error>> {
    let record = create_local_web_consent_token(
        &harness.runtime_home_path,
        LocalWebConsentTokenCreate {
            token: token.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            judgment_id: judgment_id.to_owned(),
            capture_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
            ttl_seconds: 600,
            created_metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(record.token_hash)
}

fn local_web_token_status(
    harness: &MethodHarness,
    token_hash: &str,
) -> Result<LocalWebTokenStatus, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status, consumed_at, completed_at
           FROM local_web_consent_tokens
          WHERE project_id = ?1
            AND token_hash = ?2",
        rusqlite::params![PROJECT_ID, token_hash],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
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

fn stored_run_kind(harness: &MethodHarness, run_id: &str) -> Result<String, Box<dyn Error>> {
    Ok(harness.conn()?.query_row(
        "SELECT kind
               FROM runs
              WHERE project_id = ?1
                AND run_id = ?2",
        rusqlite::params![PROJECT_ID, run_id],
        |row| row.get(0),
    )?)
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
        evidence_target: Some(supplemental_evidence_target(
            "Reused artifact for corruption coverage.",
        ))
        .into(),
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
    let acceptance_criterion_id = volicord_types::AcceptanceCriterionId::new(
        active_acceptance_criterion_id(harness, &task_id)?,
    );
    set_active_acceptance_criterion_requirement(harness, &task_id, EvidenceRequirement::Required)?;
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
    let mut artifact_input = existing_artifact_input(
        &format!("artifact_input_authority_{suffix}"),
        artifact_ref.clone(),
    );
    artifact_input.evidence_target = Some(EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: acceptance_criterion_id.clone(),
    })
    .into();
    request.artifact_inputs = vec![artifact_input];
    request.evidence_updates = vec![evidence_update_for_acceptance_criterion(
        supported_evidence_update("Reused artifact for corruption coverage."),
        &acceptance_criterion_id,
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
                write_ticket: false,
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
    harness.service.check_close(
        check_close_request(CloseTaskFixture {
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

fn write_ticket_status(
    harness: &MethodHarness,
    write_ticket_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id],
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
