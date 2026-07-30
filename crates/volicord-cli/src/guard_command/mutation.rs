use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PathAssessment {
    pub(super) raw: String,
    pub(super) normalized: Option<String>,
    pub(super) inside_repo: bool,
}

pub(super) fn assess_decoded_paths(
    repo_root: &Path,
    raw_paths: Option<&[String]>,
) -> Vec<PathAssessment> {
    raw_paths
        .into_iter()
        .flatten()
        .map(|raw| assess_path(repo_root, raw))
        .collect()
}

fn assess_path(repo_root: &Path, raw: &str) -> PathAssessment {
    let path = Path::new(raw);
    let (inside_repo, normalized) = if path.is_absolute() {
        match path.strip_prefix(repo_root) {
            Ok(relative) => normalized_relative_path(relative)
                .map(|path| (true, Some(path)))
                .unwrap_or((false, None)),
            Err(_) => (false, None),
        }
    } else {
        normalized_relative_path(path)
            .map(|path| (true, Some(path)))
            .unwrap_or((false, None))
    };
    PathAssessment {
        raw: raw.to_owned(),
        normalized,
        inside_repo,
    }
}

fn normalized_relative_path(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_targets_distinguish_inside_external_and_malformed_paths() {
        let paths = vec![
            "src/lib.rs".to_owned(),
            "/repo/src/main.rs".to_owned(),
            "/outside/file.rs".to_owned(),
            "../escape.rs".to_owned(),
        ];
        let assessed = assess_decoded_paths(Path::new("/repo"), Some(&paths));
        assert_eq!(assessed[0].normalized.as_deref(), Some("src/lib.rs"));
        assert_eq!(assessed[1].normalized.as_deref(), Some("src/main.rs"));
        assert!(assessed[0].inside_repo && assessed[1].inside_repo);
        assert!(!assessed[2].inside_repo && assessed[2].normalized.is_none());
        assert!(!assessed[3].inside_repo && assessed[3].normalized.is_none());
    }
}
