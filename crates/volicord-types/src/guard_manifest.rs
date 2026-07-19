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
        key.parse::<GuardHookPhase>()
            .ok()
            .map(|phase| self.get(phase))
    }

    /// Converts the phase-keyed command set to a deterministic map.
    pub fn to_map(&self) -> BTreeMap<String, GuardCommand> {
        GuardHookPhase::REQUIRED
            .into_iter()
            .map(|phase| (phase.as_str().to_owned(), self.get(phase).clone()))
            .collect()
    }
}

/// Number of arguments in the canonical hash-free Guard policy projection.
pub const GUARD_POLICY_COMMAND_ARGUMENT_COUNT: usize = 14;

/// Number of arguments in the canonical hash-bound Guard runtime projection.
pub const GUARD_RUNTIME_COMMAND_ARGUMENT_COUNT: usize = 16;

/// One exact projection of a typed generated Guard command invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardCommandProjection {
    Policy,
    Runtime,
}

/// An already-normalized absolute path retained exactly for a generated Guard command.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardCommandAbsolutePath(String);

impl GuardCommandAbsolutePath {
    /// Validates and retains one exact absolute command or repository path.
    pub fn parse(value: impl Into<String>) -> Result<Self, GuardCommandInvocationError> {
        let value = value.into();
        if value.as_bytes().contains(&0) || !normalized_absolute_path(Path::new(&value)) {
            return Err(GuardCommandInvocationError::InvalidAbsolutePath);
        }
        Ok(Self(value))
    }

    /// Validates and retains an absolute path without normalizing its spelling.
    pub fn from_path(path: &Path) -> Result<Self, GuardCommandInvocationError> {
        let value = path
            .to_str()
            .ok_or(GuardCommandInvocationError::InvalidAbsolutePath)?;
        Self::parse(value)
    }

    /// Returns the exact retained path text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the retained text as a native path.
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

/// Semantic owner of one generated Guard command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardCommandInvocation {
    pub executable: GuardCommandAbsolutePath,
    pub phase: GuardHookPhase,
    pub repo_root: GuardCommandAbsolutePath,
    pub connection_id: AgentConnectionId,
    pub guard_installation_id: GuardInstallationId,
    pub host_kind: HostKind,
    pub integration_profile: IntegrationProfile,
    pub policy_hash: Option<PolicyHash>,
    pub host_output: HostKind,
}

impl GuardCommandInvocation {
    /// Constructs one invocation after validating its opaque owner coordinates.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        executable: GuardCommandAbsolutePath,
        phase: GuardHookPhase,
        repo_root: GuardCommandAbsolutePath,
        connection_id: AgentConnectionId,
        guard_installation_id: GuardInstallationId,
        host_kind: HostKind,
        integration_profile: IntegrationProfile,
        policy_hash: Option<PolicyHash>,
        host_output: HostKind,
    ) -> Result<Self, GuardCommandInvocationError> {
        validate_command_identifier(connection_id.as_str())
            .map_err(|_| GuardCommandInvocationError::InvalidConnectionId)?;
        validate_command_identifier(guard_installation_id.as_str())
            .map_err(|_| GuardCommandInvocationError::InvalidGuardInstallationId)?;
        if policy_hash
            .as_ref()
            .is_some_and(|value| !canonical_sha256(value.as_str()))
        {
            return Err(GuardCommandInvocationError::InvalidPolicyHash);
        }
        Ok(Self {
            executable,
            phase,
            repo_root,
            connection_id,
            guard_installation_id,
            host_kind,
            integration_profile,
            policy_hash,
            host_output,
        })
    }

    /// Serializes the hash-free policy projection.
    pub fn to_policy_command(&self) -> GuardCommand {
        self.serialize(GuardCommandProjection::Policy)
            .expect("the policy projection never requires a policy hash")
    }

    /// Serializes the hash-bound runtime projection.
    pub fn to_runtime_command(&self) -> Result<GuardCommand, GuardCommandInvocationError> {
        self.serialize(GuardCommandProjection::Runtime)
    }

    /// Strictly parses one exact hash-free policy command.
    pub fn from_policy_command(
        command: &GuardCommand,
    ) -> Result<Self, GuardCommandInvocationError> {
        Self::parse(command, GuardCommandProjection::Policy)
    }

    /// Strictly parses one exact runtime command with a canonical policy hash.
    pub fn from_runtime_command(
        command: &GuardCommand,
    ) -> Result<Self, GuardCommandInvocationError> {
        Self::parse(command, GuardCommandProjection::Runtime)
    }

    /// Strictly parses a runtime command and requires one exact expected policy hash.
    pub fn from_runtime_command_with_policy_hash(
        command: &GuardCommand,
        expected_policy_hash: &PolicyHash,
    ) -> Result<Self, GuardCommandInvocationError> {
        let invocation = Self::from_runtime_command(command)?;
        if invocation.policy_hash.as_ref() != Some(expected_policy_hash) {
            return Err(GuardCommandInvocationError::PolicyHashMismatch);
        }
        Ok(invocation)
    }

    /// Returns whether all shared policy/runtime fields match, ignoring only the policy hash.
    pub fn fields_match_except_policy_hash(&self, other: &Self) -> bool {
        self.phase == other.phase && self.owner_fields_match_except_phase_and_policy_hash(other)
    }

    fn owner_fields_match_except_phase_and_policy_hash(&self, other: &Self) -> bool {
        self.executable == other.executable
            && self.repo_root == other.repo_root
            && self.connection_id == other.connection_id
            && self.guard_installation_id == other.guard_installation_id
            && self.host_kind == other.host_kind
            && self.integration_profile == other.integration_profile
            && self.host_output == other.host_output
    }

    fn serialize(
        &self,
        projection: GuardCommandProjection,
    ) -> Result<GuardCommand, GuardCommandInvocationError> {
        let capacity = match projection {
            GuardCommandProjection::Policy => GUARD_POLICY_COMMAND_ARGUMENT_COUNT,
            GuardCommandProjection::Runtime => GUARD_RUNTIME_COMMAND_ARGUMENT_COUNT,
        };
        let mut args = Vec::with_capacity(capacity);
        args.extend([
            "_hook".to_owned(),
            self.phase.command_name().to_owned(),
            "--repo".to_owned(),
            self.repo_root.as_str().to_owned(),
            "--connection".to_owned(),
            self.connection_id.as_str().to_owned(),
            "--guard-installation".to_owned(),
            self.guard_installation_id.as_str().to_owned(),
            "--host".to_owned(),
            self.host_kind.as_str().to_owned(),
            "--integration-profile".to_owned(),
            self.integration_profile.as_str().to_owned(),
        ]);
        if projection == GuardCommandProjection::Runtime {
            let policy_hash = self
                .policy_hash
                .as_ref()
                .ok_or(GuardCommandInvocationError::MissingPolicyHash)?;
            args.extend(["--policy-hash".to_owned(), policy_hash.as_str().to_owned()]);
        }
        args.extend([
            "--host-output".to_owned(),
            self.host_output.as_str().to_owned(),
        ]);
        Ok(GuardCommand {
            command: self.executable.as_str().to_owned(),
            args,
        })
    }

    fn parse(
        command: &GuardCommand,
        projection: GuardCommandProjection,
    ) -> Result<Self, GuardCommandInvocationError> {
        let executable = GuardCommandAbsolutePath::parse(command.command.clone())?;
        match projection {
            GuardCommandProjection::Policy => {
                let [hook, phase, repo_flag, repo_root, connection_flag, connection_id, installation_flag, guard_installation_id, host_flag, host_kind, profile_flag, integration_profile, output_flag, host_output] =
                    command.args.as_slice()
                else {
                    return Err(GuardCommandInvocationError::WrongArgumentCount);
                };
                if hook != "_hook"
                    || repo_flag != "--repo"
                    || connection_flag != "--connection"
                    || installation_flag != "--guard-installation"
                    || host_flag != "--host"
                    || profile_flag != "--integration-profile"
                    || output_flag != "--host-output"
                {
                    return Err(GuardCommandInvocationError::InvalidArgumentShape);
                }
                Self::from_parsed_fields(
                    executable,
                    phase,
                    repo_root,
                    connection_id,
                    guard_installation_id,
                    host_kind,
                    integration_profile,
                    None,
                    host_output,
                )
            }
            GuardCommandProjection::Runtime => {
                let [hook, phase, repo_flag, repo_root, connection_flag, connection_id, installation_flag, guard_installation_id, host_flag, host_kind, profile_flag, integration_profile, hash_flag, policy_hash, output_flag, host_output] =
                    command.args.as_slice()
                else {
                    return Err(GuardCommandInvocationError::WrongArgumentCount);
                };
                if hook != "_hook"
                    || repo_flag != "--repo"
                    || connection_flag != "--connection"
                    || installation_flag != "--guard-installation"
                    || host_flag != "--host"
                    || profile_flag != "--integration-profile"
                    || hash_flag != "--policy-hash"
                    || output_flag != "--host-output"
                {
                    return Err(GuardCommandInvocationError::InvalidArgumentShape);
                }
                Self::from_parsed_fields(
                    executable,
                    phase,
                    repo_root,
                    connection_id,
                    guard_installation_id,
                    host_kind,
                    integration_profile,
                    Some(policy_hash.as_str()),
                    host_output,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parsed_fields(
        executable: GuardCommandAbsolutePath,
        phase: &str,
        repo_root: &str,
        connection_id: &str,
        guard_installation_id: &str,
        host_kind: &str,
        integration_profile: &str,
        policy_hash: Option<&str>,
        host_output: &str,
    ) -> Result<Self, GuardCommandInvocationError> {
        let phase = GuardHookPhase::REQUIRED
            .into_iter()
            .find(|candidate| candidate.command_name() == phase)
            .ok_or(GuardCommandInvocationError::InvalidPhase)?;
        let repo_root = GuardCommandAbsolutePath::parse(repo_root)?;
        let host_kind = host_kind
            .parse::<HostKind>()
            .map_err(|_| GuardCommandInvocationError::InvalidHostKind)?;
        let integration_profile = match integration_profile {
            "record" => IntegrationProfile::Record,
            _ => return Err(GuardCommandInvocationError::InvalidIntegrationProfile),
        };
        let policy_hash = policy_hash
            .map(PolicyHash::parse)
            .transpose()
            .map_err(|_| GuardCommandInvocationError::InvalidPolicyHash)?;
        let host_output = host_output
            .parse::<HostKind>()
            .map_err(|_| GuardCommandInvocationError::InvalidHostOutput)?;
        Self::new(
            executable,
            phase,
            repo_root,
            AgentConnectionId::new(connection_id),
            GuardInstallationId::new(guard_installation_id),
            host_kind,
            integration_profile,
            policy_hash,
            host_output,
        )
    }
}

/// Typed invocation for each phase in one exact generated Guard command set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardCommandInvocationSet {
    pub pre_tool: GuardCommandInvocation,
    pub post_tool: GuardCommandInvocation,
    pub prompt_capture: GuardCommandInvocation,
}

impl GuardCommandInvocationSet {
    /// Constructs the three required phase invocations from one shared owner binding.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        executable: GuardCommandAbsolutePath,
        repo_root: GuardCommandAbsolutePath,
        connection_id: AgentConnectionId,
        guard_installation_id: GuardInstallationId,
        host_kind: HostKind,
        integration_profile: IntegrationProfile,
        policy_hash: Option<PolicyHash>,
        host_output: HostKind,
    ) -> Result<Self, GuardCommandInvocationError> {
        let invocation = |phase| {
            GuardCommandInvocation::new(
                executable.clone(),
                phase,
                repo_root.clone(),
                connection_id.clone(),
                guard_installation_id.clone(),
                host_kind,
                integration_profile,
                policy_hash.clone(),
                host_output,
            )
        };
        let [pre_tool, post_tool, prompt_capture] = GuardHookPhase::REQUIRED;
        Ok(Self {
            pre_tool: invocation(pre_tool)?,
            post_tool: invocation(post_tool)?,
            prompt_capture: invocation(prompt_capture)?,
        })
    }

    /// Returns the typed invocation for one phase.
    pub fn get(&self, phase: GuardHookPhase) -> &GuardCommandInvocation {
        match phase {
            GuardHookPhase::PreTool => &self.pre_tool,
            GuardHookPhase::PostTool => &self.post_tool,
            GuardHookPhase::PromptCapture => &self.prompt_capture,
        }
    }

    /// Serializes every invocation to the requested projection.
    pub fn to_commands(
        &self,
        projection: GuardCommandProjection,
    ) -> Result<GuardCommandSet, GuardCommandInvocationError> {
        let command = |phase| match projection {
            GuardCommandProjection::Policy => Ok(self.get(phase).to_policy_command()),
            GuardCommandProjection::Runtime => self.get(phase).to_runtime_command(),
        };
        Ok(GuardCommandSet {
            pre_tool: command(GuardHookPhase::PreTool)?,
            post_tool: command(GuardHookPhase::PostTool)?,
            prompt_capture: command(GuardHookPhase::PromptCapture)?,
        })
    }

    /// Strictly parses and validates a complete hash-free policy command set.
    pub fn from_policy_commands(
        commands: &GuardCommandSet,
    ) -> Result<Self, GuardCommandInvocationError> {
        Self::from_commands(commands, GuardCommandProjection::Policy, None)
    }

    /// Strictly parses and validates a complete runtime set against one policy hash.
    pub fn from_runtime_commands(
        commands: &GuardCommandSet,
        expected_policy_hash: &PolicyHash,
    ) -> Result<Self, GuardCommandInvocationError> {
        Self::from_commands(
            commands,
            GuardCommandProjection::Runtime,
            Some(expected_policy_hash),
        )
    }

    /// Returns whether every phase has identical policy/runtime owner fields.
    pub fn fields_match_except_policy_hash(&self, other: &Self) -> bool {
        GuardHookPhase::REQUIRED.into_iter().all(|phase| {
            self.get(phase)
                .fields_match_except_policy_hash(other.get(phase))
        })
    }

    fn from_commands(
        commands: &GuardCommandSet,
        projection: GuardCommandProjection,
        expected_policy_hash: Option<&PolicyHash>,
    ) -> Result<Self, GuardCommandInvocationError> {
        let command = |phase| {
            let invocation = match (projection, expected_policy_hash) {
                (GuardCommandProjection::Policy, _) => {
                    GuardCommandInvocation::from_policy_command(commands.get(phase))?
                }
                (GuardCommandProjection::Runtime, Some(policy_hash)) => {
                    GuardCommandInvocation::from_runtime_command_with_policy_hash(
                        commands.get(phase),
                        policy_hash,
                    )?
                }
                (GuardCommandProjection::Runtime, None) => {
                    return Err(GuardCommandInvocationError::MissingPolicyHash);
                }
            };
            if invocation.phase != phase {
                return Err(GuardCommandInvocationError::InvalidPhase);
            }
            Ok(invocation)
        };
        let set = Self {
            pre_tool: command(GuardHookPhase::PreTool)?,
            post_tool: command(GuardHookPhase::PostTool)?,
            prompt_capture: command(GuardHookPhase::PromptCapture)?,
        };
        let first = &set.pre_tool;
        if GuardHookPhase::REQUIRED.into_iter().skip(1).any(|phase| {
            !first.owner_fields_match_except_phase_and_policy_hash(set.get(phase))
                || first.policy_hash != set.get(phase).policy_hash
        }) {
            return Err(GuardCommandInvocationError::InconsistentCommandSet);
        }
        Ok(set)
    }
}

/// Strict generated Guard command conversion failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardCommandInvocationError {
    InvalidAbsolutePath,
    WrongArgumentCount,
    InvalidArgumentShape,
    InvalidPhase,
    InvalidConnectionId,
    InvalidGuardInstallationId,
    InvalidHostKind,
    InvalidIntegrationProfile,
    MissingPolicyHash,
    InvalidPolicyHash,
    PolicyHashMismatch,
    InvalidHostOutput,
    InconsistentCommandSet,
}

impl fmt::Display for GuardCommandInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidAbsolutePath => "Guard command paths must be normalized absolute paths",
            Self::WrongArgumentCount => "Guard command has the wrong argument count",
            Self::InvalidArgumentShape => "Guard command flags must use the exact current order",
            Self::InvalidPhase => "Guard command phase spelling is invalid",
            Self::InvalidConnectionId => "Guard command Connection ID is invalid",
            Self::InvalidGuardInstallationId => "Guard command installation ID is invalid",
            Self::InvalidHostKind => "Guard command host kind is unsupported",
            Self::InvalidIntegrationProfile => "Guard command integration profile is unsupported",
            Self::MissingPolicyHash => "Guard runtime command requires a policy hash",
            Self::InvalidPolicyHash => "Guard runtime command policy hash is invalid",
            Self::PolicyHashMismatch => "Guard runtime command policy hash does not match",
            Self::InvalidHostOutput => "Guard command host output is unsupported",
            Self::InconsistentCommandSet => "Guard command phases do not share one owner binding",
        })
    }
}

impl Error for GuardCommandInvocationError {}

fn validate_command_identifier(value: &str) -> Result<(), ()> {
    if value.trim().is_empty() || value.as_bytes().contains(&0) || value.starts_with("--") {
        Err(())
    } else {
        Ok(())
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
    let Ok(commands) = GuardCommandInvocationSet::from_runtime_commands(
        &manifest.runtime_commands,
        &manifest.policy_hash,
    ) else {
        return false;
    };
    let command = commands.get(GuardHookPhase::PreTool);
    manifest.schema == GUARD_MANIFEST_SCHEMA
        && !manifest.guard_installation_id.as_str().trim().is_empty()
        && !manifest.connection_id.as_str().trim().is_empty()
        && !manifest.project_id.as_str().trim().is_empty()
        && manifest.host_kind == HostKind::Codex
        && manifest.integration_profile == IntegrationProfile::Record
        && canonical_sha256(manifest.policy_hash.as_str())
        && manifest.required_hook_phases == GuardHookPhase::REQUIRED
        && command.connection_id == manifest.connection_id
        && command.guard_installation_id == manifest.guard_installation_id
        && command.host_kind == manifest.host_kind
        && command.integration_profile == manifest.integration_profile
        && inventory_has_exact_semantics(&manifest.managed_files, command.repo_root.as_path())
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
        && wrapper_phases
            == GuardHookPhase::REQUIRED
                .into_iter()
                .map(GuardHookPhase::as_str)
                .collect()
}

fn inventory_path_matches(file: &ManagedFileExpectation, repo_root: &Path) -> bool {
    let path = Path::new(&file.path);
    match file.kind.as_str() {
        "agents_managed_block" => path == repo_root.join("AGENTS.md"),
        "volicord_policy" => path == repo_root.join(".volicord/policy.json"),
        "host_hook_config" => path == repo_root.join(".codex/hooks.json"),
        "host_hook_dispatch" => path == repo_root.join(".codex/hooks/volicord-dispatch.sh"),
        "host_hook_wrapper" => file.phase.as_deref().is_some_and(|phase| {
            phase.parse::<GuardHookPhase>().ok().is_some_and(|phase| {
                path == repo_root.join(format!(".codex/hooks/volicord-{}.sh", phase.command_name()))
            })
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
                && file
                    .phase
                    .as_deref()
                    .is_some_and(|phase| phase.parse::<GuardHookPhase>().is_ok())
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
    let Ok(commands) = GuardCommandInvocationSet::from_runtime_commands(
        &manifest.runtime_commands,
        &manifest.policy_hash,
    ) else {
        return false;
    };
    let Some(repo_root) = binding.project_repo_root.to_str() else {
        return false;
    };
    let command = commands.get(GuardHookPhase::PreTool);
    command.repo_root.as_str() == repo_root
        && command.connection_id.as_str() == binding.row_connection_id
        && command.guard_installation_id.as_str() == binding.row_guard_installation_id
        && command.host_kind.as_str() == binding.connection_host_kind
        && command.connection_id == manifest.connection_id
        && command.guard_installation_id == manifest.guard_installation_id
        && command.host_kind == manifest.host_kind
        && command.integration_profile == manifest.integration_profile
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
                phase.parse::<GuardHookPhase>().ok().is_some_and(|phase| {
                    binding
                        .project_repo_root
                        .join(format!(".codex/hooks/volicord-{}.sh", phase.command_name()))
                        == path
                })
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
    const OTHER_HASH: &str =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111";

    fn invocation(phase: GuardHookPhase, policy_hash: Option<&str>) -> GuardCommandInvocation {
        GuardCommandInvocation::new(
            GuardCommandAbsolutePath::parse("/opt/volicord/bin/volicord").unwrap(),
            phase,
            GuardCommandAbsolutePath::parse("/work/product").unwrap(),
            AgentConnectionId::new("conn_0123456789abcdef01234567"),
            GuardInstallationId::new("guard_installation_example"),
            HostKind::Codex,
            IntegrationProfile::Record,
            policy_hash.map(|value| PolicyHash::parse(value).unwrap()),
            HostKind::Codex,
        )
        .unwrap()
    }

    fn invocation_set(policy_hash: Option<&str>) -> GuardCommandInvocationSet {
        GuardCommandInvocationSet::new(
            GuardCommandAbsolutePath::parse("/opt/volicord/bin/volicord").unwrap(),
            GuardCommandAbsolutePath::parse("/work/product").unwrap(),
            AgentConnectionId::new("conn_0123456789abcdef01234567"),
            GuardInstallationId::new("guard_installation_example"),
            HostKind::Codex,
            IntegrationProfile::Record,
            policy_hash.map(|value| PolicyHash::parse(value).unwrap()),
            HostKind::Codex,
        )
        .unwrap()
    }

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

    #[test]
    fn every_phase_serializes_to_the_exact_policy_and_runtime_grammar() {
        for phase in GuardHookPhase::REQUIRED {
            let invocation = invocation(phase, Some(HASH));
            let policy = invocation.to_policy_command();
            let runtime = invocation.to_runtime_command().unwrap();
            assert_eq!(policy.command, "/opt/volicord/bin/volicord");
            assert_eq!(
                policy.args,
                [
                    "_hook",
                    phase.command_name(),
                    "--repo",
                    "/work/product",
                    "--connection",
                    "conn_0123456789abcdef01234567",
                    "--guard-installation",
                    "guard_installation_example",
                    "--host",
                    "codex",
                    "--integration-profile",
                    "record",
                    "--host-output",
                    "codex",
                ]
            );
            assert_eq!(policy.args.len(), GUARD_POLICY_COMMAND_ARGUMENT_COUNT);
            assert!(!policy.args.iter().any(|value| value == "--policy-hash"));

            let mut expected_runtime = policy.args.clone();
            expected_runtime.splice(12..12, ["--policy-hash".to_owned(), HASH.to_owned()]);
            assert_eq!(runtime.command, policy.command);
            assert_eq!(runtime.args, expected_runtime);
            assert_eq!(runtime.args.len(), GUARD_RUNTIME_COMMAND_ARGUMENT_COUNT);
        }
    }

    #[test]
    fn strict_policy_and_runtime_parsing_round_trip_typed_fields() {
        let runtime_invocations = invocation_set(Some(HASH));
        let policy_commands = runtime_invocations
            .to_commands(GuardCommandProjection::Policy)
            .unwrap();
        let runtime_commands = runtime_invocations
            .to_commands(GuardCommandProjection::Runtime)
            .unwrap();
        let parsed_policy =
            GuardCommandInvocationSet::from_policy_commands(&policy_commands).unwrap();
        let parsed_runtime = GuardCommandInvocationSet::from_runtime_commands(
            &runtime_commands,
            &PolicyHash::parse(HASH).unwrap(),
        )
        .unwrap();
        assert!(parsed_policy.fields_match_except_policy_hash(&parsed_runtime));
        assert!(GuardHookPhase::REQUIRED.into_iter().all(|phase| {
            parsed_policy.get(phase).policy_hash.is_none()
                && parsed_runtime
                    .get(phase)
                    .policy_hash
                    .as_ref()
                    .map(PolicyHash::as_str)
                    == Some(HASH)
        }));
        assert_eq!(
            parsed_policy
                .to_commands(GuardCommandProjection::Policy)
                .unwrap(),
            policy_commands
        );
        assert_eq!(
            parsed_runtime
                .to_commands(GuardCommandProjection::Runtime)
                .unwrap(),
            runtime_commands
        );
    }

    #[test]
    fn strict_parser_rejects_every_malformed_command_coordinate() {
        let expected_hash = PolicyHash::parse(HASH).unwrap();
        let runtime = invocation(GuardHookPhase::PreTool, Some(HASH))
            .to_runtime_command()
            .unwrap();
        let rejected = |command: &GuardCommand| {
            assert!(
                GuardCommandInvocation::from_runtime_command_with_policy_hash(
                    command,
                    &expected_hash
                )
                .is_err()
            );
        };

        let mut wrong_count = runtime.clone();
        wrong_count.args.pop();
        rejected(&wrong_count);

        for flag_index in [2, 4, 6, 8, 10, 12, 14] {
            let mut unknown_flag = runtime.clone();
            unknown_flag.args[flag_index] = "--unknown".to_owned();
            rejected(&unknown_flag);
        }

        let mut wrong_order = runtime.clone();
        wrong_order.args.swap(2, 4);
        wrong_order.args.swap(3, 5);
        rejected(&wrong_order);

        let mut duplicate_flag = runtime.clone();
        duplicate_flag.args[4] = "--repo".to_owned();
        rejected(&duplicate_flag);

        let mut missing_value = runtime.clone();
        missing_value.args.remove(13);
        rejected(&missing_value);

        let mut wrong_hook = runtime.clone();
        wrong_hook.args[0] = "hook".to_owned();
        rejected(&wrong_hook);

        for phase in ["pre_tool", "unknown-phase"] {
            let mut wrong_phase = runtime.clone();
            wrong_phase.args[1] = phase.to_owned();
            rejected(&wrong_phase);
        }

        let mut relative_executable = runtime.clone();
        relative_executable.command = "bin/volicord".to_owned();
        rejected(&relative_executable);
        let mut nonnormalized_executable = runtime.clone();
        nonnormalized_executable.command = "/opt/../bin/volicord".to_owned();
        rejected(&nonnormalized_executable);

        for repo_root in ["work/product", "/work/../product"] {
            let mut wrong_repo = runtime.clone();
            wrong_repo.args[3] = repo_root.to_owned();
            rejected(&wrong_repo);
        }

        for connection_id in ["", "   ", "--guard-installation", "conn\0bad"] {
            let mut wrong_connection = runtime.clone();
            wrong_connection.args[5] = connection_id.to_owned();
            rejected(&wrong_connection);
        }
        for installation_id in ["", "   ", "--host", "guard\0bad"] {
            let mut wrong_installation = runtime.clone();
            wrong_installation.args[7] = installation_id.to_owned();
            rejected(&wrong_installation);
        }

        let mut wrong_host = runtime.clone();
        wrong_host.args[9] = "other".to_owned();
        rejected(&wrong_host);
        let mut wrong_profile = runtime.clone();
        wrong_profile.args[11] = "other".to_owned();
        rejected(&wrong_profile);
        let mut wrong_hash = runtime.clone();
        wrong_hash.args[13] = "sha256:not-canonical".to_owned();
        rejected(&wrong_hash);
        let mut mismatched_hash = runtime.clone();
        mismatched_hash.args[13] = OTHER_HASH.to_owned();
        rejected(&mismatched_hash);
        let mut wrong_output = runtime.clone();
        wrong_output.args[15] = "other".to_owned();
        rejected(&wrong_output);

        let policy = invocation(GuardHookPhase::PreTool, None).to_policy_command();
        assert!(GuardCommandInvocation::from_runtime_command(&policy).is_err());
        assert!(GuardCommandInvocation::from_policy_command(&runtime).is_err());
        assert!(invocation(GuardHookPhase::PreTool, None)
            .to_runtime_command()
            .is_err());
    }

    #[test]
    fn command_sets_reject_cross_phase_owner_drift() {
        let mut commands = invocation_set(Some(HASH))
            .to_commands(GuardCommandProjection::Runtime)
            .unwrap();
        commands.post_tool.args[3] = "/work/other".to_owned();
        assert!(GuardCommandInvocationSet::from_runtime_commands(
            &commands,
            &PolicyHash::parse(HASH).unwrap()
        )
        .is_err());
    }
}
