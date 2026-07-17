//! Release-test route to the production-owned strict manifest contract.

pub use volicord_types::{
    checked_in_codex_release_manifest as checked_in_manifest,
    compute_codex_release_evidence_digest as compute_evidence_digest,
    load_codex_release_manifest as load_manifest, parse_codex_release_manifest as parse_manifest,
    parse_test_only_codex_descriptor as parse_test_only_descriptor, CodexReleaseManifest,
    CodexReleaseManifestError, PlatformReleaseStatus, UnsupportedHostArtifact,
    CHECKED_IN_CODEX_RELEASE_MANIFEST_PATH, UNSUPPORTED_HOST_ARTIFACT_REASON,
};
