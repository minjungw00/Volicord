use std::{fmt, path::Path};

use serde_json::Value;
use toml_edit::DocumentMut;
use volicord_host_contract::{HostContractError, HostHookMatcherStrategy, McpServerKey};
use volicord_types::{GuardHookPhase, GuardManagedArtifact};

use super::HostKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostContractConfigKind {
    ProjectConfig,
    HookConfig,
    RuleConfig,
}

impl HostContractConfigKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProjectConfig => "project_config",
            Self::HookConfig => "hook_config",
            Self::RuleConfig => "rule_config",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostHookEventContract {
    pub phase: GuardHookPhase,
    pub event_name: &'static str,
    pub routes_tools: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostHookConfigShape {
    pub events: &'static [HostHookEventContract],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostIntegrationContract {
    pub host_kind: HostKind,
    pub hook_config_shape: HostHookConfigShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostContractValidationError {
    message: String,
}

impl HostContractValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostContractValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HostContractValidationError {}

const CODEX_HOOK_EVENTS: [HostHookEventContract; 3] = [
    HostHookEventContract {
        phase: GuardHookPhase::PreTool,
        event_name: "PreToolUse",
        routes_tools: true,
    },
    HostHookEventContract {
        phase: GuardHookPhase::PostTool,
        event_name: "PostToolUse",
        routes_tools: true,
    },
    HostHookEventContract {
        phase: GuardHookPhase::PromptCapture,
        event_name: "UserPromptSubmit",
        routes_tools: false,
    },
];

pub const CODEX_CONTRACT: HostIntegrationContract = HostIntegrationContract {
    host_kind: HostKind::Codex,
    hook_config_shape: HostHookConfigShape {
        events: &CODEX_HOOK_EVENTS,
    },
};

pub fn contract_for(_host_kind: HostKind) -> Option<&'static HostIntegrationContract> {
    Some(&CODEX_CONTRACT)
}

pub fn hook_event_for_phase(
    contract: &HostIntegrationContract,
    phase: GuardHookPhase,
) -> Option<&'static HostHookEventContract> {
    contract
        .hook_config_shape
        .events
        .iter()
        .find(|event| event.phase == phase)
}

/// Returns typed host-level routing for one Codex hook event.
pub fn codex_hook_matcher_strategy(
    event: &HostHookEventContract,
    server: &McpServerKey,
) -> Result<Option<HostHookMatcherStrategy>, HostContractError> {
    if event.routes_tools {
        HostHookMatcherStrategy::codex_guard(server).map(Some)
    } else {
        Ok(None)
    }
}

pub fn classify_contract_config_path(
    _host_kind: HostKind,
    path: &Path,
) -> Option<HostContractConfigKind> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let file_name = path.file_name()?.to_string_lossy();
    let hook_config_suffix = GuardManagedArtifact::HostHookConfig
        .repository_relative_path()
        .ok()?;
    if ends_with_components(&components, &[".codex", "config.toml"]) {
        Some(HostContractConfigKind::ProjectConfig)
    } else if path.ends_with(hook_config_suffix) {
        Some(HostContractConfigKind::HookConfig)
    } else if components
        .windows(2)
        .any(|window| window == [".codex", "rules"])
        && file_name.ends_with(".rules")
    {
        Some(HostContractConfigKind::RuleConfig)
    } else {
        None
    }
}

pub fn validate_contract_config(
    _host_kind: HostKind,
    kind: HostContractConfigKind,
    text: &str,
    server: Option<&McpServerKey>,
) -> Result<(), HostContractValidationError> {
    match kind {
        HostContractConfigKind::ProjectConfig => validate_codex_project_config(text),
        HostContractConfigKind::HookConfig => validate_codex_hook_config(text, server),
        HostContractConfigKind::RuleConfig => validate_codex_rule_config(text),
    }
}

fn validate_codex_project_config(text: &str) -> Result<(), HostContractValidationError> {
    let document = text.parse::<DocumentMut>().map_err(|error| {
        HostContractValidationError::new(format!("Codex project config must be TOML: {error}"))
    })?;
    let Some(servers) = document.get("mcp_servers").and_then(|item| item.as_table()) else {
        return Err(HostContractValidationError::new(
            "Codex project config must contain [mcp_servers]",
        ));
    };
    for (name, item) in servers {
        let table = item.as_table().ok_or_else(|| {
            HostContractValidationError::new(format!("Codex MCP server {name} must be a table"))
        })?;
        let command = table.get("command").and_then(|item| item.as_str());
        let url = table.get("url").and_then(|item| item.as_str());
        if command.is_some() == url.is_some() {
            return Err(HostContractValidationError::new(format!(
                "Codex MCP server {name} must define exactly one of command or url"
            )));
        }
        if command.is_some()
            && table
                .get("args")
                .is_some_and(|item| item.as_array().is_none())
        {
            return Err(HostContractValidationError::new(format!(
                "Codex MCP server {name} args must be an array"
            )));
        }
    }
    Ok(())
}

fn validate_codex_hook_config(
    text: &str,
    server: Option<&McpServerKey>,
) -> Result<(), HostContractValidationError> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        HostContractValidationError::new(format!("Codex hook config must be JSON: {error}"))
    })?;
    let hooks = value
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| HostContractValidationError::new("Codex hook config must contain hooks"))?;
    if hooks.len() != CODEX_HOOK_EVENTS.len() {
        return Err(HostContractValidationError::new(
            "Codex hook config must contain exactly the record Guard events",
        ));
    }
    for event in CODEX_HOOK_EVENTS {
        let groups = hooks
            .get(event.event_name)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                HostContractValidationError::new(format!(
                    "Codex hook config is missing {}",
                    event.event_name
                ))
            })?;
        let [group] = groups.as_slice() else {
            return Err(HostContractValidationError::new(format!(
                "{} must contain exactly one hook group",
                event.event_name
            )));
        };
        let group = group.as_object().ok_or_else(|| {
            HostContractValidationError::new(format!(
                "{} group must be an object",
                event.event_name
            ))
        })?;
        let matcher_strategy = server
            .map(|server| codex_hook_matcher_strategy(&event, server))
            .transpose()
            .map_err(|error| HostContractValidationError::new(error.to_string()))?
            .flatten();
        if !event.routes_tools {
            if group.contains_key("matcher") {
                return Err(HostContractValidationError::new(format!(
                    "{} must not define a matcher",
                    event.event_name
                )));
            }
        } else {
            let actual = group.get("matcher").and_then(Value::as_str);
            let matches = if let (Some(strategy), Some(server), Some(actual)) =
                (matcher_strategy, server, actual)
            {
                HostHookMatcherStrategy::parse_codex_guard(actual, server)
                    .is_ok_and(|reconstructed| reconstructed == strategy)
            } else {
                false
            };
            if !matches {
                return Err(HostContractValidationError::new(format!(
                    "{} has an unexpected matcher",
                    event.event_name
                )));
            }
        }
        let handlers = group
            .get("hooks")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                HostContractValidationError::new(format!(
                    "{} group must contain hooks",
                    event.event_name
                ))
            })?;
        let [handler] = handlers.as_slice() else {
            return Err(HostContractValidationError::new(format!(
                "{} must contain exactly one command handler",
                event.event_name
            )));
        };
        let handler = handler.as_object().ok_or_else(|| {
            HostContractValidationError::new(format!(
                "{} command handler must be an object",
                event.event_name
            ))
        })?;
        if handler.get("type").and_then(Value::as_str) != Some("command")
            || handler
                .get("command")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            return Err(HostContractValidationError::new(format!(
                "{} must contain a command handler",
                event.event_name
            )));
        }
    }
    Ok(())
}

fn validate_codex_rule_config(text: &str) -> Result<(), HostContractValidationError> {
    let dispatch_path = GuardManagedArtifact::HostHookDispatch
        .repository_relative_path()
        .map_err(|error| HostContractValidationError::new(error.to_string()))?;
    let dispatch_file_name = dispatch_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            HostContractValidationError::new(
                "Codex Guard dispatch artifact must have a UTF-8 file name",
            )
        })?;
    if !text.contains("prefix_rule(")
        || !text.contains("decision = \"prompt\"")
        || !text.contains(dispatch_file_name)
    {
        return Err(HostContractValidationError::new(
            "Codex Guard rule config is missing its managed prefix rule",
        ));
    }
    Ok(())
}

fn ends_with_components(components: &[String], suffix: &[&str]) -> bool {
    components.len() >= suffix.len()
        && components[components.len() - suffix.len()..]
            .iter()
            .zip(suffix)
            .all(|(actual, expected)| actual == expected)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_hook_configuration_path_is_classified_from_the_artifact_spec() {
        let canonical = Path::new("/work/product").join(
            GuardManagedArtifact::HostHookConfig
                .repository_relative_path()
                .unwrap(),
        );
        assert_eq!(
            classify_contract_config_path(HostKind::Codex, &canonical),
            Some(HostContractConfigKind::HookConfig)
        );
        assert_eq!(
            classify_contract_config_path(
                HostKind::Codex,
                Path::new("/work/product/.codex/other-hooks.json")
            ),
            None
        );
    }

    #[test]
    fn codex_rule_validation_requires_the_canonical_dispatch_file_name() {
        let dispatch_path = GuardManagedArtifact::HostHookDispatch
            .repository_relative_path()
            .unwrap();
        let dispatch_file_name = dispatch_path.file_name().unwrap().to_str().unwrap();
        let canonical =
            format!("prefix_rule(pattern = [{dispatch_file_name:?}], decision = \"prompt\")");
        assert!(validate_codex_rule_config(&canonical).is_ok());
        assert!(validate_codex_rule_config(
            "prefix_rule(pattern = [\"another-dispatch.sh\"], decision = \"prompt\")"
        )
        .is_err());
        assert!(validate_codex_rule_config(dispatch_file_name).is_err());
    }

    #[test]
    fn codex_tool_matcher_uses_typed_host_tools_and_server_namespace() {
        let event = hook_event_for_phase(&CODEX_CONTRACT, GuardHookPhase::PreTool).unwrap();
        let server = McpServerKey::parse("volicord").unwrap();
        let strategy = codex_hook_matcher_strategy(event, &server)
            .unwrap()
            .expect("tool event matcher");
        assert_eq!(
            strategy.codex_matcher().unwrap(),
            "Bash|apply_patch|Edit|Write|mcp__volicord__.*"
        );
        assert_eq!(
            HostHookMatcherStrategy::parse_codex_guard(&strategy.codex_matcher().unwrap(), &server)
                .unwrap(),
            strategy
        );
    }

    #[test]
    fn codex_hook_configuration_validation_rejects_matcher_drift() {
        let server = McpServerKey::parse("volicord").unwrap();
        let matcher = HostHookMatcherStrategy::codex_guard(&server)
            .unwrap()
            .codex_matcher()
            .unwrap();
        let command_handler = json!([{"type": "command", "command": "volicord guard hook"}]);
        let canonical = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": matcher,
                    "hooks": command_handler
                }],
                "PostToolUse": [{
                    "matcher": matcher,
                    "hooks": command_handler
                }],
                "UserPromptSubmit": [{
                    "hooks": command_handler
                }]
            }
        });
        assert!(validate_codex_hook_config(&canonical.to_string(), Some(&server)).is_ok());

        let mut drifted = canonical;
        drifted["hooks"]["PostToolUse"][0]["matcher"] =
            Value::String("Bash|apply_patch|Edit|Write|mcp__foreign__.*".to_owned());
        assert!(validate_codex_hook_config(&drifted.to_string(), Some(&server)).is_err());
    }
}
