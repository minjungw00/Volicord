use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
};

use volicord_types::product_path::ProductRelativePath;

use super::{
    bounded::{git_arguments, require_git_success, run_git},
    coordinates::capture_coordinates,
    model::{
        ensure_candidate_limit, ObservationUnavailable, ObservationUnavailableReason,
        ProductPathState, RepositoryDelta, RepositoryObservationSnapshot, RepositoryPathTransition,
    },
    path_state::{observe_tree_states, HashBudget},
    snapshot::RepositoryObserver,
};

pub(crate) fn calculate_delta(
    observer: &RepositoryObserver,
    before: &RepositoryObservationSnapshot,
    after: &RepositoryObservationSnapshot,
) -> Result<RepositoryDelta, ObservationUnavailable> {
    if before == after {
        return Ok(RepositoryDelta::default());
    }
    ensure_compatible(observer, before, after)?;
    let current = capture_coordinates(observer.repository_root(), observer.limits())?;
    if current.coordinate.repository_identity() != before.coordinate.repository_identity()
        || current.coordinate.git_layout_identity() != before.coordinate.git_layout_identity()
    {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::RepositoryIdentityChanged,
            "the Product Repository identity or Git layout changed after snapshot capture",
        ));
    }

    let mut candidates = before
        .observed_states
        .keys()
        .chain(after.observed_states.keys())
        .chain(&before.status_paths)
        .chain(&after.status_paths)
        .chain(&before.invocation_paths)
        .chain(&after.invocation_paths)
        .cloned()
        .collect::<BTreeSet<_>>();
    candidates.extend(tree_changed_paths(observer, before, after)?);
    ensure_candidate_limit(candidates.len(), observer.limits())?;

    let before_missing = candidates
        .iter()
        .filter(|path| !before.observed_states.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let after_missing = candidates
        .iter()
        .filter(|path| !after.observed_states.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut budget = HashBudget::new(observer.limits());
    let before_tree_states = observe_tree_states(
        observer.repository_root(),
        before.coordinate.tree_oid(),
        &before_missing,
        observer.limits(),
        &mut budget,
    )?;
    let after_tree_states = observe_tree_states(
        observer.repository_root(),
        after.coordinate.tree_oid(),
        &after_missing,
        observer.limits(),
        &mut budget,
    )?;

    let mut transitions = Vec::new();
    for path in candidates {
        let before_state = snapshot_path_state(before, &before_tree_states, &path)?;
        let after_state = snapshot_path_state(after, &after_tree_states, &path)?;
        if !before_state.semantically_eq(&after_state) {
            transitions.push(RepositoryPathTransition::new(
                path,
                before_state,
                after_state,
            ));
        }
    }
    let delta = RepositoryDelta::new(transitions);
    if delta.canonical_bytes().len() > observer.limits().max_serialized_bytes() {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::SerializationSizeLimitExceeded,
            "the canonical repository delta exceeds its serialization limit",
        ));
    }
    Ok(delta)
}

fn ensure_compatible(
    observer: &RepositoryObserver,
    before: &RepositoryObservationSnapshot,
    after: &RepositoryObservationSnapshot,
) -> Result<(), ObservationUnavailable> {
    if before.contract_digest != after.contract_digest
        || before.contract_digest != *observer.contract_digest()
    {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::ObserverContractMismatch,
            "repository snapshots use different observer contracts or limits",
        ));
    }
    if before.repository_root != after.repository_root
        || before.repository_root != observer.repository_root()
        || before.coordinate.repository_identity() != after.coordinate.repository_identity()
        || before.coordinate.git_layout_identity() != after.coordinate.git_layout_identity()
        || before.coordinate.worktree_identity() != after.coordinate.worktree_identity()
    {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::RepositoryIdentityChanged,
            "repository snapshots do not belong to one canonical Git worktree",
        ));
    }
    Ok(())
}

fn tree_changed_paths(
    observer: &RepositoryObserver,
    before: &RepositoryObservationSnapshot,
    after: &RepositoryObservationSnapshot,
) -> Result<BTreeSet<ProductRelativePath>, ObservationUnavailable> {
    let before_tree = before.coordinate.tree_oid();
    let after_tree = after.coordinate.tree_oid();
    if before_tree == after_tree {
        return Ok(BTreeSet::new());
    }
    let arguments = match (before_tree, after_tree) {
        (Some(before_tree), Some(after_tree)) => {
            let mut arguments = git_arguments(&[
                "diff-tree",
                "--no-commit-id",
                "--name-only",
                "-z",
                "-r",
                "--no-renames",
                "--no-ext-diff",
            ]);
            arguments.push(OsString::from(before_tree));
            arguments.push(OsString::from(after_tree));
            arguments
        }
        (Some(tree), None) | (None, Some(tree)) => {
            let mut arguments = git_arguments(&["ls-tree", "-r", "-z", "--name-only"]);
            arguments.push(OsString::from(tree));
            arguments
        }
        (None, None) => return Ok(BTreeSet::new()),
    };
    let output = require_git_success(
        run_git(observer.repository_root(), &arguments, observer.limits())?,
        "Git tree-delta candidate observation",
    )?;
    parse_nul_paths(&output)
}

fn parse_nul_paths(output: &[u8]) -> Result<BTreeSet<ProductRelativePath>, ObservationUnavailable> {
    let mut paths = BTreeSet::new();
    for raw_path in output.split(|byte| *byte == 0) {
        if raw_path.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw_path).map_err(|_| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::NonUtf8Path,
                "Git tree comparison returned a non-UTF-8 path",
            )
        })?;
        let path = ProductRelativePath::parse(path.to_owned()).map_err(|error| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::InvalidRelativePath,
                format!("Git tree comparison returned an invalid path: {error}"),
            )
        })?;
        paths.insert(path);
    }
    Ok(paths)
}

fn snapshot_path_state(
    snapshot: &RepositoryObservationSnapshot,
    tree_states: &BTreeMap<ProductRelativePath, ProductPathState>,
    path: &ProductRelativePath,
) -> Result<ProductPathState, ObservationUnavailable> {
    snapshot
        .observed_states
        .get(path)
        .or_else(|| tree_states.get(path))
        .cloned()
        .ok_or_else(|| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::GitObjectUnavailable,
                "a repository candidate has no observable snapshot state",
            )
        })
}
