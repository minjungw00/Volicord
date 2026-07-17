use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use serde::Deserialize;
use serde_json::Value;

/// Exact stored capability schema for the Codex record Guard workflow.
pub const HOST_HOOK_CAPABILITY_SCHEMA: &str = "volicord-host-hook-capability";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostHookCapability {
    schema: String,
    policy_hash: String,
    selected_profile: String,
    connection_intent: String,
    direct_file_write_matcher_coverage: bool,
    host_capabilities: CodexRecordCapabilities,
    files: Vec<ManagedCapabilityFile>,
    commands: GuardCommands,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexRecordCapabilities {
    stdio_mcp: bool,
    pre_tool_hook: bool,
    post_tool_hook: bool,
    user_prompt_submit_hook: bool,
    rule_file_support: bool,
    project_local_configuration: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardCommands {
    pre_tool: GuardCommand,
    post_tool: GuardCommand,
    prompt_capture: GuardCommand,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardCommand {
    command: String,
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedCapabilityFile {
    kind: String,
    path: String,
    status: String,
    content_hash: String,
    ownership: String,
    #[serde(default)]
    managed_marker_start: Option<String>,
    #[serde(default)]
    managed_marker_end: Option<String>,
    #[serde(default)]
    managed_marker: Option<String>,
    #[serde(default)]
    executable_required: Option<bool>,
    #[serde(default)]
    managed_script_role: Option<String>,
    #[serde(default)]
    managed_script_command: Option<String>,
    #[serde(default)]
    host_kind: Option<String>,
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    connection_id: Option<String>,
    #[serde(default)]
    guard_installation_id: Option<String>,
    #[serde(default)]
    policy_hash: Option<String>,
    #[serde(default)]
    host_output: Option<String>,
}

/// Returns whether a decoded capability has the exact Codex record shape.
pub fn host_hook_capability_has_exact_current_shape(value: &Value) -> bool {
    decode_capability(value).is_some()
}

/// Owning row and Agent Connection facts required before a stored capability may be consumed.
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

/// One generated Guard artifact coordinate decoded from an exact capability record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostHookManagedArtifactCoordinate {
    /// Normalized absolute artifact path.
    pub path: String,
    /// Raw lowercase hexadecimal SHA-256 of the expected bytes.
    pub digest: String,
}

/// Decodes the complete generated Guard artifact inventory from an exact capability record.
pub fn host_hook_capability_managed_artifacts(
    value: &Value,
) -> Option<Vec<HostHookManagedArtifactCoordinate>> {
    let capability = decode_capability(value)?;
    Some(
        capability
            .files
            .into_iter()
            .map(|file| HostHookManagedArtifactCoordinate {
                path: file.path,
                digest: file
                    .content_hash
                    .strip_prefix("sha256:")
                    .expect("exact capability hashes are prefixed")
                    .to_owned(),
            })
            .collect(),
    )
}

/// Returns whether an exact capability is bound to the supported Codex record owner facts.
pub fn host_hook_capability_matches_owner_binding(
    value: &Value,
    binding: HostHookCapabilityOwnerBinding<'_>,
) -> bool {
    let Some(capability) = decode_capability(value) else {
        return false;
    };
    binding.row_host_kind == "codex"
        && binding.connection_host_kind == "codex"
        && binding.row_guard_mode == "record"
        && capability.connection_intent == binding.connection_intent
        && !binding.row_guard_installation_id.is_empty()
        && !binding.connection_internal_id.is_empty()
        && owner_commands_match(&capability, binding)
        && owner_files_match(&capability.files, binding, &capability.policy_hash)
}

fn decode_capability(value: &Value) -> Option<HostHookCapability> {
    let capability = serde_json::from_value::<HostHookCapability>(value.clone()).ok()?;
    exact_capability_semantics(&capability).then_some(capability)
}

fn exact_capability_semantics(capability: &HostHookCapability) -> bool {
    let host = &capability.host_capabilities;
    capability.schema == HOST_HOOK_CAPABILITY_SCHEMA
        && canonical_sha256(&capability.policy_hash)
        && capability.selected_profile == "record"
        && matches!(capability.connection_intent.as_str(), "personal" | "shared")
        && {
            let _matcher_coverage_is_explicit = capability.direct_file_write_matcher_coverage;
            true
        }
        && host.stdio_mcp
        && host.pre_tool_hook
        && host.post_tool_hook
        && host.user_prompt_submit_hook
        && host.rule_file_support
        && host.project_local_configuration
        && commands_have_exact_semantics(&capability.commands, &capability.policy_hash)
        && inventory_has_exact_semantics(
            &capability.files,
            Path::new(&capability.commands.pre_tool.args[3]),
        )
}

fn commands_have_exact_semantics(commands: &GuardCommands, policy_hash: &str) -> bool {
    let commands = [
        (&commands.pre_tool, "pre-tool"),
        (&commands.post_tool, "post-tool"),
        (&commands.prompt_capture, "prompt-capture"),
    ];
    let Some(first) = commands.first().map(|(command, _)| command) else {
        return false;
    };
    commands.iter().all(|(command, phase)| {
        command.command == first.command
            && normalized_absolute_path(Path::new(&command.command))
            && command_args_have_exact_shape(command, phase, policy_hash)
    })
}

fn command_args_have_exact_shape(command: &GuardCommand, phase: &str, policy_hash: &str) -> bool {
    command.args.len() == 16
        && command.args[0] == "_hook"
        && command.args[1] == phase
        && command.args[2] == "--repo"
        && normalized_absolute_path(Path::new(&command.args[3]))
        && command.args[4] == "--connection"
        && !command.args[5].trim().is_empty()
        && command.args[6] == "--guard-installation"
        && !command.args[7].trim().is_empty()
        && command.args[8] == "--host"
        && command.args[9] == "codex"
        && command.args[10] == "--integration-profile"
        && command.args[11] == "record"
        && command.args[12] == "--policy-hash"
        && command.args[13] == policy_hash
        && command.args[14] == "--host-output"
        && command.args[15] == "codex"
}

fn inventory_has_exact_semantics(files: &[ManagedCapabilityFile], repo_root: &Path) -> bool {
    let mut kind_counts = BTreeMap::<&str, usize>::new();
    let mut paths = BTreeSet::new();
    let mut wrapper_phases = BTreeSet::new();
    for file in files {
        if !file_has_closed_semantics(file)
            || !inventory_path_matches(file, repo_root)
            || !paths.insert(file.path.as_str())
        {
            return false;
        }
        *kind_counts.entry(file.kind.as_str()).or_default() += 1;
        if file.kind == "host_hook_wrapper" {
            let Some(phase) = file.phase.as_deref() else {
                return false;
            };
            if !wrapper_phases.insert(phase) {
                return false;
            }
        }
    }
    [
        ("agents_managed_block", 1),
        ("volicord_policy", 1),
        ("host_hook_config", 1),
        ("host_hook_dispatch", 1),
        ("host_hook_wrapper", 3),
        ("host_rule_instruction", 1),
    ]
    .into_iter()
    .all(|(kind, count)| kind_counts.get(kind) == Some(&count))
        && kind_counts
            .get("git_info_exclude")
            .is_none_or(|count| *count == 1)
        && kind_counts.len() == 6 + usize::from(kind_counts.contains_key("git_info_exclude"))
        && wrapper_phases == BTreeSet::from(["pre_tool", "post_tool", "prompt_capture"])
}

fn inventory_path_matches(file: &ManagedCapabilityFile, repo_root: &Path) -> bool {
    let path = Path::new(&file.path);
    match file.kind.as_str() {
        "agents_managed_block" => path == repo_root.join("AGENTS.md"),
        "volicord_policy" => path == repo_root.join(".volicord/policy.json"),
        "host_hook_config" => path == repo_root.join(".codex/hooks.json"),
        "host_hook_dispatch" => path == repo_root.join(".codex/hooks/volicord-dispatch.sh"),
        "host_hook_wrapper" => file.phase.as_deref().is_some_and(|phase| {
            let command_name = match phase {
                "pre_tool" => "pre-tool",
                "post_tool" => "post-tool",
                "prompt_capture" => "prompt-capture",
                _ => return false,
            };
            path == repo_root.join(format!(".codex/hooks/volicord-{command_name}.sh"))
        }),
        "host_rule_instruction" => path == repo_root.join(".codex/rules/volicord.rules"),
        "git_info_exclude" => true,
        _ => false,
    }
}

fn file_has_closed_semantics(file: &ManagedCapabilityFile) -> bool {
    let status_is_known = matches!(
        file.status.as_str(),
        "planned_create" | "planned_update" | "unchanged" | "created" | "updated"
    );
    let optional_strings_are_nonempty = [
        file.managed_marker_start.as_deref(),
        file.managed_marker_end.as_deref(),
        file.managed_marker.as_deref(),
        file.managed_script_role.as_deref(),
        file.managed_script_command.as_deref(),
        file.host_kind.as_deref(),
        file.phase.as_deref(),
        file.purpose.as_deref(),
        file.connection_id.as_deref(),
        file.guard_installation_id.as_deref(),
        file.policy_hash.as_deref(),
        file.host_output.as_deref(),
    ]
    .into_iter()
    .flatten()
    .all(|value| !value.trim().is_empty());
    let no_owner_coordinates = || {
        file.host_kind.is_none()
            && file.phase.is_none()
            && file.purpose.is_none()
            && file.connection_id.is_none()
            && file.guard_installation_id.is_none()
            && file.policy_hash.is_none()
            && file.host_output.is_none()
    };
    let kind_is_coherent = match file.kind.as_str() {
        "volicord_policy" | "host_hook_config" => {
            file.ownership == "managed_json"
                && file.managed_marker_start.is_none()
                && file.managed_marker_end.is_none()
                && file.managed_marker.is_none()
                && file.executable_required.is_none()
                && file.managed_script_role.is_none()
                && file.managed_script_command.is_none()
                && no_owner_coordinates()
        }
        "agents_managed_block" | "git_info_exclude" | "host_rule_instruction" => {
            file.ownership == "managed_block"
                && file.managed_marker_start.is_some()
                && file.managed_marker_end.is_some()
                && file.managed_marker.is_none()
                && file.executable_required.is_none()
                && file.managed_script_role.is_none()
                && file.managed_script_command.is_none()
                && no_owner_coordinates()
        }
        "host_hook_dispatch" => {
            file.ownership == "managed_script"
                && file.managed_marker.is_some()
                && file.executable_required == Some(true)
                && file.managed_script_role.as_deref() == Some("codex_dispatch")
                && file.managed_script_command.is_none()
                && file.host_kind.as_deref() == Some("codex")
                && file.phase.as_deref() == Some("dispatch")
                && file.purpose.is_none()
                && file.connection_id.is_none()
                && file.guard_installation_id.is_none()
                && file.policy_hash.is_none()
                && file.host_output.is_none()
        }
        "host_hook_wrapper" => {
            file.ownership == "managed_script"
                && file.managed_marker.is_some()
                && file.executable_required == Some(true)
                && file.managed_script_role.is_none()
                && file.managed_script_command.is_some()
                && file.host_kind.as_deref() == Some("codex")
                && matches!(
                    file.phase.as_deref(),
                    Some("pre_tool" | "post_tool" | "prompt_capture")
                )
                && file.purpose.as_deref() == Some("guard")
                && file.connection_id.is_some()
                && file.guard_installation_id.is_some()
                && file.policy_hash.as_deref().is_some_and(canonical_sha256)
                && file.host_output.as_deref() == Some("codex")
        }
        _ => false,
    };
    normalized_absolute_path(Path::new(&file.path))
        && canonical_sha256(&file.content_hash)
        && status_is_known
        && optional_strings_are_nonempty
        && kind_is_coherent
}

fn owner_commands_match(
    capability: &HostHookCapability,
    binding: HostHookCapabilityOwnerBinding<'_>,
) -> bool {
    let Some(repo_root) = binding.project_repo_root else {
        return false;
    };
    let commands = [
        (&capability.commands.pre_tool, "pre-tool"),
        (&capability.commands.post_tool, "post-tool"),
        (&capability.commands.prompt_capture, "prompt-capture"),
    ];
    let Some(repo_root) = repo_root.to_str() else {
        return false;
    };
    commands.into_iter().all(|(command, phase)| {
        let expected = [
            "_hook",
            phase,
            "--repo",
            repo_root,
            "--connection",
            binding.connection_internal_id,
            "--guard-installation",
            binding.row_guard_installation_id,
            "--host",
            "codex",
            "--integration-profile",
            "record",
            "--policy-hash",
            capability.policy_hash.as_str(),
            "--host-output",
            "codex",
        ];
        command.args.iter().map(String::as_str).eq(expected)
    })
}

fn owner_files_match(
    files: &[ManagedCapabilityFile],
    binding: HostHookCapabilityOwnerBinding<'_>,
    policy_hash: &str,
) -> bool {
    let Some(repo_root) = binding.project_repo_root else {
        return files.is_empty();
    };
    if !normalized_absolute_path(repo_root) {
        return false;
    }
    files.iter().all(|file| {
        let path = Path::new(&file.path);
        let path_matches = match file.kind.as_str() {
            "volicord_policy" => path == repo_root.join(".volicord/policy.json"),
            "host_hook_config" => path == repo_root.join(".codex/hooks.json"),
            "host_hook_dispatch" => path == repo_root.join(".codex/hooks/volicord-dispatch.sh"),
            "host_hook_wrapper" => file.phase.as_deref().is_some_and(|phase| {
                let command_name = match phase {
                    "pre_tool" => "pre-tool",
                    "post_tool" => "post-tool",
                    "prompt_capture" => "prompt-capture",
                    _ => return false,
                };
                path == repo_root.join(format!(".codex/hooks/volicord-{command_name}.sh"))
            }),
            "host_rule_instruction" => path == repo_root.join(".codex/rules/volicord.rules"),
            "agents_managed_block" => path == repo_root.join("AGENTS.md"),
            "git_info_exclude" => binding.project_git_info_exclude_path == Some(path),
            "host_mcp_config" => false,
            _ => false,
        };
        path_matches
            && (file.kind != "host_hook_wrapper"
                || (file.connection_id.as_deref() == Some(binding.connection_internal_id)
                    && file.guard_installation_id.as_deref()
                        == Some(binding.row_guard_installation_id)
                    && file.policy_hash.as_deref() == Some(policy_hash)))
    })
}

fn canonical_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn normalized_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, Component::CurDir | Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{
        host_hook_capability_has_exact_current_shape, host_hook_capability_matches_owner_binding,
        HostHookCapabilityOwnerBinding, HOST_HOOK_CAPABILITY_SCHEMA,
    };

    const POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    const CONTENT_HASH: &str =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    #[cfg(not(windows))]
    const REPO_ROOT: &str = "/workspace/repo";
    #[cfg(windows)]
    const REPO_ROOT: &str = "C:/workspace/repo";
    #[cfg(not(windows))]
    const VOLICORD_COMMAND: &str = "/usr/local/bin/volicord";
    #[cfg(windows)]
    const VOLICORD_COMMAND: &str = "C:/Volicord/volicord.exe";

    fn capability() -> Value {
        let command = |phase: &str| {
            json!({
                "command": VOLICORD_COMMAND,
                "args": [
                    "_hook", phase,
                    "--repo", REPO_ROOT,
                    "--connection", "conn_a",
                    "--guard-installation", "guard_a",
                    "--host", "codex",
                    "--integration-profile", "record",
                    "--policy-hash", POLICY_HASH,
                    "--host-output", "codex"
                ]
            })
        };
        let wrapper = |phase: &str, command_name: &str| {
            json!({
                "kind": "host_hook_wrapper",
                "path": format!("{REPO_ROOT}/.codex/hooks/volicord-{command_name}.sh"),
                "status": "unchanged",
                "content_hash": CONTENT_HASH,
                "ownership": "managed_script",
                "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                "executable_required": true,
                "managed_script_command": format!("exec {VOLICORD_COMMAND}"),
                "host_kind": "codex",
                "phase": phase,
                "purpose": "guard",
                "connection_id": "conn_a",
                "guard_installation_id": "guard_a",
                "policy_hash": POLICY_HASH,
                "host_output": "codex"
            })
        };
        let files = vec![
            json!({
                "kind": "agents_managed_block",
                "path": format!("{REPO_ROOT}/AGENTS.md"),
                "status": "unchanged",
                "content_hash": CONTENT_HASH,
                "ownership": "managed_block",
                "managed_marker_start": "# BEGIN VOLICORD MANAGED AGENT GUIDANCE",
                "managed_marker_end": "# END VOLICORD MANAGED AGENT GUIDANCE"
            }),
            json!({
                "kind": "volicord_policy",
                "path": format!("{REPO_ROOT}/.volicord/policy.json"),
                "status": "unchanged",
                "content_hash": CONTENT_HASH,
                "ownership": "managed_json"
            }),
            json!({
                "kind": "host_hook_config",
                "path": format!("{REPO_ROOT}/.codex/hooks.json"),
                "status": "unchanged",
                "content_hash": CONTENT_HASH,
                "ownership": "managed_json"
            }),
            json!({
                "kind": "host_hook_dispatch",
                "path": format!("{REPO_ROOT}/.codex/hooks/volicord-dispatch.sh"),
                "status": "unchanged",
                "content_hash": CONTENT_HASH,
                "ownership": "managed_script",
                "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                "executable_required": true,
                "managed_script_role": "codex_dispatch",
                "host_kind": "codex",
                "phase": "dispatch"
            }),
            wrapper("pre_tool", "pre-tool"),
            wrapper("post_tool", "post-tool"),
            wrapper("prompt_capture", "prompt-capture"),
            json!({
                "kind": "host_rule_instruction",
                "path": format!("{REPO_ROOT}/.codex/rules/volicord.rules"),
                "status": "unchanged",
                "content_hash": CONTENT_HASH,
                "ownership": "managed_block",
                "managed_marker_start": "# BEGIN VOLICORD MANAGED CODEX RULES",
                "managed_marker_end": "# END VOLICORD MANAGED CODEX RULES"
            }),
        ];
        json!({
            "schema": HOST_HOOK_CAPABILITY_SCHEMA,
            "policy_hash": POLICY_HASH,
            "selected_profile": "record",
            "connection_intent": "shared",
            "direct_file_write_matcher_coverage": false,
            "host_capabilities": {
                "stdio_mcp": true,
                "pre_tool_hook": true,
                "post_tool_hook": true,
                "user_prompt_submit_hook": true,
                "rule_file_support": true,
                "project_local_configuration": true
            },
            "files": files,
            "commands": {
                "pre_tool": command("pre-tool"),
                "post_tool": command("post-tool"),
                "prompt_capture": command("prompt-capture")
            }
        })
    }

    #[test]
    fn exact_record_capability_closes_every_nested_object() {
        let current = capability();
        assert!(host_hook_capability_has_exact_current_shape(&current));

        for pointer in ["/host_capabilities", "/commands/pre_tool"] {
            let mut unsupported = current.clone();
            unsupported
                .pointer_mut(pointer)
                .and_then(Value::as_object_mut)
                .expect("fixture object")
                .insert("removed_surface".to_owned(), Value::Bool(true));
            assert!(!host_hook_capability_has_exact_current_shape(&unsupported));
        }
    }

    #[test]
    fn exact_record_capability_rejects_removed_phases() {
        let mut unsupported = capability();
        unsupported["commands"]["stop"] = json!({"command": "volicord", "args": []});
        assert!(!host_hook_capability_has_exact_current_shape(&unsupported));
    }

    #[test]
    fn exact_record_capability_requires_unique_complete_inventory() {
        let current = capability();
        for mutation in ["missing", "duplicate", "relocated"] {
            let mut unsupported = current.clone();
            let files = unsupported["files"].as_array_mut().expect("fixture files");
            match mutation {
                "missing" => {
                    files.remove(0);
                }
                "duplicate" => files.push(files[0].clone()),
                "relocated" => {
                    files[0]["path"] = json!(format!("{REPO_ROOT}/elsewhere/AGENTS.md"));
                }
                _ => unreachable!(),
            }
            assert!(!host_hook_capability_has_exact_current_shape(&unsupported));
        }
    }

    #[test]
    fn exact_record_capability_requires_executable_only_for_dispatch_and_wrappers() {
        let current = capability();
        assert!(host_hook_capability_has_exact_current_shape(&current));

        let files = current["files"].as_array().expect("fixture files");
        let managed_script_indexes = files
            .iter()
            .enumerate()
            .filter_map(|(index, file)| {
                matches!(
                    file["kind"].as_str(),
                    Some("host_hook_dispatch" | "host_hook_wrapper")
                )
                .then_some(index)
            })
            .collect::<Vec<_>>();
        assert_eq!(managed_script_indexes.len(), 4);
        for index in managed_script_indexes {
            let mut unsupported = current.clone();
            unsupported["files"][index]["executable_required"] = Value::Bool(false);
            assert!(!host_hook_capability_has_exact_current_shape(&unsupported));
        }

        for index in files.iter().enumerate().filter_map(|(index, file)| {
            (file["ownership"].as_str() != Some("managed_script")).then_some(index)
        }) {
            let mut unsupported = current.clone();
            unsupported["files"][index]["executable_required"] = Value::Bool(true);
            assert!(!host_hook_capability_has_exact_current_shape(&unsupported));
        }
    }

    #[test]
    fn exact_record_capability_rejects_noncanonical_hashes_and_extra_args() {
        let current = capability();
        for pointer in ["/policy_hash", "/files/0/content_hash"] {
            let mut unsupported = current.clone();
            *unsupported.pointer_mut(pointer).expect("fixture hash") = json!("sha256:not-hex");
            assert!(!host_hook_capability_has_exact_current_shape(&unsupported));
        }
        let mut extra_arg = current;
        extra_arg["commands"]["pre_tool"]["args"]
            .as_array_mut()
            .expect("fixture args")
            .push(json!("--removed-option"));
        assert!(!host_hook_capability_has_exact_current_shape(&extra_arg));
    }

    #[test]
    fn owner_binding_checks_the_managed_command_identity() {
        let current = capability();
        let binding = HostHookCapabilityOwnerBinding {
            row_host_kind: "codex",
            row_guard_mode: "record",
            row_guard_installation_id: "guard_a",
            connection_internal_id: "conn_a",
            connection_host_kind: "codex",
            connection_intent: "shared",
            project_repo_root: Some(std::path::Path::new(REPO_ROOT)),
            project_git_info_exclude_path: None,
        };
        assert!(host_hook_capability_matches_owner_binding(
            &current, binding
        ));

        let mut wrong_connection = current;
        wrong_connection["commands"]["pre_tool"]["args"][5] = json!("conn_b");
        assert!(!host_hook_capability_matches_owner_binding(
            &wrong_connection,
            binding
        ));
    }
}
