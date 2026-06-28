#![forbid(unsafe_code)]

//! Local MCP adapter for public Volicord method calls.
//!
//! This crate owns transport dispatch. It binds one MCP server process to one
//! Agent Connection, derives adapter-owned invocation facts, decodes tool
//! arguments into `volicord-types` request structs, and hands execution to
//! `volicord-core`.

use std::{
    error::Error,
    ffi::OsString,
    fmt,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Map, Value};
use volicord_core::{
    CoreBoundary, CorePipelineError, CoreService, InvocationContext, PipelineResponse,
};
use volicord_store::{
    agent_connections::{
        agent_connection_project_access, agent_connection_record, list_connection_projects,
        AgentConnectionRecord, ConnectionProjectRecord, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW,
    },
    bootstrap::{runtime_home_record, ACTIVE_PROJECT_STATUS},
    core_pipeline::CoreProjectStore,
    runtime_home::{
        resolve_runtime_home as resolve_shared_runtime_home, RuntimeHomeResolutionError,
    },
    StoreError,
};
use volicord_types::{
    public_request_schema, ActorSource, AgentConnectionId, AgentConnectionMode, CloseTaskRequest,
    IntakeRequest, MethodOperationCategory, OperationCategory, PrepareWriteRequest, ProjectId,
    RecordRunRequest, RequestUserJudgmentRequest, StageArtifactRequest, StatusRequest,
    ToolEnvelope, UpdateScopeRequest, VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-11-25";
const SERVER_NAME: &str = "volicord-mcp";
const DEFAULT_INVOCATION_BINDING_BASIS: &str = VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING;

/// Agent-facing public Volicord method tools exposed through workflow MCP connections.
pub const PUBLIC_METHOD_TOOL_NAMES: [&str; 8] = [
    "volicord.intake",
    "volicord.update_scope",
    "volicord.status",
    "volicord.prepare_write",
    "volicord.stage_artifact",
    "volicord.record_run",
    "volicord.request_user_judgment",
    "volicord.close_task",
];

/// Public method tools exposed through read-only MCP connections.
pub const READ_ONLY_METHOD_TOOL_NAMES: [&str; 2] = ["volicord.status", "volicord.close_task"];

/// Adapter-owned MCP utility tools that are not public Core methods.
pub const ADAPTER_UTILITY_TOOL_NAMES: [&str; 1] = ["volicord.list_projects"];

const LIST_PROJECTS_TOOL_NAME: &str = "volicord.list_projects";
const SERVER_INSTRUCTIONS: &str = "Volicord records task scope, write readiness, evidence, runs, user-owned judgment requests, artifacts, and close readiness for explicitly registered Product Repositories. If project selection is unclear, call volicord.list_projects and use one listed project_id; do not guess from folders, roots, labels, or memory. Volicord state management is separate from permission to edit product files: product-file edits still require the host/user path and any required Write Check. These instructions are guidance, not access control or a promise of automatic tool use.";

/// Minimal MCP adapter marker for validating dependency direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpAdapterBoundary {
    core: CoreBoundary,
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

/// Tool metadata returned by `tools/list`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Agent-Connection-bound adapter facts that are not accepted from tool arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionContext {
    pub runtime_home: PathBuf,
    pub connection_id: AgentConnectionId,
    pub mode: AgentConnectionMode,
    pub invocation_binding_basis: String,
}

impl McpConnectionContext {
    /// Resolves and validates one Agent Connection startup binding.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        connection_id: impl Into<String>,
    ) -> Result<Self, McpAdapterError> {
        let connection_id = connection_id.into();
        let (context, _, _) = resolve_connection_context(runtime_home, &connection_id)?;
        Ok(context)
    }

    /// Replaces the controlled adapter-binding basis carried into Core.
    pub fn with_invocation_binding_basis(mut self, basis: impl Into<String>) -> Self {
        let basis = basis.into();
        self.invocation_binding_basis = controlled_invocation_binding_basis(&basis).to_owned();
        self
    }
}

/// Connection-bound startup facts shared by stdio startup and preflight checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionStartupInspection {
    pub runtime_home: PathBuf,
    pub connection_id: AgentConnectionId,
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
        let connection_id = connection_id.into();
        let (context, connection, projects) =
            resolve_connection_context(runtime_home, &connection_id)?;
        let selected_projects = if let Some(project_id) = detail_project_id {
            if !projects
                .iter()
                .any(|project| project.project_id == project_id.as_str())
            {
                return Err(McpAdapterError::Environment(format!(
                    "project {} is outside connection {} project allowlist",
                    project_id.as_str(),
                    connection.connection_id
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
            connection_id: context.connection_id,
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
            connection_id: self.connection_id.clone(),
            mode: self.mode,
            invocation_binding_basis: DEFAULT_INVOCATION_BINDING_BASIS.to_owned(),
        }
    }

    /// Formats the deterministic operator preflight report.
    pub fn preflight_report(&self) -> String {
        let available_projects = self
            .projects
            .iter()
            .filter(|project| project.available)
            .count();
        let mut report = format!(
            "configuration: valid\ntransport: stdio\nruntime_home: {}\nconnection_id: {}\nmode: {}\nenabled: {}\nallowed_projects: {}\navailable_projects: {}\nverification_scope: startup_check_only\n",
            self.runtime_home.display(),
            self.connection_id.as_str(),
            self.mode.as_str(),
            self.enabled,
            self.allowed_project_count,
            available_projects
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
struct ListProjectsResult {
    connection_id: String,
    mode: AgentConnectionMode,
    projects: Vec<ListProjectItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ListProjectItem {
    project_id: String,
    available: bool,
    unavailable_reason: Option<String>,
    repo_root: String,
}

/// Invocation context derived for one tool call before entering Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDerivedInvocationContext {
    pub project_id: ProjectId,
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub invocation_binding_basis: String,
}

impl McpDerivedInvocationContext {
    fn core_invocation(&self) -> InvocationContext {
        InvocationContext::new(
            self.project_id.clone(),
            self.actor_source.clone(),
            self.operation_category,
            self.invocation_binding_basis.clone(),
        )
    }
}

/// Local MCP adapter bound to a Core service and one Agent Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAdapter {
    core: CoreService,
    runtime_home: PathBuf,
    context: McpConnectionContext,
}

impl McpAdapter {
    /// Creates an adapter for a Runtime Home and connection-bound adapter context.
    pub fn new(runtime_home: impl AsRef<Path>, context: McpConnectionContext) -> Self {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        Self {
            core: CoreService::new(&runtime_home),
            runtime_home,
            context,
        }
    }

    /// Returns the tools exposed by this adapter's current connection mode.
    pub fn tools(&self) -> Vec<McpToolDefinition> {
        mcp_tools_for_mode(self.context.mode)
    }

    /// Derives local invocation facts for one decoded request envelope.
    pub fn derive_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
    ) -> McpDerivedInvocationContext {
        McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(self.context.connection_id.clone()),
            operation_category,
            invocation_binding_basis: self.context.invocation_binding_basis.clone(),
        }
    }

    /// Calls one public Volicord method tool and returns Core's response.
    pub fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        match tool_name {
            "volicord.intake" => {
                self.call_core_method::<IntakeRequest, _>(tool_name, params, CoreService::intake)
            }
            "volicord.update_scope" => self.call_core_method::<UpdateScopeRequest, _>(
                tool_name,
                params,
                CoreService::update_scope,
            ),
            "volicord.status" => {
                self.call_core_method::<StatusRequest, _>(tool_name, params, CoreService::status)
            }
            "volicord.prepare_write" => self.call_core_method::<PrepareWriteRequest, _>(
                tool_name,
                params,
                CoreService::prepare_write,
            ),
            "volicord.stage_artifact" => self.call_core_method::<StageArtifactRequest, _>(
                tool_name,
                params,
                CoreService::stage_artifact,
            ),
            "volicord.record_run" => self.call_core_method::<RecordRunRequest, _>(
                tool_name,
                params,
                CoreService::record_run,
            ),
            "volicord.request_user_judgment" => self
                .call_core_method::<RequestUserJudgmentRequest, _>(
                    tool_name,
                    params,
                    CoreService::request_user_judgment,
                ),
            "volicord.close_task" => self.call_core_method::<CloseTaskRequest, _>(
                tool_name,
                params,
                CoreService::close_task,
            ),
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn call_core_method<T, F>(
        &self,
        tool_name: &str,
        params: Value,
        call: F,
    ) -> Result<PipelineResponse, McpAdapterError>
    where
        T: serde::de::DeserializeOwned + MethodOperationCategory + HasEnvelope,
        F: FnOnce(
            &CoreService,
            T,
            InvocationContext,
        ) -> Result<PipelineResponse, CorePipelineError>,
    {
        let prepared: PreparedCoreRequest<T> = self.prepare_typed_request(tool_name, params)?;
        call(
            &self.core,
            prepared.request,
            prepared.invocation.core_invocation(),
        )
        .map_err(McpAdapterError::Core)
    }

    fn call_adapter_tool(&self, tool_name: &str, params: Value) -> Result<Value, McpAdapterError> {
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
                let result = self.list_projects_result()?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn list_projects_result(&self) -> Result<ListProjectsResult, McpAdapterError> {
        let connection = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_id.as_str(),
            "volicord.list_projects",
        )?;
        let projects =
            list_connection_projects(&self.runtime_home, self.context.connection_id.as_str())
                .map_err(McpAdapterError::Store)?;
        let items = projects
            .iter()
            .map(|project| inspect_allowed_project(&self.runtime_home, project))
            .map(|project| ListProjectItem {
                project_id: project.project_id,
                available: project.available,
                unavailable_reason: project.unavailable_reason,
                repo_root: project.repo_root_display,
            })
            .collect::<Vec<_>>();
        let mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: "volicord.list_projects".to_owned(),
                message: error.to_string(),
            }
        })?;

        Ok(ListProjectsResult {
            connection_id: connection.connection_id,
            mode,
            projects: items,
        })
    }

    fn prepare_typed_request<T>(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PreparedCoreRequest<T>, McpAdapterError>
    where
        T: serde::de::DeserializeOwned + MethodOperationCategory + HasEnvelope,
    {
        let prepared_params = self.prepare_connection_arguments(tool_name, params)?;
        let request: T = self.decode_params(tool_name, prepared_params)?;
        if request.method_name().as_str() != tool_name {
            return Err(McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "decoded request method does not match MCP tool name".to_owned(),
            });
        }
        let operation_category = request.operation_category();
        self.ensure_mode_allows(tool_name, operation_category)?;
        let invocation =
            self.derive_invocation_context(request_envelope(&request), operation_category);
        Ok(PreparedCoreRequest {
            request,
            invocation,
        })
    }

    fn prepare_connection_arguments(
        &self,
        tool_name: &str,
        mut params: Value,
    ) -> Result<Value, McpAdapterError> {
        let object = params
            .as_object_mut()
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "tool arguments must be an object containing an envelope object"
                    .to_owned(),
            })?;
        reject_caller_owned_invocation_fields(object, tool_name)?;
        let envelope = object
            .get_mut("envelope")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "public Volicord tool arguments require an envelope object".to_owned(),
            })?;
        reject_caller_owned_invocation_fields(envelope, tool_name)?;
        let requested_project_id = optional_string_field(envelope, "project_id", tool_name)?;
        let selected_project_id = self.select_project(requested_project_id.as_deref())?;
        envelope.insert(
            "project_id".to_owned(),
            Value::String(selected_project_id.as_str().to_owned()),
        );
        Ok(params)
    }

    fn select_project(
        &self,
        requested_project_id: Option<&str>,
    ) -> Result<ProjectId, McpAdapterError> {
        let connection_id = self.context.connection_id.as_str();
        let _connection =
            current_enabled_connection(&self.runtime_home, connection_id, "project routing")?;

        if let Some(project_id) = requested_project_id {
            let access =
                agent_connection_project_access(&self.runtime_home, connection_id, project_id)
                    .map_err(McpAdapterError::Store)?
                    .ok_or_else(|| McpAdapterError::ToolExecution {
                        tool_name: "project routing".to_owned(),
                        message: format!("connection {connection_id} is not registered"),
                    })?;
            if !access.connection_enabled {
                return Err(routing_error("connection is disabled"));
            }
            if !access.project_allowed {
                return Err(routing_error(format!(
                    "project {project_id} is outside this connection project allowlist"
                )));
            }
            let project = access
                .project
                .ok_or_else(|| routing_error(format!("project {project_id} is not registered")))?;
            let project_record = ConnectionProjectRecord {
                connection_internal_id: connection_id.to_owned(),
                connection_id: connection_id.to_owned(),
                project_internal_id: project.project_internal_id.clone(),
                project_id: project.project_id.clone(),
                created_at: String::new(),
                project,
            };
            let availability = inspect_allowed_project(&self.runtime_home, &project_record);
            return selected_project_from_availability(availability);
        }

        let projects = list_connection_projects(&self.runtime_home, connection_id)
            .map_err(McpAdapterError::Store)?;
        if projects.is_empty() {
            return Err(routing_error(
                "connection has no allowed projects; ask the operator to add one",
            ));
        }
        if projects.len() != 1 {
            return Err(routing_error(
                "project selection is ambiguous for this connection; envelope.project_id is required when multiple projects are allowed",
            ));
        }

        selected_project_from_availability(inspect_allowed_project(
            &self.runtime_home,
            &projects[0],
        ))
    }

    fn ensure_mode_allows(
        &self,
        tool_name: &str,
        operation_category: OperationCategory,
    ) -> Result<(), McpAdapterError> {
        let connection = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_id.as_str(),
            tool_name,
        )?;
        let current_mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        if self
            .context
            .mode
            .allows_operation_category(operation_category)
            && current_mode.allows_operation_category(operation_category)
        {
            return Ok(());
        }
        let blocking_mode = if !current_mode.allows_operation_category(operation_category) {
            current_mode
        } else {
            self.context.mode
        };
        Err(McpAdapterError::ToolExecution {
            tool_name: tool_name.to_owned(),
            message: format!(
                "connection mode {} does not allow operation category {}",
                blocking_mode.as_str(),
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
    CloseTaskRequest,
);

fn request_envelope<T: HasEnvelope>(request: &T) -> &ToolEnvelope {
    request.envelope()
}

struct PreparedCoreRequest<T> {
    request: T,
    invocation: McpDerivedInvocationContext,
}

/// Returns the workflow-mode public Volicord method tool definitions.
pub fn public_method_tools() -> Vec<McpToolDefinition> {
    method_tools(PUBLIC_METHOD_TOOL_NAMES)
}

/// Returns adapter utility tool definitions.
pub fn adapter_utility_tools() -> Vec<McpToolDefinition> {
    ADAPTER_UTILITY_TOOL_NAMES
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        })
        .collect()
}

/// Returns workflow-mode MCP-visible tools.
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    mcp_tools_for_mode(AgentConnectionMode::Workflow)
}

/// Returns MCP-visible tools for the supplied Agent Connection mode.
pub fn mcp_tools_for_mode(mode: AgentConnectionMode) -> Vec<McpToolDefinition> {
    let mut tools = match mode {
        AgentConnectionMode::ReadOnly => method_tools(READ_ONLY_METHOD_TOOL_NAMES),
        AgentConnectionMode::Workflow => public_method_tools(),
    };
    tools.extend(adapter_utility_tools());
    tools
}

fn method_tools<const N: usize>(names: [&'static str; N]) -> Vec<McpToolDefinition> {
    names
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: mcp_visible_request_schema(name)
                .expect("public method schema should exist"),
        })
        .collect()
}

/// Runs a line-delimited JSON-RPC MCP stdio loop.
pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, mut writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let mut state = ConnectionState::AwaitingInitialize;

    for line in reader.lines() {
        let line = line.map_err(McpAdapterError::Io)?;
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                write_json_line(
                    &mut writer,
                    json_rpc_error(Value::Null, -32700, "Parse error", Some(error.to_string())),
                )?;
                continue;
            }
        };

        if let Some(response) = handle_json_rpc_message(&adapter, &mut state, message) {
            write_json_line(&mut writer, response)?;
        }
    }

    writer.flush().map_err(McpAdapterError::Io)
}

/// Runs the MCP stdio adapter from process environment and stdin/stdout.
pub fn run_stdio_from_env(connection_id: &str) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_runtime_home(process_env_var, &current_dir)?;
    let context = McpConnectionContext::resolve(&runtime_home, connection_id)?;
    let adapter = McpAdapter::new(runtime_home, context);
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio(adapter, stdin.lock(), stdout.lock())
}

/// Runs MCP startup validation from process environment.
pub fn run_preflight_check_from_env(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    preflight_check(process_env_var, &current_dir, connection_id, project_id)
}

/// Runs MCP startup validation from injected process inputs.
pub fn preflight_check<F>(
    env_var: F,
    current_dir: &Path,
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let detail_project_id = project_id.map(ProjectId::new);
    let inspection =
        McpConnectionStartupInspection::resolve(&runtime_home, connection_id, detail_project_id)?;
    Ok(inspection.preflight_report())
}

/// Resolves the Runtime Home used by the stdio entry point.
pub fn resolve_runtime_home_from_env<F>(env_var: F) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    resolve_runtime_home(env_var, &current_dir)
}

/// Resolves the Runtime Home from injected process inputs.
pub fn resolve_runtime_home<F>(env_var: F, current_dir: &Path) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    resolve_shared_runtime_home(env_var, current_dir).map_err(McpAdapterError::from)
}

fn current_dir_environment_error(error: io::Error) -> McpAdapterError {
    McpAdapterError::Environment(format!("failed to read current directory: {error}"))
}

fn process_env_var(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

fn resolve_connection_context(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
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
    validate_identifier_text("connection_id", connection_id)?;
    let connection = agent_connection_record(&runtime_home, connection_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment(format!("connection {connection_id} is not registered"))
        })?;
    let mode = validate_connection_record(&connection)?;
    let projects =
        list_connection_projects(&runtime_home, connection_id).map_err(McpAdapterError::Store)?;

    let context = McpConnectionContext {
        runtime_home,
        connection_id: AgentConnectionId::new(connection.connection_id.clone()),
        mode,
        invocation_binding_basis: DEFAULT_INVOCATION_BINDING_BASIS.to_owned(),
    };
    Ok((context, connection, projects))
}

fn validate_connection_record(
    connection: &AgentConnectionRecord,
) -> Result<AgentConnectionMode, McpAdapterError> {
    if !connection.enabled {
        return Err(McpAdapterError::Environment(format!(
            "connection {} is disabled",
            connection.connection_id
        )));
    }
    validate_identifier_text("connection_id", &connection.connection_id)?;
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

fn parse_connection_mode(mode: &str) -> Result<AgentConnectionMode, McpAdapterError> {
    match mode {
        CONNECTION_MODE_READ_ONLY => Ok(AgentConnectionMode::ReadOnly),
        CONNECTION_MODE_WORKFLOW => Ok(AgentConnectionMode::Workflow),
        other => Err(McpAdapterError::Environment(format!(
            "connection mode {other} is not supported for MCP startup"
        ))),
    }
}

fn current_enabled_connection(
    runtime_home: &Path,
    connection_id: &str,
    tool_name: &str,
) -> Result<AgentConnectionRecord, McpAdapterError> {
    let connection = agent_connection_record(runtime_home, connection_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| McpAdapterError::ToolExecution {
            tool_name: tool_name.to_owned(),
            message: format!("connection {connection_id} is not registered"),
        })?;
    validate_connection_record(&connection).map_err(|error| McpAdapterError::ToolExecution {
        tool_name: tool_name.to_owned(),
        message: error.to_string(),
    })?;
    Ok(connection)
}

fn inspect_allowed_project(
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

fn unavailable_project(
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

fn selected_project_from_availability(
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

fn routing_error(message: impl Into<String>) -> McpAdapterError {
    McpAdapterError::ToolExecution {
        tool_name: "project routing".to_owned(),
        message: message.into(),
    }
}

fn concise_store_reason(error: &StoreError) -> String {
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
        StoreError::MigrationConflict { database_kind, .. }
        | StoreError::SchemaInvariant { database_kind, .. } => {
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

fn controlled_invocation_binding_basis(value: &str) -> &'static str {
    match value.trim() {
        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING => {
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING
        }
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING => VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        _ => DEFAULT_INVOCATION_BINDING_BASIS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    AwaitingInitialize,
    AwaitingInitialized,
    Ready,
}

#[derive(Debug, PartialEq)]
enum ClientMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Debug, PartialEq)]
struct JsonRpcRequest {
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, PartialEq)]
struct JsonRpcNotification {
    method: String,
    params: Option<Value>,
}

#[derive(Debug, PartialEq)]
struct JsonRpcFailure {
    id: Value,
    code: i64,
    message: &'static str,
    data: Option<String>,
}

fn handle_json_rpc_message(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    message: Value,
) -> Option<Value> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            Some(handle_json_rpc_request(adapter, state, request))
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(state, notification);
            None
        }
        Err(error) => Some(json_rpc_error(
            error.id,
            error.code,
            error.message,
            error.data,
        )),
    }
}

fn parse_client_message(message: Value) -> Result<ClientMessage, JsonRpcFailure> {
    let object = match message {
        Value::Object(object) => object,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            return Err(invalid_request(
                Value::Null,
                "message must be a JSON object",
            ));
        }
    };

    let id = match object.get("id") {
        Some(value) => Some(valid_request_id(value)?),
        None => None,
    };
    let response_id = id.clone().unwrap_or(Value::Null);

    match object.get("jsonrpc") {
        Some(Value::String(version)) if version == "2.0" => (),
        _ => {
            return Err(invalid_request(
                response_id,
                "jsonrpc must be exactly \"2.0\"",
            ));
        }
    }

    let Some(Value::String(method)) = object.get("method") else {
        return Err(invalid_request(response_id, "method must be a string"));
    };
    let params = object.get("params").cloned();

    if let Some(id) = id {
        Ok(ClientMessage::Request(JsonRpcRequest {
            id,
            method: method.clone(),
            params,
        }))
    } else {
        Ok(ClientMessage::Notification(JsonRpcNotification {
            method: method.clone(),
            params,
        }))
    }
}

fn valid_request_id(value: &Value) -> Result<Value, JsonRpcFailure> {
    match value {
        Value::String(_) => Ok(value.clone()),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid_request(
                Value::Null,
                "id must be a string or integer",
            ))
        }
    }
}

fn handle_json_rpc_notification(state: &mut ConnectionState, notification: JsonRpcNotification) {
    if notification.method == "notifications/initialized"
        && *state == ConnectionState::AwaitingInitialized
        && notification_params_are_object_or_absent(notification.params.as_ref())
    {
        *state = ConnectionState::Ready;
    }
}

fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

fn handle_json_rpc_request(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    request: JsonRpcRequest,
) -> Value {
    if let Some(error) = lifecycle_error(*state, &request) {
        return error;
    }

    let response_id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => {
            if let Err(error) = validate_initialize_params(&response_id, request.params) {
                return error;
            }
            *state = ConnectionState::AwaitingInitialized;
            initialize_result()
        }
        "ping" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "ping")
            {
                return error;
            }
            json!({})
        }
        "tools/list" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "tools/list")
            {
                return error;
            }
            json!({ "tools": adapter.tools() })
        }
        "tools/call" => match call_tool_result(adapter, &response_id, request.params) {
            Ok(result) => result,
            Err(error) => return error,
        },
        _ => {
            return json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(request.method),
            )
        }
    };

    json!({
        "jsonrpc": "2.0",
        "id": response_id,
        "result": result
    })
}

fn lifecycle_error(state: ConnectionState, request: &JsonRpcRequest) -> Option<Value> {
    match state {
        ConnectionState::AwaitingInitialize if request.method != "initialize" => Some(
            invalid_request_response(&request.id, "initialize must be the first request"),
        ),
        ConnectionState::AwaitingInitialize => None,
        ConnectionState::AwaitingInitialized => match request.method.as_str() {
            "initialize" => Some(invalid_request_response(
                &request.id,
                "initialize has already completed",
            )),
            "tools/list" | "tools/call" => Some(invalid_request_response(
                &request.id,
                "connection is not ready",
            )),
            _ => None,
        },
        ConnectionState::Ready if request.method == "initialize" => Some(invalid_request_response(
            &request.id,
            "initialize has already completed",
        )),
        ConnectionState::Ready => None,
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": SERVER_INSTRUCTIONS
    })
}

fn validate_initialize_params(id: &Value, params: Option<Value>) -> Result<(), Value> {
    let object = required_object_params(id, params, "initialize")?;
    if !matches!(object.get("protocolVersion"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.protocolVersion must be a string",
        ));
    }
    if !matches!(object.get("capabilities"), Some(Value::Object(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.capabilities must be an object",
        ));
    }
    let Some(Value::Object(client_info)) = object.get("clientInfo") else {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo must be an object",
        ));
    };
    if !matches!(client_info.get("name"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo.name must be a string",
        ));
    }
    if !matches!(client_info.get("version"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo.version must be a string",
        ));
    }

    Ok(())
}

fn validate_optional_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<(), Value> {
    match params {
        None | Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

fn required_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<Map<String, Value>, Value> {
    match params {
        Some(Value::Object(object)) => Ok(object),
        None | Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

fn call_tool_result(
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
) -> Result<Value, Value> {
    let object = required_object_params(id, params, "tools/call")?;
    if object.contains_key("task") {
        return Err(invalid_params_response(
            id,
            "tools/call task augmentation is not supported",
        ));
    }

    let tool_name = object
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params_response(id, "tools/call params.name must be a string"))?;
    if !is_known_mcp_tool(tool_name) {
        return Err(json_rpc_error(
            id.clone(),
            -32602,
            "Invalid params",
            Some(format!("unknown MCP tool: {tool_name}")),
        ));
    }

    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            return Err(invalid_params_response(
                id,
                "tools/call params.arguments must be an object",
            ))
        }
    };
    let text = if PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) {
        match adapter.call_tool(tool_name, arguments) {
            Ok(response) => response.response_json,
            Err(error @ McpAdapterError::InvalidParams { .. })
            | Err(error @ McpAdapterError::ToolExecution { .. }) => {
                return Ok(tool_execution_error_result(&error));
            }
            Err(error) => return Err(json_rpc_error_for_adapter(id.clone(), error)),
        }
    } else {
        let response = match adapter.call_adapter_tool(tool_name, arguments) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. })
            | Err(error @ McpAdapterError::ToolExecution { .. }) => {
                return Ok(tool_execution_error_result(&error));
            }
            Err(error) => return Err(json_rpc_error_for_adapter(id.clone(), error)),
        };
        serde_json::to_string(&response)
            .map_err(McpAdapterError::Json)
            .map_err(|error| json_rpc_error_for_adapter(id.clone(), error))?
    };

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": false
    }))
}

fn is_known_mcp_tool(tool_name: &str) -> bool {
    PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) || ADAPTER_UTILITY_TOOL_NAMES.contains(&tool_name)
}

fn tool_execution_error_result(error: &McpAdapterError) -> Value {
    let text = match error {
        McpAdapterError::InvalidParams { tool_name, source } => {
            format!("Invalid arguments for {tool_name}: {source}. Check the tool input schema and retry.")
        }
        McpAdapterError::ToolExecution { tool_name, message } if tool_name == "project routing" => {
            format!("{message}. Use volicord.list_projects when project selection is unclear.")
        }
        McpAdapterError::ToolExecution { tool_name, message } => {
            format!("{tool_name} failed before reaching Core: {message}")
        }
        _ => "Tool execution failed before reaching Core.".to_owned(),
    };

    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": true
    })
}

fn json_rpc_error_for_adapter(id: Value, error: McpAdapterError) -> Value {
    let (code, message) = match error {
        McpAdapterError::UnknownTool(_) | McpAdapterError::InvalidParams { .. } => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Protocol(_)
        | McpAdapterError::Environment(_)
        | McpAdapterError::ToolExecution { .. } => (-32602, "Invalid params"),
        McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

fn invalid_request(id: Value, data: impl Into<String>) -> JsonRpcFailure {
    JsonRpcFailure {
        id,
        code: -32600,
        message: "Invalid Request",
        data: Some(data.into()),
    }
}

fn invalid_request_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32600, "Invalid Request", Some(data.into()))
}

fn invalid_params_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32602, "Invalid params", Some(data.into()))
}

fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<String>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message
    });
    if let Some(data) = data {
        error["data"] = Value::String(data);
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error
    })
}

fn write_json_line(writer: &mut impl Write, value: Value) -> Result<(), McpAdapterError> {
    serde_json::to_writer(&mut *writer, &value).map_err(McpAdapterError::Json)?;
    writer.write_all(b"\n").map_err(McpAdapterError::Io)
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "volicord.intake" => "Start, resume, supersede, or reject an ordinary user work loop.",
        "volicord.update_scope" => "Update current Task scope and Change Unit state.",
        "volicord.status" => "Read the current Core status view.",
        "volicord.prepare_write" => "Check one proposed product-file write against Core state.",
        "volicord.stage_artifact" => "Stage safe artifact bytes or a safe notice.",
        "volicord.record_run" => "Record shaping, direct, or implementation work.",
        "volicord.request_user_judgment" => "Create one pending focused user-owned judgment.",
        "volicord.close_task" => "Check or perform a selected Task close path.",
        LIST_PROJECTS_TOOL_NAME => "List projects explicitly allowed for this MCP connection.",
        _ => "Unsupported Volicord method.",
    }
}

fn optional_string_field(
    object: &Map<String, Value>,
    field: &'static str,
    tool_name: &str,
) -> Result<Option<String>, McpAdapterError> {
    match object.get(field) {
        None => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        Some(_) => Err(McpAdapterError::ToolExecution {
            tool_name: tool_name.to_owned(),
            message: format!("envelope.{field} must be a non-empty string when supplied"),
        }),
    }
}

fn mcp_visible_request_schema(method_name: &str) -> Option<Value> {
    let mut schema = public_request_schema(method_name)?;
    apply_mcp_visible_envelope_contract(&mut schema);
    Some(schema)
}

fn apply_mcp_visible_envelope_contract(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            if is_tool_envelope_schema(object) {
                if let Some(Value::Array(required)) = object.get_mut("required") {
                    required.retain(|value| !matches!(value.as_str(), Some("project_id")));
                }
            }
            for value in object.values_mut() {
                apply_mcp_visible_envelope_contract(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                apply_mcp_visible_envelope_contract(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn is_tool_envelope_schema(object: &Map<String, Value>) -> bool {
    let Some(Value::Object(properties)) = object.get("properties") else {
        return false;
    };
    [
        "project_id",
        "task_id",
        "request_id",
        "idempotency_key",
        "expected_state_version",
        "dry_run",
        "locale",
    ]
    .iter()
    .all(|field| properties.contains_key(*field))
}

fn reject_caller_owned_invocation_fields(
    object: &Map<String, Value>,
    tool_name: &str,
) -> Result<(), McpAdapterError> {
    for field in [
        "actor_source",
        "operation_category",
        "mode",
        "connection_id",
    ] {
        if object.contains_key(field) {
            return Err(McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: format!("{field} is supplied by the bound MCP connection and must not be included in MCP tool arguments"),
            });
        }
    }
    Ok(())
}

fn validate_identifier_text(field: &'static str, value: &str) -> Result<(), McpAdapterError> {
    if value.trim().is_empty() {
        return Err(McpAdapterError::Environment(format!(
            "{field} must not be empty"
        )));
    }
    if value.contains('\0') {
        return Err(McpAdapterError::Environment(format!(
            "{field} must not contain NUL bytes"
        )));
    }
    Ok(())
}

/// Adapter and stdio errors that occur before or outside public Core responses.
#[derive(Debug)]
pub enum McpAdapterError {
    UnknownTool(String),
    InvalidParams {
        tool_name: String,
        source: serde_json::Error,
    },
    ToolExecution {
        tool_name: String,
        message: String,
    },
    Core(CorePipelineError),
    Store(StoreError),
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Environment(String),
}

impl fmt::Display for McpAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(tool_name) => write!(formatter, "unknown MCP tool: {tool_name}"),
            Self::InvalidParams { tool_name, source } => {
                write!(formatter, "invalid params for {tool_name}: {source}")
            }
            Self::ToolExecution { tool_name, message } => {
                write!(formatter, "{tool_name}: {message}")
            }
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Store(error) => write!(formatter, "store error: {error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Protocol(message) | Self::Environment(message) => formatter.write_str(message),
        }
    }
}

impl Error for McpAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidParams { source, .. } => Some(source),
            Self::Core(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnknownTool(_)
            | Self::ToolExecution { .. }
            | Self::Protocol(_)
            | Self::Environment(_) => None,
        }
    }
}

impl From<RuntimeHomeResolutionError> for McpAdapterError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Environment(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        error::Error,
        io::{BufReader, Cursor},
    };

    use volicord_core::CoreBoundary;
    use volicord_store::agent_connections::{
        agent_connection_record, ensure_agent_connection, AgentConnectionRegistration,
        CONNECTION_MODE_READ_ONLY,
    };
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{
        AgentConnectionMode, OperationCategory, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    };

    use super::*;

    #[test]
    fn mcp_boundary_wraps_core_boundary() {
        assert_eq!(
            McpAdapterBoundary::new(CoreBoundary::new()).label(),
            "mcp-adapter"
        );
    }

    #[test]
    fn tool_sets_follow_connection_mode_and_exclude_user_only_recording() {
        let workflow = mcp_tools_for_mode(AgentConnectionMode::Workflow);
        let workflow_names = tool_names(&workflow);
        assert_eq!(
            &workflow_names[..PUBLIC_METHOD_TOOL_NAMES.len()],
            PUBLIC_METHOD_TOOL_NAMES
        );
        assert!(workflow_names.contains(&"volicord.request_user_judgment"));
        assert!(!workflow_names.contains(&"volicord.record_user_judgment"));
        assert_eq!(
            workflow_names.last().copied(),
            Some(LIST_PROJECTS_TOOL_NAME)
        );

        let read_only = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
        let read_only_names = tool_names(&read_only);
        assert_eq!(
            read_only_names,
            vec![
                "volicord.status",
                "volicord.close_task",
                LIST_PROJECTS_TOOL_NAME
            ]
        );
    }

    #[test]
    fn mcp_visible_schemas_make_project_selector_optional() {
        for tool in public_method_tools() {
            let required = envelope_required_fields(&tool.input_schema)
                .expect("tool schema should contain ToolEnvelope schema");
            assert!(
                !required.contains(&"project_id".to_owned()),
                "{} should not require envelope.project_id from MCP callers",
                tool.name
            );
            assert!(
                schema_has_property(&tool.input_schema, "project_id"),
                "{} should still expose envelope.project_id as an optional selector",
                tool.name
            );
            for forbidden in [
                "actor_source",
                "operation_category",
                "mode",
                "connection_id",
            ] {
                assert!(
                    !schema_has_property(&tool.input_schema, forbidden),
                    "{} should not expose invocation-only field {forbidden}",
                    tool.name
                );
            }
        }
    }

    #[test]
    fn connection_context_resolves_and_preflight_reports_allowed_project(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-context")?;

        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        assert_eq!(context.connection_id.as_str(), fixture.connection_id());
        assert_eq!(context.mode, AgentConnectionMode::Workflow);

        let report = preflight_check(
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(fixture.runtime_home_path().as_os_str().to_owned())
                } else {
                    None
                }
            },
            fixture.runtime_home_path(),
            fixture.connection_id(),
            None,
        )?;
        assert!(report.contains(&format!("connection_id: {}", fixture.connection_id())));
        assert!(report.contains("mode: workflow"));
        assert!(report.contains("allowed_projects: 1"));
        assert!(report.contains("available_projects: 1"));
        Ok(())
    }

    #[test]
    fn adapter_auto_selects_single_project_and_injects_connection_invocation(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-auto-select")?;
        let adapter = adapter(&fixture)?;
        let mut params = serde_json::to_value(fixture.status_request("req_mcp_status", None))?;
        params["envelope"]
            .as_object_mut()
            .expect("envelope object")
            .remove("project_id");

        let response = adapter.call_tool("volicord.status", params)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        let verified = response
            .verified_invocation
            .expect("Core should verify adapter invocation");
        assert_eq!(verified.project_id.as_str(), fixture.project_id());
        assert_eq!(verified.actor_source.to_string(), fixture.actor_source());
        assert_eq!(verified.operation_category, OperationCategory::Read);
        Ok(())
    }

    #[test]
    fn read_only_mode_rejects_agent_workflow_calls_before_core() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-read-only")?;
        set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
        let adapter = adapter(&fixture)?;
        let before = fixture.counts()?;

        let error = adapter
            .call_tool(
                "volicord.intake",
                serde_json::to_value(fixture.intake_request(
                    "req_mcp_read_only_intake",
                    "idem_mcp_read_only_intake",
                    false,
                    Some(0),
                ))?,
            )
            .expect_err("read_only should reject agent workflow calls");

        assert!(error.to_string().contains("mode read_only"));
        assert!(error.to_string().contains("agent_workflow"));
        assert_eq!(fixture.counts()?, before);
        Ok(())
    }

    #[test]
    fn stdio_lists_mode_filtered_tools() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-stdio-mode")?;
        set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        let names = responses[1]["result"]["tools"]
            .as_array()
            .expect("tools should be an array")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "volicord.status",
                "volicord.close_task",
                LIST_PROJECTS_TOOL_NAME
            ]
        );
        Ok(())
    }

    fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
                .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        Ok(McpAdapter::new(fixture.runtime_home_path(), context))
    }

    fn set_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
        let existing =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
                .expect("fixture connection should exist");
        ensure_agent_connection(
            fixture.runtime_home_path(),
            AgentConnectionRegistration {
                connection_id: existing.connection_id,
                host_kind: existing.host_kind,
                host_scope: existing.host_scope,
                server_name: existing.server_name,
                config_target: existing.config_target,
                mode: mode.to_owned(),
                enabled: existing.enabled,
                managed_fingerprint: existing.managed_fingerprint,
                last_verified_status: existing.last_verified_status,
                metadata_json: existing.metadata_json,
            },
        )?;
        Ok(())
    }

    fn tool_names(tools: &[McpToolDefinition]) -> Vec<&'static str> {
        tools.iter().map(|tool| tool.name).collect::<Vec<_>>()
    }

    fn schema_has_property(schema: &Value, property_name: &str) -> bool {
        match schema {
            Value::Object(object) => {
                object
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|properties| properties.contains_key(property_name))
                    || object
                        .values()
                        .any(|child| schema_has_property(child, property_name))
            }
            Value::Array(items) => items
                .iter()
                .any(|child| schema_has_property(child, property_name)),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
        }
    }

    fn envelope_required_fields(schema: &Value) -> Option<Vec<String>> {
        match schema {
            Value::Object(object) => {
                if is_tool_envelope_schema(object) {
                    return object
                        .get("required")
                        .and_then(Value::as_array)
                        .map(|required| {
                            required
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect::<Vec<_>>()
                        });
                }
                object.values().find_map(envelope_required_fields)
            }
            Value::Array(items) => items.iter().find_map(envelope_required_fields),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
        }
    }

    fn stdio_responses(output: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
        let text = std::str::from_utf8(output)?;
        let mut responses = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            responses.push(serde_json::from_str(line)?);
        }
        Ok(responses)
    }

    #[test]
    fn workflow_public_tool_names_are_unique() {
        let unique = PUBLIC_METHOD_TOOL_NAMES
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), PUBLIC_METHOD_TOOL_NAMES.len());
    }
}
