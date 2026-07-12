use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, read_managed_text, GeneratedFilePlan},
        GuardIntegrationError,
    },
    host_integration::{ConnectionIntent, HostIntegrationFileKind},
};
use volicord_types::IntegrationProfile;

pub(crate) const GIT_EXCLUDE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES";
pub(crate) const GIT_EXCLUDE_END_MARKER: &str = "# END VOLICORD MANAGED LOCAL EXCLUDES";

const MAX_GIT_CONTROL_FILE_BYTES: usize = 4096;

const ALWAYS_LOCAL_PATHS: &[&str] = &[
    "/.volicord/",
    "/.codex/hooks/volicord-dispatch.sh",
    "/.codex/hooks/volicord-session-start.sh",
    "/.codex/hooks/volicord-pre-tool.sh",
    "/.codex/hooks/volicord-post-tool.sh",
    "/.codex/hooks/volicord-prompt-capture.sh",
    "/.codex/hooks/volicord-stop.sh",
    "/.claude/hooks/volicord-session-start.sh",
    "/.claude/hooks/volicord-pre-tool.sh",
    "/.claude/hooks/volicord-post-tool.sh",
    "/.claude/hooks/volicord-prompt-capture.sh",
    "/.claude/hooks/volicord-stop.sh",
];

const PERSONAL_ONLY_PATHS: &[&str] = &[
    "/.codex/hooks.json",
    "/.codex/rules/volicord.rules",
    "/.claude/settings.local.json",
    "/.claude/rules/volicord.md",
];

pub(crate) fn plan_git_excludes(
    repo_root: &Path,
    connection_intent: ConnectionIntent,
    profile: IntegrationProfile,
) -> Result<Option<GeneratedFilePlan>, GuardIntegrationError> {
    if connection_intent != ConnectionIntent::Personal
        && matches!(
            fs::symlink_metadata(repo_root.join(".git")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        )
    {
        return Ok(None);
    }
    let target = resolve_git_exclude_target(repo_root)?;
    if target.is_linked_worktree
        && connection_intent == ConnectionIntent::Personal
        && profile == IntegrationProfile::Detective
    {
        return Err(GuardIntegrationError::runtime(
            "LINKED_WORKTREE_PERSONAL_DETECTIVE_UNSUPPORTED: personal detective init would require worktree-specific local hook paths, but this linked worktree exposes only the common Git info/exclude shared by sibling worktrees. Use --profile record, use --shared for a repository-managed detective integration, or initialize detective in a standalone Git worktree.",
        ));
    }
    let include_personal_paths =
        !target.is_linked_worktree && connection_intent == ConnectionIntent::Personal;
    plan_managed_block_file(
        HostIntegrationFileKind::GitInfoExclude,
        &target.anchor_root,
        &target.exclude_path,
        &exclude_block(include_personal_paths),
        GIT_EXCLUDE_START_MARKER,
        GIT_EXCLUDE_END_MARKER,
        false,
    )
    .map(Some)
}

pub(crate) fn always_local_paths() -> &'static [&'static str] {
    ALWAYS_LOCAL_PATHS
}

pub(crate) fn personal_only_paths() -> &'static [&'static str] {
    PERSONAL_ONLY_PATHS
}

pub(crate) fn git_exclude_path(repo_root: &Path) -> Result<Option<PathBuf>, GuardIntegrationError> {
    if matches!(
        fs::symlink_metadata(repo_root.join(".git")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    ) {
        return Ok(None);
    }
    Ok(Some(resolve_git_exclude_target(repo_root)?.exclude_path))
}

fn exclude_block(include_personal_paths: bool) -> String {
    let mut block = String::from(GIT_EXCLUDE_START_MARKER);
    block.push('\n');
    block.push_str("# Volicord local integration files that must remain outside Git.\n");
    for path in ALWAYS_LOCAL_PATHS {
        block.push_str(path);
        block.push('\n');
    }
    if include_personal_paths {
        block.push_str("# Additional host-local files for a personal Volicord integration.\n");
        for path in PERSONAL_ONLY_PATHS {
            block.push_str(path);
            block.push('\n');
        }
    }
    block.push_str(GIT_EXCLUDE_END_MARKER);
    block.push('\n');
    block
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitExcludeTarget {
    anchor_root: PathBuf,
    exclude_path: PathBuf,
    is_linked_worktree: bool,
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
            is_linked_worktree: false,
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
    let (common_dir, is_linked_worktree) = match read_managed_text(&git_dir, &commondir_path)? {
        Some(text) => {
            let value = parse_one_line_control_value(&text, &commondir_path)?;
            let path = resolve_control_path(&git_dir, value, &commondir_path)?;
            ensure_safe_directory(&path)?;
            (path, true)
        }
        None => (git_dir, false),
    };
    Ok(GitExcludeTarget {
        exclude_path: common_dir.join("info").join("exclude"),
        anchor_root: common_dir,
        is_linked_worktree,
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

    use crate::guard_integration::apply::apply_generated_file;

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
        assert!(!target.is_linked_worktree);
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
        assert!(target.is_linked_worktree);
        Ok(())
    }

    #[test]
    fn personal_block_uses_only_dedicated_volicord_paths() {
        let block = exclude_block(true);
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

    #[test]
    fn shared_block_keeps_only_intent_independent_local_overlay() {
        let block = exclude_block(false);

        assert!(block.contains("/.volicord/\n"));
        assert!(block.contains("/.codex/hooks/volicord-pre-tool.sh\n"));
        assert!(block.contains("/.claude/hooks/volicord-pre-tool.sh\n"));
        assert!(!block.contains("/.codex/hooks.json\n"));
        assert!(!block.contains("/.claude/settings.local.json\n"));
    }

    #[test]
    fn normal_repository_recomputes_managed_paths_across_intent_transitions(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = TestDirectory::new("normal-intent-transition")?;
        fs::create_dir_all(repo.path().join(".git/info"))?;

        let personal = plan_git_excludes(
            repo.path(),
            ConnectionIntent::Personal,
            IntegrationProfile::Record,
        )?
        .expect("normal Git repository should produce an exclude plan");
        apply_generated_file(&personal)?;
        let exclude_path = repo.path().join(".git/info/exclude");
        let personal_text = fs::read_to_string(&exclude_path)?;
        assert!(personal_text.contains("/.volicord/\n"));
        assert!(personal_text.contains("/.codex/hooks.json\n"));
        assert!(personal_text.contains("/.claude/settings.local.json\n"));

        let shared = plan_git_excludes(
            repo.path(),
            ConnectionIntent::Shared,
            IntegrationProfile::Record,
        )?
        .expect("normal Git repository should produce an exclude plan");
        apply_generated_file(&shared)?;
        let shared_text = fs::read_to_string(&exclude_path)?;
        assert!(shared_text.contains("/.volicord/\n"));
        assert!(shared_text.contains("/.codex/hooks/volicord-pre-tool.sh\n"));
        assert!(shared_text.contains("/.claude/hooks/volicord-pre-tool.sh\n"));
        assert!(!shared_text.contains("/.codex/hooks.json\n"));
        assert!(!shared_text.contains("/.claude/settings.local.json\n"));
        assert_eq!(shared_text.matches(GIT_EXCLUDE_START_MARKER).count(), 1);

        let personal_again = plan_git_excludes(
            repo.path(),
            ConnectionIntent::Personal,
            IntegrationProfile::Record,
        )?
        .expect("normal Git repository should produce an exclude plan");
        apply_generated_file(&personal_again)?;
        let personal_again_text = fs::read_to_string(&exclude_path)?;
        assert!(personal_again_text.contains("/.codex/hooks.json\n"));
        assert!(personal_again_text.contains("/.claude/settings.local.json\n"));
        assert_eq!(
            personal_again_text
                .matches(GIT_EXCLUDE_START_MARKER)
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn linked_personal_detective_is_rejected_before_a_file_plan_is_returned(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TestDirectory::new("linked-personal-detective")?;
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

        let error = plan_git_excludes(
            &repo,
            ConnectionIntent::Personal,
            IntegrationProfile::Detective,
        )
        .expect_err("linked personal detective must not modify a common exclude");

        assert!(error
            .to_string()
            .contains("LINKED_WORKTREE_PERSONAL_DETECTIVE_UNSUPPORTED"));
        assert!(!common.join("info/exclude").exists());
        Ok(())
    }

    #[test]
    fn linked_shared_plan_contains_only_intent_independent_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TestDirectory::new("linked-shared")?;
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

        let plan = plan_git_excludes(
            &repo,
            ConnectionIntent::Shared,
            IntegrationProfile::Detective,
        )?
        .expect("linked Git worktree should produce an exclude plan");

        assert!(plan.content.contains("/.volicord/\n"));
        assert!(plan
            .content
            .contains("/.codex/hooks/volicord-pre-tool.sh\n"));
        assert!(plan
            .content
            .contains("/.claude/hooks/volicord-pre-tool.sh\n"));
        assert!(!plan.content.contains("/.codex/hooks.json\n"));
        assert!(!plan.content.contains("/.claude/settings.local.json\n"));
        Ok(())
    }

    #[test]
    fn linked_record_transition_never_adds_sibling_sensitive_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TestDirectory::new("linked-record-transition")?;
        let repo = fixture.path().join("repo");
        let common = fixture.path().join("main/.git");
        let git_dir = common.join("worktrees/repo");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(common.join("info"))?;
        fs::create_dir_all(&git_dir)?;
        fs::write(
            repo.join(".git"),
            format!("gitdir: {}\n", git_dir.display()),
        )?;
        fs::write(git_dir.join("commondir"), "../..\n")?;

        let shared =
            plan_git_excludes(&repo, ConnectionIntent::Shared, IntegrationProfile::Record)?
                .expect("linked Git worktree should produce an exclude plan");
        apply_generated_file(&shared)?;
        let personal = plan_git_excludes(
            &repo,
            ConnectionIntent::Personal,
            IntegrationProfile::Record,
        )?
        .expect("linked Git worktree should produce an exclude plan");
        assert_eq!(personal.status.as_str(), "unchanged");

        let exclude_text = fs::read_to_string(common.join("info/exclude"))?;
        assert!(exclude_text.contains("/.volicord/\n"));
        assert!(exclude_text.contains("/.codex/hooks/volicord-pre-tool.sh\n"));
        assert!(exclude_text.contains("/.claude/hooks/volicord-pre-tool.sh\n"));
        assert!(!exclude_text.contains("/.codex/hooks.json\n"));
        assert!(!exclude_text.contains("/.claude/settings.local.json\n"));
        assert_eq!(exclude_text.matches(GIT_EXCLUDE_START_MARKER).count(), 1);
        Ok(())
    }
}
