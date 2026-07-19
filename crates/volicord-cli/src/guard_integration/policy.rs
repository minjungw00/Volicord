use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};
use volicord_types::{
    GuardCommandInvocationSet, GuardCommandSet, GuardHookPhase, IntegrationProfile,
};

use crate::{
    guard_integration::{
        files::{read_managed_text, VOLICORD_POLICY_FILE, VOLICORD_POLICY_SCHEMA},
        hooks::guard_command_specs_json,
        public_host_label, GuardIntegrationError,
    },
    host_integration::{
        guard_phase_capability_name, ConnectionIntent, HostKind, ManagedServerEntry,
    },
};

pub(crate) const POLICY_STORAGE_SCOPE: &str = "local_overlay";
const ALLOWED_POLICY_MCP_ENV_KEYS: &[&str] = &[
    "VOLICORD_HOME",
    "VOLICORD_MCP_LAUNCH",
    "VOLICORD_MCP_HOST",
    "VOLICORD_MCP_CONNECTION_ID",
    "VOLICORD_MCP_PROJECT_ID",
];
const POLICY_TOP_LEVEL_KEYS: &[&str] = &[
    "schema",
    "managed_by",
    "storage_scope",
    "connection_intent",
    "host",
    "repo_root",
    "connection_id",
    "guard_installation_id",
    "selected_profile",
    "mcp",
    "host_hook",
    "workflow",
];
const POLICY_MCP_KEYS: &[&str] = &["command", "args", "env"];
const POLICY_HOST_HOOK_KEYS: &[&str] = &["enabled", "commands"];
const POLICY_HOOK_COMMAND_KEYS: &[&str] = &["command", "args"];
const POLICY_WORKFLOW_KEYS: &[&str] = &[
    "default_direct_control",
    "default_work_control",
    "light",
    "write_ticket",
];
const POLICY_LIGHT_KEYS: &[&str] = &[
    "enabled",
    "max_intended_paths",
    "allowed_path_patterns",
    "denied_path_patterns",
    "final_acceptance",
];
const POLICY_WRITE_TICKET_KEYS: &[&str] = &["idle_timeout_minutes"];
const CONTROL_LEVELS: &[&str] = &["observe", "light", "tracked", "sensitive"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PolicyValidationIssue {
    pub(crate) code: &'static str,
    pub(crate) field_path: String,
    pub(crate) message: String,
}

impl std::fmt::Display for PolicyValidationIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} at {}: {}",
            self.code, self.field_path, self.message
        )
    }
}

pub(crate) struct LocalPolicyContext<'a> {
    pub(crate) repo_root: &'a Path,
    pub(crate) connection_id: &'a str,
    pub(crate) guard_installation_id: &'a str,
    pub(crate) connection_intent: ConnectionIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedLocalPolicy {
    pub(crate) host: String,
    pub(crate) repo_root: PathBuf,
    pub(crate) connection_intent: ConnectionIntent,
    pub(crate) selected_profile: IntegrationProfile,
    pub(crate) connection_id: String,
    pub(crate) guard_installation_id: String,
}

pub(crate) fn recorded_local_policy(
    repo_root: &Path,
) -> Result<Option<RecordedLocalPolicy>, GuardIntegrationError> {
    let path = repo_root.join(VOLICORD_POLICY_FILE);
    let Some(text) = read_managed_text(repo_root, &path)? else {
        return Ok(None);
    };
    let policy = serde_json::from_str::<Value>(&text).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "existing policy file is not valid JSON: {} ({error})",
            path.display()
        ))
    })?;
    let intent_text = policy
        .get("connection_intent")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("policy schema requires connection_intent")
        })?;
    validate_policy_schema(&policy, intent_text)?;
    let connection_intent = match intent_text {
        "personal" => ConnectionIntent::Personal,
        "shared" => ConnectionIntent::Shared,
        _ => {
            return Err(GuardIntegrationError::runtime(
                "policy schema contains an unsupported connection_intent",
            ));
        }
    };
    let selected_profile = match policy.get("selected_profile").and_then(Value::as_str) {
        Some("record") => IntegrationProfile::Record,
        _ => {
            return Err(GuardIntegrationError::runtime(
                "policy schema contains an unsupported selected_profile",
            ));
        }
    };
    let required = |field: &str| {
        policy
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "policy schema requires a non-empty {field} string"
                ))
            })
    };
    Ok(Some(RecordedLocalPolicy {
        host: required("host")?,
        repo_root: PathBuf::from(required("repo_root")?),
        connection_intent,
        selected_profile,
        connection_id: required("connection_id")?,
        guard_installation_id: required("guard_installation_id")?,
    }))
}

pub(crate) fn policy_json(
    host_kind: HostKind,
    profile: IntegrationProfile,
    context: LocalPolicyContext<'_>,
    mcp_entry: &ManagedServerEntry,
    guard_commands: &GuardCommandSet,
) -> Result<Value, GuardIntegrationError> {
    validate_policy_mcp_environment(&mcp_entry.env)?;
    let commands = guard_command_specs_json(guard_commands)?;
    let policy = json!({
        "schema": VOLICORD_POLICY_SCHEMA,
        "managed_by": "volicord",
        "storage_scope": POLICY_STORAGE_SCOPE,
        "connection_intent": context.connection_intent.as_str(),
        "host": public_host_label(host_kind),
        "repo_root": path_text(context.repo_root),
        "connection_id": context.connection_id,
        "guard_installation_id": context.guard_installation_id,
        "selected_profile": profile.as_str(),
        "mcp": {
            "command": &mcp_entry.command,
            "args": &mcp_entry.args,
            "env": &mcp_entry.env,
        },
        "host_hook": {
            "enabled": true,
            "commands": commands,
        },
        "workflow": default_workflow_policy_json(),
    });
    validate_policy_schema(&policy, context.connection_intent.as_str())?;
    Ok(policy)
}

pub(crate) fn default_workflow_policy_json() -> Value {
    json!({
        "default_direct_control": "tracked",
        "default_work_control": "tracked",
        "light": {
            "enabled": false,
            "max_intended_paths": 3,
            "allowed_path_patterns": [],
            "denied_path_patterns": [],
            "final_acceptance": "policy_dependent",
        },
        "write_ticket": {
            "idle_timeout_minutes": Value::Null,
        },
    })
}

pub(crate) fn validate_policy_schema(
    policy: &Value,
    expected_connection_intent: &str,
) -> Result<(), GuardIntegrationError> {
    validate_workflow_policy(policy, Some(expected_connection_intent))
        .map_err(|issue| GuardIntegrationError::runtime(issue.message))
}

pub(crate) fn validate_workflow_policy(
    policy: &Value,
    expected_connection_intent: Option<&str>,
) -> Result<(), PolicyValidationIssue> {
    if let Some(expected) = expected_connection_intent {
        if !matches!(expected, "personal" | "shared") {
            return Err(validation_issue(
                "POLICY_VALUE_INVALID",
                "$.connection_intent",
                "policy schema requires connection_intent=personal or connection_intent=shared",
            ));
        }
    }
    let object = require_object(policy, "$", "policy schema requires one JSON object")?;
    validate_exact_object_keys(object, POLICY_TOP_LEVEL_KEYS, "$", "top-level")?;
    for (field, expected) in [
        ("schema", VOLICORD_POLICY_SCHEMA),
        ("managed_by", "volicord"),
        ("storage_scope", POLICY_STORAGE_SCOPE),
    ] {
        if object.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(validation_issue(
                "POLICY_VALUE_INVALID",
                format!("$.{field}"),
                format!("policy schema requires {field}={expected}"),
            ));
        }
    }
    let connection_intent = object
        .get("connection_intent")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "personal" | "shared"))
        .ok_or_else(|| {
            validation_issue(
                "POLICY_VALUE_INVALID",
                "$.connection_intent",
                "policy schema requires connection_intent=personal or connection_intent=shared",
            )
        })?;
    if expected_connection_intent.is_some_and(|expected| expected != connection_intent) {
        let expected = expected_connection_intent.expect("expected intent is present");
        return Err(validation_issue(
            "POLICY_BINDING_MISMATCH",
            "$.connection_intent",
            format!("policy schema requires connection_intent={expected}"),
        ));
    }
    for field in [
        "host",
        "repo_root",
        "connection_id",
        "guard_installation_id",
        "selected_profile",
    ] {
        if object
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(validation_issue(
                "POLICY_VALUE_INVALID",
                format!("$.{field}"),
                format!("policy schema requires a non-empty {field} string"),
            ));
        }
    }
    if object.get("selected_profile").and_then(Value::as_str) != Some("record") {
        return Err(validation_issue(
            "POLICY_VALUE_INVALID",
            "$.selected_profile",
            "policy schema requires selected_profile=record",
        ));
    }
    let mcp = require_object(
        object.get("mcp").expect("required key was validated"),
        "$.mcp",
        "policy schema requires one mcp object",
    )?;
    validate_exact_object_keys(mcp, POLICY_MCP_KEYS, "$.mcp", "mcp")?;
    if mcp
        .get("command")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || !mcp
            .get("args")
            .and_then(Value::as_array)
            .is_some_and(|args| args.iter().all(Value::is_string))
        || !mcp.get("env").is_some_and(Value::is_object)
    {
        return Err(validation_issue(
            "POLICY_VALUE_INVALID",
            "$.mcp",
            "policy schema requires mcp command, string args, and an environment object",
        ));
    }
    for (key, value) in mcp
        .get("env")
        .and_then(Value::as_object)
        .expect("environment object was validated")
    {
        validate_policy_mcp_environment_key(key).map_err(|error| {
            validation_issue(
                "POLICY_ENV_KEY_INVALID",
                format!("$.mcp.env.{key}"),
                error.to_string(),
            )
        })?;
        if !value.is_string() {
            return Err(validation_issue(
                "POLICY_VALUE_INVALID",
                format!("$.mcp.env.{key}"),
                format!("policy schema requires mcp.env.{key} as a string"),
            ));
        }
    }
    let host_hook = require_object(
        object.get("host_hook").expect("required key was validated"),
        "$.host_hook",
        "policy schema requires one host_hook object",
    )?;
    validate_exact_object_keys(host_hook, POLICY_HOST_HOOK_KEYS, "$.host_hook", "host_hook")?;
    let enabled = host_hook
        .get("enabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            validation_issue(
                "POLICY_VALUE_INVALID",
                "$.host_hook.enabled",
                "policy schema requires host_hook.enabled as a bool",
            )
        })?;
    let commands = require_object(
        host_hook
            .get("commands")
            .expect("required key was validated"),
        "$.host_hook.commands",
        "policy schema requires host_hook.commands as an object",
    )?;
    let phase_keys = GuardHookPhase::REQUIRED.map(GuardHookPhase::as_str);
    validate_exact_object_keys(
        commands,
        &phase_keys,
        "$.host_hook.commands",
        "host_hook.commands",
    )?;
    for (phase, command) in commands {
        let command_path = format!("$.host_hook.commands.{phase}");
        let command = require_object(
            command,
            &command_path,
            format!("policy schema requires host_hook.commands.{phase} as an object"),
        )?;
        validate_exact_object_keys(
            command,
            POLICY_HOOK_COMMAND_KEYS,
            &command_path,
            &format!("host_hook.commands.{phase}"),
        )?;
        if command
            .get("command")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
            || !command
                .get("args")
                .and_then(Value::as_array)
                .is_some_and(|args| args.iter().all(Value::is_string))
        {
            return Err(validation_issue(
                "POLICY_VALUE_INVALID",
                command_path,
                format!(
                    "policy schema requires host_hook.commands.{phase} command and string args"
                ),
            ));
        }
    }
    let command_set = serde_json::from_value::<GuardCommandSet>(Value::Object(commands.clone()))
        .map_err(|_| {
            validation_issue(
                "POLICY_VALUE_INVALID",
                "$.host_hook.commands",
                "policy schema requires the exact current Guard command set",
            )
        })?;
    let invocations =
        GuardCommandInvocationSet::from_policy_commands(&command_set).map_err(|_| {
            validation_issue(
                "POLICY_BINDING_MISMATCH",
                "$.host_hook.commands",
                "policy schema requires exact hash-free Guard policy commands",
            )
        })?;
    let invocation = invocations.get(GuardHookPhase::PreTool);
    let command_binding_matches = object.get("repo_root").and_then(Value::as_str)
        == Some(invocation.repo_root.as_str())
        && object.get("connection_id").and_then(Value::as_str)
            == Some(invocation.connection_id.as_str())
        && object.get("guard_installation_id").and_then(Value::as_str)
            == Some(invocation.guard_installation_id.as_str())
        && object.get("host").and_then(Value::as_str) == Some(invocation.host_kind.as_str())
        && object.get("selected_profile").and_then(Value::as_str)
            == Some(invocation.integration_profile.as_str());
    if !command_binding_matches {
        return Err(validation_issue(
            "POLICY_BINDING_MISMATCH",
            "$.host_hook.commands",
            "policy Guard commands must match the policy owner fields",
        ));
    }
    if !enabled {
        return Err(validation_issue(
            "POLICY_BINDING_MISMATCH",
            "$.host_hook.enabled",
            "policy schema requires host_hook.enabled=true for the record Guard workflow",
        ));
    }

    validate_workflow_settings(object.get("workflow").expect("required key was validated"))?;
    Ok(())
}

fn validate_exact_object_keys(
    object: &serde_json::Map<String, Value>,
    allowed_keys: &[&str],
    field_path: &str,
    label: &str,
) -> Result<(), PolicyValidationIssue> {
    if let Some(key) = object
        .keys()
        .find(|key| !allowed_keys.contains(&key.as_str()))
    {
        return Err(validation_issue(
            "POLICY_FIELD_UNKNOWN",
            format!("{field_path}.{key}"),
            format!("policy schema rejects unknown {label} field {key}"),
        ));
    }
    if let Some(key) = allowed_keys.iter().find(|key| !object.contains_key(**key)) {
        return Err(validation_issue(
            "POLICY_FIELD_REQUIRED",
            format!("{field_path}.{key}"),
            format!("policy schema requires {label} field {key}"),
        ));
    }
    Ok(())
}

fn validate_workflow_settings(workflow: &Value) -> Result<(), PolicyValidationIssue> {
    let workflow = require_object(
        workflow,
        "$.workflow",
        "policy schema requires workflow as an object",
    )?;
    validate_exact_object_keys(workflow, POLICY_WORKFLOW_KEYS, "$.workflow", "workflow")?;
    for field in ["default_direct_control", "default_work_control"] {
        let value = workflow.get(field).and_then(Value::as_str);
        if !value.is_some_and(|value| CONTROL_LEVELS.contains(&value)) {
            return Err(validation_issue(
                "POLICY_VALUE_INVALID",
                format!("$.workflow.{field}"),
                format!(
                    "policy schema requires workflow.{field}=observe, light, tracked, or sensitive"
                ),
            ));
        }
    }

    let light = require_object(
        workflow.get("light").expect("required key was validated"),
        "$.workflow.light",
        "policy schema requires workflow.light as an object",
    )?;
    validate_exact_object_keys(
        light,
        POLICY_LIGHT_KEYS,
        "$.workflow.light",
        "workflow.light",
    )?;
    if !light.get("enabled").is_some_and(Value::is_boolean) {
        return Err(validation_issue(
            "POLICY_VALUE_INVALID",
            "$.workflow.light.enabled",
            "policy schema requires workflow.light.enabled as a bool",
        ));
    }
    if light
        .get("max_intended_paths")
        .and_then(Value::as_u64)
        .is_none_or(|value| value == 0)
    {
        return Err(validation_issue(
            "POLICY_VALUE_INVALID",
            "$.workflow.light.max_intended_paths",
            "policy schema requires workflow.light.max_intended_paths as a positive integer",
        ));
    }
    for field in ["allowed_path_patterns", "denied_path_patterns"] {
        let patterns = light.get(field).and_then(Value::as_array).ok_or_else(|| {
            validation_issue(
                "POLICY_VALUE_INVALID",
                format!("$.workflow.light.{field}"),
                format!("policy schema requires workflow.light.{field} as a string array"),
            )
        })?;
        for (index, pattern) in patterns.iter().enumerate() {
            let path = pattern.as_str().ok_or_else(|| {
                validation_issue(
                    "POLICY_VALUE_INVALID",
                    format!("$.workflow.light.{field}[{index}]"),
                    format!("policy schema requires workflow.light.{field} as a string array"),
                )
            })?;
            if !is_normalized_repository_prefix(path) {
                return Err(validation_issue(
                    "POLICY_PATH_PATTERN_INVALID",
                    format!("$.workflow.light.{field}[{index}]"),
                    "policy path patterns must be normalized repository-relative file or directory prefixes",
                ));
            }
        }
    }
    if !matches!(
        light.get("final_acceptance").and_then(Value::as_str),
        Some("required" | "not_required" | "policy_dependent")
    ) {
        return Err(validation_issue(
            "POLICY_VALUE_INVALID",
            "$.workflow.light.final_acceptance",
            "policy schema requires workflow.light.final_acceptance=required, not_required, or policy_dependent",
        ));
    }

    let write_ticket = require_object(
        workflow
            .get("write_ticket")
            .expect("required key was validated"),
        "$.workflow.write_ticket",
        "policy schema requires workflow.write_ticket as an object",
    )?;
    validate_exact_object_keys(
        write_ticket,
        POLICY_WRITE_TICKET_KEYS,
        "$.workflow.write_ticket",
        "workflow.write_ticket",
    )?;
    let idle_timeout = write_ticket
        .get("idle_timeout_minutes")
        .expect("required key was validated");
    if !idle_timeout.is_null() && idle_timeout.as_u64().is_none_or(|minutes| minutes == 0) {
        return Err(validation_issue(
            "POLICY_VALUE_INVALID",
            "$.workflow.write_ticket.idle_timeout_minutes",
            "policy schema requires workflow.write_ticket.idle_timeout_minutes as a positive integer or null",
        ));
    }

    Ok(())
}

fn require_object(
    value: &Value,
    field_path: impl Into<String>,
    message: impl Into<String>,
) -> Result<&serde_json::Map<String, Value>, PolicyValidationIssue> {
    value
        .as_object()
        .ok_or_else(|| validation_issue("POLICY_VALUE_INVALID", field_path.into(), message.into()))
}

fn validation_issue(
    code: &'static str,
    field_path: impl Into<String>,
    message: impl Into<String>,
) -> PolicyValidationIssue {
    PolicyValidationIssue {
        code,
        field_path: field_path.into(),
        message: message.into(),
    }
}

fn is_normalized_repository_prefix(value: &str) -> bool {
    if value.is_empty()
        || value != value.trim()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || has_windows_drive_prefix(value)
    {
        return false;
    }
    value
        .split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn validate_policy_mcp_environment(
    environment: &BTreeMap<String, String>,
) -> Result<(), GuardIntegrationError> {
    for key in environment.keys() {
        validate_policy_mcp_environment_key(key)?;
    }
    Ok(())
}

fn validate_policy_mcp_environment_key(key: &str) -> Result<(), GuardIntegrationError> {
    if ALLOWED_POLICY_MCP_ENV_KEYS.contains(&key) {
        return Ok(());
    }
    let description = if secret_like_env_key(key) {
        "secret-like"
    } else {
        "unapproved"
    };
    Err(GuardIntegrationError::runtime(format!(
        "policy serialization refuses {description} MCP environment key {key}; only Volicord-owned launch metadata keys are allowed"
    )))
}

fn secret_like_env_key(key: &str) -> bool {
    let normalized = key.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "API_KEY",
        "PRIVATE_KEY",
        "CREDENTIAL",
        "AUTH",
        "COOKIE",
        "BEARER",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

pub(crate) fn required_guard_phase_names() -> Vec<&'static str> {
    GuardHookPhase::REQUIRED
        .iter()
        .map(|phase| guard_phase_capability_name(*phase))
        .collect()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
