//! Canonical managed-host binding and verification-receipt contracts.

use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    is_canonical_sha256_digest, is_canonical_sha256_hex, AgentConnectionId, HostKind,
    IntegrationProfile, ProjectId, UtcTimestamp,
};

/// Domain separator for the exact managed-host binding identity.
pub const MANAGED_HOST_BINDING_DOMAIN: &[u8] = b"volicord.managed-host-binding\0";

/// Canonical top-level record field order used by the managed-host binding codec.
///
/// The external-contract schema identity consumes this same declaration, so a
/// field-order change cannot leave the registered descriptor claiming the old
/// codec structure.
pub(crate) const MANAGED_HOST_BINDING_CANONICAL_FIELDS: [&str; 10] = [
    "host_kind",
    "connection_scope",
    "command",
    "arguments",
    "forwarded_environment",
    "configuration_target",
    "process_binding",
    "required_capabilities",
    "platform_environment",
    "platform_release_coordinate",
];

/// Domain separator for the exact generated managed-artifact identity.
pub const GENERATED_MANAGED_ARTIFACTS_DOMAIN: &[u8] = b"volicord.generated-managed-artifacts\0";

/// Exact contract identity for a typed host verification receipt.
pub const HOST_VERIFICATION_RECEIPT_CONTRACT_ID: &str = "volicord.host-verification-receipt";

/// Exact first-release WSL2 distribution name.
pub const PINNED_WSL2_DISTRIBUTION_NAME: &str = "Ubuntu-24.04";

/// Exact first-release WSL2 distribution identifier from `/etc/os-release`.
pub const PINNED_WSL2_DISTRIBUTION_ID: &str = "ubuntu";

/// Exact first-release WSL2 distribution version from `/etc/os-release`.
pub const PINNED_WSL2_DISTRIBUTION_VERSION: &str = "24.04";

/// Exact first-release WSL2 support-policy image coordinate.
pub const PINNED_WSL2_ENVIRONMENT_IMAGE: &str = "Ubuntu-24.04-LTS-WSL2";

const MAX_BINDING_STRING_BYTES: usize = 4_096;
const MAX_PROCESS_TOKEN_BYTES: usize = 256;
const MAX_RECEIPT_IDENTIFIER_BYTES: usize = 1_024;

/// Platform environment represented by an independent release-validation cell.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PlatformEnvironment {
    /// Native Linux.
    Linux,
    /// Native macOS.
    Macos,
    /// Native Windows.
    NativeWindows,
    /// WSL2 with its independent topology requirements.
    Wsl2,
}

impl PlatformEnvironment {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::NativeWindows => "native_windows",
            Self::Wsl2 => "wsl2",
        }
    }
}

/// Exact release coordinate for the detected platform boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformReleaseCoordinate {
    /// A native Linux, macOS, or native-Windows boundary.
    Native,
    /// The exact first-release WSL2 distribution and image boundary.
    Wsl2 {
        /// Exact `WSL_DISTRO_NAME` value observed inside the distribution.
        distribution_name: String,
        /// Exact `/etc/os-release` `ID` value.
        distribution_id: String,
        /// Exact `/etc/os-release` `VERSION_ID` value.
        distribution_version: String,
        /// Exact matched support-catalog environment-image coordinate.
        environment_image: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PlatformReleaseCoordinateWire {
    Native(PlatformReleaseCoordinateNativeWire),
    Wsl2(PlatformReleaseCoordinateWsl2Wire),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformReleaseCoordinateNativeWire {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformReleaseCoordinateWsl2Wire {
    distribution_name: String,
    distribution_id: String,
    distribution_version: String,
    environment_image: String,
}

impl<'de> Deserialize<'de> for PlatformReleaseCoordinate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match PlatformReleaseCoordinateWire::deserialize(deserializer)? {
            PlatformReleaseCoordinateWire::Native(_) => Ok(Self::Native),
            PlatformReleaseCoordinateWire::Wsl2(wire) => Ok(Self::Wsl2 {
                distribution_name: wire.distribution_name,
                distribution_id: wire.distribution_id,
                distribution_version: wire.distribution_version,
                environment_image: wire.environment_image,
            }),
        }
    }
}

impl PlatformReleaseCoordinate {
    /// Returns the canonical native-platform coordinate.
    pub const fn native() -> Self {
        Self::Native
    }

    /// Returns the exact first-release WSL2 coordinate.
    pub fn first_release_wsl2() -> Self {
        Self::Wsl2 {
            distribution_name: PINNED_WSL2_DISTRIBUTION_NAME.to_owned(),
            distribution_id: PINNED_WSL2_DISTRIBUTION_ID.to_owned(),
            distribution_version: PINNED_WSL2_DISTRIBUTION_VERSION.to_owned(),
            environment_image: PINNED_WSL2_ENVIRONMENT_IMAGE.to_owned(),
        }
    }

    /// Validates an exact coordinate against its platform environment.
    pub fn validate_for(
        &self,
        platform: PlatformEnvironment,
    ) -> Result<(), ManagedHostContractError> {
        let valid = match (platform, self) {
            (
                PlatformEnvironment::Wsl2,
                Self::Wsl2 {
                    distribution_name,
                    distribution_id,
                    distribution_version,
                    environment_image,
                },
            ) => {
                distribution_name == PINNED_WSL2_DISTRIBUTION_NAME
                    && distribution_id == PINNED_WSL2_DISTRIBUTION_ID
                    && distribution_version == PINNED_WSL2_DISTRIBUTION_VERSION
                    && environment_image == PINNED_WSL2_ENVIRONMENT_IMAGE
            }
            (PlatformEnvironment::Wsl2, Self::Native) => false,
            (_, Self::Native) => true,
            (_, Self::Wsl2 { .. }) => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ManagedHostContractError::new(
                "platform_release_coordinate_invalid",
            ))
        }
    }

    /// Returns the WSL2 environment-image coordinate when present.
    pub fn wsl2_environment_image(&self) -> Option<&str> {
        match self {
            Self::Native => None,
            Self::Wsl2 {
                environment_image, ..
            } => Some(environment_image),
        }
    }
}

/// Capability proven for one exact Codex release artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CodexCapability {
    /// Managed stdio MCP can be launched and retained.
    ManagedStdioMcp,
    /// The exact personal managed-binding lifecycle is supported.
    PersonalManagedBinding,
    /// The first-release Record workflow is supported.
    RecordWorkflow,
    /// The exact shared managed-binding lifecycle is supported.
    SharedManagedBinding,
}

impl CodexCapability {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManagedStdioMcp => "managed_stdio_mcp",
            Self::PersonalManagedBinding => "personal_managed_binding",
            Self::RecordWorkflow => "record_workflow",
            Self::SharedManagedBinding => "shared_managed_binding",
        }
    }
}

/// Complete canonical capability list for the first release.
pub const FIRST_RELEASE_CODEX_CAPABILITIES: [CodexCapability; 4] = [
    CodexCapability::ManagedStdioMcp,
    CodexCapability::PersonalManagedBinding,
    CodexCapability::RecordWorkflow,
    CodexCapability::SharedManagedBinding,
];

/// Returns whether capabilities exactly equal the first-release canonical list.
pub fn has_exact_first_release_codex_capabilities(capabilities: &[CodexCapability]) -> bool {
    capabilities == FIRST_RELEASE_CODEX_CAPABILITIES
}

/// Connection ownership scope represented by a managed binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManagedConnectionScope {
    /// User-owned configuration serving a personal connection.
    Personal,
    /// Project-owned configuration serving a shared connection.
    Shared,
}

impl ManagedConnectionScope {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Shared => "shared",
        }
    }
}

/// Resolution mode for the managed executable command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManagedCommandResolution {
    /// Resolve one basename through the process path.
    PathLookup,
    /// Invoke one normalized absolute path.
    AbsolutePath,
}

impl ManagedCommandResolution {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PathLookup => "path_lookup",
            Self::AbsolutePath => "absolute_path",
        }
    }
}

/// Exact command coordinate for a managed host process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ManagedCommand {
    /// Command resolution behavior.
    pub resolution: ManagedCommandResolution,
    /// Basename or normalized absolute program path.
    pub program: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedCommandWire {
    resolution: ManagedCommandResolution,
    program: String,
}

impl<'de> Deserialize<'de> for ManagedCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ManagedCommandWire::deserialize(deserializer)?;
        let command = Self {
            resolution: wire.resolution,
            program: wire.program,
        };
        command.validate_basic().map_err(D::Error::custom)?;
        Ok(command)
    }
}

impl ManagedCommand {
    fn validate_basic(&self) -> Result<(), ManagedHostContractError> {
        validate_bounded_string(&self.program, false, "managed_command_program_invalid")?;
        if self.resolution == ManagedCommandResolution::PathLookup
            && (self.program == "."
                || self.program == ".."
                || self.program.contains('/')
                || self.program.contains('\\'))
        {
            return Err(ManagedHostContractError::new("path_lookup_program_invalid"));
        }
        Ok(())
    }
}

/// One declared environment-variable forwarding rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct EnvironmentForwarding {
    /// Name read from the launching environment.
    pub source_name: String,
    /// Name supplied to the managed process.
    pub target_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentForwardingWire {
    source_name: String,
    target_name: String,
}

impl<'de> Deserialize<'de> for EnvironmentForwarding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EnvironmentForwardingWire::deserialize(deserializer)?;
        let forwarding = Self {
            source_name: wire.source_name,
            target_name: wire.target_name,
        };
        forwarding.validate().map_err(D::Error::custom)?;
        Ok(forwarding)
    }
}

impl EnvironmentForwarding {
    fn validate(&self) -> Result<(), ManagedHostContractError> {
        if !valid_environment_name(&self.source_name) || !valid_environment_name(&self.target_name)
        {
            return Err(ManagedHostContractError::new(
                "environment_forwarding_name_invalid",
            ));
        }
        Ok(())
    }
}

/// Owner of one exact managed configuration target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationTargetOwner {
    /// User-owned personal configuration.
    User,
    /// Project-owned shared configuration.
    Project,
}

impl ConfigurationTargetOwner {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

/// Exact managed configuration file target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ConfigurationTarget {
    /// Configuration ownership class.
    pub owner: ConfigurationTargetOwner,
    /// Normalized absolute configuration path.
    pub path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigurationTargetWire {
    owner: ConfigurationTargetOwner,
    path: String,
}

impl<'de> Deserialize<'de> for ConfigurationTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConfigurationTargetWire::deserialize(deserializer)?;
        validate_bounded_string(&wire.path, false, "configuration_target_path_invalid")
            .map_err(D::Error::custom)?;
        Ok(Self {
            owner: wire.owner,
            path: wire.path,
        })
    }
}

/// Live process coordinates bound into a managed host identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProcessBinding {
    /// Nonzero operating-system process identifier.
    pub process_id: u64,
    /// Platform observation that distinguishes process-ID reuse.
    pub process_start_token: String,
    /// Platform-instance observation that distinguishes platform restart.
    pub platform_instance_token: String,
    /// Normalized absolute executable path.
    pub executable_path: String,
    /// Raw lowercase hexadecimal SHA-256 of the executable bytes.
    pub executable_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessBindingWire {
    process_id: u64,
    process_start_token: String,
    platform_instance_token: String,
    executable_path: String,
    executable_digest: String,
}

impl<'de> Deserialize<'de> for ProcessBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProcessBindingWire::deserialize(deserializer)?;
        let binding = Self {
            process_id: wire.process_id,
            process_start_token: wire.process_start_token,
            platform_instance_token: wire.platform_instance_token,
            executable_path: wire.executable_path,
            executable_digest: wire.executable_digest,
        };
        binding.validate_basic().map_err(D::Error::custom)?;
        Ok(binding)
    }
}

impl ProcessBinding {
    fn validate_basic(&self) -> Result<(), ManagedHostContractError> {
        if self.process_id == 0 {
            return Err(ManagedHostContractError::new("process_id_invalid"));
        }
        validate_process_token(&self.process_start_token)?;
        validate_process_token(&self.platform_instance_token)?;
        validate_bounded_string(&self.executable_path, false, "executable_path_invalid")?;
        validate_raw_sha256(&self.executable_digest, "executable_digest_invalid")
    }
}

/// Canonical complete binding of one live managed host process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ManagedHostBinding {
    /// Exact managed host family.
    pub host_kind: HostKind,
    /// Personal or shared connection scope.
    pub connection_scope: ManagedConnectionScope,
    /// Managed executable command coordinate.
    pub command: ManagedCommand,
    /// Ordered command arguments.
    pub arguments: Vec<String>,
    /// Canonically ordered environment forwarding declarations.
    pub forwarded_environment: Vec<EnvironmentForwarding>,
    /// Exact configuration file target.
    pub configuration_target: ConfigurationTarget,
    /// Validated live process coordinates.
    pub process_binding: ProcessBinding,
    /// Complete canonical first-release capability set.
    pub required_capabilities: Vec<CodexCapability>,
    /// Exact detected platform environment.
    pub platform_environment: PlatformEnvironment,
    /// Exact native or pinned WSL2 release coordinate.
    pub platform_release_coordinate: PlatformReleaseCoordinate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedHostBindingWire {
    host_kind: HostKind,
    connection_scope: ManagedConnectionScope,
    command: ManagedCommand,
    arguments: Vec<String>,
    forwarded_environment: Vec<EnvironmentForwarding>,
    configuration_target: ConfigurationTarget,
    process_binding: ProcessBinding,
    required_capabilities: Vec<CodexCapability>,
    platform_environment: PlatformEnvironment,
    platform_release_coordinate: PlatformReleaseCoordinate,
}

impl<'de> Deserialize<'de> for ManagedHostBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ManagedHostBindingWire::deserialize(deserializer)?;
        let binding = Self {
            host_kind: wire.host_kind,
            connection_scope: wire.connection_scope,
            command: wire.command,
            arguments: wire.arguments,
            forwarded_environment: wire.forwarded_environment,
            configuration_target: wire.configuration_target,
            process_binding: wire.process_binding,
            required_capabilities: wire.required_capabilities,
            platform_environment: wire.platform_environment,
            platform_release_coordinate: wire.platform_release_coordinate,
        };
        binding.validate().map_err(D::Error::custom)?;
        Ok(binding)
    }
}

impl ManagedHostBinding {
    /// Validates every canonical binding invariant without normalizing input.
    pub fn validate(&self) -> Result<(), ManagedHostContractError> {
        self.command.validate_basic()?;
        self.process_binding.validate_basic()?;
        for argument in &self.arguments {
            validate_bounded_string(argument, true, "managed_argument_invalid")?;
        }
        validate_canonical_environment(&self.forwarded_environment)?;
        if !has_exact_first_release_codex_capabilities(&self.required_capabilities) {
            return Err(ManagedHostContractError::new(
                "required_capabilities_invalid",
            ));
        }
        let expected_owner = match self.connection_scope {
            ManagedConnectionScope::Personal => ConfigurationTargetOwner::User,
            ManagedConnectionScope::Shared => ConfigurationTargetOwner::Project,
        };
        if self.configuration_target.owner != expected_owner {
            return Err(ManagedHostContractError::new(
                "configuration_target_owner_mismatch",
            ));
        }
        if self.command.resolution == ManagedCommandResolution::AbsolutePath {
            validate_canonical_platform_path(self.platform_environment, &self.command.program)?;
        }
        validate_canonical_platform_path(
            self.platform_environment,
            &self.configuration_target.path,
        )?;
        validate_canonical_platform_path(
            self.platform_environment,
            &self.process_binding.executable_path,
        )?;
        self.platform_release_coordinate
            .validate_for(self.platform_environment)?;
        Ok(())
    }

    /// Encodes the binding independently of Serde map order and host endianness.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ManagedHostContractError> {
        self.validate()?;
        encode_record(&[
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[0],
                encode_string(self.host_kind.as_str())?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[1],
                encode_string(self.connection_scope.as_str())?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[2],
                encode_managed_command(&self.command)?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[3],
                encode_string_list(&self.arguments)?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[4],
                encode_environment_list(&self.forwarded_environment)?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[5],
                encode_configuration_target(&self.configuration_target)?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[6],
                encode_process_binding(&self.process_binding)?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[7],
                encode_capability_list(&self.required_capabilities)?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[8],
                encode_string(self.platform_environment.as_str())?,
            ),
            (
                MANAGED_HOST_BINDING_CANONICAL_FIELDS[9],
                encode_platform_release_coordinate(&self.platform_release_coordinate)?,
            ),
        ])
    }

    /// Returns the exact domain-separated binding identity.
    pub fn binding_digest(&self) -> Result<String, ManagedHostContractError> {
        let canonical = self.canonical_bytes()?;
        let mut digest = Sha256::new();
        digest.update(MANAGED_HOST_BINDING_DOMAIN);
        digest.update(canonical);
        Ok(format!("sha256:{:x}", digest.finalize()))
    }
}

/// One generated managed artifact used to compute their aggregate identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GeneratedManagedArtifact {
    /// Normalized absolute platform path.
    pub path: String,
    /// Raw lowercase hexadecimal SHA-256 of artifact bytes.
    pub digest: String,
}

/// Computes the exact sorted identity of generated managed artifacts.
pub fn generated_managed_artifacts_digest(
    platform: PlatformEnvironment,
    artifacts: &[GeneratedManagedArtifact],
) -> Result<String, ManagedHostContractError> {
    let mut ordered = artifacts.to_vec();
    ordered.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    for (index, artifact) in ordered.iter().enumerate() {
        validate_canonical_platform_path(platform, &artifact.path)?;
        validate_raw_sha256(&artifact.digest, "generated_artifact_digest_invalid")?;
        if index > 0 && ordered[index - 1].path == artifact.path {
            return Err(ManagedHostContractError::new(
                "duplicate_generated_artifact_path",
            ));
        }
    }
    let entries = ordered
        .iter()
        .map(|artifact| {
            encode_record(&[
                ("path", encode_string(&artifact.path)?),
                ("digest", encode_string(&artifact.digest)?),
            ])
        })
        .collect::<Result<Vec<_>, _>>()?;
    let canonical = encode_list(&entries)?;
    let mut digest = Sha256::new();
    digest.update(GENERATED_MANAGED_ARTIFACTS_DOMAIN);
    digest.update(canonical);
    Ok(format!("sha256:{:x}", digest.finalize()))
}

/// Successful result value carried by an issued receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostVerificationResult {
    /// Every adapter verification check succeeded.
    Verified,
}

/// Immutable typed evidence for one exact verified managed binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct HostVerificationReceipt {
    /// Exact receipt contract identity.
    pub contract_id: String,
    /// Bound current project.
    pub project_id: ProjectId,
    /// Bound current Agent Connection.
    pub connection_id: AgentConnectionId,
    /// Bound host family.
    pub host_kind: HostKind,
    /// Bound integration profile.
    pub integration_profile: IntegrationProfile,
    /// Bound platform environment.
    pub platform_environment: PlatformEnvironment,
    /// Bound native or pinned WSL2 release coordinate.
    pub platform_release_coordinate: PlatformReleaseCoordinate,
    /// Capabilities required by the connection.
    pub required_capabilities: Vec<CodexCapability>,
    /// Capabilities verified by the adapter.
    pub verified_capabilities: Vec<CodexCapability>,
    /// Domain-separated canonical binding digest.
    pub binding_digest: String,
    /// Domain-separated generated-artifact digest.
    pub generated_artifacts_digest: String,
    /// Raw exact executable digest.
    pub executable_digest: String,
    /// Exact canonical current policy digest.
    pub policy_digest: String,
    /// Raw digest of the verifier executable bytes.
    pub verifier_build_digest: String,
    /// Canonical UTC observation instant.
    pub observed_at: UtcTimestamp,
    /// Exclusive canonical UTC expiry instant.
    pub expires_at: UtcTimestamp,
    /// Successful verification result.
    pub result: HostVerificationResult,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostVerificationReceiptWire {
    contract_id: String,
    project_id: String,
    connection_id: String,
    host_kind: HostKind,
    integration_profile: IntegrationProfile,
    platform_environment: PlatformEnvironment,
    platform_release_coordinate: PlatformReleaseCoordinate,
    required_capabilities: Vec<CodexCapability>,
    verified_capabilities: Vec<CodexCapability>,
    binding_digest: String,
    generated_artifacts_digest: String,
    executable_digest: String,
    policy_digest: String,
    verifier_build_digest: String,
    observed_at: String,
    expires_at: String,
    result: HostVerificationResult,
}

impl<'de> Deserialize<'de> for HostVerificationReceipt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = HostVerificationReceiptWire::deserialize(deserializer)?;
        let observed_at = parse_canonical_timestamp(&wire.observed_at).map_err(D::Error::custom)?;
        let expires_at = parse_canonical_timestamp(&wire.expires_at).map_err(D::Error::custom)?;
        let receipt = Self {
            contract_id: wire.contract_id,
            project_id: ProjectId::new(wire.project_id),
            connection_id: AgentConnectionId::new(wire.connection_id),
            host_kind: wire.host_kind,
            integration_profile: wire.integration_profile,
            platform_environment: wire.platform_environment,
            platform_release_coordinate: wire.platform_release_coordinate,
            required_capabilities: wire.required_capabilities,
            verified_capabilities: wire.verified_capabilities,
            binding_digest: wire.binding_digest,
            generated_artifacts_digest: wire.generated_artifacts_digest,
            executable_digest: wire.executable_digest,
            policy_digest: wire.policy_digest,
            verifier_build_digest: wire.verifier_build_digest,
            observed_at,
            expires_at,
            result: wire.result,
        };
        receipt.validate_shape().map_err(D::Error::custom)?;
        Ok(receipt)
    }
}

impl HostVerificationReceipt {
    /// Validates the receipt's closed structural contract before Core binding checks.
    pub fn validate_shape(&self) -> Result<(), ManagedHostContractError> {
        if self.contract_id != HOST_VERIFICATION_RECEIPT_CONTRACT_ID {
            return Err(ManagedHostContractError::new(
                "host_verification_contract_id_invalid",
            ));
        }
        validate_receipt_identifier(self.project_id.as_str())?;
        validate_receipt_identifier(self.connection_id.as_str())?;
        self.platform_release_coordinate
            .validate_for(self.platform_environment)?;
        if !has_exact_first_release_codex_capabilities(&self.required_capabilities)
            || self.verified_capabilities != self.required_capabilities
        {
            return Err(ManagedHostContractError::new(
                "receipt_capabilities_invalid",
            ));
        }
        validate_prefixed_sha256(&self.binding_digest, "binding_digest_invalid")?;
        validate_prefixed_sha256(
            &self.generated_artifacts_digest,
            "generated_artifacts_digest_invalid",
        )?;
        validate_raw_sha256(&self.executable_digest, "executable_digest_invalid")?;
        validate_prefixed_sha256(&self.policy_digest, "policy_digest_invalid")?;
        validate_raw_sha256(&self.verifier_build_digest, "verifier_build_digest_invalid")?;
        self.observed_at
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| ManagedHostContractError::new("observed_at_invalid"))?;
        self.expires_at
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| ManagedHostContractError::new("expires_at_invalid"))?;
        if self.observed_at >= self.expires_at {
            return Err(ManagedHostContractError::new("receipt_time_window_invalid"));
        }
        Ok(())
    }
}

/// Current typed Store and adapter facts against which Core validates a receipt.
///
/// This value contains identities only. It deliberately carries no host
/// configuration syntax, generated file content, shell command, or process
/// inspection behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentHostReceiptContext {
    /// Current resolved project.
    pub project_id: ProjectId,
    /// Current resolved Agent Connection.
    pub connection_id: AgentConnectionId,
    /// Current connection host family.
    pub host_kind: HostKind,
    /// Current connection integration profile.
    pub integration_profile: IntegrationProfile,
    /// Current binding platform.
    pub platform_environment: PlatformEnvironment,
    /// Current native or pinned WSL2 release coordinate.
    pub platform_release_coordinate: PlatformReleaseCoordinate,
    /// Current exact required capability set.
    pub required_capabilities: Vec<CodexCapability>,
    /// Digest of the current stored canonical binding.
    pub binding_digest: String,
    /// Digest of the current generated managed artifacts.
    pub generated_artifacts_digest: String,
    /// Digest of the current observed executable and matched support entry.
    pub executable_digest: String,
    /// Digest of the current canonical policy basis.
    pub policy_digest: String,
    /// Digest of the currently accepted verifier executable.
    pub verifier_build_digest: String,
}

impl CurrentHostReceiptContext {
    /// Validates the complete typed comparison basis without normalizing it.
    pub fn validate(&self) -> Result<(), ManagedHostContractError> {
        validate_receipt_identifier(self.project_id.as_str())?;
        validate_receipt_identifier(self.connection_id.as_str())?;
        self.platform_release_coordinate
            .validate_for(self.platform_environment)?;
        if !has_exact_first_release_codex_capabilities(&self.required_capabilities) {
            return Err(ManagedHostContractError::new(
                "current_required_capabilities_invalid",
            ));
        }
        validate_prefixed_sha256(&self.binding_digest, "current_binding_digest_invalid")?;
        validate_prefixed_sha256(
            &self.generated_artifacts_digest,
            "current_generated_artifacts_digest_invalid",
        )?;
        validate_raw_sha256(&self.executable_digest, "current_executable_digest_invalid")?;
        validate_prefixed_sha256(&self.policy_digest, "current_policy_digest_invalid")?;
        validate_raw_sha256(
            &self.verifier_build_digest,
            "current_verifier_build_digest_invalid",
        )
    }
}

/// Stable validation failure for canonical managed-host contract input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedHostContractError {
    reason: &'static str,
}

impl ManagedHostContractError {
    const fn new(reason: &'static str) -> Self {
        Self { reason }
    }

    /// Returns the stable machine-readable failure reason.
    pub const fn reason(self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for ManagedHostContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)
    }
}

impl Error for ManagedHostContractError {}

/// Validates one platform path as an already-normalized absolute identity.
pub fn validate_canonical_platform_path(
    platform: PlatformEnvironment,
    path: &str,
) -> Result<(), ManagedHostContractError> {
    validate_bounded_string(path, false, "canonical_platform_path_invalid")?;
    let valid = match platform {
        PlatformEnvironment::Linux | PlatformEnvironment::Macos => valid_unix_path(path),
        PlatformEnvironment::Wsl2 => {
            valid_unix_path(path) && path != "/mnt" && !path.starts_with("/mnt/")
        }
        PlatformEnvironment::NativeWindows => valid_native_windows_path(path),
    };
    if valid {
        Ok(())
    } else {
        Err(ManagedHostContractError::new(
            "canonical_platform_path_invalid",
        ))
    }
}

fn validate_bounded_string(
    value: &str,
    allow_empty: bool,
    reason: &'static str,
) -> Result<(), ManagedHostContractError> {
    if (!allow_empty && value.is_empty())
        || value.len() > MAX_BINDING_STRING_BYTES
        || value.as_bytes().contains(&0)
    {
        Err(ManagedHostContractError::new(reason))
    } else {
        Ok(())
    }
}

fn validate_process_token(value: &str) -> Result<(), ManagedHostContractError> {
    if value.is_empty()
        || value.len() > MAX_PROCESS_TOKEN_BYTES
        || value.chars().any(char::is_control)
    {
        Err(ManagedHostContractError::new("process_token_invalid"))
    } else {
        Ok(())
    }
}

fn validate_receipt_identifier(value: &str) -> Result<(), ManagedHostContractError> {
    if value.is_empty()
        || value.len() > MAX_RECEIPT_IDENTIFIER_BYTES
        || !value.chars().any(|character| !character.is_whitespace())
        || value.chars().any(char::is_control)
    {
        Err(ManagedHostContractError::new("receipt_identifier_invalid"))
    } else {
        Ok(())
    }
}

fn valid_environment_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'_'))
        && bytes.all(|byte| matches!(byte, b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}

fn validate_canonical_environment(
    values: &[EnvironmentForwarding],
) -> Result<(), ManagedHostContractError> {
    for value in values {
        value.validate()?;
    }
    for pair in values.windows(2) {
        let left = (&pair[0].target_name, &pair[0].source_name);
        let right = (&pair[1].target_name, &pair[1].source_name);
        if left >= right || pair[0].target_name == pair[1].target_name {
            return Err(ManagedHostContractError::new(
                "environment_forwarding_order_invalid",
            ));
        }
    }
    Ok(())
}

fn valid_unix_path(path: &str) -> bool {
    if path == "/" {
        return true;
    }
    if !path.starts_with('/') || path.contains('\\') || path.contains("//") {
        return false;
    }
    if path.len() > 1 && path.ends_with('/') {
        return false;
    }
    path.split('/')
        .skip(1)
        .all(|component| !matches!(component, "" | "." | ".."))
}

fn valid_native_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_uppercase()
        || bytes[1] != b':'
        || bytes[2] != b'/'
        || path.contains('\\')
        || path[3..].contains("//")
    {
        return false;
    }
    if bytes.len() > 3 && path.ends_with('/') {
        return false;
    }
    path[3..]
        .split('/')
        .all(|component| !matches!(component, "." | "..") && !component.is_empty())
        || bytes.len() == 3
}

fn validate_raw_sha256(value: &str, reason: &'static str) -> Result<(), ManagedHostContractError> {
    if is_canonical_sha256_hex(value) {
        Ok(())
    } else {
        Err(ManagedHostContractError::new(reason))
    }
}

fn validate_prefixed_sha256(
    value: &str,
    reason: &'static str,
) -> Result<(), ManagedHostContractError> {
    if is_canonical_sha256_digest(value) {
        Ok(())
    } else {
        Err(ManagedHostContractError::new(reason))
    }
}

fn parse_canonical_timestamp(value: &str) -> Result<UtcTimestamp, ManagedHostContractError> {
    let timestamp = UtcTimestamp::parse(value)
        .map_err(|_| ManagedHostContractError::new("receipt_timestamp_invalid"))?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| ManagedHostContractError::new("receipt_timestamp_invalid"))?;
    if timestamp.to_canonical_string() != value {
        return Err(ManagedHostContractError::new(
            "receipt_timestamp_not_canonical",
        ));
    }
    Ok(timestamp)
}

fn checked_u32(value: usize) -> Result<u32, ManagedHostContractError> {
    u32::try_from(value).map_err(|_| ManagedHostContractError::new("canonical_length_overflow"))
}

fn encode_blob(bytes: &[u8]) -> Result<Vec<u8>, ManagedHostContractError> {
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&checked_u32(bytes.len())?.to_be_bytes());
    encoded.extend_from_slice(bytes);
    Ok(encoded)
}

fn encode_string(value: &str) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_blob(value.as_bytes())
}

fn encode_list(items: &[Vec<u8>]) -> Result<Vec<u8>, ManagedHostContractError> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&checked_u32(items.len())?.to_be_bytes());
    for item in items {
        encoded.extend_from_slice(&encode_blob(item)?);
    }
    Ok(encoded)
}

fn encode_record(fields: &[(&str, Vec<u8>)]) -> Result<Vec<u8>, ManagedHostContractError> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&checked_u32(fields.len())?.to_be_bytes());
    for (name, value) in fields {
        encoded.extend_from_slice(&encode_string(name)?);
        encoded.extend_from_slice(&encode_blob(value)?);
    }
    Ok(encoded)
}

fn encode_string_list(values: &[String]) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_list(
        &values
            .iter()
            .map(|value| encode_string(value))
            .collect::<Result<Vec<_>, _>>()?,
    )
}

fn encode_capability_list(values: &[CodexCapability]) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_list(
        &values
            .iter()
            .map(|value| encode_string(value.as_str()))
            .collect::<Result<Vec<_>, _>>()?,
    )
}

fn encode_environment_list(
    values: &[EnvironmentForwarding],
) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_list(
        &values
            .iter()
            .map(|value| {
                encode_record(&[
                    ("source_name", encode_string(&value.source_name)?),
                    ("target_name", encode_string(&value.target_name)?),
                ])
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
}

fn encode_managed_command(value: &ManagedCommand) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_record(&[
        ("resolution", encode_string(value.resolution.as_str())?),
        ("program", encode_string(&value.program)?),
    ])
}

fn encode_configuration_target(
    value: &ConfigurationTarget,
) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_record(&[
        ("owner", encode_string(value.owner.as_str())?),
        ("path", encode_string(&value.path)?),
    ])
}

fn encode_process_binding(value: &ProcessBinding) -> Result<Vec<u8>, ManagedHostContractError> {
    encode_record(&[
        ("process_id", value.process_id.to_be_bytes().to_vec()),
        (
            "process_start_token",
            encode_string(&value.process_start_token)?,
        ),
        (
            "platform_instance_token",
            encode_string(&value.platform_instance_token)?,
        ),
        ("executable_path", encode_string(&value.executable_path)?),
        (
            "executable_digest",
            encode_string(&value.executable_digest)?,
        ),
    ])
}

fn encode_platform_release_coordinate(
    value: &PlatformReleaseCoordinate,
) -> Result<Vec<u8>, ManagedHostContractError> {
    match value {
        PlatformReleaseCoordinate::Native => encode_record(&[("kind", encode_string("native")?)]),
        PlatformReleaseCoordinate::Wsl2 {
            distribution_name,
            distribution_id,
            distribution_version,
            environment_image,
        } => encode_record(&[
            ("kind", encode_string("wsl2")?),
            ("distribution_name", encode_string(distribution_name)?),
            ("distribution_id", encode_string(distribution_id)?),
            ("distribution_version", encode_string(distribution_version)?),
            ("environment_image", encode_string(environment_image)?),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    const RAW_DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const PREFIXED_DIGEST: &str =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn linux_binding() -> ManagedHostBinding {
        ManagedHostBinding {
            host_kind: HostKind::Codex,
            connection_scope: ManagedConnectionScope::Shared,
            command: ManagedCommand {
                resolution: ManagedCommandResolution::AbsolutePath,
                program: "/opt/codex/bin/codex".to_owned(),
            },
            arguments: vec!["mcp".to_owned(), "--stdio".to_owned(), String::new()],
            forwarded_environment: vec![EnvironmentForwarding {
                source_name: "VOLICORD_HOME".to_owned(),
                target_name: "VOLICORD_HOME".to_owned(),
            }],
            configuration_target: ConfigurationTarget {
                owner: ConfigurationTargetOwner::Project,
                path: "/workspace/.codex/config.toml".to_owned(),
            },
            process_binding: ProcessBinding {
                process_id: 42,
                process_start_token: "boot-7:process-42".to_owned(),
                platform_instance_token: "boot-7".to_owned(),
                executable_path: "/opt/codex/bin/codex".to_owned(),
                executable_digest: RAW_DIGEST.to_owned(),
            },
            required_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
            platform_environment: PlatformEnvironment::Linux,
            platform_release_coordinate: PlatformReleaseCoordinate::Native,
        }
    }

    fn receipt_value() -> Value {
        json!({
            "contract_id": HOST_VERIFICATION_RECEIPT_CONTRACT_ID,
            "project_id": "project-1",
            "connection_id": "connection-1",
            "host_kind": "codex",
            "integration_profile": "record",
            "platform_environment": "linux",
            "platform_release_coordinate": {"kind": "native"},
            "required_capabilities": [
                "managed_stdio_mcp",
                "personal_managed_binding",
                "record_workflow",
                "shared_managed_binding"
            ],
            "verified_capabilities": [
                "managed_stdio_mcp",
                "personal_managed_binding",
                "record_workflow",
                "shared_managed_binding"
            ],
            "binding_digest": PREFIXED_DIGEST,
            "generated_artifacts_digest": PREFIXED_DIGEST,
            "executable_digest": RAW_DIGEST,
            "policy_digest": PREFIXED_DIGEST,
            "verifier_build_digest": RAW_DIGEST,
            "observed_at": "2026-07-17T01:02:03Z",
            "expires_at": "2026-07-17T01:07:03Z",
            "result": "verified"
        })
    }

    #[test]
    fn binding_codec_has_one_stable_domain_separated_digest() {
        let binding = linux_binding();
        assert!(binding.canonical_bytes().unwrap().len() > 500);
        assert_eq!(
            binding.binding_digest().unwrap(),
            "sha256:ec7442e92d32e3049815d9f314aeba32d064d857679f04aa0733dda5d484585b"
        );

        let mut changed = binding;
        changed.arguments.swap(0, 1);
        assert_ne!(
            changed.binding_digest().unwrap(),
            "sha256:ec7442e92d32e3049815d9f314aeba32d064d857679f04aa0733dda5d484585b"
        );
    }

    #[test]
    fn binding_json_is_closed_and_requires_canonical_ordered_values() {
        let encoded = serde_json::to_value(linux_binding()).unwrap();
        let decoded: ManagedHostBinding = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, linux_binding());

        let mut unknown = encoded.clone();
        unknown["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ManagedHostBinding>(unknown).is_err());

        let mut reordered_capabilities = encoded.clone();
        reordered_capabilities["required_capabilities"] = json!([
            "record_workflow",
            "managed_stdio_mcp",
            "personal_managed_binding",
            "shared_managed_binding"
        ]);
        assert!(serde_json::from_value::<ManagedHostBinding>(reordered_capabilities).is_err());

        let mut duplicate_target = encoded;
        duplicate_target["forwarded_environment"] = json!([
            {"source_name": "HOME", "target_name": "VOLICORD_HOME"},
            {"source_name": "VOLICORD_HOME", "target_name": "VOLICORD_HOME"}
        ]);
        assert!(serde_json::from_value::<ManagedHostBinding>(duplicate_target).is_err());
    }

    #[test]
    fn command_scope_and_process_invariants_fail_closed() {
        let mut binding = linux_binding();
        binding.command.resolution = ManagedCommandResolution::PathLookup;
        binding.command.program = "bin/codex".to_owned();
        assert_eq!(
            binding.validate().unwrap_err().reason(),
            "path_lookup_program_invalid"
        );

        let mut binding = linux_binding();
        binding.configuration_target.owner = ConfigurationTargetOwner::User;
        assert_eq!(
            binding.validate().unwrap_err().reason(),
            "configuration_target_owner_mismatch"
        );

        let mut binding = linux_binding();
        binding.process_binding.process_id = 0;
        assert_eq!(
            binding.validate().unwrap_err().reason(),
            "process_id_invalid"
        );
    }

    #[test]
    fn platform_paths_are_exact_and_wsl2_rejects_drvfs() {
        for path in ["/", "/home/user/.codex/config.toml"] {
            validate_canonical_platform_path(PlatformEnvironment::Linux, path).unwrap();
        }
        for path in ["relative", "/home//user", "/home/../user", "/home/user/"] {
            assert!(validate_canonical_platform_path(PlatformEnvironment::Linux, path).is_err());
        }
        validate_canonical_platform_path(
            PlatformEnvironment::NativeWindows,
            "C:/Users/A/.codex/config.toml",
        )
        .unwrap();
        for path in [
            "c:/Users/A",
            "C:\\Users\\A",
            "//server/share",
            "C:/Users/../A",
        ] {
            assert!(
                validate_canonical_platform_path(PlatformEnvironment::NativeWindows, path).is_err()
            );
        }
        assert!(
            validate_canonical_platform_path(PlatformEnvironment::Wsl2, "/mnt/c/Users/A").is_err()
        );
        validate_canonical_platform_path(PlatformEnvironment::Wsl2, "/home/a/.codex/config.toml")
            .unwrap();
    }

    #[test]
    fn platform_release_coordinate_is_closed_and_platform_bound() {
        let exact = PlatformReleaseCoordinate::first_release_wsl2();
        exact
            .validate_for(PlatformEnvironment::Wsl2)
            .expect("the pinned WSL2 coordinate should be valid");
        assert_eq!(
            exact
                .validate_for(PlatformEnvironment::Linux)
                .unwrap_err()
                .reason(),
            "platform_release_coordinate_invalid"
        );
        assert_eq!(
            PlatformReleaseCoordinate::Native
                .validate_for(PlatformEnvironment::Wsl2)
                .unwrap_err()
                .reason(),
            "platform_release_coordinate_invalid"
        );

        let mut wrong_distribution = exact.clone();
        let PlatformReleaseCoordinate::Wsl2 {
            distribution_name, ..
        } = &mut wrong_distribution
        else {
            unreachable!("exact coordinate is WSL2")
        };
        *distribution_name = "Ubuntu-22.04".to_owned();
        assert_eq!(
            wrong_distribution
                .validate_for(PlatformEnvironment::Wsl2)
                .unwrap_err()
                .reason(),
            "platform_release_coordinate_invalid"
        );

        assert!(serde_json::from_value::<PlatformReleaseCoordinate>(json!({
            "kind": "native",
            "distribution_name": "Ubuntu-24.04"
        }))
        .is_err());
    }

    #[test]
    fn generated_artifact_identity_sorts_paths_and_rejects_duplicates() {
        let left = GeneratedManagedArtifact {
            path: "/workspace/.codex/config.toml".to_owned(),
            digest: RAW_DIGEST.to_owned(),
        };
        let right = GeneratedManagedArtifact {
            path: "/workspace/.codex/volicord.rules".to_owned(),
            digest: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_owned(),
        };
        let forward = generated_managed_artifacts_digest(
            PlatformEnvironment::Linux,
            &[left.clone(), right.clone()],
        )
        .unwrap();
        let reverse =
            generated_managed_artifacts_digest(PlatformEnvironment::Linux, &[right, left.clone()])
                .unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            forward,
            "sha256:3c91acae75c3ac96fad37aee30fda7d3de0c5675ab43f2858e2d9030178fdd41"
        );
        assert_eq!(
            generated_managed_artifacts_digest(PlatformEnvironment::Linux, &[left.clone(), left])
                .unwrap_err()
                .reason(),
            "duplicate_generated_artifact_path"
        );
    }

    #[test]
    fn receipt_decode_is_closed_canonical_and_complete() {
        let receipt: HostVerificationReceipt = serde_json::from_value(receipt_value()).unwrap();
        receipt.validate_shape().unwrap();

        let mut unknown = receipt_value();
        unknown["unexpected"] = json!(true);
        assert!(serde_json::from_value::<HostVerificationReceipt>(unknown).is_err());

        let mut noncanonical_time = receipt_value();
        noncanonical_time["observed_at"] = json!("2026-07-17T10:02:03+09:00");
        assert!(serde_json::from_value::<HostVerificationReceipt>(noncanonical_time).is_err());

        let mut expired_at_observation = receipt_value();
        expired_at_observation["expires_at"] = json!("2026-07-17T01:02:03Z");
        assert!(serde_json::from_value::<HostVerificationReceipt>(expired_at_observation).is_err());

        let mut incomplete_capabilities = receipt_value();
        incomplete_capabilities["verified_capabilities"] = json!(["managed_stdio_mcp"]);
        assert!(
            serde_json::from_value::<HostVerificationReceipt>(incomplete_capabilities).is_err()
        );
    }

    #[test]
    fn receipt_identifiers_and_digests_are_not_normalized() {
        let mut blank_project = receipt_value();
        blank_project["project_id"] = json!("   ");
        assert!(serde_json::from_value::<HostVerificationReceipt>(blank_project).is_err());

        let mut uppercase_digest = receipt_value();
        uppercase_digest["binding_digest"] =
            json!("sha256:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef");
        assert!(serde_json::from_value::<HostVerificationReceipt>(uppercase_digest).is_err());
    }
}
