use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::{Component, Path},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AgentConnectionId, GuardHookPhase, GuardInstallationId, HostKind, IntegrationProfile,
    IntegrationRevision, ProjectId,
};

/// Exact stored schema for a Volicord Guard installation manifest.
pub const GUARD_MANIFEST_SCHEMA: &str = "volicord-guard-manifest";

/// Canonical policy-content digest owned by a Guard plan and manifest.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct PolicyHash(String);

impl PolicyHash {
    /// Parses one canonical `sha256:<64-lowercase-hex>` policy digest.
    pub fn parse(value: impl Into<String>) -> Result<Self, GuardManifestError> {
        let value = value.into();
        if canonical_sha256(&value) {
            Ok(Self(value))
        } else {
            Err(GuardManifestError::InvalidPolicyHash)
        }
    }

    /// Returns the canonical digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the canonical digest string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// One exact generated Guard command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardCommand {
    pub command: String,
    pub args: Vec<String>,
}

/// Exact commands for every current Guard hook phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardCommandSet {
    pub pre_tool: GuardCommand,
    pub post_tool: GuardCommand,
    pub prompt_capture: GuardCommand,
}

impl GuardCommandSet {
    /// Returns the command for one typed phase.
    pub fn get(&self, phase: GuardHookPhase) -> &GuardCommand {
        match phase {
            GuardHookPhase::PreTool => &self.pre_tool,
            GuardHookPhase::PostTool => &self.post_tool,
            GuardHookPhase::PromptCapture => &self.prompt_capture,
        }
    }

    /// Returns the command for one exact policy key.
    pub fn get_by_key(&self, key: &str) -> Option<&GuardCommand> {
        match key {
            "pre_tool" => Some(&self.pre_tool),
            "post_tool" => Some(&self.post_tool),
            "prompt_capture" => Some(&self.prompt_capture),
            _ => None,
        }
    }

    /// Converts the phase-keyed command set to a deterministic map.
    pub fn to_map(&self) -> BTreeMap<String, GuardCommand> {
        GuardHookPhase::REQUIRED
            .into_iter()
            .map(|phase| (phase.as_str().to_owned(), self.get(phase).clone()))
            .collect()
    }
}

/// One exact Volicord-managed file expectation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManagedFileExpectation {
    pub kind: String,
    pub path: String,
    pub content_hash: String,
    pub ownership: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_marker_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_marker_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_marker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_script_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_script_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard_installation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_output: Option<String>,
}

/// Strict current Guard installation manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardManifest {
    pub schema: String,
    pub guard_installation_id: GuardInstallationId,
    pub connection_id: AgentConnectionId,
    pub project_id: ProjectId,
    pub host_kind: HostKind,
    pub integration_profile: IntegrationProfile,
    pub policy_hash: PolicyHash,
    pub integration_revision: IntegrationRevision,
    pub runtime_commands: GuardCommandSet,
    pub managed_files: Vec<ManagedFileExpectation>,
    pub required_hook_phases: Vec<GuardHookPhase>,
}

/// Owning row, Connection, and project facts required before a manifest may be consumed.
#[derive(Debug, Clone, Copy)]
pub struct GuardManifestOwnerBinding<'a> {
    pub row_guard_installation_id: &'a str,
    pub row_connection_id: &'a str,
    pub row_project_id: &'a str,
    pub connection_host_kind: &'a str,
    pub connection_integration_revision: &'a str,
    pub project_repo_root: &'a Path,
    pub project_git_info_exclude_path: Option<&'a Path>,
}

/// One generated Guard artifact coordinate decoded from an exact manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardManagedArtifactCoordinate {
    pub path: String,
    pub digest: String,
}

/// Guard manifest decoding or semantic validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardManifestError {
    InvalidJson,
    NonCanonicalJson,
    InvalidShape,
    InvalidPolicyHash,
}

impl fmt::Display for GuardManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidJson => "Guard manifest is not valid JSON",
            Self::NonCanonicalJson => "Guard manifest JSON is not canonical",
            Self::InvalidShape => "Guard manifest does not match the exact current contract",
            Self::InvalidPolicyHash => "Guard policy hash is not canonical",
        })
    }
}

impl Error for GuardManifestError {}

/// Strict-decodes canonical stored manifest JSON.
pub fn guard_manifest_from_json(text: &str) -> Result<GuardManifest, GuardManifestError> {
    let manifest =
        serde_json::from_str::<GuardManifest>(text).map_err(|_| GuardManifestError::InvalidJson)?;
    if !exact_manifest_semantics(&manifest) {
        return Err(GuardManifestError::InvalidShape);
    }
    if serde_json::to_string(&manifest).map_err(|_| GuardManifestError::InvalidJson)? != text {
        return Err(GuardManifestError::NonCanonicalJson);
    }
    Ok(manifest)
}

/// Returns whether a decoded value has the exact current Guard manifest shape and semantics.
pub fn guard_manifest_has_exact_current_shape(value: &Value) -> bool {
    serde_json::from_value::<GuardManifest>(value.clone())
        .ok()
        .is_some_and(|manifest| exact_manifest_semantics(&manifest))
}

/// Decodes an exact current Guard manifest value.
pub fn guard_manifest_from_value(value: &Value) -> Option<GuardManifest> {
    let manifest = serde_json::from_value::<GuardManifest>(value.clone()).ok()?;
    exact_manifest_semantics(&manifest).then_some(manifest)
}

/// Decodes the complete managed artifact inventory from an exact manifest.
pub fn guard_manifest_managed_artifacts(
    value: &Value,
) -> Option<Vec<GuardManagedArtifactCoordinate>> {
    let manifest = guard_manifest_from_value(value)?;
    Some(
        manifest
            .managed_files
            .into_iter()
            .map(|file| GuardManagedArtifactCoordinate {
                path: file.path,
                digest: file
                    .content_hash
                    .strip_prefix("sha256:")
                    .expect("exact manifest hashes are prefixed")
                    .to_owned(),
            })
            .collect(),
    )
}

/// Returns whether an exact manifest is bound to its stored owners.
pub fn guard_manifest_matches_owner_binding(
    value: &Value,
    binding: GuardManifestOwnerBinding<'_>,
) -> bool {
    let Some(manifest) = guard_manifest_from_value(value) else {
        return false;
    };
    manifest.guard_installation_id.as_str() == binding.row_guard_installation_id
        && manifest.connection_id.as_str() == binding.row_connection_id
        && manifest.project_id.as_str() == binding.row_project_id
        && manifest.host_kind.as_str() == binding.connection_host_kind
        && manifest.integration_revision.as_str() == binding.connection_integration_revision
        && owner_commands_match(&manifest, binding)
        && owner_files_match(&manifest, binding)
}

fn exact_manifest_semantics(manifest: &GuardManifest) -> bool {
    manifest.schema == GUARD_MANIFEST_SCHEMA
        && !manifest.guard_installation_id.as_str().trim().is_empty()
        && !manifest.connection_id.as_str().trim().is_empty()
        && !manifest.project_id.as_str().trim().is_empty()
        && manifest.host_kind == HostKind::Codex
        && manifest.integration_profile == IntegrationProfile::Record
        && canonical_sha256(manifest.policy_hash.as_str())
        && manifest.required_hook_phases == GuardHookPhase::REQUIRED
        && commands_have_exact_semantics(&manifest.runtime_commands, manifest.policy_hash.as_str())
        && manifest
            .runtime_commands
            .pre_tool
            .args
            .get(3)
            .is_some_and(|repo_root| {
                inventory_has_exact_semantics(&manifest.managed_files, Path::new(repo_root))
            })
}

fn commands_have_exact_semantics(commands: &GuardCommandSet, policy_hash: &str) -> bool {
    let first = &commands.pre_tool;
    normalized_absolute_path(Path::new(&first.command))
        && GuardHookPhase::REQUIRED.into_iter().all(|phase| {
            let command = commands.get(phase);
            command.command == first.command
                && command_args_have_exact_shape(command, phase, policy_hash)
        })
}

fn command_args_have_exact_shape(
    command: &GuardCommand,
    phase: GuardHookPhase,
    policy_hash: &str,
) -> bool {
    command.args.len() == 16
        && command.args[0] == "_hook"
        && command.args[1] == phase.command_name()
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

fn inventory_has_exact_semantics(files: &[ManagedFileExpectation], repo_root: &Path) -> bool {
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

fn inventory_path_matches(file: &ManagedFileExpectation, repo_root: &Path) -> bool {
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

fn file_has_closed_semantics(file: &ManagedFileExpectation) -> bool {
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
        && optional_strings_are_nonempty
        && kind_is_coherent
}

fn owner_commands_match(manifest: &GuardManifest, binding: GuardManifestOwnerBinding<'_>) -> bool {
    let Some(repo_root) = binding.project_repo_root.to_str() else {
        return false;
    };
    GuardHookPhase::REQUIRED.into_iter().all(|phase| {
        let command = manifest.runtime_commands.get(phase);
        let expected = [
            "_hook",
            phase.command_name(),
            "--repo",
            repo_root,
            "--connection",
            binding.row_connection_id,
            "--guard-installation",
            binding.row_guard_installation_id,
            "--host",
            "codex",
            "--integration-profile",
            "record",
            "--policy-hash",
            manifest.policy_hash.as_str(),
            "--host-output",
            "codex",
        ];
        command.args.iter().map(String::as_str).eq(expected)
    })
}

fn owner_files_match(manifest: &GuardManifest, binding: GuardManifestOwnerBinding<'_>) -> bool {
    if !normalized_absolute_path(binding.project_repo_root) {
        return false;
    }
    manifest.managed_files.iter().all(|file| {
        let path = Path::new(&file.path);
        let path_matches = match file.kind.as_str() {
            "volicord_policy" => binding.project_repo_root.join(".volicord/policy.json") == path,
            "host_hook_config" => binding.project_repo_root.join(".codex/hooks.json") == path,
            "host_hook_dispatch" => {
                binding
                    .project_repo_root
                    .join(".codex/hooks/volicord-dispatch.sh")
                    == path
            }
            "host_hook_wrapper" => file.phase.as_deref().is_some_and(|phase| {
                let command_name = match phase {
                    "pre_tool" => "pre-tool",
                    "post_tool" => "post-tool",
                    "prompt_capture" => "prompt-capture",
                    _ => return false,
                };
                binding
                    .project_repo_root
                    .join(format!(".codex/hooks/volicord-{command_name}.sh"))
                    == path
            }),
            "host_rule_instruction" => {
                binding
                    .project_repo_root
                    .join(".codex/rules/volicord.rules")
                    == path
            }
            "agents_managed_block" => binding.project_repo_root.join("AGENTS.md") == path,
            "git_info_exclude" => binding.project_git_info_exclude_path == Some(path),
            _ => false,
        };
        path_matches
            && (file.kind != "host_hook_wrapper"
                || (file.connection_id.as_deref() == Some(binding.row_connection_id)
                    && file.guard_installation_id.as_deref()
                        == Some(binding.row_guard_installation_id)
                    && file.policy_hash.as_deref() == Some(manifest.policy_hash.as_str())))
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
    use serde_json::json;

    use super::*;

    const HASH: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn policy_hash_rejects_noncanonical_values() {
        assert!(PolicyHash::parse(HASH).is_ok());
        assert!(PolicyHash::parse("sha256:not-hex").is_err());
    }

    #[test]
    fn manifest_decoder_rejects_unknown_members() {
        let value = json!({"schema": GUARD_MANIFEST_SCHEMA, "unknown": true});
        assert!(!guard_manifest_has_exact_current_shape(&value));
    }
}
