use std::{
    collections::VecDeque,
    io::{self, Write},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use volicord_platform_process::{
    configure_child_stderr_pipe, configure_child_stdin_pipe, configure_child_stdout_pipe,
    read_child_stderr_available, read_child_stdout_available, PipeRead, ProcessContainment,
};

use super::failure::{
    bounded_io_detail, bounded_io_text, BoundedText, McpProcessFailure, McpStage,
    MAX_CAPTURED_STDERR_BYTES,
};

pub(super) const MAX_PREFLIGHT_STDOUT_BYTES: usize = 16 * 1024;
pub(super) const MAX_PROTOCOL_LINE_BYTES: usize = 64 * 1024;
pub(super) const MAX_PROTOCOL_MESSAGES: usize = 16;
const MAX_PROTOCOL_REQUEST_BYTES: usize = 4 * 1024;
const PIPE_READ_CHUNK_BYTES: usize = 4 * 1024;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(5);
pub(super) const CLEANUP_ALLOWANCE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy)]
pub(super) enum SupervisorKind {
    Preflight,
    Stdio,
}

#[derive(Debug)]
pub(super) enum ProtocolEvent {
    Line(Vec<u8>),
    Eof,
    LineTooLong { observed_bytes: usize },
    IncompleteLine { observed_bytes: usize },
    MessageLimitExceeded { limit: usize },
}

pub(super) enum ProtocolRead {
    Event(ProtocolEvent),
    Exited(ExitStatus),
}

pub(super) struct SupervisedOutput {
    pub(super) status: ExitStatus,
    pub(super) stdout: BoundedText,
    pub(super) stderr: BoundedText,
}

pub(super) struct ChildSupervisor {
    child: Child,
    containment: ProcessContainment,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    stdout_state: StdoutState,
    stderr_capture: BoundedCapture,
    status: Option<ExitStatus>,
    timeout: Duration,
    deadline: Instant,
    finalized: bool,
}

impl ChildSupervisor {
    pub(super) fn spawn(
        mut command: Command,
        kind: SupervisorKind,
        timeout: Duration,
    ) -> Result<Self, McpProcessFailure> {
        let started_at = Instant::now();
        let deadline = started_at.checked_add(timeout).unwrap_or(started_at);
        let mut containment =
            ProcessContainment::new().map_err(|error| McpProcessFailure::Spawn {
                stage: McpStage::Startup,
                io_detail: bounded_io_text(error.to_string()),
            })?;
        containment.configure_command(&mut command);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        match kind {
            SupervisorKind::Preflight => {
                command.stdin(Stdio::null());
            }
            SupervisorKind::Stdio => {
                command.stdin(Stdio::piped());
            }
        }

        let mut child = command.spawn().map_err(|error| McpProcessFailure::Spawn {
            stage: McpStage::Startup,
            io_detail: bounded_io_detail(error),
        })?;
        let attach_error = containment.attach(&child).err();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let supervisor = Self {
            child,
            containment,
            stdin,
            stdout,
            stderr,
            stdout_state: match kind {
                SupervisorKind::Preflight => StdoutState::Capture(BoundedCapture::new(
                    MAX_PREFLIGHT_STDOUT_BYTES,
                    "preflight stdout",
                )),
                SupervisorKind::Stdio => StdoutState::Protocol(ProtocolFramer::new()),
            },
            stderr_capture: BoundedCapture::new(MAX_CAPTURED_STDERR_BYTES, "stderr"),
            status: None,
            timeout,
            deadline,
            finalized: false,
        };

        let startup_failure = attach_error
            .map(|error| McpProcessFailure::Spawn {
                stage: McpStage::Startup,
                io_detail: bounded_io_text(error.to_string()),
            })
            .or_else(|| {
                supervisor
                    .stdout
                    .is_none()
                    .then(|| McpProcessFailure::PipeAcquisition {
                        stage: McpStage::Startup,
                        io_detail: bounded_io_text("MCP stdout pipe was unavailable"),
                        stderr: BoundedText::empty(),
                    })
            })
            .or_else(|| {
                supervisor
                    .stderr
                    .is_none()
                    .then(|| McpProcessFailure::PipeAcquisition {
                        stage: McpStage::Startup,
                        io_detail: bounded_io_text("MCP stderr pipe was unavailable"),
                        stderr: BoundedText::empty(),
                    })
            })
            .or_else(|| {
                matches!(kind, SupervisorKind::Stdio)
                    .then_some(())
                    .filter(|()| supervisor.stdin.is_none())
                    .map(|()| McpProcessFailure::PipeAcquisition {
                        stage: McpStage::Startup,
                        io_detail: bounded_io_text("MCP stdin pipe was unavailable"),
                        stderr: BoundedText::empty(),
                    })
            });
        if let Some(failure) = startup_failure {
            return Err(supervisor.finish_failure(failure));
        }

        if let Err(error) = supervisor.prepare_pipes() {
            let failure = McpProcessFailure::PipeAcquisition {
                stage: McpStage::Startup,
                io_detail: bounded_io_text(error),
                stderr: BoundedText::empty(),
            };
            return Err(supervisor.finish_failure(failure));
        }
        Ok(supervisor)
    }

    pub(super) fn send_json_line(
        &mut self,
        value: &Value,
        stage: McpStage,
    ) -> Result<(), McpProcessFailure> {
        if Instant::now() >= self.deadline {
            return Err(self.timeout_failure(stage));
        }
        let mut payload = serde_json::to_vec(value).map_err(|error| McpProcessFailure::Write {
            stage,
            io_detail: bounded_io_detail(error),
            stderr: BoundedText::empty(),
        })?;
        payload.push(b'\n');
        if payload.len() > MAX_PROTOCOL_REQUEST_BYTES {
            return Err(McpProcessFailure::Write {
                stage,
                io_detail: bounded_io_text(format!(
                    "MCP request exceeded the {MAX_PROTOCOL_REQUEST_BYTES}-byte limit"
                )),
                stderr: BoundedText::empty(),
            });
        }

        let mut written = 0;
        while written < payload.len() {
            let progressed = self.pump_io(stage)?;
            if let Some(status) = self.poll_status(stage)? {
                return Err(McpProcessFailure::ExitedBeforeResponse {
                    stage,
                    exit_code: status.code(),
                    stderr: BoundedText::empty(),
                });
            }
            if Instant::now() >= self.deadline {
                return Err(self.timeout_failure(stage));
            }
            let Some(stdin) = self.stdin.as_mut() else {
                return Err(McpProcessFailure::Write {
                    stage,
                    io_detail: bounded_io_text("MCP stdin pipe was unavailable"),
                    stderr: BoundedText::empty(),
                });
            };
            match stdin.write(&payload[written..]) {
                Ok(0) => {
                    return Err(McpProcessFailure::Write {
                        stage,
                        io_detail: bounded_io_text("MCP stdin accepted zero bytes"),
                        stderr: BoundedText::empty(),
                    })
                }
                Ok(count) => written += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if !progressed {
                        self.sleep_until(self.deadline);
                    }
                }
                Err(error) => {
                    return Err(McpProcessFailure::Write {
                        stage,
                        io_detail: bounded_io_detail(error),
                        stderr: BoundedText::empty(),
                    })
                }
            }
        }
        Ok(())
    }

    pub(super) fn read_protocol(
        &mut self,
        stage: McpStage,
    ) -> Result<ProtocolRead, McpProcessFailure> {
        loop {
            let progressed = self.pump_io(stage)?;
            if let StdoutState::Protocol(framer) = &mut self.stdout_state {
                if let Some(event) = framer.next_event() {
                    return Ok(ProtocolRead::Event(event));
                }
            }
            if let Some(status) = self.poll_status(stage)? {
                return Ok(ProtocolRead::Exited(status));
            }
            if Instant::now() >= self.deadline {
                return Err(self.timeout_failure(stage));
            }
            if !progressed {
                self.sleep_until(self.deadline);
            }
        }
    }

    pub(super) fn wait_for_exit(
        &mut self,
        stage: McpStage,
    ) -> Result<ExitStatus, McpProcessFailure> {
        loop {
            let progressed = self.pump_io(stage)?;
            if let Some(status) = self.poll_status(stage)? {
                return Ok(status);
            }
            if Instant::now() >= self.deadline {
                return Err(self.timeout_failure(stage));
            }
            if !progressed {
                self.sleep_until(self.deadline);
            }
        }
    }

    pub(super) fn wait_for_exit_for(
        &mut self,
        stage: McpStage,
        duration: Duration,
    ) -> Result<Option<ExitStatus>, McpProcessFailure> {
        let local_deadline = Instant::now()
            .checked_add(duration)
            .map_or(self.deadline, |candidate| candidate.min(self.deadline));
        loop {
            let progressed = self.pump_io(stage)?;
            if let Some(status) = self.poll_status(stage)? {
                return Ok(Some(status));
            }
            let now = Instant::now();
            if now >= self.deadline {
                return Err(self.timeout_failure(stage));
            }
            if now >= local_deadline {
                return Ok(None);
            }
            if !progressed {
                self.sleep_until(local_deadline);
            }
        }
    }

    pub(super) fn close_stdin(&mut self) {
        self.stdin.take();
    }

    pub(super) fn finish_success(
        mut self,
        stage: McpStage,
    ) -> Result<SupervisedOutput, McpProcessFailure> {
        self.finish(stage)
    }

    pub(super) fn finish_failure(mut self, failure: McpProcessFailure) -> McpProcessFailure {
        let stage = failure.stage();
        match self.finish(stage) {
            Ok(output) => failure.with_stderr(output.stderr),
            Err(cleanup_failure) => cleanup_failure,
        }
    }

    pub(super) fn child_id(&self) -> u32 {
        self.child.id()
    }

    fn finish(&mut self, stage: McpStage) -> Result<SupervisedOutput, McpProcessFailure> {
        self.close_stdin();
        let cleanup_deadline = Instant::now()
            .checked_add(CLEANUP_ALLOWANCE)
            .unwrap_or_else(Instant::now);
        let mut cleanup_detail = String::new();

        if let Err(error) = self.containment.terminate_tree() {
            append_cleanup_detail(&mut cleanup_detail, &error.to_string());
        }
        if self.status.is_none() {
            let _ = self.child.kill();
        }

        loop {
            match self.pump_io(stage) {
                Ok(_) => {}
                Err(error) => append_cleanup_detail(&mut cleanup_detail, &error.summary()),
            }
            match self.poll_status(stage) {
                Ok(_) => {}
                Err(error) => append_cleanup_detail(&mut cleanup_detail, &error.summary()),
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
            append_cleanup_detail(
                &mut cleanup_detail,
                "direct MCP child was not reaped within the cleanup allowance",
            );
        }
        if self.stdout.is_some() {
            append_cleanup_detail(
                &mut cleanup_detail,
                "MCP stdout remained open after contained process-tree termination",
            );
        }
        if self.stderr.is_some() {
            append_cleanup_detail(
                &mut cleanup_detail,
                "MCP stderr remained open after contained process-tree termination",
            );
        }

        let stdout = self.stdout_state.take_capture();
        let stderr = self.stderr_capture.take();
        self.finalized = true;
        if !cleanup_detail.is_empty() {
            return Err(McpProcessFailure::Cleanup {
                stage,
                io_detail: bounded_io_text(cleanup_detail),
                stderr,
            });
        }
        let status = self
            .status
            .expect("successful cleanup reaped the direct child");
        Ok(SupervisedOutput {
            status,
            stdout,
            stderr,
        })
    }

    fn prepare_pipes(&self) -> Result<(), String> {
        if let Some(stdout) = &self.stdout {
            configure_child_stdout_pipe(stdout).map_err(|error| {
                format!("failed to configure MCP stdout for bounded reads: {error}")
            })?;
        }
        if let Some(stderr) = &self.stderr {
            configure_child_stderr_pipe(stderr).map_err(|error| {
                format!("failed to configure MCP stderr for bounded reads: {error}")
            })?;
        }
        if let Some(stdin) = &self.stdin {
            configure_child_stdin_pipe(stdin).map_err(|error| {
                format!("failed to configure MCP stdin for bounded writes: {error}")
            })?;
        }
        Ok(())
    }

    fn pump_io(&mut self, stage: McpStage) -> Result<bool, McpProcessFailure> {
        let mut progressed = false;
        let mut buffer = [0_u8; PIPE_READ_CHUNK_BYTES];

        if let Some(stdout) = self.stdout.as_mut() {
            match read_child_stdout_available(stdout, &mut buffer) {
                Ok(PipeRead::Data(count)) => {
                    progressed = true;
                    self.stdout_state.push(&buffer[..count]);
                }
                Ok(PipeRead::NoData) => {}
                Ok(PipeRead::Eof) => {
                    progressed = true;
                    self.stdout.take();
                    self.stdout_state.finish_eof();
                }
                Err(error) => {
                    self.stdout.take();
                    return Err(McpProcessFailure::Read {
                        stage,
                        io_detail: bounded_io_text(format!("MCP stdout read failed: {error}")),
                        stderr: BoundedText::empty(),
                    });
                }
            }
        }

        if let Some(stderr) = self.stderr.as_mut() {
            match read_child_stderr_available(stderr, &mut buffer) {
                Ok(PipeRead::Data(count)) => {
                    progressed = true;
                    self.stderr_capture.push(&buffer[..count]);
                }
                Ok(PipeRead::NoData) => {}
                Ok(PipeRead::Eof) => {
                    progressed = true;
                    self.stderr.take();
                }
                Err(error) => {
                    self.stderr.take();
                    return Err(McpProcessFailure::Read {
                        stage,
                        io_detail: bounded_io_text(format!("MCP stderr read failed: {error}")),
                        stderr: BoundedText::empty(),
                    });
                }
            }
        }
        Ok(progressed)
    }

    fn poll_status(&mut self, stage: McpStage) -> Result<Option<ExitStatus>, McpProcessFailure> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.status = Some(status);
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            Err(error) => Err(McpProcessFailure::Wait {
                stage,
                io_detail: bounded_io_detail(error),
                stderr: BoundedText::empty(),
            }),
        }
    }

    fn timeout_failure(&self, stage: McpStage) -> McpProcessFailure {
        McpProcessFailure::Timeout {
            stage,
            timeout: self.timeout,
            stderr: BoundedText::empty(),
        }
    }

    fn sleep_until(&self, deadline: Instant) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if !remaining.is_zero() {
            thread::sleep(PROCESS_POLL_INTERVAL.min(remaining));
        }
    }
}

impl Drop for ChildSupervisor {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        self.stdin.take();
        let _ = self.containment.terminate_tree();
        let _ = self.child.kill();
        let _ = self.child.try_wait();
    }
}

enum StdoutState {
    Capture(BoundedCapture),
    Protocol(ProtocolFramer),
}

impl StdoutState {
    fn push(&mut self, bytes: &[u8]) {
        match self {
            Self::Capture(capture) => capture.push(bytes),
            Self::Protocol(framer) => framer.push(bytes),
        }
    }

    fn finish_eof(&mut self) {
        if let Self::Protocol(framer) = self {
            framer.finish_eof();
        }
    }

    fn take_capture(&mut self) -> BoundedText {
        match self {
            Self::Capture(capture) => capture.take(),
            Self::Protocol(_) => BoundedText::empty(),
        }
    }
}

struct BoundedCapture {
    bytes: Vec<u8>,
    omitted_bytes: usize,
    limit: usize,
    label: &'static str,
}

impl BoundedCapture {
    fn new(limit: usize, label: &'static str) -> Self {
        Self {
            bytes: Vec::with_capacity(limit),
            omitted_bytes: 0,
            limit,
            label,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        let retained = remaining.min(bytes.len());
        self.bytes.extend_from_slice(&bytes[..retained]);
        self.omitted_bytes = self
            .omitted_bytes
            .saturating_add(bytes.len().saturating_sub(retained));
    }

    fn take(&mut self) -> BoundedText {
        BoundedText::from_bytes(
            std::mem::take(&mut self.bytes),
            self.omitted_bytes,
            self.label,
        )
    }
}

struct ProtocolFramer {
    line: Vec<u8>,
    events: VecDeque<ProtocolEvent>,
    message_count: usize,
    terminal: bool,
}

impl ProtocolFramer {
    fn new() -> Self {
        Self {
            line: Vec::with_capacity(MAX_PROTOCOL_LINE_BYTES.min(8 * 1024)),
            events: VecDeque::with_capacity(MAX_PROTOCOL_MESSAGES + 1),
            message_count: 0,
            terminal: false,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        if self.terminal {
            return;
        }
        for byte in bytes {
            if *byte == b'\n' {
                if self.line.last() == Some(&b'\r') {
                    self.line.pop();
                }
                self.message_count += 1;
                if self.message_count > MAX_PROTOCOL_MESSAGES {
                    self.events.push_back(ProtocolEvent::MessageLimitExceeded {
                        limit: MAX_PROTOCOL_MESSAGES,
                    });
                    self.line.clear();
                    self.terminal = true;
                    return;
                }
                self.events
                    .push_back(ProtocolEvent::Line(std::mem::take(&mut self.line)));
                continue;
            }
            if self.line.len() == MAX_PROTOCOL_LINE_BYTES {
                self.events.push_back(ProtocolEvent::LineTooLong {
                    observed_bytes: MAX_PROTOCOL_LINE_BYTES + 1,
                });
                self.line.clear();
                self.terminal = true;
                return;
            }
            self.line.push(*byte);
        }
    }

    fn finish_eof(&mut self) {
        if self.terminal {
            return;
        }
        if self.line.is_empty() {
            self.events.push_back(ProtocolEvent::Eof);
        } else {
            self.events.push_back(ProtocolEvent::IncompleteLine {
                observed_bytes: self.line.len(),
            });
            self.line.clear();
        }
        self.terminal = true;
    }

    fn next_event(&mut self) -> Option<ProtocolEvent> {
        self.events.pop_front()
    }
}

fn append_cleanup_detail(target: &mut String, detail: &str) {
    if target.len() >= super::failure::MAX_IO_DETAIL_BYTES {
        return;
    }
    if !target.is_empty() {
        target.push_str("; ");
    }
    let remaining = super::failure::MAX_IO_DETAIL_BYTES.saturating_sub(target.len());
    let mut end = detail.len().min(remaining);
    while !detail.is_char_boundary(end) {
        end -= 1;
    }
    target.push_str(&detail[..end]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_command::mcp_process::test_child;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use rustix::{io::Errno, process::Pid};

    #[test]
    fn protocol_framer_bounds_lines_and_message_count() {
        let mut framer = ProtocolFramer::new();
        framer.push(&vec![b'x'; MAX_PROTOCOL_LINE_BYTES + 1]);
        assert!(matches!(
            framer.next_event(),
            Some(ProtocolEvent::LineTooLong { observed_bytes })
                if observed_bytes == MAX_PROTOCOL_LINE_BYTES + 1
        ));

        let mut framer = ProtocolFramer::new();
        for _ in 0..=MAX_PROTOCOL_MESSAGES {
            framer.push(b"{}\n");
        }
        for _ in 0..MAX_PROTOCOL_MESSAGES {
            assert!(matches!(framer.next_event(), Some(ProtocolEvent::Line(_))));
        }
        assert!(matches!(
            framer.next_event(),
            Some(ProtocolEvent::MessageLimitExceeded { limit })
                if limit == MAX_PROTOCOL_MESSAGES
        ));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn forced_termination_reaps_the_direct_child() {
        let supervisor = ChildSupervisor::spawn(
            test_child::command("hang-before-initialize"),
            SupervisorKind::Preflight,
            Duration::from_millis(25),
        )
        .expect("spawn fixture");
        let child_id = supervisor.child_id();
        let failure = supervisor.finish_failure(McpProcessFailure::Timeout {
            stage: McpStage::Startup,
            timeout: Duration::from_millis(25),
            stderr: BoundedText::empty(),
        });
        assert!(matches!(failure, McpProcessFailure::Timeout { .. }));
        let raw_pid = i32::try_from(child_id).expect("fixture PID");
        let pid = Pid::from_raw(raw_pid).expect("nonzero fixture PID");
        assert_eq!(rustix::process::test_kill_process(pid), Err(Errno::SRCH));
    }
}
