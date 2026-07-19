use std::path::{Path, PathBuf};

use volicord_platform_fs::resolve_git_worktree_layout;

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, GeneratedFilePlan},
        GuardIntegrationError,
    },
    host_integration::ConnectionIntent,
};
use volicord_types::{GuardManagedArtifact, IntegrationProfile};

pub(crate) const GIT_EXCLUDE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES";
pub(crate) const GIT_EXCLUDE_END_MARKER: &str = "# END VOLICORD MANAGED LOCAL EXCLUDES";

const ALWAYS_LOCAL_PATHS: &[&str] = &[
    "/.volicord/",
    "/.codex/hooks/volicord-dispatch.sh",
    "/.codex/hooks/volicord-pre-tool.sh",
    "/.codex/hooks/volicord-post-tool.sh",
    "/.codex/hooks/volicord-prompt-capture.sh",
];

const PERSONAL_ONLY_PATHS: &[&str] = &["/.codex/hooks.json", "/.codex/rules/volicord.rules"];

pub(crate) fn plan_git_excludes(
    repo_root: &Path,
    connection_intent: ConnectionIntent,
    _profile: IntegrationProfile,
) -> Result<Option<GeneratedFilePlan>, GuardIntegrationError> {
    plan_git_excludes_with_personal_protection(
        repo_root,
        connection_intent,
        IntegrationProfile::Record,
        connection_intent == ConnectionIntent::Personal,
    )
}

pub(crate) fn plan_git_excludes_with_personal_protection(
    repo_root: &Path,
    connection_intent: ConnectionIntent,
    _profile: IntegrationProfile,
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
    let include_personal_paths = !target.is_linked_worktree && retain_personal_paths;
    plan_managed_block_file(
        GuardManagedArtifact::GitInfoExclude,
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
