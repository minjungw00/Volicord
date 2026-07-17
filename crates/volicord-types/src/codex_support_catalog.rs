//! Embedded exact-artifact Codex runtime support policy.

use std::{collections::BTreeSet, error::Error, fmt, fs, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    codex_contract::{
        list, parse_ordered_json, record, require_exact_fields, string, OrderedJsonValue,
    },
    has_exact_first_release_codex_capabilities, is_canonical_sha256_hex, CodexCapability,
    ErrorCode, FailureCategory, IntegrationProfile, PlatformEnvironment, PlatformReleaseCoordinate,
};

/// Contract identifier for the embedded runtime support catalog.
pub const CODEX_SUPPORT_CATALOG_CONTRACT_ID: &str = "volicord.codex-support-catalog";

/// Repository-relative source path of the embedded runtime support catalog.
pub const CODEX_SUPPORT_CATALOG_PATH: &str =
    "crates/volicord-types/contracts/codex-support-catalog.json";

/// Machine-readable reason for an absent or mismatched exact Codex artifact.
pub const UNSUPPORTED_HOST_ARTIFACT_REASON: &str = "unsupported_host_artifact";

const EMBEDDED_SUPPORT_CATALOG_BYTES: &[u8] =
    include_bytes!("../contracts/codex-support-catalog.json");
const MAX_SUPPORT_CATALOG_BYTES: usize = 1024 * 1024;
const SUPPORT_CATALOG_DIGEST_DOMAIN: &[u8] = b"volicord.codex-support-catalog\0";
const CATALOG_FIELDS: &[&str] = &["contract_id", "entries"];
const ENTRY_FIELDS: &[&str] = &[
    "codex_artifact_digest",
    "platform_environment",
    "platform_release_coordinate",
    "integration_profile",
    "verified_capabilities",
];
const NATIVE_COORDINATE_FIELDS: &[&str] = &["kind"];
const WSL2_COORDINATE_FIELDS: &[&str] = &[
    "kind",
    "distribution_name",
    "distribution_id",
    "distribution_version",
    "environment_image",
];
const TEST_ONLY_DESCRIPTOR_FIELDS: &[&str] = &[
    "test_only",
    "fixture_id",
    "codex_artifact_digest",
    "platform_environment",
    "observed_capabilities",
];

/// One exact runtime support-policy coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexSupportEntry {
    /// Raw SHA-256 digest of the exact finalized Codex executable.
    pub codex_artifact_digest: String,
    /// Supported platform environment.
    pub platform_environment: PlatformEnvironment,
    /// Exact supported native or WSL2 release coordinate.
    pub platform_release_coordinate: PlatformReleaseCoordinate,
    /// Exact supported integration profile.
    pub integration_profile: IntegrationProfile,
    /// Complete verified capability list required by runtime policy.
    pub verified_capabilities: Vec<CodexCapability>,
}

/// Strict embedded Codex runtime support catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodexSupportCatalog {
    contract_id: String,
    entries: Vec<CodexSupportEntry>,
}

impl CodexSupportCatalog {
    /// Creates and validates a support catalog from exact policy entries.
    pub fn from_entries(entries: Vec<CodexSupportEntry>) -> Result<Self, CodexSupportCatalogError> {
        let catalog = Self {
            contract_id: CODEX_SUPPORT_CATALOG_CONTRACT_ID.to_owned(),
            entries,
        };
        validate_catalog(&catalog)?;
        Ok(catalog)
    }

    /// Returns the exact contract identifier.
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns canonical zero-to-four runtime support entries.
    pub fn entries(&self) -> &[CodexSupportEntry] {
        &self.entries
    }

    /// Selects one exact artifact/platform/profile/capability policy entry.
    pub fn lookup_supported_entry(
        &self,
        codex_artifact_digest: &str,
        platform_environment: PlatformEnvironment,
        platform_release_coordinate: &PlatformReleaseCoordinate,
        verified_capabilities: &[CodexCapability],
        integration_profile: IntegrationProfile,
    ) -> Result<&CodexSupportEntry, UnsupportedHostArtifact> {
        if !is_canonical_sha256_hex(codex_artifact_digest)
            || !has_exact_first_release_codex_capabilities(verified_capabilities)
            || platform_release_coordinate
                .validate_for(platform_environment)
                .is_err()
        {
            return Err(UnsupportedHostArtifact);
        }

        self.entries
            .iter()
            .find(|entry| {
                entry.codex_artifact_digest == codex_artifact_digest
                    && entry.platform_environment == platform_environment
                    && entry.platform_release_coordinate == *platform_release_coordinate
                    && entry.integration_profile == integration_profile
                    && entry.verified_capabilities == verified_capabilities
            })
            .ok_or(UnsupportedHostArtifact)
    }

    /// Computes the domain-separated identity of only this runtime policy catalog.
    pub fn identity_digest(&self) -> Result<String, CodexSupportCatalogError> {
        let entries = self
            .entries
            .iter()
            .map(encode_support_entry)
            .collect::<Result<Vec<_>, _>>()?;
        let canonical = record(vec![
            ("contract_id", string(&self.contract_id)?),
            ("entries", list(entries)?),
        ])?;
        let mut hasher = Sha256::new();
        hasher.update(SUPPORT_CATALOG_DIGEST_DOMAIN);
        hasher.update(canonical);
        Ok(format!("{:x}", hasher.finalize()))
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
    pub codex_artifact_digest: String,
    /// Fixture platform coordinate.
    pub platform_environment: PlatformEnvironment,
    /// Closed capabilities exercised by the fixture.
    pub observed_capabilities: Vec<CodexCapability>,
}

/// Strict runtime support-catalog validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSupportCatalogError {
    detail: String,
}

impl CodexSupportCatalogError {
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

impl fmt::Display for CodexSupportCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CodexSupportCatalogError {}

impl From<std::io::Error> for CodexSupportCatalogError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for CodexSupportCatalogError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for CodexSupportCatalogError {
    fn from(detail: String) -> Self {
        Self::new(detail)
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

/// Parses the build-embedded runtime support catalog.
pub fn embedded_codex_support_catalog() -> Result<CodexSupportCatalog, CodexSupportCatalogError> {
    parse_codex_support_catalog(EMBEDDED_SUPPORT_CATALOG_BYTES)
}

/// Performs production runtime lookup against only the embedded support catalog.
pub fn lookup_embedded_codex_support_entry(
    codex_artifact_digest: &str,
    platform_environment: PlatformEnvironment,
    platform_release_coordinate: &PlatformReleaseCoordinate,
    verified_capabilities: &[CodexCapability],
    integration_profile: IntegrationProfile,
) -> Result<CodexSupportEntry, UnsupportedHostArtifact> {
    embedded_codex_support_catalog()
        .map_err(|_| UnsupportedHostArtifact)?
        .lookup_supported_entry(
            codex_artifact_digest,
            platform_environment,
            platform_release_coordinate,
            verified_capabilities,
            integration_profile,
        )
        .cloned()
}

/// Loads and strictly parses a runtime support catalog file.
pub fn load_codex_support_catalog(
    path: &Path,
) -> Result<CodexSupportCatalog, CodexSupportCatalogError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(CodexSupportCatalogError::new(
            "Codex support catalog must be a regular file",
        ));
    }
    if metadata.len() > MAX_SUPPORT_CATALOG_BYTES as u64 {
        return Err(CodexSupportCatalogError::new(
            "Codex support catalog exceeds its byte bound",
        ));
    }
    parse_codex_support_catalog(&fs::read(path)?)
}

/// Strictly parses the canonical runtime support catalog.
pub fn parse_codex_support_catalog(
    bytes: &[u8],
) -> Result<CodexSupportCatalog, CodexSupportCatalogError> {
    if bytes.len() > MAX_SUPPORT_CATALOG_BYTES {
        return Err(CodexSupportCatalogError::new(
            "Codex support catalog exceeds its byte bound",
        ));
    }
    let ordered = parse_ordered_json(bytes)?;
    validate_catalog_json_shape(&ordered)?;
    let catalog: CodexSupportCatalog = serde_json::from_value(ordered.into_json())?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

/// Serializes a validated catalog in its canonical JSON field order.
pub fn serialize_codex_support_catalog(
    catalog: &CodexSupportCatalog,
) -> Result<Vec<u8>, CodexSupportCatalogError> {
    validate_catalog(catalog)?;
    Ok(serde_json::to_vec(catalog)?)
}

/// Strictly parses an explicit fixture-only descriptor.
pub fn parse_test_only_codex_descriptor(
    bytes: &[u8],
) -> Result<TestOnlyCodexDescriptor, CodexSupportCatalogError> {
    let ordered = parse_ordered_json(bytes)?;
    require_exact_fields(
        &ordered,
        TEST_ONLY_DESCRIPTOR_FIELDS,
        "TestOnlyCodexDescriptor",
    )?;
    let descriptor: TestOnlyCodexDescriptor = serde_json::from_value(ordered.into_json())?;
    if !descriptor.test_only {
        return Err(CodexSupportCatalogError::new(
            "TestOnlyCodexDescriptor.test_only must be true",
        ));
    }
    require_raw_sha256(
        "TestOnlyCodexDescriptor.codex_artifact_digest",
        &descriptor.codex_artifact_digest,
    )?;
    let mut seen = BTreeSet::new();
    if descriptor
        .observed_capabilities
        .iter()
        .any(|capability| !seen.insert(*capability))
    {
        return Err(CodexSupportCatalogError::new(
            "TestOnlyCodexDescriptor.observed_capabilities must not contain duplicates",
        ));
    }
    Ok(descriptor)
}

fn validate_catalog_json_shape(value: &OrderedJsonValue) -> Result<(), CodexSupportCatalogError> {
    let catalog_values = require_exact_fields(value, CATALOG_FIELDS, "CodexSupportCatalog")?;
    let OrderedJsonValue::Array(entries) = catalog_values[1] else {
        return Err(CodexSupportCatalogError::new(
            "CodexSupportCatalog.entries must be an array",
        ));
    };
    for (index, entry) in entries.iter().enumerate() {
        let entry_values = require_exact_fields(
            entry,
            ENTRY_FIELDS,
            &format!("CodexSupportCatalog.entries[{index}]"),
        )?;
        validate_platform_release_coordinate_json(
            entry_values[2],
            &format!("CodexSupportCatalog.entries[{index}].platform_release_coordinate"),
        )?;
    }
    Ok(())
}

fn validate_platform_release_coordinate_json(
    value: &OrderedJsonValue,
    name: &str,
) -> Result<(), CodexSupportCatalogError> {
    let OrderedJsonValue::Object(fields) = value else {
        return Err(CodexSupportCatalogError::new(format!(
            "{name} must be an object"
        )));
    };
    let Some((field, OrderedJsonValue::String(kind))) = fields.first() else {
        return Err(CodexSupportCatalogError::new(format!(
            "{name}.kind must be first and must be a string"
        )));
    };
    if field != "kind" {
        return Err(CodexSupportCatalogError::new(format!(
            "{name}.kind must be first"
        )));
    }
    match kind.as_str() {
        "native" => require_exact_fields(value, NATIVE_COORDINATE_FIELDS, name).map(|_| ())?,
        "wsl2" => require_exact_fields(value, WSL2_COORDINATE_FIELDS, name).map(|_| ())?,
        _ => {
            return Err(CodexSupportCatalogError::new(format!(
                "{name}.kind is unknown"
            )))
        }
    }
    Ok(())
}

fn validate_catalog(catalog: &CodexSupportCatalog) -> Result<(), CodexSupportCatalogError> {
    if catalog.contract_id != CODEX_SUPPORT_CATALOG_CONTRACT_ID {
        return Err(CodexSupportCatalogError::new(
            "CodexSupportCatalog.contract_id is unknown",
        ));
    }
    if catalog.entries.len() > 4 {
        return Err(CodexSupportCatalogError::new(
            "Codex support catalog may contain at most four entries",
        ));
    }
    let mut previous_platform_index = None;
    for entry in &catalog.entries {
        validate_support_entry(entry)?;
        let platform_index = platform_index(entry.platform_environment);
        if previous_platform_index.is_some_and(|previous| previous >= platform_index) {
            return Err(CodexSupportCatalogError::new(
                "Codex support entries must be unique and in linux, macos, native_windows, wsl2 order",
            ));
        }
        previous_platform_index = Some(platform_index);
    }
    Ok(())
}

fn validate_support_entry(entry: &CodexSupportEntry) -> Result<(), CodexSupportCatalogError> {
    require_raw_sha256(
        "CodexSupportEntry.codex_artifact_digest",
        &entry.codex_artifact_digest,
    )?;
    entry
        .platform_release_coordinate
        .validate_for(entry.platform_environment)
        .map_err(|_| {
            CodexSupportCatalogError::new(
                "CodexSupportEntry.platform_release_coordinate is invalid",
            )
        })?;
    if entry.integration_profile != IntegrationProfile::Record {
        return Err(CodexSupportCatalogError::new(
            "CodexSupportEntry.integration_profile must be record",
        ));
    }
    if !has_exact_first_release_codex_capabilities(&entry.verified_capabilities) {
        return Err(CodexSupportCatalogError::new(
            "CodexSupportEntry.verified_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }
    Ok(())
}

fn encode_support_entry(entry: &CodexSupportEntry) -> Result<Vec<u8>, CodexSupportCatalogError> {
    let capabilities = entry
        .verified_capabilities
        .iter()
        .map(|capability| string(capability.as_str()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(record(vec![
        (
            "codex_artifact_digest",
            string(&entry.codex_artifact_digest)?,
        ),
        (
            "platform_environment",
            string(entry.platform_environment.as_str())?,
        ),
        (
            "platform_release_coordinate",
            encode_platform_release_coordinate(&entry.platform_release_coordinate)?,
        ),
        (
            "integration_profile",
            string(entry.integration_profile.as_str())?,
        ),
        ("verified_capabilities", list(capabilities)?),
    ])?)
}

fn encode_platform_release_coordinate(
    coordinate: &PlatformReleaseCoordinate,
) -> Result<Vec<u8>, CodexSupportCatalogError> {
    match coordinate {
        PlatformReleaseCoordinate::Native => Ok(record(vec![("kind", string("native")?)])?),
        PlatformReleaseCoordinate::Wsl2 {
            distribution_name,
            distribution_id,
            distribution_version,
            environment_image,
        } => Ok(record(vec![
            ("kind", string("wsl2")?),
            ("distribution_name", string(distribution_name)?),
            ("distribution_id", string(distribution_id)?),
            ("distribution_version", string(distribution_version)?),
            ("environment_image", string(environment_image)?),
        ])?),
    }
}

fn require_raw_sha256(name: &str, value: &str) -> Result<(), CodexSupportCatalogError> {
    if is_canonical_sha256_hex(value) {
        Ok(())
    } else {
        Err(CodexSupportCatalogError::new(format!(
            "{name} must be raw lowercase SHA-256"
        )))
    }
}

fn platform_index(platform: PlatformEnvironment) -> usize {
    match platform {
        PlatformEnvironment::Linux => 0,
        PlatformEnvironment::Macos => 1,
        PlatformEnvironment::NativeWindows => 2,
        PlatformEnvironment::Wsl2 => 3,
    }
}
