use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, read_managed_text, GeneratedFilePlan},
        GuardIntegrationError,
    },
    host_integration::HostIntegrationFileKind,
};

pub(crate) const GIT_EXCLUDE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES";
pub(crate) const GIT_EXCLUDE_END_MARKER: &str = "# END VOLICORD MANAGED LOCAL EXCLUDES";

const MAX_GIT_CONTROL_FILE_BYTES: usize = 4096;

const PERSONAL_LOCAL_PATHS: &[&str] = &[
    "/.volicord/",
    "/.codex/hooks/volicord-dispatch.sh",
    "/.codex/hooks/volicord-session-start.sh",
    "/.codex/hooks/volicord-pre-tool.sh",
    "/.codex/hooks/volicord-post-tool.sh",
    "/.codex/hooks/volicord-prompt-capture.sh",
    "/.codex/hooks/volicord-stop.sh",
    "/.codex/hooks.json",
    "/.codex/rules/volicord.rules",
    "/.claude/hooks/volicord-session-start.sh",
    "/.claude/hooks/volicord-pre-tool.sh",
    "/.claude/hooks/volicord-post-tool.sh",
    "/.claude/hooks/volicord-prompt-capture.sh",
    "/.claude/hooks/volicord-stop.sh",
    "/.claude/settings.local.json",
    "/.claude/rules/volicord.md",
];

pub(crate) fn plan_personal_git_excludes(
    repo_root: &Path,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let target = resolve_git_exclude_target(repo_root)?;
    plan_managed_block_file(
        HostIntegrationFileKind::GitInfoExclude,
        &target.anchor_root,
        &target.exclude_path,
        &personal_exclude_block(),
        GIT_EXCLUDE_START_MARKER,
        GIT_EXCLUDE_END_MARKER,
        false,
    )
}

pub(crate) fn personal_local_paths() -> &'static [&'static str] {
    PERSONAL_LOCAL_PATHS
}

pub(crate) fn personal_git_exclude_path(
    repo_root: &Path,
) -> Result<PathBuf, GuardIntegrationError> {
    Ok(resolve_git_exclude_target(repo_root)?.exclude_path)
}

fn personal_exclude_block() -> String {
    let mut block = String::from(GIT_EXCLUDE_START_MARKER);
    block.push('\n');
    block.push_str("# Repository-local files created only for a personal Volicord integration.\n");
    for path in PERSONAL_LOCAL_PATHS {
        block.push_str(path);
        block.push('\n');
    }
    block.push_str(GIT_EXCLUDE_END_MARKER);
    block.push('\n');
    block
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitExcludeTarget {
    anchor_root: PathBuf,
    exclude_path: PathBuf,
}

fn resolve_git_exclude_target(repo_root: &Path) -> Result<GitExcludeTarget, GuardIntegrationError> {
    let marker = repo_root.join(".git");
    let metadata = fs::symlink_metadata(&marker).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to inspect Git repository marker {}: {error}",
            marker.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(unsafe_git_path(
            &marker,
            "the .git marker is a symbolic link",
        ));
    }
    if metadata.is_dir() {
        return Ok(GitExcludeTarget {
            anchor_root: repo_root.to_path_buf(),
            exclude_path: marker.join("info").join("exclude"),
        });
    }
    if !metadata.is_file() {
        return Err(unsafe_git_path(
            &marker,
            "the .git marker is neither a directory nor a regular gitdir file",
        ));
    }

    let marker_text = read_control_file(repo_root, &marker, ".git")?;
    let git_dir_text = marker_text.strip_prefix("gitdir: ").ok_or_else(|| {
        unsafe_git_path(
            &marker,
            "the .git file does not contain one gitdir declaration",
        )
    })?;
    let git_dir = resolve_control_path(repo_root, git_dir_text, &marker)?;
    ensure_safe_directory(&git_dir)?;

    let commondir_path = git_dir.join("commondir");
    let common_dir = match read_managed_text(&git_dir, &commondir_path)? {
        Some(text) => {
            let value = parse_one_line_control_value(&text, &commondir_path)?;
            let path = resolve_control_path(&git_dir, value, &commondir_path)?;
            ensure_safe_directory(&path)?;
            path
        }
        None => git_dir,
    };
    Ok(GitExcludeTarget {
        exclude_path: common_dir.join("info").join("exclude"),
        anchor_root: common_dir,
    })
}

fn read_control_file(
    anchor_root: &Path,
    path: &Path,
    label: &str,
) -> Result<String, GuardIntegrationError> {
    let text = read_managed_text(anchor_root, path)?
        .ok_or_else(|| unsafe_git_path(path, &format!("the {label} control file is missing")))?;
    let value = parse_one_line_control_value(&text, path)?;
    Ok(value.to_owned())
}

fn parse_one_line_control_value<'a>(
    text: &'a str,
    path: &Path,
) -> Result<&'a str, GuardIntegrationError> {
    if text.len() > MAX_GIT_CONTROL_FILE_BYTES || text.contains('\0') {
        return Err(unsafe_git_path(
            path,
            "the Git control file is too large or contains NUL",
        ));
    }
    let value = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if value.is_empty() || value.contains(['\n', '\r']) {
        return Err(unsafe_git_path(
            path,
            "the Git control file must contain one non-empty line",
        ));
    }
    Ok(value)
}

fn resolve_control_path(
    base: &Path,
    value: &str,
    control_file: &Path,
) -> Result<PathBuf, GuardIntegrationError> {
    let path = Path::new(value);
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    normalize_absolute_path(&joined).map_err(|detail| unsafe_git_path(control_file, detail))
}

fn normalize_absolute_path(path: &Path) -> Result<PathBuf, &'static str> {
    if !path.is_absolute() {
        return Err("a resolved Git directory path is not absolute");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err("a Git directory path escapes its filesystem root");
                }
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    Ok(normalized)
}

fn ensure_safe_directory(path: &Path) -> Result<(), GuardIntegrationError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to inspect resolved Git directory {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(unsafe_git_path(
            path,
            "the resolved Git path is not a regular directory",
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to canonicalize resolved Git directory {}: {error}",
            path.display()
        ))
    })?;
    if canonical != path {
        return Err(unsafe_git_path(
            path,
            "the resolved Git directory traverses a symbolic link or non-canonical component",
        ));
    }
    Ok(())
}

fn unsafe_git_path(path: &Path, detail: &str) -> GuardIntegrationError {
    GuardIntegrationError::runtime(format!(
        "unsafe Git metadata path at {}: {detail}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let path = std::env::temp_dir().join(format!(
                "volicord-git-exclude-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path)?;
            Ok(Self(fs::canonicalize(path)?))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn normal_repository_targets_repository_git_exclude() -> Result<(), Box<dyn std::error::Error>>
    {
        let repo = TestDirectory::new("normal")?;
        fs::create_dir(repo.path().join(".git"))?;

        let target = resolve_git_exclude_target(repo.path())?;

        assert_eq!(target.anchor_root, repo.path());
        assert_eq!(target.exclude_path, repo.path().join(".git/info/exclude"));
        Ok(())
    }

    #[test]
    fn linked_worktree_targets_common_git_exclude() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TestDirectory::new("worktree")?;
        let repo = fixture.path().join("repo");
        let common = fixture.path().join("main/.git");
        let git_dir = common.join("worktrees/repo");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&git_dir)?;
        fs::write(
            repo.join(".git"),
            format!("gitdir: {}\n", git_dir.display()),
        )?;
        fs::write(git_dir.join("commondir"), "../..\n")?;

        let target = resolve_git_exclude_target(&repo)?;

        assert_eq!(target.anchor_root, common);
        assert_eq!(target.exclude_path, common.join("info/exclude"));
        Ok(())
    }

    #[test]
    fn personal_block_uses_only_dedicated_volicord_paths() {
        let block = personal_exclude_block();
        assert!(block.contains("/.volicord/\n"));
        assert!(block.contains("/.codex/hooks/volicord-pre-tool.sh\n"));
        assert!(block.contains("/.claude/rules/volicord.md\n"));
        assert!(block.contains("/.codex/hooks.json\n"));
        assert!(block.contains("/.claude/settings.local.json\n"));
        assert!(!block.contains("/.codex/\n"));
        assert!(!block.contains("/.claude/\n"));
        assert!(!block.contains("/.mcp.json\n"));
        assert!(!block.contains("/.gitignore\n"));
    }
}
