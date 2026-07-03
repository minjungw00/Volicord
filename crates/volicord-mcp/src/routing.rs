use crate::errors::McpAdapterError;
use crate::prelude::*;
use crate::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionContext {
    pub runtime_home: PathBuf,
    pub connection_internal_id: AgentConnectionId,
    pub mode: AgentConnectionMode,
    pub invocation_binding_basis: String,
    pub project_allowlist: Option<Vec<ProjectId>>,
}

impl McpConnectionContext {
    /// Resolves and validates one Agent Connection startup binding.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        connection_id: impl Into<String>,
    ) -> Result<Self, McpAdapterError> {
        let connection_internal_id = connection_id.into();
        let (context, _, _) = resolve_connection_context(runtime_home, &connection_internal_id)?;
        Ok(context)
    }

    /// Replaces the controlled adapter-binding basis carried into Core.
    pub fn with_invocation_binding_basis(mut self, basis: impl Into<String>) -> Self {
        let basis = basis.into();
        self.invocation_binding_basis = controlled_invocation_binding_basis(&basis).to_owned();
        self
    }

    /// Narrows this adapter context to a transport-owned project allowlist.
    pub fn with_project_allowlist(mut self, project_ids: Vec<ProjectId>) -> Self {
        if !project_ids.is_empty() {
            self.project_allowlist = Some(unique_project_ids(project_ids));
        }
        self
    }

    pub(crate) fn project_allowlist_allows(&self, project_id: &str) -> bool {
        self.project_allowlist
            .as_ref()
            .is_none_or(|project_ids| project_ids.iter().any(|id| id.as_str() == project_id))
    }
}

/// Connection-bound startup facts shared by stdio startup and preflight checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionStartupInspection {
    pub runtime_home: PathBuf,
    pub connection_internal_id: AgentConnectionId,
    pub mode: AgentConnectionMode,
    pub enabled: bool,
    pub allowed_project_count: usize,
    pub projects: Vec<McpProjectAvailability>,
}

impl McpConnectionStartupInspection {
    /// Resolves process inputs and validates one Agent Connection MCP binding.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        connection_id: impl Into<String>,
        detail_project_id: Option<ProjectId>,
    ) -> Result<Self, McpAdapterError> {
        let connection_internal_id = connection_id.into();
        let (context, connection, projects) =
            resolve_connection_context(runtime_home, &connection_internal_id)?;
        let selected_projects = if let Some(project_id) = detail_project_id {
            if !projects
                .iter()
                .any(|project| project.project_id == project_id.as_str())
            {
                return Err(McpAdapterError::Environment(format!(
                    "project {} is outside connection {} project allowlist",
                    project_id.as_str(),
                    connection.connection_internal_id
                )));
            }
            projects
                .iter()
                .filter(|project| project.project_id == project_id.as_str())
                .cloned()
                .collect::<Vec<_>>()
        } else {
            projects.clone()
        };
        let project_reports = selected_projects
            .iter()
            .map(|project| inspect_allowed_project(&context.runtime_home, project))
            .collect::<Vec<_>>();

        Ok(Self {
            runtime_home: context.runtime_home.clone(),
            connection_internal_id: context.connection_internal_id,
            mode: context.mode,
            enabled: connection.enabled,
            allowed_project_count: projects.len(),
            projects: project_reports,
        })
    }

    /// Returns the public connection context consumed by the stdio adapter.
    pub fn connection_context(&self) -> McpConnectionContext {
        McpConnectionContext {
            runtime_home: self.runtime_home.clone(),
            connection_internal_id: self.connection_internal_id.clone(),
            mode: self.mode,
            invocation_binding_basis: DEFAULT_INVOCATION_BINDING_BASIS.to_owned(),
            project_allowlist: None,
        }
    }

    /// Formats the deterministic operator preflight report.
    pub fn preflight_report(&self) -> String {
        let available_projects = self
            .projects
            .iter()
            .filter(|project| project.available)
            .count();
        let (watcher_status, watcher_coverage_basis, watcher_partial_coverage_warning) =
            if available_projects == 1 {
                ("pending_mcp_start", "mcp_start", "")
            } else if available_projects > 1 {
                (
                    "pending_project_selection",
                    "",
                    "project_selector is required before session-watch coverage can start",
                )
            } else {
                (
                    "unavailable",
                    "",
                    "no available project is ready for session-watch coverage",
                )
            };
        let mut report = format!(
            "configuration: valid\ntransport: stdio\n{}\nruntime_home: {}\nconnection_id: {}\nmode: {}\nenabled: {}\nallowed_projects: {}\navailable_projects: {}\nverification_scope: startup_check_only\nwatcher_status: {}\nwatcher_baseline_created_at: \nwatcher_coverage_start_at: \nwatcher_coverage_basis: {}\nwatcher_partial_coverage_warning: {}\n",
            TRANSPORT_DISCLOSURE_TEXT,
            self.runtime_home.display(),
            self.connection_internal_id.as_str(),
            self.mode.as_str(),
            self.enabled,
            self.allowed_project_count,
            available_projects,
            watcher_status,
            watcher_coverage_basis,
            watcher_partial_coverage_warning
        );
        for (index, project) in self.projects.iter().enumerate() {
            report.push_str(&format!(
                "project[{index}].project_id: {}\nproject[{index}].available: {}\nproject[{index}].unavailable_reason: {}\nproject[{index}].repo_root: {}\n",
                project.project_id,
                project.available,
                project.unavailable_reason.as_deref().unwrap_or(""),
                project.repo_root_display
            ));
        }
        report
    }
}

/// MCP-visible availability facts for one connection-allowed project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpProjectAvailability {
    pub project_id: String,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub repo_root_display: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ListProjectsResult {
    pub(crate) connection_id: String,
    pub(crate) mode: AgentConnectionMode,
    pub(crate) watcher_status: SessionWatchStatus,
    pub(crate) watcher_baseline_created_at: Option<String>,
    pub(crate) watcher_coverage_start_at: Option<String>,
    pub(crate) watcher_coverage_basis: Option<SessionWatchCoverageBasis>,
    pub(crate) watcher_partial_coverage_warning: Option<String>,
    pub(crate) projects: Vec<ListProjectItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ListProjectItem {
    pub(crate) project_selector: String,
    pub(crate) available: bool,
    pub(crate) unavailable_reason: Option<String>,
    pub(crate) repo_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpSessionWatchCoverage {
    pub(crate) status: SessionWatchStatus,
    pub(crate) baseline_created_at: Option<String>,
    pub(crate) coverage_start_at: Option<String>,
    pub(crate) coverage_basis: Option<SessionWatchCoverageBasis>,
    pub(crate) partial_coverage_warning: Option<String>,
}

pub(crate) fn validate_mcp_project_allowlist(
    runtime_home: &Path,
    connection_id: &str,
    project_ids: &[ProjectId],
) -> Result<(), McpAdapterError> {
    for project_id in project_ids {
        let access =
            agent_connection_project_access(runtime_home, connection_id, project_id.as_str())
                .map_err(McpAdapterError::Store)?
                .ok_or_else(|| {
                    McpAdapterError::Environment(format!(
                        "connection {connection_id} is not registered for project {}",
                        project_id.as_str()
                    ))
                })?;
        if !access.connection_enabled {
            return Err(McpAdapterError::Environment(format!(
                "connection {connection_id} is disabled"
            )));
        }
        if !access.project_allowed {
            return Err(McpAdapterError::Environment(format!(
                "project {} is outside connection {connection_id} project allowlist",
                project_id.as_str()
            )));
        }
        let Some(project) = access.project else {
            return Err(McpAdapterError::Environment(format!(
                "project {} is not registered",
                project_id.as_str()
            )));
        };
        let availability = inspect_allowed_project(
            runtime_home,
            &ConnectionProjectRecord {
                connection_internal_id: connection_id.to_owned(),
                project_internal_id: project.project_internal_id.clone(),
                project_id: project.project_id.clone(),
                created_at: String::new(),
                project,
            },
        );
        if !availability.available {
            return Err(McpAdapterError::Environment(format!(
                "project {} is unavailable: {}",
                availability.project_id,
                availability
                    .unavailable_reason
                    .unwrap_or_else(|| "unavailable".to_owned())
            )));
        }
    }
    Ok(())
}

pub(crate) fn resolve_connection_context(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> Result<
    (
        McpConnectionContext,
        AgentConnectionRecord,
        Vec<ConnectionProjectRecord>,
    ),
    McpAdapterError,
> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    runtime_home_record(&runtime_home)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment("Runtime Home is not initialized".to_owned())
        })?;
    match require_installation_profile(&runtime_home) {
        Ok(_) => {}
        Err(StoreError::NotFound {
            entity: "installation_profile",
            ..
        }) => {
            return Err(McpAdapterError::Environment(format!(
                "SETUP_REQUIRED: installation profile is missing for Runtime Home {}; run `volicord init --host <host> --repo <path>` from the Product Repository, then run `volicord connection verify <host> --repo <path>` or `volicord doctor` before starting the MCP transport process",
                runtime_home.display()
            )))
        }
        Err(error) => return Err(McpAdapterError::Store(error)),
    }
    validate_identifier_text("connection_internal_id", connection_internal_id)?;
    let connection = agent_connection_record(&runtime_home, connection_internal_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment(format!(
                "connection {connection_internal_id} is not registered"
            ))
        })?;
    let mode = validate_connection_record(&connection)?;
    let projects = list_connection_projects(&runtime_home, connection_internal_id)
        .map_err(McpAdapterError::Store)?;
    if projects.is_empty() {
        return Err(McpAdapterError::Environment(format!(
            "connection {connection_internal_id} has no connected projects"
        )));
    }

    let context = McpConnectionContext {
        runtime_home,
        connection_internal_id: AgentConnectionId::new(connection.connection_internal_id.clone()),
        mode,
        invocation_binding_basis: DEFAULT_INVOCATION_BINDING_BASIS.to_owned(),
        project_allowlist: None,
    };
    Ok((context, connection, projects))
}

pub(crate) fn validate_connection_record(
    connection: &AgentConnectionRecord,
) -> Result<AgentConnectionMode, McpAdapterError> {
    if !connection.enabled {
        return Err(McpAdapterError::Environment(format!(
            "connection {} is disabled",
            connection.connection_internal_id
        )));
    }
    validate_identifier_text("connection_internal_id", &connection.connection_internal_id)?;
    match serde_json::from_str::<Value>(&connection.metadata_json) {
        Ok(Value::Object(_)) => (),
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered connection metadata is not an object".to_owned(),
            ))
        }
        Err(error) => return Err(McpAdapterError::Json(error)),
    }
    parse_connection_mode(&connection.mode)
}

pub(crate) fn parse_connection_mode(mode: &str) -> Result<AgentConnectionMode, McpAdapterError> {
    match mode {
        CONNECTION_MODE_READ_ONLY => Ok(AgentConnectionMode::ReadOnly),
        CONNECTION_MODE_WORKFLOW => Ok(AgentConnectionMode::Workflow),
        other => Err(McpAdapterError::Environment(format!(
            "connection mode {other} is not supported for MCP startup"
        ))),
    }
}

pub(crate) fn current_enabled_connection(
    runtime_home: &Path,
    connection_internal_id: &str,
    tool_name: &str,
) -> Result<AgentConnectionRecord, McpAdapterError> {
    let connection = agent_connection_record(runtime_home, connection_internal_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| McpAdapterError::ToolExecution {
            tool_name: tool_name.to_owned(),
            message: format!("connection {connection_internal_id} is not registered"),
        })?;
    validate_connection_record(&connection).map_err(|error| McpAdapterError::ToolExecution {
        tool_name: tool_name.to_owned(),
        message: error.to_string(),
    })?;
    Ok(connection)
}

pub(crate) fn inspect_allowed_project(
    runtime_home: &Path,
    project: &ConnectionProjectRecord,
) -> McpProjectAvailability {
    let repo_root_display = project.project.repo_root.display().to_string();
    if project.project.status != ACTIVE_PROJECT_STATUS {
        return unavailable_project(project, repo_root_display, "project is not active");
    }
    let store = match CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id)) {
        Ok(store) => store,
        Err(error) => {
            return unavailable_project(
                project,
                repo_root_display,
                format!(
                    "project is not executable: {}",
                    concise_store_reason(&error)
                ),
            )
        }
    };
    if let Err(error) = store.project_state() {
        return unavailable_project(
            project,
            repo_root_display,
            format!(
                "project state is unavailable: {}",
                concise_store_reason(&error)
            ),
        );
    }
    McpProjectAvailability {
        project_id: project.project_id.clone(),
        available: true,
        unavailable_reason: None,
        repo_root_display,
    }
}

pub(crate) fn unavailable_project(
    project: &ConnectionProjectRecord,
    repo_root_display: String,
    reason: impl Into<String>,
) -> McpProjectAvailability {
    McpProjectAvailability {
        project_id: project.project_id.clone(),
        available: false,
        unavailable_reason: Some(reason.into()),
        repo_root_display,
    }
}

pub(crate) fn selected_project_from_availability(
    project: McpProjectAvailability,
) -> Result<ProjectId, McpAdapterError> {
    if !project.available {
        return Err(routing_error(format!(
            "project {} is unavailable: {}",
            project.project_id,
            project
                .unavailable_reason
                .unwrap_or_else(|| "unavailable".to_owned())
        )));
    }
    Ok(ProjectId::new(project.project_id))
}

pub(crate) fn routing_error(message: impl Into<String>) -> McpAdapterError {
    McpAdapterError::ToolExecution {
        tool_name: "project routing".to_owned(),
        message: message.into(),
    }
}

pub(crate) fn coverage_from_watch_baseline(
    baseline: &WatchBaselineRecord,
) -> McpSessionWatchCoverage {
    let metadata = serde_json::from_str::<Value>(&baseline.metadata_json).unwrap_or(Value::Null);
    let coverage_basis = metadata
        .get("coverage_basis")
        .and_then(Value::as_str)
        .and_then(|value| serde_json::from_value(Value::String(value.to_owned())).ok());
    let fallback_warning = coverage_basis.and_then(|basis| match basis {
        SessionWatchCoverageBasis::McpStart => None,
        SessionWatchCoverageBasis::FirstProjectSelection => {
            Some(FIRST_PROJECT_SELECTION_PARTIAL_COVERAGE_WARNING.to_owned())
        }
        SessionWatchCoverageBasis::MethodBoundary => {
            Some(METHOD_BOUNDARY_PARTIAL_COVERAGE_WARNING.to_owned())
        }
    });
    McpSessionWatchCoverage {
        status: serde_json::from_value(Value::String(baseline.status.clone()))
            .unwrap_or(SessionWatchStatus::Unavailable),
        baseline_created_at: Some(baseline.created_at.clone()),
        coverage_start_at: metadata
            .get("coverage_start_at")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| Some(baseline.created_at.clone())),
        coverage_basis,
        partial_coverage_warning: metadata
            .get("partial_coverage_warning")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .or(fallback_warning),
    }
}

pub(crate) fn concise_store_reason(error: &StoreError) -> String {
    match error {
        StoreError::NotFound { entity, .. } => format!("{entity} not found"),
        StoreError::InvalidProjectRegistration {
            field,
            relationship,
            ..
        } => format!("invalid project registration ({field}, {relationship})"),
        StoreError::InvalidInput { detail } => detail.clone(),
        StoreError::Conflict { entity, .. } => format!("{entity} conflict"),
        StoreError::CorruptStoredJson { field, .. }
        | StoreError::CorruptStoredValue { field, .. } => format!("corrupt stored field {field}"),
        StoreError::CorruptOwnerStateJson { logical_column, .. }
        | StoreError::CorruptOwnerStateValue { logical_column, .. } => {
            format!("corrupt owner state field {logical_column}")
        }
        StoreError::SchemaInvariant { database_kind, .. } => {
            format!("{database_kind} schema is invalid")
        }
        StoreError::UnsupportedStorageProfile {
            actual_storage_profile,
            ..
        } => {
            format!("unsupported storage profile {actual_storage_profile}")
        }
        StoreError::Sqlite(_) | StoreError::Io(_) => "storage access failed".to_owned(),
    }
}

pub(crate) fn controlled_invocation_binding_basis(value: &str) -> &'static str {
    match value.trim() {
        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING => {
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING
        }
        VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING => {
            VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING
        }
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING => VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        _ => DEFAULT_INVOCATION_BINDING_BASIS,
    }
}

pub(crate) fn unique_project_ids(project_ids: Vec<ProjectId>) -> Vec<ProjectId> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for project_id in project_ids {
        if seen.insert(project_id.as_str().to_owned()) {
            unique.push(project_id);
        }
    }
    unique
}
