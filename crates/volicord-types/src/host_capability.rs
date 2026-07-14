use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

use serde_json::Value;

/// Exact current schema of the stored host-hook capability record.
pub const HOST_HOOK_CAPABILITY_SCHEMA: &str = "volicord-host-hook-capability-v2";

/// Closed top-level member set of the stored host-hook capability record.
pub const HOST_HOOK_CAPABILITY_FIELDS: &[&str] = &[
    "schema",
    "policy_hash",
    "selected_profile",
    "connection_intent",
    "final_output_authority_disclosure_implementation_available",
    "native_host_output_adapter",
    "native_host_output_adapter_config_verified",
    "bash_shell_mutation_coverage",
    "direct_file_write_matcher_coverage",
    "host_capabilities",
    "required_hook_phases",
    "missing_required_hooks",
    "prompt_capture",
    "files",
    "host_hook_commands",
    "hook_root_resolution",
    "hook_path_safety",
    "commands",
];

/// Closed lifecycle-phase set required by the Detective profile.
pub const DETECTIVE_REQUIRED_HOOK_PHASES: &[&str] = &[
    "session_start_hook",
    "pre_tool_hook",
    "post_tool_hook",
    "user_prompt_submit_hook",
    "stop_hook",
];

/// Returns whether a decoded value has the exact current closed v2 shape and
/// the generated nested value types consumed across Core, Store, and adapters.
pub fn host_hook_capability_has_exact_v2_shape(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let top_level_shape_matches = object.len() == HOST_HOOK_CAPABILITY_FIELDS.len()
        && HOST_HOOK_CAPABILITY_FIELDS
            .iter()
            .all(|field| object.contains_key(*field))
        && value.get("schema").and_then(Value::as_str) == Some(HOST_HOOK_CAPABILITY_SCHEMA);
    top_level_shape_matches
        && [
            "policy_hash",
            "selected_profile",
            "connection_intent",
            "native_host_output_adapter",
        ]
        .iter()
        .all(|field| value.get(*field).is_some_and(Value::is_string))
        && [
            "final_output_authority_disclosure_implementation_available",
            "native_host_output_adapter_config_verified",
            "bash_shell_mutation_coverage",
            "direct_file_write_matcher_coverage",
            "prompt_capture",
        ]
        .iter()
        .all(|field| value.get(*field).is_some_and(Value::is_boolean))
        && host_capabilities_have_exact_shape(&value["host_capabilities"])
        && ["required_hook_phases", "missing_required_hooks"]
            .iter()
            .all(|field| json_array_all(&value[*field], Value::is_string))
        && json_array_all(&value["files"], generated_file_entry_has_required_shape)
        && json_array_all(
            &value["host_hook_commands"],
            host_hook_command_has_exact_shape,
        )
        && hook_root_resolution_has_exact_shape(&value["hook_root_resolution"])
        && hook_path_safety_has_exact_shape(&value["hook_path_safety"])
        && policy_commands_have_exact_shape(&value["commands"])
        && host_hook_capability_has_coherent_v2_semantics(value)
}

/// Owning row and Agent Connection facts required before a stored host-hook
/// capability may be consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostHookCapabilityOwnerBinding<'a> {
    pub row_host_kind: &'a str,
    pub row_guard_mode: &'a str,
    pub row_guard_installation_id: &'a str,
    pub connection_internal_id: &'a str,
    pub connection_host_kind: &'a str,
    pub connection_intent: &'a str,
    pub project_repo_root: Option<&'a Path>,
    pub project_git_info_exclude_path: Option<&'a Path>,
}

/// Returns whether an exact current capability is bound to the owning row and
/// Agent Connection facts that authorize its consumption.
pub fn host_hook_capability_matches_owner_binding(
    value: &Value,
    binding: HostHookCapabilityOwnerBinding<'_>,
) -> bool {
    let HostHookCapabilityOwnerBinding {
        row_host_kind,
        row_guard_mode,
        row_guard_installation_id,
        connection_internal_id,
        connection_host_kind,
        connection_intent,
        project_repo_root,
        project_git_info_exclude_path,
    } = binding;
    if !host_hook_capability_has_exact_v2_shape(value)
        || row_host_kind != connection_host_kind
        || value["selected_profile"].as_str() != Some(row_guard_mode)
        || value["connection_intent"].as_str() != Some(connection_intent)
    {
        return false;
    }
    let adapter_matches_host = match row_host_kind {
        "codex" => matches!(
            value["native_host_output_adapter"].as_str(),
            Some("codex" | "none")
        ),
        "claude_code" => matches!(
            value["native_host_output_adapter"].as_str(),
            Some("claude-code" | "none")
        ),
        _ => value["native_host_output_adapter"].as_str() == Some("none"),
    };
    if !adapter_matches_host
        || value["host_hook_commands"]
            .as_array()
            .expect("exact capability commands were validated")
            .iter()
            .any(|command| command["host_kind"].as_str() != Some(row_host_kind))
    {
        return false;
    }
    let managed_file_host_kind = if row_host_kind == "claude_code" {
        "claude-code"
    } else {
        row_host_kind
    };
    let commands_present = value["host_hook_commands"]
        .as_array()
        .is_some_and(|commands| !commands.is_empty());
    if commands_present
        && value["native_host_output_adapter"].as_str() != Some(managed_file_host_kind)
    {
        return false;
    }
    managed_script_inventory_matches_owner(
        value,
        managed_file_host_kind,
        row_guard_installation_id,
        connection_internal_id,
        project_repo_root,
    ) && match project_repo_root {
        Some(repo_root) => {
            policy_commands_match_owner(
                value,
                row_host_kind,
                row_guard_mode,
                row_guard_installation_id,
                connection_internal_id,
                repo_root,
            ) && repository_inventory_matches_owner(
                value,
                row_host_kind,
                row_guard_mode,
                connection_intent,
                repo_root,
                project_git_info_exclude_path,
            )
        }
        None => !capability_has_repository_inventory(value),
    }
}

fn capability_has_repository_inventory(capability: &Value) -> bool {
    capability["host_hook_commands"]
        .as_array()
        .is_some_and(|commands| !commands.is_empty())
        || capability["files"].as_array().is_some_and(|files| {
            files.iter().any(|file| {
                matches!(
                    file["kind"].as_str(),
                    Some(
                        "volicord_policy"
                            | "git_info_exclude"
                            | "host_mcp_config"
                            | "host_hook_config"
                            | "host_hook_dispatch"
                            | "host_hook_wrapper"
                            | "host_rule_instruction"
                            | "agents_managed_block"
                    )
                )
            })
        })
}

fn repository_inventory_matches_owner(
    capability: &Value,
    row_host_kind: &str,
    profile: &str,
    connection_intent: &str,
    repo_root: &Path,
    git_info_exclude_path: Option<&Path>,
) -> bool {
    if !normalized_absolute_path(repo_root) {
        return false;
    }
    if git_info_exclude_path.is_some_and(|path| !normalized_absolute_path(path)) {
        return false;
    }
    let commands = capability["host_hook_commands"]
        .as_array()
        .expect("exact capability commands were validated");
    let commands_by_policy_key = commands
        .iter()
        .map(|command| {
            (
                command["policy_key"]
                    .as_str()
                    .expect("exact command policy key was validated"),
                command,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (policy_key, command) in &commands_by_policy_key {
        let Some(command_name) = hook_command_name(policy_key) else {
            return false;
        };
        let phase_path = canonical_hook_wrapper_path(repo_root, row_host_kind, command_name);
        let wrapper_path = if row_host_kind == "codex" && profile == "detective" {
            repo_root.join(".codex/hooks/volicord-dispatch.sh")
        } else {
            phase_path.clone()
        };
        if command["expected_phase_wrapper_path"]
            .as_str()
            .is_none_or(|path| Path::new(path) != phase_path)
            || command["expected_wrapper_path"]
                .as_str()
                .is_none_or(|path| Path::new(path) != wrapper_path)
            || !host_hook_command_matches_generated_contract(
                command,
                row_host_kind,
                profile,
                command_name,
            )
        {
            return false;
        }
    }

    capability["files"]
        .as_array()
        .expect("exact capability files were validated")
        .iter()
        .all(|file| {
            let Some(path) = file["path"].as_str().map(Path::new) else {
                return false;
            };
            match file["kind"].as_str() {
                Some("volicord_policy") => path == repo_root.join(".volicord/policy.json"),
                Some("agents_managed_block") => path == repo_root.join("AGENTS.md"),
                Some("host_mcp_config") => {
                    row_host_kind == "claude_code" && path == repo_root.join(".mcp.json")
                }
                Some("host_hook_wrapper") => {
                    let Some(policy_key) = file["phase"].as_str() else {
                        return false;
                    };
                    let Some(command_name) = hook_command_name(policy_key) else {
                        return false;
                    };
                    commands_by_policy_key.contains_key(policy_key)
                        && path
                            == canonical_hook_wrapper_path(repo_root, row_host_kind, command_name)
                }
                Some("host_hook_dispatch") => {
                    row_host_kind == "codex"
                        && profile == "detective"
                        && path == repo_root.join(".codex/hooks/volicord-dispatch.sh")
                }
                Some("host_hook_config") => match row_host_kind {
                    "codex" => path == repo_root.join(".codex/hooks.json"),
                    "claude_code" => {
                        let relative = if connection_intent == "personal" {
                            ".claude/settings.local.json"
                        } else {
                            ".claude/settings.json"
                        };
                        path == repo_root.join(relative)
                    }
                    _ => false,
                },
                Some("host_rule_instruction") => match row_host_kind {
                    "codex" => path == repo_root.join(".codex/rules/volicord.rules"),
                    "claude_code" => path == repo_root.join(".claude/rules/volicord.md"),
                    _ => false,
                },
                Some("git_info_exclude") => git_info_exclude_path == Some(path),
                _ => false,
            }
        })
}

fn policy_commands_match_owner(
    capability: &Value,
    row_host_kind: &str,
    profile: &str,
    row_guard_installation_id: &str,
    connection_internal_id: &str,
    repo_root: &Path,
) -> bool {
    if !normalized_absolute_path(repo_root) {
        return false;
    }
    let Some(public_host_kind) = public_host_kind(row_host_kind) else {
        return false;
    };
    let commands = capability["commands"]
        .as_object()
        .expect("exact capability policy commands were validated");
    let mut executable = None;
    for (policy_key, command_name) in [
        ("session_start", "session-start"),
        ("pre_tool", "pre-tool"),
        ("post_tool", "post-tool"),
        ("prompt_capture", "prompt-capture"),
        ("stop", "stop"),
    ] {
        let command = &commands[policy_key];
        let Some(command_executable) = command["command"].as_str() else {
            return false;
        };
        if executable
            .replace(command_executable)
            .is_some_and(|prior| prior != command_executable)
        {
            return false;
        }
        let Some(expected_args) = expected_guard_command_args(
            command_name,
            repo_root,
            connection_internal_id,
            row_guard_installation_id,
            public_host_kind,
            profile,
            None,
        ) else {
            return false;
        };
        if command["args"].as_array().is_none_or(|args| {
            args.iter()
                .map(Value::as_str)
                .ne(expected_args.iter().map(|arg| Some(arg.as_str())))
        }) {
            return false;
        }
    }
    true
}

fn host_hook_command_matches_generated_contract(
    command: &Value,
    row_host_kind: &str,
    profile: &str,
    command_name: &str,
) -> bool {
    match (row_host_kind, profile) {
        ("codex", "detective" | "record") => {
            let script = if profile == "detective" {
                format!(
                    "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" {command_name}"
                )
            } else {
                format!(
                    "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-{command_name}.sh\""
                )
            };
            command["command_shape"].as_str() == Some("shell_command_string")
                && command["args"].is_null()
                && command["command"].as_str()
                    == Some(format!("sh -c {}", shell_word(&script)).as_str())
        }
        ("claude_code", "detective" | "record") => {
            command["command_shape"].as_str() == Some("exec_form")
                && command["args"]
                    .as_array()
                    .is_some_and(|args| args.is_empty())
                && command["command"].as_str()
                    == Some(
                        format!("${{CLAUDE_PROJECT_DIR}}/.claude/hooks/volicord-{command_name}.sh")
                            .as_str(),
                    )
        }
        _ => false,
    }
}

fn shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn public_host_kind(row_host_kind: &str) -> Option<&'static str> {
    match row_host_kind {
        "codex" => Some("codex"),
        "claude_code" => Some("claude-code"),
        "generic" => Some("generic"),
        _ => None,
    }
}

fn expected_guard_command_args(
    command_name: &str,
    repo_root: &Path,
    connection_internal_id: &str,
    row_guard_installation_id: &str,
    public_host_kind: &str,
    profile: &str,
    policy_hash: Option<&str>,
) -> Option<Vec<String>> {
    let (output_flag, output_format) = match (public_host_kind, profile) {
        ("codex", "detective") => ("--host-output", "codex"),
        ("claude-code", "detective") => ("--host-output", "claude-code"),
        ("codex" | "claude-code" | "generic", "record") | ("generic", "detective") => {
            ("--output", "volicord-json")
        }
        _ => return None,
    };
    let mut args = vec![
        "_hook".to_owned(),
        command_name.to_owned(),
        "--repo".to_owned(),
        repo_root.display().to_string(),
        "--connection".to_owned(),
        connection_internal_id.to_owned(),
        "--guard-installation".to_owned(),
        row_guard_installation_id.to_owned(),
        "--host".to_owned(),
        public_host_kind.to_owned(),
        "--integration-profile".to_owned(),
        profile.to_owned(),
    ];
    if let Some(policy_hash) = policy_hash {
        args.push("--policy-hash".to_owned());
        args.push(policy_hash.to_owned());
    }
    args.push(output_flag.to_owned());
    args.push(output_format.to_owned());
    Some(args)
}

fn expected_final_output_command_args(
    repo_root: &Path,
    connection_internal_id: &str,
    row_guard_installation_id: &str,
    public_host_kind: &str,
    policy_hash: &str,
) -> Vec<String> {
    vec![
        "_final-output".to_owned(),
        "--repo".to_owned(),
        repo_root.display().to_string(),
        "--connection".to_owned(),
        connection_internal_id.to_owned(),
        "--guard-installation".to_owned(),
        row_guard_installation_id.to_owned(),
        "--host".to_owned(),
        public_host_kind.to_owned(),
        "--integration-profile".to_owned(),
        "record".to_owned(),
        "--policy-hash".to_owned(),
        policy_hash.to_owned(),
        "--host-output".to_owned(),
        public_host_kind.to_owned(),
    ]
}

fn command_line(command: &str, args: &[String]) -> String {
    std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(shell_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonical_hook_wrapper_path(repo_root: &Path, host_kind: &str, command_name: &str) -> PathBuf {
    let host_directory = if host_kind == "claude_code" {
        ".claude"
    } else {
        ".codex"
    };
    repo_root
        .join(host_directory)
        .join("hooks")
        .join(format!("volicord-{command_name}.sh"))
}

fn hook_command_name(policy_key: &str) -> Option<&'static str> {
    match policy_key {
        "session_start" => Some("session-start"),
        "pre_tool" => Some("pre-tool"),
        "post_tool" => Some("post-tool"),
        "prompt_capture" => Some("prompt-capture"),
        "stop" => Some("stop"),
        _ => None,
    }
}

fn normalized_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

fn managed_script_inventory_matches_owner(
    capability: &Value,
    public_host_kind: &str,
    row_guard_installation_id: &str,
    connection_internal_id: &str,
    project_repo_root: Option<&Path>,
) -> bool {
    let profile = capability["selected_profile"]
        .as_str()
        .expect("exact capability profile was validated");
    let expected_purpose = if profile == "detective" {
        "detective_guard"
    } else {
        "final_output_authority_disclosure"
    };
    let policy_hash = capability["policy_hash"]
        .as_str()
        .expect("exact capability policy hash was validated");
    let commands = capability["host_hook_commands"]
        .as_array()
        .expect("exact capability commands were validated");
    let commands_by_policy_key = commands
        .iter()
        .map(|command| {
            (
                command["policy_key"]
                    .as_str()
                    .expect("exact command policy key was validated"),
                command,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let files = capability["files"]
        .as_array()
        .expect("exact capability files were validated");
    let mut seen_paths = BTreeSet::new();
    let mut seen_wrapper_phases = BTreeSet::new();
    let mut dispatch_count = 0;
    let mut hook_config_count = 0;
    let mut rule_count = 0;

    for file in files {
        let path = file["path"]
            .as_str()
            .expect("exact capability file path was validated");
        if path.trim().is_empty() || !seen_paths.insert(path) {
            return false;
        }
        match file["kind"].as_str() {
            Some("host_hook_wrapper") => {
                let Some(phase) = file["phase"].as_str() else {
                    return false;
                };
                let Some(command) = commands_by_policy_key.get(phase).copied() else {
                    return false;
                };
                let Some(command_name) = hook_command_name(phase) else {
                    return false;
                };
                let Some(repo_root) = project_repo_root else {
                    return false;
                };
                let executable = capability["commands"][phase]["command"]
                    .as_str()
                    .expect("exact capability policy command was validated");
                let expected_args = if profile == "detective" {
                    let Some(args) = expected_guard_command_args(
                        command_name,
                        repo_root,
                        connection_internal_id,
                        row_guard_installation_id,
                        public_host_kind,
                        profile,
                        Some(policy_hash),
                    ) else {
                        return false;
                    };
                    args
                } else {
                    expected_final_output_command_args(
                        repo_root,
                        connection_internal_id,
                        row_guard_installation_id,
                        public_host_kind,
                        policy_hash,
                    )
                };
                if !seen_wrapper_phases.insert(phase)
                    || file["managed_marker"].as_str() != Some("VOLICORD_MANAGED_HOOK_WRAPPER")
                    || file["managed_script_command"].as_str()
                        != Some(command_line(executable, &expected_args).as_str())
                    || file["host_kind"].as_str() != Some(public_host_kind)
                    || file["purpose"].as_str() != Some(expected_purpose)
                    || file["connection_id"].as_str() != Some(connection_internal_id)
                    || file["guard_installation_id"].as_str() != Some(row_guard_installation_id)
                    || file["policy_hash"].as_str() != Some(policy_hash)
                    || file["host_output"].as_str() != Some(public_host_kind)
                    || command["purpose"].as_str() != Some(expected_purpose)
                    || command["expected_phase_wrapper_path"].as_str() != Some(path)
                {
                    return false;
                }
            }
            Some("host_hook_dispatch") => {
                dispatch_count += 1;
                if dispatch_count != 1
                    || profile != "detective"
                    || public_host_kind != "codex"
                    || commands.is_empty()
                    || file["managed_marker"].as_str() != Some("VOLICORD_MANAGED_HOOK_WRAPPER")
                    || file["host_kind"].as_str() != Some("codex")
                    || file["phase"].as_str() != Some("dispatch")
                    || commands
                        .iter()
                        .any(|command| command["expected_wrapper_path"].as_str() != Some(path))
                {
                    return false;
                }
            }
            Some("host_hook_config") => {
                hook_config_count += 1;
                let ownership = file["ownership"].as_str();
                if hook_config_count != 1
                    || match public_host_kind {
                        "codex" => ownership != Some("managed_json"),
                        "claude-code" => {
                            ownership != Some("managed_json_projection")
                                || file["managed_projection"].as_str()
                                    != Some("claude_code_settings_hooks")
                        }
                        _ => true,
                    }
                {
                    return false;
                }
            }
            Some("host_rule_instruction") => {
                rule_count += 1;
                let (start_marker, end_marker) = match public_host_kind {
                    "codex" => (
                        "# BEGIN VOLICORD MANAGED CODEX RULES",
                        "# END VOLICORD MANAGED CODEX RULES",
                    ),
                    "claude-code" => (
                        "<!-- BEGIN VOLICORD MANAGED GUIDANCE -->",
                        "<!-- END VOLICORD MANAGED GUIDANCE -->",
                    ),
                    _ => return false,
                };
                if rule_count != 1
                    || profile != "detective"
                    || file["ownership"].as_str() != Some("managed_block")
                    || file["managed_marker_start"].as_str() != Some(start_marker)
                    || file["managed_marker_end"].as_str() != Some(end_marker)
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    if commands.is_empty() {
        seen_wrapper_phases.is_empty()
            && dispatch_count == 0
            && hook_config_count == 0
            && rule_count == 0
    } else {
        seen_wrapper_phases.len() == commands.len()
            && hook_config_count == 1
            && rule_count == usize::from(profile == "detective")
            && dispatch_count == usize::from(profile == "detective" && public_host_kind == "codex")
    }
}

fn host_hook_capability_has_coherent_v2_semantics(value: &Value) -> bool {
    let Some(profile) = value["selected_profile"].as_str() else {
        return false;
    };
    if value["policy_hash"]
        .as_str()
        .is_none_or(|policy_hash| policy_hash.trim().is_empty())
        || !matches!(profile, "record" | "detective")
        || !matches!(
            value["connection_intent"].as_str(),
            Some("personal" | "shared" | "global")
        )
    {
        return false;
    }

    let Some(adapter) = value["native_host_output_adapter"].as_str() else {
        return false;
    };
    let implementation_available = value
        ["final_output_authority_disclosure_implementation_available"]
        .as_bool()
        .unwrap_or(false);
    let adapter_available = matches!(adapter, "codex" | "claude-code");
    if (!matches!(adapter, "none" | "codex" | "claude-code"))
        || implementation_available != adapter_available
        || (value["native_host_output_adapter_config_verified"] == Value::Bool(true)
            && !adapter_available)
    {
        return false;
    }

    let required = string_array_set(&value["required_hook_phases"]);
    let missing = string_array_set(&value["missing_required_hooks"]);
    let Some(required) = required else {
        return false;
    };
    let Some(missing) = missing else {
        return false;
    };
    let detective_phases = DETECTIVE_REQUIRED_HOOK_PHASES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if !missing.is_subset(&required)
        || !missing.is_subset(&detective_phases)
        || (profile == "detective" && required != detective_phases)
        || (profile == "record" && (!required.is_empty() || !missing.is_empty()))
    {
        return false;
    }
    if profile == "record" && value["prompt_capture"] != Value::Bool(false) {
        return false;
    }

    let Some(commands) = value["host_hook_commands"].as_array() else {
        return false;
    };
    let mut command_phases = BTreeSet::new();
    let mut command_host_kind = None;
    for command in commands {
        let Some(phase) = command["phase"].as_str() else {
            return false;
        };
        let Some(policy_key) = policy_key_for_capability_phase(phase) else {
            return false;
        };
        if command["policy_key"].as_str() != Some(policy_key)
            || !command_phases.insert(phase)
            || command["purpose"].as_str()
                != Some(if profile == "detective" {
                    "detective_guard"
                } else {
                    "final_output_authority_disclosure"
                })
            || !host_hook_command_shape_is_coherent(command)
        {
            return false;
        }
        let Some(host_kind) = command["host_kind"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
        else {
            return false;
        };
        if command_host_kind
            .replace(host_kind)
            .is_some_and(|prior| prior != host_kind)
        {
            return false;
        }
    }
    let configured_detective_phases = detective_phases
        .difference(&missing)
        .copied()
        .collect::<BTreeSet<_>>();
    if (profile == "detective" && command_phases != configured_detective_phases)
        || (profile == "record"
            && !(command_phases.is_empty() || command_phases == BTreeSet::from(["stop_hook"])))
    {
        return false;
    }

    hook_safety_projections_are_coherent(value, commands)
}

fn string_array_set(value: &Value) -> Option<BTreeSet<&str>> {
    let values = value.as_array()?;
    let set = values
        .iter()
        .map(Value::as_str)
        .collect::<Option<BTreeSet<_>>>()?;
    (set.len() == values.len()).then_some(set)
}

fn policy_key_for_capability_phase(phase: &str) -> Option<&'static str> {
    match phase {
        "session_start_hook" => Some("session_start"),
        "pre_tool_hook" => Some("pre_tool"),
        "post_tool_hook" => Some("post_tool"),
        "user_prompt_submit_hook" => Some("prompt_capture"),
        "stop_hook" => Some("stop"),
        _ => None,
    }
}

fn host_hook_command_shape_is_coherent(command: &Value) -> bool {
    match command["command_shape"].as_str() {
        Some("shell_command_string") => command["args"].is_null(),
        Some("exec_form") => command["args"].is_array(),
        _ => false,
    }
}

fn hook_safety_projections_are_coherent(value: &Value, commands: &[Value]) -> bool {
    if commands.is_empty() {
        return value["hook_root_resolution"].is_null() && value["hook_path_safety"].is_null();
    }
    let Some(root) = value["hook_root_resolution"].as_object() else {
        return false;
    };
    let Some(safety) = value["hook_path_safety"].as_object() else {
        return false;
    };
    let root_phases = root["phases"].as_array().expect("shape was validated");
    let safety_commands = safety["commands"].as_array().expect("shape was validated");
    if root_phases.len() != commands.len() || safety_commands.len() != commands.len() {
        return false;
    }
    let mut roots = BTreeSet::new();
    let mut all_cwd_independent = true;
    let mut all_subdirectory_safe = true;
    let mut all_wrappers_ok = true;
    for command in commands {
        let phase = command["phase"].as_str().expect("shape was validated");
        let Some(root_phase) = root_phases
            .iter()
            .find(|candidate| candidate["phase"].as_str() == Some(phase))
        else {
            return false;
        };
        let Some(safety_command) = safety_commands
            .iter()
            .find(|candidate| candidate["phase"].as_str() == Some(phase))
        else {
            return false;
        };
        for field in [
            "root_resolution_basis",
            "hook_command_path_basis",
            "cwd_independent",
            "subdirectory_safe",
            "wrapper_resolution_status",
        ] {
            if root_phase.get(field) != command.get(field) {
                return false;
            }
        }
        for field in [
            "hook_command_path_basis",
            "cwd_independent",
            "subdirectory_safe",
            "wrapper_resolution_status",
        ] {
            if safety_command.get(field) != command.get(field) {
                return false;
            }
        }
        roots.insert(
            command["root_resolution_basis"]
                .as_str()
                .expect("shape was validated"),
        );
        all_cwd_independent &= command["cwd_independent"].as_bool().unwrap_or(false);
        all_subdirectory_safe &= command["subdirectory_safe"].as_bool().unwrap_or(false);
        all_wrappers_ok &= command["wrapper_resolution_status"].as_str() == Some("ok");
    }
    let expected_basis = if roots.len() == 1 {
        roots.iter().next().copied().unwrap_or_default()
    } else {
        "mixed"
    };
    let root_ok = all_cwd_independent && all_subdirectory_safe;
    let safety_ok = root_ok && all_wrappers_ok;
    root["basis"].as_str() == Some(expected_basis)
        && root["all_cwd_independent"].as_bool() == Some(all_cwd_independent)
        && root["all_subdirectory_safe"].as_bool() == Some(all_subdirectory_safe)
        && root["overall_status"].as_str()
            == Some(if root_ok {
                "ok"
            } else {
                "relative_path_unsafe"
            })
        && safety["all_cwd_independent"].as_bool() == Some(all_cwd_independent)
        && safety["all_subdirectory_safe"].as_bool() == Some(all_subdirectory_safe)
        && safety["overall_status"].as_str()
            == Some(if safety_ok {
                "ok"
            } else {
                "relative_path_unsafe"
            })
}

fn object_has_exact_fields(value: &Value, fields: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field))
    })
}

fn json_array_all(value: &Value, predicate: fn(&Value) -> bool) -> bool {
    value
        .as_array()
        .is_some_and(|values| values.iter().all(predicate))
}

fn host_capabilities_have_exact_shape(value: &Value) -> bool {
    const FIELDS: &[&str] = &[
        "stdio_mcp",
        "http_mcp",
        "session_start_hook",
        "pre_tool_hook",
        "post_tool_hook",
        "user_prompt_submit_hook",
        "stop_hook",
        "rule_file_support",
        "project_local_configuration",
    ];
    object_has_exact_fields(value, FIELDS)
        && FIELDS
            .iter()
            .all(|field| value.get(*field).is_some_and(Value::is_boolean))
}

fn generated_file_entry_has_required_shape(value: &Value) -> bool {
    const COMMON_FIELDS: &[&str] = &["kind", "path", "status", "content_hash", "ownership"];
    if !COMMON_FIELDS
        .iter()
        .all(|field| value.get(*field).is_some_and(Value::is_string))
        || !matches!(
            value["kind"].as_str(),
            Some(
                "volicord_policy"
                    | "git_info_exclude"
                    | "host_mcp_config"
                    | "host_hook_config"
                    | "host_hook_dispatch"
                    | "host_hook_wrapper"
                    | "host_rule_instruction"
                    | "agents_managed_block"
            )
        )
        || !matches!(
            value["status"].as_str(),
            Some("planned_create" | "planned_update" | "unchanged" | "created" | "updated")
        )
    {
        return false;
    }
    match value.get("ownership").and_then(Value::as_str) {
        Some("managed_block") => {
            const FIELDS: &[&str] = &[
                "kind",
                "path",
                "status",
                "content_hash",
                "ownership",
                "managed_marker_start",
                "managed_marker_end",
            ];
            object_has_exact_fields(value, FIELDS)
                && FIELDS[5..]
                    .iter()
                    .all(|field| value.get(*field).is_some_and(Value::is_string))
        }
        Some("managed_json") => object_has_exact_fields(value, COMMON_FIELDS),
        Some("managed_json_projection") => {
            const FIELDS: &[&str] = &[
                "kind",
                "path",
                "status",
                "content_hash",
                "ownership",
                "managed_projection",
                "managed_projection_json",
            ];
            object_has_exact_fields(value, FIELDS)
                && FIELDS[5..]
                    .iter()
                    .all(|field| value.get(*field).is_some_and(Value::is_string))
        }
        Some("managed_script") => managed_script_entry_has_exact_shape(value),
        _ => false,
    }
}

fn managed_script_entry_has_exact_shape(value: &Value) -> bool {
    const DISPATCH_FIELDS: &[&str] = &[
        "kind",
        "path",
        "status",
        "content_hash",
        "ownership",
        "managed_marker",
        "executable_required",
        "managed_script_role",
        "host_kind",
        "phase",
    ];
    const WRAPPER_FIELDS: &[&str] = &[
        "kind",
        "path",
        "status",
        "content_hash",
        "ownership",
        "managed_marker",
        "executable_required",
        "managed_script_command",
        "host_kind",
        "phase",
        "purpose",
        "connection_id",
        "guard_installation_id",
        "policy_hash",
        "host_output",
    ];
    let common_types_match =
        value["managed_marker"].is_string() && value["executable_required"].is_boolean();
    if !common_types_match {
        return false;
    }
    if object_has_exact_fields(value, DISPATCH_FIELDS) {
        return value["managed_script_role"].as_str() == Some("codex_dispatch")
            && value["kind"].as_str() == Some("host_hook_dispatch")
            && DISPATCH_FIELDS[8..]
                .iter()
                .all(|field| value.get(*field).is_some_and(Value::is_string));
    }
    object_has_exact_fields(value, WRAPPER_FIELDS)
        && value["kind"].as_str() == Some("host_hook_wrapper")
        && WRAPPER_FIELDS[7..]
            .iter()
            .all(|field| value.get(*field).is_some_and(Value::is_string))
}

fn policy_commands_have_exact_shape(value: &Value) -> bool {
    const PHASES: &[&str] = &[
        "session_start",
        "pre_tool",
        "post_tool",
        "prompt_capture",
        "stop",
    ];
    object_has_exact_fields(value, PHASES)
        && PHASES.iter().all(|phase| {
            let command = &value[*phase];
            object_has_exact_fields(command, &["command", "args"])
                && command["command"]
                    .as_str()
                    .is_some_and(|command| !command.trim().is_empty())
                && json_array_all(&command["args"], Value::is_string)
        })
}

fn host_hook_command_has_exact_shape(value: &Value) -> bool {
    const FIELDS: &[&str] = &[
        "host_kind",
        "phase",
        "purpose",
        "policy_key",
        "command_shape",
        "command",
        "args",
        "expected_wrapper_path",
        "expected_phase_wrapper_path",
        "root_resolution_basis",
        "hook_command_path_basis",
        "cwd_independent",
        "subdirectory_safe",
        "wrapper_resolution_status",
        "verification",
    ];
    object_has_exact_fields(value, FIELDS)
        && FIELDS[..6]
            .iter()
            .chain(FIELDS[7..11].iter())
            .chain(FIELDS[13..14].iter())
            .all(|field| value.get(*field).is_some_and(Value::is_string))
        && (value["args"].is_null() || json_array_all(&value["args"], Value::is_string))
        && value["cwd_independent"].is_boolean()
        && value["subdirectory_safe"].is_boolean()
        && matches!(
            value["root_resolution_basis"].as_str(),
            Some("git_work_tree" | "claude_project_dir")
        )
        && matches!(
            value["hook_command_path_basis"].as_str(),
            Some("git_root_runtime" | "claude_project_dir")
        )
        && matches!(
            value["wrapper_resolution_status"].as_str(),
            Some(
                "ok" | "relative_path_unsafe"
                    | "wrapper_missing"
                    | "wrapper_not_executable"
                    | "dispatch_missing"
                    | "placeholder_unsupported"
                    | "absolute_path_stale"
                    | "policy_hash_mismatch"
                    | "host_output_mismatch"
                    | "authority_mismatch"
                    | "metadata_missing"
            )
        )
        && object_has_exact_fields(
            &value["verification"],
            &["basis_verified_by", "host_contract_source"],
        )
        && value["verification"]
            .as_object()
            .is_some_and(|verification| verification.values().all(Value::is_string))
}

fn hook_root_resolution_has_exact_shape(value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    const FIELDS: &[&str] = &[
        "basis",
        "all_cwd_independent",
        "all_subdirectory_safe",
        "overall_status",
        "phases",
    ];
    object_has_exact_fields(value, FIELDS)
        && value["basis"].is_string()
        && value["all_cwd_independent"].is_boolean()
        && value["all_subdirectory_safe"].is_boolean()
        && value["overall_status"].is_string()
        && json_array_all(&value["phases"], hook_root_phase_has_exact_shape)
}

fn hook_root_phase_has_exact_shape(value: &Value) -> bool {
    const FIELDS: &[&str] = &[
        "phase",
        "root_resolution_basis",
        "hook_command_path_basis",
        "cwd_independent",
        "subdirectory_safe",
        "wrapper_resolution_status",
    ];
    object_has_exact_fields(value, FIELDS)
        && FIELDS[..3]
            .iter()
            .chain(FIELDS[5..].iter())
            .all(|field| value.get(*field).is_some_and(Value::is_string))
        && value["cwd_independent"].is_boolean()
        && value["subdirectory_safe"].is_boolean()
}

fn hook_path_safety_has_exact_shape(value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    const FIELDS: &[&str] = &[
        "overall_status",
        "all_cwd_independent",
        "all_subdirectory_safe",
        "commands",
    ];
    object_has_exact_fields(value, FIELDS)
        && value["overall_status"].is_string()
        && value["all_cwd_independent"].is_boolean()
        && value["all_subdirectory_safe"].is_boolean()
        && json_array_all(&value["commands"], hook_path_command_has_exact_shape)
}

fn hook_path_command_has_exact_shape(value: &Value) -> bool {
    const FIELDS: &[&str] = &[
        "phase",
        "hook_command_path_basis",
        "cwd_independent",
        "subdirectory_safe",
        "wrapper_resolution_status",
    ];
    object_has_exact_fields(value, FIELDS)
        && FIELDS[..2]
            .iter()
            .chain(FIELDS[4..].iter())
            .all(|field| value.get(*field).is_some_and(Value::is_string))
        && value["cwd_independent"].is_boolean()
        && value["subdirectory_safe"].is_boolean()
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn codex_hook_command(profile: &str, command_name: &str) -> String {
        let script = if profile == "detective" {
            format!(
                "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" {command_name}"
            )
        } else {
            format!(
                "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-{command_name}.sh\""
            )
        };
        format!("sh -c {}", shell_word(&script))
    }

    fn detective_policy_command(command_name: &str) -> Value {
        json!({
            "command": "volicord",
            "args": [
                "_hook",
                command_name,
                "--repo",
                "/repo",
                "--connection",
                "conn_owner",
                "--guard-installation",
                "guard_owner",
                "--host",
                "codex",
                "--integration-profile",
                "detective",
                "--host-output",
                "codex",
            ],
        })
    }

    fn valid_capability() -> Value {
        let host_hook_commands = DETECTIVE_REQUIRED_HOOK_PHASES
            .iter()
            .map(|phase| {
                let policy_key =
                    policy_key_for_capability_phase(phase).expect("known capability phase");
                let command_name = hook_command_name(policy_key).expect("known policy key");
                json!({
                    "host_kind": "codex",
                    "phase": phase,
                    "purpose": "detective_guard",
                    "policy_key": policy_key,
                    "command_shape": "shell_command_string",
                    "command": codex_hook_command("detective", command_name),
                    "args": null,
                    "expected_wrapper_path": "/repo/.codex/hooks/volicord-dispatch.sh",
                    "expected_phase_wrapper_path": format!("/repo/.codex/hooks/volicord-{command_name}.sh"),
                    "root_resolution_basis": "git_work_tree",
                    "hook_command_path_basis": "git_root_runtime",
                    "cwd_independent": true,
                    "subdirectory_safe": true,
                    "wrapper_resolution_status": "ok",
                    "verification": {
                        "basis_verified_by": "repo_root_git_marker",
                        "host_contract_source": "codex_hook_command_string",
                    },
                })
            })
            .collect::<Vec<_>>();
        let root_phases = host_hook_commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command["phase"],
                    "root_resolution_basis": command["root_resolution_basis"],
                    "hook_command_path_basis": command["hook_command_path_basis"],
                    "cwd_independent": command["cwd_independent"],
                    "subdirectory_safe": command["subdirectory_safe"],
                    "wrapper_resolution_status": command["wrapper_resolution_status"],
                })
            })
            .collect::<Vec<_>>();
        let safety_commands = host_hook_commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command["phase"],
                    "hook_command_path_basis": command["hook_command_path_basis"],
                    "cwd_independent": command["cwd_independent"],
                    "subdirectory_safe": command["subdirectory_safe"],
                    "wrapper_resolution_status": command["wrapper_resolution_status"],
                })
            })
            .collect::<Vec<_>>();
        json!({
            "schema": HOST_HOOK_CAPABILITY_SCHEMA,
            "policy_hash": "policy-hash",
            "selected_profile": "detective",
            "connection_intent": "personal",
            "final_output_authority_disclosure_implementation_available": true,
            "native_host_output_adapter": "codex",
            "native_host_output_adapter_config_verified": true,
            "bash_shell_mutation_coverage": true,
            "direct_file_write_matcher_coverage": true,
            "host_capabilities": {
                "stdio_mcp": true,
                "http_mcp": false,
                "session_start_hook": true,
                "pre_tool_hook": true,
                "post_tool_hook": true,
                "user_prompt_submit_hook": true,
                "stop_hook": true,
                "rule_file_support": true,
                "project_local_configuration": true,
            },
            "required_hook_phases": DETECTIVE_REQUIRED_HOOK_PHASES,
            "missing_required_hooks": [],
            "prompt_capture": true,
            "files": [],
            "host_hook_commands": host_hook_commands,
            "hook_root_resolution": {
                "basis": "git_work_tree",
                "all_cwd_independent": true,
                "all_subdirectory_safe": true,
                "overall_status": "ok",
                "phases": root_phases,
            },
            "hook_path_safety": {
                "overall_status": "ok",
                "all_cwd_independent": true,
                "all_subdirectory_safe": true,
                "commands": safety_commands,
            },
            "commands": {
                "session_start": detective_policy_command("session-start"),
                "pre_tool": detective_policy_command("pre-tool"),
                "post_tool": detective_policy_command("post-tool"),
                "prompt_capture": detective_policy_command("prompt-capture"),
                "stop": detective_policy_command("stop"),
            },
        })
    }

    fn valid_owner_bound_capability() -> Value {
        let mut capability = valid_capability();
        let mut files = capability["host_hook_commands"]
            .as_array()
            .expect("commands")
            .iter()
            .map(|command| {
                json!({
                    "kind": "host_hook_wrapper",
                    "path": command["expected_phase_wrapper_path"],
                    "status": "unchanged",
                    "content_hash": "wrapper-hash",
                    "ownership": "managed_script",
                    "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                    "executable_required": true,
                    "managed_script_command": format!(
                        "volicord _hook {} --repo /repo --connection conn_owner --guard-installation guard_owner --host codex --integration-profile detective --policy-hash policy-hash --host-output codex",
                        hook_command_name(
                            command["policy_key"].as_str().expect("policy key")
                        )
                        .expect("command name")
                    ),
                    "host_kind": "codex",
                    "phase": command["policy_key"],
                    "purpose": "detective_guard",
                    "connection_id": "conn_owner",
                    "guard_installation_id": "guard_owner",
                    "policy_hash": "policy-hash",
                    "host_output": "codex",
                })
            })
            .collect::<Vec<_>>();
        files.extend([
            json!({
                "kind": "volicord_policy",
                "path": "/repo/.volicord/policy.json",
                "status": "unchanged",
                "content_hash": "policy-file-hash",
                "ownership": "managed_json",
            }),
            json!({
                "kind": "host_hook_dispatch",
                "path": "/repo/.codex/hooks/volicord-dispatch.sh",
                "status": "unchanged",
                "content_hash": "dispatch-hash",
                "ownership": "managed_script",
                "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                "executable_required": true,
                "managed_script_role": "codex_dispatch",
                "host_kind": "codex",
                "phase": "dispatch",
            }),
            json!({
                "kind": "host_hook_config",
                "path": "/repo/.codex/hooks.json",
                "status": "unchanged",
                "content_hash": "config-hash",
                "ownership": "managed_json",
            }),
            json!({
                "kind": "host_rule_instruction",
                "path": "/repo/.codex/rules/volicord.rules",
                "status": "unchanged",
                "content_hash": "rule-hash",
                "ownership": "managed_block",
                "managed_marker_start": "# BEGIN VOLICORD MANAGED CODEX RULES",
                "managed_marker_end": "# END VOLICORD MANAGED CODEX RULES",
            }),
        ]);
        capability["files"] = Value::Array(files);
        capability
    }

    fn owner_binding() -> HostHookCapabilityOwnerBinding<'static> {
        HostHookCapabilityOwnerBinding {
            row_host_kind: "codex",
            row_guard_mode: "detective",
            row_guard_installation_id: "guard_owner",
            connection_internal_id: "conn_owner",
            connection_host_kind: "codex",
            connection_intent: "personal",
            project_repo_root: Some(Path::new("/repo")),
            project_git_info_exclude_path: None,
        }
    }

    #[test]
    fn accepts_exact_v2_shape() {
        let capability = valid_capability();
        assert!(host_capabilities_have_exact_shape(
            &capability["host_capabilities"]
        ));
        assert!(json_array_all(
            &capability["host_hook_commands"],
            host_hook_command_has_exact_shape
        ));
        assert!(hook_root_resolution_has_exact_shape(
            &capability["hook_root_resolution"]
        ));
        assert!(hook_path_safety_has_exact_shape(
            &capability["hook_path_safety"]
        ));
        assert!(hook_safety_projections_are_coherent(
            &capability,
            capability["host_hook_commands"]
                .as_array()
                .expect("commands")
        ));
        assert_eq!(
            string_array_set(&capability["required_hook_phases"]),
            Some(DETECTIVE_REQUIRED_HOOK_PHASES.iter().copied().collect())
        );
        for command in capability["host_hook_commands"]
            .as_array()
            .expect("commands")
        {
            let phase = command["phase"].as_str().expect("phase");
            assert_eq!(
                command["policy_key"].as_str(),
                policy_key_for_capability_phase(phase)
            );
            assert_eq!(command["purpose"], "detective_guard");
            assert!(host_hook_command_shape_is_coherent(command));
        }
        assert!(host_hook_capability_has_coherent_v2_semantics(&capability));
        assert!(host_hook_capability_has_exact_v2_shape(&capability));
    }

    #[test]
    fn commands_are_a_closed_five_phase_map() {
        let mut capability = valid_capability();
        capability["commands"]["unexpected"] = json!({"command": "volicord", "args": []});
        assert!(!host_hook_capability_has_exact_v2_shape(&capability));

        let mut capability = valid_capability();
        capability["commands"]["pre_tool"]["unexpected"] = json!(true);
        assert!(!host_hook_capability_has_exact_v2_shape(&capability));

        let mut capability = valid_capability();
        capability["commands"]["pre_tool"]["args"] = json!(["_hook", 7]);
        assert!(!host_hook_capability_has_exact_v2_shape(&capability));
    }

    #[test]
    fn detective_phase_and_adapter_semantics_are_closed() {
        let mut missing_command = valid_capability();
        missing_command["host_hook_commands"]
            .as_array_mut()
            .expect("command array")
            .pop();
        assert!(!host_hook_capability_has_exact_v2_shape(&missing_command));

        let mut duplicate_required = valid_capability();
        duplicate_required["required_hook_phases"]
            .as_array_mut()
            .expect("phase array")
            .push(json!("stop_hook"));
        assert!(!host_hook_capability_has_exact_v2_shape(
            &duplicate_required
        ));

        let mut contradictory_adapter = valid_capability();
        contradictory_adapter["native_host_output_adapter"] = json!("none");
        assert!(!host_hook_capability_has_exact_v2_shape(
            &contradictory_adapter
        ));
    }

    #[test]
    fn complete_degraded_and_record_phase_shapes_are_distinct_and_valid() {
        let mut degraded = valid_capability();
        degraded["missing_required_hooks"] = json!(["pre_tool_hook"]);
        degraded["host_capabilities"]["pre_tool_hook"] = json!(false);
        degraded["host_hook_commands"]
            .as_array_mut()
            .expect("command array")
            .retain(|entry| entry["phase"] != "pre_tool_hook");
        degraded["hook_root_resolution"]["phases"]
            .as_array_mut()
            .expect("root phases")
            .retain(|entry| entry["phase"] != "pre_tool_hook");
        degraded["hook_path_safety"]["commands"]
            .as_array_mut()
            .expect("safety commands")
            .retain(|entry| entry["phase"] != "pre_tool_hook");
        assert!(host_hook_capability_has_exact_v2_shape(&degraded));

        let mut record = valid_capability();
        record["selected_profile"] = json!("record");
        record["required_hook_phases"] = json!([]);
        record["missing_required_hooks"] = json!([]);
        record["prompt_capture"] = json!(false);
        record["host_hook_commands"] = json!([]);
        record["hook_root_resolution"] = Value::Null;
        record["hook_path_safety"] = Value::Null;
        assert!(host_hook_capability_has_exact_v2_shape(&record));

        let mut record_stop = valid_capability();
        record_stop["selected_profile"] = json!("record");
        record_stop["required_hook_phases"] = json!([]);
        record_stop["missing_required_hooks"] = json!([]);
        record_stop["prompt_capture"] = json!(false);
        record_stop["host_hook_commands"]
            .as_array_mut()
            .expect("command array")
            .retain(|entry| entry["phase"] == "stop_hook");
        record_stop["host_hook_commands"][0]["purpose"] =
            json!("final_output_authority_disclosure");
        record_stop["hook_root_resolution"]["phases"]
            .as_array_mut()
            .expect("root phases")
            .retain(|entry| entry["phase"] == "stop_hook");
        record_stop["hook_path_safety"]["commands"]
            .as_array_mut()
            .expect("safety commands")
            .retain(|entry| entry["phase"] == "stop_hook");
        assert!(host_hook_capability_has_exact_v2_shape(&record_stop));
    }

    #[test]
    fn generated_files_are_a_closed_ownership_tagged_union() {
        let mut capability = valid_capability();
        capability["files"] = json!([{
            "kind": "host_mcp_config",
            "path": ".host/config.json",
            "status": "unchanged",
            "content_hash": "hash",
            "ownership": "managed_json",
        }]);
        assert!(host_hook_capability_has_exact_v2_shape(&capability));

        capability["files"][0]["unexpected"] = json!(true);
        assert!(!host_hook_capability_has_exact_v2_shape(&capability));

        let mut contradictory_script = valid_capability();
        contradictory_script["files"] = json!([{
            "kind": "host_hook_dispatch",
            "path": ".codex/hooks/volicord-dispatch.sh",
            "status": "unchanged",
            "content_hash": "hash",
            "ownership": "managed_script",
            "managed_marker": "managed-by-volicord",
            "executable_required": true,
            "managed_script_role": "codex_dispatch",
            "managed_script_command": "volicord _hook",
            "host_kind": "codex",
            "phase": "dispatch",
        }]);
        assert!(!host_hook_capability_has_exact_v2_shape(
            &contradictory_script
        ));
    }

    #[test]
    fn managed_script_inventory_is_bound_to_owner_policy_host_phase_and_path() {
        let valid = valid_owner_bound_capability();
        assert!(host_hook_capability_matches_owner_binding(
            &valid,
            owner_binding()
        ));

        let mut cases = Vec::new();

        let mut capability = valid.clone();
        capability["files"][0]["connection_id"] = json!("conn_other");
        cases.push(("connection_id", capability));

        let mut capability = valid.clone();
        capability["files"][0]["guard_installation_id"] = json!("guard_other");
        cases.push(("guard_installation_id", capability));

        let mut capability = valid.clone();
        capability["files"][0]["policy_hash"] = json!("other-policy");
        cases.push(("policy_hash", capability));

        let mut capability = valid.clone();
        capability["files"][0]["host_output"] = json!("claude-code");
        cases.push(("host_output", capability));

        let mut capability = valid.clone();
        capability["native_host_output_adapter"] = json!("none");
        capability["final_output_authority_disclosure_implementation_available"] = json!(false);
        capability["native_host_output_adapter_config_verified"] = json!(false);
        cases.push(("top_adapter_to_wrapper_output", capability));

        let mut capability = valid.clone();
        capability["files"][0]["host_kind"] = json!("claude-code");
        cases.push(("host_kind", capability));

        let mut capability = valid.clone();
        capability["files"][0]["purpose"] = json!("final_output_authority_disclosure");
        cases.push(("purpose", capability));

        let mut capability = valid.clone();
        capability["files"][0]["phase"] = json!("pre_tool");
        cases.push(("phase", capability));

        let mut capability = valid.clone();
        capability["files"][0]["path"] = json!("/repo/arbitrary.sh");
        cases.push(("path", capability));

        for (name, kind, path) in [
            (
                "relocated_policy",
                "volicord_policy",
                "/other/.volicord/policy.json",
            ),
            (
                "relocated_hook_config",
                "host_hook_config",
                "/other/.codex/hooks.json",
            ),
            (
                "relocated_rule",
                "host_rule_instruction",
                "/other/.codex/rules/volicord.rules",
            ),
        ] {
            let mut capability = valid.clone();
            let file = capability["files"]
                .as_array_mut()
                .expect("files")
                .iter_mut()
                .find(|file| file["kind"] == kind)
                .expect("inventory kind");
            file["path"] = json!(path);
            cases.push((name, capability));
        }

        let mut capability = valid.clone();
        for command in capability["host_hook_commands"]
            .as_array_mut()
            .expect("commands")
        {
            for field in ["expected_wrapper_path", "expected_phase_wrapper_path"] {
                let relocated = command[field]
                    .as_str()
                    .expect("command path")
                    .replacen("/repo/", "/other/", 1);
                command[field] = json!(relocated);
            }
        }
        for file in capability["files"].as_array_mut().expect("files") {
            if matches!(
                file["kind"].as_str(),
                Some("host_hook_wrapper" | "host_hook_dispatch")
            ) {
                let relocated = file["path"]
                    .as_str()
                    .expect("file path")
                    .replacen("/repo/", "/other/", 1);
                file["path"] = json!(relocated);
            }
        }
        cases.push(("coordinated_wrapper_dispatch_relocation", capability));

        let mut capability = valid.clone();
        let duplicate = capability["files"][0].clone();
        capability["files"]
            .as_array_mut()
            .expect("files")
            .push(duplicate);
        cases.push(("duplicate_path", capability));

        for kind in [
            "host_hook_wrapper",
            "host_hook_dispatch",
            "host_hook_config",
            "host_rule_instruction",
        ] {
            let mut capability = valid.clone();
            let files = capability["files"].as_array_mut().expect("files");
            let index = files
                .iter()
                .position(|file| file["kind"] == kind)
                .expect("required inventory kind");
            files.remove(index);
            cases.push((kind, capability));
        }

        for (field, capability) in cases {
            assert!(
                !host_hook_capability_matches_owner_binding(&capability, owner_binding()),
                "cross-bound {field} must fail closed"
            );
        }
    }

    #[test]
    fn generated_host_hook_command_contract_rejects_wrong_path_args_and_shape() {
        let valid = valid_owner_bound_capability();
        let mut cases = Vec::new();

        let mut capability = valid.clone();
        capability["host_hook_commands"][0]["command"] = json!(
            "sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-session-start.sh\"'"
        );
        cases.push(("phase_wrapper_instead_of_dispatch", capability));

        let mut capability = valid.clone();
        capability["host_hook_commands"][0]["command"] =
            json!(codex_hook_command("detective", "pre-tool"));
        cases.push(("wrong_dispatch_phase_argument", capability));

        let mut capability = valid.clone();
        capability["host_hook_commands"][0]["command_shape"] = json!("exec_form");
        capability["host_hook_commands"][0]["command"] =
            json!("/repo/.codex/hooks/volicord-dispatch.sh");
        capability["host_hook_commands"][0]["args"] = json!(["session-start"]);
        cases.push(("wrong_command_shape_and_args", capability));

        for (name, capability) in cases {
            assert!(
                host_hook_capability_has_exact_v2_shape(&capability),
                "the {name} case must remain structurally exact so owner validation is exercised"
            );
            assert!(
                !host_hook_capability_matches_owner_binding(&capability, owner_binding()),
                "the generated command contract must reject {name}"
            );
        }

        let valid_claude = json!({
            "command_shape": "exec_form",
            "command": "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh",
            "args": [],
        });
        assert!(host_hook_command_matches_generated_contract(
            &valid_claude,
            "claude_code",
            "detective",
            "pre-tool",
        ));
        for (field, value) in [
            (
                "command",
                json!("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-post-tool.sh"),
            ),
            ("args", json!(["pre-tool"])),
            ("command_shape", json!("shell_command_string")),
        ] {
            let mut candidate = valid_claude.clone();
            candidate[field] = value;
            assert!(
                !host_hook_command_matches_generated_contract(
                    &candidate,
                    "claude_code",
                    "detective",
                    "pre-tool",
                ),
                "Claude generated command contract must reject {field} mutation"
            );
        }
    }

    #[test]
    fn policy_commands_and_wrapper_commands_are_bound_to_owner_coordinates() {
        let valid = valid_owner_bound_capability();
        let mut cases = Vec::new();
        for (name, index, replacement) in [
            ("phase", 1, "post-tool"),
            ("repo", 3, "/other"),
            ("connection", 5, "conn_other"),
            ("guard_installation", 7, "guard_other"),
            ("host", 9, "claude-code"),
            ("profile", 11, "record"),
            ("output_flag", 12, "--output"),
            ("output_format", 13, "volicord-json"),
        ] {
            let mut capability = valid.clone();
            capability["commands"]["pre_tool"]["args"][index] = json!(replacement);
            cases.push((name, capability));
        }

        let mut capability = valid.clone();
        capability["commands"]["pre_tool"]["command"] = json!("other-volicord");
        cases.push(("inconsistent_executable", capability));

        let mut capability = valid.clone();
        capability["files"][0]["managed_script_command"] = json!("volicord _hook unexpected");
        cases.push(("managed_wrapper_command", capability));

        let mut capability = valid.clone();
        for command in capability["commands"]
            .as_object_mut()
            .expect("policy commands")
            .values_mut()
        {
            command["command"] = json!("other-volicord");
        }
        cases.push(("wrapper_executable_cross_binding", capability));

        for (name, capability) in cases {
            assert!(
                host_hook_capability_has_exact_v2_shape(&capability),
                "the {name} case must remain structurally exact so owner validation is exercised"
            );
            assert!(
                !host_hook_capability_matches_owner_binding(&capability, owner_binding()),
                "cross-bound {name} must fail closed"
            );
        }
    }

    #[test]
    fn projectless_owner_rejects_all_repository_inventory_including_git_exclude() {
        let mut capability = valid_owner_bound_capability();
        capability["selected_profile"] = json!("record");
        capability["required_hook_phases"] = json!([]);
        capability["missing_required_hooks"] = json!([]);
        capability["prompt_capture"] = json!(false);
        capability["files"] = json!([]);
        capability["host_hook_commands"] = json!([]);
        capability["hook_root_resolution"] = Value::Null;
        capability["hook_path_safety"] = Value::Null;
        let binding = HostHookCapabilityOwnerBinding {
            row_host_kind: "codex",
            row_guard_mode: "record",
            row_guard_installation_id: "guard_owner",
            connection_internal_id: "conn_owner",
            connection_host_kind: "codex",
            connection_intent: "personal",
            project_repo_root: None,
            project_git_info_exclude_path: None,
        };
        assert!(host_hook_capability_matches_owner_binding(
            &capability,
            binding
        ));

        capability["files"] = json!([{
            "kind": "git_info_exclude",
            "path": "/repo/.git/info/exclude",
            "status": "unchanged",
            "content_hash": "exclude-hash",
            "ownership": "managed_block",
            "managed_marker_start": "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES",
            "managed_marker_end": "# END VOLICORD MANAGED LOCAL EXCLUDES",
        }]);
        assert!(!host_hook_capability_matches_owner_binding(
            &capability,
            binding
        ));
    }

    #[test]
    fn git_exclude_inventory_requires_the_owner_resolved_common_git_path() {
        let mut capability = valid_owner_bound_capability();
        capability["files"]
            .as_array_mut()
            .expect("files")
            .push(json!({
                "kind": "git_info_exclude",
                "path": "/git/common/info/exclude",
                "status": "unchanged",
                "content_hash": "exclude-hash",
                "ownership": "managed_block",
                "managed_marker_start": "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES",
                "managed_marker_end": "# END VOLICORD MANAGED LOCAL EXCLUDES",
            }));
        let binding = HostHookCapabilityOwnerBinding {
            project_git_info_exclude_path: Some(Path::new("/git/common/info/exclude")),
            ..owner_binding()
        };
        assert!(host_hook_capability_matches_owner_binding(
            &capability,
            binding
        ));

        let git_exclude = capability["files"]
            .as_array_mut()
            .expect("files")
            .last_mut()
            .expect("git exclude");
        git_exclude["path"] = json!("/arbitrary/info/exclude");
        assert!(!host_hook_capability_matches_owner_binding(
            &capability,
            binding
        ));
    }
}
