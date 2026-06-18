#![forbid(unsafe_code)]

//! Shared implementation-test helpers.
//!
//! Helpers in this crate should use disposable locations, such as `/tmp`, for
//! future runtime homes and fixture output.

use std::path::{Path, PathBuf};

use harness_store::{
    bootstrap::{
        initialize_runtime_home, register_project, register_surface, ProjectRegistration,
        SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts},
    sqlite::open_project_state_database,
};
use harness_types::{TypeBoundary, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION};
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use tempfile::{Builder, TempDir};

pub mod fixtures {
    /// Placement marker for future shared fixtures.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct FixtureBoundary;
}

pub mod golden {
    /// Placement marker for future golden-output helpers.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct GoldenBoundary;
}

/// Returns a candidate disposable runtime-home path without creating it.
pub fn disposable_runtime_home(name: &str) -> PathBuf {
    std::env::temp_dir().join("harness-test-runtime").join(name)
}

/// Automatically cleaned disposable Runtime Home for implementation tests.
#[derive(Debug)]
pub struct TempRuntimeHome {
    dir: TempDir,
}

impl TempRuntimeHome {
    /// Creates a new empty Runtime Home under the system temporary directory.
    pub fn new(prefix: &str) -> std::io::Result<Self> {
        let dir = Builder::new()
            .prefix(&format!("harness-runtime-{prefix}-"))
            .tempdir()?;
        Ok(Self { dir })
    }

    /// Returns the Runtime Home directory path.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Returns the `registry.sqlite` path under this Runtime Home.
    pub fn registry_db_path(&self) -> PathBuf {
        self.path().join("registry.sqlite")
    }

    /// Returns the project home path under this Runtime Home.
    pub fn project_home_path(&self, project_id: &str) -> PathBuf {
        self.path().join("projects").join(project_id)
    }

    /// Returns the project-local `state.sqlite` path under this Runtime Home.
    pub fn project_state_db_path(&self, project_id: &str) -> PathBuf {
        self.project_home_path(project_id).join("state.sqlite")
    }

    /// Returns the transient artifact staging path under this Runtime Home.
    pub fn artifacts_tmp_path(&self, project_id: &str) -> PathBuf {
        self.project_home_path(project_id)
            .join("artifacts")
            .join("tmp")
    }
}

/// Shared Core-method fixture builders for conformance and integration tests.
pub mod core_fixtures {
    use std::{error::Error, fs, path::Path};

    use harness_store::StoreError;
    use harness_types::{
        AcceptedRiskInput, ActorKind, ArtifactInput, ArtifactInputId, ArtifactInputSourceKind,
        BaselineRef, ChangeUnitId, ChangeUnitOperation, ChangeUnitUpdate, CloseIntent, CloseReason,
        CloseTaskRequest, EvidenceCoverageItem, EvidenceCoverageState, IdempotencyKey,
        InitialScope, IntakeRequest, JsonObject, JudgmentKind, JudgmentPresentation,
        JudgmentRequiredFor, JudgmentResolutionOutcome, ObservedChanges, PrepareWriteRequest,
        ProjectId, RecordId, RecordRunRequest, RecordUserJudgmentPayload,
        RecordUserJudgmentRequest, RedactionState, RequestId, RequestUserJudgmentRequest,
        RequestedMode, ResumePolicy, RunKind, ScopeUpdate, SensitiveActionScope,
        StageArtifactRequest, StagedArtifactHandle, StateRecordKind, StateRecordRef, StatusInclude,
        StatusRequest, SurfaceId, TaskId, ToolEnvelope, UpdateScopeRequest, UserJudgmentId,
        UserJudgmentOption, UserJudgmentOptionId, WriteAuthorizationId,
    };

    use super::*;

    /// Canonical project id used by shared disposable fixtures.
    pub const DEFAULT_PROJECT_ID: &str = "project_fixture";
    /// Canonical surface id used by shared disposable fixtures.
    pub const DEFAULT_SURFACE_ID: &str = "surface_fixture";
    /// Canonical surface instance id used by shared disposable fixtures.
    pub const DEFAULT_SURFACE_INSTANCE_ID: &str = "surface_instance_fixture";
    /// Baseline ref used by shared method request fixtures.
    pub const DEFAULT_BASELINE_REF: &str = "baseline_fixture";
    /// Product path allowed by the default Change Unit fixture.
    pub const DEFAULT_PRODUCT_PATH: &str = "src/export.rs";

    /// Automatically cleaned Harness Runtime Home with one registered project and surface.
    #[derive(Debug)]
    pub struct CoreFixture {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
        project_id: String,
        surface_id: String,
        surface_instance_id: String,
    }

    impl CoreFixture {
        /// Creates a disposable Runtime Home, Product Repository registration, and local surface.
        pub fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let component = identifier_component(prefix);
            let runtime_home = TempRuntimeHome::new(&component)?;
            let repo_root = runtime_home.path().join("repo");
            fs::create_dir_all(&repo_root)?;

            let project_id = DEFAULT_PROJECT_ID.to_owned();
            let surface_id = DEFAULT_SURFACE_ID.to_owned();
            let surface_instance_id = DEFAULT_SURFACE_INSTANCE_ID.to_owned();

            initialize_runtime_home(
                runtime_home.path(),
                &format!("runtime_home_{component}"),
                "{}",
            )?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: project_id.clone(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_surface(
                runtime_home.path(),
                SurfaceRegistration {
                    project_id: project_id.clone(),
                    surface_id: surface_id.clone(),
                    surface_instance_id: surface_instance_id.clone(),
                    surface_kind: "local_test".to_owned(),
                    display_name: Some("Shared Test Surface".to_owned()),
                    capability_profile_json: default_capability_profile().to_string(),
                    local_access_json: json!({
                        "access_class": "core_mutation",
                        "authorized_access_classes": [
                            "read_status",
                            "core_mutation",
                            "write_authorization",
                            "run_recording",
                            "artifact_registration",
                            "artifact_read"
                        ],
                        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
                    })
                    .to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;

            let runtime_home_path = runtime_home.path().to_path_buf();
            Ok(Self {
                _runtime_home: runtime_home,
                runtime_home_path,
                project_id,
                surface_id,
                surface_instance_id,
            })
        }

        /// Returns the disposable Runtime Home path.
        pub fn runtime_home_path(&self) -> &Path {
            &self.runtime_home_path
        }

        /// Returns the disposable Product Repository path for this fixture project.
        pub fn product_repo_path(&self) -> PathBuf {
            self.runtime_home_path.join("repo")
        }

        /// Returns the registered project id.
        pub fn project_id(&self) -> &str {
            &self.project_id
        }

        /// Returns the registered surface id.
        pub fn surface_id(&self) -> &str {
            &self.surface_id
        }

        /// Returns the registered surface instance id.
        pub fn surface_instance_id(&self) -> &str {
            &self.surface_instance_id
        }

        /// Opens the project-local Core store.
        pub fn store(&self) -> Result<CoreProjectStore, StoreError> {
            CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(&self.project_id))
        }

        /// Reads storage-effect counters for this fixture project.
        pub fn counts(&self) -> Result<StorageEffectCounts, StoreError> {
            self.store()?.effect_counts()
        }

        /// Opens the raw project-local SQLite database for focused fixture inspection.
        pub fn conn(&self) -> Result<Connection, StoreError> {
            let path = self
                .runtime_home_path
                .join("projects")
                .join(&self.project_id)
                .join("state.sqlite");
            open_project_state_database(path)
        }

        /// Replaces the registered surface capability profile.
        pub fn set_surface_capability(&self, capability_profile: Value) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE surfaces
                    SET capability_profile_json = ?3
                  WHERE project_id = ?1
                    AND surface_id = ?2",
                rusqlite::params![
                    self.project_id,
                    self.surface_id,
                    capability_profile.to_string()
                ],
            )?;
            Ok(())
        }

        /// Replaces the registered surface local access metadata.
        pub fn set_surface_local_access(&self, local_access: Value) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE surfaces
                    SET local_access_json = ?3
                  WHERE project_id = ?1
                    AND surface_id = ?2",
                rusqlite::params![self.project_id, self.surface_id, local_access.to_string()],
            )?;
            Ok(())
        }

        /// Builds a common public request envelope.
        pub fn envelope(
            &self,
            request_id: &str,
            idempotency_key: Option<&str>,
            dry_run: bool,
            expected_state_version: Option<u64>,
            task_id: Option<&str>,
        ) -> ToolEnvelope {
            ToolEnvelope {
                project_id: ProjectId::new(&self.project_id),
                task_id: task_id.map(TaskId::new).into(),
                actor_kind: ActorKind::Agent,
                surface_id: SurfaceId::new(&self.surface_id),
                request_id: RequestId::new(request_id),
                idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
                expected_state_version: expected_state_version.into(),
                dry_run,
                locale: Some("en-US".to_owned()).into(),
            }
        }

        /// Builds a default `harness.status` request.
        pub fn status_request(&self, request_id: &str, task_id: Option<&str>) -> StatusRequest {
            StatusRequest {
                envelope: self.envelope(request_id, None, false, None, task_id),
                include: status_include_all(),
            }
        }

        /// Builds a default `harness.intake` request.
        pub fn intake_request(
            &self,
            request_id: &str,
            idempotency_key: &str,
            dry_run: bool,
            expected_state_version: Option<u64>,
        ) -> IntakeRequest {
            IntakeRequest {
                envelope: self.envelope(
                    request_id,
                    Some(idempotency_key),
                    dry_run,
                    expected_state_version,
                    None,
                ),
                plain_language_request: "Create a test export flow.".to_owned(),
                requested_mode: RequestedMode::Work,
                resume_policy: ResumePolicy::CreateNew,
                initial_scope: InitialScope {
                    boundary: "Initial test scope.".to_owned(),
                    non_goals: vec!["Changing unrelated flows.".to_owned()],
                    acceptance_criteria: vec!["The test export flow is represented.".to_owned()],
                },
                initial_context_refs: Vec::new(),
            }
        }

        /// Builds a default `harness.update_scope` request.
        pub fn update_scope_request(&self, input: UpdateScopeFixture<'_>) -> UpdateScopeRequest {
            let mut fields = Map::new();
            fields.insert(
                "scope_summary".to_owned(),
                Value::String(input.scope_summary.to_owned()),
            );
            fields.insert(
                "affected_paths".to_owned(),
                json!([DEFAULT_PRODUCT_PATH, "tests/export.rs"]),
            );
            UpdateScopeRequest {
                envelope: self.envelope(
                    input.request_id,
                    Some(input.idempotency_key),
                    input.dry_run,
                    input.expected_state_version,
                    Some(input.task_id),
                ),
                task_id: TaskId::new(input.task_id),
                goal_summary: Some(input.scope_summary.to_owned()).into(),
                scope_update: Some(ScopeUpdate {
                    include: vec![input.scope_summary.to_owned()],
                    exclude: vec!["Unrelated behavior.".to_owned()],
                })
                .into(),
                scope_boundary: Some(input.scope_summary.to_owned()).into(),
                non_goals: Some(vec!["Unrelated behavior.".to_owned()]).into(),
                acceptance_criteria: Some(vec!["The scoped behavior is represented.".to_owned()])
                    .into(),
                autonomy_boundary: Some("Stay inside the scoped test behavior.".to_owned()).into(),
                baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
                change_unit: ChangeUnitUpdate {
                    operation: input.operation,
                    fields,
                },
                related_scope_decision_refs: Vec::new(),
            }
        }

        /// Builds a default `harness.prepare_write` request.
        pub fn prepare_write_request(
            &self,
            request_id: &str,
            idempotency_key: &str,
            expected_state_version: Option<u64>,
            task_id: Option<&str>,
            change_unit_id: Option<&str>,
        ) -> PrepareWriteRequest {
            PrepareWriteRequest {
                envelope: self.envelope(
                    request_id,
                    Some(idempotency_key),
                    false,
                    expected_state_version,
                    task_id,
                ),
                task_id: task_id.map(TaskId::new).into(),
                change_unit_id: change_unit_id.map(ChangeUnitId::new).into(),
                intended_operation: "local_product_file_update".to_owned(),
                intended_paths: vec![DEFAULT_PRODUCT_PATH.to_owned()],
                product_file_write_intended: true,
                sensitive_categories: Vec::new(),
                baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
            }
        }

        /// Builds a default `harness.stage_artifact` request.
        pub fn stage_artifact_request(
            &self,
            request_id: &str,
            idempotency_key: Option<&str>,
            dry_run: bool,
            expected_state_version: Option<u64>,
            task_id: &str,
        ) -> StageArtifactRequest {
            StageArtifactRequest {
                envelope: self.envelope(
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

        /// Builds a default `harness.record_run` request.
        pub fn record_run_request(
            &self,
            request_id: &str,
            idempotency_key: &str,
            dry_run: bool,
            expected_state_version: Option<u64>,
            task_id: &str,
            change_unit_id: &str,
        ) -> RecordRunRequest {
            RecordRunRequest {
                envelope: self.envelope(
                    request_id,
                    Some(idempotency_key),
                    dry_run,
                    expected_state_version,
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                change_unit_id: ChangeUnitId::new(change_unit_id),
                kind: RunKind::Implementation,
                run_id: None.into(),
                baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
                write_authorization_id: None.into(),
                summary: "Recorded implementation run.".to_owned(),
                observed_changes: ObservedChanges {
                    changed_paths: Vec::new(),
                    product_file_write_observed: false,
                    sensitive_categories: Vec::new(),
                    baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
                },
                artifact_inputs: Vec::new(),
                evidence_updates: Vec::new(),
                close_assessment: None.into(),
            }
        }

        /// Builds a default `harness.request_user_judgment` request.
        pub fn user_judgment_request(
            &self,
            input: UserJudgmentFixture<'_>,
        ) -> RequestUserJudgmentRequest {
            RequestUserJudgmentRequest {
                envelope: self.envelope(
                    input.request_id,
                    Some(input.idempotency_key),
                    input.dry_run,
                    input.expected_state_version,
                    Some(input.task_id),
                ),
                task_id: TaskId::new(input.task_id),
                change_unit_id: input.change_unit_id.map(ChangeUnitId::new).into(),
                judgment_kind: input.judgment_kind,
                presentation: JudgmentPresentation::Short,
                question: "Choose the focused test judgment outcome.".to_owned(),
                options: vec![
                    UserJudgmentOption {
                        option_id: UserJudgmentOptionId::new("accept"),
                        label: "Accept".to_owned(),
                        description: "Record the focused user-owned judgment.".to_owned(),
                        consequence: "Only this judgment record is resolved.".to_owned(),
                        resolution_outcome: Some(JudgmentResolutionOutcome::Accepted),
                        is_default: true,
                    },
                    UserJudgmentOption {
                        option_id: UserJudgmentOptionId::new("decline"),
                        label: "Decline".to_owned(),
                        description: "Record that the focused judgment was not accepted."
                            .to_owned(),
                        consequence: "The Task remains unresolved for this question.".to_owned(),
                        resolution_outcome: Some(JudgmentResolutionOutcome::Rejected),
                        is_default: false,
                    },
                ],
                context: harness_types::UserJudgmentContext {
                    summary: "A focused test judgment needs a user-owned answer.".to_owned(),
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                    visible_risks: Vec::new(),
                    constraints: vec![
                        "The answer covers only the requested judgment kind.".to_owned()
                    ],
                },
                affected_refs: vec![self.task_ref(input.task_id, input.expected_state_version)],
                sensitive_action_scope: sensitive_action_scope_for_kind(input.judgment_kind).into(),
                required_for: JudgmentRequiredFor::Close,
                expires_at: None.into(),
            }
        }

        /// Builds a default `harness.record_user_judgment` request.
        pub fn record_judgment_request(
            &self,
            input: RecordJudgmentFixture<'_>,
        ) -> RecordUserJudgmentRequest {
            let mut envelope = self.envelope(
                input.request_id,
                Some(input.idempotency_key),
                false,
                input.expected_state_version,
                Some(input.task_id),
            );
            envelope.actor_kind = ActorKind::User;
            RecordUserJudgmentRequest {
                envelope,
                user_judgment_id: UserJudgmentId::new(input.user_judgment_id),
                judgment_kind: input.judgment_kind,
                selected_option_id: UserJudgmentOptionId::new("accept"),
                answer: input.answer,
                note: Some("Recorded by a focused conformance fixture.".to_owned()).into(),
                accepted_risks: Vec::new(),
            }
        }

        /// Builds a default `harness.close_task` request.
        pub fn close_task_request(&self, input: CloseTaskFixture<'_>) -> CloseTaskRequest {
            CloseTaskRequest {
                envelope: self.envelope(
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
                user_note: Some("Focused close-task fixture.".to_owned()).into(),
            }
        }

        /// Builds a `StateRecordRef` for a fixture Task.
        pub fn task_ref(&self, task_id: &str, state_version: Option<u64>) -> StateRecordRef {
            StateRecordRef {
                record_kind: StateRecordKind::Task,
                record_id: RecordId::new(task_id),
                project_id: ProjectId::new(&self.project_id),
                task_id: Some(TaskId::new(task_id)).into(),
                state_version: state_version.into(),
            }
        }

        /// Reads the current status of a Write Authorization row.
        pub fn write_authorization_status(
            &self,
            write_authorization_id: &str,
        ) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT status
                   FROM write_authorizations
                  WHERE project_id = ?1
                    AND write_authorization_id = ?2",
                rusqlite::params![self.project_id, write_authorization_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the basis state version of a Write Authorization row.
        pub fn write_authorization_basis(
            &self,
            write_authorization_id: &str,
        ) -> Result<u64, Box<dyn Error>> {
            let basis: i64 = self.conn()?.query_row(
                "SELECT basis_state_version
                   FROM write_authorizations
                  WHERE project_id = ?1
                    AND write_authorization_id = ?2",
                rusqlite::params![self.project_id, write_authorization_id],
                |row| row.get(0),
            )?;
            Ok(u64::try_from(basis)?)
        }

        /// Reads the current status of a user-owned judgment row.
        pub fn user_judgment_status(&self, judgment_id: &str) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT status
                   FROM user_judgments
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the current compatibility status for a user-owned judgment basis.
        pub fn user_judgment_basis_status(&self, judgment_id: &str) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT basis_status
                   FROM user_judgments
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the resolved answer JSON for a user-owned judgment row.
        pub fn user_judgment_resolution(&self, judgment_id: &str) -> Result<Value, Box<dyn Error>> {
            let text: String = self.conn()?.query_row(
                "SELECT resolution_json
                   FROM user_judgments
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id],
                |row| row.get(0),
            )?;
            Ok(serde_json::from_str(&text)?)
        }

        /// Reads the currently applied Change Unit id for a Task.
        pub fn current_change_unit_id(&self, task_id: &str) -> Result<Option<String>, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT current_change_unit_id
                   FROM tasks
                  WHERE project_id = ?1
                    AND task_id = ?2",
                rusqlite::params![self.project_id, task_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the current Change Unit scope summary for a Task.
        pub fn current_change_unit_scope(&self, task_id: &str) -> Result<String, Box<dyn Error>> {
            let text: String = self.conn()?.query_row(
                "SELECT scope_summary_json
                   FROM change_units
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND status = 'active'
                    AND is_current = 1",
                rusqlite::params![self.project_id, task_id],
                |row| row.get(0),
            )?;
            let value: Value = serde_json::from_str(&text)?;
            Ok(value["scope_summary"]
                .as_str()
                .expect("scope_summary should be a string")
                .to_owned())
        }

        /// Reads the status of a staged artifact handle.
        pub fn artifact_staging_status(&self, handle_id: &str) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT status
                   FROM artifact_staging
                  WHERE project_id = ?1
                    AND handle_id = ?2",
                rusqlite::params![self.project_id, handle_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the latest evidence summary id for a Task.
        pub fn latest_evidence_summary_id(&self, task_id: &str) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT evidence_summary_id
                   FROM evidence_summaries
                  WHERE project_id = ?1
                    AND task_id = ?2
                  ORDER BY updated_at DESC, evidence_summary_id DESC
                  LIMIT 1",
                rusqlite::params![self.project_id, task_id],
                |row| row.get(0),
            )?)
        }

        /// Returns whether an artifact has an owner link of the requested kind.
        pub fn artifact_owner_link_exists(
            &self,
            artifact_id: &str,
            owner_record_kind: &str,
        ) -> Result<bool, StoreError> {
            let count: i64 = self.conn()?.query_row(
                "SELECT COUNT(*)
                   FROM artifact_links
                  WHERE project_id = ?1
                    AND artifact_id = ?2
                    AND owner_record_kind = ?3",
                rusqlite::params![self.project_id, artifact_id, owner_record_kind],
                |row| row.get(0),
            )?;
            Ok(count > 0)
        }

        /// Reads the active Task id from `project_state`.
        pub fn active_task_id(&self) -> Result<Option<String>, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT active_task_id
                   FROM project_state
                  WHERE project_id = ?1",
                rusqlite::params![self.project_id],
                |row| row.get(0),
            )?)
        }

        /// Inserts a compatible replacement Task for supersede tests.
        pub fn insert_superseding_task(&self, task_id: &str) -> Result<(), StoreError> {
            self.conn()?.execute(
                "INSERT INTO tasks (
                    project_id,
                    task_id,
                    created_by_surface_id,
                    created_by_surface_instance_id,
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
                    ?4,
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
                rusqlite::params![
                    self.project_id,
                    task_id,
                    self.surface_id,
                    self.surface_instance_id
                ],
            )?;
            Ok(())
        }

        /// Replaces a Task owner JSON column with raw text for controlled corruption fixtures.
        pub fn set_task_owner_json_raw(
            &self,
            task_id: &str,
            logical_column: TaskOwnerJsonColumn,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            let column = logical_column.as_str();
            let sql = format!(
                "UPDATE tasks
                    SET {column} = ?3
                  WHERE project_id = ?1
                    AND task_id = ?2"
            );
            self.conn()?
                .execute(&sql, rusqlite::params![self.project_id, task_id, raw_json])?;
            Ok(())
        }

        /// Replaces a Change Unit owner JSON column with raw text for controlled corruption fixtures.
        pub fn set_change_unit_owner_json_raw(
            &self,
            change_unit_id: &str,
            logical_column: ChangeUnitOwnerJsonColumn,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            let column = logical_column.as_str();
            let sql = format!(
                "UPDATE change_units
                    SET {column} = ?3
                  WHERE project_id = ?1
                    AND change_unit_id = ?2"
            );
            self.conn()?.execute(
                &sql,
                rusqlite::params![self.project_id, change_unit_id, raw_json],
            )?;
            Ok(())
        }

        /// Updates a persistent artifact availability status.
        pub fn set_artifact_status(
            &self,
            artifact_id: &str,
            status: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE artifacts
                    SET status = ?3
                  WHERE project_id = ?1
                    AND artifact_id = ?2",
                rusqlite::params![self.project_id, artifact_id, status],
            )?;
            Ok(())
        }

        /// Replaces a user-owned judgment resolution JSON value with SQL NULL or raw text.
        pub fn set_user_judgment_resolution_raw(
            &self,
            judgment_id: &str,
            raw_json: Option<&str>,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_judgments
                    SET resolution_json = ?3,
                        status = 'resolved',
                        resolved_at = 't_corrupt_fixture'
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id, raw_json],
            )?;
            Ok(())
        }

        /// Replaces a user-owned judgment request JSON value with raw text.
        pub fn set_user_judgment_request_raw(
            &self,
            judgment_id: &str,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_judgments
                    SET request_json = ?3
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id, raw_json],
            )?;
            Ok(())
        }

        /// Replaces a user-owned judgment basis JSON value with raw text.
        pub fn set_user_judgment_basis_raw(
            &self,
            judgment_id: &str,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_judgments
                    SET basis_json = ?3
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id, raw_json],
            )?;
            Ok(())
        }

        /// Converts an existing judgment to a legacy-unbound audit row.
        pub fn set_user_judgment_legacy_unbound(
            &self,
            judgment_id: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_judgments
                    SET basis_json = NULL,
                        basis_status = 'legacy_unbound'
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                rusqlite::params![self.project_id, judgment_id],
            )?;
            Ok(())
        }

        /// Replaces a Write Authorization attempt-scope JSON value with raw text.
        pub fn set_write_authorization_attempt_scope_raw(
            &self,
            write_authorization_id: &str,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE write_authorizations
                    SET attempt_scope_json = ?3
                  WHERE project_id = ?1
                    AND write_authorization_id = ?2",
                rusqlite::params![self.project_id, write_authorization_id, raw_json],
            )?;
            Ok(())
        }

        /// Replaces artifact owner JSON with raw text for controlled corruption fixtures.
        pub fn set_artifact_owner_json_raw(
            &self,
            artifact_id: &str,
            logical_column: ArtifactOwnerJsonColumn,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            let column = logical_column.as_str();
            let sql = format!(
                "UPDATE artifacts
                    SET {column} = ?3
                  WHERE project_id = ?1
                    AND artifact_id = ?2"
            );
            self.conn()?.execute(
                &sql,
                rusqlite::params![self.project_id, artifact_id, raw_json],
            )?;
            Ok(())
        }

        /// Replaces an artifact source staging handle for provenance corruption fixtures.
        pub fn set_artifact_source_staging_handle_raw(
            &self,
            artifact_id: &str,
            source_staging_handle_id: Option<&str>,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE artifacts
                    SET source_staging_handle_id = ?3
                  WHERE project_id = ?1
                    AND artifact_id = ?2",
                rusqlite::params![self.project_id, artifact_id, source_staging_handle_id],
            )?;
            Ok(())
        }

        /// Replaces evidence-summary owner JSON with raw text for corruption fixtures.
        pub fn set_evidence_summary_owner_json_raw(
            &self,
            evidence_summary_id: &str,
            logical_column: EvidenceSummaryOwnerJsonColumn,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            let column = logical_column.as_str();
            let sql = format!(
                "UPDATE evidence_summaries
                    SET {column} = ?3
                  WHERE project_id = ?1
                    AND evidence_summary_id = ?2"
            );
            self.conn()?.execute(
                &sql,
                rusqlite::params![self.project_id, evidence_summary_id, raw_json],
            )?;
            Ok(())
        }

        /// Replaces a staged artifact expiration timestamp for timestamp fixtures.
        pub fn set_staged_artifact_expires_at(
            &self,
            handle_id: &str,
            expires_at: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE artifact_staging
                    SET expires_at = ?3
                  WHERE project_id = ?1
                    AND handle_id = ?2",
                rusqlite::params![self.project_id, handle_id, expires_at],
            )?;
            Ok(())
        }

        /// Replaces Write Authorization authority timestamps for fixed-clock tests.
        pub fn set_write_authorization_timestamps(
            &self,
            write_authorization_id: &str,
            created_at: &str,
            expires_at: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE write_authorizations
                    SET created_at = ?3,
                        expires_at = ?4
                  WHERE project_id = ?1
                    AND write_authorization_id = ?2",
                rusqlite::params![
                    self.project_id,
                    write_authorization_id,
                    created_at,
                    expires_at
                ],
            )?;
            Ok(())
        }

        /// Reads Write Authorization `created_at` and `expires_at` timestamp strings.
        pub fn write_authorization_timestamps(
            &self,
            write_authorization_id: &str,
        ) -> Result<(String, String), StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT created_at, expires_at
                   FROM write_authorizations
                  WHERE project_id = ?1
                    AND write_authorization_id = ?2",
                rusqlite::params![self.project_id, write_authorization_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        }

        /// Reads terminal lifecycle fields for a Task.
        pub fn task_terminal_fields(
            &self,
            task_id: &str,
        ) -> Result<TaskTerminalFields, Box<dyn Error>> {
            let (lifecycle_phase, result, close_summary_text, closed_at): (
                String,
                Option<String>,
                String,
                Option<String>,
            ) = self.conn()?.query_row(
                "SELECT lifecycle_phase, result, close_summary_json, closed_at
                   FROM tasks
                  WHERE project_id = ?1
                    AND task_id = ?2",
                rusqlite::params![self.project_id, task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            Ok(TaskTerminalFields {
                lifecycle_phase,
                result,
                close_summary: serde_json::from_str(&close_summary_text)?,
                closed_at,
            })
        }

        /// Reads the most recently appended task event for this fixture project.
        pub fn latest_task_event(&self) -> Result<TaskEventFixtureRow, Box<dyn Error>> {
            let (event_kind, event_payload_text, state_version): (String, String, i64) =
                self.conn()?.query_row(
                    "SELECT event_kind, event_payload_json, state_version
                       FROM task_events
                      WHERE project_id = ?1
                      ORDER BY event_seq DESC
                      LIMIT 1",
                    rusqlite::params![self.project_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
            Ok(TaskEventFixtureRow {
                event_kind,
                event_payload: serde_json::from_str(&event_payload_text)?,
                state_version: u64::try_from(state_version)?,
            })
        }
    }

    /// Task owner JSON columns intentionally exposed for corruption fixtures.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TaskOwnerJsonColumn {
        ShapingSummary,
        BoundedContext,
        AutonomyBoundary,
        CurrentCloseBasis,
        CloseSummary,
        CompletionPolicy,
    }

    impl TaskOwnerJsonColumn {
        fn as_str(self) -> &'static str {
            match self {
                Self::ShapingSummary => "shaping_summary_json",
                Self::BoundedContext => "bounded_context_json",
                Self::AutonomyBoundary => "autonomy_boundary_json",
                Self::CurrentCloseBasis => "close_basis_json",
                Self::CloseSummary => "close_summary_json",
                Self::CompletionPolicy => "completion_policy_json",
            }
        }
    }

    /// Change Unit owner JSON columns intentionally exposed for corruption fixtures.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ChangeUnitOwnerJsonColumn {
        ScopeSummary,
        BoundedPaths,
        WriteBasis,
        CloseBasis,
        Lifecycle,
    }

    impl ChangeUnitOwnerJsonColumn {
        fn as_str(self) -> &'static str {
            match self {
                Self::ScopeSummary => "scope_summary_json",
                Self::BoundedPaths => "bounded_paths_json",
                Self::WriteBasis => "write_basis_json",
                Self::CloseBasis => "close_basis_json",
                Self::Lifecycle => "lifecycle_json",
            }
        }
    }

    /// Artifact owner JSON columns intentionally exposed for corruption fixtures.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ArtifactOwnerJsonColumn {
        Producer,
        Metadata,
    }

    impl ArtifactOwnerJsonColumn {
        fn as_str(self) -> &'static str {
            match self {
                Self::Producer => "producer_json",
                Self::Metadata => "metadata_json",
            }
        }
    }

    /// Evidence-summary JSON columns intentionally exposed for corruption fixtures.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EvidenceSummaryOwnerJsonColumn {
        Coverage,
        SupportingRefs,
        Metadata,
    }

    impl EvidenceSummaryOwnerJsonColumn {
        fn as_str(self) -> &'static str {
            match self {
                Self::Coverage => "coverage_json",
                Self::SupportingRefs => "supporting_refs_json",
                Self::Metadata => "metadata_json",
            }
        }
    }

    /// Input object for update-scope request builders.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UpdateScopeFixture<'a> {
        pub request_id: &'a str,
        pub idempotency_key: &'a str,
        pub dry_run: bool,
        pub expected_state_version: Option<u64>,
        pub task_id: &'a str,
        pub operation: ChangeUnitOperation,
        pub scope_summary: &'a str,
    }

    /// Input object for request-user-judgment request builders.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UserJudgmentFixture<'a> {
        pub request_id: &'a str,
        pub idempotency_key: &'a str,
        pub dry_run: bool,
        pub expected_state_version: Option<u64>,
        pub task_id: &'a str,
        pub change_unit_id: Option<&'a str>,
        pub judgment_kind: JudgmentKind,
    }

    /// Input object for record-user-judgment request builders.
    #[derive(Debug, Clone, PartialEq)]
    pub struct RecordJudgmentFixture<'a> {
        pub request_id: &'a str,
        pub idempotency_key: &'a str,
        pub expected_state_version: Option<u64>,
        pub task_id: &'a str,
        pub user_judgment_id: &'a str,
        pub judgment_kind: JudgmentKind,
        pub answer: RecordUserJudgmentPayload,
    }

    /// Input object for close-task request builders.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CloseTaskFixture<'a> {
        pub request_id: &'a str,
        pub idempotency_key: Option<&'a str>,
        pub dry_run: bool,
        pub expected_state_version: Option<u64>,
        pub task_id: &'a str,
        pub intent: CloseIntent,
        pub close_reason: Option<CloseReason>,
        pub superseding_task_id: Option<&'a str>,
    }

    /// Terminal Task fields read from storage for close-path assertions.
    #[derive(Debug, Clone, PartialEq)]
    pub struct TaskTerminalFields {
        pub lifecycle_phase: String,
        pub result: Option<String>,
        pub close_summary: Value,
        pub closed_at: Option<String>,
    }

    /// Task event fields read from storage for audit assertions.
    #[derive(Debug, Clone, PartialEq)]
    pub struct TaskEventFixtureRow {
        pub event_kind: String,
        pub event_payload: Value,
        pub state_version: u64,
    }

    /// Returns a status include object with every supported flag enabled.
    pub fn status_include_all() -> StatusInclude {
        StatusInclude {
            task: true,
            pending_user_judgments: true,
            write_authority: true,
            evidence: true,
            close: true,
            guarantees: true,
        }
    }

    /// Builds an artifact input for a staged handle.
    pub fn artifact_input_for_handle(
        artifact_input_id: &str,
        handle: StagedArtifactHandle,
        relation_hint: Option<&str>,
        claim: Option<&str>,
    ) -> ArtifactInput {
        ArtifactInput {
            artifact_input_id: ArtifactInputId::new(artifact_input_id),
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

    /// Builds a supported evidence coverage item.
    pub fn supported_evidence_update(claim: &str) -> EvidenceCoverageItem {
        EvidenceCoverageItem {
            claim: claim.to_owned(),
            required_for_close: true,
            coverage_state: EvidenceCoverageState::Supported,
            supporting_refs: Vec::new(),
            supporting_artifact_refs: Vec::new(),
            gap_refs: Vec::new(),
        }
    }

    /// Builds an unsupported evidence coverage item.
    pub fn unsupported_evidence_update(claim: &str) -> EvidenceCoverageItem {
        EvidenceCoverageItem {
            claim: claim.to_owned(),
            required_for_close: true,
            coverage_state: EvidenceCoverageState::Unsupported,
            supporting_refs: Vec::new(),
            supporting_artifact_refs: Vec::new(),
            gap_refs: Vec::new(),
        }
    }

    fn sensitive_action_scope_for_kind(
        judgment_kind: JudgmentKind,
    ) -> Option<SensitiveActionScope> {
        match judgment_kind {
            JudgmentKind::SensitiveApproval => Some(SensitiveActionScope {
                action_kind: "local_sensitive_step".to_owned(),
                description: "Allow the named sensitive step only.".to_owned(),
                intended_paths: vec![DEFAULT_PRODUCT_PATH.to_owned()],
                sensitive_categories: vec!["network".to_owned()],
                command_or_tool_summary: Some("Run a local diagnostic command.".to_owned()).into(),
                network_or_host_summary: Some("No remote host is authorized here.".to_owned())
                    .into(),
                secret_or_credential_summary: None.into(),
                capability_claim: "This is not Write Authorization.".to_owned(),
                expires_at: None.into(),
            }),
            _ => None,
        }
    }

    /// Builds a judgment answer payload with exactly one branch populated.
    pub fn answer_payload(judgment_kind: JudgmentKind) -> RecordUserJudgmentPayload {
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
                payload.sensitive_action_scope =
                    sensitive_action_scope_for_kind(judgment_kind).into();
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

    /// Builds an accepted-risk input for close-readiness fixtures.
    pub fn accepted_risk(summary: &str) -> AcceptedRiskInput {
        AcceptedRiskInput {
            risk_id: harness_types::RiskId::new("risk_visible_001"),
            summary: summary.to_owned(),
            consequence: "The named residual risk remains after close.".to_owned(),
            related_refs: Vec::new(),
            accepted_for_close: true,
        }
    }

    /// Builds a `WriteAuthorizationId` for tests that need the typed wrapper.
    pub fn write_authorization_id(value: &str) -> WriteAuthorizationId {
        WriteAuthorizationId::new(value)
    }

    fn default_capability_profile() -> Value {
        json!({
            "supported_access_classes": [
                "read_status",
                "core_mutation",
                "write_authorization",
                "run_recording",
                "artifact_registration"
            ],
            "write_authorization": true,
            "manual_artifact_attachment_supported": true
        })
    }

    fn identifier_component(value: &str) -> String {
        let component = value
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .to_owned();
        if component.is_empty() {
            "fixture".to_owned()
        } else {
            component
        }
    }

    fn json_object(value: Value) -> JsonObject {
        match value {
            Value::Object(object) => object,
            _ => panic!("fixture helper expected a JSON object"),
        }
    }
}

/// Identifies the shared type boundary used by test helpers.
pub const fn shared_type_boundary() -> TypeBoundary {
    TypeBoundary::Domain
}

#[cfg(test)]
mod tests {
    use super::{disposable_runtime_home, shared_type_boundary, TempRuntimeHome};
    use harness_types::TypeBoundary;

    #[test]
    fn disposable_runtime_home_stays_under_system_temp() {
        let path = disposable_runtime_home("workspace-skeleton");
        assert!(path.is_absolute());
        assert!(path.ends_with("harness-test-runtime/workspace-skeleton"));
    }

    #[test]
    fn test_support_uses_domain_type_boundary() {
        assert_eq!(shared_type_boundary(), TypeBoundary::Domain);
    }

    #[test]
    fn temp_runtime_home_uses_disposable_directory() {
        let runtime_home = TempRuntimeHome::new("helpers").expect("tempdir should be created");
        assert!(runtime_home.path().is_absolute());
        assert!(runtime_home.path().exists());
        assert!(runtime_home.registry_db_path().ends_with("registry.sqlite"));
        assert!(runtime_home
            .project_state_db_path("PRJ-helpers")
            .ends_with("projects/PRJ-helpers/state.sqlite"));
        assert!(runtime_home
            .artifacts_tmp_path("PRJ-helpers")
            .ends_with("projects/PRJ-helpers/artifacts/tmp"));
    }
}
