//! Bounded child-process execution for repository tests and smoke harnesses.

#![deny(unsafe_code)]

use std::{
    ffi::{OsStr, OsString},
    fmt,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use volicord_platform_process::{
    configure_child_stderr_pipe, configure_child_stdin_pipe, configure_child_stdout_pipe,
    read_child_stderr_available, read_child_stdout_available, PipeRead, ProcessContainment,
};

const PIPE_READ_CHUNK_BYTES: usize = 4 * 1024;
const STDIN_WRITE_CHUNK_BYTES: usize = 4 * 1024;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Caller-supplied lifecycle and forced-cleanup bounds for one process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessDeadline {
    lifecycle_timeout: Duration,
    cleanup_timeout: Duration,
}

impl ProcessDeadline {
    /// Creates one deadline policy with separate lifecycle and forced-cleanup bounds.
    pub const fn new(lifecycle_timeout: Duration, cleanup_timeout: Duration) -> Self {
        Self {
            lifecycle_timeout,
            cleanup_timeout,
        }
    }

    /// Returns the total allowed time before forced process-tree cleanup begins.
    pub const fn lifecycle_timeout(self) -> Duration {
        self.lifecycle_timeout
    }

    /// Returns the allowed time for process-tree termination, reaping, and pipe closure.
    pub const fn cleanup_timeout(self) -> Duration {
        self.cleanup_timeout
    }
}

/// One deterministically bounded byte capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCapture {
    bytes: Vec<u8>,
    omitted_bytes: usize,
    limit: usize,
}

impl BoundedCapture {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(8 * 1024)),
            omitted_bytes: 0,
            limit,
        }
    }

    /// Creates a deterministic capture from already observed bytes.
    pub fn from_bytes(bytes: impl AsRef<[u8]>, limit: usize) -> Self {
        let mut capture = Self::new(limit);
        capture.push(bytes.as_ref());
        capture
    }

    fn push(&mut self, bytes: &[u8]) {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        let retained = remaining.min(bytes.len());
        self.bytes.extend_from_slice(&bytes[..retained]);
        self.omitted_bytes = self
            .omitted_bytes
            .saturating_add(bytes.len().saturating_sub(retained));
    }

    fn take(&mut self) -> Self {
        let limit = self.limit;
        std::mem::replace(self, Self::new(limit))
    }

    /// Returns retained bytes in their original order.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the capture and returns retained bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the caller-supplied retention limit.
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// Returns the exact number of observed bytes omitted after the limit.
    pub const fn omitted_bytes(&self) -> usize {
        self.omitted_bytes
    }

    /// Reports whether any observed bytes were omitted.
    pub const fn is_truncated(&self) -> bool {
        self.omitted_bytes != 0
    }

    /// Renders retained bytes lossily and appends deterministic truncation context.
    pub fn render_lossy(&self) -> String {
        let mut rendered = String::from_utf8_lossy(&self.bytes).into_owned();
        if self.is_truncated() {
            rendered.push_str(&format!(
                "\n[{} additional byte(s) omitted]",
                self.omitted_bytes
            ));
        }
        rendered
    }
}

/// Complete output from one bounded child operation.
#[derive(Debug)]
pub struct BoundedProcessOutput {
    status: ExitStatus,
    stdout: BoundedCapture,
    stderr: BoundedCapture,
    elapsed: Duration,
}

impl BoundedProcessOutput {
    /// Returns the reaped direct-child exit status.
    pub const fn status(&self) -> ExitStatus {
        self.status
    }

    /// Returns bounded stdout.
    pub const fn stdout(&self) -> &BoundedCapture {
        &self.stdout
    }

    /// Returns bounded stderr.
    pub const fn stderr(&self) -> &BoundedCapture {
        &self.stderr
    }

    /// Returns elapsed wall time from containment creation through cleanup.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// A child pipe involved in a bounded-process failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPipe {
    /// The child stdin pipe.
    Stdin,
    /// The child stdout pipe.
    Stdout,
    /// The child stderr pipe.
    Stderr,
}

impl fmt::Display for ProcessPipe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Stdin => "stdin",
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        })
    }
}

/// Stable failure categories at the bounded test-process boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedProcessFailureKind {
    /// Platform containment could not be created, attached, or terminated.
    PlatformContainment,
    /// The executable could not be spawned.
    Spawn,
    /// A required child pipe was unavailable.
    PipeAcquisition,
    /// A required child pipe could not be made nonblocking.
    PipeConfiguration,
    /// A configured child pipe could not be read.
    PipeRead,
    /// Optional stdin bytes could not be delivered.
    StdinWrite,
    /// The one total lifecycle deadline elapsed.
    Timeout,
    /// The child exited unsuccessfully when success was required.
    UnsuccessfulExit,
    /// The direct child could not be polled or reaped.
    Reap,
    /// Forced cleanup did not finish within its caller-supplied bound.
    Cleanup,
}

/// One classified failure with bounded output and cleanup context.
#[derive(Debug)]
pub struct BoundedProcessFailure {
    kind: BoundedProcessFailureKind,
    pipe: Option<ProcessPipe>,
    detail: String,
    status: Option<ExitStatus>,
    stdout: Box<BoundedCapture>,
    stderr: Box<BoundedCapture>,
    elapsed: Duration,
    cleanup_detail: Option<String>,
}

impl BoundedProcessFailure {
    fn without_child(
        kind: BoundedProcessFailureKind,
        detail: impl Into<String>,
        started_at: Instant,
        stdout_limit: usize,
        stderr_limit: usize,
    ) -> Self {
        Self {
            kind,
            pipe: None,
            detail: detail.into(),
            status: None,
            stdout: Box::new(BoundedCapture::new(stdout_limit)),
            stderr: Box::new(BoundedCapture::new(stderr_limit)),
            elapsed: started_at.elapsed(),
            cleanup_detail: None,
        }
    }

    /// Returns the stable failure category.
    pub const fn kind(&self) -> BoundedProcessFailureKind {
        self.kind
    }

    /// Returns the pipe involved in a pipe-specific failure.
    pub const fn pipe(&self) -> Option<ProcessPipe> {
        self.pipe
    }

    /// Returns implementation-facing failure detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns the direct-child status when cleanup reaped it.
    pub const fn status(&self) -> Option<ExitStatus> {
        self.status
    }

    /// Returns stdout observed before failure and during bounded cleanup.
    pub const fn stdout(&self) -> &BoundedCapture {
        &self.stdout
    }

    /// Returns stderr observed before failure and during bounded cleanup.
    pub const fn stderr(&self) -> &BoundedCapture {
        &self.stderr
    }

    /// Returns elapsed wall time through the completed cleanup attempt.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns cleanup or reap detail when the primary failure also had cleanup trouble.
    pub fn cleanup_detail(&self) -> Option<&str> {
        self.cleanup_detail.as_deref()
    }
}

impl fmt::Display for BoundedProcessFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.detail)?;
        if let Some(status) = self.status {
            write!(formatter, "\nstatus: {status}")?;
        }
        write!(
            formatter,
            "\nstdout:\n{}\nstderr:\n{}",
            self.stdout.render_lossy(),
            self.stderr.render_lossy()
        )?;
        if let Some(cleanup_detail) = &self.cleanup_detail {
            write!(formatter, "\ncleanup: {cleanup_detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BoundedProcessFailure {}

enum EnvironmentChange {
    Set(OsString, OsString),
    Remove(OsString),
}

/// Safe command specification for one bounded repository test process.
pub struct BoundedCommand {
    program: OsString,
    arguments: Vec<OsString>,
    current_dir: Option<PathBuf>,
    environment: Vec<EnvironmentChange>,
    stdin: Option<Vec<u8>>,
    deadline: ProcessDeadline,
    stdout_limit: usize,
    stderr_limit: usize,
    require_success: bool,
}

impl BoundedCommand {
    /// Creates a command with all lifecycle and capture bounds explicit.
    pub fn new(
        program: impl AsRef<OsStr>,
        deadline: ProcessDeadline,
        stdout_limit: usize,
        stderr_limit: usize,
    ) -> Self {
        Self {
            program: program.as_ref().to_owned(),
            arguments: Vec::new(),
            current_dir: None,
            environment: Vec::new(),
            stdin: None,
            deadline,
            stdout_limit,
            stderr_limit,
            require_success: false,
        }
    }

    /// Appends one argument.
    pub fn arg(mut self, argument: impl AsRef<OsStr>) -> Self {
        self.arguments.push(argument.as_ref().to_owned());
        self
    }

    /// Appends multiple arguments.
    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.arguments.extend(
            arguments
                .into_iter()
                .map(|argument| argument.as_ref().to_owned()),
        );
        self
    }

    /// Sets the child working directory.
    pub fn current_dir(mut self, directory: impl AsRef<Path>) -> Self {
        self.current_dir = Some(directory.as_ref().to_owned());
        self
    }

    /// Adds or replaces one child environment value.
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.environment.push(EnvironmentChange::Set(
            key.as_ref().to_owned(),
            value.as_ref().to_owned(),
        ));
        self
    }

    /// Removes one inherited or previously added child environment value.
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.environment
            .push(EnvironmentChange::Remove(key.as_ref().to_owned()));
        self
    }

    /// Supplies the complete finite stdin payload.
    pub fn stdin(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(bytes.into());
        self
    }

    /// Selects whether a non-success status is returned as a classified failure.
    pub const fn require_success(mut self, required: bool) -> Self {
        self.require_success = required;
        self
    }

    /// Executes the child under one process-tree containment and polling supervisor.
    pub fn run(self) -> Result<BoundedProcessOutput, BoundedProcessFailure> {
        let started_at = Instant::now();
        let lifecycle_deadline = started_at
            .checked_add(self.deadline.lifecycle_timeout)
            .unwrap_or(started_at);
        let containment = ProcessContainment::new().map_err(|error| {
            BoundedProcessFailure::without_child(
                BoundedProcessFailureKind::PlatformContainment,
                format!("failed to create process containment: {error}"),
                started_at,
                self.stdout_limit,
                self.stderr_limit,
            )
        })?;
        let mut command = Command::new(&self.program);
        command.args(&self.arguments);
        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }
        for change in &self.environment {
            match change {
                EnvironmentChange::Set(key, value) => {
                    command.env(key, value);
                }
                EnvironmentChange::Remove(key) => {
                    command.env_remove(key);
                }
            }
        }
        if self.stdin.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        containment.configure_command(&mut command);

        let child = command.spawn().map_err(|error| {
            BoundedProcessFailure::without_child(
                BoundedProcessFailureKind::Spawn,
                format!("failed to spawn process: {error}"),
                started_at,
                self.stdout_limit,
                self.stderr_limit,
            )
        })?;
        let mut supervisor = ProcessSupervisor::new(
            child,
            containment,
            started_at,
            lifecycle_deadline,
            self.deadline,
            self.stdout_limit,
            self.stderr_limit,
        );
        if let Err(error) = supervisor.containment.attach(&supervisor.child) {
            return Err(supervisor.fail(
                BoundedProcessFailureKind::PlatformContainment,
                None,
                format!("failed to attach child to process containment: {error}"),
            ));
        }
        if let Err(pipe) = supervisor.acquire_pipes(self.stdin.is_some()) {
            return Err(supervisor.fail(
                BoundedProcessFailureKind::PipeAcquisition,
                Some(pipe),
                format!("child {pipe} pipe was unavailable"),
            ));
        }
        if let Err((pipe, detail)) = supervisor.configure_pipes() {
            return Err(supervisor.fail(
                BoundedProcessFailureKind::PipeConfiguration,
                Some(pipe),
                detail,
            ));
        }

        if let Some(stdin) = self.stdin {
            if let Err(failure) = supervisor.write_stdin(&stdin) {
                return Err(supervisor.fail(failure.kind, failure.pipe, failure.detail));
            }
        }
        supervisor.wait_for_completion(self.require_success)
    }
}

struct OperationFailure {
    kind: BoundedProcessFailureKind,
    pipe: Option<ProcessPipe>,
    detail: String,
}

struct CleanupReport {
    detail: Option<String>,
    containment_failed: bool,
    reap_failed: bool,
}

struct ProcessSupervisor {
    child: Child,
    containment: ProcessContainment,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    stdout_configured: bool,
    stderr_configured: bool,
    status: Option<ExitStatus>,
    stdout_capture: BoundedCapture,
    stderr_capture: BoundedCapture,
    started_at: Instant,
    lifecycle_deadline: Instant,
    deadline: ProcessDeadline,
    termination_attempted: bool,
    finalized: bool,
}

impl ProcessSupervisor {
    fn new(
        child: Child,
        containment: ProcessContainment,
        started_at: Instant,
        lifecycle_deadline: Instant,
        deadline: ProcessDeadline,
        stdout_limit: usize,
        stderr_limit: usize,
    ) -> Self {
        Self {
            child,
            containment,
            stdin: None,
            stdout: None,
            stderr: None,
            stdout_configured: false,
            stderr_configured: false,
            status: None,
            stdout_capture: BoundedCapture::new(stdout_limit),
            stderr_capture: BoundedCapture::new(stderr_limit),
            started_at,
            lifecycle_deadline,
            deadline,
            termination_attempted: false,
            finalized: false,
        }
    }

    fn acquire_pipes(&mut self, expect_stdin: bool) -> Result<(), ProcessPipe> {
        self.stdout = self.child.stdout.take();
        if self.stdout.is_none() {
            return Err(ProcessPipe::Stdout);
        }
        self.stderr = self.child.stderr.take();
        if self.stderr.is_none() {
            return Err(ProcessPipe::Stderr);
        }
        if expect_stdin {
            self.stdin = self.child.stdin.take();
            if self.stdin.is_none() {
                return Err(ProcessPipe::Stdin);
            }
        }
        Ok(())
    }

    fn configure_pipes(&mut self) -> Result<(), (ProcessPipe, String)> {
        let stdout = self.stdout.as_ref().expect("acquired stdout");
        configure_child_stdout_pipe(stdout).map_err(|error| {
            (
                ProcessPipe::Stdout,
                format!("failed to configure child stdout for bounded reads: {error}"),
            )
        })?;
        self.stdout_configured = true;

        let stderr = self.stderr.as_ref().expect("acquired stderr");
        configure_child_stderr_pipe(stderr).map_err(|error| {
            (
                ProcessPipe::Stderr,
                format!("failed to configure child stderr for bounded reads: {error}"),
            )
        })?;
        self.stderr_configured = true;

        if let Some(stdin) = &self.stdin {
            configure_child_stdin_pipe(stdin).map_err(|error| {
                (
                    ProcessPipe::Stdin,
                    format!("failed to configure child stdin for bounded writes: {error}"),
                )
            })?;
        }
        Ok(())
    }

    fn write_stdin(&mut self, bytes: &[u8]) -> Result<(), OperationFailure> {
        let mut written = 0;
        while written < bytes.len() {
            let progressed = self.pump_io()?;
            if let Some(status) = self.poll_status()? {
                return Err(OperationFailure {
                    kind: BoundedProcessFailureKind::StdinWrite,
                    pipe: Some(ProcessPipe::Stdin),
                    detail: format!(
                        "child exited with status {status} before all stdin bytes were written"
                    ),
                });
            }
            if Instant::now() >= self.lifecycle_deadline {
                return Err(self.timeout_failure());
            }
            let Some(stdin) = self.stdin.as_mut() else {
                return Err(OperationFailure {
                    kind: BoundedProcessFailureKind::StdinWrite,
                    pipe: Some(ProcessPipe::Stdin),
                    detail: "child stdin pipe closed before all bytes were written".to_owned(),
                });
            };
            let end = written
                .saturating_add(STDIN_WRITE_CHUNK_BYTES)
                .min(bytes.len());
            match stdin.write(&bytes[written..end]) {
                Ok(0) => {
                    if !progressed {
                        self.sleep_until(self.lifecycle_deadline);
                    }
                }
                Ok(count) => written += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if !progressed {
                        self.sleep_until(self.lifecycle_deadline);
                    }
                }
                Err(error) => {
                    return Err(OperationFailure {
                        kind: BoundedProcessFailureKind::StdinWrite,
                        pipe: Some(ProcessPipe::Stdin),
                        detail: format!("failed to write child stdin: {error}"),
                    })
                }
            }
        }
        self.stdin.take();
        Ok(())
    }

    fn wait_for_completion(
        mut self,
        require_success: bool,
    ) -> Result<BoundedProcessOutput, BoundedProcessFailure> {
        loop {
            let progressed = match self.pump_io() {
                Ok(progressed) => progressed,
                Err(failure) => {
                    return Err(self.fail(failure.kind, failure.pipe, failure.detail));
                }
            };
            match self.poll_status() {
                Ok(Some(status)) if require_success && !status.success() => {
                    return Err(self.fail(
                        BoundedProcessFailureKind::UnsuccessfulExit,
                        None,
                        format!("child exited unsuccessfully with status {status}"),
                    ));
                }
                Ok(_) => {}
                Err(failure) => {
                    return Err(self.fail(failure.kind, failure.pipe, failure.detail));
                }
            }
            if self.status.is_some() && self.stdout.is_none() && self.stderr.is_none() {
                return self.finish();
            }
            if Instant::now() >= self.lifecycle_deadline {
                let failure = self.timeout_failure();
                return Err(self.fail(failure.kind, failure.pipe, failure.detail));
            }
            if !progressed {
                self.sleep_until(self.lifecycle_deadline);
            }
        }
    }

    fn finish(mut self) -> Result<BoundedProcessOutput, BoundedProcessFailure> {
        let cleanup = self.cleanup();
        if let Some(detail) = cleanup.detail {
            let kind = if cleanup.containment_failed {
                BoundedProcessFailureKind::PlatformContainment
            } else if cleanup.reap_failed {
                BoundedProcessFailureKind::Reap
            } else {
                BoundedProcessFailureKind::Cleanup
            };
            return Err(self.build_failure(kind, None, detail, None));
        }
        let status = self
            .status
            .expect("completed bounded process must have a reaped status");
        Ok(BoundedProcessOutput {
            status,
            stdout: self.stdout_capture.take(),
            stderr: self.stderr_capture.take(),
            elapsed: self.started_at.elapsed(),
        })
    }

    fn fail(
        mut self,
        kind: BoundedProcessFailureKind,
        pipe: Option<ProcessPipe>,
        detail: String,
    ) -> BoundedProcessFailure {
        let cleanup = self.cleanup();
        self.build_failure(kind, pipe, detail, cleanup.detail)
    }

    fn build_failure(
        &mut self,
        kind: BoundedProcessFailureKind,
        pipe: Option<ProcessPipe>,
        detail: String,
        cleanup_detail: Option<String>,
    ) -> BoundedProcessFailure {
        BoundedProcessFailure {
            kind,
            pipe,
            detail,
            status: self.status,
            stdout: Box::new(self.stdout_capture.take()),
            stderr: Box::new(self.stderr_capture.take()),
            elapsed: self.started_at.elapsed(),
            cleanup_detail,
        }
    }

    fn pump_io(&mut self) -> Result<bool, OperationFailure> {
        let mut progressed = false;
        let mut buffer = [0_u8; PIPE_READ_CHUNK_BYTES];
        if let Some(stdout) = self.stdout.as_mut() {
            match read_child_stdout_available(stdout, &mut buffer) {
                Ok(PipeRead::Data(count)) => {
                    self.stdout_capture.push(&buffer[..count]);
                    progressed = true;
                }
                Ok(PipeRead::NoData) => {}
                Ok(PipeRead::Eof) => {
                    self.stdout.take();
                    progressed = true;
                }
                Err(error) => {
                    self.stdout.take();
                    return Err(OperationFailure {
                        kind: BoundedProcessFailureKind::PipeRead,
                        pipe: Some(ProcessPipe::Stdout),
                        detail: format!("failed to read child stdout: {error}"),
                    });
                }
            }
        }
        if let Some(stderr) = self.stderr.as_mut() {
            match read_child_stderr_available(stderr, &mut buffer) {
                Ok(PipeRead::Data(count)) => {
                    self.stderr_capture.push(&buffer[..count]);
                    progressed = true;
                }
                Ok(PipeRead::NoData) => {}
                Ok(PipeRead::Eof) => {
                    self.stderr.take();
                    progressed = true;
                }
                Err(error) => {
                    self.stderr.take();
                    return Err(OperationFailure {
                        kind: BoundedProcessFailureKind::PipeRead,
                        pipe: Some(ProcessPipe::Stderr),
                        detail: format!("failed to read child stderr: {error}"),
                    });
                }
            }
        }
        Ok(progressed)
    }

    fn poll_status(&mut self) -> Result<Option<ExitStatus>, OperationFailure> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.status = Some(status);
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            Err(error) => Err(OperationFailure {
                kind: BoundedProcessFailureKind::Reap,
                pipe: None,
                detail: format!("failed to poll or reap direct child: {error}"),
            }),
        }
    }

    fn timeout_failure(&self) -> OperationFailure {
        OperationFailure {
            kind: BoundedProcessFailureKind::Timeout,
            pipe: None,
            detail: format!(
                "process lifecycle timed out after {:?}",
                self.deadline.lifecycle_timeout
            ),
        }
    }

    fn cleanup(&mut self) -> CleanupReport {
        if self.finalized {
            return CleanupReport {
                detail: None,
                containment_failed: false,
                reap_failed: false,
            };
        }
        self.stdin.take();
        if !self.stdout_configured {
            self.stdout.take();
        }
        if !self.stderr_configured {
            self.stderr.take();
        }
        let cleanup_deadline = Instant::now()
            .checked_add(self.deadline.cleanup_timeout)
            .unwrap_or_else(Instant::now);
        let mut issues = Vec::new();
        let mut containment_failed = false;
        let mut reap_failed = false;

        if !self.termination_attempted {
            self.termination_attempted = true;
            if let Err(error) = self.containment.terminate_tree() {
                containment_failed = true;
                issues.push(format!("process-tree termination failed: {error}"));
            }
        }
        if self.status.is_none() {
            match self.child.try_wait() {
                Ok(Some(status)) => self.status = Some(status),
                Ok(None) => {
                    if let Err(error) = self.child.kill() {
                        issues.push(format!("direct-child termination fallback failed: {error}"));
                    }
                }
                Err(error) => {
                    reap_failed = true;
                    issues.push(format!("direct-child status polling failed: {error}"));
                }
            }
        }

        loop {
            match self.pump_io() {
                Ok(_) => {}
                Err(error) => issues.push(error.detail),
            }
            if self.status.is_none() && !reap_failed {
                match self.poll_status() {
                    Ok(_) => {}
                    Err(error) => {
                        reap_failed = true;
                        issues.push(error.detail);
                    }
                }
            }
            if self.status.is_some() && self.stdout.is_none() && self.stderr.is_none() {
                break;
            }
            if Instant::now() >= cleanup_deadline {
                break;
            }
            self.sleep_until(cleanup_deadline);
        }

        if self.status.is_none() {
            reap_failed = true;
            issues.push("direct child was not reaped before the cleanup deadline".to_owned());
        }
        if self.stdout.is_some() {
            issues.push("stdout remained open after process-tree termination".to_owned());
            self.stdout.take();
        }
        if self.stderr.is_some() {
            issues.push("stderr remained open after process-tree termination".to_owned());
            self.stderr.take();
        }
        self.finalized = true;
        CleanupReport {
            detail: (!issues.is_empty()).then(|| issues.join("; ")),
            containment_failed,
            reap_failed,
        }
    }

    fn sleep_until(&self, deadline: Instant) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if !remaining.is_zero() {
            thread::sleep(PROCESS_POLL_INTERVAL.min(remaining));
        }
    }
}

impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        if !self.finalized {
            let _ = self.cleanup();
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    const CLEANUP_FIXTURE_ENV: &str = "VOLICORD_TEST_PROCESS_CLEANUP_FIXTURE";

    #[test]
    fn repeated_cleanup_is_idempotent() {
        let started_at = Instant::now();
        let deadline = ProcessDeadline::new(Duration::from_secs(1), Duration::from_secs(2));
        let lifecycle_deadline = started_at + deadline.lifecycle_timeout();
        let mut containment = ProcessContainment::new().expect("create containment");
        let mut command = Command::new(std::env::current_exe().expect("current test executable"));
        containment.configure_command(&mut command);
        command
            .args([
                "--ignored",
                "--exact",
                "internal_tests::bounded_cleanup_fixture",
            ])
            .env(CLEANUP_FIXTURE_ENV, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command.spawn().expect("spawn cleanup fixture");
        containment.attach(&child).expect("attach cleanup fixture");
        let mut supervisor = ProcessSupervisor::new(
            child,
            containment,
            started_at,
            lifecycle_deadline,
            deadline,
            1024,
            1024,
        );
        supervisor.acquire_pipes(false).expect("acquire pipes");
        supervisor.configure_pipes().expect("configure pipes");

        let first = supervisor.cleanup();
        assert!(first.detail.is_none(), "{:?}", first.detail);
        let status = supervisor.status.expect("first cleanup reaped fixture");
        let second = supervisor.cleanup();
        assert!(second.detail.is_none(), "{:?}", second.detail);
        assert_eq!(supervisor.status, Some(status));
        assert!(supervisor.finalized);
    }

    #[test]
    #[ignore = "child-process fixture for repeated_cleanup_is_idempotent"]
    fn bounded_cleanup_fixture() {
        if std::env::var_os(CLEANUP_FIXTURE_ENV).is_some() {
            loop {
                thread::park();
            }
        }
    }
}
