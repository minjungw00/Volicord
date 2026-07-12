use std::{
    ffi::OsString,
    io::{self, Read, Seek, Write},
    path::{Component, Path, PathBuf},
};

#[cfg(test)]
use std::fs;

use cap_fs_ext::{
    ambient_authority, DirExt, FollowSymlinks, MetadataExt as PortableMetadataExt,
    OpenOptionsFollowExt,
};
use cap_std::fs::{
    Dir as CapabilityDir, File as CapabilityFile, Metadata as CapabilityMetadata,
    OpenOptions as CapabilityOpenOptions,
};
use serde_json::Value;

use crate::{
    guard_integration::audit::{
        is_volicord_codex_hook_config, script_is_executable, sha256_text, ManagedJsonProjection,
        HOOK_WRAPPER_MARKER,
    },
    host_integration::{
        contracts::{
            contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
        },
        HostIntegrationFileKind, HostKind, HostLifecyclePhase, REQUIRED_GUARD_PHASES,
    },
    managed_block::{self, ManagedBlockError},
};

use super::GuardIntegrationError;

pub(crate) const VOLICORD_POLICY_SCHEMA: &str = "volicord-policy-v1";
pub(crate) const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
pub(crate) const AGENTS_FILE: &str = "AGENTS.md";
pub(crate) const GUIDANCE_START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE -->";
pub(crate) const GUIDANCE_END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE -->";

#[derive(Debug, Clone)]
pub(crate) struct GeneratedFilePlan {
    pub(crate) kind: HostIntegrationFileKind,
    pub(crate) repo_root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) content: String,
    pub(crate) status: FilePlanStatus,
    pub(crate) write_kind: GeneratedFileWriteKind,
    pub(crate) target_snapshot: ManagedTargetSnapshot,
}

impl GeneratedFilePlan {
    pub(crate) fn policy_value(&self) -> Result<Value, GuardIntegrationError> {
        serde_json::from_str::<Value>(&self.content)
            .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManagedTargetSnapshot {
    Missing,
    RegularFile(ManagedRegularFileSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedRegularFileSnapshot {
    text: String,
    identity: ManagedFileIdentity,
    len: u64,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    uid: u32,
    #[cfg(unix)]
    gid: u32,
    #[cfg(unix)]
    extended_attributes: Vec<(OsString, Vec<u8>)>,
    #[cfg(not(unix))]
    readonly: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecoveryEntrySnapshot {
    Missing,
    Regular(RecoveryRegularFileSnapshot),
    Other {
        identity: ManagedFileIdentity,
        kind: RecoveryEntryKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryRegularFileSnapshot {
    bytes: Vec<u8>,
    identity: ManagedFileIdentity,
    len: u64,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    uid: u32,
    #[cfg(unix)]
    gid: u32,
    #[cfg(unix)]
    extended_attributes: Vec<(OsString, Vec<u8>)>,
    #[cfg(not(unix))]
    readonly: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryEntryKind {
    Symlink,
    Directory,
    Other,
}

impl From<&ManagedTargetSnapshot> for RecoveryEntrySnapshot {
    fn from(snapshot: &ManagedTargetSnapshot) -> Self {
        match snapshot {
            ManagedTargetSnapshot::Missing => Self::Missing,
            ManagedTargetSnapshot::RegularFile(file) => {
                Self::Regular(RecoveryRegularFileSnapshot {
                    bytes: file.text.as_bytes().to_vec(),
                    identity: file.identity,
                    len: file.len,
                    #[cfg(unix)]
                    mode: file.mode,
                    #[cfg(unix)]
                    uid: file.uid,
                    #[cfg(unix)]
                    gid: file.gid,
                    #[cfg(unix)]
                    extended_attributes: file.extended_attributes.clone(),
                    #[cfg(not(unix))]
                    readonly: file.readonly,
                })
            }
        }
    }
}

impl ManagedTargetSnapshot {
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Self::Missing => None,
            Self::RegularFile(file) => Some(&file.text),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManagedFileIdentity {
    device: u64,
    inode: u64,
}

impl ManagedFileIdentity {
    fn from_metadata(metadata: &CapabilityMetadata) -> Self {
        Self {
            device: PortableMetadataExt::dev(metadata),
            inode: PortableMetadataExt::ino(metadata),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeneratedFileWriteKind {
    Block {
        start_marker: &'static str,
        end_marker: &'static str,
        require_existing_marker: bool,
    },
    Json,
    ExactJson,
    JsonProjection {
        projection: ManagedJsonProjection,
    },
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilePlanStatus {
    PlannedCreate,
    PlannedUpdate,
    Unchanged,
    Created,
    Updated,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedFileRetirementPlan {
    pub(crate) kind: HostIntegrationFileKind,
    pub(crate) repo_root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) status: RetirementPlanStatus,
    target_snapshot: ManagedTargetSnapshot,
    replacement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetirementPlanStatus {
    PlannedRemove,
    PlannedUpdate,
    Unchanged,
    Removed,
    Updated,
}

impl RetirementPlanStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PlannedRemove => "planned_remove",
            Self::PlannedUpdate => "planned_update",
            Self::Unchanged => "unchanged",
            Self::Removed => "removed",
            Self::Updated => "updated",
        }
    }
}

impl FilePlanStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PlannedCreate => "planned_create",
            Self::PlannedUpdate => "planned_update",
            Self::Unchanged => "unchanged",
            Self::Created => "created",
            Self::Updated => "updated",
        }
    }
}

pub(crate) fn ensure_generated_file_plan_fresh(
    plan: &GeneratedFilePlan,
) -> Result<(), GuardIntegrationError> {
    let current = read_managed_target_snapshot(&plan.repo_root, &plan.path)?;
    if current == plan.target_snapshot {
        Ok(())
    } else {
        Err(stale_managed_file_error(&plan.path))
    }
}

pub(crate) fn write_managed_file_if_fresh(
    plan: &GeneratedFilePlan,
    content: &str,
    executable: bool,
) -> Result<(), GuardIntegrationError> {
    write_managed_file_if_fresh_with_hook(plan, content, executable, |_| Ok(()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedWritePhase {
    TempReady,
    CommitReady,
    #[cfg(windows)]
    CommitReserved,
    #[cfg(windows)]
    NativeCommitReady,
    CommitApplied,
    RollbackInspecting,
    RollbackReady,
}

fn write_managed_file_if_fresh_with_hook<F>(
    plan: &GeneratedFilePlan,
    content: &str,
    executable: bool,
    mut hook: F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    ensure_generated_file_plan_fresh(plan)?;
    let parent = match open_pinned_managed_parent(&plan.repo_root, &plan.path, true)? {
        PinnedParentOpen::Ready(parent) => parent,
        PinnedParentOpen::Missing => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to create managed parent for {}",
                plan.path.display()
            )));
        }
    };
    parent.validate_attached()?;
    ensure_expected_snapshot(&parent, &plan.target_snapshot)?;

    let (temp_name, mut temp_file) = parent.create_temp_file()?;
    let temp_path = parent.absolute_entry_path(&temp_name);
    let write_result = (|| -> io::Result<TempPermissionPlan> {
        let permissions = prepare_temp_permissions(
            &temp_file,
            &plan.target_snapshot,
            executable,
            plan.kind == HostIntegrationFileKind::VolicordPolicy,
        )?;
        hook(ManagedWritePhase::TempReady)?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;
        apply_final_temp_permissions(&temp_file, &permissions)?;
        temp_file.sync_all()?;
        Ok(permissions)
    })();
    let permission_plan = match write_result {
        Ok(permission_plan) => permission_plan,
        Err(error) => {
            cleanup_temp_from_open_handle(&parent, &temp_name, temp_file).map_err(
                |cleanup_error| {
                    GuardIntegrationError::runtime(format!(
                        "temporary file preparation failed ({error}); cleanup also failed: {cleanup_error}"
                    ))
                },
            )?;
            return Err(GuardIntegrationError::runtime(format!(
                "failed to write temporary managed file {}: {error}",
                temp_path.display()
            )));
        }
    };

    let staged_snapshot = match read_open_managed_snapshot(&mut temp_file, &temp_path) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            drop(temp_file);
            return Err(recovery_residual_error(
                &parent,
                &[&temp_name],
                &format!("temporary entry could not be verified: {error}"),
            ));
        }
    };
    if staged_snapshot.text() != Some(content)
        || !staged_metadata_matches_plan(&staged_snapshot, &permission_plan)
    {
        drop(temp_file);
        return Err(recovery_residual_error(
            &parent,
            &[&temp_name],
            "temporary content or metadata changed before commit",
        ));
    }
    ensure_staged_entry(&parent, &temp_name, &staged_snapshot)?;

    if let Err(error) = ensure_expected_snapshot(&parent, &plan.target_snapshot) {
        cleanup_uncommitted_temp(&parent, &temp_name, &staged_snapshot)?;
        return Err(error);
    }
    if let Err(error) = run_write_hook(&mut hook, ManagedWritePhase::CommitReady, &plan.path) {
        cleanup_uncommitted_temp(&parent, &temp_name, &staged_snapshot)?;
        return Err(error);
    }
    if let Err(error) = parent.validate_attached() {
        cleanup_uncommitted_temp(&parent, &temp_name, &staged_snapshot)?;
        return Err(error);
    }
    ensure_staged_entry(&parent, &temp_name, &staged_snapshot)?;
    #[cfg(windows)]
    drop(temp_file);
    let result = atomic_commit_if_fresh(
        &parent,
        &temp_name,
        &plan.target_snapshot,
        &staged_snapshot,
        &mut hook,
    );
    #[cfg(not(windows))]
    drop(temp_file);
    result
}

fn read_managed_target_snapshot(
    repo_root: &Path,
    target: &Path,
) -> Result<ManagedTargetSnapshot, GuardIntegrationError> {
    match open_pinned_managed_parent(repo_root, target, false)? {
        PinnedParentOpen::Missing => Ok(ManagedTargetSnapshot::Missing),
        PinnedParentOpen::Ready(parent) => {
            parent.validate_attached()?;
            parent.read_target_snapshot()
        }
    }
}

fn run_write_hook<F>(
    hook: &mut F,
    phase: ManagedWritePhase,
    target: &Path,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    hook(phase).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "managed file commit hook failed for {}: {error}",
            target.display()
        ))
    })
}

fn ensure_expected_snapshot(
    parent: &PinnedManagedParent,
    expected: &ManagedTargetSnapshot,
) -> Result<(), GuardIntegrationError> {
    if &parent.read_target_snapshot()? == expected {
        Ok(())
    } else {
        Err(stale_managed_file_error(&parent.target_path))
    }
}

struct PinnedDirectory {
    dir: CapabilityDir,
    name_in_parent: Option<OsString>,
    identity: ManagedFileIdentity,
    display_path: PathBuf,
}

struct PinnedManagedParent {
    chain: Vec<PinnedDirectory>,
    target_name: OsString,
    target_path: PathBuf,
    parent_path: PathBuf,
}

enum PinnedParentOpen {
    Missing,
    Ready(PinnedManagedParent),
}

impl PinnedManagedParent {
    fn dir(&self) -> &CapabilityDir {
        &self
            .chain
            .last()
            .expect("a pinned parent contains the repository root")
            .dir
    }

    fn validate_attached(&self) -> Result<(), GuardIntegrationError> {
        for pair in self.chain.windows(2) {
            let parent = &pair[0];
            let child = &pair[1];
            let name = child
                .name_in_parent
                .as_ref()
                .expect("only the ambient anchor lacks a parent entry name");
            let metadata = parent.dir.symlink_metadata(name).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "failed to revalidate managed directory {}: {error}",
                    child.display_path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(managed_path_conflict(
                    &child.display_path,
                    "a pinned parent component was replaced",
                ));
            }
            if ManagedFileIdentity::from_metadata(&metadata) != child.identity {
                return Err(managed_path_conflict(
                    &child.display_path,
                    "a pinned parent component changed identity",
                ));
            }
        }
        Ok(())
    }

    fn read_target_snapshot(&self) -> Result<ManagedTargetSnapshot, GuardIntegrationError> {
        self.read_entry_snapshot(&self.target_name, &self.target_path)
    }

    fn read_entry_snapshot(
        &self,
        name: &OsString,
        display_path: &Path,
    ) -> Result<ManagedTargetSnapshot, GuardIntegrationError> {
        let named_metadata = match self.dir().symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(ManagedTargetSnapshot::Missing);
            }
            Err(error) => {
                return Err(GuardIntegrationError::runtime(format!(
                    "failed to inspect managed path {}: {error}",
                    display_path.display()
                )));
            }
        };
        if named_metadata.file_type().is_symlink() {
            return Err(managed_path_conflict(
                display_path,
                "symbolic links are not allowed",
            ));
        }
        if !named_metadata.is_file() {
            return Err(managed_path_conflict(
                display_path,
                "target is not a regular file",
            ));
        }

        let mut options = CapabilityOpenOptions::new();
        options.read(true);
        options.follow(FollowSymlinks::No);
        let mut file = self.dir().open_with(name, &options).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to open managed file {} without following links: {error}",
                display_path.display()
            ))
        })?;
        let before = file.metadata().map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to inspect opened managed file {}: {error}",
                display_path.display()
            ))
        })?;
        if !before.is_file() {
            return Err(managed_path_conflict(
                display_path,
                "opened target is not a regular file",
            ));
        }
        #[cfg(unix)]
        let extended_attributes_before = read_extended_attributes(&file).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to inspect extended attributes for managed file {}: {error}",
                display_path.display()
            ))
        })?;

        let mut first = String::new();
        file.read_to_string(&mut first).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to read managed file {}: {error}",
                display_path.display()
            ))
        })?;
        file.rewind().map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to rewind managed file {}: {error}",
                display_path.display()
            ))
        })?;
        let mut second = String::new();
        file.read_to_string(&mut second).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to re-read managed file {}: {error}",
                display_path.display()
            ))
        })?;
        let after = file.metadata().map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to re-inspect opened managed file {}: {error}",
                display_path.display()
            ))
        })?;
        #[cfg(unix)]
        let extended_attributes_after = read_extended_attributes(&file).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to re-inspect extended attributes for managed file {}: {error}",
                display_path.display()
            ))
        })?;
        let named_after = self.dir().symlink_metadata(name).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to revalidate managed file {}: {error}",
                display_path.display()
            ))
        })?;
        #[cfg(unix)]
        let extended_attributes_stable = extended_attributes_before == extended_attributes_after;
        #[cfg(not(unix))]
        let extended_attributes_stable = true;
        if first != second
            || !stable_file_metadata(&before, &after)
            || !extended_attributes_stable
            || named_after.file_type().is_symlink()
            || !named_after.is_file()
            || ManagedFileIdentity::from_metadata(&named_after)
                != ManagedFileIdentity::from_metadata(&after)
        {
            return Err(managed_path_conflict(
                display_path,
                "target changed while it was inspected",
            ));
        }
        Ok(ManagedTargetSnapshot::RegularFile(regular_file_snapshot(
            second,
            &after,
            #[cfg(unix)]
            extended_attributes_after,
        )))
    }

    fn read_recovery_target_snapshot(
        &self,
    ) -> Result<RecoveryEntrySnapshot, GuardIntegrationError> {
        self.read_recovery_entry_snapshot(&self.target_name, &self.target_path)
    }

    fn read_recovery_entry_snapshot(
        &self,
        name: &OsString,
        display_path: &Path,
    ) -> Result<RecoveryEntrySnapshot, GuardIntegrationError> {
        for _ in 0..4 {
            if let Some(snapshot) = self.try_read_recovery_entry_snapshot(name, display_path)? {
                return Ok(snapshot);
            }
        }
        Err(managed_path_conflict(
            display_path,
            "entry changed repeatedly while recovery state was inspected",
        ))
    }

    fn try_read_recovery_entry_snapshot(
        &self,
        name: &OsString,
        display_path: &Path,
    ) -> Result<Option<RecoveryEntrySnapshot>, GuardIntegrationError> {
        let named_metadata = match self.dir().symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Some(RecoveryEntrySnapshot::Missing));
            }
            Err(error) => {
                return Err(GuardIntegrationError::runtime(format!(
                    "failed to inspect recovery entry {}: {error}",
                    display_path.display()
                )));
            }
        };
        if !named_metadata.is_file() || named_metadata.file_type().is_symlink() {
            return Ok(Some(recovery_other_snapshot(&named_metadata)));
        }

        let mut options = CapabilityOpenOptions::new();
        options.read(true);
        options.follow(FollowSymlinks::No);
        let mut file = match self.dir().open_with(name, &options) {
            Ok(file) => file,
            Err(_) => return Ok(None),
        };
        let before = file.metadata().map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to inspect opened recovery entry {}: {error}",
                display_path.display()
            ))
        })?;
        #[cfg(unix)]
        let extended_attributes_before = read_extended_attributes(&file).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to inspect recovery entry metadata {}: {error}",
                display_path.display()
            ))
        })?;
        let mut first = Vec::new();
        file.read_to_end(&mut first).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to read recovery entry {}: {error}",
                display_path.display()
            ))
        })?;
        file.rewind().map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to rewind recovery entry {}: {error}",
                display_path.display()
            ))
        })?;
        let mut second = Vec::new();
        file.read_to_end(&mut second).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to re-read recovery entry {}: {error}",
                display_path.display()
            ))
        })?;
        let after = file.metadata().map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to re-inspect recovery entry {}: {error}",
                display_path.display()
            ))
        })?;
        #[cfg(unix)]
        let extended_attributes_after = read_extended_attributes(&file).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to re-inspect recovery entry metadata {}: {error}",
                display_path.display()
            ))
        })?;
        let named_after = match self.dir().symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Some(RecoveryEntrySnapshot::Missing));
            }
            Err(error) => {
                return Err(GuardIntegrationError::runtime(format!(
                    "failed to re-inspect recovery entry {}: {error}",
                    display_path.display()
                )));
            }
        };
        if !named_after.is_file() || named_after.file_type().is_symlink() {
            return Ok(Some(recovery_other_snapshot(&named_after)));
        }
        #[cfg(unix)]
        let extended_attributes_stable = extended_attributes_before == extended_attributes_after;
        #[cfg(not(unix))]
        let extended_attributes_stable = true;
        if first != second
            || !stable_file_metadata(&before, &after)
            || !extended_attributes_stable
            || ManagedFileIdentity::from_metadata(&named_after)
                != ManagedFileIdentity::from_metadata(&after)
        {
            return Ok(None);
        }
        Ok(Some(RecoveryEntrySnapshot::Regular(
            recovery_regular_file_snapshot(
                second,
                &after,
                #[cfg(unix)]
                extended_attributes_after,
            ),
        )))
    }

    fn create_temp_file(&self) -> Result<(OsString, CapabilityFile), GuardIntegrationError> {
        self.create_private_sibling_file("tmp")
    }

    fn create_private_sibling_file(
        &self,
        role: &str,
    ) -> Result<(OsString, CapabilityFile), GuardIntegrationError> {
        let target_name = self.target_name.to_string_lossy();
        for _ in 0..64 {
            let token = random_file_token()?;
            let name = OsString::from(format!(".{target_name}.volicord-{role}-{token}"));
            let mut options = CapabilityOpenOptions::new();
            options.read(true).write(true).create_new(true);
            options.follow(FollowSymlinks::No);
            match self.dir().open_with(&name, &options) {
                Ok(file) => return Ok((name, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(GuardIntegrationError::runtime(format!(
                        "failed to create private managed-file {role} entry {}: {error}",
                        self.absolute_entry_path(&name).display()
                    )));
                }
            }
        }
        Err(GuardIntegrationError::runtime(format!(
            "failed to allocate a private managed-file {role} entry for {}",
            self.target_path.display()
        )))
    }

    fn absolute_entry_path(&self, name: &OsString) -> PathBuf {
        self.parent_path.join(name)
    }

    fn sync_directory(&self) {
        if let Ok(dir) = self.dir().try_clone() {
            let _ = dir.into_std_file().sync_all();
        }
    }
}

fn open_pinned_managed_parent(
    repo_root: &Path,
    target: &Path,
    create: bool,
) -> Result<PinnedParentOpen, GuardIntegrationError> {
    let mut components = managed_target_components(repo_root, target)?;
    let target_name = components
        .pop()
        .expect("managed targets always have a final component");
    let repo_name = repo_root.file_name().ok_or_else(|| {
        managed_path_conflict(
            repo_root,
            "Product Repository root must name a directory below its parent",
        )
    })?;
    let repo_parent_path = match repo_root.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let ambient_parent = CapabilityDir::open_ambient_dir(repo_parent_path, ambient_authority())
        .map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to open Product Repository parent {}: {error}",
                repo_parent_path.display()
            ))
        })?;
    let ambient_identity = directory_identity(&ambient_parent, repo_parent_path)?;
    let repo_metadata = ambient_parent
        .symlink_metadata(repo_name)
        .map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to inspect Product Repository root {}: {error}",
                repo_root.display()
            ))
        })?;
    if repo_metadata.file_type().is_symlink() {
        return Err(managed_path_conflict(
            repo_root,
            "Product Repository root must not be a symbolic link",
        ));
    }
    if !repo_metadata.is_dir() {
        return Err(managed_path_conflict(
            repo_root,
            "Product Repository root is not a directory",
        ));
    }
    let repo_dir = ambient_parent
        .open_dir_nofollow(repo_name)
        .map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to pin Product Repository root {} without following links: {error}",
                repo_root.display()
            ))
        })?;
    let repo_identity = directory_identity(&repo_dir, repo_root)?;
    let mut chain = vec![
        PinnedDirectory {
            dir: ambient_parent,
            name_in_parent: None,
            identity: ambient_identity,
            display_path: repo_parent_path.to_path_buf(),
        },
        PinnedDirectory {
            dir: repo_dir,
            name_in_parent: Some(repo_name.to_os_string()),
            identity: repo_identity,
            display_path: repo_root.to_path_buf(),
        },
    ];
    let mut current_path = repo_root.to_path_buf();
    for component in components {
        current_path.push(&component);
        let open_result = chain
            .last()
            .expect("repository root was pinned")
            .dir
            .open_dir_nofollow(&component);
        let child = match open_result {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound && !create => {
                return Ok(PinnedParentOpen::Missing);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound && create => {
                let parent = &chain.last().expect("repository root was pinned").dir;
                match parent.create_dir(&component) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => {
                        return Err(GuardIntegrationError::runtime(format!(
                            "failed to create managed directory {}: {error}",
                            current_path.display()
                        )));
                    }
                }
                parent.open_dir_nofollow(&component).map_err(|error| {
                    managed_directory_open_error(parent, &component, &current_path, error)
                })?
            }
            Err(error) => {
                let parent = &chain.last().expect("repository root was pinned").dir;
                return Err(managed_directory_open_error(
                    parent,
                    &component,
                    &current_path,
                    error,
                ));
            }
        };
        let identity = directory_identity(&child, &current_path)?;
        chain.push(PinnedDirectory {
            dir: child,
            name_in_parent: Some(component),
            identity,
            display_path: current_path.clone(),
        });
    }
    let parent_path = target
        .parent()
        .ok_or_else(|| managed_path_conflict(target, "target does not have a parent directory"))?
        .to_path_buf();
    Ok(PinnedParentOpen::Ready(PinnedManagedParent {
        chain,
        target_name,
        target_path: target.to_path_buf(),
        parent_path,
    }))
}

fn directory_identity(
    directory: &CapabilityDir,
    display_path: &Path,
) -> Result<ManagedFileIdentity, GuardIntegrationError> {
    let metadata = directory.metadata(".").map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to inspect pinned directory {}: {error}",
            display_path.display()
        ))
    })?;
    if !metadata.is_dir() {
        return Err(managed_path_conflict(
            display_path,
            "pinned parent component is not a directory",
        ));
    }
    Ok(ManagedFileIdentity::from_metadata(&metadata))
}

fn managed_directory_open_error(
    parent: &CapabilityDir,
    component: &OsString,
    display_path: &Path,
    error: io::Error,
) -> GuardIntegrationError {
    match parent.symlink_metadata(component) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            managed_path_conflict(display_path, "symbolic links are not allowed")
        }
        Ok(_) => managed_path_conflict(display_path, "a parent component is not a directory"),
        Err(_) => GuardIntegrationError::runtime(format!(
            "failed to open managed directory {}: {error}",
            display_path.display()
        )),
    }
}

fn managed_target_components(
    repo_root: &Path,
    target: &Path,
) -> Result<Vec<OsString>, GuardIntegrationError> {
    let relative = target
        .strip_prefix(repo_root)
        .map_err(|_| managed_path_conflict(target, "target is outside the Product Repository"))?;
    let mut components = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(component) => components.push(component.to_os_string()),
            _ => {
                return Err(managed_path_conflict(
                    target,
                    "target contains a non-descendant path component",
                ));
            }
        }
    }
    if components.is_empty() {
        return Err(managed_path_conflict(
            target,
            "target must be below the Product Repository root",
        ));
    }
    Ok(components)
}

#[cfg(unix)]
fn regular_file_snapshot(
    text: String,
    metadata: &CapabilityMetadata,
    extended_attributes: Vec<(OsString, Vec<u8>)>,
) -> ManagedRegularFileSnapshot {
    use cap_std::fs::{MetadataExt as UnixMetadataExt, PermissionsExt};

    ManagedRegularFileSnapshot {
        text,
        identity: ManagedFileIdentity::from_metadata(metadata),
        len: metadata.len(),
        mode: metadata.permissions().mode(),
        uid: UnixMetadataExt::uid(metadata),
        gid: UnixMetadataExt::gid(metadata),
        extended_attributes,
    }
}

#[cfg(not(unix))]
fn regular_file_snapshot(
    text: String,
    metadata: &CapabilityMetadata,
) -> ManagedRegularFileSnapshot {
    ManagedRegularFileSnapshot {
        text,
        identity: ManagedFileIdentity::from_metadata(metadata),
        len: metadata.len(),
        readonly: metadata.permissions().readonly(),
    }
}

#[cfg(unix)]
fn recovery_regular_file_snapshot(
    bytes: Vec<u8>,
    metadata: &CapabilityMetadata,
    extended_attributes: Vec<(OsString, Vec<u8>)>,
) -> RecoveryRegularFileSnapshot {
    use cap_std::fs::{MetadataExt as UnixMetadataExt, PermissionsExt};

    RecoveryRegularFileSnapshot {
        bytes,
        identity: ManagedFileIdentity::from_metadata(metadata),
        len: metadata.len(),
        mode: metadata.permissions().mode(),
        uid: UnixMetadataExt::uid(metadata),
        gid: UnixMetadataExt::gid(metadata),
        extended_attributes,
    }
}

#[cfg(not(unix))]
fn recovery_regular_file_snapshot(
    bytes: Vec<u8>,
    metadata: &CapabilityMetadata,
) -> RecoveryRegularFileSnapshot {
    RecoveryRegularFileSnapshot {
        bytes,
        identity: ManagedFileIdentity::from_metadata(metadata),
        len: metadata.len(),
        readonly: metadata.permissions().readonly(),
    }
}

fn recovery_other_snapshot(metadata: &CapabilityMetadata) -> RecoveryEntrySnapshot {
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        RecoveryEntryKind::Symlink
    } else if file_type.is_dir() {
        RecoveryEntryKind::Directory
    } else {
        RecoveryEntryKind::Other
    };
    RecoveryEntrySnapshot::Other {
        identity: ManagedFileIdentity::from_metadata(metadata),
        kind,
    }
}

fn read_open_managed_snapshot(
    file: &mut CapabilityFile,
    display_path: &Path,
) -> Result<ManagedTargetSnapshot, GuardIntegrationError> {
    file.rewind().map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to rewind temporary managed file {}: {error}",
            display_path.display()
        ))
    })?;
    let before = file.metadata().map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to inspect temporary managed file {}: {error}",
            display_path.display()
        ))
    })?;
    if !before.is_file() {
        return Err(managed_path_conflict(
            display_path,
            "opened temporary entry is not a regular file",
        ));
    }
    #[cfg(unix)]
    let extended_attributes_before = read_extended_attributes(file).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to inspect temporary managed file metadata {}: {error}",
            display_path.display()
        ))
    })?;

    let mut first = String::new();
    file.read_to_string(&mut first).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to read temporary managed file {}: {error}",
            display_path.display()
        ))
    })?;
    file.rewind().map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to rewind temporary managed file {}: {error}",
            display_path.display()
        ))
    })?;
    let mut second = String::new();
    file.read_to_string(&mut second).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to re-read temporary managed file {}: {error}",
            display_path.display()
        ))
    })?;
    let after = file.metadata().map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to re-inspect temporary managed file {}: {error}",
            display_path.display()
        ))
    })?;
    #[cfg(unix)]
    let extended_attributes_after = read_extended_attributes(file).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to re-inspect temporary managed file metadata {}: {error}",
            display_path.display()
        ))
    })?;
    #[cfg(unix)]
    let extended_attributes_stable = extended_attributes_before == extended_attributes_after;
    #[cfg(not(unix))]
    let extended_attributes_stable = true;
    if first != second || !stable_file_metadata(&before, &after) || !extended_attributes_stable {
        return Err(managed_path_conflict(
            display_path,
            "temporary entry changed while it was inspected",
        ));
    }
    Ok(ManagedTargetSnapshot::RegularFile(regular_file_snapshot(
        second,
        &after,
        #[cfg(unix)]
        extended_attributes_after,
    )))
}

fn ensure_staged_entry(
    parent: &PinnedManagedParent,
    name: &OsString,
    staged: &ManagedTargetSnapshot,
) -> Result<(), GuardIntegrationError> {
    let expected = RecoveryEntrySnapshot::from(staged);
    let current = parent.read_recovery_entry_snapshot(name, &parent.absolute_entry_path(name))?;
    if current == expected {
        Ok(())
    } else {
        Err(recovery_residual_error(
            parent,
            &[name],
            "the temporary entry changed before commit",
        ))
    }
}

fn cleanup_temp_from_open_handle(
    parent: &PinnedManagedParent,
    name: &OsString,
    file: CapabilityFile,
) -> Result<(), GuardIntegrationError> {
    let open_identity = file
        .metadata()
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| ManagedFileIdentity::from_metadata(&metadata));
    drop(file);
    if parent
        .dir()
        .symlink_metadata(name)
        .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    let Some(open_identity) = open_identity else {
        return Err(recovery_residual_error(
            parent,
            &[name],
            "the temporary file handle could not be identified before cleanup",
        ));
    };
    let quarantine = unused_sibling_name(parent, "cleanup")?;
    rename_entry_no_replace(parent, name, &quarantine).map_err(|error| {
        recovery_residual_error(
            parent,
            &[name, &quarantine],
            &format!("the temporary entry could not be isolated for cleanup: {error}"),
        )
    })?;
    let moved_identity = parent
        .dir()
        .symlink_metadata(&quarantine)
        .ok()
        .filter(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        .map(|metadata| ManagedFileIdentity::from_metadata(&metadata));
    if moved_identity == Some(open_identity) {
        parent.dir().remove_file(&quarantine).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to remove isolated temporary managed file {}: {error}",
                parent.absolute_entry_path(&quarantine).display()
            ))
        })
    } else {
        if parent
            .dir()
            .symlink_metadata(name)
            .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
        {
            rename_entry_no_replace(parent, &quarantine, name).map_err(|error| {
                recovery_residual_error(
                    parent,
                    &[name, &quarantine],
                    &format!("the changed temporary entry could not be restored: {error}"),
                )
            })?;
        }
        Err(recovery_residual_error(
            parent,
            &[name, &quarantine],
            "the temporary entry changed before cleanup",
        ))
    }
}

#[cfg(unix)]
fn stable_file_metadata(before: &CapabilityMetadata, after: &CapabilityMetadata) -> bool {
    use cap_std::fs::{MetadataExt as UnixMetadataExt, PermissionsExt};

    ManagedFileIdentity::from_metadata(before) == ManagedFileIdentity::from_metadata(after)
        && before.len() == after.len()
        && before.permissions().mode() == after.permissions().mode()
        && UnixMetadataExt::uid(before) == UnixMetadataExt::uid(after)
        && UnixMetadataExt::gid(before) == UnixMetadataExt::gid(after)
}

#[cfg(not(unix))]
fn stable_file_metadata(before: &CapabilityMetadata, after: &CapabilityMetadata) -> bool {
    ManagedFileIdentity::from_metadata(before) == ManagedFileIdentity::from_metadata(after)
        && before.len() == after.len()
        && before.permissions().readonly() == after.permissions().readonly()
}

#[cfg(unix)]
#[derive(Debug, Clone)]
struct TempPermissionPlan {
    final_mode: u32,
    owner_group: Option<(u32, u32)>,
    extended_attributes: Option<Vec<(OsString, Vec<u8>)>>,
}

#[cfg(unix)]
fn prepare_temp_permissions(
    file: &CapabilityFile,
    snapshot: &ManagedTargetSnapshot,
    executable: bool,
    user_only: bool,
) -> io::Result<TempPermissionPlan> {
    use cap_std::fs::PermissionsExt;

    let mut permissions = file.metadata()?.permissions();
    let mut final_mode = match snapshot {
        ManagedTargetSnapshot::Missing => permissions.mode(),
        ManagedTargetSnapshot::RegularFile(existing) => existing.mode,
    };
    if executable {
        final_mode |= 0o755;
    }
    if user_only {
        final_mode = (final_mode & !0o777) | 0o600;
    }
    let (owner_group, extended_attributes) = match snapshot {
        ManagedTargetSnapshot::Missing => (None, None),
        ManagedTargetSnapshot::RegularFile(existing) => (
            Some((existing.uid, existing.gid)),
            Some(existing.extended_attributes.clone()),
        ),
    };
    if let Some(attributes) = &extended_attributes {
        reject_privileged_content_metadata(attributes)?;
    }
    permissions.set_mode(0o600);
    file.set_permissions(permissions)?;
    Ok(TempPermissionPlan {
        final_mode,
        owner_group,
        extended_attributes,
    })
}

#[cfg(unix)]
fn apply_final_temp_permissions(
    file: &CapabilityFile,
    plan: &TempPermissionPlan,
) -> io::Result<()> {
    use cap_std::fs::{MetadataExt as UnixMetadataExt, PermissionsExt};
    use rustix::fs::{fchown, Gid, Uid};

    if let Some((uid, gid)) = plan.owner_group {
        let metadata = file.metadata()?;
        let owner = (UnixMetadataExt::uid(&metadata) != uid).then(|| Uid::from_raw(uid));
        let group = (UnixMetadataExt::gid(&metadata) != gid).then(|| Gid::from_raw(gid));
        if owner.is_some() || group.is_some() {
            fchown(file, owner, group)?;
        }
    }

    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(plan.final_mode);
    file.set_permissions(permissions)?;
    if let Some(extended_attributes) = &plan.extended_attributes {
        apply_extended_attributes(file, extended_attributes)?;
    }
    Ok(())
}

#[cfg(unix)]
fn staged_metadata_matches_plan(
    snapshot: &ManagedTargetSnapshot,
    plan: &TempPermissionPlan,
) -> bool {
    let ManagedTargetSnapshot::RegularFile(file) = snapshot else {
        return false;
    };
    file.mode == plan.final_mode
        && plan
            .owner_group
            .is_none_or(|(uid, gid)| file.uid == uid && file.gid == gid)
        && plan
            .extended_attributes
            .as_ref()
            .is_none_or(|attributes| &file.extended_attributes == attributes)
}

#[cfg(unix)]
fn read_extended_attributes(file: &CapabilityFile) -> io::Result<Vec<(OsString, Vec<u8>)>> {
    use rustix::fs::{fgetxattr, flistxattr};
    use std::os::unix::ffi::OsStrExt;

    let mut names = Vec::new();
    let required = flistxattr(file, &mut names)?;
    if required == 0 {
        return Ok(Vec::new());
    }
    names.resize(required, 0);
    let length = flistxattr(file, &mut names)?;
    names.truncate(length);

    let mut attributes = Vec::new();
    for name in names
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let name = std::ffi::OsStr::from_bytes(name);
        let mut value = Vec::new();
        let required = fgetxattr(file, name, &mut value)?;
        value.resize(required, 0);
        let length = fgetxattr(file, name, &mut value)?;
        value.truncate(length);
        attributes.push((name.to_os_string(), value));
    }
    attributes.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(attributes)
}

#[cfg(unix)]
fn reject_privileged_content_metadata(attributes: &[(OsString, Vec<u8>)]) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    const CONTENT_BOUND_ATTRIBUTES: &[&[u8]] =
        &[b"security.capability", b"security.ima", b"security.evm"];
    if let Some((name, _)) = attributes
        .iter()
        .find(|(name, _)| CONTENT_BOUND_ATTRIBUTES.contains(&name.as_os_str().as_bytes()))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "managed replacement refuses content-bound extended attribute {}",
                name.to_string_lossy()
            ),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn apply_extended_attributes(
    file: &CapabilityFile,
    desired: &[(OsString, Vec<u8>)],
) -> io::Result<()> {
    use rustix::fs::{fremovexattr, fsetxattr, XattrFlags};

    let current = read_extended_attributes(file)?;
    for (name, _) in &current {
        if !desired.iter().any(|(desired_name, _)| desired_name == name) {
            fremovexattr(file, name)?;
        }
    }
    for (name, value) in desired {
        if current
            .iter()
            .find(|(current_name, _)| current_name == name)
            .is_none_or(|(_, current_value)| current_value != value)
        {
            fsetxattr(file, name, value, XattrFlags::empty())?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
#[derive(Debug, Clone, Copy)]
struct TempPermissionPlan {
    final_readonly: Option<bool>,
}

#[cfg(not(unix))]
fn prepare_temp_permissions(
    file: &CapabilityFile,
    snapshot: &ManagedTargetSnapshot,
    _executable: bool,
    _user_only: bool,
) -> io::Result<TempPermissionPlan> {
    let _ = file.metadata()?;
    Ok(TempPermissionPlan {
        final_readonly: match snapshot {
            ManagedTargetSnapshot::Missing => None,
            ManagedTargetSnapshot::RegularFile(existing) => Some(existing.readonly),
        },
    })
}

#[cfg(not(unix))]
fn apply_final_temp_permissions(
    file: &CapabilityFile,
    plan: &TempPermissionPlan,
) -> io::Result<()> {
    if let Some(readonly) = plan.final_readonly {
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(readonly);
        file.set_permissions(permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn staged_metadata_matches_plan(
    snapshot: &ManagedTargetSnapshot,
    plan: &TempPermissionPlan,
) -> bool {
    let ManagedTargetSnapshot::RegularFile(file) = snapshot else {
        return false;
    };
    plan.final_readonly
        .is_none_or(|readonly| file.readonly == readonly)
}

fn random_file_token() -> Result<String, GuardIntegrationError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to obtain randomness for a managed temporary file: {error}"
        ))
    })?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn atomic_create_if_missing<F>(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    staged: &ManagedTargetSnapshot,
    hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    let staged = RecoveryEntrySnapshot::from(staged);
    if let Err(error) = rename_entry_no_replace(parent, temp_name, &parent.target_name) {
        cleanup_owned_entry(parent, temp_name, &staged)?;
        return if error.kind() == io::ErrorKind::AlreadyExists {
            Err(stale_managed_file_error(&parent.target_path))
        } else {
            Err(GuardIntegrationError::runtime(format!(
                "failed to create managed file atomically at {}: {error}",
                parent.target_path.display()
            )))
        };
    }

    let commit_hook_failed =
        run_write_hook(hook, ManagedWritePhase::CommitApplied, &parent.target_path).is_err();
    let installed = parent.read_recovery_target_snapshot()?;
    if installed == staged && !commit_hook_failed {
        parent.sync_directory();
        return Ok(());
    }

    run_write_hook(
        hook,
        ManagedWritePhase::RollbackInspecting,
        &parent.target_path,
    )?;
    ensure_recovery_pair_unchanged(
        parent,
        temp_name,
        &installed,
        &RecoveryEntrySnapshot::Missing,
    )?;
    run_write_hook(hook, ManagedWritePhase::RollbackReady, &parent.target_path)?;
    ensure_recovery_pair_unchanged(
        parent,
        temp_name,
        &installed,
        &RecoveryEntrySnapshot::Missing,
    )?;
    if let Err(error) = rename_entry_no_replace(parent, &parent.target_name, temp_name) {
        return Err(recovery_residual_error(
            parent,
            &[temp_name],
            &format!("rejected create could not be rolled back: {error}"),
        ));
    }

    let target = parent.read_recovery_target_snapshot()?;
    let rejected =
        parent.read_recovery_entry_snapshot(temp_name, &parent.absolute_entry_path(temp_name))?;
    if target == RecoveryEntrySnapshot::Missing && rejected == installed {
        if installed == staged {
            remove_exact_entry(parent, temp_name, &rejected, "rejected create")?;
        } else {
            rename_entry_no_replace(parent, temp_name, &parent.target_name).map_err(|error| {
                recovery_residual_error(
                    parent,
                    &[temp_name],
                    &format!(
                        "rejected create changed before commit and restoring it failed: {error}"
                    ),
                )
            })?;
            if parent.read_recovery_target_snapshot()? != rejected {
                return Err(recovery_residual_error(
                    parent,
                    &[temp_name],
                    "the rejected create changed while it was restored",
                ));
            }
        }
        parent.sync_directory();
        Err(stale_managed_file_error(&parent.target_path))
    } else {
        if target == RecoveryEntrySnapshot::Missing && rejected != RecoveryEntrySnapshot::Missing {
            rename_entry_no_replace(parent, temp_name, &parent.target_name).map_err(|error| {
                recovery_residual_error(
                    parent,
                    &[temp_name],
                    &format!(
                        "create rollback observed a changed entry and restoring it failed: {error}"
                    ),
                )
            })?;
            if parent.read_recovery_target_snapshot()? != rejected {
                return Err(recovery_residual_error(
                    parent,
                    &[temp_name],
                    "the changed create entry could not be verified after restoration",
                ));
            }
        }
        parent.sync_directory();
        Err(recovery_residual_error(
            parent,
            &[temp_name],
            "a second writer changed the destination during create rollback",
        ))
    }
}

fn ensure_recovery_pair_unchanged(
    parent: &PinnedManagedParent,
    secondary_name: &OsString,
    target: &RecoveryEntrySnapshot,
    secondary: &RecoveryEntrySnapshot,
) -> Result<(), GuardIntegrationError> {
    let current_target = parent.read_recovery_target_snapshot()?;
    let current_secondary = parent.read_recovery_entry_snapshot(
        secondary_name,
        &parent.absolute_entry_path(secondary_name),
    )?;
    if &current_target == target && &current_secondary == secondary {
        Ok(())
    } else {
        Err(recovery_residual_error(
            parent,
            &[secondary_name],
            "the destination or recovery entry changed before rollback",
        ))
    }
}

fn remove_exact_entry(
    parent: &PinnedManagedParent,
    name: &OsString,
    expected: &RecoveryEntrySnapshot,
    role: &str,
) -> Result<(), GuardIntegrationError> {
    let path = parent.absolute_entry_path(name);
    let quarantine = unused_sibling_name(parent, "cleanup")?;
    rename_entry_no_replace(parent, name, &quarantine).map_err(|error| {
        recovery_residual_error(
            parent,
            &[name, &quarantine],
            &format!("the {role} could not be isolated for cleanup: {error}"),
        )
    })?;
    let quarantine_path = parent.absolute_entry_path(&quarantine);
    let isolated = parent.read_recovery_entry_snapshot(&quarantine, &quarantine_path)?;
    if &isolated != expected {
        if parent.read_recovery_entry_snapshot(name, &path)? == RecoveryEntrySnapshot::Missing {
            rename_entry_no_replace(parent, &quarantine, name).map_err(|error| {
                recovery_residual_error(
                    parent,
                    &[name, &quarantine],
                    &format!("the changed {role} could not be restored after inspection: {error}"),
                )
            })?;
        }
        return Err(recovery_residual_error(
            parent,
            &[name, &quarantine],
            &format!("the {role} changed before cleanup"),
        ));
    }
    parent.dir().remove_file(&quarantine).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "managed file operation completed but the {role} could not be removed at {}: {error}",
            quarantine_path.display()
        ))
    })
}

fn cleanup_owned_entry(
    parent: &PinnedManagedParent,
    name: &OsString,
    owner: &RecoveryEntrySnapshot,
) -> Result<(), GuardIntegrationError> {
    let path = parent.absolute_entry_path(name);
    let current = parent.read_recovery_entry_snapshot(name, &path)?;
    if current == RecoveryEntrySnapshot::Missing {
        return Ok(());
    }
    if &current != owner {
        return Err(recovery_residual_error(
            parent,
            &[name],
            "the temporary entry changed before cleanup",
        ));
    }
    remove_exact_entry(parent, name, owner, "temporary entry")
}

fn cleanup_uncommitted_temp(
    parent: &PinnedManagedParent,
    name: &OsString,
    owner: &ManagedTargetSnapshot,
) -> Result<(), GuardIntegrationError> {
    let owner = RecoveryEntrySnapshot::from(owner);
    cleanup_owned_entry(parent, name, &owner)
}

fn recovery_residual_error(
    parent: &PinnedManagedParent,
    candidate_names: &[&OsString],
    detail: &str,
) -> GuardIntegrationError {
    let mut residuals = candidate_names
        .iter()
        .filter_map(|name| {
            let path = parent.absolute_entry_path(name);
            parent.dir().symlink_metadata(name).ok().map(|_| path)
        })
        .collect::<Vec<_>>();
    residuals.sort();
    residuals.dedup();
    let suffix = if residuals.is_empty() {
        "no recovery entry remained when the directory was inspected".to_owned()
    } else {
        format!(
            "automatic recovery stopped to avoid overwriting a concurrent file; recovery entries present at inspection: {}",
            residuals
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    GuardIntegrationError::runtime(format!(
        "managed file changed during conditional replacement at {}; {detail}; {suffix}",
        parent.target_path.display()
    ))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn atomic_commit_if_fresh<F>(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    expected: &ManagedTargetSnapshot,
    staged: &ManagedTargetSnapshot,
    hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    if matches!(expected, ManagedTargetSnapshot::Missing) {
        return atomic_create_if_missing(parent, temp_name, staged, hook);
    }

    if let Err(error) = exchange_entries(parent, temp_name, &parent.target_name) {
        cleanup_uncommitted_temp(parent, temp_name, staged)?;
        return Err(GuardIntegrationError::runtime(format!(
            "failed to exchange managed file atomically at {}: {error}",
            parent.target_path.display()
        )));
    }
    let commit_hook_failed =
        run_write_hook(hook, ManagedWritePhase::CommitApplied, &parent.target_path).is_err();
    let installed = parent.read_recovery_target_snapshot()?;
    let displaced =
        parent.read_recovery_entry_snapshot(temp_name, &parent.absolute_entry_path(temp_name))?;
    let staged = RecoveryEntrySnapshot::from(staged);
    let expected = RecoveryEntrySnapshot::from(expected);
    if installed == staged && displaced == expected && !commit_hook_failed {
        remove_exact_entry(parent, temp_name, &displaced, "displaced predecessor")?;
        parent.sync_directory();
        return Ok(());
    }
    rollback_exchange_after_mismatch(parent, temp_name, &installed, &displaced, &staged, hook)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rollback_exchange_after_mismatch<F>(
    parent: &PinnedManagedParent,
    displaced_name: &OsString,
    installed: &RecoveryEntrySnapshot,
    displaced: &RecoveryEntrySnapshot,
    owned_staged: &RecoveryEntrySnapshot,
    hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    run_write_hook(
        hook,
        ManagedWritePhase::RollbackInspecting,
        &parent.target_path,
    )?;
    ensure_recovery_pair_unchanged(parent, displaced_name, installed, displaced)?;
    run_write_hook(hook, ManagedWritePhase::RollbackReady, &parent.target_path)?;
    ensure_recovery_pair_unchanged(parent, displaced_name, installed, displaced)?;
    if let Err(error) = exchange_entries(parent, displaced_name, &parent.target_name) {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name],
            &format!("rollback exchange failed: {error}"),
        ));
    }
    let rollback_displaced = parent.read_recovery_entry_snapshot(
        displaced_name,
        &parent.absolute_entry_path(displaced_name),
    )?;
    let restored = parent.read_recovery_target_snapshot()?;
    if &restored == displaced && &rollback_displaced == installed {
        if &rollback_displaced == owned_staged {
            remove_exact_entry(
                parent,
                displaced_name,
                &rollback_displaced,
                "rejected replacement",
            )?;
            parent.sync_directory();
            Err(stale_managed_file_error(&parent.target_path))
        } else {
            parent.sync_directory();
            Err(recovery_residual_error(
                parent,
                &[displaced_name],
                "the installed entry was not the Volicord-staged replacement",
            ))
        }
    } else {
        parent.sync_directory();
        Err(recovery_residual_error(
            parent,
            &[displaced_name],
            "a second writer changed the destination during rollback",
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn exchange_entries(
    parent: &PinnedManagedParent,
    first: &OsString,
    second: &OsString,
) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags};

    let dir = parent.dir().try_clone()?.into_std_file();
    renameat_with(&dir, first, &dir, second, RenameFlags::EXCHANGE).map_err(io::Error::from)
}

#[cfg(windows)]
fn atomic_commit_if_fresh<F>(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    expected: &ManagedTargetSnapshot,
    staged: &ManagedTargetSnapshot,
    hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    if matches!(expected, ManagedTargetSnapshot::Missing) {
        return atomic_create_if_missing(parent, temp_name, staged, hook);
    }

    let expected_recovery = RecoveryEntrySnapshot::from(expected);
    let staged_recovery = RecoveryEntrySnapshot::from(staged);
    let (backup_name, backup_file) = match parent.create_private_sibling_file("backup") {
        Ok(reservation) => reservation,
        Err(error) => {
            cleanup_uncommitted_temp(parent, temp_name, staged)?;
            return Err(error);
        }
    };
    let backup_sentinel = match parent
        .read_recovery_entry_snapshot(&backup_name, &parent.absolute_entry_path(&backup_name))
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            cleanup_windows_backup_and_temp(
                parent,
                temp_name,
                &staged_recovery,
                &backup_name,
                backup_file,
            )?;
            return Err(error);
        }
    };
    let predecessor_name = match reserve_windows_predecessor(parent, expected) {
        Ok(name) => name,
        Err(error) => {
            if let Err(cleanup) = cleanup_windows_backup_and_temp(
                parent,
                temp_name,
                &staged_recovery,
                &backup_name,
                backup_file,
            ) {
                return Err(GuardIntegrationError::runtime(format!(
                    "Windows predecessor reservation failed ({error}); owned-entry cleanup also failed: {cleanup}"
                )));
            }
            return Err(error);
        }
    };
    let target_guard = match volicord_platform_fs::open_file_for_replace(&parent.target_path) {
        Ok(file) => file,
        Err(error) => {
            let inspection = (|| {
                Ok::<_, GuardIntegrationError>((
                    parent.read_recovery_target_snapshot()?,
                    parent.read_recovery_entry_snapshot(
                        &backup_name,
                        &parent.absolute_entry_path(&backup_name),
                    )?,
                    parent.read_recovery_entry_snapshot(
                        &predecessor_name,
                        &parent.absolute_entry_path(&predecessor_name),
                    )?,
                ))
            })();
            let Ok((target, backup, predecessor)) = inspection else {
                drop(backup_file);
                return Err(recovery_residual_error(
                    parent,
                    &[temp_name, &backup_name, &predecessor_name],
                    &format!(
                        "the Windows recovery entries could not be inspected after its write guard failed: {error}"
                    ),
                ));
            };
            if target == expected_recovery
                && backup == backup_sentinel
                && predecessor == expected_recovery
            {
                cleanup_windows_uncommitted_reservations(
                    parent,
                    temp_name,
                    &staged_recovery,
                    &backup_name,
                    backup_file,
                    &predecessor_name,
                    &expected_recovery,
                )?;
                return Err(GuardIntegrationError::runtime(format!(
                    "failed to pin managed file for conditional replacement at {}: {error}",
                    parent.target_path.display()
                )));
            }
            drop(backup_file);
            return Err(recovery_residual_error(
                parent,
                &[temp_name, &backup_name, &predecessor_name],
                &format!("the Windows target changed while its write guard was opened: {error}"),
            ));
        }
    };
    let initial_pin_matches = match windows_pinned_file_matches_expected(&target_guard, expected) {
        Ok(matches) => matches,
        Err(error) => {
            drop(backup_file);
            return Err(recovery_residual_error(
                parent,
                &[temp_name, &backup_name, &predecessor_name],
                &format!("the pinned Windows predecessor could not be inspected: {error}"),
            ));
        }
    };
    if !initial_pin_matches {
        drop(backup_file);
        return Err(recovery_residual_error(
            parent,
            &[temp_name, &backup_name, &predecessor_name],
            "the pinned Windows file did not match the planned predecessor",
        ));
    }
    let hook_result = run_write_hook(hook, ManagedWritePhase::CommitReserved, &parent.target_path);
    let inspection = (|| {
        Ok::<_, GuardIntegrationError>((
            parent.read_recovery_target_snapshot()?,
            parent
                .read_recovery_entry_snapshot(temp_name, &parent.absolute_entry_path(temp_name))?,
            parent.read_recovery_entry_snapshot(
                &backup_name,
                &parent.absolute_entry_path(&backup_name),
            )?,
            parent.read_recovery_entry_snapshot(
                &predecessor_name,
                &parent.absolute_entry_path(&predecessor_name),
            )?,
        ))
    })();
    let Ok((target, replacement, backup, predecessor)) = inspection else {
        drop(backup_file);
        return Err(recovery_residual_error(
            parent,
            &[temp_name, &backup_name, &predecessor_name],
            "the reserved Windows replacement inputs could not be inspected",
        ));
    };
    let pin_matches = match windows_pinned_file_matches_expected(&target_guard, expected) {
        Ok(matches) => matches,
        Err(error) => {
            drop(backup_file);
            return Err(recovery_residual_error(
                parent,
                &[temp_name, &backup_name, &predecessor_name],
                &format!("the pinned Windows predecessor could not be reinspected: {error}"),
            ));
        }
    };
    let parent_attached = parent.validate_attached().is_ok();
    if hook_result.is_err()
        || !pin_matches
        || !parent_attached
        || target != expected_recovery
        || replacement != staged_recovery
        || backup != backup_sentinel
        || predecessor != expected_recovery
    {
        drop(backup_file);
        return Err(recovery_residual_error(
            parent,
            &[temp_name, &backup_name, &predecessor_name],
            "the Windows replacement inputs changed after their recovery names were reserved",
        ));
    }
    drop(backup_file);
    if run_write_hook(
        hook,
        ManagedWritePhase::NativeCommitReady,
        &parent.target_path,
    )
    .is_err()
    {
        return Err(recovery_residual_error(
            parent,
            &[temp_name, &backup_name, &predecessor_name],
            "the Windows native replacement hook failed after recovery names were reserved",
        ));
    }

    if let Err(error) = volicord_platform_fs::replace_file_with_backup(
        &parent.target_path,
        &parent.absolute_entry_path(temp_name),
        &parent.absolute_entry_path(&backup_name),
    ) {
        return recover_failed_windows_commit(
            parent,
            temp_name,
            &backup_name,
            &predecessor_name,
            &backup_sentinel,
            &expected_recovery,
            &staged_recovery,
            error,
        );
    }
    let commit_hook_failed =
        run_write_hook(hook, ManagedWritePhase::CommitApplied, &parent.target_path).is_err();
    let inspection = (|| {
        Ok::<_, GuardIntegrationError>((
            parent.read_recovery_target_snapshot()?,
            parent
                .read_recovery_entry_snapshot(temp_name, &parent.absolute_entry_path(temp_name))?,
            parent.read_recovery_entry_snapshot(
                &backup_name,
                &parent.absolute_entry_path(&backup_name),
            )?,
            parent.read_recovery_entry_snapshot(
                &predecessor_name,
                &parent.absolute_entry_path(&predecessor_name),
            )?,
        ))
    })();
    let Ok((installed, replacement, displaced, predecessor)) = inspection else {
        return Err(recovery_residual_error(
            parent,
            &[temp_name, &backup_name, &predecessor_name],
            "the Windows replacement result could not be inspected",
        ));
    };
    if installed == staged_recovery
        && replacement == RecoveryEntrySnapshot::Missing
        && displaced == expected_recovery
        && predecessor == expected_recovery
        && !commit_hook_failed
    {
        remove_exact_entry(parent, &backup_name, &displaced, "displaced predecessor")?;
        remove_exact_entry(
            parent,
            &predecessor_name,
            &predecessor,
            "preserved predecessor",
        )?;
        parent.sync_directory();
        return Ok(());
    }
    if replacement != RecoveryEntrySnapshot::Missing {
        return Err(recovery_residual_error(
            parent,
            &[temp_name, &backup_name, &predecessor_name],
            "the Windows replacement name was recreated before commit verification",
        ));
    }
    rollback_windows_after_mismatch(
        parent,
        &backup_name,
        &predecessor_name,
        &installed,
        &displaced,
        &predecessor,
        &expected_recovery,
        &staged_recovery,
        hook,
    )
}

#[cfg(windows)]
fn recover_failed_windows_commit(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    backup_name: &OsString,
    predecessor_name: &OsString,
    backup_sentinel: &RecoveryEntrySnapshot,
    expected: &RecoveryEntrySnapshot,
    staged: &RecoveryEntrySnapshot,
    error: volicord_platform_fs::ReplaceFileError,
) -> Result<(), GuardIntegrationError> {
    let inspection = (|| {
        Ok::<_, GuardIntegrationError>((
            parent.read_recovery_target_snapshot()?,
            parent
                .read_recovery_entry_snapshot(temp_name, &parent.absolute_entry_path(temp_name))?,
            parent.read_recovery_entry_snapshot(
                backup_name,
                &parent.absolute_entry_path(backup_name),
            )?,
            parent.read_recovery_entry_snapshot(
                predecessor_name,
                &parent.absolute_entry_path(predecessor_name),
            )?,
        ))
    })();
    let Ok((target, replacement, backup, predecessor)) = inspection else {
        return Err(recovery_residual_error(
            parent,
            &[temp_name, backup_name, predecessor_name],
            &format!("the Windows replacement failure state could not be inspected: {error}"),
        ));
    };

    if target == *expected
        && replacement == *staged
        && predecessor == *expected
        && (backup == *backup_sentinel || backup == RecoveryEntrySnapshot::Missing)
    {
        cleanup_owned_entry(parent, temp_name, staged)?;
        if backup == *backup_sentinel {
            cleanup_owned_entry(parent, backup_name, backup_sentinel)?;
        }
        cleanup_owned_entry(parent, predecessor_name, expected)?;
        return Err(GuardIntegrationError::runtime(format!(
            "failed to replace managed file atomically at {}: {error}",
            parent.target_path.display()
        )));
    }

    if predecessor != *expected {
        return Err(recovery_residual_error(
            parent,
            &[temp_name, backup_name, predecessor_name],
            &format!("the preserved Windows predecessor changed after replacement failed: {error}"),
        ));
    }

    if target == RecoveryEntrySnapshot::Missing {
        rename_entry_no_replace(parent, predecessor_name, &parent.target_name).map_err(
            |recovery| {
                recovery_residual_error(
                    parent,
                    &[temp_name, backup_name, predecessor_name],
                    &format!(
                        "partial Windows replacement failed ({error}) and restoring its preserved predecessor failed: {recovery}"
                    ),
                )
            },
        )?;
        let restored = parent
            .read_recovery_target_snapshot()
            .map_err(|inspection| {
                recovery_residual_error(
                    parent,
                    &[temp_name, backup_name, predecessor_name],
                    &format!(
                        "the restored Windows predecessor could not be inspected: {inspection}"
                    ),
                )
            })?;
        if restored != *expected {
            return Err(recovery_residual_error(
                parent,
                &[temp_name, backup_name, predecessor_name],
                "the preserved Windows predecessor changed while it was restored",
            ));
        }
        if replacement == *staged {
            cleanup_owned_entry(parent, temp_name, staged)?;
        }
        if backup == *expected {
            cleanup_owned_entry(parent, backup_name, expected)?;
            parent.sync_directory();
            return Err(stale_managed_file_error(&parent.target_path));
        }
        parent.sync_directory();
        return Err(recovery_residual_error(
            parent,
            &[temp_name, backup_name, predecessor_name],
            &format!(
                "partial Windows replacement preserved an unexpected displaced entry: {error}"
            ),
        ));
    }

    if replacement == RecoveryEntrySnapshot::Missing {
        let mut no_hook = |_| Ok(());
        return rollback_windows_after_mismatch(
            parent,
            backup_name,
            predecessor_name,
            &target,
            &backup,
            &predecessor,
            expected,
            staged,
            &mut no_hook,
        );
    }

    Err(recovery_residual_error(
        parent,
        &[temp_name, backup_name, predecessor_name],
        &format!("Windows replacement failed with changed participating entries: {error}"),
    ))
}

#[cfg(windows)]
fn rollback_windows_after_mismatch<F>(
    parent: &PinnedManagedParent,
    displaced_name: &OsString,
    predecessor_name: &OsString,
    installed: &RecoveryEntrySnapshot,
    displaced: &RecoveryEntrySnapshot,
    predecessor: &RecoveryEntrySnapshot,
    expected: &RecoveryEntrySnapshot,
    owned_staged: &RecoveryEntrySnapshot,
    hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    run_write_hook(
        hook,
        ManagedWritePhase::RollbackInspecting,
        &parent.target_path,
    )?;
    ensure_windows_recovery_state(
        parent,
        displaced_name,
        predecessor_name,
        installed,
        displaced,
        predecessor,
    )?;
    run_write_hook(hook, ManagedWritePhase::RollbackReady, &parent.target_path)?;
    ensure_windows_recovery_state(
        parent,
        displaced_name,
        predecessor_name,
        installed,
        displaced,
        predecessor,
    )?;
    if predecessor != expected {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name],
            "the preserved Windows predecessor no longer matches the planned file",
        ));
    }

    if installed == &RecoveryEntrySnapshot::Missing {
        rename_entry_no_replace(parent, predecessor_name, &parent.target_name).map_err(
            |error| {
                recovery_residual_error(
                    parent,
                    &[displaced_name, predecessor_name],
                    &format!("the missing Windows destination could not be restored: {error}"),
                )
            },
        )?;
        let restored = parent
            .read_recovery_target_snapshot()
            .map_err(|inspection| {
                recovery_residual_error(
                    parent,
                    &[displaced_name, predecessor_name],
                    &format!(
                        "the restored Windows destination could not be inspected: {inspection}"
                    ),
                )
            })?;
        if restored != *expected {
            return Err(recovery_residual_error(
                parent,
                &[displaced_name, predecessor_name],
                "the Windows predecessor changed while the missing destination was restored",
            ));
        }
        if displaced == expected {
            cleanup_owned_entry(parent, displaced_name, expected)?;
        }
        parent.sync_directory();
        return Err(if displaced == expected {
            stale_managed_file_error(&parent.target_path)
        } else {
            recovery_residual_error(
                parent,
                &[displaced_name, predecessor_name],
                "a concurrent displaced entry remains after the missing destination was restored",
            )
        });
    }

    let rollback_name = unused_sibling_name(parent, "rollback")?;
    rename_entry_no_replace(parent, &parent.target_name, &rollback_name).map_err(|error| {
        recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            &format!("the installed Windows entry could not be isolated for rollback: {error}"),
        )
    })?;
    let inspection = (|| {
        Ok::<_, GuardIntegrationError>((
            parent.read_recovery_target_snapshot()?,
            parent.read_recovery_entry_snapshot(
                &rollback_name,
                &parent.absolute_entry_path(&rollback_name),
            )?,
            parent.read_recovery_entry_snapshot(
                displaced_name,
                &parent.absolute_entry_path(displaced_name),
            )?,
            parent.read_recovery_entry_snapshot(
                predecessor_name,
                &parent.absolute_entry_path(predecessor_name),
            )?,
        ))
    })();
    let Ok((target_after_isolation, rejected, current_displaced, current_predecessor)) = inspection
    else {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            "the isolated Windows rollback entries could not be inspected",
        ));
    };
    if target_after_isolation != RecoveryEntrySnapshot::Missing || current_predecessor != *expected
    {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            "the Windows rollback entries changed before predecessor restoration",
        ));
    }
    rename_entry_no_replace(parent, predecessor_name, &parent.target_name).map_err(|error| {
        recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            &format!("the preserved Windows predecessor could not be restored: {error}"),
        )
    })?;
    let inspection = (|| {
        Ok::<_, GuardIntegrationError>((
            parent.read_recovery_target_snapshot()?,
            parent.read_recovery_entry_snapshot(
                &rollback_name,
                &parent.absolute_entry_path(&rollback_name),
            )?,
            parent.read_recovery_entry_snapshot(
                displaced_name,
                &parent.absolute_entry_path(displaced_name),
            )?,
            parent.read_recovery_entry_snapshot(
                predecessor_name,
                &parent.absolute_entry_path(predecessor_name),
            )?,
        ))
    })();
    let Ok((restored, rejected_after, displaced_after, predecessor_after)) = inspection else {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            "the restored Windows rollback entries could not be inspected",
        ));
    };
    if restored != *expected
        || rejected_after != rejected
        || displaced_after != current_displaced
        || predecessor_after != RecoveryEntrySnapshot::Missing
    {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            "a second writer changed the Windows entries during rollback",
        ));
    }

    let clean_rollback = rejected == *installed && rejected == *owned_staged;
    if clean_rollback {
        cleanup_owned_entry(parent, &rollback_name, owned_staged)?;
    }
    let clean_displaced = current_displaced == *displaced && current_displaced == *expected;
    if clean_displaced {
        cleanup_owned_entry(parent, displaced_name, expected)?;
    }
    parent.sync_directory();
    if clean_rollback && clean_displaced {
        Err(stale_managed_file_error(&parent.target_path))
    } else {
        Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name, &rollback_name],
            "concurrent Windows bytes were preserved after predecessor restoration",
        ))
    }
}

#[cfg(windows)]
fn cleanup_windows_backup_and_temp(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    staged: &RecoveryEntrySnapshot,
    backup_name: &OsString,
    backup_file: CapabilityFile,
) -> Result<(), GuardIntegrationError> {
    cleanup_temp_from_open_handle(parent, backup_name, backup_file)?;
    cleanup_owned_entry(parent, temp_name, staged)
}

#[cfg(windows)]
fn cleanup_windows_uncommitted_reservations(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    staged: &RecoveryEntrySnapshot,
    backup_name: &OsString,
    backup_file: CapabilityFile,
    predecessor_name: &OsString,
    expected: &RecoveryEntrySnapshot,
) -> Result<(), GuardIntegrationError> {
    cleanup_temp_from_open_handle(parent, backup_name, backup_file)?;
    cleanup_owned_entry(parent, temp_name, staged)?;
    cleanup_owned_entry(parent, predecessor_name, expected)
}

#[cfg(windows)]
fn ensure_windows_recovery_state(
    parent: &PinnedManagedParent,
    displaced_name: &OsString,
    predecessor_name: &OsString,
    target: &RecoveryEntrySnapshot,
    displaced: &RecoveryEntrySnapshot,
    predecessor: &RecoveryEntrySnapshot,
) -> Result<(), GuardIntegrationError> {
    let inspection = (|| {
        Ok::<_, GuardIntegrationError>((
            parent.read_recovery_target_snapshot()?,
            parent.read_recovery_entry_snapshot(
                displaced_name,
                &parent.absolute_entry_path(displaced_name),
            )?,
            parent.read_recovery_entry_snapshot(
                predecessor_name,
                &parent.absolute_entry_path(predecessor_name),
            )?,
        ))
    })();
    let Ok((current_target, current_displaced, current_predecessor)) = inspection else {
        return Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name],
            "the Windows destination or recovery entries could not be inspected before rollback",
        ));
    };
    if &current_target == target
        && &current_displaced == displaced
        && &current_predecessor == predecessor
    {
        Ok(())
    } else {
        Err(recovery_residual_error(
            parent,
            &[displaced_name, predecessor_name],
            "the Windows destination or recovery entries changed before rollback",
        ))
    }
}

#[cfg(windows)]
fn reserve_windows_predecessor(
    parent: &PinnedManagedParent,
    expected: &ManagedTargetSnapshot,
) -> Result<OsString, GuardIntegrationError> {
    let target_name = parent.target_name.to_string_lossy();
    let expected = RecoveryEntrySnapshot::from(expected);
    for _ in 0..64 {
        let token = random_file_token()?;
        let candidate = OsString::from(format!(".{target_name}.volicord-predecessor-{token}"));
        match parent
            .dir()
            .hard_link(&parent.target_name, parent.dir(), &candidate)
        {
            Ok(()) => {
                let inspection = (|| {
                    Ok::<_, GuardIntegrationError>((
                        parent.read_recovery_entry_snapshot(
                            &candidate,
                            &parent.absolute_entry_path(&candidate),
                        )?,
                        parent.read_recovery_target_snapshot()?,
                    ))
                })();
                let Ok((predecessor, target)) = inspection else {
                    return Err(recovery_residual_error(
                        parent,
                        &[&candidate],
                        "the reserved Windows predecessor could not be inspected",
                    ));
                };
                if predecessor == expected && target == expected {
                    return Ok(candidate);
                }
                return Err(recovery_residual_error(
                    parent,
                    &[&candidate],
                    "the Windows target changed while its predecessor hard link was reserved",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(GuardIntegrationError::runtime(format!(
                    "failed to preserve the Windows managed-file predecessor for {}: {error}",
                    parent.target_path.display()
                )));
            }
        }
    }
    Err(GuardIntegrationError::runtime(format!(
        "failed to allocate a Windows predecessor entry for {}",
        parent.target_path.display()
    )))
}

#[cfg(windows)]
fn windows_pinned_file_matches_expected(
    file: &std::fs::File,
    expected: &ManagedTargetSnapshot,
) -> io::Result<bool> {
    let ManagedTargetSnapshot::RegularFile(expected) = expected else {
        return Ok(false);
    };
    let capability_file = CapabilityFile::from_std(file.try_clone()?);
    let before = capability_file.metadata()?;
    let mut reader = file.try_clone()?;
    reader.rewind()?;
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let after = capability_file.metadata()?;
    Ok(before.is_file()
        && after.is_file()
        && ManagedFileIdentity::from_metadata(&before) == expected.identity
        && ManagedFileIdentity::from_metadata(&after) == expected.identity
        && before.len() == expected.len
        && after.len() == expected.len
        && before.permissions().readonly() == expected.readonly
        && after.permissions().readonly() == expected.readonly
        && text == expected.text)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn atomic_commit_if_fresh<F>(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    expected: &ManagedTargetSnapshot,
    staged: &ManagedTargetSnapshot,
    hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    if matches!(expected, ManagedTargetSnapshot::Missing) {
        return atomic_create_if_missing(parent, temp_name, staged, hook);
    }
    cleanup_uncommitted_temp(parent, temp_name, staged)?;
    Err(GuardIntegrationError::runtime(format!(
        "atomic conditional managed-file replacement is unsupported on this platform: {}",
        parent.target_path.display()
    )))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_entry_no_replace(
    parent: &PinnedManagedParent,
    source: &OsString,
    destination: &OsString,
) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags};

    let dir = parent.dir().try_clone()?.into_std_file();
    renameat_with(&dir, source, &dir, destination, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(windows)]
fn rename_entry_no_replace(
    parent: &PinnedManagedParent,
    source: &OsString,
    destination: &OsString,
) -> io::Result<()> {
    volicord_platform_fs::move_file_no_replace(
        &parent.absolute_entry_path(source),
        &parent.absolute_entry_path(destination),
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn rename_entry_no_replace(
    parent: &PinnedManagedParent,
    source: &OsString,
    destination: &OsString,
) -> io::Result<()> {
    parent.dir().hard_link(source, parent.dir(), destination)?;
    parent.dir().remove_file(source)
}

fn unused_sibling_name(
    parent: &PinnedManagedParent,
    role: &str,
) -> Result<OsString, GuardIntegrationError> {
    let target_name = parent.target_name.to_string_lossy();
    for _ in 0..64 {
        let token = random_file_token()?;
        let candidate = OsString::from(format!(".{target_name}.volicord-{role}-{token}"));
        match parent.dir().symlink_metadata(&candidate) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => continue,
            Err(error) => {
                return Err(GuardIntegrationError::runtime(format!(
                    "failed to inspect managed-file {role} path {}: {error}",
                    parent.absolute_entry_path(&candidate).display()
                )));
            }
        }
    }
    Err(GuardIntegrationError::runtime(format!(
        "failed to allocate a managed-file {role} path for {}",
        parent.target_path.display()
    )))
}

fn stale_managed_file_error(path: &Path) -> GuardIntegrationError {
    GuardIntegrationError::runtime(format!(
        "managed file changed since planning: {}",
        path.display()
    ))
}

fn managed_path_conflict(path: &Path, detail: &str) -> GuardIntegrationError {
    GuardIntegrationError::runtime(format!(
        "managed file conflict at {}: {detail}",
        path.display()
    ))
}

pub(crate) fn plan_managed_block_file(
    kind: HostIntegrationFileKind,
    repo_root: &Path,
    path: &Path,
    block: &str,
    start_marker: &'static str,
    end_marker: &'static str,
    require_existing_marker: bool,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let content = block.to_owned();
    let target_snapshot = read_managed_target_snapshot(repo_root, path)?;
    let status = match target_snapshot.text() {
        Some(existing) => {
            if require_existing_marker && !existing.contains(start_marker) {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists without a Volicord-managed block: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
            let updated = managed_block::apply_managed_block_with_markers(
                existing,
                &content,
                start_marker,
                end_marker,
            )
            .map_err(managed_block_conflict)?;
            if updated == existing {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        kind,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::Block {
            start_marker,
            end_marker,
            require_existing_marker,
        },
        target_snapshot,
    })
}

pub(crate) fn plan_policy_file(
    repo_root: &Path,
    path: &Path,
    policy: &Value,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let mut content = serde_json::to_string_pretty(policy)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    content.push('\n');
    let target_snapshot = read_managed_target_snapshot(repo_root, path)?;
    let status = match target_snapshot.text() {
        Some(existing) => {
            let value = serde_json::from_str::<Value>(existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing policy file is not valid JSON: {} ({error})",
                    path.display()
                ))
            })?;
            if !is_volicord_policy(&value) {
                return Err(GuardIntegrationError::runtime(format!(
                    "policy file already exists without Volicord ownership metadata: {}",
                    path.display()
                )));
            }
            if existing == content && !policy_permissions_need_repair(&target_snapshot) {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        kind: HostIntegrationFileKind::VolicordPolicy,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::Json,
        target_snapshot,
    })
}

#[cfg(unix)]
fn policy_permissions_need_repair(snapshot: &ManagedTargetSnapshot) -> bool {
    matches!(
        snapshot,
        ManagedTargetSnapshot::RegularFile(existing) if existing.mode & 0o077 != 0
    )
}

#[cfg(not(unix))]
fn policy_permissions_need_repair(_snapshot: &ManagedTargetSnapshot) -> bool {
    false
}

pub(crate) fn read_managed_text(
    anchor_root: &Path,
    path: &Path,
) -> Result<Option<String>, GuardIntegrationError> {
    Ok(read_managed_target_snapshot(anchor_root, path)?
        .text()
        .map(str::to_owned))
}

pub(crate) fn plan_managed_file_retirement(
    repo_root: &Path,
    capability_file: &Value,
) -> Result<ManagedFileRetirementPlan, GuardIntegrationError> {
    let kind_text = capability_file
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("retirement metadata is missing file kind")
        })?;
    let kind = host_integration_file_kind(kind_text).ok_or_else(|| {
        GuardIntegrationError::runtime(format!(
            "retirement metadata contains unsupported file kind {kind_text}"
        ))
    })?;
    let path = capability_file
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("retirement metadata is missing file path")
        })?;
    let target_snapshot = read_managed_target_snapshot(repo_root, &path)?;
    let Some(existing) = target_snapshot.text() else {
        return Ok(ManagedFileRetirementPlan {
            kind,
            repo_root: repo_root.to_path_buf(),
            path,
            status: RetirementPlanStatus::Unchanged,
            target_snapshot,
            replacement: None,
        });
    };
    let expected_hash = capability_file
        .get("content_hash")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "retirement metadata is missing content hash for {}",
                path.display()
            ))
        })?;
    let replacement = match capability_file.get("ownership").and_then(Value::as_str) {
        Some("managed_block") => {
            let start = capability_file
                .get("managed_marker_start")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    GuardIntegrationError::runtime(
                        "managed-block retirement is missing start marker",
                    )
                })?;
            let end = capability_file
                .get("managed_marker_end")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    GuardIntegrationError::runtime("managed-block retirement is missing end marker")
                })?;
            let (managed, remaining) = remove_verified_managed_block(existing, start, end)?;
            if sha256_text(managed) != expected_hash {
                return Err(retirement_changed_error(&path));
            }
            (!remaining.trim().is_empty()).then_some(remaining.to_owned())
        }
        Some("managed_json") | Some("managed_script") => {
            if sha256_text(existing) != expected_hash {
                return Err(retirement_changed_error(&path));
            }
            None
        }
        Some("managed_json_projection") => {
            let projection = capability_file
                .get("managed_projection")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    GuardIntegrationError::runtime(
                        "managed JSON retirement is missing projection kind",
                    )
                })?;
            if projection != ManagedJsonProjection::ClaudeCodeSettingsHooks.as_str() {
                return Err(GuardIntegrationError::runtime(format!(
                    "managed JSON retirement does not support projection {projection}"
                )));
            }
            let desired_text = capability_file
                .get("managed_projection_json")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    GuardIntegrationError::runtime(
                        "managed JSON retirement is missing projection content",
                    )
                })?;
            if sha256_text(desired_text) != expected_hash {
                return Err(GuardIntegrationError::runtime(format!(
                    "managed JSON retirement metadata is inconsistent for {}",
                    path.display()
                )));
            }
            let actual = serde_json::from_str::<Value>(existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing managed JSON retirement target is invalid: {} ({error})",
                    path.display()
                ))
            })?;
            let desired = serde_json::from_str::<Value>(desired_text).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "recorded managed JSON projection is invalid: {} ({error})",
                    path.display()
                ))
            })?;
            let retired = remove_claude_settings_hooks(&actual, &desired)
                .map_err(|_| retirement_changed_error(&path))?;
            if retired == actual {
                return Ok(ManagedFileRetirementPlan {
                    kind,
                    repo_root: repo_root.to_path_buf(),
                    path,
                    status: RetirementPlanStatus::Unchanged,
                    target_snapshot,
                    replacement: None,
                });
            }
            if retired.as_object().is_some_and(serde_json::Map::is_empty) {
                None
            } else {
                let mut text = serde_json::to_string_pretty(&retired)
                    .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
                text.push('\n');
                Some(text)
            }
        }
        _ => {
            return Err(GuardIntegrationError::runtime(format!(
                "retirement metadata has unsupported ownership for {}",
                path.display()
            )));
        }
    };
    let status = if replacement.is_some() {
        RetirementPlanStatus::PlannedUpdate
    } else {
        RetirementPlanStatus::PlannedRemove
    };
    Ok(ManagedFileRetirementPlan {
        kind,
        repo_root: repo_root.to_path_buf(),
        path,
        status,
        target_snapshot,
        replacement,
    })
}

pub(crate) fn apply_managed_file_retirement(
    plan: &ManagedFileRetirementPlan,
) -> Result<RetirementPlanStatus, GuardIntegrationError> {
    match plan.status {
        RetirementPlanStatus::Unchanged => return Ok(RetirementPlanStatus::Unchanged),
        RetirementPlanStatus::PlannedUpdate | RetirementPlanStatus::PlannedRemove => {}
        other => return Ok(other),
    }
    ensure_retirement_plan_fresh(plan)?;
    if let Some(replacement) = &plan.replacement {
        let replacement_plan = GeneratedFilePlan {
            kind: plan.kind,
            repo_root: plan.repo_root.clone(),
            path: plan.path.clone(),
            content: replacement.clone(),
            status: FilePlanStatus::PlannedUpdate,
            write_kind: GeneratedFileWriteKind::Json,
            target_snapshot: plan.target_snapshot.clone(),
        };
        write_managed_file_if_fresh(&replacement_plan, replacement, false)?;
        return Ok(RetirementPlanStatus::Updated);
    }

    let parent = match open_pinned_managed_parent(&plan.repo_root, &plan.path, false)? {
        PinnedParentOpen::Missing => return Ok(RetirementPlanStatus::Unchanged),
        PinnedParentOpen::Ready(parent) => parent,
    };
    parent.validate_attached()?;
    ensure_expected_snapshot(&parent, &plan.target_snapshot)?;
    let target_name = parent.target_name.clone();
    let expected = RecoveryEntrySnapshot::from(&plan.target_snapshot);
    remove_exact_entry(&parent, &target_name, &expected, "retired managed file")?;
    if parent.read_target_snapshot()? != ManagedTargetSnapshot::Missing {
        return Err(GuardIntegrationError::runtime(format!(
            "retired managed file still exists after removal: {}",
            plan.path.display()
        )));
    }
    Ok(RetirementPlanStatus::Removed)
}

fn ensure_retirement_plan_fresh(
    plan: &ManagedFileRetirementPlan,
) -> Result<(), GuardIntegrationError> {
    if read_managed_target_snapshot(&plan.repo_root, &plan.path)? == plan.target_snapshot {
        Ok(())
    } else {
        Err(retirement_changed_error(&plan.path))
    }
}

fn retirement_changed_error(path: &Path) -> GuardIntegrationError {
    GuardIntegrationError::runtime(format!(
        "managed retirement target changed or no longer matches Volicord ownership: {}",
        path.display()
    ))
}

fn remove_verified_managed_block<'a>(
    existing: &'a str,
    start_marker: &str,
    end_marker: &str,
) -> Result<(&'a str, String), GuardIntegrationError> {
    if existing.matches(start_marker).count() != 1 || existing.matches(end_marker).count() != 1 {
        return Err(GuardIntegrationError::runtime(
            "managed retirement target has missing or duplicate block markers",
        ));
    }
    let start = existing
        .find(start_marker)
        .expect("marker count was checked");
    let end_from_start = existing[start..].find(end_marker).ok_or_else(|| {
        GuardIntegrationError::runtime("managed retirement target has unpaired block markers")
    })?;
    let mut end = start + end_from_start + end_marker.len();
    if existing[end..].starts_with("\r\n") {
        end += 2;
    } else if existing[end..].starts_with('\n') {
        end += 1;
    }
    let managed = &existing[start..end];
    let mut remaining = String::with_capacity(existing.len() - managed.len());
    remaining.push_str(&existing[..start]);
    remaining.push_str(&existing[end..]);
    Ok((managed, remaining))
}

fn remove_claude_settings_hooks(current: &Value, desired: &Value) -> Result<Value, ()> {
    let mut root = current.as_object().cloned().ok_or(())?;
    let desired_hooks = desired.get("hooks").and_then(Value::as_object).ok_or(())?;
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Ok(Value::Object(root));
    };
    for (event_name, desired_groups) in desired_hooks {
        let desired_group = desired_groups
            .as_array()
            .and_then(|groups| groups.first())
            .ok_or(())?;
        let Some(actual_groups) = hooks.get_mut(event_name).and_then(Value::as_array_mut) else {
            continue;
        };
        let matching = actual_groups
            .iter()
            .filter(|group| *group == desired_group)
            .count();
        if matching > 1 || (matching == 0 && actual_groups.iter().any(json_value_mentions_volicord))
        {
            return Err(());
        }
        if matching == 0 {
            continue;
        }
        actual_groups.retain(|group| group != desired_group);
        if actual_groups.is_empty() {
            hooks.remove(event_name);
        }
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(Value::Object(root))
}

fn json_value_mentions_volicord(value: &Value) -> bool {
    match value {
        Value::String(value) => value.to_ascii_lowercase().contains("volicord"),
        Value::Array(values) => values.iter().any(json_value_mentions_volicord),
        Value::Object(values) => values.values().any(json_value_mentions_volicord),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn host_integration_file_kind(value: &str) -> Option<HostIntegrationFileKind> {
    match value {
        "volicord_policy" => Some(HostIntegrationFileKind::VolicordPolicy),
        "git_info_exclude" => Some(HostIntegrationFileKind::GitInfoExclude),
        "host_mcp_config" => Some(HostIntegrationFileKind::HostMcpConfig),
        "host_hook_config" => Some(HostIntegrationFileKind::HostHookConfig),
        "host_hook_dispatch" => Some(HostIntegrationFileKind::HostHookDispatch),
        "host_hook_wrapper" => Some(HostIntegrationFileKind::HostHookWrapper),
        "host_rule_instruction" => Some(HostIntegrationFileKind::HostRuleInstruction),
        "agents_managed_block" => Some(HostIntegrationFileKind::AgentsManagedBlock),
        _ => None,
    }
}

pub(crate) fn plan_managed_exact_json_file(
    kind: HostIntegrationFileKind,
    repo_root: &Path,
    path: &Path,
    value: &Value,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    content.push('\n');
    let target_snapshot = read_managed_target_snapshot(repo_root, path)?;
    let status = match target_snapshot.text() {
        Some(existing) => {
            let existing_value = serde_json::from_str::<Value>(existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing {} is not valid JSON: {} ({error})",
                    kind.as_str(),
                    path.display()
                ))
            })?;
            if existing_value == *value {
                if existing == content {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if kind == HostIntegrationFileKind::HostHookConfig
                && is_volicord_codex_hook_config(&existing_value)
            {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        kind,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::ExactJson,
        target_snapshot,
    })
}

pub(crate) fn plan_managed_json_projection_file(
    kind: HostIntegrationFileKind,
    repo_root: &Path,
    path: &Path,
    value: &Value,
    projection: ManagedJsonProjection,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let mut content = canonical_json_text(value)?;
    content.push('\n');
    let target_snapshot = read_managed_target_snapshot(repo_root, path)?;
    let status = match target_snapshot.text() {
        Some(existing) => {
            let existing_value = serde_json::from_str::<Value>(existing).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing {} is not valid JSON: {} ({error})",
                    kind.as_str(),
                    path.display()
                ))
            })?;
            let merged = managed_json_projection_merge(&existing_value, value, projection)?;
            if merged == existing_value {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        kind,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::JsonProjection { projection },
        target_snapshot,
    })
}

pub(crate) fn plan_managed_script_file(
    repo_root: &Path,
    path: &Path,
    content: &str,
    kind: HostIntegrationFileKind,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let target_snapshot = read_managed_target_snapshot(repo_root, path)?;
    let status = match target_snapshot.text() {
        Some(existing) => {
            if existing == content {
                if script_is_executable(path) {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if existing.contains(HOOK_WRAPPER_MARKER) {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        kind,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content: content.to_owned(),
        status,
        write_kind: GeneratedFileWriteKind::Script,
        target_snapshot,
    })
}

pub(crate) fn managed_json_projection_merge(
    current: &Value,
    desired: &Value,
    projection: ManagedJsonProjection,
) -> Result<Value, GuardIntegrationError> {
    let merged = match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => {
            merge_claude_settings_hooks(current, desired)
        }
        ManagedJsonProjection::ClaudeCodeMcpEntry => merge_claude_mcp_entry(current, desired),
    }?;
    validate_managed_json_projection_config(projection, &merged)?;
    Ok(merged)
}

pub(crate) fn managed_block_conflict(error: ManagedBlockError) -> GuardIntegrationError {
    match error {
        ManagedBlockError::Unterminated { start_marker } => GuardIntegrationError::runtime(
            format!("managed block starting with {start_marker} is missing its end marker"),
        ),
        ManagedBlockError::Duplicate { start_marker } => GuardIntegrationError::runtime(format!(
            "multiple managed blocks starting with {start_marker} were found"
        )),
    }
}

fn canonical_json_text(value: &Value) -> Result<String, GuardIntegrationError> {
    serde_json::to_string(value).map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

fn validate_managed_json_projection_config(
    projection: ManagedJsonProjection,
    value: &Value,
) -> Result<(), GuardIntegrationError> {
    let text = serde_json::to_string(value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let (kind, label) = match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => (
            HostContractConfigKind::ProjectSettings,
            "merged Claude Code project settings",
        ),
        ManagedJsonProjection::ClaudeCodeMcpEntry => (
            HostContractConfigKind::McpConfig,
            "merged Claude Code MCP config",
        ),
    };
    validate_contract_config(HostKind::ClaudeCode, kind, &text).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "{label} do not match the verified contract: {error}"
        ))
    })
}

fn merge_claude_mcp_entry(
    current: &Value,
    desired: &Value,
) -> Result<Value, GuardIntegrationError> {
    let mut object = current.as_object().cloned().ok_or_else(|| {
        GuardIntegrationError::runtime("Claude Code .mcp.json must be a JSON object")
    })?;
    let desired_servers = desired
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| GuardIntegrationError::runtime("managed MCP projection is invalid"))?;
    let servers = object
        .entry("mcpServers".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            GuardIntegrationError::runtime("Claude Code .mcp.json mcpServers must be an object")
        })?;
    for (name, entry) in desired_servers {
        servers.insert(name.clone(), entry.clone());
    }
    Ok(Value::Object(object))
}

fn merge_claude_settings_hooks(
    current: &Value,
    desired: &Value,
) -> Result<Value, GuardIntegrationError> {
    let mut root = current.as_object().cloned().ok_or_else(|| {
        GuardIntegrationError::runtime("Claude Code settings must be a JSON object")
    })?;
    let desired_hooks = desired
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("managed Claude Code hook projection is invalid")
        })?;
    let hooks = root
        .entry("hooks".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            GuardIntegrationError::runtime("Claude Code settings hooks must be an object")
        })?;
    for phase in REQUIRED_GUARD_PHASES {
        let event_name = claude_event_name(phase)?;
        let desired_groups = desired_hooks
            .get(event_name)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "managed Claude Code hook projection is missing {event_name}"
                ))
            })?;
        let desired_group = desired_groups.first().cloned().ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "managed Claude Code hook projection has no {event_name} group"
            ))
        })?;
        let desired_handler = claude_managed_group_signature(&desired_group, event_name)?;
        let existing_groups = hooks
            .remove(event_name)
            .map(|value| {
                value.as_array().cloned().ok_or_else(|| {
                    GuardIntegrationError::runtime(format!(
                        "Claude Code settings hook event {event_name} must be an array"
                    ))
                })
            })
            .transpose()?
            .unwrap_or_default();
        let mut preserved_groups = Vec::new();
        for group in existing_groups {
            if let Some(group) =
                remove_claude_managed_handlers(phase, event_name, &desired_handler, group)?
            {
                preserved_groups.push(group);
            }
        }
        preserved_groups.push(desired_group);
        hooks.insert(event_name.to_owned(), Value::Array(preserved_groups));
    }
    Ok(Value::Object(root))
}

fn remove_claude_managed_handlers(
    phase: HostLifecyclePhase,
    event_name: &str,
    desired_handler: &ClaudeHookHandlerSignature,
    group: Value,
) -> Result<Option<Value>, GuardIntegrationError> {
    let mut object = group.as_object().cloned().ok_or_else(|| {
        GuardIntegrationError::runtime(format!(
            "Claude Code settings hook group for {event_name} must be an object"
        ))
    })?;
    let handlers = object
        .remove("hooks")
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "Claude Code settings hook group for {event_name} must contain hooks"
            ))
        })?
        .as_array()
        .cloned()
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "Claude Code settings hook handlers for {event_name} must be an array"
            ))
        })?;
    let mut kept = Vec::new();
    let mut removed = 0usize;
    for handler in handlers {
        if is_exact_claude_managed_handler(&handler, desired_handler) {
            removed += 1;
        } else if looks_like_conflicting_claude_managed_handler(phase, &handler, desired_handler) {
            return Err(GuardIntegrationError::runtime(format!(
                "Claude Code settings contain a conflicting Volicord-managed {event_name} hook entry"
            )));
        } else {
            kept.push(handler);
        }
    }
    if removed == 0 {
        object.insert("hooks".to_owned(), Value::Array(kept));
        return Ok(Some(Value::Object(object)));
    }
    if kept.is_empty() {
        return Ok(None);
    }
    object.insert("hooks".to_owned(), Value::Array(kept));
    Ok(Some(Value::Object(object)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaudeHookHandlerSignature {
    command: String,
    args: Option<Vec<String>>,
}

fn claude_managed_group_signature(
    group: &Value,
    event_name: &str,
) -> Result<ClaudeHookHandlerSignature, GuardIntegrationError> {
    let handler = group
        .get("hooks")
        .and_then(Value::as_array)
        .and_then(|handlers| handlers.first())
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "managed Claude Code hook projection is missing {event_name} handler"
            ))
        })?;
    let command = handler
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "managed Claude Code hook projection is missing {event_name} command"
            ))
        })?;
    let args = match handler.get("args") {
        Some(value) => {
            let values = value.as_array().ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "managed Claude Code hook projection has non-array {event_name} args"
                ))
            })?;
            Some(
                values
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        GuardIntegrationError::runtime(format!(
                            "managed Claude Code hook projection has non-string {event_name} args"
                        ))
                    })?,
            )
        }
        None => None,
    };
    Ok(ClaudeHookHandlerSignature { command, args })
}

fn is_exact_claude_managed_handler(handler: &Value, desired: &ClaudeHookHandlerSignature) -> bool {
    handler.as_object().is_some_and(|object| {
        object.get("type").and_then(Value::as_str) == Some("command")
            && object
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| command == desired.command)
            && hook_handler_args(object) == desired.args
    })
}

fn looks_like_conflicting_claude_managed_handler(
    phase: HostLifecyclePhase,
    handler: &Value,
    desired: &ClaudeHookHandlerSignature,
) -> bool {
    handler.as_object().is_some_and(|object| {
        object
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| {
                (command != desired.command || hook_handler_args(object) != desired.args)
                    && ((command.contains("volicord _hook")
                        && command.contains(phase.command_name())
                        && (command.contains("--host claude-code")
                            || command.contains("--host claude_code")
                            || command.contains("--guard-installation")))
                        || command.contains(&format!(
                            ".claude/hooks/volicord-{}.sh",
                            phase.command_name()
                        )))
            })
    })
}

fn hook_handler_args(object: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    object
        .get("args")
        .and_then(Value::as_array)
        .and_then(|args| {
            args.iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
}

fn claude_event_name(phase: HostLifecyclePhase) -> Result<&'static str, GuardIntegrationError> {
    let contract = contract_for(HostKind::ClaudeCode).ok_or_else(|| {
        GuardIntegrationError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Claude Code host integration contract is available",
        )
    })?;
    hook_event_for_phase(contract, phase)
        .map(|event| event.event_name)
        .ok_or_else(|| {
            GuardIntegrationError::runtime(format!(
                "DETECTIVE_HOOKS_UNSUPPORTED: Claude Code contract is missing {} hook event data",
                phase.capability_name()
            ))
        })
}

fn is_volicord_policy(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(VOLICORD_POLICY_SCHEMA)
        && value.get("managed_by").and_then(Value::as_str) == Some("volicord")
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use volicord_test_support::TempRuntimeHome;

    use super::*;

    fn managed_auxiliary_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
        let mut paths = fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.contains(".volicord-tmp-")
                        || name.contains(".volicord-displaced-")
                        || name.contains(".volicord-backup-")
                        || name.contains(".volicord-predecessor-")
                        || name.contains(".volicord-rollback-")
                        || name.contains(".volicord-cleanup-")
                })
            })
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }

    fn owned_policy(value: &str) -> Value {
        json!({
            "schema": VOLICORD_POLICY_SCHEMA,
            "managed_by": "volicord",
            "value": value,
        })
    }

    fn claude_settings_projection() -> Result<Value, GuardIntegrationError> {
        let mut hooks = serde_json::Map::new();
        for phase in REQUIRED_GUARD_PHASES {
            hooks.insert(
                claude_event_name(phase)?.to_owned(),
                json!([{
                    "hooks": [{
                        "type": "command",
                        "command": format!(
                            "${{CLAUDE_PROJECT_DIR}}/.claude/hooks/volicord-{}.sh",
                            phase.command_name()
                        ),
                        "args": []
                    }]
                }]),
            );
        }
        Ok(json!({ "hooks": hooks }))
    }

    fn claude_settings_retirement_capability(
        path: &Path,
        desired: &Value,
    ) -> Result<Value, GuardIntegrationError> {
        let mut desired_text = canonical_json_text(desired)?;
        desired_text.push('\n');
        Ok(json!({
            "kind": HostIntegrationFileKind::HostHookConfig.as_str(),
            "path": path.to_string_lossy(),
            "content_hash": sha256_text(&desired_text),
            "ownership": "managed_json_projection",
            "managed_projection": ManagedJsonProjection::ClaudeCodeSettingsHooks.as_str(),
            "managed_projection_json": desired_text,
        }))
    }

    #[test]
    fn claude_settings_merge_is_idempotent_for_exact_projection(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let desired = claude_settings_projection()?;

        let merged = merge_claude_settings_hooks(&desired, &desired)?;

        assert_eq!(merged, desired);
        Ok(())
    }

    #[test]
    fn claude_settings_merge_preserves_unmanaged_handlers() -> Result<(), Box<dyn std::error::Error>>
    {
        let desired = claude_settings_projection()?;
        let event_name = claude_event_name(HostLifecyclePhase::PreTool)?;
        let current = json!({
            "hooks": {
                (event_name): [{
                    "hooks": [{
                        "type": "command",
                        "command": "./user-owned-pre-tool.sh"
                    }]
                }]
            }
        });

        let merged = merge_claude_settings_hooks(&current, &desired)?;
        let groups = merged["hooks"][event_name]
            .as_array()
            .expect("merged hook event should contain groups");

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0]["hooks"][0]["command"], "./user-owned-pre-tool.sh");
        Ok(())
    }

    #[test]
    fn claude_settings_merge_rejects_conflicting_hook_shape(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let desired = claude_settings_projection()?;
        let event_name = claude_event_name(HostLifecyclePhase::PreTool)?;
        let current = json!({
            "hooks": {
                (event_name): [{
                    "hooks": [{
                        "type": "command",
                        "command": "sh -c 'echo user-owned; .claude/hooks/volicord-pre-tool.sh'"
                    }]
                }]
            }
        });

        let error = merge_claude_settings_hooks(&current, &desired)
            .expect_err("a non-exact Volicord-like hook must be reported as a conflict");

        assert!(error
            .to_string()
            .contains("conflicting Volicord-managed PreToolUse hook entry"));
        Ok(())
    }

    #[test]
    fn claude_settings_retirement_preserves_unmanaged_json_and_is_idempotent(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-retire-claude-settings")?;
        let repo = fixture.path().join("repo");
        let target = repo.join(".claude/settings.local.json");
        fs::create_dir_all(target.parent().expect("settings parent"))?;
        let current = json!({
            "permissions": { "allow": ["Read"] },
            "hooks": {
                "PreToolUse": [{
                    "hooks": [{
                        "type": "command",
                        "command": "./user-owned-pre-tool.sh"
                    }]
                }]
            }
        });
        let desired = claude_settings_projection()?;
        let installed = merge_claude_settings_hooks(&current, &desired)?;
        fs::write(&target, serde_json::to_string_pretty(&installed)? + "\n")?;
        let capability = claude_settings_retirement_capability(&target, &desired)?;

        let retirement = plan_managed_file_retirement(&repo, &capability)?;
        assert_eq!(retirement.status, RetirementPlanStatus::PlannedUpdate);
        assert_eq!(
            apply_managed_file_retirement(&retirement)?,
            RetirementPlanStatus::Updated
        );
        let preserved: Value = serde_json::from_str(&fs::read_to_string(&target)?)?;
        assert_eq!(preserved["permissions"]["allow"], json!(["Read"]));
        assert_eq!(
            preserved["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
            "./user-owned-pre-tool.sh"
        );
        assert!(!fs::read_to_string(&target)?.contains("volicord"));

        let rerun = plan_managed_file_retirement(&repo, &capability)?;
        assert_eq!(rerun.status, RetirementPlanStatus::Unchanged);
        assert_eq!(
            apply_managed_file_retirement(&rerun)?,
            RetirementPlanStatus::Unchanged
        );
        Ok(())
    }

    #[test]
    fn claude_settings_retirement_fails_closed_after_managed_projection_change(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-retire-changed-claude-settings")?;
        let repo = fixture.path().join("repo");
        let target = repo.join(".claude/settings.local.json");
        fs::create_dir_all(target.parent().expect("settings parent"))?;
        let desired = claude_settings_projection()?;
        fs::write(&target, serde_json::to_string_pretty(&desired)? + "\n")?;
        let capability = claude_settings_retirement_capability(&target, &desired)?;
        let mut changed: Value = serde_json::from_str(&fs::read_to_string(&target)?)?;
        changed["hooks"]["PreToolUse"][0]["hooks"][0]["timeout"] = json!(99);
        fs::write(&target, serde_json::to_string_pretty(&changed)? + "\n")?;

        let error = plan_managed_file_retirement(&repo, &capability)
            .expect_err("changed managed hook projection must not be retired");

        assert!(error
            .to_string()
            .contains("no longer matches Volicord ownership"));
        assert_eq!(
            serde_json::from_str::<Value>(&fs::read_to_string(&target)?)?,
            changed
        );
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn existing_managed_file_is_replaced_atomically() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-existing-update")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        fs::write(
            &target,
            serde_json::to_string_pretty(&owned_policy("old"))? + "\n",
        )?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        write_managed_file_if_fresh(&plan, &plan.content, false)?;

        assert_eq!(fs::read_to_string(&target)?, plan.content);
        assert!(managed_auxiliary_files(&managed_dir)?.is_empty());
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn concurrent_change_after_freshness_check_is_restored_without_loss(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-concurrent-change")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        fs::write(
            &target,
            serde_json::to_string_pretty(&owned_policy("old"))? + "\n",
        )?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitReady {
                fs::write(&target, "concurrent writer bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a post-check concurrent change must reject the conditional update");

        assert!(error.to_string().contains("changed since planning"));
        assert_eq!(fs::read_to_string(&target)?, "concurrent writer bytes\n");
        assert!(managed_auxiliary_files(&managed_dir)?.is_empty());
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn concurrent_creation_after_freshness_check_is_not_overwritten(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-concurrent-create")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&repo)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitReady {
                fs::write(&target, "concurrent creator bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a concurrent creator must win the no-replace operation");

        assert!(error.to_string().contains("changed since planning"));
        assert_eq!(fs::read_to_string(&target)?, "concurrent creator bytes\n");
        assert!(managed_auxiliary_files(&managed_dir)?.is_empty());
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn concurrent_change_after_create_publish_is_restored_as_target(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-concurrent-published-create")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&repo)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitApplied {
                fs::write(&target, "concurrent published bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a change after no-replace publication must reject the staged create");

        assert!(error.to_string().contains("changed since planning"));
        assert_eq!(fs::read_to_string(&target)?, "concurrent published bytes\n");
        assert!(managed_auxiliary_files(&managed_dir)?.is_empty());
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn concurrent_replacement_after_exchange_is_preserved_as_residual(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-concurrent-post-exchange")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitApplied {
                fs::remove_file(&target)?;
                fs::write(&target, "concurrent post-exchange bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a replacement after exchange must stop cleanup of concurrent bytes");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(fs::read_to_string(&target)?, original);
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(
            fs::read_to_string(&auxiliary[0])?,
            "concurrent post-exchange bytes\n"
        );
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn concurrent_windows_target_replacement_is_preserved_as_residual(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-windows-concurrent-native-replace")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::NativeCommitReady {
                fs::remove_file(&target)?;
                fs::write(&target, "concurrent Windows target bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a Windows target replacement must stop successful commit");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(fs::read_to_string(&target)?, original);
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(
            fs::read_to_string(&auxiliary[0])?,
            "concurrent Windows target bytes\n"
        );
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn second_windows_writer_stops_automatic_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-windows-second-rollback-writer")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::NativeCommitReady {
                fs::remove_file(&target)?;
                fs::write(&target, "first Windows writer bytes\n")?;
            }
            if phase == ManagedWritePhase::RollbackReady {
                fs::write(&target, "second Windows writer bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a second Windows writer must stop automatic rollback");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(
            fs::read_to_string(&target)?,
            "second Windows writer bytes\n"
        );
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 2);
        let mut contents = auxiliary
            .iter()
            .map(fs::read_to_string)
            .collect::<Result<Vec<_>, _>>()?;
        contents.sort();
        let mut expected = vec![original, "first Windows writer bytes\n".to_owned()];
        expected.sort();
        assert_eq!(contents, expected);
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn changed_staged_file_is_rejected_without_deleting_concurrent_bytes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-staged-swap")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitReady {
                let auxiliary = managed_auxiliary_files(&managed_dir)?;
                assert_eq!(auxiliary.len(), 1);
                fs::write(&auxiliary[0], "changed staged bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a changed staged file must fail pre-commit validation");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(fs::read_to_string(&target)?, original);
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(fs::read_to_string(&auxiliary[0])?, "changed staged bytes\n");
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn changed_staged_create_is_rejected_without_deleting_concurrent_bytes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-staged-create-swap")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&repo)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitReady {
                let auxiliary = managed_auxiliary_files(&managed_dir)?;
                assert_eq!(auxiliary.len(), 1);
                fs::write(&auxiliary[0], "changed staged bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a changed staged create must fail pre-commit validation");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert!(!target.exists());
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(fs::read_to_string(&auxiliary[0])?, "changed staged bytes\n");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn replaced_temp_name_is_never_adopted_as_staged_content(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-atomic-temp-name-replaced")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::TempReady {
                let auxiliary = managed_auxiliary_files(&managed_dir)?;
                assert_eq!(auxiliary.len(), 1);
                fs::remove_file(&auxiliary[0])?;
                fs::write(&auxiliary[0], "replacement entry bytes\n")?;
            }
            Ok(())
        })
        .expect_err("a replacement entry at the temporary name must never become staged content");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(fs::read_to_string(&target)?, original);
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(
            fs::read_to_string(&auxiliary[0])?,
            "replacement entry bytes\n"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn symlink_at_temp_name_is_not_followed_or_deleted() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let fixture = TempRuntimeHome::new("guard-atomic-temp-name-symlink")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let external = fixture.path().join("external-bytes");
        fs::write(&external, "external bytes\n")?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::TempReady {
                let auxiliary = managed_auxiliary_files(&managed_dir)?;
                assert_eq!(auxiliary.len(), 1);
                fs::remove_file(&auxiliary[0])?;
                symlink(&external, &auxiliary[0])?;
            }
            Ok(())
        })
        .expect_err("a symbolic link at the temporary name must stop the commit");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(fs::read_to_string(&target)?, original);
        assert_eq!(fs::read_to_string(&external)?, "external bytes\n");
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert!(fs::symlink_metadata(&auxiliary[0])?
            .file_type()
            .is_symlink());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn policy_content_is_committed_with_user_only_permissions(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let fixture = TempRuntimeHome::new("guard-atomic-temp-permissions")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        fs::write(
            &target,
            serde_json::to_string_pretty(&owned_policy("old"))? + "\n",
        )?;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640))?;
        let original = fs::metadata(&target)?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;
        let mut observed_restrictive_temp = false;

        write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::TempReady {
                let auxiliary = managed_auxiliary_files(&managed_dir)?;
                assert_eq!(auxiliary.len(), 1);
                let metadata = fs::metadata(&auxiliary[0])?;
                assert_eq!(metadata.mode() & 0o777, 0o600);
                assert_eq!(metadata.len(), 0);
                observed_restrictive_temp = true;
            }
            Ok(())
        })?;

        let committed = fs::metadata(&target)?;
        assert!(observed_restrictive_temp);
        assert_eq!(committed.mode() & 0o777, 0o600);
        assert_eq!(committed.uid(), original.uid());
        assert_eq!(committed.gid(), original.gid());
        assert!(managed_auxiliary_files(&managed_dir)?.is_empty());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn content_bound_extended_attributes_are_not_copied_to_new_bytes() {
        let allowed = vec![(OsString::from("user.volicord-test"), b"value".to_vec())];
        assert!(reject_privileged_content_metadata(&allowed).is_ok());

        for name in ["security.capability", "security.ima", "security.evm"] {
            let attributes = vec![(OsString::from(name), b"value".to_vec())];
            let error = reject_privileged_content_metadata(&attributes)
                .expect_err("content-bound security metadata must reject replacement");
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            assert!(error.to_string().contains(name));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn existing_extended_attributes_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
        use rustix::fs::{getxattr, setxattr, XattrFlags};

        let fixture = TempRuntimeHome::new("guard-atomic-xattrs")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        fs::write(
            &target,
            serde_json::to_string_pretty(&owned_policy("old"))? + "\n",
        )?;
        setxattr(
            &target,
            "user.volicord-test",
            b"managed metadata",
            XattrFlags::empty(),
        )?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        write_managed_file_if_fresh(&plan, &plan.content, false)?;

        let mut value = vec![0_u8; 128];
        let length = getxattr(&target, "user.volicord-test", &mut value)?;
        value.truncate(length);
        assert_eq!(value, b"managed metadata");
        assert!(managed_auxiliary_files(&managed_dir)?.is_empty());
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn second_writer_during_rollback_is_not_overwritten() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("guard-atomic-rollback-writer")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        fs::write(
            &target,
            serde_json::to_string_pretty(&owned_policy("old"))? + "\n",
        )?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            match phase {
                ManagedWritePhase::CommitReady => {
                    fs::write(&target, "first concurrent writer bytes\n")?;
                }
                ManagedWritePhase::RollbackReady => {
                    fs::write(&target, "second concurrent writer bytes\n")?;
                }
                ManagedWritePhase::TempReady
                | ManagedWritePhase::CommitApplied
                | ManagedWritePhase::RollbackInspecting => {}
                #[cfg(windows)]
                ManagedWritePhase::CommitReserved => {}
                #[cfg(windows)]
                ManagedWritePhase::NativeCommitReady => {}
            }
            Ok(())
        })
        .expect_err("a second rollback writer must stop automatic rollback");

        assert!(error
            .to_string()
            .contains("recovery entries present at inspection"));
        assert_eq!(
            fs::read_to_string(&target)?,
            "second concurrent writer bytes\n"
        );
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert!(auxiliary[0]
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains(".volicord-tmp-")));
        assert_eq!(
            fs::read_to_string(&auxiliary[0])?,
            "first concurrent writer bytes\n"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn parent_symlink_swap_after_freshness_check_cannot_redirect_write(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let fixture = TempRuntimeHome::new("guard-atomic-parent-swap")?;
        let external = TempRuntimeHome::new("guard-atomic-parent-swap-external")?;
        let repo = fixture.path().join("repo");
        let managed_dir = repo.join(".volicord");
        let detached_dir = repo.join(".volicord-detached");
        fs::create_dir_all(&managed_dir)?;
        let target = repo.join(VOLICORD_POLICY_FILE);
        let original = serde_json::to_string_pretty(&owned_policy("old"))? + "\n";
        fs::write(&target, &original)?;
        let external_target = external.path().join("policy.json");
        fs::write(&external_target, "external bytes\n")?;
        let plan = plan_policy_file(&repo, &target, &owned_policy("new"))?;

        let error = write_managed_file_if_fresh_with_hook(&plan, &plan.content, false, |phase| {
            if phase == ManagedWritePhase::CommitReady {
                fs::rename(&managed_dir, &detached_dir)?;
                symlink(external.path(), &managed_dir)?;
            }
            Ok(())
        })
        .expect_err("a parent symlink swap must invalidate the pinned chain");

        assert!(error
            .to_string()
            .contains("pinned parent component was replaced"));
        assert_eq!(fs::read_to_string(&external_target)?, "external bytes\n");
        assert_eq!(
            fs::read_to_string(detached_dir.join("policy.json"))?,
            original
        );
        assert!(managed_auxiliary_files(&detached_dir)?.is_empty());
        Ok(())
    }
}
