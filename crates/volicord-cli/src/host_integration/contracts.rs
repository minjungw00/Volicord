use std::{fmt, path::Path, str::FromStr};

use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table};

use super::{HostKind, HostLifecyclePhase, HostScope, REQUIRED_GUARD_PHASES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractSupportStatus {
    Verified,
    Unverified,
    Unsupported,
    Disabled,
}

impl ContractSupportStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
            Self::Unsupported => "unsupported",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractCapability {
    pub status: ContractSupportStatus,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostConfigFormat {
    Json,
    Toml,
    Starlark,
    Markdown,
}

impl HostConfigFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Starlark => "starlark",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostContractConfigKind {
    ProjectConfig,
    ProjectSettings,
    McpConfig,
    HookConfig,
    RuleConfig,
}

impl HostContractConfigKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProjectConfig => "project_config",
            Self::ProjectSettings => "project_settings",
            Self::McpConfig => "mcp_config",
            Self::HookConfig => "hook_config",
            Self::RuleConfig => "rule_config",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostConfigPath {
    pub scope: HostScope,
    pub path_pattern: &'static str,
    pub kind: HostContractConfigKind,
    pub format: HostConfigFormat,
    pub shareable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostMcpConfigShape {
    pub format: HostConfigFormat,
    pub root_key: &'static str,
    pub server_name_path: &'static str,
    pub stdio_command_field: &'static str,
    pub stdio_args_field: &'static str,
    pub stdio_env_field: &'static str,
    pub http_url_field: Option<&'static str>,
    pub unknown_fields_policy: UnknownFieldsPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostHookConfigShape {
    pub format: HostConfigFormat,
    pub root_key: &'static str,
    pub command_handler_type: &'static str,
    pub events: &'static [HostHookEventContract],
    pub unknown_fields_policy: UnknownFieldsPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostRuleConfigShape {
    pub format: HostConfigFormat,
    pub path_pattern: &'static str,
    pub status: ContractSupportStatus,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostHookEventContract {
    pub phase: HostLifecyclePhase,
    pub event_name: &'static str,
    pub write_matcher_tokens: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownFieldsPolicy {
    PreserveOrIgnoreUnlessManagedFieldConflicts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostRequirement {
    pub key: &'static str,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostIntegrationContract {
    pub host_kind: HostKind,
    pub supported_config_paths: &'static [HostConfigPath],
    pub mcp_config_shape: HostMcpConfigShape,
    pub hook_config_shape: HostHookConfigShape,
    pub rule_config_shape: Option<HostRuleConfigShape>,
    pub supported_lifecycle_phases: &'static [HostLifecyclePhase],
    pub required_lifecycle_phases: &'static [HostLifecyclePhase],
    pub optional_lifecycle_phases: &'static [HostLifecyclePhase],
    pub prompt_capture: ContractCapability,
    pub reload_restart_trust_requirements: &'static [HostRequirement],
    pub managed_mode_support: ContractCapability,
    pub managed_distribution_source: Option<&'static str>,
    pub full_guarded_adapter_support: ContractCapability,
    pub known_limitations: &'static [&'static str],
    pub official_sources: &'static [&'static str],
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

const CODEX_PATHS: [HostConfigPath; 5] = [
    HostConfigPath {
        scope: HostScope::User,
        path_pattern: "~/.codex/config.toml",
        kind: HostContractConfigKind::ProjectConfig,
        format: HostConfigFormat::Toml,
        shareable: false,
    },
    HostConfigPath {
        scope: HostScope::Project,
        path_pattern: "<repo>/.codex/config.toml",
        kind: HostContractConfigKind::ProjectConfig,
        format: HostConfigFormat::Toml,
        shareable: true,
    },
    HostConfigPath {
        scope: HostScope::User,
        path_pattern: "~/.codex/hooks.json",
        kind: HostContractConfigKind::HookConfig,
        format: HostConfigFormat::Json,
        shareable: false,
    },
    HostConfigPath {
        scope: HostScope::Project,
        path_pattern: "<repo>/.codex/hooks.json",
        kind: HostContractConfigKind::HookConfig,
        format: HostConfigFormat::Json,
        shareable: true,
    },
    HostConfigPath {
        scope: HostScope::Project,
        path_pattern: "<repo>/.codex/rules/*.rules",
        kind: HostContractConfigKind::RuleConfig,
        format: HostConfigFormat::Starlark,
        shareable: true,
    },
];

const CLAUDE_CODE_PATHS: [HostConfigPath; 6] = [
    HostConfigPath {
        scope: HostScope::User,
        path_pattern: "~/.claude/settings.json",
        kind: HostContractConfigKind::ProjectSettings,
        format: HostConfigFormat::Json,
        shareable: false,
    },
    HostConfigPath {
        scope: HostScope::Project,
        path_pattern: "<repo>/.claude/settings.json",
        kind: HostContractConfigKind::ProjectSettings,
        format: HostConfigFormat::Json,
        shareable: true,
    },
    HostConfigPath {
        scope: HostScope::Local,
        path_pattern: "<repo>/.claude/settings.local.json",
        kind: HostContractConfigKind::ProjectSettings,
        format: HostConfigFormat::Json,
        shareable: false,
    },
    HostConfigPath {
        scope: HostScope::Project,
        path_pattern: "<repo>/.mcp.json",
        kind: HostContractConfigKind::McpConfig,
        format: HostConfigFormat::Json,
        shareable: true,
    },
    HostConfigPath {
        scope: HostScope::Project,
        path_pattern: "<repo>/.claude/rules/*.md",
        kind: HostContractConfigKind::RuleConfig,
        format: HostConfigFormat::Markdown,
        shareable: true,
    },
    HostConfigPath {
        scope: HostScope::User,
        path_pattern: "~/.claude.json",
        kind: HostContractConfigKind::McpConfig,
        format: HostConfigFormat::Json,
        shareable: false,
    },
];

const CODEX_WRITE_MATCHERS: [&str; 3] = ["apply_patch", "Edit", "Write"];
const CLAUDE_WRITE_MATCHERS: [&str; 3] = ["Edit", "Write", "MultiEdit"];

const CODEX_HOOK_EVENTS: [HostHookEventContract; 5] = [
    HostHookEventContract {
        phase: HostLifecyclePhase::SessionStart,
        event_name: "SessionStart",
        write_matcher_tokens: &[],
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::PreTool,
        event_name: "PreToolUse",
        write_matcher_tokens: &CODEX_WRITE_MATCHERS,
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::PostTool,
        event_name: "PostToolUse",
        write_matcher_tokens: &CODEX_WRITE_MATCHERS,
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::UserPromptSubmit,
        event_name: "UserPromptSubmit",
        write_matcher_tokens: &[],
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::Stop,
        event_name: "Stop",
        write_matcher_tokens: &[],
    },
];

const CLAUDE_CODE_HOOK_EVENTS: [HostHookEventContract; 5] = [
    HostHookEventContract {
        phase: HostLifecyclePhase::SessionStart,
        event_name: "SessionStart",
        write_matcher_tokens: &[],
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::PreTool,
        event_name: "PreToolUse",
        write_matcher_tokens: &CLAUDE_WRITE_MATCHERS,
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::PostTool,
        event_name: "PostToolUse",
        write_matcher_tokens: &CLAUDE_WRITE_MATCHERS,
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::UserPromptSubmit,
        event_name: "UserPromptSubmit",
        write_matcher_tokens: &[],
    },
    HostHookEventContract {
        phase: HostLifecyclePhase::Stop,
        event_name: "Stop",
        write_matcher_tokens: &[],
    },
];

const CODEX_REQUIREMENTS: [HostRequirement; 3] = [
    HostRequirement {
        key: "project_trust",
        detail: "Project .codex config, project-local hooks, and project-local rules load only after the project is trusted.",
    },
    HostRequirement {
        key: "hook_trust",
        detail: "Non-managed command hooks must be reviewed and trusted before they run.",
    },
    HostRequirement {
        key: "restart",
        detail: "Rules are scanned at startup; restart or reload Codex after adding rule files.",
    },
];

const CLAUDE_CODE_REQUIREMENTS: [HostRequirement; 3] = [
    HostRequirement {
        key: "project_mcp_approval",
        detail: "Project-scoped .mcp.json servers require user approval before Claude Code uses them.",
    },
    HostRequirement {
        key: "project_trust",
        detail: "Some project and local settings are honored only after workspace trust is accepted.",
    },
    HostRequirement {
        key: "reload",
        detail: "Claude Code watches settings files and reloads hooks, permissions, and related settings during a session.",
    },
];

const CODEX_LIMITATIONS: [&str; 4] = [
    "PreToolUse and PostToolUse are documented guardrails, not complete enforcement boundaries for every tool path.",
    "Codex project-local hooks require project trust and separate hook trust before running.",
    "Codex rules are documented as experimental and are not a Volicord full-guarded implementation by themselves.",
    "AGENTS.md and .volicord/policy.json remain guidance and Volicord metadata, not host hook configuration.",
];

const CLAUDE_CODE_LIMITATIONS: [&str; 4] = [
    "Project-scoped .mcp.json servers require user approval before they are available.",
    "Hook if filters and tool hooks are not a complete replacement for host permissions.",
    "Project and user settings files are rejected as a whole when strict JSON validation fails.",
    "AGENTS.md and .volicord/policy.json remain guidance and Volicord metadata, not host hook configuration.",
];

const CODEX_SOURCES: [&str; 3] = [
    "https://developers.openai.com/codex/mcp",
    "https://developers.openai.com/codex/hooks",
    "https://developers.openai.com/codex/rules",
];

const CLAUDE_CODE_SOURCES: [&str; 3] = [
    "https://code.claude.com/docs/en/settings",
    "https://code.claude.com/docs/en/mcp",
    "https://code.claude.com/docs/en/hooks",
];

pub const CODEX_CONTRACT: HostIntegrationContract = HostIntegrationContract {
    host_kind: HostKind::Codex,
    supported_config_paths: &CODEX_PATHS,
    mcp_config_shape: HostMcpConfigShape {
        format: HostConfigFormat::Toml,
        root_key: "mcp_servers",
        server_name_path: "[mcp_servers.<name>]",
        stdio_command_field: "command",
        stdio_args_field: "args",
        stdio_env_field: "env",
        http_url_field: Some("url"),
        unknown_fields_policy: UnknownFieldsPolicy::PreserveOrIgnoreUnlessManagedFieldConflicts,
    },
    hook_config_shape: HostHookConfigShape {
        format: HostConfigFormat::Json,
        root_key: "hooks",
        command_handler_type: "command",
        events: &CODEX_HOOK_EVENTS,
        unknown_fields_policy: UnknownFieldsPolicy::PreserveOrIgnoreUnlessManagedFieldConflicts,
    },
    rule_config_shape: Some(HostRuleConfigShape {
        format: HostConfigFormat::Starlark,
        path_pattern: "<repo>/.codex/rules/*.rules",
        status: ContractSupportStatus::Verified,
        detail: "Codex rules are project-local Starlark files under .codex/rules, but remain experimental.",
    }),
    supported_lifecycle_phases: &REQUIRED_GUARD_PHASES,
    required_lifecycle_phases: &REQUIRED_GUARD_PHASES,
    optional_lifecycle_phases: &[],
    prompt_capture: ContractCapability {
        status: ContractSupportStatus::Verified,
        detail: "UserPromptSubmit includes the submitted prompt.",
    },
    reload_restart_trust_requirements: &CODEX_REQUIREMENTS,
    managed_mode_support: ContractCapability {
        status: ContractSupportStatus::Unsupported,
        detail: "No verified Codex plugin or managed configuration bundle distribution contract is recorded; project-local Codex MCP, hook, policy, and rule files are guarded setup, not managed mode.",
    },
    managed_distribution_source: None,
    full_guarded_adapter_support: ContractCapability {
        status: ContractSupportStatus::Verified,
        detail: "The Codex adapter generates and verifies project-local hook commands for every required guarded lifecycle phase.",
    },
    known_limitations: &CODEX_LIMITATIONS,
    official_sources: &CODEX_SOURCES,
};

pub const CLAUDE_CODE_CONTRACT: HostIntegrationContract = HostIntegrationContract {
    host_kind: HostKind::ClaudeCode,
    supported_config_paths: &CLAUDE_CODE_PATHS,
    mcp_config_shape: HostMcpConfigShape {
        format: HostConfigFormat::Json,
        root_key: "mcpServers",
        server_name_path: "mcpServers.<name>",
        stdio_command_field: "command",
        stdio_args_field: "args",
        stdio_env_field: "env",
        http_url_field: Some("url"),
        unknown_fields_policy: UnknownFieldsPolicy::PreserveOrIgnoreUnlessManagedFieldConflicts,
    },
    hook_config_shape: HostHookConfigShape {
        format: HostConfigFormat::Json,
        root_key: "hooks",
        command_handler_type: "command",
        events: &CLAUDE_CODE_HOOK_EVENTS,
        unknown_fields_policy: UnknownFieldsPolicy::PreserveOrIgnoreUnlessManagedFieldConflicts,
    },
    rule_config_shape: Some(HostRuleConfigShape {
        format: HostConfigFormat::Markdown,
        path_pattern: "<repo>/.claude/rules/*.md",
        status: ContractSupportStatus::Verified,
        detail: "Claude Code loads .claude/rules/*.md instruction files; this is instruction config, not a hook.",
    }),
    supported_lifecycle_phases: &REQUIRED_GUARD_PHASES,
    required_lifecycle_phases: &REQUIRED_GUARD_PHASES,
    optional_lifecycle_phases: &[],
    prompt_capture: ContractCapability {
        status: ContractSupportStatus::Verified,
        detail: "UserPromptSubmit includes the submitted prompt.",
    },
    reload_restart_trust_requirements: &CLAUDE_CODE_REQUIREMENTS,
    managed_mode_support: ContractCapability {
        status: ContractSupportStatus::Unsupported,
        detail: "No verified Claude Code managed policy distribution contract is recorded; project-local Claude Code MCP, settings hook, policy, and rule files are guarded setup, not managed mode.",
    },
    managed_distribution_source: None,
    full_guarded_adapter_support: ContractCapability {
        status: ContractSupportStatus::Verified,
        detail: "The Claude Code adapter generates and verifies project-local settings hook commands for every required guarded lifecycle phase.",
    },
    known_limitations: &CLAUDE_CODE_LIMITATIONS,
    official_sources: &CLAUDE_CODE_SOURCES,
};

pub fn contract_for(host_kind: HostKind) -> Option<&'static HostIntegrationContract> {
    match host_kind {
        HostKind::Codex => Some(&CODEX_CONTRACT),
        HostKind::ClaudeCode => Some(&CLAUDE_CODE_CONTRACT),
        HostKind::Generic => None,
    }
}

pub fn contract_supports_full_guarded(contract: &HostIntegrationContract) -> bool {
    contract.full_guarded_adapter_support.status == ContractSupportStatus::Verified
}

pub fn contract_supports_managed_mode(contract: &HostIntegrationContract) -> bool {
    contract.managed_mode_support.status == ContractSupportStatus::Verified
        && contract.managed_distribution_source.is_some()
}

pub fn hook_event_for_phase(
    contract: &HostIntegrationContract,
    phase: HostLifecyclePhase,
) -> Option<&'static HostHookEventContract> {
    contract
        .hook_config_shape
        .events
        .iter()
        .find(|event| event.phase == phase)
}

pub fn classify_contract_config_path(
    host_kind: HostKind,
    path: &Path,
) -> Option<HostContractConfigKind> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let file_name = path.file_name()?.to_string_lossy();
    match host_kind {
        HostKind::Codex => {
            if ends_with_components(&components, &[".codex", "config.toml"]) {
                Some(HostContractConfigKind::ProjectConfig)
            } else if ends_with_components(&components, &[".codex", "hooks.json"]) {
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
        HostKind::ClaudeCode => {
            if file_name == ".mcp.json" {
                Some(HostContractConfigKind::McpConfig)
            } else if ends_with_components(&components, &[".claude", "settings.json"])
                || ends_with_components(&components, &[".claude", "settings.local.json"])
            {
                Some(HostContractConfigKind::ProjectSettings)
            } else if components
                .windows(2)
                .any(|window| window == [".claude", "rules"])
                && file_name.ends_with(".md")
            {
                Some(HostContractConfigKind::RuleConfig)
            } else {
                None
            }
        }
        HostKind::Generic => None,
    }
}

pub fn validate_contract_config(
    host_kind: HostKind,
    kind: HostContractConfigKind,
    text: &str,
) -> Result<(), HostContractValidationError> {
    let contract = contract_for(host_kind).ok_or_else(|| {
        HostContractValidationError::new(format!(
            "no host integration contract is defined for {}",
            host_kind.as_str()
        ))
    })?;
    match (contract.host_kind, kind) {
        (HostKind::Codex, HostContractConfigKind::ProjectConfig) => {
            validate_codex_project_config(text)
        }
        (HostKind::Codex, HostContractConfigKind::HookConfig) => {
            validate_json_hook_config(contract, text)
        }
        (HostKind::Codex, HostContractConfigKind::RuleConfig) => validate_codex_rule_config(text),
        (HostKind::ClaudeCode, HostContractConfigKind::McpConfig) => {
            validate_claude_mcp_config(text)
        }
        (HostKind::ClaudeCode, HostContractConfigKind::ProjectSettings) => {
            validate_claude_project_settings(contract, text)
        }
        (HostKind::ClaudeCode, HostContractConfigKind::HookConfig) => {
            validate_json_hook_config(contract, text)
        }
        (HostKind::ClaudeCode, HostContractConfigKind::RuleConfig) => {
            validate_claude_rule_config(text)
        }
        _ => Err(HostContractValidationError::new(format!(
            "{} does not support {} contract config",
            host_kind.as_str(),
            kind.as_str()
        ))),
    }
}

pub fn validate_hook_event_fixture(
    host_kind: HostKind,
    phase: HostLifecyclePhase,
    text: &str,
) -> Result<(), HostContractValidationError> {
    let contract = contract_for(host_kind).ok_or_else(|| {
        HostContractValidationError::new(format!(
            "no host integration contract is defined for {}",
            host_kind.as_str()
        ))
    })?;
    let event = hook_event_for_phase(contract, phase).ok_or_else(|| {
        HostContractValidationError::new(format!(
            "{} has no hook event for {:?}",
            host_kind.as_str(),
            phase
        ))
    })?;
    let value = parse_json_value(text, "hook event fixture")?;
    let object = value
        .as_object()
        .ok_or_else(|| HostContractValidationError::new("hook event fixture must be an object"))?;
    require_string(object.get("session_id"), "session_id")?;
    require_string(object.get("cwd"), "cwd")?;
    require_nullable_string(object.get("transcript_path"), "transcript_path")?;
    require_string(object.get("permission_mode"), "permission_mode")?;
    let hook_event_name = require_string(object.get("hook_event_name"), "hook_event_name")?;
    if hook_event_name != event.event_name {
        return Err(HostContractValidationError::new(format!(
            "hook_event_name must be {}, got {hook_event_name}",
            event.event_name
        )));
    }
    if host_kind == HostKind::Codex {
        require_string(object.get("model"), "model")?;
    }
    match phase {
        HostLifecyclePhase::SessionStart => {
            require_string(object.get("source"), "source")?;
        }
        HostLifecyclePhase::PreTool => {
            validate_tool_event_input(host_kind, object, false)?;
        }
        HostLifecyclePhase::PostTool => {
            validate_tool_event_input(host_kind, object, true)?;
        }
        HostLifecyclePhase::UserPromptSubmit => {
            if host_kind == HostKind::Codex {
                require_string(object.get("turn_id"), "turn_id")?;
            }
            require_string(object.get("prompt"), "prompt")?;
        }
        HostLifecyclePhase::Stop => {
            if host_kind == HostKind::Codex {
                require_string(object.get("turn_id"), "turn_id")?;
            }
            require_bool(object.get("stop_hook_active"), "stop_hook_active")?;
            require_nullable_string(
                object.get("last_assistant_message"),
                "last_assistant_message",
            )?;
            if let Some(background_tasks) = object.get("background_tasks") {
                require_array(background_tasks, "background_tasks")?;
            }
            if let Some(session_crons) = object.get("session_crons") {
                require_array(session_crons, "session_crons")?;
            }
        }
    }
    Ok(())
}

fn ends_with_components(components: &[String], suffix: &[&str]) -> bool {
    components.len() >= suffix.len()
        && components[components.len() - suffix.len()..]
            .iter()
            .zip(suffix)
            .all(|(left, right)| left == right)
}

fn parse_json_value(text: &str, label: &str) -> Result<Value, HostContractValidationError> {
    serde_json::from_str::<Value>(text)
        .map_err(|error| HostContractValidationError::new(format!("{label} must be JSON: {error}")))
}

fn validate_codex_project_config(text: &str) -> Result<(), HostContractValidationError> {
    let document = DocumentMut::from_str(text).map_err(|error| {
        HostContractValidationError::new(format!("Codex project config must be TOML: {error}"))
    })?;
    let servers = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .ok_or_else(|| {
            HostContractValidationError::new("Codex project config must contain [mcp_servers]")
        })?;
    if servers.is_empty() {
        return Err(HostContractValidationError::new(
            "Codex project config must define at least one MCP server",
        ));
    }
    for (name, item) in servers.iter() {
        let table = item.as_table().ok_or_else(|| {
            HostContractValidationError::new(format!("Codex MCP server {name} must be a table"))
        })?;
        validate_toml_mcp_server_table("Codex", name, table)?;
    }
    Ok(())
}

fn validate_toml_mcp_server_table(
    host_label: &str,
    name: &str,
    table: &Table,
) -> Result<(), HostContractValidationError> {
    let has_command = table.get("command").is_some();
    let has_url = table.get("url").is_some();
    if !has_command && !has_url {
        return Err(HostContractValidationError::new(format!(
            "{host_label} MCP server {name} must define command or url"
        )));
    }
    if let Some(command) = table.get("command") {
        if command.as_str().is_none() {
            return Err(HostContractValidationError::new(format!(
                "{host_label} MCP server {name} command must be a string"
            )));
        }
    }
    if let Some(url) = table.get("url") {
        if url.as_str().is_none() {
            return Err(HostContractValidationError::new(format!(
                "{host_label} MCP server {name} url must be a string"
            )));
        }
    }
    if let Some(args) = table.get("args") {
        let array = args.as_array().ok_or_else(|| {
            HostContractValidationError::new(format!(
                "{host_label} MCP server {name} args must be an array"
            ))
        })?;
        for arg in array.iter() {
            if arg.as_str().is_none() {
                return Err(HostContractValidationError::new(format!(
                    "{host_label} MCP server {name} args must contain only strings"
                )));
            }
        }
    }
    if let Some(env) = table.get("env") {
        let env_table = env.as_table().ok_or_else(|| {
            HostContractValidationError::new(format!(
                "{host_label} MCP server {name} env must be a table"
            ))
        })?;
        for (key, value) in env_table.iter() {
            if value.as_str().is_none() {
                return Err(HostContractValidationError::new(format!(
                    "{host_label} MCP server {name} env.{key} must be a string"
                )));
            }
        }
    }
    Ok(())
}

fn validate_claude_mcp_config(text: &str) -> Result<(), HostContractValidationError> {
    let value = parse_json_value(text, "Claude Code MCP config")?;
    let object = value.as_object().ok_or_else(|| {
        HostContractValidationError::new("Claude Code MCP config must be an object")
    })?;
    let servers = object
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            HostContractValidationError::new(
                "Claude Code MCP config must contain an mcpServers object",
            )
        })?;
    if servers.is_empty() {
        return Err(HostContractValidationError::new(
            "Claude Code MCP config must define at least one MCP server",
        ));
    }
    for (name, server) in servers {
        validate_json_mcp_server("Claude Code", name, server)?;
    }
    Ok(())
}

fn validate_json_mcp_server(
    host_label: &str,
    name: &str,
    server: &Value,
) -> Result<(), HostContractValidationError> {
    let object = server.as_object().ok_or_else(|| {
        HostContractValidationError::new(format!(
            "{host_label} MCP server {name} must be an object"
        ))
    })?;
    let has_command = object.get("command").is_some();
    let has_url = object.get("url").is_some();
    if !has_command && !has_url {
        return Err(HostContractValidationError::new(format!(
            "{host_label} MCP server {name} must define command or url"
        )));
    }
    if let Some(command) = object.get("command") {
        require_string(Some(command), "command")?;
    }
    if let Some(url) = object.get("url") {
        require_string(Some(url), "url")?;
    }
    if let Some(args) = object.get("args") {
        let array = require_array(args, "args")?;
        for arg in array {
            if arg.as_str().is_none() {
                return Err(HostContractValidationError::new(format!(
                    "{host_label} MCP server {name} args must contain only strings"
                )));
            }
        }
    }
    if let Some(env) = object.get("env") {
        let env_object = env.as_object().ok_or_else(|| {
            HostContractValidationError::new(format!(
                "{host_label} MCP server {name} env must be an object"
            ))
        })?;
        for (key, value) in env_object {
            if value.as_str().is_none() {
                return Err(HostContractValidationError::new(format!(
                    "{host_label} MCP server {name} env.{key} must be a string"
                )));
            }
        }
    }
    Ok(())
}

fn validate_claude_project_settings(
    contract: &HostIntegrationContract,
    text: &str,
) -> Result<(), HostContractValidationError> {
    let value = parse_json_value(text, "Claude Code project settings")?;
    let object = value.as_object().ok_or_else(|| {
        HostContractValidationError::new("Claude Code project settings must be an object")
    })?;
    if let Some(schema) = object.get("$schema") {
        require_string(Some(schema), "$schema")?;
    }
    if let Some(permissions) = object.get("permissions") {
        if !permissions.is_object() {
            return Err(HostContractValidationError::new(
                "Claude Code permissions must be an object",
            ));
        }
    }
    if object.get("hooks").is_some() {
        validate_json_hook_value(contract, &value)?;
    }
    Ok(())
}

fn validate_json_hook_config(
    contract: &HostIntegrationContract,
    text: &str,
) -> Result<(), HostContractValidationError> {
    let value = parse_json_value(text, "host hook config")?;
    validate_json_hook_value(contract, &value)
}

fn validate_json_hook_value(
    contract: &HostIntegrationContract,
    value: &Value,
) -> Result<(), HostContractValidationError> {
    let root = value
        .as_object()
        .ok_or_else(|| HostContractValidationError::new("host hook config must be an object"))?;
    let hooks = root
        .get(contract.hook_config_shape.root_key)
        .and_then(Value::as_object)
        .ok_or_else(|| {
            HostContractValidationError::new(format!(
                "host hook config must contain a {} object",
                contract.hook_config_shape.root_key
            ))
        })?;
    for event in contract.hook_config_shape.events {
        let groups = hooks
            .get(event.event_name)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                HostContractValidationError::new(format!(
                    "host hook config must contain {} groups",
                    event.event_name
                ))
            })?;
        if groups.is_empty() {
            return Err(HostContractValidationError::new(format!(
                "{} hook groups must not be empty",
                event.event_name
            )));
        }
        let mut saw_write_matcher = event.write_matcher_tokens.is_empty();
        for group in groups {
            let group_object = group.as_object().ok_or_else(|| {
                HostContractValidationError::new(format!(
                    "{} hook group must be an object",
                    event.event_name
                ))
            })?;
            if let Some(matcher) = group_object.get("matcher") {
                let matcher = require_string(Some(matcher), "matcher")?;
                if matcher_contains_any(matcher, event.write_matcher_tokens) {
                    saw_write_matcher = true;
                }
            }
            let handlers = group_object
                .get("hooks")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    HostContractValidationError::new(format!(
                        "{} hook group must contain hooks array",
                        event.event_name
                    ))
                })?;
            if handlers.is_empty() {
                return Err(HostContractValidationError::new(format!(
                    "{} hook handlers must not be empty",
                    event.event_name
                )));
            }
            for handler in handlers {
                validate_hook_handler(event.event_name, handler)?;
            }
        }
        if !saw_write_matcher {
            return Err(HostContractValidationError::new(format!(
                "{} hook config must include a write matcher",
                event.event_name
            )));
        }
    }
    Ok(())
}

fn validate_hook_handler(
    event_name: &str,
    handler: &Value,
) -> Result<(), HostContractValidationError> {
    let object = handler.as_object().ok_or_else(|| {
        HostContractValidationError::new(format!("{event_name} hook handler must be an object"))
    })?;
    let handler_type = require_string(object.get("type"), "type")?;
    if handler_type != "command" {
        return Err(HostContractValidationError::new(format!(
            "{event_name} hook handler type must be command"
        )));
    }
    require_string(object.get("command"), "command")?;
    if let Some(args) = object.get("args") {
        let array = require_array(args, "args")?;
        for arg in array {
            if arg.as_str().is_none() {
                return Err(HostContractValidationError::new(format!(
                    "{event_name} hook handler args must contain only strings"
                )));
            }
        }
    }
    if let Some(timeout) = object.get("timeout") {
        if !timeout.is_number() {
            return Err(HostContractValidationError::new(format!(
                "{event_name} hook handler timeout must be a number"
            )));
        }
    }
    if let Some(status_message) = object.get("statusMessage") {
        require_string(Some(status_message), "statusMessage")?;
    }
    Ok(())
}

fn validate_codex_rule_config(text: &str) -> Result<(), HostContractValidationError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(HostContractValidationError::new(
            "Codex rule config must not be empty",
        ));
    }
    if !trimmed.contains("prefix_rule(") {
        return Err(HostContractValidationError::new(
            "Codex rule config must contain a prefix_rule entry",
        ));
    }
    if !trimmed.contains("pattern = [") || !trimmed.contains("decision = ") {
        return Err(HostContractValidationError::new(
            "Codex rule config must include pattern and decision fields",
        ));
    }
    Ok(())
}

fn validate_claude_rule_config(text: &str) -> Result<(), HostContractValidationError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(HostContractValidationError::new(
            "Claude Code rule config must not be empty",
        ));
    }
    if !trimmed.starts_with('#') {
        return Err(HostContractValidationError::new(
            "Claude Code rule config is expected to be Markdown",
        ));
    }
    Ok(())
}

fn validate_tool_event_input(
    host_kind: HostKind,
    object: &serde_json::Map<String, Value>,
    require_response: bool,
) -> Result<(), HostContractValidationError> {
    if host_kind == HostKind::Codex {
        require_string(object.get("turn_id"), "turn_id")?;
    }
    let tool_name = require_string(object.get("tool_name"), "tool_name")?;
    require_string(object.get("tool_use_id"), "tool_use_id")?;
    let tool_input = object
        .get("tool_input")
        .and_then(Value::as_object)
        .ok_or_else(|| HostContractValidationError::new("tool_input must be an object"))?;
    match host_kind {
        HostKind::Codex => {
            if tool_name != "apply_patch" {
                return Err(HostContractValidationError::new(
                    "Codex write hook fixture must use apply_patch",
                ));
            }
            require_string(tool_input.get("command"), "tool_input.command")?;
        }
        HostKind::ClaudeCode => {
            if !CLAUDE_WRITE_MATCHERS.contains(&tool_name) {
                return Err(HostContractValidationError::new(
                    "Claude Code write hook fixture must use a write tool",
                ));
            }
            require_string(tool_input.get("file_path"), "tool_input.file_path")?;
        }
        HostKind::Generic => {
            return Err(HostContractValidationError::new(
                "generic host has no hook event contract",
            ));
        }
    }
    if require_response && object.get("tool_response").is_none() {
        return Err(HostContractValidationError::new(
            "PostTool hook fixture must include tool_response",
        ));
    }
    Ok(())
}

fn matcher_contains_any(matcher: &str, tokens: &[&str]) -> bool {
    tokens.is_empty() || tokens.iter().any(|token| matcher.contains(token))
}

fn require_string<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<&'a str, HostContractValidationError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| HostContractValidationError::new(format!("{field} must be a string")))
}

fn require_nullable_string(
    value: Option<&Value>,
    field: &str,
) -> Result<(), HostContractValidationError> {
    match value {
        Some(Value::String(_)) | Some(Value::Null) => Ok(()),
        _ => Err(HostContractValidationError::new(format!(
            "{field} must be a string or null"
        ))),
    }
}

fn require_bool(value: Option<&Value>, field: &str) -> Result<bool, HostContractValidationError> {
    value
        .and_then(Value::as_bool)
        .ok_or_else(|| HostContractValidationError::new(format!("{field} must be a boolean")))
}

fn require_array<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a Vec<Value>, HostContractValidationError> {
    value
        .as_array()
        .ok_or_else(|| HostContractValidationError::new(format!("{field} must be an array")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::host_integration::{host_capabilities, HostLifecyclePhase};

    const CODEX_PROJECT_CONFIG: &str =
        include_str!("../../tests/fixtures/host_contracts/codex/project_config.toml");
    const CODEX_HOOKS: &str = include_str!("../../tests/fixtures/host_contracts/codex/hooks.json");
    const CODEX_RULES: &str =
        include_str!("../../tests/fixtures/host_contracts/codex/rules/volicord.rules");
    const CLAUDE_MCP: &str =
        include_str!("../../tests/fixtures/host_contracts/claude_code/mcp_project.json");
    const CLAUDE_SETTINGS: &str =
        include_str!("../../tests/fixtures/host_contracts/claude_code/project_settings.json");
    const CLAUDE_HOOKS: &str =
        include_str!("../../tests/fixtures/host_contracts/claude_code/hooks.json");
    const CLAUDE_RULES: &str =
        include_str!("../../tests/fixtures/host_contracts/claude_code/rules/volicord.md");

    #[test]
    fn codex_contract_records_verified_full_guarded_shapes() {
        let contract = contract_for(HostKind::Codex).expect("Codex contract should exist");

        assert_eq!(contract.host_kind, HostKind::Codex);
        assert_eq!(contract.mcp_config_shape.format, HostConfigFormat::Toml);
        assert_eq!(contract.mcp_config_shape.root_key, "mcp_servers");
        assert_eq!(contract.hook_config_shape.root_key, "hooks");
        assert_eq!(contract.supported_lifecycle_phases, REQUIRED_GUARD_PHASES);
        assert_eq!(contract.required_lifecycle_phases, REQUIRED_GUARD_PHASES);
        assert!(contract.optional_lifecycle_phases.is_empty());
        assert_eq!(
            contract.prompt_capture.status,
            ContractSupportStatus::Verified
        );
        assert_eq!(
            contract.managed_mode_support.status,
            ContractSupportStatus::Unsupported
        );
        assert_eq!(
            contract.full_guarded_adapter_support.status,
            ContractSupportStatus::Verified
        );
        assert!(contract_supports_full_guarded(contract));
        assert_eq!(contract.managed_distribution_source, None);
        assert!(!contract_supports_managed_mode(contract));

        let capabilities = host_capabilities(HostKind::Codex);
        assert!(capabilities.missing_required_guard_phases().is_empty());
    }

    #[test]
    fn claude_code_contract_records_verified_full_guarded_shapes() {
        let contract =
            contract_for(HostKind::ClaudeCode).expect("Claude Code contract should exist");

        assert_eq!(contract.host_kind, HostKind::ClaudeCode);
        assert_eq!(contract.mcp_config_shape.format, HostConfigFormat::Json);
        assert_eq!(contract.mcp_config_shape.root_key, "mcpServers");
        assert_eq!(contract.hook_config_shape.root_key, "hooks");
        assert_eq!(contract.supported_lifecycle_phases, REQUIRED_GUARD_PHASES);
        assert_eq!(contract.required_lifecycle_phases, REQUIRED_GUARD_PHASES);
        assert_eq!(
            contract.prompt_capture.status,
            ContractSupportStatus::Verified
        );
        assert_eq!(
            contract.managed_mode_support.status,
            ContractSupportStatus::Unsupported
        );
        assert_eq!(
            contract.full_guarded_adapter_support.status,
            ContractSupportStatus::Verified
        );
        assert!(contract_supports_full_guarded(contract));
        assert_eq!(contract.managed_distribution_source, None);
        assert!(!contract_supports_managed_mode(contract));

        let capabilities = host_capabilities(HostKind::ClaudeCode);
        assert!(capabilities.missing_required_guard_phases().is_empty());
    }

    #[test]
    fn codex_config_fixtures_validate_against_contract() {
        validate_contract_config(
            HostKind::Codex,
            HostContractConfigKind::ProjectConfig,
            CODEX_PROJECT_CONFIG,
        )
        .expect("Codex project config fixture should validate");
        validate_contract_config(
            HostKind::Codex,
            HostContractConfigKind::HookConfig,
            CODEX_HOOKS,
        )
        .expect("Codex hook config fixture should validate");
        validate_contract_config(
            HostKind::Codex,
            HostContractConfigKind::RuleConfig,
            CODEX_RULES,
        )
        .expect("Codex rule fixture should validate");
    }

    #[test]
    fn claude_code_config_fixtures_validate_against_contract() {
        validate_contract_config(
            HostKind::ClaudeCode,
            HostContractConfigKind::McpConfig,
            CLAUDE_MCP,
        )
        .expect("Claude Code MCP fixture should validate");
        validate_contract_config(
            HostKind::ClaudeCode,
            HostContractConfigKind::ProjectSettings,
            CLAUDE_SETTINGS,
        )
        .expect("Claude Code project settings fixture should validate");
        validate_contract_config(
            HostKind::ClaudeCode,
            HostContractConfigKind::HookConfig,
            CLAUDE_HOOKS,
        )
        .expect("Claude Code hook fixture should validate");
        validate_contract_config(
            HostKind::ClaudeCode,
            HostContractConfigKind::RuleConfig,
            CLAUDE_RULES,
        )
        .expect("Claude Code rule fixture should validate");
    }

    #[test]
    fn hook_event_fixtures_validate_against_contract() {
        validate_all_event_fixtures(
            HostKind::Codex,
            [
                (
                    HostLifecyclePhase::SessionStart,
                    include_str!(
                        "../../tests/fixtures/host_contracts/codex/events/session_start.json"
                    ),
                ),
                (
                    HostLifecyclePhase::PreTool,
                    include_str!(
                        "../../tests/fixtures/host_contracts/codex/events/pre_tool_write.json"
                    ),
                ),
                (
                    HostLifecyclePhase::PostTool,
                    include_str!(
                        "../../tests/fixtures/host_contracts/codex/events/post_tool_write.json"
                    ),
                ),
                (
                    HostLifecyclePhase::UserPromptSubmit,
                    include_str!(
                        "../../tests/fixtures/host_contracts/codex/events/user_prompt_submit.json"
                    ),
                ),
                (
                    HostLifecyclePhase::Stop,
                    include_str!("../../tests/fixtures/host_contracts/codex/events/stop.json"),
                ),
            ],
        );

        validate_all_event_fixtures(
            HostKind::ClaudeCode,
            [
                (
                    HostLifecyclePhase::SessionStart,
                    include_str!(
                        "../../tests/fixtures/host_contracts/claude_code/events/session_start.json"
                    ),
                ),
                (
                    HostLifecyclePhase::PreTool,
                    include_str!(
                        "../../tests/fixtures/host_contracts/claude_code/events/pre_tool_write.json"
                    ),
                ),
                (
                    HostLifecyclePhase::PostTool,
                    include_str!(
                        "../../tests/fixtures/host_contracts/claude_code/events/post_tool_write.json"
                    ),
                ),
                (
                    HostLifecyclePhase::UserPromptSubmit,
                    include_str!(
                        "../../tests/fixtures/host_contracts/claude_code/events/user_prompt_submit.json"
                    ),
                ),
                (
                    HostLifecyclePhase::Stop,
                    include_str!(
                        "../../tests/fixtures/host_contracts/claude_code/events/stop.json"
                    ),
                ),
            ],
        );
    }

    #[test]
    fn unknown_fields_are_ignored_but_managed_field_conflicts_fail() {
        let codex_with_unknown = r#"
unknown_root = "preserved"

[mcp_servers.volicord]
command = "volicord"
args = ["mcp", "--stdio"]
unknown_server_field = "preserved"
"#;
        validate_contract_config(
            HostKind::Codex,
            HostContractConfigKind::ProjectConfig,
            codex_with_unknown,
        )
        .expect("unknown Codex fields should not fail validation");

        let codex_conflict = r#"
[mcp_servers.volicord]
command = ["volicord"]
"#;
        let error = validate_contract_config(
            HostKind::Codex,
            HostContractConfigKind::ProjectConfig,
            codex_conflict,
        )
        .expect_err("managed command field type conflict should fail");
        assert!(error.message().contains("command must be a string"));

        let claude_with_unknown = json!({
            "mcpServers": {
                "volicord": {
                    "command": "volicord",
                    "args": ["mcp", "--stdio"],
                    "volicordUnknown": true
                }
            },
            "unknownRoot": true
        });
        validate_contract_config(
            HostKind::ClaudeCode,
            HostContractConfigKind::McpConfig,
            &serde_json::to_string_pretty(&claude_with_unknown).unwrap(),
        )
        .expect("unknown Claude Code fields should not fail validation");

        let claude_conflict = json!({
            "mcpServers": {
                "volicord": {
                    "command": "volicord",
                    "args": "mcp --stdio"
                }
            }
        });
        let error = validate_contract_config(
            HostKind::ClaudeCode,
            HostContractConfigKind::McpConfig,
            &serde_json::to_string_pretty(&claude_conflict).unwrap(),
        )
        .expect_err("managed args field type conflict should fail");
        assert!(error.message().contains("args must be an array"));
    }

    #[test]
    fn path_classifier_distinguishes_host_hook_config_from_guidance_and_policy() {
        assert_eq!(
            classify_contract_config_path(HostKind::Codex, Path::new(".codex/hooks.json")),
            Some(HostContractConfigKind::HookConfig)
        );
        assert_eq!(
            classify_contract_config_path(HostKind::Codex, Path::new(".codex/config.toml")),
            Some(HostContractConfigKind::ProjectConfig)
        );
        assert_eq!(
            classify_contract_config_path(HostKind::ClaudeCode, Path::new(".claude/settings.json")),
            Some(HostContractConfigKind::ProjectSettings)
        );
        assert_eq!(
            classify_contract_config_path(HostKind::ClaudeCode, Path::new(".mcp.json")),
            Some(HostContractConfigKind::McpConfig)
        );
        assert_eq!(
            classify_contract_config_path(
                HostKind::ClaudeCode,
                Path::new(".claude/rules/volicord.md")
            ),
            Some(HostContractConfigKind::RuleConfig)
        );

        assert_eq!(
            classify_contract_config_path(HostKind::Codex, Path::new("AGENTS.md")),
            None
        );
        assert_eq!(
            classify_contract_config_path(HostKind::ClaudeCode, Path::new("AGENTS.md")),
            None
        );
        assert_eq!(
            classify_contract_config_path(HostKind::Codex, Path::new(".volicord/policy.json")),
            None
        );
        assert_eq!(
            classify_contract_config_path(HostKind::ClaudeCode, Path::new(".volicord/policy.json")),
            None
        );
    }

    fn validate_all_event_fixtures<const N: usize>(
        host_kind: HostKind,
        fixtures: [(HostLifecyclePhase, &str); N],
    ) {
        for (phase, text) in fixtures {
            validate_hook_event_fixture(host_kind, phase, text)
                .unwrap_or_else(|error| panic!("{host_kind:?} {phase:?}: {error}"));
        }
    }
}
