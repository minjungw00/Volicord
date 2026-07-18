use super::*;

pub(in crate::connection_command) fn repo_file_changes_json(changes: &[RepoFileChange]) -> Value {
    Value::Array(
        changes
            .iter()
            .map(|change| {
                json!({
                    "status": change.status.as_str(),
                    "path": change.path,
                })
            })
            .collect(),
    )
}

pub(in crate::connection_command) fn changed_repo_files_json(changes: &[RepoFileChange]) -> Value {
    Value::Array(
        changes
            .iter()
            .filter(|change| change.status.is_actual())
            .map(|change| {
                json!({
                    "status": change.status.as_str(),
                    "path": change.path,
                })
            })
            .collect(),
    )
}

pub(in crate::connection_command) fn init_checks_json(
    verification: Option<&VerificationReport>,
    guard_status: &str,
    guard_state: &GuardOperationalState,
) -> Value {
    if let Some(report) = verification {
        return serde_json::to_value(report.report.checks()).unwrap_or(Value::Array(Vec::new()));
    }
    let mut checks = Vec::new();
    checks.push(json!({
        "id": "guard_installation",
        "status": if guard_status == "passed" { "passed" } else if guard_status == "failed" { "failed" } else { "pending" },
        "summary": "Codex Record Guard installation status",
    }));
    checks.extend(guard_checks_json_values(guard_state));
    Value::Array(checks)
}
pub(in crate::connection_command) fn connection_states_json(
    connection_state: &str,
    project_registration: &str,
    mcp_config: &str,
    guard_state: &GuardOperationalState,
    host_reload_required: bool,
) -> Value {
    json!({
        "runtime_home": "ready",
        "connection": connection_state,
        "project_registration": project_registration,
        "mcp_config": mcp_config,
        "selected_profile": guard_state.selected_profile(),
        "control_surface": guard_state.control_surface_json(),
        "generated_config_verified": guard_state.generated_config_verified,
        "cooperative_pre_tool_warning_available": guard_state.cooperative_pre_tool_warning_available(),
        "cooperative_pre_tool_denial_available": guard_state.cooperative_pre_tool_denial_available(),
        "post_tool_correlation_available": guard_state.post_tool_correlation_available(),
        "direct_file_write_matcher_coverage": guard_state.direct_file_write_matcher_coverage,
        "bypass_detection_active": guard_state.bypass_detection_active(),
        "prompt_capture_available": guard_state.prompt_capture_available(),
        "guard_installation": &guard_state.installation_state,
        "guard_configuration": &guard_state.configuration_state,
        "guard_observation": &guard_state.observation_state,
        "guard_effective": &guard_state.effective_state,
        "guard_files": &guard_state.files_state,
        "agents_managed_block": &guard_state.agents_block_state,
        "volicord_policy_file": &guard_state.policy_file_state,
        "rule_instruction_config": &guard_state.rule_instruction_state,
        "hook_config": &guard_state.hook_config_state,
        "required_hook_phases": guard_state.required_hook_phases_state(),
        "missing_required_hooks": &guard_state.missing_required_hooks,
        "guard_hook_observed": &guard_state.hook_observed_state,
        "guard_observed": guard_state.guard_observed(),
        "last_guard_observed_at": &guard_state.last_observed_at,
        "last_guard_event_at": &guard_state.last_guard_event_at,
        "prompt_capture": &guard_state.prompt_capture_state,
        "guard_blockers": &guard_state.unresolved_blockers,
        "host_reload_required": host_reload_required,
    })
}

pub(in crate::connection_command) fn actions_json_values(actions: &[UserAction]) -> Value {
    Value::Array(
        actions
            .iter()
            .map(|action| {
                json!({
                    "id": user_action_id(action.kind),
                    "instruction": action.message,
                })
            })
            .collect(),
    )
}

pub(in crate::connection_command) fn checks_json(
    connection: &AgentConnectionRecord,
    verification: Option<&VerificationReport>,
    current_report: Option<&volicord_types::ConnectionVerificationReport>,
    _guard_state: &GuardOperationalState,
) -> Value {
    let report = verification
        .map(|verification| verification.report.clone())
        .or_else(|| current_report.cloned())
        .or_else(|| effective_connection_report(connection).ok());
    let checks = report
        .and_then(|report| serde_json::to_value(report.checks()).ok())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    Value::Array(checks)
}

fn guard_checks_json_values(guard_state: &GuardOperationalState) -> Vec<Value> {
    vec![
        json!({
            "id": "guard_files",
            "status": match guard_state.files_state.as_str() {
                "installed" => "passed",
                "not_configured" => "pending",
                _ => "failed",
            },
            "summary": format!("Codex Record Guard files are {}", guard_state.files_state),
        }),
        json!({
            "id": "guard_hooks",
            "status": if guard_state.host_hook_guard_available() {
                "passed"
            } else if guard_state.missing_required_hooks.is_empty() {
                "pending"
            } else {
                "failed"
            },
            "summary": format!("Codex Record Guard is {}", guard_state.effective_state),
        }),
        json!({
            "id": "prompt_capture",
            "status": match guard_state.prompt_capture_state.as_str() {
                "active" | "observed" | "configured" => "passed",
                "reload_required" => "pending",
                _ => "failed",
            },
            "summary": format!(
                "Codex prompt observation is {}; UserAction resolution is CLI inbox only",
                guard_state.prompt_capture_state
            ),
        }),
    ]
}

pub(in crate::connection_command) fn connection_json(
    connection: &AgentConnectionRecord,
    project_ids: &[String],
    _user_actions: Option<&[UserAction]>,
) -> Value {
    let verification_report = effective_connection_report(connection)
        .and_then(|report| {
            serde_json::to_value(report)
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        })
        .unwrap_or(Value::Null);
    json!({
        "connection_id": connection.connection_internal_id,
        "host_kind": connection.host_kind,
        "connection_intent": connection.intent,
        "host_scope": connection.host_scope,
        "mode": connection.mode,
        "enabled": connection.enabled,
        "connected_projects": project_ids,
        "verification_report": verification_report,
        "metadata_state": persisted_object_state_json(
            &connection.metadata_json,
            PERSISTED_CONNECTION_METADATA_CORRUPT_REASON,
            "recreate_or_repair_the_agent_connection_registration",
        ),
        "server_name": connection.server_name,
        "config_target": connection.config_target,
    })
}

pub(in crate::connection_command) fn verification_json(report: &VerificationReport) -> Value {
    serde_json::to_value(&report.report).unwrap_or(Value::Null)
}
