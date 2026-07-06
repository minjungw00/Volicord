use crate::errors::McpAdapterError;
use crate::prelude::*;
use crate::routing::*;
use crate::tool_registry::*;
use crate::util::*;

/// Minimal MCP adapter marker for validating dependency direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpAdapterBoundary {
    pub(crate) core: CoreBoundary,
}

impl McpAdapterBoundary {
    /// Creates an inert MCP adapter boundary marker.
    pub const fn new(core: CoreBoundary) -> Self {
        Self { core }
    }

    /// Returns the adapter boundary label.
    pub const fn label(self) -> &'static str {
        let _ = self.core;
        "mcp-adapter"
    }
}

/// Invocation context derived for one tool call before entering Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDerivedInvocationContext {
    pub project_id: ProjectId,
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub invocation_binding_basis: String,
    pub session_id: Option<String>,
    pub host_elicitation_available: bool,
    pub local_web_consent_available: bool,
}

impl McpDerivedInvocationContext {
    fn core_invocation(&self) -> InvocationContext {
        let mut invocation = InvocationContext::new(
            self.project_id.clone(),
            self.actor_source.clone(),
            self.operation_category,
            self.invocation_binding_basis.clone(),
        )
        .with_host_elicitation_available(self.host_elicitation_available)
        .with_local_web_consent_available(self.local_web_consent_available);
        if let Some(session_id) = self.session_id.as_ref() {
            invocation = invocation.with_session_id(session_id.clone());
        }
        invocation
    }
}

/// Loopback consent endpoint facts available to adapter fallback selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWebConsentContext {
    pub base_url: String,
}

/// Local MCP adapter bound to a Core service and one Agent Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAdapter {
    pub(crate) core: CoreService,
    pub(crate) runtime_home: PathBuf,
    pub(crate) context: McpConnectionContext,
    pub(crate) local_web_consent: Option<LocalWebConsentContext>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StartupObservationResult {
    Recorded,
    SkippedVerificationProbe,
    SkippedReadonlyStorage,
    FailedButNonfatal { reason: String },
    NotAttempted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedLifecycleEvent {
    Startup,
    InitializeResponse,
    ToolsList,
    ToolCallReceived,
    ToolCallCompleted,
}

impl ManagedLifecycleEvent {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "managed_host_startup",
            Self::InitializeResponse => "managed_host_initialize_response",
            Self::ToolsList => "managed_host_tools_list",
            Self::ToolCallReceived => "managed_host_tool_call",
            Self::ToolCallCompleted => "managed_host_tool_call_completed",
        }
    }
}

impl McpAdapter {
    /// Creates an adapter for a Runtime Home and connection-bound adapter context.
    pub fn new(runtime_home: impl AsRef<Path>, context: McpConnectionContext) -> Self {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        Self {
            core: CoreService::new(&runtime_home),
            runtime_home,
            context,
            local_web_consent: None,
        }
    }

    /// Enables local loopback web consent fallback for pending user judgments.
    pub fn with_local_web_consent(mut self, context: LocalWebConsentContext) -> Self {
        self.local_web_consent = Some(context);
        self
    }

    pub(crate) fn startup_session_watch_observation_best_effort(
        &self,
        session_id: &str,
    ) -> StartupObservationResult {
        match self.startup_session_watch_observation(session_id, None) {
            Ok(result) => result,
            Err(error) if startup_observation_storage_is_readonly(&error) => {
                StartupObservationResult::SkippedReadonlyStorage
            }
            Err(error) => StartupObservationResult::FailedButNonfatal {
                reason: error.to_string(),
            },
        }
    }

    pub(crate) fn startup_session_watch_observation_best_effort_with_origin(
        &self,
        session_id: &str,
        launch_origin: &str,
    ) -> StartupObservationResult {
        match self.startup_session_watch_observation(session_id, Some(launch_origin)) {
            Ok(result) => result,
            Err(error) if startup_observation_storage_is_readonly(&error) => {
                StartupObservationResult::SkippedReadonlyStorage
            }
            Err(error) => StartupObservationResult::FailedButNonfatal {
                reason: error.to_string(),
            },
        }
    }

    pub(crate) fn managed_lifecycle_observation_best_effort(
        &self,
        session_id: &str,
        launch_origin: &str,
        lifecycle_event: ManagedLifecycleEvent,
        tool_name: Option<&str>,
    ) -> StartupObservationResult {
        match self.managed_lifecycle_observation(
            session_id,
            launch_origin,
            lifecycle_event,
            tool_name,
        ) {
            Ok(result) => result,
            Err(error) if startup_observation_storage_is_readonly(&error) => {
                StartupObservationResult::SkippedReadonlyStorage
            }
            Err(error) => StartupObservationResult::FailedButNonfatal {
                reason: error.to_string(),
            },
        }
    }

    fn startup_session_watch_observation(
        &self,
        session_id: &str,
        launch_origin: Option<&str>,
    ) -> Result<StartupObservationResult, McpAdapterError> {
        let Some(project_id) = self.project_bound_startup_project()? else {
            return Ok(StartupObservationResult::NotAttempted);
        };
        self.ensure_session_watch_baseline(
            &project_id,
            session_id,
            SessionWatchCoverageBasis::McpStart,
            launch_origin,
        )?;
        Ok(StartupObservationResult::Recorded)
    }

    fn managed_lifecycle_observation(
        &self,
        session_id: &str,
        launch_origin: &str,
        lifecycle_event: ManagedLifecycleEvent,
        tool_name: Option<&str>,
    ) -> Result<StartupObservationResult, McpAdapterError> {
        let Some(project_id) = self.project_bound_startup_project()? else {
            return Ok(StartupObservationResult::NotAttempted);
        };
        self.ensure_session_watch_baseline(
            &project_id,
            session_id,
            SessionWatchCoverageBasis::McpStart,
            Some(launch_origin),
        )?;
        self.append_managed_lifecycle_event(
            &project_id,
            session_id,
            launch_origin,
            lifecycle_event,
            tool_name,
        )?;
        Ok(StartupObservationResult::Recorded)
    }

    fn project_bound_startup_project(&self) -> Result<Option<ProjectId>, McpAdapterError> {
        let available_projects = self
            .allowed_project_availabilities("session watch startup")?
            .into_iter()
            .filter(|project| project.available)
            .collect::<Vec<_>>();
        if available_projects.len() == 1 {
            Ok(Some(ProjectId::new(&available_projects[0].project_id)))
        } else {
            Ok(None)
        }
    }

    fn ensure_session_watch_baseline(
        &self,
        project_id: &ProjectId,
        session_id: &str,
        coverage_basis: SessionWatchCoverageBasis,
        launch_origin: Option<&str>,
    ) -> Result<(), McpAdapterError> {
        if latest_watch_baseline_for_session(&self.runtime_home, project_id.as_str(), session_id)
            .map_err(McpAdapterError::Store)?
            .is_some()
        {
            return Ok(());
        }

        let now = CoreProjectStore::open(&self.runtime_home, project_id)
            .and_then(|store| store.current_timestamp())
            .map_err(McpAdapterError::Store)?;
        self.ensure_agent_session_for_watch(project_id, session_id, &now)?;

        if latest_watch_baseline_for_session(&self.runtime_home, project_id.as_str(), session_id)
            .map_err(McpAdapterError::Store)?
            .is_some()
        {
            return Ok(());
        }

        let store = CoreProjectStore::open(&self.runtime_home, project_id)
            .map_err(McpAdapterError::Store)?;
        let snapshot = match snapshot_product_repository(
            &self.runtime_home,
            &store.project_record().repo_root,
            WatchSnapshotOptions::default(),
        ) {
            Ok(snapshot) => snapshot,
            Err(_) => return Ok(()),
        };
        let partial_coverage_warning = match coverage_basis {
            SessionWatchCoverageBasis::McpStart => None,
            SessionWatchCoverageBasis::FirstProjectSelection => {
                Some(FIRST_PROJECT_SELECTION_PARTIAL_COVERAGE_WARNING)
            }
            SessionWatchCoverageBasis::MethodBoundary => {
                Some(METHOD_BOUNDARY_PARTIAL_COVERAGE_WARNING)
            }
        };
        let mut metadata = json!({
            "source": WATCH_METADATA_SOURCE,
            "status_detail": "active",
            "detector_role": "detective",
            "does_not_prevent_writes": true,
            "does_not_identify_actor": true,
            "coverage_start_at": now,
            "coverage_basis": coverage_basis.as_str(),
            "scan_summary": Self::session_watch_scan_summary_from_snapshot(&snapshot),
        });
        if let Some(warning) = partial_coverage_warning {
            metadata["partial_coverage_warning"] = json!(warning);
        }
        if let Some(launch_origin) = launch_origin {
            metadata["launch_origin"] = json!(launch_origin);
        }
        create_watch_baseline(
            &self.runtime_home,
            project_id.as_str(),
            WatchBaselineCreate {
                watch_baseline_id: generated_metadata_id(
                    "watch_base",
                    project_id.as_str(),
                    session_id,
                ),
                session_id: session_id.to_owned(),
                connection_internal_id: self.context.connection_internal_id.as_str().to_owned(),
                guard_installation_id: self.selected_guard_installation_id(project_id)?,
                status: StoreSessionWatchStatus::Active,
                snapshot,
                created_at: metadata["coverage_start_at"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                metadata_json: serde_json::to_string(&metadata).map_err(McpAdapterError::Json)?,
            },
        )
        .map_err(McpAdapterError::Store)?;
        Ok(())
    }

    fn append_managed_lifecycle_event(
        &self,
        project_id: &ProjectId,
        session_id: &str,
        launch_origin: &str,
        lifecycle_event: ManagedLifecycleEvent,
        tool_name: Option<&str>,
    ) -> Result<(), McpAdapterError> {
        let Some(baseline) =
            latest_watch_baseline_for_session(&self.runtime_home, project_id.as_str(), session_id)
                .map_err(McpAdapterError::Store)?
        else {
            return Ok(());
        };
        let now = CoreProjectStore::open(&self.runtime_home, project_id)
            .and_then(|store| store.current_timestamp())
            .map_err(McpAdapterError::Store)?;
        let mut metadata =
            serde_json::from_str::<Value>(&baseline.metadata_json).unwrap_or_else(|_| json!({}));
        if !metadata.is_object() {
            metadata = json!({});
        }
        let object = metadata
            .as_object_mut()
            .expect("metadata was normalized to an object");
        object.insert("host_kind".to_owned(), json!("codex"));
        object.insert("launch_origin".to_owned(), json!(launch_origin));
        object.insert(
            "connection_id".to_owned(),
            json!(self.context.connection_internal_id.as_str()),
        );
        object.insert("project_id".to_owned(), json!(project_id.as_str()));
        object.insert(
            "latest_lifecycle_event".to_owned(),
            json!(lifecycle_event.as_str()),
        );
        object.insert("latest_lifecycle_observed_at".to_owned(), json!(&now));

        let mut event =
            self.managed_lifecycle_event_metadata(project_id, launch_origin, lifecycle_event, &now);
        if let Some(tool_name) = tool_name {
            event["tool_name"] = json!(tool_name);
        }

        let events = object
            .entry("lifecycle_events".to_owned())
            .or_insert_with(|| json!([]));
        if !events.is_array() {
            *events = json!([]);
        }
        events
            .as_array_mut()
            .expect("lifecycle_events was normalized to an array")
            .push(event);

        let status = session_watch_status_from_storage(&baseline.status)?;
        update_watch_status(
            &self.runtime_home,
            project_id.as_str(),
            &baseline.watch_baseline_id,
            WatchStatusUpdate {
                status,
                updated_at: now,
                metadata_json: serde_json::to_string(&metadata).map_err(McpAdapterError::Json)?,
            },
        )
        .map_err(McpAdapterError::Store)?;
        Ok(())
    }

    fn managed_lifecycle_event_metadata(
        &self,
        project_id: &ProjectId,
        launch_origin: &str,
        lifecycle_event: ManagedLifecycleEvent,
        timestamp: &str,
    ) -> Value {
        let storage_capability = self
            .storage_capability_for_project(project_id)
            .unwrap_or(McpStorageCapability::Unknown);
        let effective_tool_mode = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            lifecycle_event.as_str(),
        )
        .ok()
        .and_then(|connection| parse_connection_mode(&connection.mode).ok())
        .map(|mode| effective_tool_mode_for_mode_and_storage(mode, storage_capability).as_str())
        .unwrap_or("unknown");
        json!({
            "connection_id": self.context.connection_internal_id.as_str(),
            "project_id": project_id.as_str(),
            "host_kind": "codex",
            "launch_origin": launch_origin,
            "lifecycle_event": lifecycle_event.as_str(),
            "timestamp": timestamp,
            "storage_capability": storage_capability.as_str(),
            "effective_tool_mode": effective_tool_mode,
        })
    }

    fn session_watch_scan_summary_from_snapshot(
        snapshot: &volicord_store::session_watch::WatchSnapshot,
    ) -> SessionWatchScanSummary {
        let summary = &snapshot.scan_summary;
        SessionWatchScanSummary {
            files_scanned: summary.files_scanned,
            files_skipped: summary.files_skipped,
            unreadable_paths_count: summary.unreadable_paths_count,
            degraded_reasons: summary.degraded_reasons.clone(),
            degraded_reason_counts: summary.degraded_reason_counts.clone(),
            skipped_paths_sample: summary.skipped_paths_sample.clone(),
            skipped_paths_truncated: summary.skipped_paths_truncated,
            default_excluded_paths: volicord_store::session_watch::default_watch_excluded_paths(),
            max_file_size_bytes: volicord_store::session_watch::DEFAULT_MAX_FILE_HASH_BYTES,
            max_file_count: volicord_store::session_watch::DEFAULT_MAX_SCAN_FILE_COUNT,
            follows_symlinks: false,
            not_full_filesystem_monitoring: true,
        }
    }

    fn ensure_agent_session_for_watch(
        &self,
        project_id: &ProjectId,
        session_id: &str,
        now: &str,
    ) -> Result<(), McpAdapterError> {
        if agent_session(&self.runtime_home, project_id.as_str(), session_id)
            .map_err(McpAdapterError::Store)?
            .is_some()
        {
            return Ok(());
        }
        let record = guard_health_record(
            &self.runtime_home,
            project_id.as_str(),
            self.context.connection_internal_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?;
        let guard_installation_id = record
            .guard_installation
            .as_ref()
            .map(|installation| installation.guard_installation_id.clone());
        let guard_mode = record
            .guard_installation
            .as_ref()
            .map(|installation| installation.guard_mode.clone())
            .or_else(|| {
                record
                    .latest_session
                    .as_ref()
                    .map(|session| session.guard_mode.clone())
            })
            .unwrap_or_else(|| IntegrationProfile::Record.as_str().to_owned());
        let host_kind = record
            .guard_installation
            .as_ref()
            .map(|installation| installation.host_kind.clone())
            .or_else(|| {
                record
                    .connection
                    .as_ref()
                    .map(|connection| connection.host_kind.clone())
            })
            .unwrap_or_else(|| "unknown".to_owned());

        insert_agent_session(
            &self.runtime_home,
            project_id.as_str(),
            AgentSessionInsert {
                session_id: session_id.to_owned(),
                connection_internal_id: self.context.connection_internal_id.as_str().to_owned(),
                guard_installation_id,
                host_kind,
                guard_mode,
                started_at: now.to_owned(),
                metadata_json: serde_json::to_string(&json!({
                    "source": WATCH_METADATA_SOURCE,
                    "session_watch_initialized": true
                }))
                .map_err(McpAdapterError::Json)?,
            },
        )
        .map_err(McpAdapterError::Store)?;
        Ok(())
    }

    fn selected_guard_installation_id(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<String>, McpAdapterError> {
        guard_health_record(
            &self.runtime_home,
            project_id.as_str(),
            self.context.connection_internal_id.as_str(),
        )
        .map(|record| {
            record
                .guard_installation
                .map(|installation| installation.guard_installation_id)
        })
        .map_err(McpAdapterError::Store)
    }

    pub(crate) fn allowed_project_availabilities(
        &self,
        tool_name: &str,
    ) -> Result<Vec<McpProjectAvailability>, McpAdapterError> {
        current_enabled_connection(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            tool_name,
        )?;
        let projects = list_connection_projects_read_only(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?;
        Ok(projects
            .iter()
            .filter(|project| {
                self.context
                    .project_allowlist_allows(project.project_id.as_str())
            })
            .map(inspect_allowed_project)
            .collect())
    }

    fn session_watch_coverage_for_projects(
        &self,
        session_id: Option<&str>,
        projects: &[McpProjectAvailability],
    ) -> Result<McpSessionWatchCoverage, McpAdapterError> {
        if let Some(session_id) = session_id {
            for project in projects.iter().filter(|project| project.available) {
                if let Some(baseline) = latest_watch_baseline_for_session(
                    &self.runtime_home,
                    &project.project_id,
                    session_id,
                )
                .map_err(McpAdapterError::Store)?
                {
                    return Ok(coverage_from_watch_baseline(&baseline));
                }
            }
        }
        let available_project_count = projects.iter().filter(|project| project.available).count();
        if available_project_count == 1 {
            Ok(McpSessionWatchCoverage {
                status: SessionWatchStatus::Unavailable,
                baseline_created_at: None,
                coverage_start_at: None,
                coverage_basis: None,
                partial_coverage_warning: Some(
                    "Session-watch baseline has not been created for this MCP session.".to_owned(),
                ),
            })
        } else {
            Ok(McpSessionWatchCoverage {
                status: SessionWatchStatus::PendingProjectSelection,
                baseline_created_at: None,
                coverage_start_at: None,
                coverage_basis: None,
                partial_coverage_warning: Some(
                    "Session-watch coverage is pending until the MCP request names an explicit project_selector."
                        .to_owned(),
                ),
            })
        }
    }

    /// Returns the tools exposed by this adapter's current connection mode.
    pub fn tools(&self) -> Result<Vec<McpToolDefinition>, McpAdapterError> {
        let connection = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            "tools/list",
        )?;
        let mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: "tools/list".to_owned(),
                message: error.to_string(),
            }
        })?;
        let storage_capability = self.session_storage_capability()?;
        Ok(mcp_tools_for_mode_and_storage(mode, storage_capability))
    }

    pub(crate) fn session_storage_capability(
        &self,
    ) -> Result<McpStorageCapability, McpAdapterError> {
        let projects = self.allowed_project_availabilities("storage capability")?;
        Ok(storage_capability_for_projects(&projects))
    }

    /// Derives local invocation facts for one decoded request envelope.
    pub fn derive_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> McpDerivedInvocationContext {
        McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(
                self.context.connection_internal_id.clone(),
            ),
            operation_category,
            invocation_binding_basis: self.context.invocation_binding_basis.clone(),
            session_id: session_id.map(str::to_owned),
            host_elicitation_available,
            local_web_consent_available: self.local_web_consent.is_some(),
        }
    }

    /// Calls one public Volicord method tool and returns Core's response.
    pub fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        self.call_tool_for_session(tool_name, params, None)
    }

    pub(crate) fn call_tool_for_session(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        self.call_tool_for_session_with_capabilities(tool_name, params, session_id, false)
    }

    pub(crate) fn call_tool_for_session_with_capabilities(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        if let Some(response) = self.readonly_storage_rejection_for_tool(tool_name)? {
            return Ok(response);
        }
        match tool_name {
            INTAKE_TOOL_NAME => {
                self.call_intake(tool_name, params, session_id, host_elicitation_available)
            }
            UPDATE_SCOPE_TOOL_NAME => {
                self.call_update_scope(tool_name, params, session_id, host_elicitation_available)
            }
            STATUS_TOOL_NAME => {
                self.call_status(tool_name, params, session_id, host_elicitation_available)
            }
            PREPARE_WRITE_TOOL_NAME => {
                self.call_prepare_write(tool_name, params, session_id, host_elicitation_available)
            }
            STAGE_ARTIFACT_TOOL_NAME => {
                self.call_stage_artifact(tool_name, params, session_id, host_elicitation_available)
            }
            RECORD_RUN_TOOL_NAME => {
                self.call_record_run(tool_name, params, session_id, host_elicitation_available)
            }
            REQUEST_USER_JUDGMENT_TOOL_NAME => self.call_request_user_judgment(
                tool_name,
                params,
                session_id,
                host_elicitation_available,
            ),
            RECONCILE_CHANGES_TOOL_NAME => self.call_reconcile_changes(
                tool_name,
                params,
                session_id,
                host_elicitation_available,
            ),
            CHECK_CLOSE_TOOL_NAME => {
                self.call_check_close(tool_name, params, session_id, host_elicitation_available)
            }
            CLOSE_TASK_TOOL_NAME => {
                self.call_close_task(tool_name, params, session_id, host_elicitation_available)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn call_intake(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpIntakeArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            None,
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            IntakeRequest {
                envelope,
                plain_language_request: args.plain_language_request,
                requested_mode: args.requested_mode,
                resume_policy: args.resume_policy,
                initial_scope: args.initial_scope,
                initial_context_refs: args.initial_context_refs,
            },
            CoreService::intake,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_update_scope(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpUpdateScopeArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            UpdateScopeRequest {
                envelope,
                task_id,
                goal_summary: args.goal_summary,
                scope_update: args.scope_update,
                scope_boundary: args.scope_boundary,
                non_goals: args.non_goals,
                acceptance_criteria: args.acceptance_criteria,
                autonomy_boundary: args.autonomy_boundary,
                baseline_ref: args.baseline_ref,
                change_unit: args.change_unit,
                related_scope_decision_refs: args.related_scope_decision_refs,
            },
            CoreService::update_scope,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_status(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStatusArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            task_id.as_ref(),
            OperationCategory::Read,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            StatusRequest {
                envelope,
                include: args.detail.include(),
            },
            CoreService::status,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_prepare_write(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareWriteArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            task_id.as_ref(),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            PrepareWriteRequest {
                envelope,
                task_id,
                change_unit_id: args.change_unit_id,
                intended_operation: args.intended_operation,
                intended_paths: args.intended_paths,
                product_file_write_intended: args.product_file_write_intended,
                sensitive_categories: args.sensitive_categories,
                baseline_ref: args.baseline_ref,
            },
            CoreService::prepare_write,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_stage_artifact(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStageArtifactArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            StageArtifactRequest {
                envelope,
                task_id,
                display_name: args.display_name,
                content_type: args.content_type,
                redaction_state: args.redaction_state,
                safe_bytes_or_notice: args.safe_bytes_or_notice,
                expected_sha256: args.expected_sha256,
                expected_size_bytes: args.expected_size_bytes,
                relation_hint: args.relation_hint,
            },
            CoreService::stage_artifact,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_record_run(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRecordRunArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            RecordRunRequest {
                envelope,
                task_id,
                change_unit_id: args.change_unit_id,
                kind: args.kind,
                run_id: args.run_id,
                baseline_ref: args.baseline_ref,
                write_ticket_id: args.write_ticket_id,
                summary: args.summary,
                observed_changes: args.observed_changes,
                artifact_inputs: args.artifact_inputs,
                evidence_updates: args.evidence_updates,
                evidence_observations: args.evidence_observations,
                close_assessment: args.close_assessment,
            },
            CoreService::record_run,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_request_user_judgment(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRequestUserJudgmentArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            RequestUserJudgmentRequest {
                envelope,
                task_id,
                change_unit_id: args.change_unit_id,
                sensitive_action_scope: args.sensitive_action_scope,
                judgment_kind: args.judgment_kind,
                presentation: args.presentation,
                question: args.question,
                options: args.options,
                context: args.context,
                affected_refs: args.affected_refs,
                required_for: args.required_for,
                expires_at: args.expires_at,
            },
            CoreService::request_user_judgment,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_reconcile_changes(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpReconcileChangesArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            ReconcileChangesRequest {
                envelope,
                task_id,
                resolution_requests: args.resolution_requests,
            },
            CoreService::reconcile_changes,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_check_close(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCheckCloseArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::Read,
        )?;
        self.call_core_request(
            tool_name,
            CheckCloseRequest { envelope, task_id },
            CoreService::check_close,
            session_id,
            host_elicitation_available,
        )
    }

    fn call_close_task(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCloseTaskArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            CloseTaskRequest {
                envelope,
                task_id,
                intent: args.intent,
                close_reason: args.close_reason,
                superseding_task_id: args.superseding_task_id,
                user_note: args.user_note,
            },
            CoreService::close_task,
            session_id,
            host_elicitation_available,
        )
    }

    fn readonly_storage_rejection_for_tool(
        &self,
        tool_name: &str,
    ) -> Result<Option<PipelineResponse>, McpAdapterError> {
        let Some(operation_category) = public_tool_operation_category(tool_name) else {
            return Ok(None);
        };
        if operation_category == OperationCategory::Read {
            return Ok(None);
        }
        let storage_capability = self.session_storage_capability()?;
        if storage_capability.allows_mutation() {
            return Ok(None);
        }
        let mut details = Map::new();
        details.insert(
            "storage_capability".to_owned(),
            Value::String(storage_capability.as_str().to_owned()),
        );
        details.insert(
            "required_storage_capability".to_owned(),
            Value::String(McpStorageCapability::ReadWrite.as_str().to_owned()),
        );
        details.insert("tool_name".to_owned(), Value::String(tool_name.to_owned()));
        details.insert(
            "operation_category".to_owned(),
            Value::String(operation_category.as_str().to_owned()),
        );
        let response = rejected_response(
            false,
            None,
            vec![tool_error(
                ErrorCode::McpUnavailable,
                "Volicord project state is not writable in the current MCP host environment.",
                false,
                Some(details),
            )],
        );
        let response_value = serde_json::to_value(response).map_err(McpAdapterError::Json)?;
        let response_json =
            serde_json::to_string(&response_value).map_err(McpAdapterError::Json)?;
        Ok(Some(PipelineResponse {
            response_json,
            response_value,
            verified_invocation: None,
            resolved_task_id: None,
            replayed: false,
        }))
    }

    fn call_core_request<T, F>(
        &self,
        tool_name: &str,
        request: T,
        call: F,
        session_id: Option<&str>,
        host_elicitation_available: bool,
    ) -> Result<PipelineResponse, McpAdapterError>
    where
        T: MethodOperationCategory + HasEnvelope,
        F: FnOnce(
            &CoreService,
            T,
            InvocationContext,
        ) -> Result<PipelineResponse, CorePipelineError>,
    {
        let operation_category = request.operation_category();
        self.ensure_mode_allows(tool_name, operation_category)?;
        let invocation = self.derive_invocation_context(
            request_envelope(&request),
            operation_category,
            session_id,
            host_elicitation_available,
        );
        call(&self.core, request, invocation.core_invocation()).map_err(McpAdapterError::Core)
    }

    pub(crate) fn call_adapter_tool(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value, McpAdapterError> {
        match tool_name {
            LIST_PROJECTS_TOOL_NAME => {
                let object = params
                    .as_object()
                    .ok_or_else(|| McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: "volicord.list_projects arguments must be an object".to_owned(),
                    })?;
                if !object.is_empty() {
                    return Err(McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: "volicord.list_projects does not accept arguments".to_owned(),
                    });
                }
                let result = self.list_projects_result(session_id)?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn list_projects_result(
        &self,
        session_id: Option<&str>,
    ) -> Result<ListProjectsResult, McpAdapterError> {
        let connection = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            "volicord.list_projects",
        )?;
        let availabilities = self.allowed_project_availabilities("volicord.list_projects")?;
        let items = availabilities
            .iter()
            .map(|project| ListProjectItem {
                project_selector: project.project_id.clone(),
                available: project.available,
                unavailable_reason: project.unavailable_reason.clone(),
                repo_root: project.repo_root_display.clone(),
            })
            .collect::<Vec<_>>();
        let coverage = self.session_watch_coverage_for_projects(session_id, &availabilities)?;
        let mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: "volicord.list_projects".to_owned(),
                message: error.to_string(),
            }
        })?;

        Ok(ListProjectsResult {
            connection_id: connection.connection_internal_id,
            mode,
            watcher_status: coverage.status,
            watcher_baseline_created_at: coverage.baseline_created_at,
            watcher_coverage_start_at: coverage.coverage_start_at,
            watcher_coverage_basis: coverage.coverage_basis,
            watcher_partial_coverage_warning: coverage.partial_coverage_warning,
            projects: items,
        })
    }

    fn prepare_mcp_arguments<T>(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<PreparedMcpArguments<T>, McpAdapterError>
    where
        T: serde::de::DeserializeOwned,
    {
        let object = params
            .as_object()
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "tool arguments must be an object".to_owned(),
            })?;
        reject_internal_mcp_argument_fields(object, tool_name)?;
        let requested_project_selector =
            optional_string_field(object, "project_selector", tool_name)?;
        let selected_project_id = self.select_project(requested_project_selector.as_deref())?;
        if let Some(session_id) = session_id {
            if self.storage_capability_for_project(&selected_project_id)?
                == McpStorageCapability::ReadWrite
            {
                let coverage_basis = if requested_project_selector.is_some() {
                    SessionWatchCoverageBasis::FirstProjectSelection
                } else {
                    SessionWatchCoverageBasis::MethodBoundary
                };
                self.ensure_session_watch_baseline(
                    &selected_project_id,
                    session_id,
                    coverage_basis,
                    None,
                )?;
            }
        }
        let arguments = self.decode_params(tool_name, params)?;
        Ok(PreparedMcpArguments {
            arguments,
            project_id: selected_project_id,
        })
    }

    fn storage_capability_for_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<McpStorageCapability, McpAdapterError> {
        let access = agent_connection_project_access_read_only(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            project_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| routing_error("connection is not registered"))?;
        let Some(project) = access.project else {
            return Ok(McpStorageCapability::Unavailable);
        };
        let availability = inspect_allowed_project(&ConnectionProjectRecord {
            connection_internal_id: self.context.connection_internal_id.as_str().to_owned(),
            project_internal_id: project.project_internal_id.clone(),
            project_id: project.project_id.clone(),
            created_at: String::new(),
            project,
        });
        Ok(availability.storage_capability)
    }

    fn generated_envelope(
        &self,
        tool_name: &str,
        project_id: &ProjectId,
        task_id: Option<&volicord_types::TaskId>,
        operation_category: OperationCategory,
    ) -> Result<ToolEnvelope, McpAdapterError> {
        let state_version = if operation_category == OperationCategory::Read {
            None
        } else {
            Some(self.current_state_version(project_id)?)
        };
        let idempotency_key = if operation_category == OperationCategory::Read {
            RequiredNullable::null()
        } else {
            RequiredNullable::some(IdempotencyKey::new(generated_metadata_id(
                "idem",
                self.context.connection_internal_id.as_str(),
                tool_name,
            )))
        };

        Ok(ToolEnvelope {
            project_id: project_id.clone(),
            task_id: task_id.cloned().into(),
            request_id: RequestId::new(generated_metadata_id(
                "req",
                self.context.connection_internal_id.as_str(),
                tool_name,
            )),
            idempotency_key,
            expected_state_version: state_version.into(),
            dry_run: false,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        })
    }

    fn current_state_version(&self, project_id: &ProjectId) -> Result<u64, McpAdapterError> {
        let store = CoreProjectStore::open(&self.runtime_home, project_id)
            .map_err(McpAdapterError::Store)?;
        store
            .project_state()
            .map(|state| state.state_version)
            .map_err(McpAdapterError::Store)
    }

    fn select_project(
        &self,
        requested_project_id: Option<&str>,
    ) -> Result<ProjectId, McpAdapterError> {
        let connection_internal_id = self.context.connection_internal_id.as_str();
        let _connection = current_enabled_connection(
            &self.runtime_home,
            connection_internal_id,
            "project routing",
        )?;

        if let Some(project_id) = requested_project_id {
            if !self.context.project_allowlist_allows(project_id) {
                return Err(routing_error(format!(
                    "project selector {project_id} is outside this HTTP serve project allowlist"
                )));
            }
            let access = agent_connection_project_access_read_only(
                &self.runtime_home,
                connection_internal_id,
                project_id,
            )
            .map_err(McpAdapterError::Store)?
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: "project routing".to_owned(),
                message: format!("connection {connection_internal_id} is not registered"),
            })?;
            if !access.connection_enabled {
                return Err(routing_error("connection is disabled"));
            }
            if !access.project_allowed {
                return Err(routing_error(format!(
                    "project selector {project_id} is outside this connection project allowlist"
                )));
            }
            let project = access
                .project
                .ok_or_else(|| routing_error(format!("project {project_id} is not registered")))?;
            let project_record = ConnectionProjectRecord {
                connection_internal_id: connection_internal_id.to_owned(),
                project_internal_id: project.project_internal_id.clone(),
                project_id: project.project_id.clone(),
                created_at: String::new(),
                project,
            };
            let availability = inspect_allowed_project(&project_record);
            return selected_project_from_availability(availability);
        }

        let projects =
            list_connection_projects_read_only(&self.runtime_home, connection_internal_id)
                .map_err(McpAdapterError::Store)?;
        let projects = projects
            .into_iter()
            .filter(|project| {
                self.context
                    .project_allowlist_allows(project.project_id.as_str())
            })
            .collect::<Vec<_>>();
        if projects.is_empty() {
            return Err(routing_error(
                "connection has no allowed projects matching this transport allowlist; ask the operator to add one",
            ));
        }
        if projects.len() != 1 {
            return Err(routing_error(
                "project selection is ambiguous for this connection; project_selector is required when multiple projects are allowed",
            ));
        }

        selected_project_from_availability(inspect_allowed_project(&projects[0]))
    }

    fn ensure_mode_allows(
        &self,
        tool_name: &str,
        operation_category: OperationCategory,
    ) -> Result<(), McpAdapterError> {
        let connection = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            tool_name,
        )?;
        let current_mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        if current_mode.allows_operation_category(operation_category) {
            return Ok(());
        }
        Err(McpAdapterError::ToolExecution {
            tool_name: tool_name.to_owned(),
            message: format!(
                "connection mode {} does not allow operation category {}",
                current_mode.as_str(),
                operation_category.as_str()
            ),
        })
    }

    fn decode_params<T>(&self, tool_name: &str, params: Value) -> Result<T, McpAdapterError>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value(params).map_err(|source| McpAdapterError::InvalidParams {
            tool_name: tool_name.to_owned(),
            source,
        })
    }
}

fn startup_observation_storage_is_readonly(error: &McpAdapterError) -> bool {
    let McpAdapterError::Store(error) = error else {
        return false;
    };
    match error {
        StoreError::Io(error) => error.kind() == io::ErrorKind::PermissionDenied,
        StoreError::Sqlite(_) => error.classification().category == "database_access_denied",
        _ => false,
    }
}

fn session_watch_status_from_storage(
    status: &str,
) -> Result<StoreSessionWatchStatus, McpAdapterError> {
    match status {
        "disabled" => Ok(StoreSessionWatchStatus::Disabled),
        "active" => Ok(StoreSessionWatchStatus::Active),
        "degraded" => Ok(StoreSessionWatchStatus::Degraded),
        "unavailable" => Ok(StoreSessionWatchStatus::Unavailable),
        _ => Err(McpAdapterError::ToolExecution {
            tool_name: "managed MCP lifecycle observation".to_owned(),
            message: format!("session-watch baseline has unsupported status {status}"),
        }),
    }
}

trait HasEnvelope {
    fn envelope(&self) -> &ToolEnvelope;
}

macro_rules! impl_has_envelope {
    ($($request:ty),* $(,)?) => {
        $(
            impl HasEnvelope for $request {
                fn envelope(&self) -> &ToolEnvelope {
                    &self.envelope
                }
            }
        )*
    };
}

impl_has_envelope!(
    IntakeRequest,
    UpdateScopeRequest,
    StatusRequest,
    PrepareWriteRequest,
    StageArtifactRequest,
    RecordRunRequest,
    RequestUserJudgmentRequest,
    ReconcileChangesRequest,
    CheckCloseRequest,
    CloseTaskRequest,
);

fn request_envelope<T: HasEnvelope>(request: &T) -> &ToolEnvelope {
    request.envelope()
}

fn public_tool_operation_category(tool_name: &str) -> Option<OperationCategory> {
    match tool_name {
        STATUS_TOOL_NAME | CHECK_CLOSE_TOOL_NAME => Some(OperationCategory::Read),
        INTAKE_TOOL_NAME
        | UPDATE_SCOPE_TOOL_NAME
        | PREPARE_WRITE_TOOL_NAME
        | STAGE_ARTIFACT_TOOL_NAME
        | RECORD_RUN_TOOL_NAME
        | REQUEST_USER_JUDGMENT_TOOL_NAME
        | RECONCILE_CHANGES_TOOL_NAME
        | CLOSE_TASK_TOOL_NAME => Some(OperationCategory::AgentWorkflow),
        _ => None,
    }
}

struct PreparedMcpArguments<T> {
    arguments: T,
    project_id: ProjectId,
}
