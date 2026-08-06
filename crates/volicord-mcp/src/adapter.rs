use crate::action_form::{bind_fixed_arguments, retry_contract, workflow_action_form_catalog};
use crate::authority_refresh::{validated_authority_refresh, MutationRefreshContext};
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
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use volicord_core::pipeline::{
    CorePipelineError, CoreService, GitWorkspaceContext, InvocationContext, PipelineResponse,
};
use volicord_core::TransitionSubmission;
use volicord_host_contract::{CodexMcpCorrelation, HostNativeCorrelation};
use volicord_mcp_wire::{
    status_include, AuthoritativeArgumentContext, McpAdvanceTaskArguments,
    McpArgumentFailurePresentation, McpCheckCloseArguments, McpCloseTaskArguments,
    McpFinalizeAdviceArguments, McpGetOperationResultArguments, McpIntakeArguments,
    McpPrepareEvidenceCaptureArguments, McpPrepareWriteArguments, McpReconcileChangesArguments,
    McpRecordRunArguments, McpRecordShapingCheckpointArguments, McpRequestUserActionArguments,
    McpRequestUserActionOperation, McpStageArtifactArguments, McpStatusArguments, McpToolErrorCode,
    McpToolErrorIssue, McpToolIssueCode, McpUpdateScopeArguments, McpWorkflowAdmissionRejection,
    McpWorkflowContractDiagnostics, McpWorkflowContractStage, WorkflowActionForm,
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
    AgentRuntimeSessionId, AgentSessionId, BaselineRef, IdempotencyKey, ProjectId, RequestHash,
    RequestId, TaskId,
};
use volicord_types::integration_verification::{
    BeginIntegrationVerificationArguments, IntegrationVerificationIdArguments,
};
use volicord_types::methods::{
    public_method_contract, AdvanceTaskRequest, CheckCloseRequest, CloseTaskRequest,
    FinalizeAdviceRequest, GetOperationResultRequest, IntakeRequest, MethodOperationCategory,
    MethodResponseContract, PrepareEvidenceCaptureRequest, PrepareWriteRequest,
    ReconcileChangesRequest, RecordRunRequest, RecordShapingCheckpointRequest,
    RequestUserActionRequest, RequestUserActionResponse, StageArtifactRequest, StatusRequest,
    UpdateScopeRequest, WorkflowActionAdmissionClass,
};
use volicord_types::schema::{
    RequiredNullable, ToolEnvelope, TransitionAttemptDetails, WorkflowActionKey, WorkflowProjection,
};
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
        self.core_invocation_for_category(self.operation_category)
    }

    fn core_invocation_for_category(
        &self,
        operation_category: OperationCategory,
    ) -> InvocationContext {
        let mut invocation = InvocationContext::agent_connection(
            operation_category,
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
    planning_action_key: Option<WorkflowActionKey>,
}

impl PartialEq for McpAdapter {
    fn eq(&self, other: &Self) -> bool {
        self.runtime_home == other.runtime_home
            && self.routing_runtime_home_identity == other.routing_runtime_home_identity
            && self.context == other.context
            && self.default_agent_session_binding == other.default_agent_session_binding
            && self.planning_action_key == other.planning_action_key
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
            planning_action_key: None,
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
        if tool.method().is_some() {
            self.ensure_mode_allows(context, tool_name, tool.category().operation_category())?;
        }
        if tool.method().is_some()
            && session.is_none()
            && self.default_agent_session_binding.is_none()
        {
            if let Err(validation_error) = validate_mcp_tool_arguments(tool_name, &params) {
                return Err(self.enrich_invalid_arguments(
                    context,
                    tool,
                    &params,
                    session,
                    validation_error,
                ));
            }
            let object = params
                .as_object()
                .ok_or_else(|| McpAdapterError::ToolExecution {
                    tool_name: tool_name.to_owned(),
                    message: "tool arguments must be an object".to_owned(),
                })?;
            let requested_project_selector =
                optional_string_field(object, "project_selector", tool_name)?;
            self.select_project(context, requested_project_selector.as_deref())?;
            return Err(McpAdapterError::Environment(
                "agent_session_missing: project tools require a current managed runtime and project session"
                    .to_owned(),
            ));
        }
        let current_action_form = if let Some(method) = tool.method() {
            match self.admit_workflow_action(context, method, &params, session) {
                Ok(form) => form,
                Err(admission_error) => {
                    if matches!(admission_error, McpAdapterError::ToolExecution { .. }) {
                        if let Err(validation_error) =
                            validate_mcp_tool_arguments(tool_name, &params)
                        {
                            return Err(self.enrich_invalid_arguments(
                                context,
                                tool,
                                &params,
                                session,
                                validation_error,
                            ));
                        }
                    }
                    return Err(admission_error);
                }
            }
        } else {
            None
        };
        if let Some(form) = current_action_form.as_ref() {
            let binding = bind_fixed_arguments(form, &params).map_err(|_| {
                McpAdapterError::SchemaContractFailure {
                    tool_name: tool_name.to_owned(),
                }
            })?;
            if !binding.mismatches.is_empty() {
                let authoritative_context = self
                    .load_authoritative_argument_context(context, &params, session)?
                    .unwrap_or_else(unloaded_authoritative_argument_context);
                let catalog = authoritative_context
                    .action_form_catalog
                    .as_ref()
                    .ok_or_else(|| McpAdapterError::SchemaContractFailure {
                        tool_name: tool_name.to_owned(),
                    })?;
                let issues = binding
                    .mismatches
                    .iter()
                    .map(|mismatch| {
                        McpToolErrorIssue::new(
                            mismatch.path.clone(),
                            McpToolIssueCode::ActionFormArgumentMismatch,
                            "the action form was current, but a fixed authority value was altered or omitted; Core was not reached; retry with the exact fixed arguments",
                        )
                    })
                    .collect();
                return Err(McpAdapterError::InvalidParams {
                    code: McpToolErrorCode::ActionFormArgumentMismatch,
                    tool_name: tool_name.to_owned(),
                    issues,
                    truncated: binding.truncated,
                    selected_variant: Some(form.action_key.semantic_variant.as_str().to_owned()),
                    canonical_example: None,
                    authoritative_context: Some(Box::new(authoritative_context.clone())),
                    retry_contract: Some(Box::new(
                        retry_contract(
                            form.action_key,
                            form.action_key,
                            authoritative_context.workflow.as_ref().ok_or_else(|| {
                                McpAdapterError::SchemaContractFailure {
                                    tool_name: tool_name.to_owned(),
                                }
                            })?,
                            catalog,
                            TransitionAttemptDetails::None,
                            binding
                                .mismatches
                                .iter()
                                .map(|mismatch| mismatch.path.clone())
                                .collect(),
                        )
                        .map_err(|_| {
                            McpAdapterError::SchemaContractFailure {
                                tool_name: tool_name.to_owned(),
                            }
                        })?,
                    )),
                    failure: Some(Box::new(argument_failure_presentation(
                        &authoritative_context,
                        false,
                    ))),
                    workflow_admission: None,
                    action_form_argument_mismatches: Box::new(binding.mismatches),
                });
            }
        }
        if let Err(error) = validate_mcp_tool_arguments(tool_name, &params) {
            return Err(self.enrich_invalid_arguments(context, tool, &params, session, error));
        }
        let bound_state_version = current_action_form
            .as_ref()
            .map(|form| form.expected_state_version);
        match tool.method() {
            Some(MethodName::Intake) => self.call_intake(context, tool_name, params, session),
            Some(MethodName::UpdateScope) => {
                self.call_update_scope(context, tool_name, params, session, bound_state_version)
            }
            Some(MethodName::RecordShapingCheckpoint) => self.call_record_shaping_checkpoint(
                context,
                tool_name,
                params,
                session,
                bound_state_version,
            ),
            Some(MethodName::FinalizeAdvice) => {
                self.call_finalize_advice(context, tool_name, params, session, bound_state_version)
            }
            Some(MethodName::AdvanceTask) => {
                self.call_advance_task(context, tool_name, params, session, bound_state_version)
            }
            Some(MethodName::Status) => self.call_status(context, tool_name, params, session),
            Some(MethodName::GetOperationResult) => {
                self.call_get_operation_result(context, tool_name, params, session)
            }
            Some(MethodName::PrepareEvidenceCapture) => self.call_prepare_evidence_capture(
                context,
                tool_name,
                params,
                session,
                bound_state_version,
            ),
            Some(MethodName::PrepareWrite) => {
                self.call_prepare_write(context, tool_name, params, session, bound_state_version)
            }
            Some(MethodName::StageArtifact) => {
                self.call_stage_artifact(context, tool_name, params, session, bound_state_version)
            }
            Some(MethodName::RecordRun) => {
                self.call_record_run(context, tool_name, params, session, bound_state_version)
            }
            Some(MethodName::RequestUserAction) => self.call_request_user_action(
                context,
                tool_name,
                params,
                session,
                bound_state_version,
            ),
            Some(MethodName::ReconcileChanges) => self.call_reconcile_changes(
                context,
                tool_name,
                params,
                session,
                bound_state_version,
            ),
            Some(MethodName::CheckClose) => {
                self.call_check_close(context, tool_name, params, session)
            }
            Some(MethodName::CloseTask) => {
                self.call_close_task(context, tool_name, params, session, bound_state_version)
            }
            None | Some(MethodName::ResolveUserAction) => {
                Err(McpAdapterError::UnknownTool(tool_name.to_owned()))
            }
        }
    }

    fn enrich_invalid_arguments(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool: AgentToolId,
        params: &Value,
        session: Option<AgentSessionCoordinates<'_>>,
        mut error: McpAdapterError,
    ) -> McpAdapterError {
        let authoritative_context =
            match self.load_authoritative_argument_context(context, params, session) {
                Ok(context) => context.unwrap_or_else(unloaded_authoritative_argument_context),
                Err(error) => return error,
            };
        if let McpAdapterError::InvalidParams {
            issues,
            authoritative_context: error_context,
            retry_contract: error_retry_contract,
            failure,
            workflow_admission,
            ..
        } = &mut error
        {
            let invalid_paths = issues
                .iter()
                .map(|issue| issue.path.clone())
                .collect::<Vec<_>>();
            let retry = authoritative_context
                .action_form_catalog
                .as_ref()
                .zip(authoritative_context.workflow.as_ref())
                .and_then(|(catalog, workflow)| {
                    tool.method().and_then(|method| {
                        volicord_mcp_wire::submitted_action_form_semantic_variant(method, params)
                            .and_then(|variant| catalog.form(method, variant))
                            .map(|form| (form, catalog, workflow))
                    })
                });
            *error_retry_contract = match retry {
                Some((form, catalog, workflow)) => match retry_contract(
                    form.action_key,
                    form.action_key,
                    workflow,
                    catalog,
                    TransitionAttemptDetails::None,
                    invalid_paths,
                ) {
                    Ok(contract) => Some(Box::new(contract)),
                    Err(_) => {
                        return McpAdapterError::SchemaContractFailure {
                            tool_name: tool.wire_name().to_owned(),
                        }
                    }
                },
                None => None,
            };
            *failure = Some(Box::new(argument_failure_presentation(
                &authoritative_context,
                false,
            )));
            *error_context = Some(Box::new(authoritative_context));
            *workflow_admission = None;
        }
        error
    }

    fn admit_workflow_action(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        method: MethodName,
        params: &Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<Option<WorkflowActionForm>, McpAdapterError> {
        if public_method_contract(method).workflow_action_admission()
            != WorkflowActionAdmissionClass::TaskStateBound
        {
            return Ok(None);
        }
        if method == MethodName::RequestUserAction
            && params.pointer("/request/operation").and_then(Value::as_str) == Some("resume")
        {
            return Ok(None);
        }
        let submitted_task_id = admission_task_id(params);
        let authoritative_context = self
            .load_authoritative_argument_context(context, params, session)?
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: method.as_str().to_owned(),
                message: submitted_task_id.as_ref().map_or_else(
                    || "current authoritative workflow could not be loaded".to_owned(),
                    |task_id| {
                        format!(
                            "current authoritative workflow could not be loaded for Task {}",
                            task_id.as_str()
                        )
                    },
                ),
            })?;
        let catalog = authoritative_context
            .action_form_catalog
            .as_ref()
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: method.as_str().to_owned(),
                message: "current authoritative workflow action-form catalog is unavailable"
                    .to_owned(),
            })?;
        let workflow = authoritative_context.workflow.as_ref().ok_or_else(|| {
            McpAdapterError::ToolExecution {
                tool_name: method.as_str().to_owned(),
                message: "current authoritative workflow is unavailable".to_owned(),
            }
        })?;
        let supplied = params
            .get("action_form_ref")
            .cloned()
            .and_then(|value| serde_json::from_value::<RequestHash>(value).ok());
        let called_semantic_variant = volicord_mcp_wire::submitted_action_form_semantic_variant(
            method, params,
        )
        .or_else(|| {
            supplied
                .as_ref()
                .and_then(|form_ref| catalog.form_by_ref(form_ref))
                .filter(|form| form.action_key.method == method)
                .map(|form| form.action_key.semantic_variant)
        });
        let method_forms = catalog
            .forms_for_method(method)
            .cloned()
            .collect::<Vec<_>>();
        let required_method_forms = catalog.required_forms().cloned().collect::<Vec<_>>();
        let required_method = workflow
            .transition_catalog()
            .required_transition()
            .map(|transition| transition.action_key.method);
        let mut allowed_methods = catalog
            .forms
            .iter()
            .map(|form| form.action_key.method)
            .collect::<Vec<_>>();
        allowed_methods.dedup();
        let valid_called_method_variants = method_forms
            .iter()
            .map(|form| form.action_key.semantic_variant)
            .collect::<Vec<_>>();
        let valid_called_method_form_refs = method_forms
            .iter()
            .map(|form| form.form_ref.clone())
            .collect::<Vec<_>>();
        let called_method_form = called_semantic_variant
            .and_then(|variant| catalog.form(method, variant))
            .cloned();
        let admission = McpWorkflowAdmissionRejection {
            called_method: method,
            called_semantic_variant: RequiredNullable::new(called_semantic_variant),
            current_workflow_kind: workflow.kind(),
            required_method: RequiredNullable::new(required_method),
            allowed_methods,
            valid_called_method_variants,
            valid_called_method_form_refs,
            called_method_form: RequiredNullable::new(called_method_form.clone()),
            required_method_forms,
            state_change_applied: false,
            reached_core: false,
        };
        if method_forms.is_empty() {
            return Err(McpAdapterError::InvalidParams {
                code: McpToolErrorCode::WorkflowActionNotAllowed,
                tool_name: method.as_str().to_owned(),
                issues: vec![McpToolErrorIssue::new(
                    String::new(),
                    McpToolIssueCode::WorkflowActionNotAllowed,
                    format!(
                        "{} is not current; Core was not reached and state did not change; required method is {}",
                        method.as_str(),
                        required_method
                            .as_ref()
                            .map(|required| required.as_str())
                            .unwrap_or("none")
                    ),
                )],
                truncated: false,
                selected_variant: None,
                canonical_example: None,
                authoritative_context: Some(Box::new(authoritative_context.clone())),
                retry_contract: None,
                failure: Some(Box::new(argument_failure_presentation(
                    &authoritative_context,
                    false,
                ))),
                workflow_admission: Some(Box::new(admission)),
                action_form_argument_mismatches: Box::default(),
            });
        }
        if supplied
            .as_ref()
            .and_then(|form_ref| catalog.form_by_ref(form_ref))
            .is_some_and(|form| form.action_key.method != method)
        {
            return Err(McpAdapterError::InvalidParams {
                code: McpToolErrorCode::ActionFormStale,
                tool_name: method.as_str().to_owned(),
                issues: vec![McpToolErrorIssue::new(
                    "/action_form_ref",
                    McpToolIssueCode::ActionFormMismatch,
                    "action_form_ref belongs to another method; Core was not reached and state did not change",
                )],
                truncated: false,
                selected_variant: called_semantic_variant
                    .map(|variant| variant.as_str().to_owned()),
                canonical_example: None,
                authoritative_context: Some(Box::new(authoritative_context.clone())),
                retry_contract: called_method_form
                    .as_ref()
                    .map(|form| {
                        retry_contract(
                            form.action_key,
                            form.action_key,
                            workflow,
                            catalog,
                            TransitionAttemptDetails::None,
                            vec!["/action_form_ref".to_owned()],
                        )
                        .map(Box::new)
                        .map_err(|_| McpAdapterError::SchemaContractFailure {
                            tool_name: method.as_str().to_owned(),
                        })
                    })
                    .transpose()?,
                failure: Some(Box::new(argument_failure_presentation(
                    &authoritative_context,
                    false,
                ))),
                workflow_admission: Some(Box::new(admission)),
                action_form_argument_mismatches: Box::default(),
            });
        }
        let Some(called_semantic_variant) = called_semantic_variant else {
            return Err(McpAdapterError::ToolExecution {
                tool_name: method.as_str().to_owned(),
                message: "the submitted workflow action semantic variant could not be decoded"
                    .to_owned(),
            });
        };
        let Some(called_method_form) = called_method_form else {
            let variant_path = if method == MethodName::RecordShapingCheckpoint {
                "/checkpoint_operation/operation"
            } else {
                "/change_unit/operation"
            };
            return Err(McpAdapterError::InvalidParams {
                code: McpToolErrorCode::WorkflowActionVariantNotAllowed,
                tool_name: method.as_str().to_owned(),
                issues: vec![McpToolErrorIssue::new(
                    variant_path,
                    McpToolIssueCode::WorkflowActionVariantNotAllowed,
                    format!(
                        "{} variant {} is not current; Core was not reached and state did not change",
                        method.as_str(),
                        called_semantic_variant.as_str()
                    ),
                )],
                truncated: false,
                selected_variant: Some(called_semantic_variant.as_str().to_owned()),
                canonical_example: None,
                authoritative_context: Some(Box::new(authoritative_context.clone())),
                retry_contract: None,
                failure: Some(Box::new(argument_failure_presentation(
                    &authoritative_context,
                    false,
                ))),
                workflow_admission: Some(Box::new(admission)),
                action_form_argument_mismatches: Box::default(),
            });
        };
        if supplied
            .as_ref()
            .and_then(|form_ref| catalog.form_by_ref(form_ref))
            == Some(&called_method_form)
        {
            return Ok(Some(called_method_form));
        }
        Err(McpAdapterError::InvalidParams {
            code: McpToolErrorCode::ActionFormStale,
            tool_name: method.as_str().to_owned(),
            issues: vec![McpToolErrorIssue::new(
                "/action_form_ref",
                McpToolIssueCode::ActionFormMismatch,
                "action_form_ref is missing, malformed, stale, foreign, or does not belong to the called method and semantic variant",
            )],
            truncated: false,
            selected_variant: None,
            canonical_example: None,
            authoritative_context: Some(Box::new(authoritative_context.clone())),
            retry_contract: Some(Box::new(
                retry_contract(
                    called_method_form.action_key,
                    called_method_form.action_key,
                    workflow,
                    catalog,
                    TransitionAttemptDetails::None,
                    vec!["/action_form_ref".to_owned()],
                )
                .map_err(|_| McpAdapterError::SchemaContractFailure {
                    tool_name: method.as_str().to_owned(),
                })?,
            )),
            failure: Some(Box::new(argument_failure_presentation(
                &authoritative_context,
                false,
            ))),
            workflow_admission: Some(Box::new(admission)),
            action_form_argument_mismatches: Box::default(),
        })
    }

    fn load_authoritative_argument_context(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        params: &Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<Option<AuthoritativeArgumentContext>, McpAdapterError> {
        let Some(object) = params.as_object() else {
            return Ok(None);
        };
        let project_selector = match object.get("project_selector") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => Some(value.as_str()),
            Some(_) => return Ok(None),
        };
        let Some(project_id) = self.select_project(context, project_selector).ok() else {
            return Ok(None);
        };
        let submitted_task_id = admission_task_id(params);
        let task_id = if object.contains_key("action_form_ref") {
            let Some(task_id) = CoreProjectStore::open_for_mutation(context, &project_id)
                .ok()
                .and_then(|store| store.project_state().ok())
                .and_then(|state| state.active_task_id.map(TaskId::new))
                .or(submitted_task_id)
            else {
                return Ok(None);
            };
            task_id
        } else {
            let Some(task_id) = submitted_task_id else {
                return Ok(None);
            };
            task_id
        };
        let refresh_context = MutationRefreshContext {
            project_id: project_id.clone(),
            task_id: task_id.clone(),
        };
        let Some(response) = self
            .refresh_authority_status(context, &project_id, &task_id, session)
            .ok()
        else {
            return Ok(None);
        };
        let Some(authority) = validated_authority_refresh(&refresh_context, &response).ok() else {
            return Ok(None);
        };
        let scope_revision = response
            .response_value
            .pointer("/active_task/scope_revision")
            .and_then(Value::as_u64);
        let baseline_ref = response
            .response_value
            .pointer("/active_task/baseline_ref")
            .and_then(|value| serde_json::from_value::<Option<BaselineRef>>(value.clone()).ok())
            .flatten();
        let current_checkpoint_ref = authority
            .workflow
            .checkpoint()
            .map(|checkpoint| checkpoint.checkpoint_ref.clone());
        let action_form_catalog = self
            .validated_workflow_action_form_catalog(
                context,
                &project_id,
                &authority.workflow,
                session,
            )
            .map_err(|failure| McpAdapterError::InternalContractInconsistent {
                tool_name: "workflow_action_form_catalog".to_owned(),
                reached_core: failure.reached_core(),
                transition_rejection: None,
                diagnostics: Box::new(McpWorkflowContractDiagnostics {
                    normalized_workflow_snapshot: authority.workflow.clone(),
                    current_transition_catalog: authority.workflow.transition_catalog().clone(),
                    current_action_forms: RequiredNullable::null(),
                    attempted_action_key: RequiredNullable::null(),
                    typed_rejection_reason: RequiredNullable::null(),
                    recovery_action_key: RequiredNullable::null(),
                    failed_action_key: RequiredNullable::new(failure.action_key),
                    failed_stage: RequiredNullable::some(failure.stage),
                    workflow_contract_digest:
                        volicord_types::managed_guidance::workflow_contract_semantic_digest(),
                    action_form_contract_digest:
                        volicord_types::managed_guidance::action_form_contract_semantic_digest(),
                    semantic_schema_digest:
                        volicord_types::managed_guidance::mcp_semantic_schema_digest(),
                    scalar_contract_digest:
                        volicord_types::canonical_scalar::baseline_ref_scalar_contract_digest(),
                }),
            })?;
        Ok(Some(AuthoritativeArgumentContext {
            context_loaded: true,
            project_id: RequiredNullable::some(project_id),
            state_version: RequiredNullable::some(authority.receipt.state_version),
            task_mode: RequiredNullable::some(authority.task_mode),
            work_phase: RequiredNullable::some(authority.work_phase),
            scope_revision: RequiredNullable::new(scope_revision),
            baseline_ref: RequiredNullable::new(baseline_ref),
            current_checkpoint_ref: RequiredNullable::new(current_checkpoint_ref),
            workflow: RequiredNullable::some(authority.workflow),
            action_form_catalog: RequiredNullable::some(action_form_catalog),
        }))
    }

    pub(crate) fn validated_workflow_action_form_catalog(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        project_id: &ProjectId,
        workflow: &WorkflowProjection,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<
        volicord_mcp_wire::WorkflowActionFormCatalog,
        crate::action_form::ActionFormCatalogError,
    > {
        workflow_action_form_catalog(project_id, workflow, |form, mut witness| {
            witness
                .as_object_mut()
                .ok_or_else(|| {
                    (
                        McpWorkflowContractStage::AdapterProjection,
                        "complete action-form witness is not an object".to_owned(),
                    )
                })?
                .insert("project_selector".to_owned(), serde_json::json!(project_id));
            self.plan_action_form_submission_no_commit(context, form, witness, session)
        })
    }

    fn plan_action_form_submission_no_commit(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        form: &WorkflowActionForm,
        witness: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<(), (McpWorkflowContractStage, String)> {
        let mut planner = self.clone();
        planner.planning_action_key = Some(form.action_key);
        let tool = AgentToolId::from_method(form.action_key.method).ok_or_else(|| {
            (
                McpWorkflowContractStage::AdapterProjection,
                "current action has no MCP adapter projection".to_owned(),
            )
        })?;
        let tool_name = tool.wire_name();
        let version = Some(form.expected_state_version);
        let result = match form.action_key.method {
            MethodName::UpdateScope => {
                planner.call_update_scope(context, tool_name, witness, session, version)
            }
            MethodName::RecordShapingCheckpoint => planner
                .call_record_shaping_checkpoint(context, tool_name, witness, session, version),
            MethodName::FinalizeAdvice => {
                planner.call_finalize_advice(context, tool_name, witness, session, version)
            }
            MethodName::AdvanceTask => {
                planner.call_advance_task(context, tool_name, witness, session, version)
            }
            MethodName::PrepareEvidenceCapture => {
                planner.call_prepare_evidence_capture(context, tool_name, witness, session, version)
            }
            MethodName::PrepareWrite => {
                planner.call_prepare_write(context, tool_name, witness, session, version)
            }
            MethodName::StageArtifact => {
                planner.call_stage_artifact(context, tool_name, witness, session, version)
            }
            MethodName::RecordRun => {
                planner.call_record_run(context, tool_name, witness, session, version)
            }
            MethodName::RequestUserAction => {
                planner.call_request_user_action(context, tool_name, witness, session, version)
            }
            MethodName::ReconcileChanges => {
                planner.call_reconcile_changes(context, tool_name, witness, session, version)
            }
            MethodName::CheckClose => {
                planner.call_check_close(context, tool_name, witness, session)
            }
            MethodName::CloseTask => {
                planner.call_close_task(context, tool_name, witness, session, version)
            }
            MethodName::Intake
            | MethodName::Status
            | MethodName::GetOperationResult
            | MethodName::ResolveUserAction => {
                return Err((
                    McpWorkflowContractStage::AdapterProjection,
                    "current Agent transition has no exact state-bound adapter planner".to_owned(),
                ));
            }
        };
        result.map(|_| ()).map_err(|error| {
            let (stage, detail) = match &error {
                McpAdapterError::Core(CorePipelineError::Invariant { detail })
                    if detail.contains("expected-result") || detail.contains("expected result") =>
                {
                    (
                        McpWorkflowContractStage::ExpectedResultValidation,
                        detail.clone(),
                    )
                }
                McpAdapterError::Core(error) => {
                    (McpWorkflowContractStage::CorePlanning, error.to_string())
                }
                _ => (
                    McpWorkflowContractStage::AdapterProjection,
                    error.to_string(),
                ),
            };
            (stage, detail)
        })
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
            None,
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
        bound_expected_state_version: Option<u64>,
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
            bound_expected_state_version,
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
            None,
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

    fn call_record_shaping_checkpoint(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
        bound_expected_state_version: Option<u64>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRecordShapingCheckpointArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
            bound_expected_state_version,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            RecordShapingCheckpointRequest {
                envelope,
                task_id,
                checkpoint_operation: args.checkpoint_operation,
                scope_revision: args.scope_revision,
                baseline_ref: args.baseline_ref,
                summary: args.summary,
                implementation_boundary: args.implementation_boundary,
                gaps: args.gaps,
                source_refs: args.source_refs,
                evidence_refs: args.evidence_refs,
            },
            CoreService::record_shaping_checkpoint,
            session,
        )
    }

    fn call_finalize_advice(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
        bound_expected_state_version: Option<u64>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpFinalizeAdviceArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
            bound_expected_state_version,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            FinalizeAdviceRequest {
                envelope,
                task_id,
                shaping_checkpoint_id: args.shaping_checkpoint_id,
                change_unit_id: args.change_unit_id,
                scope_revision: args.scope_revision,
                baseline_ref: args.baseline_ref,
                user_action_resolution_ids: args.user_action_resolution_ids,
                result_summary: args.result_summary,
                result_refs: args.result_refs,
                evidence_refs: args.evidence_refs,
                residual_risks: args.residual_risks,
                recovery_constraints: args.recovery_constraints,
            },
            CoreService::finalize_advice,
            session,
        )
    }

    fn call_advance_task(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
        bound_expected_state_version: Option<u64>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpAdvanceTaskArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
            bound_expected_state_version,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            AdvanceTaskRequest {
                envelope,
                task_id,
                shaping_checkpoint_id: args.shaping_checkpoint_id,
                change_unit_id: args.change_unit_id,
                scope_revision: args.scope_revision,
                baseline_ref: args.baseline_ref,
                user_action_resolution_ids: args.user_action_resolution_ids,
            },
            CoreService::advance_task,
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
            None,
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
            None,
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
        bound_expected_state_version: Option<u64>,
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
            bound_expected_state_version,
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
        bound_expected_state_version: Option<u64>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareWriteArguments> =
            self.prepare_mcp_arguments(context, tool_name, params, session)?;
        let task_id = prepared.arguments.task_id.clone();
        let envelope = self.generated_envelope(
            context,
            tool_name,
            &prepared.project_id,
            Some(&task_id),
            OperationCategory::AgentWorkflow,
            bound_expected_state_version,
        )?;
        let args = prepared.arguments;
        self.call_core_request(
            context,
            tool_name,
            PrepareWriteRequest {
                envelope,
                task_id: RequiredNullable::some(task_id),
                change_unit_id: RequiredNullable::some(args.change_unit_id),
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
        bound_expected_state_version: Option<u64>,
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
            bound_expected_state_version,
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
        bound_expected_state_version: Option<u64>,
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
            bound_expected_state_version,
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
        bound_expected_state_version: Option<u64>,
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
                    bound_expected_state_version,
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
        bound_expected_state_version: Option<u64>,
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
            bound_expected_state_version,
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
            None,
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
        bound_expected_state_version: Option<u64>,
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
            bound_expected_state_version,
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
        T: MethodOperationCategory
            + MethodResponseContract
            + HasEnvelope
            + IntoTransitionSubmission
            + Clone,
        F: FnOnce(
            &CoreService,
            &RuntimeHomeMutationContext<'_>,
            T,
            InvocationContext,
        ) -> Result<PipelineResponse, CorePipelineError>,
    {
        let operation_category = request.operation_category();
        if self.planning_action_key.is_none() {
            self.ensure_storage_writable_for_tool(context, tool_name)?;
            self.ensure_mode_allows(context, tool_name, operation_category)?;
        }
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
        let invocation = if self.planning_action_key.is_some() {
            self.derive_read_only_invocation_context(
                context,
                request_envelope(&request),
                OperationCategory::Read,
                session,
            )?
        } else if operation_category == OperationCategory::Read {
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
        if let Some(action_key) = self.planning_action_key {
            let submission = request
                .clone()
                .into_transition_submission()
                .ok_or_else(|| McpAdapterError::SchemaContractFailure {
                    tool_name: tool_name.to_owned(),
                })?;
            let plan = core
                .plan_transition_submission_no_commit(
                    context,
                    action_key,
                    submission,
                    invocation.core_invocation_for_category(operation_category),
                )
                .map_err(McpAdapterError::Core)?;
            return Ok(PipelineResponse::from_no_commit_transition_plan(plan));
        }
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
                .map(|timestamp| timestamp.to_string())
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
        bound_expected_state_version: Option<u64>,
    ) -> Result<ToolEnvelope, McpAdapterError> {
        let state_version = if operation_category == OperationCategory::Read {
            None
        } else if AgentToolId::from_wire_name(tool_name)
            .ok()
            .and_then(AgentToolId::method)
            .is_some_and(|method| {
                public_method_contract(method).workflow_action_admission()
                    == WorkflowActionAdmissionClass::TaskStateBound
            })
        {
            Some(bound_expected_state_version.ok_or_else(|| {
                McpAdapterError::SchemaContractFailure {
                    tool_name: tool_name.to_owned(),
                }
            })?)
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
        serde_json::from_value(params).map_err(|_| McpAdapterError::SchemaContractFailure {
            tool_name: tool_name.to_owned(),
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

trait HasEnvelope {
    fn envelope(&self) -> &ToolEnvelope;
}

trait IntoTransitionSubmission {
    fn into_transition_submission(self) -> Option<TransitionSubmission>;
}

macro_rules! impl_transition_submission {
    ($($request:ty => $variant:ident),* $(,)?) => {
        $(
            impl IntoTransitionSubmission for $request {
                fn into_transition_submission(self) -> Option<TransitionSubmission> {
                    Some(TransitionSubmission::$variant(self))
                }
            }
        )*
    };
}

macro_rules! impl_no_transition_submission {
    ($($request:ty),* $(,)?) => {
        $(
            impl IntoTransitionSubmission for $request {
                fn into_transition_submission(self) -> Option<TransitionSubmission> {
                    None
                }
            }
        )*
    };
}

impl_transition_submission!(
    UpdateScopeRequest => UpdateScope,
    RecordShapingCheckpointRequest => RecordShapingCheckpoint,
    FinalizeAdviceRequest => FinalizeAdvice,
    AdvanceTaskRequest => AdvanceTask,
    PrepareEvidenceCaptureRequest => PrepareEvidenceCapture,
    PrepareWriteRequest => PrepareWrite,
    StageArtifactRequest => StageArtifact,
    RecordRunRequest => RecordRun,
    RequestUserActionRequest => RequestUserAction,
    ReconcileChangesRequest => ReconcileChanges,
    CheckCloseRequest => CheckClose,
    CloseTaskRequest => CloseTask,
);

impl_no_transition_submission!(IntakeRequest, StatusRequest, GetOperationResultRequest,);

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
    RecordShapingCheckpointRequest,
    FinalizeAdviceRequest,
    AdvanceTaskRequest,
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

fn admission_task_id(params: &Value) -> Option<TaskId> {
    params
        .get("task_id")
        .or_else(|| params.pointer("/request/task_id"))
        .and_then(Value::as_str)
        .map(TaskId::new)
}

fn unloaded_authoritative_argument_context() -> AuthoritativeArgumentContext {
    AuthoritativeArgumentContext {
        context_loaded: false,
        project_id: RequiredNullable::null(),
        state_version: RequiredNullable::null(),
        task_mode: RequiredNullable::null(),
        work_phase: RequiredNullable::null(),
        scope_revision: RequiredNullable::null(),
        baseline_ref: RequiredNullable::null(),
        current_checkpoint_ref: RequiredNullable::null(),
        workflow: RequiredNullable::<WorkflowProjection>::null(),
        action_form_catalog: RequiredNullable::null(),
    }
}

fn argument_failure_presentation(
    context: &AuthoritativeArgumentContext,
    reached_core: bool,
) -> McpArgumentFailurePresentation {
    McpArgumentFailurePresentation {
        method_committed: false,
        reached_core,
        current_task_phase: context.work_phase.clone(),
        current_state_version: context.state_version.clone(),
        checkpoint_recorded: false,
        user_action_created: false,
        product_repository_changed: false,
        core_state_unchanged: true,
        current_baseline_canonical: RequiredNullable::new(context.context_loaded.then_some(true)),
        submitted_baseline_canonical: RequiredNullable::null(),
        submitted_baseline_matches_current: RequiredNullable::null(),
        submitted_baseline_compatible_with_transition: RequiredNullable::null(),
        exact_retry_action: RequiredNullable::null(),
        repair_required: false,
    }
}

struct PreparedMcpArguments<T> {
    arguments: T,
    project_id: ProjectId,
}
