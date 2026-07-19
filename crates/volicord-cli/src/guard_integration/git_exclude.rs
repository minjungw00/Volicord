use std::path::{Component, Path, PathBuf};

use volicord_platform_fs::resolve_git_worktree_layout;

use crate::{
    guard_integration::{
        files::{plan_managed_block_file, GeneratedFilePlan},
        GuardIntegrationError,
    },
    host_integration::ConnectionIntent,
};
use volicord_types::{GuardHookPhase, GuardManagedArtifact, IntegrationProfile};

pub(crate) const GIT_EXCLUDE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES";
pub(crate) const GIT_EXCLUDE_END_MARKER: &str = "# END VOLICORD MANAGED LOCAL EXCLUDES";

const ALWAYS_LOCAL_ARTIFACTS: &[GitExcludePathPolicy] = &[
    GitExcludePathPolicy::directory_parent(GuardManagedArtifact::VolicordPolicy),
    GitExcludePathPolicy::file(GuardManagedArtifact::HostHookDispatch),
    GitExcludePathPolicy::file(GuardManagedArtifact::HostHookWrapper(
        GuardHookPhase::PreTool,
    )),
    GitExcludePathPolicy::file(GuardManagedArtifact::HostHookWrapper(
        GuardHookPhase::PostTool,
    )),
    GitExcludePathPolicy::file(GuardManagedArtifact::HostHookWrapper(
        GuardHookPhase::PromptCapture,
    )),
];

const PERSONAL_ONLY_ARTIFACTS: &[GitExcludePathPolicy] = &[
    GitExcludePathPolicy::file(GuardManagedArtifact::HostHookConfig),
    GitExcludePathPolicy::file(GuardManagedArtifact::HostRuleInstruction),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitExcludePathFormat {
    File,
    ParentDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GitExcludePathPolicy {
    artifact: GuardManagedArtifact,
    format: GitExcludePathFormat,
}

impl GitExcludePathPolicy {
    const fn file(artifact: GuardManagedArtifact) -> Self {
        Self {
            artifact,
            format: GitExcludePathFormat::File,
        }
    }

    const fn directory_parent(artifact: GuardManagedArtifact) -> Self {
        Self {
            artifact,
            format: GitExcludePathFormat::ParentDirectory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitExcludePath {
    artifact: GuardManagedArtifact,
    pattern: String,
    tracking_path: String,
    ignore_probe: String,
}

impl GitExcludePath {
    pub(crate) const fn artifact(&self) -> GuardManagedArtifact {
        self.artifact
    }

    pub(crate) fn pattern(&self) -> &str {
        &self.pattern
    }

    pub(crate) fn tracking_path(&self) -> &str {
        &self.tracking_path
    }

    pub(crate) fn ignore_probe(&self) -> &str {
        &self.ignore_probe
    }

    pub(crate) fn ignore_probe_pattern(&self) -> String {
        format!("/{}", self.ignore_probe)
    }
}

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
    let block = exclude_block(include_personal_paths)?;
    plan_managed_block_file(
        GuardManagedArtifact::GitInfoExclude,
        &target.anchor_root,
        &target.exclude_path,
        &block,
        GIT_EXCLUDE_START_MARKER,
        GIT_EXCLUDE_END_MARKER,
        false,
    )
    .map(Some)
}

pub(crate) fn always_local_paths() -> Result<Vec<GitExcludePath>, GuardIntegrationError> {
    generated_paths(ALWAYS_LOCAL_ARTIFACTS)
}

pub(crate) fn personal_only_paths() -> Result<Vec<GitExcludePath>, GuardIntegrationError> {
    generated_paths(PERSONAL_ONLY_ARTIFACTS)
}

pub(crate) fn git_exclude_path(repo_root: &Path) -> Result<Option<PathBuf>, GuardIntegrationError> {
    Ok(resolve_git_exclude_target(repo_root)?.map(|target| target.exclude_path))
}

fn exclude_block(include_personal_paths: bool) -> Result<String, GuardIntegrationError> {
    let mut block = String::from(GIT_EXCLUDE_START_MARKER);
    block.push('\n');
    block.push_str("# Volicord local integration files that must remain outside Git.\n");
    for path in always_local_paths()? {
        block.push_str(path.pattern());
        block.push('\n');
    }
    if include_personal_paths {
        block.push_str("# Additional host-local files for a personal Volicord integration.\n");
        for path in personal_only_paths()? {
            block.push_str(path.pattern());
            block.push('\n');
        }
    }
    block.push_str(GIT_EXCLUDE_END_MARKER);
    block.push('\n');
    Ok(block)
}

fn generated_paths(
    policies: &[GitExcludePathPolicy],
) -> Result<Vec<GitExcludePath>, GuardIntegrationError> {
    generated_paths_with(policies, |artifact| {
        artifact
            .repository_relative_path()
            .map_err(|error| error.to_string())
    })
}

fn generated_paths_with(
    policies: &[GitExcludePathPolicy],
    mut repository_path: impl FnMut(GuardManagedArtifact) -> Result<PathBuf, String>,
) -> Result<Vec<GitExcludePath>, GuardIntegrationError> {
    policies
        .iter()
        .map(|policy| {
            let artifact_path = repository_path(policy.artifact).map_err(|detail| {
                GuardIntegrationError::runtime(format!(
                    "invalid Guard managed-artifact path contract: {detail}"
                ))
            })?;
            let ignore_probe = slash_relative_path(&artifact_path)?;
            let tracking_path = match policy.format {
                GitExcludePathFormat::File => ignore_probe.clone(),
                GitExcludePathFormat::ParentDirectory => {
                    let parent = artifact_path.parent().ok_or_else(|| {
                        GuardIntegrationError::runtime(format!(
                            "Guard artifact {} has no repository-relative parent directory",
                            policy.artifact.kind().as_str()
                        ))
                    })?;
                    slash_relative_path(parent)?
                }
            };
            let pattern = match policy.format {
                GitExcludePathFormat::File => format!("/{tracking_path}"),
                GitExcludePathFormat::ParentDirectory => format!("/{tracking_path}/"),
            };
            Ok(GitExcludePath {
                artifact: policy.artifact,
                pattern,
                tracking_path,
                ignore_probe,
            })
        })
        .collect()
}

fn slash_relative_path(path: &Path) -> Result<String, GuardIntegrationError> {
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str().map(str::to_owned).ok_or_else(|| {
                GuardIntegrationError::runtime(
                    "Guard managed-artifact path must use repository-relative UTF-8 components",
                )
            }),
            _ => Err(GuardIntegrationError::runtime(
                "Guard managed-artifact path must be repository-relative and normalized",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() {
        return Err(GuardIntegrationError::runtime(
            "Guard managed-artifact path must not be empty",
        ));
    }
    Ok(components.join("/"))
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
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn generated_exclude_blocks_preserve_exact_personal_and_shared_bytes() {
        assert_eq!(
            exclude_block(false).unwrap(),
            "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES\n\
# Volicord local integration files that must remain outside Git.\n\
/.volicord/\n\
/.codex/hooks/volicord-dispatch.sh\n\
/.codex/hooks/volicord-pre-tool.sh\n\
/.codex/hooks/volicord-post-tool.sh\n\
/.codex/hooks/volicord-prompt-capture.sh\n\
# END VOLICORD MANAGED LOCAL EXCLUDES\n"
        );
        assert_eq!(
            exclude_block(true).unwrap(),
            "# BEGIN VOLICORD MANAGED LOCAL EXCLUDES\n\
# Volicord local integration files that must remain outside Git.\n\
/.volicord/\n\
/.codex/hooks/volicord-dispatch.sh\n\
/.codex/hooks/volicord-pre-tool.sh\n\
/.codex/hooks/volicord-post-tool.sh\n\
/.codex/hooks/volicord-prompt-capture.sh\n\
# Additional host-local files for a personal Volicord integration.\n\
/.codex/hooks.json\n\
/.codex/rules/volicord.rules\n\
# END VOLICORD MANAGED LOCAL EXCLUDES\n"
        );
    }

    #[test]
    fn generated_paths_cover_typed_locality_policy_once() {
        let always = always_local_paths().unwrap();
        let personal = personal_only_paths().unwrap();
        assert_eq!(
            always
                .iter()
                .map(GitExcludePath::artifact)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                GuardManagedArtifact::VolicordPolicy,
                GuardManagedArtifact::HostHookDispatch,
                GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
                GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PostTool),
                GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PromptCapture),
            ])
        );
        assert_eq!(
            personal
                .iter()
                .map(GitExcludePath::artifact)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                GuardManagedArtifact::HostHookConfig,
                GuardManagedArtifact::HostRuleInstruction,
            ])
        );
        assert_eq!(always.len(), 5);
        assert_eq!(personal.len(), 2);
    }

    #[test]
    fn policy_directory_and_doctor_probe_derive_from_the_policy_artifact() {
        let policy = always_local_paths()
            .unwrap()
            .into_iter()
            .find(|path| path.artifact() == GuardManagedArtifact::VolicordPolicy)
            .unwrap();
        assert_eq!(policy.pattern(), "/.volicord/");
        assert_eq!(policy.tracking_path(), ".volicord");
        assert_eq!(policy.ignore_probe(), ".volicord/policy.json");
        assert_eq!(policy.ignore_probe_pattern(), "/.volicord/policy.json");
    }

    #[test]
    fn generated_consumer_paths_follow_the_resolved_artifact_path() {
        let paths = generated_paths_with(
            &[GitExcludePathPolicy::directory_parent(
                GuardManagedArtifact::VolicordPolicy,
            )],
            |_| Ok(PathBuf::from(".alternate/policy.json")),
        )
        .unwrap();
        assert_eq!(paths[0].pattern(), "/.alternate/");
        assert_eq!(paths[0].ignore_probe(), ".alternate/policy.json");
    }
}
