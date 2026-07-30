use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use volicord_platform_process::ProcessContainment;

use super::model::{ContentIdentity, ObservationUnavailable, ObservationUnavailableReason};

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[cfg(test)]
thread_local! {
    static TEST_GIT_GLOBAL_CONFIG: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_test_git_global_config(path: &Path) {
    TEST_GIT_GLOBAL_CONFIG.with(|current| {
        *current.borrow_mut() = Some(path.to_path_buf());
    });
}

/// Explicit resource limits for one repository observer contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverLimits {
    max_git_output_bytes: usize,
    max_process_input_bytes: usize,
    max_candidate_paths: usize,
    max_total_hashed_bytes: u64,
    max_file_bytes: u64,
    max_process_duration: Duration,
    max_serialized_bytes: usize,
    max_serialization_depth: usize,
    max_stability_attempts: usize,
}

impl Default for ObserverLimits {
    fn default() -> Self {
        Self {
            max_git_output_bytes: 512 * 1024 * 1024,
            max_process_input_bytes: 1024 * 1024,
            max_candidate_paths: 8_192,
            max_total_hashed_bytes: 2 * 1024 * 1024 * 1024,
            max_file_bytes: 512 * 1024 * 1024,
            max_process_duration: Duration::from_secs(10),
            max_serialized_bytes: 8 * 1024 * 1024,
            max_serialization_depth: 8,
            max_stability_attempts: 2,
        }
    }
}

impl ObserverLimits {
    /// Returns a copy with a different maximum combined Git stdout/stderr size.
    pub fn with_max_git_output_bytes(mut self, value: usize) -> Self {
        self.max_git_output_bytes = value;
        self
    }

    /// Returns a copy with a different maximum encoded Git argument size.
    pub fn with_max_process_input_bytes(mut self, value: usize) -> Self {
        self.max_process_input_bytes = value;
        self
    }

    /// Returns a copy with a different semantic candidate-path limit.
    pub fn with_max_candidate_paths(mut self, value: usize) -> Self {
        self.max_candidate_paths = value;
        self
    }

    /// Returns a copy with a different aggregate hashing limit.
    pub fn with_max_total_hashed_bytes(mut self, value: u64) -> Self {
        self.max_total_hashed_bytes = value;
        self
    }

    /// Returns a copy with a different per-file hashing limit.
    pub fn with_max_file_bytes(mut self, value: u64) -> Self {
        self.max_file_bytes = value;
        self
    }

    /// Returns a copy with a different per-process duration limit.
    pub fn with_max_process_duration(mut self, value: Duration) -> Self {
        self.max_process_duration = value;
        self
    }

    /// Returns a copy with a different canonical serialization size limit.
    pub fn with_max_serialized_bytes(mut self, value: usize) -> Self {
        self.max_serialized_bytes = value;
        self
    }

    /// Returns a copy with a different canonical serialization depth limit.
    pub fn with_max_serialization_depth(mut self, value: usize) -> Self {
        self.max_serialization_depth = value;
        self
    }

    /// Returns a copy with a different snapshot stabilization-attempt limit.
    pub fn with_max_stability_attempts(mut self, value: usize) -> Self {
        self.max_stability_attempts = value;
        self
    }

    /// Maximum combined stdout/stderr bytes accepted from one Git process.
    pub const fn max_git_output_bytes(&self) -> usize {
        self.max_git_output_bytes
    }

    /// Maximum encoded argument bytes accepted for one Git process.
    pub const fn max_process_input_bytes(&self) -> usize {
        self.max_process_input_bytes
    }

    /// Maximum paths in one semantic candidate union.
    pub const fn max_candidate_paths(&self) -> usize {
        self.max_candidate_paths
    }

    /// Maximum aggregate bytes hashed during one snapshot or delta operation.
    pub const fn max_total_hashed_bytes(&self) -> u64 {
        self.max_total_hashed_bytes
    }

    /// Maximum bytes hashed for one regular file or symbolic-link target.
    pub const fn max_file_bytes(&self) -> u64 {
        self.max_file_bytes
    }

    /// Maximum duration of one Git child process.
    pub const fn max_process_duration(&self) -> Duration {
        self.max_process_duration
    }

    /// Maximum canonical snapshot, delta, or typed-input serialization size.
    pub const fn max_serialized_bytes(&self) -> usize {
        self.max_serialized_bytes
    }

    /// Maximum semantic serialization depth.
    pub const fn max_serialization_depth(&self) -> usize {
        self.max_serialization_depth
    }

    /// Maximum attempts used to establish a stable snapshot.
    pub const fn max_stability_attempts(&self) -> usize {
        self.max_stability_attempts
    }

    pub(crate) fn validate(&self) -> Result<(), ObservationUnavailable> {
        if self.max_git_output_bytes == 0
            || self.max_process_input_bytes == 0
            || self.max_candidate_paths == 0
            || self.max_total_hashed_bytes == 0
            || self.max_file_bytes == 0
            || self.max_process_duration.is_zero()
            || self.max_serialized_bytes == 0
            || self.max_serialization_depth == 0
            || self.max_stability_attempts == 0
        {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::InvalidObserverLimits,
                "observer limits must all be non-zero",
            ));
        }
        Ok(())
    }
}

pub(crate) struct GitOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

pub(crate) struct GitFileHashOutput {
    pub(crate) output: GitOutput,
    pub(crate) exact_worktree_bytes: ContentIdentity,
    pub(crate) source_bytes: u64,
}

pub(crate) fn git_arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

pub(crate) fn run_git(
    repository_root: &Path,
    arguments: &[OsString],
    limits: &ObserverLimits,
) -> Result<GitOutput, ObservationUnavailable> {
    ensure_process_input(arguments, limits)?;
    let mut command = git_command(repository_root);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (mut child, containment) = spawn_contained_git(command)?;
    let stdout = child.stdout.take().ok_or_else(|| {
        terminate_and_reap(&containment, &mut child);
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            "Git stdout was not captured",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        terminate_and_reap(&containment, &mut child);
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            "Git stderr was not captured",
        )
    })?;

    let observed_bytes = Arc::new(AtomicUsize::new(0));
    let output_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_bounded_reader(
        stdout,
        Arc::clone(&observed_bytes),
        Arc::clone(&output_exceeded),
        limits.max_git_output_bytes,
    );
    let stderr_reader = spawn_bounded_reader(
        stderr,
        observed_bytes,
        Arc::clone(&output_exceeded),
        limits.max_git_output_bytes,
    );
    let status = wait_for_child(
        &mut child,
        &containment,
        limits.max_process_duration,
        &output_exceeded,
    )?;
    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;
    if output_exceeded.load(Ordering::Acquire) {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::GitOutputLimitExceeded,
            "Git output exceeded its configured byte limit",
        ));
    }
    Ok(GitOutput {
        status,
        stdout,
        stderr,
    })
}

pub(crate) fn require_git_success(
    output: GitOutput,
    operation: &'static str,
) -> Result<Vec<u8>, ObservationUnavailable> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(ObservationUnavailable::new(
        ObservationUnavailableReason::GitCommandFailed,
        format!("{operation} failed: {}", stderr.trim()),
    ))
}

pub(crate) fn run_git_with_file_stdin(
    repository_root: &Path,
    arguments: &[OsString],
    file: File,
    remaining_total_hash_bytes: u64,
    limits: &ObserverLimits,
) -> Result<GitFileHashOutput, ObservationUnavailable> {
    ensure_process_input(arguments, limits)?;
    let mut command = git_command(repository_root);
    command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (mut child, containment) = spawn_contained_git(command)?;
    let stdin = child.stdin.take().ok_or_else(|| {
        terminate_and_reap(&containment, &mut child);
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            "Git stdin was not captured",
        )
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        terminate_and_reap(&containment, &mut child);
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            "Git stdout was not captured",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        terminate_and_reap(&containment, &mut child);
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            "Git stderr was not captured",
        )
    })?;

    let observed_bytes = Arc::new(AtomicUsize::new(0));
    let stop_requested = Arc::new(AtomicBool::new(false));
    let limit_reason = Arc::new(AtomicU8::new(0));
    let stdin_writer = spawn_file_writer(
        file,
        stdin,
        Arc::clone(&stop_requested),
        Arc::clone(&limit_reason),
        remaining_total_hash_bytes,
        limits.max_file_bytes,
    );
    let stdout_reader = spawn_bounded_reader(
        stdout,
        Arc::clone(&observed_bytes),
        Arc::clone(&stop_requested),
        limits.max_git_output_bytes,
    );
    let stderr_reader = spawn_bounded_reader(
        stderr,
        Arc::clone(&observed_bytes),
        Arc::clone(&stop_requested),
        limits.max_git_output_bytes,
    );
    let status_result = wait_for_child(
        &mut child,
        &containment,
        limits.max_process_duration,
        &stop_requested,
    );
    let streamed = stdin_writer.join().map_err(|_| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            "the Git input writer terminated unexpectedly",
        )
    })?;
    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;
    let status = status_result?;
    match limit_reason.load(Ordering::Acquire) {
        1 => {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::FileSizeLimitExceeded,
                "a Product Repository file exceeds the configured per-file byte limit",
            ));
        }
        2 => {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::TotalHashBytesLimitExceeded,
                "repository hashing exceeds the configured aggregate byte limit",
            ));
        }
        _ => {}
    }
    if observed_bytes.load(Ordering::Acquire) > limits.max_git_output_bytes {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::GitOutputLimitExceeded,
            "Git output exceeded its configured byte limit",
        ));
    }
    let (exact_worktree_bytes, source_bytes) = match streamed {
        Ok(value) => value,
        Err(FileStreamError::Read(error)) => {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::InaccessiblePath,
                format!("Product Repository regular-file content observation failed: {error}"),
            ));
        }
        Err(FileStreamError::Write(error)) => {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::GitCommandFailed,
                format!("Git canonical-content input failed: {error}"),
            ));
        }
    };
    if !status.success() {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandFailed,
            format!(
                "Git canonical-content conversion failed: {}",
                String::from_utf8_lossy(&stderr).trim()
            ),
        ));
    }
    Ok(GitFileHashOutput {
        output: GitOutput {
            status,
            stdout,
            stderr,
        },
        exact_worktree_bytes,
        source_bytes,
    })
}

pub(crate) fn git_command(repository_root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("--literal-pathspecs")
        .arg("-C")
        .arg(repository_root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_CEILING_DIRECTORIES")
        .env_remove("GIT_DISCOVERY_ACROSS_FILESYSTEM")
        .env_remove("GIT_PREFIX")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_CONFIG_COUNT")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("LC_ALL", "C");
    #[cfg(test)]
    TEST_GIT_GLOBAL_CONFIG.with(|path| {
        if let Some(path) = path.borrow().as_ref() {
            command
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", path);
        }
    });
    command
}

pub(crate) fn ensure_process_input(
    arguments: &[OsString],
    limits: &ObserverLimits,
) -> Result<(), ObservationUnavailable> {
    let encoded_bytes = arguments.iter().try_fold(0usize, |total, argument| {
        os_str_bytes(argument)
            .len()
            .checked_add(1)
            .and_then(|length| total.checked_add(length))
    });
    if encoded_bytes.is_none_or(|bytes| bytes > limits.max_process_input_bytes) {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::ProcessInputLimitExceeded,
            "Git process input exceeds its configured byte limit",
        ));
    }
    Ok(())
}

fn wait_for_child(
    child: &mut std::process::Child,
    containment: &ProcessContainment,
    duration: Duration,
    stop_requested: &AtomicBool,
) -> Result<ExitStatus, ObservationUnavailable> {
    let deadline = Instant::now() + duration;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                containment.terminate_tree().map_err(containment_error)?;
                return Ok(status);
            }
            Ok(None) => {}
            Err(error) => {
                terminate_and_reap(containment, child);
                return Err(git_wait_error(error));
            }
        }
        if stop_requested.load(Ordering::Acquire) {
            return terminate_tree_and_reap(containment, child);
        }
        if Instant::now() >= deadline {
            terminate_tree_and_reap(containment, child)?;
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::ProcessTimeout,
                "Git exceeded its configured process duration",
            ));
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn terminate_tree_and_reap(
    containment: &ProcessContainment,
    child: &mut std::process::Child,
) -> Result<ExitStatus, ObservationUnavailable> {
    let containment_result = containment.terminate_tree();
    if containment_result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(git_wait_error)?;
    containment_result.map_err(containment_error)?;
    Ok(status)
}

fn spawn_bounded_reader(
    mut reader: impl Read + Send + 'static,
    observed_bytes: Arc<AtomicUsize>,
    output_exceeded: Arc<AtomicBool>,
    max_bytes: usize,
) -> thread::JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(output);
            }
            let previous = observed_bytes.fetch_add(read, Ordering::AcqRel);
            if previous.saturating_add(read) > max_bytes {
                output_exceeded.store(true, Ordering::Release);
                return Ok(output);
            }
            output.extend_from_slice(&buffer[..read]);
        }
    })
}

fn spawn_file_writer(
    mut file: File,
    mut stdin: std::process::ChildStdin,
    stop_requested: Arc<AtomicBool>,
    limit_reason: Arc<AtomicU8>,
    remaining_total_hash_bytes: u64,
    max_file_bytes: u64,
) -> thread::JoinHandle<Result<(ContentIdentity, u64), FileStreamError>> {
    thread::spawn(move || {
        let mut digest = Sha256::new();
        let mut hashed_bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer).map_err(FileStreamError::Read)?;
            if read == 0 {
                return Ok((
                    ContentIdentity::from_digest(digest.finalize()),
                    hashed_bytes,
                ));
            }
            hashed_bytes = hashed_bytes.saturating_add(read as u64);
            if hashed_bytes > max_file_bytes {
                set_limit_reason(&limit_reason, 1);
                stop_requested.store(true, Ordering::Release);
                return Ok((
                    ContentIdentity::from_digest(digest.finalize()),
                    hashed_bytes,
                ));
            }
            if hashed_bytes > remaining_total_hash_bytes {
                set_limit_reason(&limit_reason, 2);
                stop_requested.store(true, Ordering::Release);
                return Ok((
                    ContentIdentity::from_digest(digest.finalize()),
                    hashed_bytes,
                ));
            }
            digest.update(&buffer[..read]);
            stdin
                .write_all(&buffer[..read])
                .map_err(FileStreamError::Write)?;
        }
    })
}

enum FileStreamError {
    Read(io::Error),
    Write(io::Error),
}

fn set_limit_reason(reason: &AtomicU8, value: u8) {
    let _ = reason.compare_exchange(0, value, Ordering::AcqRel, Ordering::Acquire);
}

fn join_reader(
    reader: thread::JoinHandle<io::Result<Vec<u8>>>,
) -> Result<Vec<u8>, ObservationUnavailable> {
    reader
        .join()
        .map_err(|_| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::GitCommandFailed,
                "a Git output reader terminated unexpectedly",
            )
        })?
        .map_err(|error| {
            ObservationUnavailable::new(
                ObservationUnavailableReason::GitCommandFailed,
                format!("Git output could not be read: {error}"),
            )
        })
}

fn spawn_contained_git(
    mut command: Command,
) -> Result<(std::process::Child, ProcessContainment), ObservationUnavailable> {
    let mut containment = ProcessContainment::new().map_err(containment_error)?;
    containment.configure_command(&mut command);
    let mut child = command.spawn().map_err(|error| {
        ObservationUnavailable::new(
            ObservationUnavailableReason::GitCommandUnavailable,
            format!("Git could not be started: {error}"),
        )
    })?;
    if let Err(error) = containment.attach(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(containment_error(error));
    }
    Ok((child, containment))
}

fn terminate_and_reap(containment: &ProcessContainment, child: &mut std::process::Child) {
    let _ = containment.terminate_tree();
    let _ = child.kill();
    let _ = child.wait();
}

fn containment_error(
    error: volicord_platform_process::PlatformProcessError,
) -> ObservationUnavailable {
    ObservationUnavailable::new(
        ObservationUnavailableReason::GitCommandFailed,
        format!("Git process containment failed: {error}"),
    )
}

fn git_wait_error(error: io::Error) -> ObservationUnavailable {
    ObservationUnavailable::new(
        ObservationUnavailableReason::GitCommandFailed,
        format!("Git process state could not be observed: {error}"),
    )
}

#[cfg(unix)]
fn os_str_bytes(value: &OsStr) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes()
}

#[cfg(windows)]
fn os_str_bytes(value: &OsStr) -> &[u8] {
    value.to_str().unwrap_or_default().as_bytes()
}
