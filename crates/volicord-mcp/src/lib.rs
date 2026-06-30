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
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
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
    bootstrap::{require_installation_profile, runtime_home_record, ACTIVE_PROJECT_STATUS},
    core_pipeline::CoreProjectStore,
    runtime_home::{
        resolve_runtime_home as resolve_shared_runtime_home, RuntimeHomeResolutionError,
    },
    StoreError,
};
use volicord_types::{
    mcp_request_schema, ActorSource, AgentConnectionId, AgentConnectionMode, CloseIntent,
    CloseTaskRequest, IdempotencyKey, IntakeRequest, JsonObject, JudgmentKind, JudgmentRationale,
    JudgmentResolutionOutcome, McpCheckCloseArguments, McpCloseTaskArguments, McpIntakeArguments,
    McpPrepareWriteArguments, McpRecordRunArguments, McpRequestUserJudgmentArguments,
    McpStageArtifactArguments, McpStatusArguments, McpUpdateScopeArguments,
    MethodOperationCategory, OperationCategory, PrepareWriteRequest, ProjectId, RecordRunRequest,
    RecordUserJudgmentPayload, RecordUserJudgmentRequest, RequestId, RequestUserJudgmentRequest,
    RequiredNullable, StageArtifactRequest, StatusRequest, ToolEnvelope, UpdateScopeRequest,
    UserJudgment, UserJudgmentOption, UserJudgmentOptionAction,
    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-11-25";
const SERVER_NAME: &str = "volicord-mcp";
const DEFAULT_INVOCATION_BINDING_BASIS: &str = VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING;
const DEFAULT_LOCALE: &str = "en-US";
const CHECK_CLOSE_TOOL_NAME: &str = "volicord.check_close";
const ELICITATION_CREATE_METHOD: &str = "elicitation/create";
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Agent-facing Volicord tools exposed through workflow MCP connections.
pub const PUBLIC_METHOD_TOOL_NAMES: [&str; 9] = [
    "volicord.intake",
    "volicord.update_scope",
    "volicord.status",
    "volicord.prepare_write",
    "volicord.stage_artifact",
    "volicord.record_run",
    "volicord.request_user_judgment",
    CHECK_CLOSE_TOOL_NAME,
    "volicord.close_task",
];

/// Public method tools exposed through read-only MCP connections.
pub const READ_ONLY_METHOD_TOOL_NAMES: [&str; 2] = ["volicord.status", CHECK_CLOSE_TOOL_NAME];

/// Adapter-owned MCP utility tools that are not public Core methods.
pub const ADAPTER_UTILITY_TOOL_NAMES: [&str; 1] = ["volicord.list_projects"];

const LIST_PROJECTS_TOOL_NAME: &str = "volicord.list_projects";
const SERVER_INSTRUCTIONS: &str = "Volicord records task scope, write readiness, evidence, runs, user-owned judgment requests, artifacts, and close readiness for explicitly registered Product Repositories. If project selection is unclear, call volicord.list_projects and use one listed project_selector; do not guess from folders, roots, labels, or memory. Volicord state management is separate from permission to edit product files: product-file edits still require the host/user path and any required Write Check. These instructions are guidance, not access control or a promise of automatic tool use.";

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
    pub connection_internal_id: AgentConnectionId,
    pub mode: AgentConnectionMode,
    pub invocation_binding_basis: String,
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
            self.connection_internal_id.as_str(),
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
    project_selector: String,
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
        Ok(mcp_tools_for_mode(mode))
    }

    /// Derives local invocation facts for one decoded request envelope.
    pub fn derive_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
    ) -> McpDerivedInvocationContext {
        McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(
                self.context.connection_internal_id.clone(),
            ),
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
            "volicord.intake" => self.call_intake(tool_name, params),
            "volicord.update_scope" => self.call_update_scope(tool_name, params),
            "volicord.status" => self.call_status(tool_name, params),
            "volicord.prepare_write" => self.call_prepare_write(tool_name, params),
            "volicord.stage_artifact" => self.call_stage_artifact(tool_name, params),
            "volicord.record_run" => self.call_record_run(tool_name, params),
            "volicord.request_user_judgment" => self.call_request_user_judgment(tool_name, params),
            CHECK_CLOSE_TOOL_NAME => self.call_check_close(tool_name, params),
            "volicord.close_task" => self.call_close_task(tool_name, params),
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn call_intake(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpIntakeArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
        )
    }

    fn call_update_scope(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpUpdateScopeArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
        )
    }

    fn call_status(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStatusArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
        )
    }

    fn call_prepare_write(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareWriteArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
        )
    }

    fn call_stage_artifact(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStageArtifactArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
        )
    }

    fn call_record_run(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRecordRunArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
                write_check_id: args.write_check_id,
                summary: args.summary,
                observed_changes: args.observed_changes,
                artifact_inputs: args.artifact_inputs,
                evidence_updates: args.evidence_updates,
                evidence_observations: args.evidence_observations,
                close_assessment: args.close_assessment,
            },
            CoreService::record_run,
        )
    }

    fn call_request_user_judgment(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRequestUserJudgmentArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
        )
    }

    fn call_check_close(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCheckCloseArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::Read,
        )?;
        self.call_core_request(
            tool_name,
            CloseTaskRequest {
                envelope,
                task_id,
                intent: CloseIntent::Check,
                close_reason: RequiredNullable::null(),
                superseding_task_id: RequiredNullable::null(),
                user_note: RequiredNullable::null(),
            },
            CoreService::close_task,
        )
    }

    fn call_close_task(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCloseTaskArguments> =
            self.prepare_mcp_arguments(tool_name, params)?;
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
                intent: args.intent.into(),
                close_reason: args.close_reason,
                superseding_task_id: args.superseding_task_id,
                user_note: args.user_note,
            },
            CoreService::close_task,
        )
    }

    fn call_core_request<T, F>(
        &self,
        tool_name: &str,
        request: T,
        call: F,
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
        let invocation =
            self.derive_invocation_context(request_envelope(&request), operation_category);
        call(&self.core, request, invocation.core_invocation()).map_err(McpAdapterError::Core)
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
            self.context.connection_internal_id.as_str(),
            "volicord.list_projects",
        )?;
        let projects = list_connection_projects(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?;
        let items = projects
            .iter()
            .map(|project| inspect_allowed_project(&self.runtime_home, project))
            .map(|project| ListProjectItem {
                project_selector: project.project_id,
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
            connection_id: connection.connection_internal_id,
            mode,
            projects: items,
        })
    }

    fn prepare_mcp_arguments<T>(
        &self,
        tool_name: &str,
        params: Value,
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
        let arguments = self.decode_params(tool_name, params)?;
        Ok(PreparedMcpArguments {
            arguments,
            project_id: selected_project_id,
        })
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
            let access = agent_connection_project_access(
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
            let availability = inspect_allowed_project(&self.runtime_home, &project_record);
            return selected_project_from_availability(availability);
        }

        let projects = list_connection_projects(&self.runtime_home, connection_internal_id)
            .map_err(McpAdapterError::Store)?;
        if projects.is_empty() {
            return Err(routing_error(
                "connection has no allowed projects; ask the operator to add one",
            ));
        }
        if projects.len() != 1 {
            return Err(routing_error(
                "project selection is ambiguous for this connection; project_selector is required when multiple projects are allowed",
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

struct PreparedMcpArguments<T> {
    arguments: T,
    project_id: ProjectId,
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
            input_schema: mcp_request_schema(name).expect("MCP tool schema should exist"),
        })
        .collect()
}

/// Runs a line-delimited JSON-RPC MCP stdio loop.
pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, mut writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let mut state = ConnectionState::default();
    let mut lines = reader.lines();

    while let Some(line) = lines.next() {
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

        if let Some(response) =
            handle_json_rpc_message(&adapter, &mut state, message, &mut lines, &mut writer)?
        {
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
                "setup has not been completed for Runtime Home {}; run `volicord setup` before starting `volicord mcp --stdio`",
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

    let context = McpConnectionContext {
        runtime_home,
        connection_internal_id: AgentConnectionId::new(connection.connection_internal_id.clone()),
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
enum ConnectionPhase {
    AwaitingInitialize,
    AwaitingInitialized,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConnectionState {
    phase: ConnectionPhase,
    client_supports_elicitation: bool,
    next_server_request_id: u64,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::AwaitingInitialize,
            client_supports_elicitation: false,
            next_server_request_id: 1,
        }
    }
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
    lines: &mut io::Lines<impl BufRead>,
    writer: &mut impl Write,
) -> Result<Option<Value>, McpAdapterError> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            handle_json_rpc_request(adapter, state, request, lines, writer).map(Some)
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(state, notification);
            Ok(None)
        }
        Err(error) => Ok(Some(json_rpc_error(
            error.id,
            error.code,
            error.message,
            error.data,
        ))),
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
        && state.phase == ConnectionPhase::AwaitingInitialized
        && notification_params_are_object_or_absent(notification.params.as_ref())
    {
        state.phase = ConnectionPhase::Ready;
    }
}

fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

fn handle_json_rpc_request<R, W>(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    request: JsonRpcRequest,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<Value, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    if let Some(error) = lifecycle_error(state.phase, &request) {
        return Ok(error);
    }

    let response_id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => {
            match validate_initialize_params(&response_id, request.params) {
                Ok(capabilities) => {
                    state.client_supports_elicitation = capabilities.elicitation;
                    state.phase = ConnectionPhase::AwaitingInitialized;
                }
                Err(error) => return Ok(error),
            }
            initialize_result()
        }
        "ping" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "ping")
            {
                return Ok(error);
            }
            json!({})
        }
        "tools/list" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "tools/list")
            {
                return Ok(error);
            }
            match adapter.tools() {
                Ok(tools) => json!({ "tools": tools }),
                Err(error) => return Ok(json_rpc_error_for_adapter(response_id, error)),
            }
        }
        "tools/call" => match call_tool_result_with_elicitation(
            adapter,
            &response_id,
            request.params,
            state.client_supports_elicitation,
            &mut state.next_server_request_id,
            lines,
            writer,
        )? {
            Ok(result) => result,
            Err(error) => return Ok(error),
        },
        _ => {
            return Ok(json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(request.method),
            ))
        }
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": response_id,
        "result": result
    }))
}

fn lifecycle_error(state: ConnectionPhase, request: &JsonRpcRequest) -> Option<Value> {
    match state {
        ConnectionPhase::AwaitingInitialize if request.method != "initialize" => Some(
            invalid_request_response(&request.id, "initialize must be the first request"),
        ),
        ConnectionPhase::AwaitingInitialize => None,
        ConnectionPhase::AwaitingInitialized => match request.method.as_str() {
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
        ConnectionPhase::Ready if request.method == "initialize" => Some(invalid_request_response(
            &request.id,
            "initialize has already completed",
        )),
        ConnectionPhase::Ready => None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClientCapabilities {
    elicitation: bool,
}

fn validate_initialize_params(
    id: &Value,
    params: Option<Value>,
) -> Result<ClientCapabilities, Value> {
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

    let elicitation = object
        .get("capabilities")
        .and_then(Value::as_object)
        .and_then(|capabilities| capabilities.get("elicitation"))
        .is_some_and(Value::is_object);

    Ok(ClientCapabilities { elicitation })
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

fn call_tool_result_with_elicitation<R, W>(
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
    client_supports_elicitation: bool,
    server_request_sequence: &mut u64,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<Result<Value, Value>, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let object = match required_object_params(id, params, "tools/call") {
        Ok(object) => object,
        Err(error) => return Ok(Err(error)),
    };
    if object.contains_key("task") {
        return Ok(Err(invalid_params_response(
            id,
            "tools/call task augmentation is not supported",
        )));
    }

    let tool_name = match object.get("name").and_then(Value::as_str) {
        Some(tool_name) => tool_name,
        None => {
            return Ok(Err(invalid_params_response(
                id,
                "tools/call params.name must be a string",
            )))
        }
    };
    if !is_known_mcp_tool(tool_name) {
        return Ok(Err(json_rpc_error(
            id.clone(),
            -32602,
            "Invalid params",
            Some(format!("unknown MCP tool: {tool_name}")),
        )));
    }

    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            return Ok(Err(invalid_params_response(
                id,
                "tools/call params.arguments must be an object",
            )))
        }
    };

    let output = if PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) {
        match adapter.call_tool(tool_name, arguments) {
            Ok(response) if tool_name == "volicord.request_user_judgment" => {
                user_judgment_tool_output(
                    adapter,
                    response,
                    client_supports_elicitation,
                    server_request_sequence,
                    lines,
                    writer,
                )?
            }
            Ok(response) => ToolCallOutput::success(response.response_json),
            Err(error @ McpAdapterError::InvalidParams { .. })
            | Err(error @ McpAdapterError::ToolExecution { .. }) => {
                return Ok(Ok(tool_execution_error_result(&error)));
            }
            Err(error) => return Ok(Err(json_rpc_error_for_adapter(id.clone(), error))),
        }
    } else {
        let response = match adapter.call_adapter_tool(tool_name, arguments) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. })
            | Err(error @ McpAdapterError::ToolExecution { .. }) => {
                return Ok(Ok(tool_execution_error_result(&error)));
            }
            Err(error) => return Ok(Err(json_rpc_error_for_adapter(id.clone(), error))),
        };
        let text = serde_json::to_string(&response)
            .map_err(McpAdapterError::Json)
            .map_err(|error| json_rpc_error_for_adapter(id.clone(), error));
        match text {
            Ok(text) => ToolCallOutput::success(text),
            Err(error) => return Ok(Err(error)),
        }
    };

    Ok(Ok(tool_call_result_from_output(output)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolCallOutput {
    primary_text: String,
    extra_texts: Vec<String>,
    is_error: bool,
}

impl ToolCallOutput {
    fn success(primary_text: String) -> Self {
        Self {
            primary_text,
            extra_texts: Vec::new(),
            is_error: false,
        }
    }

    fn with_extra(mut self, text: impl Into<String>) -> Self {
        self.extra_texts.push(text.into());
        self
    }
}

fn tool_call_result_from_output(output: ToolCallOutput) -> Value {
    let mut content = vec![json!({
        "type": "text",
        "text": output.primary_text
    })];
    content.extend(output.extra_texts.into_iter().map(|text| {
        json!({
            "type": "text",
            "text": text
        })
    }));

    json!({
        "content": content,
        "isError": output.is_error
    })
}

fn user_judgment_tool_output<R, W>(
    adapter: &McpAdapter,
    pending_response: PipelineResponse,
    client_supports_elicitation: bool,
    server_request_sequence: &mut u64,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<ToolCallOutput, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let Some(pending) = pending_judgment_from_response(&pending_response) else {
        return Ok(ToolCallOutput::success(pending_response.response_json));
    };

    if !client_supports_elicitation {
        return Ok(ToolCallOutput::success(pending_response.response_json)
            .with_extra(chat_capture_fallback_instructions(adapter, &pending)?));
    }

    if let Some(reason) = elicitation_secret_request_risk(&pending) {
        return Ok(ToolCallOutput::success(pending_response.response_json).with_extra(format!(
            "Volicord did not open MCP elicitation for pending judgment `{}` because the prompt text appears to request or expose sensitive secret material ({reason}). Do not ask the user to enter secrets, credentials, tokens, or private keys through MCP elicitation. The judgment remains pending for a safe User Channel recovery path.",
            pending.judgment_id.as_str()
        )));
    }

    let request_id = next_server_request_id("elicit_user_judgment", server_request_sequence);
    let request = elicitation_create_request(&request_id, &pending);
    write_json_line(writer, request)?;
    writer.flush().map_err(McpAdapterError::Io)?;

    match read_elicitation_response(&request_id, lines) {
        ElicitationReply::Accepted {
            selected_option_id,
            note,
        } => match record_elicited_judgment(adapter, &pending, &selected_option_id, note)? {
            ElicitedRecordOutcome::Recorded(recorded) => Ok(ToolCallOutput::success(
                recorded.response_json,
            )
            .with_extra(format!(
                "Volicord recorded pending judgment `{}` through MCP elicitation with User Channel basis `{}`.",
                pending.judgment_id.as_str(),
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            ))),
            ElicitedRecordOutcome::InvalidSelection(message) => Ok(ToolCallOutput::success(
                pending_response.response_json,
            )
            .with_extra(format!(
                "{message} The pending judgment remains unresolved."
            ))),
        },
        ElicitationReply::Declined => match reject_option_id(&pending) {
            Some(option_id) => match record_elicited_judgment(adapter, &pending, option_id, None)? {
                ElicitedRecordOutcome::Recorded(recorded) => Ok(ToolCallOutput::success(
                    recorded.response_json,
                )
                .with_extra(format!(
                    "Volicord recorded pending judgment `{}` as rejected through MCP elicitation with User Channel basis `{}`.",
                    pending.judgment_id.as_str(),
                    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
                ))),
                ElicitedRecordOutcome::InvalidSelection(message) => Ok(ToolCallOutput::success(
                    pending_response.response_json,
                )
                .with_extra(format!(
                    "{message} The pending judgment remains unresolved."
                ))),
            },
            None => Ok(ToolCallOutput::success(pending_response.response_json).with_extra(
                "The MCP client declined the elicitation request, but this judgment has no Core reject option to record. The pending judgment remains unresolved.",
            )),
        },
        ElicitationReply::Cancelled => Ok(ToolCallOutput::success(pending_response.response_json)
            .with_extra(format!(
                "The MCP client cancelled or dismissed elicitation for pending judgment `{}`. Volicord did not record an answer; the judgment remains pending.",
                pending.judgment_id.as_str()
            ))),
        ElicitationReply::Invalid(message) => Ok(ToolCallOutput::success(
            pending_response.response_json,
        )
        .with_extra(format!(
            "Volicord rejected the MCP elicitation response: {message}. The pending judgment remains unresolved."
        ))),
        ElicitationReply::Unavailable(message) => Ok(ToolCallOutput::success(
            pending_response.response_json,
        )
        .with_extra(format!(
            "MCP elicitation was unavailable after the client advertised support: {message}. {}",
            chat_capture_fallback_instructions(adapter, &pending)?
        ))),
    }
}

fn pending_judgment_from_response(response: &PipelineResponse) -> Option<UserJudgment> {
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return None;
    }
    let judgment = serde_json::from_value::<UserJudgment>(
        response.response_value.get("user_judgment")?.clone(),
    )
    .ok()?;
    (judgment.resolution.is_none()).then_some(judgment)
}

fn elicitation_create_request(id: &str, judgment: &UserJudgment) -> Value {
    let option_ids = judgment
        .options
        .iter()
        .map(|option| option.option_id.as_str())
        .collect::<Vec<_>>();
    let option_names = judgment
        .options
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<_>>();
    let option_lines = judgment
        .options
        .iter()
        .map(|option| {
            format!(
                "- {} (`{}`): {}",
                option.label,
                option.option_id.as_str(),
                option.consequence
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let message = format!(
        "Volicord needs a user-owned judgment for Task `{}`.\n\nQuestion: {}\n\nContext: {}\n\nOptions:\n{}\n\nSelect exactly one option. Do not enter secrets, credentials, tokens, private keys, or other private secret material.",
        judgment.task_id.as_str(),
        judgment.question,
        judgment.context.summary,
        option_lines
    );

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": ELICITATION_CREATE_METHOD,
        "params": {
            "message": message,
            "requestedSchema": {
                "type": "object",
                "properties": {
                    "selected_option_id": {
                        "type": "string",
                        "title": "Judgment option",
                        "description": "The exact Volicord option_id selected by the user.",
                        "enum": option_ids,
                        "enumNames": option_names
                    },
                    "note": {
                        "type": "string",
                        "title": "Optional note",
                        "description": "Optional user note for this judgment. Do not include secrets, credentials, tokens, or private keys.",
                        "maxLength": 1000
                    }
                },
                "required": ["selected_option_id"]
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ElicitationReply {
    Accepted {
        selected_option_id: String,
        note: Option<String>,
    },
    Declined,
    Cancelled,
    Invalid(String),
    Unavailable(String),
}

fn read_elicitation_response<R: BufRead>(
    request_id: &str,
    lines: &mut io::Lines<R>,
) -> ElicitationReply {
    let Some(line) = lines.next() else {
        return ElicitationReply::Unavailable(
            "stdin closed before the client responded".to_owned(),
        );
    };
    let line = match line {
        Ok(line) => line,
        Err(error) => {
            return ElicitationReply::Unavailable(format!(
                "failed to read elicitation response: {error}"
            ))
        }
    };
    let value: Value = match serde_json::from_str(&line) {
        Ok(value) => value,
        Err(error) => {
            return ElicitationReply::Invalid(format!("response was not valid JSON: {error}"))
        }
    };
    let Some(object) = value.as_object() else {
        return ElicitationReply::Invalid("response must be a JSON-RPC object".to_owned());
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return ElicitationReply::Invalid("response jsonrpc must be exactly \"2.0\"".to_owned());
    }
    if object.get("id").and_then(Value::as_str) != Some(request_id) {
        return ElicitationReply::Invalid(
            "response id did not match the elicitation request".to_owned(),
        );
    }
    if let Some(error) = object.get("error") {
        return ElicitationReply::Unavailable(format!(
            "client returned JSON-RPC error: {}",
            concise_json(error)
        ));
    }
    let Some(result) = object.get("result").and_then(Value::as_object) else {
        return ElicitationReply::Invalid("response result must be an object".to_owned());
    };
    match result.get("action").and_then(Value::as_str) {
        Some("accept") => {
            let Some(content) = result.get("content").and_then(Value::as_object) else {
                return ElicitationReply::Invalid(
                    "accepted elicitation must include object content".to_owned(),
                );
            };
            let Some(selected_option_id) =
                content.get("selected_option_id").and_then(Value::as_str)
            else {
                return ElicitationReply::Invalid(
                    "accepted elicitation content.selected_option_id must be a string".to_owned(),
                );
            };
            if selected_option_id.trim().is_empty() {
                return ElicitationReply::Invalid(
                    "accepted elicitation selected_option_id must not be empty".to_owned(),
                );
            }
            let note = match content.get("note") {
                None | Some(Value::Null) => None,
                Some(Value::String(note)) if note.len() <= 1000 => Some(note.clone()),
                Some(Value::String(_)) => {
                    return ElicitationReply::Invalid(
                        "accepted elicitation note must be at most 1000 characters".to_owned(),
                    )
                }
                Some(_) => {
                    return ElicitationReply::Invalid(
                        "accepted elicitation note must be a string when supplied".to_owned(),
                    )
                }
            };
            ElicitationReply::Accepted {
                selected_option_id: selected_option_id.to_owned(),
                note,
            }
        }
        Some("decline") => ElicitationReply::Declined,
        Some("cancel") => ElicitationReply::Cancelled,
        Some(other) => {
            ElicitationReply::Invalid(format!("unsupported elicitation action `{other}`"))
        }
        None => ElicitationReply::Invalid("response result.action must be a string".to_owned()),
    }
}

enum ElicitedRecordOutcome {
    Recorded(PipelineResponse),
    InvalidSelection(String),
}

fn record_elicited_judgment(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
    selected_option_id: &str,
    note: Option<String>,
) -> Result<ElicitedRecordOutcome, McpAdapterError> {
    let Some(selected_option) = judgment
        .options
        .iter()
        .find(|option| option.option_id.as_str() == selected_option_id)
    else {
        return Ok(ElicitedRecordOutcome::InvalidSelection(format!(
            "MCP elicitation selected unknown option_id `{selected_option_id}` for pending judgment `{}`.",
            judgment.judgment_id.as_str()
        )));
    };
    let state_version = judgment.basis.created_at_state_version + 1;
    let request = RecordUserJudgmentRequest {
        envelope: ToolEnvelope {
            project_id: judgment.project_id.clone(),
            task_id: Some(judgment.task_id.clone()).into(),
            request_id: RequestId::new(generated_metadata_id(
                "req_mcp_elicitation_record",
                adapter.context.connection_internal_id.as_str(),
                "volicord.record_user_judgment",
            )),
            idempotency_key: Some(IdempotencyKey::new(generated_metadata_id(
                "idem_mcp_elicitation_record",
                adapter.context.connection_internal_id.as_str(),
                "volicord.record_user_judgment",
            )))
            .into(),
            expected_state_version: Some(state_version).into(),
            dry_run: false,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        },
        user_judgment_id: judgment.judgment_id.clone(),
        judgment_kind: judgment.judgment_kind,
        selected_option_id: selected_option.option_id.clone(),
        answer: answer_payload_for_judgment(judgment, selected_option)?,
        rationale: rationale_for_selected_option(judgment.judgment_kind, selected_option),
        note: note.into(),
        accepted_risks: accepted_risks_for_judgment(judgment, selected_option),
    };
    let invocation = InvocationContext::new(
        judgment.project_id.clone(),
        ActorSource::LocalUser,
        OperationCategory::UserOnly,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
    );
    adapter
        .core
        .record_user_judgment(request, invocation)
        .map(ElicitedRecordOutcome::Recorded)
        .map_err(McpAdapterError::Core)
}

fn answer_payload_for_judgment(
    judgment: &UserJudgment,
    selected_option: &UserJudgmentOption,
) -> Result<RecordUserJudgmentPayload, McpAdapterError> {
    let mut payload = empty_answer_payload();
    let branch = json_object(json!({
        "summary": format!("User selected option {}", selected_option.option_id.as_str()),
        "selected_option": selected_option.option_id.as_str(),
        "selected_option_label": selected_option.label,
        "selected_option_consequence": selected_option.consequence,
    }));
    match judgment.judgment_kind {
        JudgmentKind::ProductDecision => payload.product_decision = Some(branch).into(),
        JudgmentKind::TechnicalDecision => payload.technical_decision = Some(branch).into(),
        JudgmentKind::ScopeDecision => payload.scope_decision = Some(branch).into(),
        JudgmentKind::SensitiveApproval => {
            let Some(scope) = judgment.basis.sensitive_action_scope.as_ref() else {
                return Err(McpAdapterError::ToolExecution {
                    tool_name: "volicord.request_user_judgment".to_owned(),
                    message: "pending sensitive approval is missing its Core-derived sensitive action scope".to_owned(),
                });
            };
            payload.sensitive_action_scope = Some(scope.clone()).into();
        }
        JudgmentKind::FinalAcceptance => payload.final_acceptance = Some(branch).into(),
        JudgmentKind::ResidualRiskAcceptance => {
            payload.residual_risk_acceptance = Some(json_object(json!({
                "summary": format!("User selected option {}", selected_option.option_id.as_str()),
                "selected_option": selected_option.option_id.as_str(),
                "risk_ids": accepted_risk_ids(selected_option, judgment),
            })))
            .into();
        }
        JudgmentKind::Cancellation => payload.cancellation = Some(branch).into(),
    }
    Ok(payload)
}

fn empty_answer_payload() -> RecordUserJudgmentPayload {
    RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    }
}

fn rationale_for_selected_option(
    judgment_kind: JudgmentKind,
    selected_option: &UserJudgmentOption,
) -> JudgmentRationale {
    let accepted = selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted;
    JudgmentRationale {
        summary: format!(
            "User selected `{}` for `{}` through MCP elicitation.",
            selected_option.option_id.as_str(),
            judgment_kind_value(judgment_kind)
        ),
        selected_reason: Some(format!(
            "{} {}",
            selected_option.description, selected_option.consequence
        ))
        .into(),
        considered_alternatives: Vec::new(),
        rejected_alternatives: Vec::new(),
        assumptions: vec!["The answer covers only the addressed Core UserJudgment.".to_owned()],
        tradeoffs: if accepted {
            vec![selected_option.consequence.clone()]
        } else {
            Vec::new()
        },
        uncertainties: Vec::new(),
        review_triggers: if accepted {
            vec!["Revisit if the captured judgment basis becomes stale or superseded.".to_owned()]
        } else {
            Vec::new()
        },
        related_refs: Vec::new(),
        artifact_refs: Vec::new(),
    }
}

fn accepted_risks_for_judgment(
    judgment: &UserJudgment,
    selected_option: &UserJudgmentOption,
) -> Vec<volicord_types::AcceptedRiskInput> {
    if judgment.judgment_kind == JudgmentKind::ResidualRiskAcceptance
        && selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted
    {
        judgment.context.visible_risks.clone()
    } else {
        Vec::new()
    }
}

fn accepted_risk_ids(selected_option: &UserJudgmentOption, judgment: &UserJudgment) -> Vec<String> {
    if selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted {
        judgment
            .context
            .visible_risks
            .iter()
            .map(|risk| risk.risk_id.as_str().to_owned())
            .collect()
    } else {
        Vec::new()
    }
}

fn reject_option_id(judgment: &UserJudgment) -> Option<&str> {
    judgment
        .options
        .iter()
        .find(|option| option.machine_action == UserJudgmentOptionAction::Reject)
        .map(|option| option.option_id.as_str())
}

fn chat_capture_fallback_instructions(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
) -> Result<String, McpAdapterError> {
    let store = CoreProjectStore::open(&adapter.runtime_home, &judgment.project_id)
        .map_err(McpAdapterError::Store)?;
    let records = store
        .user_judgment_records_for_task(&judgment.task_id)
        .map_err(McpAdapterError::Store)?;
    let chat_index = records
        .iter()
        .position(|record| record.judgment_id == judgment.judgment_id.as_str())
        .map(|index| index + 1)
        .unwrap_or(1);
    let chat_id = format!("J-{chat_index}");
    let options = judgment
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            format!(
                "`Volicord: answer {chat_id} {}` for option `{}` ({})",
                chat_option_selector(index + 1, option),
                option.option_id.as_str(),
                option.label
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Ok(format!(
        "MCP elicitation is unavailable. The pending judgment `{}` remains unresolved. To use chat prompt capture, ask the user to send one exact command in chat: {options}. To defer with a note, use `Volicord: note {chat_id} \"text\"`. Do not ask the user to include secrets, credentials, tokens, or private keys.",
        judgment.judgment_id.as_str()
    ))
}

fn chat_option_selector(index: usize, option: &UserJudgmentOption) -> String {
    match option.machine_action {
        UserJudgmentOptionAction::Reject => "reject".to_owned(),
        UserJudgmentOptionAction::Defer => "defer".to_owned(),
        UserJudgmentOptionAction::Accept => index.to_string(),
    }
}

fn elicitation_secret_request_risk(judgment: &UserJudgment) -> Option<&'static str> {
    let mut text = String::new();
    text.push_str(&judgment.question);
    text.push('\n');
    text.push_str(&judgment.context.summary);
    for constraint in &judgment.context.constraints {
        text.push('\n');
        text.push_str(constraint);
    }
    for option in &judgment.options {
        text.push('\n');
        text.push_str(&option.label);
        text.push('\n');
        text.push_str(&option.description);
        text.push('\n');
        text.push_str(&option.consequence);
    }
    let normalized = text.to_ascii_lowercase();
    [
        "password",
        "passphrase",
        "private key",
        "api key",
        "secret",
        "credential",
        "token",
    ]
    .into_iter()
    .find(|needle| normalized.contains(needle))
}

fn judgment_kind_value(value: JudgmentKind) -> &'static str {
    match value {
        JudgmentKind::ProductDecision => "product_decision",
        JudgmentKind::TechnicalDecision => "technical_decision",
        JudgmentKind::ScopeDecision => "scope_decision",
        JudgmentKind::SensitiveApproval => "sensitive_approval",
        JudgmentKind::FinalAcceptance => "final_acceptance",
        JudgmentKind::ResidualRiskAcceptance => "residual_risk_acceptance",
        JudgmentKind::Cancellation => "cancellation",
    }
}

fn next_server_request_id(prefix: &str, next_server_request_id: &mut u64) -> String {
    let sequence = *next_server_request_id;
    *next_server_request_id = next_server_request_id.saturating_add(1);
    format!("{prefix}_{sequence}")
}

fn concise_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "unserializable JSON value".to_owned())
}

fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => JsonObject::new(),
    }
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
        CHECK_CLOSE_TOOL_NAME => "Check close readiness for a selected Task.",
        "volicord.close_task" => "Perform a selected Task close path.",
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
            message: format!("{field} must be a non-empty string when supplied"),
        }),
    }
}

fn reject_internal_mcp_argument_fields(
    object: &Map<String, Value>,
    tool_name: &str,
) -> Result<(), McpAdapterError> {
    for field in [
        "envelope",
        "project_id",
        "request_id",
        "idempotency_key",
        "expected_state_version",
        "dry_run",
        "locale",
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

fn generated_metadata_id(prefix: &str, connection_id: &str, tool_name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{prefix}_{}_{}_{}_{}",
        sanitize_metadata_component(connection_id),
        sanitize_metadata_component(tool_name),
        nanos,
        sequence
    )
}

fn sanitize_metadata_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
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
        assert!(workflow_names.contains(&CHECK_CLOSE_TOOL_NAME));
        assert!(workflow_names.contains(&"volicord.close_task"));
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
                CHECK_CLOSE_TOOL_NAME,
                LIST_PROJECTS_TOOL_NAME
            ]
        );
    }

    #[test]
    fn mcp_visible_schemas_hide_envelope_and_metadata() {
        for tool in public_method_tools() {
            let properties = root_properties(&tool.input_schema);
            let required = root_required_fields(&tool.input_schema);
            assert!(
                properties.contains(&"project_selector".to_owned()),
                "{} should expose the public project selector",
                tool.name
            );
            assert!(
                !required.contains(&"project_selector".to_owned()),
                "{} should not require project selection for single-project connections",
                tool.name
            );
            for forbidden in [
                "envelope",
                "project_id",
                "request_id",
                "idempotency_key",
                "expected_state_version",
                "dry_run",
                "locale",
                "actor_source",
                "operation_category",
                "mode",
                "connection_id",
            ] {
                assert!(
                    !properties.contains(&forbidden.to_owned()),
                    "{} should not expose MCP-internal field {forbidden}",
                    tool.name
                );
            }
            assert!(
                !schema_has_definition(&tool.input_schema, "ToolEnvelope"),
                "{} should not include the internal ToolEnvelope schema",
                tool.name
            );
        }
    }

    #[test]
    fn connection_context_resolves_and_preflight_reports_allowed_project(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-context")?;

        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        assert_eq!(
            context.connection_internal_id.as_str(),
            fixture.connection_id()
        );
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

        let response = adapter.call_tool("volicord.status", json!({}))?;

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
                json!({
                    "plain_language_request": "Exercise read-only rejection.",
                    "requested_mode": "work",
                    "resume_policy": "create_new",
                    "initial_scope": {
                        "boundary": "Read-only rejection.",
                        "non_goals": [],
                        "acceptance_criteria": ["No Core mutation occurs."]
                    },
                    "initial_context_refs": []
                }),
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
                CHECK_CLOSE_TOOL_NAME,
                LIST_PROJECTS_TOOL_NAME
            ]
        );
        Ok(())
    }

    #[test]
    fn stdio_elicitation_accept_records_user_judgment() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicitation-accept")?;
        let setup_adapter = adapter(&fixture)?;
        let (task_id, state_version) = create_task(&setup_adapter)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({ "elicitation": {} })),
            initialized_notification(),
            tools_call(
                2,
                "volicord.request_user_judgment",
                product_judgment_args(&fixture, &task_id, state_version),
            ),
            elicitation_accept("keep", None),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        assert_eq!(values.len(), 3);
        assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
        assert_eq!(values[1]["id"], "elicit_user_judgment_1");
        assert_eq!(
            values[1]["params"]["requestedSchema"]["properties"]["selected_option_id"]["enum"][0],
            "keep"
        );
        let response = volicord_response_from_tool(&values[2])?;
        assert_eq!(response["base"]["response_kind"], "result");
        assert_eq!(response["user_judgment"]["status"], "resolved");
        assert_eq!(
            response["user_judgment"]["resolution"]["resolved_by_actor_source"],
            "local_user"
        );
        assert_eq!(
            response["user_judgment"]["resolution"]["selected_option_id"],
            "keep"
        );
        assert_eq!(
            stored_resolution_basis(&fixture, &task_id, &response)?,
            VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
        );
        Ok(())
    }

    #[test]
    fn stdio_elicitation_decline_records_rejected_authority_judgment() -> Result<(), Box<dyn Error>>
    {
        let fixture = CoreFixture::new("mcp-elicitation-decline")?;
        let setup_adapter = adapter(&fixture)?;
        let (task_id, state_version) = create_task(&setup_adapter)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({ "elicitation": {} })),
            initialized_notification(),
            tools_call(
                2,
                "volicord.request_user_judgment",
                authority_judgment_args(&fixture, &task_id, state_version),
            ),
            elicitation_action("decline"),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
        let response = volicord_response_from_tool(&values[2])?;
        assert_eq!(response["user_judgment"]["status"], "resolved");
        assert_eq!(
            response["user_judgment"]["resolution"]["selected_option_id"],
            "reject"
        );
        assert_eq!(
            response["user_judgment"]["resolution"]["resolution_outcome"],
            "rejected"
        );
        assert_eq!(
            stored_resolution_basis(&fixture, &task_id, &response)?,
            VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
        );
        Ok(())
    }

    #[test]
    fn stdio_elicitation_accept_can_record_deferred_judgment() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicitation-defer")?;
        let setup_adapter = adapter(&fixture)?;
        let (task_id, state_version) = create_task(&setup_adapter)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({ "elicitation": {} })),
            initialized_notification(),
            tools_call(
                2,
                "volicord.request_user_judgment",
                authority_judgment_args(&fixture, &task_id, state_version),
            ),
            elicitation_accept("defer", Some("Not enough context yet.")),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        let response = volicord_response_from_tool(&values[2])?;
        assert_eq!(response["user_judgment"]["status"], "resolved");
        assert_eq!(
            response["user_judgment"]["resolution"]["selected_option_id"],
            "defer"
        );
        assert_eq!(
            response["user_judgment"]["resolution"]["resolution_outcome"],
            "deferred"
        );
        assert_eq!(
            response["user_judgment"]["resolution"]["note"],
            "Not enough context yet."
        );
        Ok(())
    }

    #[test]
    fn stdio_elicitation_cancel_leaves_judgment_pending() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicitation-cancel")?;
        let setup_adapter = adapter(&fixture)?;
        let (task_id, state_version) = create_task(&setup_adapter)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({ "elicitation": {} })),
            initialized_notification(),
            tools_call(
                2,
                "volicord.request_user_judgment",
                product_judgment_args(&fixture, &task_id, state_version),
            ),
            elicitation_action("cancel"),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        let response = volicord_response_from_tool(&values[2])?;
        assert_eq!(response["user_judgment"]["status"], "pending");
        assert!(values[2]["result"]["content"][1]["text"]
            .as_str()
            .expect("extra text")
            .contains("remains pending"));
        let record = stored_judgment_record(&fixture, &task_id, &response)?;
        assert_eq!(record.status, "pending");
        assert!(record.resolved_verification_basis.is_none());
        Ok(())
    }

    #[test]
    fn stdio_elicitation_invalid_response_leaves_judgment_pending() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicitation-invalid")?;
        let setup_adapter = adapter(&fixture)?;
        let (task_id, state_version) = create_task(&setup_adapter)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({ "elicitation": {} })),
            initialized_notification(),
            tools_call(
                2,
                "volicord.request_user_judgment",
                product_judgment_args(&fixture, &task_id, state_version),
            ),
            elicitation_accept("not_an_option", None),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        let response = volicord_response_from_tool(&values[2])?;
        assert_eq!(response["user_judgment"]["status"], "pending");
        assert!(values[2]["result"]["content"][1]["text"]
            .as_str()
            .expect("extra text")
            .contains("unknown option_id"));
        let record = stored_judgment_record(&fixture, &task_id, &response)?;
        assert_eq!(record.status, "pending");
        Ok(())
    }

    #[test]
    fn stdio_without_elicitation_capability_returns_chat_capture_fallback(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicitation-unavailable")?;
        let setup_adapter = adapter(&fixture)?;
        let (task_id, state_version) = create_task(&setup_adapter)?;
        let adapter = adapter(&fixture)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({})),
            initialized_notification(),
            tools_call(
                2,
                "volicord.request_user_judgment",
                product_judgment_args(&fixture, &task_id, state_version),
            ),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        assert_eq!(values.len(), 2);
        let response = volicord_response_from_tool(&values[1])?;
        assert_eq!(response["user_judgment"]["status"], "pending");
        let fallback = values[1]["result"]["content"][1]["text"]
            .as_str()
            .expect("fallback text");
        assert!(fallback.contains("MCP elicitation is unavailable"));
        assert!(fallback.contains("Volicord: answer J-1 1"));
        assert!(fallback.contains("Volicord: note J-1"));
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
                connection_internal_id: existing.connection_internal_id,
                host_kind: existing.host_kind,
                intent: existing.intent,
                host_scope: existing.host_scope,
                server_name: existing.server_name,
                config_target: existing.config_target,
                mode: mode.to_owned(),
                enabled: existing.enabled,
                managed_fingerprint: existing.managed_fingerprint,
                last_verification_status: existing.last_verification_status,
                last_verification_report_json: existing.last_verification_report_json,
                last_user_actions_json: existing.last_user_actions_json,
                metadata_json: existing.metadata_json,
            },
        )?;
        Ok(())
    }

    fn create_task(adapter: &McpAdapter) -> Result<(String, u64), Box<dyn Error>> {
        let response = adapter.call_tool(
            "volicord.intake",
            json!({
                "plain_language_request": "Create a task for MCP elicitation tests.",
                "requested_mode": "work",
                "resume_policy": "create_new",
                "initial_scope": {
                    "boundary": "MCP elicitation test task.",
                    "non_goals": ["Changing unrelated behavior."],
                    "acceptance_criteria": ["A pending judgment can be requested."]
                },
                "initial_context_refs": []
            }),
        )?;
        let task_id = response.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task id")
            .to_owned();
        let state_version = response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version");
        Ok((task_id, state_version))
    }

    fn initialize_request(id: u64, capabilities: Value) -> Value {
        request(
            id,
            "initialize",
            json!({
                "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
                "capabilities": capabilities,
                "clientInfo": {
                    "name": "volicord-unit-test",
                    "version": "0.0.0"
                }
            }),
        )
    }

    fn initialized_notification() -> Value {
        notification("notifications/initialized", json!({}))
    }

    fn request(id: u64, method: &str, params: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        })
    }

    fn notification(method: &str, params: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        })
    }

    fn tools_call(id: u64, name: &str, arguments: Value) -> Value {
        request(
            id,
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments
            }),
        )
    }

    fn product_judgment_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
        judgment_args(
            fixture,
            task_id,
            state_version,
            "product_decision",
            json!([
                {
                    "option_id": "keep",
                    "label": "Keep focused behavior",
                    "description": "Record the user-owned product decision to keep the behavior.",
                    "consequence": "Only this focused judgment is resolved.",
                    "is_default": true
                },
                {
                    "option_id": "change",
                    "label": "Change focused behavior",
                    "description": "Record the user-owned product decision to change the behavior.",
                    "consequence": "Only this focused judgment is resolved with the alternate option.",
                    "is_default": false
                }
            ]),
            json!(["close_complete"]),
        )
    }

    fn authority_judgment_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
        judgment_args(
            fixture,
            task_id,
            state_version,
            "scope_decision",
            Value::Null,
            json!(["scope_update"]),
        )
    }

    fn judgment_args(
        fixture: &CoreFixture,
        task_id: &str,
        state_version: u64,
        judgment_kind: &str,
        options: Value,
        required_for: Value,
    ) -> Value {
        json!({
            "task_id": task_id,
            "change_unit_id": null,
            "judgment_kind": judgment_kind,
            "presentation": "short",
            "question": "Choose the focused MCP elicitation test outcome.",
            "options": options,
            "context": {
                "summary": "A focused test judgment needs a user-owned answer.",
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": ["The answer covers only this pending judgment."]
            },
            "affected_refs": [
                {
                    "record_kind": "task",
                    "record_id": task_id,
                    "project_id": fixture.project_id(),
                    "task_id": task_id,
                    "state_version": state_version
                }
            ],
            "required_for": required_for,
            "expires_at": null
        })
    }

    fn elicitation_accept(selected_option_id: &str, note: Option<&str>) -> Value {
        let mut content = json!({
            "selected_option_id": selected_option_id
        });
        if let Some(note) = note {
            content["note"] = json!(note);
        }
        json!({
            "jsonrpc": "2.0",
            "id": "elicit_user_judgment_1",
            "result": {
                "action": "accept",
                "content": content
            }
        })
    }

    fn elicitation_action(action: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": "elicit_user_judgment_1",
            "result": {
                "action": action
            }
        })
    }

    fn json_lines(messages: &[Value]) -> Result<Vec<u8>, serde_json::Error> {
        let mut output = Vec::new();
        for message in messages {
            serde_json::to_writer(&mut output, message)?;
            output.push(b'\n');
        }
        Ok(output)
    }

    fn volicord_response_from_tool(response: &Value) -> Result<Value, Box<dyn Error>> {
        assert_eq!(response["result"]["isError"], json!(false));
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("tools/call response should include text content")?;
        Ok(serde_json::from_str(text)?)
    }

    fn stored_resolution_basis(
        fixture: &CoreFixture,
        task_id: &str,
        response: &Value,
    ) -> Result<String, Box<dyn Error>> {
        let record = stored_judgment_record(fixture, task_id, response)?;
        record
            .resolved_verification_basis
            .ok_or_else(|| "stored judgment should have a resolution basis".into())
    }

    fn stored_judgment_record(
        fixture: &CoreFixture,
        task_id: &str,
        response: &Value,
    ) -> Result<volicord_store::core_pipeline::UserJudgmentRecord, Box<dyn Error>> {
        let judgment_id = response["user_judgment_ref"]["record_id"]
            .as_str()
            .ok_or("response should include user_judgment_ref.record_id")?;
        let store = CoreProjectStore::open(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;
        let record = store
            .user_judgment_records_for_task(&volicord_types::TaskId::new(task_id))?
            .into_iter()
            .find(|record| record.judgment_id == judgment_id)
            .ok_or("stored judgment record should exist")?;
        Ok(record)
    }

    fn tool_names(tools: &[McpToolDefinition]) -> Vec<&'static str> {
        tools.iter().map(|tool| tool.name).collect::<Vec<_>>()
    }

    fn root_properties(schema: &Value) -> Vec<String> {
        schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|properties| properties.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn root_required_fields(schema: &Value) -> Vec<String> {
        schema
            .get("required")
            .and_then(Value::as_array)
            .map(|required| {
                required
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn schema_has_definition(schema: &Value, name: &str) -> bool {
        schema
            .get("definitions")
            .and_then(Value::as_object)
            .is_some_and(|definitions| definitions.contains_key(name))
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
