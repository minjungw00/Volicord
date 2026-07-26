use std::{
    ffi::OsString,
    io::{self, Read, Seek, Write},
    path::{Component, Path, PathBuf},
};

use cap_fs_ext::{
    ambient_authority, DirExt, FollowSymlinks, MetadataExt as PortableMetadataExt,
    OpenOptionsFollowExt,
};
use cap_std::fs::{
    Dir as CapabilityDir, File as CapabilityFile, Metadata as CapabilityMetadata,
    OpenOptions as CapabilityOpenOptions,
};
use serde_json::Value;
use volicord_types::guard_manifest::{
    GuardManagedArtifact, GuardManagedMarkerSemantics, GuardManagedOwnership,
    ManagedFileExpectation,
};

use crate::{
    guard_integration::audit::{
        hook_wrapper_comment_value, hook_wrapper_exec_command, is_volicord_codex_hook_config,
        script_is_executable, sha256_text, HOOK_WRAPPER_MARKER,
    },
    managed_block::{self, ManagedBlockError},
};

use super::GuardIntegrationError;

pub(crate) const VOLICORD_POLICY_SCHEMA: &str = volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID;
pub(crate) const GUIDANCE_START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE -->";
pub(crate) const GUIDANCE_END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE -->";

#[derive(Debug, Clone)]
pub(crate) struct GeneratedFilePlan {
    pub(crate) artifact: GuardManagedArtifact,
    pub(crate) repo_root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) content: String,
    pub(crate) status: FilePlanStatus,
    pub(crate) write_kind: GeneratedFileWriteKind,
    pub(crate) target_snapshot: ManagedTargetSnapshot,
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
    Script,
}

pub(crate) fn generated_file_plan_matches_artifact_spec(file: &GeneratedFilePlan) -> bool {
    let spec = file.artifact.spec();
    let git_owner_path =
        (file.artifact == GuardManagedArtifact::GitInfoExclude).then_some(file.path.as_path());
    spec.expected_path(&file.repo_root, git_owner_path)
        .as_deref()
        == Some(file.path.as_path())
        && matches!(
            (spec.ownership, spec.marker_semantics, file.write_kind),
            (
                GuardManagedOwnership::ManagedBlock,
                GuardManagedMarkerSemantics::BlockPair,
                GeneratedFileWriteKind::Block { .. }
            ) | (
                GuardManagedOwnership::ManagedJson,
                GuardManagedMarkerSemantics::None,
                GeneratedFileWriteKind::Json | GeneratedFileWriteKind::ExactJson
            ) | (
                GuardManagedOwnership::ManagedScript,
                GuardManagedMarkerSemantics::ScriptMarker,
                GeneratedFileWriteKind::Script
            )
        )
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
    pub(crate) artifact: GuardManagedArtifact,
    pub(crate) repo_root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) status: RetirementPlanStatus,
    target_snapshot: ManagedTargetSnapshot,
    pub(crate) replacement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetirementPlanStatus {
    PlannedRemove,
    PlannedUpdate,
    Unchanged,
    Removed,
    Updated,
}

pub(crate) fn generated_file_target_bytes(
    file: &GeneratedFilePlan,
) -> Result<Vec<u8>, GuardIntegrationError> {
    if !generated_file_plan_matches_artifact_spec(file) {
        return Err(GuardIntegrationError::runtime(
            "managed file plan does not match the Guard artifact registry",
        ));
    }
    let content = match file.write_kind {
        GeneratedFileWriteKind::Block {
            start_marker,
            end_marker,
            require_existing_marker,
        } => {
            let existing = file.target_snapshot.text().unwrap_or("");
            if require_existing_marker
                && file.target_snapshot.text().is_some()
                && !existing.contains(start_marker)
            {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists without a Volicord-managed block",
                    file.path.display()
                )));
            }
            managed_block::apply_managed_block_with_markers(
                existing,
                &file.content,
                start_marker,
                end_marker,
            )
            .map_err(managed_block_conflict)?
        }
        GeneratedFileWriteKind::Json
        | GeneratedFileWriteKind::ExactJson
        | GeneratedFileWriteKind::Script => file.content.clone(),
    };
    Ok(content.into_bytes())
}

pub(crate) fn generated_file_original_bytes(file: &GeneratedFilePlan) -> Option<&[u8]> {
    file.target_snapshot.text().map(str::as_bytes)
}

pub(crate) fn retirement_file_original_bytes(
    retirement: &ManagedFileRetirementPlan,
) -> Option<&[u8]> {
    retirement.target_snapshot.text().map(str::as_bytes)
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
            plan.artifact == GuardManagedArtifact::VolicordPolicy,
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
    artifact: GuardManagedArtifact,
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
                    artifact.kind().as_str(),
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
        artifact,
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
        artifact: GuardManagedArtifact::VolicordPolicy,
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
    manifest_file: &ManagedFileExpectation,
) -> Result<ManagedFileRetirementPlan, GuardIntegrationError> {
    let artifact = manifest_file.artifact();
    let path = manifest_file.path().to_path_buf();
    if artifact.expected_path(repo_root, None).as_deref() != Some(path.as_path()) {
        return Err(GuardIntegrationError::runtime(format!(
            "managed retirement target is not the registered {} path: {}",
            artifact.kind().as_str(),
            path.display()
        )));
    }
    let target_snapshot = read_managed_target_snapshot(repo_root, &path)?;
    let Some(existing) = target_snapshot.text() else {
        return Ok(ManagedFileRetirementPlan {
            artifact,
            repo_root: repo_root.to_path_buf(),
            path,
            status: RetirementPlanStatus::Unchanged,
            target_snapshot,
            replacement: None,
        });
    };
    let expected_hash = manifest_file.content_hash().as_str();
    let replacement = match manifest_file {
        ManagedFileExpectation::GitInfoExclude { .. }
        | ManagedFileExpectation::HostRuleInstruction { .. }
        | ManagedFileExpectation::AgentsManagedBlock { .. } => {
            let (start, end) = manifest_file
                .block_markers()
                .expect("managed-block variants carry markers");
            let (managed, remaining) = remove_verified_managed_block(existing, start, end)?;
            if sha256_text(managed) != expected_hash {
                return Err(retirement_changed_error(&path));
            }
            (!remaining.trim().is_empty()).then_some(remaining.to_owned())
        }
        ManagedFileExpectation::VolicordPolicy { .. }
        | ManagedFileExpectation::HostHookConfig { .. } => {
            if sha256_text(existing) != expected_hash {
                return Err(retirement_changed_error(&path));
            }
            None
        }
        ManagedFileExpectation::HostHookDispatch { .. }
        | ManagedFileExpectation::HostHookWrapper { .. } => {
            if sha256_text(existing) != expected_hash
                || !managed_script_retirement_metadata_matches_content(manifest_file, existing)
            {
                return Err(retirement_changed_error(&path));
            }
            None
        }
    };
    let status = if replacement.is_some() {
        RetirementPlanStatus::PlannedUpdate
    } else {
        RetirementPlanStatus::PlannedRemove
    };
    Ok(ManagedFileRetirementPlan {
        artifact,
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
            artifact: plan.artifact,
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

fn managed_script_retirement_metadata_matches_content(
    file: &ManagedFileExpectation,
    content: &str,
) -> bool {
    let managed_marker = match file {
        ManagedFileExpectation::HostHookDispatch { managed_marker, .. }
        | ManagedFileExpectation::HostHookWrapper { managed_marker, .. } => managed_marker,
        _ => return false,
    };
    if managed_marker != HOOK_WRAPPER_MARKER
        || !content
            .lines()
            .any(|line| line == format!("# {HOOK_WRAPPER_MARKER}"))
    {
        return false;
    }
    match file {
        ManagedFileExpectation::HostHookDispatch {
            managed_script_role,
            host_kind,
            phase,
            ..
        } => {
            *managed_script_role
                == volicord_types::guard_manifest::GuardManagedScriptRole::CodexDispatch
                && hook_wrapper_comment_value(content, "host_kind") == Some(host_kind.as_str())
                && hook_wrapper_comment_value(content, "phase")
                    == Some(match phase {
                        volicord_types::guard_manifest::GuardDispatchPhase::Dispatch => "dispatch",
                    })
                && hook_wrapper_comment_value(content, "script_role") == Some("codex_dispatch")
        }
        ManagedFileExpectation::HostHookWrapper {
            managed_script_command: expected_command,
            host_kind,
            phase,
            purpose,
            connection_id,
            guard_installation_id,
            policy_hash,
            host_output,
            ..
        } => {
            hook_wrapper_exec_command(content) == Some(expected_command)
                && hook_wrapper_comment_value(content, "host_kind") == Some(host_kind.as_str())
                && hook_wrapper_comment_value(content, "phase") == Some(phase.as_str())
                && hook_wrapper_comment_value(content, "purpose")
                    == Some(match purpose {
                        volicord_types::guard_manifest::GuardManagedScriptPurpose::Guard => "guard",
                    })
                && hook_wrapper_comment_value(content, "connection_id")
                    == Some(connection_id.as_str())
                && hook_wrapper_comment_value(content, "guard_installation_id")
                    == Some(guard_installation_id.as_str())
                && hook_wrapper_comment_value(content, "policy_hash") == Some(policy_hash.as_str())
                && hook_wrapper_comment_value(content, "host_output") == Some(host_output.as_str())
        }
        _ => false,
    }
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

pub(crate) fn plan_managed_exact_json_file(
    artifact: GuardManagedArtifact,
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
                    artifact.kind().as_str(),
                    path.display()
                ))
            })?;
            if existing_value == *value {
                if existing == content {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if artifact == GuardManagedArtifact::HostHookConfig
                && is_volicord_codex_hook_config(&existing_value)
            {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    artifact.kind().as_str(),
                    path.display()
                )));
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        artifact,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::ExactJson,
        target_snapshot,
    })
}

pub(crate) fn plan_managed_script_file(
    repo_root: &Path,
    path: &Path,
    content: &str,
    artifact: GuardManagedArtifact,
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
                    artifact.kind().as_str(),
                    path.display()
                )));
            }
        }
        None => FilePlanStatus::PlannedCreate,
    };
    Ok(GeneratedFilePlan {
        artifact,
        repo_root: repo_root.to_path_buf(),
        path: path.to_path_buf(),
        content: content.to_owned(),
        status,
        write_kind: GeneratedFileWriteKind::Script,
        target_snapshot,
    })
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

fn is_volicord_policy(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(VOLICORD_POLICY_SCHEMA)
        && value.get("managed_by").and_then(Value::as_str) == Some("volicord")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use volicord_test_support::TempRuntimeHome;
    use volicord_types::guard_manifest::{
        GuardArtifactContentHash, GuardManagedArtifact, ManagedFileExpectation,
    };

    use super::{plan_managed_file_retirement, sha256_text, RetirementPlanStatus};

    #[test]
    fn retirement_accepts_only_the_registered_owned_path_and_exact_content(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("guard-retirement-registry")?;
        let repo_root = fixture.create_product_repo("product")?;
        let current_path = GuardManagedArtifact::HostHookConfig
            .expected_path(&repo_root, None)
            .expect("registered hook config path");
        fs::create_dir_all(current_path.parent().expect("hook config parent"))?;
        let current_content = "{\"hooks\":{}}\n";
        fs::write(&current_path, current_content)?;
        let content_hash = GuardArtifactContentHash::parse(sha256_text(current_content))?;
        let current = ManagedFileExpectation::managed_json(
            GuardManagedArtifact::HostHookConfig,
            current_path.clone(),
            content_hash.clone(),
        )?;
        let plan = plan_managed_file_retirement(&repo_root, &current)?;
        assert_eq!(plan.status, RetirementPlanStatus::PlannedRemove);

        let unrelated_path = repo_root.join("user-owned.json");
        fs::write(&unrelated_path, current_content)?;
        let unrelated = ManagedFileExpectation::managed_json(
            GuardManagedArtifact::HostHookConfig,
            unrelated_path.clone(),
            content_hash,
        )?;
        assert!(plan_managed_file_retirement(&repo_root, &unrelated).is_err());
        assert_eq!(fs::read_to_string(&unrelated_path)?, current_content);

        fs::write(&current_path, "{\"hooks\":{\"changed\":true}}\n")?;
        assert!(plan_managed_file_retirement(&repo_root, &current).is_err());
        assert!(current_path.is_file());
        Ok(())
    }
}
