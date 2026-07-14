use crate::errors::McpAdapterError;
use crate::prelude::*;
use crate::routing::*;
use crate::schema_validation::validate_mcp_tool_arguments;
use crate::tool_registry::*;
use crate::util::*;
use chrono::{DateTime, SecondsFormat, Utc};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex,
};
use volicord_platform_fs::capture_git_workspace_snapshot;

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
    pub git_workspace_context: Option<GitWorkspaceContext>,
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
        if let Some(workspace) = self.git_workspace_context.as_ref() {
            invocation = invocation.with_git_workspace_context(workspace.clone());
        }
        if let Some(session_id) = self.session_id.as_ref() {
            invocation = invocation.with_session_id(session_id.clone());
        }
        invocation
    }
}

/// Transport capabilities that may make a User Channel available for one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpUserChannelCapabilities {
    pub(crate) host_elicitation_available: bool,
    pub(crate) model_invisible_user_surface: bool,
    pub(crate) launch_origin: &'static str,
    pub(crate) client_name: Option<String>,
    pub(crate) client_version: Option<String>,
}

impl Default for McpUserChannelCapabilities {
    fn default() -> Self {
        Self::new(false, false)
    }
}

impl McpUserChannelCapabilities {
    pub(crate) const fn new(
        host_elicitation_available: bool,
        model_invisible_user_surface: bool,
    ) -> Self {
        Self {
            host_elicitation_available,
            model_invisible_user_surface,
            launch_origin: "unknown",
            client_name: None,
            client_version: None,
        }
    }

    pub(crate) fn with_stdio_session(
        mut self,
        launch_origin: &'static str,
        client_name: Option<&str>,
        client_version: Option<&str>,
    ) -> Self {
        self.launch_origin = launch_origin;
        self.client_name = client_name.map(str::to_owned);
        self.client_version = client_version.map(str::to_owned);
        self
    }
}

/// Loopback consent endpoint facts available to adapter fallback selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWebConsentContext {
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalWebConsentReadiness(Arc<LocalWebConsentReadinessInner>);

#[derive(Debug)]
struct LocalWebConsentReadinessInner {
    ready: AtomicBool,
    issuance_gate: LocalWebConsentIssuanceGate,
}

pub(crate) struct LocalWebConsentIssuanceLease<'a> {
    issuance_gate: &'a LocalWebConsentIssuanceGate,
}

pub(crate) struct LocalWebConsentListenerGuard(LocalWebConsentReadiness);

#[derive(Debug)]
struct LocalWebConsentIssuanceGate {
    state: Mutex<LocalWebConsentIssuanceState>,
    drained: Condvar,
}

#[derive(Debug, Default)]
struct LocalWebConsentIssuanceState {
    active_issuances: usize,
    invalidating: bool,
    drain_waiting: bool,
}

impl LocalWebConsentIssuanceGate {
    fn new() -> Self {
        Self {
            state: Mutex::new(LocalWebConsentIssuanceState::default()),
            drained: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert!(state.active_issuances > 0);
        state.active_issuances -= 1;
        if state.active_issuances == 0 {
            self.drained.notify_all();
        }
    }
}

impl Drop for LocalWebConsentIssuanceLease<'_> {
    fn drop(&mut self) {
        self.issuance_gate.release();
    }
}

impl LocalWebConsentReadiness {
    pub(crate) fn tracked() -> (Self, LocalWebConsentListenerGuard) {
        let readiness = Self(Arc::new(LocalWebConsentReadinessInner {
            ready: AtomicBool::new(true),
            issuance_gate: LocalWebConsentIssuanceGate::new(),
        }));
        let guard = LocalWebConsentListenerGuard(readiness.clone());
        (readiness, guard)
    }

    #[cfg(test)]
    pub(crate) fn ready_for_test() -> Self {
        Self(Arc::new(LocalWebConsentReadinessInner {
            ready: AtomicBool::new(true),
            issuance_gate: LocalWebConsentIssuanceGate::new(),
        }))
    }

    pub(crate) fn is_ready(&self) -> bool {
        !self.0.issuance_gate.state.is_poisoned() && self.0.ready.load(Ordering::Acquire)
    }

    pub(crate) fn acquire_issuance_lease(&self) -> Option<LocalWebConsentIssuanceLease<'_>> {
        if !self.0.ready.load(Ordering::Acquire) {
            return None;
        }
        let mut state = self.0.issuance_gate.state.lock().ok()?;
        if !self.0.ready.load(Ordering::Acquire) || state.invalidating {
            return None;
        }
        state.active_issuances += 1;
        Some(LocalWebConsentIssuanceLease {
            issuance_gate: &self.0.issuance_gate,
        })
    }

    pub(crate) fn mark_unavailable(&self) {
        self.mark_unavailable_with_observers(|| {}, |_| {});
    }

    fn mark_unavailable_with_observers(
        &self,
        after_publish: impl FnOnce(),
        after_drain_attempt: impl FnOnce(bool),
    ) {
        self.0.ready.store(false, Ordering::Release);
        after_publish();
        let mut state = self
            .0
            .issuance_gate
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.invalidating = true;
        state.drain_waiting = state.active_issuances != 0;
        after_drain_attempt(state.drain_waiting);
        while state.active_issuances != 0 {
            state = self
                .0
                .issuance_gate
                .drained
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state.drain_waiting = false;
    }

    #[cfg(test)]
    pub(crate) fn mark_unavailable_with_observers_for_test(
        &self,
        after_publish: impl FnOnce(),
        after_drain_attempt: impl FnOnce(bool),
    ) {
        self.mark_unavailable_with_observers(after_publish, after_drain_attempt);
    }

    #[cfg(test)]
    pub(crate) fn issuance_lease_is_held_for_test(&self) -> bool {
        self.0
            .issuance_gate
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .active_issuances
            != 0
    }

    #[cfg(test)]
    fn drain_state_for_test(&self) -> (bool, usize) {
        let state = self
            .0
            .issuance_gate
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (state.drain_waiting, state.active_issuances)
    }
}

impl LocalWebConsentListenerGuard {
    pub(crate) fn mark_unavailable(&self) {
        self.0.mark_unavailable();
    }
}

impl Drop for LocalWebConsentListenerGuard {
    fn drop(&mut self) {
        self.mark_unavailable();
    }
}

/// Local MCP adapter bound to a Core service and one Agent Connection.
#[derive(Debug, Clone)]
pub struct McpAdapter {
    pub(crate) core: CoreService,
    pub(crate) runtime_home: PathBuf,
    pub(crate) context: McpConnectionContext,
    pub(crate) local_web_consent: Option<LocalWebConsentContext>,
    pub(crate) local_web_consent_readiness: Option<LocalWebConsentReadiness>,
    expected_evidence_artifact_sha256: Option<String>,
}

impl PartialEq for McpAdapter {
    fn eq(&self, other: &Self) -> bool {
        self.core == other.core
            && self.runtime_home == other.runtime_home
            && self.context == other.context
            && self.local_web_consent == other.local_web_consent
            && self.expected_evidence_artifact_sha256 == other.expected_evidence_artifact_sha256
    }
}

impl Eq for McpAdapter {}

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
            local_web_consent_readiness: None,
            expected_evidence_artifact_sha256: None,
        }
    }

    /// Retained for source compatibility with untracked callers; the
    /// fail-closed behavior is owned by the MCP Transport reference.
    #[deprecated(
        since = "0.8.0",
        note = "untracked base URLs no longer enable local web; use a supported process entry point"
    )]
    pub fn with_local_web_consent(mut self, context: LocalWebConsentContext) -> Self {
        self.local_web_consent = Some(context);
        self.local_web_consent_readiness = None;
        self
    }

    pub(crate) fn with_local_web_consent_readiness(
        mut self,
        context: LocalWebConsentContext,
        readiness: LocalWebConsentReadiness,
    ) -> Self {
        self.local_web_consent = Some(context);
        self.local_web_consent_readiness = Some(readiness);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_expected_evidence_artifact_sha256_for_test(
        mut self,
        evidence_artifact_sha256: impl Into<String>,
    ) -> Self {
        self.expected_evidence_artifact_sha256 = Some(evidence_artifact_sha256.into());
        self
    }

    pub(crate) fn effective_local_web_consent_available(
        &self,
        capabilities: &McpUserChannelCapabilities,
    ) -> bool {
        capabilities.model_invisible_user_surface
            && capabilities.launch_origin == "managed_host"
            && self.local_web_consent_listener_ready()
            && self.current_host_capability_verification_matches(capabilities)
    }

    fn current_host_capability_verification_matches(
        &self,
        capabilities: &McpUserChannelCapabilities,
    ) -> bool {
        let Some(evidence_artifact_sha256) = self.expected_evidence_artifact_sha256.as_deref()
        else {
            return false;
        };
        let Some(client_name) = capabilities
            .client_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return false;
        };
        let Some(client_version) = capabilities
            .client_version
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return false;
        };
        let Ok(Some(connection)) = agent_connection_record_read_only(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
        ) else {
            return false;
        };
        if !connection.enabled || connection.host_kind == "generic" {
            return false;
        }
        let Some(executable_sha256) = crate::build_info::current_executable_sha256() else {
            return false;
        };
        let build = crate::build_info();
        let now =
            DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Nanos, true);
        let expectation = HostCapabilityVerificationExpectation {
            connection_internal_id: connection.connection_internal_id,
            capability: HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE.to_owned(),
            host_kind: connection.host_kind,
            host_version: client_version.to_owned(),
            client_name: client_name.to_owned(),
            client_version: client_version.to_owned(),
            adapter_profile: HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1.to_owned(),
            adapter_version: build.package_version.to_owned(),
            managed_fingerprint: connection.managed_fingerprint,
            volicord_build_id: build.build_id,
            source_revision: build.git_commit.to_owned(),
            target_triple: build.target_triple.to_owned(),
            executable_sha256: executable_sha256.to_owned(),
            evidence_artifact_sha256: evidence_artifact_sha256.to_owned(),
        };
        matches!(
            evaluate_current_host_capability_verification_read_only(
                &self.runtime_home,
                &expectation,
                &now,
            ),
            Ok(Some(_))
        )
    }

    pub(crate) fn local_web_consent_listener_ready(&self) -> bool {
        self.local_web_consent.is_some()
            && self
                .local_web_consent_readiness
                .as_ref()
                .is_some_and(LocalWebConsentReadiness::is_ready)
    }

    pub(crate) fn local_web_consent_issuance_lease(
        &self,
        capabilities: &McpUserChannelCapabilities,
    ) -> Option<LocalWebConsentIssuanceLease<'_>> {
        if !capabilities.model_invisible_user_surface
            || capabilities.launch_origin != "managed_host"
            || self.local_web_consent.is_none()
        {
            return None;
        }
        let lease = self
            .local_web_consent_readiness
            .as_ref()?
            .acquire_issuance_lease()?;
        self.current_host_capability_verification_matches(capabilities)
            .then_some(lease)
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
    ) -> Result<McpDerivedInvocationContext, McpAdapterError> {
        self.derive_invocation_context_with_user_channel_capabilities(
            envelope,
            operation_category,
            session_id,
            McpUserChannelCapabilities::new(host_elicitation_available, false),
        )
    }

    fn derive_invocation_context_with_user_channel_capabilities(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<McpDerivedInvocationContext, McpAdapterError> {
        let store = CoreProjectStore::open(&self.runtime_home, &envelope.project_id)
            .map_err(McpAdapterError::Store)?;
        let git_workspace_context =
            capture_git_workspace_snapshot(&store.project_record().repo_root)
                .map_err(|error| {
                    McpAdapterError::Environment(format!(
                "failed to capture the selected Product Repository Git workspace context: {error}"
            ))
                })?
                .map(|snapshot| GitWorkspaceContext {
                    git_common_dir: snapshot.layout.common_dir.display().to_string(),
                    worktree_id: snapshot.worktree_id,
                    branch_ref: snapshot.branch_ref,
                    head_sha: snapshot.head_sha,
                    workspace_fingerprint: snapshot.workspace_fingerprint,
                });
        Ok(McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(
                self.context.connection_internal_id.clone(),
            ),
            operation_category,
            invocation_binding_basis: self.context.invocation_binding_basis.clone(),
            session_id: session_id.map(str::to_owned),
            host_elicitation_available: capabilities.host_elicitation_available,
            local_web_consent_available: self.effective_local_web_consent_available(&capabilities),
            git_workspace_context,
        })
    }

    fn derive_read_only_invocation_context_with_user_channel_capabilities(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<McpDerivedInvocationContext, McpAdapterError> {
        let store = CoreProjectStore::open_read_only(&self.runtime_home, &envelope.project_id)
            .map_err(McpAdapterError::Store)?;
        let git_workspace_context =
            capture_git_workspace_snapshot(&store.project_record().repo_root)
                .map_err(|error| {
                    McpAdapterError::Environment(format!(
                "failed to capture the selected Product Repository Git workspace context: {error}"
            ))
                })?
                .map(|snapshot| GitWorkspaceContext {
                    git_common_dir: snapshot.layout.common_dir.display().to_string(),
                    worktree_id: snapshot.worktree_id,
                    branch_ref: snapshot.branch_ref,
                    head_sha: snapshot.head_sha,
                    workspace_fingerprint: snapshot.workspace_fingerprint,
                });
        Ok(McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(
                self.context.connection_internal_id.clone(),
            ),
            operation_category,
            invocation_binding_basis: self.context.invocation_binding_basis.clone(),
            session_id: session_id.map(str::to_owned),
            host_elicitation_available: capabilities.host_elicitation_available,
            local_web_consent_available: self.effective_local_web_consent_available(&capabilities),
            git_workspace_context,
        })
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
        self.call_tool_for_session_with_user_channel_capabilities(
            tool_name,
            params,
            session_id,
            McpUserChannelCapabilities::new(host_elicitation_available, false),
        )
    }

    pub(crate) fn call_tool_for_session_with_user_channel_capabilities(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<PipelineResponse, McpAdapterError> {
        validate_mcp_tool_arguments(tool_name, &params)?;
        match tool_name {
            INTAKE_TOOL_NAME => self.call_intake(tool_name, params, session_id, capabilities),
            UPDATE_SCOPE_TOOL_NAME => {
                self.call_update_scope(tool_name, params, session_id, capabilities)
            }
            STATUS_TOOL_NAME => self.call_status(tool_name, params, session_id, capabilities),
            GET_OPERATION_RESULT_TOOL_NAME => {
                self.call_get_operation_result(tool_name, params, session_id, capabilities)
            }
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME => {
                self.call_prepare_evidence_capture(tool_name, params, session_id, capabilities)
            }
            PREPARE_WRITE_TOOL_NAME => {
                self.call_prepare_write(tool_name, params, session_id, capabilities)
            }
            STAGE_ARTIFACT_TOOL_NAME => {
                self.call_stage_artifact(tool_name, params, session_id, capabilities)
            }
            RECORD_RUN_TOOL_NAME => {
                self.call_record_run(tool_name, params, session_id, capabilities)
            }
            REQUEST_USER_ACTION_TOOL_NAME => {
                self.call_request_user_action(tool_name, params, session_id, capabilities)
            }
            RECONCILE_CHANGES_TOOL_NAME => {
                self.call_reconcile_changes(tool_name, params, session_id, capabilities)
            }
            CHECK_CLOSE_TOOL_NAME => {
                self.call_check_close(tool_name, params, session_id, capabilities)
            }
            CLOSE_TASK_TOOL_NAME => {
                self.call_close_task(tool_name, params, session_id, capabilities)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn call_intake(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
                acceptance_policy: args.acceptance_policy,
                lineage: args.lineage,
                initial_scope: args.initial_scope,
                initial_context_refs: args.initial_context_refs,
                initial_source_refs: args.initial_source_refs,
            },
            CoreService::intake,
            session_id,
            capabilities,
        )
    }

    fn call_update_scope(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
        )
    }

    fn call_status(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
        )
    }

    fn call_get_operation_result(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpGetOperationResultArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            None,
            OperationCategory::Read,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            tool_name,
            GetOperationResultRequest {
                envelope,
                operation_result_ref: args.operation_result_ref,
                cursor: args.cursor,
            },
            CoreService::get_operation_result,
            session_id,
            capabilities,
        )
    }

    pub(crate) fn refresh_authority_status_with_user_channel_capabilities(
        &self,
        project_id: &ProjectId,
        task_id: &TaskId,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let envelope = self.generated_envelope(
            STATUS_TOOL_NAME,
            project_id,
            Some(task_id),
            OperationCategory::Read,
        )?;
        self.call_core_request(
            STATUS_TOOL_NAME,
            StatusRequest {
                envelope,
                include: StatusDetailLevel::Workflow.include(),
            },
            CoreService::status,
            session_id,
            capabilities,
        )
    }

    pub(crate) fn user_channel_inbox_projection(
        &self,
        project_id: &ProjectId,
        task_id: &TaskId,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<Option<volicord_core::UserChannelInboxProjection>, McpAdapterError> {
        let mut invocation = InvocationContext::new(
            project_id.clone(),
            ActorSource::agent_connection(self.context.connection_internal_id.clone()),
            OperationCategory::Read,
            self.context.invocation_binding_basis.clone(),
        )
        .with_host_elicitation_available(capabilities.host_elicitation_available)
        .with_local_web_consent_available(
            self.effective_local_web_consent_available(&capabilities),
        );
        if let Some(session_id) = session_id {
            invocation = invocation.with_session_id(session_id.to_owned());
        }
        self.core
            .user_channel_inbox_projection(
                volicord_core::UserChannelInboxProjectionRequest {
                    project_id: project_id.clone(),
                    task_id: task_id.clone(),
                },
                invocation,
            )
            .map_err(McpAdapterError::Core)
    }

    fn call_prepare_evidence_capture(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareEvidenceCaptureArguments> =
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
            PrepareEvidenceCaptureRequest {
                envelope,
                task_id,
                change_unit_id: args.change_unit_id,
                baseline_ref: args.baseline_ref,
                target: args.target,
                capture: args.capture.into(),
            },
            CoreService::prepare_evidence_capture,
            session_id,
            capabilities,
        )
    }

    fn call_prepare_write(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
        )
    }

    fn call_stage_artifact(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
        )
    }

    fn call_record_run(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
                evidence_updates: args.evidence_updates.into_iter().map(Into::into).collect(),
                evidence_observations: args
                    .evidence_observations
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                close_assessment: args.close_assessment,
            },
            CoreService::record_run,
            session_id,
            capabilities,
        )
    }

    fn call_request_user_action(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRequestUserActionArguments> =
            self.prepare_mcp_arguments(tool_name, params, session_id)?;
        match prepared.arguments.request {
            McpRequestUserActionOperation::Create {
                task_id,
                change_unit_id,
                action,
                required_for,
                expires_at,
            } => {
                let envelope = self.generated_envelope(
                    tool_name,
                    &prepared.project_id,
                    Some(&task_id),
                    OperationCategory::AgentWorkflow,
                )?;
                self.call_core_request(
                    tool_name,
                    RequestUserActionRequest {
                        envelope,
                        task_id,
                        change_unit_id,
                        action,
                        required_for,
                        expires_at,
                    },
                    CoreService::request_user_action,
                    session_id,
                    capabilities,
                )
            }
            McpRequestUserActionOperation::Resume {
                user_action_request_id,
            } => {
                self.ensure_mode_allows(tool_name, OperationCategory::AgentWorkflow)?;
                let envelope = ToolEnvelope {
                    project_id: prepared.project_id.clone(),
                    task_id: RequiredNullable::null(),
                    request_id: RequestId::new("req_internal_user_action_resume"),
                    idempotency_key: RequiredNullable::null(),
                    expected_state_version: RequiredNullable::null(),
                    dry_run: false,
                    locale: RequiredNullable::null(),
                };
                let invocation = self
                    .derive_read_only_invocation_context_with_user_channel_capabilities(
                        &envelope,
                        OperationCategory::AgentWorkflow,
                        session_id,
                        capabilities,
                    )?;
                self.core
                    .resume_user_action_request(
                        prepared.project_id,
                        user_action_request_id,
                        invocation.core_invocation(),
                    )
                    .map_err(McpAdapterError::Core)?
                    .ok_or_else(|| McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: "the resumed user-action request is unavailable or was created by another Agent Connection".to_owned(),
                    })
            }
        }
    }

    fn call_reconcile_changes(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
        )
    }

    fn call_check_close(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
        )
    }

    fn call_close_task(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
        capabilities: McpUserChannelCapabilities,
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
            capabilities,
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
            operation_result_ref: None,
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
        capabilities: McpUserChannelCapabilities,
    ) -> Result<PipelineResponse, McpAdapterError>
    where
        T: MethodOperationCategory + HasEnvelope,
        F: FnOnce(
            &CoreService,
            T,
            InvocationContext,
        ) -> Result<PipelineResponse, CorePipelineError>,
    {
        if let Some(response) = self.readonly_storage_rejection_for_tool(tool_name)? {
            return Ok(response);
        }
        let operation_category = request.operation_category();
        self.ensure_mode_allows(tool_name, operation_category)?;
        let invocation = self.derive_invocation_context_with_user_channel_capabilities(
            request_envelope(&request),
            operation_category,
            session_id,
            capabilities,
        )?;
        call(&self.core, request, invocation.core_invocation()).map_err(McpAdapterError::Core)
    }

    pub(crate) fn call_adapter_tool(
        &self,
        tool_name: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value, McpAdapterError> {
        validate_mcp_tool_arguments(tool_name, &params)?;
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
        let arguments = self.decode_params(tool_name, params)?;
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
                    "project selector {project_id} is outside this MCP transport project allowlist"
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
        let diagnostic_params = params.clone();
        serde_json::from_value(params).map_err(|source| {
            let guidance = invalid_argument_guidance(tool_name, &diagnostic_params, &source);
            let message = match guidance {
                Some(guidance) => format!(
                    "Invalid arguments for {tool_name} {guidance}. Decoder detail: {source}."
                ),
                None => format!(
                    "Invalid arguments for {tool_name}: {source}. Check the tool input schema and retry."
                ),
            };
            McpAdapterError::InvalidParams {
                tool_name: tool_name.to_owned(),
                issues: vec![McpToolErrorIssue {
                    path: String::new(),
                    code: McpToolIssueCode::ArgumentDecodeFailed,
                    message,
                }],
                truncated: false,
                source: Some(source),
            }
        })
    }
}

fn invalid_argument_guidance(
    tool_name: &str,
    params: &Value,
    source: &serde_json::Error,
) -> Option<String> {
    let source_text = source.to_string();
    match tool_name {
        RECORD_RUN_TOOL_NAME => record_run_invalid_argument_guidance(params, &source_text),
        REQUEST_USER_ACTION_TOOL_NAME => {
            request_user_action_invalid_argument_guidance(params, &source_text)
        }
        UPDATE_SCOPE_TOOL_NAME => update_scope_invalid_argument_guidance(params, &source_text),
        PREPARE_WRITE_TOOL_NAME => prepare_write_invalid_argument_guidance(params, &source_text),
        STATUS_TOOL_NAME => status_invalid_argument_guidance(params, &source_text),
        CHECK_CLOSE_TOOL_NAME => check_close_invalid_argument_guidance(params, &source_text),
        _ => None,
    }
}

fn record_run_invalid_argument_guidance(params: &Value, source: &str) -> Option<String> {
    object_shape_guidance(
        params.get("observed_changes"),
        "observed_changes",
        &[
            "changed_paths",
            "product_file_write_observed",
            "sensitive_categories",
            "baseline_ref",
        ],
        r#"{"changed_paths":[],"product_file_write_observed":false,"sensitive_categories":[],"baseline_ref":"baseline_001"}"#,
    )
    .or_else(|| {
        array_item_shape_guidance(
            params.get("artifact_inputs"),
            "artifact_inputs",
            &[
                "artifact_input_id",
                "source_kind",
                "staged_artifact_handle",
                "existing_artifact_ref",
                "relation_hint",
                "evidence_target",
                "expected_sha256",
                "expected_size_bytes",
                "redaction_state",
            ],
            r#"{"artifact_input_id":"artifact_input_001","source_kind":"existing_artifact","staged_artifact_handle":null,"existing_artifact_ref":null,"relation_hint":null,"evidence_target":null,"expected_sha256":null,"expected_size_bytes":null,"redaction_state":null}"#,
        )
    })
    .or_else(|| {
        array_item_shape_guidance(
            params.get("evidence_observations"),
            "evidence_observations",
            &[
                "target",
                "source_kind",
                "assurance_level",
                "observed_by_actor_source",
                "tool_name",
                "tool_invocation_id",
                "tool_metadata",
                "input_refs",
                "source_refs",
                "output_artifact_refs",
                "limitations",
                "observed_at",
            ],
            r#"{"target":{"target_kind":"acceptance_criterion","acceptance_criterion_id":"criterion_001"},"source_kind":"agent_report","assurance_level":"cooperative_report","observed_by_actor_source":null,"tool_name":null,"tool_invocation_id":null,"tool_metadata":{},"input_refs":[],"source_refs":[],"output_artifact_refs":[],"limitations":[],"observed_at":"2026-06-18T00:00:00Z"}"#,
        )
    })
    .or_else(|| {
        string_value_guidance(
            params,
            "kind",
            &["shaping_update", "implementation", "direct"],
        )
    })
    .or_else(|| root_shape_guidance_for_source(params, source, record_run_root_fields(), crate::tool_registry::RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_ARGUMENTS_JSON))
}

fn request_user_action_invalid_argument_guidance(params: &Value, source: &str) -> Option<String> {
    let request = params.get("request").unwrap_or(&Value::Null);
    string_value_guidance(request, "operation", &["create", "resume"])
        .or_else(|| options_shape_guidance(request.get("action").unwrap_or(&Value::Null)))
        .or_else(|| {
            object_shape_guidance(
                params.pointer("/request/action/context"),
                "request.action.context",
                &[
                    "summary",
                    "related_refs",
                    "artifact_refs",
                    "visible_risks",
                    "constraints",
                ],
                r#"{"summary":"User-visible context.","related_refs":[],"artifact_refs":[],"visible_risks":[],"constraints":[]}"#,
            )
        })
        .or_else(|| {
            array_item_shape_guidance(
                params.pointer("/request/action/context/visible_risks"),
                "request.action.context.visible_risks",
                &[
                    "risk_id",
                    "summary",
                    "consequence",
                    "related_refs",
                    "accepted_for_close",
                ],
                r#"{"risk_id":"risk_001","summary":"Known risk.","consequence":"User-visible consequence.","related_refs":[],"accepted_for_close":false}"#,
            )
        })
        .or_else(|| {
            array_item_shape_guidance(
                params.pointer("/request/action/affected_refs"),
                "request.action.affected_refs",
                state_record_ref_fields(),
                state_record_ref_skeleton(),
            )
        })
        .or_else(|| {
            string_value_guidance(
                request.get("action").unwrap_or(&Value::Null),
                "judgment_kind",
                &[
                    "product_decision",
                    "technical_decision",
                    "scope_decision",
                    "sensitive_approval",
                    "final_acceptance",
                    "residual_risk_acceptance",
                    "cancellation",
                ],
            )
        })
        .or_else(|| {
            string_value_guidance(
                request.get("action").unwrap_or(&Value::Null),
                "action_type",
                &["choice", "evidence_observation"],
            )
        })
        .or_else(|| {
            string_value_guidance(
                request.get("action").unwrap_or(&Value::Null),
                "presentation",
                &["short"],
            )
        })
        .or_else(|| {
            array_string_values_guidance(
                request.get("required_for"),
                "request.required_for",
                &[
                    "scope_update",
                    "prepare_write",
                    "record_run",
                    "close_complete",
                    "close_cancel",
                    "close_supersede",
                    "informational",
                ],
            )
        })
        .or_else(|| root_shape_guidance_for_source(params, source, request_user_action_root_fields(), crate::tool_registry::REQUEST_USER_ACTION_FINAL_ACCEPTANCE_ARGUMENTS_JSON))
}

fn update_scope_invalid_argument_guidance(params: &Value, source: &str) -> Option<String> {
    object_shape_guidance(
        params.get("change_unit"),
        "change_unit",
        &["operation"],
        r#"{"operation":"keep_current"}"#,
    )
    .or_else(|| {
        params.pointer("/change_unit/operation").and_then(|_| {
            nested_string_value_guidance(
                params,
                "/change_unit/operation",
                "change_unit.operation",
                &["keep_current", "create_current", "replace_current"],
            )
        })
    })
    .or_else(|| {
        root_shape_guidance_for_source(
            params,
            source,
            update_scope_root_fields(),
            crate::tool_registry::UPDATE_SCOPE_KEEP_CURRENT_ARGUMENTS_JSON,
        )
    })
}

fn prepare_write_invalid_argument_guidance(params: &Value, source: &str) -> Option<String> {
    root_shape_guidance_for_source(
        params,
        source,
        prepare_write_root_fields(),
        crate::tool_registry::PREPARE_WRITE_SIMPLE_ARGUMENTS_JSON,
    )
}

fn status_invalid_argument_guidance(params: &Value, source: &str) -> Option<String> {
    string_value_guidance(params, "detail", &["summary", "workflow", "full"]).or_else(|| {
        root_shape_guidance_for_source(
            params,
            source,
            status_root_fields(),
            crate::tool_registry::STATUS_READ_ONLY_ARGUMENTS_JSON,
        )
    })
}

fn check_close_invalid_argument_guidance(params: &Value, source: &str) -> Option<String> {
    root_shape_guidance_for_source(
        params,
        source,
        check_close_root_fields(),
        crate::tool_registry::CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_ARGUMENTS_JSON,
    )
}

fn options_shape_guidance(params: &Value) -> Option<String> {
    let options = params.get("options")?;
    match options {
        Value::Null => None,
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let path = format!("options[{index}]");
                let Some(object) = item.as_object() else {
                    return Some(format!(
                        "at {path}: expected object with fields {}. Valid skeleton: {}",
                        field_set(option_input_fields()),
                        option_input_skeleton()
                    ));
                };
                if object.contains_key("id") && !object.contains_key("option_id") {
                    return Some(format!(
                        "at {path}: expected option_id, not id. Expected object with fields {}. Valid skeleton: {}",
                        field_set(option_input_fields()),
                        option_input_skeleton()
                    ));
                }
                if let Some(message) = object_field_problem(
                    object,
                    &path,
                    option_input_fields(),
                    option_input_skeleton(),
                ) {
                    return Some(message);
                }
            }
            None
        }
        _ => Some(format!(
            "at options: expected null or an array of objects with fields {}. Valid skeleton: [{}]",
            field_set(option_input_fields()),
            option_input_skeleton()
        )),
    }
}

fn object_shape_guidance(
    value: Option<&Value>,
    path: &str,
    fields: &[&str],
    skeleton: &str,
) -> Option<String> {
    match value {
        None => Some(format!(
            "at {path}: missing required object. Expected fields {}. Valid skeleton: {skeleton}",
            field_set(fields)
        )),
        Some(Value::Object(object)) => object_field_problem(object, path, fields, skeleton),
        Some(_) => Some(format!(
            "at {path}: expected object with fields {}. Valid skeleton: {skeleton}",
            field_set(fields)
        )),
    }
}

fn array_item_shape_guidance(
    value: Option<&Value>,
    path: &str,
    fields: &[&str],
    skeleton: &str,
) -> Option<String> {
    let value = value?;
    let Value::Array(items) = value else {
        return Some(format!(
            "at {path}: expected array of objects with fields {}. Valid item skeleton: {skeleton}",
            field_set(fields)
        ));
    };
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let Some(object) = item.as_object() else {
            return Some(format!(
                "at {item_path}: expected object with fields {}. Valid skeleton: {skeleton}",
                field_set(fields)
            ));
        };
        if let Some(message) = object_field_problem(object, &item_path, fields, skeleton) {
            return Some(message);
        }
    }
    None
}

fn object_field_problem(
    object: &Map<String, Value>,
    path: &str,
    fields: &[&str],
    skeleton: &str,
) -> Option<String> {
    let missing = fields
        .iter()
        .copied()
        .filter(|field| !object.contains_key(*field))
        .collect::<Vec<_>>();
    let unknown = object
        .keys()
        .filter(|field| !fields.contains(&field.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if missing.is_empty() && unknown.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    if !missing.is_empty() {
        parts.push(format!("missing {}", missing.join(", ")));
    }
    if !unknown.is_empty() {
        parts.push(format!("unknown {}", unknown.join(", ")));
    }
    Some(format!(
        "at {path}: {}. Expected fields {}. Valid skeleton: {skeleton}",
        parts.join("; "),
        field_set(fields)
    ))
}

fn root_shape_guidance_for_source(
    params: &Value,
    source: &str,
    fields: &[&str],
    skeleton: &str,
) -> Option<String> {
    if !source.contains("missing field") && !source.contains("unknown field") {
        return None;
    }
    object_shape_guidance(Some(params), "arguments", fields, skeleton)
}

fn string_value_guidance(params: &Value, field: &str, allowed: &[&str]) -> Option<String> {
    let Some(Value::String(value)) = params.get(field) else {
        return None;
    };
    if allowed.contains(&value.as_str()) {
        return None;
    }
    Some(format!(
        "at {field}: unsupported value `{value}`; expected one of {}",
        value_set(allowed)
    ))
}

fn nested_string_value_guidance(
    params: &Value,
    pointer: &str,
    path: &str,
    allowed: &[&str],
) -> Option<String> {
    let Some(Value::String(value)) = params.pointer(pointer) else {
        return None;
    };
    if allowed.contains(&value.as_str()) {
        return None;
    }
    Some(format!(
        "at {path}: unsupported value `{value}`; expected one of {}",
        value_set(allowed)
    ))
}

fn array_string_values_guidance(
    value: Option<&Value>,
    path: &str,
    allowed: &[&str],
) -> Option<String> {
    let Some(Value::Array(items)) = value else {
        return None;
    };
    for (index, item) in items.iter().enumerate() {
        let Some(value) = item.as_str() else {
            return Some(format!(
                "at {path}[{index}]: expected string value from {}",
                value_set(allowed)
            ));
        };
        if !allowed.contains(&value) {
            return Some(format!(
                "at {path}[{index}]: unsupported value `{value}`; expected one of {}",
                value_set(allowed)
            ));
        }
    }
    None
}

fn field_set(fields: &[&str]) -> String {
    format!("{{ {} }}", fields.join(", "))
}

fn value_set(values: &[&str]) -> String {
    format!("{{ {} }}", values.join(", "))
}

fn record_run_root_fields() -> &'static [&'static str] {
    &[
        "project_selector",
        "task_id",
        "change_unit_id",
        "kind",
        "run_id",
        "baseline_ref",
        "write_ticket_id",
        "summary",
        "observed_changes",
        "artifact_inputs",
        "evidence_updates",
        "evidence_observations",
        "close_assessment",
    ]
}

fn request_user_action_root_fields() -> &'static [&'static str] {
    &["project_selector", "detail", "request"]
}

fn update_scope_root_fields() -> &'static [&'static str] {
    &[
        "project_selector",
        "task_id",
        "goal_summary",
        "scope_update",
        "scope_boundary",
        "non_goals",
        "acceptance_criteria",
        "autonomy_boundary",
        "baseline_ref",
        "change_unit",
        "related_scope_decision_refs",
    ]
}

fn prepare_write_root_fields() -> &'static [&'static str] {
    &[
        "project_selector",
        "task_id",
        "change_unit_id",
        "intended_operation",
        "intended_paths",
        "product_file_write_intended",
        "sensitive_categories",
        "baseline_ref",
    ]
}

fn status_root_fields() -> &'static [&'static str] {
    &["project_selector", "task_id", "detail"]
}

fn check_close_root_fields() -> &'static [&'static str] {
    &["project_selector", "task_id"]
}

fn option_input_fields() -> &'static [&'static str] {
    &[
        "option_id",
        "label",
        "description",
        "consequence",
        "is_default",
    ]
}

fn option_input_skeleton() -> &'static str {
    r#"{"option_id":"accept","label":"Accept","description":"Record the user's selected option.","consequence":"The option is recorded for this judgment.","is_default":true}"#
}

fn state_record_ref_fields() -> &'static [&'static str] {
    &[
        "record_kind",
        "record_id",
        "project_id",
        "task_id",
        "produced_at_state_version",
    ]
}

fn state_record_ref_skeleton() -> &'static str {
    r#"{"record_kind":"task","record_id":"task_001","project_id":"proj_001","task_id":"task_001","produced_at_state_version":1}"#
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
    GetOperationResultRequest,
    PrepareEvidenceCaptureRequest,
    PrepareWriteRequest,
    StageArtifactRequest,
    RecordRunRequest,
    RequestUserActionRequest,
    ReconcileChangesRequest,
    CheckCloseRequest,
    CloseTaskRequest,
);

fn request_envelope<T: HasEnvelope>(request: &T) -> &ToolEnvelope {
    request.envelope()
}

fn public_tool_operation_category(tool_name: &str) -> Option<OperationCategory> {
    match tool_name {
        STATUS_TOOL_NAME | GET_OPERATION_RESULT_TOOL_NAME | CHECK_CLOSE_TOOL_NAME => {
            Some(OperationCategory::Read)
        }
        INTAKE_TOOL_NAME
        | UPDATE_SCOPE_TOOL_NAME
        | PREPARE_EVIDENCE_CAPTURE_TOOL_NAME
        | PREPARE_WRITE_TOOL_NAME
        | STAGE_ARTIFACT_TOOL_NAME
        | RECORD_RUN_TOOL_NAME
        | REQUEST_USER_ACTION_TOOL_NAME
        | RECONCILE_CHANGES_TOOL_NAME
        | CLOSE_TASK_TOOL_NAME => Some(OperationCategory::AgentWorkflow),
        _ => None,
    }
}

struct PreparedMcpArguments<T> {
    arguments: T,
    project_id: ProjectId,
}

#[cfg(test)]
mod local_web_readiness_tests {
    use super::*;
    use std::sync::mpsc::{self, TryRecvError};

    #[test]
    fn invalidation_publishes_unavailable_then_drains_granted_issuance() {
        let readiness = LocalWebConsentReadiness::ready_for_test();
        let lease = readiness
            .acquire_issuance_lease()
            .expect("ready listener must grant an issuance lease");
        let worker_readiness = readiness.clone();
        let (published_tx, published_rx) = mpsc::channel();
        let (drain_attempt_tx, drain_attempt_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let invalidator = thread::spawn(move || {
            worker_readiness.mark_unavailable_with_observers_for_test(
                || {
                    published_tx
                        .send(())
                        .expect("test receiver must observe readiness publication");
                },
                |blocked_by_granted_issuance| {
                    drain_attempt_tx
                        .send(blocked_by_granted_issuance)
                        .expect("test receiver must observe the drain attempt");
                },
            );
            completed_tx
                .send(())
                .expect("test receiver must observe completed invalidation");
        });

        published_rx
            .recv()
            .expect("invalidation must publish unavailable before draining");
        assert!(!readiness.is_ready());
        assert!(readiness.acquire_issuance_lease().is_none());
        assert!(
            drain_attempt_rx
                .recv()
                .expect("invalidation must attempt to drain the issuance gate"),
            "the granted issuance lease must block invalidation's drain attempt"
        );
        assert_eq!(readiness.drain_state_for_test(), (true, 1));
        assert_eq!(completed_rx.try_recv(), Err(TryRecvError::Empty));

        drop(lease);
        completed_rx
            .recv()
            .expect("invalidation must complete after granted issuance drains");
        invalidator
            .join()
            .expect("invalidation worker must not panic");
        assert!(!readiness.is_ready());
        assert!(readiness.acquire_issuance_lease().is_none());
    }
}
