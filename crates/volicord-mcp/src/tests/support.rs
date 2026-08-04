use super::*;
use std::path::Path;

pub(super) fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    adapter_at_runtime_home(fixture, fixture.runtime_home_path())
}

pub(super) fn adapter_at_runtime_home(
    fixture: &CoreFixture,
    runtime_home: &Path,
) -> Result<McpAdapter, Box<dyn Error>> {
    let context = McpConnectionContext::resolve(runtime_home, fixture.connection_id())?;
    let guard = guard_health_record(runtime_home, fixture.project_id(), fixture.connection_id())?;
    let session = volicord_test_support::seed_test_agent_session(
        runtime_home,
        fixture.project_id(),
        fixture.connection_id(),
        guard
            .guard_installation
            .as_ref()
            .map(|installation| installation.guard_installation_id.as_str()),
    )?;
    Ok(
        McpAdapter::new(runtime_home, context).with_managed_agent_session_binding(
            ManagedAgentSessionBinding {
                runtime_session_id: session.runtime_session_id.as_str().to_owned(),
                correlation: volicord_host_contract::CodexMcpCorrelation {
                    session_id: volicord_host_contract::HostSessionId::parse(
                        session.host_session_id,
                    )?,
                    thread_id: volicord_host_contract::HostThreadId::parse(session.host_thread_id)?,
                    turn_id: volicord_host_contract::HostTurnId::parse(session.host_turn_id)?,
                },
            },
        ),
    )
}

pub(super) fn test_agent_invocation(
    fixture: &CoreFixture,
    operation_category: OperationCategory,
) -> InvocationContext {
    let guard = guard_health_record(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )
    .expect("guard authority fixture must load");
    let session = volicord_test_support::seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        guard
            .guard_installation
            .as_ref()
            .map(|installation| installation.guard_installation_id.as_str()),
    )
    .expect("managed Agent Session fixture must seed");
    let validated = CoreService::for_read_only(fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            operation_category,
        )
        .expect("managed Agent Session fixture must validate");
    InvocationContext::agent_connection(operation_category, validated)
}

pub(super) fn adapter_for_additional_connection(
    fixture: &CoreFixture,
    connection_id: &str,
) -> Result<McpAdapter, Box<dyn Error>> {
    let existing = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
        .expect("fixture connection should exist");
    ensure_agent_connection(
        &fixture.mutation_context()?,
        AgentConnectionRegistration {
            connection_internal_id: connection_id.to_owned(),
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: format!("{}_additional", existing.config_target),
            mode: existing.mode,
            enabled: existing.enabled,
            managed_fingerprint: format!("{}_additional", existing.managed_fingerprint),
            metadata_json: existing.metadata_json,
        },
    )?;
    add_connection_project(
        &fixture.mutation_context()?,
        ConnectionProjectRegistration {
            connection_internal_id: connection_id.to_owned(),
            project_id: fixture.project_id().to_owned(),
        },
    )?;
    let context = McpConnectionContext::resolve(fixture.runtime_home_path(), connection_id)?;
    let session = volicord_test_support::seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        connection_id,
        None,
    )?;
    Ok(
        McpAdapter::new(fixture.runtime_home_path(), context).with_managed_agent_session_binding(
            ManagedAgentSessionBinding {
                runtime_session_id: session.runtime_session_id.as_str().to_owned(),
                correlation: volicord_host_contract::CodexMcpCorrelation {
                    session_id: volicord_host_contract::HostSessionId::parse(
                        session.host_session_id,
                    )?,
                    thread_id: volicord_host_contract::HostThreadId::parse(session.host_thread_id)?,
                    turn_id: volicord_host_contract::HostTurnId::parse(session.host_turn_id)?,
                },
            },
        ),
    )
}

pub(super) fn project_bound_adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_project_allowlist(vec![ProjectId::new(fixture.project_id())]);
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

pub(super) fn install_record_guard(fixture: &CoreFixture) -> Result<(), Box<dyn Error>> {
    let repo_root = fixture.product_repo_path();
    let guard_installation_id = "guard_installation_mcp_record";
    let policy_hash = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
    upsert_guard_installation(
        &fixture.mutation_context()?,
        GuardInstallationUpsert {
            guard_installation_id: guard_installation_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: fixture.project_id().to_owned(),
            manifest_json: volicord_test_support::test_guard_manifest_json(
                fixture.runtime_home_path(),
                &repo_root,
                fixture.project_id(),
                fixture.connection_id(),
                guard_installation_id,
                policy_hash,
            ),
        },
    )?;
    Ok(())
}

pub(super) fn set_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
    volicord_test_support::transition_test_connection_mode(
        fixture.runtime_home_path(),
        &fixture.product_repo_path(),
        fixture.project_id(),
        fixture.connection_id(),
        mode,
    )?;
    Ok(())
}

pub(super) fn add_allowed_project(
    fixture: &CoreFixture,
    project_id: &str,
) -> Result<(), Box<dyn Error>> {
    let repo_root = fixture.create_product_repo(format!("repo-{project_id}"))?;
    register_project(
        &fixture.mutation_context()?,
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        &fixture.mutation_context()?,
        ConnectionProjectRegistration {
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: project_id.to_owned(),
        },
    )?;
    Ok(())
}

pub(super) fn create_pending_product_action(
    fixture: &CoreFixture,
) -> Result<(String, PipelineResponse), Box<dyn Error>> {
    let setup_adapter = adapter(fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let response = setup_adapter.call_tool(
        "volicord.request_user_action",
        product_action_args(fixture, &task_id, state_version),
    )?;
    Ok((task_id, response))
}

pub(super) fn user_action_side_effect_snapshot(
    fixture: &CoreFixture,
) -> Result<(volicord_store::core_pipeline::StorageEffectCounts, String), Box<dyn Error>> {
    let counts = fixture.counts()?;
    let project_updated_at =
        fixture
            .conn()?
            .query_row("SELECT updated_at FROM project_state", [], |row| row.get(0))?;
    Ok((counts, project_updated_at))
}

pub(super) fn create_task(adapter: &McpAdapter) -> Result<(String, u64), Box<dyn Error>> {
    let response = adapter.call_tool(
        "volicord.intake",
        json!({
            "plain_language_request": "Create a task for User Channel tests.",
            "requested_mode": "work",
            "resume_policy": "create_new",
            "acceptance_policy": null,
            "lineage": null,
            "initial_scope": {
                "boundary": "User Channel test task.",
                "non_goals": ["Changing unrelated behavior."],
                "acceptance_criteria": [{
                    "statement": "A pending judgment can be requested.",
                    "evidence_requirement": "required"
                }]
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

#[cfg(unix)]
pub(super) struct ReadOnlyProjectStateGuard {
    state_db_path: std::path::PathBuf,
    state_dir: std::path::PathBuf,
    old_state_mode: u32,
    old_dir_mode: u32,
}

#[cfg(unix)]
impl Drop for ReadOnlyProjectStateGuard {
    fn drop(&mut self) {
        let _ = fs::set_permissions(
            &self.state_dir,
            fs::Permissions::from_mode(self.old_dir_mode),
        );
        let _ = fs::set_permissions(
            &self.state_db_path,
            fs::Permissions::from_mode(self.old_state_mode),
        );
    }
}

#[cfg(unix)]
pub(super) fn make_project_state_readonly(
    fixture: &CoreFixture,
) -> Result<ReadOnlyProjectStateGuard, Box<dyn Error>> {
    let state_db_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let state_dir = state_db_path
        .parent()
        .expect("project state database should have a parent directory")
        .to_path_buf();
    let old_state_mode = fs::metadata(&state_db_path)?.permissions().mode();
    let old_dir_mode = fs::metadata(&state_dir)?.permissions().mode();

    fs::set_permissions(
        &state_db_path,
        fs::Permissions::from_mode(old_state_mode & !0o222),
    )?;
    fs::set_permissions(
        &state_dir,
        fs::Permissions::from_mode(old_dir_mode & !0o222),
    )?;

    Ok(ReadOnlyProjectStateGuard {
        state_db_path,
        state_dir,
        old_state_mode,
        old_dir_mode,
    })
}

pub(super) fn initialize_request(id: u64, capabilities: Value) -> Value {
    initialize_request_with_client_info(
        id,
        capabilities,
        CODEX_MANAGED_MCP_CLIENT_NAME,
        CODEX_TEST_CLIENT_VERSION,
    )
}

pub(super) fn initialize_request_with_client_info(
    id: u64,
    capabilities: Value,
    client_name: &str,
    client_version: &str,
) -> Value {
    initialize_request_for_protocol(
        id,
        capabilities,
        client_name,
        client_version,
        ProtocolRegistry::production()
            .preferred_server_profile()
            .revision()
            .as_str(),
    )
}

pub(super) fn initialize_request_for_protocol(
    id: u64,
    capabilities: Value,
    client_name: &str,
    client_version: &str,
    protocol_version: &str,
) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": protocol_version,
            "capabilities": capabilities,
            "clientInfo": {
                "name": client_name,
                "version": client_version
            }
        }),
    )
}

pub(super) fn initialized_notification() -> Value {
    notification("notifications/initialized", json!({}))
}

pub(super) fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

pub(super) fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
}

pub(super) fn tools_call(id: u64, name: &str, arguments: Value) -> Value {
    tools_call_with_codex_metadata(
        id,
        name,
        arguments,
        CODEX_TEST_SESSION_ID,
        CODEX_TEST_THREAD_ID,
        CODEX_TEST_TURN_ID,
    )
}

pub(super) fn tools_call_with_codex_metadata(
    id: u64,
    name: &str,
    arguments: Value,
    session_id: &str,
    thread_id: &str,
    turn_id: &str,
) -> Value {
    request(
        id,
        "tools/call",
        json!({
            "name": name,
            "arguments": arguments,
            "_meta": {
                "threadId": thread_id,
                "x-codex-turn-metadata": {
                    "session_id": session_id,
                    "thread_id": thread_id,
                    "turn_id": turn_id
                }
            }
        }),
    )
}

pub(super) fn intake_args(project_selector: Option<&str>) -> Value {
    let mut arguments = json!({
        "plain_language_request": "Exercise MCP lifecycle gating.",
        "requested_mode": "work",
        "resume_policy": "create_new",
        "acceptance_policy": null,
        "lineage": null,
        "initial_scope": {
            "boundary": "MCP lifecycle gating test.",
            "non_goals": ["Changing Core method behavior."],
            "acceptance_criteria": [{
                "statement": "tools/call is gated until notifications/initialized.",
                "evidence_requirement": "required"
            }]
        },
        "initial_context_refs": []
    });
    if let Some(project_selector) = project_selector {
        arguments["project_selector"] = json!(project_selector);
    }
    arguments
}

pub(super) fn product_action_args(
    fixture: &CoreFixture,
    task_id: &str,
    state_version: u64,
) -> Value {
    action_args(
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

pub(super) fn evidence_observation_action_args(
    task_id: &str,
    change_unit_id: &str,
    target_candidates: Vec<Value>,
    artifact_candidate_ids: Vec<String>,
) -> Value {
    json!({
        "request": {
            "operation": "create",
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "action": {
                "action_type": "evidence_observation",
                "question": "Does an exact stored artifact support the selected target?",
                "context_summary": "Inspect stored candidate bytes and record a user-owned observation.",
                "target_candidates": target_candidates,
                "artifact_candidate_ids": artifact_candidate_ids
            },
            "required_for": ["record_run"],
            "expires_at": null
        }
    })
}

pub(super) fn resume_user_action_args(
    fixture: &CoreFixture,
    user_action_request_id: &str,
) -> Value {
    json!({
        "project_selector": fixture.project_id(),
        "detail": "full",
        "request": {
            "operation": "resume",
            "user_action_request_id": user_action_request_id
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum McpUserActionCloseBasis {
    None,
    NoResidualRisks,
    VisibleResidualRisk,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum McpUserActionLeakageCaseKind {
    Choice {
        required_for: &'static [&'static str],
        close_basis: McpUserActionCloseBasis,
        sensitive: bool,
    },
    EvidenceObservation,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct McpUserActionLeakageCase {
    pub(super) name: &'static str,
    pub(super) kind: McpUserActionLeakageCaseKind,
}

impl McpUserActionLeakageCase {
    pub(super) const fn choice(
        name: &'static str,
        required_for: &'static [&'static str],
        close_basis: McpUserActionCloseBasis,
        sensitive: bool,
    ) -> Self {
        Self {
            name,
            kind: McpUserActionLeakageCaseKind::Choice {
                required_for,
                close_basis,
                sensitive,
            },
        }
    }

    pub(super) const fn evidence_observation() -> Self {
        Self {
            name: "evidence_observation",
            kind: McpUserActionLeakageCaseKind::EvidenceObservation,
        }
    }
}

pub(super) struct PreparedMcpUserActionLeakageCase {
    pub(super) task_id: String,
    pub(super) arguments: Value,
    pub(super) private_markers: Vec<String>,
}

pub(super) fn prepare_mcp_user_action_leakage_case(
    fixture: &CoreFixture,
    case: McpUserActionLeakageCase,
) -> Result<PreparedMcpUserActionLeakageCase, Box<dyn Error>> {
    let core = CoreService::for_mutation(&fixture.mutation_context()?);
    let invocation = || test_agent_invocation(fixture, OperationCategory::AgentWorkflow);
    let intake = core.intake(
        &fixture.mutation_context()?,
        fixture.intake_request(
            &format!("req_mcp_user_action_{}_task", case.name),
            &format!("idem_mcp_user_action_{}_task", case.name),
            false,
            Some(0),
        ),
        invocation(),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .ok_or("intake response should expose the Task")?
        .to_owned();
    let scope_request_id = format!("req_mcp_user_action_{}_scope", case.name);
    let scope_idempotency_key = format!("idem_mcp_user_action_{}_scope", case.name);
    let scope = core.update_scope(
        &fixture.mutation_context()?,
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: &scope_request_id,
            idempotency_key: &scope_idempotency_key,
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Exercise the UserAction adapter boundary.",
        }),
        invocation(),
    )?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope response should expose the current Change Unit")?
        .to_owned();
    let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .ok_or("scope response should expose the acceptance criterion")?
        .to_owned();
    let mut state_version = scope.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("scope response should expose state_version")?;
    let shaped = core.record_shaping_checkpoint(
        &fixture.mutation_context()?,
        RecordShapingCheckpointRequest {
            envelope: fixture.envelope(
                &format!("req_mcp_user_action_{}_shaping", case.name),
                Some(&format!("idem_mcp_user_action_{}_shaping", case.name)),
                false,
                Some(state_version),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            checkpoint_operation: volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
            scope_revision: 1,
            baseline_ref: RequiredNullable::some(BaselineRef::new(
                volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            )),
            summary: "The UserAction adapter fixture boundary is ready.".to_owned(),
            implementation_boundary: RequiredNullable::some(
                "Exercise only the current UserAction adapter boundary.".to_owned(),
            ),
            gaps: Vec::new(),
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
        },
        invocation(),
    )?;
    state_version = shaped.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("record_shaping_checkpoint response should expose state_version")?;
    let shaping_checkpoint_id = shaped.response_value["shaping_checkpoint"]
        ["shaping_checkpoint_id"]
        .as_str()
        .ok_or("record_shaping_checkpoint response should expose its checkpoint")?;
    let advanced = core.advance_task(
        &fixture.mutation_context()?,
        AdvanceTaskRequest {
            envelope: fixture.envelope(
                &format!("req_mcp_user_action_{}_advance", case.name),
                Some(&format!("idem_mcp_user_action_{}_advance", case.name)),
                false,
                Some(state_version),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(shaping_checkpoint_id),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new(
                volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            ),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(),
    )?;
    state_version = advanced.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("advance_task response should expose state_version")?;
    let mut registered_artifact_id = None;
    if let McpUserActionLeakageCaseKind::Choice { close_basis, .. } = case.kind {
        if close_basis != McpUserActionCloseBasis::None {
            let request_id = format!("req_mcp_user_action_{}_run", case.name);
            let idempotency_key = format!("idem_mcp_user_action_{}_run", case.name);
            let mut request = fixture.record_run_request(
                &request_id,
                &idempotency_key,
                false,
                Some(state_version),
                &task_id,
                &change_unit_id,
            );
            let residual_risks = if close_basis == McpUserActionCloseBasis::VisibleResidualRisk {
                vec![ResidualRiskInput {
                    summary: "A visible fixture risk remains.".to_owned(),
                    consequence: "The user must decide whether this fixture risk is acceptable."
                        .to_owned(),
                    acceptance_required: true,
                    source_refs: Vec::new(),
                }]
            } else {
                Vec::new()
            };
            request.close_assessment = Some(CloseAssessmentInput {
                result_summary: "Current close evidence is available.".to_owned(),
                result_refs: Vec::new(),
                residual_risks,
                sensitive_categories: Vec::new(),
                recovery_constraints: Vec::new(),
            })
            .into();
            let recorded = core.record_run(&fixture.mutation_context()?, request, invocation())?;
            state_version = recorded.response_value["base"]["state_version"]
                .as_u64()
                .ok_or("record_run response should expose state_version")?;
        }
    }
    if matches!(case.kind, McpUserActionLeakageCaseKind::EvidenceObservation) {
        let staged_request_id = format!("req_mcp_user_action_{}_stage", case.name);
        let staged_idempotency_key = format!("idem_mcp_user_action_{}_stage", case.name);
        let staged = core.stage_artifact(
            &fixture.mutation_context()?,
            fixture.stage_artifact_request(
                &staged_request_id,
                Some(&staged_idempotency_key),
                false,
                Some(state_version),
                &task_id,
            ),
            invocation(),
        )?;
        let handle: StagedArtifactHandle =
            serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;
        let request_id = format!("req_mcp_user_action_{}_run", case.name);
        let idempotency_key = format!("idem_mcp_user_action_{}_run", case.name);
        let mut request = fixture.record_run_request(
            &request_id,
            &idempotency_key,
            false,
            Some(state_version),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            "artifact_input_mcp_user_action_evidence_observation",
            handle,
            Some("user_action_candidate"),
            None,
        )];
        let recorded = core.record_run(&fixture.mutation_context()?, request, invocation())?;
        state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .ok_or("record_run response should expose state_version")?;
        registered_artifact_id = recorded.response_value["registered_artifacts"][0]["artifact_id"]
            .as_str()
            .map(str::to_owned);
    }

    let question_marker = format!("PRIVATE_{}_QUESTION_MUST_NOT_ESCAPE", case.name);
    let context_marker = format!("PRIVATE_{}_CONTEXT_MUST_NOT_ESCAPE", case.name);
    let option_marker = format!("PRIVATE_{}_OPTION_MUST_NOT_ESCAPE", case.name);
    let mut arguments = match case.kind {
        McpUserActionLeakageCaseKind::Choice {
            required_for,
            sensitive,
            ..
        } => {
            let options = if matches!(case.name, "product_decision" | "technical_decision") {
                json!([
                    {
                        "option_id": "keep",
                        "label": option_marker,
                        "description": "Keep the focused fixture behavior.",
                        "consequence": "Only this fixture action is resolved.",
                        "is_default": true
                    },
                    {
                        "option_id": "change",
                        "label": "Change the focused fixture behavior",
                        "description": "Change only the focused fixture behavior.",
                        "consequence": "Only this fixture action is resolved differently.",
                        "is_default": false
                    }
                ])
            } else {
                Value::Null
            };
            let mut arguments = action_args(
                fixture,
                &task_id,
                state_version,
                case.name,
                options,
                json!(required_for),
            );
            arguments["request"]["change_unit_id"] = json!(change_unit_id);
            arguments["request"]["action"]["question"] = json!(question_marker);
            arguments["request"]["action"]["context"]["summary"] = json!(context_marker);
            if sensitive {
                arguments["request"]["action"]["sensitive_action_scope"] = json!({
                    "action_kind": "mcp_user_action_leakage_fixture",
                    "description": "Authorize only the named fixture-sensitive step.",
                    "intended_paths": ["src/fixture.rs"],
                    "sensitive_categories": ["network"],
                    "command_or_tool_summary": "Run one local fixture command.",
                    "network_or_host_summary": "No remote host is authorized.",
                    "secret_or_credential_summary": null,
                    "capability_claim": "This fixture approval is not a write ticket.",
                    "expires_at": null
                });
            }
            arguments
        }
        McpUserActionLeakageCaseKind::EvidenceObservation => {
            let artifact_id = registered_artifact_id
                .ok_or("evidence-observation setup must register an artifact")?;
            let mut arguments = evidence_observation_action_args(
                &task_id,
                &change_unit_id,
                vec![json!({
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": criterion_id
                })],
                vec![artifact_id],
            );
            arguments["detail"] = json!("full");
            arguments["request"]["action"]["question"] = json!(question_marker);
            arguments["request"]["action"]["context_summary"] = json!(context_marker);
            arguments
        }
    };
    arguments["project_selector"] = json!(fixture.project_id());

    Ok(PreparedMcpUserActionLeakageCase {
        task_id,
        arguments,
        private_markers: vec![question_marker, context_marker, option_marker],
    })
}

pub(super) fn action_args(
    fixture: &CoreFixture,
    task_id: &str,
    state_version: u64,
    judgment_kind: &str,
    options: Value,
    required_for: Value,
) -> Value {
    json!({
        "detail": "full",
        "request": {
            "operation": "create",
            "task_id": task_id,
            "change_unit_id": null,
            "action": {
                "action_type": "choice",
                "judgment_kind": judgment_kind,
                "presentation": "short",
                "question": "Choose the focused User Channel test outcome.",
                "options": options,
                "context": {
                    "summary": "A focused test user action needs a user-owned answer.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": ["The answer covers only this pending user action."]
                },
                "affected_refs": [
                    {
                        "record_kind": "task",
                        "record_id": task_id,
                        "project_id": fixture.project_id(),
                        "task_id": task_id,
                        "produced_at_state_version": state_version
                    }
                ],
                "sensitive_action_scope": null
            },
            "required_for": required_for,
            "expires_at": null
        }
    })
}

pub(super) fn json_lines(messages: &[Value]) -> Result<Vec<u8>, serde_json::Error> {
    let mut output = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut output, message)?;
        output.push(b'\n');
    }
    Ok(output)
}

pub(super) fn generated_json_rpc_value(seed: u64, depth: usize) -> Value {
    if depth >= 4 {
        return match seed % 5 {
            0 => Value::Null,
            1 => Value::Bool(seed & 1 == 0),
            2 => json!(seed as i64 - 1_024),
            3 => json!(format!("value-{seed}")),
            _ => json!([seed, seed.wrapping_mul(17)]),
        };
    }

    match seed % 9 {
        0 => Value::Null,
        1 => Value::Bool(seed & 1 == 0),
        2 => json!(seed as i64 - 1_024),
        3 => json!(format!("json-rpc-{seed}")),
        4 => Value::Array(
            (0..(seed as usize % 4))
                .map(|index| {
                    generated_json_rpc_value(
                        seed.wrapping_mul(31).wrapping_add(index as u64),
                        depth + 1,
                    )
                })
                .collect(),
        ),
        5 => json!({
            "jsonrpc": if seed & 1 == 0 { "2.0" } else { "1.0" },
            "id": generated_json_rpc_value(seed.wrapping_add(1), depth + 1),
            "method": generated_json_rpc_value(seed.wrapping_add(2), depth + 1),
            "params": generated_json_rpc_value(seed.wrapping_add(3), depth + 1),
        }),
        6 => json!({
            "jsonrpc": "2.0",
            "id": seed,
            "method": "initialize",
            "params": generated_json_rpc_value(seed.wrapping_mul(7), depth + 1),
        }),
        7 => json!({
            "jsonrpc": "2.0",
            "id": seed,
            "method": "tools/call",
            "params": {
                "name": generated_json_rpc_value(seed.wrapping_add(5), depth + 1),
                "arguments": generated_json_rpc_value(seed.wrapping_add(6), depth + 1),
            },
        }),
        _ => json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": generated_json_rpc_value(seed.wrapping_add(9), depth + 1),
        }),
    }
}

pub(super) fn volicord_response_from_tool(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "{}",
        serde_json::to_string_pretty(response)?
    );
    let structured = response["result"]
        .get("structuredContent")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or("tools/call response should include structured content")?;
    Ok(structured
        .get("method_result")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or(structured))
}

pub(super) fn json_values_for_key<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    fn collect<'a>(value: &'a Value, key: &str, values: &mut Vec<&'a Value>) {
        match value {
            Value::Object(object) => {
                if let Some(value) = object.get(key) {
                    values.push(value);
                }
                for value in object.values() {
                    collect(value, key, values);
                }
            }
            Value::Array(array) => {
                for value in array {
                    collect(value, key, values);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    let mut values = Vec::new();
    collect(value, key, &mut values);
    values
}

pub(super) fn stored_action_record(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<volicord_store::core_pipeline::StoredUserActionRecordSet, Box<dyn Error>> {
    let request_id = response
        .pointer("/agent_workflow_result/user_action_request_summary/user_action_request_id")
        .or_else(|| response.pointer("/user_action_request_summary/user_action_request_id"))
        .and_then(Value::as_str)
        .ok_or("response should include user_action_request_summary.user_action_request_id")?;
    let store = CoreProjectStore::open_read_only(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    let now = store.current_timestamp()?;
    let record = store
        .user_action_history_for_task(&volicord_types::ids::TaskId::new(task_id), &now)?
        .into_iter()
        .find(|record| record.request().user_action_request_id() == request_id)
        .ok_or("stored user-action record should exist")?;
    Ok(record)
}

pub(super) fn tool_names(tools: &[CanonicalToolDefinition]) -> Vec<&'static str> {
    tools
        .iter()
        .map(|tool| tool.id.wire_name())
        .collect::<Vec<_>>()
}

pub(super) fn tool_definition(tool_name: &str) -> CanonicalToolDefinition {
    mcp_tools_for_mode_and_storage(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadWrite,
    )
    .into_iter()
    .find(|tool| tool.id.wire_name() == tool_name)
    .unwrap_or_else(|| panic!("missing tool definition for {tool_name}"))
}

pub(super) fn canonical_example(
    tool_name: &str,
    example_id: &str,
) -> &'static crate::tool_registry::McpToolExample {
    canonical_tool_examples(
        AgentToolId::from_wire_name(tool_name)
            .unwrap_or_else(|_| panic!("unknown canonical tool {tool_name}")),
    )
    .iter()
    .find(|example| example.id == example_id)
    .unwrap_or_else(|| panic!("missing canonical example {example_id} for {tool_name}"))
}

pub(super) fn canonical_example_value(
    tool_name: &str,
    example_id: &str,
) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(
        canonical_example(tool_name, example_id).arguments_json,
    )?)
}

pub(super) fn decode_mcp_arguments_to_value(
    tool_name: &str,
    value: Value,
) -> Result<Value, serde_json::Error> {
    match AgentToolId::from_wire_name(tool_name)
        .unwrap_or_else(|_| panic!("unsupported MCP tool example decoder: {tool_name}"))
    {
        AgentToolId::INTAKE => {
            serde_json::to_value(serde_json::from_value::<McpIntakeArguments>(value)?)
        }
        AgentToolId::UPDATE_SCOPE => {
            serde_json::to_value(serde_json::from_value::<McpUpdateScopeArguments>(value)?)
        }
        AgentToolId::STATUS => {
            serde_json::to_value(serde_json::from_value::<McpStatusArguments>(value)?)
        }
        AgentToolId::GET_OPERATION_RESULT => serde_json::to_value(serde_json::from_value::<
            McpGetOperationResultArguments,
        >(value)?),
        AgentToolId::PREPARE_EVIDENCE_CAPTURE => serde_json::to_value(serde_json::from_value::<
            McpPrepareEvidenceCaptureArguments,
        >(value)?),
        AgentToolId::PREPARE_WRITE => {
            serde_json::to_value(serde_json::from_value::<McpPrepareWriteArguments>(value)?)
        }
        AgentToolId::STAGE_ARTIFACT => {
            serde_json::to_value(serde_json::from_value::<McpStageArtifactArguments>(value)?)
        }
        AgentToolId::RECORD_RUN => {
            serde_json::to_value(serde_json::from_value::<McpRecordRunArguments>(value)?)
        }
        AgentToolId::REQUEST_USER_ACTION => serde_json::to_value(serde_json::from_value::<
            McpRequestUserActionArguments,
        >(value)?),
        AgentToolId::RECONCILE_CHANGES => serde_json::to_value(serde_json::from_value::<
            McpReconcileChangesArguments,
        >(value)?),
        AgentToolId::CHECK_CLOSE => {
            serde_json::to_value(serde_json::from_value::<McpCheckCloseArguments>(value)?)
        }
        AgentToolId::CLOSE_TASK => {
            serde_json::to_value(serde_json::from_value::<McpCloseTaskArguments>(value)?)
        }
        other => panic!("unsupported MCP tool example decoder: {other}"),
    }
}

pub(super) fn structured_tool_error(tool_name: &str, error: &McpAdapterError) -> Value {
    let result = tool_execution_error_result(tool_name, error);
    let parsed = structured_error_result(&result);
    assert_eq!(parsed["tool_name"], tool_name);
    match error {
        McpAdapterError::InvalidParams { .. } => {
            assert_eq!(parsed["code"], "MCP_INVALID_ARGUMENTS");
            assert_eq!(parsed["retryable"], true);
        }
        McpAdapterError::ToolExecution { .. } => {
            assert_eq!(parsed["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
            assert_eq!(parsed["retryable"], false);
        }
        _ => {}
    }
    parsed
}

pub(super) fn structured_error_result(result: &Value) -> Value {
    assert_eq!(result["isError"], true);
    assert!(
        serde_json::to_vec(result)
            .expect("tool error result should serialize")
            .len()
            <= MAX_MCP_TOOL_ERROR_RESULT_BYTES
    );
    let parsed: Value = serde_json::from_str(
        result["content"][0]["text"]
            .as_str()
            .expect("tool error compatibility text"),
    )
    .expect("tool error compatibility text should be JSON");
    assert_eq!(result["structuredContent"], parsed);
    serde_json::from_value::<McpToolErrorResponse>(parsed.clone())
        .expect("structured tool error should match its advertised response type");
    assert_eq!(parsed["reached_core"], false);
    assert_eq!(parsed["committed"], false);
    assert_eq!(
        parsed["reported_issue_count"].as_u64(),
        parsed["issues"]
            .as_array()
            .map(|issues| issues.len() as u64)
    );
    assert!(parsed["truncated"].is_boolean());
    parsed
}

pub(super) fn tool_error_issue<'a>(response: &'a Value, path: &str, code: &str) -> &'a Value {
    response["issues"]
        .as_array()
        .expect("tool error issues should be an array")
        .iter()
        .find(|issue| issue["path"] == path && issue["code"] == code)
        .unwrap_or_else(|| panic!("missing issue {code} at {path}: {response}"))
}

pub(super) fn tool_names_from_list_response(response: &Value) -> Vec<&str> {
    response["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>()
}

pub(super) fn assert_compatible_tool_definitions(tools: &[CanonicalToolDefinition]) {
    if let Err(errors) = validate_tools_list_schema_compatibility(tools) {
        panic!(
            "MCP tool definitions should be client-compatible:\n{}",
            errors.join("\n")
        );
    }
}

pub(super) fn assert_tools_list_json_client_compatible(tools: &[Value]) {
    if let Err(errors) = validate_tools_list_json_compatibility(tools) {
        panic!(
            "MCP tools/list response should be client-compatible:\n{}",
            errors.join("\n")
        );
    }
}

pub(super) fn preflight_report_for_fixture(
    fixture: &CoreFixture,
    project_id: Option<&str>,
) -> Result<McpPreflightReport, Box<dyn Error>> {
    Ok(preflight_check(
        |name| {
            if name == "VOLICORD_HOME" {
                Some(fixture.runtime_home_path().as_os_str().to_owned())
            } else {
                None
            }
        },
        fixture.runtime_home_path(),
        fixture.connection_id(),
        project_id,
    )?)
}

pub(super) fn read_only_state_version(fixture: &CoreFixture) -> Result<u64, Box<dyn Error>> {
    let state_db_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let conn = open_project_state_database_read_only(state_db_path)?;
    Ok(conn.query_row(
        "SELECT state_version FROM project_state WHERE project_id = ?1",
        [fixture.project_id()],
        |row| row.get(0),
    )?)
}

pub(super) fn read_only_table_count(
    fixture: &CoreFixture,
    table: &str,
) -> Result<i64, Box<dyn Error>> {
    let state_db_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let conn = open_project_state_database_read_only(state_db_path)?;
    let sql = format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE project_id = ?1",
        table.replace('"', "\"\"")
    );
    Ok(conn.query_row(&sql, [fixture.project_id()], |row| row.get(0))?)
}

pub(super) fn json_member_exists(value: &Value, member: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(member)
                || object
                    .values()
                    .any(|child| json_member_exists(child, member))
        }
        Value::Array(items) => items.iter().any(|child| json_member_exists(child, member)),
        _ => false,
    }
}

pub(super) fn workflow_metric_row(
    rows: &[WorkflowMetricAggregateRow],
    metric_kind: WorkflowMetricKind,
    method_name: Option<MethodName>,
    outcome: Option<WorkflowMetricOutcome>,
) -> &WorkflowMetricAggregateRow {
    rows.iter()
        .find(|row| {
            row.metric_kind == metric_kind.as_str()
                && row.method_name.as_deref() == method_name.map(MethodName::as_str)
                && row.outcome.as_deref() == outcome.map(WorkflowMetricOutcome::as_str)
        })
        .unwrap_or_else(|| {
            panic!(
                "missing workflow metric row kind={} method={:?} outcome={:?}; rows={rows:?}",
                metric_kind.as_str(),
                method_name.map(MethodName::as_str),
                outcome.map(WorkflowMetricOutcome::as_str),
            )
        })
}

pub(super) fn assert_local_schema_refs_resolve(schema: &Value, tool_name: &str) {
    let definitions = schema.get("definitions").and_then(Value::as_object);
    assert_schema_value_refs_resolve(schema, definitions, tool_name);
}

pub(super) fn assert_schema_value_refs_resolve(
    value: &Value,
    definitions: Option<&Map<String, Value>>,
    tool_name: &str,
) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                let name = reference.strip_prefix("#/definitions/").unwrap_or_else(|| {
                    panic!("{tool_name} has a non-local runtime ref {reference}")
                });
                assert!(
                    definitions.is_some_and(|definitions| definitions.contains_key(name)),
                    "{tool_name} has an unresolved runtime ref {reference}"
                );
            }
            for child in object.values() {
                assert_schema_value_refs_resolve(child, definitions, tool_name);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_schema_value_refs_resolve(child, definitions, tool_name);
            }
        }
        _ => {}
    }
}

pub(super) fn strip_schema_presentation_for_test(value: &mut Value) {
    compact_runtime_schema(value);
}

pub(super) fn root_properties(schema: &Value) -> Vec<String> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

pub(super) fn root_required_fields(schema: &Value) -> Vec<String> {
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

pub(super) fn schema_has_definition(schema: &Value, name: &str) -> bool {
    schema
        .get("definitions")
        .and_then(Value::as_object)
        .is_some_and(|definitions| {
            definitions.keys().any(|definition| {
                definition == name || definition.starts_with(&format!("{name}_for_"))
            })
        })
}

pub(super) fn schema_variant_by_tag<'a>(
    schema: &'a Value,
    tag: &str,
    value: &str,
) -> Option<&'a Value> {
    let tag_schema = schema
        .get("properties")
        .and_then(|properties| properties.get(tag));
    let matches_tag = tag_schema.is_some_and(|tag_schema| {
        tag_schema.get("const").and_then(Value::as_str) == Some(value)
            || tag_schema
                .get("enum")
                .and_then(Value::as_array)
                .is_some_and(|values| values.iter().any(|candidate| candidate == value))
    });
    if matches_tag {
        return Some(schema);
    }
    match schema {
        Value::Array(values) => values
            .iter()
            .find_map(|schema| schema_variant_by_tag(schema, tag, value)),
        Value::Object(object) => object
            .values()
            .find_map(|schema| schema_variant_by_tag(schema, tag, value)),
        _ => None,
    }
}

pub(super) fn schema_requires_property(schema: &Value, field: &str) -> bool {
    if schema
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| properties.contains_key(field))
        && root_required_fields(schema)
            .iter()
            .any(|required| required == field)
    {
        return true;
    }
    match schema {
        Value::Array(values) => values
            .iter()
            .any(|value| schema_requires_property(value, field)),
        Value::Object(object) => object
            .values()
            .any(|value| schema_requires_property(value, field)),
        _ => false,
    }
}

pub(super) fn stdio_responses(output: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
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

pub(super) fn latest_runtime_session_id(fixture: &CoreFixture) -> Result<String, Box<dyn Error>> {
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    Ok(registry.query_row(
        "SELECT runtime_session_id
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
          ORDER BY process_started_at DESC, runtime_session_id DESC
          LIMIT 1",
        [fixture.connection_id()],
        |row| row.get(0),
    )?)
}
