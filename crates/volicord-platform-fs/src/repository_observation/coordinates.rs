use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

use volicord_types::{canonical::canonical_git_object_id, product_path::ProductRelativePath};

use crate::capture_git_workspace_snapshot;

use super::{
    bounded::{git_arguments, require_git_success, run_git, ObserverLimits},
    model::{
        hash_fields, ContentIdentity, ObservationUnavailable, ObservationUnavailableReason,
        RepositoryObservationCoordinate,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedCoordinates {
    pub(crate) repository_root: PathBuf,
    pub(crate) coordinate: RepositoryObservationCoordinate,
    pub(crate) status_paths: BTreeSet<ProductRelativePath>,
}

pub(crate) fn capture_coordinates(
    repository_root: &Path,
    limits: &ObserverLimits,
) -> Result<CapturedCoordinates, ObservationUnavailable> {
    let workspace = capture_git_workspace_snapshot(repository_root)
        .map_err(|error| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::GitLayoutUnavailable,
                format!("Git layout could not be captured: {error}"),
            )
        })?
        .ok_or_else(|| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::NotGitRepository,
                "the Product Repository has no supported Git worktree layout",
            )
        })?;
    let repository_root_text = path_text(&workspace.layout.repository_root)?;
    let git_dir_text = path_text(&workspace.layout.git_dir)?;
    let common_dir_text = path_text(&workspace.layout.common_dir)?;
    let linked_text = if workspace.layout.is_linked_worktree {
        "linked"
    } else {
        "primary"
    };
    let repository_identity = hash_fields(&[
        "repository_identity",
        &repository_root_text,
        &common_dir_text,
        &workspace.worktree_id,
    ]);
    let git_layout_identity = hash_fields(&[
        "git_layout_identity",
        &repository_root_text,
        &git_dir_text,
        &common_dir_text,
        linked_text,
    ]);

    let status_output = require_git_success(
        run_git(
            &workspace.layout.repository_root,
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
        "Git status observation",
    )?;
    let status_paths = parse_status_paths(&status_output)?;
    let status_identity = ContentIdentity::for_bytes(&status_output)
        .as_str()
        .to_owned();
    let tree_oid = workspace
        .head_sha
        .as_deref()
        .map(|head_oid| capture_tree_oid(&workspace.layout.repository_root, head_oid, limits))
        .transpose()?;
    let coordinate = RepositoryObservationCoordinate::new(
        repository_identity,
        git_layout_identity,
        workspace.worktree_id,
        workspace.head_sha,
        tree_oid,
        status_identity,
    );
    Ok(CapturedCoordinates {
        repository_root: workspace.layout.repository_root,
        coordinate,
        status_paths,
    })
}

fn capture_tree_oid(
    repository_root: &Path,
    head_oid: &str,
    limits: &ObserverLimits,
) -> Result<String, ObservationUnavailable> {
    let expression = OsString::from(format!("{head_oid}^{{tree}}"));
    let output = require_git_success(
        run_git(
            repository_root,
            &[
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                expression,
            ],
            limits,
        )?,
        "Git tree-coordinate observation",
    )?;
    let text = std::str::from_utf8(&output).map_err(|_| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::NonUtf8Path,
            "Git returned a non-UTF-8 tree coordinate",
        )
    })?;
    let oid = text.strip_suffix('\n').unwrap_or(text);
    if oid.is_empty() || oid.contains(['\n', '\r']) {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::GitObjectUnavailable,
            "Git returned an invalid tree coordinate",
        ));
    }
    canonical_git_object_id(oid).map_err(|_| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitObjectUnavailable,
            "Git returned a non-canonical tree coordinate",
        )
    })
}

fn parse_status_paths(
    output: &[u8],
) -> Result<BTreeSet<ProductRelativePath>, ObservationUnavailable> {
    let mut paths = BTreeSet::new();
    let mut records = output.split(|byte| *byte == 0).peekable();
    while let Some(record) = records.next() {
        if record.is_empty() {
            if records.peek().is_none() {
                break;
            }
            return Err(invalid_status("Git status contained an empty record"));
        }
        match record[0] {
            b'1' => insert_record_path(record, 9, &mut paths)?,
            b'2' => {
                insert_record_path(record, 10, &mut paths)?;
                let original = records
                    .next()
                    .ok_or_else(|| invalid_status("Git rename status omitted the original path"))?;
                insert_path(original, &mut paths)?;
            }
            b'u' => insert_record_path(record, 11, &mut paths)?,
            b'?' | b'!' => {
                let path = record
                    .strip_prefix(&[record[0], b' '])
                    .ok_or_else(|| invalid_status("Git status path record is malformed"))?;
                insert_path(path, &mut paths)?;
            }
            b'#' => {}
            _ => return Err(invalid_status("Git status returned an unknown record kind")),
        }
    }
    Ok(paths)
}

fn insert_record_path(
    record: &[u8],
    field_count: usize,
    paths: &mut BTreeSet<ProductRelativePath>,
) -> Result<(), ObservationUnavailable> {
    let path = record
        .splitn(field_count, |byte| *byte == b' ')
        .nth(field_count - 1)
        .ok_or_else(|| invalid_status("Git status record omitted its path"))?;
    insert_path(path, paths)
}

fn insert_path(
    raw_path: &[u8],
    paths: &mut BTreeSet<ProductRelativePath>,
) -> Result<(), ObservationUnavailable> {
    let path = std::str::from_utf8(raw_path).map_err(|_| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::NonUtf8Path,
            "Git status returned a non-UTF-8 Product Repository path",
        )
    })?;
    let path = ProductRelativePath::parse(path.to_owned()).map_err(|error| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::InvalidRelativePath,
            format!("Git status returned an invalid Product Repository path: {error}"),
        )
    })?;
    paths.insert(path);
    Ok(())
}

fn path_text(path: &Path) -> Result<String, ObservationUnavailable> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::NonUtf8Path,
            "the canonical Git layout contains a non-UTF-8 path",
        )
    })
}

fn invalid_status(detail: &'static str) -> ObservationUnavailable {
    ObservationUnavailable::new(ObservationUnavailableReason::GitCommandFailed, detail)
}
