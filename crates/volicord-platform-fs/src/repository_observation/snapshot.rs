use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use volicord_types::product_path::ProductRelativePath;

use super::{
    bounded::ObserverLimits,
    coordinates::{capture_coordinates, CapturedCoordinates},
    delta::calculate_delta,
    model::{
        ensure_candidate_limit, validate_checkpoint, InvocationObservationPaths,
        ObservationUnavailable, ObservationUnavailableReason, RepositoryDelta,
        RepositoryObservationCheckpoint, RepositoryObservationSnapshot,
        SemanticObserverContractDigest, SNAPSHOT_SERIALIZATION_DEPTH,
    },
    path_state::{observe_worktree_states, HashBudget},
};

/// Reusable bounded Product Repository observation engine.
#[derive(Debug, Clone)]
pub struct RepositoryObserver {
    repository_root: PathBuf,
    limits: ObserverLimits,
    contract_digest: SemanticObserverContractDigest,
}

impl RepositoryObserver {
    /// Constructs an observer for one canonical Product Repository root.
    pub fn new(
        repository_root: impl AsRef<Path>,
        limits: ObserverLimits,
    ) -> Result<Self, ObservationUnavailable> {
        limits.validate()?;
        if limits.max_serialization_depth() < SNAPSHOT_SERIALIZATION_DEPTH {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::SerializationDepthLimitExceeded,
                "the configured serialization depth cannot represent a repository snapshot",
            ));
        }
        let repository_root = fs::canonicalize(repository_root.as_ref()).map_err(|error| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::InvalidRepositoryRoot,
                format!("the Product Repository root could not be canonicalized: {error}"),
            )
        })?;
        let metadata = fs::metadata(&repository_root).map_err(|error| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::InvalidRepositoryRoot,
                format!("the Product Repository root could not be inspected: {error}"),
            )
        })?;
        if !metadata.is_dir() {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::InvalidRepositoryRoot,
                "the Product Repository root is not a directory",
            ));
        }
        let contract_digest = SemanticObserverContractDigest::for_limits(&limits);
        Ok(Self {
            repository_root,
            limits,
            contract_digest,
        })
    }

    /// Active observer limits.
    pub fn limits(&self) -> &ObserverLimits {
        &self.limits
    }

    /// Semantic contract and resource-limit digest for this observer.
    pub fn contract_digest(&self) -> &SemanticObserverContractDigest {
        &self.contract_digest
    }

    /// Captures one stable invocation-scoped repository snapshot.
    pub fn snapshot(
        &self,
        invocation_paths: &InvocationObservationPaths,
    ) -> Result<RepositoryObservationSnapshot, ObservationUnavailable> {
        self.snapshot_with_stability_hook(invocation_paths, |_, _| {})
    }

    /// Calculates the exact deterministic net delta between two snapshots.
    pub fn delta(
        &self,
        before: &RepositoryObservationSnapshot,
        after: &RepositoryObservationSnapshot,
    ) -> Result<RepositoryDelta, ObservationUnavailable> {
        calculate_delta(self, before, after)
    }

    /// Restores one validated checkpoint under this observer's root, limits, and semantics.
    pub fn restore_checkpoint(
        &self,
        checkpoint: RepositoryObservationCheckpoint,
    ) -> Result<RepositoryObservationSnapshot, ObservationUnavailable> {
        if checkpoint.contract_digest != self.contract_digest {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::ObserverContractMismatch,
                "the repository observation checkpoint uses a different observer contract",
            ));
        }
        validate_checkpoint(&checkpoint)?;
        ensure_candidate_limit(checkpoint.observed_states.len(), &self.limits)?;
        let snapshot = RepositoryObservationSnapshot {
            repository_root: self.repository_root.clone(),
            coordinate: checkpoint.coordinate,
            observed_states: checkpoint.observed_states,
            status_paths: checkpoint.status_paths,
            invocation_paths: checkpoint.invocation_paths,
            contract_digest: checkpoint.contract_digest,
            limits: self.limits.clone(),
        };
        snapshot.canonical_bytes()?;
        Ok(snapshot)
    }

    pub(crate) fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    pub(crate) fn snapshot_with_stability_hook(
        &self,
        invocation_paths: &InvocationObservationPaths,
        mut before_recheck: impl FnMut(usize, &Path),
    ) -> Result<RepositoryObservationSnapshot, ObservationUnavailable> {
        let invocation_paths = invocation_paths.canonical_set();
        ensure_candidate_limit(invocation_paths.len(), &self.limits)?;
        ensure_input_serialization_size(&invocation_paths, &self.limits)?;
        let mut hash_budget = HashBudget::new(&self.limits);

        for attempt in 0..self.limits.max_stability_attempts() {
            let before = capture_coordinates(&self.repository_root, &self.limits)?;
            self.ensure_same_root(&before)?;
            let candidates = candidate_paths(&before, &invocation_paths, &self.limits)?;
            let first_states = observe_worktree_states(
                &self.repository_root,
                &candidates,
                &self.limits,
                &mut hash_budget,
            )?;
            let middle = capture_coordinates(&self.repository_root, &self.limits)?;
            self.ensure_same_root(&middle)?;
            if before.coordinate != middle.coordinate || before.status_paths != middle.status_paths
            {
                continue;
            }

            before_recheck(attempt, &self.repository_root);
            let second_states = observe_worktree_states(
                &self.repository_root,
                &candidates,
                &self.limits,
                &mut hash_budget,
            )?;
            let after = capture_coordinates(&self.repository_root, &self.limits)?;
            self.ensure_same_root(&after)?;
            if middle.coordinate != after.coordinate
                || middle.status_paths != after.status_paths
                || first_states != second_states
            {
                continue;
            }

            let snapshot = RepositoryObservationSnapshot {
                repository_root: self.repository_root.clone(),
                coordinate: after.coordinate,
                observed_states: second_states,
                status_paths: after.status_paths,
                invocation_paths: invocation_paths.clone(),
                contract_digest: self.contract_digest.clone(),
                limits: self.limits.clone(),
            };
            snapshot.canonical_bytes()?;
            return Ok(snapshot);
        }
        Err(ObservationUnavailable::new(
            ObservationUnavailableReason::UnstableRepository,
            "the Product Repository changed while its snapshot was being captured",
        ))
    }

    fn ensure_same_root(
        &self,
        coordinates: &CapturedCoordinates,
    ) -> Result<(), ObservationUnavailable> {
        if coordinates.repository_root != self.repository_root {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::RepositoryIdentityChanged,
                "the canonical Product Repository root changed during observation",
            ));
        }
        Ok(())
    }
}

fn candidate_paths(
    coordinates: &CapturedCoordinates,
    invocation_paths: &BTreeSet<ProductRelativePath>,
    limits: &ObserverLimits,
) -> Result<BTreeSet<ProductRelativePath>, ObservationUnavailable> {
    let candidates = coordinates
        .status_paths
        .union(invocation_paths)
        .cloned()
        .collect::<BTreeSet<_>>();
    ensure_candidate_limit(candidates.len(), limits)?;
    Ok(candidates)
}

fn ensure_input_serialization_size(
    paths: &BTreeSet<ProductRelativePath>,
    limits: &ObserverLimits,
) -> Result<(), ObservationUnavailable> {
    let encoded_size = paths.iter().try_fold(8usize, |total, path| {
        total
            .checked_add(8)
            .and_then(|value| value.checked_add(path.as_str().len()))
    });
    if encoded_size.is_none_or(|size| size > limits.max_serialized_bytes()) {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::SerializationSizeLimitExceeded,
            "typed invocation paths exceed the configured serialization size limit",
        ));
    }
    Ok(())
}
