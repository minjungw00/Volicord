use crate::adapter::*;
use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError};
use crate::local_http::generate_bearer_token;
use crate::local_web_consent::start_stdio_local_web_consent_listener;
use crate::prelude::*;
use crate::repository_discovery::RepositoryDiscoveryHost;
use crate::routing::*;
use crate::util::*;

const VOLICORD_MCP_VERIFICATION: &str = "VOLICORD_MCP_VERIFICATION";
const VOLICORD_MCP_LAUNCH: &str = "VOLICORD_MCP_LAUNCH";
const VOLICORD_MCP_HOST: &str = "VOLICORD_MCP_HOST";
const VOLICORD_MCP_CONNECTION_ID: &str = "VOLICORD_MCP_CONNECTION_ID";
const VOLICORD_MCP_PROJECT_ID: &str = "VOLICORD_MCP_PROJECT_ID";
const MANAGED_HOST_LAUNCH_VALUE: &str = "managed_host";
const CODEX_HOST_VALUE: &str = "codex";
pub(crate) const MAX_MCP_COMPACT_MUTATION_RESULT_BYTES: usize = 65_536;
pub(crate) const MAX_MCP_FULL_MUTATION_RESULT_BYTES: usize = 256 * 1024;
pub(crate) const MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES: usize = 512;

pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    run_stdio_with_options(adapter, reader, writer, StdioRunOptions::default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StdioRunOptions {
    startup_session_watch: bool,
    launch_origin: McpLaunchOrigin,
}

impl Default for StdioRunOptions {
    fn default() -> Self {
        Self {
            startup_session_watch: false,
            launch_origin: McpLaunchOrigin::ManualCli,
        }
    }
}

fn run_stdio_with_options<R, W>(
    adapter: McpAdapter,
    reader: R,
    mut writer: W,
    options: StdioRunOptions,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let mut state = ConnectionState::for_launch_origin(options.launch_origin);
    start_transport_diagnostic_session_best_effort(&adapter, &state);
    let _startup_observation =
        if options.startup_session_watch && state.managed_host_lifecycle_observations {
            adapter.managed_lifecycle_observation_best_effort(
                &state.session_id,
                options.launch_origin.as_str(),
                ManagedLifecycleEvent::Startup,
                None,
            )
        } else if options.startup_session_watch {
            adapter.startup_session_watch_observation_best_effort_with_origin(
                &state.session_id,
                options.launch_origin.as_str(),
            )
        } else {
            StartupObservationResult::SkippedVerificationProbe
        };
    let mut lines = reader.lines();

    while let Some(line) = lines.next() {
        let line = line.map_err(McpAdapterError::Io)?;
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                write_json_line(
                    &mut writer,
                    json_rpc_error(Value::Null, -32700, "Parse error", Some(error.to_string())),
                )?;
                continue;
            }
        };

        if let Some(response) =
            handle_json_rpc_message(&adapter, &mut state, message, &mut lines, &mut writer)?
        {
            write_json_line(&mut writer, response)?;
        }
    }

    writer.flush().map_err(McpAdapterError::Io)
}

/// Runs the MCP stdio adapter from process environment and stdin/stdout.
pub fn run_stdio_from_env(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let launch_origin = classify_launch_origin(process_env_var, connection_id, project_id);
    let startup_session_watch = launch_origin == McpLaunchOrigin::ManagedHost;
    let runtime_home = resolve_runtime_home(process_env_var, &current_dir)?;
    let project_allowlist = project_id
        .map(ProjectId::new)
        .into_iter()
        .collect::<Vec<_>>();
    validate_mcp_project_allowlist(&runtime_home, connection_id, &project_allowlist)?;
    let context = McpConnectionContext::resolve(&runtime_home, connection_id)?
        .with_project_allowlist(project_allowlist);
    let local_web_consent = start_stdio_local_web_consent_listener(&runtime_home, &context).ok();
    let mut adapter = McpAdapter::new(runtime_home, context);
    if let Some(local_web_consent) = local_web_consent {
        adapter = adapter.with_local_web_consent(local_web_consent);
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            startup_session_watch,
            launch_origin,
        },
    )
}

/// Runs stdio from a clone-portable repository descriptor.
///
/// The descriptor carries only the host selector. Connection and project
/// identities are resolved from the current Git repository and the selected
/// local Runtime Home before the transport starts.
pub fn run_stdio_discover_repository_from_env(
    host: RepositoryDiscoveryHost,
) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let launch_origin = if mcp_verification_launch(process_env_var) {
        McpLaunchOrigin::CliVerification
    } else {
        McpLaunchOrigin::ManagedHost
    };
    let runtime_home = resolve_runtime_home(process_env_var, &current_dir)?;
    let resolution = RepositoryDiscoveryResolution::resolve(&runtime_home, &current_dir, host)?;
    let local_web_consent =
        start_stdio_local_web_consent_listener(&runtime_home, &resolution.context).ok();
    let mut adapter = McpAdapter::new(runtime_home, resolution.context);
    if let Some(local_web_consent) = local_web_consent {
        adapter = adapter.with_local_web_consent(local_web_consent);
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            startup_session_watch: launch_origin == McpLaunchOrigin::ManagedHost,
            launch_origin,
        },
    )
}

#[cfg(test)]
pub(crate) fn run_stdio_with_env_marker<R, W, F>(
    adapter: McpAdapter,
    reader: R,
    writer: W,
    env_var: F,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
    F: Fn(&str) -> Option<OsString>,
{
    let launch_origin = classify_launch_origin_for_adapter(&adapter, &env_var);
    run_stdio_with_options(
        adapter,
        reader,
        writer,
        StdioRunOptions {
            startup_session_watch: launch_origin == McpLaunchOrigin::ManagedHost,
            launch_origin,
        },
    )
}

/// Runs MCP startup validation from process environment.
pub fn run_preflight_check_from_env(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    preflight_check(process_env_var, &current_dir, connection_id, project_id)
}

/// Runs MCP startup validation from injected process inputs.
pub fn preflight_check<F>(
    env_var: F,
    current_dir: &Path,
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let detail_project_id = project_id.map(ProjectId::new);
    let inspection =
        McpConnectionStartupInspection::resolve(&runtime_home, connection_id, detail_project_id)?;
    Ok(inspection.preflight_report())
}

/// Resolves the Runtime Home used by the stdio entry point.
pub fn resolve_runtime_home_from_env<F>(env_var: F) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    resolve_runtime_home(env_var, &current_dir)
}

/// Resolves the Runtime Home from injected process inputs.
pub fn resolve_runtime_home<F>(env_var: F, current_dir: &Path) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    resolve_shared_runtime_home(env_var, current_dir).map_err(McpAdapterError::from)
}

fn mcp_verification_launch<F>(env_var: F) -> bool
where
    F: Fn(&str) -> Option<OsString>,
{
    env_var(VOLICORD_MCP_VERIFICATION).is_some_and(|value| value.to_str() == Some("1"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpLaunchOrigin {
    CliVerification,
    ManagedHost,
    ManualCli,
    InvalidManagedMarker,
    Unknown,
}

impl McpLaunchOrigin {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CliVerification => "cli_verification",
            Self::ManagedHost => "managed_host",
            Self::ManualCli => "manual_cli",
            Self::InvalidManagedMarker => "invalid_managed_marker",
            Self::Unknown => "unknown",
        }
    }
}

pub(crate) fn classify_launch_origin<F>(
    env_var: F,
    connection_id: &str,
    project_id: Option<&str>,
) -> McpLaunchOrigin
where
    F: Fn(&str) -> Option<OsString>,
{
    if mcp_verification_launch(&env_var) {
        return McpLaunchOrigin::CliVerification;
    }

    let launch = env_text(&env_var, VOLICORD_MCP_LAUNCH);
    let host = env_text(&env_var, VOLICORD_MCP_HOST);
    let marker_connection_id = env_text(&env_var, VOLICORD_MCP_CONNECTION_ID);
    let marker_project_id = env_text(&env_var, VOLICORD_MCP_PROJECT_ID);
    if launch.is_none()
        && host.is_none()
        && marker_connection_id.is_none()
        && marker_project_id.is_none()
    {
        return McpLaunchOrigin::ManualCli;
    }

    let project_matches = match project_id {
        Some(project_id) => marker_project_id.as_deref() == Some(project_id),
        None => marker_project_id.is_none(),
    };
    if launch.as_deref() == Some(MANAGED_HOST_LAUNCH_VALUE)
        && host.as_deref() == Some(CODEX_HOST_VALUE)
        && marker_connection_id.as_deref() == Some(connection_id)
        && project_matches
    {
        McpLaunchOrigin::ManagedHost
    } else {
        McpLaunchOrigin::InvalidManagedMarker
    }
}

#[cfg(test)]
fn classify_launch_origin_for_adapter<F>(adapter: &McpAdapter, env_var: &F) -> McpLaunchOrigin
where
    F: Fn(&str) -> Option<OsString>,
{
    let project_id = adapter
        .context
        .project_allowlist
        .as_ref()
        .and_then(|project_ids| project_ids.as_slice().first())
        .map(|project_id| project_id.as_str());
    classify_launch_origin(
        env_var,
        adapter.context.connection_internal_id.as_str(),
        project_id,
    )
}

fn env_text<F>(env_var: &F, name: &str) -> Option<String>
where
    F: Fn(&str) -> Option<OsString>,
{
    env_var(name).and_then(|value| value.into_string().ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionPhase {
    AwaitingInitialize,
    AwaitingInitialized,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionState {
    pub(crate) phase: ConnectionPhase,
    pub(crate) client_supports_elicitation: bool,
    pub(crate) next_server_request_id: u64,
    pub(crate) session_id: String,
    pub(crate) managed_host_lifecycle_observations: bool,
    pub(crate) launch_origin: &'static str,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::AwaitingInitialize,
            client_supports_elicitation: false,
            next_server_request_id: 1,
            session_id: generated_metadata_id("session", "mcp", "stdio"),
            managed_host_lifecycle_observations: false,
            launch_origin: McpLaunchOrigin::Unknown.as_str(),
        }
    }
}

impl ConnectionState {
    fn for_launch_origin(launch_origin: McpLaunchOrigin) -> Self {
        Self {
            managed_host_lifecycle_observations: launch_origin == McpLaunchOrigin::ManagedHost,
            launch_origin: launch_origin.as_str(),
            ..Self::default()
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ClientMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcRequest {
    pub(crate) id: Value,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcNotification {
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcFailure {
    pub(crate) id: Value,
    pub(crate) code: i64,
    pub(crate) message: &'static str,
    pub(crate) data: Option<String>,
}

pub(crate) fn handle_json_rpc_message(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    message: Value,
    lines: &mut io::Lines<impl BufRead>,
    writer: &mut impl Write,
) -> Result<Option<Value>, McpAdapterError> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            handle_json_rpc_request(adapter, state, request, lines, writer).map(Some)
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(state, notification);
            Ok(None)
        }
        Err(error) => Ok(Some(json_rpc_error(
            error.id,
            error.code,
            error.message,
            error.data,
        ))),
    }
}

pub(crate) fn parse_client_message(message: Value) -> Result<ClientMessage, JsonRpcFailure> {
    let object = match message {
        Value::Object(object) => object,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            return Err(invalid_request(
                Value::Null,
                "message must be a JSON object",
            ));
        }
    };

    let id = match object.get("id") {
        Some(value) => Some(valid_request_id(value)?),
        None => None,
    };
    let response_id = id.clone().unwrap_or(Value::Null);

    match object.get("jsonrpc") {
        Some(Value::String(version)) if version == "2.0" => (),
        _ => {
            return Err(invalid_request(
                response_id,
                "jsonrpc must be exactly \"2.0\"",
            ));
        }
    }

    let Some(Value::String(method)) = object.get("method") else {
        return Err(invalid_request(response_id, "method must be a string"));
    };
    let params = object.get("params").cloned();

    if let Some(id) = id {
        Ok(ClientMessage::Request(JsonRpcRequest {
            id,
            method: method.clone(),
            params,
        }))
    } else {
        Ok(ClientMessage::Notification(JsonRpcNotification {
            method: method.clone(),
            params,
        }))
    }
}

pub(crate) fn valid_request_id(value: &Value) -> Result<Value, JsonRpcFailure> {
    match value {
        Value::String(_) => Ok(value.clone()),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid_request(
                Value::Null,
                "id must be a string or integer",
            ))
        }
    }
}

pub(crate) fn handle_json_rpc_notification(
    state: &mut ConnectionState,
    notification: JsonRpcNotification,
) {
    if notification.method == "notifications/initialized"
        && state.phase == ConnectionPhase::AwaitingInitialized
        && notification_params_are_object_or_absent(notification.params.as_ref())
    {
        state.phase = ConnectionPhase::Ready;
    }
}

pub(crate) fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

pub(crate) fn handle_json_rpc_request<R, W>(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    request: JsonRpcRequest,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<Value, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    if let Some(error) = lifecycle_error(state.phase, &request) {
        return Ok(error);
    }

    let response_id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => {
            match validate_initialize_params(&response_id, request.params) {
                Ok(capabilities) => {
                    state.client_supports_elicitation = capabilities.elicitation;
                    state.phase = ConnectionPhase::AwaitingInitialized;
                }
                Err(error) => return Ok(error),
            }
            record_managed_lifecycle_event(
                adapter,
                state,
                ManagedLifecycleEvent::InitializeResponse,
                None,
            );
            initialize_result()
        }
        "ping" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "ping")
            {
                return Ok(error);
            }
            json!({})
        }
        "tools/list" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "tools/list")
            {
                return Ok(error);
            }
            match adapter.tools() {
                Ok(tools) => {
                    record_managed_lifecycle_event(
                        adapter,
                        state,
                        ManagedLifecycleEvent::ToolsList,
                        None,
                    );
                    json!({ "tools": tools })
                }
                Err(error) => return Ok(json_rpc_error_for_adapter(response_id, error)),
            }
        }
        "tools/call" => {
            match call_tool_result_with_elicitation(
                adapter,
                &response_id,
                request.params,
                state,
                lines,
                writer,
            )? {
                Ok(result) => result,
                Err(error) => return Ok(error),
            }
        }
        _ => {
            return Ok(json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(request.method),
            ))
        }
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": response_id,
        "result": result
    }))
}

pub(crate) fn lifecycle_error(state: ConnectionPhase, request: &JsonRpcRequest) -> Option<Value> {
    match state {
        ConnectionPhase::AwaitingInitialize if request.method != "initialize" => Some(
            invalid_request_response(&request.id, "initialize must be the first request"),
        ),
        ConnectionPhase::AwaitingInitialize => None,
        ConnectionPhase::AwaitingInitialized => match request.method.as_str() {
            "initialize" => Some(invalid_request_response(
                &request.id,
                "initialize has already completed",
            )),
            "tools/list" => None,
            "tools/call" => Some(invalid_request_response(
                &request.id,
                "tools/call requires notifications/initialized",
            )),
            _ => None,
        },
        ConnectionPhase::Ready if request.method == "initialize" => Some(invalid_request_response(
            &request.id,
            "initialize has already completed",
        )),
        ConnectionPhase::Ready => None,
    }
}

fn record_managed_lifecycle_event(
    adapter: &McpAdapter,
    state: &ConnectionState,
    lifecycle_event: ManagedLifecycleEvent,
    tool_name: Option<&str>,
) {
    if !state.managed_host_lifecycle_observations {
        return;
    }
    let _observation = adapter.managed_lifecycle_observation_best_effort(
        &state.session_id,
        state.launch_origin,
        lifecycle_event,
        tool_name,
    );
}

pub(crate) fn initialize_result() -> Value {
    let build = crate::build_info();
    let package_version = build.package_version;
    json!({
        "_meta": {
            "io.volicord/build": build
        },
        "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": package_version
        },
        "instructions": SERVER_INSTRUCTIONS
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClientCapabilities {
    elicitation: bool,
}

fn validate_initialize_params(
    id: &Value,
    params: Option<Value>,
) -> Result<ClientCapabilities, Value> {
    let object = required_object_params(id, params, "initialize")?;
    if !matches!(object.get("protocolVersion"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.protocolVersion must be a string",
        ));
    }
    if !matches!(object.get("capabilities"), Some(Value::Object(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.capabilities must be an object",
        ));
    }
    let Some(Value::Object(client_info)) = object.get("clientInfo") else {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo must be an object",
        ));
    };
    if !matches!(client_info.get("name"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo.name must be a string",
        ));
    }
    if !matches!(client_info.get("version"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo.version must be a string",
        ));
    }

    let elicitation = object
        .get("capabilities")
        .and_then(Value::as_object)
        .and_then(|capabilities| capabilities.get("elicitation"))
        .is_some_and(Value::is_object);

    Ok(ClientCapabilities { elicitation })
}

pub(crate) fn validate_optional_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<(), Value> {
    match params {
        None | Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

pub(crate) fn required_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<Map<String, Value>, Value> {
    match params {
        Some(Value::Object(object)) => Ok(object),
        None | Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

pub(crate) fn call_tool_result_with_elicitation<R, W>(
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
    state: &mut ConnectionState,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<Result<Value, Value>, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let diagnostic_started = Instant::now();
    let diagnostic_request_bytes = params
        .as_ref()
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let diagnostic_tool_name = params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|object| object.get("name"))
        .and_then(Value::as_str)
        .filter(|tool_name| is_known_mcp_tool(tool_name))
        .map(str::to_owned);
    let object = match required_object_params(id, params, "tools/call") {
        Ok(object) => object,
        Err(error) => {
            record_tool_diagnostic_best_effort(
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                diagnostic_tool_name.as_deref(),
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    if object.contains_key("task") {
        let error = invalid_params_response(id, "tools/call task augmentation is not supported");
        record_tool_diagnostic_best_effort(
            adapter,
            state,
            diagnostic_started,
            diagnostic_request_bytes,
            diagnostic_tool_name.as_deref(),
            Some(&error),
            ToolDiagnosticFacts::default(),
            true,
            DiagnosticOutcome::ValidationFailure,
        );
        return Ok(Err(error));
    }

    let tool_name = match object.get("name").and_then(Value::as_str) {
        Some(tool_name) => tool_name,
        None => {
            let error = invalid_params_response(id, "tools/call params.name must be a string");
            record_tool_diagnostic_best_effort(
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                None,
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    if !is_known_mcp_tool(tool_name) {
        let error = json_rpc_error(
            id.clone(),
            -32602,
            "Invalid params",
            Some(format!("unknown MCP tool: {tool_name}")),
        );
        record_tool_diagnostic_best_effort(
            adapter,
            state,
            diagnostic_started,
            diagnostic_request_bytes,
            None,
            Some(&error),
            ToolDiagnosticFacts::default(),
            true,
            DiagnosticOutcome::ValidationFailure,
        );
        return Ok(Err(error));
    }
    record_managed_lifecycle_event(
        adapter,
        state,
        ManagedLifecycleEvent::ToolCallReceived,
        Some(tool_name),
    );

    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            let error =
                invalid_params_response(id, "tools/call params.arguments must be an object");
            record_tool_diagnostic_best_effort(
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                Some(tool_name),
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    let mutation_detail = mutation_detail_for_tool(tool_name, &arguments);

    let session_id = state.session_id.clone();
    let output = if PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) {
        match adapter.call_tool_for_session_with_capabilities(
            tool_name,
            arguments,
            Some(&session_id),
            state.client_supports_elicitation,
        ) {
            Ok(response) if tool_name == REQUEST_USER_JUDGMENT_TOOL_NAME => {
                let pending_response = response.clone();
                match user_judgment_tool_output(
                    adapter,
                    response,
                    state.client_supports_elicitation,
                    &mut state.next_server_request_id,
                    lines,
                    writer,
                ) {
                    Ok(output) => output,
                    Err(_) => ToolCallOutput::from_pipeline_response(&pending_response)?
                        .with_post_effect_failure(
                            McpPostEffectFailureCode::McpPostEffectAdapterFailed,
                        ),
                }
            }
            Ok(response) => ToolCallOutput::from_pipeline_response(&response)?,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    true,
                    DiagnosticOutcome::ValidationFailure,
                );
                return Ok(Ok(response));
            }
            Err(error @ McpAdapterError::ToolExecution { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::ToolError,
                );
                return Ok(Ok(response));
            }
            Err(error) => {
                let response = json_rpc_error_for_adapter(id.clone(), error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(response));
            }
        }
    } else {
        let response = match adapter.call_adapter_tool(tool_name, arguments, Some(&session_id)) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    true,
                    DiagnosticOutcome::ValidationFailure,
                );
                return Ok(Ok(response));
            }
            Err(error @ McpAdapterError::ToolExecution { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::ToolError,
                );
                return Ok(Ok(response));
            }
            Err(error) => {
                let response = json_rpc_error_for_adapter(id.clone(), error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(response));
            }
        };
        let text = serde_json::to_string(&response)
            .map_err(McpAdapterError::Json)
            .map_err(|error| json_rpc_error_for_adapter(id.clone(), error));
        match text {
            Ok(text) => ToolCallOutput::success(text)?,
            Err(error) => {
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&error),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(error));
            }
        }
    };
    let output = finalize_mutation_output(adapter, state, tool_name, mutation_detail, output)?;

    record_managed_lifecycle_event(
        adapter,
        state,
        ManagedLifecycleEvent::ToolCallCompleted,
        Some(tool_name),
    );
    let diagnostic_facts = output.diagnostic_facts();
    let diagnostic_outcome =
        if output.structured_content["base"]["response_kind"].as_str() == Some("rejected") {
            DiagnosticOutcome::Rejected
        } else if output.is_error {
            DiagnosticOutcome::ToolError
        } else {
            DiagnosticOutcome::Success
        };
    let response = tool_call_result_from_output(output);
    record_tool_diagnostic_best_effort(
        adapter,
        state,
        diagnostic_started,
        diagnostic_request_bytes,
        Some(tool_name),
        Some(&response),
        diagnostic_facts,
        false,
        diagnostic_outcome,
    );
    Ok(Ok(response))
}

fn mutation_detail_for_tool(tool_name: &str, arguments: &Value) -> Option<MutationDetailLevel> {
    (!READ_ONLY_METHOD_TOOL_NAMES.contains(&tool_name)
        && PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name))
    .then(|| {
        arguments
            .get("detail")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MutationRefreshContext {
    project_id: ProjectId,
    task_id: TaskId,
}

impl MutationRefreshContext {
    fn from_pipeline_response(response: &PipelineResponse) -> Option<Self> {
        Some(Self {
            project_id: response.verified_invocation.as_ref()?.project_id.clone(),
            task_id: response.resolved_task_id.clone()?,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ToolDiagnosticFacts {
    core_reached: bool,
    core_committed: bool,
    replayed: bool,
    effect_kind: Option<EffectKind>,
    effect_applied: bool,
    operation_token: Option<String>,
    user_channel_kind: Option<DiagnosticUserChannelKind>,
    fallback_kind: Option<DiagnosticFallbackKind>,
    product_file_write_count: u64,
    authoritative_refresh_failure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCallOutput {
    primary_text: String,
    structured_content: Value,
    extra_texts: Vec<String>,
    is_error: bool,
    diagnostic_facts: ToolDiagnosticFacts,
    mutation_refresh_context: Option<MutationRefreshContext>,
    post_effect_failure: Option<McpPostEffectFailureCode>,
}

impl ToolCallOutput {
    fn success(primary_text: String) -> Result<Self, McpAdapterError> {
        let structured_content: Value =
            serde_json::from_str(&primary_text).map_err(McpAdapterError::Json)?;
        if !structured_content.is_object() {
            return Err(McpAdapterError::Protocol(
                "successful MCP tool output must be a JSON object".to_owned(),
            ));
        }
        Ok(Self {
            primary_text,
            structured_content,
            extra_texts: Vec::new(),
            is_error: false,
            diagnostic_facts: ToolDiagnosticFacts::default(),
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    }

    fn from_pipeline_response(response: &PipelineResponse) -> Result<Self, McpAdapterError> {
        let mut output = Self::success(response.response_json.clone())?;
        output.apply_pipeline_diagnostics(response);
        Ok(output)
    }

    fn with_pipeline_diagnostics(mut self, response: &PipelineResponse) -> Self {
        self.apply_pipeline_diagnostics(response);
        self
    }

    fn apply_pipeline_diagnostics(&mut self, response: &PipelineResponse) {
        self.diagnostic_facts.core_reached = response.verified_invocation.is_some();
        self.diagnostic_facts.effect_kind = response
            .response_value
            .pointer("/base/effect_kind")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        self.diagnostic_facts.core_committed = !response.replayed
            && self.diagnostic_facts.effect_kind == Some(EffectKind::CoreCommitted);
        self.diagnostic_facts.effect_applied = matches!(
            self.diagnostic_facts.effect_kind,
            Some(EffectKind::CoreCommitted | EffectKind::StagingCreated)
        );
        self.diagnostic_facts.operation_token = mutation_operation_token(response);
        self.diagnostic_facts.replayed = response.replayed;
        self.diagnostic_facts.product_file_write_count = response
            .response_value
            .pointer("/run_summary/observed_changes/product_file_write_observed")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            as u64;
        self.mutation_refresh_context = MutationRefreshContext::from_pipeline_response(response);
    }

    fn with_user_channel(mut self, channel: DiagnosticUserChannelKind) -> Self {
        self.diagnostic_facts.user_channel_kind = Some(channel);
        self
    }

    fn with_fallback(mut self, fallback: DiagnosticFallbackKind) -> Self {
        self.diagnostic_facts.fallback_kind = Some(fallback);
        self
    }

    fn with_post_effect_failure(mut self, code: McpPostEffectFailureCode) -> Self {
        self.post_effect_failure = Some(code);
        self
    }

    fn diagnostic_facts(&self) -> ToolDiagnosticFacts {
        self.diagnostic_facts.clone()
    }

    fn with_extra(mut self, text: impl Into<String>) -> Self {
        self.extra_texts.push(text.into());
        self
    }

    fn with_extras(mut self, texts: impl IntoIterator<Item = String>) -> Self {
        self.extra_texts.extend(texts);
        self
    }
}

fn mutation_operation_token(response: &PipelineResponse) -> Option<String> {
    if let Some(event_id) = response
        .response_value
        .pointer("/base/events/0/event_id")
        .and_then(Value::as_str)
    {
        return Some(format!("authority_event:{event_id}"));
    }
    if let Some(handle_id) = response
        .response_value
        .pointer("/staged_artifact_handle/handle_id")
        .and_then(Value::as_str)
    {
        return Some(format!("staged_artifact:{handle_id}"));
    }
    let effect_kind = response
        .response_value
        .pointer("/base/effect_kind")
        .and_then(Value::as_str)?;
    if !matches!(effect_kind, "core_committed" | "staging_created") {
        return None;
    }
    let project_id = response.verified_invocation.as_ref()?.project_id.as_str();
    let state_version = response
        .response_value
        .pointer("/base/state_version")
        .and_then(Value::as_u64)?;
    Some(format!("state_effect:{project_id}:{state_version}"))
}

fn finalize_mutation_output(
    adapter: &McpAdapter,
    state: &ConnectionState,
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    output: ToolCallOutput,
) -> Result<ToolCallOutput, McpAdapterError> {
    finalize_mutation_output_with_refresh(tool_name, detail, output, |context| {
        adapter.refresh_authority_status(
            &context.project_id,
            &context.task_id,
            Some(&state.session_id),
            state.client_supports_elicitation,
        )
    })
}

fn finalize_mutation_output_with_refresh<F>(
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    mut output: ToolCallOutput,
    refresh: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: FnOnce(&MutationRefreshContext) -> Result<PipelineResponse, McpAdapterError>,
{
    let Some(detail) = detail else {
        return Ok(output);
    };
    if output.is_error {
        return Ok(output);
    }
    if output.structured_content["base"]["response_kind"].as_str() != Some("result") {
        output.primary_text = bounded_mutation_compatibility_text(format!(
            "Volicord {tool_name} returned response_kind={}; inspect structuredContent for the authoritative result.",
            output.structured_content["base"]["response_kind"]
                .as_str()
                .unwrap_or("unknown")
        ));
        return Ok(output);
    }

    let original_method_result = output.structured_content.clone();
    let compact_method_result_for_refresh_failure =
        || compact_mutation_method_result(tool_name, &original_method_result).ok();
    let Some(context) = output.mutation_refresh_context.clone() else {
        return authoritative_refresh_failure_output(
            tool_name,
            output.diagnostic_facts,
            compact_method_result_for_refresh_failure(),
        );
    };
    let (receipt, next_actions) = match refresh(&context) {
        Ok(response) => match validated_authority_refresh(&context, &response) {
            Ok(refreshed) => refreshed,
            Err(()) => {
                return authoritative_refresh_failure_output(
                    tool_name,
                    output.diagnostic_facts,
                    compact_method_result_for_refresh_failure(),
                )
            }
        },
        Err(_) => {
            return authoritative_refresh_failure_output(
                tool_name,
                output.diagnostic_facts,
                compact_method_result_for_refresh_failure(),
            )
        }
    };

    if let Some(code) = output.post_effect_failure {
        return mutation_post_effect_failure_output(
            tool_name,
            detail,
            code,
            output.diagnostic_facts,
            receipt,
            None,
        );
    }
    output.primary_text = match authority_receipt_compatibility_text(tool_name, &receipt) {
        Ok(text) => text,
        Err(_) => {
            return mutation_post_effect_failure_output(
                tool_name,
                detail,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
                output.diagnostic_facts,
                receipt,
                Some(original_method_result),
            )
        }
    };
    output.mutation_refresh_context = None;
    let compact_recovery_method_result =
        match compact_mutation_method_result(tool_name, &original_method_result) {
            Ok(method_result) => method_result,
            Err(_) => {
                return mutation_post_effect_failure_output(
                    tool_name,
                    detail,
                    McpPostEffectFailureCode::McpResponseProjectionFailed,
                    output.diagnostic_facts,
                    receipt,
                    Some(original_method_result),
                )
            }
        };
    let method_result = match detail {
        MutationDetailLevel::Full => original_method_result.clone(),
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => {
            compact_recovery_method_result.clone()
        }
    };
    let projected = match detail {
        MutationDetailLevel::Summary => serde_json::to_value(McpMutationSummaryResponse {
            authority_receipt: receipt.clone(),
            method_result,
        }),
        MutationDetailLevel::Workflow => serde_json::to_value(McpMutationWorkflowResponse {
            authority_receipt: receipt.clone(),
            method_result,
            next_actions,
        }),
        MutationDetailLevel::Full => serde_json::to_value(McpMutationFullResponse {
            authority_receipt: receipt.clone(),
            method_result,
        }),
    };
    output.structured_content = match projected {
        Ok(projected) => projected,
        Err(_) => {
            return mutation_post_effect_failure_output(
                tool_name,
                detail,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
                output.diagnostic_facts,
                receipt,
                Some(original_method_result),
            )
        }
    };

    let result = tool_call_result_from_output(output.clone());
    let response_budget = match detail {
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => {
            MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        }
        MutationDetailLevel::Full => MAX_MCP_FULL_MUTATION_RESULT_BYTES,
    };
    let rendered_size = match serde_json::to_vec(&result) {
        Ok(rendered) => rendered.len(),
        Err(_) => {
            return mutation_post_effect_failure_output(
                tool_name,
                detail,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
                output.diagnostic_facts,
                receipt,
                Some(original_method_result),
            )
        }
    };
    if rendered_size > response_budget {
        return mutation_response_budget_exceeded_output(
            tool_name,
            detail,
            output.diagnostic_facts,
            compact_recovery_method_result,
        );
    }
    Ok(output)
}

fn compact_mutation_method_result(
    tool_name: &str,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let effect = compact_mutation_effect(method_result)?;
    match tool_name {
        PREPARE_WRITE_TOOL_NAME => {
            let result: PrepareWriteResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpPrepareWriteCompactResult {
                effect,
                decision: result.decision,
                write_ticket_id: result.write_ticket_id,
                write_ticket_ref: result.write_ticket_ref,
                write_ticket: result.write_ticket,
                write_ticket_effect: result.write_ticket_effect,
                allowed_path_patterns: result.allowed_path_patterns,
                denied_path_patterns: result.denied_path_patterns,
                write_decision_reasons: result.write_decision_reasons,
                user_judgment_candidate: result.user_judgment_candidate,
            })
            .map_err(McpAdapterError::Json)
        }
        STAGE_ARTIFACT_TOOL_NAME => {
            let result: StageArtifactResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpStageArtifactCompactResult {
                effect,
                evidence_state: result.evidence_state,
                staged_artifact_handle: result.staged_artifact_handle,
                expires_at: result.expires_at,
            })
            .map_err(McpAdapterError::Json)
        }
        REQUEST_USER_JUDGMENT_TOOL_NAME => {
            compact_request_user_judgment_result(effect, method_result)
        }
        RECONCILE_CHANGES_TOOL_NAME => {
            let result: ReconcileChangesResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpReconcileChangesCompactResult {
                effect,
                unresolved_changes: result.unresolved_changes,
                resolved_changes: result.resolved_changes,
                pending_user_judgment_refs: result.pending_user_judgment_refs,
                rejected_resolution_requests: result.rejected_resolution_requests,
            })
            .map_err(McpAdapterError::Json)
        }
        INTAKE_TOOL_NAME | UPDATE_SCOPE_TOOL_NAME | RECORD_RUN_TOOL_NAME | CLOSE_TASK_TOOL_NAME => {
            serde_json::to_value(effect).map_err(McpAdapterError::Json)
        }
        _ => Err(McpAdapterError::Protocol(format!(
            "missing compact mutation result projection for {tool_name}"
        ))),
    }
}

fn compact_mutation_effect(
    method_result: &Value,
) -> Result<McpMutationEffectSummary, McpAdapterError> {
    let base: ToolResultBase =
        serde_json::from_value(method_result["base"].clone()).map_err(McpAdapterError::Json)?;
    Ok(McpMutationEffectSummary {
        effect_kind: base.effect_kind,
        state_version: base.state_version,
        events: base.events,
    })
}

fn compact_request_user_judgment_result(
    effect: McpMutationEffectSummary,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let judgment_ref: StateRecordRef =
        serde_json::from_value(method_result["user_judgment_ref"].clone())
            .map_err(McpAdapterError::Json)?;
    let judgment: UserJudgment = serde_json::from_value(method_result["user_judgment"].clone())
        .map_err(McpAdapterError::Json)?;
    let (selected_option_id, selected_option_label, resolution_outcome) =
        if let Some(resolution) = judgment.resolution.as_ref() {
            let selected_option = judgment
                .options
                .iter()
                .find(|option| option.option_id == resolution.selected_option_id)
                .ok_or_else(|| {
                    McpAdapterError::Protocol(format!(
                        "resolved judgment {} does not contain selected option {}",
                        judgment.judgment_id.as_str(),
                        resolution.selected_option_id.as_str()
                    ))
                })?;
            (
                Some(resolution.selected_option_id.clone()),
                Some(selected_option.label.clone()),
                Some(resolution.resolution_outcome),
            )
        } else {
            (None, None, None)
        };
    serde_json::to_value(McpRequestUserJudgmentCompactResult {
        effect,
        judgment_ref,
        status: judgment.status,
        selected_option_id: selected_option_id.into(),
        selected_option_label: selected_option_label.into(),
        resolution_outcome: resolution_outcome.into(),
    })
    .map_err(McpAdapterError::Json)
}

fn validated_authority_refresh(
    context: &MutationRefreshContext,
    response: &PipelineResponse,
) -> Result<(AuthorityReceipt, Vec<NextActionSummary>), ()> {
    let status =
        serde_json::from_value::<StatusResult>(response.response_value.clone()).map_err(|_| ())?;
    if status.base.response_kind != ResponseKind::Result
        || status.base.effect_kind != EffectKind::ReadOnly
        || status.base.dry_run
    {
        return Err(());
    }
    let state_version = status.base.state_version.ok_or(())?;
    let receipt = status.authority_receipt.clone().ok_or(())?;
    let active_task = status.active_task.as_ref().ok_or(())?;
    let active_task_ref = active_task.task_ref.as_ref().ok_or(())?;
    if receipt.project_id != context.project_id
        || receipt.task_ref.project_id != context.project_id
        || receipt.task_ref.record_id.as_str() != context.task_id.as_str()
        || receipt.task_ref.task_id.as_ref() != Some(&context.task_id)
        || receipt.state_version != state_version
        || receipt.task_ref.produced_at_state_version.as_ref() != Some(&state_version)
        || active_task.project_id != context.project_id
        || active_task.state_version != state_version
        || active_task_ref != &receipt.task_ref
        || active_task.scope_revision != receipt.scope_revision
        || !optional_state_record_identity_matches(
            active_task.active_change_unit_ref.as_ref(),
            receipt.change_unit_ref.as_ref(),
        )
        || status.close_state != Some(receipt.close_state)
        || status.close_blockers.as_ref() != Some(&receipt.close_blockers)
        || status
            .evidence_gate
            .as_ref()
            .and_then(RequiredNullable::as_ref)
            != receipt.evidence_gate.as_ref()
        || receipt
            .next_action
            .as_ref()
            .is_some_and(|action| !status.next_actions.contains(action))
    {
        return Err(());
    }
    Ok((receipt, status.next_actions))
}

fn optional_state_record_identity_matches(
    left: Option<&StateRecordRef>,
    right: Option<&StateRecordRef>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.record_kind == right.record_kind
                && left.record_id == right.record_id
                && left.project_id == right.project_id
                && left.task_id == right.task_id
        }
        _ => false,
    }
}

fn authority_receipt_compatibility_text(
    tool_name: &str,
    receipt: &AuthorityReceipt,
) -> Result<String, McpAdapterError> {
    let close_state = serde_json::to_value(receipt.close_state)
        .map_err(McpAdapterError::Json)?
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    let next_actor = serde_json::to_value(receipt.next_actor)
        .map_err(McpAdapterError::Json)?
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    Ok(bounded_mutation_compatibility_text(format!(
        "Volicord {tool_name} refreshed Task {} at state_version {}; close_state={close_state}; next_actor={next_actor}. Inspect structuredContent for the authority receipt.",
        receipt.task_ref.record_id.as_str(),
        receipt.state_version,
    )))
}

fn mutation_response_budget_exceeded_output(
    tool_name: &str,
    requested_detail: MutationDetailLevel,
    mut facts: ToolDiagnosticFacts,
    method_result: Value,
) -> Result<ToolCallOutput, McpAdapterError> {
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let requested_detail_label = match requested_detail {
        MutationDetailLevel::Summary => "summary",
        MutationDetailLevel::Workflow => "workflow",
        MutationDetailLevel::Full => "full",
    };
    facts.authoritative_refresh_failure = false;
    let build_output = |method_result: Option<Value>| -> Result<ToolCallOutput, McpAdapterError> {
        let method_result_preserved = method_result.is_some();
        let structured_content = serde_json::to_value(McpMutationResponseBudgetExceeded::<Value> {
            code: McpMutationProjectionErrorCode::McpResponseBudgetExceeded,
            tool_name: method_name,
            requested_detail,
            retryable: false,
            reached_core: facts.core_reached,
            committed: facts.core_committed,
            effect_kind: facts.effect_kind.into(),
            effect_applied: facts.effect_applied,
            operation_token: facts.operation_token.clone().into(),
            method_result: RequiredNullable::new(method_result),
            authoritative_refresh_succeeded: true,
            response_projection_omitted: true,
            status_read_required: true,
            completion_claim_withheld: true,
        })
        .map_err(McpAdapterError::Json)?;
        let method_result_guidance = if method_result_preserved {
            "The compact method_result is preserved; use it and read volicord.status"
        } else {
            "The compact method_result also exceeded the recovery budget; read volicord.status"
        };
        Ok(ToolCallOutput {
            primary_text: bounded_mutation_compatibility_text(format!(
                "Volicord {tool_name} reached Core (effect_applied={}, committed={}) and refreshed current authority, but the requested {requested_detail_label} projection exceeded the MCP response budget. {method_result_guidance} before making an authority claim. Do not retry this mutation.",
                facts.effect_applied, facts.core_committed
            )),
            structured_content,
            extra_texts: Vec::new(),
            is_error: false,
            diagnostic_facts: facts.clone(),
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    };
    let output = build_output(Some(method_result))?;
    if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
        return Ok(output);
    }
    let output = build_output(None)?;
    if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
        Ok(output)
    } else {
        Err(McpAdapterError::Protocol(
            "bounded mutation budget recovery exceeded its fixed output budget".to_owned(),
        ))
    }
}

fn mutation_post_effect_failure_output(
    tool_name: &str,
    requested_detail: MutationDetailLevel,
    code: McpPostEffectFailureCode,
    mut facts: ToolDiagnosticFacts,
    authority_receipt: AuthorityReceipt,
    method_result: Option<Value>,
) -> Result<ToolCallOutput, McpAdapterError> {
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    facts.authoritative_refresh_failure = false;
    let method_result = method_result
        .map(|method_result| {
            method_result.as_object().cloned().ok_or_else(|| {
                McpAdapterError::Protocol(
                    "post-effect method_result must remain a JSON object".to_owned(),
                )
            })
        })
        .transpose()?;
    let build_output = |authority_receipt: Option<AuthorityReceipt>,
                        method_result: Option<JsonObject>,
                        response_projection_omitted: bool|
     -> Result<ToolCallOutput, McpAdapterError> {
        let structured_content = serde_json::to_value(McpMutationPostEffectFailure {
            code,
            tool_name: method_name,
            requested_detail,
            retryable: false,
            reached_core: facts.core_reached,
            committed: facts.core_committed,
            effect_kind: facts.effect_kind.into(),
            effect_applied: facts.effect_applied,
            operation_token: facts.operation_token.clone().into(),
            authority_receipt: authority_receipt.into(),
            method_result: method_result.into(),
            authoritative_refresh_succeeded: true,
            response_projection_omitted,
            status_read_required: true,
            completion_claim_withheld: true,
        })
        .map_err(McpAdapterError::Json)?;
        Ok(ToolCallOutput {
            primary_text: bounded_mutation_compatibility_text(format!(
                "Volicord {tool_name} observed an applied mutation effect and refreshed current authority, but post-effect adapter work could not produce the normal response projection. Do not retry this mutation; inspect structuredContent and read volicord.status before acting."
            )),
            structured_content,
            extra_texts: Vec::new(),
            is_error: false,
            diagnostic_facts: facts.clone(),
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    };
    let output = build_output(Some(authority_receipt), method_result.clone(), false)?;
    if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
        return Ok(output);
    }
    let output = build_output(None, method_result, true)?;
    if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
        return Ok(output);
    }
    let output = build_output(None, None, true)?;
    if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
        Ok(output)
    } else {
        Err(McpAdapterError::Protocol(
            "bounded post-effect recovery exceeded its fixed output budget".to_owned(),
        ))
    }
}

fn rendered_tool_call_output_size(output: &ToolCallOutput) -> Result<usize, McpAdapterError> {
    serde_json::to_vec(&tool_call_result_from_output(output.clone()))
        .map(|rendered| rendered.len())
        .map_err(McpAdapterError::Json)
}

fn authoritative_refresh_failure_output(
    tool_name: &str,
    mut facts: ToolDiagnosticFacts,
    method_result: Option<Value>,
) -> Result<ToolCallOutput, McpAdapterError> {
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    facts.authoritative_refresh_failure = true;
    let build_output = |method_result: Option<Value>| -> Result<ToolCallOutput, McpAdapterError> {
        let method_result_preserved = method_result.is_some();
        let structured_content = serde_json::to_value(McpAuthoritativeRefreshFailure::<Value> {
            code: ErrorCode::McpUnavailable,
            tool_name: method_name,
            retryable: false,
            reached_core: facts.core_reached,
            committed: facts.core_committed,
            effect_kind: facts.effect_kind.into(),
            effect_applied: facts.effect_applied,
            operation_token: facts.operation_token.clone().into(),
            method_result: RequiredNullable::new(method_result),
            status_read_required: true,
            completion_claim_withheld: true,
        })
        .map_err(McpAdapterError::Json)?;
        let method_result_guidance = if method_result_preserved {
            "The compact method_result is preserved; use it and read volicord.status"
        } else {
            "The compact method_result could not be included; read volicord.status"
        };
        Ok(ToolCallOutput {
            primary_text: bounded_mutation_compatibility_text(format!(
                "Volicord withheld the {tool_name} success or completion claim because authoritative status refresh was unavailable. {method_result_guidance} before acting. Do not retry this mutation."
            )),
            structured_content,
            extra_texts: Vec::new(),
            is_error: false,
            diagnostic_facts: facts.clone(),
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    };
    if let Some(method_result) = method_result {
        let output = build_output(Some(method_result))?;
        if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
            return Ok(output);
        }
    }
    let output = build_output(None)?;
    if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
        Ok(output)
    } else {
        Err(McpAdapterError::Protocol(
            "bounded authoritative refresh recovery exceeded its fixed output budget".to_owned(),
        ))
    }
}

fn method_name_for_tool(tool_name: &str) -> Option<MethodName> {
    match tool_name {
        INTAKE_TOOL_NAME => Some(MethodName::Intake),
        UPDATE_SCOPE_TOOL_NAME => Some(MethodName::UpdateScope),
        PREPARE_WRITE_TOOL_NAME => Some(MethodName::PrepareWrite),
        STAGE_ARTIFACT_TOOL_NAME => Some(MethodName::StageArtifact),
        RECORD_RUN_TOOL_NAME => Some(MethodName::RecordRun),
        REQUEST_USER_JUDGMENT_TOOL_NAME => Some(MethodName::RequestUserJudgment),
        RECONCILE_CHANGES_TOOL_NAME => Some(MethodName::ReconcileChanges),
        CLOSE_TASK_TOOL_NAME => Some(MethodName::CloseTask),
        _ => None,
    }
}

fn bounded_mutation_compatibility_text(mut text: String) -> String {
    if text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES {
        return text;
    }
    let mut boundary = MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES.saturating_sub(3);
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text.truncate(boundary);
    text.push_str("...");
    text
}

fn start_transport_diagnostic_session_best_effort(adapter: &McpAdapter, state: &ConnectionState) {
    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .ok()
    .flatten();
    let host_kind = connection
        .as_ref()
        .map(|record| DiagnosticHostKind::from_connection_host_kind(&record.host_kind));
    let project_id = adapter
        .context
        .project_allowlist
        .as_ref()
        .filter(|projects| projects.len() == 1)
        .and_then(|projects| projects.first())
        .map(|project| project.as_str().to_owned())
        .or_else(|| {
            list_connection_projects_read_only(
                &adapter.runtime_home,
                adapter.context.connection_internal_id.as_str(),
            )
            .ok()
            .filter(|projects| projects.len() == 1)
            .and_then(|projects| projects.first().map(|project| project.project_id.clone()))
        });
    let transport = if state.launch_origin == McpLaunchOrigin::Unknown.as_str() {
        DiagnosticTransport::LocalHttp
    } else {
        DiagnosticTransport::McpStdio
    };
    let build = crate::build_info();
    let _ = start_diagnostic_session(
        &adapter.runtime_home,
        DiagnosticSessionStart {
            session_id: &state.session_id,
            connection_id: Some(adapter.context.connection_internal_id.as_str()),
            project_id: project_id.as_deref(),
            transport,
            host_kind,
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn record_tool_diagnostic_best_effort(
    adapter: &McpAdapter,
    state: &ConnectionState,
    started: Instant,
    request_bytes: u64,
    tool_name: Option<&str>,
    response: Option<&Value>,
    facts: ToolDiagnosticFacts,
    validation_failure: bool,
    outcome: DiagnosticOutcome,
) {
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let response_bytes = response
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    start_transport_diagnostic_session_best_effort(adapter, state);
    let _ = record_diagnostic_event(
        &adapter.runtime_home,
        DiagnosticEvent {
            session_id: &state.session_id,
            event_kind: DiagnosticEventKind::McpToolCall,
            tool_name,
            latency_micros: elapsed,
            request_bytes,
            response_bytes,
            validation_failure,
            core_reached: facts.core_reached,
            core_committed: facts.core_committed,
            replayed: facts.replayed,
            user_channel_kind: facts.user_channel_kind,
            fallback_kind: facts.fallback_kind,
            product_file_write_count: facts.product_file_write_count,
            authoritative_refresh_failure: facts.authoritative_refresh_failure,
            outcome,
        },
    );
}

pub(crate) fn tool_call_result_from_output(output: ToolCallOutput) -> Value {
    let mut content = vec![json!({
        "type": "text",
        "text": output.primary_text
    })];
    content.extend(output.extra_texts.into_iter().map(|text| {
        json!({
            "type": "text",
            "text": text
        })
    }));

    json!({
        "content": content,
        "structuredContent": output.structured_content,
        "isError": output.is_error
    })
}

pub(crate) fn user_judgment_tool_output<R, W>(
    adapter: &McpAdapter,
    pending_response: PipelineResponse,
    client_supports_elicitation: bool,
    server_request_sequence: &mut u64,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<ToolCallOutput, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let Some(pending) = pending_judgment_from_response(&pending_response) else {
        return ToolCallOutput::from_pipeline_response(&pending_response);
    };

    if !client_supports_elicitation {
        let fallback = user_judgment_fallback(adapter, &pending)?;
        let fallback_kind = fallback.kind;
        return Ok(ToolCallOutput::success(response_json_with_inbox_capture(
            &pending_response,
            &fallback,
        )?)?
        .with_pipeline_diagnostics(&pending_response)
        .with_fallback(fallback_kind)
        .with_extras(fallback.texts));
    }

    if let Some(reason) = elicitation_secret_request_risk(&pending) {
        let fallback = user_judgment_fallback(adapter, &pending)?;
        let fallback_kind = fallback.kind;
        return Ok(ToolCallOutput::success(response_json_with_inbox_capture(
            &pending_response,
            &fallback,
        )?)?
            .with_pipeline_diagnostics(&pending_response)
            .with_fallback(fallback_kind)
            .with_extra(format!(
                "Volicord did not open host prompt input for pending judgment `{}` because the prompt text appears to request or expose sensitive secret material ({reason}). Do not ask the user to enter secrets, credentials, tokens, or private keys through host prompt input.",
                pending.judgment_id.as_str()
            ))
            .with_extras(fallback.texts));
    }

    let request_id = next_server_request_id("elicit_user_judgment", server_request_sequence);
    let request = elicitation_create_request(&request_id, &pending);
    write_json_line(writer, request)?;
    writer.flush().map_err(McpAdapterError::Io)?;

    match read_elicitation_response(&request_id, lines) {
        ElicitationReply::Accepted {
            selected_option_id,
            note,
        } => match record_elicited_judgment(adapter, &pending, &selected_option_id, note)? {
            ElicitedRecordOutcome::Recorded(recorded) => Ok(
                ToolCallOutput::from_pipeline_response(&recorded)?
                    .with_user_channel(DiagnosticUserChannelKind::McpElicitation)
                    .with_extra(format!(
                "Volicord recorded pending judgment `{}` through host prompt input with User Channel basis `{}`.",
                pending.judgment_id.as_str(),
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            )),
            ),
            ElicitedRecordOutcome::PostEffectFailure => Ok(
                ToolCallOutput::from_pipeline_response(&pending_response)?
                    .with_post_effect_failure(
                        McpPostEffectFailureCode::McpPostEffectAdapterFailed,
                    ),
            ),
            ElicitedRecordOutcome::InvalidSelection(message) => Ok(
                ToolCallOutput::from_pipeline_response(&pending_response)?.with_extra(format!(
                "{message} The pending judgment remains unresolved."
            )),
            ),
        },
        ElicitationReply::Declined => match reject_option_id(&pending) {
            Some(option_id) => match record_elicited_judgment(adapter, &pending, option_id, None)? {
                ElicitedRecordOutcome::Recorded(recorded) => Ok(
                    ToolCallOutput::from_pipeline_response(&recorded)?
                        .with_user_channel(DiagnosticUserChannelKind::McpElicitation)
                        .with_extra(format!(
                    "Volicord recorded pending judgment `{}` as rejected through host prompt input with User Channel basis `{}`.",
                    pending.judgment_id.as_str(),
                    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
                )),
                ),
                ElicitedRecordOutcome::PostEffectFailure => Ok(
                    ToolCallOutput::from_pipeline_response(&pending_response)?
                        .with_post_effect_failure(
                            McpPostEffectFailureCode::McpPostEffectAdapterFailed,
                        ),
                ),
                ElicitedRecordOutcome::InvalidSelection(message) => Ok(
                    ToolCallOutput::from_pipeline_response(&pending_response)?.with_extra(format!(
                    "{message} The pending judgment remains unresolved."
                )),
                ),
            },
            None => Ok(ToolCallOutput::from_pipeline_response(&pending_response)?.with_extra(
                    "The MCP client declined the host prompt request, but this judgment has no reject option to record. The pending judgment remains unresolved.",
                )),
        },
        ElicitationReply::Cancelled => Ok(
            ToolCallOutput::from_pipeline_response(&pending_response)?.with_extra(format!(
                "The MCP client cancelled or dismissed host prompt input for pending judgment `{}`. Volicord did not record an answer; the judgment remains pending.",
                pending.judgment_id.as_str()
            )),
        ),
        ElicitationReply::Invalid(message) => Ok(
            ToolCallOutput::from_pipeline_response(&pending_response)?.with_extra(format!(
            "Volicord rejected the host prompt response: {message}. The pending judgment remains unresolved."
        )),
        ),
        ElicitationReply::Unavailable(message) => {
            let fallback = user_judgment_fallback(adapter, &pending)?;
            let fallback_kind = fallback.kind;
            Ok(ToolCallOutput::success(response_json_with_inbox_capture(
                &pending_response,
                &fallback,
            )?)?
            .with_pipeline_diagnostics(&pending_response)
            .with_fallback(fallback_kind)
            .with_extra(format!(
                "Host prompt input was unavailable after the client advertised support: {message}."
            ))
            .with_extras(fallback.texts))
        }
    }
}

pub(crate) fn pending_judgment_from_response(response: &PipelineResponse) -> Option<UserJudgment> {
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return None;
    }
    let judgment = serde_json::from_value::<UserJudgment>(
        response.response_value.get("user_judgment")?.clone(),
    )
    .ok()?;
    (judgment.resolution.is_none()).then_some(judgment)
}

pub(crate) fn elicitation_create_request(id: &str, judgment: &UserJudgment) -> Value {
    let option_ids = judgment
        .options
        .iter()
        .map(|option| option.option_id.as_str())
        .collect::<Vec<_>>();
    let option_names = judgment
        .options
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<_>>();
    let option_lines = judgment
        .options
        .iter()
        .map(|option| {
            format!(
                "- {} (`{}`): {}",
                option.label,
                option.option_id.as_str(),
                option.consequence
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let message = format!(
        "Volicord needs a user-owned judgment for Task `{}`.\n\nQuestion: {}\n\nContext: {}\n\nOptions:\n{}\n\nSelect exactly one option. Do not enter secrets, credentials, tokens, private keys, or other private secret material.",
        judgment.task_id.as_str(),
        judgment.question,
        judgment.context.summary,
        option_lines
    );

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": ELICITATION_CREATE_METHOD,
        "params": {
            "message": message,
            "requestedSchema": {
                "type": "object",
                "properties": {
                    "selected_option_id": {
                        "type": "string",
                        "title": "Judgment option",
                        "description": "The exact Volicord option_id selected by the user.",
                        "enum": option_ids,
                        "enumNames": option_names
                    },
                    "note": {
                        "type": "string",
                        "title": "Optional note",
                        "description": "Optional user note for this judgment. Do not include secrets, credentials, tokens, or private keys.",
                        "maxLength": 1000
                    }
                },
                "required": ["selected_option_id"]
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElicitationReply {
    Accepted {
        selected_option_id: String,
        note: Option<String>,
    },
    Declined,
    Cancelled,
    Invalid(String),
    Unavailable(String),
}

pub(crate) fn read_elicitation_response<R: BufRead>(
    request_id: &str,
    lines: &mut io::Lines<R>,
) -> ElicitationReply {
    let Some(line) = lines.next() else {
        return ElicitationReply::Unavailable(
            "stdin closed before the client responded".to_owned(),
        );
    };
    let line = match line {
        Ok(line) => line,
        Err(error) => {
            return ElicitationReply::Unavailable(format!(
                "failed to read elicitation response: {error}"
            ))
        }
    };
    let value: Value = match serde_json::from_str(&line) {
        Ok(value) => value,
        Err(error) => {
            return ElicitationReply::Invalid(format!("response was not valid JSON: {error}"))
        }
    };
    let Some(object) = value.as_object() else {
        return ElicitationReply::Invalid("response must be a JSON-RPC object".to_owned());
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return ElicitationReply::Invalid("response jsonrpc must be exactly \"2.0\"".to_owned());
    }
    if object.get("id").and_then(Value::as_str) != Some(request_id) {
        return ElicitationReply::Invalid(
            "response id did not match the elicitation request".to_owned(),
        );
    }
    if let Some(error) = object.get("error") {
        return ElicitationReply::Unavailable(format!(
            "client returned JSON-RPC error: {}",
            concise_json(error)
        ));
    }
    let Some(result) = object.get("result").and_then(Value::as_object) else {
        return ElicitationReply::Invalid("response result must be an object".to_owned());
    };
    match result.get("action").and_then(Value::as_str) {
        Some("accept") => {
            let Some(content) = result.get("content").and_then(Value::as_object) else {
                return ElicitationReply::Invalid(
                    "accepted elicitation must include object content".to_owned(),
                );
            };
            let Some(selected_option_id) =
                content.get("selected_option_id").and_then(Value::as_str)
            else {
                return ElicitationReply::Invalid(
                    "accepted elicitation content.selected_option_id must be a string".to_owned(),
                );
            };
            if selected_option_id.trim().is_empty() {
                return ElicitationReply::Invalid(
                    "accepted elicitation selected_option_id must not be empty".to_owned(),
                );
            }
            let note = match content.get("note") {
                None | Some(Value::Null) => None,
                Some(Value::String(note)) if note.len() <= 1000 => Some(note.clone()),
                Some(Value::String(_)) => {
                    return ElicitationReply::Invalid(
                        "accepted elicitation note must be at most 1000 characters".to_owned(),
                    )
                }
                Some(_) => {
                    return ElicitationReply::Invalid(
                        "accepted elicitation note must be a string when supplied".to_owned(),
                    )
                }
            };
            ElicitationReply::Accepted {
                selected_option_id: selected_option_id.to_owned(),
                note,
            }
        }
        Some("decline") => ElicitationReply::Declined,
        Some("cancel") => ElicitationReply::Cancelled,
        Some(other) => {
            ElicitationReply::Invalid(format!("unsupported elicitation action `{other}`"))
        }
        None => ElicitationReply::Invalid("response result.action must be a string".to_owned()),
    }
}

pub(crate) enum ElicitedRecordOutcome {
    Recorded(Box<PipelineResponse>),
    PostEffectFailure,
    InvalidSelection(String),
}

pub(crate) fn record_elicited_judgment(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
    selected_option_id: &str,
    note: Option<String>,
) -> Result<ElicitedRecordOutcome, McpAdapterError> {
    let Some(selected_option) = judgment
        .options
        .iter()
        .find(|option| option.option_id.as_str() == selected_option_id)
    else {
        return Ok(ElicitedRecordOutcome::InvalidSelection(format!(
            "Host prompt input selected unknown option_id `{selected_option_id}` for pending judgment `{}`.",
            judgment.judgment_id.as_str()
        )));
    };
    let state_version = judgment.basis.created_at_state_version + 1;
    let request = RecordUserJudgmentRequest {
        envelope: ToolEnvelope {
            project_id: judgment.project_id.clone(),
            task_id: Some(judgment.task_id.clone()).into(),
            request_id: RequestId::new(generated_metadata_id(
                "req_mcp_elicitation_record",
                adapter.context.connection_internal_id.as_str(),
                "volicord.record_user_judgment",
            )),
            idempotency_key: Some(IdempotencyKey::new(generated_metadata_id(
                "idem_mcp_elicitation_record",
                adapter.context.connection_internal_id.as_str(),
                "volicord.record_user_judgment",
            )))
            .into(),
            expected_state_version: Some(state_version).into(),
            dry_run: false,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        },
        user_judgment_id: judgment.judgment_id.clone(),
        judgment_kind: judgment.judgment_kind,
        selected_option_id: selected_option.option_id.clone(),
        answer: answer_payload_for_judgment(judgment, selected_option)?,
        rationale: rationale_for_selected_option(
            judgment.judgment_kind,
            selected_option,
            "host prompt input",
        ),
        note: note.into(),
        accepted_risks: accepted_risks_for_judgment(judgment, selected_option),
    };
    let invocation = InvocationContext::new(
        judgment.project_id.clone(),
        ActorSource::LocalUser,
        OperationCategory::UserOnly,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
    );
    let response = adapter
        .core
        .record_user_judgment(request, invocation)
        .map_err(McpAdapterError::Core)?;
    if response.response_value["base"]["response_kind"].as_str() == Some("result") {
        Ok(ElicitedRecordOutcome::Recorded(Box::new(response)))
    } else {
        Ok(ElicitedRecordOutcome::PostEffectFailure)
    }
}

pub(crate) fn answer_payload_for_judgment(
    judgment: &UserJudgment,
    selected_option: &UserJudgmentOption,
) -> Result<RecordUserJudgmentPayload, McpAdapterError> {
    let mut payload = empty_answer_payload();
    let branch = json_object(json!({
        "summary": format!("User selected option {}", selected_option.option_id.as_str()),
        "selected_option": selected_option.option_id.as_str(),
        "selected_option_label": selected_option.label,
        "selected_option_consequence": selected_option.consequence,
    }));
    match judgment.judgment_kind {
        JudgmentKind::ProductDecision => payload.product_decision = Some(branch).into(),
        JudgmentKind::TechnicalDecision => payload.technical_decision = Some(branch).into(),
        JudgmentKind::ScopeDecision => payload.scope_decision = Some(branch).into(),
        JudgmentKind::SensitiveApproval => {
            let Some(scope) = judgment.basis.sensitive_action_scope.as_ref() else {
                return Err(McpAdapterError::ToolExecution {
                    tool_name: "volicord.request_user_judgment".to_owned(),
                    message: "pending sensitive approval is missing its Core-derived sensitive action scope".to_owned(),
                });
            };
            payload.sensitive_action_scope = Some(scope.clone()).into();
        }
        JudgmentKind::FinalAcceptance => payload.final_acceptance = Some(branch).into(),
        JudgmentKind::ResidualRiskAcceptance => {
            payload.residual_risk_acceptance = Some(json_object(json!({
                "summary": format!("User selected option {}", selected_option.option_id.as_str()),
                "selected_option": selected_option.option_id.as_str(),
                "risk_ids": accepted_risk_ids(selected_option, judgment),
            })))
            .into();
        }
        JudgmentKind::Cancellation => payload.cancellation = Some(branch).into(),
    }
    Ok(payload)
}

pub(crate) fn empty_answer_payload() -> RecordUserJudgmentPayload {
    RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    }
}

pub(crate) fn rationale_for_selected_option(
    judgment_kind: JudgmentKind,
    selected_option: &UserJudgmentOption,
    capture_path: &str,
) -> JudgmentRationale {
    let accepted = selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted;
    JudgmentRationale {
        summary: format!(
            "User selected `{}` for `{}` through {capture_path}.",
            selected_option.option_id.as_str(),
            judgment_kind_value(judgment_kind)
        ),
        selected_reason: Some(format!(
            "{} {}",
            selected_option.description, selected_option.consequence
        ))
        .into(),
        considered_alternatives: Vec::new(),
        rejected_alternatives: Vec::new(),
        assumptions: vec!["The answer covers only the addressed Core UserJudgment.".to_owned()],
        tradeoffs: if accepted {
            vec![selected_option.consequence.clone()]
        } else {
            Vec::new()
        },
        uncertainties: Vec::new(),
        review_triggers: if accepted {
            vec!["Revisit if the captured judgment basis becomes stale or superseded.".to_owned()]
        } else {
            Vec::new()
        },
        related_refs: Vec::new(),
        artifact_refs: Vec::new(),
    }
}

pub(crate) fn accepted_risks_for_judgment(
    judgment: &UserJudgment,
    selected_option: &UserJudgmentOption,
) -> Vec<volicord_types::AcceptedRiskInput> {
    if judgment.judgment_kind == JudgmentKind::ResidualRiskAcceptance
        && selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted
    {
        judgment.context.visible_risks.clone()
    } else {
        Vec::new()
    }
}

pub(crate) fn accepted_risk_ids(
    selected_option: &UserJudgmentOption,
    judgment: &UserJudgment,
) -> Vec<String> {
    if selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted {
        judgment
            .context
            .visible_risks
            .iter()
            .map(|risk| risk.risk_id.as_str().to_owned())
            .collect()
    } else {
        Vec::new()
    }
}

pub(crate) fn reject_option_id(judgment: &UserJudgment) -> Option<&str> {
    judgment
        .options
        .iter()
        .find(|option| option.machine_action == UserJudgmentOptionAction::Reject)
        .map(|option| option.option_id.as_str())
}

pub(crate) struct UserJudgmentFallback {
    texts: Vec<String>,
    preferred_capture_path: Option<Value>,
    fallbacks: Vec<Value>,
    kind: DiagnosticFallbackKind,
}

pub(crate) fn user_judgment_fallback(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
) -> Result<UserJudgmentFallback, McpAdapterError> {
    let availability = guard_health_record(
        &adapter.runtime_home,
        judgment.project_id.as_str(),
        adapter.context.connection_internal_id.as_str(),
    )
    .and_then(|record| prompt_capture_availability(&record))
    .map_err(McpAdapterError::Store)?;
    if availability.can_use_chat_commands() {
        return chat_capture_fallback(adapter, judgment, availability.status.as_str());
    }

    if adapter.local_web_consent.is_some() {
        match local_web_consent_fallback(adapter, judgment) {
            Ok(fallback) => return Ok(fallback),
            Err(_) => {
                return Ok(cli_recovery_fallback(
                    adapter,
                    judgment,
                    availability.status.as_str(),
                    "LOCAL_WEB_CONSENT_TOKEN_UNAVAILABLE",
                ))
            }
        }
    }

    Ok(cli_recovery_fallback(
        adapter,
        judgment,
        availability.status.as_str(),
        "LOCAL_WEB_CONSENT_DISABLED",
    ))
}

pub(crate) fn response_json_with_inbox_capture(
    response: &PipelineResponse,
    fallback: &UserJudgmentFallback,
) -> Result<String, McpAdapterError> {
    let mut value = response.response_value.clone();
    if let Some(inbox_item) = value.get_mut("inbox_item").and_then(Value::as_object_mut) {
        if let Some(preferred_capture_path) = fallback.preferred_capture_path.clone() {
            inbox_item.insert("preferred_capture_path".to_owned(), preferred_capture_path);
        }
        if !fallback.fallbacks.is_empty() {
            inbox_item.insert(
                "fallbacks".to_owned(),
                Value::Array(fallback.fallbacks.clone()),
            );
        }
    }
    serde_json::to_string(&value).map_err(McpAdapterError::Json)
}

pub(crate) fn chat_capture_fallback(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
    prompt_capture_status: &str,
) -> Result<UserJudgmentFallback, McpAdapterError> {
    let store = CoreProjectStore::open(&adapter.runtime_home, &judgment.project_id)
        .map_err(McpAdapterError::Store)?;
    let records = store
        .user_judgment_records_for_task(&judgment.task_id)
        .map_err(McpAdapterError::Store)?;
    let chat_index = records
        .iter()
        .position(|record| record.judgment_id == judgment.judgment_id.as_str())
        .map(|index| index + 1)
        .unwrap_or(1);
    let requested_at = records
        .iter()
        .find(|record| record.judgment_id == judgment.judgment_id.as_str())
        .map(|record| record.requested_at.clone())
        .unwrap_or_else(|| judgment.created_at.to_canonical_string());
    let chat_id = format!("J-{chat_index}");
    let verification_code = chat_judgment_verification_code(
        judgment.project_id.as_str(),
        judgment.task_id.as_str(),
        judgment.judgment_id.as_str(),
        &requested_at,
        adapter.context.connection_internal_id.as_str(),
    );
    let commands = judgment
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            format!(
                "`Volicord: answer {chat_id} {} {verification_code}` for option `{}` ({})",
                chat_option_selector(index + 1, option),
                option.option_id.as_str(),
                option.label
            )
        })
        .collect::<Vec<_>>();
    let options = commands.join("; ");
    let note_command = format!("Volicord: note {chat_id} \"text\" {verification_code}");
    let human_text = format!(
        "Host prompt input is unavailable. The pending judgment `{}` remains unresolved. To use chat command capture, ask the user to send one exact command in chat: {options}. To defer with a note, use `Volicord: note {chat_id} \"text\" {verification_code}`. Do not ask the user to include secrets, credentials, tokens, or private keys.",
        judgment.judgment_id.as_str()
    );
    let structured_text = fallback_state_json(json!({
        "kind": "prompt_capture",
        "project_id": judgment.project_id.as_str(),
        "connection_id": adapter.context.connection_internal_id.as_str(),
        "judgment_id": judgment.judgment_id.as_str(),
        "prompt_capture_status": prompt_capture_status,
        "commands": commands,
        "note_command": note_command
    }));
    Ok(UserJudgmentFallback {
        texts: vec![human_text, structured_text],
        preferred_capture_path: Some(prompt_capture_path_json()),
        fallbacks: vec![cli_inbox_capture_path_json(judgment)],
        kind: DiagnosticFallbackKind::PromptCapture,
    })
}

pub(crate) fn local_web_consent_fallback(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
) -> Result<UserJudgmentFallback, McpAdapterError> {
    let Some(context) = adapter.local_web_consent.as_ref() else {
        return Err(McpAdapterError::Environment(
            "local consent URL is not available".to_owned(),
        ));
    };
    let token = generate_bearer_token()?;
    let record = create_local_web_consent_token(
        &adapter.runtime_home,
        LocalWebConsentTokenCreate {
            token: token.clone(),
            project_id: judgment.project_id.as_str().to_owned(),
            connection_internal_id: adapter.context.connection_internal_id.to_string(),
            judgment_id: judgment.judgment_id.as_str().to_owned(),
            capture_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
            ttl_seconds: LOCAL_WEB_CONSENT_TOKEN_TTL_SECONDS,
            created_metadata_json: json!({
                "fallback_kind": "local_web_consent",
                "endpoint": LOCAL_WEB_CONSENT_PATH
            })
            .to_string(),
        },
    )
    .map_err(McpAdapterError::Store)?;
    let url = format!(
        "{}{}?project={}&token={}",
        context.base_url,
        LOCAL_WEB_CONSENT_PATH,
        percent_encode_query(judgment.project_id.as_str()),
        percent_encode_query(&token)
    );
    let human_text = format!(
        "Host prompt input and chat command capture are unavailable. The pending judgment `{}` remains unresolved. Open this local Volicord consent link before {}: {}",
        judgment.judgment_id.as_str(),
        record.expires_at,
        url
    );
    let structured_text = fallback_state_json(json!({
        "kind": "local_web_consent",
        "url": url,
        "expires_at": record.expires_at,
        "project_id": record.project_id,
        "connection_id": record.connection_internal_id,
        "judgment_id": record.judgment_id,
        "capture_basis": record.capture_basis,
        "ttl_seconds": LOCAL_WEB_CONSENT_TOKEN_TTL_SECONDS,
        "endpoint": LOCAL_WEB_CONSENT_PATH
    }));
    Ok(UserJudgmentFallback {
        texts: vec![human_text, structured_text],
        preferred_capture_path: Some(local_web_consent_path_json(
            judgment,
            &record.capture_basis,
            &record.expires_at,
            &url,
        )),
        fallbacks: vec![cli_inbox_capture_path_json(judgment)],
        kind: DiagnosticFallbackKind::LocalWebConsent,
    })
}

pub(crate) fn cli_recovery_fallback(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
    prompt_capture_status: &str,
    local_web_reason: &'static str,
) -> UserJudgmentFallback {
    let human_text = format!(
        "Host prompt input is unavailable. The pending judgment `{}` remains unresolved. Chat command capture is not available for this connection (prompt_capture_status={prompt_capture_status}). Local consent URL is unavailable ({local_web_reason}). Use `volicord inbox` and `volicord inbox answer` as the CLI inbox path.",
        judgment.judgment_id.as_str()
    );
    let structured_text = fallback_state_json(json!({
        "kind": "cli_recovery",
        "project_id": judgment.project_id.as_str(),
        "connection_id": adapter.context.connection_internal_id.as_str(),
        "judgment_id": judgment.judgment_id.as_str(),
        "command": format!("volicord inbox answer {} --choice <choice>", judgment.judgment_id.as_str()),
        "prompt_capture_status": prompt_capture_status,
        "local_web_consent": {
            "available": false,
            "reason": local_web_reason
        }
    }));
    UserJudgmentFallback {
        texts: vec![human_text, structured_text],
        preferred_capture_path: Some(cli_inbox_capture_path_json(judgment)),
        fallbacks: Vec::new(),
        kind: DiagnosticFallbackKind::CliInbox,
    }
}

pub(crate) fn prompt_capture_path_json() -> Value {
    json!({
        "kind": "prompt_capture",
        "label": "Chat command capture",
        "available": true,
        "command": null,
        "url": null,
        "capture_basis": VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
        "expires_at": null,
        "detail": "Use the displayed chat command with the current verification code."
    })
}

pub(crate) fn local_web_consent_path_json(
    judgment: &UserJudgment,
    capture_basis: &str,
    expires_at: &str,
    url: &str,
) -> Value {
    json!({
        "kind": "local_web_consent",
        "label": "Local consent URL",
        "available": true,
        "command": null,
        "url": url,
        "capture_basis": capture_basis,
        "expires_at": expires_at,
        "detail": format!(
            "Open the local consent URL to answer pending judgment {}.",
            judgment.judgment_id.as_str()
        )
    })
}

pub(crate) fn cli_inbox_capture_path_json(judgment: &UserJudgment) -> Value {
    json!({
        "kind": "cli",
        "label": "CLI inbox",
        "available": true,
        "command": format!("volicord inbox answer {} --choice <choice>", judgment.judgment_id.as_str()),
        "url": null,
        "capture_basis": VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        "expires_at": null,
        "detail": "Answer from the local terminal as the user."
    })
}

pub(crate) fn fallback_state_json(state: Value) -> String {
    json!({ "volicord_fallback": state }).to_string()
}

pub(crate) fn percent_encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

pub(crate) fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + (value - 10)) as char,
        _ => unreachable!("hex digit input is masked to four bits"),
    }
}

pub(crate) fn chat_option_selector(index: usize, option: &UserJudgmentOption) -> String {
    match option.machine_action {
        UserJudgmentOptionAction::Reject => "reject".to_owned(),
        UserJudgmentOptionAction::Defer => "defer".to_owned(),
        UserJudgmentOptionAction::Accept => index.to_string(),
    }
}

pub(crate) fn elicitation_secret_request_risk(judgment: &UserJudgment) -> Option<&'static str> {
    let mut text = String::new();
    text.push_str(&judgment.question);
    text.push('\n');
    text.push_str(&judgment.context.summary);
    for constraint in &judgment.context.constraints {
        text.push('\n');
        text.push_str(constraint);
    }
    for option in &judgment.options {
        text.push('\n');
        text.push_str(&option.label);
        text.push('\n');
        text.push_str(&option.description);
        text.push('\n');
        text.push_str(&option.consequence);
    }
    let normalized = text.to_ascii_lowercase();
    [
        "password",
        "passphrase",
        "private key",
        "api key",
        "secret",
        "credential",
        "token",
    ]
    .into_iter()
    .find(|needle| normalized.contains(needle))
}

pub(crate) fn judgment_kind_value(value: JudgmentKind) -> &'static str {
    match value {
        JudgmentKind::ProductDecision => "product_decision",
        JudgmentKind::TechnicalDecision => "technical_decision",
        JudgmentKind::ScopeDecision => "scope_decision",
        JudgmentKind::SensitiveApproval => "sensitive_approval",
        JudgmentKind::FinalAcceptance => "final_acceptance",
        JudgmentKind::ResidualRiskAcceptance => "residual_risk_acceptance",
        JudgmentKind::Cancellation => "cancellation",
    }
}

pub(crate) fn next_server_request_id(prefix: &str, next_server_request_id: &mut u64) -> String {
    let sequence = *next_server_request_id;
    *next_server_request_id = next_server_request_id.saturating_add(1);
    format!("{prefix}_{sequence}")
}

pub(crate) fn concise_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "unserializable JSON value".to_owned())
}

pub(crate) fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => JsonObject::new(),
    }
}

pub(crate) fn is_known_mcp_tool(tool_name: &str) -> bool {
    PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) || ADAPTER_UTILITY_TOOL_NAMES.contains(&tool_name)
}

pub(crate) fn tool_execution_error_result(
    requested_tool_name: &str,
    error: &McpAdapterError,
) -> Value {
    let structured = match error {
        McpAdapterError::InvalidParams {
            issues, truncated, ..
        } => McpToolErrorResponse {
            code: McpToolErrorCode::InvalidArguments,
            tool_name: requested_tool_name.to_owned(),
            retryable: true,
            reached_core: false,
            committed: false,
            reported_issue_count: issues.len(),
            truncated: *truncated,
            issues: issues.clone(),
        },
        McpAdapterError::ToolExecution { tool_name, message } => {
            let (path, message) = if tool_name == "project routing" {
                (
                    "/project_selector".to_owned(),
                    format!(
                        "{message}. Use volicord.list_projects when project selection is unclear."
                    ),
                )
            } else {
                (
                    String::new(),
                    format!("{tool_name} failed before reaching Core: {message}"),
                )
            };
            McpToolErrorResponse {
                code: McpToolErrorCode::AdapterPreconditionFailed,
                tool_name: requested_tool_name.to_owned(),
                retryable: false,
                reached_core: false,
                committed: false,
                reported_issue_count: 1,
                truncated: false,
                issues: vec![McpToolErrorIssue {
                    path,
                    code: McpToolIssueCode::AdapterPreconditionFailed,
                    message,
                }],
            }
        }
        _ => McpToolErrorResponse {
            code: McpToolErrorCode::AdapterPreconditionFailed,
            tool_name: requested_tool_name.to_owned(),
            retryable: false,
            reached_core: false,
            committed: false,
            reported_issue_count: 1,
            truncated: false,
            issues: vec![McpToolErrorIssue {
                path: String::new(),
                code: McpToolIssueCode::AdapterPreconditionFailed,
                message: "Tool execution failed before reaching Core.".to_owned(),
            }],
        },
    };
    bounded_tool_error_result(structured)
}

fn bounded_tool_error_result(mut structured: McpToolErrorResponse) -> Value {
    let mut truncated = structured.truncated;
    if structured.issues.len() > MAX_VALIDATION_ISSUES {
        structured.issues.truncate(MAX_VALIDATION_ISSUES);
        truncated = true;
    }
    structured.issues = structured
        .issues
        .into_iter()
        .map(|issue| {
            let (issue, issue_truncated) = bound_mcp_tool_error_issue(issue);
            truncated |= issue_truncated;
            issue
        })
        .collect();
    if structured.issues.is_empty() {
        structured.issues.push(McpToolErrorIssue {
            path: String::new(),
            code: McpToolIssueCode::AdapterPreconditionFailed,
            message: "Tool execution failed before reaching Core.".to_owned(),
        });
        truncated = true;
    }

    loop {
        structured.reported_issue_count = structured.issues.len();
        structured.truncated = truncated;
        let result = serialize_tool_error_result(&structured);
        let result_bytes = serde_json::to_vec(&result)
            .expect("MCP tool error result should serialize")
            .len();
        if result_bytes <= MAX_MCP_TOOL_ERROR_RESULT_BYTES {
            return result;
        }
        if structured.issues.len() > 1 {
            structured.issues.pop();
            truncated = true;
            continue;
        }

        // Individual field limits and known tool names make this fallback
        // unreachable in normal operation, but keep the byte contract closed
        // if surrounding JSON overhead changes later.
        structured.issues[0].path.clear();
        structured.issues[0].message = "Validation failed before reaching Core.".to_owned();
        structured.truncated = true;
        let fallback = serialize_tool_error_result(&structured);
        assert!(
            serde_json::to_vec(&fallback)
                .expect("fallback MCP tool error result should serialize")
                .len()
                <= MAX_MCP_TOOL_ERROR_RESULT_BYTES,
            "known-tool MCP error fallback exceeded its response byte limit"
        );
        return fallback;
    }
}

fn serialize_tool_error_result(structured: &McpToolErrorResponse) -> Value {
    let structured_content =
        serde_json::to_value(structured).expect("MCP tool error should serialize");
    let text = serde_json::to_string(&structured_content)
        .expect("MCP tool error compatibility text should serialize");

    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "structuredContent": structured_content,
        "isError": true
    })
}

pub(crate) fn json_rpc_error_for_adapter(id: Value, error: McpAdapterError) -> Value {
    let (code, message) = match error {
        McpAdapterError::UnknownTool(_) | McpAdapterError::InvalidParams { .. } => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Protocol(_)
        | McpAdapterError::Environment(_)
        | McpAdapterError::ToolExecution { .. } => (-32602, "Invalid params"),
        McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

pub(crate) fn invalid_request(id: Value, data: impl Into<String>) -> JsonRpcFailure {
    JsonRpcFailure {
        id,
        code: -32600,
        message: "Invalid Request",
        data: Some(data.into()),
    }
}

pub(crate) fn invalid_request_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32600, "Invalid Request", Some(data.into()))
}

pub(crate) fn invalid_params_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32602, "Invalid params", Some(data.into()))
}

pub(crate) fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<String>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message
    });
    if let Some(data) = data {
        error["data"] = Value::String(data);
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error
    })
}

pub(crate) fn write_json_line(
    writer: &mut impl Write,
    value: Value,
) -> Result<(), McpAdapterError> {
    serde_json::to_writer(&mut *writer, &value).map_err(McpAdapterError::Json)?;
    writer.write_all(b"\n").map_err(McpAdapterError::Io)
}

#[cfg(test)]
mod mutation_output_tests {
    use super::*;
    use volicord_test_support::core_fixtures::{
        CoreFixture, UpdateScopeFixture, UserJudgmentFixture,
    };
    use volicord_types::ChangeUnitOperation;

    #[test]
    fn idempotent_mutation_replay_default_summary_returns_refreshed_authority_receipt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-mutation-replay-summary")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let request = fixture.intake_request(
            "req_mcp_mutation_replay_summary",
            "idem_mcp_mutation_replay_summary",
            false,
            Some(0),
        );
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };

        let committed = core.intake(request.clone(), workflow_invocation())?;
        assert!(!committed.replayed);
        let replayed = core.intake(request, workflow_invocation())?;
        assert!(replayed.replayed);
        let task_id = replayed
            .resolved_task_id
            .clone()
            .expect("replay preserves the resolved Task identity");

        let detail = mutation_detail_for_tool(INTAKE_TOOL_NAME, &json!({}));
        assert_eq!(detail, Some(MutationDetailLevel::Summary));
        let output = ToolCallOutput::from_pipeline_response(&replayed)?;
        let output =
            finalize_mutation_output_with_refresh(INTAKE_TOOL_NAME, detail, output, |context| {
                assert_eq!(context.project_id.as_str(), fixture.project_id());
                assert_eq!(context.task_id, task_id);
                core.status(
                    fixture.status_request(
                        "req_mcp_mutation_replay_summary_refresh",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            })?;

        assert!(!output.is_error);
        assert!(output.diagnostic_facts.replayed);
        assert!(output.diagnostic_facts.core_reached);
        assert!(!output.diagnostic_facts.core_committed);
        assert_eq!(
            output.structured_content["authority_receipt"]["project_id"],
            fixture.project_id()
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert!(output.structured_content["authority_receipt"]["state_version"].is_u64());
        assert_eq!(
            output.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert!(output.structured_content.get("code").is_none());
        assert!(output
            .structured_content
            .get("completion_claim_withheld")
            .is_none());
        Ok(())
    }

    #[test]
    fn full_projection_pairs_exact_method_result_with_newer_fresh_receipt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-full-fresh-receipt")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_full_fresh_receipt",
                "idem_mcp_full_fresh_receipt",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .as_ref()
            .expect("intake should resolve a Task")
            .clone();
        let original_method_result = intake.response_value.clone();
        core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_full_fresh_receipt_scope",
                idempotency_key: "idem_mcp_full_fresh_receipt_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance authority after the original method result.",
            }),
            workflow_invocation(),
        )?;

        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&intake)?,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_full_fresh_receipt_status",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["method_result"],
            original_method_result
        );
        assert_eq!(
            output.structured_content["method_result"]["base"]["state_version"],
            1
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            2
        );
        Ok(())
    }

    #[test]
    fn refresh_failure_withholds_success_and_does_not_return_private_error_body() {
        let private_error = "private-refresh-owner-body-must-not-escape";
        let mut output = ToolCallOutput::success(
            json!({
                "base": {
                    "response_kind": "result",
                    "effect_kind": "core_committed"
                }
            })
            .to_string(),
        )
        .expect("tool output");
        output.diagnostic_facts.core_reached = true;
        output.diagnostic_facts.core_committed = true;
        output.diagnostic_facts.effect_kind = Some(EffectKind::CoreCommitted);
        output.diagnostic_facts.effect_applied = true;
        output.diagnostic_facts.operation_token =
            Some("authority_event:event_refresh_failure".to_owned());
        output.mutation_refresh_context = Some(MutationRefreshContext {
            project_id: ProjectId::new("project_refresh_failure"),
            task_id: TaskId::new("task_refresh_failure"),
        });

        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Err(McpAdapterError::Environment(private_error.to_owned())),
        )
        .expect("fail-closed output");

        assert!(!output.is_error);
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["operation_token"],
            "authority_event:event_refresh_failure"
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        assert!(output.diagnostic_facts.authoritative_refresh_failure);
        let rendered =
            serde_json::to_string(&tool_call_result_from_output(output)).expect("rendered result");
        assert!(!rendered.contains(private_error));
        assert!(!rendered.contains("response_kind\":\"result"));
    }

    #[test]
    fn post_effect_adapter_failure_refreshes_authority_without_recommending_replay(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-adapter-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_post_effect_adapter_failure",
                "idem_mcp_post_effect_adapter_failure",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let refreshed = core.status(
            fixture.status_request(
                "req_mcp_post_effect_adapter_failure_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let output = ToolCallOutput::from_pipeline_response(&committed)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["method_result"], Value::Null);
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert_eq!(
            output.structured_content["authoritative_refresh_succeeded"],
            true
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            false
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn elicited_record_rejection_preserves_the_initial_pending_judgment_effect(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicited-record-race")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_elicited_record_race_intake",
                "idem_mcp_elicited_record_race_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let pending_response = core.request_user_judgment(
            fixture.user_judgment_request(UserJudgmentFixture {
                request_id: "req_mcp_elicited_record_race_pending",
                idempotency_key: "idem_mcp_elicited_record_race_pending",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        assert!(pending_judgment_from_response(&pending_response).is_some());
        core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_elicited_record_race_scope",
                idempotency_key: "idem_mcp_elicited_record_race_scope",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance state while host elicitation remains open.",
            }),
            workflow_invocation(),
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);

        let elicitation_response = json!({
            "jsonrpc": "2.0",
            "id": "elicit_user_judgment_1",
            "result": {
                "action": "accept",
                "content": { "selected_option_id": "accept" }
            }
        });
        let input = format!("{}\n", serde_json::to_string(&elicitation_response)?);
        let mut lines = io::BufReader::new(io::Cursor::new(input)).lines();
        let mut elicitation_request = Vec::new();
        let mut request_sequence = 1;
        let output = user_judgment_tool_output(
            &adapter,
            pending_response,
            true,
            &mut request_sequence,
            &mut lines,
            &mut elicitation_request,
        )?;
        assert_eq!(
            output.post_effect_failure,
            Some(McpPostEffectFailureCode::McpPostEffectAdapterFailed)
        );
        assert!(
            String::from_utf8(elicitation_request)?.contains("\"method\":\"elicitation/create\"")
        );
        let output = finalize_mutation_output_with_refresh(
            REQUEST_USER_JUDGMENT_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_elicited_record_race_status",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert!(output.structured_content["operation_token"]
            .as_str()
            .is_some_and(|token| token.starts_with("authority_event:")));
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            3
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn projection_failure_preserves_effect_facts_and_exact_method_result(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-projection-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_post_effect_projection_failure",
                "idem_mcp_post_effect_projection_failure",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let refreshed = core.status(
            fixture.status_request(
                "req_mcp_post_effect_projection_failure_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        output.structured_content["base"]["effect_kind"] = json!("invalid_projection_fixture");
        let exact_unprojectable_result = output.structured_content.clone();
        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_PROJECTION_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["method_result"],
            exact_unprojectable_result
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            false
        );
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn staging_refresh_failure_reports_applied_handle_as_non_retryable_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-staging-refresh-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_staging_refresh_failure",
                "idem_mcp_staging_refresh_failure",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .as_ref()
            .expect("intake should resolve a Task")
            .clone();
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake should report state version");
        let staged = core.stage_artifact(
            fixture.stage_artifact_request(
                "req_mcp_staging_refresh_failure_stage",
                None,
                false,
                Some(state_version),
                task_id.as_str(),
            ),
            workflow_invocation(),
        )?;
        let handle_id = staged.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("stage result should include a handle")
            .to_owned();
        let output = ToolCallOutput::from_pipeline_response(&staged)?;
        assert_eq!(
            output.diagnostic_facts.effect_kind,
            Some(EffectKind::StagingCreated)
        );
        assert!(output.diagnostic_facts.effect_applied);
        assert!(!output.diagnostic_facts.core_committed);
        let operation_token = format!("staged_artifact:{handle_id}");
        assert_eq!(
            output.diagnostic_facts.operation_token.as_deref(),
            Some(operation_token.as_str())
        );

        let output = finalize_mutation_output_with_refresh(
            STAGE_ARTIFACT_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| {
                Err(McpAdapterError::Environment(
                    "refresh unavailable".to_owned(),
                ))
            },
        )?;

        assert!(!output.is_error);
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["committed"], false);
        assert_eq!(output.structured_content["effect_kind"], "staging_created");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["operation_token"],
            operation_token
        );
        assert_eq!(
            output.structured_content["method_result"]["staged_artifact_handle"]["handle_id"],
            handle_id
        );
        assert!(output.structured_content["method_result"]["expires_at"].is_string());
        assert_eq!(output.structured_content["status_read_required"], true);
        Ok(())
    }

    #[test]
    fn oversized_valid_projection_preserves_effect_and_refresh_truth_within_each_budget(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-mutation-oversized-fresh-receipt")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_mutation_oversized_fresh_receipt",
                "idem_mcp_mutation_oversized_fresh_receipt",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves the Task");
        let mut refreshed = core.status(
            fixture.status_request(
                "req_mcp_mutation_oversized_fresh_receipt_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("fresh intake status should expose a close blocker");
        let omitted_marker = "oversized-valid-criterion-blocker-must-not-escape";
        blocker["message"] = Value::String(format!(
            "{omitted_marker}{}",
            "x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2)
        ));
        let oversized_blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] =
            oversized_blockers.clone();
        refreshed.response_value["close_blockers"] = oversized_blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        for detail in [
            MutationDetailLevel::Summary,
            MutationDetailLevel::Workflow,
            MutationDetailLevel::Full,
        ] {
            let output = ToolCallOutput::from_pipeline_response(&committed)?;
            let refreshed = refreshed.clone();
            let output = finalize_mutation_output_with_refresh(
                INTAKE_TOOL_NAME,
                Some(detail),
                output,
                |_| Ok(refreshed),
            )?;

            assert!(!output.is_error);
            assert_eq!(
                output.structured_content["code"],
                "MCP_RESPONSE_BUDGET_EXCEEDED"
            );
            assert_eq!(
                output.structured_content["requested_detail"],
                serde_json::to_value(detail)?
            );
            assert_eq!(output.structured_content["retryable"], false);
            assert_eq!(output.structured_content["reached_core"], true);
            assert_eq!(output.structured_content["committed"], true);
            assert_eq!(output.structured_content["effect_kind"], "core_committed");
            assert_eq!(output.structured_content["effect_applied"], true);
            assert!(output.structured_content["operation_token"]
                .as_str()
                .is_some_and(|token| token.starts_with("authority_event:")));
            assert_eq!(
                output.structured_content["authoritative_refresh_succeeded"],
                true
            );
            assert_eq!(
                output.structured_content["response_projection_omitted"],
                true
            );
            assert_eq!(output.structured_content["status_read_required"], true);
            assert_eq!(output.structured_content["completion_claim_withheld"], true);
            assert_eq!(
                output.structured_content["method_result"]["effect_kind"],
                "core_committed"
            );
            assert!(!output.diagnostic_facts.authoritative_refresh_failure);

            let rendered = serde_json::to_vec(&tool_call_result_from_output(output))?;
            assert!(rendered.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
            assert!(!String::from_utf8(rendered)?.contains(omitted_marker));
        }

        let output = ToolCallOutput::from_pipeline_response(&committed)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;
        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["authority_receipt"], Value::Null);
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn oversized_compact_method_result_is_omitted_from_fixed_budget_recoveries(
    ) -> Result<(), Box<dyn Error>> {
        let facts = ToolDiagnosticFacts {
            core_reached: true,
            core_committed: true,
            effect_kind: Some(EffectKind::CoreCommitted),
            effect_applied: true,
            operation_token: Some("authority_event:event_oversized_compact".to_owned()),
            ..ToolDiagnosticFacts::default()
        };
        let oversized_method_result = json!({
            "effect_kind": "core_committed",
            "oversized": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES * 2),
        });
        let output = mutation_response_budget_exceeded_output(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            facts.clone(),
            oversized_method_result.clone(),
        )?;

        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_BUDGET_EXCEEDED"
        );
        assert_eq!(output.structured_content["method_result"], Value::Null);
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );

        let output = authoritative_refresh_failure_output(
            INTAKE_TOOL_NAME,
            facts,
            Some(oversized_method_result),
        )?;
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["method_result"], Value::Null);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn oversized_stage_projection_preserves_the_staging_handle_in_bounded_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-stage-oversized-fresh-receipt")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_stage_oversized_intake",
                "idem_mcp_stage_oversized_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake state version");
        let staged = core.stage_artifact(
            fixture.stage_artifact_request(
                "req_mcp_stage_oversized_stage",
                None,
                false,
                Some(state_version),
                task_id.as_str(),
            ),
            workflow_invocation(),
        )?;
        let expected_handle = staged.response_value["staged_artifact_handle"].clone();
        let mut refreshed = core.status(
            fixture.status_request("req_mcp_stage_oversized_status", Some(task_id.as_str())),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("status exposes a close blocker");
        blocker["message"] = Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2));
        let blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] = blockers.clone();
        refreshed.response_value["close_blockers"] = blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        let output = finalize_mutation_output_with_refresh(
            STAGE_ARTIFACT_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            ToolCallOutput::from_pipeline_response(&staged)?,
            |_| Ok(refreshed),
        )?;

        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_BUDGET_EXCEEDED"
        );
        assert_eq!(
            output.structured_content["method_result"]["effect"]["effect_kind"],
            "staging_created"
        );
        assert_eq!(
            output.structured_content["method_result"]["staged_artifact_handle"],
            expected_handle
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }
}
