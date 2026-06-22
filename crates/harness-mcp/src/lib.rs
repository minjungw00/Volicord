#![forbid(unsafe_code)]

//! Local MCP adapter for public Harness method calls.
//!
//! This crate owns only transport dispatch: it registers the documented tools,
//! decodes tool arguments into `harness-types` request structs, derives local
//! invocation facts from adapter context, and hands execution to `harness-core`.

use std::{
    error::Error,
    ffi::OsString,
    fmt,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use harness_core::{
    rejected_response, tool_error, AdapterSessionBinding, CoreBoundary, CorePipelineError,
    CoreService, InvocationContext, PipelineResponse,
};
use harness_store::{
    agent_integrations::{
        agent_integration_project_access, agent_integration_record, list_integration_projects,
        AgentIntegrationRecord, IntegrationProjectRecord, AGENT_INTERACTION_ROLE,
    },
    bootstrap::{runtime_home_record, SurfaceRecord, ACTIVE_PROJECT_STATUS},
    core_pipeline::CoreProjectStore,
    runtime_home::{
        resolve_runtime_home as resolve_shared_runtime_home, RuntimeHomeResolutionError,
    },
    StoreError,
};
use harness_types::{
    public_request_schema, AccessClass, CloseTaskRequest, ErrorCode, IntakeRequest,
    MethodAccessClass, PrepareWriteRequest, ProjectId, RecordRunRequest, RecordUserJudgmentRequest,
    RequestUserJudgmentRequest, StageArtifactRequest, StatusRequest, SurfaceId, SurfaceInstanceId,
    SurfaceInteractionRole, ToolEnvelope, ToolError, UpdateScopeRequest,
    BASELINE_WORKFLOW_ACCESS_CLASSES, VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};
use serde::Serialize;
use serde_json::{json, Map, Value};

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-11-25";
const SERVER_NAME: &str = "harness-mcp";
const DEFAULT_INVOCATION_BINDING_BASIS: &str = VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING;

/// The exact public Harness method tools exposed through MCP.
pub const PUBLIC_METHOD_TOOL_NAMES: [&str; 9] = [
    "harness.intake",
    "harness.update_scope",
    "harness.status",
    "harness.prepare_write",
    "harness.stage_artifact",
    "harness.record_run",
    "harness.request_user_judgment",
    "harness.record_user_judgment",
    "harness.close_task",
];

/// Adapter-owned MCP utility tools that are not public Harness Core methods.
pub const ADAPTER_UTILITY_TOOL_NAMES: [&str; 1] = ["harness.list_projects"];

const LIST_PROJECTS_TOOL_NAME: &str = "harness.list_projects";
const SERVER_INSTRUCTIONS: &str = "Harness records task scope, write readiness, evidence, runs, user-owned judgments, artifacts, and close readiness for explicitly registered Product Repositories. If project selection is unclear, call harness.list_projects and use one listed project_id; do not guess from folders, roots, labels, or memory. Harness state management is separate from permission to edit product files: product-file edits still require the host/user path and any required Write Authorization. These instructions are guidance, not access control or a promise of automatic tool use.";

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

/// Integration-bound adapter facts that are not accepted from tool arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpIntegrationContext {
    pub runtime_home: PathBuf,
    pub integration_id: String,
    pub interaction_role: SurfaceInteractionRole,
    pub surface_id: SurfaceId,
    pub surface_instance_id: SurfaceInstanceId,
    pub invocation_binding_basis: String,
}

impl McpIntegrationContext {
    /// Resolves and validates one Agent Integration Profile startup binding.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        integration_id: impl Into<String>,
    ) -> Result<Self, McpAdapterError> {
        let integration_id = integration_id.into();
        let (context, _, _) = resolve_integration_context(runtime_home, &integration_id)?;
        Ok(context)
    }

    /// Replaces the controlled adapter-binding basis carried into Core.
    pub fn with_invocation_binding_basis(mut self, basis: impl Into<String>) -> Self {
        let basis = basis.into();
        self.invocation_binding_basis = controlled_invocation_binding_basis(&basis).to_owned();
        self
    }
}

/// Integration-bound startup facts shared by stdio startup and preflight checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpIntegrationStartupInspection {
    pub runtime_home: PathBuf,
    pub integration_id: String,
    pub interaction_role: SurfaceInteractionRole,
    pub surface_id: SurfaceId,
    pub surface_instance_id: SurfaceInstanceId,
    pub enabled: bool,
    pub allowed_project_count: usize,
    pub default_project_id: Option<ProjectId>,
    pub projects: Vec<McpProjectAvailability>,
}

impl McpIntegrationStartupInspection {
    /// Resolves process inputs and validates one integration-bound MCP binding.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        integration_id: impl Into<String>,
        detail_project_id: Option<ProjectId>,
    ) -> Result<Self, McpAdapterError> {
        let integration_id = integration_id.into();
        let (context, integration, projects) =
            resolve_integration_context(runtime_home, &integration_id)?;
        let selected_projects = if let Some(project_id) = detail_project_id {
            if !projects
                .iter()
                .any(|project| project.project_id == project_id.as_str())
            {
                return Err(McpAdapterError::Environment(format!(
                    "project {} is not allowed for integration {}",
                    project_id.as_str(),
                    integration.integration_id
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
            .map(|project| inspect_allowed_project(&context, &integration, project, None))
            .collect::<Vec<_>>();

        Ok(Self {
            runtime_home: context.runtime_home.clone(),
            integration_id: integration.integration_id,
            interaction_role: context.interaction_role,
            surface_id: context.surface_id,
            surface_instance_id: context.surface_instance_id,
            enabled: integration.enabled,
            allowed_project_count: projects.len(),
            default_project_id: integration.default_project_id.map(ProjectId::new),
            projects: project_reports,
        })
    }

    /// Returns the public integration context consumed by the stdio adapter.
    pub fn integration_context(&self) -> McpIntegrationContext {
        McpIntegrationContext {
            runtime_home: self.runtime_home.clone(),
            integration_id: self.integration_id.clone(),
            interaction_role: self.interaction_role,
            surface_id: self.surface_id.clone(),
            surface_instance_id: self.surface_instance_id.clone(),
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
        let default_project_id = self
            .default_project_id
            .as_ref()
            .map(ProjectId::as_str)
            .unwrap_or("");
        let mut report = format!(
            "configuration: valid\ntransport: stdio\nruntime_home: {}\nintegration_id: {}\ninteraction_role: {}\nsurface_id: {}\nsurface_instance_id: {}\nenabled: {}\nallowed_projects: {}\navailable_projects: {}\ndefault_project_id: {}\nverification_scope: startup_check_only\n",
            self.runtime_home.display(),
            self.integration_id,
            self.interaction_role.as_str(),
            self.surface_id.as_str(),
            self.surface_instance_id.as_str(),
            self.enabled,
            self.allowed_project_count,
            available_projects,
            default_project_id
        );
        for (index, project) in self.projects.iter().enumerate() {
            let missing = project
                .missing_access_classes
                .iter()
                .map(|access_class| access_class.as_str())
                .collect::<Vec<_>>()
                .join(",");
            report.push_str(&format!(
                "project[{index}].project_id: {}\nproject[{index}].default: {}\nproject[{index}].available: {}\nproject[{index}].unavailable_reason: {}\nproject[{index}].repo_root: {}\nproject[{index}].baseline_workflow_access: {}\nproject[{index}].missing_access_classes: {}\n",
                project.project_id,
                project.is_default,
                project.available,
                project.unavailable_reason.as_deref().unwrap_or(""),
                project.repo_root_display,
                project.baseline_workflow_access,
                missing
            ));
        }
        report
    }
}

/// MCP-visible availability facts for one integration-allowed project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpProjectAvailability {
    pub project_id: String,
    pub is_default: bool,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub repo_root_display: String,
    pub baseline_workflow_access: String,
    pub missing_access_classes: Vec<AccessClass>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ListProjectsResult {
    integration_id: String,
    default_project_id: Option<String>,
    projects: Vec<ListProjectItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ListProjectItem {
    project_id: String,
    is_default: bool,
    available: bool,
    unavailable_reason: Option<String>,
    repo_root: String,
}

/// Invocation context derived for one tool call before entering Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDerivedInvocationContext {
    pub project_id: ProjectId,
    pub surface_id: SurfaceId,
    pub surface_instance_id: SurfaceInstanceId,
    pub requested_access_class: AccessClass,
    pub invocation_binding_basis: String,
}

impl McpDerivedInvocationContext {
    fn core_invocation(&self) -> InvocationContext {
        InvocationContext {
            binding: AdapterSessionBinding::new(
                self.project_id.clone(),
                self.surface_id.clone(),
                self.surface_instance_id.clone(),
                self.invocation_binding_basis.clone(),
            ),
            requested_access_class: self.requested_access_class,
        }
    }
}

/// Local MCP adapter bound to a Core service and one Agent Integration Profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAdapter {
    core: CoreService,
    runtime_home: PathBuf,
    context: McpIntegrationContext,
}

impl McpAdapter {
    /// Creates an adapter for a Runtime Home and integration-bound adapter context.
    pub fn new(runtime_home: impl AsRef<Path>, context: McpIntegrationContext) -> Self {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        Self {
            core: CoreService::new(&runtime_home),
            runtime_home,
            context,
        }
    }

    /// Returns the public Harness method tools and adapter utility tools exposed by this adapter.
    pub fn tools(&self) -> Vec<McpToolDefinition> {
        mcp_tools()
    }

    /// Derives local invocation facts for one request envelope.
    pub fn derive_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        requested_access_class: AccessClass,
    ) -> Result<McpDerivedInvocationContext, ToolError> {
        let context = &self.context;
        if envelope.surface_id != context.surface_id {
            return Err(local_access_mismatch_error("envelope.surface_id"));
        }

        Ok(McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            surface_id: context.surface_id.clone(),
            surface_instance_id: context.surface_instance_id.clone(),
            requested_access_class,
            invocation_binding_basis: context.invocation_binding_basis.clone(),
        })
    }

    /// Calls one public Harness method tool and returns Core's response.
    pub fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        match tool_name {
            "harness.intake" => {
                let prepared: PreparedCoreRequest<IntakeRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .intake(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.update_scope" => {
                let prepared: PreparedCoreRequest<UpdateScopeRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .update_scope(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.status" => {
                let prepared: PreparedCoreRequest<StatusRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .status(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.prepare_write" => {
                let prepared: PreparedCoreRequest<PrepareWriteRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .prepare_write(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.stage_artifact" => {
                let prepared: PreparedCoreRequest<StageArtifactRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .stage_artifact(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.record_run" => {
                let prepared: PreparedCoreRequest<RecordRunRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .record_run(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.request_user_judgment" => {
                let prepared: PreparedCoreRequest<RequestUserJudgmentRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .request_user_judgment(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.record_user_judgment" => {
                let prepared: PreparedCoreRequest<RecordUserJudgmentRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .record_user_judgment(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.close_task" => {
                let prepared: PreparedCoreRequest<CloseTaskRequest> =
                    self.prepare_typed_request(tool_name, params)?;
                let invocation = match prepared.invocation {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(prepared.request.envelope.dry_run, error)
                    }
                };
                self.core
                    .close_task(prepared.request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn call_adapter_tool(&self, tool_name: &str, params: Value) -> Result<Value, McpAdapterError> {
        match tool_name {
            LIST_PROJECTS_TOOL_NAME => {
                let object = params
                    .as_object()
                    .ok_or_else(|| McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: "harness.list_projects arguments must be an object".to_owned(),
                    })?;
                if !object.is_empty() {
                    return Err(McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: "harness.list_projects does not accept arguments".to_owned(),
                    });
                }
                let result = self.list_projects_result()?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn list_projects_result(&self) -> Result<ListProjectsResult, McpAdapterError> {
        let context = &self.context;
        let integration = current_enabled_integration(&self.runtime_home, &context.integration_id)?;
        let projects = list_integration_projects(&self.runtime_home, &context.integration_id)
            .map_err(McpAdapterError::Store)?;
        let items = projects
            .iter()
            .map(|project| inspect_allowed_project(context, &integration, project, None))
            .map(|project| ListProjectItem {
                project_id: project.project_id,
                is_default: project.is_default,
                available: project.available,
                unavailable_reason: project.unavailable_reason,
                repo_root: project.repo_root_display,
            })
            .collect::<Vec<_>>();

        Ok(ListProjectsResult {
            integration_id: context.integration_id.clone(),
            default_project_id: integration.default_project_id,
            projects: items,
        })
    }

    fn prepare_typed_request<T>(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PreparedCoreRequest<T>, McpAdapterError>
    where
        T: serde::de::DeserializeOwned + MethodAccessClass,
    {
        let requested_access_class = raw_requested_access_class(tool_name, &params)?;
        let (prepared_params, invocation) =
            self.prepare_integration_arguments(tool_name, params, requested_access_class)?;
        let request: T = self.decode_params(tool_name, prepared_params)?;
        Ok(PreparedCoreRequest {
            request,
            invocation: Ok(invocation),
        })
    }

    fn prepare_integration_arguments(
        &self,
        tool_name: &str,
        mut params: Value,
        requested_access_class: AccessClass,
    ) -> Result<(Value, McpDerivedInvocationContext), McpAdapterError> {
        let context = &self.context;
        let object = params
            .as_object_mut()
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "tool arguments must be an object containing an envelope object"
                    .to_owned(),
            })?;
        let envelope = object
            .get_mut("envelope")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "public Harness tool arguments require an envelope object".to_owned(),
            })?;
        let requested_project_id = optional_string_field(envelope, "project_id", tool_name)?;
        if let Some(surface_id) = optional_string_field(envelope, "surface_id", tool_name)? {
            if surface_id != context.surface_id.as_str() {
                return Err(McpAdapterError::ToolExecution {
                    tool_name: tool_name.to_owned(),
                    message: "envelope.surface_id does not match the integration-bound surface"
                        .to_owned(),
                });
            }
        }

        let selected = self.select_project(
            context,
            requested_project_id.as_deref(),
            requested_access_class,
        )?;
        envelope.insert(
            "project_id".to_owned(),
            Value::String(selected.project_id.as_str().to_owned()),
        );
        envelope.insert(
            "surface_id".to_owned(),
            Value::String(context.surface_id.as_str().to_owned()),
        );

        Ok((params, selected))
    }

    fn select_project(
        &self,
        context: &McpIntegrationContext,
        requested_project_id: Option<&str>,
        requested_access_class: AccessClass,
    ) -> Result<McpDerivedInvocationContext, McpAdapterError> {
        let integration = current_enabled_integration(&self.runtime_home, &context.integration_id)?;

        if let Some(project_id) = requested_project_id {
            let access = agent_integration_project_access(
                &self.runtime_home,
                &context.integration_id,
                project_id,
            )
            .map_err(McpAdapterError::Store)?
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: "project routing".to_owned(),
                message: format!("integration {} is not registered", context.integration_id),
            })?;
            if !access.integration_enabled {
                return Err(routing_error("integration is disabled"));
            }
            if !access.project_allowed {
                return Err(routing_error(format!(
                    "project {project_id} is not allowed for this integration"
                )));
            }
            let project = access
                .project
                .ok_or_else(|| routing_error(format!("project {project_id} is not registered")))?;
            let project_record = IntegrationProjectRecord {
                integration_id: context.integration_id.clone(),
                project_id: project_id.to_owned(),
                created_at: String::new(),
                is_default: access.is_default,
                project,
            };
            let availability = inspect_allowed_project(
                context,
                &integration,
                &project_record,
                Some(requested_access_class),
            );
            return selected_project_from_availability(
                context,
                availability,
                requested_access_class,
            );
        }

        let projects = list_integration_projects(&self.runtime_home, &context.integration_id)
            .map_err(McpAdapterError::Store)?;
        if projects.is_empty() {
            return Err(routing_error(
                "integration has no allowed projects; ask the operator to add one",
            ));
        }
        let availabilities = projects
            .iter()
            .map(|project| {
                inspect_allowed_project(
                    context,
                    &integration,
                    project,
                    Some(requested_access_class),
                )
            })
            .collect::<Vec<_>>();
        let available = availabilities
            .iter()
            .filter(|project| project.available)
            .collect::<Vec<_>>();
        if available.len() == 1 {
            return selected_project_from_availability(
                context,
                (*available[0]).clone(),
                requested_access_class,
            );
        }
        if let Some(default_project_id) = &integration.default_project_id {
            if let Some(default) = availabilities
                .iter()
                .find(|project| project.project_id == *default_project_id && project.available)
            {
                return selected_project_from_availability(
                    context,
                    default.clone(),
                    requested_access_class,
                );
            }
        }

        Err(routing_error(
            "project selection is ambiguous; call harness.list_projects and retry with envelope.project_id",
        ))
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

struct PreparedCoreRequest<T> {
    request: T,
    invocation: Result<McpDerivedInvocationContext, ToolError>,
}

/// Returns the exact public Harness method tool definitions.
pub fn public_method_tools() -> Vec<McpToolDefinition> {
    PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: mcp_visible_request_schema(name)
                .expect("public method schema should exist"),
        })
        .collect()
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

/// Returns all MCP-visible tools.
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    let mut tools = public_method_tools();
    tools.extend(adapter_utility_tools());
    tools
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
pub fn run_stdio_from_env(integration_id: &str) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_runtime_home(process_env_var, &current_dir)?;
    let context = McpIntegrationContext::resolve(&runtime_home, integration_id)?;
    let adapter = McpAdapter::new(runtime_home, context);
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio(adapter, stdin.lock(), stdout.lock())
}

/// Runs MCP startup validation from process environment.
pub fn run_preflight_check_from_env(
    integration_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    preflight_check(process_env_var, &current_dir, integration_id, project_id)
}

/// Runs MCP startup validation from injected process inputs.
pub fn preflight_check<F>(
    env_var: F,
    current_dir: &Path,
    integration_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let detail_project_id = project_id.map(ProjectId::new);
    let inspection =
        McpIntegrationStartupInspection::resolve(&runtime_home, integration_id, detail_project_id)?;
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

fn resolve_integration_context(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
) -> Result<
    (
        McpIntegrationContext,
        AgentIntegrationRecord,
        Vec<IntegrationProjectRecord>,
    ),
    McpAdapterError,
> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    runtime_home_record(&runtime_home)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment("Runtime Home is not initialized".to_owned())
        })?;
    validate_identifier_text("integration_id", integration_id)?;
    let integration = agent_integration_record(&runtime_home, integration_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment(format!("integration {integration_id} is not registered"))
        })?;
    let interaction_role = validate_integration_record(&integration)?;
    let projects =
        list_integration_projects(&runtime_home, integration_id).map_err(McpAdapterError::Store)?;
    if projects.is_empty() {
        return Err(McpAdapterError::Environment(format!(
            "integration {integration_id} has no allowed projects"
        )));
    }

    let context = McpIntegrationContext {
        runtime_home,
        integration_id: integration.integration_id.clone(),
        interaction_role,
        surface_id: SurfaceId::new(integration.surface_id.clone()),
        surface_instance_id: SurfaceInstanceId::new(integration.surface_instance_id.clone()),
        invocation_binding_basis: DEFAULT_INVOCATION_BINDING_BASIS.to_owned(),
    };
    Ok((context, integration, projects))
}

fn validate_integration_record(
    integration: &AgentIntegrationRecord,
) -> Result<SurfaceInteractionRole, McpAdapterError> {
    if !integration.enabled {
        return Err(McpAdapterError::Environment(format!(
            "integration {} is disabled",
            integration.integration_id
        )));
    }
    validate_identifier_text("surface_id", &integration.surface_id)?;
    validate_identifier_text("surface_instance_id", &integration.surface_instance_id)?;
    match serde_json::from_str::<Value>(&integration.metadata_json) {
        Ok(Value::Object(_)) => (),
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered integration metadata is not an object".to_owned(),
            ))
        }
        Err(error) => return Err(McpAdapterError::Json(error)),
    }
    match integration.interaction_role.as_str() {
        AGENT_INTERACTION_ROLE => Ok(SurfaceInteractionRole::Agent),
        _ => Err(McpAdapterError::Environment(format!(
            "integration role {} is not supported for MCP agent startup",
            integration.interaction_role
        ))),
    }
}

fn current_enabled_integration(
    runtime_home: &Path,
    integration_id: &str,
) -> Result<AgentIntegrationRecord, McpAdapterError> {
    let integration = agent_integration_record(runtime_home, integration_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| McpAdapterError::ToolExecution {
            tool_name: "project routing".to_owned(),
            message: format!("integration {integration_id} is not registered"),
        })?;
    validate_integration_record(&integration).map_err(|error| McpAdapterError::ToolExecution {
        tool_name: "project routing".to_owned(),
        message: error.to_string(),
    })?;
    Ok(integration)
}

fn inspect_allowed_project(
    context: &McpIntegrationContext,
    integration: &AgentIntegrationRecord,
    project: &IntegrationProjectRecord,
    requested_access_class: Option<AccessClass>,
) -> McpProjectAvailability {
    let repo_root_display = project.project.repo_root.display().to_string();
    if project.project.status != ACTIVE_PROJECT_STATUS {
        return unavailable_project(
            project,
            repo_root_display,
            "project is not active",
            Vec::new(),
        );
    }
    let store =
        match CoreProjectStore::open(&context.runtime_home, &ProjectId::new(&project.project_id)) {
            Ok(store) => store,
            Err(error) => {
                return unavailable_project(
                    project,
                    repo_root_display,
                    format!(
                        "project is not executable: {}",
                        concise_store_reason(&error)
                    ),
                    Vec::new(),
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
            Vec::new(),
        );
    }
    let surface = match store.surface(&context.surface_id, context.surface_instance_id.as_str()) {
        Ok(Some(surface)) => surface,
        Ok(None) => {
            return unavailable_project(
                project,
                repo_root_display,
                "integration surface instance is not registered for this project",
                Vec::new(),
            )
        }
        Err(error) => {
            return unavailable_project(
                project,
                repo_root_display,
                format!("surface lookup failed: {}", concise_store_reason(&error)),
                Vec::new(),
            )
        }
    };
    let startup_surface = match valid_startup_surface(surface) {
        Ok(surface) => surface,
        Err(error) => {
            return unavailable_project(
                project,
                repo_root_display,
                format!("integration surface is invalid: {error}"),
                Vec::new(),
            )
        }
    };
    if startup_surface.interaction_role != SurfaceInteractionRole::Agent
        || integration.interaction_role != AGENT_INTERACTION_ROLE
    {
        return unavailable_project(
            project,
            repo_root_display,
            "integration surface role is not agent",
            startup_surface.access_classes,
        );
    }
    if let Some(access_class) = requested_access_class {
        if !startup_surface.access_classes.contains(&access_class) {
            return unavailable_project(
                project,
                repo_root_display,
                format!(
                    "requested access class {} is not authorized for this surface instance",
                    access_class.as_str()
                ),
                startup_surface.access_classes,
            );
        }
    }
    let missing = missing_baseline_access(&startup_surface.access_classes);
    McpProjectAvailability {
        project_id: project.project_id.clone(),
        is_default: project.is_default,
        available: true,
        unavailable_reason: None,
        repo_root_display,
        baseline_workflow_access: if missing.is_empty() {
            "full".to_owned()
        } else {
            "partial".to_owned()
        },
        missing_access_classes: missing,
    }
}

fn unavailable_project(
    project: &IntegrationProjectRecord,
    repo_root_display: String,
    reason: impl Into<String>,
    access_classes: Vec<AccessClass>,
) -> McpProjectAvailability {
    McpProjectAvailability {
        project_id: project.project_id.clone(),
        is_default: project.is_default,
        available: false,
        unavailable_reason: Some(reason.into()),
        repo_root_display,
        baseline_workflow_access: "unavailable".to_owned(),
        missing_access_classes: missing_baseline_access(&access_classes),
    }
}

fn missing_baseline_access(access_classes: &[AccessClass]) -> Vec<AccessClass> {
    BASELINE_WORKFLOW_ACCESS_CLASSES
        .iter()
        .copied()
        .filter(|access_class| !access_classes.contains(access_class))
        .collect()
}

fn selected_project_from_availability(
    context: &McpIntegrationContext,
    project: McpProjectAvailability,
    requested_access_class: AccessClass,
) -> Result<McpDerivedInvocationContext, McpAdapterError> {
    if !project.available {
        return Err(routing_error(format!(
            "project {} is unavailable: {}",
            project.project_id,
            project
                .unavailable_reason
                .unwrap_or_else(|| "unavailable".to_owned())
        )));
    }
    Ok(McpDerivedInvocationContext {
        project_id: ProjectId::new(project.project_id),
        surface_id: context.surface_id.clone(),
        surface_instance_id: context.surface_instance_id.clone(),
        requested_access_class,
        invocation_binding_basis: context.invocation_binding_basis.clone(),
    })
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartupSurface {
    project_id: String,
    surface_id: String,
    surface_instance_id: String,
    interaction_role: SurfaceInteractionRole,
    access_classes: Vec<AccessClass>,
}

fn valid_startup_surface(surface: SurfaceRecord) -> Result<StartupSurface, McpAdapterError> {
    let interaction_role = match surface.interaction_role.as_str() {
        "agent" => SurfaceInteractionRole::Agent,
        "user_interaction" => SurfaceInteractionRole::UserInteraction,
        _ => {
            return Err(McpAdapterError::Environment(
                "registered surface interaction role is not recognized".to_owned(),
            ));
        }
    };
    match serde_json::from_str::<Value>(&surface.capability_profile_json) {
        Ok(Value::Object(_)) => (),
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered surface capability profile is not an object".to_owned(),
            ));
        }
        Err(error) => return Err(McpAdapterError::Json(error)),
    };
    match serde_json::from_str::<Value>(&surface.metadata_json) {
        Ok(Value::Object(_)) => (),
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered surface metadata is not an object".to_owned(),
            ));
        }
        Err(error) => return Err(McpAdapterError::Json(error)),
    };
    let access_classes = match startup_authorized_access_classes(&surface.local_access_json) {
        Ok(access_classes) if !access_classes.is_empty() => access_classes,
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered surface local access grant is empty".to_owned(),
            ))
        }
        Err(error) => return Err(error),
    };

    Ok(StartupSurface {
        project_id: surface.project_id,
        surface_id: surface.surface_id,
        surface_instance_id: surface.surface_instance_id,
        interaction_role,
        access_classes,
    })
}

fn startup_authorized_access_classes(text: &str) -> Result<Vec<AccessClass>, McpAdapterError> {
    let value = serde_json::from_str::<Value>(text).map_err(McpAdapterError::Json)?;
    let object = value.as_object().ok_or_else(|| {
        McpAdapterError::Environment("registered surface local access is not an object".to_owned())
    })?;
    if object.contains_key("access_class") {
        return Err(McpAdapterError::Environment(
            "registered surface local access uses obsolete access_class".to_owned(),
        ));
    }
    let mut access_classes = Vec::new();
    let values = object
        .get("authorized_access_classes")
        .ok_or_else(|| {
            McpAdapterError::Environment(
                "registered surface local access grant is missing".to_owned(),
            )
        })?
        .as_array()
        .ok_or_else(|| {
            McpAdapterError::Environment(
                "registered surface authorized access classes are not an array".to_owned(),
            )
        })?;
    for value in values {
        let access_class = startup_access_class(value)?;
        if !access_classes.contains(&access_class) {
            access_classes.push(access_class);
        }
    }
    if access_classes.is_empty() {
        return Err(McpAdapterError::Environment(
            "registered surface local access grant is empty".to_owned(),
        ));
    }

    match object.get("verification_basis") {
        Some(Value::String(text)) if !text.trim().is_empty() => (),
        _ => {
            return Err(McpAdapterError::Environment(
                "registered surface verification basis is invalid".to_owned(),
            ));
        }
    }

    Ok(access_classes)
}

fn startup_access_class(value: &Value) -> Result<AccessClass, McpAdapterError> {
    serde_json::from_value(value.clone()).map_err(McpAdapterError::Json)
}

fn rejected_pipeline_response(
    dry_run: bool,
    error: ToolError,
) -> Result<PipelineResponse, McpAdapterError> {
    let response = rejected_response(dry_run, None, vec![error]);
    let response_value = serde_json::to_value(&response).map_err(McpAdapterError::Json)?;
    let response_json = serde_json::to_string(&response_value).map_err(McpAdapterError::Json)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        verified_surface: None,
        resolved_task_id: None,
        replayed: false,
    })
}

fn local_access_mismatch_error(field: &'static str) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(
        ErrorCode::LocalAccessMismatch,
        "local surface context does not match the registered surface",
        false,
        Some(details),
    )
}

fn controlled_invocation_binding_basis(value: &str) -> &'static str {
    match value.trim() {
        VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING => {
            VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING
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
            format!("{message}. Use harness.list_projects when project selection is unclear.")
        }
        McpAdapterError::ToolExecution { tool_name, message } => {
            format!("{tool_name} failed before reaching Harness Core: {message}")
        }
        _ => "Tool execution failed before reaching Harness Core.".to_owned(),
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
        "harness.intake" => "Start, resume, supersede, or reject an ordinary user work loop.",
        "harness.update_scope" => "Update current Task scope and Change Unit state.",
        "harness.status" => "Read the current Core status view.",
        "harness.prepare_write" => "Check one proposed product-file write against Core state.",
        "harness.stage_artifact" => "Stage safe artifact bytes or a safe notice.",
        "harness.record_run" => "Record shaping, direct, or implementation work.",
        "harness.request_user_judgment" => "Create one pending focused user-owned judgment.",
        "harness.record_user_judgment" => "Record the user's answer to one pending judgment.",
        "harness.close_task" => "Check or perform a selected Task close path.",
        LIST_PROJECTS_TOOL_NAME => "List projects explicitly allowed for this MCP integration.",
        _ => "Unsupported Harness method.",
    }
}

fn raw_requested_access_class(
    tool_name: &str,
    params: &Value,
) -> Result<AccessClass, McpAdapterError> {
    let access_class = match tool_name {
        "harness.status" => AccessClass::ReadStatus,
        "harness.intake"
        | "harness.update_scope"
        | "harness.request_user_judgment"
        | "harness.record_user_judgment" => AccessClass::CoreMutation,
        "harness.prepare_write" => AccessClass::WriteAuthorization,
        "harness.stage_artifact" => AccessClass::ArtifactRegistration,
        "harness.record_run" => AccessClass::RunRecording,
        "harness.close_task" => {
            if params
                .get("intent")
                .and_then(Value::as_str)
                .is_some_and(|intent| intent == "check")
            {
                AccessClass::ReadStatus
            } else {
                AccessClass::CoreMutation
            }
        }
        other => return Err(McpAdapterError::UnknownTool(other.to_owned())),
    };
    Ok(access_class)
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
    mark_adapter_managed_envelope_fields(&mut schema);
    Some(schema)
}

fn mark_adapter_managed_envelope_fields(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            if is_tool_envelope_schema(object) {
                if let Some(Value::Array(required)) = object.get_mut("required") {
                    required.retain(|value| {
                        !matches!(value.as_str(), Some("project_id") | Some("surface_id"))
                    });
                }
            }
            for value in object.values_mut() {
                mark_adapter_managed_envelope_fields(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                mark_adapter_managed_envelope_fields(value);
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
        "surface_id",
        "request_id",
        "actor_kind",
        "dry_run",
    ]
    .iter()
    .all(|field| properties.contains_key(*field))
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use std::{
        collections::BTreeSet,
        fs,
        io::{BufReader, Cursor},
        path::PathBuf,
    };

    use harness_core::{AdapterSessionBinding, CoreBoundary, CoreService, InvocationContext};
    use harness_store::{
        agent_integrations::{
            add_integration_project, register_agent_integration, remove_integration_project,
            set_agent_integration_default_project, set_agent_integration_enabled,
            AgentIntegrationRegistration, IntegrationProjectRegistration,
        },
        bootstrap::{
            initialize_runtime_home, register_project, register_surface, ProjectRegistration,
            SurfaceRegistration, ACTIVE_PROJECT_STATUS,
        },
        core_pipeline::{CoreProjectStore, StorageEffectCounts},
        runtime_home::resolve_runtime_home as resolve_store_runtime_home,
        sqlite::{open_registry_database, registry_db_path},
    };
    use harness_test_support::TempRuntimeHome;
    use harness_types::{
        ActorKind, ChangeUnitOperation, InitialScope, RedactionState, RequestedMode, ResumePolicy,
        StatusInclude, SurfaceInteractionRole, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    };
    use serde_json::json;

    use super::*;

    const PROJECT_ID: &str = "project_mcp";
    const SURFACE_ID: &str = "surface_mcp";
    const SURFACE_INSTANCE_ID: &str = "surface_instance_mcp";
    const INTEGRATION_ID: &str = "agent_integration_mcp";
    static NEXT_INTEGRATION_SUFFIX: AtomicUsize = AtomicUsize::new(0);

    struct TestHarness {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
    }

    impl TestHarness {
        fn new(capability_profile: Value) -> Result<Self, Box<dyn Error>> {
            Self::with_local_access(
                capability_profile,
                json!({
                    "authorized_access_classes": [
                        "read_status",
                        "core_mutation",
                        "write_authorization",
                        "run_recording",
                        "artifact_registration",
                        "artifact_read"
                    ],
                    "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
                }),
            )
        }

        fn with_local_access(
            capability_profile: Value,
            local_access: Value,
        ) -> Result<Self, Box<dyn Error>> {
            Self::with_role_and_local_access(
                capability_profile,
                local_access,
                SurfaceInteractionRole::Agent,
            )
        }

        fn with_role_and_local_access(
            capability_profile: Value,
            local_access: Value,
            interaction_role: SurfaceInteractionRole,
        ) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("mcp")?;
            let repo_root = runtime_home.create_product_repo("repo")?;
            initialize_runtime_home(runtime_home.path(), "runtime_home_mcp", "{}")?;
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
            register_surface(
                runtime_home.path(),
                SurfaceRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    surface_id: SURFACE_ID.to_owned(),
                    surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                    surface_kind: "mcp_test".to_owned(),
                    interaction_role,
                    display_name: Some("MCP Test Surface".to_owned()),
                    capability_profile_json: capability_profile.to_string(),
                    local_access_json: local_access.to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;

            Ok(Self {
                runtime_home_path: runtime_home.path().to_path_buf(),
                _runtime_home: runtime_home,
            })
        }

        fn adapter(&self) -> McpAdapter {
            let integration_id = next_integration_id();
            self.integration_adapter_for(&integration_id)
        }

        fn integration_adapter(&self) -> McpAdapter {
            self.integration_adapter_for(INTEGRATION_ID)
        }

        fn integration_adapter_for(&self, integration_id: &str) -> McpAdapter {
            self.register_integration(integration_id, SURFACE_ID, SURFACE_INSTANCE_ID)
                .expect("integration registration should succeed");
            let context = McpIntegrationContext::resolve(&self.runtime_home_path, integration_id)
                .expect("integration context should resolve")
                .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
            McpAdapter::new(&self.runtime_home_path, context)
        }

        fn register_integration(
            &self,
            integration_id: &str,
            surface_id: &str,
            surface_instance_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            register_agent_integration(
                &self.runtime_home_path,
                AgentIntegrationRegistration {
                    integration_id: integration_id.to_owned(),
                    interaction_role: "agent".to_owned(),
                    surface_id: surface_id.to_owned(),
                    surface_instance_id: surface_instance_id.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_integration_project(
                &self.runtime_home_path,
                IntegrationProjectRegistration {
                    integration_id: integration_id.to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                },
            )?;
            Ok(())
        }

        fn add_project_with_surface(
            &self,
            project_id: &str,
            register_bound_surface: bool,
        ) -> Result<(), Box<dyn Error>> {
            let repo_root = self
                ._runtime_home
                .create_product_repo(format!("repo-{project_id}"))?;
            register_project(
                &self.runtime_home_path,
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            if register_bound_surface {
                register_surface(
                    &self.runtime_home_path,
                    SurfaceRegistration {
                        project_id: project_id.to_owned(),
                        surface_id: SURFACE_ID.to_owned(),
                        surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                        surface_kind: "mcp_test".to_owned(),
                        interaction_role: SurfaceInteractionRole::Agent,
                        display_name: Some("MCP Test Surface".to_owned()),
                        capability_profile_json: "{}".to_owned(),
                        local_access_json: local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES)
                            .to_string(),
                        metadata_json: "{}".to_owned(),
                    },
                )?;
            }
            Ok(())
        }

        fn add_integration_project(
            &self,
            integration_id: &str,
            project_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            add_integration_project(
                &self.runtime_home_path,
                IntegrationProjectRegistration {
                    integration_id: integration_id.to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
            Ok(())
        }

        fn core(&self) -> CoreService {
            CoreService::new(&self.runtime_home_path)
        }

        fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
            Ok(
                CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?
                    .effect_counts()?,
            )
        }

        fn counts_for_project(
            &self,
            project_id: &str,
        ) -> Result<StorageEffectCounts, Box<dyn Error>> {
            Ok(
                CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(project_id))?
                    .effect_counts()?,
            )
        }
    }

    fn next_integration_id() -> String {
        let suffix = NEXT_INTEGRATION_SUFFIX.fetch_add(1, Ordering::Relaxed);
        format!("{INTEGRATION_ID}_{suffix}")
    }

    #[test]
    fn mcp_boundary_wraps_core_boundary() {
        assert_eq!(
            McpAdapterBoundary::new(CoreBoundary::new()).label(),
            "mcp-adapter"
        );
    }

    #[test]
    fn registers_exactly_documented_public_method_tools() {
        let tools = public_method_tools();
        let names = tools.iter().map(|tool| tool.name).collect::<Vec<_>>();
        let unique_names = names.iter().copied().collect::<BTreeSet<_>>();

        assert_eq!(names, PUBLIC_METHOD_TOOL_NAMES);
        assert_eq!(tools.len(), 9);
        assert_eq!(unique_names.len(), 9);
    }

    #[test]
    fn mcp_tools_keep_public_methods_and_adapter_utilities_separate() {
        let tools = mcp_tools();
        let names = tools.iter().map(|tool| tool.name).collect::<Vec<_>>();

        assert_eq!(
            &names[..PUBLIC_METHOD_TOOL_NAMES.len()],
            PUBLIC_METHOD_TOOL_NAMES
        );
        assert_eq!(
            &names[PUBLIC_METHOD_TOOL_NAMES.len()..],
            ADAPTER_UTILITY_TOOL_NAMES
        );
        assert_eq!(public_method_tools().len(), 9);
        assert_eq!(adapter_utility_tools().len(), 1);
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn initialization_result_includes_concise_server_instructions() {
        let result = initialize_result();
        let instructions = result["instructions"]
            .as_str()
            .expect("initialize result should include instructions");

        assert!(instructions.len() < 2048);
        assert!(instructions[..instructions.len().min(512)].contains("Harness records"));
        assert!(instructions.contains("harness.list_projects"));
        assert!(instructions.contains("do not guess"));
        assert!(instructions.contains("separate from permission to edit product files"));
    }

    #[test]
    fn public_method_tool_schemas_are_closed_request_shapes() {
        let tools = public_method_tools();

        for tool in &tools {
            assert_eq!(
                tool.input_schema["additionalProperties"], false,
                "{} should reject additional top-level properties",
                tool.name
            );
            assert_required_fields(tool.name, &tool.input_schema);
            for forbidden in [
                "verified",
                "surface_instance_id",
                "verified_surface_context",
                "access_class",
                "capability_profile",
                "verification_basis",
            ] {
                assert!(
                    !schema_has_property(&tool.input_schema, forbidden),
                    "{} schema should not expose invocation-only field {forbidden}",
                    tool.name
                );
            }
        }
        let request_judgment = tools
            .iter()
            .find(|tool| tool.name == "harness.request_user_judgment")
            .expect("request_user_judgment tool should be registered");
        for forbidden in ["machine_action", "resolution_outcome"] {
            assert!(
                !schema_has_property(&request_judgment.input_schema, forbidden),
                "request_user_judgment schema should not expose caller option {forbidden}"
            );
        }
    }

    #[test]
    fn mcp_visible_schemas_make_project_and_surface_adapter_managed() {
        for tool in public_method_tools() {
            let required = envelope_required_fields(&tool.input_schema)
                .expect("tool schema should contain ToolEnvelope schema");
            assert!(
                !required.contains(&"project_id".to_owned()),
                "{} should not require envelope.project_id from MCP callers",
                tool.name
            );
            assert!(
                !required.contains(&"surface_id".to_owned()),
                "{} should not require envelope.surface_id from MCP callers",
                tool.name
            );
            assert!(
                schema_has_property(&tool.input_schema, "project_id"),
                "{} should still expose envelope.project_id as an optional selector",
                tool.name
            );
            assert!(
                schema_has_property(&tool.input_schema, "surface_id"),
                "{} should still expose envelope.surface_id for compatibility",
                tool.name
            );
        }
    }

    #[test]
    fn integration_adapter_implicitly_routes_single_project_and_injects_surface(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        let adapter = harness.integration_adapter();
        let mut params = serde_json::to_value(status_request("req_integration_single"))?;
        let envelope = params["envelope"]
            .as_object_mut()
            .expect("envelope should be an object");
        envelope.remove("project_id");
        envelope.remove("surface_id");

        let response = adapter.call_tool("harness.status", params)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        let verified = response
            .verified_surface
            .as_ref()
            .expect("Core should verify injected surface");
        assert_eq!(verified.project_id.as_str(), PROJECT_ID);
        assert_eq!(verified.surface_id.as_str(), SURFACE_ID);
        assert_eq!(verified.surface_instance_id.as_str(), SURFACE_INSTANCE_ID);
        Ok(())
    }

    #[test]
    fn integration_adapter_routes_explicit_project_and_isolates_state() -> Result<(), Box<dyn Error>>
    {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        let other_project_id = "project_mcp_other";
        harness.register_integration(INTEGRATION_ID, SURFACE_ID, SURFACE_INSTANCE_ID)?;
        harness.add_project_with_surface(other_project_id, true)?;
        harness.add_integration_project(INTEGRATION_ID, other_project_id)?;
        let context = McpIntegrationContext::resolve(&harness.runtime_home_path, INTEGRATION_ID)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&harness.runtime_home_path, context);
        let before_bound = harness.counts()?;
        let before_other = harness.counts_for_project(other_project_id)?;
        let mut params = serde_json::to_value(intake_request(
            "req_integration_route_b",
            false,
            Some("idem_integration_route_b"),
        ))?;
        params["envelope"]["project_id"] = json!(other_project_id);
        params["envelope"]
            .as_object_mut()
            .expect("envelope object")
            .remove("surface_id");

        let response = adapter.call_tool("harness.intake", params)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(harness.counts()?, before_bound);
        let after_other = harness.counts_for_project(other_project_id)?;
        assert_eq!(after_other.state_version, before_other.state_version + 1);
        assert_eq!(after_other.tasks, before_other.tasks + 1);
        Ok(())
    }

    #[test]
    fn integration_adapter_uses_default_project_when_selection_is_implicit(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        let default_project_id = "project_mcp_default";
        harness.register_integration(INTEGRATION_ID, SURFACE_ID, SURFACE_INSTANCE_ID)?;
        harness.add_project_with_surface(default_project_id, true)?;
        harness.add_integration_project(INTEGRATION_ID, default_project_id)?;
        set_agent_integration_default_project(
            &harness.runtime_home_path,
            INTEGRATION_ID,
            default_project_id,
        )?;
        let context = McpIntegrationContext::resolve(&harness.runtime_home_path, INTEGRATION_ID)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&harness.runtime_home_path, context);
        let mut params = serde_json::to_value(status_request("req_integration_default"))?;
        params["envelope"]
            .as_object_mut()
            .expect("envelope object")
            .remove("project_id");

        let response = adapter.call_tool("harness.status", params)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(
            response
                .verified_surface
                .as_ref()
                .expect("verified surface")
                .project_id
                .as_str(),
            default_project_id
        );
        Ok(())
    }

    #[test]
    fn integration_adapter_rejects_ambiguous_project_and_lists_allowed_only(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        let allowed_project_id = "project_mcp_allowed";
        let unrelated_project_id = "project_mcp_unrelated";
        harness.register_integration(INTEGRATION_ID, SURFACE_ID, SURFACE_INSTANCE_ID)?;
        harness.add_project_with_surface(allowed_project_id, true)?;
        harness.add_integration_project(INTEGRATION_ID, allowed_project_id)?;
        harness.add_project_with_surface(unrelated_project_id, true)?;
        let context = McpIntegrationContext::resolve(&harness.runtime_home_path, INTEGRATION_ID)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&harness.runtime_home_path, context);
        let mut params = serde_json::to_value(status_request("req_integration_ambiguous"))?;
        params["envelope"]
            .as_object_mut()
            .expect("envelope object")
            .remove("project_id");

        let error = adapter
            .call_tool("harness.status", params)
            .expect_err("ambiguous routing should be rejected before Core");

        assert!(matches!(error, McpAdapterError::ToolExecution { .. }));
        assert!(error.to_string().contains("ambiguous"));
        let list = adapter.call_adapter_tool(LIST_PROJECTS_TOOL_NAME, json!({}))?;
        let project_ids = list["projects"]
            .as_array()
            .expect("projects should be an array")
            .iter()
            .map(|project| project["project_id"].as_str().expect("project id"))
            .collect::<Vec<_>>();
        assert_eq!(project_ids, vec![PROJECT_ID, allowed_project_id]);
        assert!(!project_ids.contains(&unrelated_project_id));
        Ok(())
    }

    #[test]
    fn integration_adapter_rechecks_membership_for_running_process() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        harness.register_integration(INTEGRATION_ID, SURFACE_ID, SURFACE_INSTANCE_ID)?;
        let context = McpIntegrationContext::resolve(&harness.runtime_home_path, INTEGRATION_ID)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&harness.runtime_home_path, context);
        let response = adapter.call_tool(
            "harness.status",
            serde_json::to_value(status_request("req_integration_before_revoke"))?,
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");

        remove_integration_project(&harness.runtime_home_path, INTEGRATION_ID, PROJECT_ID)?;

        let error = adapter
            .call_tool(
                "harness.status",
                serde_json::to_value(status_request("req_integration_after_revoke"))?,
            )
            .expect_err("revoked project should be rejected by running process");

        assert!(error.to_string().contains("not allowed"));
        Ok(())
    }

    #[test]
    fn list_projects_reports_inactive_and_missing_surface_without_exposing_unrelated(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        let inactive_project_id = "project_mcp_inactive";
        let missing_surface_project_id = "project_mcp_missing_surface";
        harness.register_integration(INTEGRATION_ID, SURFACE_ID, SURFACE_INSTANCE_ID)?;
        harness.add_project_with_surface(inactive_project_id, true)?;
        harness.add_integration_project(INTEGRATION_ID, inactive_project_id)?;
        harness.add_project_with_surface(missing_surface_project_id, false)?;
        harness.add_integration_project(INTEGRATION_ID, missing_surface_project_id)?;
        let conn = open_registry_database(registry_db_path(&harness.runtime_home_path))?;
        conn.pragma_update(None, "ignore_check_constraints", "ON")?;
        conn.execute(
            "UPDATE projects SET status = 'inactive' WHERE project_id = ?1",
            [inactive_project_id],
        )?;
        let context = McpIntegrationContext::resolve(&harness.runtime_home_path, INTEGRATION_ID)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&harness.runtime_home_path, context);

        let list = adapter.call_adapter_tool(LIST_PROJECTS_TOOL_NAME, json!({}))?;

        let projects = list["projects"].as_array().expect("projects array");
        let inactive = projects
            .iter()
            .find(|project| project["project_id"] == inactive_project_id)
            .expect("inactive project should be listed");
        assert_eq!(inactive["available"], false);
        assert!(inactive["unavailable_reason"]
            .as_str()
            .expect("reason")
            .contains("not active"));
        let missing_surface = projects
            .iter()
            .find(|project| project["project_id"] == missing_surface_project_id)
            .expect("missing surface project should be listed");
        assert_eq!(missing_surface["available"], false);
        assert!(missing_surface["unavailable_reason"]
            .as_str()
            .expect("reason")
            .contains("surface instance"));

        let mut params = serde_json::to_value(status_request("req_inactive_rejected"))?;
        params["envelope"]["project_id"] = json!(inactive_project_id);
        let error = adapter
            .call_tool("harness.status", params)
            .expect_err("inactive project should be rejected before Core");
        assert!(error.to_string().contains("unavailable"));
        Ok(())
    }

    #[test]
    fn disabled_or_missing_integration_startup_is_rejected() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_role_and_local_access(
            json!({}),
            local_access(&BASELINE_WORKFLOW_ACCESS_CLASSES),
            SurfaceInteractionRole::Agent,
        )?;
        let missing = McpIntegrationContext::resolve(&harness.runtime_home_path, "missing_agent")
            .expect_err("missing integration should fail startup");
        assert!(missing.to_string().contains("not registered"));

        harness.register_integration(INTEGRATION_ID, SURFACE_ID, SURFACE_INSTANCE_ID)?;
        set_agent_integration_enabled(&harness.runtime_home_path, INTEGRATION_ID, false)?;
        let disabled = McpIntegrationContext::resolve(&harness.runtime_home_path, INTEGRATION_ID)
            .expect_err("disabled integration should fail startup");
        assert!(disabled.to_string().contains("disabled"));
        Ok(())
    }

    #[test]
    fn stdio_tools_list_exposes_exactly_public_method_tools() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let adapter = harness.adapter();
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"harness-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[0]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        let capabilities = responses[0]["result"]["capabilities"]
            .as_object()
            .expect("initialize result capabilities should be an object");
        assert!(capabilities.contains_key("tools"));
        assert!(!capabilities.contains_key("tasks"));
        assert!(!capabilities.contains_key("prompts"));
        assert!(!capabilities.contains_key("resources"));
        let response = &responses[1];
        let names = response["result"]["tools"]
            .as_array()
            .expect("tools should be an array")
            .iter()
            .map(|tool| {
                assert!(tool.get("execution").is_none());
                tool["name"].as_str().expect("tool name")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            &names[..PUBLIC_METHOD_TOOL_NAMES.len()],
            PUBLIC_METHOD_TOOL_NAMES
        );
        assert_eq!(
            names[PUBLIC_METHOD_TOOL_NAMES.len()],
            LIST_PROJECTS_TOOL_NAME
        );
        assert_eq!(names.len(), 10);
        Ok(())
    }

    #[test]
    fn stdio_lifecycle_negotiates_version_and_requires_initialized_notification(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let adapter = harness.adapter();
        let status_arguments = serde_json::to_value(status_request("req_stdio_lifecycle"))?;
        let responses = run_stdio_messages(
            adapter,
            &[
                request_message(json!(1), "tools/list", Some(json!({}))),
                initialize_request_message(json!(2), "2030-01-01"),
                request_message(json!(3), "tools/list", Some(json!({}))),
                request_message(json!("ping-waiting"), "ping", None),
                initialized_notification_with_params(json!([])),
                request_message(
                    json!(4),
                    "tools/call",
                    Some(json!({
                        "name": "harness.status",
                        "arguments": status_arguments.clone()
                    })),
                ),
                initialized_notification(),
                request_message(json!(5), "tools/list", None),
                request_message(
                    json!(6),
                    "tools/call",
                    Some(json!({
                        "name": "harness.status",
                        "arguments": status_arguments
                    })),
                ),
                initialize_request_message(json!(7), SUPPORTED_PROTOCOL_VERSION),
                request_message(json!(8), "tools/list", Some(json!({}))),
            ],
        )?;

        assert_eq!(responses.len(), 9);
        let response_ids = responses
            .iter()
            .map(|response| response["id"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            response_ids,
            vec![
                json!(1),
                json!(2),
                json!(3),
                json!("ping-waiting"),
                json!(4),
                json!(5),
                json!(6),
                json!(7),
                json!(8)
            ]
        );
        assert_error_response(&responses[0], json!(1), -32600);
        assert_eq!(responses[1]["id"], 2);
        assert_eq!(
            responses[1]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        assert_error_response(&responses[2], json!(3), -32600);
        assert_eq!(responses[3]["id"], "ping-waiting");
        assert_eq!(responses[3]["result"], json!({}));
        assert_error_response(&responses[4], json!(4), -32600);
        assert_eq!(responses[5]["id"], 5);
        assert_eq!(
            responses[5]["result"]["tools"]
                .as_array()
                .expect("tools should be an array")
                .len(),
            10
        );
        assert_eq!(responses[6]["id"], 6);
        assert_eq!(responses[6]["result"]["isError"], false);
        let tool_text = responses[6]["result"]["content"][0]["text"]
            .as_str()
            .expect("tools/call response should contain text content");
        let tool_response: Value = serde_json::from_str(tool_text)?;
        assert_eq!(tool_response["base"]["response_kind"], "result");
        assert_error_response(&responses[7], json!(7), -32600);
        assert_eq!(
            responses[8]["result"]["tools"]
                .as_array()
                .expect("tools should be an array")
                .len(),
            10
        );
        Ok(())
    }

    #[test]
    fn stdio_early_initialized_notification_does_not_make_connection_ready(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let responses = run_stdio_messages(
            harness.adapter(),
            &[
                initialized_notification(),
                request_message(json!(1), "tools/list", Some(json!({}))),
                initialize_request_message(json!(2), SUPPORTED_PROTOCOL_VERSION),
                request_message(json!(3), "tools/list", Some(json!({}))),
                initialized_notification(),
                request_message(json!(4), "tools/list", Some(json!({}))),
            ],
        )?;

        assert_eq!(responses.len(), 4);
        assert_error_response(&responses[0], json!(1), -32600);
        assert_eq!(
            responses[1]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        assert_error_response(&responses[2], json!(3), -32600);
        assert_eq!(
            responses[3]["result"]["tools"]
                .as_array()
                .expect("tools should be an array")
                .len(),
            10
        );
        Ok(())
    }

    #[test]
    fn stdio_invalid_initialize_does_not_advance_lifecycle() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let adapter = harness.adapter();
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"harness-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"harness-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":4,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 4);
        assert_eq!(responses[0]["id"], 1);
        assert_eq!(responses[0]["error"]["code"], -32602);
        assert_eq!(responses[1]["id"], 2);
        assert_eq!(responses[1]["error"]["code"], -32600);
        assert_eq!(responses[2]["id"], 3);
        assert_eq!(
            responses[2]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        assert_eq!(responses[3]["id"], 4);
        assert_eq!(
            responses[3]["result"]["tools"]
                .as_array()
                .expect("tools should be an array")
                .len(),
            10
        );
        Ok(())
    }

    #[test]
    fn stdio_arrays_return_one_invalid_request_response() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let adapter = harness.adapter();
        let input = Cursor::new(
            br#"[{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}},{"jsonrpc":"2.0","id":2,"method":"ping","params":{}}]
"#
            .to_vec(),
        );
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["id"], Value::Null);
        assert_eq!(responses[0]["error"]["code"], -32600);
        Ok(())
    }

    #[test]
    fn stdio_invalid_json_rpc_shape_uses_valid_id_when_available() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let adapter = harness.adapter();
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":"request-a","params":{}}
{"jsonrpc":"2.0","id":1.5,"method":"initialize","params":{}}
"#
            .to_vec(),
        );
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0]["id"], "request-a");
        assert_eq!(responses[0]["error"]["code"], -32600);
        assert_eq!(responses[1]["id"], Value::Null);
        assert_eq!(responses[1]["error"]["code"], -32600);
        Ok(())
    }

    #[test]
    fn stdio_rejects_malformed_json_rpc_envelopes_with_single_protocol_errors(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let cases = [
            ("invalid_json", "{not json}\n", Value::Null, -32700),
            ("top_level_array", "[{}]\n", Value::Null, -32600),
            ("empty_array", "[]\n", Value::Null, -32600),
            ("null_root", "null\n", Value::Null, -32600),
            ("bool_root", "true\n", Value::Null, -32600),
            ("number_root", "17\n", Value::Null, -32600),
            ("string_root", "\"message\"\n", Value::Null, -32600),
            (
                "missing_jsonrpc",
                "{\"id\":\"missing-jsonrpc\",\"method\":\"initialize\",\"params\":{}}\n",
                json!("missing-jsonrpc"),
                -32600,
            ),
            (
                "wrong_jsonrpc",
                "{\"jsonrpc\":\"2.1\",\"id\":\"wrong-jsonrpc\",\"method\":\"initialize\",\"params\":{}}\n",
                json!("wrong-jsonrpc"),
                -32600,
            ),
            (
                "missing_method",
                "{\"jsonrpc\":\"2.0\",\"id\":\"missing-method\",\"params\":{}}\n",
                json!("missing-method"),
                -32600,
            ),
            (
                "non_string_method",
                "{\"jsonrpc\":\"2.0\",\"id\":\"bad-method\",\"method\":7,\"params\":{}}\n",
                json!("bad-method"),
                -32600,
            ),
            (
                "malformed_no_id_object",
                "{\"jsonrpc\":\"2.0\",\"params\":{}}\n",
                Value::Null,
                -32600,
            ),
            (
                "null_id",
                "{\"jsonrpc\":\"2.0\",\"id\":null,\"method\":\"initialize\",\"params\":{}}\n",
                Value::Null,
                -32600,
            ),
            (
                "float_id",
                "{\"jsonrpc\":\"2.0\",\"id\":1.5,\"method\":\"initialize\",\"params\":{}}\n",
                Value::Null,
                -32600,
            ),
            (
                "boolean_id",
                "{\"jsonrpc\":\"2.0\",\"id\":true,\"method\":\"initialize\",\"params\":{}}\n",
                Value::Null,
                -32600,
            ),
            (
                "object_id",
                "{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"initialize\",\"params\":{}}\n",
                Value::Null,
                -32600,
            ),
            (
                "array_id",
                "{\"jsonrpc\":\"2.0\",\"id\":[],\"method\":\"initialize\",\"params\":{}}\n",
                Value::Null,
                -32600,
            ),
        ];

        for (name, input, expected_id, expected_code) in cases {
            let responses = run_stdio_text(harness.adapter(), input)?;
            assert_eq!(responses.len(), 1, "{name} should produce one response");
            assert_error_response(&responses[0], expected_id, expected_code);
        }
        Ok(())
    }

    #[test]
    fn stdio_accepts_and_echoes_string_and_integer_request_ids() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let responses = run_stdio_messages(
            harness.adapter(),
            &[
                initialize_request_message(json!("init-string-id"), SUPPORTED_PROTOCOL_VERSION),
                initialized_notification(),
                request_message(json!(42), "ping", None),
                request_message(json!("tools-string-id"), "tools/list", Some(json!({}))),
            ],
        )?;

        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["id"], "init-string-id");
        assert_eq!(
            responses[0]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        assert_eq!(responses[1]["id"], 42);
        assert_eq!(responses[1]["result"], json!({}));
        assert_eq!(responses[2]["id"], "tools-string-id");
        assert!(responses[2]["result"]["tools"].is_array());
        Ok(())
    }

    #[test]
    fn stdio_notifications_do_not_respond_or_execute_request_only_methods(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let before = harness.counts()?;
        let intake_arguments =
            serde_json::to_value(intake_request("req_notification_tool_call", false, None))?;
        let responses = run_stdio_messages(
            harness.adapter(),
            &[
                notification_message("notifications/unknown", Some(json!({ "ignored": true }))),
                initialize_request_message(json!(1), SUPPORTED_PROTOCOL_VERSION),
                initialized_notification(),
                notification_message("tools/list", Some(json!({}))),
                notification_message("ping", Some(json!([]))),
                notification_message(
                    "initialize",
                    Some(json!({
                        "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": {
                            "name": "harness-unit-test",
                            "version": "0.0.0"
                        }
                    })),
                ),
                notification_message("tools/call", Some(Value::Null)),
                notification_message("tools/call", Some(json!([]))),
                notification_message(
                    "tools/call",
                    Some(json!({
                        "name": "harness.intake",
                        "arguments": intake_arguments
                    })),
                ),
                request_message(json!(2), "tools/list", Some(json!({}))),
            ],
        )?;

        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[0]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        assert!(responses[1]["result"]["tools"].is_array());
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stdio_validates_method_params_before_tool_execution() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let before = harness.counts()?;
        let responses = run_stdio_messages(
            harness.adapter(),
            &[
                initialize_request_message(json!(1), SUPPORTED_PROTOCOL_VERSION),
                initialized_notification(),
                request_message(json!(2), "ping", None),
                request_message(json!(3), "ping", Some(json!({}))),
                request_message(json!(4), "tools/list", None),
                request_message(json!(5), "tools/list", Some(json!({}))),
                request_message(json!(6), "tools/call", Some(Value::Null)),
                request_message(json!(7), "tools/call", Some(json!({}))),
                request_message(json!(8), "tools/call", Some(json!({ "name": 7 }))),
                request_message(
                    json!(9),
                    "tools/call",
                    Some(json!({ "name": "harness.not_real" })),
                ),
                request_message(
                    json!(10),
                    "tools/call",
                    Some(json!({ "name": "harness.status", "arguments": null })),
                ),
                request_message(
                    json!(11),
                    "tools/call",
                    Some(json!({ "name": "harness.status", "arguments": [] })),
                ),
                request_message(
                    json!(12),
                    "tools/call",
                    Some(json!({
                        "name": "harness.status",
                        "task": {},
                        "arguments": {}
                    })),
                ),
                request_message(
                    json!(13),
                    "tools/call",
                    Some(json!({ "name": "harness.status" })),
                ),
                request_message(json!(15), "tools/call", Some(json!([]))),
                request_message(json!(14), "not/a-method", Some(json!({}))),
            ],
        )?;

        assert_eq!(responses.len(), 15);
        assert_eq!(
            responses[0]["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );
        for response in responses.iter().take(5).skip(1) {
            assert!(response.get("error").is_none());
        }
        for (response_index, id) in [(5, 6), (6, 7), (7, 8), (8, 9), (9, 10), (10, 11), (11, 12)] {
            assert_error_response(&responses[response_index], json!(id), -32602);
        }
        assert!(responses[12].get("error").is_none());
        assert_eq!(responses[12]["id"], 13);
        assert_eq!(responses[12]["result"]["isError"], true);
        let text = responses[12]["result"]["content"][0]["text"]
            .as_str()
            .expect("typed validation failure should include text content");
        assert!(text.contains("before reaching Harness Core"));
        assert_error_response(&responses[13], json!(15), -32602);
        assert_error_response(&responses[14], json!(14), -32601);
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn bootstrap_registered_surface_can_call_status_through_adapter() -> Result<(), Box<dyn Error>>
    {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        let adapter = harness.adapter();
        let request = status_request("req_status_adapter");

        let response = adapter.call_tool("harness.status", serde_json::to_value(request)?)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        let verified = response
            .verified_surface
            .as_ref()
            .expect("Core should return verified surface context");
        assert_eq!(verified.project_id.as_str(), PROJECT_ID);
        assert_eq!(verified.surface_id.as_str(), SURFACE_ID);
        assert_eq!(verified.surface_instance_id.as_str(), SURFACE_INSTANCE_ID);
        assert_eq!(
            verified.verification_basis,
            "local_admin_registration:test_fixture_binding"
        );
        Ok(())
    }

    #[test]
    fn adapter_preserves_structured_core_store_rejection() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        fs::remove_file(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
        let adapter = harness.adapter();

        let error = adapter
            .call_tool(
                "harness.status",
                serde_json::to_value(status_request("req_status_missing_db_adapter"))?,
            )
            .expect_err("missing project state should fail during integration routing");

        assert!(matches!(error, McpAdapterError::ToolExecution { .. }));
        let body = error.to_string();
        assert!(body.contains("project_state_database not found"));
        let runtime_home = harness.runtime_home_path.to_string_lossy();
        assert!(!body.contains(runtime_home.as_ref()));
        assert!(!body.contains("state.sqlite"));
        assert!(!body.contains("SELECT "));
        Ok(())
    }

    #[test]
    fn adapter_and_direct_core_status_have_equivalent_response_meaning(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        let task_id = create_task_with_change_unit(&harness, "status_equiv")?;
        let adapter = harness.adapter();
        let mut request = status_request("req_status_equiv");
        request.envelope.task_id = Some(harness_types::TaskId::new(&task_id)).into();
        let direct = harness
            .core()
            .status(request.clone(), invocation(AccessClass::ReadStatus))?;
        let adapted = adapter.call_tool("harness.status", serde_json::to_value(request)?)?;

        assert_eq!(adapted.response_value, direct.response_value);
        assert_eq!(adapted.response_json, direct.response_json);
        assert_eq!(adapted.response_value["close_state"], "blocked");
        assert!(adapted.response_value["close_blockers"]
            .as_array()
            .expect("close blockers")
            .iter()
            .any(|blocker| blocker["code"] == "missing_current_close_basis"));
        assert_eq!(
            adapted.response_value["guarantee_display"]["level"],
            "cooperative"
        );
        Ok(())
    }

    #[test]
    fn adapter_and_direct_core_intake_dry_run_have_equivalent_response_meaning(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "core_mutation",
            "supported_access_classes": ["core_mutation"]
        }))?;
        let adapter = harness.adapter();
        let request = intake_request("req_intake_equiv", true, None);
        let direct = harness
            .core()
            .intake(request.clone(), invocation(AccessClass::CoreMutation))?;
        let adapted = adapter.call_tool("harness.intake", serde_json::to_value(request)?)?;

        assert_eq!(
            normalize_dry_run_required_ref_ids(adapted.response_value),
            normalize_dry_run_required_ref_ids(direct.response_value)
        );
        Ok(())
    }

    #[test]
    fn adapter_derives_access_class_per_method_call() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        let adapter = harness.adapter();
        let response = adapter.call_tool(
            "harness.status",
            serde_json::to_value(status_request("req_status_derived_read"))?,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        Ok(())
    }

    #[test]
    fn integration_requested_access_class_cannot_elevate_registered_grant(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_local_access(
            json!({
                "access_class": "core_mutation",
                "supported_access_classes": ["core_mutation"]
            }),
            json!({
                "authorized_access_classes": ["read_status"],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
        )?;
        let adapter = harness.adapter();

        let error = adapter
            .call_tool(
                "harness.intake",
                serde_json::to_value(intake_request("req_env_elevate", true, None))?,
            )
            .expect_err("unauthorized access class should fail before Core");

        assert!(matches!(error, McpAdapterError::ToolExecution { .. }));
        assert!(error.to_string().contains("core_mutation"));
        Ok(())
    }

    #[test]
    fn adapter_does_not_bypass_artifact_registration_capability() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"]
        }))?;
        let task_id = create_task(&harness, "req_stage_task", "idem_stage_task")?;
        let adapter = harness.adapter();

        let response = adapter.call_tool(
            "harness.stage_artifact",
            serde_json::to_value(stage_artifact_request(
                "req_stage_missing_capability",
                &task_id,
            ))?,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "CAPABILITY_INSUFFICIENT"
        );
        Ok(())
    }

    #[test]
    fn caller_submitted_invocation_context_is_rejected_before_core() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"],
            "manual_artifact_attachment_supported": true
        }))?;
        let task_id = create_task(&harness, "req_stage_task_forged", "idem_stage_task_forged")?;
        let adapter = harness.adapter();
        let mut params =
            serde_json::to_value(stage_artifact_request("req_stage_forged", &task_id))?;
        params["surface_instance_id"] = json!("forged_surface_instance");
        let before = harness.counts()?;

        let error = adapter
            .call_tool("harness.stage_artifact", params)
            .expect_err("forged invocation context should be invalid params");

        assert!(matches!(error, McpAdapterError::InvalidParams { .. }));
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn missing_required_nullable_param_is_rejected_before_core() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"],
            "manual_artifact_attachment_supported": true
        }))?;
        let task_id = create_task(
            &harness,
            "req_stage_missing_required_task",
            "idem_stage_missing_required_task",
        )?;
        let adapter = harness.adapter();
        let mut params = serde_json::to_value(stage_artifact_request(
            "req_stage_missing_required_nullable",
            &task_id,
        ))?;
        params
            .as_object_mut()
            .expect("params should be an object")
            .remove("expected_sha256");
        let before = harness.counts()?;

        let error = adapter
            .call_tool("harness.stage_artifact", params)
            .expect_err("missing required nullable field should be invalid params");

        assert!(matches!(error, McpAdapterError::InvalidParams { .. }));
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn replay_hash_uses_decoded_typed_request_not_raw_property_order() -> Result<(), Box<dyn Error>>
    {
        let harness = TestHarness::new(json!({
            "access_class": "core_mutation",
            "supported_access_classes": ["core_mutation"]
        }))?;
        let adapter = harness.adapter();
        let first = serde_json::to_value(intake_request(
            "req_intake_order_replay",
            false,
            Some("idem_intake_order_replay"),
        ))?;
        let second: Value = serde_json::from_str(
            r#"{
                "initial_context_refs": [],
                "initial_scope": {
                    "acceptance_criteria": ["Adapter calls return Core responses."],
                    "non_goals": ["Changing Core method semantics."],
                    "boundary": "Local MCP adapter behavior."
                },
                "resume_policy": "create_new",
                "requested_mode": "work",
                "plain_language_request": "Prepare a local MCP adapter test task.",
                "envelope": {
                    "locale": "en-US",
                    "dry_run": false,
                    "expected_state_version": 0,
                    "idempotency_key": "idem_intake_order_replay",
                    "request_id": "req_intake_order_replay",
                    "surface_id": "surface_mcp",
                    "actor_kind": "agent",
                    "task_id": null,
                    "project_id": "project_mcp"
                }
            }"#,
        )?;

        let first = adapter.call_tool("harness.intake", first)?;
        let after_first = harness.counts()?;
        let second = adapter.call_tool("harness.intake", second)?;

        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    fn assert_required_fields(tool_name: &str, schema: &Value) {
        let required = schema["required"]
            .as_array()
            .expect("schema should have required fields")
            .iter()
            .map(|value| value.as_str().expect("required field name"))
            .collect::<BTreeSet<_>>();
        let expected = expected_required_fields(tool_name)
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(required, expected, "{tool_name} required fields");
    }

    fn expected_required_fields(tool_name: &str) -> &'static [&'static str] {
        match tool_name {
            "harness.intake" => &[
                "envelope",
                "plain_language_request",
                "requested_mode",
                "resume_policy",
                "initial_scope",
                "initial_context_refs",
            ],
            "harness.update_scope" => &[
                "envelope",
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
            ],
            "harness.status" => &["envelope", "include"],
            "harness.prepare_write" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "intended_operation",
                "intended_paths",
                "product_file_write_intended",
                "sensitive_categories",
                "baseline_ref",
            ],
            "harness.stage_artifact" => &[
                "envelope",
                "task_id",
                "display_name",
                "content_type",
                "redaction_state",
                "safe_bytes_or_notice",
                "expected_sha256",
                "expected_size_bytes",
                "relation_hint",
            ],
            "harness.record_run" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "kind",
                "run_id",
                "baseline_ref",
                "write_authorization_id",
                "summary",
                "observed_changes",
                "artifact_inputs",
                "evidence_updates",
                "close_assessment",
            ],
            "harness.request_user_judgment" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "judgment_kind",
                "presentation",
                "question",
                "context",
                "affected_refs",
                "required_for",
                "expires_at",
            ],
            "harness.record_user_judgment" => &[
                "envelope",
                "user_judgment_id",
                "judgment_kind",
                "selected_option_id",
                "answer",
                "note",
                "accepted_risks",
            ],
            "harness.close_task" => &[
                "envelope",
                "task_id",
                "intent",
                "close_reason",
                "superseding_task_id",
                "user_note",
            ],
            other => panic!("unexpected tool name: {other}"),
        }
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

    fn create_task(
        harness: &TestHarness,
        request_id: &str,
        idempotency_key: &str,
    ) -> Result<String, Box<dyn Error>> {
        let response = harness.core().intake(
            intake_request(request_id, false, Some(idempotency_key)),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");
        Ok(response.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task id should be present")
            .to_owned())
    }

    fn create_task_with_change_unit(
        harness: &TestHarness,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let task_id = create_task(
            harness,
            &format!("req_{suffix}_task"),
            &format!("idem_{suffix}_task"),
        )?;
        let response = harness.core().update_scope(
            update_scope_request(
                &format!("req_{suffix}_scope"),
                &format!("idem_{suffix}_scope"),
                &task_id,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert!(response.response_value["change_unit_ref"].is_object());
        Ok(task_id)
    }

    fn normalize_dry_run_required_ref_ids(mut value: Value) -> Value {
        let Some(next_actions) = value["dry_run_summary"]["next_actions"].as_array_mut() else {
            return value;
        };
        for action in next_actions {
            let Some(required_refs) = action["required_refs"].as_array_mut() else {
                continue;
            };
            for required_ref in required_refs {
                if required_ref["record_id"].is_string() {
                    required_ref["record_id"] = json!("<opaque-record-id>");
                }
                if required_ref["task_id"].is_string() {
                    required_ref["task_id"] = json!("<opaque-task-id>");
                }
            }
        }
        value
    }

    fn status_request(request_id: &str) -> StatusRequest {
        StatusRequest {
            envelope: envelope(request_id, None, false, None, None),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_authority: false,
                evidence: false,
                close: true,
                guarantees: true,
            },
        }
    }

    fn intake_request(
        request_id: &str,
        dry_run: bool,
        idempotency_key: Option<&str>,
    ) -> IntakeRequest {
        IntakeRequest {
            envelope: envelope(request_id, idempotency_key, dry_run, Some(0), None),
            plain_language_request: "Prepare a local MCP adapter test task.".to_owned(),
            requested_mode: RequestedMode::Work,
            resume_policy: ResumePolicy::CreateNew,
            initial_scope: InitialScope {
                boundary: "Local MCP adapter behavior.".to_owned(),
                non_goals: vec!["Changing Core method semantics.".to_owned()],
                acceptance_criteria: vec!["Adapter calls return Core responses.".to_owned()],
            },
            initial_context_refs: Vec::new(),
        }
    }

    fn update_scope_request(
        request_id: &str,
        idempotency_key: &str,
        task_id: &str,
    ) -> UpdateScopeRequest {
        let fields = match json!({
            "scope_summary": "MCP adapter status parity Change Unit.",
            "affected_paths": ["src/mcp_adapter.rs"]
        }) {
            Value::Object(object) => object,
            _ => unreachable!("literal object"),
        };
        UpdateScopeRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                false,
                Some(1),
                Some(task_id),
            ),
            task_id: harness_types::TaskId::new(task_id),
            goal_summary: None.into(),
            scope_update: None.into(),
            scope_boundary: Some("MCP adapter status parity scope.".to_owned()).into(),
            non_goals: None.into(),
            acceptance_criteria: None.into(),
            autonomy_boundary: None.into(),
            baseline_ref: Some(harness_types::BaselineRef::new("baseline_mcp")).into(),
            change_unit: harness_types::ChangeUnitUpdate {
                operation: ChangeUnitOperation::CreateCurrent,
                fields,
            },
            related_scope_decision_refs: Vec::new(),
        }
    }

    fn stage_artifact_request(request_id: &str, task_id: &str) -> StageArtifactRequest {
        StageArtifactRequest {
            envelope: envelope(request_id, None, false, None, Some(task_id)),
            task_id: harness_types::TaskId::new(task_id),
            display_name: "adapter-note.txt".to_owned(),
            content_type: "text/plain".to_owned(),
            redaction_state: RedactionState::None,
            safe_bytes_or_notice: "Adapter staging test note.".to_owned(),
            expected_sha256: None.into(),
            expected_size_bytes: None.into(),
            relation_hint: Some("adapter_test".to_owned()).into(),
        }
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
            task_id: task_id.map(harness_types::TaskId::new).into(),
            actor_kind: ActorKind::Agent,
            surface_id: SurfaceId::new(SURFACE_ID),
            request_id: harness_types::RequestId::new(request_id),
            idempotency_key: idempotency_key
                .map(harness_types::IdempotencyKey::new)
                .into(),
            expected_state_version: expected_state_version.into(),
            dry_run,
            locale: Some("en-US".to_owned()).into(),
        }
    }

    fn invocation(access_class: AccessClass) -> InvocationContext {
        InvocationContext {
            binding: AdapterSessionBinding::new(
                ProjectId::new(PROJECT_ID),
                SurfaceId::new(SURFACE_ID),
                SurfaceInstanceId::new(SURFACE_INSTANCE_ID),
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
            requested_access_class: access_class,
        }
    }

    fn initialize_request_message(id: Value, protocol_version: &str) -> Value {
        request_message(
            id,
            "initialize",
            Some(json!({
                "protocolVersion": protocol_version,
                "capabilities": {},
                "clientInfo": {
                    "name": "harness-unit-test",
                    "version": "0.0.0"
                }
            })),
        )
    }

    fn request_message(id: Value, method: &str, params: Option<Value>) -> Value {
        let mut message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method
        });
        if let Some(params) = params {
            message["params"] = params;
        }
        message
    }

    fn notification_message(method: &str, params: Option<Value>) -> Value {
        let mut message = json!({
            "jsonrpc": "2.0",
            "method": method
        });
        if let Some(params) = params {
            message["params"] = params;
        }
        message
    }

    fn initialized_notification() -> Value {
        initialized_notification_with_params(json!({}))
    }

    fn initialized_notification_with_params(params: Value) -> Value {
        notification_message("notifications/initialized", Some(params))
    }

    fn run_stdio_messages(
        adapter: McpAdapter,
        messages: &[Value],
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut input = Vec::new();
        for message in messages {
            serde_json::to_writer(&mut input, message)?;
            input.push(b'\n');
        }
        run_stdio_bytes(adapter, input)
    }

    fn run_stdio_text(adapter: McpAdapter, input: &str) -> Result<Vec<Value>, Box<dyn Error>> {
        run_stdio_bytes(adapter, input.as_bytes().to_vec())
    }

    fn run_stdio_bytes(adapter: McpAdapter, input: Vec<u8>) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut output = Vec::new();
        run_stdio(adapter, BufReader::new(Cursor::new(input)), &mut output)?;
        stdio_responses(&output)
    }

    fn assert_error_response(response: &Value, expected_id: Value, expected_code: i64) {
        assert_eq!(response["id"], expected_id);
        assert_eq!(response["error"]["code"], expected_code);
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
    fn runtime_home_env_resolution_matches_shared_resolver() -> Result<(), Box<dyn Error>> {
        let current_dir = TempRuntimeHome::new("mcp-current-dir")?;
        fn env_var(name: &str) -> Option<OsString> {
            match name {
                "HARNESS_HOME" => Some(OsString::from("runtime-home")),
                _ => None,
            }
        }

        let adapter = resolve_runtime_home(env_var, current_dir.path())?;
        let shared = resolve_store_runtime_home(env_var, current_dir.path())?;

        assert_eq!(adapter, shared);
        assert_eq!(adapter, current_dir.path().join("runtime-home"));
        Ok(())
    }

    #[test]
    fn runtime_home_env_resolution_uses_shared_fallback_order() -> Result<(), Box<dyn Error>> {
        let current_dir = TempRuntimeHome::new("mcp-fallback-current-dir")?;

        let resolved = resolve_runtime_home(
            |name| match name {
                "HOME" => Some(OsString::new()),
                "USERPROFILE" => Some(OsString::from("userprofile-home")),
                "HOMEDRIVE" => Some(OsString::from("unused-drive")),
                "HOMEPATH" => Some(OsString::from("unused-path")),
                _ => None,
            },
            current_dir.path(),
        )?;

        assert_eq!(
            resolved,
            current_dir.path().join("userprofile-home").join(".harness")
        );
        Ok(())
    }

    #[test]
    fn runtime_home_env_resolution_errors_are_environment_errors() {
        let current_dir = TempRuntimeHome::new("mcp-empty-current-dir").expect("temp current dir");

        let error = resolve_runtime_home(
            |name| {
                if name == "HARNESS_HOME" {
                    Some(OsString::new())
                } else {
                    None
                }
            },
            current_dir.path(),
        )
        .expect_err("empty HARNESS_HOME should fail");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error.to_string().contains("HARNESS_HOME must not be empty"));
    }

    #[test]
    fn compatible_runtime_home_from_env_helper_still_accepts_absolute_override(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("mcp-helper")?;
        let explicit = runtime_home.path().join("explicit");

        let resolved = resolve_runtime_home_from_env(|name| {
            if name == "HARNESS_HOME" {
                Some(explicit.clone().into_os_string())
            } else {
                None
            }
        })?;

        assert_eq!(resolved, explicit);
        Ok(())
    }

    fn local_access(access_classes: &[AccessClass]) -> Value {
        let names = access_classes
            .iter()
            .map(|access_class| access_class.as_str())
            .collect::<Vec<_>>();
        json!({
            "authorized_access_classes": names,
            "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
        })
    }

    #[test]
    fn startup_local_access_rejects_obsolete_or_incomplete_shapes() {
        for grant in [
            json!({"access_class": "read_status"}),
            json!({
                "access_class": "read_status",
                "authorized_access_classes": ["read_status"],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({"verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION}),
            json!({
                "authorized_access_classes": "read_status",
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({
                "authorized_access_classes": [],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({
                "authorized_access_classes": ["unknown"],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({"authorized_access_classes": ["read_status"]}),
            json!({
                "authorized_access_classes": ["read_status"],
                "verification_basis": ""
            }),
        ] {
            assert!(
                startup_authorized_access_classes(&grant.to_string()).is_err(),
                "grant should be rejected: {grant}"
            );
        }
    }
}
