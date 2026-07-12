use std::path::{Path, PathBuf};

use volicord_platform_fs::resolve_git_worktree_layout;

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, GeneratedFilePlan},
        GuardIntegrationError,
    },
    host_integration::{ConnectionIntent, HostIntegrationFileKind},
};
use volicord_types::IntegrationProfile;

pub(crate) const GIT_EXCLUDE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES";
pub(crate) const GIT_EXCLUDE_END_MARKER: &str = "# END VOLICORD MANAGED LOCAL EXCLUDES";

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
    plan_git_excludes_with_personal_protection(
        repo_root,
        connection_intent,
        profile,
        connection_intent == ConnectionIntent::Personal,
    )
}

pub(crate) fn plan_git_excludes_with_personal_protection(
    repo_root: &Path,
    connection_intent: ConnectionIntent,
    profile: IntegrationProfile,
    retain_personal_paths: bool,
) -> Result<Option<GeneratedFilePlan>, GuardIntegrationError> {
    let Some(target) = resolve_git_exclude_target(repo_root)? else {
        if connection_intent == ConnectionIntent::Personal {
            let marker = repo_root.join(".git");
            return Err(GuardIntegrationError::runtime(format!(
                "failed to inspect Git repository marker {}: file not found",
                marker.display()
            )));
        }
        return Ok(None);
    };
    if target.is_linked_worktree
        && connection_intent == ConnectionIntent::Personal
        && profile == IntegrationProfile::Detective
    {
        return Err(GuardIntegrationError::runtime(
            "LINKED_WORKTREE_PERSONAL_DETECTIVE_UNSUPPORTED: personal detective init would require worktree-specific local hook paths, but this linked worktree exposes only the common Git info/exclude shared by sibling worktrees. Use --profile record, use --shared for a repository-managed detective integration, or initialize detective in a standalone Git worktree.",
        ));
    }
    let include_personal_paths = !target.is_linked_worktree && retain_personal_paths;
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
    Ok(resolve_git_exclude_target(repo_root)?.map(|target| target.exclude_path))
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

fn resolve_git_exclude_target(
    repo_root: &Path,
) -> Result<Option<GitExcludeTarget>, GuardIntegrationError> {
    let marker = repo_root.join(".git");
    let layout = resolve_git_worktree_layout(repo_root).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            GuardIntegrationError::runtime(format!(
                "unsafe Git metadata path at {}: {error}",
                marker.display()
            ))
        } else {
            GuardIntegrationError::runtime(format!(
                "failed to inspect Git repository marker {}: {error}",
                marker.display()
            ))
        }
    })?;
    Ok(layout.map(|layout| GitExcludeTarget {
        exclude_path: layout.common_dir.join("info").join("exclude"),
        anchor_root: layout.common_dir,
        is_linked_worktree: layout.is_linked_worktree,
    }))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

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

        let target = resolve_git_exclude_target(repo.path())?
            .expect("normal repository should resolve Git layout");

        assert_eq!(target.anchor_root, repo.path().join(".git"));
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

        let target =
            resolve_git_exclude_target(&repo)?.expect("linked worktree should resolve Git layout");

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
