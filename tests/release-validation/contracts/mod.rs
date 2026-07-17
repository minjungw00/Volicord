//! Release-test routes to the target matrix, runtime policy, and external evidence contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};
use volicord_types::{IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple};

pub use volicord_types::{
    compute_codex_release_evidence_digest, embedded_codex_support_catalog,
    load_codex_release_evidence_manifest, load_codex_support_catalog,
    parse_codex_release_evidence_manifest, parse_codex_support_catalog,
    parse_test_only_codex_descriptor, serialize_codex_release_evidence_manifest,
    serialize_codex_support_catalog, CodexReleaseCellStatus, CodexReleaseEvidenceError,
    CodexReleaseEvidenceManifest, CodexSupportCatalog, CodexSupportCatalogError,
    UnsupportedHostArtifact, CODEX_SUPPORT_CATALOG_PATH, UNSUPPORTED_HOST_ARTIFACT_REASON,
};

/// Repository-relative path of the external checked-in release-evidence manifest.
pub const CODEX_RELEASE_EVIDENCE_MANIFEST_PATH: &str =
    "tests/release-validation/contracts/codex-release-evidence-manifest.json";

/// Repository-relative path of the canonical published-target and environment-cell contract.
pub const RELEASE_TARGETS_PATH: &str = "tests/release-validation/contracts/release-targets.json";

/// Exact contract identifier for the release target matrix.
pub const RELEASE_TARGETS_CONTRACT_ID: &str = "volicord.release-targets";

const MAX_RELEASE_TARGETS_BYTES: usize = 1024 * 1024;

/// Canonical checked-in contract values after strict parsing and cross-validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedInContracts {
    pub release_targets: ReleaseTargetContract,
    pub support_catalog: CodexSupportCatalog,
    pub evidence_manifest: CodexReleaseEvidenceManifest,
}

/// One required release-validation cell, excluding the captured artifact digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseCell {
    pub target_triple: ReleaseTargetTriple,
    pub platform_environment: PlatformEnvironment,
    pub integration_profile: IntegrationProfile,
}

/// Canonical published targets and their independently required environment cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTargetContract {
    contract_id: String,
    published_targets: Vec<ReleaseTargetTriple>,
    required_cells: Vec<ReleaseCell>,
}

impl ReleaseTargetContract {
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    pub fn published_targets(&self) -> &[ReleaseTargetTriple] {
        &self.published_targets
    }

    pub fn required_cells(&self) -> &[ReleaseCell] {
        &self.required_cells
    }

    pub fn require_cell(
        &self,
        target_triple: ReleaseTargetTriple,
        platform_environment: PlatformEnvironment,
        integration_profile: IntegrationProfile,
    ) -> Result<ReleaseCell, String> {
        self.required_cells
            .iter()
            .copied()
            .find(|cell| {
                cell.target_triple == target_triple
                    && cell.platform_environment == platform_environment
                    && cell.integration_profile == integration_profile
            })
            .ok_or_else(|| {
                format!(
                    "unknown release cell {target_triple}/{}/{}",
                    platform_environment.as_str(),
                    integration_profile.as_str()
                )
            })
    }
}

/// Loads and validates the canonical release target matrix.
pub fn load_release_target_contract(path: &Path) -> Result<ReleaseTargetContract, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("release target contract must be a regular file".to_owned());
    }
    if metadata.len() > MAX_RELEASE_TARGETS_BYTES as u64 {
        return Err("release target contract exceeds its byte bound".to_owned());
    }
    parse_release_target_contract(&fs::read(path).map_err(|error| error.to_string())?)
}

/// Parses and validates one release target matrix.
pub fn parse_release_target_contract(bytes: &[u8]) -> Result<ReleaseTargetContract, String> {
    if bytes.len() > MAX_RELEASE_TARGETS_BYTES {
        return Err("release target contract exceeds its byte bound".to_owned());
    }
    let contract: ReleaseTargetContract =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    validate_release_target_contract(&contract)?;
    Ok(contract)
}

/// Loads and statically validates the three canonical checked-in release contracts.
///
/// Static validation accepts every valid population state. Release completeness is
/// enforced separately by the production release-bundle verifier.
pub fn load_checked_in_contracts(repository_root: &Path) -> Result<CheckedInContracts, String> {
    let release_targets_path = repository_root.join(RELEASE_TARGETS_PATH);
    let support_catalog_path = repository_root.join(CODEX_SUPPORT_CATALOG_PATH);
    let evidence_manifest_path = repository_root.join(CODEX_RELEASE_EVIDENCE_MANIFEST_PATH);

    let release_targets = load_release_target_contract(&release_targets_path).map_err(|error| {
        format!(
            "release target contract is invalid at {}: {error}",
            release_targets_path.display()
        )
    })?;
    let embedded_support_catalog = embedded_codex_support_catalog()
        .map_err(|error| format!("embedded Codex support catalog is invalid: {error}"))?;
    let support_bytes = fs::read(&support_catalog_path).map_err(|error| {
        format!(
            "cannot read on-disk Codex support catalog at {}: {error}",
            support_catalog_path.display()
        )
    })?;
    let disk_support_catalog =
        load_codex_support_catalog(&support_catalog_path).map_err(|error| {
            format!(
                "on-disk Codex support catalog is invalid at {}: {error}",
                support_catalog_path.display()
            )
        })?;
    let canonical_support = serialize_codex_support_catalog(&disk_support_catalog)
        .map_err(|error| format!("cannot serialize Codex support catalog: {error}"))?;
    require_canonical_checked_in_bytes(
        "Codex support catalog",
        &support_bytes,
        &canonical_support,
    )?;

    let evidence_bytes = fs::read(&evidence_manifest_path).map_err(|error| {
        format!(
            "cannot read external Codex release-evidence manifest at {}: {error}",
            evidence_manifest_path.display()
        )
    })?;
    let evidence_manifest =
        load_codex_release_evidence_manifest(&evidence_manifest_path).map_err(|error| {
            format!(
                "external Codex release-evidence manifest is invalid at {}: {error}",
                evidence_manifest_path.display()
            )
        })?;
    let canonical_evidence = serialize_codex_release_evidence_manifest(&evidence_manifest)
        .map_err(|error| format!("cannot serialize Codex release evidence: {error}"))?;
    require_canonical_checked_in_bytes(
        "Codex release-evidence manifest",
        &evidence_bytes,
        &canonical_evidence,
    )?;

    validate_static_contract_values(
        &release_targets,
        &embedded_support_catalog,
        &disk_support_catalog,
        &evidence_manifest,
    )?;

    Ok(CheckedInContracts {
        release_targets,
        support_catalog: disk_support_catalog,
        evidence_manifest,
    })
}

/// Validates static contract relationships without treating release completeness as success.
pub fn validate_static_contract_values(
    targets: &ReleaseTargetContract,
    embedded_support_catalog: &CodexSupportCatalog,
    disk_support_catalog: &CodexSupportCatalog,
    evidence_manifest: &CodexReleaseEvidenceManifest,
) -> Result<(), String> {
    if embedded_support_catalog != disk_support_catalog {
        return Err("embedded and on-disk Codex support catalogs differ".to_owned());
    }
    evidence_manifest
        .validate_against_support_catalog(embedded_support_catalog)
        .map_err(|error| {
            format!(
                "external Codex release evidence is not supported by the embedded catalog: {error}"
            )
        })?;

    for entry in embedded_support_catalog.entries() {
        targets
            .require_cell(
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
            .map_err(|_| {
                format!(
                    "support-catalog entry {}/{} cannot map to an actual required release target cell",
                    entry.target_triple,
                    entry.platform_environment.as_str()
                )
            })?;
    }

    let mut evidence_cells = BTreeSet::new();
    let mut volicord_digests = BTreeMap::new();
    let mut source_revision = None;
    for entry in evidence_manifest.entries() {
        targets
            .require_cell(
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
            .map_err(|_| {
                format!(
                    "release-evidence entry {}/{} is not a required release target cell",
                    entry.target_triple,
                    entry.platform_environment.as_str()
                )
            })?;
        if !evidence_cells.insert((
            entry.target_triple,
            entry.platform_environment,
            entry.integration_profile,
        )) {
            return Err(format!(
                "release-evidence cell {}/{} is duplicated or ambiguous",
                entry.target_triple,
                entry.platform_environment.as_str()
            ));
        }

        let volicord_digest = &entry.validation_evidence.volicord_artifact_digest;
        if volicord_digests
            .insert(entry.target_triple, volicord_digest)
            .is_some_and(|previous| previous != volicord_digest)
        {
            return Err(format!(
                "release-evidence entries for target {} reference different Volicord artifacts",
                entry.target_triple
            ));
        }
        let entry_revision = entry.validation_evidence.source_revision.as_str();
        if source_revision
            .replace(entry_revision)
            .is_some_and(|previous| previous != entry_revision)
        {
            return Err(
                "release-evidence entries reference different Volicord source revisions".to_owned(),
            );
        }
    }
    Ok(())
}

fn require_canonical_checked_in_bytes(
    label: &str,
    checked_in: &[u8],
    canonical: &[u8],
) -> Result<(), String> {
    let mut expected = canonical.to_vec();
    expected.push(b'\n');
    if checked_in == expected {
        Ok(())
    } else {
        Err(format!(
            "checked-in {label} bytes must equal canonical serialization followed by one final LF"
        ))
    }
}

fn validate_release_target_contract(contract: &ReleaseTargetContract) -> Result<(), String> {
    if contract.contract_id != RELEASE_TARGETS_CONTRACT_ID {
        return Err("release target contract_id is unknown".to_owned());
    }
    if contract.published_targets.is_empty() || contract.required_cells.is_empty() {
        return Err("published targets and required cells must both be nonempty".to_owned());
    }
    let published = contract
        .published_targets
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if published.len() != contract.published_targets.len() {
        return Err("published targets must not contain duplicates".to_owned());
    }
    let cells = contract
        .required_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if cells.len() != contract.required_cells.len() {
        return Err("required release cells must not contain duplicates".to_owned());
    }
    for cell in &contract.required_cells {
        if !published.contains(&cell.target_triple) {
            return Err(format!(
                "required cell target {} is not published",
                cell.target_triple
            ));
        }
        if !cell
            .target_triple
            .supports_environment(cell.platform_environment)
        {
            return Err(format!(
                "required cell target {} does not match platform environment {}",
                cell.target_triple,
                cell.platform_environment.as_str()
            ));
        }
        if cell.integration_profile != IntegrationProfile::Record {
            return Err("required release cell profile must be record".to_owned());
        }
    }
    for target in &contract.published_targets {
        if !contract
            .required_cells
            .iter()
            .any(|cell| cell.target_triple == *target)
        {
            return Err(format!(
                "published target {target} has no corresponding required cell"
            ));
        }
    }
    Ok(())
}
