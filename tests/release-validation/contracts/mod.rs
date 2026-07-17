//! Release-test routes to distinct runtime policy and external evidence contracts.

pub use volicord_types::{
    compute_codex_release_evidence_digest, embedded_codex_support_catalog,
    load_codex_release_evidence_manifest, load_codex_support_catalog,
    parse_codex_release_evidence_manifest, parse_codex_support_catalog,
    parse_test_only_codex_descriptor, serialize_codex_release_evidence_manifest,
    serialize_codex_support_catalog, CodexReleaseEvidenceError, CodexReleaseEvidenceManifest,
    CodexReleasePlatformStatus, CodexSupportCatalog, CodexSupportCatalogError,
    UnsupportedHostArtifact, CODEX_SUPPORT_CATALOG_PATH, UNSUPPORTED_HOST_ARTIFACT_REASON,
};

/// Repository-relative path of the external checked-in release-evidence manifest.
pub const CODEX_RELEASE_EVIDENCE_MANIFEST_PATH: &str =
    "tests/release-validation/contracts/codex-release-evidence-manifest.json";
