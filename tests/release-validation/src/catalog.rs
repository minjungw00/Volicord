//! Deterministic proposed-entry generation for the embedded Codex support catalog.

use std::path::Path;

use volicord_types::{
    CodexCapability, CodexSupportCatalog, CodexSupportEntry, IntegrationProfile,
    PlatformEnvironment, PlatformReleaseCoordinate, ReleaseTargetTriple,
};

use crate::{
    error::{ValidationError, ValidationResult},
    io::{sha256_external_file, ValidationContext},
};

const MAX_CODEX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;

/// Hashes an actual Codex executable and returns one validated proposed policy entry.
pub fn generate_support_entry(
    context: &ValidationContext,
    codex_path: &Path,
    target_triple: ReleaseTargetTriple,
    platform_environment: PlatformEnvironment,
    integration_profile: IntegrationProfile,
    declared_capabilities: &[CodexCapability],
) -> ValidationResult<CodexSupportEntry> {
    let capabilities = normalize_capabilities(declared_capabilities)?;
    let codex_artifact_digest =
        sha256_external_file(context, codex_path, Some(MAX_CODEX_ARTIFACT_BYTES))?;
    let entry = CodexSupportEntry {
        codex_artifact_digest,
        target_triple,
        platform_environment,
        platform_release_coordinate: if platform_environment == PlatformEnvironment::Wsl2 {
            PlatformReleaseCoordinate::first_release_wsl2()
        } else {
            PlatformReleaseCoordinate::native()
        },
        integration_profile,
        verified_capabilities: capabilities,
    };
    CodexSupportCatalog::from_entries(vec![entry.clone()]).map_err(|error| {
        ValidationError::new(format!("invalid proposed Codex support entry: {error}"))
    })?;
    Ok(entry)
}

/// Serializes one validated entry in its declared canonical field order.
pub fn serialize_support_entry(entry: &CodexSupportEntry) -> ValidationResult<Vec<u8>> {
    CodexSupportCatalog::from_entries(vec![entry.clone()]).map_err(|error| {
        ValidationError::new(format!("invalid proposed Codex support entry: {error}"))
    })?;
    serde_json::to_vec(entry).map_err(ValidationError::from)
}

/// Parses a comma-delimited capability declaration and returns canonical order.
pub fn parse_declared_capabilities(value: &str) -> ValidationResult<Vec<CodexCapability>> {
    if value.is_empty() {
        return Err(ValidationError::new(
            "declared Codex capabilities must not be empty",
        ));
    }
    let mut capabilities = Vec::new();
    for raw in value.split(',') {
        let capability = match raw.trim() {
            "managed_stdio_mcp" => CodexCapability::ManagedStdioMcp,
            "personal_managed_binding" => CodexCapability::PersonalManagedBinding,
            "record_workflow" => CodexCapability::RecordWorkflow,
            "shared_managed_binding" => CodexCapability::SharedManagedBinding,
            unknown => {
                return Err(ValidationError::new(format!(
                    "unknown Codex capability {unknown}"
                )))
            }
        };
        capabilities.push(capability);
    }
    normalize_capabilities(&capabilities)
}

fn normalize_capabilities(
    declared_capabilities: &[CodexCapability],
) -> ValidationResult<Vec<CodexCapability>> {
    let mut capabilities = declared_capabilities.to_vec();
    capabilities.sort();
    if capabilities.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ValidationError::new(
            "declared Codex capabilities must not contain duplicates",
        ));
    }
    Ok(capabilities)
}
