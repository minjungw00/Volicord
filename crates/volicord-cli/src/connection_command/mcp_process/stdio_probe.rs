use std::{fs, process::Command, time::Duration};

use serde_json::{json, Value};
use tempfile::TempDir;
use volicord_mcp::{MaterializedManagedMcpLaunch, VOLICORD_HOME_ENV};
use volicord_mcp_protocol::{McpProtocolRevision, ProtocolRegistry};
use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
        ConnectionProjectRegistration, CONNECTION_INTENT_PERSONAL, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_USER,
    },
    bootstrap::{
        initialize_runtime_home, register_project, write_installation_profile,
        InstallationProfileRegistration, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
};
use volicord_types::{AgentConnectionMode, AgentToolId};

use crate::connection_command::managed_host_round_trip_tool;

use super::{
    failure::{
        bounded_protocol_detail, BoundedText, McpProcessFailure, McpProtocolFailureKind, McpStage,
    },
    host_compatibility::{self, HostCompatibilityFixture, HostCompatibilityProfile},
    pinned_schema,
    supervisor::{
        ChildSupervisor, ProtocolEvent, ProtocolRead, SupervisorKind, MAX_PROTOCOL_LINE_BYTES,
    },
};

const EARLY_EXIT_STATUS_WAIT: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpExchangeProgress {
    pub(in crate::connection_command) process_id: Option<u32>,
    pub(in crate::connection_command) requested_revision: Option<String>,
    pub(in crate::connection_command) negotiated_revision: Option<String>,
    pub(in crate::connection_command) initialize_completed: bool,
    pub(in crate::connection_command) initialized_notification_completed: bool,
    pub(in crate::connection_command) tools_list: Option<Vec<String>>,
    pub(in crate::connection_command) pinned_schema_validated: bool,
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
            process_id: None,
            requested_revision: None,
            negotiated_revision: None,
            initialize_completed,
            initialized_notification_completed: initialize_completed,
            tools_list,
            pinned_schema_validated: required_tools_validated,
            required_tools_validated,
            safe_tool_call_completed,
            shutdown_completed,
        }
    }
}

impl McpExchangeProgress {
    fn for_revision(revision: McpProtocolRevision) -> Self {
        Self {
            requested_revision: Some(revision.as_str().to_owned()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRevisionProbeOutcome {
    pub(in crate::connection_command) revision: String,
    pub(in crate::connection_command) progress: McpExchangeProgress,
    pub(in crate::connection_command) failure: Option<McpProcessFailure>,
    pub(in crate::connection_command) diagnostic: Option<McpPersistedDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpHostProbeOutcome {
    pub(in crate::connection_command) profile: HostCompatibilityProfile,
    pub(in crate::connection_command) fixture_id: String,
    pub(in crate::connection_command) progress: McpExchangeProgress,
    pub(in crate::connection_command) failure: Option<McpProcessFailure>,
    pub(in crate::connection_command) diagnostic: Option<McpPersistedDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPersistedDiagnostic {
    pub(in crate::connection_command) finding_id: String,
    pub(in crate::connection_command) code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpExchangeOutcome {
    pub(in crate::connection_command) progress: McpExchangeProgress,
    pub(in crate::connection_command) failure: Option<McpProcessFailure>,
    pub(in crate::connection_command) diagnostic: Option<McpPersistedDiagnostic>,
    pub(in crate::connection_command) conformance: Vec<McpRevisionProbeOutcome>,
    pub(in crate::connection_command) host_compatibility: Vec<McpHostProbeOutcome>,
}

impl McpExchangeOutcome {
    pub fn failed(progress: McpExchangeProgress, failure: McpProcessFailure) -> Self {
        Self {
            progress,
            failure: Some(failure),
            diagnostic: None,
            conformance: Vec::new(),
            host_compatibility: Vec::new(),
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
            diagnostic: None,
            conformance: Vec::new(),
            host_compatibility: Vec::new(),
        }
    }

    fn matrix(
        conformance: Vec<McpRevisionProbeOutcome>,
        host_compatibility: Vec<McpHostProbeOutcome>,
    ) -> Self {
        let failure = conformance
            .iter()
            .find_map(|probe| probe.failure.clone())
            .or_else(|| {
                host_compatibility
                    .iter()
                    .find_map(|probe| probe.failure.clone())
            });
        Self {
            progress: McpExchangeProgress::not_started(),
            failure,
            diagnostic: None,
            conformance,
            host_compatibility,
        }
    }

    pub(super) fn failure_summary(&self, failure: &McpProcessFailure) -> String {
        if let Some(probe) = self
            .conformance
            .iter()
            .find(|probe| probe.failure.as_ref() == Some(failure))
        {
            return format!(
                "MCP server conformance revision {} failed: {}",
                probe.revision,
                failure.summary()
            );
        }
        if let Some(probe) = self
            .host_compatibility
            .iter()
            .find(|probe| probe.failure.as_ref() == Some(failure))
        {
            return format!(
                "MCP host compatibility profile {} fixture {} failed: {}",
                probe.profile.as_str(),
                probe.fixture_id,
                failure.summary()
            );
        }
        failure.summary()
    }
}

pub(super) fn verify_mcp_stdio_process(
    launch: &MaterializedManagedMcpLaunch,
    mode: &str,
    timeout: Duration,
) -> McpExchangeOutcome {
    let fixture = match DisposableConformanceFixture::new(launch, mode) {
        Ok(fixture) => fixture,
        Err(detail) => {
            return McpExchangeOutcome::failed(
                McpExchangeProgress::not_started(),
                McpProcessFailure::Spawn {
                    stage: McpStage::Startup,
                    io_detail: BoundedText::from_utf8(
                        detail,
                        super::failure::MAX_IO_DETAIL_BYTES,
                        "disposable conformance fixture",
                    ),
                },
            )
        }
    };
    verify_mcp_stdio_command_factory(|| fixture.command(), mode, timeout)
}

struct DisposableConformanceFixture {
    _temp_dir: TempDir,
    runtime_home: std::path::PathBuf,
    repo_root: std::path::PathBuf,
    command: String,
}

impl DisposableConformanceFixture {
    const CONNECTION_ID: &'static str = "connection_verification_fixture";
    const PROJECT_ID: &'static str = "project_verification_fixture";

    fn new(launch: &MaterializedManagedMcpLaunch, mode: &str) -> Result<Self, String> {
        if !matches!(mode, CONNECTION_MODE_WORKFLOW | CONNECTION_MODE_READ_ONLY) {
            return Err(format!("unsupported connection mode: {mode}"));
        }
        let temp_dir = tempfile::Builder::new()
            .prefix("volicord-connection-verify-")
            .tempdir()
            .map_err(|error| {
                format!("failed to create disposable verification fixture: {error}")
            })?;
        let runtime_home = temp_dir.path().join("runtime-home");
        let repo_root = temp_dir.path().join("product-repository");
        fs::create_dir_all(&repo_root)
            .map_err(|error| format!("failed to create disposable Product Repository: {error}"))?;
        initialize_runtime_home(&runtime_home, "runtime_home_verification_fixture", "{}")
            .map_err(|error| format!("failed to initialize disposable Runtime Home: {error}"))?;
        write_installation_profile(
            &runtime_home,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: launch.command().to_owned(),
                volicord_mcp_command: launch.command().to_owned(),
                bin_dir: runtime_home.join("bin"),
                default_connection_mode: mode.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .map_err(|error| format!("failed to write disposable installation profile: {error}"))?;
        register_project(
            &runtime_home,
            ProjectRegistration {
                project_id: Self::PROJECT_ID.to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .map_err(|error| format!("failed to register disposable project: {error}"))?;
        ensure_agent_connection(
            &runtime_home,
            AgentConnectionRegistration {
                connection_internal_id: Self::CONNECTION_ID.to_owned(),
                host_kind: HOST_KIND_CODEX.to_owned(),
                intent: CONNECTION_INTENT_PERSONAL.to_owned(),
                host_scope: HOST_SCOPE_USER.to_owned(),
                server_name: "volicord-verification-fixture".to_owned(),
                config_target: temp_dir
                    .path()
                    .join("codex-config.toml")
                    .to_string_lossy()
                    .into_owned(),
                mode: mode.to_owned(),
                enabled: true,
                managed_fingerprint: "disposable-verification-fixture".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .map_err(|error| format!("failed to register disposable connection: {error}"))?;
        add_connection_project(
            &runtime_home,
            ConnectionProjectRegistration {
                connection_internal_id: Self::CONNECTION_ID.to_owned(),
                project_id: Self::PROJECT_ID.to_owned(),
            },
        )
        .map_err(|error| format!("failed to attach disposable project: {error}"))?;
        Ok(Self {
            _temp_dir: temp_dir,
            runtime_home,
            repo_root,
            command: launch.command().to_owned(),
        })
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.command);
        command.args([
            "mcp",
            "serve",
            "--connection",
            Self::CONNECTION_ID,
            "--project",
            Self::PROJECT_ID,
        ]);
        command.env_remove(VOLICORD_HOME_ENV);
        for name in [
            "VOLICORD_MCP_LAUNCH",
            "VOLICORD_MCP_HOST",
            "VOLICORD_MCP_CONNECTION_ID",
            "VOLICORD_MCP_VERIFICATION",
            "VOLICORD_MCP_PROJECT_ID",
        ] {
            command.env_remove(name);
        }
        command.env(VOLICORD_HOME_ENV, &self.runtime_home);
        command.current_dir(&self.repo_root);
        command
    }
}

#[cfg(test)]
fn verify_mcp_stdio_command(command: Command, mode: &str, timeout: Duration) -> McpExchangeOutcome {
    let probe = ProbeRequest::server_conformance(
        "2025-11-25"
            .parse()
            .expect("single-probe test revision is production-supported"),
    );
    verify_mcp_probe_command(command, mode, timeout, &probe)
}

fn verify_mcp_stdio_command_factory(
    mut command: impl FnMut() -> Command,
    mode: &str,
    timeout: Duration,
) -> McpExchangeOutcome {
    let conformance = ProtocolRegistry::production()
        .oldest_to_newest()
        .map(|profile| {
            let revision = profile.revision();
            let outcome = verify_mcp_probe_command(
                command(),
                mode,
                timeout,
                &ProbeRequest::server_conformance(revision),
            );
            McpRevisionProbeOutcome {
                revision: revision.as_str().to_owned(),
                progress: outcome.progress,
                failure: outcome.failure,
                diagnostic: None,
            }
        })
        .collect();
    let host_compatibility = host_compatibility::fixtures()
        .iter()
        .copied()
        .map(|fixture| {
            let outcome = verify_mcp_probe_command(
                command(),
                mode,
                timeout,
                &ProbeRequest::host_compatibility(fixture),
            );
            McpHostProbeOutcome {
                profile: fixture.profile,
                fixture_id: fixture.fixture_id.to_owned(),
                progress: outcome.progress,
                failure: outcome.failure,
                diagnostic: None,
            }
        })
        .collect();
    McpExchangeOutcome::matrix(conformance, host_compatibility)
}

fn verify_mcp_probe_command(
    command: Command,
    mode: &str,
    timeout: Duration,
    probe: &ProbeRequest,
) -> McpExchangeOutcome {
    let mut supervisor = match ChildSupervisor::spawn(command, SupervisorKind::Stdio, timeout) {
        Ok(supervisor) => supervisor,
        Err(failure) => {
            return McpExchangeOutcome::failed(
                McpExchangeProgress::for_revision(probe.revision),
                failure,
            )
        }
    };
    let process_id = supervisor.child_id();

    let exchange = perform_mcp_exchange(&mut supervisor, mode, probe);
    supervisor.close_stdin();
    match exchange {
        Ok(mut progress) => match supervisor.wait_for_exit(McpStage::Shutdown) {
            Ok(status) if status.success() => match supervisor.finish_success(McpStage::Shutdown) {
                Ok(_) => {
                    progress.shutdown_completed = true;
                    progress.process_id = Some(process_id);
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
                let mut progress = progress;
                progress.process_id = Some(process_id);
                McpExchangeOutcome::failed(progress, supervisor.finish_failure(failure))
            }
            Err(failure) => {
                let mut progress = progress;
                progress.process_id = Some(process_id);
                McpExchangeOutcome::failed(progress, supervisor.finish_failure(failure))
            }
        },
        Err(PendingExchangeFailure { progress, failure }) => {
            let failure = resolve_pending_failure(&mut supervisor, *failure);
            let mut progress = progress;
            progress.process_id = Some(process_id);
            McpExchangeOutcome::failed(progress, supervisor.finish_failure(failure))
        }
    }
}

#[derive(Debug, Clone)]
struct ProbeRequest {
    revision: McpProtocolRevision,
    initialize_params: Value,
    call_metadata: Option<Value>,
}

impl ProbeRequest {
    fn server_conformance(revision: McpProtocolRevision) -> Self {
        Self {
            revision,
            initialize_params: json!({
                "protocolVersion": revision.as_str(),
                "capabilities": {},
                "clientInfo": {
                    "name": "volicord-conformance-probe",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
            call_metadata: None,
        }
    }

    fn host_compatibility(fixture: HostCompatibilityFixture) -> Self {
        Self {
            revision: fixture.revision,
            initialize_params: fixture.initialize_params(),
            call_metadata: Some(fixture.call_metadata()),
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
    probe: &ProbeRequest,
) -> Result<McpExchangeProgress, PendingExchangeFailure> {
    let mut progress = McpExchangeProgress::for_revision(probe.revision);
    supervisor
        .send_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": probe.initialize_params
            }),
            McpStage::Initialize,
        )
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure.into()))?;
    let initialize = read_json_response(supervisor, McpStage::Initialize)
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    let negotiated_revision = validate_initialize_response(&initialize, probe.revision)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::Initialize, problem))
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    progress.initialize_completed = true;
    progress.negotiated_revision = Some(negotiated_revision);

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
    progress.initialized_notification_completed = true;
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
    let tools = validate_tools_response(&tools_response, probe.revision)
        .map_err(|problem| PendingMcpFailure::protocol(McpStage::ToolsList, problem))
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    progress.tools_list = Some(tools);
    progress.pinned_schema_validated = true;
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

    let mut call_params = json!({
        "name": managed_host_round_trip_tool().wire_name(),
        "arguments": {}
    });
    if let Some(metadata) = &probe.call_metadata {
        call_params
            .as_object_mut()
            .expect("tool call params are an object")
            .insert("_meta".to_owned(), metadata.clone());
    }
    supervisor
        .send_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": call_params
            }),
            McpStage::SafeToolCall,
        )
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure.into()))?;
    let safe_response = read_json_response(supervisor, McpStage::SafeToolCall)
        .map_err(|failure| PendingExchangeFailure::new(&progress, failure))?;
    validate_safe_tool_response(&safe_response, probe.revision)
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
    Lifecycle(Box<McpProcessFailure>),
    Protocol {
        stage: McpStage,
        problem: ProtocolProblem,
    },
}

impl From<McpProcessFailure> for PendingMcpFailure {
    fn from(failure: McpProcessFailure) -> Self {
        Self::Lifecycle(Box::new(failure))
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

    fn may_be_early_exit(&self) -> bool {
        matches!(
            self,
            Self::Lifecycle(failure)
                if matches!(
                    failure.as_ref(),
                    McpProcessFailure::Read { .. } | McpProcessFailure::Write { .. }
                )
        )
    }

    fn into_failure(self) -> McpProcessFailure {
        match self {
            Self::Eof { stage } => McpProcessFailure::Read {
                stage,
                io_detail: super::failure::bounded_io_text("MCP stdout ended unexpectedly"),
                stderr: BoundedText::empty(),
            },
            Self::Lifecycle(failure) => *failure,
            Self::Protocol { stage, problem } => McpProcessFailure::Protocol {
                stage,
                kind: problem.kind,
                protocol_detail: bounded_protocol_detail(problem.detail),
                json_rpc_error_code: problem.json_rpc_error_code,
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
        ProtocolRead::Exited(status) => Err(PendingMcpFailure::Lifecycle(Box::new(
            McpProcessFailure::ExitedBeforeResponse {
                stage,
                exit_code: status.code(),
                stderr: BoundedText::empty(),
            },
        ))),
        ProtocolRead::Event(ProtocolEvent::Line(line)) => {
            serde_json::from_slice::<Value>(&line).map_err(|error| {
                PendingMcpFailure::protocol(
                    stage,
                    ProtocolProblem::new(McpProtocolFailureKind::MalformedResponse, format!(
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
                ProtocolProblem::new(McpProtocolFailureKind::MessageSizeExceeded, format!(
                    "response line exceeded the {MAX_PROTOCOL_LINE_BYTES}-byte limit (observed at least {observed_bytes} bytes)"
                )),
            ))
        }
        ProtocolRead::Event(ProtocolEvent::IncompleteLine { observed_bytes }) => {
            Err(PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(McpProtocolFailureKind::FramingFailure, format!(
                    "response ended without newline-delimited framing after {observed_bytes} bytes"
                )),
            ))
        }
        ProtocolRead::Event(ProtocolEvent::MessageLimitExceeded { limit }) => {
            Err(PendingMcpFailure::protocol(
                stage,
                ProtocolProblem::new(McpProtocolFailureKind::MessageSizeExceeded, format!(
                    "protocol output exceeded the {limit}-message limit"
                )),
            ))
        }
    }
}

#[derive(Debug)]
struct ProtocolProblem {
    kind: McpProtocolFailureKind,
    detail: String,
    json_rpc_error_code: Option<i64>,
    missing_tools: Vec<String>,
}

impl ProtocolProblem {
    fn new(kind: McpProtocolFailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            json_rpc_error_code: None,
            missing_tools: Vec::new(),
        }
    }

    fn missing_tools(missing_tools: Vec<String>) -> Self {
        Self {
            kind: McpProtocolFailureKind::RequiredToolMissing,
            detail: format!(
                "tools/list omitted {} required tool(s)",
                missing_tools.len()
            ),
            json_rpc_error_code: None,
            missing_tools,
        }
    }

    fn json_rpc(
        kind: McpProtocolFailureKind,
        detail: impl Into<String>,
        error_code: Option<i64>,
    ) -> Self {
        Self {
            kind,
            detail: detail.into(),
            json_rpc_error_code: error_code,
            missing_tools: Vec::new(),
        }
    }
}

fn response_error(
    value: &Value,
    operation: &str,
    kind: McpProtocolFailureKind,
) -> Option<ProtocolProblem> {
    let error = value.get("error")?;
    let error_code = error.get("code").and_then(Value::as_i64);
    let detail = error_code.map_or_else(
        || format!("{operation} response returned a JSON-RPC error"),
        |code| format!("{operation} response returned JSON-RPC error code {code}"),
    );
    Some(ProtocolProblem::json_rpc(kind, detail, error_code))
}

fn validate_initialize_response(
    value: &Value,
    revision: McpProtocolRevision,
) -> Result<String, ProtocolProblem> {
    if let Some(problem) = response_error(value, "initialize", McpProtocolFailureKind::JsonRpcError)
    {
        return Err(problem);
    }
    let result = value.get("result").ok_or_else(|| {
        ProtocolProblem::new(
            McpProtocolFailureKind::MalformedResponse,
            "initialize response was missing result",
        )
    })?;
    pinned_schema::validate_definition(revision, "InitializeResult", result).map_err(|error| {
        ProtocolProblem::new(
            McpProtocolFailureKind::RevisionSchemaProjectionFailure,
            format!(
                "initialize result failed the {} pinned schema: {error}",
                revision.as_str()
            ),
        )
    })?;
    let negotiated = result
        .get("protocolVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProtocolProblem::new(
                McpProtocolFailureKind::MalformedProtocolVersion,
                "initialize response was missing result.protocolVersion",
            )
        })?;
    if negotiated != revision.as_str() {
        return Err(ProtocolProblem::new(
            McpProtocolFailureKind::UnsupportedProtocolRevision,
            format!(
                "initialize selected protocol revision {negotiated}, expected {}",
                revision.as_str()
            ),
        ));
    }
    Ok(negotiated.to_owned())
}

fn validate_tools_response(
    value: &Value,
    revision: McpProtocolRevision,
) -> Result<Vec<String>, ProtocolProblem> {
    if let Some(problem) = response_error(
        value,
        "tools/list",
        McpProtocolFailureKind::ToolListProtocolError,
    ) {
        return Err(problem);
    }
    let result = value.get("result").ok_or_else(|| {
        ProtocolProblem::new(
            McpProtocolFailureKind::MalformedResponse,
            "tools/list response was missing result",
        )
    })?;
    pinned_schema::validate_definition(revision, "ListToolsResult", result).map_err(|error| {
        ProtocolProblem::new(
            McpProtocolFailureKind::ToolListSchemaFailure,
            format!(
                "tools/list result failed the {} pinned schema: {error}",
                revision.as_str()
            ),
        )
    })?;
    let tools = result
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProtocolProblem::new(
                McpProtocolFailureKind::ToolListSchemaFailure,
                "tools/list response was missing result.tools",
            )
        })?;
    let mut names = Vec::new();
    for tool in tools {
        let name = tool.get("name").and_then(Value::as_str).ok_or_else(|| {
            ProtocolProblem::new(
                McpProtocolFailureKind::InvalidToolDefinitionProjection,
                "tools/list contained a tool without a name",
            )
        })?;
        names.push(name.to_owned());
    }
    Ok(names)
}

fn validate_safe_tool_response(
    value: &Value,
    revision: McpProtocolRevision,
) -> Result<(), ProtocolProblem> {
    if let Some(problem) = response_error(
        value,
        "designated read-only tool call",
        McpProtocolFailureKind::SafeToolProtocolError,
    ) {
        return Err(problem);
    }
    let result = value.get("result").ok_or_else(|| {
        ProtocolProblem::new(
            McpProtocolFailureKind::MalformedResponse,
            "designated read-only tool response was missing result",
        )
    })?;
    pinned_schema::validate_definition(revision, "CallToolResult", result).map_err(|error| {
        ProtocolProblem::new(
            McpProtocolFailureKind::OutputSchemaFailure,
            format!(
                "designated read-only tool result failed the {} pinned schema: {error}",
                revision.as_str()
            ),
        )
    })?;
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(ProtocolProblem::new(
            McpProtocolFailureKind::SafeReadOnlyToolFailure,
            "designated read-only tool response set isError=true",
        ));
    }
    Ok(())
}

fn validate_tools_for_mode_problem(mode: &str, tools: &[String]) -> Result<(), ProtocolProblem> {
    match mode {
        CONNECTION_MODE_READ_ONLY => {
            validate_required_tools_problem(tools, read_only_required_tools())
        }
        CONNECTION_MODE_WORKFLOW => {
            validate_required_tools_problem(tools, workflow_required_tools())
        }
        other => Err(ProtocolProblem::new(
            McpProtocolFailureKind::Unexpected,
            format!("unsupported connection mode for tool validation: {other}"),
        )),
    }
}

fn validate_required_tools_problem(
    tools: &[String],
    expected: impl IntoIterator<Item = AgentToolId>,
) -> Result<(), ProtocolProblem> {
    let missing_tools = expected
        .into_iter()
        .filter(|expected| !tools.iter().any(|tool| tool == expected.wire_name()))
        .map(|tool| tool.wire_name().to_owned())
        .collect::<Vec<_>>();
    if missing_tools.is_empty() {
        Ok(())
    } else {
        Err(ProtocolProblem::missing_tools(missing_tools))
    }
}

pub(super) fn workflow_required_tools() -> impl Iterator<Item = AgentToolId> {
    AgentToolId::ALL
        .iter()
        .copied()
        .filter(|tool| tool.available_in(AgentConnectionMode::Workflow))
}

pub(super) fn read_only_required_tools() -> impl Iterator<Item = AgentToolId> {
    AgentToolId::ALL
        .iter()
        .copied()
        .filter(|tool| tool.available_in(AgentConnectionMode::ReadOnly))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::Path,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::Instant,
    };

    use volicord_mcp::{
        ManagedMcpInvocationPurpose, ManagedMcpLaunchSpec, ManagedMcpMaterializationInput,
        ManagedMcpWorkingDirectory,
    };

    use super::*;
    use crate::connection_command::mcp_process::test_child;

    static TEST_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn test_revision(value: &str) -> McpProtocolRevision {
        value.parse().expect("production test revision")
    }

    #[test]
    fn safe_tool_call_rejects_json_rpc_and_tool_errors() {
        assert!(validate_safe_tool_response(
            &json!({"jsonrpc": "2.0", "id": 3, "result": {"content": []}}),
            test_revision("2024-11-05"),
        )
        .is_ok());
        assert!(validate_safe_tool_response(
            &json!({"jsonrpc": "2.0", "id": 3, "error": {"code": -32603}}),
            test_revision("2024-11-05"),
        )
        .is_err());
        assert!(validate_safe_tool_response(
            &json!({"jsonrpc": "2.0", "id": 3, "result": {"isError": true}}),
            test_revision("2024-11-05"),
        )
        .is_err());
    }

    #[test]
    fn active_verification_tool_is_resolved_from_the_canonical_verification_role() {
        assert_eq!(managed_host_round_trip_tool(), AgentToolId::LIST_PROJECTS);
    }

    #[test]
    fn active_conformance_fixture_never_uses_the_selected_live_runtime_home() {
        let selected_live_runtime_home = Path::new("/selected/live-runtime-home");
        let launch = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord"),
            selected_live_runtime_home,
            "connection_live",
        )
        .expect("personal launch")
        .materialize(ManagedMcpMaterializationInput::new(
            ManagedMcpInvocationPurpose::CliStdioHandshake,
            BTreeMap::new(),
            ManagedMcpWorkingDirectory::Inherited,
        ))
        .expect("materialized launch");

        let fixture = DisposableConformanceFixture::new(&launch, CONNECTION_MODE_READ_ONLY)
            .expect("disposable fixture");
        assert_ne!(fixture.runtime_home, selected_live_runtime_home);
        assert!(fixture.runtime_home.starts_with(fixture._temp_dir.path()));
        let command = fixture.command();
        assert_eq!(
            command
                .get_envs()
                .find(|(name, _)| *name == VOLICORD_HOME_ENV)
                .and_then(|(_, value)| value),
            Some(fixture.runtime_home.as_os_str())
        );
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
        assert_eq!(
            outcome.progress.requested_revision.as_deref(),
            Some("2025-11-25")
        );
        assert!(!outcome.progress.initialize_completed);
        assert!(matches!(
            outcome.failure,
            Some(McpProcessFailure::Spawn {
                stage: McpStage::Startup,
                ..
            })
        ));
    }

    #[test]
    fn successful_stdio_probe_uses_bounded_shared_supervision() {
        let outcome = verify_mcp_stdio_command(
            test_child::command("stdio-success"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.failure.is_none());
        assert!(outcome.progress.shutdown_completed);
    }

    #[test]
    fn matrix_probes_every_production_profile_and_independent_codex_fixture() {
        let outcome = verify_mcp_stdio_command_factory(
            || test_child::command("stdio-success"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.failure.is_none(), "{outcome:?}");
        let expected_revisions = ProtocolRegistry::production()
            .oldest_to_newest()
            .map(|profile| profile.revision().as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            outcome
                .conformance
                .iter()
                .map(|probe| probe.revision.as_str())
                .collect::<Vec<_>>(),
            expected_revisions
        );
        for probe in &outcome.conformance {
            assert!(probe.failure.is_none(), "{probe:?}");
            assert_eq!(
                probe.progress.requested_revision,
                probe.progress.negotiated_revision
            );
            assert!(probe.progress.initialize_completed);
            assert!(probe.progress.initialized_notification_completed);
            assert!(probe.progress.pinned_schema_validated);
            assert!(probe.progress.required_tools_validated);
            assert!(probe.progress.safe_tool_call_completed);
            assert!(probe.progress.shutdown_completed);
        }
        assert_eq!(outcome.host_compatibility.len(), 1);
        let codex = &outcome.host_compatibility[0];
        assert_eq!(codex.profile, HostCompatibilityProfile::Codex);
        assert_eq!(
            codex.progress.requested_revision.as_deref(),
            Some("2025-06-18")
        );
        assert_eq!(
            codex.progress.negotiated_revision.as_deref(),
            Some("2025-06-18")
        );
        assert!(codex.failure.is_none(), "{codex:?}");
        assert!(codex.progress.shutdown_completed);
    }

    #[test]
    fn one_revision_failure_fails_the_aggregate_and_identifies_that_revision() {
        let outcome = verify_mcp_stdio_command_factory(
            || test_child::command("one-revision-failure"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.failure.is_some());
        assert_eq!(outcome.conformance.len(), 5);
        let failed = outcome
            .conformance
            .iter()
            .filter(|probe| probe.failure.is_some())
            .collect::<Vec<_>>();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].revision, "2025-03-26");
        assert!(outcome
            .failure_summary(outcome.failure.as_ref().expect("aggregate failure"))
            .contains("conformance revision 2025-03-26 failed"));
        assert!(matches!(
            failed[0].failure,
            Some(McpProcessFailure::Protocol {
                stage: McpStage::ToolsList,
                ..
            })
        ));
        let Some(McpProcessFailure::Protocol {
            protocol_detail, ..
        }) = &failed[0].failure
        else {
            unreachable!("failure kind was asserted above")
        };
        assert!(protocol_detail.text.contains("pinned schema"));
        assert_eq!(
            outcome
                .conformance
                .iter()
                .filter(|probe| probe.failure.is_none())
                .count(),
            4
        );
        assert!(outcome
            .host_compatibility
            .iter()
            .all(|probe| probe.failure.is_none()));
    }

    #[test]
    fn exit_before_initialize_reports_status_and_bounded_stderr() {
        let outcome = verify_mcp_stdio_command(
            test_child::command("exit-before-initialize"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert_eq!(
            outcome.progress.requested_revision.as_deref(),
            Some("2025-11-25")
        );
        assert!(!outcome.progress.initialize_completed);
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

    #[test]
    fn initialize_timeout_terminates_and_reaps_the_child() {
        let timeout = Duration::from_millis(100);
        let started = Instant::now();
        let outcome = verify_mcp_stdio_command(
            test_child::command("hang-before-initialize"),
            CONNECTION_MODE_READ_ONLY,
            timeout,
        );
        let elapsed = started.elapsed();
        assert!(elapsed >= timeout);
        assert!(elapsed < Duration::from_secs(2));
        assert_eq!(
            outcome.progress.requested_revision.as_deref(),
            Some("2025-11-25")
        );
        assert!(!outcome.progress.initialize_completed);
        match outcome.failure.expect("initialize must time out") {
            McpProcessFailure::Timeout {
                stage,
                timeout: observed_timeout,
                stderr,
            } => {
                assert_eq!(stage, McpStage::Initialize);
                assert_eq!(observed_timeout, timeout);
                assert!(stderr.text.contains("waiting for initialize"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }

    #[test]
    fn tools_list_failure_retains_its_typed_stage() {
        let outcome = verify_mcp_stdio_command(
            test_child::command("tools-list-failure"),
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

    #[test]
    fn required_tool_validation_failure_preserves_the_observed_tool_list() {
        let observed_tools = vec!["fixture.alpha".to_owned(), "fixture.beta".to_owned()];
        let outcome = verify_mcp_stdio_command(
            test_child::command("missing-required-tools"),
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

    #[test]
    fn safe_tool_call_failure_retains_completed_progress() {
        let outcome = verify_mcp_stdio_command(
            test_child::command("read-only-tool-failure"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.progress.initialize_completed);
        assert_eq!(
            outcome.progress.tools_list,
            Some(
                read_only_required_tools()
                    .map(|tool| tool.wire_name().to_owned())
                    .collect(),
            )
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

    #[test]
    fn shutdown_failure_preserves_every_completed_exchange_observation() {
        let expected_tools = read_only_required_tools()
            .map(|tool| tool.wire_name().to_owned())
            .collect::<Vec<_>>();
        let outcome = verify_mcp_stdio_command(
            test_child::command("shutdown-failure"),
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

    #[test]
    fn malformed_json_is_a_bounded_protocol_failure_without_raw_line_echo() {
        let outcome = verify_mcp_stdio_command(
            test_child::command("malformed-json"),
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

    #[test]
    fn large_stderr_is_truncated_after_an_early_exit() {
        let outcome = verify_mcp_stdio_command(
            test_child::command("large-stderr-exit"),
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

    #[test]
    fn stderr_is_drained_while_waiting_for_initialize() {
        let started = Instant::now();
        let outcome = verify_mcp_stdio_command(
            test_child::command("sustained-stderr"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(3),
        );
        assert!(outcome.failure.is_none());
        assert!(outcome.progress.shutdown_completed);
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn successful_shutdown_reaps_the_child() {
        let marker = std::env::temp_dir().join(format!(
            "volicord-mcp-shutdown-{}-{}",
            std::process::id(),
            TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut command = test_child::command("graceful-eof");
        command.arg(&marker);
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

    #[test]
    fn stdio_probe_does_not_wait_for_a_descendant_inherited_pipe() {
        let started = Instant::now();
        let outcome = verify_mcp_stdio_command(
            test_child::command("descendant-output-hold"),
            CONNECTION_MODE_READ_ONLY,
            Duration::from_secs(1),
        );
        assert!(outcome.failure.is_none(), "{outcome:?}");
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
