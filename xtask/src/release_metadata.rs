use crate::repository::{normalize_existing_root, repo_relative};
use crate::workspace_manifests::{read_toml_document, workspace_package_version};
use anyhow::Result;
use std::path::Path;
use toml_edit::Item;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReleaseVersionReport {
    workspace_version: String,
    member_package_count: usize,
    checked_tag: Option<String>,
}

impl ReleaseVersionReport {
    pub fn workspace_version(&self) -> &str {
        &self.workspace_version
    }

    pub fn member_package_count(&self) -> usize {
        self.member_package_count
    }

    pub fn checked_tag(&self) -> Option<&str> {
        self.checked_tag.as_deref()
    }
}

pub fn run_release_version_check(
    root: &Path,
    release_tag: Option<&str>,
) -> Result<ReleaseVersionReport> {
    let root = normalize_existing_root(root)?;
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_toml_document(&root_manifest_path, "root Cargo.toml")?;
    let Some(workspace_version) = workspace_package_version(&root_manifest) else {
        anyhow::bail!(
            "release-version-check requires [workspace.package].version in the root Cargo.toml"
        );
    };
    let workspace_version = workspace_version.to_owned();
    let Some(members) = root_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(Item::as_array)
    else {
        anyhow::bail!(
            "release-version-check requires an explicit [workspace].members array in the root Cargo.toml"
        );
    };

    let mut member_package_count = 0usize;
    for member in members.iter() {
        let Some(member) = member.as_str() else {
            anyhow::bail!("workspace member entries must be strings");
        };
        let manifest_path = root.join(member).join("Cargo.toml");
        let relative_manifest = repo_relative(&root, &manifest_path);
        let manifest = read_toml_document(&manifest_path, &relative_manifest)?;
        let package_name = manifest
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(Item::as_str)
            .unwrap_or(member);
        let inherits_workspace_version = manifest
            .get("package")
            .and_then(|package| package.get("version"))
            .and_then(|version| version.get("workspace"))
            .and_then(Item::as_bool);
        if inherits_workspace_version != Some(true) {
            anyhow::bail!(
                "workspace package {package_name} in {relative_manifest} must set version.workspace = true"
            );
        }
        member_package_count += 1;
    }
    if member_package_count == 0 {
        anyhow::bail!("release-version-check requires at least one workspace member package");
    }

    if let Some(release_tag) = release_tag {
        let expected_tag = format!("v{workspace_version}");
        if release_tag != expected_tag {
            anyhow::bail!(
                "release tag {release_tag:?} does not match workspace package version; expected {expected_tag:?}"
            );
        }
    }

    Ok(ReleaseVersionReport {
        workspace_version,
        member_package_count,
        checked_tag: release_tag.map(str::to_owned),
    })
}
