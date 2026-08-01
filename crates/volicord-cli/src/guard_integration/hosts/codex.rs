use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};
use volicord_host_contract::McpServerKey;
use volicord_types::guard_manifest::GuardManagedArtifact;
use volicord_types::integration_verification::IntegrationVerificationWorkflowState;
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::GuardHookPhase;

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, plan_managed_exact_json_file, GeneratedFilePlan},
        hooks::{
            codex_guard_hook_script, shell_word, HostHookCommand, HostHookCommandShape,
            HostHookPathSafetyContract, HostHookPurpose,
        },
        GuardIntegrationError,
    },
    host_integration::{
        contracts::{
            codex_hook_matcher_strategy, contract_for, hook_event_for_phase,
            validate_contract_config, HostContractConfigKind,
        },
        guard_phase_capability_name, HostKind,
    },
};

const CODEX_RULE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED CODEX RULES";
const CODEX_RULE_END_MARKER: &str = "# END VOLICORD MANAGED CODEX RULES";

pub(crate) fn plan_codex_hook_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
    phases: &[GuardHookPhase],
    server: &McpServerKey,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let contract = contract_for(HostKind::Codex).ok_or_else(|| {
        GuardIntegrationError::runtime(
            "GUARD_HOOKS_UNSUPPORTED: no Codex host integration contract is available",
        )
    })?;
    let hooks = phases
        .iter()
        .map(|phase| {
            let event = hook_event_for_phase(contract, *phase).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "GUARD_HOOKS_UNSUPPORTED: Codex contract is missing {} hook event data",
                    guard_phase_capability_name(*phase)
                ))
            })?;
            let hook_command = hook_commands.get(phase.as_str()).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "missing generated hook command for {}",
                    phase.as_str()
                ))
            })?;
            let mut group = serde_json::Map::new();
            let matcher_strategy = codex_hook_matcher_strategy(event, server)
                .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
            if let Some(matcher_strategy) = matcher_strategy {
                group.insert(
                    "matcher".to_owned(),
                    Value::String(
                        matcher_strategy
                            .codex_matcher()
                            .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?,
                    ),
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
    validate_contract_config(
        HostKind::Codex,
        HostContractConfigKind::HookConfig,
        &text,
        Some(server),
    )
    .map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "generated Codex hook config does not match the verified contract: {error}"
        ))
    })?;
    plan_managed_exact_json_file(
        GuardManagedArtifact::HostHookConfig,
        repo_root,
        &GuardManagedArtifact::HostHookConfig
            .expected_path(repo_root, None)
            .expect("the Guard hook configuration has a repository-owned path"),
        &value,
    )
}

pub(crate) fn plan_codex_rule_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    if hook_commands.len() != GuardHookPhase::REQUIRED.len() {
        return Err(GuardIntegrationError::runtime(
            "Codex rule generation requires the exact Guard hook phases",
        ));
    }
    let mut command_lines = Vec::with_capacity(GuardHookPhase::REQUIRED.len());
    let mut hook_scripts = Vec::with_capacity(GuardHookPhase::REQUIRED.len());
    for phase in GuardHookPhase::REQUIRED {
        let command = hook_commands.get(phase.as_str()).ok_or_else(|| {
            GuardIntegrationError::runtime(
                "Codex rule generation requires the exact Guard hook phases",
            )
        })?;
        if command.host_kind != HostKind::Codex
            || command.phase != phase
            || command.purpose != HostHookPurpose::Guard
        {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires exact Codex Guard hook commands",
            ));
        }
        let HostHookCommandShape::ShellCommandString { command_text, argv } =
            &command.generated_command_shape;
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
        let expected_script = codex_guard_hook_script(phase);
        let expected_command_text = format!("sh -c {}", shell_word(&expected_script));
        if script != &expected_script || command_text != &expected_command_text {
            return Err(GuardIntegrationError::runtime(
                "Codex rule generation requires exact generated hook commands",
            ));
        }
        command_lines.push(command_text);
        hook_scripts.push(script);
    }
    let mut body = format!(
        "# Hook review and trust remain user/host owned.\n\
# Manual stdio and CLI preflight are diagnostic, not managed-host evidence.\n\
# Canonical verification request: Run the Volicord integration verification.\n\
# Agent sequence: call {}, then {}; follow workflow.kind and call the returned workflow.tool once.\n\
# Workflow states: {} uses {} once; {} uses {} once; {} and {} call no verification tool.\n\
# Do not use shell sleep or poll loops, make repeated status calls, or automatically restart the workflow in the same turn.\n\
# Begin, probe, and status expose the same tagged workflow state.\n\
# If tools are unavailable, report managed MCP unavailable; do not synthesize raw stdio or Codex _meta.\n\
# volicord connection verify is optional active diagnostics only; it does not replace the managed-host workflow.\n\
prefix_rule(\n    pattern = [\"sh\", \"-c\", [\n",
        AgentToolId::LIST_PROJECTS.wire_name(),
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
        IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND,
        AgentToolId::GUARD_PROBE.wire_name(),
        IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND,
        AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        IntegrationVerificationWorkflowState::REPAIR_REQUIRED_KIND,
        IntegrationVerificationWorkflowState::COMPLETE_KIND,
    );
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
    validate_contract_config(
        HostKind::Codex,
        HostContractConfigKind::RuleConfig,
        &body,
        None,
    )
    .map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "generated Codex rule config does not match the verified contract: {error}"
        ))
    })?;
    let block = format!("{CODEX_RULE_START_MARKER}\n{body}{CODEX_RULE_END_MARKER}\n");
    plan_managed_block_file(
        GuardManagedArtifact::HostRuleInstruction,
        repo_root,
        &GuardManagedArtifact::HostRuleInstruction
            .expected_path(repo_root, None)
            .expect("the Guard rule instruction has a repository-owned path"),
        &block,
        CODEX_RULE_START_MARKER,
        CODEX_RULE_END_MARKER,
        true,
    )
}

fn codex_hook_handler_value(
    phase: GuardHookPhase,
    command: &HostHookCommand,
) -> Result<Value, GuardIntegrationError> {
    let HostHookCommandShape::ShellCommandString { command_text, .. } =
        &command.generated_command_shape;
    if command.path_safety_contract != HostHookPathSafetyContract::CODEX_GIT_ROOT_RUNTIME {
        return Err(GuardIntegrationError::runtime(
            "Codex hook generation requires the current Git-root runtime path-safety contract",
        ));
    }
    let mut handler = serde_json::Map::new();
    handler.insert("type".to_owned(), Value::String("command".to_owned()));
    handler.insert("command".to_owned(), Value::String(command_text.clone()));
    handler.insert("timeout".to_owned(), Value::Number(30.into()));
    let status_message = match phase {
        GuardHookPhase::PreTool => Some("Checking Volicord write"),
        GuardHookPhase::PostTool => Some("Recording Volicord write"),
        GuardHookPhase::PromptCapture => None,
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
