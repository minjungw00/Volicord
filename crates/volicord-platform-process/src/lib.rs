//! Safe platform primitives for bounded child-process containment and pipe polling.

#![deny(unsafe_code)]

use std::{
    fmt,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
#[path = "unsupported.rs"]
mod platform;

/// Stable operation categories for platform-process failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformProcessOperation {
    /// Create an OS process-containment object.
    CreateContainment,
    /// Configure the newly created containment object.
    ConfigureContainment,
    /// Attach a spawned direct child to its containment object.
    AttachChild,
    /// Terminate the attached process tree.
    TerminateProcessTree,
    /// Configure a child pipe for bounded polling.
    ConfigurePipe,
    /// Inspect or read currently available child-pipe bytes.
    ReadPipe,
}

impl PlatformProcessOperation {
    const fn failure_reason(self) -> &'static str {
        match self {
            Self::CreateContainment => "create_containment_failed",
            Self::ConfigureContainment => "configure_containment_failed",
            Self::AttachChild => "attach_child_failed",
            Self::TerminateProcessTree => "terminate_process_tree_failed",
            Self::ConfigurePipe => "configure_pipe_failed",
            Self::ReadPipe => "read_pipe_failed",
        }
    }
}

/// Stable classes for failures at the platform-process boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformProcessErrorKind {
    /// The requested primitive is unavailable on the current target.
    UnsupportedPlatform,
    /// The supplied child process cannot be represented by the target API.
    InvalidChildProcess,
    /// The operating system rejected the requested operation.
    OperatingSystem,
}

/// A narrowly classified platform-process failure with implementation detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformProcessError {
    kind: PlatformProcessErrorKind,
    operation: PlatformProcessOperation,
    reason: &'static str,
    detail: String,
}

impl PlatformProcessError {
    /// Returns the stable failure class.
    pub const fn kind(&self) -> PlatformProcessErrorKind {
        self.kind
    }

    /// Returns the operation that failed.
    pub const fn operation(&self) -> PlatformProcessOperation {
        self.operation
    }

    /// Returns the stable machine-readable reason.
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Returns implementation-facing detail without any raw platform value.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub(crate) fn operating_system(
        operation: PlatformProcessOperation,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: PlatformProcessErrorKind::OperatingSystem,
            operation,
            reason: operation.failure_reason(),
            detail: detail.into(),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn invalid_child(reason: &'static str, detail: impl Into<String>) -> Self {
        Self {
            kind: PlatformProcessErrorKind::InvalidChildProcess,
            operation: PlatformProcessOperation::AttachChild,
            reason,
            detail: detail.into(),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    pub(crate) fn unsupported(
        operation: PlatformProcessOperation,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: PlatformProcessErrorKind::UnsupportedPlatform,
            operation,
            reason: "platform_process_unsupported",
            detail: detail.into(),
        }
    }
}

impl fmt::Display for PlatformProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for PlatformProcessError {}

/// The result of one nonblocking child-pipe poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeRead {
    /// Bytes were copied into the caller's buffer.
    Data(usize),
    /// The pipe remains open but no bytes are currently available.
    NoData,
    /// The writer has closed or disconnected the pipe.
    Eof,
}

/// Safe ownership boundary for an isolated child process tree.
pub struct ProcessContainment {
    inner: platform::ProcessContainment,
}

impl ProcessContainment {
    /// Creates an empty platform containment object.
    pub fn new() -> Result<Self, PlatformProcessError> {
        platform::ProcessContainment::new().map(|inner| Self { inner })
    }

    /// Applies target-specific containment configuration before spawning a child.
    pub fn configure_command(&self, command: &mut Command) {
        self.inner.configure_command(command);
    }

    /// Attaches the spawned direct child to this containment object.
    pub fn attach(&mut self, child: &Child) -> Result<(), PlatformProcessError> {
        self.inner.attach(child)
    }

    /// Terminates the attached process tree.
    pub fn terminate_tree(&self) -> Result<(), PlatformProcessError> {
        self.inner.terminate_tree()
    }
}

/// Configures a child stdout pipe for bounded polling.
pub fn configure_child_stdout_pipe(pipe: &ChildStdout) -> Result<(), PlatformProcessError> {
    platform::configure_pipe(pipe)
}

/// Configures a child stderr pipe for bounded polling.
pub fn configure_child_stderr_pipe(pipe: &ChildStderr) -> Result<(), PlatformProcessError> {
    platform::configure_pipe(pipe)
}

/// Configures a child stdin pipe for bounded writes.
pub fn configure_child_stdin_pipe(pipe: &ChildStdin) -> Result<(), PlatformProcessError> {
    platform::configure_pipe(pipe)
}

/// Reads currently available child stdout bytes without waiting indefinitely.
pub fn read_child_stdout_available(
    pipe: &mut ChildStdout,
    buffer: &mut [u8],
) -> Result<PipeRead, PlatformProcessError> {
    if buffer.is_empty() {
        return Ok(PipeRead::NoData);
    }
    platform::read_pipe_available(pipe, buffer)
}

/// Reads currently available child stderr bytes without waiting indefinitely.
pub fn read_child_stderr_available(
    pipe: &mut ChildStderr,
    buffer: &mut [u8],
) -> Result<PipeRead, PlatformProcessError> {
    if buffer.is_empty() {
        return Ok(PipeRead::NoData);
    }
    platform::read_pipe_available(pipe, buffer)
}
