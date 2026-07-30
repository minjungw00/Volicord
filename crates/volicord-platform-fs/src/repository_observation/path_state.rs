use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File},
    io,
    path::Path,
};

use volicord_types::product_path::ProductRelativePath;

use crate::{ObservedProductRepository, PlatformDiagnosticKind};

use super::{
    bounded::{
        git_arguments, require_git_success, run_git, run_git_with_file_stdin, ObserverLimits,
    },
    model::{
        ContentIdentity, GitObjectIdentity, ObservationUnavailable, ObservationUnavailableReason,
        ProductPathState, RegularFileContentEvidence,
    },
};

pub(crate) struct HashBudget<'limits> {
    limits: &'limits ObserverLimits,
    total_hashed_bytes: u64,
}

impl<'limits> HashBudget<'limits> {
    pub(crate) fn new(limits: &'limits ObserverLimits) -> Self {
        Self {
            limits,
            total_hashed_bytes: 0,
        }
    }

    pub(crate) fn remaining(&self) -> u64 {
        self.limits
            .max_total_hashed_bytes()
            .saturating_sub(self.total_hashed_bytes)
    }

    pub(crate) fn charge(&mut self, bytes: u64) -> Result<(), ObservationUnavailable> {
        self.total_hashed_bytes = self
            .total_hashed_bytes
            .checked_add(bytes)
            .ok_or_else(total_hash_limit)?;
        if self.total_hashed_bytes > self.limits.max_total_hashed_bytes() {
            return Err(total_hash_limit());
        }
        Ok(())
    }

    fn ensure_file_size(&self, bytes: u64) -> Result<(), ObservationUnavailable> {
        if bytes > self.limits.max_file_bytes() {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::FileSizeLimitExceeded,
                "a Product Repository file exceeds the configured per-file byte limit",
            ));
        }
        if bytes > self.remaining() {
            return Err(total_hash_limit());
        }
        Ok(())
    }
}

pub(crate) fn observe_worktree_states(
    repository_root: &Path,
    candidates: &BTreeSet<ProductRelativePath>,
    limits: &ObserverLimits,
    budget: &mut HashBudget<'_>,
) -> Result<BTreeMap<ProductRelativePath, ProductPathState>, ObservationUnavailable> {
    let observed_repository =
        ObservedProductRepository::observe(repository_root).map_err(map_platform_observation)?;
    let mut states = BTreeMap::new();
    for path in candidates {
        observed_repository
            .observe_path(path.clone())
            .map_err(map_platform_observation)?;
        let state = observe_worktree_path(repository_root, path, limits, budget)?;
        states.insert(path.clone(), state);
    }
    Ok(states)
}

pub(crate) fn observe_tree_states(
    repository_root: &Path,
    tree_oid: Option<&str>,
    candidates: &BTreeSet<ProductRelativePath>,
    limits: &ObserverLimits,
    budget: &mut HashBudget<'_>,
) -> Result<BTreeMap<ProductRelativePath, ProductPathState>, ObservationUnavailable> {
    let mut states = candidates
        .iter()
        .cloned()
        .map(|path| (path, ProductPathState::Absent))
        .collect::<BTreeMap<_, _>>();
    let Some(tree_oid) = tree_oid else {
        return Ok(states);
    };
    if candidates.is_empty() {
        return Ok(states);
    }

    let mut arguments = git_arguments(&["ls-tree", "-z", "--full-name"]);
    arguments.push(OsString::from(tree_oid));
    arguments.push(OsString::from("--"));
    arguments.extend(candidates.iter().map(|path| OsString::from(path.as_str())));
    let output = require_git_success(
        run_git(repository_root, &arguments, limits)?,
        "Git tree path observation",
    )?;
    for record in output.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let (metadata, raw_path) = split_tab(record).ok_or_else(|| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::GitObjectUnavailable,
                "Git tree returned a malformed entry",
            )
        })?;
        let metadata = std::str::from_utf8(metadata).map_err(|_| non_utf8_git_object())?;
        let mut fields = metadata.split(' ');
        let mode = fields.next().unwrap_or_default();
        let object_kind = fields.next().unwrap_or_default();
        let object_oid = fields.next().unwrap_or_default();
        if fields.next().is_some() {
            return Err(git_object_unavailable("Git tree metadata is malformed"));
        }
        let path = parse_git_path(raw_path)?;
        if !candidates.contains(&path) {
            return Err(git_object_unavailable(
                "Git tree returned a path outside the candidate set",
            ));
        }
        let state = match (mode, object_kind) {
            ("100644", "blob") => ProductPathState::RegularFile {
                content_evidence: RegularFileContentEvidence::GitTree {
                    canonical_git_blob: GitObjectIdentity::parse(object_oid.to_owned()).map_err(
                        |_| git_object_unavailable("Git tree blob has a non-canonical object ID"),
                    )?,
                },
                executable: false,
            },
            ("100755", "blob") => ProductPathState::RegularFile {
                content_evidence: RegularFileContentEvidence::GitTree {
                    canonical_git_blob: GitObjectIdentity::parse(object_oid.to_owned()).map_err(
                        |_| git_object_unavailable("Git tree blob has a non-canonical object ID"),
                    )?,
                },
                executable: true,
            },
            ("120000", "blob") => ProductPathState::SymbolicLink {
                target: hash_tree_link_target(repository_root, object_oid, limits, budget)?,
            },
            ("160000", "commit") => ProductPathState::Gitlink {
                commit_oid: GitObjectIdentity::parse(object_oid.to_owned()).map_err(|_| {
                    git_object_unavailable("Git tree returned a non-canonical Gitlink commit")
                })?,
            },
            _ => {
                return Err(ObservationUnavailable::new(
                    ObservationUnavailableReason::UnsupportedPathState,
                    "Git tree contains an unsupported Product Repository path state",
                ));
            }
        };
        states.insert(path, state);
    }
    Ok(states)
}

fn observe_worktree_path(
    repository_root: &Path,
    path: &ProductRelativePath,
    limits: &ObserverLimits,
    budget: &mut HashBudget<'_>,
) -> Result<ProductPathState, ObservationUnavailable> {
    let absolute = repository_root.join(path.as_str());
    let metadata = match fs::symlink_metadata(&absolute) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ProductPathState::Absent);
        }
        Err(error) => return Err(path_io_error("path metadata", error)),
    };
    let file_type = metadata.file_type();
    if file_type.is_file() {
        let content_evidence = hash_regular_file(
            repository_root,
            path,
            &absolute,
            metadata.len(),
            limits,
            budget,
        )?;
        return Ok(ProductPathState::RegularFile {
            content_evidence,
            executable: executable_bit(&metadata),
        });
    }
    if file_type.is_symlink() {
        let target = fs::read_link(&absolute)
            .map_err(|error| path_io_error("symbolic-link target", error))?;
        let target = target.to_str().ok_or_else(|| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::NonUtf8Path,
                "a Product Repository symbolic link has a non-UTF-8 target",
            )
        })?;
        budget.ensure_file_size(target.len() as u64)?;
        budget.charge(target.len() as u64)?;
        return Ok(ProductPathState::SymbolicLink {
            target: ContentIdentity::for_bytes(target.as_bytes()),
        });
    }
    if file_type.is_dir() {
        return observe_gitlink(&absolute, repository_root, path, limits);
    }
    Err(ObservationUnavailable::new(
        ObservationUnavailableReason::UnsupportedPathState,
        "a Product Repository candidate is not a regular file, symbolic link, or Gitlink",
    ))
}

fn hash_regular_file(
    repository_root: &Path,
    relative_path: &ProductRelativePath,
    absolute_path: &Path,
    expected_size: u64,
    limits: &ObserverLimits,
    budget: &mut HashBudget<'_>,
) -> Result<RegularFileContentEvidence, ObservationUnavailable> {
    budget.ensure_file_size(expected_size)?;
    let file =
        File::open(absolute_path).map_err(|error| path_io_error("regular-file content", error))?;
    let mut arguments = git_arguments(&["hash-object"]);
    arguments.push(OsString::from(format!("--path={}", relative_path.as_str())));
    arguments.push(OsString::from("--stdin"));
    let hashed = run_git_with_file_stdin(
        repository_root,
        &arguments,
        file,
        budget.remaining(),
        limits,
    )?;
    budget.charge(hashed.source_bytes)?;
    let output = require_git_success(hashed.output, "Git canonical-content observation")?;
    let canonical_git_blob = parse_canonical_git_identity(&output)?;
    Ok(RegularFileContentEvidence::Worktree {
        exact_worktree_bytes: hashed.exact_worktree_bytes,
        canonical_git_blob,
    })
}

pub(super) fn parse_canonical_git_identity(
    output: &[u8],
) -> Result<GitObjectIdentity, ObservationUnavailable> {
    let output = std::str::from_utf8(output)
        .map_err(|_| git_object_unavailable("Git canonical-content identity is not valid UTF-8"))?;
    let object_oid = output.strip_suffix('\n').unwrap_or(output);
    if object_oid.is_empty() || object_oid.contains(['\n', '\r']) {
        return Err(git_object_unavailable(
            "Git returned a malformed canonical-content identity",
        ));
    }
    let identity = GitObjectIdentity::parse(object_oid.to_owned()).map_err(|_| {
        git_object_unavailable("Git returned a non-canonical canonical-content identity")
    })?;
    if identity.as_str() != object_oid {
        return Err(git_object_unavailable(
            "Git returned a non-canonical canonical-content identity",
        ));
    }
    Ok(identity)
}

fn observe_gitlink(
    absolute_path: &Path,
    repository_root: &Path,
    path: &ProductRelativePath,
    limits: &ObserverLimits,
) -> Result<ProductPathState, ObservationUnavailable> {
    let mut arguments = git_arguments(&["ls-files", "--stage", "-z", "--"]);
    arguments.push(OsString::from(path.as_str()));
    let output = require_git_success(
        run_git(repository_root, &arguments, limits)?,
        "Gitlink index observation",
    )?;
    let records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .collect::<Vec<_>>();
    if records.len() != 1 {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::UnsupportedPathState,
            "a directory candidate is not one exact stage-zero Gitlink",
        ));
    }
    let (metadata, raw_path) = split_tab(records[0])
        .ok_or_else(|| git_object_unavailable("Gitlink index entry is malformed"))?;
    let metadata = std::str::from_utf8(metadata).map_err(|_| non_utf8_git_object())?;
    let fields = metadata.split(' ').collect::<Vec<_>>();
    if fields.len() != 3
        || fields[0] != "160000"
        || fields[2] != "0"
        || &parse_git_path(raw_path)? != path
    {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::UnsupportedPathState,
            "a directory candidate is not one exact stage-zero Gitlink",
        ));
    }

    let status = require_git_success(
        run_git(
            absolute_path,
            &git_arguments(&[
                "-c",
                "core.filemode=true",
                "-c",
                "core.fsmonitor=false",
                "status",
                "--porcelain=v2",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=none",
                "--no-renames",
            ]),
            limits,
        )?,
        "nested Gitlink status observation",
    )?;
    if !status.is_empty() {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::UnsupportedPathState,
            "a dirty Gitlink cannot be represented as one exact commit state",
        ));
    }
    let head = require_git_success(
        run_git(
            absolute_path,
            &git_arguments(&["rev-parse", "--verify", "HEAD"]),
            limits,
        )?,
        "Gitlink HEAD observation",
    )?;
    let head = std::str::from_utf8(&head).map_err(|_| non_utf8_git_object())?;
    let head = head.strip_suffix('\n').unwrap_or(head);
    let commit_oid = GitObjectIdentity::parse(head.to_owned())
        .map_err(|_| git_object_unavailable("Gitlink HEAD is not a canonical object ID"))?;
    Ok(ProductPathState::Gitlink { commit_oid })
}

fn hash_tree_link_target(
    repository_root: &Path,
    object_oid: &str,
    limits: &ObserverLimits,
    budget: &mut HashBudget<'_>,
) -> Result<ContentIdentity, ObservationUnavailable> {
    GitObjectIdentity::parse(object_oid.to_owned())
        .map_err(|_| git_object_unavailable("Git link blob has a non-canonical object ID"))?;
    let output = require_git_success(
        run_git(
            repository_root,
            &[
                OsString::from("cat-file"),
                OsString::from("blob"),
                OsString::from(object_oid),
            ],
            limits,
        )?,
        "Git symbolic-link target observation",
    )?;
    std::str::from_utf8(&output).map_err(|_| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::NonUtf8Path,
            "a Git symbolic-link target is not valid UTF-8",
        )
    })?;
    budget.ensure_file_size(output.len() as u64)?;
    budget.charge(output.len() as u64)?;
    Ok(ContentIdentity::for_bytes(&output))
}

fn parse_git_path(raw_path: &[u8]) -> Result<ProductRelativePath, ObservationUnavailable> {
    let path = std::str::from_utf8(raw_path).map_err(|_| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::NonUtf8Path,
            "Git returned a non-UTF-8 Product Repository path",
        )
    })?;
    ProductRelativePath::parse(path.to_owned()).map_err(|error| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::InvalidRelativePath,
            format!("Git returned an invalid Product Repository path: {error}"),
        )
    })
}

fn split_tab(record: &[u8]) -> Option<(&[u8], &[u8])> {
    let index = record.iter().position(|byte| *byte == b'\t')?;
    Some((&record[..index], &record[index + 1..]))
}

fn map_platform_observation(error: crate::PlatformBoundaryError) -> ObservationUnavailable {
    let reason = match error.kind() {
        PlatformDiagnosticKind::ProductPathContainmentFailure => {
            ObservationUnavailableReason::PathOutsideRepository
        }
        PlatformDiagnosticKind::ProductRepositoryNotFound
        | PlatformDiagnosticKind::InvalidProductRepositoryRoot => {
            ObservationUnavailableReason::InvalidRepositoryRoot
        }
        _ => ObservationUnavailableReason::InaccessiblePath,
    };
    ObservationUnavailable::new(reason, error.to_string())
}

fn path_io_error(stage: &'static str, error: io::Error) -> ObservationUnavailable {
    ObservationUnavailable::new(
        ObservationUnavailableReason::InaccessiblePath,
        format!("Product Repository {stage} observation failed: {error}"),
    )
}

fn executable_bit(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        false
    }
}

fn total_hash_limit() -> ObservationUnavailable {
    ObservationUnavailable::new(
        ObservationUnavailableReason::TotalHashBytesLimitExceeded,
        "repository hashing exceeds the configured aggregate byte limit",
    )
}

fn git_object_unavailable(detail: &'static str) -> ObservationUnavailable {
    ObservationUnavailable::new(ObservationUnavailableReason::GitObjectUnavailable, detail)
}

fn non_utf8_git_object() -> ObservationUnavailable {
    ObservationUnavailable::new(
        ObservationUnavailableReason::GitObjectUnavailable,
        "Git object metadata is not valid UTF-8",
    )
}
