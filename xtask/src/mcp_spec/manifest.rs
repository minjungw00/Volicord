//! Pinned MCP specification manifest representation and deterministic rendering.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub(super) const MANIFEST_NAME: &str = "manifest.toml";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub(super) format_version: u32,
    pub(super) upstream_repository: String,
    pub(super) license: Vec<LicenseArtifact>,
    pub(super) revision: Vec<Revision>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LicenseArtifact {
    pub(super) id: String,
    pub(super) spdx_expression: String,
    pub(super) attribution: String,
    pub(super) upstream_release: String,
    pub(super) upstream_commit: String,
    pub(super) upstream_path: String,
    pub(super) local_path: String,
    pub(super) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Revision {
    pub(super) protocol_version: String,
    pub(super) release_status: ReleaseStatus,
    pub(super) handshake_family: HandshakeFamily,
    pub(super) upstream_release: String,
    pub(super) upstream_commit: String,
    pub(super) license_id: String,
    pub(super) production_supported: bool,
    pub(super) pre_release_only: bool,
    pub(super) artifact: Vec<SchemaArtifact>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ReleaseStatus {
    Released,
    ReleaseCandidate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum HandshakeFamily {
    InitializationBased,
    PerRequestMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SchemaArtifact {
    pub(super) upstream_path: String,
    pub(super) local_path: String,
    pub(super) sha256: String,
}

pub(super) fn read_manifest(fixture_root: &Path) -> Result<Manifest> {
    let manifest_path = fixture_root.join(MANIFEST_NAME);
    let contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    toml_edit::de::from_str(&contents)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))
}

pub(super) fn render_manifest(manifest: &Manifest) -> Result<String> {
    let mut rendered = toml_edit::ser::to_string_pretty(manifest)
        .context("failed to render the MCP specification manifest deterministically")?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_MANIFEST: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/conformance/mcp-spec/manifest.toml"
    ));

    #[test]
    fn sync_rendering_preserves_reviewed_production_support_metadata() {
        let manifest: Manifest =
            toml_edit::de::from_str(CURRENT_MANIFEST).expect("current manifest should parse");
        let before = manifest
            .revision
            .iter()
            .map(|revision| {
                (
                    revision.protocol_version.clone(),
                    revision.production_supported,
                    revision.pre_release_only,
                )
            })
            .collect::<Vec<_>>();

        let rendered = render_manifest(&manifest).expect("sync manifest rendering");
        let reparsed: Manifest =
            toml_edit::de::from_str(&rendered).expect("rendered manifest should parse");
        let after = reparsed
            .revision
            .iter()
            .map(|revision| {
                (
                    revision.protocol_version.clone(),
                    revision.production_supported,
                    revision.pre_release_only,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(after, before);
        assert_eq!(
            render_manifest(&reparsed).expect("second rendering"),
            rendered
        );
    }
}
