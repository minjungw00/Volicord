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
        let mut checks = vec![
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
            json!({
                "id": "guard_installation",
                "status": guard_status,
                "summary": "detective installation status was recorded",
            }),
        ];
        checks.extend(guard_checks_json_values(guard_state));
        Value::Array(checks)
    } else {
        let mut checks = vec![json!({
            "id": "init_plan",
            "status": "passed",
            "summary": "init plan was built without writing files or Runtime Home records"
        })];
        checks.extend(guard_checks_json_values(guard_state));
        Value::Array(checks)
    }
}

fn guard_checks_json_values(guard_state: &GuardOperationalState) -> Vec<Value> {
    let detective_hooks_applicable = guard_state.detective_hooks_applicable();
    let files_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_files_installed",
            "status": "skipped",
            "summary": "detective host-hook files are not applicable for the record profile",
        })
    } else {
        match guard_state.files_state.as_str() {
            "installed" => json!({
                "id": "guard_files_installed",
                "status": "passed",
                "summary": "detective host-hook files are installed",
            }),
            "missing" => json!({
                "id": "guard_files_installed",
                "status": "failed",
                "summary": "detective host-hook files are missing",
                "details": guard_file_details_json(guard_state),
            }),
            "stale" => json!({
                "id": "guard_files_installed",
                "status": "failed",
                "summary": "detective host-hook files are stale",
                "details": guard_file_details_json(guard_state),
            }),
            "broken" => json!({
                "id": "guard_files_installed",
                "status": "failed",
                "summary": "detective host-hook files are broken",
                "details": guard_file_details_json(guard_state),
            }),
            "disabled" => json!({
                "id": "guard_files_installed",
                "status": "skipped",
                "summary": "host hook files are disabled for record profile",
            }),
            other => json!({
                "id": "guard_files_installed",
                "status": "skipped",
                "summary": format!("detective host-hook files are {other}"),
            }),
        }
    };
    let reload_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_host_reload_required",
            "status": "skipped",
            "summary": "detective host reload is not applicable for the record profile",
        })
    } else if guard_state.installation_state == "reload_required" {
        json!({
            "id": "guard_host_reload_required",
            "status": "failed",
            "summary": "host reload is required before detective host hooks are active",
        })
    } else {
        json!({
            "id": "guard_host_reload_required",
            "status": "passed",
            "summary": "host reload is not currently required by detective installation state",
        })
    };
    let hook_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_hook_observed",
            "status": "skipped",
            "summary": "detective host-hook observation is not applicable for the record profile",
        })
    } else {
        match guard_state.hook_observed_state.as_str() {
            "observed" => json!({
                "id": "guard_hook_observed",
                "status": "passed",
                "summary": "detective host hook has been observed",
                "details": {
                    "last_observed_at": &guard_state.last_observed_at,
                    "last_guard_event_at": &guard_state.last_guard_event_at,
                },
            }),
            "not_observed" => json!({
                "id": "guard_hook_observed",
                "status": "failed",
                "summary": "detective host hook has not been observed",
                "details": {
                    "last_observed_at": Value::Null,
                    "last_guard_event_at": &guard_state.last_guard_event_at,
                },
            }),
            other => json!({
                "id": "guard_hook_observed",
                "status": "skipped",
                "summary": format!("detective host-hook observation is {other}"),
            }),
        }
    };
    let status_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_status_active",
            "status": "skipped",
            "summary": "detective signal active status is not applicable for the record profile",
        })
    } else if guard_state.effective_state == "active" {
        json!({
            "id": "guard_status_active",
            "status": "passed",
            "summary": "effective detective signal status is active",
        })
    } else {
        json!({
            "id": "guard_status_active",
            "status": "failed",
            "summary": format!("effective detective signal status is {}", guard_state.effective_state),
            "details": {
                "installation_status": &guard_state.installation_state,
                "configuration_health": &guard_state.configuration_state,
                "observation_health": &guard_state.observation_state,
                "effective_health": &guard_state.effective_state,
                "missing_required_hooks": &guard_state.missing_required_hooks,
                "unresolved_blockers": &guard_state.unresolved_blockers,
            },
        })
    };
    let capability_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_required_hooks_supported",
            "status": "skipped",
            "summary": "detective host-hook capabilities are not applicable for the record profile",
        })
    } else if guard_state.missing_required_hooks.is_empty() {
        json!({
            "id": "guard_required_hooks_supported",
            "status": "passed",
            "summary": "required detective host-hook capabilities are supported",
        })
    } else {
        json!({
            "id": "guard_required_hooks_supported",
            "status": "failed",
            "summary": "required detective host-hook capabilities are missing",
            "details": {
                "missing_required_hooks": &guard_state.missing_required_hooks,
            },
        })
    };
    let prompt_capture_check = if !detective_hooks_applicable {
        json!({
            "id": "prompt_capture_available",
            "status": "skipped",
            "summary": "prompt capture is not applicable for the record profile",
        })
    } else {
        match guard_state.prompt_capture_state.as_str() {
            "active" | "observed" | "configured" => json!({
                "id": "prompt_capture_available",
                "status": "passed",
                "summary": format!("prompt capture is {}", guard_state.prompt_capture_state),
            }),
            "reload_required" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "prompt capture needs host reload",
            }),
            "unsupported_by_host" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "host does not support prompt capture",
            }),
            "not_configured" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "prompt capture is not configured",
            }),
            "degraded" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "prompt capture is degraded",
            }),
            other => json!({
                "id": "prompt_capture_available",
                "status": "skipped",
                "summary": format!("prompt capture is {other}"),
            }),
        }
    };
    vec![
        files_check,
        reload_check,
        hook_check,
        capability_check,
        status_check,
        prompt_capture_check,
    ]
}

fn guard_file_details_json(guard_state: &GuardOperationalState) -> Value {
    json!({
        "missing_files": &guard_state.missing_files,
        "stale_files": &guard_state.stale_files,
        "broken_files": &guard_state.broken_files,
        "missing_required_hooks": &guard_state.missing_required_hooks,
        "hook_path_safety": &guard_state.hook_path_safety_state,
        "hook_path_safety_details": &guard_state.hook_path_safety_details,
    })
}

pub(in crate::connection_command) fn connection_states_json(
    connection_state: &str,
    project_registration: &str,
    mcp_config: &str,
    guard_state: &GuardOperationalState,
    host_feature_diagnostic: ConnectionHostFeatureDiagnostics,
    host_reload_required: bool,
) -> Value {
    let guard_files_state = if guard_state.detective_hooks_applicable() {
        guard_state.files_state.as_str()
    } else {
        "disabled"
    };
    let mut states = json!({
        "runtime_home": "ready",
        "connection": connection_state,
        "project_registration": project_registration,
        "mcp_config": mcp_config,
        "selected_profile": guard_state.selected_profile(),
        "control_surface": guard_state.control_surface_json(),
        "generated_config_verified": guard_state.generated_config_verified,
        "native_host_output_adapter_config_verified": guard_state.native_host_output_adapter_config_verified,
        "host_feature_support": host_feature_diagnostic.host_feature_support_json(),
        "final_output_authority_disclosure": host_feature_diagnostic.final_output_authority_disclosure_json(),
        "cooperative_pre_tool_warning_available": guard_state.cooperative_pre_tool_warning_available(),
        "cooperative_pre_tool_denial_available": guard_state.cooperative_pre_tool_denial_available(),
        "post_tool_correlation_available": guard_state.post_tool_correlation_available(),
        "bash_shell_mutation_coverage": guard_state.bash_shell_mutation_coverage,
        "direct_file_write_matcher_coverage": guard_state.direct_file_write_matcher_coverage,
        "bypass_detection_active": guard_state.bypass_detection_active(),
        "prompt_capture_available": guard_state.prompt_capture_available(),
        "local_web_consent_available": false,
        "guard_installation": &guard_state.installation_state,
        "guard_configuration": &guard_state.configuration_state,
        "guard_observation": &guard_state.observation_state,
        "guard_effective": &guard_state.effective_state,
        "guard_files": guard_files_state,
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
    });
    if let Some(object) = states.as_object_mut() {
        object.insert(
            "hook_path_safety".to_owned(),
            Value::String(guard_state.hook_path_safety_state.clone()),
        );
        object.insert(
            "hook_commands_cwd_independent".to_owned(),
            Value::Bool(guard_state.hook_commands_cwd_independent),
        );
        object.insert(
            "hook_commands_subdirectory_safe".to_owned(),
            Value::Bool(guard_state.hook_commands_subdirectory_safe),
        );
    }
    states
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
        return Value::Array(checks);
    }
    let mut checks = stored_checks_json(connection, current_host);
    checks.extend(guard_checks_json_values(guard_state));
    Value::Array(checks)
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
    let user_actions = user_actions
        .map(|actions| serde_json::to_value(actions).unwrap_or_else(|_| json!([])))
        .unwrap_or_else(|| json_array_text(&connection.last_user_actions_json));
    json!({
        "connection_id": connection.connection_internal_id,
        "host_kind": connection.host_kind,
        "connection_intent": connection.intent,
        "host_scope": connection.host_scope,
        "mode": connection.mode,
        "enabled": connection.enabled,
        "connected_projects": project_ids,
        "verification_status": connection.last_verification_status,
        "verification_report": json_object_text(&connection.last_verification_report_json),
        "user_actions": user_actions,
        "server_name": connection.server_name,
        "config_target": connection.config_target,
    })
}

pub(in crate::connection_command) fn json_object_text(text: &str) -> Value {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn json_array_text(text: &str) -> Value {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]))
}

pub(in crate::connection_command) fn verification_json(report: &VerificationReport) -> Value {
    json!({
        "status": report.status.as_str(),
        "disclosure": detective_observation_disclosure_json(),
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
