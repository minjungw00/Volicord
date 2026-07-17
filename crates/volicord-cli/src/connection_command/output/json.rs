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
    let mut checks = Vec::new();
    if let Some(report) = verification {
        checks.extend([
            json!({
                "id": "host",
                "status": report.host.status.as_str(),
                "summary": report.host.details,
            }),
            json!({
                "id": "cli_mcp_preflight",
                "status": report.preflight.status.as_str(),
                "summary": report.preflight.details,
            }),
            json!({
                "id": "cli_mcp_handshake",
                "status": report.handshake.status.as_str(),
                "summary": report.handshake.details,
            }),
        ]);
    }
    checks.push(json!({
        "id": "guard_installation",
        "status": guard_status,
        "summary": "Codex Record Guard installation status",
    }));
    checks.push(json!({
        "id": "guard_files",
        "status": match guard_state.files_state.as_str() {
            "installed" => "passed",
            "planned" | "not_configured" => "skipped",
            _ => "failed",
        },
        "summary": format!("Codex Record Guard files are {}", guard_state.files_state),
    }));
    checks.push(json!({
        "id": "guard_hooks",
        "status": if guard_state.host_hook_guard_available() {
            "passed"
        } else if guard_state.missing_required_hooks.is_empty() {
            "action_required"
        } else {
            "failed"
        },
        "summary": format!(
            "Codex Record Guard effective state is {}",
            guard_state.effective_state
        ),
    }));
    checks.push(json!({
        "id": "prompt_capture",
        "status": match guard_state.prompt_capture_state.as_str() {
            "active" | "observed" | "configured" => "passed",
            "reload_required" => "action_required",
            _ => "failed",
        },
        "summary": format!(
            "Codex prompt observation is {} and UserAction resolution remains CLI inbox only",
            guard_state.prompt_capture_state
        ),
    }));
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
    current_host: Option<&Verification>,
    guard_state: &GuardOperationalState,
) -> Value {
    let persisted_user_actions = decode_persisted_user_actions(&connection.last_user_actions_json);
    if let Some(verification) = verification {
        let mut checks = vec![json!({
            "id": "host",
            "status": verification.host.status.as_str(),
            "summary": verification.host.details,
            "details": {
                "host_state": verification.host.host_state.as_str(),
                "managed_config": verification.host.managed_config.as_str(),
                "host_executable": verification.host.host_executable.as_str(),
                "host_gate": verification.host.host_gate.as_str(),
                "host_configuration": verification.host.host_configuration.as_str(),
                "host_policy_overlay": &verification.host.host_policy_overlay,
            }
        })];
        checks.extend(host_diagnostic_checks_json(&verification.host));
        checks.extend([
            json!({
                "id": "cli_mcp_preflight",
                "status": verification.preflight.status.as_str(),
                "summary": verification.preflight.details,
            }),
            json!({
                "id": "cli_mcp_handshake",
                "status": verification.handshake.status.as_str(),
                "summary": verification.handshake.details,
            }),
        ]);
        checks.extend(preflight_storage_checks_json(
            verification.preflight.preflight_diagnostics.as_ref(),
        ));
        checks.extend(guard_checks_json_values(guard_state));
        checks.push(persisted_user_actions_check_json(&persisted_user_actions));
        return Value::Array(checks);
    }
    let mut checks = stored_checks_json(connection, current_host);
    checks.extend(guard_checks_json_values(guard_state));
    checks.push(persisted_user_actions_check_json(&persisted_user_actions));
    Value::Array(checks)
}

fn guard_checks_json_values(guard_state: &GuardOperationalState) -> Vec<Value> {
    vec![
        json!({
            "id": "guard_files",
            "status": match guard_state.files_state.as_str() {
                "installed" => "passed",
                "not_configured" => "skipped",
                _ => "failed",
            },
            "summary": format!("Codex Record Guard files are {}", guard_state.files_state),
        }),
        json!({
            "id": "guard_hooks",
            "status": if guard_state.host_hook_guard_available() {
                "passed"
            } else if guard_state.missing_required_hooks.is_empty() {
                "action_required"
            } else {
                "failed"
            },
            "summary": format!("Codex Record Guard is {}", guard_state.effective_state),
        }),
        json!({
            "id": "prompt_capture",
            "status": match guard_state.prompt_capture_state.as_str() {
                "active" | "observed" | "configured" => "passed",
                "reload_required" => "action_required",
                _ => "failed",
            },
            "summary": format!(
                "Codex prompt observation is {}; UserAction resolution is CLI inbox only",
                guard_state.prompt_capture_state
            ),
        }),
    ]
}

fn stored_checks_json(
    connection: &AgentConnectionRecord,
    current_host: Option<&Verification>,
) -> Vec<Value> {
    let report = json_object_text(&connection.last_verification_report_json);
    let Some(object) = report.as_object() else {
        return current_host
            .map(host_diagnostic_checks_json)
            .unwrap_or_default();
    };
    let mut checks = Vec::new();
    if let Some(host) = object.get("host").and_then(Value::as_object) {
        checks.push(json!({
            "id": "host",
            "status": host.get("status").and_then(Value::as_str).unwrap_or("not_verified"),
            "summary": host
                .get("details")
                .and_then(Value::as_str)
                .unwrap_or("stored host verification state"),
            "details": host,
        }));
    }
    if let Some(host) = current_host {
        checks.extend(host_diagnostic_checks_json(host));
    } else {
        checks.extend(stored_host_diagnostic_checks_json(object));
    }
    if let Some(preflight) = object.get("cli_mcp_preflight").and_then(Value::as_object) {
        checks.push(json!({
            "id": "cli_mcp_preflight",
            "status": preflight.get("status").and_then(Value::as_str).unwrap_or("skipped"),
            "summary": preflight
                .get("details")
                .and_then(Value::as_str)
                .unwrap_or("stored CLI MCP preflight state"),
        }));
    }
    if let Some(handshake) = object.get("cli_mcp_handshake").and_then(Value::as_object) {
        checks.push(json!({
            "id": "cli_mcp_handshake",
            "status": handshake.get("status").and_then(Value::as_str).unwrap_or("skipped"),
            "summary": handshake
                .get("details")
                .and_then(Value::as_str)
                .unwrap_or("stored CLI MCP handshake state"),
        }));
    }
    checks.extend(preflight_storage_checks_json(
        stored_preflight_diagnostics_from_report(object).as_ref(),
    ));
    checks
}

fn preflight_storage_checks_json(diagnostics: Option<&McpPreflightDiagnostics>) -> Vec<Value> {
    let Some(diagnostics) = diagnostics else {
        return Vec::new();
    };
    vec![
        json!({
            "id": "cli_mcp_storage_read",
            "status": storage_read_check_status(&diagnostics.storage_read),
            "summary": format!("CLI MCP storage read: {}", diagnostic_value_text(&diagnostics.storage_read)),
            "details": {
                "source": "cli_mcp_preflight",
                "preflight_field": "project_state_read",
                "value": &diagnostics.storage_read,
            },
        }),
        json!({
            "id": "cli_mcp_storage_write",
            "status": storage_write_check_status(
                &diagnostics.storage_write,
                &diagnostics.effective_tool_mode,
            ),
            "summary": format!("CLI MCP storage write: {}", diagnostic_value_text(&diagnostics.storage_write)),
            "details": {
                "source": "cli_mcp_preflight",
                "preflight_field": "project_state_write",
                "value": &diagnostics.storage_write,
            },
        }),
        json!({
            "id": "cli_mcp_effective_tools",
            "status": effective_tool_mode_check_status(&diagnostics.effective_tool_mode),
            "summary": format!("CLI MCP effective tools: {}", diagnostic_value_text(&diagnostics.effective_tool_mode)),
            "details": {
                "source": "cli_mcp_preflight",
                "preflight_field": "effective_tool_mode",
                "value": &diagnostics.effective_tool_mode,
            },
        }),
    ]
}

fn host_diagnostic_checks_json(host: &Verification) -> Vec<Value> {
    let mut checks = Vec::new();
    if let Some(overlay) = &host.host_policy_overlay {
        checks.push(json!({
            "id": "codex_tool_approval_policy",
            "status": if overlay.accepted { "passed" } else { "failed" },
            "summary": overlay.details,
            "details": overlay,
        }));
    }
    if let Some(trust) = &host.project_trust {
        checks.push(json!({
            "id": "codex_project_trust",
            "status": project_trust_check_status(trust.status),
            "summary": trust.details,
            "details": trust,
        }));
    }
    if let Some(runtime) = &host.host_runtime {
        checks.extend(managed_host_runtime_checks_json(runtime));
    }
    if let Some(command) = &host.host_mcp_command {
        checks.push(json!({
            "id": "host_mcp_command",
            "status": host_mcp_command_check_status(command),
            "summary": command.details,
            "details": command,
        }));
    }
    checks
}

fn managed_host_runtime_checks_json(runtime: &HostRuntimeDiagnostic) -> Vec<Value> {
    let mut checks = vec![
        json!({
            "id": "managed_host_startup",
            "status": host_runtime_check_status(runtime.managed_host_startup),
            "summary": format!(
                "Managed Codex MCP startup: {}",
                host_runtime_text(runtime.managed_host_startup)
            ),
            "details": {
                "value": runtime.managed_host_startup.as_str(),
                "source": "managed_codex_lifecycle",
                "last_observed_at": &runtime.last_observed_at,
            },
        }),
        json!({
            "id": "managed_host_tools_list",
            "status": host_runtime_check_status(runtime.managed_host_tools_list),
            "summary": format!(
                "Managed Codex tools/list: {}",
                host_runtime_text(runtime.managed_host_tools_list)
            ),
            "details": {
                "value": runtime.managed_host_tools_list.as_str(),
                "source": "managed_codex_lifecycle",
                "last_observed_at": &runtime.last_observed_at,
            },
        }),
        json!({
            "id": "managed_host_tool_call",
            "status": host_runtime_check_status(runtime.managed_host_tool_call),
            "summary": format!(
                "Managed Codex tool call: {}",
                host_runtime_text(runtime.managed_host_tool_call)
            ),
            "details": {
                "value": runtime.managed_host_tool_call.as_str(),
                "source": "managed_codex_lifecycle",
                "last_observed_at": &runtime.last_observed_at,
            },
        }),
        json!({
            "id": "active_tool_exposure",
            "status": active_tool_exposure_check_status(runtime.active_tool_exposure),
            "summary": format!(
                "Active Codex tool exposure: {}",
                active_tool_exposure_text(runtime.active_tool_exposure)
            ),
            "details": {
                "value": runtime.active_tool_exposure.as_str(),
                "source": "managed_codex_tool_call",
                "last_observed_at": &runtime.last_observed_at,
            },
        }),
    ];
    if let Some(storage) = &runtime.managed_host_storage {
        checks.extend(managed_host_storage_checks_json(storage));
    }
    checks
}

fn managed_host_storage_checks_json(storage: &ManagedHostStorageDiagnostic) -> Vec<Value> {
    vec![
        json!({
            "id": "managed_host_storage_read",
            "status": storage_read_check_status(&storage.storage_read),
            "summary": format!(
                "Managed host storage read: {}",
                diagnostic_value_text(&storage.storage_read)
            ),
            "details": {
                "source": "managed_codex_lifecycle",
                "source_lifecycle_event": &storage.source_lifecycle_event,
                "observed_at": &storage.observed_at,
                "value": &storage.storage_read,
            },
        }),
        json!({
            "id": "managed_host_storage_write",
            "status": storage_write_check_status(
                &storage.storage_write,
                &storage.effective_tool_mode,
            ),
            "summary": format!(
                "Managed host storage write: {}",
                diagnostic_value_text(&storage.storage_write)
            ),
            "details": {
                "source": "managed_codex_lifecycle",
                "source_lifecycle_event": &storage.source_lifecycle_event,
                "observed_at": &storage.observed_at,
                "value": &storage.storage_write,
            },
        }),
        json!({
            "id": "managed_host_effective_tools",
            "status": effective_tool_mode_check_status(&storage.effective_tool_mode),
            "summary": format!(
                "Managed host effective tools: {}",
                diagnostic_value_text(&storage.effective_tool_mode)
            ),
            "details": {
                "source": "managed_codex_lifecycle",
                "source_lifecycle_event": &storage.source_lifecycle_event,
                "observed_at": &storage.observed_at,
                "value": &storage.effective_tool_mode,
            },
        }),
    ]
}

fn stored_host_diagnostic_checks_json(object: &serde_json::Map<String, Value>) -> Vec<Value> {
    let host = object.get("host").and_then(Value::as_object);
    let project_trust = object
        .get("project_trust")
        .or_else(|| host.and_then(|host| host.get("project_trust")));
    let host_runtime = object
        .get("host_runtime")
        .or_else(|| host.and_then(|host| host.get("host_runtime")));
    let host_mcp_command = object
        .get("host_mcp_command")
        .or_else(|| host.and_then(|host| host.get("host_mcp_command")));
    let host_policy_overlay = object
        .get("host_policy_overlay")
        .or_else(|| host.and_then(|host| host.get("host_policy_overlay")));
    let mut checks = Vec::new();
    if let Some(overlay) = host_policy_overlay.and_then(Value::as_object) {
        let accepted = overlay
            .get("accepted")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        checks.push(json!({
            "id": "codex_tool_approval_policy",
            "status": if accepted { "passed" } else { "failed" },
            "summary": overlay.get("details").and_then(Value::as_str).unwrap_or("stored Codex tool approval policy state"),
            "details": overlay,
        }));
    }
    if let Some(trust) = project_trust.and_then(Value::as_object) {
        let status = trust
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        checks.push(json!({
            "id": "codex_project_trust",
            "status": stored_project_trust_check_status(status),
            "summary": trust.get("details").and_then(Value::as_str).unwrap_or("stored Codex project trust state"),
            "details": trust,
        }));
    }
    if let Some(runtime) = host_runtime.and_then(Value::as_object) {
        checks.extend(stored_managed_host_runtime_checks_json(runtime));
    }
    if let Some(command) = host_mcp_command.and_then(Value::as_object) {
        checks.push(json!({
            "id": "host_mcp_command",
            "status": if command.get("risk").is_some_and(|risk| !risk.is_null()) {
                "warning"
            } else {
                "passed"
            },
            "summary": command.get("details").and_then(Value::as_str).unwrap_or("stored host MCP command state"),
            "details": command,
        }));
    }
    checks
}

fn stored_managed_host_runtime_checks_json(runtime: &serde_json::Map<String, Value>) -> Vec<Value> {
    let startup = stored_runtime_status_field(runtime, "managed_host_startup");
    let tools_list = stored_runtime_status_field(runtime, "managed_host_tools_list");
    let tool_call = stored_runtime_status_field(runtime, "managed_host_tool_call");
    let active_exposure = stored_active_tool_exposure_value(runtime, tool_call);
    let last_observed_at = runtime
        .get("last_observed_at")
        .cloned()
        .unwrap_or(Value::Null);
    let mut checks = vec![
        json!({
            "id": "managed_host_startup",
            "status": stored_host_runtime_check_status(startup),
            "summary": format!("Managed Codex MCP startup: {}", startup.replace('_', " ")),
            "details": {
                "value": startup,
                "source": "managed_codex_lifecycle",
                "last_observed_at": &last_observed_at,
            },
        }),
        json!({
            "id": "managed_host_tools_list",
            "status": stored_host_runtime_check_status(tools_list),
            "summary": format!("Managed Codex tools/list: {}", tools_list.replace('_', " ")),
            "details": {
                "value": tools_list,
                "source": "managed_codex_lifecycle",
                "last_observed_at": &last_observed_at,
            },
        }),
        json!({
            "id": "managed_host_tool_call",
            "status": stored_host_runtime_check_status(tool_call),
            "summary": format!("Managed Codex tool call: {}", tool_call.replace('_', " ")),
            "details": {
                "value": tool_call,
                "source": "managed_codex_lifecycle",
                "last_observed_at": &last_observed_at,
            },
        }),
        json!({
            "id": "active_tool_exposure",
            "status": stored_active_tool_exposure_check_status(active_exposure),
            "summary": format!("Active Codex tool exposure: {}", active_exposure.replace('_', " ")),
            "details": {
                "value": active_exposure,
                "source": "managed_codex_tool_call",
                "last_observed_at": &last_observed_at,
            },
        }),
    ];
    if let Some(storage) = runtime
        .get("managed_host_storage")
        .and_then(Value::as_object)
    {
        checks.extend(stored_managed_host_storage_checks_json(storage));
    }
    checks
}

fn stored_runtime_status_field<'a>(
    runtime: &'a serde_json::Map<String, Value>,
    field: &str,
) -> &'a str {
    runtime
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}

fn stored_active_tool_exposure_value<'a>(
    runtime: &'a serde_json::Map<String, Value>,
    tool_call: &'a str,
) -> &'a str {
    runtime
        .get("active_tool_exposure")
        .and_then(Value::as_str)
        .unwrap_or(match tool_call {
            "observed" => "confirmed",
            "unknown" => "unknown",
            _ => "unconfirmed",
        })
}

fn stored_managed_host_storage_checks_json(storage: &serde_json::Map<String, Value>) -> Vec<Value> {
    let storage_read = storage
        .get("storage_read")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let storage_write = storage
        .get("storage_write")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let effective_tool_mode = storage
        .get("effective_tool_mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let source_lifecycle_event = storage
        .get("source_lifecycle_event")
        .cloned()
        .unwrap_or(Value::Null);
    let observed_at = storage.get("observed_at").cloned().unwrap_or(Value::Null);
    vec![
        json!({
            "id": "managed_host_storage_read",
            "status": storage_read_check_status(storage_read),
            "summary": format!(
                "Managed host storage read: {}",
                diagnostic_value_text(storage_read)
            ),
            "details": {
                "source": "managed_codex_lifecycle",
                "source_lifecycle_event": &source_lifecycle_event,
                "observed_at": &observed_at,
                "value": storage_read,
            },
        }),
        json!({
            "id": "managed_host_storage_write",
            "status": storage_write_check_status(storage_write, effective_tool_mode),
            "summary": format!(
                "Managed host storage write: {}",
                diagnostic_value_text(storage_write)
            ),
            "details": {
                "source": "managed_codex_lifecycle",
                "source_lifecycle_event": &source_lifecycle_event,
                "observed_at": &observed_at,
                "value": storage_write,
            },
        }),
        json!({
            "id": "managed_host_effective_tools",
            "status": effective_tool_mode_check_status(effective_tool_mode),
            "summary": format!(
                "Managed host effective tools: {}",
                diagnostic_value_text(effective_tool_mode)
            ),
            "details": {
                "source": "managed_codex_lifecycle",
                "source_lifecycle_event": &source_lifecycle_event,
                "observed_at": &observed_at,
                "value": effective_tool_mode,
            },
        }),
    ]
}

fn project_trust_check_status(status: ProjectTrustStatus) -> &'static str {
    match status {
        ProjectTrustStatus::Trusted => "passed",
        ProjectTrustStatus::Untrusted => "action_required",
        ProjectTrustStatus::Missing | ProjectTrustStatus::Unknown => "unknown",
        ProjectTrustStatus::Unreadable | ProjectTrustStatus::Malformed => "failed",
    }
}

fn host_runtime_check_status(status: HostRuntimeObservationStatus) -> &'static str {
    match status {
        HostRuntimeObservationStatus::Observed => "passed",
        HostRuntimeObservationStatus::NotObserved => "action_required",
        HostRuntimeObservationStatus::Unknown => "unknown",
    }
}

fn active_tool_exposure_check_status(status: ActiveToolExposureStatus) -> &'static str {
    match status {
        ActiveToolExposureStatus::Confirmed => "passed",
        ActiveToolExposureStatus::Unconfirmed => "action_required",
        ActiveToolExposureStatus::Unknown => "unknown",
    }
}

fn stored_project_trust_check_status(status: &str) -> &'static str {
    match status {
        "trusted" => "passed",
        "untrusted" => "action_required",
        "missing" | "unknown" => "unknown",
        "unreadable" | "malformed" => "failed",
        _ => "unknown",
    }
}

fn stored_host_runtime_check_status(status: &str) -> &'static str {
    match status {
        "observed" => "passed",
        "not_observed" => "action_required",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

fn stored_active_tool_exposure_check_status(status: &str) -> &'static str {
    match status {
        "confirmed" => "passed",
        "unconfirmed" => "action_required",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

pub(in crate::connection_command) fn connection_json(
    connection: &AgentConnectionRecord,
    project_ids: &[String],
    user_actions: Option<&[UserAction]>,
) -> Value {
    let persisted_user_actions = decode_persisted_user_actions(&connection.last_user_actions_json);
    let user_actions = user_actions
        .map(|actions| serde_json::to_value(actions).unwrap_or_else(|_| json!([])))
        .or_else(|| {
            persisted_user_actions
                .actions()
                .and_then(|actions| serde_json::to_value(actions).ok())
        })
        .unwrap_or(Value::Null);
    let verification_report =
        decode_persisted_object(&connection.last_verification_report_json).unwrap_or(Value::Null);
    json!({
        "connection_id": connection.connection_internal_id,
        "host_kind": connection.host_kind,
        "connection_intent": connection.intent,
        "host_scope": connection.host_scope,
        "mode": connection.mode,
        "enabled": connection.enabled,
        "connected_projects": project_ids,
        "verification_status": connection.last_verification_status,
        "verification_report": verification_report,
        "verification_report_state": persisted_object_state_json(
            &connection.last_verification_report_json,
            PERSISTED_VERIFICATION_REPORT_CORRUPT_REASON,
            "connection_verify_regenerates_current_typed_values",
        ),
        "user_actions": user_actions,
        "user_actions_state": persisted_user_actions.state_json(),
        "metadata_state": persisted_object_state_json(
            &connection.metadata_json,
            PERSISTED_CONNECTION_METADATA_CORRUPT_REASON,
            "recreate_or_repair_the_agent_connection_registration",
        ),
        "server_name": connection.server_name,
        "config_target": connection.config_target,
    })
}

pub(in crate::connection_command) fn json_object_text(text: &str) -> Value {
    decode_persisted_object(text).unwrap_or(Value::Null)
}

pub(in crate::connection_command) fn verification_json(report: &VerificationReport) -> Value {
    json!({
        "status": report.status.as_str(),
        "host_verification_receipt": &report.receipt,
        "disclosure": cooperative_host_decision_disclosure_json(),
        "project_trust": &report.host.project_trust,
        "host_policy_overlay": &report.host.host_policy_overlay,
        "host_runtime": &report.host.host_runtime,
        "managed_host_startup": runtime_observation_json(
            report.host.host_runtime.as_ref(),
            |runtime| runtime.managed_host_startup,
        ),
        "managed_host_tools_list": runtime_observation_json(
            report.host.host_runtime.as_ref(),
            |runtime| runtime.managed_host_tools_list,
        ),
        "managed_host_tool_call": runtime_observation_json(
            report.host.host_runtime.as_ref(),
            |runtime| runtime.managed_host_tool_call,
        ),
        "active_tool_exposure": active_tool_exposure_json(report.host.host_runtime.as_ref()),
        "host_mcp_command": &report.host.host_mcp_command,
        "host": {
            "status": report.host.status.as_str(),
            "host_state": report.host.host_state.as_str(),
            "host_version": &report.host.host_version,
            "managed_config": report.host.managed_config.as_str(),
            "host_executable": report.host.host_executable.as_str(),
            "host_gate": report.host.host_gate.as_str(),
            "host_configuration": report.host.host_configuration.as_str(),
            "host_policy_overlay": &report.host.host_policy_overlay,
            "project_trust": &report.host.project_trust,
            "host_runtime": &report.host.host_runtime,
            "host_mcp_command": &report.host.host_mcp_command,
            "mcp_handshake_allowed": report.host.mcp_handshake_allowed,
            "details": report.host.details,
            "diagnostic": report.host.diagnostic,
            "failure_category": report.host.failure_category,
            "failure_reason": report.host.failure_reason,
            "managed_host_binding": report.host.managed_host_evidence.as_ref().map(|evidence| &evidence.binding),
            "binding_digest": report.host.managed_host_evidence.as_ref().map(|evidence| &evidence.binding_digest),
            "generated_artifacts_digest": report.host.managed_host_evidence.as_ref().map(|evidence| &evidence.generated_artifacts_digest),
            "user_actions": report.host.user_actions,
        },
        "cli_mcp_preflight": step_json(&report.preflight),
        "cli_mcp_handshake": step_json(&report.handshake),
        "tools": report.tools,
    })
}

fn runtime_observation_json(
    runtime: Option<&HostRuntimeDiagnostic>,
    field: fn(&HostRuntimeDiagnostic) -> HostRuntimeObservationStatus,
) -> Value {
    runtime
        .map(|runtime| json!(field(runtime).as_str()))
        .unwrap_or(Value::Null)
}

fn active_tool_exposure_json(runtime: Option<&HostRuntimeDiagnostic>) -> Value {
    runtime
        .map(|runtime| json!(runtime.active_tool_exposure.as_str()))
        .unwrap_or(Value::Null)
}

pub(in crate::connection_command) fn detailed_verification_report_json(
    report: &VerificationReport,
) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(&verification_json(report))
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn step_json(step: &VerificationStep) -> Value {
    let mut value = json!({
        "status": step.status.as_str(),
        "details": step.details,
    });
    if let Some(diagnostics) = &step.preflight_diagnostics {
        value
            .as_object_mut()
            .expect("step JSON should be an object")
            .insert("diagnostics".to_owned(), diagnostics.to_json());
    }
    value
}
