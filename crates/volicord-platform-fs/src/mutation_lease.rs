use std::{
    fmt, fs,
    fs::{File, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use crate::{
    observe_local_platform_boundary, observe_path_filesystem, PathFilesystemKind,
    PlatformBoundaryError, PlatformEnvironment,
};

const MUTATION_LOCK_DOMAIN: &[u8] = b"volicord.runtime-home-mutation-lock";
const MUTATION_COORDINATION_DIRECTORY: &str = "runtime-home-mutation";
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

/// One exact canonical Runtime Home target used for mutation admission.
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

/// Opaque domain-separated identity for one mutation coordination file.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RuntimeHomeMutationLockIdentity(String);

impl RuntimeHomeMutationLockIdentity {
    /// Returns the complete opaque digest identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RuntimeHomeMutationLockIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RuntimeHomeMutationLockIdentity")
            .field(&self.0)
            .finish()
    }
}

/// Admission mode held for one canonical Runtime Home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeMutationLeaseMode {
    /// Admission for an ordinary Runtime Home writer.
    SharedWriter,
    /// Exclusive admission for a Runtime Home setup transaction.
    ExclusiveSetup,
}

impl RuntimeHomeMutationLeaseMode {
    /// Stable machine-readable mode value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SharedWriter => "shared_writer",
            Self::ExclusiveSetup => "exclusive_setup",
        }
    }
}

/// Bounded acquisition behavior for one mutation-lease request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeMutationWaitPolicy {
    /// Attempt acquisition once without waiting.
    Immediate,
    /// Retry until the lease is acquired or the timeout expires.
    Bounded {
        /// Maximum time spent attempting acquisition.
        timeout: Duration,
    },
}

impl RuntimeHomeMutationWaitPolicy {
    /// Stable machine-readable wait-policy value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Bounded { .. } => "bounded",
        }
    }

    const fn timeout(self) -> Option<Duration> {
        match self {
            Self::Immediate => None,
            Self::Bounded { timeout } => Some(timeout),
        }
    }
}

/// Bounded facts returned when another mutation lease conflicts with a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeMutationBusy {
    target: CanonicalRuntimeHomePath,
    requested_mode: RuntimeHomeMutationLeaseMode,
    wait_policy: RuntimeHomeMutationWaitPolicy,
    elapsed: Duration,
}

impl RuntimeHomeMutationBusy {
    /// Canonical Runtime Home whose mutation admission is currently occupied.
    pub fn target(&self) -> &CanonicalRuntimeHomePath {
        &self.target
    }

    /// Requested lease mode.
    pub const fn requested_mode(&self) -> RuntimeHomeMutationLeaseMode {
        self.requested_mode
    }

    /// Bounded acquisition policy used by this request.
    pub const fn wait_policy(&self) -> RuntimeHomeMutationWaitPolicy {
        self.wait_policy
    }

    /// Time spent attempting the bounded acquisition.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Typed result of a Runtime Home mutation-lease request.
#[derive(Debug)]
pub enum RuntimeHomeMutationLeaseOutcome {
    /// The requested lease was acquired.
    Acquired(RuntimeHomeMutationLease),
    /// A live conflicting lease prevented acquisition.
    Busy(RuntimeHomeMutationBusy),
}

/// A process-wide and cross-process mutation-admission lease for one Runtime Home.
///
/// The OS lock is released when this non-cloneable value closes its file
/// handle or when the process terminates. The persistent coordination file is
/// not removed by `Drop` and its existence does not indicate lease ownership.
pub struct RuntimeHomeMutationLease {
    target: CanonicalRuntimeHomePath,
    lock_identity: RuntimeHomeMutationLockIdentity,
    mode: RuntimeHomeMutationLeaseMode,
    platform_lock: PlatformFileLock,
}

impl RuntimeHomeMutationLease {
    /// Acquires the requested mutation lease under the requested bounded policy.
    pub fn acquire(
        target: impl AsRef<Path>,
        mode: RuntimeHomeMutationLeaseMode,
        wait_policy: RuntimeHomeMutationWaitPolicy,
    ) -> Result<RuntimeHomeMutationLeaseOutcome, RuntimeHomeMutationLeaseError> {
        Self::acquire_with_busy_observer(target.as_ref(), mode, wait_policy, || {})
    }

    fn acquire_with_busy_observer(
        target: &Path,
        mode: RuntimeHomeMutationLeaseMode,
        wait_policy: RuntimeHomeMutationWaitPolicy,
        mut observe_busy: impl FnMut(),
    ) -> Result<RuntimeHomeMutationLeaseOutcome, RuntimeHomeMutationLeaseError> {
        let started = Instant::now();
        let target = canonical_runtime_home_path(target)?;
        let lock_identity = runtime_home_mutation_lock_identity(&target);
        let coordination_directory = mutation_coordination_directory()?;
        ensure_coordination_directory(&coordination_directory)?;
        let lock_path = coordination_file_path(&coordination_directory, &lock_identity);
        let platform_lock = PlatformFileLock::open(&lock_path)?;

        loop {
            match platform_lock.try_acquire(mode)? {
                FileLockAttempt::Acquired => {
                    return Ok(RuntimeHomeMutationLeaseOutcome::Acquired(Self {
                        target,
                        lock_identity,
                        mode,
                        platform_lock,
                    }));
                }
                FileLockAttempt::Busy => {
                    observe_busy();
                    let elapsed = started.elapsed();
                    let Some(timeout) = wait_policy.timeout() else {
                        return Ok(RuntimeHomeMutationLeaseOutcome::Busy(
                            RuntimeHomeMutationBusy {
                                target,
                                requested_mode: mode,
                                wait_policy,
                                elapsed,
                            },
                        ));
                    };
                    let Some(remaining) = timeout.checked_sub(elapsed) else {
                        return Ok(RuntimeHomeMutationLeaseOutcome::Busy(
                            RuntimeHomeMutationBusy {
                                target,
                                requested_mode: mode,
                                wait_policy,
                                elapsed,
                            },
                        ));
                    };
                    if remaining.is_zero() {
                        return Ok(RuntimeHomeMutationLeaseOutcome::Busy(
                            RuntimeHomeMutationBusy {
                                target,
                                requested_mode: mode,
                                wait_policy,
                                elapsed,
                            },
                        ));
                    }
                    thread::sleep(remaining.min(LOCK_RETRY_INTERVAL));
                }
            }
        }
    }

    /// Exact canonical Runtime Home protected by this lease.
    pub fn target(&self) -> &CanonicalRuntimeHomePath {
        &self.target
    }

    /// Opaque full-digest identity used for coordination.
    pub fn lock_identity(&self) -> &RuntimeHomeMutationLockIdentity {
        &self.lock_identity
    }

    /// Admission mode held by this lease.
    pub const fn mode(&self) -> RuntimeHomeMutationLeaseMode {
        self.mode
    }

    /// Borrows this live lease as a higher-layer mutation capability.
    pub fn permit(&self) -> RuntimeHomeMutationPermit<'_> {
        RuntimeHomeMutationPermit { lease: self }
    }

    /// Returns whether a path resolves to this lease's exact canonical target.
    pub fn matches_target(
        &self,
        target: impl AsRef<Path>,
    ) -> Result<bool, RuntimeHomeMutationLeaseError> {
        canonical_runtime_home_path(target.as_ref())
            .map(|target| runtime_home_mutation_lock_identity(&target) == self.lock_identity)
    }
}

impl fmt::Debug for RuntimeHomeMutationLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeHomeMutationLease")
            .field("target", &self.target)
            .field("lock_identity", &self.lock_identity)
            .field("mode", &self.mode)
            .field("platform_lock", &self.platform_lock)
            .finish()
    }
}

/// Borrowed proof that one live Runtime Home mutation lease is held.
///
/// The permit cannot outlive its lease, cannot be constructed by callers, and
/// has no destructive drop behavior.
pub struct RuntimeHomeMutationPermit<'lease> {
    lease: &'lease RuntimeHomeMutationLease,
}

impl RuntimeHomeMutationPermit<'_> {
    /// Exact canonical Runtime Home covered by this permit.
    pub fn target(&self) -> &CanonicalRuntimeHomePath {
        self.lease.target()
    }

    /// Admission mode held by the borrowed lease.
    pub const fn mode(&self) -> RuntimeHomeMutationLeaseMode {
        self.lease.mode()
    }
}

impl fmt::Debug for RuntimeHomeMutationPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeHomeMutationPermit")
            .field("target", self.target())
            .field("mode", &self.mode())
            .finish()
    }
}

/// Failure to canonicalize, locate, open, or lock mutation coordination state.
#[derive(Debug)]
pub struct RuntimeHomeMutationLeaseError {
    stage: &'static str,
    detail: String,
    source: Option<io::Error>,
}

impl RuntimeHomeMutationLeaseError {
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

impl fmt::Display for RuntimeHomeMutationLeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Runtime Home mutation lease failed during {}: {}",
            self.stage, self.detail
        )
    }
}

impl std::error::Error for RuntimeHomeMutationLeaseError {
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
    fn open(path: &Path) -> Result<Self, RuntimeHomeMutationLeaseError> {
        let file = open_coordination_file(path).map_err(|error| {
            RuntimeHomeMutationLeaseError::io(
                "open_coordination_file",
                "the Volicord-owned coordination file could not be opened",
                error,
            )
        })?;
        Ok(Self { file })
    }

    fn try_acquire(
        &self,
        mode: RuntimeHomeMutationLeaseMode,
    ) -> Result<FileLockAttempt, RuntimeHomeMutationLeaseError> {
        try_lock_file(&self.file, mode).map_err(|error| {
            RuntimeHomeMutationLeaseError::io(
                "acquire_platform_lock",
                "the operating system rejected mutation-lock acquisition",
                error,
            )
        })
    }
}

impl fmt::Debug for PlatformFileLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.file;
        formatter.write_str("PlatformFileLock(<OS file lock>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileLockAttempt {
    Acquired,
    Busy,
}

fn canonical_runtime_home_path(
    target: &Path,
) -> Result<CanonicalRuntimeHomePath, RuntimeHomeMutationLeaseError> {
    if !target.is_absolute() {
        return Err(RuntimeHomeMutationLeaseError::invalid(
            "the Runtime Home mutation target must be absolute",
        ));
    }
    let normalized = normalize_absolute_target(target)?;
    let mut candidate = normalized.clone();
    let mut unresolved = Vec::new();
    let canonical = loop {
        match fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_dir() => {
                let canonical = fs::canonicalize(&candidate).map_err(|error| {
                    RuntimeHomeMutationLeaseError::io(
                        "canonicalize_target",
                        "the nearest existing Runtime Home ancestor could not be canonicalized",
                        error,
                    )
                })?;
                break canonical;
            }
            Ok(_) => {
                return Err(RuntimeHomeMutationLeaseError::invalid(format!(
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
                    RuntimeHomeMutationLeaseError::invalid(
                        "the Runtime Home mutation target has no existing directory ancestor",
                    )
                })?;
                unresolved.push(component.to_os_string());
                candidate = candidate
                    .parent()
                    .ok_or_else(|| {
                        RuntimeHomeMutationLeaseError::invalid(
                            "the Runtime Home mutation target has no existing directory ancestor",
                        )
                    })?
                    .to_path_buf();
            }
            Err(error) => {
                return Err(RuntimeHomeMutationLeaseError::io(
                    "canonicalize_target",
                    format!(
                        "the Runtime Home mutation target could not be inspected at {}",
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

fn normalize_absolute_target(target: &Path) -> Result<PathBuf, RuntimeHomeMutationLeaseError> {
    let mut normalized = PathBuf::new();
    for component in target.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(RuntimeHomeMutationLeaseError::invalid(
                        "the Runtime Home mutation target escapes its filesystem root",
                    ));
                }
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    if normalized.is_absolute() {
        Ok(normalized)
    } else {
        Err(RuntimeHomeMutationLeaseError::invalid(
            "the Runtime Home mutation target did not resolve to an absolute path",
        ))
    }
}

fn validate_runtime_home_platform(target: &Path) -> Result<(), RuntimeHomeMutationLeaseError> {
    let boundary = observe_local_platform_boundary()
        .map_err(|error| RuntimeHomeMutationLeaseError::platform("validate_platform", error))?;
    if boundary.environment != PlatformEnvironment::Wsl2 {
        return Ok(());
    }
    let filesystem = observe_path_filesystem(target)
        .map_err(|error| RuntimeHomeMutationLeaseError::platform("validate_platform", error))?;
    if filesystem == PathFilesystemKind::LinuxExt4 {
        Ok(())
    } else {
        Err(RuntimeHomeMutationLeaseError::at(
            "validate_platform",
            "the WSL2 Runtime Home mutation target is outside the distribution ext4 filesystem",
        ))
    }
}

fn runtime_home_mutation_lock_identity(
    target: &CanonicalRuntimeHomePath,
) -> RuntimeHomeMutationLockIdentity {
    let path_identity = canonical_path_identity_bytes(target.as_path());
    let mut hasher = Sha256::new();
    hasher.update((MUTATION_LOCK_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(MUTATION_LOCK_DOMAIN);
    hasher.update((path_identity.len() as u64).to_be_bytes());
    hasher.update(path_identity);
    RuntimeHomeMutationLockIdentity(format!("sha256:{:x}", hasher.finalize()))
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
fn mutation_coordination_directory() -> Result<PathBuf, RuntimeHomeMutationLeaseError> {
    let uid = rustix::process::geteuid().as_raw();
    Ok(PathBuf::from("/tmp").join(format!("volicord-{uid}-{MUTATION_COORDINATION_DIRECTORY}")))
}

#[cfg(windows)]
fn mutation_coordination_directory() -> Result<PathBuf, RuntimeHomeMutationLeaseError> {
    Ok(std::env::temp_dir()
        .join("Volicord")
        .join(MUTATION_COORDINATION_DIRECTORY))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn mutation_coordination_directory() -> Result<PathBuf, RuntimeHomeMutationLeaseError> {
    Err(RuntimeHomeMutationLeaseError::at(
        "prepare_coordination_directory",
        "the current platform has no Runtime Home mutation-lock location",
    ))
}

fn ensure_coordination_directory(path: &Path) -> Result<(), RuntimeHomeMutationLeaseError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(path).map_err(|error| {
            RuntimeHomeMutationLeaseError::io(
                "prepare_coordination_directory",
                "the Volicord-owned coordination directory could not be created",
                error,
            )
        })?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(path).map_err(|error| {
        RuntimeHomeMutationLeaseError::io(
            "prepare_coordination_directory",
            "the Volicord-owned coordination directory could not be created",
            error,
        )
    })?;

    let metadata = fs::symlink_metadata(path).map_err(|error| {
        RuntimeHomeMutationLeaseError::io(
            "prepare_coordination_directory",
            "the Volicord-owned coordination directory could not be inspected",
            error,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(RuntimeHomeMutationLeaseError::at(
            "prepare_coordination_directory",
            "the Volicord-owned mutation coordination path is not a directory",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let effective_uid = rustix::process::geteuid().as_raw();
        if metadata.uid() != effective_uid || metadata.mode() & 0o077 != 0 {
            return Err(RuntimeHomeMutationLeaseError::at(
                "prepare_coordination_directory",
                "the Volicord-owned mutation coordination directory must be owned by the effective user and inaccessible to group or other users",
            ));
        }
    }
    Ok(())
}

fn coordination_file_path(directory: &Path, identity: &RuntimeHomeMutationLockIdentity) -> PathBuf {
    let digest = identity
        .as_str()
        .strip_prefix("sha256:")
        .expect("mutation lock identities always use the sha256 prefix");
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
            "mutation coordination target is not a regular file",
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn try_lock_file(file: &File, mode: RuntimeHomeMutationLeaseMode) -> io::Result<FileLockAttempt> {
    use rustix::fs::{flock, FlockOperation};

    let operation = match mode {
        RuntimeHomeMutationLeaseMode::SharedWriter => FlockOperation::NonBlockingLockShared,
        RuntimeHomeMutationLeaseMode::ExclusiveSetup => FlockOperation::NonBlockingLockExclusive,
    };
    match flock(file, operation) {
        Ok(()) => Ok(FileLockAttempt::Acquired),
        Err(error) if error == rustix::io::Errno::AGAIN => Ok(FileLockAttempt::Busy),
        Err(error) => Err(io::Error::from(error)),
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn try_lock_file(file: &File, mode: RuntimeHomeMutationLeaseMode) -> io::Result<FileLockAttempt> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::ERROR_LOCK_VIOLATION,
        Storage::FileSystem::{LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY},
        System::IO::OVERLAPPED,
    };

    let flags = LOCKFILE_FAIL_IMMEDIATELY
        | match mode {
            RuntimeHomeMutationLeaseMode::SharedWriter => 0,
            RuntimeHomeMutationLeaseMode::ExclusiveSetup => LOCKFILE_EXCLUSIVE_LOCK,
        };
    let mut overlapped = OVERLAPPED::default();
    // SAFETY: the handle is borrowed from a live owned `File` for the complete
    // call. `overlapped` is a valid aligned structure whose zero offset selects
    // the first byte, and Windows retains no Rust pointer after this
    // synchronous fail-immediately request. Both modes lock the same one-byte
    // range. The handle remains owned by `PlatformFileLock`; closing it
    // releases the byte-range lock.
    let locked = unsafe { LockFileEx(file.as_raw_handle(), flags, 0, 1, 0, &mut overlapped) };
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
fn try_lock_file(_file: &File, _mode: RuntimeHomeMutationLeaseMode) -> io::Result<FileLockAttempt> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Runtime Home mutation file locks are unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use tempfile::tempdir;

    fn acquire(path: &Path, mode: RuntimeHomeMutationLeaseMode) -> RuntimeHomeMutationLeaseOutcome {
        RuntimeHomeMutationLease::acquire(path, mode, RuntimeHomeMutationWaitPolicy::Immediate)
            .expect("mutation lease acquisition")
    }

    fn acquired(path: &Path, mode: RuntimeHomeMutationLeaseMode) -> RuntimeHomeMutationLease {
        let RuntimeHomeMutationLeaseOutcome::Acquired(lease) = acquire(path, mode) else {
            panic!("mutation lease should be acquired");
        };
        lease
    }

    #[test]
    fn shared_writers_coexist_for_one_runtime_home() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let (held_tx, held_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        std::thread::scope(|scope| {
            let first_target = target.clone();
            let holder = scope.spawn(move || {
                let first = acquired(&first_target, RuntimeHomeMutationLeaseMode::SharedWriter);
                held_tx.send(first.target().clone()).expect("signal holder");
                release_rx.recv().expect("wait for release");
                first
            });
            let first_target = held_rx.recv().expect("holder acquired shared lease");
            let second = acquired(&target, RuntimeHomeMutationLeaseMode::SharedWriter);

            assert_eq!(&first_target, second.target());
            assert_eq!(second.mode(), RuntimeHomeMutationLeaseMode::SharedWriter);
            release_tx.send(()).expect("release holder");
            let first = holder.join().expect("shared holder panicked");
            assert_eq!(first.mode(), RuntimeHomeMutationLeaseMode::SharedWriter);
        });
    }

    #[test]
    fn shared_and_exclusive_modes_use_one_conflicting_lock_region() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");

        let shared = acquired(&target, RuntimeHomeMutationLeaseMode::SharedWriter);
        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup),
            RuntimeHomeMutationLeaseOutcome::Busy(_)
        ));
        drop(shared);

        let exclusive = acquired(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::SharedWriter),
            RuntimeHomeMutationLeaseOutcome::Busy(_)
        ));
        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup),
            RuntimeHomeMutationLeaseOutcome::Busy(_)
        ));
        drop(exclusive);

        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::SharedWriter),
            RuntimeHomeMutationLeaseOutcome::Acquired(_)
        ));
    }

    #[test]
    fn last_shared_writer_release_allows_exclusive_setup() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let first = acquired(&target, RuntimeHomeMutationLeaseMode::SharedWriter);
        let second = acquired(&target, RuntimeHomeMutationLeaseMode::SharedWriter);

        drop(first);
        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup),
            RuntimeHomeMutationLeaseOutcome::Busy(_)
        ));
        drop(second);
        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup),
            RuntimeHomeMutationLeaseOutcome::Acquired(_)
        ));
    }

    #[test]
    fn lexical_aliases_share_one_mutation_lock() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let alias = fixture
            .path()
            .join("nested")
            .join("..")
            .join("runtime-home");

        let first = acquired(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        let RuntimeHomeMutationLeaseOutcome::Busy(second) =
            acquire(&alias, RuntimeHomeMutationLeaseMode::SharedWriter)
        else {
            panic!("lexical alias should observe the held mutation lease");
        };
        assert_eq!(first.target(), second.target());
    }

    #[cfg(unix)]
    #[test]
    fn existing_directory_symlink_aliases_share_one_mutation_lock() {
        use std::os::unix::fs::symlink;

        let fixture = tempdir().expect("fixture");
        let real_parent = fixture.path().join("real");
        let alias_parent = fixture.path().join("alias");
        fs::create_dir(&real_parent).expect("real parent");
        symlink(&real_parent, &alias_parent).expect("directory symlink");
        let target = real_parent.join("runtime-home");
        let alias = alias_parent.join("runtime-home");

        let _first = acquired(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        assert!(matches!(
            acquire(&alias, RuntimeHomeMutationLeaseMode::SharedWriter),
            RuntimeHomeMutationLeaseOutcome::Busy(_)
        ));
    }

    #[test]
    fn distinct_runtime_homes_have_independent_mutation_locks() {
        let fixture = tempdir().expect("fixture");
        let first_target = fixture.path().join("runtime-home-a");
        let second_target = fixture.path().join("runtime-home-b");

        let first = acquired(&first_target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        let second = acquired(&second_target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        assert_ne!(first.lock_identity(), second.lock_identity());
    }

    #[test]
    fn coordination_file_persists_without_owning_a_later_lease() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let (directory, identity) = {
            let lease = acquired(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
            (
                mutation_coordination_directory().expect("coordination directory"),
                lease.lock_identity().clone(),
            )
        };
        let path = coordination_file_path(&directory, &identity);
        assert!(path.is_file());
        assert!(matches!(
            acquire(&target, RuntimeHomeMutationLeaseMode::SharedWriter),
            RuntimeHomeMutationLeaseOutcome::Acquired(_)
        ));
        assert!(path.is_file());
    }

    #[test]
    fn bounded_wait_succeeds_after_conflicting_lease_release() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let holder = acquired(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        let (busy_tx, busy_rx) = mpsc::channel();

        let outcome = std::thread::scope(|scope| {
            let waiter_target = target.clone();
            let waiter = scope.spawn(move || {
                let mut observed_busy = false;
                RuntimeHomeMutationLease::acquire_with_busy_observer(
                    &waiter_target,
                    RuntimeHomeMutationLeaseMode::SharedWriter,
                    RuntimeHomeMutationWaitPolicy::Bounded {
                        timeout: Duration::from_secs(2),
                    },
                    || {
                        if !observed_busy {
                            observed_busy = true;
                            busy_tx.send(()).expect("signal observed busy");
                        }
                    },
                )
                .expect("bounded mutation lease acquisition")
            });
            busy_rx.recv().expect("waiter observed conflicting lease");
            drop(holder);
            waiter.join().expect("bounded waiter panicked")
        });

        assert!(matches!(
            outcome,
            RuntimeHomeMutationLeaseOutcome::Acquired(_)
        ));
    }

    #[test]
    fn bounded_wait_returns_busy_after_deadline() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let _holder = acquired(&target, RuntimeHomeMutationLeaseMode::ExclusiveSetup);
        let wait_policy = RuntimeHomeMutationWaitPolicy::Bounded {
            timeout: Duration::from_millis(25),
        };

        let RuntimeHomeMutationLeaseOutcome::Busy(busy) = RuntimeHomeMutationLease::acquire(
            &target,
            RuntimeHomeMutationLeaseMode::SharedWriter,
            wait_policy,
        )
        .expect("bounded mutation lease acquisition") else {
            panic!("bounded acquisition should report busy");
        };

        assert_eq!(
            busy.requested_mode(),
            RuntimeHomeMutationLeaseMode::SharedWriter
        );
        assert_eq!(busy.wait_policy(), wait_policy);
        assert!(busy.elapsed() >= Duration::from_millis(25));
    }

    #[test]
    fn permit_borrows_the_exact_live_lease() {
        let fixture = tempdir().expect("fixture");
        let target = fixture.path().join("runtime-home");
        let lease = acquired(&target, RuntimeHomeMutationLeaseMode::SharedWriter);
        let permit = lease.permit();

        assert_eq!(permit.target(), lease.target());
        assert_eq!(permit.mode(), lease.mode());
    }

    #[cfg(windows)]
    #[test]
    fn windows_case_aliases_share_identity_without_collapsing_non_bmp_names() {
        let upper = CanonicalRuntimeHomePath(PathBuf::from(r"C:\Users\Example\Runtime"));
        let lower = CanonicalRuntimeHomePath(PathBuf::from(r"c:\users\example\runtime"));
        assert_eq!(
            runtime_home_mutation_lock_identity(&upper),
            runtime_home_mutation_lock_identity(&lower)
        );

        let first = CanonicalRuntimeHomePath(PathBuf::from(r"C:\Runtime\😀"));
        let second = CanonicalRuntimeHomePath(PathBuf::from(r"C:\Runtime\😁"));
        assert_ne!(
            runtime_home_mutation_lock_identity(&first),
            runtime_home_mutation_lock_identity(&second)
        );
    }
}
