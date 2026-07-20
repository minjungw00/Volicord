use std::{process::Command, time::Duration};

use serde_json::{json, Value};
use volicord_mcp::MaterializedManagedMcpLaunch;
use volicord_store::agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW};
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, LIST_PROJECTS_TOOL_NAME, READ_ONLY_METHOD_TOOL_NAMES,
    WORKFLOW_METHOD_TOOL_NAMES,
};

use super::{
    failure::{bounded_protocol_detail, BoundedText, McpProcessFailure, McpStage},
    supervisor::{
        ChildSupervisor, ProtocolEvent, ProtocolRead, SupervisorKind, MAX_PROTOCOL_LINE_BYTES,
    },
};

const EARLY_EXIT_STATUS_WAIT: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpExchangeProgress {
    pub(in crate::connection_command) initialize_completed: bool,
    pub(in crate::connection_command) tools_list: Option<Vec<String>>,
    pub(in crate::connection_command) required_tools_validated: bool,
    pub(in crate::connection_command) safe_tool_call_completed: bool,
    pub(in crate::connection_command) shutdown_completed: bool,
}

impl McpExchangeProgress {
    pub fn not_started() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(in crate::connection_command) fn observed(
        initialize_completed: bool,
        tools_list: Option<Vec<String>>,
        required_tools_validated: bool,
        safe_tool_call_completed: bool,
        shutdown_completed: bool,
    ) -> Self {
        Self {
            initialize_completed,
            tools_list,
            required_tools_validated,
            safe_tool_call_completed,
            shutdown_completed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpExchangeOutcome {
    pub(in crate::connection_command) progress: McpExchangeProgress,
    pub(in crate::connection_command) failure: Option<McpProcessFailure>,
}

impl McpExchangeOutcome {
    pub fn failed(progress: McpExchangeProgress, failure: McpProcessFailure) -> Self {
        Self {
            progress,
            failure: Some(failure),
        }
    }

    pub(in crate::connection_command) fn completed(progress: McpExchangeProgress) -> Self {
        debug_assert!(progress.initialize_completed);
        debug_assert!(progress.tools_list.is_some());
        debug_assert!(progress.required_tools_validated);
        debug_assert!(progress.safe_tool_call_completed);
        debug_assert!(progress.shutdown_completed);
        Self {
            progress,
            failure: None,
        }
    }
}

pub(super) fn verify_mcp_stdio_process(
    launch: &MaterializedManagedMcpLaunch,
    mode: &str,
    timeout: Duration,
) -> McpExchangeOutcome {
    verify_mcp_stdio_command(launch.process_command(), mode, timeout)
}

fn verify_mcp_stdio_command(command: Command, mode: &str, timeout: Duration) -> McpExchangeOutcome {
    let mut supervisor = match ChildSupervisor::spawn(command, SupervisorKind::Stdio, timeout) {
        Ok(supervisor) => supervisor,
        Err(failure) => {
            return McpExchangeOutcome::failed(McpExchangeProgress::not_started(), failure)
        }
    };

    let exchange = perform_mcp_exchange(&mut supervisor, mode);
    supervisor.close_stdin();
    match exchange {
        Ok(mut progress) => match supervisor.wait_for_exit(McpStage::Shutdown) {
            Ok(status) if status.success() => match supervisor.finish_success(McpStage::Shutdown) {
                Ok(_) => {
                    progress.shutdown_completed = true;
                    McpExchangeOutcome::completed(progress)
                }
                Err(failure) => McpExchangeOutcome::failed(progress, failure),
            },
            Ok(status) => {
                let failure = McpProcessFailure::Shutdown {
                    stage: McpStage::Shutdown,
                    exit_code: status.code(),
                    stderr: BoundedText::empty(),
                };
                McpExchangeOutcome::failed(progress, supervisor.finish_failure(failure))
            }
            Err(failure) => {
                McpExchangeOutcome::failed(progress, supervisor.finish_failure(failure))
            }
        },
        Err(PendingExchangeFailure { progress, failure }) => {
            let failure = resolve_pending_failure(&mut supervisor, *failure);
            McpExchangeOutcome::failed(progress, supervisor.finish_failure(failure))
        }
    }
}

fn resolve_pending_failure(
    supervisor: &mut ChildSupervisor,
    pending: PendingMcpFailure,
) -> McpProcessFailure {
    let stage = pending.stage();
    if matches!(pending, PendingMcpFailure::Eof { .. }) {
        return match supervisor.wait_for_exit(stage) {
            Ok(status) => McpProcessFailure::ExitedBeforeResponse {
                stage,
                exit_code: status.code(),
                stderr: BoundedText::empty(),
            },
            Err(failure) => failure,
        };
    }
    if pending.may_be_early_exit() {
        match supervisor.wait_for_exit_for(stage, EARLY_EXIT_STATUS_WAIT) {
            Ok(Some(status)) => {
                return McpProcessFailure::ExitedBeforeResponse {
                    stage,
                    exit_code: status.code(),
                    stderr: BoundedText::empty(),
                }
            }
            Ok(None) => {}
            Err(failure) => return failure,
        }
    }
    pending.into_failure()
}

fn perform_mcp_exchange(
    supervisor: &mut ChildSupervisor,
    mode: &str,
) -> Result<McpExchangeProgress, PendingExchangeFailure> {
    let mut progress = McpExchangeProgress::not_started();
    supervisor
        .send_json_line(
            &json!({
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
        )
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure.into()))?;
    let initialize = read_json_response(supervisor, McpStage::Initialize)
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    validate_initialize_response(&initialize)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::Initialize, problem))
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    progress.initialize_completed = true;

    supervisor
        .send_json_line(
            &json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            }),
            McpStage::ToolsList,
        )
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure.into()))?;
    supervisor
        .send_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
            McpStage::ToolsList,
        )
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure.into()))?;
    let tools_response = read_json_response(supervisor, McpStage::ToolsList)
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    let tools = validate_tools_response(&tools_response)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::ToolsList, problem))
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    progress.tools_list = Some(tools);
    if let Err(problem) = validate_tools_for_mode_problem(
        mode,
        progress
            .tools_list
            .as_deref()
            .expect("tools/list was just recorded"),
    ) {
        return Err(PendingExchangeFailure::new(
            &progress,
            PendingMcpFailure::protocol(McpStage::ToolsList, problem),
        ));
    }
    progress.required_tools_validated = true;

    supervisor
        .send_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": LIST_PROJECTS_TOOL_NAME,
                    "arguments": {}
                }
            }),
            McpStage::SafeToolCall,
        )
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure.into()))?;
    let safe_response = read_json_response(supervisor, McpStage::SafeToolCall)
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    validate_safe_tool_response(&safe_response)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::SafeToolCall, problem))
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    progress.safe_tool_call_completed = true;
    Ok(progress)
}

#[derive(Debug)]
struct PendingExchangeFailure {
    progress: McpExchangeProgress,
    failure: Box<PendingMcpFailure>,
}

impl PendingExchangeFailure {
    fn new(progress: &McpExchangeProgress, failure: PendingMcpFailure) -> Self {
        Self {
            progress: progress.clone(),
            failure: Box::new(failure),
        }
    }
}

#[derive(Debug)]
enum PendingMcpFailure {
    Eof {
        stage: McpStage,
    },
    Lifecycle(McpProcessFailure),
    Protocol {
        stage: McpStage,
        problem: ProtocolProblem,
    },
}

impl From<McpProcessFailure> for PendingMcpFailure {
    fn from(failure: McpProcessFailure) -> Self {
        Self::Lifecycle(failure)
    }
}

impl PendingMcpFailure {
    fn protocol(stage: McpStage, problem: ProtocolProblem) -> Self {
        Self::Protocol { stage, problem }
    }

    const fn stage(&self) -> McpStage {
        match self {
            Self::Eof { stage } | Self::Protocol { stage, .. } => *stage,
            Self::Lifecycle(failure) => failure.stage(),
        }
    }

    const fn may_be_early_exit(&self) -> bool {
        matches!(
            self,
            Self::Lifecycle(McpProcessFailure::Read { .. } | McpProcessFailure::Write { .. })
        )
    }

    fn into_failure(self) -> McpProcessFailure {
        match self {
            Self::Eof { stage } => McpProcessFailure::Read {
                stage,
                io_detail: super::failure::bounded_io_text("MCP stdout ended unexpectedly"),
                stderr: BoundedText::empty(),
            },
            Self::Lifecycle(failure) => failure,
            Self::Protocol { stage, problem } => McpProcessFailure::Protocol {
                stage,
                protocol_detail: bounded_protocol_detail(problem.detail),
                missing_tools: problem.missing_tools,
                stderr: BoundedText::empty(),
            },
        }
    }
}

fn read_json_response(
    supervisor: &mut ChildSupervisor,
    stage: McpStage,
) -> Result<Value, PendingMcpFailure> {
    match supervisor.read_protocol(stage).map_err(PendingMcpFailure::from)? {
        ProtocolRead::Exited(status) => Err(PendingMcpFailure::Lifecycle(
            McpProcessFailure::ExitedBeforeResponse {
                stage,
                exit_code: status.code(),
                stderr: BoundedText::empty(),
            },
        )),
        ProtocolRead::Event(ProtocolEvent::Line(line)) => {
            serde_json::from_slice::<Value>(&line).map_err(|error| {
                PendingMcpFailure::protocol(
                    stage,
                    ProtocolProblem::new(format!(
                        "response was not valid JSON at line {} column {}",
                        error.line(),
                        error.column()
                    )),
                )
            })
        }
        ProtocolRead::Event(ProtocolEvent::Eof) => Err(PendingMcpFailure::Eof { stage }),
        ProtocolRead::Event(ProtocolEvent::LineTooLong { observed_bytes }) => {
            Err(PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(format!(
                    "response line exceeded the {MAX_PROTOCOL_LINE_BYTES}-byte limit (observed at least {observed_bytes} bytes)"
                )),
            ))
        }
        ProtocolRead::Event(ProtocolEvent::IncompleteLine { observed_bytes }) => {
            Err(PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(format!(
                    "response ended without newline-delimited framing after {observed_bytes} bytes"
                )),
            ))
        }
        ProtocolRead::Event(ProtocolEvent::MessageLimitExceeded { limit }) => {
            Err(PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(format!(
                    "protocol output exceeded the {limit}-message limit"
                )),
            ))
        }
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::Instant,
    };

    use super::*;

    static TEST_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    fn spawn_failure_is_typed_without_child_state() {
        let missing = std::env::temp_dir().join(format!(
            "volicord-missing-mcp-process-{}-{}",
            std::process::id(),
            TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let outcome = verify_mcp_stdio_command(
            Command::new(missing),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_millis(50),
        );
        assert_eq!(outcome.progress, McpExchangeProgress::not_started());
        assert!(matches!(
            outcome.failure,
            Some(McpProcessFailure::Spawn {
                stage: McpStage::Startup,
                ..
            })
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
    fn successful_stdio_probe_uses_bounded_shared_supervision() {
        let outcome = verify_mcp_stdio_command(
            shell_command(&successful_protocol_script("", "cat >/dev/null")),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.failure.is_none());
        assert!(outcome.progress.shutdown_completed);
    }

    #[cfg(unix)]
    #[test]
    fn exit_before_initialize_reports_status_and_bounded_stderr() {
        let outcome = verify_mcp_stdio_command(
            shell_command("printf '%s\\n' 'fixture startup failure' >&2; exit 23"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert_eq!(outcome.progress, McpExchangeProgress::not_started());
        match outcome.failure.expect("early exit must fail") {
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
        let outcome = verify_mcp_stdio_command(
            shell_command("printf '%s\\n' 'waiting for initialize' >&2; while :; do :; done"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_millis(50),
        );
        assert_eq!(outcome.progress, McpExchangeProgress::not_started());
        match outcome.failure.expect("initialize must time out") {
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
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.progress.initialize_completed);
        assert!(outcome.progress.tools_list.is_none());
        match outcome.failure.expect("tools/list error must fail") {
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
    fn required_tool_validation_failure_preserves_the_observed_tool_list() {
        let observed_tools = vec!["fixture.alpha".to_owned(), "fixture.beta".to_owned()];
        let tools_response = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": observed_tools
                    .iter()
                    .map(|name| json!({"name": name}))
                    .collect::<Vec<_>>()
            },
        });
        let script = protocol_script(
            "",
            &tools_response.to_string(),
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "result": {"content": []},
            })
            .to_string(),
            "",
        );
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.progress.initialize_completed);
        assert_eq!(outcome.progress.tools_list, Some(observed_tools));
        assert!(!outcome.progress.required_tools_validated);
        assert!(!outcome.progress.safe_tool_call_completed);
        assert!(!outcome.progress.shutdown_completed);
        assert!(matches!(
            outcome.failure,
            Some(McpProcessFailure::Protocol {
                stage: McpStage::ToolsList,
                ref missing_tools,
                ..
            }) if !missing_tools.is_empty()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn safe_tool_call_failure_retains_completed_progress() {
        let safe_error = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "error": {"code": -32603, "message": "ignored child prose"},
        });
        let script = protocol_script("", &read_only_tools_response(), &safe_error.to_string(), "");
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.progress.initialize_completed);
        assert_eq!(
            outcome.progress.tools_list,
            Some(read_only_required_tool_names().map(str::to_owned).collect())
        );
        assert!(outcome.progress.required_tools_validated);
        assert!(!outcome.progress.safe_tool_call_completed);
        assert!(matches!(
            outcome.failure,
            Some(McpProcessFailure::Protocol {
                stage: McpStage::SafeToolCall,
                ..
            })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn shutdown_failure_preserves_every_completed_exchange_observation() {
        let expected_tools = read_only_required_tool_names()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let script = successful_protocol_script("", "exit 17");
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.progress.initialize_completed);
        assert_eq!(outcome.progress.tools_list, Some(expected_tools));
        assert!(outcome.progress.required_tools_validated);
        assert!(outcome.progress.safe_tool_call_completed);
        assert!(!outcome.progress.shutdown_completed);
        assert!(matches!(
            outcome.failure,
            Some(McpProcessFailure::Shutdown {
                stage: McpStage::Shutdown,
                exit_code: Some(17),
                ..
            })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn malformed_json_is_a_bounded_protocol_failure_without_raw_line_echo() {
        let outcome = verify_mcp_stdio_command(
            shell_command("IFS= read -r request; printf '%s\\n' '{not-json}'"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        match outcome.failure.expect("malformed JSON must fail") {
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
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        match outcome.failure.expect("early exit must fail") {
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
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(3),
        );
        assert!(outcome.failure.is_none());
        assert!(outcome.progress.shutdown_completed);
    }

    #[cfg(unix)]
    #[test]
    fn successful_shutdown_reaps_the_child() {
        let marker = std::env::temp_dir().join(format!(
            "volicord-mcp-shutdown-{}-{}",
            std::process::id(),
            TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let script =
            successful_protocol_script("", "cat >/dev/null; printf '%s' 'reaped' > \"$1\"");
        let mut command = shell_command(&script);
        command.arg("fixture").arg(&marker);
        let outcome =
            verify_mcp_stdio_command(command, CONNECTION_MODE_READ_ONLY, Duration::from_secs(2));
        assert!(outcome.failure.is_none());
        assert!(outcome.progress.shutdown_completed);
        assert_eq!(
            fs::read_to_string(&marker).expect("shutdown marker"),
            "reaped"
        );
        fs::remove_file(marker).expect("remove shutdown marker");
    }

    #[cfg(unix)]
    #[test]
    fn stdio_probe_does_not_wait_for_a_descendant_inherited_pipe() {
        let script = successful_protocol_script("", "cat >/dev/null; sleep 30 &");
        let started = Instant::now();
        let outcome = verify_mcp_stdio_command(
            shell_command(&script),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.failure.is_none(), "{outcome:?}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
