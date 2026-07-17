//! External exact-artifact Codex release-validation evidence.

use std::{error::Error, fmt, fs, path::Path};

use chrono::{DateTime, SecondsFormat};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    codex_contract::{
        list, nullable, parse_ordered_json, record, require_exact_fields, string, OrderedJsonValue,
    },
    has_exact_first_release_codex_capabilities, is_canonical_sha256_hex, CodexCapability,
    CodexSupportCatalog, IntegrationProfile, PlatformEnvironment, PlatformReleaseCoordinate,
    ReleaseTargetTriple, RequiredNullable, PINNED_WSL2_ENVIRONMENT_IMAGE,
};

/// Contract identifier for the external release-evidence manifest.
pub const CODEX_RELEASE_EVIDENCE_MANIFEST_CONTRACT_ID: &str =
    "volicord.codex-release-evidence-manifest";

const MAX_RELEASE_EVIDENCE_BYTES: usize = 4 * 1024 * 1024;
const EVIDENCE_DIGEST_DOMAIN: &[u8] = b"volicord.codex-release-validation-evidence\0";
const MANIFEST_FIELDS: &[&str] = &["contract_id", "entries"];
const ENTRY_FIELDS: &[&str] = &[
    "codex_artifact_digest",
    "target_triple",
    "platform_environment",
    "observed_capabilities",
    "integration_profile",
    "validation_evidence",
];
const EVIDENCE_FIELDS: &[&str] = &[
    "validation_result",
    "codex_artifact_digest",
    "target_triple",
    "platform_environment",
    "observed_capabilities",
    "integration_profile",
    "volicord_artifact_digest",
    "source_revision",
    "runner",
    "scenario_results",
    "evidence_digest",
    "observed_at",
];
const RUNNER_FIELDS: &[&str] = &[
    "runner_id",
    "target_triple",
    "architecture",
    "os_release",
    "environment_image",
];
const SCENARIO_RESULT_FIELDS: &[&str] = &[
    "scenario_id",
    "status",
    "reason",
    "evidence_digest",
    "observed_at",
];

/// Top-level result of one qualifying platform-cell attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodexReleaseValidationResult {
    /// Every required scenario passed.
    Passed,
    /// At least one required scenario failed.
    Failed,
    /// No scenario failed, but an unavailable prerequisite prevented completion.
    Unavailable,
}

impl CodexReleaseValidationResult {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Result of one required release-validation scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodexReleaseScenarioStatus {
    /// The scenario completed and passed.
    Passed,
    /// The scenario completed far enough to fail an assertion.
    Failed,
    /// A prerequisite was unavailable after a qualifying attempt began.
    Unavailable,
    /// The scenario was not executed after an earlier prerequisite failure.
    NotRun,
}

impl CodexReleaseScenarioStatus {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
            Self::NotRun => "not_run",
        }
    }
}

/// Architecture recorded for the exact release runner target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodexReleaseRunnerArchitecture {
    /// x86-64 runner.
    X86_64,
    /// AArch64 runner.
    Aarch64,
}

impl CodexReleaseRunnerArchitecture {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }
}

/// Closed identifier set for required Codex release scenarios.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CodexReleaseScenarioId {
    FreshInstall,
    RuntimeHomeCreation,
    PersonalManagedBinding,
    SharedManagedBinding,
    ReceiptCreateAndValidate,
    ConfigurationDriftDetection,
    RepairAfterDrift,
    SafeUninstall,
    SymlinkAndCanonicalPath,
    CodexRestart,
    ProjectMove,
    RecordWriteWorkflow,
    SuppressionUnavailable,
    UnsupportedHost,
    UnsupportedHostArtifact,
    WslShutdownRestart,
    Wsl2Ext4Project,
    Wsl2DrvfsRejection,
    Wsl2CrossTopologyRejection,
    Wsl1Rejection,
    Wsl2NativeWindowsReceiptReuseRejection,
}

impl CodexReleaseScenarioId {
    /// Required scenarios for every release platform.
    pub const BASE: [Self; 15] = [
        Self::FreshInstall,
        Self::RuntimeHomeCreation,
        Self::PersonalManagedBinding,
        Self::SharedManagedBinding,
        Self::ReceiptCreateAndValidate,
        Self::ConfigurationDriftDetection,
        Self::RepairAfterDrift,
        Self::SafeUninstall,
        Self::SymlinkAndCanonicalPath,
        Self::CodexRestart,
        Self::ProjectMove,
        Self::RecordWriteWorkflow,
        Self::SuppressionUnavailable,
        Self::UnsupportedHost,
        Self::UnsupportedHostArtifact,
    ];

    /// Additional scenarios required only for the independent WSL2 cell.
    pub const WSL2_ADDITIONAL: [Self; 6] = [
        Self::WslShutdownRestart,
        Self::Wsl2Ext4Project,
        Self::Wsl2DrvfsRejection,
        Self::Wsl2CrossTopologyRejection,
        Self::Wsl1Rejection,
        Self::Wsl2NativeWindowsReceiptReuseRejection,
    ];

    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FreshInstall => "fresh_install",
            Self::RuntimeHomeCreation => "runtime_home_creation",
            Self::PersonalManagedBinding => "personal_managed_binding",
            Self::SharedManagedBinding => "shared_managed_binding",
            Self::ReceiptCreateAndValidate => "receipt_create_and_validate",
            Self::ConfigurationDriftDetection => "configuration_drift_detection",
            Self::RepairAfterDrift => "repair_after_drift",
            Self::SafeUninstall => "safe_uninstall",
            Self::SymlinkAndCanonicalPath => "symlink_and_canonical_path",
            Self::CodexRestart => "codex_restart",
            Self::ProjectMove => "project_move",
            Self::RecordWriteWorkflow => "record_write_workflow",
            Self::SuppressionUnavailable => "suppression_unavailable",
            Self::UnsupportedHost => "unsupported_host",
            Self::UnsupportedHostArtifact => "unsupported_host_artifact",
            Self::WslShutdownRestart => "wsl_shutdown_restart",
            Self::Wsl2Ext4Project => "wsl2_ext4_project",
            Self::Wsl2DrvfsRejection => "wsl2_drvfs_rejection",
            Self::Wsl2CrossTopologyRejection => "wsl2_cross_topology_rejection",
            Self::Wsl1Rejection => "wsl1_rejection",
            Self::Wsl2NativeWindowsReceiptReuseRejection => {
                "wsl2_native_windows_receipt_reuse_rejection"
            }
        }
    }
}

/// Exact environment and target coordinates for one release-validation runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseEvidenceRunner {
    pub runner_id: String,
    pub target_triple: ReleaseTargetTriple,
    pub architecture: CodexReleaseRunnerArchitecture,
    pub os_release: String,
    pub environment_image: String,
}

/// Evidence outcome for one required scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseScenarioResult {
    pub scenario_id: CodexReleaseScenarioId,
    pub status: CodexReleaseScenarioStatus,
    pub reason: RequiredNullable<String>,
    pub evidence_digest: RequiredNullable<String>,
    pub observed_at: RequiredNullable<String>,
}

/// Complete validation evidence for one qualifying exact-artifact platform attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseValidationEvidence {
    pub validation_result: CodexReleaseValidationResult,
    pub codex_artifact_digest: String,
    pub target_triple: ReleaseTargetTriple,
    pub platform_environment: PlatformEnvironment,
    pub observed_capabilities: Vec<CodexCapability>,
    pub integration_profile: IntegrationProfile,
    pub volicord_artifact_digest: String,
    pub source_revision: String,
    pub runner: CodexReleaseEvidenceRunner,
    pub scenario_results: Vec<CodexReleaseScenarioResult>,
    pub evidence_digest: String,
    pub observed_at: String,
}

/// One external exact-artifact release-evidence entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseEvidenceEntry {
    pub codex_artifact_digest: String,
    pub target_triple: ReleaseTargetTriple,
    pub platform_environment: PlatformEnvironment,
    pub observed_capabilities: Vec<CodexCapability>,
    pub integration_profile: IntegrationProfile,
    pub validation_evidence: CodexReleaseValidationEvidence,
}

/// Strict external Codex release-evidence manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseEvidenceManifest {
    contract_id: String,
    entries: Vec<CodexReleaseEvidenceEntry>,
}

impl CodexReleaseEvidenceManifest {
    /// Creates and validates an external evidence manifest.
    pub fn from_entries(
        entries: Vec<CodexReleaseEvidenceEntry>,
    ) -> Result<Self, CodexReleaseEvidenceError> {
        let manifest = Self {
            contract_id: CODEX_RELEASE_EVIDENCE_MANIFEST_CONTRACT_ID.to_owned(),
            entries,
        };
        validate_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Returns the exact contract identifier.
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns the zero-to-six reviewed evidence entries.
    pub fn entries(&self) -> &[CodexReleaseEvidenceEntry] {
        &self.entries
    }

    /// Returns the actual or derived result for one exact target/environment/profile cell.
    pub fn cell_status(
        &self,
        target_triple: ReleaseTargetTriple,
        platform_environment: PlatformEnvironment,
        integration_profile: IntegrationProfile,
    ) -> CodexReleaseCellStatus {
        self.entries
            .iter()
            .find(|entry| {
                entry.target_triple == target_triple
                    && entry.platform_environment == platform_environment
                    && entry.integration_profile == integration_profile
            })
            .map(|entry| match entry.validation_evidence.validation_result {
                CodexReleaseValidationResult::Passed => CodexReleaseCellStatus::Passed,
                CodexReleaseValidationResult::Failed => CodexReleaseCellStatus::Failed,
                CodexReleaseValidationResult::Unavailable => CodexReleaseCellStatus::Unavailable,
            })
            .unwrap_or(CodexReleaseCellStatus::NotRun)
    }

    /// Rejects any evidence entry whose exact Codex coordinates are absent from policy.
    pub fn validate_against_support_catalog(
        &self,
        support_catalog: &CodexSupportCatalog,
    ) -> Result<(), CodexReleaseEvidenceError> {
        for entry in &self.entries {
            let platform_release_coordinate =
                if entry.platform_environment == PlatformEnvironment::Wsl2 {
                    PlatformReleaseCoordinate::first_release_wsl2()
                } else {
                    PlatformReleaseCoordinate::native()
                };
            support_catalog
                .lookup_supported_entry(
                    &entry.codex_artifact_digest,
                    entry.target_triple,
                    entry.platform_environment,
                    &platform_release_coordinate,
                    &entry.observed_capabilities,
                    entry.integration_profile,
                )
                .map_err(|_| {
                    CodexReleaseEvidenceError::new(format!(
                        "release evidence for {}/{} has no exact Codex support-catalog entry",
                        entry.target_triple,
                        entry.platform_environment.as_str(),
                    ))
                })?;
        }
        Ok(())
    }
}

/// Derived release-evidence status for one exact required cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexReleaseCellStatus {
    Passed,
    Failed,
    Unavailable,
    NotRun,
}

/// Strict external release-evidence validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexReleaseEvidenceError {
    detail: String,
}

impl CodexReleaseEvidenceError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Returns bounded implementation-facing failure detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for CodexReleaseEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CodexReleaseEvidenceError {}

impl From<std::io::Error> for CodexReleaseEvidenceError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for CodexReleaseEvidenceError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for CodexReleaseEvidenceError {
    fn from(detail: String) -> Self {
        Self::new(detail)
    }
}

/// Loads and strictly parses an external release-evidence manifest file.
pub fn load_codex_release_evidence_manifest(
    path: &Path,
) -> Result<CodexReleaseEvidenceManifest, CodexReleaseEvidenceError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(CodexReleaseEvidenceError::new(
            "Codex release-evidence manifest must be a regular file",
        ));
    }
    if metadata.len() > MAX_RELEASE_EVIDENCE_BYTES as u64 {
        return Err(CodexReleaseEvidenceError::new(
            "Codex release-evidence manifest exceeds its byte bound",
        ));
    }
    parse_codex_release_evidence_manifest(&fs::read(path)?)
}

/// Strictly parses a canonical external release-evidence manifest.
pub fn parse_codex_release_evidence_manifest(
    bytes: &[u8],
) -> Result<CodexReleaseEvidenceManifest, CodexReleaseEvidenceError> {
    if bytes.len() > MAX_RELEASE_EVIDENCE_BYTES {
        return Err(CodexReleaseEvidenceError::new(
            "Codex release-evidence manifest exceeds its byte bound",
        ));
    }
    let ordered = parse_ordered_json(bytes)?;
    validate_manifest_json_shape(&ordered)?;
    let manifest: CodexReleaseEvidenceManifest = serde_json::from_value(ordered.into_json())?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Serializes validated evidence in canonical JSON field order.
pub fn serialize_codex_release_evidence_manifest(
    manifest: &CodexReleaseEvidenceManifest,
) -> Result<Vec<u8>, CodexReleaseEvidenceError> {
    validate_manifest(manifest)?;
    Ok(serde_json::to_vec(manifest)?)
}

/// Computes the canonical domain-separated digest for one external evidence record.
pub fn compute_codex_release_evidence_digest(
    evidence: &CodexReleaseValidationEvidence,
) -> Result<String, CodexReleaseEvidenceError> {
    let runner = record(vec![
        ("runner_id", string(&evidence.runner.runner_id)?),
        (
            "target_triple",
            string(evidence.runner.target_triple.as_str())?,
        ),
        (
            "architecture",
            string(evidence.runner.architecture.as_str())?,
        ),
        ("os_release", string(&evidence.runner.os_release)?),
        (
            "environment_image",
            string(&evidence.runner.environment_image)?,
        ),
    ])?;
    let scenario_results = evidence
        .scenario_results
        .iter()
        .map(encode_scenario_result)
        .collect::<Result<Vec<_>, _>>()?;
    let capabilities = evidence
        .observed_capabilities
        .iter()
        .map(|capability| string(capability.as_str()))
        .collect::<Result<Vec<_>, _>>()?;
    let canonical = record(vec![
        (
            "validation_result",
            string(evidence.validation_result.as_str())?,
        ),
        (
            "codex_artifact_digest",
            string(&evidence.codex_artifact_digest)?,
        ),
        ("target_triple", string(evidence.target_triple.as_str())?),
        (
            "platform_environment",
            string(evidence.platform_environment.as_str())?,
        ),
        ("observed_capabilities", list(capabilities)?),
        (
            "integration_profile",
            string(evidence.integration_profile.as_str())?,
        ),
        (
            "volicord_artifact_digest",
            string(&evidence.volicord_artifact_digest)?,
        ),
        ("source_revision", string(&evidence.source_revision)?),
        ("runner", runner),
        ("scenario_results", list(scenario_results)?),
        ("observed_at", string(&evidence.observed_at)?),
    ])?;
    let mut hasher = Sha256::new();
    hasher.update(EVIDENCE_DIGEST_DOMAIN);
    hasher.update(canonical);
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_manifest_json_shape(value: &OrderedJsonValue) -> Result<(), CodexReleaseEvidenceError> {
    let manifest_values =
        require_exact_fields(value, MANIFEST_FIELDS, "CodexReleaseEvidenceManifest")?;
    let OrderedJsonValue::Array(entries) = manifest_values[1] else {
        return Err(CodexReleaseEvidenceError::new(
            "CodexReleaseEvidenceManifest.entries must be an array",
        ));
    };
    for (index, entry) in entries.iter().enumerate() {
        let entry_values = require_exact_fields(
            entry,
            ENTRY_FIELDS,
            &format!("CodexReleaseEvidenceManifest.entries[{index}]"),
        )?;
        let evidence_values = require_exact_fields(
            entry_values[5],
            EVIDENCE_FIELDS,
            &format!("CodexReleaseEvidenceManifest.entries[{index}].validation_evidence"),
        )?;
        require_exact_fields(
            evidence_values[8],
            RUNNER_FIELDS,
            &format!("CodexReleaseEvidenceManifest.entries[{index}].validation_evidence.runner"),
        )?;
        let OrderedJsonValue::Array(results) = evidence_values[9] else {
            return Err(CodexReleaseEvidenceError::new(format!(
                "CodexReleaseEvidenceManifest.entries[{index}].validation_evidence.scenario_results must be an array"
            )));
        };
        for (scenario_index, result) in results.iter().enumerate() {
            require_exact_fields(
                result,
                SCENARIO_RESULT_FIELDS,
                &format!(
                    "CodexReleaseEvidenceManifest.entries[{index}].validation_evidence.scenario_results[{scenario_index}]"
                ),
            )?;
        }
    }
    Ok(())
}

fn validate_manifest(
    manifest: &CodexReleaseEvidenceManifest,
) -> Result<(), CodexReleaseEvidenceError> {
    if manifest.contract_id != CODEX_RELEASE_EVIDENCE_MANIFEST_CONTRACT_ID {
        return Err(CodexReleaseEvidenceError::new(
            "CodexReleaseEvidenceManifest.contract_id is unknown",
        ));
    }
    if manifest.entries.len() > 6 {
        return Err(CodexReleaseEvidenceError::new(
            "Codex release-evidence manifest may contain at most six entries",
        ));
    }
    let mut previous_identity = None;
    for entry in &manifest.entries {
        validate_entry(entry)?;
        let identity = evidence_identity(entry);
        if previous_identity
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(CodexReleaseEvidenceError::new(
                "Codex release-evidence entries must be unique and ordered by exact artifact/target/environment/profile identity",
            ));
        }
        previous_identity = Some(identity);
    }
    Ok(())
}

fn validate_entry(entry: &CodexReleaseEvidenceEntry) -> Result<(), CodexReleaseEvidenceError> {
    require_raw_sha256(
        "CodexReleaseEvidenceEntry.codex_artifact_digest",
        &entry.codex_artifact_digest,
    )?;
    if !entry
        .target_triple
        .supports_environment(entry.platform_environment)
    {
        return Err(CodexReleaseEvidenceError::new(
            "CodexReleaseEvidenceEntry target and platform environment do not match",
        ));
    }
    if !has_exact_first_release_codex_capabilities(&entry.observed_capabilities) {
        return Err(CodexReleaseEvidenceError::new(
            "CodexReleaseEvidenceEntry.observed_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }
    if entry.integration_profile != IntegrationProfile::Record {
        return Err(CodexReleaseEvidenceError::new(
            "CodexReleaseEvidenceEntry.integration_profile must be record",
        ));
    }
    let evidence = &entry.validation_evidence;
    if evidence.codex_artifact_digest != entry.codex_artifact_digest
        || evidence.target_triple != entry.target_triple
        || evidence.platform_environment != entry.platform_environment
        || evidence.observed_capabilities != entry.observed_capabilities
        || evidence.integration_profile != entry.integration_profile
    {
        return Err(CodexReleaseEvidenceError::new(
            "Codex release evidence coordinates must exactly match the owning entry",
        ));
    }
    validate_evidence(evidence)
}

fn validate_evidence(
    evidence: &CodexReleaseValidationEvidence,
) -> Result<(), CodexReleaseEvidenceError> {
    require_raw_sha256(
        "validation_evidence.codex_artifact_digest",
        &evidence.codex_artifact_digest,
    )?;
    require_raw_sha256(
        "validation_evidence.volicord_artifact_digest",
        &evidence.volicord_artifact_digest,
    )?;
    require_raw_sha256(
        "validation_evidence.evidence_digest",
        &evidence.evidence_digest,
    )?;
    validate_source_revision(&evidence.source_revision)?;
    validate_canonical_utc_timestamp("validation_evidence.observed_at", &evidence.observed_at)?;
    if !has_exact_first_release_codex_capabilities(&evidence.observed_capabilities) {
        return Err(CodexReleaseEvidenceError::new(
            "validation_evidence.observed_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }
    if evidence.integration_profile != IntegrationProfile::Record {
        return Err(CodexReleaseEvidenceError::new(
            "validation_evidence.integration_profile must be record",
        ));
    }
    validate_bounded_runner_string("runner.runner_id", &evidence.runner.runner_id, 256)?;
    validate_bounded_runner_string("runner.os_release", &evidence.runner.os_release, 512)?;
    validate_bounded_runner_string(
        "runner.environment_image",
        &evidence.runner.environment_image,
        512,
    )?;
    if evidence.runner.target_triple != evidence.target_triple {
        return Err(CodexReleaseEvidenceError::new(
            "runner.target_triple must exactly match validation_evidence.target_triple",
        ));
    }
    let expected_architecture = match evidence.target_triple.architecture() {
        "x86_64" => CodexReleaseRunnerArchitecture::X86_64,
        "aarch64" => CodexReleaseRunnerArchitecture::Aarch64,
        _ => unreachable!("ReleaseTargetTriple has a closed architecture set"),
    };
    if evidence.runner.architecture != expected_architecture {
        return Err(CodexReleaseEvidenceError::new(
            "runner.architecture must match the exact target triple",
        ));
    }
    if !evidence
        .target_triple
        .supports_environment(evidence.platform_environment)
    {
        return Err(CodexReleaseEvidenceError::new(
            "validation evidence target and platform environment do not match",
        ));
    }
    if evidence.platform_environment == PlatformEnvironment::Wsl2
        && evidence.runner.environment_image != PINNED_WSL2_ENVIRONMENT_IMAGE
    {
        return Err(CodexReleaseEvidenceError::new(
            "WSL2 runner.environment_image must equal the pinned first-release image coordinate",
        ));
    }

    let expected_scenarios = expected_scenarios(evidence.platform_environment);
    let actual_scenarios = evidence
        .scenario_results
        .iter()
        .map(|result| result.scenario_id)
        .collect::<Vec<_>>();
    if actual_scenarios != expected_scenarios {
        return Err(CodexReleaseEvidenceError::new(
            "validation_evidence.scenario_results must equal the canonical platform scenario catalog",
        ));
    }
    for result in &evidence.scenario_results {
        validate_scenario_result(result)?;
    }
    validate_validation_result(evidence)?;
    let expected_digest = compute_codex_release_evidence_digest(evidence)?;
    if evidence.evidence_digest != expected_digest {
        return Err(CodexReleaseEvidenceError::new(
            "validation_evidence.evidence_digest does not match canonical evidence",
        ));
    }
    Ok(())
}

fn expected_scenarios(platform: PlatformEnvironment) -> Vec<CodexReleaseScenarioId> {
    if platform == PlatformEnvironment::Wsl2 {
        CodexReleaseScenarioId::BASE
            .into_iter()
            .chain(CodexReleaseScenarioId::WSL2_ADDITIONAL)
            .collect()
    } else {
        CodexReleaseScenarioId::BASE.to_vec()
    }
}

fn evidence_identity(
    entry: &CodexReleaseEvidenceEntry,
) -> (&str, ReleaseTargetTriple, PlatformEnvironment, &str) {
    (
        &entry.codex_artifact_digest,
        entry.target_triple,
        entry.platform_environment,
        entry.integration_profile.as_str(),
    )
}

fn validate_scenario_result(
    result: &CodexReleaseScenarioResult,
) -> Result<(), CodexReleaseEvidenceError> {
    let reason = result.reason.as_ref();
    let digest = result.evidence_digest.as_ref();
    let observed_at = result.observed_at.as_ref();
    match result.status {
        CodexReleaseScenarioStatus::Passed => {
            if reason.is_some() || digest.is_none() || observed_at.is_none() {
                return Err(CodexReleaseEvidenceError::new(
                    "passed scenario requires null reason and non-null digest and timestamp",
                ));
            }
        }
        CodexReleaseScenarioStatus::Failed => {
            if reason.is_none() || digest.is_none() || observed_at.is_none() {
                return Err(CodexReleaseEvidenceError::new(
                    "failed scenario requires reason, digest, and timestamp",
                ));
            }
        }
        CodexReleaseScenarioStatus::Unavailable => {
            if reason.is_none() || observed_at.is_none() {
                return Err(CodexReleaseEvidenceError::new(
                    "unavailable scenario requires reason and timestamp",
                ));
            }
        }
        CodexReleaseScenarioStatus::NotRun => {
            if reason.is_none() || digest.is_some() || observed_at.is_some() {
                return Err(CodexReleaseEvidenceError::new(
                    "not_run scenario requires reason and null digest and timestamp",
                ));
            }
        }
    }
    if let Some(reason) = reason {
        validate_reason(reason)?;
    }
    if let Some(digest) = digest {
        require_raw_sha256("scenario_result.evidence_digest", digest)?;
    }
    if let Some(observed_at) = observed_at {
        validate_canonical_utc_timestamp("scenario_result.observed_at", observed_at)?;
    }
    Ok(())
}

fn validate_validation_result(
    evidence: &CodexReleaseValidationEvidence,
) -> Result<(), CodexReleaseEvidenceError> {
    let has_failed = evidence
        .scenario_results
        .iter()
        .any(|result| result.status == CodexReleaseScenarioStatus::Failed);
    let has_unavailable = evidence
        .scenario_results
        .iter()
        .any(|result| result.status == CodexReleaseScenarioStatus::Unavailable);
    let all_passed = evidence
        .scenario_results
        .iter()
        .all(|result| result.status == CodexReleaseScenarioStatus::Passed);
    let valid = match evidence.validation_result {
        CodexReleaseValidationResult::Passed => all_passed,
        CodexReleaseValidationResult::Failed => has_failed,
        CodexReleaseValidationResult::Unavailable => !has_failed && has_unavailable,
    };
    if valid {
        Ok(())
    } else {
        Err(CodexReleaseEvidenceError::new(
            "validation_result does not match scenario results",
        ))
    }
}

fn require_raw_sha256(name: &str, value: &str) -> Result<(), CodexReleaseEvidenceError> {
    if is_canonical_sha256_hex(value) {
        Ok(())
    } else {
        Err(CodexReleaseEvidenceError::new(format!(
            "{name} must be raw lowercase SHA-256"
        )))
    }
}

fn validate_source_revision(value: &str) -> Result<(), CodexReleaseEvidenceError> {
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(CodexReleaseEvidenceError::new(
            "validation_evidence.source_revision must be a raw lowercase 40- or 64-hex Git object ID",
        ))
    }
}

fn validate_bounded_runner_string(
    name: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), CodexReleaseEvidenceError> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.chars().any(|character| character.is_control())
    {
        Err(CodexReleaseEvidenceError::new(format!(
            "{name} must be nonempty bounded control-free UTF-8"
        )))
    } else {
        Ok(())
    }
}

fn validate_reason(reason: &str) -> Result<(), CodexReleaseEvidenceError> {
    let valid = !reason.is_empty()
        && reason.len() <= 128
        && reason.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' => true,
            b'0'..=b'9' | b'_' => index > 0,
            _ => false,
        });
    if valid {
        Ok(())
    } else {
        Err(CodexReleaseEvidenceError::new(
            "scenario reason must match [a-z][a-z0-9_]{0,127}",
        ))
    }
}

fn validate_canonical_utc_timestamp(
    name: &str,
    value: &str,
) -> Result<(), CodexReleaseEvidenceError> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| {
        CodexReleaseEvidenceError::new(format!("{name} must be canonical RFC 3339 UTC"))
    })?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::AutoSi, true) != value
    {
        return Err(CodexReleaseEvidenceError::new(format!(
            "{name} must be canonical RFC 3339 UTC"
        )));
    }
    Ok(())
}

fn encode_scenario_result(
    result: &CodexReleaseScenarioResult,
) -> Result<Vec<u8>, CodexReleaseEvidenceError> {
    Ok(record(vec![
        ("scenario_id", string(result.scenario_id.as_str())?),
        ("status", string(result.status.as_str())?),
        (
            "reason",
            nullable(
                result
                    .reason
                    .as_ref()
                    .map(|value| string(value))
                    .transpose()?,
            )?,
        ),
        (
            "evidence_digest",
            nullable(
                result
                    .evidence_digest
                    .as_ref()
                    .map(|value| string(value))
                    .transpose()?,
            )?,
        ),
        (
            "observed_at",
            nullable(
                result
                    .observed_at
                    .as_ref()
                    .map(|value| string(value))
                    .transpose()?,
            )?,
        ),
    ])?)
}
