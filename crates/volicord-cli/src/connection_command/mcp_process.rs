use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use volicord_mcp::{
    ManagedMcpInvocationPurpose, ManagedMcpLaunchSpec, ManagedMcpMaterializationInput,
    ManagedMcpWorkingDirectory, MaterializedManagedMcpLaunch, VOLICORD_HOME_ENV,
};
use volicord_store::agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW};
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, LIST_PROJECTS_TOOL_NAME, READ_ONLY_METHOD_TOOL_NAMES,
    WORKFLOW_METHOD_TOOL_NAMES,
};

use super::verification::{McpPreflightDiagnostics, VerificationStep};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const READER_CHANNEL_CAPACITY: usize = 1;
const MAX_PREFLIGHT_STDOUT_BYTES: usize = 16 * 1024;
const MAX_CAPTURED_STDERR_BYTES: usize = 2 * 1024;
const MAX_PROTOCOL_LINE_BYTES: usize = 64 * 1024;
const MAX_PROTOCOL_DETAIL_BYTES: usize = 2 * 1024;
const MAX_IO_DETAIL_BYTES: usize = 1024;
const CHILD_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const EARLY_EXIT_STATUS_WAIT: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStage {
    Startup,
    Initialize,
    ToolsList,
    SafeToolCall,
    Shutdown,
}

impl McpStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Initialize => "initialize",
            Self::ToolsList => "tools_list",
            Self::SafeToolCall => "safe_tool_call",
            Self::Shutdown => "shutdown",
        }
    }

    const fn check_code(self) -> &'static str {
        match self {
            Self::Startup | Self::Shutdown => "mcp_server_process_failed",
            Self::Initialize => "mcp_server_initialize_failed",
            Self::ToolsList => "mcp_server_tools_list_failed",
            Self::SafeToolCall => "mcp_server_safe_call_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedText {
    text: String,
    truncated: bool,
    omitted_bytes: usize,
}

impl BoundedText {
    fn empty() -> Self {
        Self {
            text: String::new(),
            truncated: false,
            omitted_bytes: 0,
        }
    }

    fn from_utf8(value: impl AsRef<str>, limit: usize, label: &str) -> Self {
        let value = value.as_ref();
        if value.len() <= limit {
            return Self {
                text: value.to_owned(),
                truncated: false,
                omitted_bytes: 0,
            };
        }
        let mut end = limit;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        let omitted_bytes = value.len() - end;
        Self {
            text: format!(
                "{}\n...[{label} truncated; {omitted_bytes} bytes omitted]",
                &value[..end]
            ),
            truncated: true,
            omitted_bytes,
        }
    }

    fn from_bytes(bytes: Vec<u8>, omitted_bytes: usize, label: &str) -> Self {
        let mut text = String::from_utf8_lossy(&bytes)
            .chars()
            .map(|character| {
                if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
                    '\u{fffd}'
                } else {
                    character
                }
            })
            .collect::<String>();
        if omitted_bytes > 0 {
            text.push_str(&format!(
                "\n...[{label} truncated; {omitted_bytes} bytes omitted]"
            ));
        }
        Self {
            text,
            truncated: omitted_bytes > 0,
            omitted_bytes,
        }
    }

    pub(super) fn to_json(&self) -> Value {
        json!({
            "text": self.text,
            "truncated": self.truncated,
            "omitted_bytes": self.omitted_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProcessFailure {
    Spawn {
        stage: McpStage,
        io_detail: BoundedText,
    },
    ExitedBeforeResponse {
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: BoundedText,
    },
    Timeout {
        stage: McpStage,
        timeout: Duration,
        stderr: BoundedText,
    },
    Read {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Write {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Protocol {
        stage: McpStage,
        protocol_detail: BoundedText,
        missing_tools: Vec<String>,
        stderr: BoundedText,
    },
    Wait {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Shutdown {
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: BoundedText,
    },
}

impl McpProcessFailure {
    pub fn protocol(stage: McpStage, detail: impl Into<String>) -> Self {
        Self::Protocol {
            stage,
            protocol_detail: bounded_protocol_detail(detail.into()),
            missing_tools: Vec::new(),
            stderr: BoundedText::empty(),
        }
    }

    pub const fn stage(&self) -> McpStage {
        match self {
            Self::Spawn { stage, .. }
            | Self::ExitedBeforeResponse { stage, .. }
            | Self::Timeout { stage, .. }
            | Self::Read { stage, .. }
            | Self::Write { stage, .. }
            | Self::Protocol { stage, .. }
            | Self::Wait { stage, .. }
            | Self::Shutdown { stage, .. } => *stage,
        }
    }

    pub const fn check_code(&self) -> &'static str {
        self.stage().check_code()
    }

    fn with_stderr(mut self, stderr: BoundedText) -> Self {
        match &mut self {
            Self::Spawn { .. } => {}
            Self::ExitedBeforeResponse {
                stderr: captured, ..
            }
            | Self::Timeout {
                stderr: captured, ..
            }
            | Self::Read {
                stderr: captured, ..
            }
            | Self::Write {
                stderr: captured, ..
            }
            | Self::Protocol {
                stderr: captured, ..
            }
            | Self::Wait {
                stderr: captured, ..
            }
            | Self::Shutdown {
                stderr: captured, ..
            } => *captured = stderr,
        }
        self
    }

    fn summary(&self) -> String {
        match self {
            Self::Spawn { io_detail, .. } => {
                format!("MCP process spawn failed: {}", io_detail.text)
            }
            Self::ExitedBeforeResponse {
                stage, exit_code, ..
            } => format!(
                "MCP process exited before a response during {} with exit code {}",
                stage.as_str(),
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unavailable".to_owned())
            ),
            Self::Timeout { stage, .. } => {
                format!("MCP process timed out during {}", stage.as_str())
            }
            Self::Read {
                stage, io_detail, ..
            } => format!(
                "MCP process read failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Write {
                stage, io_detail, ..
            } => format!(
                "MCP process write failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Protocol {
                stage,
                protocol_detail,
                ..
            } => format!(
                "MCP protocol failed during {}: {}",
                stage.as_str(),
                protocol_detail.text
            ),
            Self::Wait {
                stage, io_detail, ..
            } => format!(
                "MCP process wait failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Shutdown { exit_code, .. } => format!(
                "MCP process shutdown failed with exit code {}",
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unavailable".to_owned())
            ),
        }
    }

    pub(super) fn to_json(&self) -> Value {
        let mut failure = serde_json::Map::new();
        failure.insert("kind".to_owned(), Value::String(self.kind().to_owned()));
        failure.insert(
            "stage".to_owned(),
            Value::String(self.stage().as_str().to_owned()),
        );
        match self {
            Self::Spawn { io_detail, .. } => {
                failure.insert("io_detail".to_owned(), io_detail.to_json());
            }
            Self::ExitedBeforeResponse {
                exit_code, stderr, ..
            }
            | Self::Shutdown {
                exit_code, stderr, ..
            } => {
                failure.insert(
                    "exit_code".to_owned(),
                    exit_code.map_or(Value::Null, |code| Value::from(i64::from(code))),
                );
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
            Self::Timeout {
                timeout, stderr, ..
            } => {
                failure.insert(
                    "timeout_ms".to_owned(),
                    Value::from(timeout.as_millis() as u64),
                );
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
            Self::Read {
                io_detail, stderr, ..
            }
            | Self::Write {
                io_detail, stderr, ..
            }
            | Self::Wait {
                io_detail, stderr, ..
            } => {
                failure.insert("io_detail".to_owned(), io_detail.to_json());
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
            Self::Protocol {
                protocol_detail,
                missing_tools,
                stderr,
                ..
            } => {
                failure.insert("protocol_detail".to_owned(), protocol_detail.to_json());
                if !missing_tools.is_empty() {
                    failure.insert("missing_tools".to_owned(), json!(missing_tools));
                }
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
        }
        Value::Object(failure)
    }

    const fn kind(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "spawn",
            Self::ExitedBeforeResponse { .. } => "exited_before_response",
            Self::Timeout { .. } => "timeout",
            Self::Read { .. } => "read",
            Self::Write { .. } => "write",
            Self::Protocol { .. } => "protocol",
            Self::Wait { .. } => "wait",
            Self::Shutdown { .. } => "shutdown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProcessOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait ConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
    ) -> Result<ConnectionProcessOutput, String>;
    fn verify_mcp_stdio(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
        connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, McpProcessFailure>;
}

pub struct ProductionConnectionProcess;

impl ConnectionProcess for ProductionConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }

    fn run_preflight(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
    ) -> Result<ConnectionProcessOutput, String> {
        let mut child = launch.process_command();
        child
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = child.spawn().map_err(|error| {
            format!(
                "failed to run managed MCP preflight with {}: {error}",
                launch.command()
            )
        })?;
        let Some(stdout) = child.stdout.take() else {
            let cleanup = terminate_and_reap(&mut child).err();
            return Err(cleanup.unwrap_or_else(|| {
                "managed MCP preflight stdout pipe was unavailable".to_owned()
            }));
        };
        let Some(stderr) = child.stderr.take() else {
            let cleanup = terminate_and_reap(&mut child).err();
            return Err(cleanup.unwrap_or_else(|| {
                "managed MCP preflight stderr pipe was unavailable".to_owned()
            }));
        };
        let stdout_reader = thread::spawn(move || {
            drain_bounded_stream(stdout, MAX_PREFLIGHT_STDOUT_BYTES, "preflight stdout")
        });
        let stderr_reader = thread::spawn(move || drain_stderr(stderr));
        let (status, timed_out) =
            match wait_for_child_until(&mut child, Instant::now() + DEFAULT_TIMEOUT) {
                ChildWait::Exited(status) => (status, false),
                ChildWait::TimedOut => match terminate_and_reap(&mut child) {
                    Ok(status) => (status, true),
                    Err(error) => {
                        let _ = stdout_reader.join();
                        let _ = stderr_reader.join();
                        return Err(error);
                    }
                },
                ChildWait::Failed(error) => {
                    let cleanup = terminate_and_reap(&mut child).err();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(cleanup.unwrap_or_else(|| {
                        format!("failed to wait for managed MCP preflight: {error}")
                    }));
                }
            };
        let stdout = stdout_reader.join();
        let stderr = stderr_reader.join();
        let stdout =
            stdout.map_err(|_| "managed MCP preflight stdout reader panicked".to_owned())?;
        let stderr =
            stderr.map_err(|_| "managed MCP preflight stderr reader panicked".to_owned())?;
        if let Some(error) = stdout.read_error.or(stderr.read_error) {
            return Err(format!(
                "failed to read managed MCP preflight output: {error}"
            ));
        }
        let stdout_truncated = stdout.captured.truncated;
        let child_stderr = stderr.captured.text;
        let stderr_text = if timed_out {
            append_diagnostic_context(
                format!(
                    "managed MCP preflight timed out after {} ms",
                    DEFAULT_TIMEOUT.as_millis()
                ),
                &child_stderr,
            )
        } else if stdout_truncated {
            append_diagnostic_context(
                format!(
                    "managed MCP preflight stdout exceeded the {MAX_PREFLIGHT_STDOUT_BYTES}-byte limit"
                ),
                &child_stderr,
            )
        } else {
            child_stderr
        };
        Ok(ConnectionProcessOutput {
            success: status.success() && !timed_out && !stdout_truncated,
            status_code: status.code(),
            stdout: if stdout_truncated {
                String::new()
            } else {
                stdout.captured.text
            },
            stderr: stderr_text,
        })
    }

    fn verify_mcp_stdio(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
        _connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, McpProcessFailure> {
        verify_mcp_stdio_process(launch, mode, DEFAULT_TIMEOUT)
    }
}

#[derive(Debug, Clone)]
pub struct McpVerification {
    pub(super) step: VerificationStep,
    pub(super) tools: Vec<String>,
    pub(super) failure: Option<McpProcessFailure>,
}

impl McpVerification {
    pub(super) fn passed(tools: Vec<String>) -> Self {
        Self {
            step: VerificationStep::passed_with_code(
                "mcp_server_ready",
                format!(
                    "MCP initialize, tools/list, required-tool validation, and designated read-only tool call succeeded; tools/list returned {} tools",
                    tools.len()
                ),
            ),
            tools,
            failure: None,
        }
    }

    pub fn failed(failure: McpProcessFailure) -> Self {
        let code = failure.check_code();
        let details = failure.summary();
        Self {
            step: VerificationStep::failed_with_code(code, details),
            tools: Vec::new(),
            failure: Some(failure),
        }
    }
}

pub(super) fn run_connection_preflight(
    process: &mut impl ConnectionProcess,
    launch: &MaterializedManagedMcpLaunch,
    connection_id: &str,
    mode: &str,
) -> VerificationStep {
    match process.run_preflight(launch) {
        Ok(output) if output.success => {
            match validate_connection_preflight_report(&output.stdout, connection_id, mode) {
                Ok(diagnostics) => VerificationStep::passed_with_code(
                    "mcp_server_preflight_passed",
                    "volicord mcp preflight passed",
                )
                .with_preflight_diagnostics(diagnostics),
                Err(message) => {
                    VerificationStep::failed_with_code("mcp_server_preflight_invalid", message)
                }
            }
        }
        Ok(output) => VerificationStep::failed_with_code(
            "mcp_server_preflight_failed",
            format!(
                "volicord mcp preflight failed with status {}; stderr: {}",
                status_text(output.status_code),
                compact_stream(&output.stderr)
            ),
        ),
        Err(message) => VerificationStep::failed_with_code("mcp_server_process_failed", message),
    }
}

fn validate_connection_preflight_report(
    stdout: &str,
    connection_id: &str,
    mode: &str,
) -> Result<Option<McpPreflightDiagnostics>, String> {
    let report = parse_colon_report(stdout)?;
    expect_report_field(&report, "configuration", "valid")?;
    expect_report_field(&report, "transport", "stdio")?;
    expect_report_field(&report, "connection_id", connection_id)?;
    expect_report_field(&report, "mode", mode)?;
    expect_report_field(&report, "enabled", "true")?;
    expect_report_field(&report, "registry_read", "passed")?;
    expect_report_field(&report, "project_state_read", "passed")?;
    match mode {
        CONNECTION_MODE_WORKFLOW => {
            expect_report_field(&report, "project_state_write", "passed")?;
            expect_report_field(&report, "effective_tool_mode", "workflow")?;
        }
        CONNECTION_MODE_READ_ONLY => {
            expect_report_field(&report, "project_state_write", "passed")?;
            expect_report_field(&report, "effective_tool_mode", "read_only")?;
        }
        other => return Err(format!("unsupported connection mode: {other}")),
    }
    expect_report_field(&report, "tools_list_schema_validation", "passed")?;
    Ok(McpPreflightDiagnostics::from_preflight_report(&report))
}

fn parse_colon_report(stdout: &str) -> Result<BTreeMap<String, String>, String> {
    let mut report = BTreeMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once(':') {
            report.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    if report.is_empty() {
        Err("preflight did not return a key-value report".to_owned())
    } else {
        Ok(report)
    }
}

fn expect_report_field(
    report: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    match report.get(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!(
            "preflight field {key} was {actual}, expected {expected}"
        )),
        None => Err(format!("preflight field {key} was missing")),
    }
}

pub(super) fn materialize_connection_invocation(
    launch: &ManagedMcpLaunchSpec,
    runtime_home: &Path,
    repo_root: &Path,
    purpose: ManagedMcpInvocationPurpose,
) -> Result<MaterializedManagedMcpLaunch, volicord_mcp::ManagedMcpLaunchError> {
    let mut forwarded_environment = BTreeMap::new();
    if launch
        .environment()
        .forwarded_names()
        .contains(VOLICORD_HOME_ENV)
    {
        forwarded_environment.insert(
            VOLICORD_HOME_ENV.to_owned(),
            runtime_home.as_os_str().to_owned(),
        );
    }
    let working_directory = match launch.host_scope() {
        volicord_types::HostScope::User => ManagedMcpWorkingDirectory::Inherited,
        volicord_types::HostScope::Project => {
            ManagedMcpWorkingDirectory::ProductRepository(repo_root.to_path_buf())
        }
    };
    launch.materialize(ManagedMcpMaterializationInput::new(
        purpose,
        forwarded_environment,
        working_directory,
    ))
}

fn verify_mcp_stdio_process(
    launch: &MaterializedManagedMcpLaunch,
    mode: &str,
    timeout: Duration,
) -> Result<McpVerification, McpProcessFailure> {
    verify_mcp_stdio_command(launch.process_command(), mode, timeout)
}

fn verify_mcp_stdio_command(
    mut command: Command,
    mode: &str,
    timeout: Duration,
) -> Result<McpVerification, McpProcessFailure> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| McpProcessFailure::Spawn {
        stage: McpStage::Startup,
        io_detail: bounded_io_detail(error),
    })?;
    let deadline = Instant::now() + timeout;

    let Some(stderr) = child.stderr.take() else {
        let cleanup = terminate_and_reap(&mut child).err();
        return Err(cleanup.map_or_else(
            || McpProcessFailure::Read {
                stage: McpStage::Startup,
                io_detail: bounded_io_text("MCP stderr pipe was unavailable"),
                stderr: BoundedText::empty(),
            },
            |error| McpProcessFailure::Wait {
                stage: McpStage::Startup,
                io_detail: bounded_io_text(error),
                stderr: BoundedText::empty(),
            },
        ));
    };
    let stderr_reader = thread::spawn(move || drain_stderr(stderr));

    let Some(stdout) = child.stdout.take() else {
        let cleanup = terminate_and_reap(&mut child).err();
        let readers = join_stderr_only(stderr_reader);
        let failure = cleanup.map_or_else(
            || McpProcessFailure::Read {
                stage: McpStage::Startup,
                io_detail: bounded_io_text("MCP stdout pipe was unavailable"),
                stderr: BoundedText::empty(),
            },
            |error| McpProcessFailure::Wait {
                stage: McpStage::Startup,
                io_detail: bounded_io_text(error),
                stderr: BoundedText::empty(),
            },
        );
        return Err(apply_reader_completion(failure, readers));
    };
    let (stdout_tx, stdout_rx) = mpsc::sync_channel(READER_CHANNEL_CAPACITY);
    let stdout_reader = thread::spawn(move || read_stdout_lines(BufReader::new(stdout), stdout_tx));
    let readers = ReaderHandles {
        stdout: stdout_reader,
        stderr: stderr_reader,
    };

    let Some(mut stdin) = child.stdin.take() else {
        let cleanup = terminate_and_reap(&mut child).err();
        let failure = cleanup.map_or_else(
            || McpProcessFailure::Write {
                stage: McpStage::Startup,
                io_detail: bounded_io_text("MCP stdin pipe was unavailable"),
                stderr: BoundedText::empty(),
            },
            |error| McpProcessFailure::Wait {
                stage: McpStage::Startup,
                io_detail: bounded_io_text(error),
                stderr: BoundedText::empty(),
            },
        );
        return Err(finish_failure(failure, stdout_rx, readers));
    };

    let exchange = perform_mcp_exchange(&mut stdin, &stdout_rx, deadline, timeout, mode);
    drop(stdin);
    match exchange {
        Ok(tools) => match wait_for_child_until(&mut child, deadline) {
            ChildWait::Exited(status) if status.success() => {
                finish_success(stdout_rx, readers)?;
                Ok(McpVerification::passed(tools))
            }
            ChildWait::Exited(status) => Err(finish_failure(
                McpProcessFailure::Shutdown {
                    stage: McpStage::Shutdown,
                    exit_code: status.code(),
                    stderr: BoundedText::empty(),
                },
                stdout_rx,
                readers,
            )),
            ChildWait::TimedOut => {
                let cleanup = terminate_and_reap(&mut child).err();
                let failure = cleanup.map_or_else(
                    || McpProcessFailure::Timeout {
                        stage: McpStage::Shutdown,
                        timeout,
                        stderr: BoundedText::empty(),
                    },
                    |error| McpProcessFailure::Wait {
                        stage: McpStage::Shutdown,
                        io_detail: bounded_io_text(error),
                        stderr: BoundedText::empty(),
                    },
                );
                Err(finish_failure(failure, stdout_rx, readers))
            }
            ChildWait::Failed(error) => {
                let cleanup = terminate_and_reap(&mut child).err();
                let detail = cleanup.unwrap_or(error);
                Err(finish_failure(
                    McpProcessFailure::Wait {
                        stage: McpStage::Shutdown,
                        io_detail: bounded_io_text(detail),
                        stderr: BoundedText::empty(),
                    },
                    stdout_rx,
                    readers,
                ))
            }
        },
        Err(PendingMcpFailure::Eof { stage }) => {
            let failure = match wait_for_child_until(&mut child, deadline) {
                ChildWait::Exited(status) => McpProcessFailure::ExitedBeforeResponse {
                    stage,
                    exit_code: status.code(),
                    stderr: BoundedText::empty(),
                },
                ChildWait::TimedOut => {
                    let cleanup = terminate_and_reap(&mut child).err();
                    cleanup.map_or_else(
                        || McpProcessFailure::Timeout {
                            stage,
                            timeout,
                            stderr: BoundedText::empty(),
                        },
                        |error| McpProcessFailure::Wait {
                            stage,
                            io_detail: bounded_io_text(error),
                            stderr: BoundedText::empty(),
                        },
                    )
                }
                ChildWait::Failed(error) => {
                    let cleanup = terminate_and_reap(&mut child).err();
                    McpProcessFailure::Wait {
                        stage,
                        io_detail: bounded_io_text(cleanup.unwrap_or(error)),
                        stderr: BoundedText::empty(),
                    }
                }
            };
            Err(finish_failure(failure, stdout_rx, readers))
        }
        Err(pending) => {
            let stage = pending.stage();
            if pending.may_be_early_exit() {
                let status_deadline = Instant::now()
                    + EARLY_EXIT_STATUS_WAIT
                        .min(deadline.saturating_duration_since(Instant::now()));
                match wait_for_child_until(&mut child, status_deadline) {
                    ChildWait::Exited(status) => {
                        return Err(finish_failure(
                            McpProcessFailure::ExitedBeforeResponse {
                                stage,
                                exit_code: status.code(),
                                stderr: BoundedText::empty(),
                            },
                            stdout_rx,
                            readers,
                        ));
                    }
                    ChildWait::Failed(error) => {
                        let cleanup = terminate_and_reap(&mut child).err();
                        return Err(finish_failure(
                            McpProcessFailure::Wait {
                                stage,
                                io_detail: bounded_io_text(cleanup.unwrap_or(error)),
                                stderr: BoundedText::empty(),
                            },
                            stdout_rx,
                            readers,
                        ));
                    }
                    ChildWait::TimedOut => {}
                }
            }
            let cleanup = terminate_and_reap(&mut child).err();
            let failure = cleanup.map_or_else(
                || pending.into_failure(timeout),
                |error| McpProcessFailure::Wait {
                    stage,
                    io_detail: bounded_io_text(error),
                    stderr: BoundedText::empty(),
                },
            );
            Err(finish_failure(failure, stdout_rx, readers))
        }
    }
}

fn perform_mcp_exchange(
    stdin: &mut impl Write,
    stdout_rx: &Receiver<StdoutEvent>,
    deadline: Instant,
    timeout: Duration,
    mode: &str,
) -> Result<Vec<String>, PendingMcpFailure> {
    write_json_line(
        stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "volicord-cli", "version": env!("CARGO_PKG_VERSION")}
            }
        }),
        McpStage::Initialize,
    )?;
    let initialize = read_json_response(stdout_rx, deadline, timeout, McpStage::Initialize)?;
    validate_initialize_response(&initialize)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::Initialize, problem))?;

    write_json_line(
        stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        McpStage::ToolsList,
    )?;
    write_json_line(
        stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
        McpStage::ToolsList,
    )?;
    let tools_response = read_json_response(stdout_rx, deadline, timeout, McpStage::ToolsList)?;
    let tools = validate_tools_response(&tools_response)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::ToolsList, problem))?;
    validate_tools_for_mode_problem(mode, &tools)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::ToolsList, problem))?;

    write_json_line(
        stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": LIST_PROJECTS_TOOL_NAME,
                "arguments": {}
            }
        }),
        McpStage::SafeToolCall,
    )?;
    let safe_response = read_json_response(stdout_rx, deadline, timeout, McpStage::SafeToolCall)?;
    validate_safe_tool_response(&safe_response)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::SafeToolCall, problem))?;
    Ok(tools)
}

#[derive(Debug)]
enum PendingMcpFailure {
    Eof {
        stage: McpStage,
    },
    Timeout {
        stage: McpStage,
    },
    Read {
        stage: McpStage,
        detail: String,
    },
    Write {
        stage: McpStage,
        detail: String,
    },
    Protocol {
        stage: McpStage,
        problem: ProtocolProblem,
    },
}

impl PendingMcpFailure {
    fn protocol(stage: McpStage, problem: ProtocolProblem) -> Self {
        Self::Protocol { stage, problem }
    }

    const fn stage(&self) -> McpStage {
        match self {
            Self::Eof { stage }
            | Self::Timeout { stage }
            | Self::Read { stage, .. }
            | Self::Write { stage, .. }
            | Self::Protocol { stage, .. } => *stage,
        }
    }

    const fn may_be_early_exit(&self) -> bool {
        matches!(self, Self::Read { .. } | Self::Write { .. })
    }

    fn into_failure(self, timeout: Duration) -> McpProcessFailure {
        match self {
            Self::Eof { stage } => McpProcessFailure::Read {
                stage,
                io_detail: bounded_io_text("MCP stdout ended unexpectedly"),
                stderr: BoundedText::empty(),
            },
            Self::Timeout { stage } => McpProcessFailure::Timeout {
                stage,
                timeout,
                stderr: BoundedText::empty(),
            },
            Self::Read { stage, detail } => McpProcessFailure::Read {
                stage,
                io_detail: bounded_io_text(detail),
                stderr: BoundedText::empty(),
            },
            Self::Write { stage, detail } => McpProcessFailure::Write {
                stage,
                io_detail: bounded_io_text(detail),
                stderr: BoundedText::empty(),
            },
            Self::Protocol { stage, problem } => McpProcessFailure::Protocol {
                stage,
                protocol_detail: bounded_protocol_detail(problem.detail),
                missing_tools: problem.missing_tools,
                stderr: BoundedText::empty(),
            },
        }
    }
}

#[derive(Debug)]
enum StdoutEvent {
    Line(Vec<u8>),
    Eof,
    ReadFailed(String),
    LineTooLong { observed_bytes: usize },
    IncompleteLine { observed_bytes: usize },
}

fn read_stdout_lines(mut reader: impl BufRead, sender: SyncSender<StdoutEvent>) {
    loop {
        let mut line = Vec::with_capacity(MAX_PROTOCOL_LINE_BYTES.min(8 * 1024));
        let read = reader
            .by_ref()
            .take((MAX_PROTOCOL_LINE_BYTES + 2) as u64)
            .read_until(b'\n', &mut line);
        let event = match read {
            Ok(0) => StdoutEvent::Eof,
            Ok(_) if line.last() != Some(&b'\n') && line.len() > MAX_PROTOCOL_LINE_BYTES => {
                StdoutEvent::LineTooLong {
                    observed_bytes: line.len(),
                }
            }
            Ok(_) if line.last() != Some(&b'\n') => StdoutEvent::IncompleteLine {
                observed_bytes: line.len(),
            },
            Ok(_) => {
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                if line.len() > MAX_PROTOCOL_LINE_BYTES {
                    StdoutEvent::LineTooLong {
                        observed_bytes: line.len(),
                    }
                } else {
                    StdoutEvent::Line(line)
                }
            }
            Err(error) => StdoutEvent::ReadFailed(error.to_string()),
        };
        let terminal = !matches!(event, StdoutEvent::Line(_));
        if sender.send(event).is_err() || terminal {
            break;
        }
    }
}

fn write_json_line(
    writer: &mut impl Write,
    value: Value,
    stage: McpStage,
) -> Result<(), PendingMcpFailure> {
    serde_json::to_writer(&mut *writer, &value).map_err(|error| PendingMcpFailure::Write {
        stage,
        detail: error.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|error| PendingMcpFailure::Write {
            stage,
            detail: error.to_string(),
        })
}

fn read_json_response(
    receiver: &Receiver<StdoutEvent>,
    deadline: Instant,
    _timeout: Duration,
    stage: McpStage,
) -> Result<Value, PendingMcpFailure> {
    let now = Instant::now();
    if now >= deadline {
        return Err(PendingMcpFailure::Timeout { stage });
    }
    match receiver.recv_timeout(deadline.saturating_duration_since(now)) {
        Ok(StdoutEvent::Line(line)) => serde_json::from_slice::<Value>(&line).map_err(|error| {
            PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(format!(
                    "response was not valid JSON at line {} column {}",
                    error.line(),
                    error.column()
                )),
            )
        }),
        Ok(StdoutEvent::Eof) => Err(PendingMcpFailure::Eof { stage }),
        Ok(StdoutEvent::ReadFailed(error)) => {
            Err(PendingMcpFailure::Read { stage, detail: error })
        }
        Ok(StdoutEvent::LineTooLong { observed_bytes }) => Err(PendingMcpFailure::protocol(
            stage,
            ProtocolProblem::new(format!(
                "response line exceeded the {MAX_PROTOCOL_LINE_BYTES}-byte limit (observed at least {observed_bytes} bytes)"
            )),
        )),
        Ok(StdoutEvent::IncompleteLine { observed_bytes }) => {
            Err(PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(format!(
                    "response ended without newline-delimited framing after {observed_bytes} bytes"
                )),
            ))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Err(PendingMcpFailure::Timeout { stage }),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(PendingMcpFailure::Read {
            stage,
            detail: "MCP stdout reader disconnected".to_owned(),
        }),
    }
}

#[derive(Debug)]
struct ProtocolProblem {
    detail: String,
    missing_tools: Vec<String>,
}

impl ProtocolProblem {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            missing_tools: Vec::new(),
        }
    }

    fn missing_tools(missing_tools: Vec<String>) -> Self {
        Self {
            detail: format!(
                "tools/list omitted {} required tool(s)",
                missing_tools.len()
            ),
            missing_tools,
        }
    }
}

fn response_error(value: &Value, operation: &str) -> Option<ProtocolProblem> {
    let error = value.get("error")?;
    let detail = error.get("code").and_then(Value::as_i64).map_or_else(
        || format!("{operation} response returned a JSON-RPC error"),
        |code| format!("{operation} response returned JSON-RPC error code {code}"),
    );
    Some(ProtocolProblem::new(detail))
}

fn validate_initialize_response(value: &Value) -> Result<(), ProtocolProblem> {
    if let Some(problem) = response_error(value, "initialize") {
        return Err(problem);
    }
    let result = value
        .get("result")
        .ok_or_else(|| ProtocolProblem::new("initialize response was missing result"))?;
    if result
        .get("instructions")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(ProtocolProblem::new(
            "initialize response was missing nonempty instructions",
        ));
    }
    Ok(())
}

fn validate_tools_response(value: &Value) -> Result<Vec<String>, ProtocolProblem> {
    if let Some(problem) = response_error(value, "tools/list") {
        return Err(problem);
    }
    let tools = value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolProblem::new("tools/list response was missing result.tools"))?;
    let mut names = Vec::new();
    for tool in tools {
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| ProtocolProblem::new("tools/list contained a tool without a name"))?;
        names.push(name.to_owned());
    }
    Ok(names)
}

fn validate_safe_tool_response(value: &Value) -> Result<(), ProtocolProblem> {
    if let Some(problem) = response_error(value, "designated read-only tool call") {
        return Err(problem);
    }
    let result = value.get("result").ok_or_else(|| {
        ProtocolProblem::new("designated read-only tool response was missing result")
    })?;
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(ProtocolProblem::new(
            "designated read-only tool response set isError=true",
        ));
    }
    Ok(())
}

fn validate_tools_for_mode_problem(mode: &str, tools: &[String]) -> Result<(), ProtocolProblem> {
    match mode {
        CONNECTION_MODE_READ_ONLY => {
            validate_required_tools_problem(tools, read_only_required_tool_names())
        }
        CONNECTION_MODE_WORKFLOW => {
            validate_required_tools_problem(tools, workflow_required_tool_names())
        }
        other => Err(ProtocolProblem::new(format!(
            "unsupported connection mode for tool validation: {other}"
        ))),
    }
}

fn validate_required_tools_problem(
    tools: &[String],
    expected: impl IntoIterator<Item = &'static str>,
) -> Result<(), ProtocolProblem> {
    let missing_tools = expected
        .into_iter()
        .filter(|name| !tools.iter().any(|tool| tool == name))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if missing_tools.is_empty() {
        Ok(())
    } else {
        Err(ProtocolProblem::missing_tools(missing_tools))
    }
}

pub(super) fn workflow_required_tool_names() -> impl Iterator<Item = &'static str> {
    WORKFLOW_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

pub(super) fn read_only_required_tool_names() -> impl Iterator<Item = &'static str> {
    READ_ONLY_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

#[derive(Debug)]
struct StderrReaderResult {
    captured: BoundedText,
    read_error: Option<String>,
}

fn drain_stderr(mut reader: impl Read) -> StderrReaderResult {
    drain_bounded_stream(&mut reader, MAX_CAPTURED_STDERR_BYTES, "stderr")
}

fn drain_bounded_stream(mut reader: impl Read, limit: usize, label: &str) -> StderrReaderResult {
    let mut captured = Vec::with_capacity(limit);
    let mut omitted_bytes = 0usize;
    let mut chunk = [0u8; 4 * 1024];
    let read_error = loop {
        match reader.read(&mut chunk) {
            Ok(0) => break None,
            Ok(read) => {
                let remaining = limit.saturating_sub(captured.len());
                let retained = remaining.min(read);
                captured.extend_from_slice(&chunk[..retained]);
                omitted_bytes = omitted_bytes.saturating_add(read - retained);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => break Some(error.to_string()),
        }
    };
    StderrReaderResult {
        captured: BoundedText::from_bytes(captured, omitted_bytes, label),
        read_error,
    }
}

struct ReaderHandles {
    stdout: JoinHandle<()>,
    stderr: JoinHandle<StderrReaderResult>,
}

struct ReaderCompletion {
    stderr: BoundedText,
    read_error: Option<String>,
}

fn join_readers(handles: ReaderHandles) -> ReaderCompletion {
    let stdout_error = handles
        .stdout
        .join()
        .err()
        .map(|_| "MCP stdout reader panicked".to_owned());
    let stderr = handles.stderr.join();
    match stderr {
        Ok(stderr) => ReaderCompletion {
            stderr: stderr.captured,
            read_error: stdout_error.or(stderr.read_error),
        },
        Err(_) => ReaderCompletion {
            stderr: BoundedText::empty(),
            read_error: stdout_error.or_else(|| Some("MCP stderr reader panicked".to_owned())),
        },
    }
}

fn join_stderr_only(stderr: JoinHandle<StderrReaderResult>) -> ReaderCompletion {
    match stderr.join() {
        Ok(stderr) => ReaderCompletion {
            stderr: stderr.captured,
            read_error: stderr.read_error,
        },
        Err(_) => ReaderCompletion {
            stderr: BoundedText::empty(),
            read_error: Some("MCP stderr reader panicked".to_owned()),
        },
    }
}

fn apply_reader_completion(
    failure: McpProcessFailure,
    completion: ReaderCompletion,
) -> McpProcessFailure {
    if let Some(error) = completion.read_error {
        McpProcessFailure::Read {
            stage: failure.stage(),
            io_detail: bounded_io_text(error),
            stderr: completion.stderr,
        }
    } else {
        failure.with_stderr(completion.stderr)
    }
}

fn finish_failure(
    failure: McpProcessFailure,
    receiver: Receiver<StdoutEvent>,
    readers: ReaderHandles,
) -> McpProcessFailure {
    drop(receiver);
    apply_reader_completion(failure, join_readers(readers))
}

fn finish_success(
    receiver: Receiver<StdoutEvent>,
    readers: ReaderHandles,
) -> Result<(), McpProcessFailure> {
    drop(receiver);
    let completion = join_readers(readers);
    if let Some(error) = completion.read_error {
        Err(McpProcessFailure::Read {
            stage: McpStage::Shutdown,
            io_detail: bounded_io_text(error),
            stderr: completion.stderr,
        })
    } else {
        Ok(())
    }
}

enum ChildWait {
    Exited(ExitStatus),
    TimedOut,
    Failed(String),
}

fn wait_for_child_until(child: &mut Child, deadline: Instant) -> ChildWait {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return ChildWait::Exited(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(CHILD_WAIT_POLL_INTERVAL),
            Ok(None) => return ChildWait::TimedOut,
            Err(error) => return ChildWait::Failed(error.to_string()),
        }
    }
}

fn terminate_and_reap(child: &mut Child) -> Result<ExitStatus, String> {
    let inspection_error = match child.try_wait() {
        Ok(Some(status)) => return Ok(status),
        Ok(None) => None,
        Err(error) => Some(format!("failed to inspect MCP process status: {error}")),
    };
    let kill_error = child.kill().err();
    match child.wait() {
        Ok(status) => inspection_error.map_or(Ok(status), Err),
        Err(wait_error) => {
            let cleanup_error = kill_error.map_or_else(
                || format!("failed to reap MCP process: {wait_error}"),
                |kill_error| {
                    format!(
                        "failed to terminate MCP process: {kill_error}; failed to reap MCP process: {wait_error}"
                    )
                },
            );
            Err(match inspection_error {
                Some(inspection_error) => format!("{inspection_error}; {cleanup_error}"),
                None => cleanup_error,
            })
        }
    }
}

fn bounded_protocol_detail(detail: impl AsRef<str>) -> BoundedText {
    BoundedText::from_utf8(detail, MAX_PROTOCOL_DETAIL_BYTES, "protocol detail")
}

fn bounded_io_detail(error: impl std::fmt::Display) -> BoundedText {
    bounded_io_text(error.to_string())
}

fn bounded_io_text(detail: impl AsRef<str>) -> BoundedText {
    BoundedText::from_utf8(detail, MAX_IO_DETAIL_BYTES, "I/O detail")
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn append_diagnostic_context(summary: String, context: &str) -> String {
    if context.is_empty() {
        summary
    } else {
        format!("{summary}\n{context}")
    }
}

fn compact_stream(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Cursor,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static TEST_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn shared_verification_materializes_selected_runtime_home_and_repository() {
        let launch = ManagedMcpLaunchSpec::shared_repository(volicord_types::HostKind::Codex)
            .expect("shared launch");
        let materialized = materialize_connection_invocation(
            &launch,
            Path::new("/selected/runtime-home"),
            Path::new("/workspace/product"),
            ManagedMcpInvocationPurpose::CliStdioHandshake,
        )
        .expect("shared verification launch");
        assert_eq!(
            materialized.environment().get(VOLICORD_HOME_ENV),
            Some(&OsString::from("/selected/runtime-home"))
        );
        assert_eq!(
            materialized.working_directory(),
            &ManagedMcpWorkingDirectory::ProductRepository(PathBuf::from("/workspace/product"))
        );
    }

    #[test]
    fn personal_verification_uses_static_runtime_home_and_repository_independent_cwd() {
        let launch = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord"),
            Path::new("/contract/runtime-home"),
            "connection_alpha",
            None,
        )
        .expect("personal launch");
        let materialized = materialize_connection_invocation(
            &launch,
            Path::new("/decoy/selected-runtime-home"),
            Path::new("/workspace/product"),
            ManagedMcpInvocationPurpose::CliStdioHandshake,
        )
        .expect("personal verification launch");
        assert_eq!(
            materialized.environment().get(VOLICORD_HOME_ENV),
            Some(&OsString::from("/contract/runtime-home"))
        );
        assert_eq!(
            materialized.working_directory(),
            &ManagedMcpWorkingDirectory::Inherited
        );
    }

    #[test]
    fn preflight_requires_current_storage_and_tool_schema_checks() {
        let report = "configuration: valid\ntransport: stdio\nconnection_id: connection_fixture\nmode: workflow\nenabled: true\nregistry_read: passed\nproject_state_read: passed\nproject_state_write: passed\neffective_tool_mode: workflow\ntools_list_schema_validation: passed\n";
        assert!(validate_connection_preflight_report(
            report,
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_ok());
        assert!(validate_connection_preflight_report(
            &report.replace("registry_read: passed\n", ""),
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_err());
        let read_only = report.replace("mode: workflow", "mode: read_only").replace(
            "effective_tool_mode: workflow",
            "effective_tool_mode: read_only",
        );
        assert!(validate_connection_preflight_report(
            &read_only,
            "connection_fixture",
            CONNECTION_MODE_READ_ONLY
        )
        .is_ok());
        assert!(validate_connection_preflight_report(
            &report.replace(
                "tools_list_schema_validation: passed",
                "tools_list_schema_validation: failed"
            ),
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_err());
    }

    #[test]
    fn safe_tool_call_rejects_json_rpc_and_tool_errors() {
        assert!(validate_safe_tool_response(
            &json!({"jsonrpc": "2.0", "id": 3, "result": {"content": []}})
        )
        .is_ok());
        assert!(validate_safe_tool_response(
            &json!({"jsonrpc": "2.0", "id": 3, "error": {"code": -32603}})
        )
        .is_err());
        assert!(validate_safe_tool_response(
            &json!({"jsonrpc": "2.0", "id": 3, "result": {"isError": true}})
        )
        .is_err());
    }

    #[test]
    fn typed_failures_map_directly_to_current_check_codes() {
        let cases = [
            (
                McpProcessFailure::Spawn {
                    stage: McpStage::Startup,
                    io_detail: bounded_io_text("not found"),
                },
                "mcp_server_process_failed",
            ),
            (
                McpProcessFailure::Timeout {
                    stage: McpStage::Initialize,
                    timeout: Duration::from_millis(50),
                    stderr: BoundedText::empty(),
                },
                "mcp_server_initialize_failed",
            ),
            (
                McpProcessFailure::protocol(McpStage::ToolsList, "tools/list response was invalid"),
                "mcp_server_tools_list_failed",
            ),
            (
                McpProcessFailure::protocol(
                    McpStage::SafeToolCall,
                    "designated read-only tool response was invalid",
                ),
                "mcp_server_safe_call_failed",
            ),
            (
                McpProcessFailure::Wait {
                    stage: McpStage::Shutdown,
                    io_detail: bounded_io_text("wait failed"),
                    stderr: BoundedText::empty(),
                },
                "mcp_server_process_failed",
            ),
        ];
        for (failure, expected) in cases {
            assert_eq!(failure.check_code(), expected);
        }
    }

    #[test]
    fn stderr_capture_has_a_deterministic_byte_bound_and_truncation_marker() {
        let result = drain_stderr(Cursor::new(vec![b'x'; MAX_CAPTURED_STDERR_BYTES + 17]));
        assert!(result.read_error.is_none());
        assert!(result.captured.truncated);
        assert_eq!(result.captured.omitted_bytes, 17);
        assert!(result
            .captured
            .text
            .ends_with("...[stderr truncated; 17 bytes omitted]"));
    }

    #[test]
    fn protocol_reader_rejects_oversized_newline_delimited_lines() {
        let mut input = vec![b'x'; MAX_PROTOCOL_LINE_BYTES + 1];
        input.push(b'\n');
        let (sender, receiver) = mpsc::sync_channel(READER_CHANNEL_CAPACITY);
        read_stdout_lines(BufReader::new(Cursor::new(input)), sender);
        assert!(matches!(
            receiver.recv().expect("reader event"),
            StdoutEvent::LineTooLong { observed_bytes }
                if observed_bytes == MAX_PROTOCOL_LINE_BYTES + 1
        ));
    }

    #[test]
    fn disconnected_stdout_reader_is_a_typed_read_failure() {
        let (sender, receiver) = mpsc::sync_channel(READER_CHANNEL_CAPACITY);
        drop(sender);
        let failure = read_json_response(
            &receiver,
            Instant::now() + Duration::from_secs(1),
            Duration::from_secs(1),
            McpStage::Initialize,
        )
        .expect_err("reader disconnection must fail");
        assert!(matches!(
            failure,
            PendingMcpFailure::Read {
                stage: McpStage::Initialize,
                ..
            }
        ));
    }

    #[test]
    fn spawn_failure_is_typed_without_child_state() {
        let missing = std::env::temp_dir().join(format!(
            "volicord-missing-mcp-process-{}-{}",
            std::process::id(),
            TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let failure = verify_mcp_stdio_command(
            Command::new(missing),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_millis(50),
        )
        .expect_err("missing process must not spawn");
        assert!(matches!(
            failure,
            McpProcessFailure::Spawn {
                stage: McpStage::Startup,
                ..
            }
        ));
    }

    #[cfg(unix)]
    fn shell_command(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    #[cfg(unix)]
    fn read_only_tools_response() -> String {
        let tools = read_only_required_tool_names()
            .map(|name| json!({"name": name}))
            .collect::<Vec<_>>();
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"tools": tools},
        })
        .to_string()
    }

    #[cfg(unix)]
    fn protocol_script(
        prefix: &str,
        tools_response: &str,
        safe_response: &str,
        suffix: &str,
    ) -> String {
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"instructions": "fixture"},
        });
        format!(
            concat!(
                "{prefix}\n",
                "IFS= read -r initialize_request\n",
                "printf '%s\\n' '{initialize}'\n",
                "IFS= read -r initialized_notification\n",
                "IFS= read -r tools_request\n",
                "printf '%s\\n' '{tools_response}'\n",
                "IFS= read -r safe_request\n",
                "printf '%s\\n' '{safe_response}'\n",
                "{suffix}\n",
            ),
            prefix = prefix,
            initialize = initialize,
            tools_response = tools_response,
            safe_response = safe_response,
            suffix = suffix,
        )
    }

    #[cfg(unix)]
    fn successful_protocol_script(prefix: &str, suffix: &str) -> String {
        protocol_script(
            prefix,
            &read_only_tools_response(),
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "result": {"content": []},
            })
            .to_string(),
            suffix,
        )
    }

    #[cfg(unix)]
    #[test]
    fn exit_before_initialize_reports_status_and_bounded_stderr() {
        let failure = verify_mcp_stdio_command(
            shell_command("printf '%s\\n' 'fixture startup failure' >&2; exit 23"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        )
        .expect_err("early exit must fail");
        match failure {
            McpProcessFailure::ExitedBeforeResponse {
                stage,
                exit_code,
                stderr,
            } => {
                assert_eq!(stage, McpStage::Initialize);
                assert_eq!(exit_code, Some(23));
                assert_eq!(stderr.text.trim(), "fixture startup failure");
                assert!(!stderr.truncated);
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn initialize_timeout_terminates_and_reaps_the_child() {
        let failure = verify_mcp_stdio_command(
            shell_command("printf '%s\\n' 'waiting for initialize' >&2; while :; do :; done"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_millis(50),
        )
        .expect_err("initialize must time out");
        match failure {
            McpProcessFailure::Timeout { stage, stderr, .. } => {
                assert_eq!(stage, McpStage::Initialize);
                assert!(stderr.text.contains("waiting for initialize"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn tools_list_failure_retains_its_typed_stage() {
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"instructions": "fixture"},
        });
        let tools_error = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "error": {"code": -32603, "message": "fixture prose is not classified"},
        });
        let script = format!(
            concat!(
                "IFS= read -r initialize_request\n",
                "printf '%s\\n' '{initialize}'\n",
                "IFS= read -r initialized_notification\n",
                "IFS= read -r tools_request\n",
                "printf '%s\\n' '{tools_error}'\n",
            ),
            initialize = initialize,
            tools_error = tools_error,
        );
        let failure = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        )
        .expect_err("tools/list error must fail");
        match failure {
            McpProcessFailure::Protocol {
                stage,
                protocol_detail,
                ..
            } => {
                assert_eq!(stage, McpStage::ToolsList);
                assert!(protocol_detail.text.contains("-32603"));
                assert!(!protocol_detail.text.contains("fixture prose"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn safe_tool_call_failure_retains_its_typed_stage() {
        let safe_error = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "error": {"code": -32603, "message": "ignored child prose"},
        });
        let script = protocol_script("", &read_only_tools_response(), &safe_error.to_string(), "");
        let failure = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        )
        .expect_err("designated read-only call error must fail");
        assert!(matches!(
            failure,
            McpProcessFailure::Protocol {
                stage: McpStage::SafeToolCall,
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn malformed_json_is_a_bounded_protocol_failure_without_raw_line_echo() {
        let failure = verify_mcp_stdio_command(
            shell_command("IFS= read -r request; printf '%s\\n' '{not-json}'"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        )
        .expect_err("malformed JSON must fail");
        match failure {
            McpProcessFailure::Protocol {
                stage,
                protocol_detail,
                ..
            } => {
                assert_eq!(stage, McpStage::Initialize);
                assert!(protocol_detail.text.contains("not valid JSON"));
                assert!(!protocol_detail.text.contains("{not-json}"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn large_stderr_is_truncated_after_an_early_exit() {
        let chunk = "x".repeat(1024);
        let script = format!(
            "i=0; while [ \"$i\" -lt 8 ]; do printf '%s' '{chunk}' >&2; i=$((i + 1)); done; exit 19"
        );
        let failure = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        )
        .expect_err("early exit must fail");
        match failure {
            McpProcessFailure::ExitedBeforeResponse { stderr, .. } => {
                assert!(stderr.truncated);
                assert_eq!(stderr.omitted_bytes, 6 * 1024);
                assert!(stderr
                    .text
                    .ends_with("...[stderr truncated; 6144 bytes omitted]"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn stderr_is_drained_while_waiting_for_initialize() {
        let chunk = "x".repeat(1024);
        let prefix = format!(
            "i=0; while [ \"$i\" -lt 256 ]; do printf '%s' '{chunk}' >&2; i=$((i + 1)); done"
        );
        let script = successful_protocol_script(&prefix, "cat >/dev/null");
        let verification = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(3),
        )
        .expect("stderr pipe pressure must not block initialize");
        assert_eq!(verification.step.status.as_str(), "passed");
    }

    #[cfg(unix)]
    #[test]
    fn successful_shutdown_observes_eof_and_reaps_the_child() {
        let marker = std::env::temp_dir().join(format!(
            "volicord-mcp-shutdown-{}-{}",
            std::process::id(),
            TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let script =
            successful_protocol_script("", "cat >/dev/null; printf '%s' 'reaped' > \"$1\"");
        let mut command = shell_command(&script);
        command.arg("fixture").arg(&marker);
        let verification =
            verify_mcp_stdio_command(command, CONNECTION_MODE_READ_ONLY, Duration::from_secs(2))
                .expect("graceful child shutdown");
        assert_eq!(verification.step.status.as_str(), "passed");
        assert_eq!(
            fs::read_to_string(&marker).expect("shutdown marker"),
            "reaped"
        );
        fs::remove_file(marker).expect("remove shutdown marker");
    }
}
