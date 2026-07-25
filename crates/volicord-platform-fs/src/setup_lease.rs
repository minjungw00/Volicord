use std::{
    fmt, fs,
    fs::{File, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use crate::{
    observe_local_platform_boundary, observe_path_filesystem, PathFilesystemKind,
    PlatformBoundaryError, PlatformEnvironment,
};

const SETUP_LOCK_DOMAIN: &[u8] = b"volicord.runtime-home-setup-lock.v1";
const SETUP_COORDINATION_DIRECTORY: &str = "runtime-home-setup-v1";

/// One exact canonical Runtime Home target used for setup coordination.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalRuntimeHomePath(PathBuf);

impl CanonicalRuntimeHomePath {
    /// Returns the canonical final Runtime Home path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Debug for CanonicalRuntimeHomePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CanonicalRuntimeHomePath")
            .field(&self.0)
            .finish()
    }
}

/// Opaque domain-separated identity for one setup coordination file.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RuntimeHomeSetupLockIdentity(String);

impl RuntimeHomeSetupLockIdentity {
    /// Returns the complete opaque digest identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RuntimeHomeSetupLockIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RuntimeHomeSetupLockIdentity")
            .field(&self.0)
            .finish()
    }
}

/// Setup operation requesting the Runtime Home lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeSetupOperation {
    Init,
    ConnectionAdd,
}

impl RuntimeHomeSetupOperation {
    /// Stable machine-readable operation value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::ConnectionAdd => "connection_add",
        }
    }
}

/// Bounded acquisition behavior for one setup lease request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeSetupWaitPolicy {
    /// Attempt acquisition once without waiting.
    Immediate,
}

impl RuntimeHomeSetupWaitPolicy {
    /// Stable machine-readable wait-policy value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
        }
    }
}

/// Bounded facts returned when another setup transaction owns the lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeSetupBusy {
    target: CanonicalRuntimeHomePath,
    operation: RuntimeHomeSetupOperation,
    wait_policy: RuntimeHomeSetupWaitPolicy,
    elapsed: Duration,
}

impl RuntimeHomeSetupBusy {
    /// Canonical Runtime Home whose setup is currently serialized.
    pub fn target(&self) -> &CanonicalRuntimeHomePath {
        &self.target
    }

    /// Requested setup operation.
    pub const fn operation(&self) -> RuntimeHomeSetupOperation {
        self.operation
    }

    /// Bounded acquisition policy used by this request.
    pub const fn wait_policy(&self) -> RuntimeHomeSetupWaitPolicy {
        self.wait_policy
    }

    /// Time spent attempting the bounded acquisition.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Typed result of an exclusive Runtime Home setup-lease request.
#[derive(Debug)]
pub enum RuntimeHomeSetupLeaseOutcome {
    Acquired(RuntimeHomeSetupLease),
    Busy(RuntimeHomeSetupBusy),
}

/// A process-wide and cross-process exclusive setup lease for one Runtime Home.
///
/// The OS lock is released when this non-cloneable value closes its file
/// handle or when the process terminates. The persistent coordination file is
/// not removed by `Drop` and its existence does not indicate lease ownership.
pub struct RuntimeHomeSetupLease {
    target: CanonicalRuntimeHomePath,
    lock_identity: RuntimeHomeSetupLockIdentity,
    platform_lock: PlatformFileLock,
}

impl RuntimeHomeSetupLease {
    /// Acquires the exclusive setup lease under the requested bounded policy.
    pub fn acquire(
        target: impl AsRef<Path>,
        operation: RuntimeHomeSetupOperation,
        wait_policy: RuntimeHomeSetupWaitPolicy,
    ) -> Result<RuntimeHomeSetupLeaseOutcome, RuntimeHomeSetupLeaseError> {
        let started = Instant::now();
        let target = canonical_runtime_home_path(target.as_ref())?;
        let lock_identity = runtime_home_setup_lock_identity(&target);
        let coordination_directory = setup_coordination_directory()?;
        ensure_coordination_directory(&coordination_directory)?;
        let lock_path = coordination_file_path(&coordination_directory, &lock_identity);
        let platform_lock = match PlatformFileLock::try_acquire(&lock_path)? {
            PlatformFileLockOutcome::Acquired(lock) => lock,
            PlatformFileLockOutcome::Busy => {
                return Ok(RuntimeHomeSetupLeaseOutcome::Busy(RuntimeHomeSetupBusy {
                    target,
                    operation,
                    wait_policy,
                    elapsed: started.elapsed(),
                }));
            }
        };
        Ok(RuntimeHomeSetupLeaseOutcome::Acquired(Self {
            target,
            lock_identity,
            platform_lock,
        }))
    }

    /// Exact canonical Runtime Home protected by this lease.
    pub fn target(&self) -> &CanonicalRuntimeHomePath {
        &self.target
    }

    /// Opaque full-digest identity used for coordination.
    pub fn lock_identity(&self) -> &RuntimeHomeSetupLockIdentity {
        &self.lock_identity
    }

    /// Returns whether a path resolves to this lease's exact canonical target.
    pub fn matches_target(
        &self,
        target: impl AsRef<Path>,
    ) -> Result<bool, RuntimeHomeSetupLeaseError> {
        canonical_runtime_home_path(target.as_ref())
            .map(|target| runtime_home_setup_lock_identity(&target) == self.lock_identity)
    }
}

impl fmt::Debug for RuntimeHomeSetupLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeHomeSetupLease")
            .field("target", &self.target)
            .field("lock_identity", &self.lock_identity)
            .field("platform_lock", &self.platform_lock)
            .finish()
    }
}

/// Failure to canonicalize, locate, open, or lock setup coordination state.
#[derive(Debug)]
pub struct RuntimeHomeSetupLeaseError {
    stage: &'static str,
    detail: String,
    source: Option<io::Error>,
}

impl RuntimeHomeSetupLeaseError {
    fn io(stage: &'static str, detail: impl Into<String>, source: io::Error) -> Self {
        Self {
            stage,
            detail: detail.into(),
            source: Some(source),
        }
    }

    fn platform(stage: &'static str, source: PlatformBoundaryError) -> Self {
        Self {
            stage,
            detail: source.to_string(),
            source: None,
        }
    }

    fn invalid(detail: impl Into<String>) -> Self {
        Self::at("canonicalize_target", detail)
    }

    fn at(stage: &'static str, detail: impl Into<String>) -> Self {
        Self {
            stage,
            detail: detail.into(),
            source: None,
        }
    }

    /// Stable implementation stage at which acquisition failed.
    pub const fn stage(&self) -> &'static str {
        self.stage
    }
}

impl fmt::Display for RuntimeHomeSetupLeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Runtime Home setup lease failed during {}: {}",
            self.stage, self.detail
        )
    }
}

impl std::error::Error for RuntimeHomeSetupLeaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

struct PlatformFileLock {
    file: File,
}

impl PlatformFileLock {
    fn try_acquire(path: &Path) -> Result<PlatformFileLockOutcome, RuntimeHomeSetupLeaseError> {
        let file = open_coordination_file(path).map_err(|error| {
            RuntimeHomeSetupLeaseError::io(
                "open_coordination_file",
                "the Volicord-owned coordination file could not be opened",
                error,
            )
        })?;
        match try_lock_file_exclusive(&file).map_err(|error| {
            RuntimeHomeSetupLeaseError::io(
                "acquire_platform_lock",
                "the operating system rejected setup-lock acquisition",
                error,
            )
        })? {
            FileLockAttempt::Acquired => Ok(PlatformFileLockOutcome::Acquired(Self { file })),
            FileLockAttempt::Busy => Ok(PlatformFileLockOutcome::Busy),
        }
    }
}

impl fmt::Debug for PlatformFileLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.file;
        formatter.write_str("PlatformFileLock(<exclusive OS file lock>)")
    }
}

enum PlatformFileLockOutcome {
    Acquired(PlatformFileLock),
    Busy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileLockAttempt {
    Acquired,
    Busy,
}

fn canonical_runtime_home_path(
    target: &Path,
) -> Result<CanonicalRuntimeHomePath, RuntimeHomeSetupLeaseError> {
    if !target.is_absolute() {
        return Err(RuntimeHomeSetupLeaseError::invalid(
            "the Runtime Home setup target must be absolute",
        ));
    }
    let normalized = normalize_absolute_target(target)?;
    let mut candidate = normalized.clone();
    let mut unresolved = Vec::new();
    let canonical = loop {
        match fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_dir() => {
                let canonical = fs::canonicalize(&candidate).map_err(|error| {
                    RuntimeHomeSetupLeaseError::io(
                        "canonicalize_target",
                        "the nearest existing Runtime Home ancestor could not be canonicalized",
                        error,
                    )
                })?;
                break canonical;
            }
            Ok(_) => {
                return Err(RuntimeHomeSetupLeaseError::invalid(format!(
                    "an existing Runtime Home path component is not a directory: {}",
                    candidate.display()
                )));
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                let component = candidate.file_name().ok_or_else(|| {
                    RuntimeHomeSetupLeaseError::invalid(
                        "the Runtime Home setup target has no existing directory ancestor",
                    )
                })?;
                unresolved.push(component.to_os_string());
                candidate = candidate
                    .parent()
                    .ok_or_else(|| {
                        RuntimeHomeSetupLeaseError::invalid(
                            "the Runtime Home setup target has no existing directory ancestor",
                        )
                    })?
                    .to_path_buf();
            }
            Err(error) => {
                return Err(RuntimeHomeSetupLeaseError::io(
                    "canonicalize_target",
                    format!(
                        "the Runtime Home setup target could not be inspected at {}",
                        candidate.display()
                    ),
                    error,
                ));
            }
        }
    };
    let mut canonical_target = canonical;
    unresolved.reverse();
    for component in unresolved {
        canonical_target.push(component);
    }
    validate_runtime_home_platform(&canonical_target)?;
    Ok(CanonicalRuntimeHomePath(canonical_target))
}

fn normalize_absolute_target(target: &Path) -> Result<PathBuf, RuntimeHomeSetupLeaseError> {
    let mut normalized = PathBuf::new();
    for component in target.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(RuntimeHomeSetupLeaseError::invalid(
                        "the Runtime Home setup target escapes its filesystem root",
                    ));
                }
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    if normalized.is_absolute() {
        Ok(normalized)
    } else {
        Err(RuntimeHomeSetupLeaseError::invalid(
            "the Runtime Home setup target did not resolve to an absolute path",
        ))
    }
}

fn validate_runtime_home_platform(target: &Path) -> Result<(), RuntimeHomeSetupLeaseError> {
    let boundary = observe_local_platform_boundary()
        .map_err(|error| RuntimeHomeSetupLeaseError::platform("validate_platform", error))?;
    if boundary.environment != PlatformEnvironment::Wsl2 {
        return Ok(());
    }
    let filesystem = observe_path_filesystem(target)
        .map_err(|error| RuntimeHomeSetupLeaseError::platform("validate_platform", error))?;
    if filesystem == PathFilesystemKind::LinuxExt4 {
        Ok(())
    } else {
        Err(RuntimeHomeSetupLeaseError::at(
            "validate_platform",
            "the WSL2 Runtime Home setup target is outside the distribution ext4 filesystem",
        ))
    }
}

fn runtime_home_setup_lock_identity(
    target: &CanonicalRuntimeHomePath,
) -> RuntimeHomeSetupLockIdentity {
    let path_identity = canonical_path_identity_bytes(target.as_path());
    let mut hasher = Sha256::new();
    hasher.update((SETUP_LOCK_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(SETUP_LOCK_DOMAIN);
    hasher.update((path_identity.len() as u64).to_be_bytes());
    hasher.update(path_identity);
    RuntimeHomeSetupLockIdentity(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(not(windows))]
fn canonical_path_identity_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(windows)]
fn canonical_path_identity_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    let normalized: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .map(|unit| {
            if unit == b'/' as u16 {
                b'\\' as u16
            } else {
                unit
            }
        })
        .collect();
    let mut identity = Vec::with_capacity(normalized.len() * 2);
    for decoded in char::decode_utf16(normalized) {
        match decoded {
            Ok(character) => {
                for folded in character.to_lowercase() {
                    let mut units = [0_u16; 2];
                    for unit in folded.encode_utf16(&mut units) {
                        identity.extend_from_slice(&unit.to_le_bytes());
                    }
                }
            }
            Err(unpaired) => {
                identity.extend_from_slice(&unpaired.unpaired_surrogate().to_le_bytes())
            }
        }
    }
    identity
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn setup_coordination_directory() -> Result<PathBuf, RuntimeHomeSetupLeaseError> {
    let uid = rustix::process::geteuid().as_raw();
    Ok(PathBuf::from("/tmp").join(format!("volicord-{uid}-{SETUP_COORDINATION_DIRECTORY}")))
}

#[cfg(windows)]
fn setup_coordination_directory() -> Result<PathBuf, RuntimeHomeSetupLeaseError> {
    Ok(std::env::temp_dir()
        .join("Volicord")
        .join(SETUP_COORDINATION_DIRECTORY))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn setup_coordination_directory() -> Result<PathBuf, RuntimeHomeSetupLeaseError> {
    Err(RuntimeHomeSetupLeaseError::at(
        "prepare_coordination_directory",
        "the current platform has no Runtime Home setup-lock location",
    ))
}

fn ensure_coordination_directory(path: &Path) -> Result<(), RuntimeHomeSetupLeaseError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(path).map_err(|error| {
            RuntimeHomeSetupLeaseError::io(
                "prepare_coordination_directory",
                "the Volicord-owned coordination directory could not be created",
                error,
            )
        })?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(path).map_err(|error| {
        RuntimeHomeSetupLeaseError::io(
            "prepare_coordination_directory",
            "the Volicord-owned coordination directory could not be created",
            error,
        )
    })?;

    let metadata = fs::symlink_metadata(path).map_err(|error| {
        RuntimeHomeSetupLeaseError::io(
            "prepare_coordination_directory",
            "the Volicord-owned coordination directory could not be inspected",
            error,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(RuntimeHomeSetupLeaseError::at(
            "prepare_coordination_directory",
            "the Volicord-owned setup coordination path is not a directory",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let effective_uid = rustix::process::geteuid().as_raw();
        if metadata.uid() != effective_uid || metadata.mode() & 0o077 != 0 {
            return Err(RuntimeHomeSetupLeaseError::at(
                "prepare_coordination_directory",
                "the Volicord-owned setup coordination directory must be owned by the effective user and inaccessible to group or other users",
            ));
        }
    }
    Ok(())
}

fn coordination_file_path(directory: &Path, identity: &RuntimeHomeSetupLockIdentity) -> PathBuf {
    let digest = identity
        .as_str()
        .strip_prefix("sha256:")
        .expect("setup lock identities always use the sha256 prefix");
    directory.join(format!("runtime-home-{digest}.lock"))
}

fn open_coordination_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if metadata.is_file() {
        Ok(file)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "setup coordination target is not a regular file",
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn try_lock_file_exclusive(file: &File) -> io::Result<FileLockAttempt> {
    use rustix::fs::{flock, FlockOperation};
    match flock(file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(FileLockAttempt::Acquired),
        Err(error) if error == rustix::io::Errno::AGAIN => Ok(FileLockAttempt::Busy),
        Err(error) => Err(io::Error::from(error)),
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn try_lock_file_exclusive(file: &File) -> io::Result<FileLockAttempt> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::ERROR_LOCK_VIOLATION,
        Storage::FileSystem::{LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY},
        System::IO::OVERLAPPED,
    };

    let mut overlapped = OVERLAPPED::default();
    // SAFETY: the handle is borrowed from a live owned `File` for the complete
    // call. `overlapped` is a valid aligned structure whose zero offset selects
    // the first byte, and Windows retains no Rust pointer after this
    // synchronous fail-immediately request. The handle remains owned by
    // `PlatformFileLock`; closing it releases the byte-range lock.
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if locked != 0 {
        return Ok(FileLockAttempt::Acquired);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error().map(|code| code as u32) == Some(ERROR_LOCK_VIOLATION) {
        Ok(FileLockAttempt::Busy)
    } else {
        Err(error)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn try_lock_file_exclusive(_file: &File) -> io::Result<FileLockAttempt> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "exclusive setup file locks are unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn acquire(path: &Path) -> RuntimeHomeSetupLeaseOutcome {
        RuntimeHomeSetupLease::acquire(
            path,
            RuntimeHomeSetupOperation::Init,
            RuntimeHomeSetupWaitPolicy::Immediate,
        )
        .expect("setup lease acquisition")
    }

    #[test]
    fn lexical_aliases_share_one_setup_lock() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let alias = fixture
            .path()
            .join("nested")
            .join("..")
            .join("runtime-home");

        let RuntimeHomeSetupLeaseOutcome::Acquired(first) = acquire(&target) else {
            panic!("first setup lease should be acquired");
        };
        let RuntimeHomeSetupLeaseOutcome::Busy(second) = acquire(&alias) else {
            panic!("lexical alias should observe the held setup lease");
        };
        assert_eq!(first.target(), second.target());
    }

    #[cfg(unix)]
    #[test]
    fn existing_directory_symlink_aliases_share_one_setup_lock() {
        use std::os::unix::fs::symlink;

        let fixture = tempdir().expect("fixture");
        let real_parent = fixture.path().join("real");
        let alias_parent = fixture.path().join("alias");
        fs::create_dir(&real_parent).expect("real parent");
        symlink(&real_parent, &alias_parent).expect("directory symlink");
        let target = real_parent.join("runtime-home");
        let alias = alias_parent.join("runtime-home");

        let RuntimeHomeSetupLeaseOutcome::Acquired(_first) = acquire(&target) else {
            panic!("first setup lease should be acquired");
        };
        assert!(matches!(
            acquire(&alias),
            RuntimeHomeSetupLeaseOutcome::Busy(_)
        ));
    }

    #[test]
    fn distinct_runtime_homes_have_independent_setup_locks() {
        let fixture = tempdir().expect("fixture");
        let first_target = fixture.path().join("runtime-home-a");
        let second_target = fixture.path().join("runtime-home-b");

        let RuntimeHomeSetupLeaseOutcome::Acquired(first) = acquire(&first_target) else {
            panic!("first setup lease should be acquired");
        };
        let RuntimeHomeSetupLeaseOutcome::Acquired(second) = acquire(&second_target) else {
            panic!("distinct setup lease should be acquired independently");
        };
        assert_ne!(first.lock_identity(), second.lock_identity());
    }

    #[test]
    fn coordination_file_persists_without_owning_a_later_lease() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let (directory, identity) = {
            let RuntimeHomeSetupLeaseOutcome::Acquired(lease) = acquire(&target) else {
                panic!("first setup lease should be acquired");
            };
            (
                setup_coordination_directory().expect("coordination directory"),
                lease.lock_identity().clone(),
            )
        };
        let path = coordination_file_path(&directory, &identity);
        assert!(path.is_file());
        assert!(matches!(
            acquire(&target),
            RuntimeHomeSetupLeaseOutcome::Acquired(_)
        ));
        assert!(path.is_file());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn unix_setup_lock_is_owned_by_the_open_file_description() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let RuntimeHomeSetupLeaseOutcome::Acquired(lease) = acquire(&target) else {
            panic!("first setup lease should be acquired");
        };
        assert!(matches!(
            acquire(&target),
            RuntimeHomeSetupLeaseOutcome::Busy(_)
        ));
        drop(lease);
        assert!(matches!(
            acquire(&target),
            RuntimeHomeSetupLeaseOutcome::Acquired(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_case_aliases_share_identity_without_collapsing_non_bmp_names() {
        let upper = CanonicalRuntimeHomePath(PathBuf::from(r"C:\Users\Example\Runtime"));
        let lower = CanonicalRuntimeHomePath(PathBuf::from(r"c:\users\example\runtime"));
        assert_eq!(
            runtime_home_setup_lock_identity(&upper),
            runtime_home_setup_lock_identity(&lower)
        );

        let first = CanonicalRuntimeHomePath(PathBuf::from(r"C:\Runtime\😀"));
        let second = CanonicalRuntimeHomePath(PathBuf::from(r"C:\Runtime\😁"));
        assert_ne!(
            runtime_home_setup_lock_identity(&first),
            runtime_home_setup_lock_identity(&second)
        );
    }
}
