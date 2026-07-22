//! Explicit networked synchronization of pinned MCP specification inputs.

use anyhow::{bail, Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};
use volicord_mcp_protocol::ProtocolRegistry;

use super::{
    manifest::{read_manifest, render_manifest, MANIFEST_NAME},
    report::McpSpecSyncReport,
    validation::{
        check_mcp_spec_fixture, checked_relative_path, require_repository_root, sha256,
        validate_manifest_metadata,
    },
    FIXTURE_PATH,
};

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
