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
        is_volicord_codex_hook_config, script_is_executable, ManagedJsonProjection,
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
pub(crate) const GUIDANCE_START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE v1 -->";
pub(crate) const GUIDANCE_END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE v1 -->";

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
    #[cfg(not(unix))]
    readonly: bool,
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
    CommitReady,
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
    let write_result = (|| -> io::Result<()> {
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;
        configure_temp_permissions(&temp_file, &plan.target_snapshot, executable)?;
        temp_file.sync_all()
    })();
    drop(temp_file);
    if let Err(error) = write_result {
        let _ = parent.dir().remove_file(&temp_name);
        return Err(GuardIntegrationError::runtime(format!(
            "failed to write temporary managed file {}: {error}",
            temp_path.display()
        )));
    }

    let staged_snapshot = match parent.read_entry_snapshot(&temp_name, &temp_path) {
        Ok(snapshot @ ManagedTargetSnapshot::RegularFile(_)) => snapshot,
        Ok(ManagedTargetSnapshot::Missing) => {
            return Err(GuardIntegrationError::runtime(format!(
                "temporary managed file disappeared before commit: {}",
                temp_path.display()
            )));
        }
        Err(error) => {
            let _ = parent.dir().remove_file(&temp_name);
            return Err(error);
        }
    };

    ensure_expected_snapshot(&parent, &plan.target_snapshot).inspect_err(|_| {
        let _ = parent.dir().remove_file(&temp_name);
    })?;
    run_write_hook(&mut hook, ManagedWritePhase::CommitReady, &plan.path).inspect_err(|_| {
        let _ = parent.dir().remove_file(&temp_name);
    })?;
    parent.validate_attached().inspect_err(|_| {
        let _ = parent.dir().remove_file(&temp_name);
    })?;
    atomic_commit_if_fresh(
        &parent,
        &temp_name,
        &plan.target_snapshot,
        &staged_snapshot,
        &mut hook,
    )
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
        let named_after = self.dir().symlink_metadata(name).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to revalidate managed file {}: {error}",
                display_path.display()
            ))
        })?;
        if first != second
            || !stable_file_metadata(&before, &after)
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
            second, &after,
        )))
    }

    fn create_temp_file(&self) -> Result<(OsString, CapabilityFile), GuardIntegrationError> {
        let target_name = self.target_name.to_string_lossy();
        for attempt in 0..1_000_u32 {
            let name = OsString::from(format!(
                ".{target_name}.volicord-tmp-{}-{attempt}",
                std::process::id()
            ));
            let mut options = CapabilityOpenOptions::new();
            options.write(true).create_new(true);
            options.follow(FollowSymlinks::No);
            match self.dir().open_with(&name, &options) {
                Ok(file) => return Ok((name, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(GuardIntegrationError::runtime(format!(
                        "failed to create temporary managed file {}: {error}",
                        self.absolute_entry_path(&name).display()
                    )));
                }
            }
        }
        Err(GuardIntegrationError::runtime(format!(
            "failed to allocate a temporary managed file for {}",
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
) -> ManagedRegularFileSnapshot {
    use cap_std::fs::PermissionsExt;

    ManagedRegularFileSnapshot {
        text,
        identity: ManagedFileIdentity::from_metadata(metadata),
        len: metadata.len(),
        mode: metadata.permissions().mode(),
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
fn stable_file_metadata(before: &CapabilityMetadata, after: &CapabilityMetadata) -> bool {
    use cap_std::fs::PermissionsExt;

    ManagedFileIdentity::from_metadata(before) == ManagedFileIdentity::from_metadata(after)
        && before.len() == after.len()
        && before.permissions().mode() == after.permissions().mode()
}

#[cfg(not(unix))]
fn stable_file_metadata(before: &CapabilityMetadata, after: &CapabilityMetadata) -> bool {
    ManagedFileIdentity::from_metadata(before) == ManagedFileIdentity::from_metadata(after)
        && before.len() == after.len()
        && before.permissions().readonly() == after.permissions().readonly()
}

#[cfg(unix)]
fn configure_temp_permissions(
    file: &CapabilityFile,
    snapshot: &ManagedTargetSnapshot,
    executable: bool,
) -> io::Result<()> {
    use cap_std::fs::PermissionsExt;

    let mut permissions = file.metadata()?.permissions();
    if let ManagedTargetSnapshot::RegularFile(existing) = snapshot {
        permissions.set_mode(existing.mode);
    }
    if executable {
        permissions.set_mode(permissions.mode() | 0o755);
    }
    file.set_permissions(permissions)
}

#[cfg(not(unix))]
fn configure_temp_permissions(
    file: &CapabilityFile,
    snapshot: &ManagedTargetSnapshot,
    _executable: bool,
) -> io::Result<()> {
    if let ManagedTargetSnapshot::RegularFile(existing) = snapshot {
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(existing.readonly);
        file.set_permissions(permissions)?;
    }
    Ok(())
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
        return match rename_entry_no_replace(parent, temp_name, &parent.target_name) {
            Ok(()) => {
                parent.sync_directory();
                Ok(())
            }
            Err(error) => {
                let _ = parent.dir().remove_file(temp_name);
                if error.kind() == io::ErrorKind::AlreadyExists {
                    Err(stale_managed_file_error(&parent.target_path))
                } else {
                    Err(GuardIntegrationError::runtime(format!(
                        "failed to create managed file atomically at {}: {error}",
                        parent.target_path.display()
                    )))
                }
            }
        };
    }

    if let Err(error) = exchange_entries(parent, temp_name, &parent.target_name) {
        let _ = parent.dir().remove_file(temp_name);
        return Err(GuardIntegrationError::runtime(format!(
            "failed to exchange managed file atomically at {}: {error}",
            parent.target_path.display()
        )));
    }
    let displaced = parent.read_entry_snapshot(temp_name, &parent.absolute_entry_path(temp_name));
    if displaced
        .as_ref()
        .is_ok_and(|snapshot| snapshot == expected)
    {
        parent.dir().remove_file(temp_name).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "managed file was committed but its displaced predecessor could not be removed at {}: {error}",
                parent.absolute_entry_path(temp_name).display()
            ))
        })?;
        parent.sync_directory();
        return Ok(());
    }
    rollback_exchange_after_mismatch(parent, temp_name, staged, hook)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rollback_exchange_after_mismatch<F>(
    parent: &PinnedManagedParent,
    displaced_name: &OsString,
    staged: &ManagedTargetSnapshot,
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
    let current = parent.read_target_snapshot();
    if !current.as_ref().is_ok_and(|snapshot| snapshot == staged) {
        let preserved = preserve_entry(parent, displaced_name);
        return Err(concurrent_rollback_error(
            &parent.target_path,
            &preserved,
            "the destination changed before rollback",
        ));
    }
    run_write_hook(hook, ManagedWritePhase::RollbackReady, &parent.target_path)?;
    if let Err(error) = exchange_entries(parent, displaced_name, &parent.target_name) {
        let preserved = preserve_entry(parent, displaced_name);
        return Err(concurrent_rollback_error(
            &parent.target_path,
            &preserved,
            &format!("rollback exchange failed: {error}"),
        ));
    }
    let rollback_displaced =
        parent.read_entry_snapshot(displaced_name, &parent.absolute_entry_path(displaced_name));
    if rollback_displaced
        .as_ref()
        .is_ok_and(|snapshot| snapshot == staged)
    {
        parent.dir().remove_file(displaced_name).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "stale managed-file rollback succeeded but staged bytes could not be removed at {}: {error}",
                parent.absolute_entry_path(displaced_name).display()
            ))
        })?;
        parent.sync_directory();
        Err(stale_managed_file_error(&parent.target_path))
    } else {
        let preserved = preserve_entry(parent, displaced_name);
        parent.sync_directory();
        Err(concurrent_rollback_error(
            &parent.target_path,
            &preserved,
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
        return match rename_entry_no_replace(parent, temp_name, &parent.target_name) {
            Ok(()) => {
                parent.sync_directory();
                Ok(())
            }
            Err(error) => {
                let _ = parent.dir().remove_file(temp_name);
                if error.kind() == io::ErrorKind::AlreadyExists {
                    Err(stale_managed_file_error(&parent.target_path))
                } else {
                    Err(GuardIntegrationError::runtime(format!(
                        "failed to create managed file atomically at {}: {error}",
                        parent.target_path.display()
                    )))
                }
            }
        };
    }

    let backup_name = unused_sibling_name(parent, "displaced")?;
    if let Err(error) =
        replace_file_with_backup(parent, temp_name, &parent.target_name, &backup_name)
    {
        let _ = parent.dir().remove_file(temp_name);
        return Err(GuardIntegrationError::runtime(format!(
            "failed to replace managed file atomically at {}: {error}",
            parent.target_path.display()
        )));
    }
    let displaced =
        parent.read_entry_snapshot(&backup_name, &parent.absolute_entry_path(&backup_name));
    if displaced
        .as_ref()
        .is_ok_and(|snapshot| snapshot == expected)
    {
        parent.dir().remove_file(&backup_name).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "managed file was committed but its displaced predecessor could not be removed at {}: {error}",
                parent.absolute_entry_path(&backup_name).display()
            ))
        })?;
        parent.sync_directory();
        return Ok(());
    }
    rollback_windows_after_mismatch(parent, &backup_name, staged, hook)
}

#[cfg(windows)]
fn rollback_windows_after_mismatch<F>(
    parent: &PinnedManagedParent,
    displaced_name: &OsString,
    staged: &ManagedTargetSnapshot,
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
    let current = parent.read_target_snapshot();
    if !current.as_ref().is_ok_and(|snapshot| snapshot == staged) {
        let preserved = preserve_entry(parent, displaced_name);
        return Err(concurrent_rollback_error(
            &parent.target_path,
            &preserved,
            "the destination changed before rollback",
        ));
    }
    run_write_hook(hook, ManagedWritePhase::RollbackReady, &parent.target_path)?;
    let rollback_backup = unused_sibling_name(parent, "rollback")?;
    if let Err(error) = replace_file_with_backup(
        parent,
        displaced_name,
        &parent.target_name,
        &rollback_backup,
    ) {
        let preserved = preserve_entry(parent, displaced_name);
        return Err(concurrent_rollback_error(
            &parent.target_path,
            &preserved,
            &format!("rollback replacement failed: {error}"),
        ));
    }
    let rollback_displaced = parent.read_entry_snapshot(
        &rollback_backup,
        &parent.absolute_entry_path(&rollback_backup),
    );
    if rollback_displaced
        .as_ref()
        .is_ok_and(|snapshot| snapshot == staged)
    {
        parent.dir().remove_file(&rollback_backup).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "stale managed-file rollback succeeded but staged bytes could not be removed at {}: {error}",
                parent.absolute_entry_path(&rollback_backup).display()
            ))
        })?;
        parent.sync_directory();
        Err(stale_managed_file_error(&parent.target_path))
    } else {
        let preserved = preserve_entry(parent, &rollback_backup);
        parent.sync_directory();
        Err(concurrent_rollback_error(
            &parent.target_path,
            &preserved,
            "a second writer changed the destination during rollback",
        ))
    }
}

#[cfg(windows)]
fn replace_file_with_backup(
    parent: &PinnedManagedParent,
    replacement: &OsString,
    replaced: &OsString,
    backup: &OsString,
) -> io::Result<()> {
    use std::ptr;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let replaced = wide_path(&parent.absolute_entry_path(replaced));
    let replacement = wide_path(&parent.absolute_entry_path(replacement));
    let backup = wide_path(&parent.absolute_entry_path(backup));
    let success = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            backup.as_ptr(),
            0,
            ptr::null(),
            ptr::null(),
        )
    };
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn atomic_commit_if_fresh<F>(
    parent: &PinnedManagedParent,
    temp_name: &OsString,
    expected: &ManagedTargetSnapshot,
    _staged: &ManagedTargetSnapshot,
    _hook: &mut F,
) -> Result<(), GuardIntegrationError>
where
    F: FnMut(ManagedWritePhase) -> io::Result<()>,
{
    if matches!(expected, ManagedTargetSnapshot::Missing) {
        let result = parent
            .dir()
            .hard_link(temp_name, parent.dir(), &parent.target_name);
        return match result {
            Ok(()) => {
                parent.dir().remove_file(temp_name).map_err(|error| {
                    GuardIntegrationError::runtime(format!(
                        "managed file was created but its temporary link could not be removed at {}: {error}",
                        parent.absolute_entry_path(temp_name).display()
                    ))
                })?;
                parent.sync_directory();
                Ok(())
            }
            Err(error) => {
                let _ = parent.dir().remove_file(temp_name);
                if error.kind() == io::ErrorKind::AlreadyExists {
                    Err(stale_managed_file_error(&parent.target_path))
                } else {
                    Err(GuardIntegrationError::runtime(format!(
                        "failed to create managed file atomically at {}: {error}",
                        parent.target_path.display()
                    )))
                }
            }
        };
    }
    let _ = parent.dir().remove_file(temp_name);
    Err(GuardIntegrationError::runtime(format!(
        "atomic conditional managed-file replacement is unsupported on this platform: {}",
        parent.target_path.display()
    )))
}

fn preserve_entry(parent: &PinnedManagedParent, source: &OsString) -> PathBuf {
    let target_name = parent.target_name.to_string_lossy();
    for attempt in 0..1_000_u32 {
        let preserved_name = OsString::from(format!(
            ".{target_name}.volicord-preserved-{}-{attempt}",
            std::process::id()
        ));
        match rename_entry_no_replace(parent, source, &preserved_name) {
            Ok(()) => {
                parent.sync_directory();
                return parent.absolute_entry_path(&preserved_name);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => break,
        }
    }
    parent.absolute_entry_path(source)
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
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source = wide_path(&parent.absolute_entry_path(source));
    let destination = wide_path(&parent.absolute_entry_path(destination));
    let success = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
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

#[cfg(windows)]
fn unused_sibling_name(
    parent: &PinnedManagedParent,
    role: &str,
) -> Result<OsString, GuardIntegrationError> {
    let target_name = parent.target_name.to_string_lossy();
    for attempt in 0..1_000_u32 {
        let candidate = OsString::from(format!(
            ".{target_name}.volicord-{role}-{}-{attempt}",
            std::process::id()
        ));
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

fn concurrent_rollback_error(
    target: &Path,
    preserved: &Path,
    detail: &str,
) -> GuardIntegrationError {
    GuardIntegrationError::runtime(format!(
        "managed file changed during conditional replacement at {}; {detail}; concurrent bytes were preserved at {}",
        target.display(),
        preserved.display()
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
            if existing == content {
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
                        || name.contains(".volicord-rollback-")
                        || name.contains(".volicord-preserved-")
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
    fn second_writer_during_rollback_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
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
                ManagedWritePhase::RollbackInspecting => {}
            }
            Ok(())
        })
        .expect_err("a second rollback writer must be preserved and reported");

        assert!(error.to_string().contains("second writer"));
        assert_eq!(
            fs::read_to_string(&target)?,
            "first concurrent writer bytes\n"
        );
        let auxiliary = managed_auxiliary_files(&managed_dir)?;
        assert_eq!(auxiliary.len(), 1);
        assert!(auxiliary[0]
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains(".volicord-preserved-")));
        assert_eq!(
            fs::read_to_string(&auxiliary[0])?,
            "second concurrent writer bytes\n"
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
