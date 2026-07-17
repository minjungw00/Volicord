//! Release-test routes to the target matrix, runtime policy, and external evidence contracts.

use std::{collections::BTreeSet, fs, path::Path};

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
