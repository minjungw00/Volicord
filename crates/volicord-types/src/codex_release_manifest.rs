//! Exact Codex release-cell evidence and checked-in support-manifest contract.

use std::{collections::BTreeSet, error::Error, fmt, fs, path::Path};

use chrono::{DateTime, SecondsFormat};
use schemars::JsonSchema;
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use crate::{
    has_exact_first_release_codex_capabilities, is_canonical_sha256_hex, CodexCapability,
    ErrorCode, FailureCategory, IntegrationProfile, PlatformEnvironment, PlatformReleaseCoordinate,
    RequiredNullable, PINNED_WSL2_ENVIRONMENT_IMAGE,
};

/// Repository-relative path of the only Codex release support manifest.
pub const CHECKED_IN_CODEX_RELEASE_MANIFEST_PATH: &str =
    "tests/release-validation/contracts/codex-release-manifest.json";

/// Machine-readable reason for an absent or mismatched exact Codex artifact.
pub const UNSUPPORTED_HOST_ARTIFACT_REASON: &str = "unsupported_host_artifact";

/// Independent platform cells in canonical manifest order.
pub const CODEX_RELEASE_PLATFORMS: [PlatformEnvironment; 4] = [
    PlatformEnvironment::Linux,
    PlatformEnvironment::Macos,
    PlatformEnvironment::NativeWindows,
    PlatformEnvironment::Wsl2,
];

const CHECKED_IN_MANIFEST_BYTES: &[u8] =
    include_bytes!("../../../tests/release-validation/contracts/codex-release-manifest.json");
const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
const EVIDENCE_DIGEST_DOMAIN: &[u8] = b"volicord.codex-release-validation-evidence\0";

const CELL_FIELDS: &[&str] = &[
    "artifact_digest",
    "platform",
    "observed_capabilities",
    "integration_profile",
    "validation_evidence",
];
const EVIDENCE_FIELDS: &[&str] = &[
    "status",
    "artifact_digest",
    "platform",
    "observed_capabilities",
    "integration_profile",
    "volicord_artifact_digest",
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
const TEST_ONLY_DESCRIPTOR_FIELDS: &[&str] = &[
    "test_only",
    "fixture_id",
    "artifact_digest",
    "platform",
    "observed_capabilities",
];

/// Top-level result of one qualifying platform-cell attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodexReleaseValidationStatus {
    /// Every required scenario passed.
    Passed,
    /// At least one required scenario failed.
    Failed,
    /// No scenario failed, but an unavailable prerequisite prevented completion.
    Unavailable,
}

impl CodexReleaseValidationStatus {
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

/// Architecture recorded for the exact release runner.
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
    /// Fresh managed installation.
    FreshInstall,
    /// Runtime Home creation and validation.
    RuntimeHomeCreation,
    /// Personal managed-binding lifecycle.
    PersonalManagedBinding,
    /// Shared managed-binding lifecycle.
    SharedManagedBinding,
    /// Receipt issuance and current validation.
    ReceiptCreateAndValidate,
    /// Managed-configuration drift detection.
    ConfigurationDriftDetection,
    /// Canonical repair after drift.
    RepairAfterDrift,
    /// Ownership-safe uninstall.
    SafeUninstall,
    /// Symlink and canonical-path behavior.
    SymlinkAndCanonicalPath,
    /// Codex restart and stale evidence rejection.
    CodexRestart,
    /// Product Repository move and stale evidence rejection.
    ProjectMove,
    /// Complete Record-profile write workflow.
    RecordWriteWorkflow,
    /// Conservative suppression-unavailable behavior.
    SuppressionUnavailable,
    /// Unsupported host rejection.
    UnsupportedHost,
    /// Unsupported exact artifact rejection.
    UnsupportedHostArtifact,
    /// WSL shutdown/restart behavior.
    WslShutdownRestart,
    /// WSL2 ext4 topology acceptance.
    Wsl2Ext4Project,
    /// WSL2 DrvFS rejection.
    Wsl2DrvfsRejection,
    /// Cross-environment topology rejection.
    Wsl2CrossTopologyRejection,
    /// WSL1 rejection.
    Wsl1Rejection,
    /// Native-Windows receipt reuse rejection.
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

/// Exact environment coordinates for one release-validation runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseRunnerCoordinate {
    /// Stable runner identity.
    pub runner_id: String,
    /// Exact Rust-style target triple.
    pub target_triple: String,
    /// Runner architecture.
    pub architecture: CodexReleaseRunnerArchitecture,
    /// Exact operating-system release coordinate.
    pub os_release: String,
    /// Exact runner environment or pinned WSL2 image.
    pub environment_image: String,
}

/// Evidence outcome for one required scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseScenarioResult {
    /// Scenario identity.
    pub scenario_id: CodexReleaseScenarioId,
    /// Scenario execution status.
    pub status: CodexReleaseScenarioStatus,
    /// Required nullable machine-readable reason.
    pub reason: RequiredNullable<String>,
    /// Required nullable raw evidence digest.
    pub evidence_digest: RequiredNullable<String>,
    /// Required nullable canonical observation timestamp.
    pub observed_at: RequiredNullable<String>,
}

/// Complete evidence for one qualifying exact-artifact platform attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexReleaseValidationEvidence {
    /// Aggregate cell status.
    pub status: CodexReleaseValidationStatus,
    /// Raw digest of the exact finalized Codex executable.
    pub artifact_digest: String,
    /// Independent platform environment.
    pub platform: PlatformEnvironment,
    /// Exact complete canonical capability list.
    pub observed_capabilities: Vec<CodexCapability>,
    /// Exact first-release integration profile.
    pub integration_profile: IntegrationProfile,
    /// Raw digest of the exact Volicord executable under test.
    pub volicord_artifact_digest: String,
    /// Exact runner coordinates.
    pub runner: CodexReleaseRunnerCoordinate,
    /// Complete ordered platform scenario catalog.
    pub scenario_results: Vec<CodexReleaseScenarioResult>,
    /// Raw digest of the canonical evidence encoding.
    pub evidence_digest: String,
    /// Canonical UTC timestamp for the complete attempt.
    pub observed_at: String,
}

/// One exact finalized-artifact release-validation cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CodexReleaseCell {
    /// Raw digest of the exact finalized Codex executable.
    pub artifact_digest: String,
    /// Independent platform environment.
    pub platform: PlatformEnvironment,
    /// Exact complete canonical capability list.
    pub observed_capabilities: Vec<CodexCapability>,
    /// Exact first-release integration profile.
    pub integration_profile: IntegrationProfile,
    /// Complete validated release evidence.
    pub validation_evidence: CodexReleaseValidationEvidence,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexReleaseCellWire {
    artifact_digest: String,
    platform: PlatformEnvironment,
    observed_capabilities: Vec<CodexCapability>,
    integration_profile: IntegrationProfile,
    validation_evidence: CodexReleaseValidationEvidence,
}

impl<'de> Deserialize<'de> for CodexReleaseCell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CodexReleaseCellWire::deserialize(deserializer)?;
        let cell = Self {
            artifact_digest: wire.artifact_digest,
            platform: wire.platform,
            observed_capabilities: wire.observed_capabilities,
            integration_profile: wire.integration_profile,
            validation_evidence: wire.validation_evidence,
        };
        validate_cell(&cell).map_err(serde::de::Error::custom)?;
        Ok(cell)
    }
}

/// Explicit fixture-only Codex coordinate that can never register support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestOnlyCodexDescriptor {
    /// Must be the exact boolean `true`.
    pub test_only: bool,
    /// Fixture identity, separate from a release artifact.
    pub fixture_id: String,
    /// Raw fixture digest.
    pub artifact_digest: String,
    /// Fixture platform coordinate.
    pub platform: PlatformEnvironment,
    /// Closed capabilities exercised by the fixture.
    pub observed_capabilities: Vec<CodexCapability>,
}

/// Strict checked-in Codex release manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexReleaseManifest {
    cells: Vec<CodexReleaseCell>,
}

/// Derived release status for one independent platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformReleaseStatus {
    /// A qualifying attempt passed.
    Passed,
    /// A qualifying attempt failed.
    Failed,
    /// A qualifying attempt could not complete.
    Unavailable,
    /// No qualifying attempt is present in the manifest.
    NotRun,
}

/// Strict manifest or release-evidence validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexReleaseManifestError {
    detail: String,
}

impl CodexReleaseManifestError {
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

impl fmt::Display for CodexReleaseManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CodexReleaseManifestError {}

impl From<std::io::Error> for CodexReleaseManifestError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for CodexReleaseManifestError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

/// Exact-lookup failure for an absent or mismatched Codex artifact coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedHostArtifact;

impl UnsupportedHostArtifact {
    /// Returns the product-wide failure category.
    pub const fn failure_category(self) -> FailureCategory {
        FailureCategory::UnsupportedContract
    }

    /// Returns the public API error code.
    pub const fn error_code(self) -> ErrorCode {
        ErrorCode::UnsupportedContract
    }

    /// Returns the exact machine-readable reason.
    pub const fn reason(self) -> &'static str {
        UNSUPPORTED_HOST_ARTIFACT_REASON
    }
}

impl fmt::Display for UnsupportedHostArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(UNSUPPORTED_HOST_ARTIFACT_REASON)
    }
}

impl Error for UnsupportedHostArtifact {}

impl CodexReleaseManifest {
    /// Returns the zero-to-four reviewed cells in canonical platform order.
    pub fn cells(&self) -> &[CodexReleaseCell] {
        &self.cells
    }

    /// Returns the actual or derived status for one independent platform.
    pub fn platform_status(&self, platform: PlatformEnvironment) -> PlatformReleaseStatus {
        self.cells
            .iter()
            .find(|cell| cell.platform == platform)
            .map(|cell| match cell.validation_evidence.status {
                CodexReleaseValidationStatus::Passed => PlatformReleaseStatus::Passed,
                CodexReleaseValidationStatus::Failed => PlatformReleaseStatus::Failed,
                CodexReleaseValidationStatus::Unavailable => PlatformReleaseStatus::Unavailable,
            })
            .unwrap_or(PlatformReleaseStatus::NotRun)
    }

    /// Returns whether every independent platform has an exact passing cell.
    pub fn has_four_passing_platforms(&self) -> bool {
        CODEX_RELEASE_PLATFORMS
            .into_iter()
            .all(|platform| self.platform_status(platform) == PlatformReleaseStatus::Passed)
    }

    /// Selects only one exact passing artifact/platform/profile/capability cell.
    pub fn lookup_supported_cell(
        &self,
        artifact_digest: &str,
        platform: PlatformEnvironment,
        platform_release_coordinate: &PlatformReleaseCoordinate,
        observed_capabilities: &[CodexCapability],
        integration_profile: IntegrationProfile,
    ) -> Result<&CodexReleaseCell, UnsupportedHostArtifact> {
        if !is_canonical_sha256_hex(artifact_digest)
            || !has_exact_first_release_codex_capabilities(observed_capabilities)
            || platform_release_coordinate.validate_for(platform).is_err()
        {
            return Err(UnsupportedHostArtifact);
        }

        self.cells
            .iter()
            .find(|cell| {
                cell.validation_evidence.status == CodexReleaseValidationStatus::Passed
                    && cell.artifact_digest == artifact_digest
                    && cell.platform == platform
                    && cell.observed_capabilities == observed_capabilities
                    && cell.integration_profile == integration_profile
                    && (platform != PlatformEnvironment::Wsl2
                        || platform_release_coordinate.wsl2_environment_image()
                            == Some(cell.validation_evidence.runner.environment_image.as_str()))
            })
            .ok_or(UnsupportedHostArtifact)
    }
}

/// Parses the build projection of the single canonical checked-in manifest.
pub fn checked_in_codex_release_manifest() -> Result<CodexReleaseManifest, CodexReleaseManifestError>
{
    parse_codex_release_manifest(CHECKED_IN_MANIFEST_BYTES)
}

/// Performs production support lookup against only the checked-in manifest.
pub fn lookup_checked_in_supported_codex_release_cell(
    artifact_digest: &str,
    platform: PlatformEnvironment,
    platform_release_coordinate: &PlatformReleaseCoordinate,
    observed_capabilities: &[CodexCapability],
    integration_profile: IntegrationProfile,
) -> Result<CodexReleaseCell, UnsupportedHostArtifact> {
    checked_in_codex_release_manifest()
        .map_err(|_| UnsupportedHostArtifact)?
        .lookup_supported_cell(
            artifact_digest,
            platform,
            platform_release_coordinate,
            observed_capabilities,
            integration_profile,
        )
        .cloned()
}

/// Loads and strictly parses a manifest file through the production parser.
pub fn load_codex_release_manifest(
    path: &Path,
) -> Result<CodexReleaseManifest, CodexReleaseManifestError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(CodexReleaseManifestError::new(
            "Codex release manifest must be a regular file",
        ));
    }
    if metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(CodexReleaseManifestError::new(
            "Codex release manifest exceeds its byte bound",
        ));
    }
    parse_codex_release_manifest(&fs::read(path)?)
}

/// Strictly parses a canonical zero-to-four-cell Codex release manifest.
pub fn parse_codex_release_manifest(
    bytes: &[u8],
) -> Result<CodexReleaseManifest, CodexReleaseManifestError> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(CodexReleaseManifestError::new(
            "Codex release manifest exceeds its byte bound",
        ));
    }

    let ordered = parse_ordered_json(bytes)?;
    validate_manifest_json_shape(&ordered)?;
    let cells: Vec<CodexReleaseCell> = serde_json::from_value(ordered.into_json())?;
    validate_manifest_cells(&cells)?;
    Ok(CodexReleaseManifest { cells })
}

/// Strictly parses an explicit fixture-only descriptor.
pub fn parse_test_only_codex_descriptor(
    bytes: &[u8],
) -> Result<TestOnlyCodexDescriptor, CodexReleaseManifestError> {
    let ordered = parse_ordered_json(bytes)?;
    require_exact_fields(
        &ordered,
        TEST_ONLY_DESCRIPTOR_FIELDS,
        "TestOnlyCodexDescriptor",
    )?;
    let descriptor: TestOnlyCodexDescriptor = serde_json::from_value(ordered.into_json())?;
    if !descriptor.test_only {
        return Err(CodexReleaseManifestError::new(
            "TestOnlyCodexDescriptor.test_only must be true",
        ));
    }
    require_raw_sha256(
        "TestOnlyCodexDescriptor.artifact_digest",
        &descriptor.artifact_digest,
    )?;
    let mut seen = BTreeSet::new();
    if descriptor
        .observed_capabilities
        .iter()
        .any(|capability| !seen.insert(*capability))
    {
        return Err(CodexReleaseManifestError::new(
            "TestOnlyCodexDescriptor.observed_capabilities must not contain duplicates",
        ));
    }
    Ok(descriptor)
}

/// Computes the canonical domain-separated digest for release evidence.
pub fn compute_codex_release_evidence_digest(
    evidence: &CodexReleaseValidationEvidence,
) -> Result<String, CodexReleaseManifestError> {
    let runner = record(vec![
        ("runner_id", string(&evidence.runner.runner_id)?),
        ("target_triple", string(&evidence.runner.target_triple)?),
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
        ("status", string(evidence.status.as_str())?),
        ("artifact_digest", string(&evidence.artifact_digest)?),
        ("platform", string(evidence.platform.as_str())?),
        ("observed_capabilities", list(capabilities)?),
        (
            "integration_profile",
            string(evidence.integration_profile.as_str())?,
        ),
        (
            "volicord_artifact_digest",
            string(&evidence.volicord_artifact_digest)?,
        ),
        ("runner", runner),
        ("scenario_results", list(scenario_results)?),
        ("observed_at", string(&evidence.observed_at)?),
    ])?;

    let mut hasher = Sha256::new();
    hasher.update(EVIDENCE_DIGEST_DOMAIN);
    hasher.update(canonical);
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_manifest_json_shape(value: &OrderedJsonValue) -> Result<(), CodexReleaseManifestError> {
    let OrderedJsonValue::Array(cells) = value else {
        return Err(CodexReleaseManifestError::new(
            "Codex release manifest must be a JSON array",
        ));
    };
    for (index, cell) in cells.iter().enumerate() {
        let cell_values =
            require_exact_fields(cell, CELL_FIELDS, &format!("CodexReleaseCell[{index}]"))?;
        let evidence_values = require_exact_fields(
            cell_values[4],
            EVIDENCE_FIELDS,
            &format!("CodexReleaseCell[{index}].validation_evidence"),
        )?;
        require_exact_fields(
            evidence_values[6],
            RUNNER_FIELDS,
            &format!("CodexReleaseCell[{index}].validation_evidence.runner"),
        )?;
        let OrderedJsonValue::Array(results) = evidence_values[7] else {
            return Err(CodexReleaseManifestError::new(format!(
                "CodexReleaseCell[{index}].validation_evidence.scenario_results must be an array"
            )));
        };
        for (scenario_index, result) in results.iter().enumerate() {
            require_exact_fields(
                result,
                SCENARIO_RESULT_FIELDS,
                &format!(
                    "CodexReleaseCell[{index}].validation_evidence.scenario_results[{scenario_index}]"
                ),
            )?;
        }
    }
    Ok(())
}

fn validate_manifest_cells(cells: &[CodexReleaseCell]) -> Result<(), CodexReleaseManifestError> {
    if cells.len() > CODEX_RELEASE_PLATFORMS.len() {
        return Err(CodexReleaseManifestError::new(
            "Codex release manifest may contain at most four cells",
        ));
    }

    let mut previous_platform_index = None;
    for cell in cells {
        let platform_index = CODEX_RELEASE_PLATFORMS
            .iter()
            .position(|platform| platform == &cell.platform)
            .expect("PlatformEnvironment is closed");
        if previous_platform_index.is_some_and(|previous| previous >= platform_index) {
            return Err(CodexReleaseManifestError::new(
                "Codex release manifest cells must be unique and in linux, macos, native_windows, wsl2 order",
            ));
        }
        previous_platform_index = Some(platform_index);
        validate_cell(cell)?;
    }
    Ok(())
}

fn validate_cell(cell: &CodexReleaseCell) -> Result<(), CodexReleaseManifestError> {
    require_raw_sha256("CodexReleaseCell.artifact_digest", &cell.artifact_digest)?;
    if !has_exact_first_release_codex_capabilities(&cell.observed_capabilities) {
        return Err(CodexReleaseManifestError::new(
            "CodexReleaseCell.observed_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }
    if cell.integration_profile != IntegrationProfile::Record {
        return Err(CodexReleaseManifestError::new(
            "CodexReleaseCell.integration_profile must be record",
        ));
    }

    let evidence = &cell.validation_evidence;
    if evidence.artifact_digest != cell.artifact_digest
        || evidence.platform != cell.platform
        || evidence.observed_capabilities != cell.observed_capabilities
        || evidence.integration_profile != cell.integration_profile
    {
        return Err(CodexReleaseManifestError::new(
            "Codex release evidence coordinates must exactly match the owning cell",
        ));
    }

    validate_evidence(evidence)
}

fn validate_evidence(
    evidence: &CodexReleaseValidationEvidence,
) -> Result<(), CodexReleaseManifestError> {
    require_raw_sha256(
        "validation_evidence.artifact_digest",
        &evidence.artifact_digest,
    )?;
    require_raw_sha256(
        "validation_evidence.volicord_artifact_digest",
        &evidence.volicord_artifact_digest,
    )?;
    require_raw_sha256(
        "validation_evidence.evidence_digest",
        &evidence.evidence_digest,
    )?;
    validate_canonical_utc_timestamp("validation_evidence.observed_at", &evidence.observed_at)?;
    if !has_exact_first_release_codex_capabilities(&evidence.observed_capabilities) {
        return Err(CodexReleaseManifestError::new(
            "validation_evidence.observed_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }
    if evidence.integration_profile != IntegrationProfile::Record {
        return Err(CodexReleaseManifestError::new(
            "validation_evidence.integration_profile must be record",
        ));
    }

    validate_bounded_runner_string("runner.runner_id", &evidence.runner.runner_id, 256)?;
    validate_bounded_runner_string("runner.target_triple", &evidence.runner.target_triple, 256)?;
    validate_bounded_runner_string("runner.os_release", &evidence.runner.os_release, 512)?;
    validate_bounded_runner_string(
        "runner.environment_image",
        &evidence.runner.environment_image,
        512,
    )?;
    if evidence.platform == PlatformEnvironment::Wsl2
        && evidence.runner.environment_image != PINNED_WSL2_ENVIRONMENT_IMAGE
    {
        return Err(CodexReleaseManifestError::new(
            "WSL2 runner.environment_image must equal the pinned first-release image coordinate",
        ));
    }

    let expected_scenarios = expected_scenarios(evidence.platform);
    let actual_scenarios = evidence
        .scenario_results
        .iter()
        .map(|result| result.scenario_id)
        .collect::<Vec<_>>();
    if actual_scenarios != expected_scenarios {
        return Err(CodexReleaseManifestError::new(
            "validation_evidence.scenario_results must contain the exact ordered platform catalog",
        ));
    }

    for result in &evidence.scenario_results {
        validate_scenario_result(result)?;
    }
    validate_evidence_status(evidence)?;

    let expected_digest = compute_codex_release_evidence_digest(evidence)?;
    if evidence.evidence_digest != expected_digest {
        return Err(CodexReleaseManifestError::new(
            "validation_evidence.evidence_digest does not match canonical evidence bytes",
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

fn validate_scenario_result(
    result: &CodexReleaseScenarioResult,
) -> Result<(), CodexReleaseManifestError> {
    let reason = result.reason.as_ref();
    let digest = result.evidence_digest.as_ref();
    let observed_at = result.observed_at.as_ref();

    match result.status {
        CodexReleaseScenarioStatus::Passed => {
            if reason.is_some() || digest.is_none() || observed_at.is_none() {
                return Err(CodexReleaseManifestError::new(
                    "passed scenario requires null reason and non-null digest and observed_at",
                ));
            }
        }
        CodexReleaseScenarioStatus::Failed => {
            if reason.is_none() || digest.is_none() || observed_at.is_none() {
                return Err(CodexReleaseManifestError::new(
                    "failed scenario requires non-null reason, digest, and observed_at",
                ));
            }
        }
        CodexReleaseScenarioStatus::Unavailable => {
            if reason.is_none() || observed_at.is_none() {
                return Err(CodexReleaseManifestError::new(
                    "unavailable scenario requires non-null reason and observed_at",
                ));
            }
        }
        CodexReleaseScenarioStatus::NotRun => {
            if reason.is_none() || digest.is_some() || observed_at.is_some() {
                return Err(CodexReleaseManifestError::new(
                    "not_run scenario requires non-null reason and null digest and observed_at",
                ));
            }
        }
    }

    if let Some(reason) = reason {
        validate_reason(reason)?;
    }
    if let Some(digest) = digest {
        require_raw_sha256("scenario_results[].evidence_digest", digest)?;
    }
    if let Some(observed_at) = observed_at {
        validate_canonical_utc_timestamp("scenario_results[].observed_at", observed_at)?;
    }
    Ok(())
}

fn validate_evidence_status(
    evidence: &CodexReleaseValidationEvidence,
) -> Result<(), CodexReleaseManifestError> {
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

    let valid = match evidence.status {
        CodexReleaseValidationStatus::Passed => all_passed,
        CodexReleaseValidationStatus::Failed => has_failed,
        CodexReleaseValidationStatus::Unavailable => !has_failed && has_unavailable,
    };
    if !valid {
        return Err(CodexReleaseManifestError::new(
            "validation_evidence.status does not match its scenario results",
        ));
    }
    Ok(())
}

fn require_raw_sha256(name: &str, value: &str) -> Result<(), CodexReleaseManifestError> {
    if is_canonical_sha256_hex(value) {
        Ok(())
    } else {
        Err(CodexReleaseManifestError::new(format!(
            "{name} must be 64 lowercase hexadecimal characters"
        )))
    }
}

fn validate_bounded_runner_string(
    name: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), CodexReleaseManifestError> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        Err(CodexReleaseManifestError::new(format!(
            "{name} must be nonempty, control-free UTF-8 of at most {max_bytes} bytes"
        )))
    } else {
        Ok(())
    }
}

fn validate_reason(reason: &str) -> Result<(), CodexReleaseManifestError> {
    let bytes = reason.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .skip(1)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(CodexReleaseManifestError::new(
            "scenario reason must match [a-z][a-z0-9_]{0,127}",
        ))
    }
}

fn validate_canonical_utc_timestamp(
    name: &str,
    value: &str,
) -> Result<(), CodexReleaseManifestError> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| {
        CodexReleaseManifestError::new(format!("{name} must be canonical RFC 3339 UTC"))
    })?;
    if !value.ends_with('Z') || parsed.to_rfc3339_opts(SecondsFormat::AutoSi, true) != value {
        return Err(CodexReleaseManifestError::new(format!(
            "{name} must be canonical RFC 3339 UTC"
        )));
    }
    Ok(())
}

fn encode_scenario_result(
    result: &CodexReleaseScenarioResult,
) -> Result<Vec<u8>, CodexReleaseManifestError> {
    record(vec![
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
    ])
}

fn u32be(value: usize) -> Result<[u8; 4], CodexReleaseManifestError> {
    let value = u32::try_from(value)
        .map_err(|_| CodexReleaseManifestError::new("canonical evidence length exceeds u32"))?;
    Ok(value.to_be_bytes())
}

fn blob(value: &[u8]) -> Result<Vec<u8>, CodexReleaseManifestError> {
    let mut encoded = Vec::with_capacity(4 + value.len());
    encoded.extend_from_slice(&u32be(value.len())?);
    encoded.extend_from_slice(value);
    Ok(encoded)
}

fn string(value: &str) -> Result<Vec<u8>, CodexReleaseManifestError> {
    blob(value.as_bytes())
}

fn list(items: Vec<Vec<u8>>) -> Result<Vec<u8>, CodexReleaseManifestError> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u32be(items.len())?);
    for item in items {
        encoded.extend_from_slice(&blob(&item)?);
    }
    Ok(encoded)
}

fn record(fields: Vec<(&str, Vec<u8>)>) -> Result<Vec<u8>, CodexReleaseManifestError> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u32be(fields.len())?);
    for (name, value) in fields {
        encoded.extend_from_slice(&string(name)?);
        encoded.extend_from_slice(&blob(&value)?);
    }
    Ok(encoded)
}

fn nullable(value: Option<Vec<u8>>) -> Result<Vec<u8>, CodexReleaseManifestError> {
    match value {
        None => Ok(vec![0]),
        Some(value) => {
            let mut encoded = vec![1];
            encoded.extend_from_slice(&blob(&value)?);
            Ok(encoded)
        }
    }
}

fn require_exact_fields<'a>(
    value: &'a OrderedJsonValue,
    expected: &[&str],
    name: &str,
) -> Result<Vec<&'a OrderedJsonValue>, CodexReleaseManifestError> {
    let OrderedJsonValue::Object(fields) = value else {
        return Err(CodexReleaseManifestError::new(format!(
            "{name} must be an object"
        )));
    };
    let actual = fields
        .iter()
        .map(|(field, _)| field.as_str())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(CodexReleaseManifestError::new(format!(
            "{name} fields must be present exactly once in canonical order"
        )));
    }
    Ok(fields.iter().map(|(_, value)| value).collect())
}

fn parse_ordered_json(bytes: &[u8]) -> Result<OrderedJsonValue, CodexReleaseManifestError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = OrderedJsonValue::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
enum OrderedJsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl OrderedJsonValue {
    fn into_json(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(value),
            Self::Number(value) => Value::Number(value),
            Self::String(value) => Value::String(value),
            Self::Array(values) => Value::Array(values.into_iter().map(Self::into_json).collect()),
            Self::Object(fields) => Value::Object(
                fields
                    .into_iter()
                    .map(|(name, value)| (name, value.into_json()))
                    .collect::<Map<_, _>>(),
            ),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

struct OrderedJsonVisitor;

impl<'de> Visitor<'de> for OrderedJsonVisitor {
    type Value = OrderedJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object fields")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(OrderedJsonValue::Number)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(OrderedJsonValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut fields = Vec::new();
        while let Some((name, value)) = map.next_entry::<String, OrderedJsonValue>()? {
            if !seen.insert(name.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate JSON field {name}"
                )));
            }
            fields.push((name, value));
        }
        Ok(OrderedJsonValue::Object(fields))
    }
}
