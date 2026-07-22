use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use volicord_mcp_protocol::ProtocolRegistry;

const FIXTURE_PATH: &str = "tests/conformance/mcp-spec";
const MANIFEST_NAME: &str = "manifest.toml";
const MANIFEST_FORMAT_VERSION: u32 = 3;
const OFFICIAL_REPOSITORY: &str =
    "https://github.com/modelcontextprotocol/modelcontextprotocol.git";
const REQUIRED_DRAFT_PROTOCOLS: &[&str] = &["2026-07-28"];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    upstream_repository: String,
    license: Vec<LicenseArtifact>,
    revision: Vec<Revision>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LicenseArtifact {
    id: String,
    spdx_expression: String,
    attribution: String,
    upstream_release: String,
    upstream_commit: String,
    upstream_path: String,
    local_path: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    protocol_version: String,
    release_status: ReleaseStatus,
    handshake_family: HandshakeFamily,
    upstream_release: String,
    upstream_commit: String,
    license_id: String,
    production_supported: bool,
    pre_release_only: bool,
    artifact: Vec<SchemaArtifact>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ReleaseStatus {
    Released,
    ReleaseCandidate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum HandshakeFamily {
    InitializationBased,
    PerRequestMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SchemaArtifact {
    upstream_path: String,
    local_path: String,
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpSpecCheckReport {
    pinned_revision_count: usize,
    production_supported_count: usize,
    pre_release_only_count: usize,
}

impl McpSpecCheckReport {
    pub fn pinned_revision_count(self) -> usize {
        self.pinned_revision_count
    }

    pub fn production_supported_count(self) -> usize {
        self.production_supported_count
    }

    pub fn pre_release_only_count(self) -> usize {
        self.pre_release_only_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpSpecSyncReport {
    revision_count: usize,
    artifact_count: usize,
}

impl McpSpecSyncReport {
    pub fn revision_count(self) -> usize {
        self.revision_count
    }

    pub fn artifact_count(self) -> usize {
        self.artifact_count
    }
}

pub fn run_mcp_spec_check(root: &Path) -> Result<McpSpecCheckReport> {
    require_repository_root(root)?;
    check_mcp_spec_fixture(&root.join(FIXTURE_PATH))
}

pub fn check_mcp_spec_fixture(fixture_root: &Path) -> Result<McpSpecCheckReport> {
    let production_profiles = ProtocolRegistry::production()
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str())
        .collect::<Vec<_>>();
    check_mcp_spec_fixture_with_production_profiles(fixture_root, &production_profiles)
}

/// Checks a fixture against explicit production profiles. The ordinary command
/// supplies the compiled protocol registry; an explicit set keeps both parity
/// failure directions durably testable without another production list.
pub fn check_mcp_spec_fixture_with_production_profiles(
    fixture_root: &Path,
    production_profiles: &[&str],
) -> Result<McpSpecCheckReport> {
    let manifest = read_manifest(fixture_root)?;
    validate_manifest_metadata(&manifest, production_profiles)?;
    validate_pinned_files(fixture_root, &manifest)?;

    Ok(McpSpecCheckReport {
        pinned_revision_count: manifest.revision.len(),
        production_supported_count: manifest
            .revision
            .iter()
            .filter(|revision| revision.production_supported)
            .count(),
        pre_release_only_count: manifest
            .revision
            .iter()
            .filter(|revision| revision.pre_release_only)
            .count(),
    })
}

pub fn run_mcp_spec_sync(root: &Path) -> Result<McpSpecSyncReport> {
    require_repository_root(root)?;
    let fixture_root = root.join(FIXTURE_PATH);
    let mut manifest = read_manifest(&fixture_root)?;
    let production_profiles = ProtocolRegistry::production()
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str())
        .collect::<Vec<_>>();
    validate_manifest_metadata(&manifest, &production_profiles)?;

    let fixture_parent = fixture_root
        .parent()
        .context("MCP specification fixture path has no parent")?;
    let work = tempfile::Builder::new()
        .prefix(".mcp-spec-sync-")
        .tempdir_in(fixture_parent)
        .with_context(|| {
            format!(
                "failed to create MCP specification sync directory under {}",
                fixture_parent.display()
            )
        })?;
    let repository = work.path().join("upstream");
    fs::create_dir(&repository)
        .with_context(|| format!("failed to create {}", repository.display()))?;
    run_git(&repository, &["init", "--quiet"])?;
    run_git(
        &repository,
        &["remote", "add", "origin", &manifest.upstream_repository],
    )?;

    let candidate = work.path().join("candidate");
    fs::create_dir(&candidate)
        .with_context(|| format!("failed to create {}", candidate.display()))?;
    let mut fetched = BTreeSet::new();
    let mut blobs = BTreeMap::new();

    for license in &mut manifest.license {
        let bytes = download_blob(
            &repository,
            &mut fetched,
            &mut blobs,
            &license.upstream_release,
            &license.upstream_commit,
            &license.upstream_path,
        )?;
        license.sha256 = sha256(&bytes);
        write_candidate_file(&candidate, &license.local_path, &bytes)?;
    }

    for revision in &mut manifest.revision {
        for artifact in &mut revision.artifact {
            let bytes = download_blob(
                &repository,
                &mut fetched,
                &mut blobs,
                &revision.upstream_release,
                &revision.upstream_commit,
                &artifact.upstream_path,
            )?;
            artifact.sha256 = sha256(&bytes);
            write_candidate_file(&candidate, &artifact.local_path, &bytes)?;
        }
    }

    manifest
        .license
        .sort_by(|left, right| left.id.cmp(&right.id));
    manifest
        .revision
        .sort_by(|left, right| left.protocol_version.cmp(&right.protocol_version));
    for revision in &mut manifest.revision {
        revision
            .artifact
            .sort_by(|left, right| left.local_path.cmp(&right.local_path));
    }

    let rendered = render_manifest(&manifest)?;
    fs::write(candidate.join(MANIFEST_NAME), rendered)
        .context("failed to write the candidate MCP specification manifest")?;

    let checked = check_mcp_spec_fixture(&candidate)
        .context("downloaded MCP specification candidate failed offline validation")?;
    replace_fixture_directory(&fixture_root, &candidate, work.path())?;

    Ok(McpSpecSyncReport {
        revision_count: checked.pinned_revision_count,
        artifact_count: manifest.license.len()
            + manifest
                .revision
                .iter()
                .map(|revision| revision.artifact.len())
                .sum::<usize>(),
    })
}

fn read_manifest(fixture_root: &Path) -> Result<Manifest> {
    let manifest_path = fixture_root.join(MANIFEST_NAME);
    let contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    toml_edit::de::from_str(&contents)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))
}

fn render_manifest(manifest: &Manifest) -> Result<String> {
    let mut rendered = toml_edit::ser::to_string_pretty(manifest)
        .context("failed to render the MCP specification manifest deterministically")?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

fn validate_manifest_metadata(manifest: &Manifest, production_profiles: &[&str]) -> Result<()> {
    if manifest.format_version != MANIFEST_FORMAT_VERSION {
        bail!(
            "MCP specification manifest format_version must be {MANIFEST_FORMAT_VERSION}, found {}",
            manifest.format_version,
        );
    }
    if manifest.upstream_repository != OFFICIAL_REPOSITORY {
        bail!("MCP specification upstream_repository must be {OFFICIAL_REPOSITORY}");
    }
    if manifest.license.is_empty() {
        bail!("MCP specification manifest must pin at least one license artifact");
    }
    ensure_sorted_unique(
        manifest.license.iter().map(|license| license.id.as_str()),
        "license ids",
    )?;

    let mut license_ids = BTreeSet::new();
    for license in &manifest.license {
        validate_nonempty(&license.id, "license id")?;
        validate_nonempty(&license.spdx_expression, "license SPDX expression")?;
        validate_nonempty(&license.attribution, "license attribution")?;
        validate_source(
            &license.upstream_release,
            &license.upstream_commit,
            &license.upstream_path,
            "license",
        )?;
        checked_relative_path(&license.local_path, "license local_path")?;
        validate_sha256(&license.sha256, "license sha256")?;
        license_ids.insert(license.id.as_str());
    }

    if manifest.revision.is_empty() {
        bail!("MCP specification manifest must pin at least one revision");
    }
    let mut protocol_versions = BTreeSet::new();
    for revision in &manifest.revision {
        if !protocol_versions.insert(revision.protocol_version.as_str()) {
            bail!(
                "duplicate MCP protocol string {}",
                revision.protocol_version
            );
        }
    }
    ensure_sorted_unique(
        manifest
            .revision
            .iter()
            .map(|revision| revision.protocol_version.as_str()),
        "protocol versions",
    )?;

    for required in REQUIRED_DRAFT_PROTOCOLS {
        let revision = manifest
            .revision
            .iter()
            .find(|revision| revision.protocol_version == *required)
            .with_context(|| format!("required draft MCP revision {required} is missing"))?;
        if revision.release_status == ReleaseStatus::Released
            || revision.production_supported
            || !revision.pre_release_only
        {
            bail!(
                "draft MCP revision {required} must remain pre-release-only and outside production support"
            );
        }
    }

    for revision in &manifest.revision {
        validate_nonempty(&revision.protocol_version, "protocol_version")?;
        validate_source(
            &revision.upstream_release,
            &revision.upstream_commit,
            revision
                .artifact
                .first()
                .map(|artifact| artifact.upstream_path.as_str())
                .unwrap_or(""),
            &format!("revision {}", revision.protocol_version),
        )?;
        if !license_ids.contains(revision.license_id.as_str()) {
            bail!(
                "revision {} references unknown license_id {}",
                revision.protocol_version,
                revision.license_id
            );
        }
        match revision.release_status {
            ReleaseStatus::Released if revision.pre_release_only => bail!(
                "released MCP revision {} cannot be pre-release-only",
                revision.protocol_version
            ),
            ReleaseStatus::ReleaseCandidate if !revision.pre_release_only => bail!(
                "pre-release MCP revision {} must be marked pre-release-only",
                revision.protocol_version
            ),
            _ => {}
        }
        if revision.production_supported && revision.release_status != ReleaseStatus::Released {
            bail!(
                "pre-release MCP revision {} cannot be production-supported",
                revision.protocol_version
            );
        }
        if revision.production_supported && revision.pre_release_only {
            bail!(
                "pre-release-only MCP revision {} cannot be production-supported",
                revision.protocol_version
            );
        }
        if revision.artifact.is_empty() {
            bail!(
                "MCP revision {} has no pinned schema artifacts",
                revision.protocol_version
            );
        }
        ensure_sorted_unique(
            revision
                .artifact
                .iter()
                .map(|artifact| artifact.local_path.as_str()),
            &format!("artifact paths for {}", revision.protocol_version),
        )?;
        for artifact in &revision.artifact {
            checked_relative_path(&artifact.upstream_path, "schema upstream_path")?;
            checked_relative_path(&artifact.local_path, "schema local_path")?;
            validate_sha256(&artifact.sha256, "schema sha256")?;
        }
    }

    validate_revision_set_parity(manifest, production_profiles)?;

    Ok(())
}

fn validate_revision_set_parity(manifest: &Manifest, production_profiles: &[&str]) -> Result<()> {
    let manifest_revisions = manifest
        .revision
        .iter()
        .map(|revision| revision.protocol_version.as_str())
        .collect::<BTreeSet<_>>();
    let manifest_production = manifest
        .revision
        .iter()
        .filter(|revision| revision.production_supported)
        .map(|revision| revision.protocol_version.as_str())
        .collect::<BTreeSet<_>>();
    let production_profiles = checked_revision_set(production_profiles, "production profiles")?;

    for revision in &manifest_production {
        if !production_profiles.contains(revision) {
            bail!(
                "production-supported released MCP revision {revision} has no production protocol profile"
            );
        }
    }
    for revision in &production_profiles {
        if !manifest_revisions.contains(revision) {
            bail!("production protocol profile {revision} is missing from the MCP specification manifest");
        }
        if !manifest_production.contains(revision) {
            bail!(
                "production protocol profile {revision} is not marked production-supported in the MCP specification manifest"
            );
        }
    }
    if manifest_production != production_profiles {
        bail!(
            "released MCP production support and production protocol profiles must have exact revision-set parity"
        );
    }
    Ok(())
}

fn checked_revision_set<'a>(values: &'a [&'a str], label: &str) -> Result<BTreeSet<&'a str>> {
    let mut revisions = BTreeSet::new();
    for revision in values {
        validate_nonempty(revision, label)?;
        if !revisions.insert(*revision) {
            bail!("{label} contain duplicate MCP revision {revision}");
        }
    }
    Ok(revisions)
}

fn validate_pinned_files(fixture_root: &Path, manifest: &Manifest) -> Result<()> {
    for license in &manifest.license {
        let bytes = read_pinned_file(fixture_root, &license.local_path, "license artifact")?;
        verify_checksum(&license.local_path, &license.sha256, &bytes)?;
        let text = std::str::from_utf8(&bytes)
            .with_context(|| format!("license artifact {} is not UTF-8", license.local_path))?;
        if !text.contains(&license.attribution) {
            bail!(
                "license artifact {} does not contain its recorded attribution",
                license.local_path
            );
        }
        if !text.contains("License") {
            bail!(
                "license artifact {} has no license text",
                license.local_path
            );
        }
    }

    for revision in &manifest.revision {
        for artifact in &revision.artifact {
            let bytes = read_pinned_file(fixture_root, &artifact.local_path, "schema artifact")?;
            verify_checksum(&artifact.local_path, &artifact.sha256, &bytes)?;
            validate_schema(&artifact.local_path, &bytes, revision.handshake_family)?;
        }
    }
    Ok(())
}

fn validate_schema(local_path: &str, bytes: &[u8], family: HandshakeFamily) -> Result<()> {
    let schema: Value = serde_json::from_slice(bytes)
        .with_context(|| format!("schema artifact {local_path} is not valid JSON"))?;
    let definitions = schema.get("definitions").or_else(|| schema.get("$defs"));
    match family {
        HandshakeFamily::InitializationBased => {
            let definitions = definitions
                .and_then(Value::as_object)
                .with_context(|| format!("schema artifact {local_path} has no definitions"))?;
            if !definitions.contains_key("InitializeRequest")
                || !definitions.contains_key("InitializeResult")
            {
                bail!(
                    "initialization-based schema artifact {local_path} lacks InitializeRequest or InitializeResult"
                );
            }
        }
        HandshakeFamily::PerRequestMetadata => {
            let protocol_version = definitions
                .and_then(|value| value.get("RequestMetaObject"))
                .and_then(|value| value.get("properties"))
                .and_then(|value| value.get("io.modelcontextprotocol/protocolVersion"));
            if protocol_version.is_none() {
                bail!(
                    "per-request-metadata schema artifact {local_path} lacks the protocol version request metadata field"
                );
            }
        }
    }
    Ok(())
}

fn validate_source(release: &str, commit: &str, upstream_path: &str, label: &str) -> Result<()> {
    validate_nonempty(release, &format!("{label} upstream_release"))?;
    if matches!(release, "HEAD" | "main" | "master")
        || !release
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
    {
        bail!("{label} has invalid or mutable upstream_release {release:?}");
    }
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must use a full lowercase immutable upstream commit");
    }
    checked_relative_path(upstream_path, &format!("{label} upstream_path"))?;
    Ok(())
}

fn ensure_sorted_unique<'a>(values: impl IntoIterator<Item = &'a str>, label: &str) -> Result<()> {
    let values: Vec<_> = values.into_iter().collect();
    for pair in values.windows(2) {
        if pair[0] >= pair[1] {
            bail!("{label} must be unique and sorted in ascending byte order");
        }
    }
    Ok(())
}

fn validate_nonempty(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(())
}

fn validate_sha256(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be 64 lowercase hexadecimal characters");
    }
    Ok(())
}

fn checked_relative_path(value: &str, label: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if value.is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("{label} must be a non-empty normalized relative path");
    }
    Ok(path.to_path_buf())
}

fn read_pinned_file(fixture_root: &Path, relative: &str, label: &str) -> Result<Vec<u8>> {
    let relative = checked_relative_path(relative, label)?;
    let path = fixture_root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("missing {label} {}", path.display()))?;
    if !metadata.file_type().is_file() {
        bail!("{label} {} must be a regular file", path.display());
    }
    fs::read(&path).with_context(|| format!("failed to read {label} {}", path.display()))
}

fn verify_checksum(local_path: &str, expected: &str, bytes: &[u8]) -> Result<()> {
    let actual = sha256(bytes);
    if actual != expected {
        bail!("checksum mismatch for {local_path}: expected {expected}, found {actual}");
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn require_repository_root(root: &Path) -> Result<()> {
    if !root.join("Cargo.toml").is_file() || !root.join("xtask/Cargo.toml").is_file() {
        bail!("MCP specification tooling must run from the repository root");
    }
    Ok(())
}

fn download_blob(
    repository: &Path,
    fetched: &mut BTreeSet<(String, String)>,
    blobs: &mut BTreeMap<(String, String), Vec<u8>>,
    release: &str,
    commit: &str,
    upstream_path: &str,
) -> Result<Vec<u8>> {
    let source = (release.to_owned(), commit.to_owned());
    if fetched.insert(source) {
        let tag = format!("refs/tags/{release}");
        run_git(
            repository,
            &["fetch", "--quiet", "--depth", "1", "origin", &tag],
        )?;
        let resolved = String::from_utf8(run_git(repository, &["rev-parse", "FETCH_HEAD^{}"])?)
            .context("git returned a non-UTF-8 commit id")?;
        if resolved.trim() != commit {
            bail!(
                "upstream release {release} resolved to {}, expected immutable commit {commit}",
                resolved.trim()
            );
        }
    }

    let key = (commit.to_owned(), upstream_path.to_owned());
    if let Some(bytes) = blobs.get(&key) {
        return Ok(bytes.clone());
    }
    let object = format!("{commit}:{upstream_path}");
    let bytes = run_git(repository, &["show", &object])?;
    blobs.insert(key, bytes.clone());
    Ok(bytes)
}

fn run_git(repository: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repository)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn write_candidate_file(candidate: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    let relative = checked_relative_path(relative, "candidate artifact path")?;
    let destination = candidate.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&destination, bytes)
        .with_context(|| format!("failed to write {}", destination.display()))
}

fn replace_fixture_directory(target: &Path, candidate: &Path, work: &Path) -> Result<()> {
    let backup = work.join("previous");
    let had_target = target.exists();
    if had_target {
        fs::rename(target, &backup).with_context(|| {
            format!(
                "failed to move existing MCP specification fixture {} aside",
                target.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(candidate, target) {
        if had_target {
            fs::rename(&backup, target).with_context(|| {
                format!(
                    "failed to restore MCP specification fixture {} after replacement error: {error}",
                    target.display()
                )
            })?;
        }
        return Err(error).with_context(|| {
            format!(
                "failed to replace MCP specification fixture {}",
                target.display()
            )
        });
    }
    Ok(())
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
