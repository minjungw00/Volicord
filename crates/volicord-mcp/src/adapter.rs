use crate::constants::DEFAULT_LOCALE;
use crate::errors::McpAdapterError;
use crate::mutation_admission::with_mcp_runtime_home_mutation;
use crate::routing::{
    current_enabled_connection, inspect_allowed_project, parse_connection_mode, routing_error,
    selected_project_from_availability, storage_capability_for_projects, ListProjectItem,
    ListProjectsResult, McpConnectionContext, McpProjectAvailability, McpStorageCapability,
};
use crate::schema_validation::validate_mcp_tool_arguments;
use crate::tool_registry::{
    mcp_tools_for_mode_and_storage_with_detail, CanonicalToolDefinition, ToolSchemaDetail,
};
use crate::util::{
    generated_metadata_id, optional_string_field, reject_internal_mcp_argument_fields,
};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use volicord_core::pipeline::{
    CorePipelineError, CoreService, GitWorkspaceContext, InvocationContext, PipelineResponse,
};
use volicord_host_contract::{CodexMcpCorrelation, HostNativeCorrelation};
use volicord_mcp_wire::{
    status_include, McpCheckCloseArguments, McpCloseTaskArguments, McpGetOperationResultArguments,
    McpIntakeArguments, McpPrepareEvidenceCaptureArguments, McpPrepareWriteArguments,
    McpReconcileChangesArguments, McpRecordRunArguments, McpRequestUserActionArguments,
    McpRequestUserActionOperation, McpStageArtifactArguments, McpStatusArguments,
    McpToolErrorIssue, McpToolIssueCode, McpUpdateScopeArguments,
};
use volicord_platform_fs::capture_git_workspace_snapshot;
use volicord_platform_fs::{canonical_runtime_home_path, CanonicalRuntimeHomePath};
use volicord_store::agent_connections::{
    agent_connection_project_access_read_only, list_connection_projects_read_only,
    ConnectionProjectRecord,
};
use volicord_store::core_pipeline::CoreProjectStore;
use volicord_store::guards::{
    bind_agent_session_runtime, current_project_agent_session_coordinates,
    list_guard_installations, AgentSessionRuntimeBinding,
};
use volicord_store::integration_verification::{
    acknowledge_guard_integration_probe, begin_guard_integration_verification,
    get_guard_integration_verification, BeginGuardIntegrationVerificationInput,
    GuardIntegrationVerificationCaller,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{
    AgentRuntimeSessionId, AgentSessionId, IdempotencyKey, ProjectId, RequestId, TaskId,
};
use volicord_types::integration_verification::{
    BeginIntegrationVerificationArguments, IntegrationVerificationIdArguments,
};
use volicord_types::methods::{
    CheckCloseRequest, CloseTaskRequest, GetOperationResultRequest, IntakeRequest,
    MethodOperationCategory, MethodResponseContract, PrepareEvidenceCaptureRequest,
    PrepareWriteRequest, ReconcileChangesRequest, RecordRunRequest, RequestUserActionRequest,
    RequestUserActionResponse, StageArtifactRequest, StatusRequest, UpdateScopeRequest,
};
use volicord_types::schema::{RequiredNullable, ToolEnvelope};
use volicord_types::tool_names::{AgentToolId, AgentToolOwner};
use volicord_types::values::{
    IntegrationProfile, MethodName, OperationCategory, StatusDetailLevel, UtcTimestamp,
};

/// Invocation context derived for one tool call before entering Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpDerivedInvocationContext {
    pub operation_category: OperationCategory,
    pub validated_agent_session: volicord_core::ValidatedAgentSession,
    pub git_workspace_context: Option<GitWorkspaceContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentSessionCoordinates<'a> {
    pub(crate) runtime_session_id: &'a str,
    pub(crate) project_session_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedAgentSessionCoordinates {
    runtime_session_id: String,
    project_session_id: String,
}

impl OwnedAgentSessionCoordinates {
    pub(crate) fn borrowed(&self) -> AgentSessionCoordinates<'_> {
        AgentSessionCoordinates {
            runtime_session_id: &self.runtime_session_id,
            project_session_id: &self.project_session_id,
        }
    }
}

/// Runtime-owned host correlation passed from the stdio lifecycle to project binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedAgentSessionBinding {
    pub(crate) runtime_session_id: String,
    pub(crate) correlation: CodexMcpCorrelation,
}

impl McpDerivedInvocationContext {
    fn core_invocation(&self) -> InvocationContext {
        let mut invocation = InvocationContext::agent_connection(
            self.operation_category,
            self.validated_agent_session.clone(),
        );
        if let Some(workspace) = self.git_workspace_context.as_ref() {
            invocation = invocation.with_git_workspace_context(workspace.clone());
        }
        invocation
    }
}

/// Local MCP adapter bound to one pre-operation Runtime Home route and Agent Connection.
#[derive(Debug, Clone)]
pub struct McpAdapter {
    pub(crate) runtime_home: PathBuf,
    routing_runtime_home_identity: Result<CanonicalRuntimeHomePath, String>,
    pub(crate) context: McpConnectionContext,
    default_agent_session_binding: Option<ManagedAgentSessionBinding>,
}

impl PartialEq for McpAdapter {
    fn eq(&self, other: &Self) -> bool {
        self.runtime_home == other.runtime_home
            && self.routing_runtime_home_identity == other.routing_runtime_home_identity
            && self.context == other.context
            && self.default_agent_session_binding == other.default_agent_session_binding
    }
}

impl Eq for McpAdapter {}

impl McpAdapter {
    /// Creates an adapter for a Runtime Home and connection-bound adapter context.
    pub fn new(runtime_home: impl AsRef<Path>, context: McpConnectionContext) -> Self {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        let routing_runtime_home_identity =
            canonical_runtime_home_path(&runtime_home).map_err(|error| error.to_string());
        Self {
            runtime_home,
            routing_runtime_home_identity,
            context,
            default_agent_session_binding: None,
        }
    }

    pub(crate) fn admitted_runtime_home<'a>(
        &self,
        context: &'a RuntimeHomeMutationContext<'_>,
    ) -> Result<&'a Path, McpAdapterError> {
        let expected = self
            .routing_runtime_home_identity
            .as_ref()
            .map_err(|detail| {
                McpAdapterError::Environment(format!(
                    "runtime_home_routing_identity_unavailable: {detail}"
                ))
            })?;
        if expected != context.runtime_home() {
            return Err(McpAdapterError::Environment(
                "runtime_home_mutation_context_mismatch: the live mutation context does not match the MCP routing Runtime Home"
                    .to_owned(),
            ));
        }
        Ok(context.runtime_home().as_path())
    }

    #[cfg(test)]
    pub(crate) fn with_managed_agent_session_binding(
        mut self,
        binding: ManagedAgentSessionBinding,
    ) -> Self {
        self.default_agent_session_binding = Some(binding);
        self
    }

    pub(crate) fn allowed_project_availabilities(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
    ) -> Result<Vec<McpProjectAvailability>, McpAdapterError> {
        let runtime_home = self.admitted_runtime_home(context)?;
        current_enabled_connection(
            runtime_home,
            self.context.connection_internal_id.as_str(),
            tool_name,
        )?;
        let projects = list_connection_projects_read_only(
            runtime_home,
            self.context.connection_internal_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?;
        Ok(projects
            .iter()
            .filter(|project| {
                self.context
                    .project_allowlist_allows(project.project_id.as_str())
            })
            .map(|project| inspect_allowed_project(context, project))
            .collect())
    }

    /// Returns the tools exposed by this adapter's current connection mode.
    pub fn tools(&self) -> Result<Vec<CanonicalToolDefinition>, McpAdapterError> {
        with_mcp_runtime_home_mutation(&self.runtime_home, "mcp.tools_list", |context| {
            self.tools_for_context(context)
        })
    }

    pub(crate) fn tools_for_context(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> Result<Vec<CanonicalToolDefinition>, McpAdapterError> {
        let runtime_home = self.admitted_runtime_home(context)?;
        let connection = current_enabled_connection(
            runtime_home,
            self.context.connection_internal_id.as_str(),
            "tools/list",
        )?;
        let mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: "tools/list".to_owned(),
                message: error.to_string(),
            }
        })?;
        let storage_capability = self.session_storage_capability(context)?;
        Ok(mcp_tools_for_mode_and_storage_with_detail(
            mode,
            storage_capability,
            ToolSchemaDetail::RuntimeCompact,
        ))
    }

    pub(crate) fn session_storage_capability(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> Result<McpStorageCapability, McpAdapterError> {
        let projects = self.allowed_project_availabilities(context, "storage capability")?;
        Ok(storage_capability_for_projects(&projects))
    }

    /// Derives local invocation facts for one decoded request envelope.
    pub(crate) fn derive_invocation_context(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<McpDerivedInvocationContext, McpAdapterError> {
        self.admitted_runtime_home(context)?;
        let store = CoreProjectStore::open_for_mutation(context, &envelope.project_id)
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
        let validated_agent_session = self.validated_session_for_project(
            context,
            &envelope.project_id,
            operation_category,
            session,
        )?;
        Ok(McpDerivedInvocationContext {
            operation_category,
            validated_agent_session,
            git_workspace_context,
        })
    }

    fn derive_read_only_invocation_context(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<McpDerivedInvocationContext, McpAdapterError> {
        let store = CoreProjectStore::open_read_only(
            self.admitted_runtime_home(context)?,
            &envelope.project_id,
        )
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
        let validated_agent_session = self.validated_session_for_project(
            context,
            &envelope.project_id,
            operation_category,
            session,
        )?;
        Ok(McpDerivedInvocationContext {
            operation_category,
            validated_agent_session,
            git_workspace_context,
        })
    }

    fn validated_session_for_project(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        project_id: &ProjectId,
        operation_category: OperationCategory,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<volicord_core::ValidatedAgentSession, McpAdapterError> {
        self.admitted_runtime_home(context)?;
        let session = session.ok_or_else(|| {
            McpAdapterError::Environment(
                "agent_session_missing: project tools require a current managed runtime and project session"
                    .to_owned(),
            )
        })?;
        CoreService::for_mutation(context)
            .validate_agent_session(
                self.context.connection_internal_id.clone(),
                project_id.clone(),
                AgentRuntimeSessionId::new(session.runtime_session_id),
                AgentSessionId::new(session.project_session_id),
                operation_category,
            )
            .map_err(|error| {
                McpAdapterError::Environment(format!(
                    "{}: current managed Agent Session did not authorize this tool call",
                    error.reason()
                ))
            })
    }

    /// Calls one public Volicord method tool and returns Core's response.
    pub fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let tool = AgentToolId::from_wire_name(tool_name)
            .map_err(|_| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
        with_mcp_runtime_home_mutation(&self.runtime_home, "mcp.tool_call", |context| {
            let coordinates =
                self.default_agent_session_coordinates_for_tool(context, tool, &params)?;
            self.call_tool_for_session(
                context,
                tool,
                params,
                coordinates.as_ref().map(|value| value.borrowed()),
            )
        })
    }

    pub(crate) fn call_tool_for_session(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool: AgentToolId,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let tool_name = tool.wire_name();
        validate_mcp_tool_arguments(tool_name, &params)?;
        match tool.method() {
            Some(MethodName::Intake) => self.call_intake(context, tool_name, params, session),
            Some(MethodName::UpdateScope) => {
                self.call_update_scope(context, tool_name, params, session)
            }
            Some(MethodName::Status) => self.call_status(context, tool_name, params, session),
            Some(MethodName::GetOperationResult) => {
                self.call_get_operation_result(context, tool_name, params, session)
            }
            Some(MethodName::PrepareEvidenceCapture) => {
                self.call_prepare_evidence_capture(context, tool_name, params, session)
            }
            Some(MethodName::PrepareWrite) => {
                self.call_prepare_write(context, tool_name, params, session)
            }
            Some(MethodName::StageArtifact) => {
                self.call_stage_artifact(context, tool_name, params, session)
            }
            Some(MethodName::RecordRun) => {
                self.call_record_run(context, tool_name, params, session)
            }
            Some(MethodName::RequestUserAction) => {
                self.call_request_user_action(context, tool_name, params, session)
            }
            Some(MethodName::ReconcileChanges) => {
                self.call_reconcile_changes(context, tool_name, params, session)
            }
            Some(MethodName::CheckClose) => {
                self.call_check_close(context, tool_name, params, session)
            }
            Some(MethodName::CloseTask) => {
                self.call_close_task(context, tool_name, params, session)
            }
            None | Some(MethodName::ResolveUserAction) => {
                Err(McpAdapterError::UnknownTool(tool_name.to_owned()))
            }
        }
    }

    fn call_intake(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpIntakeArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            None,
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            IntakeRequest {
                envelope,
                plain_language_request: args.plain_language_request,
                requested_mode: args.requested_mode,
                requested_control_level: args.requested_control_level,
                resume_policy: args.resume_policy,
                acceptance_policy: args.acceptance_policy,
                lineage: args.lineage,
                initial_scope: args.initial_scope,
                initial_context_refs: args.initial_context_refs,
                initial_source_refs: args.initial_source_refs,
            },
            CoreService::intake,
            session,
        )
    }

    fn call_update_scope(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpUpdateScopeArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
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
            session,
        )
    }

    fn call_status(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStatusArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            task_id.as_ref(),
            OperationCategory::Read,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            StatusRequest {
                envelope,
                continuity_page: args.continuity_page,
                include: status_include(args.detail),
            },
            |core, _, request, invocation| core.status(request, invocation),
            session,
        )
    }

    fn call_get_operation_result(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpGetOperationResultArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            None,
            OperationCategory::Read,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            GetOperationResultRequest {
                envelope,
                operation_result_ref: args.operation_result_ref,
                cursor: args.cursor,
            },
            |core, _, request, invocation| core.get_operation_result(request, invocation),
            session,
        )
    }

    pub(crate) fn refresh_authority_status(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        project_id: &ProjectId,
        task_id: &TaskId,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let owned_session = if session.is_none() {
            self.default_agent_session_binding
                .as_ref()
                .map(|binding| self.ensure_agent_session_binding(context, project_id, binding))
                .transpose()?
        } else {
            None
        };
        let session = session.or_else(|| owned_session.as_ref().map(|value| value.borrowed()));
        let status_tool_name = AgentToolId::STATUS.wire_name();
        let envelope = self.generated_envelope(
            context,
            status_tool_name,
            project_id,
            Some(task_id),
            OperationCategory::Read,
        )?;
        self.call_core_request(
            context,
            status_tool_name,
            StatusRequest {
                envelope,
                continuity_page: None,
                include: status_include(StatusDetailLevel::Workflow),
            },
            |core, _, request, invocation| core.status(request, invocation),
            session,
        )
    }

    pub(crate) fn default_agent_session_coordinates_for_tool(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool: AgentToolId,
        params: &Value,
    ) -> Result<Option<OwnedAgentSessionCoordinates>, McpAdapterError> {
        self.default_agent_session_binding
            .as_ref()
            .map(|binding| {
                self.ensure_agent_session_binding_for_tool(context, tool, params, binding)
            })
            .transpose()
            .map(Option::flatten)
    }

    fn call_prepare_evidence_capture(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareEvidenceCaptureArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            PrepareEvidenceCaptureRequest {
                envelope,
                task_id,
                change_unit_id: args.change_unit_id,
                baseline_ref: args.baseline_ref,
                target: args.target,
                capture: args.capture.into_core(),
            },
            CoreService::prepare_evidence_capture,
            session,
        )
    }

    fn call_prepare_write(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareWriteArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            task_id.as_ref(),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
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
            session,
        )
    }

    fn call_stage_artifact(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStageArtifactArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
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
            session,
        )
    }

    fn call_record_run(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRecordRunArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            RecordRunRequest {
                envelope,
                task_id,
                change_unit_id: args.change_unit_id,
                kind: args.kind,
                run_id: args.run_id,
                baseline_ref: args.baseline_ref,
                write_ticket_id: args.write_ticket_id,
                performed_operation: args.performed_operation,
                summary: args.summary,
                observed_changes: args.observed_changes,
                artifact_inputs: args.artifact_inputs,
                evidence_updates: args
                    .evidence_updates
                    .into_iter()
                    .map(|update| update.into_core())
                    .collect(),
                evidence_observations: args
                    .evidence_observations
                    .into_iter()
                    .map(|observation| observation.into_core())
                    .collect(),
                close_assessment: args.close_assessment,
            },
            CoreService::record_run,
            session,
        )
    }

    fn call_request_user_action(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRequestUserActionArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        match prepared.arguments.request {
            McpRequestUserActionOperation::Create {
                task_id,
                change_unit_id,
                action,
                required_for,
                expires_at,
            } => {
                let envelope = self.generated_envelope(
                    context,
                    tool_name,
                    &prepared.project_id,
                    Some(&task_id),
                    OperationCategory::AgentWorkflow,
                )?;
                self.call_core_request(
                    context,
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
                    session,
                )
            }
            McpRequestUserActionOperation::Resume {
                user_action_request_id,
            } => {
                self.ensure_mode_allows(context, tool_name, OperationCategory::AgentWorkflow)?;
                let owned_session = if session.is_none() {
                    self.default_agent_session_binding
                        .as_ref()
                        .map(|binding| {
                            self.ensure_agent_session_binding(
                                context,
                                &prepared.project_id,
                                binding,
                            )
                        })
                        .transpose()?
                } else {
                    None
                };
                let session =
                    session.or_else(|| owned_session.as_ref().map(|value| value.borrowed()));
                let envelope = ToolEnvelope {
                    project_id: prepared.project_id.clone(),
                    task_id: RequiredNullable::null(),
                    request_id: RequestId::new("req_internal_user_action_resume"),
                    idempotency_key: RequiredNullable::null(),
                    expected_state_version: RequiredNullable::null(),
                    dry_run: volicord_types::schema::DryRunIntent::NotRequested,
                    locale: RequiredNullable::null(),
                };
                let invocation = self.derive_read_only_invocation_context(
                    context,
                    &envelope,
                    OperationCategory::AgentWorkflow,
                    session,
                )?;
                let response = CoreService::for_mutation(context)
                    .resume_user_action_request(
                        prepared.project_id,
                        user_action_request_id,
                        invocation.core_invocation(),
                    )
                    .map_err(McpAdapterError::Core)?
                    .ok_or_else(|| McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: "the resumed user-action request is unavailable or was created by another Agent Connection".to_owned(),
                    })?;
                serde_json::from_value::<RequestUserActionResponse>(
                    response.response_value.clone(),
                )
                .map_err(McpAdapterError::Json)?;
                Ok(response)
            }
        }
    }

    fn call_reconcile_changes(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpReconcileChangesArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            ReconcileChangesRequest {
                envelope,
                task_id,
                resolution_requests: args.resolution_requests,
            },
            CoreService::reconcile_changes,
            session,
        )
    }

    fn call_check_close(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCheckCloseArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::Read,
        )?;
        self.call_core_request(
            context,
            tool_name,
            CheckCloseRequest { envelope, task_id },
            |core, _, request, invocation| core.check_close(request, invocation),
            session,
        )
    }

    fn call_close_task(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCloseTaskArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
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
            session,
        )
    }

    fn ensure_storage_writable_for_tool(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
    ) -> Result<(), McpAdapterError> {
        let Some(operation_category) = public_tool_operation_category(tool_name) else {
            return Ok(());
        };
        if operation_category == OperationCategory::Read {
            return Ok(());
        }
        let storage_capability = self.session_storage_capability(context)?;
        if storage_capability.allows_mutation() {
            return Ok(());
        }
        Err(McpAdapterError::OperationalUnavailable {
            retryable: false,
            reached_core: false,
        })
    }

    fn call_core_request<T, F>(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        request: T,
        call: F,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError>
    where
        T: MethodOperationCategory + MethodResponseContract + HasEnvelope,
        F: FnOnce(
            &CoreService,
            &RuntimeHomeMutationContext<'_>,
            T,
            InvocationContext,
        ) -> Result<PipelineResponse, CorePipelineError>,
    {
        self.ensure_storage_writable_for_tool(context, tool_name)?;
        let operation_category = request.operation_category();
        self.ensure_mode_allows(context, tool_name, operation_category)?;
        let owned_session = if session.is_none() {
            self.default_agent_session_binding
                .as_ref()
                .map(|binding| {
                    self.ensure_agent_session_binding(
                        context,
                        &request_envelope(&request).project_id,
                        binding,
                    )
                })
                .transpose()?
        } else {
            None
        };
        let session = session.or_else(|| owned_session.as_ref().map(|value| value.borrowed()));
        let invocation = if operation_category == OperationCategory::Read {
            self.derive_read_only_invocation_context(
                context,
                request_envelope(&request),
                operation_category,
                session,
            )?
        } else {
            self.derive_invocation_context(
                context,
                request_envelope(&request),
                operation_category,
                session,
            )?
        };
        let core = CoreService::for_mutation(context);
        let response = call(&core, context, request, invocation.core_invocation())
            .map_err(McpAdapterError::Core)?;
        serde_json::from_value::<T::Response>(response.response_value.clone())
            .map_err(McpAdapterError::Json)?;
        Ok(response)
    }

    pub(crate) fn call_adapter_tool(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool: AgentToolId,
        params: Value,
        binding: Option<&ManagedAgentSessionBinding>,
        session: Option<&OwnedAgentSessionCoordinates>,
    ) -> Result<Value, McpAdapterError> {
        let tool_name = tool.wire_name();
        validate_mcp_tool_arguments(tool_name, &params)?;
        match tool {
            AgentToolId::LIST_PROJECTS => {
                let object = params
                    .as_object()
                    .ok_or_else(|| McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: format!("{tool_name} arguments must be an object"),
                    })?;
                if !object.is_empty() {
                    return Err(McpAdapterError::ToolExecution {
                        tool_name: tool_name.to_owned(),
                        message: format!("{tool_name} does not accept arguments"),
                    });
                }
                let result = self.list_projects_result(context)?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION => {
                let binding = required_integration_binding(tool_name, binding)?;
                let session = session.ok_or_else(|| McpAdapterError::ToolExecution {
                    tool_name: tool_name.to_owned(),
                    message: "integration verification requires a writable current project session"
                        .to_owned(),
                })?;
                let arguments: BeginIntegrationVerificationArguments =
                    self.decode_params(tool_name, params)?;
                let project_id =
                    self.select_project(context, arguments.project_selector.as_deref())?;
                let observed_at = integration_verification_timestamp();
                let result = begin_guard_integration_verification(
                    context,
                    BeginGuardIntegrationVerificationInput {
                        caller: integration_verification_caller(
                            self.context.connection_internal_id.as_str(),
                            binding,
                        ),
                        project_id: project_id.as_str().to_owned(),
                        project_session_id: session.project_session_id.clone(),
                        observed_at: observed_at.clone(),
                    },
                )
                .map_err(McpAdapterError::Store)?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            AgentToolId::GUARD_PROBE => {
                let binding = required_integration_binding(tool_name, binding)?;
                let arguments: IntegrationVerificationIdArguments =
                    self.decode_params(tool_name, params)?;
                let result = acknowledge_guard_integration_probe(
                    context,
                    arguments.verification_id.as_str(),
                    &integration_verification_caller(
                        self.context.connection_internal_id.as_str(),
                        binding,
                    ),
                    &integration_verification_timestamp(),
                )
                .map_err(McpAdapterError::Store)?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            AgentToolId::GET_INTEGRATION_VERIFICATION => {
                let binding = required_integration_binding(tool_name, binding)?;
                let arguments: IntegrationVerificationIdArguments =
                    self.decode_params(tool_name, params)?;
                let result = get_guard_integration_verification(
                    context,
                    arguments.verification_id.as_str(),
                    &integration_verification_caller(
                        self.context.connection_internal_id.as_str(),
                        binding,
                    ),
                    &integration_verification_timestamp(),
                )
                .map_err(McpAdapterError::Store)?;
                serde_json::to_value(result).map_err(McpAdapterError::Json)
            }
            other => Err(McpAdapterError::UnknownTool(other.wire_name().to_owned())),
        }
    }

    fn list_projects_result(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> Result<ListProjectsResult, McpAdapterError> {
        let tool_name = AgentToolId::LIST_PROJECTS.wire_name();
        let runtime_home = self.admitted_runtime_home(context)?;
        let connection = current_enabled_connection(
            runtime_home,
            self.context.connection_internal_id.as_str(),
            tool_name,
        )?;
        let availabilities = self.allowed_project_availabilities(context, tool_name)?;
        let items = availabilities
            .iter()
            .map(|project| ListProjectItem {
                project_selector: project.project_id.clone(),
                available: project.available,
                unavailable_reason: project.unavailable_reason.clone(),
                repo_root: project.repo_root_display.clone(),
            })
            .collect::<Vec<_>>();
        let mode = parse_connection_mode(&connection.mode).map_err(|error| {
            McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
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
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        _session: Option<AgentSessionCoordinates<'_>>,
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
        let selected_project_id =
            self.select_project(context, requested_project_selector.as_deref())?;
        Ok(PreparedMcpArguments {
            arguments,
            project_id: selected_project_id,
        })
    }

    pub(crate) fn ensure_agent_session_binding_for_tool(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool: AgentToolId,
        params: &Value,
        binding: &ManagedAgentSessionBinding,
    ) -> Result<Option<OwnedAgentSessionCoordinates>, McpAdapterError> {
        let tool_name = tool.wire_name();
        let object = params
            .as_object()
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "tool arguments must be an object".to_owned(),
            })?;
        if matches!(
            tool,
            AgentToolId::GUARD_PROBE | AgentToolId::GET_INTEGRATION_VERIFICATION
        ) {
            return Ok(None);
        }
        let project_id = if tool == AgentToolId::LIST_PROJECTS {
            let projects = self.allowed_project_availabilities(context, tool_name)?;
            let [project] = projects.as_slice() else {
                return Ok(None);
            };
            if project.storage_capability != McpStorageCapability::ReadWrite {
                return Ok(None);
            }
            ProjectId::new(&project.project_id)
        } else {
            let requested_project_selector =
                optional_string_field(object, "project_selector", tool_name)?;
            self.select_project(context, requested_project_selector.as_deref())?
        };
        self.ensure_agent_session_binding(context, &project_id, binding)
            .map(Some)
    }

    pub(crate) fn ensure_agent_session_binding(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        project_id: &ProjectId,
        binding: &ManagedAgentSessionBinding,
    ) -> Result<OwnedAgentSessionCoordinates, McpAdapterError> {
        let runtime_home = self.admitted_runtime_home(context)?;
        let _connection = current_enabled_connection(
            runtime_home,
            self.context.connection_internal_id.as_str(),
            "managed stdio session binding",
        )?;
        let guard_installations = list_guard_installations(
            runtime_home,
            self.context.connection_internal_id.as_str(),
            Some(project_id.as_str()),
        )
        .map_err(McpAdapterError::Store)?;
        let guard_installation = match guard_installations.as_slice() {
            [] => None,
            [installation] => Some(installation),
            _ => {
                return Err(McpAdapterError::Environment(
                    "managed_stdio_session_guard_ownership_ambiguous: project has multiple current Guard installations"
                        .to_owned(),
                ))
            }
        };
        if let Some(installation) = guard_installation {
            let manifest = volicord_types::guard_manifest::guard_manifest_from_json(&installation.manifest_json)
                .map_err(|_| {
                    McpAdapterError::Environment(
                        "managed_stdio_session_manifest_invalid: current Guard installation manifest is malformed"
                            .to_owned(),
                    )
                })?;
            if manifest.integration_profile != IntegrationProfile::Record {
                return Err(McpAdapterError::Environment(
                    "managed_stdio_session_profile_mismatch: current Guard installation is not the Record profile"
                        .to_owned(),
                ));
            }
        }
        let guard_installation_id =
            guard_installation.map(|installation| installation.guard_installation_id.clone());
        let session_id = if self.storage_capability_for_project(context, project_id)?
            == McpStorageCapability::ReadWrite
        {
            let observed_at = CoreProjectStore::open_for_mutation(context, project_id)
                .and_then(|store| store.current_timestamp())
                .map_err(McpAdapterError::Store)?;
            bind_agent_session_runtime(
                context,
                project_id.as_str(),
                AgentSessionRuntimeBinding {
                    runtime_session_id: binding.runtime_session_id.clone(),
                    connection_internal_id: self.context.connection_internal_id.as_str().to_owned(),
                    guard_installation_id,
                    correlation: binding.correlation.clone(),
                    observed_at,
                },
            )
            .map_err(McpAdapterError::Store)?
            .session_id
        } else {
            current_project_agent_session_coordinates(
                runtime_home,
                project_id.as_str(),
                self.context.connection_internal_id.as_str(),
                guard_installation_id.as_deref(),
                &HostNativeCorrelation::CodexMcp(binding.correlation.clone()),
            )
            .map_err(McpAdapterError::Store)?
            .session_id
        };
        Ok(OwnedAgentSessionCoordinates {
            runtime_session_id: binding.runtime_session_id.clone(),
            project_session_id: session_id,
        })
    }

    fn storage_capability_for_project(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        project_id: &ProjectId,
    ) -> Result<McpStorageCapability, McpAdapterError> {
        let runtime_home = self.admitted_runtime_home(context)?;
        let access = agent_connection_project_access_read_only(
            runtime_home,
            self.context.connection_internal_id.as_str(),
            project_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| routing_error("connection is not registered"))?;
        let Some(project) = access.project else {
            return Ok(McpStorageCapability::Unavailable);
        };
        let availability = inspect_allowed_project(
            context,
            &ConnectionProjectRecord {
                connection_internal_id: self.context.connection_internal_id.as_str().to_owned(),
                project_internal_id: project.project_internal_id.clone(),
                project_id: project.project_id.clone(),
                created_at: String::new(),
                project,
            },
        );
        Ok(availability.storage_capability)
    }

    fn generated_envelope(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        project_id: &ProjectId,
        task_id: Option<&volicord_types::ids::TaskId>,
        operation_category: OperationCategory,
    ) -> Result<ToolEnvelope, McpAdapterError> {
        let state_version = if operation_category == OperationCategory::Read {
            None
        } else {
            Some(self.current_state_version(context, project_id)?)
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
            dry_run: volicord_types::schema::DryRunIntent::NotRequested,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        })
    }

    fn current_state_version(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        project_id: &ProjectId,
    ) -> Result<u64, McpAdapterError> {
        let store = CoreProjectStore::open_for_mutation(context, project_id)
            .map_err(McpAdapterError::Store)?;
        store
            .project_state()
            .map(|state| state.state_version)
            .map_err(McpAdapterError::Store)
    }

    fn select_project(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        requested_project_id: Option<&str>,
    ) -> Result<ProjectId, McpAdapterError> {
        let runtime_home = self.admitted_runtime_home(context)?;
        let connection_internal_id = self.context.connection_internal_id.as_str();
        let _connection =
            current_enabled_connection(runtime_home, connection_internal_id, "project routing")?;

        if let Some(project_id) = requested_project_id {
            if !self.context.project_allowlist_allows(project_id) {
                return Err(routing_error(format!(
                    "project selector {project_id} is outside this MCP transport project allowlist"
                )));
            }
            let access = agent_connection_project_access_read_only(
                runtime_home,
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
            let availability = inspect_allowed_project(context, &project_record);
            return selected_project_from_availability(availability);
        }

        let projects = list_connection_projects_read_only(runtime_home, connection_internal_id)
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

        selected_project_from_availability(inspect_allowed_project(context, &projects[0]))
    }

    fn ensure_mode_allows(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        operation_category: OperationCategory,
    ) -> Result<(), McpAdapterError> {
        let connection = current_enabled_connection(
            self.admitted_runtime_home(context)?,
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

fn required_integration_binding<'a>(
    tool_name: &str,
    binding: Option<&'a ManagedAgentSessionBinding>,
) -> Result<&'a ManagedAgentSessionBinding, McpAdapterError> {
    binding.ok_or_else(|| McpAdapterError::ToolExecution {
        tool_name: tool_name.to_owned(),
        message: "integration verification is available only in a current managed Codex session"
            .to_owned(),
    })
}

fn integration_verification_caller(
    connection_internal_id: &str,
    binding: &ManagedAgentSessionBinding,
) -> GuardIntegrationVerificationCaller {
    GuardIntegrationVerificationCaller {
        connection_internal_id: connection_internal_id.to_owned(),
        runtime_session_id: binding.runtime_session_id.clone(),
        host_session_id: binding.correlation.session_id.as_str().to_owned(),
        host_turn_id: binding.correlation.turn_id.as_str().to_owned(),
    }
}

fn integration_verification_timestamp() -> String {
    UtcTimestamp::from_datetime(DateTime::<Utc>::from(SystemTime::now())).to_canonical_string()
}

fn invalid_argument_guidance(
    tool_name: &str,
    params: &Value,
    source: &serde_json::Error,
) -> Option<String> {
    let source_text = source.to_string();
    match AgentToolId::from_wire_name(tool_name).ok()? {
        AgentToolId::RECORD_RUN => record_run_invalid_argument_guidance(params, &source_text),
        AgentToolId::REQUEST_USER_ACTION => {
            request_user_action_invalid_argument_guidance(params, &source_text)
        }
        AgentToolId::UPDATE_SCOPE => update_scope_invalid_argument_guidance(params, &source_text),
        AgentToolId::PREPARE_WRITE => prepare_write_invalid_argument_guidance(params, &source_text),
        AgentToolId::STATUS => status_invalid_argument_guidance(params, &source_text),
        AgentToolId::CHECK_CLOSE => check_close_invalid_argument_guidance(params, &source_text),
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
        "performed_operation",
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
    let tool = AgentToolId::from_wire_name(tool_name).ok()?;
    matches!(tool.owner(), AgentToolOwner::CoreMethod(_))
        .then(|| tool.category().operation_category())
}

struct PreparedMcpArguments<T> {
    arguments: T,
    project_id: ProjectId,
}
