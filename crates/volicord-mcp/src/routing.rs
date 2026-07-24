use crate::errors::McpAdapterError;
use crate::prelude::*;
use crate::util::*;
use schemars::{schema_for, JsonSchema};
use volicord_host_contract::{HostContractProfileId, McpServerKey};
use volicord_platform_fs::resolve_git_worktree_layout;
use volicord_types::HostKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionContext {
    pub runtime_home: PathBuf,
    pub connection_internal_id: AgentConnectionId,
    pub mode: AgentConnectionMode,
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

/// Local binding selected from a clone-portable shared managed launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDiscoveryResolution {
    pub runtime_home: PathBuf,
    pub repository_root: PathBuf,
    pub host: HostKind,
    pub connection_internal_id: AgentConnectionId,
    pub project_id: ProjectId,
    pub context: McpConnectionContext,
}

impl RepositoryDiscoveryResolution {
    /// Resolves exactly one enabled shared connection for the current repository.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        current_dir: &Path,
        host: HostKind,
    ) -> Result<Self, McpAdapterError> {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        let repository_root = repository_root_from_current_dir(current_dir)?;
        let project = project_record_by_repo_root_read_only(&runtime_home, &repository_root)
            .map_err(McpAdapterError::Store)?
            .ok_or_else(|| {
                McpAdapterError::Environment(format!(
                    "REPOSITORY_DISCOVERY_PROJECT_NOT_REGISTERED: repository {} is not registered in Runtime Home {}; run `volicord init --shared --host {} --repo {}` from this clone",
                    repository_root.display(),
                    runtime_home.display(),
                    host.as_str(),
                    repository_root.display(),
                ))
            })?;
        let mut candidates = Vec::new();
        for connection in list_agent_connections_read_only(&runtime_home)
            .map_err(McpAdapterError::Store)?
            .into_iter()
            .filter(|connection| {
                connection.enabled
                    && connection.host_kind == host.as_str()
                    && connection.intent == CONNECTION_INTENT_SHARED
                    && connection.host_scope == HOST_SCOPE_PROJECT
            })
        {
            let projects = list_connection_projects_read_only(
                &runtime_home,
                &connection.connection_internal_id,
            )
            .map_err(McpAdapterError::Store)?;
            if projects
                .iter()
                .any(|candidate| candidate.project_internal_id == project.project_internal_id)
            {
                candidates.push(connection);
            }
        }

        let connection = match candidates.as_slice() {
            [connection] => connection,
            [] => {
                return Err(McpAdapterError::Environment(format!(
                    "REPOSITORY_DISCOVERY_CONNECTION_NOT_FOUND: repository {} has no enabled shared {} Agent Connection in Runtime Home {}; run `volicord init --shared --host {} --repo {}` and then `volicord connection verify {} --shared --repo {}`",
                    repository_root.display(),
                    host.as_str(),
                    runtime_home.display(),
                    host.as_str(),
                    repository_root.display(),
                    host.as_str(),
                    repository_root.display(),
                )))
            }
            _ => {
                return Err(McpAdapterError::Environment(format!(
                    "REPOSITORY_DISCOVERY_CONNECTION_AMBIGUOUS: repository {} has {} enabled shared {} Agent Connections in Runtime Home {}; run `volicord connection list --repo {}` and remove duplicate shared connections before retrying",
                    repository_root.display(),
                    candidates.len(),
                    host.as_str(),
                    runtime_home.display(),
                    repository_root.display(),
                )))
            }
        };
        let project_id = ProjectId::new(project.project_id.clone());
        validate_mcp_project_allowlist(
            &runtime_home,
            &connection.connection_internal_id,
            std::slice::from_ref(&project_id),
        )?;
        let context = McpConnectionContext::resolve(
            &runtime_home,
            connection.connection_internal_id.clone(),
        )?
        .with_project_allowlist(vec![project_id.clone()]);
        Ok(Self {
            runtime_home,
            repository_root,
            host,
            connection_internal_id: context.connection_internal_id.clone(),
            project_id,
            context,
        })
    }
}

fn repository_root_from_current_dir(current_dir: &Path) -> Result<PathBuf, McpAdapterError> {
    let mut cursor = std::fs::canonicalize(current_dir).map_err(|error| {
        McpAdapterError::Environment(format!(
            "REPOSITORY_DISCOVERY_CWD_UNAVAILABLE: current directory {} is not accessible: {error}",
            current_dir.display()
        ))
    })?;
    if !std::fs::metadata(&cursor)
        .map_err(McpAdapterError::Io)?
        .is_dir()
    {
        return Err(McpAdapterError::Environment(format!(
            "REPOSITORY_DISCOVERY_CWD_UNAVAILABLE: current path {} is not a directory",
            cursor.display()
        )));
    }
    loop {
        match resolve_git_worktree_layout(&cursor) {
            Ok(Some(layout)) => return Ok(layout.repository_root),
            Ok(None) => {}
            Err(error) => {
                return Err(McpAdapterError::Environment(format!(
                    "REPOSITORY_DISCOVERY_GIT_INVALID: failed to inspect Git repository marker at {}: {error}",
                    cursor.display()
                )))
            }
        }
        if !cursor.pop() {
            break;
        }
    }
    Err(McpAdapterError::Environment(format!(
        "REPOSITORY_DISCOVERY_NOT_GIT_REPOSITORY: no Git repository root was found from {}; open the host inside a registered Git clone and rerun `volicord init --shared --host <host> --repo <path>` if needed",
        current_dir.display()
    )))
}

/// Effective storage capability observed for an MCP session or project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStorageCapability {
    ReadWrite,
    ReadOnly,
    Unavailable,
    Unknown,
}

impl McpStorageCapability {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ReadWrite => "read_write",
            Self::ReadOnly => "read_only",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) const fn allows_mutation(self) -> bool {
        matches!(self, Self::ReadWrite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpEffectiveToolMode {
    Workflow,
    ReadOnlyDegraded,
    ReadOnly,
    Unavailable,
}

pub(crate) fn effective_tool_mode_for_mode_and_storage(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
) -> McpEffectiveToolMode {
    match (mode, storage_capability) {
        (_, McpStorageCapability::Unavailable) => McpEffectiveToolMode::Unavailable,
        (AgentConnectionMode::ReadOnly, _) => McpEffectiveToolMode::ReadOnly,
        (AgentConnectionMode::Workflow, McpStorageCapability::ReadWrite) => {
            McpEffectiveToolMode::Workflow
        }
        (AgentConnectionMode::Workflow, McpStorageCapability::ReadOnly)
        | (AgentConnectionMode::Workflow, McpStorageCapability::Unknown) => {
            McpEffectiveToolMode::ReadOnlyDegraded
        }
    }
}

/// Connection-bound startup facts shared by stdio startup and preflight checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionStartupInspection {
    pub runtime_home: PathBuf,
    pub connection_internal_id: AgentConnectionId,
    pub mode: AgentConnectionMode,
    pub enabled: bool,
    pub server_key: McpServerKey,
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
            .map(inspect_allowed_project_read_only)
            .collect::<Vec<_>>();
        let server_key = McpServerKey::parse(connection.server_name.clone()).map_err(|error| {
            McpAdapterError::Environment(format!(
                "Agent Connection has an invalid MCP server registration key: {error}"
            ))
        })?;
        crate::tool_registry::canonical_mcp_tool_catalog(&server_key).map_err(|error| {
            McpAdapterError::Environment(format!(
                "Agent Connection MCP callable catalog is invalid: {error}"
            ))
        })?;

        Ok(Self {
            runtime_home: context.runtime_home.clone(),
            connection_internal_id: context.connection_internal_id,
            mode: context.mode,
            enabled: connection.enabled,
            server_key,
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
            project_allowlist: None,
        }
    }

    /// Projects the deterministic read-only preflight report.
    pub fn preflight_report(&self) -> McpPreflightReport {
        let available_projects = self
            .projects
            .iter()
            .filter(|project| project.available)
            .count();
        let tools = crate::tool_registry::mcp_tools_for_mode_and_storage_with_detail(
            self.mode,
            McpStorageCapability::Unknown,
            crate::tool_registry::ToolSchemaDetail::RuntimeCompact,
        );
        let tools_list_schema_validation =
            crate::tool_registry::tools_list_schema_validation_status(&tools);
        let tool_naming_style = crate::tool_registry::mcp_tool_naming_style(&tools);
        let host_callable_tools =
            crate::tool_registry::effective_mcp_tool_catalog(&self.server_key, &tools)
                .expect("startup validated the complete MCP callable catalog")
                .identities()
                .iter()
                .map(|identity| McpPreflightHostToolIdentity {
                    profile: identity.profile().as_str(),
                    server_key: identity.source().server().as_str().to_owned(),
                    raw_tool_name: identity.source().raw_tool_name().as_str().to_owned(),
                    callable_name: identity.callable_name().as_str().to_owned(),
                })
                .collect();
        McpPreflightReport {
            operation: "mcp_preflight",
            status: if self.projects.iter().all(|project| project.available) {
                "passed"
            } else {
                "failed"
            },
            side_effects: Vec::new(),
            evidence_class: "read_only_preflight",
            does_not_prove: vec![
                "store_writeability",
                "active_protocol_conformance",
                "managed_host_operation",
                "agent_connection_authority",
            ],
            configuration: "valid",
            canonical_managed_entry: "passed",
            transport: "stdio",
            transport_disclosure: TRANSPORT_DISCLOSURE_TEXT,
            runtime_home: self.runtime_home.display().to_string(),
            connection_id: self.connection_internal_id.as_str().to_owned(),
            mode: self.mode.as_str(),
            enabled: self.enabled,
            registry_read: "passed",
            project_state_read: self.project_state_read_status(),
            writeability: McpPreflightWriteability {
                status: "not_checked",
                requirement: "requires_active_verification",
            },
            effective_tool_mode: "requires_active_verification",
            tools_list_schema_validation,
            tool_naming_style,
            protocol_profiles: ProtocolRegistry::production()
                .oldest_to_newest()
                .map(|profile| profile.revision().as_str().to_owned())
                .collect(),
            host_contracts: HostContractProfileId::ALL
                .into_iter()
                .map(|profile| McpPreflightHostContract {
                    profile: profile.as_str(),
                    digest: profile.contract_digest(),
                })
                .collect(),
            host_callable_tools,
            allowed_projects: self.allowed_project_count,
            available_projects,
            projects: self
                .projects
                .iter()
                .map(|project| McpPreflightProject {
                    project_id: project.project_id.clone(),
                    available: project.available,
                    state_read: project.state_read_status(),
                    state_write: "not_checked",
                    writeability_requirement: "requires_active_verification",
                    unavailable_reason: project.unavailable_reason.clone(),
                    repo_root: project.repo_root_display.clone(),
                })
                .collect(),
        }
    }

    fn project_state_read_status(&self) -> &'static str {
        if self.projects.iter().all(|project| project.available) {
            "passed"
        } else {
            "failed"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpPreflightReport {
    pub operation: &'static str,
    pub status: &'static str,
    pub side_effects: Vec<&'static str>,
    pub evidence_class: &'static str,
    pub does_not_prove: Vec<&'static str>,
    pub configuration: &'static str,
    pub canonical_managed_entry: &'static str,
    pub transport: &'static str,
    pub transport_disclosure: &'static str,
    pub runtime_home: String,
    pub connection_id: String,
    pub mode: &'static str,
    pub enabled: bool,
    pub registry_read: &'static str,
    pub project_state_read: &'static str,
    pub writeability: McpPreflightWriteability,
    pub effective_tool_mode: &'static str,
    pub tools_list_schema_validation: &'static str,
    pub tool_naming_style: &'static str,
    pub protocol_profiles: Vec<String>,
    pub host_contracts: Vec<McpPreflightHostContract>,
    pub host_callable_tools: Vec<McpPreflightHostToolIdentity>,
    pub allowed_projects: usize,
    pub available_projects: usize,
    pub projects: Vec<McpPreflightProject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpPreflightHostToolIdentity {
    pub profile: &'static str,
    pub server_key: String,
    pub raw_tool_name: String,
    pub callable_name: String,
}

impl McpPreflightReport {
    pub fn render_human(&self, verbose: bool) -> String {
        let mut output = format!(
            "Operation: MCP preflight\nStatus: {}\nSide effects: none\nEvidence class: {}\nDoes not prove: {}\nWriteability: {} ({})\n",
            self.status,
            self.evidence_class,
            self.does_not_prove.join(", "),
            self.writeability.status,
            self.writeability.requirement,
        );
        if verbose {
            output.push_str(&format!(
                "Configuration: {}\nCanonical managed entry: {}\nTransport: {}\n{}\nRuntime Home: {}\nConnection: {}\nMode: {}\nRegistry read: {}\nProject state read: {}\nTool schema validation: {}\nTool naming style: {}\nProtocol profiles: {}\nHost contracts: {}\nHost callable tools: {}\nAllowed projects: {}\nAvailable projects: {}\n",
                self.configuration,
                self.canonical_managed_entry,
                self.transport,
                self.transport_disclosure,
                self.runtime_home,
                self.connection_id,
                self.mode,
                self.registry_read,
                self.project_state_read,
                self.tools_list_schema_validation,
                self.tool_naming_style,
                self.protocol_profiles.join(", "),
                self.host_contracts
                    .iter()
                    .map(|contract| contract.profile)
                    .collect::<Vec<_>>()
                    .join(", "),
                self.host_callable_tools
                    .iter()
                    .map(|identity| identity.callable_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                self.allowed_projects,
                self.available_projects,
            ));
            for project in &self.projects {
                output.push_str(&format!(
                    "Project {}: available={}, state_read={}, state_write={} ({}), repo_root={}\n",
                    project.project_id,
                    project.available,
                    project.state_read,
                    project.state_write,
                    project.writeability_requirement,
                    project.repo_root,
                ));
            }
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpPreflightWriteability {
    pub status: &'static str,
    pub requirement: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpPreflightProject {
    pub project_id: String,
    pub available: bool,
    pub state_read: &'static str,
    pub state_write: &'static str,
    pub writeability_requirement: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
    pub repo_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpPreflightHostContract {
    pub profile: &'static str,
    pub digest: String,
}

/// MCP-visible availability facts for one connection-allowed project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpProjectAvailability {
    pub project_id: String,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub repo_root_display: String,
    pub(crate) storage_capability: McpStorageCapability,
}

impl McpProjectAvailability {
    fn state_read_status(&self) -> &'static str {
        if self.available {
            "passed"
        } else {
            "failed"
        }
    }
}

pub(crate) fn storage_capability_for_projects(
    projects: &[McpProjectAvailability],
) -> McpStorageCapability {
    let readable = projects
        .iter()
        .filter(|project| project.available)
        .map(|project| project.storage_capability)
        .collect::<Vec<_>>();
    if readable.is_empty() {
        return McpStorageCapability::Unavailable;
    }
    if readable
        .iter()
        .all(|capability| *capability == McpStorageCapability::ReadWrite)
    {
        return McpStorageCapability::ReadWrite;
    }
    if readable.contains(&McpStorageCapability::ReadOnly) {
        return McpStorageCapability::ReadOnly;
    }
    McpStorageCapability::Unknown
}

fn inspect_allowed_project_read_only(project: &ConnectionProjectRecord) -> McpProjectAvailability {
    inspect_allowed_project_with_write_probe(project, false)
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub(crate) struct ListProjectsResult {
    pub(crate) connection_id: String,
    pub(crate) mode: AgentConnectionMode,
    pub(crate) projects: Vec<ListProjectItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub(crate) struct ListProjectItem {
    pub(crate) project_selector: String,
    pub(crate) available: bool,
    pub(crate) unavailable_reason: Option<String>,
    pub(crate) repo_root: String,
}

pub(crate) fn list_projects_output_schema() -> Value {
    let mut schema = serde_json::to_value(schema_for!(
        volicord_types::McpToolStructuredContent<ListProjectsResult>
    ))
    .expect("list-projects output schema should serialize");
    schema
        .as_object_mut()
        .expect("list-projects output schema should be an object")
        .insert("type".to_owned(), Value::String("object".to_owned()));
    schema
}

pub(crate) fn validate_mcp_project_allowlist(
    runtime_home: &Path,
    connection_id: &str,
    project_ids: &[ProjectId],
) -> Result<(), McpAdapterError> {
    for project_id in project_ids {
        let access = agent_connection_project_access_read_only(
            runtime_home,
            connection_id,
            project_id.as_str(),
        )
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
        let availability = inspect_allowed_project(&ConnectionProjectRecord {
            connection_internal_id: connection_id.to_owned(),
            project_internal_id: project.project_internal_id.clone(),
            project_id: project.project_id.clone(),
            created_at: String::new(),
            project,
        });
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
    runtime_home_record_read_only(&runtime_home)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment("Runtime Home is not initialized".to_owned())
        })?;
    match require_installation_profile_read_only(&runtime_home) {
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
    let connection = agent_connection_record_read_only(&runtime_home, connection_internal_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment(format!(
                "connection {connection_internal_id} is not registered"
            ))
        })?;
    let mode = validate_connection_record(&connection)?;
    let projects = list_connection_projects_read_only(&runtime_home, connection_internal_id)
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
    let connection = agent_connection_record_read_only(runtime_home, connection_internal_id)
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

pub(crate) fn inspect_allowed_project(project: &ConnectionProjectRecord) -> McpProjectAvailability {
    inspect_allowed_project_with_write_probe(project, true)
}

fn inspect_allowed_project_with_write_probe(
    project: &ConnectionProjectRecord,
    probe_writeability: bool,
) -> McpProjectAvailability {
    let repo_root_display = project.project.repo_root.display().to_string();
    if project.project.status != ACTIVE_PROJECT_STATUS {
        return unavailable_project(project, repo_root_display, "project is not active");
    }
    let conn = match open_project_state_database_read_only(&project.project.state_db_path) {
        Ok(conn) => conn,
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
    if let Err(error) = conn.query_row(
        "SELECT project_id FROM project_state WHERE project_id = ?1",
        [project.project_id.as_str()],
        |row| row.get::<_, String>(0),
    ) {
        return unavailable_project(
            project,
            repo_root_display,
            format!(
                "project state is unavailable: {}",
                concise_store_reason(&StoreError::from(error))
            ),
        );
    }
    let storage_capability = if probe_writeability {
        match sqlite_database_write_capability(&project.project.state_db_path) {
            Ok(true) => McpStorageCapability::ReadWrite,
            Ok(false) => McpStorageCapability::ReadOnly,
            Err(_) => McpStorageCapability::Unknown,
        }
    } else {
        McpStorageCapability::Unknown
    };
    McpProjectAvailability {
        project_id: project.project_id.clone(),
        available: true,
        unavailable_reason: None,
        repo_root_display,
        storage_capability,
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
        storage_capability: McpStorageCapability::Unavailable,
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
        StoreError::UnsupportedPlatformEnvironment { reason, .. }
        | StoreError::PlatformEnvironmentUnavailable { reason, .. } => (*reason).to_owned(),
        StoreError::Sqlite(_) | StoreError::Io(_) => "storage access failed".to_owned(),
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

#[cfg(test)]
mod repository_discovery_tests {
    use std::{error::Error, fs, path::PathBuf};

    use volicord_store::{
        agent_connections::{
            add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
            ConnectionProjectRegistration, CONNECTION_INTENT_SHARED, CONNECTION_MODE_WORKFLOW,
            HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
        },
        bootstrap::{
            initialize_runtime_home, register_project, write_installation_profile,
            InstallationProfileRegistration, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
    };
    use volicord_test_support::TempRuntimeHome;

    use crate::ManagedMcpLaunchSpec;
    use volicord_types::HostKind;

    use super::RepositoryDiscoveryResolution;

    struct DiscoveryFixture {
        _runtime: TempRuntimeHome,
        runtime_home: PathBuf,
        repo_root: PathBuf,
    }

    fn discovery_fixture(
        label: &str,
        project_id: &str,
        connection_ids: &[&str],
    ) -> Result<DiscoveryFixture, Box<dyn Error>> {
        let runtime = TempRuntimeHome::new(label)?;
        let repo_root = runtime.create_product_repo("clone")?;
        fs::create_dir(repo_root.join(".git"))?;
        initialize_runtime_home(runtime.path(), &format!("runtime_{label}"), "{}")?;
        write_installation_profile(
            runtime.path(),
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "volicord".to_owned(),
                volicord_mcp_command: "volicord".to_owned(),
                bin_dir: runtime.path().join("bin"),
                default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        register_project(
            runtime.path(),
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        for (index, connection_id) in connection_ids.iter().enumerate() {
            ensure_agent_connection(
                runtime.path(),
                AgentConnectionRegistration {
                    connection_internal_id: (*connection_id).to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: format!("volicord-{index}"),
                    config_target: repo_root
                        .join(format!(".codex/config-{index}.toml"))
                        .display()
                        .to_string(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: format!("fingerprint-{index}"),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                runtime.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: (*connection_id).to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
        }
        Ok(DiscoveryFixture {
            runtime_home: runtime.path().to_path_buf(),
            repo_root,
            _runtime: runtime,
        })
    }

    #[test]
    fn one_portable_shared_launch_resolves_clone_local_ids_in_two_runtime_homes(
    ) -> Result<(), Box<dyn Error>> {
        let first = discovery_fixture("discovery-clone-a", "project_clone_a", &["connection_a"])?;
        let second = discovery_fixture("discovery-clone-b", "project_clone_b", &["connection_b"])?;
        let launch = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        assert_eq!(
            launch.args(),
            ["_host-launch", "codex", "--discover-repository"]
        );

        let first_resolution = RepositoryDiscoveryResolution::resolve(
            &first.runtime_home,
            &first.repo_root,
            launch.host_kind(),
        )?;
        let second_resolution = RepositoryDiscoveryResolution::resolve(
            &second.runtime_home,
            &second.repo_root,
            launch.host_kind(),
        )?;

        assert_eq!(
            first_resolution.connection_internal_id.as_str(),
            "connection_a"
        );
        assert_eq!(first_resolution.project_id.as_str(), "project_clone_a");
        assert_eq!(
            second_resolution.connection_internal_id.as_str(),
            "connection_b"
        );
        assert_eq!(second_resolution.project_id.as_str(), "project_clone_b");
        assert_eq!(
            first_resolution.context.project_allowlist,
            Some(vec![first_resolution.project_id.clone()])
        );
        assert_eq!(
            second_resolution.context.project_allowlist,
            Some(vec![second_resolution.project_id.clone()])
        );
        Ok(())
    }

    #[test]
    fn repository_discovery_fails_closed_for_unregistered_and_ambiguous_local_state(
    ) -> Result<(), Box<dyn Error>> {
        let unregistered_runtime = TempRuntimeHome::new("discovery-unregistered")?;
        let unregistered_repo = unregistered_runtime.create_product_repo("clone")?;
        fs::create_dir(unregistered_repo.join(".git"))?;
        let unregistered = RepositoryDiscoveryResolution::resolve(
            unregistered_runtime.path(),
            &unregistered_repo,
            HostKind::Codex,
        )
        .expect_err("unregistered clone must fail closed");
        assert!(unregistered
            .to_string()
            .contains("REPOSITORY_DISCOVERY_PROJECT_NOT_REGISTERED"));
        assert!(unregistered.to_string().contains("volicord init --shared"));

        let ambiguous = discovery_fixture(
            "discovery-ambiguous",
            "project_ambiguous",
            &["connection_one", "connection_two"],
        )?;
        let ambiguous_error = RepositoryDiscoveryResolution::resolve(
            &ambiguous.runtime_home,
            &ambiguous.repo_root,
            HostKind::Codex,
        )
        .expect_err("duplicate shared connections must fail closed");
        assert!(ambiguous_error
            .to_string()
            .contains("REPOSITORY_DISCOVERY_CONNECTION_AMBIGUOUS"));
        assert!(ambiguous_error
            .to_string()
            .contains("volicord connection list"));
        Ok(())
    }
}
