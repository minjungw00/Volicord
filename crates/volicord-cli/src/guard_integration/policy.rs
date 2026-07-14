use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        files::{read_managed_text, VOLICORD_POLICY_FILE, VOLICORD_POLICY_SCHEMA},
        hooks::GuardCommandSpec,
        public_host_label, GuardIntegrationError,
    },
    host_integration::{
        ConnectionIntent, HostKind, HostLifecyclePhase, ManagedServerEntry, REQUIRED_GUARD_PHASES,
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
];
const POLICY_MCP_KEYS: &[&str] = &["command", "args", "env"];
const POLICY_HOST_HOOK_KEYS: &[&str] = &["enabled", "commands"];
const POLICY_HOOK_PHASE_KEYS: &[&str] = &[
    "session_start",
    "pre_tool",
    "post_tool",
    "prompt_capture",
    "stop",
];
const POLICY_HOOK_COMMAND_KEYS: &[&str] = &["command", "args"];

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
        "global" => ConnectionIntent::Global,
        _ => {
            return Err(GuardIntegrationError::runtime(
                "policy schema contains an unsupported connection_intent",
            ));
        }
    };
    let selected_profile = match policy.get("selected_profile").and_then(Value::as_str) {
        Some("record") => IntegrationProfile::Record,
        Some("detective") => IntegrationProfile::Detective,
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
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
) -> Result<Value, GuardIntegrationError> {
    validate_policy_mcp_environment(&mcp_entry.env)?;
    let commands = guard_commands
        .iter()
        .map(|(phase, spec)| {
            (
                phase.clone(),
                json!({
                    "command": &spec.command,
                    "args": &spec.args,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
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
            "enabled": profile != IntegrationProfile::Record,
            "commands": commands,
        },
    });
    validate_policy_schema(&policy, context.connection_intent.as_str())?;
    Ok(policy)
}

pub(crate) fn validate_policy_schema(
    policy: &Value,
    expected_connection_intent: &str,
) -> Result<(), GuardIntegrationError> {
    if !matches!(expected_connection_intent, "personal" | "shared" | "global") {
        return Err(GuardIntegrationError::runtime(
            "policy schema requires connection_intent=personal, connection_intent=shared, or connection_intent=global",
        ));
    }
    let object = policy
        .as_object()
        .ok_or_else(|| GuardIntegrationError::runtime("policy schema requires one JSON object"))?;
    validate_exact_object_keys(object, POLICY_TOP_LEVEL_KEYS, "top-level")?;
    for (field, expected) in [
        ("schema", VOLICORD_POLICY_SCHEMA),
        ("managed_by", "volicord"),
        ("storage_scope", POLICY_STORAGE_SCOPE),
        ("connection_intent", expected_connection_intent),
    ] {
        if object.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(GuardIntegrationError::runtime(format!(
                "policy schema requires {field}={expected}"
            )));
        }
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
            return Err(GuardIntegrationError::runtime(format!(
                "policy schema requires a non-empty {field} string"
            )));
        }
    }
    if !matches!(
        object.get("selected_profile").and_then(Value::as_str),
        Some("record" | "detective")
    ) {
        return Err(GuardIntegrationError::runtime(
            "policy schema requires selected_profile=record or selected_profile=detective",
        ));
    }
    let mcp = object
        .get("mcp")
        .and_then(Value::as_object)
        .ok_or_else(|| GuardIntegrationError::runtime("policy schema requires one mcp object"))?;
    validate_exact_object_keys(mcp, POLICY_MCP_KEYS, "mcp")?;
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
        return Err(GuardIntegrationError::runtime(
            "policy schema requires mcp command, string args, and an environment object",
        ));
    }
    for (key, value) in mcp
        .get("env")
        .and_then(Value::as_object)
        .expect("environment object was validated")
    {
        validate_policy_mcp_environment_key(key)?;
        if !value.is_string() {
            return Err(GuardIntegrationError::runtime(format!(
                "policy schema requires mcp.env.{key} as a string"
            )));
        }
    }
    let host_hook = object
        .get("host_hook")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("policy schema requires one host_hook object")
        })?;
    validate_exact_object_keys(host_hook, POLICY_HOST_HOOK_KEYS, "host_hook")?;
    let enabled = host_hook
        .get("enabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("policy schema requires host_hook.enabled as a bool")
        })?;
    let commands = host_hook
        .get("commands")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("policy schema requires host_hook.commands as an object")
        })?;
    validate_exact_object_keys(commands, POLICY_HOOK_PHASE_KEYS, "host_hook.commands")?;
    for (phase, command) in commands {
        let command = command.as_object().ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "policy schema requires host_hook.commands.{phase} as an object"
            ))
        })?;
        validate_exact_object_keys(
            command,
            POLICY_HOOK_COMMAND_KEYS,
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
            return Err(GuardIntegrationError::runtime(format!(
                "policy schema requires host_hook.commands.{phase} command and string args"
            )));
        }
    }
    let detective = object.get("selected_profile").and_then(Value::as_str) == Some("detective");
    if enabled != detective {
        return Err(GuardIntegrationError::runtime(
            "policy schema requires host_hook.enabled to match selected_profile",
        ));
    }
    Ok(())
}

fn validate_exact_object_keys(
    object: &serde_json::Map<String, Value>,
    allowed_keys: &[&str],
    label: &str,
) -> Result<(), GuardIntegrationError> {
    if let Some(key) = object
        .keys()
        .find(|key| !allowed_keys.contains(&key.as_str()))
    {
        return Err(GuardIntegrationError::runtime(format!(
            "policy schema rejects unknown {label} field {key}"
        )));
    }
    if let Some(key) = allowed_keys.iter().find(|key| !object.contains_key(**key)) {
        return Err(GuardIntegrationError::runtime(format!(
            "policy schema requires {label} field {key}"
        )));
    }
    Ok(())
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
    REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| phase.capability_name())
        .collect()
}

pub(crate) fn lifecycle_phase_names(phases: &[HostLifecyclePhase]) -> Vec<&'static str> {
    phases.iter().map(|phase| phase.capability_name()).collect()
}

pub(crate) fn guard_has_prompt_capture_commands(policy: &Value) -> bool {
    policy
        .get("host_hook")
        .and_then(|guard| guard.get("commands"))
        .and_then(|commands| commands.get("prompt_capture"))
        .is_some()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with_env(key: &str, value: &str) -> ManagedServerEntry {
        ManagedServerEntry {
            command: "volicord".to_owned(),
            args: vec![
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--connection".to_owned(),
                "conn_test".to_owned(),
            ],
            env: BTreeMap::from([(key.to_owned(), value.to_owned())]),
            env_vars: Vec::new(),
        }
    }

    fn test_guard_commands() -> BTreeMap<String, GuardCommandSpec> {
        POLICY_HOOK_PHASE_KEYS
            .iter()
            .map(|phase| {
                (
                    (*phase).to_owned(),
                    GuardCommandSpec {
                        command: "volicord".to_owned(),
                        args: vec!["_hook".to_owned(), phase.replace('_', "-")],
                    },
                )
            })
            .collect()
    }

    fn test_policy_context(connection_intent: ConnectionIntent) -> LocalPolicyContext<'static> {
        LocalPolicyContext {
            repo_root: Path::new("/repo"),
            connection_id: "conn_test",
            guard_installation_id: "guard_test",
            connection_intent,
        }
    }

    #[test]
    fn policy_accepts_only_volicord_launch_environment() -> Result<(), Box<dyn std::error::Error>> {
        let policy = policy_json(
            HostKind::Codex,
            IntegrationProfile::Record,
            test_policy_context(ConnectionIntent::Personal),
            &entry_with_env("VOLICORD_HOME", "/runtime"),
            &test_guard_commands(),
        )?;

        assert_eq!(policy["mcp"]["env"]["VOLICORD_HOME"], "/runtime");
        assert_eq!(policy["storage_scope"], POLICY_STORAGE_SCOPE);
        assert_eq!(policy["connection_intent"], "personal");
        Ok(())
    }

    #[test]
    fn policy_rejects_secret_like_environment_without_echoing_value() {
        let error = policy_json(
            HostKind::Codex,
            IntegrationProfile::Record,
            test_policy_context(ConnectionIntent::Personal),
            &entry_with_env("SERVICE_API_TOKEN", "do-not-print-this"),
            &test_guard_commands(),
        )
        .expect_err("secret-like environment must be rejected");

        assert!(error
            .to_string()
            .contains("secret-like MCP environment key"));
        assert!(error.to_string().contains("SERVICE_API_TOKEN"));
        assert!(!error.to_string().contains("do-not-print-this"));
    }

    #[test]
    fn policy_rejects_other_unapproved_environment_keys() {
        let error = policy_json(
            HostKind::ClaudeCode,
            IntegrationProfile::Record,
            test_policy_context(ConnectionIntent::Shared),
            &entry_with_env("RUST_LOG", "debug"),
            &test_guard_commands(),
        )
        .expect_err("unapproved environment must be rejected");

        assert!(error
            .to_string()
            .contains("unapproved MCP environment key RUST_LOG"));
    }

    #[test]
    fn policy_schema_rejects_wrong_intent_and_non_local_storage_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut policy = policy_json(
            HostKind::Codex,
            IntegrationProfile::Record,
            test_policy_context(ConnectionIntent::Shared),
            &entry_with_env("VOLICORD_HOME", "/runtime"),
            &test_guard_commands(),
        )?;

        policy["connection_intent"] = Value::String("personal".to_owned());
        let error = validate_policy_schema(&policy, "shared")
            .expect_err("a policy for a different intent must be rejected");
        assert!(error.to_string().contains("connection_intent=shared"));

        policy["connection_intent"] = Value::String("shared".to_owned());
        policy["storage_scope"] = Value::String("repository_shared".to_owned());
        let error = validate_policy_schema(&policy, "shared")
            .expect_err("the policy cannot become a shared repository projection");
        assert!(error.to_string().contains("storage_scope=local_overlay"));

        policy["storage_scope"] = Value::String(POLICY_STORAGE_SCOPE.to_owned());
        policy["connection_intent"] = Value::String("unknown".to_owned());
        let error = validate_policy_schema(&policy, "unknown")
            .expect_err("an unknown self-consistent intent must still be rejected");
        assert!(error.to_string().contains("connection_intent=personal"));
        Ok(())
    }

    #[test]
    fn policy_schema_rejects_unknown_fields_and_non_string_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let policy = policy_json(
            HostKind::Codex,
            IntegrationProfile::Record,
            test_policy_context(ConnectionIntent::Shared),
            &entry_with_env("VOLICORD_HOME", "/runtime"),
            &test_guard_commands(),
        )?;

        let mut candidate = policy.clone();
        candidate
            .as_object_mut()
            .expect("policy object")
            .insert("unexpected".to_owned(), Value::Bool(true));
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("an unknown top-level field must be rejected");
        assert!(error.to_string().contains("unknown top-level field"));

        let mut candidate = policy.clone();
        candidate["mcp"]
            .as_object_mut()
            .expect("mcp object")
            .insert("unexpected".to_owned(), Value::Bool(true));
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("an unknown MCP field must be rejected");
        assert!(error.to_string().contains("unknown mcp field"));

        let mut candidate = policy.clone();
        candidate["host_hook"]
            .as_object_mut()
            .expect("host_hook object")
            .insert("unexpected".to_owned(), Value::Bool(true));
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("an unknown host_hook field must be rejected");
        assert!(error.to_string().contains("unknown host_hook field"));

        let mut candidate = policy.clone();
        candidate["host_hook"]["commands"]
            .as_object_mut()
            .expect("host_hook commands object")
            .insert(
                "unexpected_phase".to_owned(),
                json!({"command": "volicord", "args": []}),
            );
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("an unknown hook phase must be rejected");
        assert!(error
            .to_string()
            .contains("unknown host_hook.commands field"));

        let mut candidate = policy.clone();
        candidate["host_hook"]["commands"]["pre_tool"]
            .as_object_mut()
            .expect("pre_tool command object")
            .insert("unexpected".to_owned(), Value::Bool(true));
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("an unknown hook command field must be rejected");
        assert!(error
            .to_string()
            .contains("unknown host_hook.commands.pre_tool field"));

        let mut candidate = policy.clone();
        candidate["mcp"]["env"]["VOLICORD_HOME"] = Value::Bool(true);
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("a non-string environment value must be rejected");
        assert!(error
            .to_string()
            .contains("mcp.env.VOLICORD_HOME as a string"));

        let mut candidate = policy;
        candidate["host_hook"]["commands"]["pre_tool"]["args"] = json!(["_hook", 7]);
        let error = validate_policy_schema(&candidate, "shared")
            .expect_err("a non-string hook argument must be rejected");
        assert!(error
            .to_string()
            .contains("host_hook.commands.pre_tool command and string args"));
        Ok(())
    }
}
