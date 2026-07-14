use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, plan_managed_exact_json_file, GeneratedFilePlan},
        hooks::{
            codex_detective_hook_script, shell_word, HostHookCommand, HostHookCommandShape,
            HostHookPurpose,
        },
        GuardIntegrationError,
    },
    host_integration::{
        codex,
        contracts::{
            contract_for, hook_event_for_phase, validate_contract_config,
            validate_final_output_contract_config, HostContractConfigKind,
        },
        HostIntegrationFileKind, HostKind, HostLifecyclePhase, FINAL_OUTPUT_PHASES,
        REQUIRED_GUARD_PHASES,
    },
};

const CODEX_RULE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED CODEX RULES";
const CODEX_RULE_END_MARKER: &str = "# END VOLICORD MANAGED CODEX RULES";

pub(crate) fn plan_codex_hook_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
    phases: &[HostLifecyclePhase],
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let contract = contract_for(HostKind::Codex).ok_or_else(|| {
        GuardIntegrationError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Codex host integration contract is available",
        )
    })?;
    let hooks = phases
        .iter()
        .map(|phase| {
            let event = hook_event_for_phase(contract, *phase).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "DETECTIVE_HOOKS_UNSUPPORTED: Codex contract is missing {} hook event data",
                    phase.capability_name()
                ))
            })?;
            let hook_command = hook_commands.get(phase.policy_key()).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "missing generated hook command for {}",
                    phase.policy_key()
                ))
            })?;
            let mut group = serde_json::Map::new();
            if !event.write_matcher_tokens.is_empty() {
                group.insert(
                    "matcher".to_owned(),
                    Value::String(event.write_matcher_tokens.join("|")),
                );
            } else if *phase == HostLifecyclePhase::SessionStart {
                group.insert(
                    "matcher".to_owned(),
                    Value::String("startup|resume".to_owned()),
                );
            }
            group.insert(
                "hooks".to_owned(),
                Value::Array(vec![codex_hook_handler_value(*phase, hook_command)?]),
            );
            Ok::<(String, Value), GuardIntegrationError>((
                event.event_name.to_owned(),
                Value::Array(vec![Value::Object(group)]),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    let value = json!({ "hooks": hooks });
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let validation = if phases == FINAL_OUTPUT_PHASES {
        validate_final_output_contract_config(
            HostKind::Codex,
            HostContractConfigKind::HookConfig,
            &text,
        )
    } else {
        validate_contract_config(HostKind::Codex, HostContractConfigKind::HookConfig, &text)
    };
    validation.map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "generated Codex hook config does not match the verified contract: {error}"
        ))
    })?;
    plan_managed_exact_json_file(
        HostIntegrationFileKind::HostHookConfig,
        repo_root,
        &codex::project_hooks_path(repo_root),
        &value,
    )
}

pub(crate) fn plan_codex_rule_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    if hook_commands.len() != REQUIRED_GUARD_PHASES.len() {
        return Err(GuardIntegrationError::runtime(
            "Codex rule generation requires the exact five detective hook phases",
        ));
    }
    let mut command_lines = Vec::with_capacity(REQUIRED_GUARD_PHASES.len());
    let mut hook_scripts = Vec::with_capacity(REQUIRED_GUARD_PHASES.len());
    for phase in REQUIRED_GUARD_PHASES {
        let command = hook_commands.get(phase.policy_key()).ok_or_else(|| {
            GuardIntegrationError::runtime(
                "Codex rule generation requires the exact five detective hook phases",
            )
        })?;
        if command.host_kind != HostKind::Codex
            || command.phase != phase
            || command.purpose != HostHookPurpose::DetectiveGuard
        {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires exact Codex detective hook commands",
            ));
        }
        let HostHookCommandShape::ShellCommandString { command_text, argv } =
            &command.generated_command_shape
        else {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires command-string form",
            ));
        };
        let [shell, flag, script] = argv.as_slice() else {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires exact sh -c hook argv",
            ));
        };
        if shell != "sh" || flag != "-c" {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires exact sh -c hook argv",
            ));
        }
        let expected_script = codex_detective_hook_script(phase);
        let expected_command_text = format!("sh -c {}", shell_word(&expected_script));
        if script != &expected_script || command_text != &expected_command_text {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires exact generated hook commands",
            ));
        }
        command_lines.push(command_text);
        hook_scripts.push(script);
    }
    let mut body = String::from("prefix_rule(\n    pattern = [\"sh\", \"-c\", [\n");
    for script in hook_scripts {
        body.push_str("        ");
        body.push_str(&starlark_string(script));
        body.push_str(",\n");
    }
    body.push_str(
        "    ]],\n    decision = \"prompt\",\n    justification = \"Volicord hook wrappers record local lifecycle events.\",\n    match = [\n",
    );
    for command in command_lines {
        body.push_str("        ");
        body.push_str(&starlark_string(command));
        body.push_str(",\n");
    }
    body.push_str(
        "    ],\n    not_match = [\n        \"sh -c 'echo unrelated'\",\n        \"volicord status\",\n    ],\n)\n",
    );
    validate_contract_config(HostKind::Codex, HostContractConfigKind::RuleConfig, &body).map_err(
        |error| {
            GuardIntegrationError::runtime(format!(
                "generated Codex rule config does not match the verified contract: {error}"
            ))
        },
    )?;
    let block = format!("{CODEX_RULE_START_MARKER}\n{body}{CODEX_RULE_END_MARKER}\n");
    plan_managed_block_file(
        HostIntegrationFileKind::HostRuleInstruction,
        repo_root,
        &codex::project_rule_path(repo_root),
        &block,
        CODEX_RULE_START_MARKER,
        CODEX_RULE_END_MARKER,
        true,
    )
}

fn codex_hook_handler_value(
    phase: HostLifecyclePhase,
    command: &HostHookCommand,
) -> Result<Value, GuardIntegrationError> {
    let HostHookCommandShape::ShellCommandString { command_text, .. } =
        &command.generated_command_shape
    else {
        return Err(GuardIntegrationError::runtime(
            "Codex hook command generation requires command-string form",
        ));
    };
    let mut handler = serde_json::Map::new();
    handler.insert("type".to_owned(), Value::String("command".to_owned()));
    handler.insert("command".to_owned(), Value::String(command_text.clone()));
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

fn starlark_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
