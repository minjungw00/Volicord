#![forbid(unsafe_code)]

//! Shared implementation-test helpers.
//!
//! Helpers in this crate should use disposable locations, such as `/tmp`, for
//! future runtime homes and fixture output.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use tempfile::{Builder, TempDir};
use volicord_store::{
    agent_connections::{
        add_connection_project, agent_connection_record_read_only, ensure_agent_connection,
        AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_MODE_WORKFLOW,
        HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
    },
    bootstrap::{
        initialize_runtime_home, register_project, write_installation_profile,
        InstallationProfileRegistration, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts},
    guards::{
        agent_session_matches_current_integration, guard_health_record, upsert_agent_session,
        AgentSessionUpsert,
    },
    operational_sessions::{
        connection_integration_revision, start_mcp_runtime_session, McpRuntimeSessionStart,
    },
    sqlite::open_project_state_database,
    StoreResult,
};
use volicord_types::{
    managed_stdio_session_id, AgentConnectionId, AgentRuntimeSessionId, AgentSessionId,
    GuardCommand, GuardCommandSet, GuardHookPhase, GuardInstallationId, GuardManifest, HostKind,
    IntegrationProfile, ManagedFileExpectation, McpRuntimeSessionSource, PolicyHash, ProjectId,
    TypeBoundary, GUARD_MANIFEST_SCHEMA,
};

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

/// Non-product invocation label for local-user and negative-path test fixtures.
pub const TEST_FIXTURE_INVOCATION_BINDING_BASIS: &str = "test_fixture_binding";

static TEST_AGENT_SESSION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Coordinates created through the same authoritative Store APIs as a managed MCP session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestAgentSessionFixture {
    pub runtime_session_id: AgentRuntimeSessionId,
    pub project_session_id: AgentSessionId,
    pub host_session_id: String,
    pub host_thread_id: String,
    pub host_turn_id: String,
}

/// Builds a strict canonical Guard manifest for shared implementation fixtures.
pub fn test_guard_manifest_json(
    runtime_home: impl AsRef<Path>,
    repo_root: &Path,
    project_id: &str,
    connection_id: &str,
    guard_installation_id: &str,
    policy_hash: &str,
) -> String {
    let runtime_home = runtime_home.as_ref();
    let connection = agent_connection_record_read_only(runtime_home, connection_id)
        .expect("fixture connection lookup")
        .expect("fixture connection");
    let integration_revision =
        connection_integration_revision(&connection).expect("fixture integration revision");
    let command_path = runtime_home.join("bin/volicord").display().to_string();
    let command = |phase: GuardHookPhase| GuardCommand {
        command: command_path.clone(),
        args: vec![
            "_hook".to_owned(),
            phase.command_name().to_owned(),
            "--repo".to_owned(),
            repo_root.display().to_string(),
            "--connection".to_owned(),
            connection_id.to_owned(),
            "--guard-installation".to_owned(),
            guard_installation_id.to_owned(),
            "--host".to_owned(),
            "codex".to_owned(),
            "--integration-profile".to_owned(),
            "record".to_owned(),
            "--policy-hash".to_owned(),
            policy_hash.to_owned(),
            "--host-output".to_owned(),
            "codex".to_owned(),
        ],
    };
    let hash = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let managed = |kind: &str, path: PathBuf, ownership: &str| ManagedFileExpectation {
        kind: kind.to_owned(),
        path: path.display().to_string(),
        content_hash: hash.to_owned(),
        ownership: ownership.to_owned(),
        managed_marker_start: (ownership == "managed_block").then(|| "VOLICORD_START".to_owned()),
        managed_marker_end: (ownership == "managed_block").then(|| "VOLICORD_END".to_owned()),
        managed_marker: None,
        executable_required: None,
        managed_script_role: None,
        managed_script_command: None,
        host_kind: None,
        phase: None,
        purpose: None,
        connection_id: None,
        guard_installation_id: None,
        policy_hash: None,
        host_output: None,
    };
    let wrapper = |phase: GuardHookPhase| ManagedFileExpectation {
        kind: "host_hook_wrapper".to_owned(),
        path: repo_root
            .join(format!(".codex/hooks/volicord-{}.sh", phase.command_name()))
            .display()
            .to_string(),
        content_hash: hash.to_owned(),
        ownership: "managed_script".to_owned(),
        managed_marker_start: None,
        managed_marker_end: None,
        managed_marker: Some("VOLICORD_MANAGED_HOOK_WRAPPER".to_owned()),
        executable_required: Some(true),
        managed_script_role: None,
        managed_script_command: Some("exec volicord".to_owned()),
        host_kind: Some("codex".to_owned()),
        phase: Some(phase.as_str().to_owned()),
        purpose: Some("guard".to_owned()),
        connection_id: Some(connection_id.to_owned()),
        guard_installation_id: Some(guard_installation_id.to_owned()),
        policy_hash: Some(policy_hash.to_owned()),
        host_output: Some("codex".to_owned()),
    };
    let mut managed_files = vec![
        managed(
            "agents_managed_block",
            repo_root.join("AGENTS.md"),
            "managed_block",
        ),
        managed(
            "volicord_policy",
            repo_root.join(".volicord/policy.json"),
            "managed_json",
        ),
        managed(
            "host_hook_config",
            repo_root.join(".codex/hooks.json"),
            "managed_json",
        ),
        ManagedFileExpectation {
            kind: "host_hook_dispatch".to_owned(),
            path: repo_root
                .join(".codex/hooks/volicord-dispatch.sh")
                .display()
                .to_string(),
            content_hash: hash.to_owned(),
            ownership: "managed_script".to_owned(),
            managed_marker_start: None,
            managed_marker_end: None,
            managed_marker: Some("VOLICORD_MANAGED_HOOK_WRAPPER".to_owned()),
            executable_required: Some(true),
            managed_script_role: Some("codex_dispatch".to_owned()),
            managed_script_command: None,
            host_kind: Some("codex".to_owned()),
            phase: Some("dispatch".to_owned()),
            purpose: None,
            connection_id: None,
            guard_installation_id: None,
            policy_hash: None,
            host_output: None,
        },
        managed(
            "host_rule_instruction",
            repo_root.join(".codex/rules/volicord.rules"),
            "managed_block",
        ),
    ];
    managed_files.extend(GuardHookPhase::REQUIRED.into_iter().map(wrapper));
    let manifest = GuardManifest {
        schema: GUARD_MANIFEST_SCHEMA.to_owned(),
        guard_installation_id: GuardInstallationId::new(guard_installation_id),
        connection_id: AgentConnectionId::new(connection_id),
        project_id: ProjectId::new(project_id),
        host_kind: HostKind::Codex,
        integration_profile: IntegrationProfile::Record,
        policy_hash: PolicyHash::parse(policy_hash).expect("fixture policy hash"),
        integration_revision,
        runtime_commands: GuardCommandSet {
            pre_tool: command(GuardHookPhase::PreTool),
            post_tool: command(GuardHookPhase::PostTool),
            prompt_capture: command(GuardHookPhase::PromptCapture),
        },
        managed_files,
        required_hook_phases: GuardHookPhase::REQUIRED.to_vec(),
    };
    serde_json::to_string(&manifest).expect("canonical fixture Guard manifest")
}

/// Seeds one real managed runtime/project session for adapter and Core tests.
pub fn seed_test_agent_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_id: &str,
    guard_installation_id: Option<&str>,
) -> StoreResult<TestAgentSessionFixture> {
    let health = guard_health_record(runtime_home.as_ref(), project_id, connection_id)?;
    if let Some(existing) = health.latest_session.as_ref() {
        if agent_session_matches_current_integration(
            runtime_home.as_ref(),
            existing,
            guard_installation_id,
        )? {
            return Ok(TestAgentSessionFixture {
                runtime_session_id: AgentRuntimeSessionId::new(
                    existing
                        .runtime_session_id
                        .as_deref()
                        .expect("current test Agent Session must be runtime-bound"),
                ),
                project_session_id: AgentSessionId::new(&existing.session_id),
                host_session_id: existing.host_session_id.clone(),
                host_thread_id: existing.host_thread_id.clone(),
                host_turn_id: existing.last_host_turn_id.clone(),
            });
        }
    }
    let sequence = TEST_AGENT_SESSION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let host_session_id = format!("test-session-{sequence}");
    let host_thread_id = format!("test-thread-{sequence}");
    let host_turn_id = format!("test-turn-{sequence}");
    let project_session_id = managed_stdio_session_id(connection_id, &host_session_id)
        .expect("generated test host session identity must be valid");
    let store = CoreProjectStore::open(runtime_home.as_ref(), &project_id.into())?;
    let observed_at = store.current_timestamp()?;
    let runtime_session_id = start_mcp_runtime_session(
        runtime_home.as_ref(),
        McpRuntimeSessionStart {
            connection_internal_id: connection_id.to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: Some("future-host-version-9999.0".to_owned()),
            process_id: std::process::id(),
            process_started_at: observed_at.clone(),
        },
    )?
    .runtime_session_id;
    upsert_agent_session(
        runtime_home,
        project_id,
        AgentSessionUpsert {
            session_id: project_session_id.clone(),
            runtime_session_id: Some(runtime_session_id.clone()),
            connection_internal_id: connection_id.to_owned(),
            guard_installation_id: guard_installation_id.map(str::to_owned),
            host_session_id: host_session_id.clone(),
            host_thread_id: host_thread_id.clone(),
            host_turn_id: host_turn_id.clone(),
            observed_at,
        },
    )?;
    Ok(TestAgentSessionFixture {
        runtime_session_id: AgentRuntimeSessionId::new(runtime_session_id),
        project_session_id: AgentSessionId::new(project_session_id),
        host_session_id,
        host_thread_id,
        host_turn_id,
    })
}

/// Returns a candidate disposable runtime-home path without creating it.
pub fn disposable_runtime_home(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join("volicord-test-runtime")
        .join(name)
}

/// Automatically cleaned disposable Runtime Home for implementation tests.
#[derive(Debug)]
pub struct TempRuntimeHome {
    dir: TempDir,
    runtime_home_path: PathBuf,
}

impl TempRuntimeHome {
    /// Creates a new empty Runtime Home under the system temporary directory.
    pub fn new(prefix: &str) -> std::io::Result<Self> {
        let dir = Builder::new()
            .prefix(&format!("volicord-runtime-{prefix}-"))
            .tempdir()?;
        let runtime_home_path = dir.path().join("runtime-home");
        fs::create_dir_all(&runtime_home_path)?;
        Ok(Self {
            dir,
            runtime_home_path,
        })
    }

    /// Returns the Runtime Home directory path.
    pub fn path(&self) -> &Path {
        &self.runtime_home_path
    }

    /// Returns a sibling Product Repository path inside this disposable fixture root.
    pub fn product_repo_path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.dir
            .path()
            .join("product-repositories")
            .join(name.as_ref())
    }

    /// Creates and returns a sibling Product Repository directory.
    pub fn create_product_repo(&self, name: impl AsRef<Path>) -> std::io::Result<PathBuf> {
        let path = self.product_repo_path(name);
        fs::create_dir_all(&path)?;
        Ok(path)
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

    use volicord_store::StoreError;
    use volicord_types::{
        AcceptanceCriterionInput, AcceptanceCriterionReplacement, AcceptedRiskInput, ArtifactId,
        ArtifactInput, ArtifactInputId, ArtifactInputSourceKind, BaselineRef, ChangeUnitId,
        ChangeUnitOperation, ChangeUnitUpdate, CheckCloseRequest, CloseIntent, CloseMutationIntent,
        CloseReason, CloseTaskRequest, EvidenceAssuranceLevel, EvidenceClaimId,
        EvidenceCoverageUpdate, EvidenceCoverageUpdateState, EvidenceRelevanceStatus,
        EvidenceRequirement, EvidenceSourceKind, EvidenceTarget, EvidenceUpdateProvenance,
        IdempotencyKey, InitialScope, IntakeRequest, JsonObject, JudgmentKind,
        JudgmentPresentation, ObservedChanges, PrepareWriteRequest, ProjectId, RecordId,
        RecordRunRequest, RedactionState, RequestId, RequestUserActionRequest,
        RequestedControlLevel, RequestedMode, RequiredNullable, ResolveUserActionRequest,
        ResumePolicy, RunKind, ScopeUpdate, SensitiveActionScope, StageArtifactRequest,
        StagedArtifactHandle, StateRecordKind, StateRecordRef, StatusInclude, StatusRequest,
        TaskId, ToolEnvelope, UpdateScopeRequest, UserActionChoiceDraft, UserActionContext,
        UserActionDraft, UserActionEvidenceObservationDraft, UserActionOptionId,
        UserActionOptionInput, UserActionRequestId, UserActionRequiredFor,
        UserActionResolutionInput, WriteTicketId,
    };

    use super::*;

    /// Canonical project id used by shared disposable fixtures.
    pub const DEFAULT_PROJECT_ID: &str = "project_fixture";
    /// Canonical Agent Connection id used by shared disposable fixtures.
    pub const DEFAULT_CONNECTION_ID: &str = "connection_fixture";
    /// Baseline ref used by shared method request fixtures.
    pub const DEFAULT_BASELINE_REF: &str = "baseline_fixture";
    /// Product path allowed by the default Change Unit fixture.
    pub const DEFAULT_PRODUCT_PATH: &str = "src/export.rs";

    /// Automatically cleaned Volicord Runtime Home with one registered project and Agent Connection.
    #[derive(Debug)]
    pub struct CoreFixture {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
        product_repo_path: PathBuf,
        project_id: String,
        connection_id: String,
    }

    impl CoreFixture {
        /// Creates a disposable Runtime Home, Product Repository registration, and Agent Connection.
        pub fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            Self::new_with_host_kind(prefix, HOST_KIND_CODEX)
        }

        /// Creates the fixture with an exact built-in Agent Connection host kind.
        pub fn new_with_host_kind(prefix: &str, host_kind: &str) -> Result<Self, Box<dyn Error>> {
            let component = identifier_component(prefix);
            let runtime_home = TempRuntimeHome::new(&component)?;
            let repo_root = runtime_home.create_product_repo("repo")?;

            let project_id = DEFAULT_PROJECT_ID.to_owned();
            let connection_id = DEFAULT_CONNECTION_ID.to_owned();

            initialize_runtime_home(
                runtime_home.path(),
                &format!("runtime_home_{component}"),
                "{}",
            )?;
            write_installation_profile(
                runtime_home.path(),
                InstallationProfileRegistration {
                    installation_id: "default".to_owned(),
                    volicord_command: "volicord".to_owned(),
                    volicord_mcp_command: "volicord".to_owned(),
                    bin_dir: runtime_home.path().join("bin"),
                    default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: project_id.clone(),
                    repo_root: repo_root.clone(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                runtime_home.path(),
                AgentConnectionRegistration {
                    connection_internal_id: connection_id.clone(),
                    host_kind: host_kind.to_owned(),
                    intent: volicord_store::agent_connections::CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: "volicord-test".to_owned(),
                    config_target: runtime_home
                        .path()
                        .join("agent-connections")
                        .join(&component)
                        .to_string_lossy()
                        .into_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: format!("fixture:{component}"),
                    verification_report_json: None,
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                runtime_home.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: connection_id.clone(),
                    project_id: project_id.clone(),
                },
            )?;

            let runtime_home_path = runtime_home.path().to_path_buf();
            Ok(Self {
                _runtime_home: runtime_home,
                runtime_home_path,
                product_repo_path: repo_root,
                project_id,
                connection_id,
            })
        }

        /// Returns the disposable Runtime Home path.
        pub fn runtime_home_path(&self) -> &Path {
            &self.runtime_home_path
        }

        /// Returns the disposable Product Repository path for this fixture project.
        pub fn product_repo_path(&self) -> PathBuf {
            self.product_repo_path.clone()
        }

        /// Creates a disposable sibling Product Repository path for an extra fixture project.
        pub fn create_product_repo(&self, name: impl AsRef<Path>) -> std::io::Result<PathBuf> {
            let path = self
                .product_repo_path
                .parent()
                .expect("fixture product repo has a parent")
                .join(name.as_ref());
            fs::create_dir_all(&path)?;
            Ok(path)
        }

        /// Returns the registered project id.
        pub fn project_id(&self) -> &str {
            &self.project_id
        }

        /// Returns the registered Agent Connection id.
        pub fn connection_id(&self) -> &str {
            &self.connection_id
        }

        /// Returns the actor source associated with the fixture Agent Connection.
        pub fn actor_source(&self) -> String {
            format!("agent_connection:{}", self.connection_id)
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

        /// Replaces the project-owned enforcement profile JSON for focused corruption tests.
        pub fn set_project_enforcement_profile_json(
            &self,
            profile_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE project_state
                    SET enforcement_profile_json = ?2
                  WHERE project_id = ?1",
                rusqlite::params![self.project_id, profile_json],
            )?;
            Ok(())
        }

        /// Reads the project-owned enforcement profile JSON.
        pub fn project_enforcement_profile_json(&self) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT enforcement_profile_json
                   FROM project_state
                  WHERE project_id = ?1",
                rusqlite::params![self.project_id],
                |row| row.get(0),
            )?)
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
                request_id: RequestId::new(request_id),
                idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
                expected_state_version: expected_state_version.into(),
                dry_run,
                locale: Some("en-US".to_owned()).into(),
            }
        }

        /// Builds a default `volicord.status` request.
        pub fn status_request(&self, request_id: &str, task_id: Option<&str>) -> StatusRequest {
            StatusRequest {
                envelope: self.envelope(request_id, None, false, None, task_id),
                include: status_include_all(),
                continuity_page: None,
            }
        }

        /// Builds a default `volicord.intake` request.
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
                requested_control_level: RequestedControlLevel::Auto,
                resume_policy: ResumePolicy::CreateNew,
                acceptance_policy: RequiredNullable::null(),
                lineage: RequiredNullable::null(),
                initial_scope: InitialScope {
                    boundary: "Initial test scope.".to_owned(),
                    non_goals: vec!["Changing unrelated flows.".to_owned()],
                    acceptance_criteria: vec![AcceptanceCriterionInput {
                        statement: "The test export flow is represented.".to_owned(),
                        evidence_requirement: EvidenceRequirement::NotRequired,
                    }],
                },
                initial_context_refs: Vec::new(),
                initial_source_refs: Vec::new(),
            }
        }

        /// Builds a default `volicord.update_scope` request.
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
                acceptance_criteria: Some(vec![AcceptanceCriterionReplacement {
                    acceptance_criterion_id: None.into(),
                    statement: "The scoped behavior is represented.".to_owned(),
                    evidence_requirement: EvidenceRequirement::NotRequired,
                }])
                .into(),
                autonomy_boundary: Some("Stay inside the scoped test behavior.".to_owned()).into(),
                baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
                change_unit: ChangeUnitUpdate {
                    operation: input.operation,
                    effect_contract: None,
                    fields,
                },
                related_scope_decision_refs: Vec::new(),
            }
        }

        /// Builds a default `volicord.prepare_write` request.
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

        /// Builds a default `volicord.stage_artifact` request.
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

        /// Builds a default `volicord.record_run` request.
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
                write_ticket_id: None.into(),
                performed_operation: None.into(),
                summary: "Recorded implementation run.".to_owned(),
                observed_changes: ObservedChanges {
                    changed_paths: Vec::new(),
                    product_file_write_observed: false,
                    sensitive_categories: Vec::new(),
                    baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
                },
                artifact_inputs: Vec::new(),
                evidence_updates: Vec::new(),
                evidence_observations: Vec::new(),
                close_assessment: None.into(),
            }
        }

        /// Builds a default choice-shaped `volicord.request_user_action` request.
        pub fn user_action_request(
            &self,
            input: UserActionFixture<'_>,
        ) -> RequestUserActionRequest {
            let options = if matches!(
                input.judgment_kind,
                JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision
            ) {
                vec![
                    UserActionOptionInput {
                        option_id: UserActionOptionId::new("accept"),
                        label: "Accept".to_owned(),
                        description: "Resolve the focused user-owned choice.".to_owned(),
                        consequence: "Only this user action is accepted.".to_owned(),
                        is_default: true,
                    },
                    UserActionOptionInput {
                        option_id: UserActionOptionId::new("decline"),
                        label: "Decline".to_owned(),
                        description: "Record that the focused choice was not accepted.".to_owned(),
                        consequence: "The user action resolves without acceptance.".to_owned(),
                        is_default: false,
                    },
                ]
            } else {
                Vec::new()
            };

            RequestUserActionRequest {
                envelope: self.envelope(
                    input.request_id,
                    Some(input.idempotency_key),
                    input.dry_run,
                    input.expected_state_version,
                    Some(input.task_id),
                ),
                task_id: TaskId::new(input.task_id),
                change_unit_id: input.change_unit_id.map(ChangeUnitId::new).into(),
                action: UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
                    judgment_kind: input.judgment_kind,
                    presentation: JudgmentPresentation::Short,
                    question: "Choose the focused test user-action outcome.".to_owned(),
                    options: Some(options).into(),
                    context: UserActionContext {
                        summary: "A focused test user action needs a user-owned answer.".to_owned(),
                        related_refs: Vec::new(),
                        artifact_refs: Vec::new(),
                        visible_risks: Vec::new(),
                        constraints: vec![
                            "The answer covers only the requested action kind.".to_owned()
                        ],
                    },
                    affected_refs: vec![self.task_ref(input.task_id, input.expected_state_version)],
                    sensitive_action_scope: sensitive_action_scope_for_kind(input.judgment_kind)
                        .into(),
                })),
                required_for: required_for_for_kind(input.judgment_kind),
                expires_at: None.into(),
            }
        }

        /// Builds an evidence-observation `volicord.request_user_action` request.
        pub fn observation_user_action_request(
            &self,
            input: ObservationUserActionFixture<'_>,
        ) -> RequestUserActionRequest {
            RequestUserActionRequest {
                envelope: self.envelope(
                    input.request_id,
                    Some(input.idempotency_key),
                    input.dry_run,
                    input.expected_state_version,
                    Some(input.task_id),
                ),
                task_id: TaskId::new(input.task_id),
                change_unit_id: Some(ChangeUnitId::new(input.change_unit_id)).into(),
                action: UserActionDraft::EvidenceObservation(UserActionEvidenceObservationDraft {
                    question: "Classify the focused evidence observation.".to_owned(),
                    context_summary: "A user must assess the candidate artifact for the target."
                        .to_owned(),
                    target_candidates: input.target_candidates,
                    artifact_candidate_ids: input.artifact_candidate_ids,
                }),
                required_for: vec![UserActionRequiredFor::RecordRun],
                expires_at: None.into(),
            }
        }

        /// Builds a `volicord.resolve_user_action` request for a verified User Channel.
        pub fn resolve_user_action_request(
            &self,
            input: ResolveUserActionFixture<'_>,
        ) -> ResolveUserActionRequest {
            ResolveUserActionRequest {
                envelope: self.envelope(
                    input.request_id,
                    Some(input.channel_submission_id),
                    false,
                    None,
                    Some(input.task_id),
                ),
                user_action_request_id: UserActionRequestId::new(input.user_action_request_id),
                channel_submission_id: input.channel_submission_id.to_owned(),
                resolution: input.resolution,
            }
        }

        /// Builds a default `volicord.close_task` request.
        pub fn close_task_request(&self, input: CloseTaskFixture<'_>) -> CloseTaskRequest {
            let intent = match input.intent {
                CloseIntent::Complete => CloseMutationIntent::Complete,
                CloseIntent::Cancel => CloseMutationIntent::Cancel,
                CloseIntent::Supersede => CloseMutationIntent::Supersede,
                CloseIntent::Check => {
                    panic!("use check_close_request for close-readiness checks")
                }
            };
            CloseTaskRequest {
                envelope: self.envelope(
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
                user_note: Some("Focused close-task fixture.".to_owned()).into(),
            }
        }

        /// Builds a default `volicord.check_close` request.
        pub fn check_close_request(&self, input: CloseTaskFixture<'_>) -> CheckCloseRequest {
            CheckCloseRequest {
                envelope: self.envelope(
                    input.request_id,
                    input.idempotency_key,
                    input.dry_run,
                    input.expected_state_version,
                    Some(input.task_id),
                ),
                task_id: TaskId::new(input.task_id),
            }
        }

        /// Builds a `StateRecordRef` for a fixture Task.
        pub fn task_ref(&self, task_id: &str, state_version: Option<u64>) -> StateRecordRef {
            StateRecordRef {
                record_kind: StateRecordKind::Task,
                record_id: RecordId::new(task_id),
                project_id: ProjectId::new(&self.project_id),
                task_id: Some(TaskId::new(task_id)).into(),
                produced_at_state_version: state_version.into(),
            }
        }

        /// Reads the current status of a Write Ticket row.
        pub fn write_ticket_status(&self, write_ticket_id: &str) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT status
                   FROM write_tickets
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                rusqlite::params![self.project_id, write_ticket_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the basis state version of a Write Ticket row.
        pub fn write_ticket_basis(&self, write_ticket_id: &str) -> Result<u64, Box<dyn Error>> {
            let basis: i64 = self.conn()?.query_row(
                "SELECT basis_state_version
                   FROM write_tickets
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                rusqlite::params![self.project_id, write_ticket_id],
                |row| row.get(0),
            )?;
            Ok(u64::try_from(basis)?)
        }

        /// Reads the effective non-expiry status of a user-action request fixture.
        pub fn user_action_status(
            &self,
            user_action_request_id: &str,
        ) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT CASE
                          WHEN request.basis_status = 'stale' THEN 'stale'
                          WHEN request.basis_status = 'superseded' THEN 'superseded'
                          WHEN resolution.user_action_resolution_id IS NOT NULL THEN 'resolved'
                          ELSE 'pending'
                        END
                   FROM user_action_requests AS request
              LEFT JOIN user_action_resolutions AS resolution
                     ON resolution.project_id = request.project_id
                    AND resolution.user_action_request_id = request.user_action_request_id
                  WHERE request.project_id = ?1
                    AND request.user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the current compatibility status for a user-action request basis.
        pub fn user_action_basis_status(
            &self,
            user_action_request_id: &str,
        ) -> Result<String, StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT basis_status
                   FROM user_action_requests
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id],
                |row| row.get(0),
            )?)
        }

        /// Reads the immutable resolution JSON for a user-action request.
        pub fn user_action_resolution(
            &self,
            user_action_request_id: &str,
        ) -> Result<Value, Box<dyn Error>> {
            let text: String = self.conn()?.query_row(
                "SELECT resolution_json
                   FROM user_action_resolutions
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id],
                |row| row.get(0),
            )?;
            Ok(serde_json::from_str(&text)?)
        }

        /// Reads the Core-derived resolution outcome for a choice user action.
        pub fn user_action_resolution_outcome(
            &self,
            user_action_request_id: &str,
        ) -> Result<Option<String>, StoreError> {
            let Some(text) = self
                .conn()?
                .query_row(
                    "SELECT resolution_json
                       FROM user_action_resolutions
                      WHERE project_id = ?1
                        AND user_action_request_id = ?2",
                    rusqlite::params![self.project_id, user_action_request_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            else {
                return Ok(None);
            };
            let value: Value = serde_json::from_str(&text).map_err(|_| {
                StoreError::corrupt_owner_state_json(
                    "user_action_resolutions",
                    user_action_request_id,
                    "resolution_json",
                )
            })?;
            Ok(value
                .get("resolution_outcome")
                .and_then(Value::as_str)
                .map(str::to_owned))
        }

        /// Reads the Core-derived machine action for a choice user action.
        pub fn user_action_resolution_machine_action(
            &self,
            user_action_request_id: &str,
        ) -> Result<Option<String>, StoreError> {
            let Some(text) = self
                .conn()?
                .query_row(
                    "SELECT resolution_json
                       FROM user_action_resolutions
                      WHERE project_id = ?1
                        AND user_action_request_id = ?2",
                    rusqlite::params![self.project_id, user_action_request_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            else {
                return Ok(None);
            };
            let value: Value = serde_json::from_str(&text).map_err(|_| {
                StoreError::corrupt_owner_state_json(
                    "user_action_resolutions",
                    user_action_request_id,
                    "resolution_json",
                )
            })?;
            Ok(value
                .get("machine_action")
                .and_then(Value::as_str)
                .map(str::to_owned))
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
                  ORDER BY produced_at_state_version DESC, evidence_summary_id DESC
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
                rusqlite::params![self.project_id, task_id, self.actor_source()],
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

        /// Rewrites persisted artifact integrity facts for controlled integrity fixtures.
        pub fn set_artifact_integrity(
            &self,
            artifact_id: &str,
            integrity_status: &str,
            content_type: Option<&str>,
            sha256: Option<&str>,
            size_bytes: Option<u64>,
        ) -> Result<(), StoreError> {
            let size_bytes = size_bytes.and_then(|value| i64::try_from(value).ok());
            self.conn()?.execute(
                "UPDATE artifacts
                    SET integrity_status = ?3,
                        content_type = ?4,
                        sha256 = ?5,
                        size_bytes = ?6
                  WHERE project_id = ?1
                    AND artifact_id = ?2",
                rusqlite::params![
                    self.project_id,
                    artifact_id,
                    integrity_status,
                    content_type,
                    sha256,
                    size_bytes
                ],
            )?;
            Ok(())
        }

        /// Replaces a user-action resolution JSON value with SQL NULL or raw text.
        pub fn set_user_action_resolution_raw(
            &self,
            user_action_request_id: &str,
            raw_json: Option<&str>,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_action_resolutions
                    SET resolution_json = ?3
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id, raw_json],
            )?;
            Ok(())
        }

        /// Rewrites the stored choice outcome for controlled authority fixtures.
        pub fn set_user_action_resolution_outcome(
            &self,
            user_action_request_id: &str,
            outcome: Option<&str>,
        ) -> Result<(), Box<dyn Error>> {
            let outcome = outcome.ok_or("resolution outcome is required for current fixtures")?;
            let current_json: String = self.conn()?.query_row(
                "SELECT resolution_json
                   FROM user_action_resolutions
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id],
                |row| row.get(0),
            )?;
            let mut value: Value = serde_json::from_str(&current_json)?;
            value["resolution_outcome"] = Value::String(outcome.to_owned());
            let machine_action = match outcome {
                "accepted" => "accept",
                "rejected" => "reject",
                "deferred" => "defer",
                "blocked" => {
                    return Err("blocked has no current machine action".into());
                }
                value => {
                    return Err(format!("unsupported test outcome {value}").into());
                }
            };
            value["machine_action"] = Value::String(machine_action.to_owned());
            self.conn()?.execute(
                "UPDATE user_action_resolutions
                    SET resolution_json = ?3
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![
                    self.project_id,
                    user_action_request_id,
                    serde_json::to_string(&value)?
                ],
            )?;
            Ok(())
        }

        /// Rewrites the stored resolving actor for controlled authority fixtures.
        pub fn set_user_action_resolution_actor(
            &self,
            user_action_request_id: &str,
            actor_source: &str,
        ) -> Result<(), Box<dyn Error>> {
            self.conn()?.execute(
                "UPDATE user_action_resolutions
                    SET resolved_by_actor_source = ?3
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id, actor_source],
            )?;
            Ok(())
        }

        /// Replaces a user-action request JSON value with raw text.
        pub fn set_user_action_request_raw(
            &self,
            user_action_request_id: &str,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_action_requests
                    SET request_json = ?3
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id, raw_json],
            )?;
            Ok(())
        }

        /// Replaces a user-action request basis JSON value with raw text.
        pub fn set_user_action_basis_raw(
            &self,
            user_action_request_id: &str,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_action_requests
                    SET basis_json = ?3
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id, raw_json],
            )?;
            Ok(())
        }

        /// Attempts to clear the required user-action request basis JSON value.
        pub fn clear_user_action_basis(
            &self,
            user_action_request_id: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE user_action_requests
                    SET basis_json = NULL
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                rusqlite::params![self.project_id, user_action_request_id],
            )?;
            Ok(())
        }

        /// Replaces a Write Ticket attempt-scope JSON value with raw text.
        pub fn set_write_ticket_attempt_scope_raw(
            &self,
            write_ticket_id: &str,
            raw_json: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE write_tickets
                    SET attempt_scope_json = ?3
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                rusqlite::params![self.project_id, write_ticket_id, raw_json],
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

        /// Replaces Write Ticket creation and optional idle-timeout timestamps for fixed-clock tests.
        pub fn set_write_ticket_timestamps(
            &self,
            write_ticket_id: &str,
            created_at: &str,
            idle_expires_at: &str,
        ) -> Result<(), StoreError> {
            self.conn()?.execute(
                "UPDATE write_tickets
                    SET created_at = ?3,
                        idle_expires_at = ?4
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                rusqlite::params![
                    self.project_id,
                    write_ticket_id,
                    created_at,
                    idle_expires_at
                ],
            )?;
            Ok(())
        }

        /// Reads Write Ticket `created_at` and optional `idle_expires_at` timestamp strings.
        pub fn write_ticket_timestamps(
            &self,
            write_ticket_id: &str,
        ) -> Result<(String, Option<String>), StoreError> {
            Ok(self.conn()?.query_row(
                "SELECT created_at, idle_expires_at
                   FROM write_tickets
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                rusqlite::params![self.project_id, write_ticket_id],
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

        /// Reads the most recently appended Task-scoped authority event.
        pub fn latest_authority_event(&self) -> Result<AuthorityEventFixtureRow, Box<dyn Error>> {
            let (event_kind, event_payload_text, state_version): (String, String, i64) =
                self.conn()?.query_row(
                    "SELECT event_type, payload_json, state_version
                       FROM authority_events
                      WHERE project_id = ?1
                        AND task_id IS NOT NULL
                      ORDER BY event_seq DESC
                      LIMIT 1",
                    rusqlite::params![self.project_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
            Ok(AuthorityEventFixtureRow {
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
    }

    impl TaskOwnerJsonColumn {
        fn as_str(self) -> &'static str {
            match self {
                Self::ShapingSummary => "shaping_summary_json",
                Self::BoundedContext => "bounded_context_json",
                Self::AutonomyBoundary => "autonomy_boundary_json",
                Self::CurrentCloseBasis => "close_basis_json",
                Self::CloseSummary => "close_summary_json",
            }
        }
    }

    /// Change Unit owner JSON columns intentionally exposed for corruption fixtures.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ChangeUnitOwnerJsonColumn {
        ScopeSummary,
        BoundedPaths,
        WriteBasis,
        Lifecycle,
    }

    impl ChangeUnitOwnerJsonColumn {
        fn as_str(self) -> &'static str {
            match self {
                Self::ScopeSummary => "scope_summary_json",
                Self::BoundedPaths => "bounded_paths_json",
                Self::WriteBasis => "write_basis_json",
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

    /// Input object for choice-shaped request-user-action builders.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UserActionFixture<'a> {
        pub request_id: &'a str,
        pub idempotency_key: &'a str,
        pub dry_run: bool,
        pub expected_state_version: Option<u64>,
        pub task_id: &'a str,
        pub change_unit_id: Option<&'a str>,
        pub judgment_kind: JudgmentKind,
    }

    /// Input object for evidence-observation request-user-action builders.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ObservationUserActionFixture<'a> {
        pub request_id: &'a str,
        pub idempotency_key: &'a str,
        pub dry_run: bool,
        pub expected_state_version: Option<u64>,
        pub task_id: &'a str,
        pub change_unit_id: &'a str,
        pub target_candidates: Vec<EvidenceTarget>,
        pub artifact_candidate_ids: Vec<ArtifactId>,
    }

    /// Input object for resolve-user-action request builders.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ResolveUserActionFixture<'a> {
        pub request_id: &'a str,
        pub task_id: &'a str,
        pub user_action_request_id: &'a str,
        /// Stable User Channel submission id, also used as the idempotency key.
        pub channel_submission_id: &'a str,
        pub resolution: UserActionResolutionInput,
    }

    fn required_for_for_kind(judgment_kind: JudgmentKind) -> Vec<UserActionRequiredFor> {
        match judgment_kind {
            JudgmentKind::ScopeDecision => vec![UserActionRequiredFor::ScopeUpdate],
            JudgmentKind::SensitiveApproval => vec![
                UserActionRequiredFor::PrepareWrite,
                UserActionRequiredFor::CloseComplete,
            ],
            JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance => {
                vec![UserActionRequiredFor::CloseComplete]
            }
            JudgmentKind::Cancellation => vec![UserActionRequiredFor::CloseCancel],
            JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision => {
                vec![UserActionRequiredFor::CloseComplete]
            }
        }
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

    /// Task-scoped authority event fields read from the canonical record table.
    #[derive(Debug, Clone, PartialEq)]
    pub struct AuthorityEventFixtureRow {
        pub event_kind: String,
        pub event_payload: Value,
        pub state_version: u64,
    }

    /// Returns a status include object with every supported flag enabled.
    pub fn status_include_all() -> StatusInclude {
        StatusInclude {
            task: true,
            pending_user_actions: true,
            write_ticket: true,
            evidence: true,
            close: true,
            guarantees: true,
            continuity: true,
        }
    }

    /// Builds a choice resolution that preserves the selected user value and a bounded note.
    pub fn choice_user_action_resolution(selected_option_id: &str) -> UserActionResolutionInput {
        UserActionResolutionInput::Choice {
            selected_option_id: UserActionOptionId::new(selected_option_id),
            note: Some("Recorded by a focused user-action fixture.".to_owned()).into(),
        }
    }

    /// Builds a user-owned evidence-observation resolution with no caller-supplied time.
    pub fn observation_user_action_resolution(
        target: EvidenceTarget,
        artifact_ids: Vec<ArtifactId>,
        relevance_status: EvidenceRelevanceStatus,
        summary: impl Into<String>,
    ) -> UserActionResolutionInput {
        UserActionResolutionInput::EvidenceObservation {
            target,
            artifact_ids,
            relevance_status,
            summary: summary.into(),
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
            evidence_target: claim.map(supplemental_evidence_target).into(),
            expected_sha256: Some(handle.sha256).into(),
            expected_size_bytes: Some(handle.size_bytes).into(),
            redaction_state: Some(handle.redaction_state).into(),
        }
    }

    /// Builds a supported evidence coverage item.
    pub fn supported_evidence_update(claim: &str) -> EvidenceCoverageUpdate {
        EvidenceCoverageUpdate {
            target: supplemental_evidence_target(claim),
            coverage_state: EvidenceCoverageUpdateState::Supported,
            provenance: Some(EvidenceUpdateProvenance {
                source_kind: EvidenceSourceKind::ExternalTool,
                assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
                observed_at: None.into(),
                tool_name: Some("fixture-evidence-check".to_owned()).into(),
                tool_invocation_id: None.into(),
                tool_metadata: JsonObject::new(),
                source_refs: Vec::new(),
                limitations: Vec::new(),
            }),
            supporting_run_refs: Vec::new(),
            observation_refs: Vec::new(),
            supporting_artifact_refs: Vec::new(),
            gap_refs: Vec::new(),
        }
    }

    /// Builds an unsupported evidence coverage item.
    pub fn unsupported_evidence_update(claim: &str) -> EvidenceCoverageUpdate {
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

    fn supplemental_evidence_target(statement: &str) -> EvidenceTarget {
        let statement_hex = statement
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id: EvidenceClaimId::new(format!("claim_{statement_hex}")),
            statement: statement.to_owned(),
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
                capability_claim: "This is not a Write Ticket result.".to_owned(),
                expires_at: None.into(),
            }),
            _ => None,
        }
    }

    /// Builds an accepted-risk input for close-readiness fixtures.
    pub fn accepted_risk(summary: &str) -> AcceptedRiskInput {
        AcceptedRiskInput {
            risk_id: volicord_types::RiskId::new("risk_visible_001"),
            summary: summary.to_owned(),
            consequence: "The named residual risk remains after close.".to_owned(),
            related_refs: Vec::new(),
            accepted_for_close: true,
        }
    }

    /// Builds a `WriteTicketId` for tests that need the typed wrapper.
    pub fn write_ticket_id(value: &str) -> WriteTicketId {
        WriteTicketId::new(value)
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
}

/// Identifies the shared type boundary used by test helpers.
pub const fn shared_type_boundary() -> TypeBoundary {
    TypeBoundary::Domain
}

#[cfg(test)]
mod tests {
    use super::{
        core_fixtures::{choice_user_action_resolution, observation_user_action_resolution},
        disposable_runtime_home, shared_type_boundary, TempRuntimeHome,
    };
    use volicord_types::{
        ArtifactId, EvidenceClaimId, EvidenceRelevanceStatus, EvidenceTarget, TypeBoundary,
        UserActionResolutionInput,
    };

    #[test]
    fn disposable_runtime_home_stays_under_system_temp() {
        let path = disposable_runtime_home("workspace-skeleton");
        assert!(path.is_absolute());
        assert!(path.ends_with("volicord-test-runtime/workspace-skeleton"));
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

    #[test]
    fn user_action_resolution_helpers_keep_user_selection_bounded() {
        let choice = choice_user_action_resolution("accept");
        assert!(matches!(
            choice,
            UserActionResolutionInput::Choice {
                selected_option_id,
                note,
            } if selected_option_id.as_str() == "accept" && note.is_some()
        ));

        let observation = observation_user_action_resolution(
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id: EvidenceClaimId::new("claim_fixture"),
                statement: "The exact artifact is relevant.".to_owned(),
            },
            vec![ArtifactId::new("artifact_fixture")],
            EvidenceRelevanceStatus::Supported,
            "The user selected the exact candidate.",
        );
        assert!(matches!(
            observation,
            UserActionResolutionInput::EvidenceObservation { .. }
        ));
    }
}
