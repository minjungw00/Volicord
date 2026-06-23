use std::{
    fs,
    path::{Component, Path},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductPathError {
    Invalid,
    LocalAccess,
}

pub(crate) fn normalize_product_paths(
    repo_root: &Path,
    raw_paths: &[String],
) -> Result<Vec<String>, ProductPathError> {
    let canonical_repo_root =
        fs::canonicalize(repo_root).map_err(|_| ProductPathError::LocalAccess)?;
    raw_paths
        .iter()
        .map(|path| normalize_product_path(repo_root, &canonical_repo_root, path))
        .collect()
}

fn normalize_product_path(
    repo_root: &Path,
    canonical_repo_root: &Path,
    raw_path: &str,
) -> Result<String, ProductPathError> {
    if raw_path.trim().is_empty() || raw_path.contains('\\') {
        return Err(ProductPathError::Invalid);
    }
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return Err(ProductPathError::Invalid);
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(ProductPathError::Invalid);
                }
            }
            Component::Normal(value) => {
                let value = value.to_str().ok_or(ProductPathError::Invalid)?;
                if value.is_empty() {
                    return Err(ProductPathError::Invalid);
                }
                parts.push(value.to_owned());
            }
            Component::RootDir | Component::Prefix(_) => return Err(ProductPathError::Invalid),
        }
    }
    if parts.is_empty() {
        return Err(ProductPathError::Invalid);
    }

    let normalized = parts.join("/");
    ensure_product_path_does_not_escape(repo_root, canonical_repo_root, &normalized)?;
    Ok(normalized)
}

fn ensure_product_path_does_not_escape(
    repo_root: &Path,
    canonical_repo_root: &Path,
    normalized_path: &str,
) -> Result<(), ProductPathError> {
    let mut candidate = repo_root.join(normalized_path);
    while !candidate.exists() {
        if !candidate.pop() {
            return Err(ProductPathError::LocalAccess);
        }
    }
    let canonical_candidate =
        fs::canonicalize(candidate).map_err(|_| ProductPathError::LocalAccess)?;
    if canonical_candidate.starts_with(canonical_repo_root) {
        Ok(())
    } else {
        Err(ProductPathError::LocalAccess)
    }
}

pub(crate) fn path_is_within(path: &str, scope: &str) -> bool {
    path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}

pub(crate) fn paths_are_authorized(observed_paths: &[String], authorized_paths: &[String]) -> bool {
    !observed_paths.is_empty()
        && !authorized_paths.is_empty()
        && observed_paths.iter().all(|path| {
            authorized_paths
                .iter()
                .any(|authorized| path_is_within(path, authorized))
        })
}
