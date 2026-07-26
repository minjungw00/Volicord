use std::{
    error::Error,
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::pipeline::method_result_base;
use crate::{
    CurrentUserActionFacts, CurrentUserActionRead, PendingUserActionFacts,
    PendingUserActionFactsRequest, UserActionResolutionFactsBody,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::OptionalExtension;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
        ConnectionProjectRegistration, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX,
        HOST_SCOPE_PROJECT,
    },
    bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts, TaskRevisionRecord},
    diagnostics::read_core_rejection_diagnostics,
    evidence_capture::MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES,
    guards::{
        insert_unrecorded_change, unrecorded_change, upsert_guard_installation,
        GuardInstallationUpsert, UnrecordedChangeInsert, UnrecordedChangeRecord,
    },
    workflow_records::{project_write_authority_fingerprint, ProjectWorkflowPolicyUpsert},
    RuntimeHomeMutationContext,
};
use volicord_test_support::{
    open_project_fixture_database, with_test_runtime_home_setup, TempRuntimeHome,
    TestRuntimeHomeMutation,
    TEST_FIXTURE_INVOCATION_BINDING_BASIS as VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};
use volicord_types::ids::{
    prefixed_durable_id, DurableIdError, DurableIdGenerator, DurableIdKind, IdempotencyKey,
    RequestId, SequenceDurableIdGenerator,
};
use volicord_types::methods::{ChangeUnitUpdate, InitialScope, ScopeUpdate};
use volicord_types::schema::{
    ChangeUnitEffectContract, EvidenceUpdateProvenance, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON,
};
use volicord_types::values::CloseMutationIntent;
use volicord_types::values::{
    ActorSource, ChangeUnitEffectKind, EvidenceAssuranceLevel, EvidenceSourceKind,
    OperationCategory, VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
};
use volicord_types::{
    canonical::canonical_json_size_bytes, ids::*, methods::*, schema::*, values::*,
};

use super::*;

const PROJECT_ID: &str = "project_methods";
const CONNECTION_ID: &str = "connection_methods";
const AGENT_ACTOR_SOURCE: &str = "agent_connection:connection_methods";
const LOCAL_USER_ACTOR_SOURCE: &str = "local_user";
const DEFAULT_METHOD_TEST_CLOCK: &str = "2026-06-18T00:00:00Z";

fn assert_typed_result_contract<T>(response: &PipelineResponse)
where
    T: DeserializeOwned + Serialize,
{
    let decoded: T =
        serde_json::from_str(&response.response_json).expect("typed method result should decode");
    assert_eq!(
        serde_json::to_value(decoded).expect("typed method result should serialize"),
        response.response_value,
        "typed method result should round-trip without changing the public JSON"
    );

    let mut unknown = response.response_value.clone();
    unknown
        .as_object_mut()
        .expect("method result should be an object")
        .insert("__unexpected_result_field".to_owned(), Value::Bool(true));
    assert!(
        serde_json::from_value::<T>(unknown).is_err(),
        "typed method result should reject an unknown top-level field"
    );
}

#[derive(Debug, Clone)]
struct ManualClock {
    now: Arc<Mutex<DateTime<Utc>>>,
    samples: Arc<Mutex<usize>>,
}

impl ManualClock {
    fn at(timestamp: &str) -> Self {
        let now = DateTime::parse_from_rfc3339(timestamp)
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);
        Self::from_datetime(now)
    }

    fn from_datetime(now: DateTime<Utc>) -> Self {
        Self {
            now: Arc::new(Mutex::new(now)),
            samples: Arc::new(Mutex::new(0)),
        }
    }

    fn advance(&self, duration: Duration) {
        let mut now = self
            .now
            .lock()
            .expect("manual clock mutex should not be poisoned");
        *now += duration;
    }

    fn sample_count(&self) -> usize {
        *self
            .samples
            .lock()
            .expect("manual clock sample counter mutex should not be poisoned")
    }
}

impl crate::pipeline::Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        *self
            .samples
            .lock()
            .expect("manual clock sample counter mutex should not be poisoned") += 1;
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
    service: AdmittedCoreService,
}

struct AdmittedCoreService {
    inner: CoreService,
    mutation: TestRuntimeHomeMutation,
}

impl AdmittedCoreService {
    fn context(&self) -> RuntimeHomeMutationContext<'_> {
        self.mutation
            .context()
            .expect("test mutation lease must match its Runtime Home")
    }

    fn intake(
        &self,
        request: IntakeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner.intake(&self.context(), request, invocation)
    }

    fn update_scope(
        &self,
        request: UpdateScopeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .update_scope(&self.context(), request, invocation)
    }

    fn prepare_write(
        &self,
        request: PrepareWriteRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .prepare_write(&self.context(), request, invocation)
    }

    fn record_run(
        &self,
        request: RecordRunRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner.record_run(&self.context(), request, invocation)
    }

    fn stage_artifact(
        &self,
        request: StageArtifactRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .stage_artifact(&self.context(), request, invocation)
    }

    fn prepare_evidence_capture(
        &self,
        request: PrepareEvidenceCaptureRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .prepare_evidence_capture(&self.context(), request, invocation)
    }

    fn request_user_action(
        &self,
        request: RequestUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .request_user_action(&self.context(), request, invocation)
    }

    fn resolve_user_action(
        &self,
        request: ResolveUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .resolve_user_action(&self.context(), request, invocation)
    }

    fn reconcile_changes(
        &self,
        request: ReconcileChangesRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner
            .reconcile_changes(&self.context(), request, invocation)
    }

    fn close_task(
        &self,
        request: CloseTaskRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        self.inner.close_task(&self.context(), request, invocation)
    }
}

impl Deref for AdmittedCoreService {
    type Target = CoreService;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
struct ContinuityRecordRow {
    kind: String,
}

impl MethodHarness {
    fn new() -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("core-methods")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        with_test_runtime_home_setup(runtime_home.path(), |context| {
            initialize_runtime_home(context, "runtime_home_methods", "{}")?;
            register_project(
                context,
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                context,
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
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                context,
                ConnectionProjectRegistration {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                },
            )?;
            Ok(())
        })?;

        let runtime_home_path = runtime_home.path().to_path_buf();
        open_project_fixture_database(
            runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?
        .execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            rusqlite::params![PROJECT_ID, DEFAULT_METHOD_TEST_CLOCK],
        )?;
        let mutation = TestRuntimeHomeMutation::acquire(&runtime_home_path)?;
        let service = CoreService::for_mutation_with_clock(
            &mutation.context()?,
            ManualClock::at(DEFAULT_METHOD_TEST_CLOCK),
        );
        Ok(Self {
            _runtime_home: runtime_home,
            runtime_home_path: runtime_home_path.clone(),
            service: AdmittedCoreService {
                inner: service,
                mutation,
            },
        })
    }

    fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        let store =
            CoreProjectStore::open_read_only(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
        Ok(store.effect_counts()?)
    }

    fn conn(&self) -> Result<rusqlite::Connection, Box<dyn Error>> {
        Ok(open_project_fixture_database(
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
            "SELECT kind
             FROM project_continuity_records
             WHERE project_id = ?1
             ORDER BY julianday(created_at), continuity_record_id",
        )?;
        let rows = stmt.query_map([PROJECT_ID], |row| {
            Ok(ContinuityRecordRow { kind: row.get(0)? })
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

    fn set_workflow_policy(&self, workflow: Value) -> Result<(), Box<dyn Error>> {
        self.set_workflow_policy_version(1, workflow)
    }

    fn set_workflow_policy_version(
        &self,
        policy_version: u64,
        workflow: Value,
    ) -> Result<(), Box<dyn Error>> {
        let policy_value = json!({
            "schema": volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID,
            "workflow": workflow,
        });
        let policy_json = volicord_types::canonical::canonical_json_string(&policy_value)?;
        let policy_fingerprint = volicord_types::canonical::canonical_json_sha256(&policy_value)?
            .as_str()
            .to_owned();
        let context = self.service.context();
        let mut store = CoreProjectStore::open_for_mutation(&context, &ProjectId::new(PROJECT_ID))?;
        store.upsert_project_workflow_policy(ProjectWorkflowPolicyUpsert {
            policy_version,
            policy_json,
            policy_fingerprint,
            source: "test_fixture".to_owned(),
            applied_at: DEFAULT_METHOD_TEST_CLOCK.to_owned(),
            created_at: DEFAULT_METHOD_TEST_CLOCK.to_owned(),
        })?;
        Ok(())
    }

    fn use_generator_and_clock(
        &mut self,
        generator: CountingDurableIdGenerator,
        clock: ManualClock,
    ) {
        self.service.inner = CoreService::for_mutation_with_id_generator_and_clock(
            &self.service.context(),
            generator,
            clock,
        );
    }

    fn use_clock(&mut self, clock: ManualClock) {
        self.service.inner = CoreService::for_mutation_with_clock(&self.service.context(), clock);
    }
}

fn response_record_id(response_value: &Value, field: &str) -> String {
    if field == "user_action_request_ref" {
        return response_value["user_action_request_summary"]["user_action_request_id"]
            .as_str()
            .expect("agent-safe user-action request summary should identify the request")
            .to_owned();
    }
    response_value[field]["record_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{field}.record_id should be present: {response_value}"))
        .to_owned()
}

fn response_event_id(response_value: &Value) -> String {
    response_value["base"]["events"][0]["event_id"]
        .as_str()
        .expect("event_id should be present")
        .to_owned()
}

fn pending_user_action_summary(user_action_request_id: &str) -> Value {
    json!({
        "user_action_request_id": user_action_request_id,
        "status": "pending",
        "next_actor": "user"
    })
}

fn local_pending_user_action_facts(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<PendingUserActionFacts, Box<dyn Error>> {
    let service = CoreService::for_read_only_with_clock(
        &harness.runtime_home_path,
        ManualClock::at(DEFAULT_METHOD_TEST_CLOCK),
    );
    Ok(service
        .pending_user_action_facts(
            PendingUserActionFactsRequest {
                project_id: ProjectId::new(PROJECT_ID),
                task_id: TaskId::new(task_id),
            },
            InvocationContext::local_user(
                ProjectId::new(PROJECT_ID),
                OperationCategory::Read,
                UserActionChannelKind::Cli,
            ),
        )?
        .expect("authenticated local user should receive pending semantic facts"))
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

mod close_readiness;
mod close_task;
mod intake;
mod operation_result;
mod preflight;
mod prepare_evidence_capture;
mod prepare_write;
mod projection_boundary;
mod reconcile_changes;
mod record_run;
mod replay;
mod stage_artifact;
mod status;
mod update_scope;
mod user_action;
mod user_actions;
mod workflow_metrics;

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
    if operation_category == OperationCategory::UserOnly {
        return InvocationContext::local_user(
            ProjectId::new(PROJECT_ID),
            operation_category,
            UserActionChannelKind::Cli,
        );
    }
    invocation_with_actor(
        actor_source_for_operation_category(operation_category),
        operation_category,
    )
}

fn invocation_with_session(
    operation_category: OperationCategory,
    session_id: &str,
) -> InvocationContext {
    if operation_category == OperationCategory::UserOnly {
        return invocation(operation_category).with_session_id(session_id.to_owned());
    }
    InvocationContext::agent_connection(
        operation_category,
        crate::agent_session::validated_agent_session_for_test_with_project_session(
            CONNECTION_ID,
            PROJECT_ID,
            session_id,
        ),
    )
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
    match actor_source {
        ActorSource::AgentConnection(connection_id) => InvocationContext::agent_connection(
            operation_category,
            crate::agent_session::validated_agent_session_for_test(
                connection_id.as_str(),
                PROJECT_ID,
            ),
        ),
        ActorSource::LocalUser => InvocationContext::local_user(
            ProjectId::new(PROJECT_ID),
            operation_category,
            UserActionChannelKind::Cli,
        ),
        ActorSource::System => panic!("system authority is not a public Core invocation input"),
    }
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

fn product_repo_root(harness: &MethodHarness) -> Result<PathBuf, Box<dyn Error>> {
    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    Ok(store.project_record().repo_root.clone())
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
        if operation_category == OperationCategory::UserOnly {
            VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL.to_owned()
        } else {
            crate::agent_session::validated_agent_session_for_test(CONNECTION_ID, PROJECT_ID)
                .verification_basis()
        }
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
    assert_store_rejection(response, "PERSISTED_DATA_CORRUPT", corruption_category);
    assert_eq!(response.response_value["errors"][0]["category"], "corrupt");
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
        pending_user_actions: true,
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
) -> volicord_types::methods::IntakeRequest {
    volicord_types::methods::IntakeRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            None,
        ),
        plain_language_request: "Create a test export flow.".to_owned(),
        requested_mode,
        requested_control_level: RequestedControlLevel::Auto,
        resume_policy: ResumePolicy::CreateNew,
        acceptance_policy: RequiredNullable::null(),
        lineage: RequiredNullable::null(),
        initial_scope: InitialScope {
            boundary: "Initial test scope.".to_owned(),
            non_goals: vec!["Changing unrelated flows.".to_owned()],
            acceptance_criteria: vec![volicord_types::schema::AcceptanceCriterionInput {
                statement: "The test export flow is represented.".to_owned(),
                evidence_requirement: EvidenceRequirement::NotRequired,
            }],
        },
        initial_context_refs: Vec::new(),
        initial_source_refs: Vec::new(),
    }
}

fn light_workflow_policy() -> Value {
    json!({
        "default_direct_control": "light",
        "default_work_control": "tracked",
        "light": {
            "enabled": true,
            "max_intended_paths": 2,
            "allowed_path_patterns": ["src", "tests"],
            "denied_path_patterns": ["src/denied"],
            "final_acceptance": "policy_dependent"
        },
        "write_ticket": { "idle_timeout_minutes": null }
    })
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
        acceptance_criteria: Some(vec![
            volicord_types::schema::AcceptanceCriterionReplacement {
                acceptance_criterion_id: None.into(),
                statement: "The scoped behavior is represented.".to_owned(),
                evidence_requirement: EvidenceRequirement::NotRequired,
            },
        ])
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
        kind: volicord_types::values::RunKind::Implementation,
        run_id: None.into(),
        baseline_ref: BaselineRef::new("baseline_test"),
        write_ticket_id: None.into(),
        performed_operation: Some("local_sensitive_step".to_owned()).into(),
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
) -> Result<String, Box<dyn Error>> {
    const POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let guard_installation_id = format!("guard_installation_{suffix}");
    let repo_root = harness._runtime_home.product_repo_path("repo");
    let context = harness.service.context();
    upsert_guard_installation(
        &context,
        GuardInstallationUpsert {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            manifest_json: volicord_test_support::test_guard_manifest_json(
                &harness.runtime_home_path,
                &repo_root,
                PROJECT_ID,
                CONNECTION_ID,
                &guard_installation_id,
                POLICY_HASH,
            ),
        },
    )?;
    Ok(guard_installation_id)
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
    let context = harness.service.context();
    insert_unrecorded_change(
        &context,
        project_id,
        UnrecordedChangeInsert {
            unrecorded_change_id: unrecorded_change_id.clone(),
            correlation: None,
            connection_internal_id: CONNECTION_ID.to_owned(),
            task_id,
            confidence: UnrecordedChangeConfidence::Confirmed.as_str().to_owned(),
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
    let context = harness.service.context();
    register_project(
        &context,
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        &context,
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

struct UserObservationFixture<'a> {
    task_id: &'a str,
    change_unit_id: &'a str,
    expected_state_version: u64,
    suffix: &'a str,
    target: EvidenceTarget,
    artifact_ref: &'a ArtifactRef,
    relevance_status: EvidenceRelevanceStatus,
}

fn request_and_resolve_user_observation(
    harness: &MethodHarness,
    input: UserObservationFixture<'_>,
) -> Result<(u64, StateRecordRef), Box<dyn Error>> {
    let UserObservationFixture {
        task_id,
        change_unit_id,
        expected_state_version,
        suffix,
        target,
        artifact_ref,
        relevance_status,
    } = input;
    let requested = harness.service.request_user_action(
        volicord_types::methods::RequestUserActionRequest {
            envelope: envelope(
                &format!("req_user_action_observation_{suffix}"),
                Some(&format!("idem_user_action_observation_{suffix}")),
                false,
                Some(expected_state_version),
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
            action: volicord_types::schema::UserActionDraft::EvidenceObservation(
                volicord_types::schema::UserActionEvidenceObservationDraft {
                    question: "Does this exact artifact support the selected target?".to_owned(),
                    context_summary: "The user must inspect the exact candidate bytes.".to_owned(),
                    target_candidates: vec![target.clone()],
                    artifact_candidate_ids: vec![artifact_ref.artifact_id.clone()],
                },
            ),
            required_for: vec![volicord_types::values::UserActionRequiredFor::RecordRun],
            expires_at: None.into(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id =
        response_record_id(&requested.response_value, "user_action_request_ref");
    let resolved = harness.service.resolve_user_action(
        volicord_types::methods::ResolveUserActionRequest {
            envelope: envelope(
                &format!("req_user_action_observation_resolve_{suffix}"),
                Some(&format!("submission_user_action_observation_{suffix}")),
                false,
                None,
                Some(task_id),
            ),
            user_action_request_id: volicord_types::ids::UserActionRequestId::new(
                user_action_request_id,
            ),
            channel_submission_id: format!("submission_user_action_observation_{suffix}"),
            resolution: volicord_types::schema::UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids: vec![artifact_ref.artifact_id.clone()],
                relevance_status,
                summary: "The user assessed the exact candidate bytes.".to_owned(),
            },
        },
        invocation(OperationCategory::UserOnly),
    )?;
    let state_version = resolved.response_value["base"]["state_version"]
        .as_u64()
        .expect("user-action resolution state version");
    let resolution_ref: StateRecordRef =
        serde_json::from_value(resolved.response_value["user_action_resolution_ref"].clone())?;
    let raw_resolution: (String, String) = harness.conn()?.query_row(
        "SELECT action_kind, resolution_json
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let decoded_resolution: volicord_types::schema::UserActionResolutionBody =
        serde_json::from_str(&raw_resolution.1).unwrap_or_else(|error| {
            panic!(
                "fresh resolution JSON should decode: {error}: {}",
                raw_resolution.1
            )
        });
    decoded_resolution.validate().unwrap_or_else(|error| {
        panic!(
            "fresh resolution JSON should validate (field={}, message={}): {}",
            error.field(),
            error.message(),
            raw_resolution.1
        )
    });
    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    store
        .user_action_resolution_record(resolution_ref.record_id.as_str())
        .unwrap_or_else(|error| {
            panic!(
                "fresh user-action resolution should reread (kind={}, json={}): {error}",
                raw_resolution.0, raw_resolution.1
            )
        })
        .expect("fresh user-action resolution should exist");
    Ok((state_version, resolution_ref))
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
            acceptance_criterion_id: volicord_types::ids::AcceptanceCriterionId::new(
                acceptance_criterion_id,
            ),
        };
    }
    let mut current_state_version = expected_state_version;
    let mut evidence_observations = Vec::new();
    for (index, update) in evidence_updates.iter().enumerate() {
        if update.provenance.as_ref().is_some_and(|provenance| {
            provenance.source_kind == EvidenceSourceKind::ExternalTool
                && provenance.assurance_level == EvidenceAssuranceLevel::ExternalToolResult
        }) {
            let artifact_suffix = format!("close_evidence_{suffix}_{index}");
            let handle = stage_artifact_for_record_run(
                harness,
                task_id,
                &artifact_suffix,
                current_state_version,
            )?;
            let mut artifact_input = artifact_input_for_handle(
                &format!("artifact_input_{artifact_suffix}"),
                handle,
                Some("evidence observation output"),
                None,
            );
            artifact_input.evidence_target = Some(update.target.clone()).into();
            let mut promotion = record_run_request(
                &format!("req_promote_{artifact_suffix}"),
                &format!("idem_promote_{artifact_suffix}"),
                false,
                Some(current_state_version),
                task_id,
                change_unit_id,
            );
            promotion.artifact_inputs = vec![artifact_input];
            let promoted = harness
                .service
                .record_run(promotion, invocation(OperationCategory::AgentWorkflow))?;
            current_state_version = promoted.response_value["base"]["state_version"]
                .as_u64()
                .expect("artifact promotion state version");
            let artifact_ref: ArtifactRef =
                serde_json::from_value(promoted.response_value["registered_artifacts"][0].clone())?;
            let (resolved_state_version, user_observation_ref) =
                request_and_resolve_user_observation(
                    harness,
                    UserObservationFixture {
                        task_id,
                        change_unit_id,
                        expected_state_version: current_state_version,
                        suffix: &artifact_suffix,
                        target: update.target.clone(),
                        artifact_ref: &artifact_ref,
                        relevance_status: EvidenceRelevanceStatus::Supported,
                    },
                )?;
            current_state_version = resolved_state_version;
            evidence_observations.push(EvidenceObservationInput {
                target: update.target.clone(),
                source_kind: EvidenceSourceKind::UserObservation,
                assurance_level: EvidenceAssuranceLevel::UserObserved,
                observed_by_actor_source: None.into(),
                tool_name: None.into(),
                tool_invocation_id: None.into(),
                tool_metadata: JsonObject::new(),
                input_refs: vec![user_observation_ref],
                source_refs: Vec::new(),
                output_artifact_refs: vec![artifact_ref],
                limitations: Vec::new(),
                observed_at: UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
            });
        }
    }
    for update in &mut evidence_updates {
        if update.provenance.as_ref().is_some_and(|provenance| {
            provenance.source_kind == EvidenceSourceKind::ExternalTool
                && provenance.assurance_level == EvidenceAssuranceLevel::ExternalToolResult
        }) {
            update.provenance = None;
        }
    }
    let request_id = format!("req_close_evidence_{suffix}");
    let idempotency_key = format!("idem_close_evidence_{suffix}");
    let mut request = record_run_request(
        &request_id,
        &idempotency_key,
        false,
        Some(current_state_version),
        task_id,
        change_unit_id,
    );
    request.evidence_observations = evidence_observations;
    request.evidence_updates = evidence_updates;
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
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
    assert_eq!(
        response.response_value["base"]["response_kind"], "result",
        "close-evidence helper should commit: {}",
        response.response_value
    );
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
    residual_risks: Vec<volicord_types::schema::ResidualRiskInput>,
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
    let action = harness.service.request_user_action(
        user_action_request(
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
    let user_action_request_id =
        response_record_id(&action.response_value, "user_action_request_ref");
    let record_request_id = format!("req_close_final_record_{suffix}");
    let record_idempotency_key = format!("idem_close_final_record_{suffix}");
    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            &record_request_id,
            &record_idempotency_key,
            None,
            task_id,
            &user_action_request_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, user_action_request_id))
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
    let action = harness.service.request_user_action(
        user_action_request(
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
    let user_action_request_id =
        response_record_id(&action.response_value, "user_action_request_ref");
    let record_request_id = format!("req_cancel_authority_record_{suffix}");
    let record_idempotency_key = format!("idem_cancel_authority_record_{suffix}");
    let request = resolve_user_action_request(
        &record_request_id,
        &record_idempotency_key,
        None,
        task_id,
        &user_action_request_id,
        if accepted { "accept" } else { "reject" },
    );
    let response = harness
        .service
        .resolve_user_action(request, invocation(OperationCategory::UserOnly))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    Ok((state_version, user_action_request_id))
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
    let action = harness.service.request_user_action(
        user_action_request(
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
    let user_action_request_id =
        response_record_id(&action.response_value, "user_action_request_ref");
    let record_request_id = format!("req_scope_authority_record_{suffix}");
    let record_idempotency_key = format!("idem_scope_authority_record_{suffix}");
    let request = resolve_user_action_request(
        &record_request_id,
        &record_idempotency_key,
        None,
        task_id,
        &user_action_request_id,
        if accepted { "accept" } else { "reject" },
    );
    let response = harness
        .service
        .resolve_user_action(request, invocation(OperationCategory::UserOnly))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let resolution_ref =
        serde_json::from_value(response.response_value["user_action_resolution_ref"].clone())?;
    Ok((state_version, resolution_ref, user_action_request_id))
}

fn sensitive_scope(
    action_kind: &str,
    intended_paths: Vec<&str>,
    sensitive_categories: Vec<&str>,
) -> volicord_types::schema::SensitiveActionScope {
    volicord_types::schema::SensitiveActionScope {
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
        artifact_input_id: volicord_types::ids::ArtifactInputId::new(artifact_input_id),
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
        evidence_claim_id: volicord_types::ids::EvidenceClaimId::new(format!(
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
    acceptance_criterion_id: &volicord_types::ids::AcceptanceCriterionId,
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
    residual_risks: Vec<volicord_types::schema::ResidualRiskInput>,
) -> volicord_types::schema::CloseAssessmentInput {
    volicord_types::schema::CloseAssessmentInput {
        result_summary: summary.to_owned(),
        result_refs: Vec::new(),
        residual_risks,
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    }
}

fn residual_risk_input(summary: &str) -> volicord_types::schema::ResidualRiskInput {
    volicord_types::schema::ResidualRiskInput {
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

fn assert_close_blocker_resolution(response_value: &Value, code: &str) {
    let blocker = close_blocker_by_code(response_value, code);
    assert!(blocker.get("can_resolve_in_chat").is_none());
    assert!(blocker.get("outside_chat_action_required").is_none());
    let next_actions = blocker["next_actions"]
        .as_array()
        .expect("guard blocker next_actions should be an array");
    assert!(
        !next_actions.is_empty(),
        "guard blocker should include a next action: {blocker:?}"
    );
    for action in next_actions {
        assert!(
            action["owner_method"].is_string(),
            "workflow next action must identify its retry owner: {action:?}"
        );
        assert!(
            action["allowed_operation_categories"]
                .as_array()
                .is_some_and(|categories| !categories.is_empty()),
            "workflow next action must identify an allowed operation category: {action:?}"
        );
    }
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
    create_task_with_policy_and_change_unit(harness, prefix, requested_mode, None)
}

fn create_task_with_policy_and_change_unit(
    harness: &MethodHarness,
    prefix: &str,
    requested_mode: RequestedMode,
    acceptance_policy: Option<AcceptancePolicy>,
) -> Result<(String, String), Box<dyn Error>> {
    let initial_state_version = harness.counts()?.state_version;
    let intake_request_id = format!("req_{prefix}_task");
    let intake_idempotency_key = format!("idem_{prefix}_task");
    let mut request = intake_request(
        &intake_request_id,
        &intake_idempotency_key,
        false,
        Some(initial_state_version),
        requested_mode,
    );
    request.acceptance_policy = acceptance_policy.into();
    let intake = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
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
            Some(initial_state_version + 1),
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
            volicord_types::schema::AcceptanceCriterionReplacement {
                acceptance_criterion_id: if index == 0 {
                    Some(volicord_types::ids::AcceptanceCriterionId::new(&current_id)).into()
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
                requested_control_level,
                effective_control_level,
                control_level_reason,
                work_phase,
                acceptance_policy,
                acceptance_policy_reason,
                carry_forward_json,
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
                'tracked',
                'tracked',
                'Superseding work uses tracked control.',
                'shaping',
                'required',
                'Superseding work requires explicit acceptance.',
                '[]',
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

fn user_action_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    change_unit_id: Option<&str>,
    judgment_kind: JudgmentKind,
) -> volicord_types::methods::RequestUserActionRequest {
    let options = if matches!(
        judgment_kind,
        JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision
    ) {
        vec![
            volicord_types::schema::UserActionOptionInput {
                option_id: volicord_types::ids::UserActionOptionId::new("accept"),
                label: "Accept".to_owned(),
                description: "Record the focused user-owned judgment.".to_owned(),
                consequence: "Only this judgment record is resolved.".to_owned(),
                is_default: true,
            },
            volicord_types::schema::UserActionOptionInput {
                option_id: volicord_types::ids::UserActionOptionId::new("decline"),
                label: "Decline".to_owned(),
                description: "Record that the focused judgment was not accepted.".to_owned(),
                consequence: "The Task remains unresolved for this question.".to_owned(),
                is_default: false,
            },
        ]
    } else {
        Vec::new()
    };

    volicord_types::methods::RequestUserActionRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: change_unit_id.map(ChangeUnitId::new).into(),
        action: volicord_types::schema::UserActionDraft::Choice(Box::new(
            volicord_types::schema::UserActionChoiceDraft {
                judgment_kind,
                presentation: volicord_types::values::JudgmentPresentation::Short,
                question: "Choose the focused test user-action outcome.".to_owned(),
                options: (!options.is_empty()).then_some(options).into(),
                context: volicord_types::schema::UserActionContext {
                    summary: "A focused test user action needs a user-owned answer.".to_owned(),
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                    visible_risks: Vec::new(),
                    constraints: vec![
                        "The answer covers only the requested action kind.".to_owned()
                    ],
                },
                affected_refs: vec![StateRecordRef {
                    record_kind: StateRecordKind::Task,
                    record_id: RecordId::new(task_id),
                    project_id: ProjectId::new(PROJECT_ID),
                    task_id: Some(TaskId::new(task_id)).into(),
                    produced_at_state_version: expected_state_version.into(),
                }],
                sensitive_action_scope: sensitive_action_scope_for_kind(judgment_kind).into(),
            },
        )),
        required_for: required_for_for_kind(judgment_kind),
        expires_at: None.into(),
    }
}

fn observation_action_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: u64,
    task_id: &str,
    change_unit_id: &str,
    target: EvidenceTarget,
    artifact_ids: Vec<volicord_types::ids::ArtifactId>,
) -> volicord_types::methods::RequestUserActionRequest {
    volicord_types::methods::RequestUserActionRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            false,
            Some(expected_state_version),
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
        action: volicord_types::schema::UserActionDraft::EvidenceObservation(
            volicord_types::schema::UserActionEvidenceObservationDraft {
                question: "Does the selected artifact support this exact target?".to_owned(),
                context_summary: "The user must inspect the candidate artifact bytes.".to_owned(),
                target_candidates: vec![target],
                artifact_candidate_ids: artifact_ids,
            },
        ),
        required_for: vec![volicord_types::values::UserActionRequiredFor::RecordRun],
        expires_at: None.into(),
    }
}

fn required_for_for_kind(
    judgment_kind: JudgmentKind,
) -> Vec<volicord_types::values::UserActionRequiredFor> {
    match judgment_kind {
        JudgmentKind::ScopeDecision => {
            vec![volicord_types::values::UserActionRequiredFor::ScopeUpdate]
        }
        JudgmentKind::SensitiveApproval => vec![
            volicord_types::values::UserActionRequiredFor::PrepareWrite,
            volicord_types::values::UserActionRequiredFor::CloseComplete,
        ],
        JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance => {
            vec![volicord_types::values::UserActionRequiredFor::CloseComplete]
        }
        JudgmentKind::Cancellation => {
            vec![volicord_types::values::UserActionRequiredFor::CloseCancel]
        }
        JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision => {
            vec![volicord_types::values::UserActionRequiredFor::CloseComplete]
        }
    }
}

fn sensitive_action_scope_for_kind(
    judgment_kind: JudgmentKind,
) -> Option<volicord_types::schema::SensitiveActionScope> {
    match judgment_kind {
        JudgmentKind::SensitiveApproval => Some(volicord_types::schema::SensitiveActionScope {
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

fn resolve_user_action_request(
    request_id: &str,
    channel_submission_id: &str,
    expected_state_version: Option<u64>,
    task_id: &str,
    user_action_request_id: &str,
    selected_option_id: &str,
) -> volicord_types::methods::ResolveUserActionRequest {
    volicord_types::methods::ResolveUserActionRequest {
        envelope: envelope(
            request_id,
            Some(channel_submission_id),
            false,
            expected_state_version,
            Some(task_id),
        ),
        user_action_request_id: volicord_types::ids::UserActionRequestId::new(
            user_action_request_id,
        ),
        channel_submission_id: channel_submission_id.to_owned(),
        resolution: volicord_types::schema::UserActionResolutionInput::Choice {
            selected_option_id: volicord_types::ids::UserActionOptionId::new(selected_option_id),
            note: Some("Recorded by the focused user-action test.".to_owned()).into(),
        },
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
    let scope_revision: i64 = conn.query_row(
        "SELECT scope_revision FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, input.task_id],
        |row| row.get(0),
    )?;
    let policy_json = conn
        .query_row(
            "SELECT policy_json
               FROM project_workflow_policies
              WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let write_authority_fingerprint = project_write_authority_fingerprint(policy_json.as_deref())?;
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
    let validity_basis_json = json!({
        "task_id": input.task_id,
        "change_unit_id": input.change_unit_id,
        "scope_revision": scope_revision,
        "baseline_ref": "baseline_test",
        "workspace_context_sha256": null,
        "write_authority_fingerprint": write_authority_fingerprint,
        "approval_basis_refs": []
    })
    .to_string();
    let allowed_path_prefixes_json = serde_json::to_string(input.intended_paths)?;
    conn.execute(
        "INSERT INTO write_tickets (
                project_id,
                write_ticket_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                validity_basis_json,
                allowed_path_prefixes_json,
                denied_path_prefixes_json,
                attempt_scope_json,
                created_by_actor_source,
                idle_expires_at,
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
                ?9,
                ?10,
                ?11,
                ?12
            )",
        rusqlite::params![
            PROJECT_ID,
            input.write_ticket_id,
            input.task_id,
            input.change_unit_id,
            i64::try_from(input.basis_state_version)?,
            validity_basis_json,
            allowed_path_prefixes_json,
            "[]",
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

fn mutate_write_ticket_validity_basis_json(
    harness: &MethodHarness,
    write_ticket_id: &str,
    mutate: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT validity_basis_json
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
            SET validity_basis_json = ?3
          WHERE project_id = ?1
            AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id, value.to_string()],
    )?;
    Ok(())
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
               FROM authority_events
              WHERE project_id = ?1
                AND task_id IS NOT NULL
                AND event_type = 'write_decision_recorded'",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count)?)
}

fn latest_authority_event(harness: &MethodHarness) -> Result<(String, Value, u64), Box<dyn Error>> {
    let conn = harness.conn()?;
    let (event_kind, event_payload_text, state_version): (String, String, i64) = conn.query_row(
        "SELECT event_type, payload_json, state_version
                   FROM authority_events
                  WHERE project_id = ?1
                    AND task_id IS NOT NULL
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
    let (event_kind, payload, event_state_version) = latest_authority_event(harness)?;
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
) -> Result<(String, Option<String>), Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT created_at, idle_expires_at
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

fn user_action_status(
    harness: &MethodHarness,
    user_action_request_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    let (basis_status, has_resolution, expired): (String, bool, bool) = conn.query_row(
        "SELECT
                 request.basis_status,
                 EXISTS (
                   SELECT 1
                     FROM user_action_resolutions AS resolution
                    WHERE resolution.project_id = request.project_id
                      AND resolution.user_action_request_id = request.user_action_request_id
                 ),
                 request.expires_at IS NOT NULL
                   AND julianday(request.expires_at) <= julianday('now')
               FROM user_action_requests AS request
              WHERE request.project_id = ?1
                AND request.user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if basis_status != "current" {
        return Ok(basis_status);
    }
    if has_resolution {
        return Ok("resolved".to_owned());
    }
    if expired {
        return Ok("expired".to_owned());
    }
    Ok("pending".to_owned())
}

fn user_action_basis_status(
    harness: &MethodHarness,
    user_action_request_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT basis_status
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?)
}

fn user_action_resolution_outcome(
    harness: &MethodHarness,
    user_action_request_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let value = resolution_json(harness, user_action_request_id)?;
    Ok(value
        .get("resolution_outcome")
        .and_then(Value::as_str)
        .map(str::to_owned))
}

fn clear_user_action_actor_provenance(
    harness: &MethodHarness,
    user_action_request_id: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolved_verification_basis = '',
                resolved_assurance_level = ''
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn resolution_json(
    harness: &MethodHarness,
    user_action_request_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT resolution_json
               FROM user_action_resolutions
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&text)?)
}

fn task_revision(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<TaskRevisionRecord, Box<dyn Error>> {
    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
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

fn set_task_baseline_owner_state(
    harness: &MethodHarness,
    task_id: &str,
    baseline_ref: &str,
) -> Result<(), Box<dyn Error>> {
    let raw: String = harness.conn()?.query_row(
        "SELECT shaping_summary_json
           FROM tasks
          WHERE project_id = ?1
            AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    let mut shaping: Value = serde_json::from_str(&raw)?;
    shaping["baseline_ref"] = json!(baseline_ref);
    set_task_owner_json(
        harness,
        task_id,
        "shaping_summary_json",
        Some(&serde_json::to_string(&shaping)?),
    )
}

fn set_task_initial_source_refs_owner_state(
    harness: &MethodHarness,
    task_id: &str,
    source_refs: &[SourceRef],
) -> Result<(), Box<dyn Error>> {
    let raw: String = harness.conn()?.query_row(
        "SELECT shaping_summary_json
           FROM tasks
          WHERE project_id = ?1
            AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    let mut shaping: Value = serde_json::from_str(&raw)?;
    shaping["initial_source_refs"] = serde_json::to_value(source_refs)?;
    set_task_owner_json(
        harness,
        task_id,
        "shaping_summary_json",
        Some(&serde_json::to_string(&shaping)?),
    )
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

fn set_user_action_resolution_json(
    harness: &MethodHarness,
    user_action_request_id: &str,
    value: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    let existing_resolution_id = conn
        .query_row(
            "SELECT user_action_resolution_id
               FROM user_action_resolutions
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            rusqlite::params![PROJECT_ID, user_action_request_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    drop(conn);

    if existing_resolution_id.is_none() {
        let task_id: String = harness.conn()?.query_row(
            "SELECT task_id
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            rusqlite::params![PROJECT_ID, user_action_request_id],
            |row| row.get(0),
        )?;
        let response = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!("req_corrupt_resolution_{user_action_request_id}"),
                &format!("submission_corrupt_resolution_{user_action_request_id}"),
                None,
                &task_id,
                user_action_request_id,
                "accept",
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");
    }

    let conn = harness.conn()?;
    let user_action_resolution_id: String = conn.query_row(
        "SELECT user_action_resolution_id
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolution_json = ?3
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_resolution_id, value],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(user_action_resolution_id)
}

fn set_user_action_resolution_actor(
    harness: &MethodHarness,
    user_action_request_id: &str,
    actor_kind: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolved_by_actor_source = ?3
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id, actor_kind],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn set_user_action_resolved_by_actor_source(
    harness: &MethodHarness,
    user_action_request_id: &str,
    role: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.pragma_update(None, "ignore_check_constraints", true)?;
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolved_by_actor_source = ?3
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id, role],
    )?;
    conn.pragma_update(None, "ignore_check_constraints", false)?;
    Ok(())
}

fn set_user_action_required_for(
    harness: &MethodHarness,
    user_action_request_id: &str,
    required_for: &[volicord_types::values::UserActionRequiredFor],
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT request_json
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    value["required_for"] = serde_json::to_value(required_for)?;
    conn.execute(
        "UPDATE user_action_requests
            SET request_json = ?3,
                required_for_json = ?4
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            user_action_request_id,
            value.to_string(),
            serde_json::to_string(required_for)?
        ],
    )?;
    Ok(())
}

fn set_user_action_affected_refs(
    harness: &MethodHarness,
    user_action_request_id: &str,
    affected_refs: &[StateRecordRef],
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT request_json
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    value["body"]["affected_refs"] = serde_json::to_value(affected_refs)?;
    set_user_action_owner_json(
        harness,
        user_action_request_id,
        "request_json",
        Some(&value.to_string()),
    )
}

fn set_user_action_expires_at(
    harness: &MethodHarness,
    user_action_request_id: &str,
    expires_at: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT request_json
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    value["expires_at"] = json!(expires_at);
    conn.execute(
        "UPDATE user_action_requests
            SET request_json = ?3,
                expires_at = ?4
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            user_action_request_id,
            value.to_string(),
            expires_at
        ],
    )?;
    Ok(())
}

fn set_user_action_owner_json(
    harness: &MethodHarness,
    user_action_request_id: &str,
    logical_column: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "request_json" => {
            "UPDATE user_action_requests
                SET request_json = ?3
              WHERE project_id = ?1
                AND user_action_request_id = ?2"
        }
        "basis_json" => {
            "UPDATE user_action_requests
                SET basis_json = ?3
              WHERE project_id = ?1
                AND user_action_request_id = ?2"
        }
        "resolution_json" => {
            "UPDATE user_action_resolutions
                SET resolution_json = ?3
              WHERE project_id = ?1
                AND user_action_request_id = ?2"
        }
        _ => panic!("unsupported user-action owner JSON column {logical_column}"),
    };
    harness.conn()?.execute(
        sql,
        rusqlite::params![PROJECT_ID, user_action_request_id, value],
    )?;
    Ok(())
}

fn mutate_user_action_basis_json(
    harness: &MethodHarness,
    user_action_request_id: &str,
    mutate: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT basis_json
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    let mut value: Value = serde_json::from_str(&text)?;
    mutate(&mut value);
    set_user_action_owner_json(
        harness,
        user_action_request_id,
        "basis_json",
        Some(&value.to_string()),
    )
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
              ORDER BY produced_at_state_version DESC
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
        artifact_input_id: volicord_types::ids::ArtifactInputId::new(artifact_input_id),
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
    let acceptance_criterion_id = volicord_types::ids::AcceptanceCriterionId::new(
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
            continuity_page: None,
            include: StatusInclude {
                task: true,
                pending_user_actions: false,
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
                SET created_at = '1999-12-31T23:59:59.000Z',
                    expires_at = '2000-01-01T00:00:00.000Z'
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

fn set_staged_artifact_window(
    harness: &MethodHarness,
    handle_id: &str,
    created_at: &str,
    expires_at: &str,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE artifact_staging
            SET created_at = ?3,
                expires_at = ?4
          WHERE project_id = ?1
            AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id, created_at, expires_at],
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
