use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};

use crate::{
    guard_integration::{
        files::{
            plan_managed_block_file, plan_managed_json_projection_file, GeneratedFilePlan,
            GUIDANCE_END_MARKER, GUIDANCE_START_MARKER, VOLICORD_POLICY_FILE,
        },
        hooks::{host_hook_command_lines, HostHookCommand, HostHookCommandShape},
        GuardIntegrationError, ManagedJsonProjection,
    },
    host_integration::{
        claude_code,
        contracts::{
            contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
        },
        HostIntegrationFileKind, HostKind, HostLifecyclePhase, ManagedServerEntry,
        REQUIRED_GUARD_PHASES,
    },
};

pub(crate) fn plan_claude_mcp_file(
    repo_root: &Path,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let value = claude_mcp_projection(server_name, entry);
    plan_managed_json_projection_file(
        HostIntegrationFileKind::HostMcpConfig,
        &repo_root.join(".mcp.json"),
        &value,
        ManagedJsonProjection::ClaudeCodeMcpEntry,
    )
}

pub(crate) fn plan_claude_project_settings_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let value = claude_settings_hooks_projection(hook_commands)?;
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    validate_contract_config(
        HostKind::ClaudeCode,
        HostContractConfigKind::ProjectSettings,
        &text,
    )
    .map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "generated Claude Code settings hooks do not match the verified contract: {error}"
        ))
    })?;
    plan_managed_json_projection_file(
        HostIntegrationFileKind::HostHookConfig,
        &claude_code::project_settings_path(repo_root),
        &value,
        ManagedJsonProjection::ClaudeCodeSettingsHooks,
    )
}

pub(crate) fn plan_claude_rule_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let command_lines = host_hook_command_lines(hook_commands);
    let rule_path = claude_code::project_rule_path(repo_root);
    let rule_block = managed_guidance_block(&claude_code::project_rule_block(
        VOLICORD_POLICY_FILE,
        &command_lines,
    ));
    plan_managed_block_file(
        HostIntegrationFileKind::HostRuleInstruction,
        &rule_path,
        &rule_block,
        GUIDANCE_START_MARKER,
        GUIDANCE_END_MARKER,
        true,
    )
}

fn claude_mcp_projection(server_name: &str, entry: &ManagedServerEntry) -> Value {
    let mut servers = serde_json::Map::new();
    servers.insert(server_name.to_owned(), entry.to_json_value());
    let mut root = serde_json::Map::new();
    root.insert("mcpServers".to_owned(), Value::Object(servers));
    Value::Object(root)
}

fn claude_settings_hooks_projection(
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<Value, GuardIntegrationError> {
    let contract = contract_for(HostKind::ClaudeCode).ok_or_else(|| {
        GuardIntegrationError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Claude Code host integration contract is available",
        )
    })?;
    let hooks = REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| {
            let event = hook_event_for_phase(contract, *phase).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "DETECTIVE_HOOKS_UNSUPPORTED: Claude Code contract is missing {} hook event data",
                    phase.capability_name()
                ))
            })?;
            let hook_command = hook_commands.get(phase.policy_key()).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "missing generated hook command for {}",
                    phase.policy_key()
                ))
            })?;
            Ok::<(String, Value), GuardIntegrationError>((
                event.event_name.to_owned(),
                Value::Array(vec![claude_hook_group_value(
                    *phase,
                    event.write_matcher_tokens,
                    hook_command,
                )?]),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    Ok(json!({ "hooks": hooks }))
}

fn claude_hook_group_value(
    phase: HostLifecyclePhase,
    write_matcher_tokens: &[&str],
    command: &HostHookCommand,
) -> Result<Value, GuardIntegrationError> {
    let mut group = serde_json::Map::new();
    if !write_matcher_tokens.is_empty() {
        group.insert(
            "matcher".to_owned(),
            Value::String(write_matcher_tokens.join("|")),
        );
    } else if phase == HostLifecyclePhase::SessionStart {
        group.insert(
            "matcher".to_owned(),
            Value::String("startup|resume".to_owned()),
        );
    }
    group.insert(
        "hooks".to_owned(),
        Value::Array(vec![claude_hook_handler_value(phase, command)?]),
    );
    Ok(Value::Object(group))
}

fn claude_hook_handler_value(
    phase: HostLifecyclePhase,
    command: &HostHookCommand,
) -> Result<Value, GuardIntegrationError> {
    let HostHookCommandShape::Exec { command, args } = &command.generated_command_shape else {
        return Err(GuardIntegrationError::runtime(
            "Claude Code hook command generation requires exec-form command and args",
        ));
    };
    let mut handler = serde_json::Map::new();
    handler.insert("type".to_owned(), Value::String("command".to_owned()));
    handler.insert("command".to_owned(), Value::String(command.clone()));
    handler.insert(
        "args".to_owned(),
        Value::Array(args.iter().cloned().map(Value::String).collect()),
    );
    handler.insert("timeout".to_owned(), Value::Number(30.into()));
    let status_message = match phase {
        HostLifecyclePhase::SessionStart => Some("Checking Volicord session"),
        HostLifecyclePhase::PreTool => Some("Checking Volicord write"),
        HostLifecyclePhase::PostTool => Some("Recording Volicord write"),
        HostLifecyclePhase::UserPromptSubmit | HostLifecyclePhase::Stop => None,
    };
    if let Some(status_message) = status_message {
        handler.insert(
            "statusMessage".to_owned(),
            Value::String(status_message.to_owned()),
        );
    }
    Ok(Value::Object(handler))
}

fn managed_guidance_block(body: &str) -> String {
    format!("{GUIDANCE_START_MARKER}\n{body}{GUIDANCE_END_MARKER}\n")
}
