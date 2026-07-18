use crate::errors::McpAdapterError;
use crate::prelude::*;
use crate::routing::*;
use crate::schema_validation::validate_mcp_tool_arguments;
use crate::tool_registry::*;
use crate::util::*;
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
pub(crate) struct McpDerivedInvocationContext {
    pub project_id: ProjectId,
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub validated_agent_session: volicord_core::ValidatedAgentSession,
    pub git_workspace_context: Option<GitWorkspaceContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentSessionCoordinates<'a> {
    pub(crate) runtime_session_id: &'a str,
    pub(crate) project_session_id: &'a str,
}

/// Runtime-owned host correlation passed from the stdio lifecycle to project binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedAgentSessionBinding {
    pub(crate) runtime_session_id: String,
    pub(crate) session_id: String,
    pub(crate) host_session_id: String,
    pub(crate) host_thread_id: String,
    pub(crate) host_turn_id: String,
}

impl ManagedAgentSessionBinding {
    pub(crate) fn coordinates(&self) -> AgentSessionCoordinates<'_> {
        AgentSessionCoordinates {
            runtime_session_id: &self.runtime_session_id,
            project_session_id: &self.session_id,
        }
    }
}

impl McpDerivedInvocationContext {
    fn core_invocation(&self) -> InvocationContext {
        let mut invocation = InvocationContext::new(
            self.project_id.clone(),
            self.actor_source.clone(),
            self.operation_category,
            "",
        );
        if let Some(workspace) = self.git_workspace_context.as_ref() {
            invocation = invocation.with_git_workspace_context(workspace.clone());
        }
        invocation = invocation.with_validated_agent_session(self.validated_agent_session.clone());
        invocation
    }
}

/// Local MCP adapter bound to a Core service and one Agent Connection.
#[derive(Debug, Clone)]
pub struct McpAdapter {
    pub(crate) core: CoreService,
    pub(crate) runtime_home: PathBuf,
    pub(crate) context: McpConnectionContext,
    default_agent_session_binding: Option<ManagedAgentSessionBinding>,
}

impl PartialEq for McpAdapter {
    fn eq(&self, other: &Self) -> bool {
        self.core == other.core
            && self.runtime_home == other.runtime_home
            && self.context == other.context
            && self.default_agent_session_binding == other.default_agent_session_binding
    }
}

impl Eq for McpAdapter {}

impl McpAdapter {
    /// Creates an adapter for a Runtime Home and connection-bound adapter context.
    pub fn new(runtime_home: impl AsRef<Path>, context: McpConnectionContext) -> Self {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        Self {
            core: CoreService::new(&runtime_home),
            runtime_home,
            context,
            default_agent_session_binding: None,
        }
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
        Ok(mcp_tools_for_mode_and_storage_with_detail(
            mode,
            storage_capability,
            ToolSchemaDetail::RuntimeCompact,
        ))
    }

    pub(crate) fn session_storage_capability(
        &self,
    ) -> Result<McpStorageCapability, McpAdapterError> {
        let projects = self.allowed_project_availabilities("storage capability")?;
        Ok(storage_capability_for_projects(&projects))
    }

    /// Derives local invocation facts for one decoded request envelope.
    pub(crate) fn derive_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session: Option<AgentSessionCoordinates<'_>>,
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
        let validated_agent_session =
            self.validated_session_for_project(&envelope.project_id, operation_category, session)?;
        Ok(McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(
                self.context.connection_internal_id.clone(),
            ),
            operation_category,
            validated_agent_session,
            git_workspace_context,
        })
    }

    fn derive_read_only_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        operation_category: OperationCategory,
        session: Option<AgentSessionCoordinates<'_>>,
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
        let validated_agent_session =
            self.validated_session_for_project(&envelope.project_id, operation_category, session)?;
        Ok(McpDerivedInvocationContext {
            project_id: envelope.project_id.clone(),
            actor_source: ActorSource::agent_connection(
                self.context.connection_internal_id.clone(),
            ),
            operation_category,
            validated_agent_session,
            git_workspace_context,
        })
    }

    fn validated_session_for_project(
        &self,
        project_id: &ProjectId,
        operation_category: OperationCategory,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<volicord_core::ValidatedAgentSession, McpAdapterError> {
        let session = session.ok_or_else(|| {
            McpAdapterError::Environment(
                "agent_session_missing: project tools require a current managed runtime and project session"
                    .to_owned(),
            )
        })?;
        self.core
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
        if let Some(binding) = self.default_agent_session_binding.as_ref() {
            self.ensure_agent_session_binding_for_tool(tool_name, &params, binding)?;
        }
        self.call_tool_for_session(
            tool_name,
            params,
            self.default_agent_session_binding
                .as_ref()
                .map(ManagedAgentSessionBinding::coordinates),
        )
    }

    pub(crate) fn call_tool_for_session(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        validate_mcp_tool_arguments(tool_name, &params)?;
        match tool_name {
            INTAKE_TOOL_NAME => self.call_intake(tool_name, params, session),
            UPDATE_SCOPE_TOOL_NAME => self.call_update_scope(tool_name, params, session),
            STATUS_TOOL_NAME => self.call_status(tool_name, params, session),
            GET_OPERATION_RESULT_TOOL_NAME => {
                self.call_get_operation_result(tool_name, params, session)
            }
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME => {
                self.call_prepare_evidence_capture(tool_name, params, session)
            }
            PREPARE_WRITE_TOOL_NAME => self.call_prepare_write(tool_name, params, session),
            STAGE_ARTIFACT_TOOL_NAME => self.call_stage_artifact(tool_name, params, session),
            RECORD_RUN_TOOL_NAME => self.call_record_run(tool_name, params, session),
            REQUEST_USER_ACTION_TOOL_NAME => {
                self.call_request_user_action(tool_name, params, session)
            }
            RECONCILE_CHANGES_TOOL_NAME => self.call_reconcile_changes(tool_name, params, session),
            CHECK_CLOSE_TOOL_NAME => self.call_check_close(tool_name, params, session),
            CLOSE_TASK_TOOL_NAME => self.call_close_task(tool_name, params, session),
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn call_intake(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpIntakeArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpUpdateScopeArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    fn call_status(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStatusArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
                continuity_page: args.continuity_page,
                include: args.detail.include(),
            },
            CoreService::status,
            session,
        )
    }

    fn call_get_operation_result(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpGetOperationResultArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    pub(crate) fn refresh_authority_status(
        &self,
        project_id: &ProjectId,
        task_id: &TaskId,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let session = session.or_else(|| {
            self.default_agent_session_binding
                .as_ref()
                .map(ManagedAgentSessionBinding::coordinates)
        });
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
                continuity_page: None,
                include: StatusDetailLevel::Workflow.include(),
            },
            CoreService::status,
            session,
        )
    }

    pub(crate) fn has_default_agent_session(&self) -> bool {
        self.default_agent_session_binding.is_some()
    }

    fn call_prepare_evidence_capture(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareEvidenceCaptureArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    fn call_prepare_write(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpPrepareWriteArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    fn call_stage_artifact(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpStageArtifactArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    fn call_record_run(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRecordRunArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
                performed_operation: args.performed_operation,
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
            session,
        )
    }

    fn call_request_user_action(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpRequestUserActionArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
                    session,
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
                let invocation = self.derive_read_only_invocation_context(
                    &envelope,
                    OperationCategory::AgentWorkflow,
                    session,
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
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpReconcileChangesArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    fn call_check_close(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCheckCloseArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
        )
    }

    fn call_close_task(
        &self,
        tool_name: &str,
        params: Value,
        session: Option<AgentSessionCoordinates<'_>>,
    ) -> Result<PipelineResponse, McpAdapterError> {
        let prepared: PreparedMcpArguments<McpCloseTaskArguments> =
            self.prepare_mcp_arguments(tool_name, params, session)?;
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
            session,
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
        session: Option<AgentSessionCoordinates<'_>>,
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
        let invocation = self.derive_invocation_context(
            request_envelope(&request),
            operation_category,
            session,
        )?;
        call(&self.core, request, invocation.core_invocation()).map_err(McpAdapterError::Core)
    }

    pub(crate) fn call_adapter_tool(
        &self,
        tool_name: &str,
        params: Value,
        _session_id: Option<&str>,
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
        let selected_project_id = self.select_project(requested_project_selector.as_deref())?;
        Ok(PreparedMcpArguments {
            arguments,
            project_id: selected_project_id,
        })
    }

    pub(crate) fn ensure_agent_session_binding_for_tool(
        &self,
        tool_name: &str,
        params: &Value,
        binding: &ManagedAgentSessionBinding,
    ) -> Result<(), McpAdapterError> {
        if !PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) && tool_name != LIST_PROJECTS_TOOL_NAME {
            return Ok(());
        }
        let object = params
            .as_object()
            .ok_or_else(|| McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: "tool arguments must be an object".to_owned(),
            })?;
        let project_id = if tool_name == LIST_PROJECTS_TOOL_NAME {
            let projects = self.allowed_project_availabilities(tool_name)?;
            let [project] = projects.as_slice() else {
                return Ok(());
            };
            if project.storage_capability != McpStorageCapability::ReadWrite {
                return Ok(());
            }
            ProjectId::new(&project.project_id)
        } else {
            let requested_project_selector =
                optional_string_field(object, "project_selector", tool_name)?;
            self.select_project(requested_project_selector.as_deref())?
        };
        if self.storage_capability_for_project(&project_id)? == McpStorageCapability::ReadWrite {
            self.ensure_agent_session_binding(&project_id, binding)?;
        }
        Ok(())
    }

    fn ensure_agent_session_binding(
        &self,
        project_id: &ProjectId,
        binding: &ManagedAgentSessionBinding,
    ) -> Result<(), McpAdapterError> {
        let _connection = current_enabled_connection(
            &self.runtime_home,
            self.context.connection_internal_id.as_str(),
            "managed stdio session binding",
        )?;
        validate_managed_stdio_session_id(&binding.session_id).map_err(|_| {
            McpAdapterError::Environment(
                "managed_stdio_session_identity_invalid: managed stdio requires a canonical internal session coordinate"
                    .to_owned(),
            )
        })?;
        let guard_installations = list_guard_installations(
            &self.runtime_home,
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
            let manifest = volicord_types::guard_manifest_from_json(&installation.manifest_json)
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
        let observed_at = CoreProjectStore::open(&self.runtime_home, project_id)
            .and_then(|store| store.current_timestamp())
            .map_err(McpAdapterError::Store)?;
        upsert_agent_session(
            &self.runtime_home,
            project_id.as_str(),
            AgentSessionUpsert {
                session_id: binding.session_id.clone(),
                runtime_session_id: Some(binding.runtime_session_id.clone()),
                connection_internal_id: self.context.connection_internal_id.as_str().to_owned(),
                guard_installation_id,
                host_session_id: binding.host_session_id.clone(),
                host_thread_id: binding.host_thread_id.clone(),
                host_turn_id: binding.host_turn_id.clone(),
                observed_at,
            },
        )
        .map_err(McpAdapterError::Store)?;
        Ok(())
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
