use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item};

pub(crate) fn read_toml_document(path: &Path, label: &str) -> Result<DocumentMut> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read {label} at {}", path.display()))?;
    contents
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {label} at {}", path.display()))
}

pub(crate) fn workspace_package_version(manifest: &DocumentMut) -> Option<&str> {
    workspace_package_string(manifest, "version")
}

pub(crate) fn workspace_rust_version(manifest: &DocumentMut) -> Option<&str> {
    workspace_package_string(manifest, "rust-version")
}

fn workspace_package_string<'a>(manifest: &'a DocumentMut, field: &str) -> Option<&'a str> {
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get(field))
        .and_then(Item::as_str)
}

pub(crate) fn dependency_names(manifest: &DocumentMut) -> BTreeSet<String> {
    ["dependencies", "dev-dependencies", "build-dependencies"]
        .into_iter()
        .filter_map(|section| manifest.get(section).and_then(Item::as_table_like))
        .flat_map(|table| table.iter().map(|(name, _)| name.to_owned()))
        .collect()
}
