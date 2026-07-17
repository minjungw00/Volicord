use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use volicord_types::{
    generated_managed_artifacts_digest, lookup_embedded_codex_support_entry, AgentConnectionId,
    CodexCapability, ConfigurationTarget, EnvironmentForwarding, FailureCategory,
    GeneratedManagedArtifact, HostVerificationReceipt, HostVerificationResult, IntegrationProfile,
    ManagedCommand, ManagedCommandResolution, ManagedHostBinding, PlatformEnvironment,
    PlatformReleaseCoordinate, ProjectId, ReleaseTargetTriple, UtcTimestamp,
    FIRST_RELEASE_CODEX_CAPABILITIES, HOST_VERIFICATION_RECEIPT_CONTRACT_ID,
};

use crate::host_integration::process::canonical_existing_platform_path;
use crate::host_integration::{HostKind, HostPlan, HostTarget};

use super::executable::CodexExecutableAvailability;

pub(crate) trait CodexSupportCatalogPolicy {
    fn require_exact_supported_entry(
        &self,
        codex_artifact_digest: &str,
        target_triple: ReleaseTargetTriple,
        platform: PlatformEnvironment,
        platform_release_coordinate: &PlatformReleaseCoordinate,
        capabilities: &[CodexCapability],
        profile: IntegrationProfile,
    ) -> Result<(), ManagedHostEvidenceError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EmbeddedCodexSupportCatalogPolicy;

impl CodexSupportCatalogPolicy for EmbeddedCodexSupportCatalogPolicy {
    fn require_exact_supported_entry(
        &self,
        codex_artifact_digest: &str,
        target_triple: ReleaseTargetTriple,
        platform: PlatformEnvironment,
        platform_release_coordinate: &PlatformReleaseCoordinate,
        capabilities: &[CodexCapability],
        profile: IntegrationProfile,
    ) -> Result<(), ManagedHostEvidenceError> {
        platform_release_coordinate
            .validate_for(platform)
            .map_err(|error| {
                ManagedHostEvidenceError::new(
                    FailureCategory::UnsupportedContract,
                    error.reason(),
                    "the observed platform release coordinate is not supported",
                )
            })?;
        lookup_embedded_codex_support_entry(
            codex_artifact_digest,
            target_triple,
            platform,
            platform_release_coordinate,
            capabilities,
            profile,
        )
        .map_err(|error| {
            ManagedHostEvidenceError::new(
                error.failure_category(),
                error.reason(),
                "the observed Codex artifact has no exact embedded support-catalog entry",
            )
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ManagedHostEvidence {
    pub(crate) binding: ManagedHostBinding,
    pub(crate) binding_digest: String,
    pub(crate) generated_artifacts: Vec<GeneratedManagedArtifact>,
    pub(crate) generated_artifacts_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedHostEvidenceError {
    category: FailureCategory,
    reason: &'static str,
    detail: String,
}

impl ManagedHostEvidenceError {
    pub(crate) fn new(
        category: FailureCategory,
        reason: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            category,
            reason,
            detail: detail.into(),
        }
    }

    pub(crate) const fn category(&self) -> FailureCategory {
        self.category
    }

    pub(crate) const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl std::fmt::Display for ManagedHostEvidenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.reason, self.detail)
    }
}

impl std::error::Error for ManagedHostEvidenceError {}

pub(crate) fn managed_host_evidence_for_plan(
    plan: &HostPlan,
    executable: &CodexExecutableAvailability,
    support_catalog: &impl CodexSupportCatalogPolicy,
) -> Result<ManagedHostEvidence, ManagedHostEvidenceError> {
    let platform = executable.platform_environment.ok_or_else(|| {
        unavailable(
            "platform_environment_unavailable",
            "the Codex process observation did not carry a platform environment",
        )
    })?;
    let target_triple = executable.target_triple.ok_or_else(|| {
        unavailable(
            "target_triple_unavailable",
            "the Codex process observation did not carry an exact target triple",
        )
    })?;
    let process_binding = executable.process_binding.clone().ok_or_else(|| {
        unavailable(
            "process_binding_unavailable",
            "the Codex process observation did not carry live process coordinates",
        )
    })?;
    let platform_release_coordinate =
        executable
            .platform_release_coordinate
            .clone()
            .ok_or_else(|| {
                unavailable(
                    "platform_release_coordinate_unavailable",
                    "the Codex process observation did not carry a platform release coordinate",
                )
            })?;
    platform_release_coordinate
        .validate_for(platform)
        .map_err(|error| {
            rejected(
                error.reason(),
                "the Codex process platform release coordinate is not canonical",
            )
        })?;
    let HostTarget::File(configuration_path) = &plan.target else {
        return Err(rejected(
            "configuration_target_invalid",
            "the Codex managed configuration target is not a file",
        ));
    };
    let configuration_target_path = canonical_existing_platform_path(configuration_path, platform)
        .map_err(|detail| platform_path_failure("canonical_platform_path_unavailable", detail))?;
    let artifact_bytes = fs::read(configuration_path).map_err(|error| {
        unavailable(
            "generated_artifact_read_failed",
            format!("the managed Codex configuration could not be read: {error}"),
        )
    })?;
    managed_host_evidence_for_live_process(
        plan,
        process_binding,
        target_triple,
        platform,
        platform_release_coordinate,
        vec![GeneratedManagedArtifact {
            path: configuration_target_path,
            digest: format!("{:x}", Sha256::digest(artifact_bytes)),
        }],
        support_catalog,
    )
}

pub(crate) fn managed_host_evidence_for_live_process(
    plan: &HostPlan,
    process_binding: volicord_types::ProcessBinding,
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    platform_release_coordinate: PlatformReleaseCoordinate,
    generated_artifacts: Vec<GeneratedManagedArtifact>,
    support_catalog: &impl CodexSupportCatalogPolicy,
) -> Result<ManagedHostEvidence, ManagedHostEvidenceError> {
    if !plan.entry.env.is_empty() {
        return Err(rejected(
            "managed_environment_values_unsupported",
            "canonical managed bindings accept forwarding declarations, not stored environment values",
        ));
    }
    let HostTarget::File(configuration_path) = &plan.target else {
        return Err(rejected(
            "configuration_target_invalid",
            "the Codex managed configuration target is not a file",
        ));
    };
    let configuration_target_path = canonical_existing_platform_path(configuration_path, platform)
        .map_err(|detail| platform_path_failure("canonical_platform_path_unavailable", detail))?;
    let command = managed_command(&plan.entry.command, platform)?;
    let forwarded_environment = plan
        .entry
        .env_vars
        .iter()
        .map(|name| EnvironmentForwarding {
            source_name: name.clone(),
            target_name: name.clone(),
        })
        .collect::<Vec<_>>();
    let binding = ManagedHostBinding {
        host_kind: HostKind::Codex,
        connection_scope: plan.connection_intent,
        command,
        arguments: plan.entry.args.clone(),
        forwarded_environment,
        configuration_target: ConfigurationTarget {
            owner: plan.host_scope,
            path: configuration_target_path.clone(),
        },
        process_binding,
        required_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        platform_environment: platform,
        platform_release_coordinate: platform_release_coordinate.clone(),
    };
    binding.validate().map_err(|error| {
        rejected(
            error.reason(),
            "the observed Codex managed binding is not canonical",
        )
    })?;
    let binding_digest = binding.binding_digest().map_err(|error| {
        rejected(
            error.reason(),
            "the canonical Codex binding digest could not be computed",
        )
    })?;
    if platform == PlatformEnvironment::Wsl2 {
        for artifact in &generated_artifacts {
            let observed_path =
                canonical_existing_platform_path(Path::new(&artifact.path), platform).map_err(
                    |detail| platform_path_failure("generated_artifact_path_unavailable", detail),
                )?;
            if observed_path != artifact.path {
                return Err(rejected(
                    "generated_artifact_path_not_canonical",
                    "a managed artifact path differs from its canonical ext4 identity",
                ));
            }
        }
    }
    let generated_artifacts_digest =
        generated_managed_artifacts_digest(platform, &generated_artifacts).map_err(|error| {
            rejected(
                error.reason(),
                "the generated managed-artifact identity is not canonical",
            )
        })?;
    support_catalog.require_exact_supported_entry(
        &binding.process_binding.executable_digest,
        target_triple,
        platform,
        &platform_release_coordinate,
        &binding.required_capabilities,
        IntegrationProfile::Record,
    )?;
    if !generated_artifacts
        .iter()
        .any(|artifact| artifact.path == configuration_target_path)
    {
        return Err(rejected(
            "managed_configuration_artifact_missing",
            "the complete managed-artifact inventory omits the Codex configuration target",
        ));
    }
    Ok(ManagedHostEvidence {
        binding,
        binding_digest,
        generated_artifacts,
        generated_artifacts_digest,
    })
}

fn managed_command(
    command: &str,
    platform: PlatformEnvironment,
) -> Result<ManagedCommand, ManagedHostEvidenceError> {
    let path = Path::new(command);
    if path.is_absolute() {
        let canonical = canonical_existing_platform_path(path, platform)
            .map_err(|detail| unavailable("managed_command_path_unavailable", detail))?;
        return Ok(ManagedCommand {
            resolution: ManagedCommandResolution::AbsolutePath,
            program: canonical,
        });
    }
    if path.components().count() == 1 && !command.contains(['/', '\\']) {
        return Ok(ManagedCommand {
            resolution: ManagedCommandResolution::PathLookup,
            program: command.to_owned(),
        });
    }
    Err(rejected(
        "managed_command_program_invalid",
        "the managed command is neither one PATH basename nor an absolute canonical path",
    ))
}

pub(crate) struct HostVerificationReceiptIssue<'a> {
    pub(crate) project_id: &'a str,
    pub(crate) connection_id: &'a str,
    pub(crate) evidence: &'a ManagedHostEvidence,
    pub(crate) policy_digest: &'a str,
    pub(crate) verifier_build_digest: &'a str,
    pub(crate) observed_at: UtcTimestamp,
    pub(crate) expires_at: UtcTimestamp,
}

pub(crate) fn issue_host_verification_receipt(
    input: HostVerificationReceiptIssue<'_>,
) -> Result<HostVerificationReceipt, ManagedHostEvidenceError> {
    let HostVerificationReceiptIssue {
        project_id,
        connection_id,
        evidence,
        policy_digest,
        verifier_build_digest,
        observed_at,
        expires_at,
    } = input;
    let receipt = HostVerificationReceipt {
        contract_id: HOST_VERIFICATION_RECEIPT_CONTRACT_ID.to_owned(),
        project_id: ProjectId::new(project_id),
        connection_id: AgentConnectionId::new(connection_id),
        host_kind: HostKind::Codex,
        integration_profile: IntegrationProfile::Record,
        platform_environment: evidence.binding.platform_environment,
        platform_release_coordinate: evidence.binding.platform_release_coordinate.clone(),
        required_capabilities: evidence.binding.required_capabilities.clone(),
        verified_capabilities: evidence.binding.required_capabilities.clone(),
        binding_digest: evidence.binding_digest.clone(),
        generated_artifacts_digest: evidence.generated_artifacts_digest.clone(),
        executable_digest: evidence.binding.process_binding.executable_digest.clone(),
        policy_digest: policy_digest.to_owned(),
        verifier_build_digest: verifier_build_digest.to_owned(),
        observed_at,
        expires_at,
        result: HostVerificationResult::Verified,
    };
    receipt.validate_shape().map_err(|error| {
        rejected(
            error.reason(),
            "the completed Codex verification could not issue a canonical receipt",
        )
    })?;
    Ok(receipt)
}

fn rejected(reason: &'static str, detail: impl Into<String>) -> ManagedHostEvidenceError {
    ManagedHostEvidenceError::new(FailureCategory::Rejected, reason, detail)
}

fn unavailable(reason: &'static str, detail: impl Into<String>) -> ManagedHostEvidenceError {
    ManagedHostEvidenceError::new(FailureCategory::Unavailable, reason, detail)
}

fn platform_path_failure(
    unavailable_reason: &'static str,
    detail: String,
) -> ManagedHostEvidenceError {
    if detail == "unsupported_wsl2_filesystem" || detail.starts_with("unsupported_wsl2_filesystem:")
    {
        ManagedHostEvidenceError::new(
            FailureCategory::UnsupportedContract,
            "unsupported_wsl2_filesystem",
            detail,
        )
    } else {
        unavailable(unavailable_reason, detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ConfigurationTargetOwner as HostScope, ProcessBinding, FIRST_RELEASE_CODEX_CAPABILITIES,
    };

    const RAW_DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn canonical_wsl2_binding_rejects_drvfs_configuration() {
        let mut binding = binding_for(
            PlatformEnvironment::Wsl2,
            "/home/user/.codex/config.toml",
            "/home/user/.local/bin/codex",
        );
        binding.validate().expect("ext4-shaped WSL2 binding");
        binding.configuration_target.path = "/mnt/c/repo/.codex/config.toml".to_owned();
        assert_eq!(
            binding.validate().unwrap_err().reason(),
            "canonical_platform_path_invalid"
        );
    }

    #[test]
    fn canonical_native_windows_binding_uses_drive_and_forward_slashes() {
        let binding = binding_for(
            PlatformEnvironment::NativeWindows,
            "C:/Users/A/.codex/config.toml",
            "C:/Users/A/AppData/Local/codex.exe",
        );
        binding
            .validate()
            .expect("canonical native Windows binding");
        let mut invalid = binding;
        invalid.process_binding.executable_path = "C:\\Users\\A\\codex.exe".to_owned();
        assert_eq!(
            invalid.validate().unwrap_err().reason(),
            "canonical_platform_path_invalid"
        );
    }

    #[test]
    fn wsl2_filesystem_failure_keeps_unsupported_contract_identity() {
        let error = platform_path_failure(
            "generated_artifact_path_unavailable",
            "unsupported_wsl2_filesystem: managed artifact is on DrvFS".to_owned(),
        );
        assert_eq!(error.category(), FailureCategory::UnsupportedContract);
        assert_eq!(error.reason(), "unsupported_wsl2_filesystem");
    }

    fn binding_for(
        platform_environment: PlatformEnvironment,
        configuration_path: &str,
        executable_path: &str,
    ) -> ManagedHostBinding {
        ManagedHostBinding {
            host_kind: HostKind::Codex,
            connection_scope: volicord_types::ManagedConnectionScope::Shared,
            command: ManagedCommand {
                resolution: ManagedCommandResolution::PathLookup,
                program: "volicord".to_owned(),
            },
            arguments: vec!["mcp".to_owned(), "--stdio".to_owned()],
            forwarded_environment: vec![EnvironmentForwarding {
                source_name: "VOLICORD_HOME".to_owned(),
                target_name: "VOLICORD_HOME".to_owned(),
            }],
            configuration_target: ConfigurationTarget {
                owner: HostScope::Project,
                path: configuration_path.to_owned(),
            },
            process_binding: ProcessBinding {
                process_id: 42,
                process_start_token: "process-42".to_owned(),
                platform_instance_token: "boot-7".to_owned(),
                executable_path: executable_path.to_owned(),
                executable_digest: RAW_DIGEST.to_owned(),
            },
            required_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
            platform_environment,
            platform_release_coordinate: if platform_environment == PlatformEnvironment::Wsl2 {
                PlatformReleaseCoordinate::first_release_wsl2()
            } else {
                PlatformReleaseCoordinate::Native
            },
        }
    }
}
