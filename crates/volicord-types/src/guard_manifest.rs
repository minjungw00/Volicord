use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::{Component, Path, PathBuf},
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

/// Closed serialized kind of one Guard-managed artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GuardManagedArtifactKind {
    VolicordPolicy,
    GitInfoExclude,
    HostHookConfig,
    HostHookDispatch,
    HostHookWrapper,
    HostRuleInstruction,
    AgentsManagedBlock,
}

impl GuardManagedArtifactKind {
    /// Returns the exact current manifest spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VolicordPolicy => "volicord_policy",
            Self::GitInfoExclude => "git_info_exclude",
            Self::HostHookConfig => "host_hook_config",
            Self::HostHookDispatch => "host_hook_dispatch",
            Self::HostHookWrapper => "host_hook_wrapper",
            Self::HostRuleInstruction => "host_rule_instruction",
            Self::AgentsManagedBlock => "agents_managed_block",
        }
    }
}

/// One exact artifact coordinate in the current Guard inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardManagedArtifact {
    VolicordPolicy,
    GitInfoExclude,
    HostHookConfig,
    HostHookDispatch,
    HostHookWrapper(GuardHookPhase),
    HostRuleInstruction,
    AgentsManagedBlock,
}

impl GuardManagedArtifact {
    /// Every current coordinate, including the optional Git owner coordinate.
    pub const ALL: [Self; 9] = [
        Self::AgentsManagedBlock,
        Self::VolicordPolicy,
        Self::HostHookConfig,
        Self::HostHookDispatch,
        Self::HostHookWrapper(GuardHookPhase::PreTool),
        Self::HostHookWrapper(GuardHookPhase::PostTool),
        Self::HostHookWrapper(GuardHookPhase::PromptCapture),
        Self::HostRuleInstruction,
        Self::GitInfoExclude,
    ];

    /// Returns the shared serialized kind for this coordinate.
    pub const fn kind(self) -> GuardManagedArtifactKind {
        match self {
            Self::VolicordPolicy => GuardManagedArtifactKind::VolicordPolicy,
            Self::GitInfoExclude => GuardManagedArtifactKind::GitInfoExclude,
            Self::HostHookConfig => GuardManagedArtifactKind::HostHookConfig,
            Self::HostHookDispatch => GuardManagedArtifactKind::HostHookDispatch,
            Self::HostHookWrapper(_) => GuardManagedArtifactKind::HostHookWrapper,
            Self::HostRuleInstruction => GuardManagedArtifactKind::HostRuleInstruction,
            Self::AgentsManagedBlock => GuardManagedArtifactKind::AgentsManagedBlock,
        }
    }

    /// Returns this coordinate's one canonical inventory specification.
    pub fn spec(self) -> &'static GuardManagedArtifactSpec {
        GUARD_MANAGED_ARTIFACT_SPECS
            .iter()
            .find(|spec| spec.artifact == self)
            .expect("every closed Guard artifact has one specification")
    }

    /// Constructs the exact owned path for this coordinate.
    pub fn expected_path(
        self,
        repo_root: &Path,
        project_git_info_exclude_path: Option<&Path>,
    ) -> Option<PathBuf> {
        self.spec()
            .expected_path(repo_root, project_git_info_exclude_path)
    }
}

/// Closed ownership form serialized in a Guard manifest.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GuardManagedOwnership {
    ManagedJson,
    ManagedBlock,
    ManagedScript,
}

/// Marker contract required by one managed artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardManagedMarkerSemantics {
    None,
    BlockPair,
    ScriptMarker,
}

/// Canonical path rule owned by one managed-artifact specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardManagedArtifactPath {
    RepositoryRelative(&'static str),
    HookWrapper,
    GitInfoExclude,
}

/// Closed role of a managed Guard script.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GuardManagedScriptRole {
    CodexDispatch,
}

/// Closed dispatch-script phase spelling retained by the current wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardDispatchPhase {
    Dispatch,
}

/// Closed wrapper purpose retained by the current wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardManagedScriptPurpose {
    Guard,
}

/// Canonical content digest of one exact managed artifact projection.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct GuardArtifactContentHash(String);

impl GuardArtifactContentHash {
    /// Parses one canonical `sha256:<64-lowercase-hex>` content digest.
    pub fn parse(value: impl Into<String>) -> Result<Self, GuardManifestError> {
        let value = value.into();
        if canonical_sha256(&value) {
            Ok(Self(value))
        } else {
            Err(GuardManifestError::InvalidContentHash)
        }
    }

    /// Returns the canonical digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One coordinate's canonical path and semantic requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardManagedArtifactSpec {
    pub artifact: GuardManagedArtifact,
    pub path: GuardManagedArtifactPath,
    pub expected_count: usize,
    pub ownership: GuardManagedOwnership,
    pub marker_semantics: GuardManagedMarkerSemantics,
    pub executable_required: bool,
    pub script_role: Option<GuardManagedScriptRole>,
    pub optional_under_git_owner: bool,
}

impl GuardManagedArtifactSpec {
    /// Returns the repository-relative path when this artifact is repository-owned.
    pub fn repository_relative_path(self) -> Option<PathBuf> {
        match (self.path, self.artifact) {
            (GuardManagedArtifactPath::RepositoryRelative(path), _) => Some(PathBuf::from(path)),
            (
                GuardManagedArtifactPath::HookWrapper,
                GuardManagedArtifact::HostHookWrapper(phase),
            ) => Some(PathBuf::from(format!(
                ".codex/hooks/volicord-{}.sh",
                phase.command_name()
            ))),
            (GuardManagedArtifactPath::GitInfoExclude, GuardManagedArtifact::GitInfoExclude) => {
                None
            }
            _ => None,
        }
    }

    /// Constructs this specification's exact owned path.
    pub fn expected_path(
        self,
        repo_root: &Path,
        project_git_info_exclude_path: Option<&Path>,
    ) -> Option<PathBuf> {
        match self.path {
            GuardManagedArtifactPath::GitInfoExclude => {
                project_git_info_exclude_path.map(Path::to_path_buf)
            }
            GuardManagedArtifactPath::RepositoryRelative(_)
            | GuardManagedArtifactPath::HookWrapper => self
                .repository_relative_path()
                .map(|path| repo_root.join(path)),
        }
    }
}

/// The sole current Guard managed-artifact inventory.
pub const GUARD_MANAGED_ARTIFACT_SPECS: [GuardManagedArtifactSpec; 9] = [
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::AgentsManagedBlock,
        path: GuardManagedArtifactPath::RepositoryRelative("AGENTS.md"),
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedBlock,
        marker_semantics: GuardManagedMarkerSemantics::BlockPair,
        executable_required: false,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::VolicordPolicy,
        path: GuardManagedArtifactPath::RepositoryRelative(".volicord/policy.json"),
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedJson,
        marker_semantics: GuardManagedMarkerSemantics::None,
        executable_required: false,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::HostHookConfig,
        path: GuardManagedArtifactPath::RepositoryRelative(".codex/hooks.json"),
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedJson,
        marker_semantics: GuardManagedMarkerSemantics::None,
        executable_required: false,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::HostHookDispatch,
        path: GuardManagedArtifactPath::RepositoryRelative(".codex/hooks/volicord-dispatch.sh"),
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedScript,
        marker_semantics: GuardManagedMarkerSemantics::ScriptMarker,
        executable_required: true,
        script_role: Some(GuardManagedScriptRole::CodexDispatch),
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
        path: GuardManagedArtifactPath::HookWrapper,
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedScript,
        marker_semantics: GuardManagedMarkerSemantics::ScriptMarker,
        executable_required: true,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PostTool),
        path: GuardManagedArtifactPath::HookWrapper,
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedScript,
        marker_semantics: GuardManagedMarkerSemantics::ScriptMarker,
        executable_required: true,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PromptCapture),
        path: GuardManagedArtifactPath::HookWrapper,
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedScript,
        marker_semantics: GuardManagedMarkerSemantics::ScriptMarker,
        executable_required: true,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::HostRuleInstruction,
        path: GuardManagedArtifactPath::RepositoryRelative(".codex/rules/volicord.rules"),
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedBlock,
        marker_semantics: GuardManagedMarkerSemantics::BlockPair,
        executable_required: false,
        script_role: None,
        optional_under_git_owner: false,
    },
    GuardManagedArtifactSpec {
        artifact: GuardManagedArtifact::GitInfoExclude,
        path: GuardManagedArtifactPath::GitInfoExclude,
        expected_count: 1,
        ownership: GuardManagedOwnership::ManagedBlock,
        marker_semantics: GuardManagedMarkerSemantics::BlockPair,
        executable_required: false,
        script_role: None,
        optional_under_git_owner: true,
    },
];

/// One exact Volicord-managed file expectation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManagedFileExpectation {
    VolicordPolicy {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
    },
    GitInfoExclude {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
        managed_marker_start: String,
        managed_marker_end: String,
    },
    HostHookConfig {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
    },
    HostHookDispatch {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
        managed_marker: String,
        executable_required: bool,
        managed_script_role: GuardManagedScriptRole,
        host_kind: HostKind,
        phase: GuardDispatchPhase,
    },
    HostHookWrapper {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
        managed_marker: String,
        executable_required: bool,
        managed_script_command: String,
        host_kind: HostKind,
        phase: GuardHookPhase,
        purpose: GuardManagedScriptPurpose,
        connection_id: AgentConnectionId,
        guard_installation_id: GuardInstallationId,
        policy_hash: PolicyHash,
        host_output: HostKind,
    },
    HostRuleInstruction {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
        managed_marker_start: String,
        managed_marker_end: String,
    },
    AgentsManagedBlock {
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        ownership: GuardManagedOwnership,
        managed_marker_start: String,
        managed_marker_end: String,
    },
}

impl ManagedFileExpectation {
    /// Builds one canonical managed-JSON expectation.
    pub fn managed_json(
        artifact: GuardManagedArtifact,
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
    ) -> Result<Self, GuardManifestError> {
        match artifact {
            GuardManagedArtifact::VolicordPolicy => Ok(Self::VolicordPolicy {
                path,
                content_hash,
                ownership: GuardManagedOwnership::ManagedJson,
            }),
            GuardManagedArtifact::HostHookConfig => Ok(Self::HostHookConfig {
                path,
                content_hash,
                ownership: GuardManagedOwnership::ManagedJson,
            }),
            _ => Err(GuardManifestError::InvalidShape),
        }
    }

    /// Builds one canonical managed-block expectation.
    pub fn managed_block(
        artifact: GuardManagedArtifact,
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        managed_marker_start: impl Into<String>,
        managed_marker_end: impl Into<String>,
    ) -> Result<Self, GuardManifestError> {
        let managed_marker_start = managed_marker_start.into();
        let managed_marker_end = managed_marker_end.into();
        match artifact {
            GuardManagedArtifact::GitInfoExclude => Ok(Self::GitInfoExclude {
                path,
                content_hash,
                ownership: GuardManagedOwnership::ManagedBlock,
                managed_marker_start,
                managed_marker_end,
            }),
            GuardManagedArtifact::HostRuleInstruction => Ok(Self::HostRuleInstruction {
                path,
                content_hash,
                ownership: GuardManagedOwnership::ManagedBlock,
                managed_marker_start,
                managed_marker_end,
            }),
            GuardManagedArtifact::AgentsManagedBlock => Ok(Self::AgentsManagedBlock {
                path,
                content_hash,
                ownership: GuardManagedOwnership::ManagedBlock,
                managed_marker_start,
                managed_marker_end,
            }),
            _ => Err(GuardManifestError::InvalidShape),
        }
    }

    /// Builds the canonical Codex dispatch-script expectation.
    pub fn codex_dispatch_script(
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        managed_marker: impl Into<String>,
    ) -> Self {
        Self::HostHookDispatch {
            path,
            content_hash,
            ownership: GuardManagedOwnership::ManagedScript,
            managed_marker: managed_marker.into(),
            executable_required: true,
            managed_script_role: GuardManagedScriptRole::CodexDispatch,
            host_kind: HostKind::Codex,
            phase: GuardDispatchPhase::Dispatch,
        }
    }

    /// Builds one canonical phase-wrapper expectation.
    #[allow(clippy::too_many_arguments)]
    pub fn hook_wrapper(
        phase: GuardHookPhase,
        path: PathBuf,
        content_hash: GuardArtifactContentHash,
        managed_marker: impl Into<String>,
        managed_script_command: impl Into<String>,
        connection_id: AgentConnectionId,
        guard_installation_id: GuardInstallationId,
        policy_hash: PolicyHash,
    ) -> Self {
        Self::HostHookWrapper {
            path,
            content_hash,
            ownership: GuardManagedOwnership::ManagedScript,
            managed_marker: managed_marker.into(),
            executable_required: true,
            managed_script_command: managed_script_command.into(),
            host_kind: HostKind::Codex,
            phase,
            purpose: GuardManagedScriptPurpose::Guard,
            connection_id,
            guard_installation_id,
            policy_hash,
            host_output: HostKind::Codex,
        }
    }

    /// Returns the exact typed artifact coordinate.
    pub const fn artifact(&self) -> GuardManagedArtifact {
        match self {
            Self::VolicordPolicy { .. } => GuardManagedArtifact::VolicordPolicy,
            Self::GitInfoExclude { .. } => GuardManagedArtifact::GitInfoExclude,
            Self::HostHookConfig { .. } => GuardManagedArtifact::HostHookConfig,
            Self::HostHookDispatch { .. } => GuardManagedArtifact::HostHookDispatch,
            Self::HostHookWrapper { phase, .. } => GuardManagedArtifact::HostHookWrapper(*phase),
            Self::HostRuleInstruction { .. } => GuardManagedArtifact::HostRuleInstruction,
            Self::AgentsManagedBlock { .. } => GuardManagedArtifact::AgentsManagedBlock,
        }
    }

    /// Returns the shared serialized artifact kind.
    pub const fn kind(&self) -> GuardManagedArtifactKind {
        self.artifact().kind()
    }

    /// Returns the exact managed path.
    pub fn path(&self) -> &Path {
        match self {
            Self::VolicordPolicy { path, .. }
            | Self::GitInfoExclude { path, .. }
            | Self::HostHookConfig { path, .. }
            | Self::HostHookDispatch { path, .. }
            | Self::HostHookWrapper { path, .. }
            | Self::HostRuleInstruction { path, .. }
            | Self::AgentsManagedBlock { path, .. } => path,
        }
    }

    /// Returns the exact expected content digest.
    pub fn content_hash(&self) -> &GuardArtifactContentHash {
        match self {
            Self::VolicordPolicy { content_hash, .. }
            | Self::GitInfoExclude { content_hash, .. }
            | Self::HostHookConfig { content_hash, .. }
            | Self::HostHookDispatch { content_hash, .. }
            | Self::HostHookWrapper { content_hash, .. }
            | Self::HostRuleInstruction { content_hash, .. }
            | Self::AgentsManagedBlock { content_hash, .. } => content_hash,
        }
    }

    /// Returns the closed ownership form.
    pub const fn ownership(&self) -> GuardManagedOwnership {
        match self {
            Self::VolicordPolicy { ownership, .. }
            | Self::GitInfoExclude { ownership, .. }
            | Self::HostHookConfig { ownership, .. }
            | Self::HostHookDispatch { ownership, .. }
            | Self::HostHookWrapper { ownership, .. }
            | Self::HostRuleInstruction { ownership, .. }
            | Self::AgentsManagedBlock { ownership, .. } => *ownership,
        }
    }

    /// Returns the managed block markers when this is a block-owned artifact.
    pub fn block_markers(&self) -> Option<(&str, &str)> {
        match self {
            Self::GitInfoExclude {
                managed_marker_start,
                managed_marker_end,
                ..
            }
            | Self::HostRuleInstruction {
                managed_marker_start,
                managed_marker_end,
                ..
            }
            | Self::AgentsManagedBlock {
                managed_marker_start,
                managed_marker_end,
                ..
            } => Some((managed_marker_start, managed_marker_end)),
            _ => None,
        }
    }

    /// Returns whether this expectation carries the platform-independent executable contract.
    pub const fn executable_required(&self) -> Option<bool> {
        match self {
            Self::HostHookDispatch {
                executable_required,
                ..
            }
            | Self::HostHookWrapper {
                executable_required,
                ..
            } => Some(*executable_required),
            _ => None,
        }
    }
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
    InvalidContentHash,
}

impl fmt::Display for GuardManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidJson => "Guard manifest is not valid JSON",
            Self::NonCanonicalJson => "Guard manifest JSON is not canonical",
            Self::InvalidShape => "Guard manifest does not match the exact current contract",
            Self::InvalidPolicyHash => "Guard policy hash is not canonical",
            Self::InvalidContentHash => "Guard artifact content hash is not canonical",
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
                path: file.path().display().to_string(),
                digest: file
                    .content_hash()
                    .as_str()
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
    let mut artifact_counts = BTreeMap::<GuardManagedArtifact, usize>::new();
    let mut paths = BTreeSet::new();
    for file in files {
        if !file_has_closed_semantics(file)
            || !inventory_path_matches(file, repo_root)
            || !paths.insert(file.path())
        {
            return false;
        }
        *artifact_counts.entry(file.artifact()).or_default() += 1;
    }
    GUARD_MANAGED_ARTIFACT_SPECS.iter().all(|spec| {
        let count = artifact_counts.get(&spec.artifact).copied().unwrap_or(0);
        count == spec.expected_count || (spec.optional_under_git_owner && count == 0)
    }) && artifact_counts.len()
        == GUARD_MANAGED_ARTIFACT_SPECS
            .iter()
            .filter(|spec| artifact_counts.contains_key(&spec.artifact))
            .count()
}

fn inventory_path_matches(file: &ManagedFileExpectation, repo_root: &Path) -> bool {
    let artifact = file.artifact();
    artifact == GuardManagedArtifact::GitInfoExclude
        || artifact.expected_path(repo_root, None).as_deref() == Some(file.path())
}

fn file_has_closed_semantics(file: &ManagedFileExpectation) -> bool {
    let spec = file.artifact().spec();
    if !normalized_absolute_path(file.path())
        || !canonical_sha256(file.content_hash().as_str())
        || file.ownership() != spec.ownership
        || file.executable_required()
            != spec.executable_required.then_some(spec.executable_required)
    {
        return false;
    }
    let marker_semantics_match = match spec.marker_semantics {
        GuardManagedMarkerSemantics::None => file.block_markers().is_none(),
        GuardManagedMarkerSemantics::BlockPair => file
            .block_markers()
            .is_some_and(|(start, end)| !start.trim().is_empty() && !end.trim().is_empty()),
        GuardManagedMarkerSemantics::ScriptMarker => match file {
            ManagedFileExpectation::HostHookDispatch { managed_marker, .. }
            | ManagedFileExpectation::HostHookWrapper { managed_marker, .. } => {
                !managed_marker.trim().is_empty()
            }
            _ => false,
        },
    };
    let script_role = match file {
        ManagedFileExpectation::HostHookDispatch {
            managed_script_role,
            ..
        } => Some(*managed_script_role),
        _ => None,
    };
    if !marker_semantics_match || script_role != spec.script_role {
        return false;
    }
    match file {
        ManagedFileExpectation::VolicordPolicy { .. }
        | ManagedFileExpectation::HostHookConfig { .. }
        | ManagedFileExpectation::GitInfoExclude { .. }
        | ManagedFileExpectation::HostRuleInstruction { .. }
        | ManagedFileExpectation::AgentsManagedBlock { .. } => true,
        ManagedFileExpectation::HostHookDispatch {
            managed_script_role,
            host_kind,
            phase,
            ..
        } => {
            *managed_script_role == GuardManagedScriptRole::CodexDispatch
                && *host_kind == HostKind::Codex
                && *phase == GuardDispatchPhase::Dispatch
        }
        ManagedFileExpectation::HostHookWrapper {
            managed_script_command,
            host_kind,
            purpose,
            connection_id,
            guard_installation_id,
            policy_hash,
            host_output,
            ..
        } => {
            !managed_script_command.trim().is_empty()
                && *host_kind == HostKind::Codex
                && *purpose == GuardManagedScriptPurpose::Guard
                && !connection_id.as_str().trim().is_empty()
                && !guard_installation_id.as_str().trim().is_empty()
                && canonical_sha256(policy_hash.as_str())
                && *host_output == HostKind::Codex
        }
    }
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
        let path_matches = file
            .artifact()
            .expected_path(
                binding.project_repo_root,
                binding.project_git_info_exclude_path,
            )
            .as_deref()
            == Some(file.path());
        path_matches
            && match file {
                ManagedFileExpectation::HostHookWrapper {
                    connection_id,
                    guard_installation_id,
                    policy_hash,
                    ..
                } => {
                    connection_id.as_str() == binding.row_connection_id
                        && guard_installation_id.as_str() == binding.row_guard_installation_id
                        && policy_hash == &manifest.policy_hash
                }
                _ => true,
            }
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

    fn managed_inventory(include_git_exclude: bool) -> Vec<ManagedFileExpectation> {
        let repo_root = Path::new("/work/product");
        let content_hash = || GuardArtifactContentHash::parse(HASH).unwrap();
        let path = |artifact: GuardManagedArtifact| {
            artifact
                .expected_path(repo_root, None)
                .expect("repository-owned test artifact")
        };
        let mut files = vec![
            ManagedFileExpectation::managed_block(
                GuardManagedArtifact::AgentsManagedBlock,
                path(GuardManagedArtifact::AgentsManagedBlock),
                content_hash(),
                "# BEGIN VOLICORD",
                "# END VOLICORD",
            )
            .unwrap(),
            ManagedFileExpectation::managed_json(
                GuardManagedArtifact::VolicordPolicy,
                path(GuardManagedArtifact::VolicordPolicy),
                content_hash(),
            )
            .unwrap(),
            ManagedFileExpectation::managed_json(
                GuardManagedArtifact::HostHookConfig,
                path(GuardManagedArtifact::HostHookConfig),
                content_hash(),
            )
            .unwrap(),
            ManagedFileExpectation::codex_dispatch_script(
                path(GuardManagedArtifact::HostHookDispatch),
                content_hash(),
                "VOLICORD_MANAGED_HOOK_WRAPPER",
            ),
            ManagedFileExpectation::managed_block(
                GuardManagedArtifact::HostRuleInstruction,
                path(GuardManagedArtifact::HostRuleInstruction),
                content_hash(),
                "# BEGIN VOLICORD",
                "# END VOLICORD",
            )
            .unwrap(),
        ];
        files.extend(GuardHookPhase::REQUIRED.into_iter().map(|phase| {
            ManagedFileExpectation::hook_wrapper(
                phase,
                path(GuardManagedArtifact::HostHookWrapper(phase)),
                content_hash(),
                "VOLICORD_MANAGED_HOOK_WRAPPER",
                "/opt/volicord/bin/volicord _hook",
                AgentConnectionId::new("conn_0123456789abcdef01234567"),
                GuardInstallationId::new("guard_installation_example"),
                PolicyHash::parse(HASH).unwrap(),
            )
        }));
        if include_git_exclude {
            files.push(
                ManagedFileExpectation::managed_block(
                    GuardManagedArtifact::GitInfoExclude,
                    PathBuf::from("/work/product/.git/info/exclude"),
                    content_hash(),
                    "# BEGIN VOLICORD",
                    "# END VOLICORD",
                )
                .unwrap(),
            );
        }
        files
    }

    fn manifest(include_git_exclude: bool) -> GuardManifest {
        GuardManifest {
            schema: GUARD_MANIFEST_SCHEMA.to_owned(),
            guard_installation_id: GuardInstallationId::new("guard_installation_example"),
            connection_id: AgentConnectionId::new("conn_0123456789abcdef01234567"),
            project_id: ProjectId::new("project_example"),
            host_kind: HostKind::Codex,
            integration_profile: IntegrationProfile::Record,
            policy_hash: PolicyHash::parse(HASH).unwrap(),
            integration_revision: IntegrationRevision::parse(HASH).unwrap(),
            runtime_commands: invocation_set(Some(HASH))
                .to_commands(GuardCommandProjection::Runtime)
                .unwrap(),
            managed_files: managed_inventory(include_git_exclude),
            required_hook_phases: GuardHookPhase::REQUIRED.to_vec(),
        }
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
    fn canonical_inventory_has_exact_coordinates_counts_and_paths() {
        assert_eq!(
            GUARD_MANAGED_ARTIFACT_SPECS.len(),
            GuardManagedArtifact::ALL.len()
        );
        assert_eq!(
            GUARD_MANAGED_ARTIFACT_SPECS
                .iter()
                .map(|spec| spec.artifact)
                .collect::<BTreeSet<_>>(),
            GuardManagedArtifact::ALL.into_iter().collect()
        );
        assert!(GUARD_MANAGED_ARTIFACT_SPECS
            .iter()
            .all(|spec| spec.expected_count == 1));

        let wrapper_paths = GuardHookPhase::REQUIRED
            .into_iter()
            .map(|phase| {
                GuardManagedArtifact::HostHookWrapper(phase)
                    .spec()
                    .repository_relative_path()
                    .unwrap()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(wrapper_paths.len(), GuardHookPhase::REQUIRED.len());
        assert_eq!(
            wrapper_paths,
            BTreeSet::from([
                PathBuf::from(".codex/hooks/volicord-pre-tool.sh"),
                PathBuf::from(".codex/hooks/volicord-post-tool.sh"),
                PathBuf::from(".codex/hooks/volicord-prompt-capture.sh"),
            ])
        );
    }

    #[test]
    fn inventory_script_and_non_script_executable_contracts_are_platform_independent() {
        let files = managed_inventory(true);
        assert!(files.iter().all(|file| {
            if file.ownership() == GuardManagedOwnership::ManagedScript {
                file.executable_required() == Some(true)
            } else {
                file.executable_required().is_none()
            }
        }));
        assert!(GUARD_MANAGED_ARTIFACT_SPECS.iter().all(|spec| {
            spec.executable_required == (spec.ownership == GuardManagedOwnership::ManagedScript)
        }));
    }

    #[test]
    fn managed_artifact_wire_vocabulary_is_strict() {
        let mut wrapper = serde_json::to_value(
            managed_inventory(false)
                .into_iter()
                .find(|file| {
                    file.artifact()
                        == GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool)
                })
                .unwrap(),
        )
        .unwrap();
        for (field, value) in [
            ("kind", "unknown_kind"),
            ("ownership", "unknown_ownership"),
            ("phase", "unknown_phase"),
            ("host_kind", "unknown_host"),
        ] {
            let mut candidate = wrapper.clone();
            candidate[field] = Value::String(value.to_owned());
            assert!(serde_json::from_value::<ManagedFileExpectation>(candidate).is_err());
        }

        let dispatch = managed_inventory(false)
            .into_iter()
            .find(|file| file.artifact() == GuardManagedArtifact::HostHookDispatch)
            .unwrap();
        wrapper = serde_json::to_value(dispatch).unwrap();
        wrapper["managed_script_role"] = Value::String("unknown_role".to_owned());
        assert!(serde_json::from_value::<ManagedFileExpectation>(wrapper).is_err());
    }

    #[test]
    fn exact_inventory_rejects_duplicate_paths_and_wrapper_coordinates() {
        let mut duplicate_path = manifest(true);
        let agents_path = GuardManagedArtifact::AgentsManagedBlock
            .expected_path(Path::new("/work/product"), None)
            .unwrap();
        let git = duplicate_path
            .managed_files
            .iter_mut()
            .find(|file| file.artifact() == GuardManagedArtifact::GitInfoExclude)
            .unwrap();
        if let ManagedFileExpectation::GitInfoExclude { path, .. } = git {
            *path = agents_path;
        }
        assert!(!guard_manifest_has_exact_current_shape(
            &serde_json::to_value(duplicate_path).unwrap()
        ));

        let mut duplicate_wrapper = manifest(false);
        let pre_tool = duplicate_wrapper
            .managed_files
            .iter()
            .find(|file| {
                file.artifact() == GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool)
            })
            .unwrap()
            .clone();
        let post_tool = duplicate_wrapper
            .managed_files
            .iter_mut()
            .find(|file| {
                file.artifact() == GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PostTool)
            })
            .unwrap();
        *post_tool = pre_tool;
        assert!(!guard_manifest_has_exact_current_shape(
            &serde_json::to_value(duplicate_wrapper).unwrap()
        ));
    }

    #[test]
    fn optional_git_exclude_is_valid_only_under_its_external_owner_path() {
        let without_git = manifest(false);
        let without_git_value = serde_json::to_value(&without_git).unwrap();
        assert!(guard_manifest_has_exact_current_shape(&without_git_value));
        assert!(guard_manifest_matches_owner_binding(
            &without_git_value,
            GuardManifestOwnerBinding {
                row_guard_installation_id: "guard_installation_example",
                row_connection_id: "conn_0123456789abcdef01234567",
                row_project_id: "project_example",
                connection_host_kind: "codex",
                connection_integration_revision: HASH,
                project_repo_root: Path::new("/work/product"),
                project_git_info_exclude_path: None,
            }
        ));

        let with_git_value = serde_json::to_value(manifest(true)).unwrap();
        assert!(guard_manifest_has_exact_current_shape(&with_git_value));
        assert!(!guard_manifest_matches_owner_binding(
            &with_git_value,
            GuardManifestOwnerBinding {
                row_guard_installation_id: "guard_installation_example",
                row_connection_id: "conn_0123456789abcdef01234567",
                row_project_id: "project_example",
                connection_host_kind: "codex",
                connection_integration_revision: HASH,
                project_repo_root: Path::new("/work/product"),
                project_git_info_exclude_path: None,
            }
        ));
        assert!(guard_manifest_matches_owner_binding(
            &with_git_value,
            GuardManifestOwnerBinding {
                row_guard_installation_id: "guard_installation_example",
                row_connection_id: "conn_0123456789abcdef01234567",
                row_project_id: "project_example",
                connection_host_kind: "codex",
                connection_integration_revision: HASH,
                project_repo_root: Path::new("/work/product"),
                project_git_info_exclude_path: Some(Path::new("/work/product/.git/info/exclude")),
            }
        ));
    }

    #[test]
    fn exact_validator_consumes_paths_and_semantics_from_the_registry() {
        let manifest = manifest(true);
        for file in &manifest.managed_files {
            let expected = file.artifact().expected_path(
                Path::new("/work/product"),
                Some(Path::new("/work/product/.git/info/exclude")),
            );
            assert_eq!(expected.as_deref(), Some(file.path()));
            assert_eq!(file.ownership(), file.artifact().spec().ownership);
        }
        let text = serde_json::to_string(&manifest).unwrap();
        assert_eq!(guard_manifest_from_json(&text).unwrap(), manifest);
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
