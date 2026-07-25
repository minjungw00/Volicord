//! Recoverable file boundaries for a bounded administrative Store mutation set.
//!
//! Callers prepare recovery entries before opening mutating Store APIs, checkpoint
//! after each successful Store mutation group, and either commit the recovery
//! entries or roll back bytes that still match the last checkpoint.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{RuntimeHomeMutationContext, StoreError, StoreResult};

/// Prepared recovery boundary for existing Store database files.
#[derive(Debug)]
pub struct PreparedStoreMutationBoundary<'mutation> {
    context: &'mutation RuntimeHomeMutationContext<'mutation>,
    entries: Vec<StoreRecoveryEntry>,
}

/// Read-only digest captured for a Store file used during setup planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMutationInput {
    path: PathBuf,
    digest: String,
}

impl<'mutation> PreparedStoreMutationBoundary<'mutation> {
    /// Captures exact Store input digests without opening a writable database.
    pub fn inspect_inputs(paths: &[PathBuf]) -> StoreResult<Vec<StoreMutationInput>> {
        paths
            .iter()
            .map(|path| {
                let metadata = fs::symlink_metadata(path)?;
                if !metadata.file_type().is_file() {
                    return Err(store_conflict(
                        path,
                        "setup Store input is not a regular file",
                    ));
                }
                Ok(StoreMutationInput {
                    path: path.clone(),
                    digest: digest(&fs::read(path)?),
                })
            })
            .collect()
    }

    /// Revalidates planning snapshots without creating recovery entries.
    pub fn validate_planned_inputs(inputs: &[StoreMutationInput]) -> StoreResult<()> {
        for input in inputs {
            input.validate_current()?;
        }
        Ok(())
    }

    /// Copies every existing Store input to a same-directory recovery entry.
    pub fn prepare(
        context: &'mutation RuntimeHomeMutationContext<'mutation>,
        inputs: &[StoreMutationInput],
    ) -> StoreResult<Self> {
        context.require_exclusive_setup()?;
        let mut entries = Vec::with_capacity(inputs.len());
        for input in inputs {
            match StoreRecoveryEntry::prepare(input) {
                Ok(entry) => entries.push(entry),
                Err(error) => {
                    for entry in &mut entries {
                        let _ = entry.commit();
                    }
                    return Err(error);
                }
            }
        }
        Ok(Self { context, entries })
    }

    /// Revalidates that Store inputs still match their preparation snapshots.
    pub fn validate_inputs(&self) -> StoreResult<()> {
        self.context.require_exclusive_setup()?;
        for entry in &self.entries {
            entry.validate_original()?;
        }
        Ok(())
    }

    /// Records the exact bytes produced by the last successful mutation group.
    pub fn checkpoint(&mut self) -> StoreResult<()> {
        self.context.require_exclusive_setup()?;
        for entry in &mut self.entries {
            entry.checkpoint()?;
        }
        Ok(())
    }

    /// Restores checkpointed bytes when no later writer changed the Store file.
    pub fn rollback(&mut self) -> StoreMutationRollbackSummary {
        if let Err(error) = self.context.require_exclusive_setup() {
            return StoreMutationRollbackSummary {
                restored: 0,
                preserved: self.entries.len(),
                errors: vec![error.to_string()],
            };
        }
        let mut summary = StoreMutationRollbackSummary::default();
        for entry in self.entries.iter_mut().rev() {
            match entry.rollback() {
                Ok(()) => summary.restored += 1,
                Err(error) => {
                    summary.preserved += 1;
                    summary.errors.push(error.to_string());
                }
            }
        }
        summary
    }

    /// Discards recovery entries after the setup transaction commits.
    pub fn commit(&mut self) -> StoreResult<()> {
        self.context.require_exclusive_setup()?;
        for entry in &mut self.entries {
            entry.commit()?;
        }
        Ok(())
    }
}

impl StoreMutationInput {
    fn validate_current(&self) -> StoreResult<()> {
        let current = fs::read(&self.path)?;
        if digest(&current) == self.digest {
            Ok(())
        } else {
            Err(store_conflict(
                &self.path,
                "SETUP_CONCURRENT_MODIFICATION: Store input changed after planning",
            ))
        }
    }
}

impl Drop for PreparedStoreMutationBoundary<'_> {
    fn drop(&mut self) {
        for entry in &mut self.entries {
            entry.cleanup_if_safe();
        }
    }
}

/// Outcome of a best-effort Store rollback.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StoreMutationRollbackSummary {
    pub restored: usize,
    pub preserved: usize,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct StoreRecoveryEntry {
    target: PathBuf,
    backup: PathBuf,
    original_digest: String,
    checkpoint_digest: Option<String>,
    retain_recovery: bool,
}

impl StoreRecoveryEntry {
    fn prepare(input: &StoreMutationInput) -> StoreResult<Self> {
        let target = &input.path;
        let metadata = fs::symlink_metadata(target)?;
        if !metadata.file_type().is_file() {
            return Err(store_conflict(
                target,
                "setup Store input is not a regular file",
            ));
        }
        let bytes = fs::read(target)?;
        if digest(&bytes) != input.digest {
            return Err(store_conflict(
                target,
                "SETUP_CONCURRENT_MODIFICATION: Store input changed before recovery preparation",
            ));
        }
        let (backup, mut file) = create_sibling_temp(target)?;
        if let Err(error) = (|| -> io::Result<()> {
            file.write_all(&bytes)?;
            file.flush()?;
            file.set_permissions(metadata.permissions())?;
            file.sync_all()
        })() {
            let _ = fs::remove_file(&backup);
            return Err(StoreError::Io(error));
        }
        Ok(Self {
            target: target.to_path_buf(),
            backup,
            original_digest: digest(&bytes),
            checkpoint_digest: None,
            retain_recovery: false,
        })
    }

    fn validate_original(&self) -> StoreResult<()> {
        let current = fs::read(&self.target)?;
        if digest(&current) == self.original_digest {
            Ok(())
        } else {
            Err(store_conflict(
                &self.target,
                "SETUP_CONCURRENT_MODIFICATION: Store input changed after preparation",
            ))
        }
    }

    fn checkpoint(&mut self) -> StoreResult<()> {
        self.checkpoint_digest = Some(digest(&fs::read(&self.target)?));
        Ok(())
    }

    fn rollback(&mut self) -> StoreResult<()> {
        let current = match fs::read(&self.target) {
            Ok(bytes) => digest(&bytes),
            Err(error) => {
                self.retain_recovery = true;
                return Err(StoreError::Io(error));
            }
        };
        if self.checkpoint_digest.as_deref() != Some(current.as_str()) {
            self.retain_recovery = true;
            return Err(store_conflict(
                &self.target,
                format!(
                    "SETUP_PARTIAL_ROLLBACK: Store target changed after the last setup checkpoint and was preserved; recovery entry: {}",
                    self.backup.display()
                ),
            ));
        }
        self.backup = match replace_with_recovery(&self.backup, &self.target) {
            Ok(recovery) => recovery,
            Err(error) => {
                self.retain_recovery = true;
                return Err(StoreError::Io(error));
            }
        };
        let restored = match fs::read(&self.target) {
            Ok(bytes) => digest(&bytes),
            Err(error) => {
                self.retain_recovery = true;
                return Err(StoreError::Io(error));
            }
        };
        if restored != self.original_digest {
            self.retain_recovery = true;
            return Err(store_conflict(
                &self.target,
                "SETUP_PARTIAL_ROLLBACK: restored Store bytes could not be verified",
            ));
        }
        fs::remove_file(&self.backup)?;
        Ok(())
    }

    fn commit(&mut self) -> StoreResult<()> {
        match fs::remove_file(&self.backup) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StoreError::Io(error)),
        }
    }

    fn cleanup_if_safe(&mut self) {
        if !self.retain_recovery {
            let _ = self.commit();
        }
    }
}

fn create_sibling_temp(target: &Path) -> StoreResult<(PathBuf, File)> {
    let parent = target
        .parent()
        .ok_or_else(|| store_conflict(target, "setup Store input has no parent directory"))?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("store");
    for attempt in 0..1024u32 {
        let path = parent.join(format!(
            ".{name}.volicord-store-recovery-{}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(StoreError::Io(error)),
        }
    }
    Err(store_conflict(
        target,
        "could not allocate a same-directory Store recovery entry",
    ))
}

fn store_conflict(target: &Path, detail: impl Into<String>) -> StoreError {
    StoreError::Conflict {
        entity: "setup_store_file",
        id: target.display().to_string(),
        detail: detail.into(),
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
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
                "{error}; Windows Store recovery entry: {}",
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
            "atomic Store replacement with recovery is unsupported on this platform: {}",
            target.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutation::TestRuntimeHomeAdmission;

    #[test]
    fn checkpointed_store_bytes_restore_exactly() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let setup = TestRuntimeHomeAdmission::exclusive(root.path())?;
        let context = setup.context()?;
        let target = root.path().join("registry.sqlite");
        fs::write(&target, b"original")?;
        let inputs = PreparedStoreMutationBoundary::inspect_inputs(std::slice::from_ref(&target))?;
        let mut boundary = PreparedStoreMutationBoundary::prepare(&context, &inputs)?;
        boundary.validate_inputs()?;
        fs::write(&target, b"mutated")?;
        boundary.checkpoint()?;

        let summary = boundary.rollback();
        assert_eq!(summary.restored, 1);
        assert_eq!(summary.preserved, 0);
        assert_eq!(fs::read(&target)?, b"original");
        assert!(recovery_entries(root.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn later_store_writer_is_preserved_during_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let setup = TestRuntimeHomeAdmission::exclusive(root.path())?;
        let context = setup.context()?;
        let target = root.path().join("registry.sqlite");
        fs::write(&target, b"original")?;
        let inputs = PreparedStoreMutationBoundary::inspect_inputs(std::slice::from_ref(&target))?;
        let mut boundary = PreparedStoreMutationBoundary::prepare(&context, &inputs)?;
        fs::write(&target, b"setup")?;
        boundary.checkpoint()?;
        fs::write(&target, b"external")?;

        let summary = boundary.rollback();
        assert_eq!(summary.restored, 0);
        assert_eq!(summary.preserved, 1);
        assert_eq!(fs::read(&target)?, b"external");
        assert_eq!(recovery_entries(root.path())?.len(), 1);
        Ok(())
    }

    #[test]
    fn successful_commit_discards_plaintext_recovery_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let setup = TestRuntimeHomeAdmission::exclusive(root.path())?;
        let context = setup.context()?;
        let target = root.path().join("registry.sqlite");
        fs::write(&target, b"original")?;
        let inputs = PreparedStoreMutationBoundary::inspect_inputs(std::slice::from_ref(&target))?;
        let mut boundary = PreparedStoreMutationBoundary::prepare(&context, &inputs)?;
        fs::write(&target, b"committed")?;
        boundary.checkpoint()?;
        boundary.commit()?;

        assert_eq!(fs::read(&target)?, b"committed");
        assert!(recovery_entries(root.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn preparation_rejects_store_bytes_changed_after_planning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let setup = TestRuntimeHomeAdmission::exclusive(root.path())?;
        let context = setup.context()?;
        let target = root.path().join("registry.sqlite");
        fs::write(&target, b"planned")?;
        let inputs = PreparedStoreMutationBoundary::inspect_inputs(std::slice::from_ref(&target))?;
        fs::write(&target, b"external")?;

        let error = PreparedStoreMutationBoundary::prepare(&context, &inputs)
            .expect_err("stale Store input must be rejected");
        assert!(error.to_string().contains("SETUP_CONCURRENT_MODIFICATION"));
        assert_eq!(fs::read(&target)?, b"external");
        assert!(recovery_entries(root.path())?.is_empty());
        Ok(())
    }

    fn recovery_entries(root: &Path) -> io::Result<Vec<PathBuf>> {
        fs::read_dir(root)?
            .filter_map(|entry| match entry {
                Ok(entry)
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .contains(".volicord-store-recovery-") =>
                {
                    Some(Ok(entry.path()))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }
}
