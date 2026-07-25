use std::{
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use volicord_platform_fs::{
    DirectoryEntryDurability, DirectoryTreeRemovalEffect, DirectoryTreeRemovalOutcome,
    DirectoryTreeTargetState, RuntimeHomeMutationLeaseMode,
};
use volicord_store::bootstrap::{
    commit_runtime_home, inspect_runtime_home_bootstrap, prepare_runtime_home_with_installation,
    InstallationProfileRegistration, PreparedRuntimeHome, RuntimeHomeBootstrapState,
    RuntimeHomePublicationGuard, RuntimeHomePublicationOutcome,
    RuntimeHomePublicationRollbackOutcome,
};
use volicord_store::setup_transaction::{PreparedStoreMutationBoundary, StoreMutationInput};
use volicord_store::RuntimeHomeMutationContext;
use volicord_types::IntegrationActivationPlan;

use super::{
    output::{RuntimeHomePublicationStatus, RuntimeHomeRollbackResult},
    ConnectionCommandError,
};

pub(super) const FAULT_AFTER_RUNTIME_HOME_PREPARATION: &str = "after_runtime_home_preparation";
pub(super) const FAULT_AFTER_REGISTRY_MUTATION_PREPARATION: &str =
    "after_registry_mutation_preparation";
pub(super) const FAULT_RUNTIME_HOME_PARENT_DIRECTORY_SYNC: &str =
    "runtime_home_parent_directory_sync";
pub(super) const FAULT_RUNTIME_HOME_PUBLICATION_READ_BACK: &str =
    "runtime_home_publication_read_back";
pub(super) const FAULT_RUNTIME_HOME_PUBLICATION_MANIFEST_VALIDATION: &str =
    "runtime_home_publication_manifest_validation";
pub(super) const FAULT_BEFORE_CODEX_CONFIG_REPLACE: &str = "before_codex_config_replace";
pub(super) const FAULT_AFTER_CODEX_CONFIG_REPLACE: &str = "after_codex_config_replace";
pub(super) const FAULT_BEFORE_INTEGRATION_REVISION_COMMIT: &str =
    "before_integration_revision_commit";
pub(super) const FAULT_DURING_ROLLBACK: &str = "during_rollback";

#[derive(Debug)]
pub(super) struct SetupPlan {
    pub(super) runtime_home: RuntimeHomePlan,
    pub(super) store_inputs: Vec<StoreMutationInput>,
    pub(super) registry_mutations: Vec<StoreMutation>,
    pub(super) host_file_mutations: Vec<AtomicFileMutation>,
    pub(super) repository_file_mutations: Vec<AtomicFileMutation>,
    pub(super) activation_plan: IntegrationActivationPlan,
}

impl SetupPlan {
    pub(super) fn planned_file_count(&self) -> usize {
        self.host_file_mutations.len() + self.repository_file_mutations.len()
    }
}

#[derive(Debug, Clone)]
pub(super) enum RuntimeHomePlan {
    Create {
        final_path: PathBuf,
        runtime_home_id: String,
        metadata_json: String,
        installation: InstallationProfileRegistration,
    },
    Validate {
        final_path: PathBuf,
    },
}

impl RuntimeHomePlan {
    pub(super) fn final_path(&self) -> &Path {
        match self {
            Self::Create { final_path, .. } | Self::Validate { final_path } => final_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoreMutation {
    pub(super) kind: StoreMutationKind,
    pub(super) target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StoreMutationKind {
    InstallationProfile,
    ProjectRegistration,
    WorkflowPolicy,
    Connection,
    ConnectionMembership,
    GuardInstallation,
    IntegrationRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AtomicFileMutationRole {
    CodexConfig,
    GuardManagedFile(String),
}

#[derive(Debug, Clone)]
pub(super) struct AtomicFileMutation {
    pub(super) target: PathBuf,
    pub(super) target_existed: bool,
    pub(super) original_metadata: Option<OriginalMetadata>,
    pub(super) original_digest: Option<String>,
    pub(super) staged_digest: Option<String>,
    desired_bytes: Option<Vec<u8>>,
    original: FileSnapshot,
    executable: bool,
    pub(super) role: AtomicFileMutationRole,
    staged_path: Option<PathBuf>,
    committed: bool,
}

#[derive(Debug, Clone)]
pub(super) struct OriginalMetadata {
    permissions: Permissions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FileSnapshot {
    Missing,
    Present(Vec<u8>),
}

impl AtomicFileMutation {
    pub(super) fn plan(
        target: PathBuf,
        desired_bytes: Option<Vec<u8>>,
        expected_bytes: Option<&[u8]>,
        executable: bool,
        role: AtomicFileMutationRole,
    ) -> Result<Option<Self>, ConnectionCommandError> {
        validate_target_path_read_only(&target)?;
        let (original, metadata) = read_file_snapshot(&target)?;
        let expected = expected_bytes.map_or(FileSnapshot::Missing, |bytes| {
            FileSnapshot::Present(bytes.to_vec())
        });
        if original != expected {
            return Err(ConnectionCommandError::concurrent_modification(format!(
                "SETUP_CONCURRENT_MODIFICATION: setup target changed during plan construction: {}; external bytes were preserved",
                target.display()
            )));
        }
        if matches!(
            (&original, desired_bytes.as_deref()),
            (FileSnapshot::Missing, None)
        ) || matches!(
            (&original, desired_bytes.as_deref()),
            (FileSnapshot::Present(current), Some(desired)) if current == desired
        ) {
            return Ok(None);
        }
        let original_digest = match &original {
            FileSnapshot::Missing => None,
            FileSnapshot::Present(bytes) => Some(digest(bytes)),
        };
        let staged_digest = desired_bytes.as_deref().map(digest);
        Ok(Some(Self {
            target,
            target_existed: matches!(original, FileSnapshot::Present(_)),
            original_metadata: metadata.map(|permissions| OriginalMetadata { permissions }),
            original_digest,
            staged_digest,
            desired_bytes,
            original,
            executable,
            role,
            staged_path: None,
            committed: false,
        }))
    }

    pub(super) fn prepare(
        &mut self,
        created_directories: &mut Vec<PathBuf>,
    ) -> Result<(), ConnectionCommandError> {
        debug_assert_eq!(
            self.target_existed,
            matches!(self.original, FileSnapshot::Present(_))
        );
        create_parent_directories(&self.target, created_directories)?;
        let bytes = self
            .desired_bytes
            .as_deref()
            .or(match &self.original {
                FileSnapshot::Present(bytes) => Some(bytes.as_slice()),
                FileSnapshot::Missing => None,
            })
            .unwrap_or_default();
        let (path, mut file) = create_sibling_temp(&self.target)?;
        file.write_all(bytes)
            .map_err(file_io_error(&path, "write"))?;
        file.flush().map_err(file_io_error(&path, "flush"))?;
        apply_staged_permissions(&file, self.original_metadata.as_ref(), self.executable)?;
        file.sync_all()
            .map_err(file_io_error(&path, "synchronize"))?;
        drop(file);
        if digest(&fs::read(&path).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to verify staged setup file {}: {error}",
                path.display()
            ))
        })?) != digest(bytes)
        {
            let _ = fs::remove_file(&path);
            return Err(ConnectionCommandError::runtime(format!(
                "staged setup bytes changed before commit: {}",
                path.display()
            )));
        }
        self.staged_path = Some(path);
        Ok(())
    }

    pub(super) fn validate_input(&self) -> Result<(), ConnectionCommandError> {
        let (current, _) = read_file_snapshot(&self.target)?;
        if current == self.original {
            Ok(())
        } else {
            Err(ConnectionCommandError::concurrent_modification(format!(
                "SETUP_CONCURRENT_MODIFICATION: setup target changed after planning: {}; external bytes were preserved",
                self.target.display()
            )))
        }
    }

    pub(super) fn commit(&mut self) -> Result<(), ConnectionCommandError> {
        self.validate_input()?;
        let staged_path = self.staged_path.as_ref().ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "setup mutation was not prepared: {}",
                self.target.display()
            ))
        })?;
        match (&self.original, self.desired_bytes.as_ref()) {
            (FileSnapshot::Missing, Some(_)) => {
                rename_no_replace(staged_path, &self.target).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to create setup target atomically at {}: {error}",
                        self.target.display()
                    ))
                })?;
            }
            (FileSnapshot::Present(_), Some(_)) => {
                let recovery =
                    replace_with_recovery(staged_path, &self.target).map_err(|error| {
                        ConnectionCommandError::runtime(format!(
                            "failed to replace setup target atomically at {}: {error}",
                            self.target.display()
                        ))
                    })?;
                self.staged_path = Some(recovery);
            }
            (FileSnapshot::Present(_), None) => {
                fs::remove_file(staged_path).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to release setup deletion staging path {}: {error}",
                        staged_path.display()
                    ))
                })?;
                rename_no_replace(&self.target, staged_path).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to retire managed setup target atomically at {}: {error}",
                        self.target.display()
                    ))
                })?;
            }
            (FileSnapshot::Missing, None) => {
                return Err(ConnectionCommandError::runtime(
                    "setup mutation unexpectedly planned a missing-file no-op",
                ));
            }
        }
        self.committed = true;
        if self.current_digest()? != self.staged_digest {
            return Err(ConnectionCommandError::concurrent_modification(format!(
                "SETUP_CONCURRENT_MODIFICATION: setup target changed during atomic replacement: {}; external bytes were preserved",
                self.target.display()
            )));
        }
        sync_parent(&self.target);
        Ok(())
    }

    pub(super) fn rollback(&mut self) -> Result<(), ConnectionCommandError> {
        if !self.committed {
            return self.cleanup_staging();
        }
        let staged_path = self.staged_path.as_ref().ok_or_else(|| {
            ConnectionCommandError::runtime("committed setup mutation lost its recovery entry")
        })?;
        match (&self.original, self.desired_bytes.as_ref()) {
            (FileSnapshot::Missing, Some(_)) => {
                if self.current_digest()? != self.staged_digest {
                    return Err(concurrent_rollback_error(
                        &self.target,
                        self.staged_path.as_deref(),
                    ));
                }
                rename_no_replace(&self.target, staged_path).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to roll back created setup target {}: {error}",
                        self.target.display()
                    ))
                })?;
                fs::remove_file(staged_path).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to remove rolled-back setup bytes {}: {error}",
                        staged_path.display()
                    ))
                })?;
            }
            (FileSnapshot::Present(_), Some(_)) => {
                if self.current_digest()? != self.staged_digest {
                    return Err(concurrent_rollback_error(
                        &self.target,
                        self.staged_path.as_deref(),
                    ));
                }
                let recovery =
                    replace_with_recovery(staged_path, &self.target).map_err(|error| {
                        ConnectionCommandError::runtime(format!(
                            "failed to restore setup target {}: {error}",
                            self.target.display()
                        ))
                    })?;
                self.staged_path = Some(recovery);
                if self.current_digest()? != self.original_digest {
                    return Err(ConnectionCommandError::runtime(format!(
                        "setup rollback could not verify restored bytes at {}",
                        self.target.display()
                    )));
                }
                let recovery = self
                    .staged_path
                    .as_ref()
                    .expect("recovery path was replaced");
                fs::remove_file(recovery).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to remove displaced setup bytes {}: {error}",
                        recovery.display()
                    ))
                })?;
                self.staged_path = None;
            }
            (FileSnapshot::Present(_), None) => {
                if self.target.try_exists().map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to inspect rollback target {}: {error}",
                        self.target.display()
                    ))
                })? {
                    return Err(concurrent_rollback_error(
                        &self.target,
                        self.staged_path.as_deref(),
                    ));
                }
                rename_no_replace(staged_path, &self.target).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to restore removed setup target {}: {error}",
                        self.target.display()
                    ))
                })?;
            }
            (FileSnapshot::Missing, None) => unreachable!("missing-file no-op is not planned"),
        }
        self.committed = false;
        sync_parent(&self.target);
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Result<(), ConnectionCommandError> {
        self.cleanup_staging()
    }

    fn cleanup_staging(&mut self) -> Result<(), ConnectionCommandError> {
        if let Some(path) = self.staged_path.take() {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(ConnectionCommandError::runtime(format!(
                        "failed to remove setup staging file {}: {error}",
                        path.display()
                    )));
                }
            }
        }
        Ok(())
    }

    fn current_digest(&self) -> Result<Option<String>, ConnectionCommandError> {
        let (snapshot, _) = read_file_snapshot(&self.target)?;
        Ok(match snapshot {
            FileSnapshot::Missing => None,
            FileSnapshot::Present(bytes) => Some(digest(&bytes)),
        })
    }
}

#[derive(Debug)]
pub(super) struct PreparedSetup<'mutation> {
    mutation_context: &'mutation RuntimeHomeMutationContext<'mutation>,
    pub(super) plan: SetupPlan,
    runtime_home_publication: SetupRuntimeHomePublication,
    pub(super) created_directories: Vec<PathBuf>,
    store_boundary: Option<PreparedStoreMutationBoundary<'mutation>>,
    pub(super) store_applied: bool,
    pub(super) created_project_home: Option<PathBuf>,
}

#[derive(Debug)]
enum SetupRuntimeHomePublication {
    ExistingReady,
    Prepared(PreparedRuntimeHome),
    NotPublished,
    OwnedPublished(RuntimeHomePublicationGuard),
    OwnedConfirmed(RuntimeHomePublicationGuard),
    OwnedPreserved(RuntimeHomePublicationGuard),
    OwnedRolledBack,
    OwnedRemovalIncomplete,
    OwnershipLost,
    Transitioning,
}

impl<'mutation> PreparedSetup<'mutation> {
    pub(super) fn prepare(
        mut plan: SetupPlan,
        context: &'mutation RuntimeHomeMutationContext<'mutation>,
    ) -> Result<Self, ConnectionCommandError> {
        if context.mode() != RuntimeHomeMutationLeaseMode::ExclusiveSetup {
            return Err(ConnectionCommandError::runtime(
                "setup preparation requires an exclusive Runtime Home mutation lease",
            ));
        }
        context.ensure_runtime_home(plan.runtime_home.final_path())?;
        let runtime_home_publication = match &plan.runtime_home {
            RuntimeHomePlan::Create {
                final_path,
                runtime_home_id,
                metadata_json,
                installation,
            } => SetupRuntimeHomePublication::Prepared(prepare_runtime_home_with_installation(
                context,
                final_path,
                runtime_home_id,
                metadata_json,
                installation.clone(),
            )?),
            RuntimeHomePlan::Validate { final_path } => {
                match inspect_runtime_home_bootstrap(final_path)? {
                    RuntimeHomeBootstrapState::Ready(_) => {
                        SetupRuntimeHomePublication::ExistingReady
                    }
                    RuntimeHomeBootstrapState::Absent => {
                        return Err(ConnectionCommandError::concurrent_modification(format!(
                            "SETUP_CONCURRENT_MODIFICATION: Runtime Home disappeared after planning: {}",
                            final_path.display()
                        )));
                    }
                    RuntimeHomeBootstrapState::Incompatible(mismatch) => {
                        return Err(ConnectionCommandError::from(
                            volicord_store::StoreError::RuntimeHomeSchemaMismatch(Box::new(
                                mismatch,
                            )),
                        ));
                    }
                    RuntimeHomeBootstrapState::Corrupt(corruption) => {
                        return Err(ConnectionCommandError::from(
                            volicord_store::StoreError::RuntimeHomeCorruption(corruption),
                        ));
                    }
                }
            }
        };

        let mut created_directories = Vec::new();
        let prepared_files = (|| {
            for mutation in plan
                .repository_file_mutations
                .iter_mut()
                .chain(plan.host_file_mutations.iter_mut())
            {
                mutation.prepare(&mut created_directories)?;
            }
            Ok::<(), ConnectionCommandError>(())
        })();
        if let Err(error) = prepared_files {
            cleanup_file_staging(&mut plan);
            remove_empty_directories(&created_directories);
            return Err(error);
        }

        let store_boundary = if matches!(plan.runtime_home, RuntimeHomePlan::Validate { .. }) {
            match PreparedStoreMutationBoundary::prepare(context, &plan.store_inputs) {
                Ok(boundary) => Some(boundary),
                Err(error) => {
                    cleanup_file_staging(&mut plan);
                    remove_empty_directories(&created_directories);
                    return Err(error.into());
                }
            }
        } else {
            None
        };

        Ok(Self {
            mutation_context: context,
            plan,
            runtime_home_publication,
            created_directories,
            store_boundary,
            store_applied: false,
            created_project_home: None,
        })
    }

    pub(super) fn validate_inputs(&self) -> Result<(), ConnectionCommandError> {
        match &self.plan.runtime_home {
            RuntimeHomePlan::Create { .. } => {}
            RuntimeHomePlan::Validate { final_path } => {
                if !matches!(
                    inspect_runtime_home_bootstrap(final_path)?,
                    RuntimeHomeBootstrapState::Ready(_)
                ) {
                    return Err(ConnectionCommandError::concurrent_modification(format!(
                        "SETUP_CONCURRENT_MODIFICATION: Runtime Home changed after setup preparation: {}",
                        final_path.display()
                    )));
                }
            }
        }
        for mutation in self
            .plan
            .repository_file_mutations
            .iter()
            .chain(self.plan.host_file_mutations.iter())
        {
            mutation.validate_input()?;
        }
        if let Some(boundary) = &self.store_boundary {
            boundary.validate_inputs()?;
        }
        Ok(())
    }

    pub(super) fn publish_runtime_home(&mut self) -> Result<(), ConnectionCommandError> {
        let state = std::mem::replace(
            &mut self.runtime_home_publication,
            SetupRuntimeHomePublication::Transitioning,
        );
        match state {
            SetupRuntimeHomePublication::Prepared(prepared) => {
                let final_path = prepared.final_path().to_path_buf();
                match commit_runtime_home(self.mutation_context, prepared) {
                    Ok(RuntimeHomePublicationOutcome::PublishedByThisInvocation {
                        publication,
                    }) => {
                        self.runtime_home_publication =
                            SetupRuntimeHomePublication::OwnedPublished(publication);
                    }
                    Ok(RuntimeHomePublicationOutcome::ObservedConcurrentWinner { .. }) => {
                        let observed = inspect_runtime_home_bootstrap(&final_path)?;
                        self.runtime_home_publication = SetupRuntimeHomePublication::NotPublished;
                        return Err(concurrent_runtime_home_modification(
                            &final_path,
                            bootstrap_state_name(&observed),
                        ));
                    }
                    Err(error) => {
                        self.runtime_home_publication = SetupRuntimeHomePublication::NotPublished;
                        if let Ok(observed) = inspect_runtime_home_bootstrap(&final_path) {
                            if !matches!(observed, RuntimeHomeBootstrapState::Absent) {
                                return Err(concurrent_runtime_home_modification(
                                    &final_path,
                                    bootstrap_state_name(&observed),
                                ));
                            }
                        }
                        return Err(error.into());
                    }
                }
            }
            other => self.runtime_home_publication = other,
        }
        Ok(())
    }

    pub(super) fn confirm_runtime_home(
        &mut self,
        mut phase_fault: impl FnMut(&str) -> Result<(), ConnectionCommandError>,
    ) -> Result<(), ConnectionCommandError> {
        let state = std::mem::replace(
            &mut self.runtime_home_publication,
            SetupRuntimeHomePublication::Transitioning,
        );
        let SetupRuntimeHomePublication::OwnedPublished(mut publication) = state else {
            self.runtime_home_publication = state;
            return Ok(());
        };
        let confirmed = (|| {
            phase_fault(FAULT_RUNTIME_HOME_PARENT_DIRECTORY_SYNC)?;
            publication.synchronize_parent_directory(self.mutation_context)?;
            phase_fault(FAULT_RUNTIME_HOME_PUBLICATION_READ_BACK)?;
            let _ = publication.read_back(self.mutation_context)?;
            phase_fault(FAULT_RUNTIME_HOME_PUBLICATION_MANIFEST_VALIDATION)?;
            let _ = publication.validate_manifest_and_confirm(self.mutation_context)?;
            Ok::<(), ConnectionCommandError>(())
        })();
        match confirmed {
            Ok(()) => {
                self.runtime_home_publication =
                    SetupRuntimeHomePublication::OwnedConfirmed(publication);
                Ok(())
            }
            Err(error) => {
                self.runtime_home_publication =
                    SetupRuntimeHomePublication::OwnedPublished(publication);
                Err(error)
            }
        }
    }

    pub(super) fn mark_store_applied(
        &mut self,
        created_project_home: Option<PathBuf>,
    ) -> Result<(), ConnectionCommandError> {
        if created_project_home.is_some() {
            self.created_project_home = created_project_home;
        }
        self.store_applied = true;
        if let Some(boundary) = &mut self.store_boundary {
            boundary.checkpoint()?;
        }
        Ok(())
    }

    pub(super) fn rollback_files(&mut self) -> RollbackSummary {
        let mut summary = RollbackSummary::default();
        for mutation in self
            .plan
            .host_file_mutations
            .iter_mut()
            .rev()
            .chain(self.plan.repository_file_mutations.iter_mut().rev())
        {
            let was_committed = mutation.committed;
            match mutation.rollback() {
                Ok(()) if was_committed => summary.rolled_back += 1,
                Ok(()) => {}
                Err(error) => {
                    summary.partially_rolled_back += 1;
                    summary.errors.push(error.to_string());
                }
            }
        }
        summary
    }

    pub(super) fn rollback_store(&mut self, summary: &mut RollbackSummary) {
        let state = std::mem::replace(
            &mut self.runtime_home_publication,
            SetupRuntimeHomePublication::Transitioning,
        );
        match state {
            SetupRuntimeHomePublication::OwnedPublished(mut publication)
            | SetupRuntimeHomePublication::OwnedConfirmed(mut publication)
            | SetupRuntimeHomePublication::OwnedPreserved(mut publication) => {
                match publication.rollback_if_owned(self.mutation_context) {
                    Ok(RuntimeHomePublicationRollbackOutcome::RolledBack {
                        durability,
                        failure,
                    }) => {
                        summary.runtime_home_rollback = Some(RuntimeHomeRollbackResult::Removed {
                            durability,
                            failure_phase: failure.as_ref().map(|error| error.phase),
                        });
                        if failure.is_some()
                            || durability == DirectoryEntryDurability::ParentSynchronizationFailed
                        {
                            summary.partially_rolled_back += 1;
                            summary.errors.push(format!(
                                "SETUP_PARTIAL_ROLLBACK: the owned Runtime Home publication was removed, but parent-directory durability was not confirmed ({})",
                                durability.as_str()
                            ));
                        } else {
                            summary.rolled_back += 1;
                        }
                        self.runtime_home_publication =
                            SetupRuntimeHomePublication::OwnedRolledBack;
                    }
                    Ok(RuntimeHomePublicationRollbackOutcome::AlreadyRolledBack { durability }) => {
                        summary.runtime_home_rollback = Some(RuntimeHomeRollbackResult::Removed {
                            durability,
                            failure_phase: None,
                        });
                        if durability == DirectoryEntryDurability::ParentSynchronizationFailed {
                            summary.partially_rolled_back += 1;
                            summary.errors.push(
                                "SETUP_PARTIAL_ROLLBACK: the owned Runtime Home publication was already removed, but parent-directory synchronization had failed"
                                    .to_owned(),
                            );
                        } else {
                            summary.rolled_back += 1;
                        }
                        self.runtime_home_publication =
                            SetupRuntimeHomePublication::OwnedRolledBack;
                    }
                    Ok(RuntimeHomePublicationRollbackOutcome::RemovalIncomplete { failure }) => {
                        summary.partially_rolled_back += 1;
                        summary.runtime_home_rollback =
                            Some(RuntimeHomeRollbackResult::RemovalIncomplete {
                                effect: failure.effect,
                                phase: failure.phase,
                                final_path: failure.target_state,
                            });
                        summary.errors.push(format!(
                            "SETUP_PARTIAL_ROLLBACK: Runtime Home removal was incomplete or uncertain (effect: {}, phase: {}, final path: {}): {}",
                            failure.effect.as_str(),
                            failure.phase.as_str(),
                            failure.target_state.as_str(),
                            failure.io_error()
                        ));
                        self.runtime_home_publication =
                            SetupRuntimeHomePublication::OwnedRemovalIncomplete;
                    }
                    Ok(RuntimeHomePublicationRollbackOutcome::Preserved { reason }) => {
                        summary.partially_rolled_back += 1;
                        summary.runtime_home_rollback =
                            Some(RuntimeHomeRollbackResult::Preserved {
                                reason: reason.as_str().to_owned(),
                            });
                        summary.errors.push(format!(
                            "SETUP_PARTIAL_ROLLBACK: the owned Runtime Home publication was preserved ({})",
                            reason.as_str()
                        ));
                        self.runtime_home_publication =
                            SetupRuntimeHomePublication::OwnedPreserved(publication);
                    }
                    Ok(RuntimeHomePublicationRollbackOutcome::OwnershipLost { reason }) => {
                        summary.partially_rolled_back += 1;
                        summary.runtime_home_rollback =
                            Some(RuntimeHomeRollbackResult::OwnershipLost {
                                reason: reason.as_str().to_owned(),
                            });
                        summary.errors.push(format!(
                            "SETUP_PARTIAL_ROLLBACK: Runtime Home publication ownership was lost ({})",
                            reason.as_str()
                        ));
                        self.runtime_home_publication = SetupRuntimeHomePublication::OwnershipLost;
                    }
                    Err(error) => {
                        summary.partially_rolled_back += 1;
                        summary.errors.push(error.to_string());
                        self.runtime_home_publication =
                            SetupRuntimeHomePublication::OwnedPublished(publication);
                    }
                }
                return;
            }
            other => self.runtime_home_publication = other,
        }
        if matches!(
            self.runtime_home_publication,
            SetupRuntimeHomePublication::OwnedRolledBack
                | SetupRuntimeHomePublication::OwnedRemovalIncomplete
                | SetupRuntimeHomePublication::OwnedPreserved(_)
                | SetupRuntimeHomePublication::OwnershipLost
        ) {
            return;
        }
        if !self.store_applied {
            return;
        }
        if let Some(boundary) = &mut self.store_boundary {
            let store = boundary.rollback();
            summary.rolled_back += store.restored;
            summary.partially_rolled_back += store.preserved;
            summary.errors.extend(store.errors);
        }
        if let Some(project_home) = self.created_project_home.take() {
            match remove_created_project_home(&project_home) {
                Ok(outcome) => {
                    debug_assert_eq!(outcome.effect, DirectoryTreeRemovalEffect::Removed);
                    summary.rolled_back += 1;
                }
                Err(error)
                    if error.effect == DirectoryTreeRemovalEffect::NotRemoved
                        && error.target_state == DirectoryTreeTargetState::Absent =>
                {
                    summary.rolled_back += 1;
                }
                Err(error) => {
                    summary.partially_rolled_back += 1;
                    summary.errors.push(format!(
                        "SETUP_PARTIAL_ROLLBACK: project Store cleanup effect {}, phase {}, final path {}, durability {}: {}",
                        error.effect.as_str(),
                        error.phase.as_str(),
                        error.target_state.as_str(),
                        error.durability.as_str(),
                        error.io_error()
                    ));
                }
            }
        }
    }

    pub(super) fn cleanup_after_success(&mut self) -> Result<(), ConnectionCommandError> {
        for mutation in self
            .plan
            .repository_file_mutations
            .iter_mut()
            .chain(self.plan.host_file_mutations.iter_mut())
        {
            mutation.finish()?;
        }
        if let Some(boundary) = &mut self.store_boundary {
            boundary.commit()?;
        }
        Ok(())
    }

    pub(super) fn has_committed_effects(&self) -> bool {
        matches!(
            self.runtime_home_publication,
            SetupRuntimeHomePublication::OwnedPublished(_)
                | SetupRuntimeHomePublication::OwnedConfirmed(_)
                | SetupRuntimeHomePublication::OwnedPreserved(_)
                | SetupRuntimeHomePublication::OwnedRemovalIncomplete
                | SetupRuntimeHomePublication::OwnershipLost
        ) || self.store_applied
            || self
                .plan
                .repository_file_mutations
                .iter()
                .chain(self.plan.host_file_mutations.iter())
                .any(|mutation| mutation.committed)
    }

    pub(super) fn externally_visible_managed_file_committed(&self) -> bool {
        self.plan
            .host_file_mutations
            .iter()
            .chain(self.plan.repository_file_mutations.iter())
            .any(|mutation| mutation.committed)
    }

    pub(super) fn preserve_created_runtime_home(&mut self) {
        let state = std::mem::replace(
            &mut self.runtime_home_publication,
            SetupRuntimeHomePublication::Transitioning,
        );
        self.runtime_home_publication = match state {
            SetupRuntimeHomePublication::OwnedPublished(mut publication)
            | SetupRuntimeHomePublication::OwnedConfirmed(mut publication) => {
                publication.preserve();
                SetupRuntimeHomePublication::OwnedPreserved(publication)
            }
            other => other,
        };
    }

    pub(super) fn publication_status(&self) -> RuntimeHomePublicationStatus {
        match self.runtime_home_publication {
            SetupRuntimeHomePublication::ExistingReady => {
                RuntimeHomePublicationStatus::ExistingReady
            }
            SetupRuntimeHomePublication::Prepared(_)
            | SetupRuntimeHomePublication::NotPublished
            | SetupRuntimeHomePublication::Transitioning => {
                RuntimeHomePublicationStatus::NotPublished
            }
            SetupRuntimeHomePublication::OwnedPublished(_)
            | SetupRuntimeHomePublication::OwnedConfirmed(_) => {
                RuntimeHomePublicationStatus::PublishedByThisInvocation
            }
            SetupRuntimeHomePublication::OwnedPreserved(_) => {
                RuntimeHomePublicationStatus::OwnedPublicationPreserved
            }
            SetupRuntimeHomePublication::OwnedRolledBack => {
                RuntimeHomePublicationStatus::OwnedPublicationRolledBack
            }
            SetupRuntimeHomePublication::OwnedRemovalIncomplete => {
                RuntimeHomePublicationStatus::OwnedPublicationRemovalIncomplete
            }
            SetupRuntimeHomePublication::OwnershipLost => {
                RuntimeHomePublicationStatus::OwnershipLostDuringRollback
            }
        }
    }
}

impl Drop for PreparedSetup<'_> {
    fn drop(&mut self) {
        cleanup_file_staging(&mut self.plan);
        remove_empty_directories(&self.created_directories);
    }
}

fn bootstrap_state_name(state: &RuntimeHomeBootstrapState) -> &'static str {
    match state {
        RuntimeHomeBootstrapState::Absent => "absent",
        RuntimeHomeBootstrapState::Ready(_) => "ready",
        RuntimeHomeBootstrapState::Incompatible(_) => "incompatible",
        RuntimeHomeBootstrapState::Corrupt(_) => "corrupt",
    }
}

fn concurrent_runtime_home_modification(
    final_path: &Path,
    observed_state: &str,
) -> ConnectionCommandError {
    ConnectionCommandError::concurrent_modification(format!(
        "SETUP_CONCURRENT_MODIFICATION: the Runtime Home target appeared while its setup lease was held (observed state: {observed_state}); the stale setup plan was aborted before Store or managed-file mutation; rerun setup against the current state: {}",
        final_path.display()
    ))
}

#[derive(Debug, Default, Clone)]
pub(super) struct RollbackSummary {
    pub(super) rolled_back: usize,
    pub(super) partially_rolled_back: usize,
    pub(super) errors: Vec<String>,
    pub(super) runtime_home_rollback: Option<RuntimeHomeRollbackResult>,
}

impl RollbackSummary {
    pub(super) fn is_complete(&self) -> bool {
        self.partially_rolled_back == 0
    }
}

fn cleanup_file_staging(plan: &mut SetupPlan) {
    for mutation in plan
        .repository_file_mutations
        .iter_mut()
        .chain(plan.host_file_mutations.iter_mut())
    {
        if !mutation.committed {
            let _ = mutation.cleanup_staging();
        }
    }
}

fn read_file_snapshot(
    target: &Path,
) -> Result<(FileSnapshot, Option<Permissions>), ConnectionCommandError> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_file() => {
            let bytes = fs::read(target).map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "failed to read setup target {}: {error}",
                    target.display()
                ))
            })?;
            Ok((FileSnapshot::Present(bytes), Some(metadata.permissions())))
        }
        Ok(_) => Err(ConnectionCommandError::runtime(format!(
            "setup target is not a regular file: {}",
            target.display()
        ))),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Ok((FileSnapshot::Missing, None))
        }
        Err(error) => Err(ConnectionCommandError::runtime(format!(
            "failed to inspect setup target {}: {error}",
            target.display()
        ))),
    }
}

fn validate_target_path_read_only(target: &Path) -> Result<(), ConnectionCommandError> {
    let parent = target.parent().ok_or_else(|| {
        ConnectionCommandError::runtime(format!(
            "setup target has no parent directory: {}",
            target.display()
        ))
    })?;
    let mut cursor = Some(parent);
    while let Some(path) = cursor {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_dir() => return Ok(()),
            Ok(_) => {
                return Err(ConnectionCommandError::runtime(format!(
                    "setup target parent is not a directory: {}",
                    path.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                cursor = path.parent();
            }
            Err(error) => {
                return Err(ConnectionCommandError::runtime(format!(
                    "failed to inspect setup target parent {}: {error}",
                    path.display()
                )));
            }
        }
    }
    Err(ConnectionCommandError::runtime(format!(
        "setup target has no accessible parent: {}",
        target.display()
    )))
}

fn create_parent_directories(
    target: &Path,
    created: &mut Vec<PathBuf>,
) -> Result<(), ConnectionCommandError> {
    let parent = target.parent().ok_or_else(|| {
        ConnectionCommandError::runtime(format!(
            "setup target has no parent directory: {}",
            target.display()
        ))
    })?;
    let mut missing = Vec::new();
    let mut cursor = parent;
    while !cursor.try_exists().map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "failed to inspect setup parent {}: {error}",
            cursor.display()
        ))
    })? {
        missing.push(cursor.to_path_buf());
        cursor = cursor.parent().ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "setup parent has no existing ancestor: {}",
                parent.display()
            ))
        })?;
    }
    for directory in missing.iter().rev() {
        fs::create_dir(directory).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to create setup staging directory {}: {error}",
                directory.display()
            ))
        })?;
        created.push(directory.clone());
    }
    Ok(())
}

fn create_sibling_temp(target: &Path) -> Result<(PathBuf, File), ConnectionCommandError> {
    let parent = target.parent().ok_or_else(|| {
        ConnectionCommandError::runtime(format!(
            "setup target has no parent directory: {}",
            target.display()
        ))
    })?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("managed");
    for attempt in 0..1024u32 {
        let path = parent.join(format!(
            ".{name}.volicord-setup-{}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ConnectionCommandError::runtime(format!(
                    "failed to create setup staging file {}: {error}",
                    path.display()
                )));
            }
        }
    }
    Err(ConnectionCommandError::runtime(format!(
        "failed to allocate setup staging file for {}",
        target.display()
    )))
}

#[cfg(unix)]
fn apply_staged_permissions(
    file: &File,
    original: Option<&OriginalMetadata>,
    executable: bool,
) -> Result<(), ConnectionCommandError> {
    use std::os::unix::fs::PermissionsExt;
    let mut mode = original
        .map(|metadata| metadata.permissions.mode())
        .unwrap_or(if executable { 0o755 } else { 0o600 });
    if executable {
        mode |= 0o111;
    }
    file.set_permissions(Permissions::from_mode(mode))
        .map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to apply setup staging permissions: {error}"
            ))
        })
}

#[cfg(not(unix))]
fn apply_staged_permissions(
    file: &File,
    original: Option<&OriginalMetadata>,
    _executable: bool,
) -> Result<(), ConnectionCommandError> {
    if let Some(metadata) = original {
        file.set_permissions(metadata.permissions.clone())
            .map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "failed to apply setup staging permissions: {error}"
                ))
            })?;
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn file_io_error<'a>(
    path: &'a Path,
    operation: &'static str,
) -> impl FnOnce(io::Error) -> ConnectionCommandError + 'a {
    move |error| {
        ConnectionCommandError::runtime(format!(
            "failed to {operation} setup staging file {}: {error}",
            path.display()
        ))
    }
}

fn concurrent_rollback_error(target: &Path, recovery: Option<&Path>) -> ConnectionCommandError {
    let recovery = recovery
        .map(|path| format!("; preserved recovery entry: {}", path.display()))
        .unwrap_or_default();
    ConnectionCommandError::runtime(format!(
        "SETUP_PARTIAL_ROLLBACK: setup target changed after commit and external bytes were preserved: {}{recovery}",
        target.display(),
    ))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn replace_with_recovery(replacement: &Path, target: &Path) -> io::Result<PathBuf> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};
    renameat_with(CWD, replacement, CWD, target, RenameFlags::EXCHANGE).map_err(io::Error::from)?;
    Ok(replacement.to_path_buf())
}

#[cfg(windows)]
fn replace_with_recovery(replacement: &Path, target: &Path) -> io::Result<PathBuf> {
    use volicord_platform_fs::{replace_file_with_backup, ReplaceFailureEffect};

    let (backup, reservation) =
        create_sibling_temp(target).map_err(|error| io::Error::other(error.to_string()))?;
    drop(reservation);
    match replace_file_with_backup(target, replacement, &backup) {
        Ok(()) => Ok(backup),
        Err(error) => {
            if error.effect() == ReplaceFailureEffect::ReplacedMovedToBackup
                && !target.try_exists()?
            {
                volicord_platform_fs::move_file_no_replace(&backup, target)?;
            }
            Err(io::Error::other(format!(
                "{error}; Windows recovery entry: {}",
                backup.display()
            )))
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn replace_with_recovery(_replacement: &Path, target: &Path) -> io::Result<PathBuf> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "atomic replacement with recovery is unsupported on this platform: {}",
            target.display()
        ),
    ))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_no_replace(source: &Path, target: &Path) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};
    renameat_with(CWD, source, CWD, target, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(windows)]
fn rename_no_replace(source: &Path, target: &Path) -> io::Result<()> {
    volicord_platform_fs::move_file_no_replace(source, target)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn rename_no_replace(source: &Path, target: &Path) -> io::Result<()> {
    fs::hard_link(source, target)?;
    fs::remove_file(source)
}

fn sync_parent(target: &Path) {
    if let Some(parent) = target.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
    }
}

fn remove_empty_directories(directories: &[PathBuf]) {
    for directory in directories.iter().rev() {
        let _ = fs::remove_dir(directory);
    }
}

fn remove_created_project_home(
    path: &Path,
) -> Result<DirectoryTreeRemovalOutcome, volicord_platform_fs::DirectoryTreeRemovalError> {
    volicord_platform_fs::remove_owned_directory_tree(path)
}

#[cfg(test)]
mod removal_tests {
    use tempfile::tempdir;
    use volicord_platform_fs::directory_tree_removal_test_support::{
        fail_next_directory_tree_removal, DirectoryTreeRemovalFault,
    };

    use super::*;

    #[test]
    fn created_project_home_cleanup_uses_effect_aware_removal() -> io::Result<()> {
        let root = tempdir()?;
        let project_home = root.path().join("project-home");
        fs::create_dir(&project_home)?;
        fs::write(project_home.join("state.sqlite"), b"fixture")?;

        let outcome = remove_created_project_home(&project_home)
            .map_err(|error| io::Error::other(error.to_string()))?;

        assert_eq!(outcome.effect, DirectoryTreeRemovalEffect::Removed);
        assert_eq!(outcome.target_state, DirectoryTreeTargetState::Absent);
        assert!(!project_home.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn created_project_home_cleanup_retains_removed_parent_sync_failure() -> io::Result<()> {
        let root = tempdir()?;
        let project_home = root.path().join("project-home");
        fs::create_dir(&project_home)?;
        fs::write(project_home.join("state.sqlite"), b"fixture")?;
        fail_next_directory_tree_removal(DirectoryTreeRemovalFault::ParentDirectorySyncFailure);

        let error = remove_created_project_home(&project_home)
            .expect_err("parent synchronization fault must be retained");

        assert_eq!(error.effect, DirectoryTreeRemovalEffect::Removed);
        assert_eq!(error.target_state, DirectoryTreeTargetState::Absent);
        assert!(!project_home.exists());
        Ok(())
    }
}
