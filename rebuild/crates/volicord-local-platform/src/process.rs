use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsFd, unix::process::CommandExt},
    path::{Path, PathBuf},
    process::{ChildStdin, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;

#[cfg(target_os = "linux")]
use rustix::{
    fs::{fcntl_getfl, fcntl_setfl, OFlags},
    io::Errno,
    process::{kill_process_group, Pid, Signal},
};

const POLL_INTERVAL: Duration = Duration::from_millis(2);
const READ_CHUNK: usize = 16 * 1024;
const MAX_STDIN_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct CancellationFlag(Arc<AtomicBool>);

impl CancellationFlag {
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStopTrigger {
    Timeout,
    Cancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTermination {
    ExitCode(i32),
    Signal(i32),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessTreeCleanup {
    NotRequired,
    Confirmed,
    Incomplete { detail: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStreamCompleteness {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessStreamArtifact {
    path: PathBuf,
    bytes: u64,
    completeness: ProcessStreamCompleteness,
}

impl ProcessStreamArtifact {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    pub const fn completeness(&self) -> ProcessStreamCompleteness {
        self.completeness
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessCompletion {
    Exited(ProcessTermination),
    ObservationFailed {
        termination: ProcessTermination,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessObservation {
    completion: ProcessCompletion,
    stop_trigger: Option<ProcessStopTrigger>,
    cleanup: ProcessTreeCleanup,
    stdout: ProcessStreamArtifact,
    stderr: ProcessStreamArtifact,
    duration: Duration,
}

impl ProcessObservation {
    pub fn completion(&self) -> &ProcessCompletion {
        &self.completion
    }

    pub const fn stop_trigger(&self) -> Option<ProcessStopTrigger> {
        self.stop_trigger
    }

    pub fn cleanup(&self) -> &ProcessTreeCleanup {
        &self.cleanup
    }

    pub const fn stdout(&self) -> &ProcessStreamArtifact {
        &self.stdout
    }

    pub const fn stderr(&self) -> &ProcessStreamArtifact {
        &self.stderr
    }

    pub const fn duration(&self) -> Duration {
        self.duration
    }

    pub fn succeeded(&self) -> bool {
        self.stop_trigger.is_none()
            && matches!(
                self.completion,
                ProcessCompletion::Exited(ProcessTermination::ExitCode(0))
            )
            && self.stdout.completeness == ProcessStreamCompleteness::Complete
            && self.stderr.completeness == ProcessStreamCompleteness::Complete
            && !matches!(self.cleanup, ProcessTreeCleanup::Incomplete { .. })
    }
}

#[derive(Debug)]
pub struct ProcessStartError {
    detail: String,
    duration: Duration,
}

impl ProcessStartError {
    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub const fn duration(&self) -> Duration {
        self.duration
    }
}

impl std::fmt::Display for ProcessStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for ProcessStartError {}

pub struct ProcessRequest {
    program: OsString,
    arguments: Vec<OsString>,
    current_dir: Option<PathBuf>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    timeout: Duration,
    cleanup_timeout: Duration,
    cancellation: CancellationFlag,
    stdin: Option<Vec<u8>>,
}

impl ProcessRequest {
    pub fn new(
        program: impl AsRef<OsStr>,
        stdout_path: impl Into<PathBuf>,
        stderr_path: impl Into<PathBuf>,
        timeout: Duration,
        cleanup_timeout: Duration,
    ) -> Self {
        Self {
            program: program.as_ref().to_owned(),
            arguments: Vec::new(),
            current_dir: None,
            stdout_path: stdout_path.into(),
            stderr_path: stderr_path.into(),
            timeout,
            cleanup_timeout,
            cancellation: CancellationFlag::default(),
            stdin: None,
        }
    }

    pub fn arg(mut self, argument: impl AsRef<OsStr>) -> Self {
        self.arguments.push(argument.as_ref().to_owned());
        self
    }

    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.arguments
            .extend(arguments.into_iter().map(|value| value.as_ref().to_owned()));
        self
    }

    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(path.into());
        self
    }

    pub fn cancellation(mut self, cancellation: CancellationFlag) -> Self {
        self.cancellation = cancellation;
        self
    }

    /// Supplies bounded in-memory stdin without placing request content in the
    /// child argv or a maintained process artifact.
    pub fn stdin_bytes(mut self, stdin: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    pub fn run(self) -> Result<ProcessObservation, ProcessStartError> {
        run_request(self)
    }
}

fn start_error(started: Instant, detail: impl Into<String>) -> ProcessStartError {
    ProcessStartError {
        detail: detail.into(),
        duration: started.elapsed(),
    }
}

fn create_artifact(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(target_os = "linux")]
    options.mode(0o600);
    options.open(path)
}

#[cfg(not(target_os = "linux"))]
fn run_request(_request: ProcessRequest) -> Result<ProcessObservation, ProcessStartError> {
    Err(ProcessStartError {
        detail: "local process observation is supported only on Linux".to_owned(),
        duration: Duration::ZERO,
    })
}

#[cfg(target_os = "linux")]
fn run_request(request: ProcessRequest) -> Result<ProcessObservation, ProcessStartError> {
    let started = Instant::now();
    if request
        .stdin
        .as_ref()
        .is_some_and(|input| input.len() > MAX_STDIN_BYTES)
    {
        return Err(start_error(
            started,
            format!("stdin exceeds the bounded {MAX_STDIN_BYTES} byte limit"),
        ));
    }
    if request.stdout_path == request.stderr_path {
        return Err(start_error(
            started,
            "stdout and stderr artifact paths must differ",
        ));
    }
    let mut stdout_file = create_artifact(&request.stdout_path)
        .map_err(|error| start_error(started, format!("cannot create stdout artifact: {error}")))?;
    let mut stderr_file = create_artifact(&request.stderr_path)
        .map_err(|error| start_error(started, format!("cannot create stderr artifact: {error}")))?;
    let mut command = Command::new(&request.program);
    command
        .args(&request.arguments)
        .stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = &request.current_dir {
        command.current_dir(directory);
    }
    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| start_error(started, format!("cannot spawn child process: {error}")))?;
    let stdin_writer = match request.stdin {
        Some(input) => {
            let Some(stdin) = child.stdin.take() else {
                abort_started_child(&mut child, None);
                return Err(start_error(started, "child stdin pipe is unavailable"));
            };
            Some(spawn_stdin_writer(stdin, input))
        }
        None => None,
    };
    let raw_pid = match i32::try_from(child.id()) {
        Ok(pid) => pid,
        Err(_) => {
            abort_started_child(&mut child, None);
            return Err(start_error(
                started,
                "child process ID is outside the Linux PID range",
            ));
        }
    };
    let pid = match Pid::from_raw(raw_pid) {
        Some(pid) => pid,
        None => {
            abort_started_child(&mut child, None);
            return Err(start_error(started, "child process ID is unavailable"));
        }
    };
    let mut stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            abort_started_child(&mut child, Some(pid));
            return Err(start_error(started, "child stdout pipe is unavailable"));
        }
    };
    let mut stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            abort_started_child(&mut child, Some(pid));
            return Err(start_error(started, "child stderr pipe is unavailable"));
        }
    };
    if let Err(error) = configure_nonblocking(&stdout) {
        abort_started_child(&mut child, Some(pid));
        return Err(start_error(
            started,
            format!("cannot configure stdout pipe: {error}"),
        ));
    }
    if let Err(error) = configure_nonblocking(&stderr) {
        abort_started_child(&mut child, Some(pid));
        return Err(start_error(
            started,
            format!("cannot configure stderr pipe: {error}"),
        ));
    }

    let lifecycle_deadline = started.checked_add(request.timeout).unwrap_or(started);
    let mut cleanup_deadline = None;
    let mut stop_trigger = None;
    let mut status = None;
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut stdout_bytes = 0_u64;
    let mut stderr_bytes = 0_u64;
    let mut cleanup_requested = false;
    let mut cleanup_issue = None;
    let mut observation_issue = None;

    loop {
        if let Err(error) = drain_pipe(
            &mut stdout,
            &mut stdout_file,
            &mut stdout_bytes,
            &mut stdout_open,
        ) {
            observation_issue.get_or_insert_with(|| format!("stdout observation failed: {error}"));
        }
        if let Err(error) = drain_pipe(
            &mut stderr,
            &mut stderr_file,
            &mut stderr_bytes,
            &mut stderr_open,
        ) {
            observation_issue.get_or_insert_with(|| format!("stderr observation failed: {error}"));
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(value) => status = value,
                Err(error) => {
                    observation_issue
                        .get_or_insert_with(|| format!("child status observation failed: {error}"));
                }
            };
        }

        if stop_trigger.is_none() && status.is_none() {
            let trigger = if request.cancellation.is_requested() {
                Some(ProcessStopTrigger::Cancellation)
            } else if Instant::now() >= lifecycle_deadline {
                Some(ProcessStopTrigger::Timeout)
            } else {
                None
            };
            if let Some(trigger) = trigger {
                stop_trigger = Some(trigger);
                cleanup_deadline = Some(
                    Instant::now()
                        .checked_add(request.cleanup_timeout)
                        .unwrap_or_else(Instant::now),
                );
            }
        }
        if (stop_trigger.is_some() || status.is_some()) && !cleanup_requested {
            cleanup_requested = true;
            cleanup_deadline.get_or_insert_with(|| {
                Instant::now()
                    .checked_add(request.cleanup_timeout)
                    .unwrap_or_else(Instant::now)
            });
            match kill_process_group(pid, Signal::KILL) {
                Ok(()) | Err(Errno::SRCH) => {}
                Err(error) => {
                    cleanup_issue = Some(format!("process-group termination failed: {error}"))
                }
            }
        }
        if status.is_some() && !stdout_open && !stderr_open {
            break;
        }
        if let Some(deadline) = cleanup_deadline {
            if Instant::now() >= deadline {
                break;
            }
        }
        thread::sleep(POLL_INTERVAL);
    }

    if status.is_none() {
        status = child.try_wait().ok().flatten();
    }
    if let Some(writer) = stdin_writer {
        match writer.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) if stop_trigger.is_some() => {
                observation_issue.get_or_insert_with(|| {
                    format!("stdin delivery ended during forced process stop: {error}")
                });
            }
            Ok(Err(error)) => {
                observation_issue.get_or_insert_with(|| format!("stdin delivery failed: {error}"));
            }
            Err(_) => {
                observation_issue.get_or_insert_with(|| "stdin writer panicked".to_owned());
            }
        }
    }
    let group_absent = matches!(kill_process_group(pid, Signal::KILL), Err(Errno::SRCH));
    for (name, file) in [("stdout", &mut stdout_file), ("stderr", &mut stderr_file)] {
        if let Err(error) = file.flush().and_then(|()| file.sync_all()) {
            observation_issue
                .get_or_insert_with(|| format!("{name} artifact synchronization failed: {error}"));
        }
    }
    let complete = status.is_some() && !stdout_open && !stderr_open && observation_issue.is_none();
    let completeness = if complete {
        ProcessStreamCompleteness::Complete
    } else {
        ProcessStreamCompleteness::Incomplete
    };
    let termination = status
        .map(termination)
        .unwrap_or(ProcessTermination::Unknown);
    let cleanup = if cleanup_issue.is_some() || !group_absent || status.is_none() {
        ProcessTreeCleanup::Incomplete {
            detail: cleanup_issue.unwrap_or_else(|| {
                "process-tree termination was not confirmed before the cleanup deadline".to_owned()
            }),
        }
    } else if cleanup_requested
        && (stop_trigger.is_some() || !matches!(termination, ProcessTermination::ExitCode(_)))
    {
        ProcessTreeCleanup::Confirmed
    } else {
        ProcessTreeCleanup::NotRequired
    };
    let completion = match observation_issue {
        Some(detail) => ProcessCompletion::ObservationFailed {
            termination,
            detail,
        },
        None => ProcessCompletion::Exited(termination),
    };
    Ok(ProcessObservation {
        completion,
        stop_trigger,
        cleanup,
        stdout: ProcessStreamArtifact {
            path: request.stdout_path,
            bytes: stdout_bytes,
            completeness,
        },
        stderr: ProcessStreamArtifact {
            path: request.stderr_path,
            bytes: stderr_bytes,
            completeness,
        },
        duration: started.elapsed(),
    })
}

#[cfg(target_os = "linux")]
fn spawn_stdin_writer(mut stdin: ChildStdin, input: Vec<u8>) -> thread::JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        stdin.write_all(&input)?;
        stdin.flush()
    })
}

#[cfg(target_os = "linux")]
fn abort_started_child(child: &mut std::process::Child, pid: Option<Pid>) {
    if let Some(pid) = pid {
        let _ = kill_process_group(pid, Signal::KILL);
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(target_os = "linux")]
fn configure_nonblocking(pipe: impl AsFd) -> io::Result<()> {
    let flags = fcntl_getfl(&pipe).map_err(io::Error::from)?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK).map_err(io::Error::from)
}

fn drain_pipe<R: Read>(
    reader: &mut R,
    file: &mut File,
    bytes: &mut u64,
    open: &mut bool,
) -> io::Result<()> {
    if !*open {
        return Ok(());
    }
    let mut buffer = [0_u8; READ_CHUNK];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                *open = false;
                return Ok(());
            }
            Ok(count) => {
                file.write_all(&buffer[..count])?;
                *bytes = bytes.saturating_add(count as u64);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => {
                *open = false;
                return Err(error);
            }
        }
    }
}

fn termination(status: ExitStatus) -> ProcessTermination {
    use std::os::unix::process::ExitStatusExt;
    if let Some(code) = status.code() {
        ProcessTermination::ExitCode(code)
    } else if let Some(signal) = status.signal() {
        ProcessTermination::Signal(signal)
    } else {
        ProcessTermination::Unknown
    }
}
