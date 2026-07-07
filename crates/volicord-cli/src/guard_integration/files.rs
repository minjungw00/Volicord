use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{
    guard_integration::audit::{
        is_volicord_codex_hook_config, script_is_executable, ManagedJsonProjection,
        HOOK_WRAPPER_MARKER,
    },
    host_integration::{
        contracts::{
            contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
        },
        HostIntegrationFileKind, HostKind, HostLifecyclePhase, REQUIRED_GUARD_PHASES,
    },
    managed_block::{self, ManagedBlockError},
};

use super::GuardIntegrationError;

pub(crate) const VOLICORD_POLICY_SCHEMA: &str = "volicord-policy-v1";
pub(crate) const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
pub(crate) const AGENTS_FILE: &str = "AGENTS.md";
pub(crate) const GUIDANCE_START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE v1 -->";
pub(crate) const GUIDANCE_END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE v1 -->";

#[derive(Debug, Clone)]
pub(crate) struct GeneratedFilePlan {
    pub(crate) kind: HostIntegrationFileKind,
    pub(crate) path: PathBuf,
    pub(crate) content: String,
    pub(crate) status: FilePlanStatus,
    pub(crate) write_kind: GeneratedFileWriteKind,
}

impl GeneratedFilePlan {
    pub(crate) fn policy_value(&self) -> Result<Value, GuardIntegrationError> {
        serde_json::from_str::<Value>(&self.content)
            .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeneratedFileWriteKind {
    Block {
        start_marker: &'static str,
        end_marker: &'static str,
        require_existing_marker: bool,
    },
    Json,
    ExactJson,
    JsonProjection {
        projection: ManagedJsonProjection,
    },
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilePlanStatus {
    PlannedCreate,
    PlannedUpdate,
    Unchanged,
    Created,
    Updated,
}

impl FilePlanStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PlannedCreate => "planned_create",
            Self::PlannedUpdate => "planned_update",
            Self::Unchanged => "unchanged",
            Self::Created => "created",
            Self::Updated => "updated",
        }
    }
}

pub(crate) fn plan_managed_block_file(
    kind: HostIntegrationFileKind,
    path: &Path,
    block: &str,
    start_marker: &'static str,
    end_marker: &'static str,
    require_existing_marker: bool,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let content = block.to_owned();
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            if require_existing_marker && !existing.contains(start_marker) {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists without a Volicord-managed block: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
            let updated = managed_block::apply_managed_block_with_markers(
                &existing,
                &content,
                start_marker,
                end_marker,
            )
            .map_err(managed_block_conflict)?;
            if updated == existing {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::Block {
            start_marker,
            end_marker,
            require_existing_marker,
        },
    })
}

pub(crate) fn plan_policy_file(
    path: &Path,
    policy: &Value,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let mut content = serde_json::to_string_pretty(policy)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    content.push('\n');
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            let value = serde_json::from_str::<Value>(&existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing policy file is not valid JSON: {} ({error})",
                    path.display()
                ))
            })?;
            if !is_volicord_policy(&value) {
                return Err(GuardIntegrationError::runtime(format!(
                    "policy file already exists without Volicord ownership metadata: {}",
                    path.display()
                )));
            }
            if existing == content {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind: HostIntegrationFileKind::VolicordPolicy,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::Json,
    })
}

pub(crate) fn plan_managed_exact_json_file(
    kind: HostIntegrationFileKind,
    path: &Path,
    value: &Value,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    content.push('\n');
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            let existing_value = serde_json::from_str::<Value>(&existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing {} is not valid JSON: {} ({error})",
                    kind.as_str(),
                    path.display()
                ))
            })?;
            if existing_value == *value {
                if existing == content {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if kind == HostIntegrationFileKind::HostHookConfig
                && is_volicord_codex_hook_config(&existing_value)
            {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::ExactJson,
    })
}

pub(crate) fn plan_managed_json_projection_file(
    kind: HostIntegrationFileKind,
    path: &Path,
    value: &Value,
    projection: ManagedJsonProjection,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let mut content = canonical_json_text(value)?;
    content.push('\n');
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            let existing_value = serde_json::from_str::<Value>(&existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing {} is not valid JSON: {} ({error})",
                    kind.as_str(),
                    path.display()
                ))
            })?;
            let merged = managed_json_projection_merge(&existing_value, value, projection)?;
            if merged == existing_value {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::JsonProjection { projection },
    })
}

pub(crate) fn plan_managed_script_file(
    path: &Path,
    content: &str,
    kind: HostIntegrationFileKind,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            if existing == content {
                if script_is_executable(path) {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if existing.contains(HOOK_WRAPPER_MARKER) {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content: content.to_owned(),
        status,
        write_kind: GeneratedFileWriteKind::Script,
    })
}

pub(crate) fn managed_json_projection_merge(
    current: &Value,
    desired: &Value,
    projection: ManagedJsonProjection,
) -> Result<Value, GuardIntegrationError> {
    let merged = match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => {
            merge_claude_settings_hooks(current, desired)
        }
        ManagedJsonProjection::ClaudeCodeMcpEntry => merge_claude_mcp_entry(current, desired),
    }?;
    validate_managed_json_projection_config(projection, &merged)?;
    Ok(merged)
}

pub(crate) fn managed_block_conflict(error: ManagedBlockError) -> GuardIntegrationError {
    match error {
        ManagedBlockError::Unterminated { start_marker } => GuardIntegrationError::runtime(
            format!("managed block starting with {start_marker} is missing its end marker"),
        ),
        ManagedBlockError::Duplicate { start_marker } => GuardIntegrationError::runtime(format!(
            "multiple managed blocks starting with {start_marker} were found"
        )),
    }
}

fn canonical_json_text(value: &Value) -> Result<String, GuardIntegrationError> {
    serde_json::to_string(value).map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

fn validate_managed_json_projection_config(
    projection: ManagedJsonProjection,
    value: &Value,
) -> Result<(), GuardIntegrationError> {
    let text = serde_json::to_string(value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let (kind, label) = match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => (
            HostContractConfigKind::ProjectSettings,
            "merged Claude Code project settings",
        ),
        ManagedJsonProjection::ClaudeCodeMcpEntry => (
            HostContractConfigKind::McpConfig,
            "merged Claude Code MCP config",
        ),
    };
    validate_contract_config(HostKind::ClaudeCode, kind, &text).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "{label} do not match the verified contract: {error}"
        ))
    })
}

fn merge_claude_mcp_entry(
    current: &Value,
    desired: &Value,
) -> Result<Value, GuardIntegrationError> {
    let mut object = current.as_object().cloned().ok_or_else(|| {
        GuardIntegrationError::runtime("Claude Code .mcp.json must be a JSON object")
    })?;
    let desired_servers = desired
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| GuardIntegrationError::runtime("managed MCP projection is invalid"))?;
    let servers = object
        .entry("mcpServers".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            GuardIntegrationError::runtime("Claude Code .mcp.json mcpServers must be an object")
        })?;
    for (name, entry) in desired_servers {
        servers.insert(name.clone(), entry.clone());
    }
    Ok(Value::Object(object))
}

fn merge_claude_settings_hooks(
    current: &Value,
    desired: &Value,
) -> Result<Value, GuardIntegrationError> {
    let mut root = current.as_object().cloned().ok_or_else(|| {
        GuardIntegrationError::runtime("Claude Code settings must be a JSON object")
    })?;
    let desired_hooks = desired
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("managed Claude Code hook projection is invalid")
        })?;
    let hooks = root
        .entry("hooks".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            GuardIntegrationError::runtime("Claude Code settings hooks must be an object")
        })?;
    for phase in REQUIRED_GUARD_PHASES {
        let event_name = claude_event_name(phase)?;
        let desired_groups = desired_hooks
            .get(event_name)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "managed Claude Code hook projection is missing {event_name}"
                ))
            })?;
        let desired_group = desired_groups.first().cloned().ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "managed Claude Code hook projection has no {event_name} group"
            ))
        })?;
        let desired_handler = claude_managed_group_signature(&desired_group, event_name)?;
        let existing_groups = hooks
            .remove(event_name)
            .map(|value| {
                value.as_array().cloned().ok_or_else(|| {
                    GuardIntegrationError::runtime(format!(
                        "Claude Code settings hook event {event_name} must be an array"
                    ))
                })
            })
            .transpose()?
            .unwrap_or_default();
        let mut preserved_groups = Vec::new();
        for group in existing_groups {
            if let Some(group) =
                remove_claude_managed_handlers(phase, event_name, &desired_handler, group)?
            {
                preserved_groups.push(group);
            }
        }
        preserved_groups.push(desired_group);
        hooks.insert(event_name.to_owned(), Value::Array(preserved_groups));
    }
    Ok(Value::Object(root))
}

fn remove_claude_managed_handlers(
    phase: HostLifecyclePhase,
    event_name: &str,
    desired_handler: &ClaudeHookHandlerSignature,
    group: Value,
) -> Result<Option<Value>, GuardIntegrationError> {
    let mut object = group.as_object().cloned().ok_or_else(|| {
        GuardIntegrationError::runtime(format!(
            "Claude Code settings hook group for {event_name} must be an object"
        ))
    })?;
    let handlers = object
        .remove("hooks")
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "Claude Code settings hook group for {event_name} must contain hooks"
            ))
        })?
        .as_array()
        .cloned()
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "Claude Code settings hook handlers for {event_name} must be an array"
            ))
        })?;
    let mut kept = Vec::new();
    let mut removed = 0usize;
    for handler in handlers {
        if is_exact_claude_managed_handler(&handler, desired_handler)
            || is_legacy_claude_managed_handler(phase, &handler)
        {
            removed += 1;
        } else if looks_like_conflicting_claude_managed_handler(phase, &handler, desired_handler) {
            return Err(GuardIntegrationError::runtime(format!(
                "Claude Code settings contain a conflicting Volicord-managed {event_name} hook entry"
            )));
        } else {
            kept.push(handler);
        }
    }
    if removed == 0 {
        object.insert("hooks".to_owned(), Value::Array(kept));
        return Ok(Some(Value::Object(object)));
    }
    if kept.is_empty() {
        return Ok(None);
    }
    object.insert("hooks".to_owned(), Value::Array(kept));
    Ok(Some(Value::Object(object)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaudeHookHandlerSignature {
    command: String,
    args: Option<Vec<String>>,
}

fn claude_managed_group_signature(
    group: &Value,
    event_name: &str,
) -> Result<ClaudeHookHandlerSignature, GuardIntegrationError> {
    let handler = group
        .get("hooks")
        .and_then(Value::as_array)
        .and_then(|handlers| handlers.first())
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "managed Claude Code hook projection is missing {event_name} handler"
            ))
        })?;
    let command = handler
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "managed Claude Code hook projection is missing {event_name} command"
            ))
        })?;
    let args = match handler.get("args") {
        Some(value) => {
            let values = value.as_array().ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "managed Claude Code hook projection has non-array {event_name} args"
                ))
            })?;
            Some(
                values
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        GuardIntegrationError::runtime(format!(
                            "managed Claude Code hook projection has non-string {event_name} args"
                        ))
                    })?,
            )
        }
        None => None,
    };
    Ok(ClaudeHookHandlerSignature { command, args })
}

fn is_exact_claude_managed_handler(handler: &Value, desired: &ClaudeHookHandlerSignature) -> bool {
    handler.as_object().is_some_and(|object| {
        object.get("type").and_then(Value::as_str) == Some("command")
            && object
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| command == desired.command)
            && hook_handler_args(object) == desired.args
    })
}

fn is_legacy_claude_managed_handler(phase: HostLifecyclePhase, handler: &Value) -> bool {
    handler.as_object().is_some_and(|object| {
        object.get("type").and_then(Value::as_str) == Some("command")
            && object
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| {
                    let legacy_direct = command
                        .contains(&format!("volicord _hook {}", phase.command_name()))
                        && command.contains("--connection")
                        && command.contains("--guard-installation")
                        && (command.contains("--host claude-code")
                            || command.contains("--host claude_code"))
                        && (command.contains("--host-output claude-code")
                            || command.contains("--host-output claude_code"));
                    let legacy_wrapper = command.contains(&format!(
                        ".claude/hooks/volicord-{}.sh",
                        phase.command_name()
                    ));
                    legacy_direct || legacy_wrapper
                })
    })
}

fn looks_like_conflicting_claude_managed_handler(
    phase: HostLifecyclePhase,
    handler: &Value,
    desired: &ClaudeHookHandlerSignature,
) -> bool {
    handler.as_object().is_some_and(|object| {
        object
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| {
                (command != desired.command || hook_handler_args(object) != desired.args)
                    && ((command.contains("volicord _hook")
                        && command.contains(phase.command_name())
                        && (command.contains("--host claude-code")
                            || command.contains("--host claude_code")
                            || command.contains("--guard-installation")))
                        || command.contains(&format!(
                            ".claude/hooks/volicord-{}.sh",
                            phase.command_name()
                        )))
            })
    })
}

fn hook_handler_args(object: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    object
        .get("args")
        .and_then(Value::as_array)
        .and_then(|args| {
            args.iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
}

fn claude_event_name(phase: HostLifecyclePhase) -> Result<&'static str, GuardIntegrationError> {
    let contract = contract_for(HostKind::ClaudeCode).ok_or_else(|| {
        GuardIntegrationError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Claude Code host integration contract is available",
        )
    })?;
    hook_event_for_phase(contract, phase)
        .map(|event| event.event_name)
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "DETECTIVE_HOOKS_UNSUPPORTED: Claude Code contract is missing {} hook event data",
                phase.capability_name()
            ))
        })
}

fn is_volicord_policy(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(VOLICORD_POLICY_SCHEMA)
        && value.get("managed_by").and_then(Value::as_str) == Some("volicord")
}
