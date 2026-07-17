//! Build-artifact and release-evidence continuity checks for publication.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::OpenOptions,
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};
use volicord_types::{
    is_canonical_sha256_hex, CodexCapability, CodexReleaseValidationResult, CodexSupportCatalog,
    IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple,
};

use crate::{
    contracts::{
        embedded_codex_support_catalog, load_codex_release_evidence_manifest,
        load_codex_support_catalog, load_release_target_contract, CODEX_SUPPORT_CATALOG_PATH,
        RELEASE_TARGETS_PATH,
    },
    error::{ValidationError, ValidationResult},
    gate::scenarios::verify_retained_scenario_evidence,
    io::{read_strict_json, sha256_external_file, ValidationContext},
};

const BUILD_METADATA_CONTRACT_ID: &str = "volicord.release-build-artifact";
const BUILD_METADATA_FILE: &str = "build-metadata.json";
const BUILD_DIGEST_FILE: &str = "volicord.sha256";
const RELEASE_EVIDENCE_FILE: &str = "release-evidence.json";
const SCENARIO_EVIDENCE_DIRECTORY: &str = "scenario-evidence";
const MAX_BUILD_METADATA_BYTES: u64 = 16 * 1024;
const MAX_RELEASE_EVIDENCE_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
const MAX_BINARY_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_DIGEST_FILE_BYTES: u64 = 1024;
const MAX_VERIFIED_INDEX_BYTES: usize = 1024 * 1024;

pub const VERIFIED_RELEASE_INDEX_CONTRACT_ID: &str = "volicord.verified-release-index";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseBuildMetadata {
    contract_id: String,
    target_triple: ReleaseTargetTriple,
    source_revision: String,
    binary_name: String,
    binary_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBuildArtifact {
    pub target_triple: ReleaseTargetTriple,
    pub binary_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCellEvidence {
    pub target_triple: ReleaseTargetTriple,
    pub platform_environment: PlatformEnvironment,
    pub binary_sha256: String,
    pub codex_artifact_digest: String,
    pub integration_profile: IntegrationProfile,
    pub observed_capabilities: Vec<CodexCapability>,
    pub evidence_sha256: String,
    pub evidence_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedPublishedArtifact {
    pub target_triple: ReleaseTargetTriple,
    pub binary_name: String,
    pub binary_sha256: String,
    pub build_artifact: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedReleaseEvidenceReference {
    pub target_triple: ReleaseTargetTriple,
    pub platform_environment: PlatformEnvironment,
    pub integration_profile: IntegrationProfile,
    pub codex_artifact_digest: String,
    pub observed_capabilities: Vec<CodexCapability>,
    pub volicord_artifact_digest: String,
    pub evidence_digest: String,
    pub evidence_manifest_sha256: String,
    pub evidence_artifact: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedReleaseIndex {
    contract_id: String,
    pub source_revision: String,
    pub support_catalog_identity_digest: String,
    pub published_artifacts: Vec<VerifiedPublishedArtifact>,
    pub release_evidence: Vec<VerifiedReleaseEvidenceReference>,
}

impl VerifiedReleaseIndex {
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }
}

pub fn build_artifact_name(target: ReleaseTargetTriple, run_id: &str, run_attempt: &str) -> String {
    format!("volicord-build-{target}-{run_id}-{run_attempt}")
}

pub fn evidence_artifact_name(
    target: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    run_id: &str,
    run_attempt: &str,
) -> String {
    format!(
        "volicord-release-evidence-{target}-{}-{run_id}-{run_attempt}",
        platform.as_str()
    )
}

pub fn verify_build_artifact(
    artifact_directory: &Path,
    expected_target: ReleaseTargetTriple,
    expected_source_revision: &str,
) -> ValidationResult<VerifiedBuildArtifact> {
    let context = workflow_validation_context()?;
    validate_source_revision(expected_source_revision)?;
    verify_build_artifact_with_context(
        &context,
        artifact_directory,
        expected_target,
        expected_source_revision,
    )
}

pub fn verify_cell_evidence(
    build_artifact_directory: &Path,
    evidence_artifact_directory: &Path,
    expected_source_revision: &str,
    expected_target: ReleaseTargetTriple,
    expected_platform: PlatformEnvironment,
) -> ValidationResult<VerifiedCellEvidence> {
    let context = workflow_validation_context()?;
    let (_, support_catalog) = load_pipeline_contracts(&context)?;
    let build = verify_build_artifact_with_context(
        &context,
        build_artifact_directory,
        expected_target,
        expected_source_revision,
    )?;
    verify_cell_evidence_with_context(
        &context,
        evidence_artifact_directory,
        expected_source_revision,
        expected_target,
        expected_platform,
        &build.binary_sha256,
        &support_catalog,
    )
}

pub fn verify_publish_inputs(
    build_root: &Path,
    evidence_root: &Path,
    expected_source_revision: &str,
    run_id: &str,
    run_attempt: &str,
) -> ValidationResult<VerifiedReleaseIndex> {
    let context = workflow_validation_context()?;
    let (targets, support_catalog) = load_pipeline_contracts(&context)?;
    verify_publish_inputs_with_contracts(
        &context,
        build_root,
        evidence_root,
        expected_source_revision,
        run_id,
        run_attempt,
        &targets,
        &support_catalog,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn verify_publish_inputs_with_contracts(
    context: &ValidationContext,
    build_root: &Path,
    evidence_root: &Path,
    expected_source_revision: &str,
    run_id: &str,
    run_attempt: &str,
    targets: &crate::contracts::ReleaseTargetContract,
    support_catalog: &CodexSupportCatalog,
    production_mode: bool,
) -> ValidationResult<VerifiedReleaseIndex> {
    validate_source_revision(expected_source_revision)?;
    validate_run_coordinate("run ID", run_id)?;
    validate_run_coordinate("run attempt", run_attempt)?;
    context.validate_existing_directory(build_root)?;
    context.validate_existing_directory(evidence_root)?;
    if production_mode && support_catalog.entries().is_empty() {
        return Err(ValidationError::new(
            "production release requires a nonempty Codex support catalog",
        ));
    }
    validate_support_catalog_bindings(targets, support_catalog)?;

    let expected_build_names = targets
        .published_targets()
        .iter()
        .map(|target| build_artifact_name(*target, run_id, run_attempt))
        .collect::<Vec<_>>();
    require_exact_directory_names(build_root, &expected_build_names)?;

    let mut published_artifacts = Vec::with_capacity(targets.published_targets().len());
    let mut digests = BTreeMap::new();
    for target in targets.published_targets() {
        let artifact_name = build_artifact_name(*target, run_id, run_attempt);
        let build = verify_build_artifact_with_context(
            context,
            &build_root.join(&artifact_name),
            *target,
            expected_source_revision,
        )?;
        digests.insert(*target, build.binary_sha256.clone());
        published_artifacts.push(VerifiedPublishedArtifact {
            target_triple: *target,
            binary_name: release_binary_name(*target).to_owned(),
            binary_sha256: build.binary_sha256,
            build_artifact: artifact_name,
        });
    }

    let expected_evidence_names = targets
        .required_cells()
        .iter()
        .map(|cell| {
            evidence_artifact_name(
                cell.target_triple,
                cell.platform_environment,
                run_id,
                run_attempt,
            )
        })
        .collect::<Vec<_>>();
    require_exact_directory_names(evidence_root, &expected_evidence_names)?;

    let mut release_evidence = Vec::with_capacity(targets.required_cells().len());
    let mut used_support_entries = BTreeSet::new();
    let mut used_evidence_entries = BTreeSet::new();
    for cell in targets.required_cells() {
        let digest = digests.get(&cell.target_triple).ok_or_else(|| {
            ValidationError::new(format!(
                "required release cell target {} has no verified build artifact",
                cell.target_triple
            ))
        })?;
        let artifact_name = evidence_artifact_name(
            cell.target_triple,
            cell.platform_environment,
            run_id,
            run_attempt,
        );
        let verified = verify_cell_evidence_with_context(
            context,
            &evidence_root.join(&artifact_name),
            expected_source_revision,
            cell.target_triple,
            cell.platform_environment,
            digest,
            support_catalog,
        )?;
        let identity = (
            verified.codex_artifact_digest.clone(),
            verified.target_triple,
            verified.platform_environment,
            verified.integration_profile,
        );
        if !used_support_entries.insert(identity.clone())
            || !used_evidence_entries.insert((
                identity,
                verified.evidence_sha256.clone(),
                verified.evidence_manifest_sha256.clone(),
            ))
        {
            return Err(ValidationError::new(
                "release evidence is duplicated or ambiguous",
            ));
        }
        release_evidence.push(VerifiedReleaseEvidenceReference {
            target_triple: verified.target_triple,
            platform_environment: verified.platform_environment,
            integration_profile: verified.integration_profile,
            codex_artifact_digest: verified.codex_artifact_digest,
            observed_capabilities: verified.observed_capabilities,
            volicord_artifact_digest: verified.binary_sha256,
            evidence_digest: verified.evidence_sha256,
            evidence_manifest_sha256: verified.evidence_manifest_sha256,
            evidence_artifact: artifact_name,
        });
    }

    let catalog_identities = support_catalog
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.codex_artifact_digest.clone(),
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
        })
        .collect::<BTreeSet<_>>();
    if catalog_identities != used_support_entries {
        return Err(ValidationError::new(
            "every production support-catalog entry must map to and be used by exactly one required release cell",
        ));
    }

    let index = VerifiedReleaseIndex {
        contract_id: VERIFIED_RELEASE_INDEX_CONTRACT_ID.to_owned(),
        source_revision: expected_source_revision.to_owned(),
        support_catalog_identity_digest: support_catalog.identity_digest().map_err(|error| {
            ValidationError::new(format!("cannot digest support catalog: {error}"))
        })?,
        published_artifacts,
        release_evidence,
    };
    validate_verified_release_index(&index)?;
    Ok(index)
}

fn verify_build_artifact_with_context(
    context: &ValidationContext,
    artifact_directory: &Path,
    expected_target: ReleaseTargetTriple,
    expected_source_revision: &str,
) -> ValidationResult<VerifiedBuildArtifact> {
    context.validate_existing_directory(artifact_directory)?;
    let binary_name = release_binary_name(expected_target);
    require_exact_directory_names(
        artifact_directory,
        &[
            BUILD_METADATA_FILE.to_owned(),
            BUILD_DIGEST_FILE.to_owned(),
            binary_name.to_owned(),
        ],
    )?;
    let metadata: ReleaseBuildMetadata = read_strict_json(
        context,
        &artifact_directory.join(BUILD_METADATA_FILE),
        MAX_BUILD_METADATA_BYTES,
    )?;
    if metadata.contract_id != BUILD_METADATA_CONTRACT_ID
        || metadata.target_triple != expected_target
        || metadata.source_revision != expected_source_revision
        || metadata.binary_name != binary_name
        || !is_canonical_sha256_hex(&metadata.binary_sha256)
    {
        return Err(ValidationError::new(format!(
            "build metadata does not match the exact {expected_target} release artifact"
        )));
    }
    let digest_path = artifact_directory.join(BUILD_DIGEST_FILE);
    context.validate_existing_file(&digest_path)?;
    if fs::metadata(&digest_path)?.len() > MAX_DIGEST_FILE_BYTES {
        return Err(ValidationError::new(
            "build digest metadata exceeds its byte bound",
        ));
    }
    let digest_record = fs::read_to_string(&digest_path)?;
    let expected_digest_record = format!("{}  {binary_name}\n", metadata.binary_sha256);
    if digest_record != expected_digest_record {
        return Err(ValidationError::new(format!(
            "digest metadata does not match build metadata for {expected_target}"
        )));
    }
    let actual_digest = sha256_external_file(
        context,
        &artifact_directory.join(binary_name),
        Some(MAX_BINARY_BYTES),
    )?;
    if actual_digest != metadata.binary_sha256 {
        return Err(ValidationError::new(format!(
            "raw {expected_target} binary digest differs from immutable build metadata"
        )));
    }
    Ok(VerifiedBuildArtifact {
        target_triple: expected_target,
        binary_sha256: actual_digest,
    })
}

fn verify_cell_evidence_with_context(
    context: &ValidationContext,
    artifact_directory: &Path,
    expected_source_revision: &str,
    expected_target: ReleaseTargetTriple,
    expected_platform: PlatformEnvironment,
    expected_binary_digest: &str,
    support_catalog: &CodexSupportCatalog,
) -> ValidationResult<VerifiedCellEvidence> {
    context.validate_existing_directory(artifact_directory)?;
    require_exact_directory_names(
        artifact_directory,
        &[
            RELEASE_EVIDENCE_FILE.to_owned(),
            SCENARIO_EVIDENCE_DIRECTORY.to_owned(),
        ],
    )?;
    let manifest_path = artifact_directory.join(RELEASE_EVIDENCE_FILE);
    context.validate_existing_file(&manifest_path)?;
    let evidence_manifest_sha256 = sha256_external_file(
        context,
        &manifest_path,
        Some(MAX_RELEASE_EVIDENCE_MANIFEST_BYTES),
    )?;
    let manifest = load_codex_release_evidence_manifest(&manifest_path).map_err(|error| {
        ValidationError::new(format!(
            "release-cell evidence manifest is invalid at {}: {error}",
            manifest_path.display()
        ))
    })?;
    manifest
        .validate_against_support_catalog(support_catalog)
        .map_err(|error| ValidationError::new(format!("unsupported release evidence: {error}")))?;
    let [entry] = manifest.entries() else {
        return Err(ValidationError::new(
            "each release-cell evidence artifact must contain exactly one entry",
        ));
    };
    if entry.target_triple != expected_target
        || entry.platform_environment != expected_platform
        || entry.integration_profile != IntegrationProfile::Record
        || entry.validation_evidence.validation_result != CodexReleaseValidationResult::Passed
        || entry.validation_evidence.volicord_artifact_digest != expected_binary_digest
        || entry.validation_evidence.source_revision != expected_source_revision
    {
        return Err(ValidationError::new(format!(
            "release evidence does not pass the exact {expected_target}/{} build digest",
            expected_platform.as_str()
        )));
    }
    verify_retained_scenario_evidence(
        context,
        &artifact_directory.join(SCENARIO_EVIDENCE_DIRECTORY),
        entry,
    )?;
    Ok(VerifiedCellEvidence {
        target_triple: expected_target,
        platform_environment: expected_platform,
        binary_sha256: expected_binary_digest.to_owned(),
        codex_artifact_digest: entry.codex_artifact_digest.clone(),
        integration_profile: entry.integration_profile,
        observed_capabilities: entry.observed_capabilities.clone(),
        evidence_sha256: entry.validation_evidence.evidence_digest.clone(),
        evidence_manifest_sha256,
    })
}

fn load_pipeline_contracts(
    context: &ValidationContext,
) -> ValidationResult<(crate::contracts::ReleaseTargetContract, CodexSupportCatalog)> {
    let targets =
        load_release_target_contract(&context.source_checkout().join(RELEASE_TARGETS_PATH))
            .map_err(|error| {
                ValidationError::new(format!("invalid release target contract: {error}"))
            })?;
    let embedded = embedded_codex_support_catalog().map_err(|error| {
        ValidationError::new(format!("invalid embedded support catalog: {error}"))
    })?;
    let disk =
        load_codex_support_catalog(&context.source_checkout().join(CODEX_SUPPORT_CATALOG_PATH))
            .map_err(|error| {
                ValidationError::new(format!("invalid on-disk support catalog: {error}"))
            })?;
    if embedded != disk {
        return Err(ValidationError::new(
            "embedded and on-disk Codex support catalogs differ",
        ));
    }
    Ok((targets, embedded))
}

fn validate_support_catalog_bindings(
    targets: &crate::contracts::ReleaseTargetContract,
    support_catalog: &CodexSupportCatalog,
) -> ValidationResult<()> {
    let mut mapped_cells = BTreeSet::new();
    for entry in support_catalog.entries() {
        targets
            .require_cell(
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
            .map_err(|_| {
                ValidationError::new(format!(
                    "support-catalog entry {}/{} cannot map to the release matrix",
                    entry.target_triple,
                    entry.platform_environment.as_str()
                ))
            })?;
        if !mapped_cells.insert((
            entry.target_triple,
            entry.platform_environment,
            entry.integration_profile,
        )) {
            return Err(ValidationError::new(format!(
                "multiple support-catalog entries ambiguously map to {}/{}",
                entry.target_triple,
                entry.platform_environment.as_str()
            )));
        }
    }
    Ok(())
}

pub fn serialize_verified_release_index(index: &VerifiedReleaseIndex) -> ValidationResult<Vec<u8>> {
    validate_verified_release_index(index)?;
    let bytes = serde_json::to_vec(index)?;
    if bytes.len() > MAX_VERIFIED_INDEX_BYTES {
        return Err(ValidationError::new(
            "verified release index exceeds its byte bound",
        ));
    }
    Ok(bytes)
}

pub fn write_verified_release_index(
    output_path: &Path,
    index: &VerifiedReleaseIndex,
) -> ValidationResult<()> {
    let context = workflow_validation_context()?;
    write_verified_release_index_with_context(&context, output_path, index)
}

pub(crate) fn write_verified_release_index_with_context(
    context: &ValidationContext,
    output_path: &Path,
    index: &VerifiedReleaseIndex,
) -> ValidationResult<()> {
    context.validate_new_output(output_path)?;
    let mut bytes = serialize_verified_release_index(index)?;
    bytes.push(b'\n');
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|error| {
            ValidationError::new(format!(
                "cannot create verified release index {}: {error}",
                output_path.display()
            ))
        })?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    Ok(())
}

fn validate_verified_release_index(index: &VerifiedReleaseIndex) -> ValidationResult<()> {
    if index.contract_id != VERIFIED_RELEASE_INDEX_CONTRACT_ID {
        return Err(ValidationError::new(
            "verified release index contract_id is unknown",
        ));
    }
    validate_source_revision(&index.source_revision)?;
    if !is_canonical_sha256_hex(&index.support_catalog_identity_digest) {
        return Err(ValidationError::new(
            "verified release index support catalog digest is malformed",
        ));
    }
    if index.published_artifacts.is_empty() || index.release_evidence.is_empty() {
        return Err(ValidationError::new(
            "verified release index requires published artifacts and release evidence",
        ));
    }
    let mut builds = BTreeMap::new();
    for artifact in &index.published_artifacts {
        if artifact.binary_name != release_binary_name(artifact.target_triple)
            || !is_canonical_sha256_hex(&artifact.binary_sha256)
            || artifact.build_artifact.is_empty()
            || artifact
                .build_artifact
                .chars()
                .any(|character| character.is_control())
            || builds
                .insert(artifact.target_triple, artifact.binary_sha256.as_str())
                .is_some()
        {
            return Err(ValidationError::new(
                "verified release index contains an invalid or duplicate published artifact",
            ));
        }
    }
    let mut cells = BTreeSet::new();
    for evidence in &index.release_evidence {
        let build_digest = builds.get(&evidence.target_triple);
        if !evidence
            .target_triple
            .supports_environment(evidence.platform_environment)
            || evidence.integration_profile != IntegrationProfile::Record
            || !volicord_types::has_exact_first_release_codex_capabilities(
                &evidence.observed_capabilities,
            )
            || !is_canonical_sha256_hex(&evidence.codex_artifact_digest)
            || !is_canonical_sha256_hex(&evidence.volicord_artifact_digest)
            || !is_canonical_sha256_hex(&evidence.evidence_digest)
            || !is_canonical_sha256_hex(&evidence.evidence_manifest_sha256)
            || build_digest.copied() != Some(evidence.volicord_artifact_digest.as_str())
            || evidence.evidence_artifact.is_empty()
            || evidence
                .evidence_artifact
                .chars()
                .any(|character| character.is_control())
            || !cells.insert((
                evidence.target_triple,
                evidence.platform_environment,
                evidence.integration_profile,
            ))
        {
            return Err(ValidationError::new(
                "verified release index contains invalid, duplicate, or mismatched release evidence",
            ));
        }
    }
    Ok(())
}

fn workflow_validation_context() -> ValidationResult<ValidationContext> {
    let current_directory = env::current_dir()?;
    if env::var_os("VOLICORD_CODEX_RELEASE_WSL2_DISTRIBUTION").is_some() {
        ValidationContext::from_process_environment(
            &current_directory,
            None,
            env::var_os("HOME"),
            env::var_os("USERPROFILE"),
        )
    } else {
        ValidationContext::from_process(&current_directory)
    }
}

fn release_binary_name(target: ReleaseTargetTriple) -> &'static str {
    if target == ReleaseTargetTriple::X86_64PcWindowsMsvc {
        "volicord.exe"
    } else {
        "volicord"
    }
}

fn require_exact_directory_names(directory: &Path, expected: &[String]) -> ValidationResult<()> {
    let mut actual = fs::read_dir(directory)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("directory entry is not UTF-8"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    actual.sort();
    let mut expected = expected.to_vec();
    expected.sort();
    if actual != expected {
        return Err(ValidationError::new(format!(
            "release artifact directory {} does not contain the exact expected entries",
            directory.display()
        )));
    }
    Ok(())
}

fn validate_source_revision(value: &str) -> ValidationResult<()> {
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ValidationError::new(
            "source revision must be a raw lowercase 40- or 64-hex Git object ID",
        ))
    }
}

fn validate_run_coordinate(label: &str, value: &str) -> ValidationResult<()> {
    if !value.is_empty() && value.len() <= 32 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(ValidationError::new(format!(
            "release workflow {label} must contain only ASCII digits"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::sha256_bytes;

    #[test]
    fn immutable_build_metadata_binds_target_revision_name_and_exact_bytes() {
        let temporary = tempfile::tempdir().expect("temporary build artifact root");
        let artifact = temporary.path().join("build");
        fs::create_dir(&artifact).expect("build artifact directory");
        let binary = b"exact release candidate bytes\n";
        let digest = sha256_bytes(binary);
        fs::write(artifact.join("volicord"), binary).expect("raw binary");
        fs::write(
            artifact.join(BUILD_DIGEST_FILE),
            format!("{digest}  volicord\n"),
        )
        .expect("digest metadata");
        fs::write(
            artifact.join(BUILD_METADATA_FILE),
            format!(
                "{{\"contract_id\":\"{BUILD_METADATA_CONTRACT_ID}\",\"target_triple\":\"x86_64-unknown-linux-gnu\",\"source_revision\":\"{}\",\"binary_name\":\"volicord\",\"binary_sha256\":\"{digest}\"}}\n",
                "a".repeat(40)
            ),
        )
        .expect("build metadata");

        let verified = verify_build_artifact(
            &artifact,
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &"a".repeat(40),
        )
        .expect("exact build artifact");
        assert_eq!(verified.binary_sha256, digest);

        fs::write(artifact.join("volicord"), b"replaced bytes\n").expect("tampered binary");
        assert!(verify_build_artifact(
            &artifact,
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &"a".repeat(40),
        )
        .is_err());
    }

    #[test]
    fn artifact_names_include_target_cell_and_workflow_attempt() {
        assert_eq!(
            build_artifact_name(ReleaseTargetTriple::X86_64UnknownLinuxGnu, "123", "2"),
            "volicord-build-x86_64-unknown-linux-gnu-123-2"
        );
        assert_eq!(
            evidence_artifact_name(
                ReleaseTargetTriple::X86_64UnknownLinuxGnu,
                PlatformEnvironment::Wsl2,
                "123",
                "2"
            ),
            "volicord-release-evidence-x86_64-unknown-linux-gnu-wsl2-123-2"
        );
    }
}
