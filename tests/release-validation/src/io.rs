use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{
    de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use serde_json::{Number, Value};
use sha2::{Digest, Sha256};

use crate::error::{ValidationError, ValidationResult};

pub const MAX_CANDIDATE_JSON_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_CELL_JSON_BYTES: u64 = 1024 * 1024;
pub const MAX_MANIFEST_JSON_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_AUDIT_JSON_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_EVIDENCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_VERSION_OUTPUT_BYTES: u64 = 16 * 1024;
const CANDIDATE_VERSION_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const RELEASE_RESULT_ROOT_LOCK_NAME: &str = ".volicord-live-publication.lock";
const RELEASE_RESULT_ROOT_CLEAN_STATE: &[u8] = b"volicord-live-publication-v1 clean\n";
pub(crate) const RELEASE_RESULT_ROOT_ACTIVE_STATE: &[u8] = b"volicord-live-publication-v1 active\n";
const MAX_RELEASE_RESULT_ROOT_STATE_BYTES: u64 = 128;
#[cfg(unix)]
const RESULT_ROOT_LEASE_RETRY_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(unix)]
const RESULT_ROOT_LEASE_RETRY_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateBuildIdentity {
    pub build_id: String,
    pub package_version: String,
    pub git_commit: String,
    pub tree: String,
    pub metadata_source: String,
    pub target: String,
    pub profile: String,
    pub profile_class: String,
    pub profile_exact: String,
    pub opt: String,
    pub debug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateArtifactInspection {
    pub sha256_before: String,
    pub private_copy_sha256: String,
    pub sha256_after_held: String,
    pub sha256_after_path: Option<String>,
    pub path_identity_stable: bool,
    pub build: CandidateBuildIdentity,
}

#[derive(Debug, Clone)]
pub struct ValidationContext {
    source_checkout: PathBuf,
    target_directory: PathBuf,
    docs_directory: PathBuf,
    runtime_homes: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultRootLeaseMode {
    Exclusive,
    Shared,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilesystemIdentity {
    device: u64,
    inode: u64,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilesystemIdentity {
    volume_serial_number: u32,
    file_index: u64,
    attributes: u32,
}

#[cfg(not(any(unix, windows)))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilesystemIdentity;

#[derive(Debug)]
struct RetainedResultPath {
    file: File,
    path: PathBuf,
    identity: FilesystemIdentity,
    directory: bool,
}

struct ResultRootLayout {
    result_root: PathBuf,
    output_directory: PathBuf,
    evidence_directory: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ResultRootLease {
    _lock: File,
    mode: ResultRootLeaseMode,
    lock_path: PathBuf,
    lock_identity: FilesystemIdentity,
    result_root: PathBuf,
    output_directory: PathBuf,
    evidence_directory: Option<PathBuf>,
    retained_paths: Vec<RetainedResultPath>,
    publication_attempt_active: bool,
}

impl ResultRootLease {
    pub fn prevalidate_cell_path(
        context: &ValidationContext,
        cell_path: &Path,
    ) -> ValidationResult<()> {
        cell_result_root_layout(context, cell_path).map(|_| ())
    }

    pub fn prevalidate_auxiliary_path(
        context: &ValidationContext,
        output_path: &Path,
    ) -> ValidationResult<()> {
        auxiliary_result_root_layout(context, output_path).map(|_| ())
    }

    pub fn prevalidate_summary_output(
        context: &ValidationContext,
        cell_directory: &Path,
        output_path: &Path,
    ) -> ValidationResult<()> {
        let layout = cell_directory_result_root_layout(context, cell_directory)?;
        context.validate_new_output(output_path)?;
        if output_path.starts_with(&layout.output_directory)
            || layout
                .evidence_directory
                .as_ref()
                .is_some_and(|directory| output_path.starts_with(directory))
        {
            return Err(ValidationError::new(format!(
                "release summary output must be outside the live cell and evidence directories: {}",
                output_path.display()
            )));
        }
        Ok(())
    }

    pub fn acquire_exclusive_for_cell_path(
        context: &ValidationContext,
        cell_path: &Path,
    ) -> ValidationResult<Self> {
        let layout = cell_result_root_layout(context, cell_path)?;
        let lease = Self::acquire(
            context,
            &layout.result_root,
            &layout.output_directory,
            layout.evidence_directory.as_deref(),
            ResultRootLeaseMode::Exclusive,
            true,
        )?;
        lease.validate_attached(context)?;
        context.validate_new_output(cell_path)?;
        Ok(lease)
    }

    pub fn acquire_exclusive_for_auxiliary_path(
        context: &ValidationContext,
        output_path: &Path,
    ) -> ValidationResult<Self> {
        let layout = auxiliary_result_root_layout(context, output_path)?;
        let lease = Self::acquire(
            context,
            &layout.result_root,
            &layout.output_directory,
            None,
            ResultRootLeaseMode::Exclusive,
            true,
        )?;
        lease.validate_attached(context)?;
        context.validate_new_output(output_path)?;
        Ok(lease)
    }

    pub fn acquire_shared_for_cell_directory(
        context: &ValidationContext,
        cell_directory: &Path,
    ) -> ValidationResult<Self> {
        let layout = cell_directory_result_root_layout(context, cell_directory)?;
        let lease = Self::acquire(
            context,
            &layout.result_root,
            &layout.output_directory,
            layout.evidence_directory.as_deref(),
            ResultRootLeaseMode::Shared,
            false,
        )?;
        lease.validate_attached(context)?;
        Ok(lease)
    }

    fn acquire(
        context: &ValidationContext,
        result_root: &Path,
        output_directory: &Path,
        evidence_directory: Option<&Path>,
        mode: ResultRootLeaseMode,
        create_lock: bool,
    ) -> ValidationResult<Self> {
        let mut retained_paths = Vec::with_capacity(4);
        retained_paths.push(retain_result_path(context, result_root, true)?);
        retained_paths.push(retain_result_path(context, output_directory, true)?);
        if let Some(evidence_directory) = evidence_directory {
            retained_paths.push(retain_result_path(context, evidence_directory, true)?);
        }

        let lock_path = result_root.join(RELEASE_RESULT_ROOT_LOCK_NAME);
        let (mut lock, lock_created) =
            open_result_root_lock(context, &retained_paths[0], &lock_path, create_lock, mode)?;
        validate_opened_result_path(&lock, &lock_path, false)?;
        let lock_identity = filesystem_identity(&lock)?;
        acquire_file_lease(&lock, mode).map_err(|error| {
            ValidationError::new(format!(
                "cannot acquire {mode:?} result-root lease {}: {error}",
                lock_path.display()
            ))
        })?;
        if lock_created {
            require_new_result_root_inputs_empty(
                context,
                result_root,
                output_directory,
                evidence_directory,
            )?;
        }
        require_clean_result_root_state(&mut lock, lock_created)?;
        if lock_created {
            sync_retained_directory(&retained_paths[0])?;
        }
        if mode == ResultRootLeaseMode::Exclusive {
            if let Some(evidence_directory) = evidence_directory {
                validate_append_only_result_root_consistency(
                    context,
                    output_directory,
                    evidence_directory,
                )?;
            }
        }

        let lease = Self {
            _lock: lock,
            mode,
            lock_path,
            lock_identity,
            result_root: result_root.to_path_buf(),
            output_directory: output_directory.to_path_buf(),
            evidence_directory: evidence_directory.map(Path::to_path_buf),
            retained_paths,
            publication_attempt_active: false,
        };
        lease.validate_attached(context)?;
        Ok(lease)
    }

    pub fn result_root(&self) -> &Path {
        &self.result_root
    }

    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    pub fn evidence_directory(&self) -> Option<&Path> {
        self.evidence_directory.as_deref()
    }

    pub fn begin_publication_attempt(&mut self) -> ValidationResult<()> {
        if self.mode != ResultRootLeaseMode::Exclusive {
            return Err(ValidationError::new(
                "a shared result-root lease cannot begin a publication attempt",
            ));
        }
        if self.publication_attempt_active {
            return Err(ValidationError::new(
                "result-root publication attempt is already active",
            ));
        }
        require_exact_result_root_state(&mut self._lock, RELEASE_RESULT_ROOT_CLEAN_STATE)?;
        write_result_root_state(&mut self._lock, RELEASE_RESULT_ROOT_ACTIVE_STATE)?;
        self.publication_attempt_active = true;
        Ok(())
    }

    pub fn complete_publication_attempt(&mut self) -> ValidationResult<()> {
        if self.mode != ResultRootLeaseMode::Exclusive {
            return Err(ValidationError::new(
                "a shared result-root lease cannot complete a publication attempt",
            ));
        }
        if !self.publication_attempt_active {
            return Ok(());
        }
        require_exact_result_root_state(&mut self._lock, RELEASE_RESULT_ROOT_ACTIVE_STATE)?;
        // Callers synchronize final artifacts and owning directories first. The complete
        // exact clean record is the externally observable commit marker. A write or
        // sync error can be indeterminate after those bytes become visible, so callers
        // still report the error and abandon the run; later readers accept only exact
        // clean state plus independent cell/evidence validation.
        write_result_root_state(&mut self._lock, RELEASE_RESULT_ROOT_CLEAN_STATE)?;
        self.publication_attempt_active = false;
        Ok(())
    }

    pub fn validate_attached(&self, context: &ValidationContext) -> ValidationResult<()> {
        for retained in &self.retained_paths {
            if retained.directory {
                context.validate_existing_directory(&retained.path)?;
            } else {
                context.validate_existing_file(&retained.path)?;
            }
            let current = open_retained_result_path(&retained.path, retained.directory)?;
            validate_opened_result_path(&current, &retained.path, retained.directory)?;
            if filesystem_identity(&current)? != retained.identity {
                return Err(ValidationError::new(format!(
                    "retained result-root path was replaced: {}",
                    retained.path.display()
                )));
            }
        }
        context.validate_existing_file(&self.lock_path)?;
        let current_lock = open_retained_result_path(&self.lock_path, false)?;
        validate_opened_result_path(&current_lock, &self.lock_path, false)?;
        if filesystem_identity(&current_lock)? != self.lock_identity {
            return Err(ValidationError::new(format!(
                "result-root lease entry was replaced: {}",
                self.lock_path.display()
            )));
        }
        Ok(())
    }
}

fn require_new_result_root_inputs_empty(
    context: &ValidationContext,
    result_root: &Path,
    output_directory: &Path,
    evidence_directory: Option<&Path>,
) -> ValidationResult<()> {
    let mut directories = vec![output_directory.to_path_buf()];
    if let Some(evidence_directory) = evidence_directory {
        directories.push(evidence_directory.to_path_buf());
    } else {
        let cells = result_root.join("cells");
        let evidence = result_root.join("evidence");
        if cells.is_dir() && evidence.is_dir() {
            directories.push(cells);
            directories.push(evidence);
        }
    }
    directories.sort();
    directories.dedup();
    for directory in directories {
        context.validate_existing_directory(&directory)?;
        if fs::read_dir(&directory)?.next().is_some() {
            return Err(ValidationError::new(format!(
                "a new result-root lease cannot adopt pre-existing live result entries: {}",
                directory.display()
            )));
        }
    }
    Ok(())
}

fn cell_result_root_layout(
    context: &ValidationContext,
    cell_path: &Path,
) -> ValidationResult<ResultRootLayout> {
    context.validate_new_output(cell_path)?;
    let cell_directory = cell_path.parent().ok_or_else(|| {
        ValidationError::new(format!(
            "release cell path has no parent: {}",
            cell_path.display()
        ))
    })?;
    cell_directory_result_root_layout(context, cell_directory)
}

fn cell_directory_result_root_layout(
    context: &ValidationContext,
    cell_directory: &Path,
) -> ValidationResult<ResultRootLayout> {
    require_directory_name(cell_directory, "cells", "release cell directory")?;
    let result_root = cell_directory.parent().ok_or_else(|| {
        ValidationError::new(format!(
            "release cell directory has no result root: {}",
            cell_directory.display()
        ))
    })?;
    let evidence_directory = result_root.join("evidence");
    context.validate_existing_directory(result_root)?;
    context.validate_existing_directory(cell_directory)?;
    context.validate_existing_directory(&evidence_directory)?;
    Ok(ResultRootLayout {
        result_root: result_root.to_path_buf(),
        output_directory: cell_directory.to_path_buf(),
        evidence_directory: Some(evidence_directory),
    })
}

fn auxiliary_result_root_layout(
    context: &ValidationContext,
    output_path: &Path,
) -> ValidationResult<ResultRootLayout> {
    context.validate_new_output(output_path)?;
    let auxiliary_directory = output_path.parent().ok_or_else(|| {
        ValidationError::new(format!(
            "auxiliary live result has no parent: {}",
            output_path.display()
        ))
    })?;
    require_directory_name(
        auxiliary_directory,
        "auxiliary",
        "auxiliary live-result directory",
    )?;
    let result_root = auxiliary_directory.parent().ok_or_else(|| {
        ValidationError::new(format!(
            "auxiliary live-result directory has no result root: {}",
            auxiliary_directory.display()
        ))
    })?;
    context.validate_existing_directory(result_root)?;
    context.validate_existing_directory(auxiliary_directory)?;
    Ok(ResultRootLayout {
        result_root: result_root.to_path_buf(),
        output_directory: auxiliary_directory.to_path_buf(),
        evidence_directory: None,
    })
}

fn require_clean_result_root_state(
    file: &mut File,
    initialize_empty: bool,
) -> ValidationResult<()> {
    let state = read_result_root_state(file)?;
    if state.is_empty() && initialize_empty {
        write_result_root_state(file, RELEASE_RESULT_ROOT_CLEAN_STATE)?;
        return Ok(());
    }
    if state == RELEASE_RESULT_ROOT_CLEAN_STATE {
        return Ok(());
    }
    if state == RELEASE_RESULT_ROOT_ACTIVE_STATE {
        return Err(ValidationError::new(
            "result root contains an incomplete prior publication attempt and must be abandoned",
        ));
    }
    Err(ValidationError::new(
        "result-root publication state is missing or malformed",
    ))
}

fn require_exact_result_root_state(file: &mut File, expected: &[u8]) -> ValidationResult<()> {
    if read_result_root_state(file)? == expected {
        Ok(())
    } else {
        Err(ValidationError::new(
            "result-root publication state changed while its lease was held",
        ))
    }
}

fn read_result_root_state(file: &mut File) -> ValidationResult<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    Read::by_ref(file)
        .take(MAX_RELEASE_RESULT_ROOT_STATE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_RELEASE_RESULT_ROOT_STATE_BYTES {
        return Err(ValidationError::new(
            "result-root publication state exceeds its byte bound",
        ));
    }
    Ok(bytes)
}

fn write_result_root_state(file: &mut File, state: &[u8]) -> ValidationResult<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(state)?;
    file.sync_all()?;
    Ok(())
}

fn validate_append_only_result_root_consistency(
    context: &ValidationContext,
    cell_directory: &Path,
    evidence_directory: &Path,
) -> ValidationResult<()> {
    use crate::evaluation::validate_cell_shape;
    use crate::schema::Cell;

    let mut cell_paths = fs::read_dir(cell_directory)?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(ValidationError::from)
        })
        .collect::<ValidationResult<Vec<_>>>()?;
    cell_paths.sort();
    if cell_paths.len() >= 12 {
        return Err(ValidationError::new(
            "result root already contains a complete or oversized cell set",
        ));
    }

    let mut referenced_evidence = BTreeSet::new();
    for cell_path in cell_paths {
        if cell_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            return Err(ValidationError::new(format!(
                "result root contains a non-final cell entry and must be abandoned: {}",
                cell_path.display()
            )));
        }
        let cell: Cell = read_strict_json(context, &cell_path, MAX_CELL_JSON_BYTES)?;
        validate_cell_shape(&cell)?;
        match (
            cell.evidence_artifact_path.as_ref(),
            cell.evidence_artifact_sha256.as_ref(),
        ) {
            (Some(path), Some(expected_sha256)) => {
                let path = PathBuf::from(path);
                if path.parent() != Some(evidence_directory) {
                    return Err(ValidationError::new(format!(
                        "maintained live result cell references evidence outside its result root: {}",
                        path.display()
                    )));
                }
                let observed_sha256 =
                    sha256_external_file(context, &path, Some(MAX_EVIDENCE_BYTES))?;
                if &observed_sha256 != expected_sha256 {
                    return Err(ValidationError::new(format!(
                        "committed live result evidence digest mismatch: {}",
                        path.display()
                    )));
                }
                if !referenced_evidence.insert(path.clone()) {
                    return Err(ValidationError::new(format!(
                        "multiple committed cells reference the same evidence entry: {}",
                        path.display()
                    )));
                }
            }
            (None, None) => {}
            _ => {
                return Err(ValidationError::new(format!(
                    "committed live result cell has an incomplete evidence reference: {}",
                    cell_path.display()
                )))
            }
        }
    }

    let mut evidence_paths = fs::read_dir(evidence_directory)?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(ValidationError::from)
        })
        .collect::<ValidationResult<Vec<_>>>()?;
    evidence_paths.sort();
    let observed_evidence = evidence_paths.into_iter().collect::<BTreeSet<_>>();
    if observed_evidence != referenced_evidence {
        return Err(ValidationError::new(
            "result root contains missing, staged, or orphan evidence and must be abandoned",
        ));
    }
    Ok(())
}

fn require_directory_name(path: &Path, expected: &str, label: &str) -> ValidationResult<()> {
    if path.file_name() != Some(OsStr::new(expected)) {
        return Err(ValidationError::new(format!(
            "{label} must be named `{expected}`: {}",
            path.display()
        )));
    }
    Ok(())
}

fn retain_result_path(
    context: &ValidationContext,
    path: &Path,
    directory: bool,
) -> ValidationResult<RetainedResultPath> {
    if directory {
        context.validate_existing_directory(path)?;
    } else {
        context.validate_existing_file(path)?;
    }
    let file = open_retained_result_path(path, directory)?;
    validate_opened_result_path(&file, path, directory)?;
    let retained = RetainedResultPath {
        identity: filesystem_identity(&file)?,
        file,
        path: path.to_path_buf(),
        directory,
    };
    let current = open_retained_result_path(path, directory)?;
    if filesystem_identity(&current)? != retained.identity {
        return Err(ValidationError::new(format!(
            "result-root path changed while it was being pinned: {}",
            path.display()
        )));
    }
    Ok(retained)
}

fn open_result_root_lock(
    context: &ValidationContext,
    result_root: &RetainedResultPath,
    lock_path: &Path,
    create: bool,
    mode: ResultRootLeaseMode,
) -> ValidationResult<(File, bool)> {
    let mut created = false;
    if create {
        match fs::symlink_metadata(lock_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                context.validate_new_output(lock_path)?;
                match open_result_root_lock_create_new(&result_root.file, lock_path) {
                    Ok(file) => {
                        created = true;
                        return finish_open_result_root_lock(context, lock_path, file, created);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(_) => {}
            Err(error) => return Err(error.into()),
        }
    }
    context.validate_existing_file(lock_path)?;
    let file = open_result_root_lock_existing(
        &result_root.file,
        lock_path,
        mode == ResultRootLeaseMode::Exclusive,
    )?;
    finish_open_result_root_lock(context, lock_path, file, created)
}

fn finish_open_result_root_lock(
    context: &ValidationContext,
    lock_path: &Path,
    file: File,
    created: bool,
) -> ValidationResult<(File, bool)> {
    context.validate_existing_file(lock_path)?;
    validate_opened_result_path(&file, lock_path, false)?;
    let current = open_retained_result_path(lock_path, false)?;
    if filesystem_identity(&current)? != filesystem_identity(&file)? {
        return Err(ValidationError::new(format!(
            "result-root lease entry changed while it was being opened: {}",
            lock_path.display()
        )));
    }
    Ok((file, created))
}

#[cfg(unix)]
fn open_retained_result_path(path: &Path, directory: bool) -> std::io::Result<File> {
    use rustix::fs::{open, Mode, OFlags};

    let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW;
    if directory {
        flags |= OFlags::DIRECTORY;
    }
    open(path, flags, Mode::empty())
        .map(File::from)
        .map_err(std::io::Error::from)
}

#[cfg(windows)]
fn open_retained_result_path(path: &Path, directory: bool) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    options.open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_retained_result_path(path: &Path, _directory: bool) -> std::io::Result<File> {
    File::open(path)
}

fn validate_opened_result_path(file: &File, path: &Path, directory: bool) -> ValidationResult<()> {
    let metadata = file.metadata()?;
    if metadata.is_dir() != directory || metadata.is_file() == directory {
        return Err(ValidationError::new(format!(
            "result-root path has an invalid type: {}",
            path.display()
        )));
    }
    #[cfg(windows)]
    if filesystem_identity(file)?.attributes
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
    {
        return Err(ValidationError::new(format!(
            "result-root path must not be a reparse point: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn filesystem_identity(file: &File) -> ValidationResult<FilesystemIdentity> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata()?;

    Ok(FilesystemIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn filesystem_identity(file: &File) -> ValidationResult<FilesystemIdentity> {
    use std::{mem::zeroed, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    Ok(FilesystemIdentity {
        volume_serial_number: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        attributes: information.dwFileAttributes,
    })
}

#[cfg(not(any(unix, windows)))]
fn filesystem_identity(_file: &File) -> ValidationResult<FilesystemIdentity> {
    Ok(FilesystemIdentity)
}

#[cfg(unix)]
fn open_result_root_lock_create_new(
    result_root: &File,
    _lock_path: &Path,
) -> std::io::Result<File> {
    use rustix::fs::{openat, Mode, OFlags};

    openat(
        result_root,
        RELEASE_RESULT_ROOT_LOCK_NAME,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::RUSR | Mode::WUSR,
    )
    .map(File::from)
    .map_err(std::io::Error::from)
}

#[cfg(unix)]
fn open_result_root_lock_existing(
    result_root: &File,
    _lock_path: &Path,
    writable: bool,
) -> std::io::Result<File> {
    use rustix::fs::{openat, Mode, OFlags};

    let access = if writable {
        OFlags::RDWR
    } else {
        OFlags::RDONLY
    };
    openat(
        result_root,
        RELEASE_RESULT_ROOT_LOCK_NAME,
        access | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(std::io::Error::from)
}

#[cfg(windows)]
fn open_result_root_lock_create_new(
    _result_root: &File,
    lock_path: &Path,
) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(lock_path)
}

#[cfg(windows)]
fn open_result_root_lock_existing(
    _result_root: &File,
    lock_path: &Path,
    writable: bool,
) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(writable)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(lock_path)
}

#[cfg(not(any(unix, windows)))]
fn open_result_root_lock_create_new(
    _result_root: &File,
    lock_path: &Path,
) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(lock_path)
}

#[cfg(not(any(unix, windows)))]
fn open_result_root_lock_existing(
    _result_root: &File,
    lock_path: &Path,
    writable: bool,
) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(writable)
        .open(lock_path)
}

#[cfg(unix)]
fn sync_retained_directory(directory: &RetainedResultPath) -> ValidationResult<()> {
    directory.file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_retained_directory(_directory: &RetainedResultPath) -> ValidationResult<()> {
    Ok(())
}

#[cfg(unix)]
fn acquire_file_lease(file: &File, mode: ResultRootLeaseMode) -> std::io::Result<()> {
    use rustix::fs::{flock, FlockOperation};

    let operation = match mode {
        ResultRootLeaseMode::Exclusive => FlockOperation::NonBlockingLockExclusive,
        ResultRootLeaseMode::Shared => FlockOperation::NonBlockingLockShared,
    };
    let deadline = Instant::now() + RESULT_ROOT_LEASE_RETRY_TIMEOUT;
    loop {
        match flock(file, operation) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let error = std::io::Error::from(error);
                if error.kind() != std::io::ErrorKind::WouldBlock || Instant::now() >= deadline {
                    return Err(error);
                }
            }
        }
        thread::sleep(RESULT_ROOT_LEASE_RETRY_INTERVAL);
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn acquire_file_lease(file: &File, mode: ResultRootLeaseMode) -> std::io::Result<()> {
    use std::{mem::zeroed, os::windows::io::AsRawHandle};
    use windows_sys::Win32::{
        Storage::FileSystem::{LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY},
        System::IO::OVERLAPPED,
    };

    let mut overlapped: OVERLAPPED = unsafe { zeroed() };
    let mut flags = LOCKFILE_FAIL_IMMEDIATELY;
    if mode == ResultRootLeaseMode::Exclusive {
        flags |= LOCKFILE_EXCLUSIVE_LOCK;
    }
    let result = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            flags,
            0,
            u32::MAX,
            u32::MAX,
            &mut overlapped,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn acquire_file_lease(_file: &File, _mode: ResultRootLeaseMode) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "result-root leases are unsupported on this platform",
    ))
}

impl ValidationContext {
    pub fn new(
        source_checkout: PathBuf,
        target_directory: PathBuf,
        docs_directory: PathBuf,
        runtime_homes: Vec<PathBuf>,
    ) -> ValidationResult<Self> {
        let source_checkout = fs::canonicalize(&source_checkout).map_err(|error| {
            ValidationError::new(format!(
                "cannot canonicalize source checkout {}: {error}",
                source_checkout.display()
            ))
        })?;
        if !source_checkout.is_dir() {
            return Err(ValidationError::new(format!(
                "source checkout is not a directory: {}",
                source_checkout.display()
            )));
        }
        let target_directory = normalize_configured_root(&target_directory, &source_checkout)?;
        let docs_directory = normalize_configured_root(&docs_directory, &source_checkout)?;
        let mut runtime_homes = runtime_homes
            .into_iter()
            .map(|path| normalize_configured_root(&path, &source_checkout))
            .collect::<ValidationResult<Vec<_>>>()?;
        runtime_homes.sort();
        runtime_homes.dedup();
        Ok(Self {
            source_checkout,
            target_directory,
            docs_directory,
            runtime_homes,
        })
    }

    pub fn from_process(current_dir: &Path) -> ValidationResult<Self> {
        Self::from_process_environment(
            current_dir,
            env::var_os("VOLICORD_HOME"),
            env::var_os("HOME"),
            env::var_os("USERPROFILE"),
        )
    }

    pub(crate) fn from_process_environment(
        current_dir: &Path,
        volicord_home: Option<OsString>,
        home: Option<OsString>,
        user_profile: Option<OsString>,
    ) -> ValidationResult<Self> {
        let source_checkout = git_toplevel(current_dir)?;
        let target_directory = cargo_target_directory(&source_checkout)?;
        let docs_directory = source_checkout.join("docs");
        let mut runtime_homes = Vec::new();
        if let Some(value) = volicord_home.filter(|value| !value.is_empty()) {
            let path = PathBuf::from(value);
            runtime_homes.push(if path.is_absolute() {
                path
            } else {
                current_dir.join(path)
            });
        }
        if let Some(home) = home
            .filter(|value| !value.is_empty())
            .or_else(|| user_profile.filter(|value| !value.is_empty()))
        {
            let home = PathBuf::from(home);
            let home = if home.is_absolute() {
                home
            } else {
                current_dir.join(home)
            };
            runtime_homes.push(home.join(".volicord"));
        }
        Self::new(
            source_checkout,
            target_directory,
            docs_directory,
            runtime_homes,
        )
    }

    pub fn source_checkout(&self) -> &Path {
        &self.source_checkout
    }

    pub fn target_directory(&self) -> &Path {
        &self.target_directory
    }

    pub fn add_runtime_home(&mut self, runtime_home: &Path) -> ValidationResult<()> {
        if !runtime_home.is_absolute() {
            return Err(ValidationError::new(format!(
                "observed Volicord Runtime Home exclusion must be absolute: {}",
                runtime_home.display()
            )));
        }
        let runtime_home = normalize_configured_root(runtime_home, &self.source_checkout)?;
        self.runtime_homes.push(runtime_home);
        self.runtime_homes.sort();
        self.runtime_homes.dedup();
        Ok(())
    }

    pub fn validate_existing_file(&self, path: &Path) -> ValidationResult<()> {
        validate_absolute_normalized(path)?;
        self.validate_external(path, false)?;
        ensure_no_symlink_components(path)?;
        let canonical = fs::canonicalize(path).map_err(|error| {
            ValidationError::new(format!("cannot canonicalize {}: {error}", path.display()))
        })?;
        if canonical.as_os_str() != path.as_os_str() {
            return Err(ValidationError::new(format!(
                "path is not canonical and symlink-free: {}",
                path.display()
            )));
        }
        Ok(())
    }

    pub fn validate_existing_directory(&self, path: &Path) -> ValidationResult<()> {
        self.validate_existing_file(path)?;
        let metadata = fs::metadata(path)?;
        if !metadata.is_dir() {
            return Err(ValidationError::new(format!(
                "path is not a directory: {}",
                path.display()
            )));
        }
        self.validate_external(path, true)
    }

    pub fn validate_new_output(&self, path: &Path) -> ValidationResult<()> {
        self.validate_new_path(path, false, "output")
    }

    pub fn validate_new_directory(&self, path: &Path) -> ValidationResult<()> {
        self.validate_new_path(path, true, "directory")
    }

    fn validate_new_path(&self, path: &Path, directory: bool, role: &str) -> ValidationResult<()> {
        validate_absolute_normalized(path)?;
        self.validate_external(path, directory)?;
        let parent = path.parent().ok_or_else(|| {
            ValidationError::new(format!("{role} has no parent: {}", path.display()))
        })?;
        validate_absolute_normalized(parent)?;
        self.validate_external(parent, true)?;
        ensure_no_symlink_components(parent)?;
        let canonical_parent = fs::canonicalize(parent).map_err(|error| {
            ValidationError::new(format!(
                "cannot canonicalize output parent {}: {error}",
                parent.display()
            ))
        })?;
        if canonical_parent.as_os_str() != parent.as_os_str() {
            return Err(ValidationError::new(format!(
                "output parent is not canonical and symlink-free: {}",
                parent.display()
            )));
        }
        if !fs::metadata(parent)?.is_dir() {
            return Err(ValidationError::new(format!(
                "{role} parent is not a directory: {}",
                parent.display()
            )));
        }
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(ValidationError::new(format!(
                "{role} already exists: {}",
                path.display()
            ))),
            Err(error) => Err(ValidationError::new(format!(
                "cannot inspect {role} {}: {error}",
                path.display()
            ))),
        }
    }

    fn validate_external(&self, path: &Path, directory: bool) -> ValidationResult<()> {
        for (label, excluded) in [
            ("source checkout", &self.source_checkout),
            ("Cargo target directory", &self.target_directory),
            ("maintained documentation", &self.docs_directory),
        ] {
            if path.starts_with(excluded) || (directory && excluded.starts_with(path)) {
                return Err(ValidationError::new(format!(
                    "path {} overlaps {label} {}",
                    path.display(),
                    excluded.display()
                )));
            }
        }
        for runtime_home in &self.runtime_homes {
            if path.starts_with(runtime_home) || (directory && runtime_home.starts_with(path)) {
                return Err(ValidationError::new(format!(
                    "path {} overlaps Volicord Runtime Home {}",
                    path.display(),
                    runtime_home.display()
                )));
            }
        }
        for ancestor in path.ancestors() {
            let registry = ancestor.join("registry.sqlite");
            if registry.is_file() {
                return Err(ValidationError::new(format!(
                    "path {} is inside a directory containing a Volicord registry",
                    path.display()
                )));
            }
        }
        Ok(())
    }
}

pub fn read_strict_json<T: DeserializeOwned>(
    context: &ValidationContext,
    path: &Path,
    max_bytes: u64,
) -> ValidationResult<T> {
    let bytes = read_bounded_external_file(context, path, max_bytes)?;
    parse_strict_json(&bytes)
}

pub fn parse_strict_json<T: DeserializeOwned>(bytes: &[u8]) -> ValidationResult<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let unique = UniqueJsonValue::deserialize(&mut deserializer)
        .map_err(|error| ValidationError::new(format!("invalid strict JSON: {error}")))?;
    deserializer
        .end()
        .map_err(|error| ValidationError::new(format!("trailing JSON data: {error}")))?;
    serde_json::from_value(unique.0)
        .map_err(|error| ValidationError::new(format!("JSON schema mismatch: {error}")))
}

pub fn read_bounded_external_file(
    context: &ValidationContext,
    path: &Path,
    max_bytes: u64,
) -> ValidationResult<Vec<u8>> {
    context.validate_existing_file(path)?;
    let (mut file, metadata) = open_regular_file(path)?;
    if metadata.len() > max_bytes {
        return Err(ValidationError::new(format!(
            "file exceeds {max_bytes} byte bound: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(ValidationError::new(format!(
            "file grew beyond {max_bytes} byte bound: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

pub fn sha256_external_file(
    context: &ValidationContext,
    path: &Path,
    max_bytes: Option<u64>,
) -> ValidationResult<String> {
    context.validate_existing_file(path)?;
    let (mut file, metadata) = open_regular_file(path)?;
    if max_bytes.is_some_and(|bound| metadata.len() > bound) {
        return Err(ValidationError::new(format!(
            "file exceeds {} byte bound: {}",
            max_bytes.expect("bound was checked"),
            path.display()
        )));
    }
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| ValidationError::new("file length overflow while hashing"))?;
        if max_bytes.is_some_and(|bound| total > bound) {
            return Err(ValidationError::new(format!(
                "file grew beyond {} byte bound: {}",
                max_bytes.expect("bound was checked"),
                path.display()
            )));
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

pub fn inspect_candidate_artifact(
    context: &ValidationContext,
    candidate_path: &Path,
    expected_sha256: &str,
) -> ValidationResult<CandidateArtifactInspection> {
    context.validate_existing_file(candidate_path)?;
    let (mut held_candidate, initial_metadata) = open_regular_file(candidate_path)?;
    let sha256_before = sha256_file_handle(&mut held_candidate)?;
    if sha256_before != expected_sha256 {
        return Err(ValidationError::new(format!(
            "candidate digest differs from the descriptor before execution: {}",
            candidate_path.display()
        )));
    }

    held_candidate.seek(SeekFrom::Start(0))?;
    let private_directory = tempfile::Builder::new()
        .prefix("volicord-release-candidate-")
        .tempdir()
        .map_err(|error| {
            ValidationError::new(format!(
                "cannot create private candidate directory: {error}"
            ))
        })?;
    let private_candidate_path = private_directory.path().join("candidate");
    let mut private_candidate = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&private_candidate_path)
        .map_err(|error| {
            ValidationError::new(format!("cannot create private candidate copy: {error}"))
        })?;
    let private_copy_sha256 = copy_and_hash(&mut held_candidate, &mut private_candidate)?;
    if private_copy_sha256 != expected_sha256 {
        return Err(ValidationError::new(
            "candidate changed while copying from the held file handle",
        ));
    }
    private_candidate.sync_all()?;
    make_private_copy_executable(&private_candidate)?;
    drop(private_candidate);

    let mut command = Command::new(&private_candidate_path);
    command
        .arg("--version")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = spawn_private_candidate(&mut command, candidate_path)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ValidationError::new("candidate version stdout is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ValidationError::new("candidate version stderr is unavailable"))?;
    let stdout_reader = thread::spawn(move || read_pipe_bounded(stdout, MAX_VERSION_OUTPUT_BYTES));
    let stderr_reader = thread::spawn(move || read_pipe_bounded(stderr, MAX_VERSION_OUTPUT_BYTES));
    let deadline = Instant::now() + CANDIDATE_VERSION_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ValidationError::new(
                "candidate --version exceeded the 10 second bound",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| ValidationError::new("candidate stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| ValidationError::new("candidate stderr reader panicked"))??;
    if !status.success() {
        return Err(ValidationError::new(format!(
            "candidate --version failed with status {status}"
        )));
    }
    if !stderr.is_empty() {
        return Err(ValidationError::new(
            "candidate --version must not write stderr",
        ));
    }
    let stdout = std::str::from_utf8(&stdout)
        .map_err(|_| ValidationError::new("candidate --version output is not UTF-8"))?;
    let build = parse_candidate_version(stdout)?;
    let sha256_after_held = sha256_file_handle(&mut held_candidate)?;
    let (sha256_after_path, path_identity_stable) =
        inspect_final_candidate_path(context, candidate_path, &initial_metadata);
    Ok(CandidateArtifactInspection {
        sha256_before,
        private_copy_sha256,
        sha256_after_held,
        sha256_after_path,
        path_identity_stable,
        build,
    })
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(hasher.finalize().as_slice())
}

pub fn write_json_create_new<T: Serialize>(
    context: &ValidationContext,
    path: &Path,
    value: &T,
    max_bytes: u64,
) -> ValidationResult<()> {
    context.validate_new_output(path)?;
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > max_bytes {
        return Err(ValidationError::new(format!(
            "serialized output exceeds {max_bytes} byte bound"
        )));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            ValidationError::new(format!(
                "cannot create new output {}: {error}",
                path.display()
            ))
        })?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn git_head(source_checkout: &Path) -> ValidationResult<String> {
    run_git_text(source_checkout, &["rev-parse", "HEAD"], 256)
}

pub fn git_is_clean(source_checkout: &Path) -> ValidationResult<bool> {
    Ok(run_git_text(
        source_checkout,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        1024 * 1024,
    )?
    .is_empty())
}

pub fn git_archive_sha256(
    source_checkout: &Path,
    source_revision: &str,
) -> ValidationResult<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(source_checkout)
        .args(["archive", "--format=tar", source_revision])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| ValidationError::new(format!("cannot start git archive: {error}")))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| ValidationError::new("git archive stdout is unavailable"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = stdout.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(ValidationError::new(format!(
            "git archive failed with status {status}"
        )));
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

fn cargo_target_directory(source_checkout: &Path) -> ValidationResult<PathBuf> {
    if let Some(value) = env::var_os("CARGO_TARGET_DIR").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(value);
        return Ok(if path.is_absolute() {
            path
        } else {
            source_checkout.join(path)
        });
    }
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version=1")
        .current_dir(source_checkout)
        .output()
        .map_err(|error| ValidationError::new(format!("cannot run cargo metadata: {error}")))?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "cargo metadata failed with status {}",
            output.status
        )));
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let path = value["target_directory"]
        .as_str()
        .ok_or_else(|| ValidationError::new("cargo metadata omitted target_directory"))?;
    Ok(PathBuf::from(path))
}

fn git_toplevel(current_dir: &Path) -> ValidationResult<PathBuf> {
    let root = run_git_text(current_dir, &["rev-parse", "--show-toplevel"], 16 * 1024)?;
    let path = PathBuf::from(root);
    fs::canonicalize(&path).map_err(|error| {
        ValidationError::new(format!(
            "cannot canonicalize source checkout {}: {error}",
            path.display()
        ))
    })
}

fn run_git_text(root: &Path, args: &[&str], max_bytes: usize) -> ValidationResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| ValidationError::new(format!("cannot run git: {error}")))?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "git command failed with status {}",
            output.status
        )));
    }
    if output.stdout.len() > max_bytes {
        return Err(ValidationError::new("git command output exceeds bound"));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| ValidationError::new("git command output is not UTF-8"))?;
    Ok(text.trim_end_matches(['\r', '\n']).to_owned())
}

fn validate_absolute_normalized(path: &Path) -> ValidationResult<()> {
    if path.to_str().is_none() {
        return Err(ValidationError::new(format!(
            "release-evidence path is not valid UTF-8: {}",
            path.display()
        )));
    }
    if !path.is_absolute() {
        return Err(ValidationError::new(format!(
            "path must be absolute: {}",
            path.display()
        )));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ValidationError::new(format!(
            "path must not contain dot components: {}",
            path.display()
        )));
    }
    let normalized = path.components().collect::<PathBuf>();
    if normalized.as_os_str() != path.as_os_str() {
        return Err(ValidationError::new(format!(
            "path must be lexically normalized: {}",
            path.display()
        )));
    }
    Ok(())
}

fn normalize_configured_root(path: &Path, relative_base: &Path) -> ValidationResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        relative_base.join(path)
    };
    let resolved = canonicalize_existing_prefix(&absolute)?;
    lexical_normalize_absolute(&resolved)
}

fn lexical_normalize_absolute(path: &Path) -> ValidationResult<PathBuf> {
    if !path.is_absolute() {
        return Err(ValidationError::new(format!(
            "configured exclusion root must resolve to an absolute path: {}",
            path.display()
        )));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(ValidationError::new(format!(
            "configured exclusion root did not remain absolute: {}",
            path.display()
        )));
    }
    Ok(normalized)
}

fn canonicalize_existing_prefix(path: &Path) -> ValidationResult<PathBuf> {
    let mut existing = path.to_path_buf();
    let mut suffix = Vec::<OsString>::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing.file_name().ok_or_else(|| {
                    ValidationError::new(format!(
                        "cannot find an existing prefix for configured exclusion root {}",
                        path.display()
                    ))
                })?;
                suffix.push(name.to_os_string());
                existing = existing
                    .parent()
                    .ok_or_else(|| {
                        ValidationError::new(format!(
                            "configured exclusion root has no parent: {}",
                            path.display()
                        ))
                    })?
                    .to_path_buf();
            }
            Err(error) => {
                return Err(ValidationError::new(format!(
                    "cannot inspect configured exclusion root {}: {error}",
                    path.display()
                )))
            }
        }
    }
    let mut canonical = fs::canonicalize(&existing).map_err(|error| {
        ValidationError::new(format!(
            "cannot canonicalize configured exclusion prefix {}: {error}",
            existing.display()
        ))
    })?;
    for component in suffix.into_iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

fn ensure_no_symlink_components(path: &Path) -> ValidationResult<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            ValidationError::new(format!(
                "cannot inspect path component {}: {error}",
                current.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(ValidationError::new(format!(
                "symbolic links are not allowed in release-evidence paths: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

fn open_regular_file(path: &Path) -> ValidationResult<(File, fs::Metadata)> {
    let before = fs::symlink_metadata(path)?;
    if before.file_type().is_symlink() || !before.is_file() {
        return Err(ValidationError::new(format!(
            "input is not a regular non-symlink file: {}",
            path.display()
        )));
    }
    let file = File::open(path)?;
    let after = file.metadata()?;
    if !after.is_file() || !same_file(&before, &after) {
        return Err(ValidationError::new(format!(
            "input changed while opening: {}",
            path.display()
        )));
    }
    Ok((file, after))
}

fn sha256_file_handle(file: &mut File) -> ValidationResult<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

fn copy_and_hash(source: &mut File, destination: &mut File) -> ValidationResult<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        destination.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
    }
    destination.flush()?;
    Ok(hex_digest(hasher.finalize().as_slice()))
}

#[cfg(unix)]
fn make_private_copy_executable(file: &File) -> ValidationResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o500);
    file.set_permissions(permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_private_copy_executable(_: &File) -> ValidationResult<()> {
    Ok(())
}

fn inspect_final_candidate_path(
    context: &ValidationContext,
    candidate_path: &Path,
    initial_metadata: &fs::Metadata,
) -> (Option<String>, bool) {
    let inspection = (|| -> ValidationResult<(String, bool)> {
        context.validate_existing_file(candidate_path)?;
        let (mut final_file, final_metadata) = open_regular_file(candidate_path)?;
        let digest = sha256_file_handle(&mut final_file)?;
        Ok((digest, same_file(initial_metadata, &final_metadata)))
    })();
    match inspection {
        Ok((digest, identity_stable)) => (Some(digest), identity_stable),
        Err(_) => (None, false),
    }
}

fn read_pipe_bounded(mut pipe: impl Read, max_bytes: u64) -> ValidationResult<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.by_ref().take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(ValidationError::new(
            "candidate --version output exceeds bound",
        ));
    }
    Ok(bytes)
}

fn spawn_private_candidate(
    command: &mut Command,
    descriptor_path: &Path,
) -> ValidationResult<std::process::Child> {
    for attempt in 0..20 {
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if is_text_file_busy(&error) && attempt < 19 => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => {
                return Err(ValidationError::new(format!(
                    "cannot execute the private candidate copy for {} --version: {error}",
                    descriptor_path.display(),
                )))
            }
        }
    }
    unreachable!("the bounded candidate spawn loop always returns")
}

#[cfg(unix)]
fn is_text_file_busy(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(26)
}

#[cfg(not(unix))]
fn is_text_file_busy(_: &std::io::Error) -> bool {
    false
}

fn parse_candidate_version(output: &str) -> ValidationResult<CandidateBuildIdentity> {
    let line = output
        .strip_suffix('\n')
        .ok_or_else(|| ValidationError::new("candidate --version must end with one LF"))?;
    if line.contains(['\r', '\n']) {
        return Err(ValidationError::new(
            "candidate --version must contain exactly one line",
        ));
    }
    let body = line
        .strip_prefix("volicord ")
        .ok_or_else(|| ValidationError::new("candidate version prefix mismatch"))?;
    let (package_version, build_id) = body
        .split_once(" (build_id=")
        .ok_or_else(|| ValidationError::new("candidate version build_id wrapper mismatch"))?;
    let build_id = build_id
        .strip_suffix(')')
        .ok_or_else(|| ValidationError::new("candidate version closing wrapper mismatch"))?;
    validate_version_component("package_version", package_version)?;
    let mut components = build_id.split(';');
    let embedded_package = components
        .next()
        .ok_or_else(|| ValidationError::new("build_id package version is missing"))?;
    if embedded_package != package_version {
        return Err(ValidationError::new(
            "outer and build_id package versions differ",
        ));
    }
    let names = [
        "git",
        "tree",
        "metadata_source",
        "target",
        "profile",
        "profile_class",
        "profile_exact",
        "opt",
        "debug",
    ];
    let mut values = BTreeMap::new();
    for name in names {
        let component = components
            .next()
            .ok_or_else(|| ValidationError::new(format!("build_id {name} is missing")))?;
        let (actual_name, value) = component
            .split_once('=')
            .ok_or_else(|| ValidationError::new(format!("build_id {name} is malformed")))?;
        if actual_name != name {
            return Err(ValidationError::new(format!(
                "build_id expected {name}, found {actual_name}"
            )));
        }
        validate_version_component(name, value)?;
        values.insert(name, value.to_owned());
    }
    if components.next().is_some() {
        return Err(ValidationError::new(
            "build_id contains an additional component",
        ));
    }
    Ok(CandidateBuildIdentity {
        build_id: build_id.to_owned(),
        package_version: package_version.to_owned(),
        git_commit: values.remove("git").expect("validated component"),
        tree: values.remove("tree").expect("validated component"),
        metadata_source: values
            .remove("metadata_source")
            .expect("validated component"),
        target: values.remove("target").expect("validated component"),
        profile: values.remove("profile").expect("validated component"),
        profile_class: values.remove("profile_class").expect("validated component"),
        profile_exact: values.remove("profile_exact").expect("validated component"),
        opt: values.remove("opt").expect("validated component"),
        debug: values.remove("debug").expect("validated component"),
    })
}

fn validate_version_component(field: &str, value: &str) -> ValidationResult<()> {
    if value.is_empty()
        || value.len() > 512
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b';' | b'(' | b')'))
    {
        return Err(ValidationError::new(format!(
            "candidate build {field} is empty, oversized, or malformed"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
        && left.file_type().is_file() == right.file_type().is_file()
        && left.modified().ok() == right.modified().ok()
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

struct UniqueJsonValue(Value);

impl<'de> Deserialize<'de> for UniqueJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor).map(Self)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        UniqueJsonValue::deserialize(deserializer).map(|value| value.0)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueJsonValue>()? {
            values.push(value.0);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object member: {key}"
                )));
            }
            let value = map.next_value::<UniqueJsonValue>()?;
            values.insert(key, value.0);
        }
        Ok(Value::Object(values.into_iter().collect()))
    }
}
