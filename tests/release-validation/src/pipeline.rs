//! Build-artifact and release-evidence continuity checks for publication.

use std::{collections::BTreeMap, env, fs, path::Path};

use serde::Deserialize;
use volicord_types::{
    is_canonical_sha256_hex, CodexReleaseValidationResult, CodexSupportCatalog, IntegrationProfile,
    PlatformEnvironment, ReleaseTargetTriple,
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
const MAX_BINARY_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_DIGEST_FILE_BYTES: u64 = 1024;

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
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPublishInputs {
    pub builds: Vec<VerifiedBuildArtifact>,
    pub cells: Vec<VerifiedCellEvidence>,
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
) -> ValidationResult<VerifiedPublishInputs> {
    validate_source_revision(expected_source_revision)?;
    validate_run_coordinate("run ID", run_id)?;
    validate_run_coordinate("run attempt", run_attempt)?;
    let context = workflow_validation_context()?;
    context.validate_existing_directory(build_root)?;
    context.validate_existing_directory(evidence_root)?;
    let (targets, support_catalog) = load_pipeline_contracts(&context)?;

    let expected_build_names = targets
        .published_targets()
        .iter()
        .map(|target| build_artifact_name(*target, run_id, run_attempt))
        .collect::<Vec<_>>();
    require_exact_directory_names(build_root, &expected_build_names)?;

    let mut builds = Vec::with_capacity(targets.published_targets().len());
    let mut digests = BTreeMap::new();
    for target in targets.published_targets() {
        let build = verify_build_artifact_with_context(
            &context,
            &build_root.join(build_artifact_name(*target, run_id, run_attempt)),
            *target,
            expected_source_revision,
        )?;
        digests.insert(*target, build.binary_sha256.clone());
        builds.push(build);
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

    let mut cells = Vec::with_capacity(targets.required_cells().len());
    for cell in targets.required_cells() {
        let digest = digests.get(&cell.target_triple).ok_or_else(|| {
            ValidationError::new(format!(
                "required release cell target {} has no verified build artifact",
                cell.target_triple
            ))
        })?;
        cells.push(verify_cell_evidence_with_context(
            &context,
            &evidence_root.join(evidence_artifact_name(
                cell.target_triple,
                cell.platform_environment,
                run_id,
                run_attempt,
            )),
            cell.target_triple,
            cell.platform_environment,
            digest,
            &support_catalog,
        )?);
    }

    Ok(VerifiedPublishInputs { builds, cells })
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
        evidence_sha256: entry.validation_evidence.evidence_digest.clone(),
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
