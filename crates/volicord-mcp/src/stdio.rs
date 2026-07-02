use crate::adapter::*;
use crate::errors::McpAdapterError;
use crate::local_http::generate_bearer_token;
use crate::local_web_consent::start_stdio_local_web_consent_listener;
use crate::prelude::*;
use crate::routing::*;
use crate::util::*;

pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, mut writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let mut state = ConnectionState::default();
    adapter.initialize_startup_session_watch(&state.session_id)?;
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
    run_stdio(adapter, stdin.lock(), stdout.lock())
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
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::AwaitingInitialize,
            client_supports_elicitation: false,
            next_server_request_id: 1,
            session_id: generated_metadata_id("session", "mcp", "stdio"),
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
                Ok(tools) => json!({ "tools": tools }),
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
            "tools/list" | "tools/call" => Some(invalid_request_response(
                &request.id,
                "connection is not ready",
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

pub(crate) fn initialize_result() -> Value {
    json!({
        "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION")
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
    let object = match required_object_params(id, params, "tools/call") {
        Ok(object) => object,
        Err(error) => return Ok(Err(error)),
    };
    if object.contains_key("task") {
        return Ok(Err(invalid_params_response(
            id,
            "tools/call task augmentation is not supported",
        )));
    }

    let tool_name = match object.get("name").and_then(Value::as_str) {
        Some(tool_name) => tool_name,
        None => {
            return Ok(Err(invalid_params_response(
                id,
                "tools/call params.name must be a string",
            )))
        }
    };
    if !is_known_mcp_tool(tool_name) {
        return Ok(Err(json_rpc_error(
            id.clone(),
            -32602,
            "Invalid params",
            Some(format!("unknown MCP tool: {tool_name}")),
        )));
    }

    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            return Ok(Err(invalid_params_response(
                id,
                "tools/call params.arguments must be an object",
            )))
        }
    };

    let session_id = state.session_id.clone();
    let output = if PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) {
        match adapter.call_tool_for_session(tool_name, arguments, Some(&session_id)) {
            Ok(response) if tool_name == "volicord.request_user_judgment" => {
                user_judgment_tool_output(
                    adapter,
                    response,
                    state.client_supports_elicitation,
                    &mut state.next_server_request_id,
                    lines,
                    writer,
                )?
            }
            Ok(response) => ToolCallOutput::success(response.response_json),
            Err(error @ McpAdapterError::InvalidParams { .. })
            | Err(error @ McpAdapterError::ToolExecution { .. }) => {
                return Ok(Ok(tool_execution_error_result(&error)));
            }
            Err(error) => return Ok(Err(json_rpc_error_for_adapter(id.clone(), error))),
        }
    } else {
        let response = match adapter.call_adapter_tool(tool_name, arguments, Some(&session_id)) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. })
            | Err(error @ McpAdapterError::ToolExecution { .. }) => {
                return Ok(Ok(tool_execution_error_result(&error)));
            }
            Err(error) => return Ok(Err(json_rpc_error_for_adapter(id.clone(), error))),
        };
        let text = serde_json::to_string(&response)
            .map_err(McpAdapterError::Json)
            .map_err(|error| json_rpc_error_for_adapter(id.clone(), error));
        match text {
            Ok(text) => ToolCallOutput::success(text),
            Err(error) => return Ok(Err(error)),
        }
    };

    Ok(Ok(tool_call_result_from_output(output)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCallOutput {
    primary_text: String,
    extra_texts: Vec<String>,
    is_error: bool,
}

impl ToolCallOutput {
    fn success(primary_text: String) -> Self {
        Self {
            primary_text,
            extra_texts: Vec::new(),
            is_error: false,
        }
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
        return Ok(ToolCallOutput::success(pending_response.response_json));
    };

    if !client_supports_elicitation {
        let fallback = user_judgment_fallback(adapter, &pending)?;
        return Ok(ToolCallOutput::success(response_json_with_inbox_capture(
            &pending_response,
            &fallback,
        )?)
        .with_extras(fallback.texts));
    }

    if let Some(reason) = elicitation_secret_request_risk(&pending) {
        let fallback = user_judgment_fallback(adapter, &pending)?;
        return Ok(ToolCallOutput::success(response_json_with_inbox_capture(
            &pending_response,
            &fallback,
        )?)
            .with_extra(format!(
                "Volicord did not open MCP elicitation for pending judgment `{}` because the prompt text appears to request or expose sensitive secret material ({reason}). Do not ask the user to enter secrets, credentials, tokens, or private keys through MCP elicitation.",
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
            ElicitedRecordOutcome::Recorded(recorded) => Ok(ToolCallOutput::success(
                recorded.response_json,
            )
            .with_extra(format!(
                "Volicord recorded pending judgment `{}` through MCP elicitation with User Channel basis `{}`.",
                pending.judgment_id.as_str(),
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            ))),
            ElicitedRecordOutcome::InvalidSelection(message) => Ok(ToolCallOutput::success(
                pending_response.response_json,
            )
            .with_extra(format!(
                "{message} The pending judgment remains unresolved."
            ))),
        },
        ElicitationReply::Declined => match reject_option_id(&pending) {
            Some(option_id) => match record_elicited_judgment(adapter, &pending, option_id, None)? {
                ElicitedRecordOutcome::Recorded(recorded) => Ok(ToolCallOutput::success(
                    recorded.response_json,
                )
                .with_extra(format!(
                    "Volicord recorded pending judgment `{}` as rejected through MCP elicitation with User Channel basis `{}`.",
                    pending.judgment_id.as_str(),
                    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
                ))),
                ElicitedRecordOutcome::InvalidSelection(message) => Ok(ToolCallOutput::success(
                    pending_response.response_json,
                )
                .with_extra(format!(
                    "{message} The pending judgment remains unresolved."
                ))),
            },
            None => Ok(ToolCallOutput::success(pending_response.response_json).with_extra(
                "The MCP client declined the elicitation request, but this judgment has no Core reject option to record. The pending judgment remains unresolved.",
            )),
        },
        ElicitationReply::Cancelled => Ok(ToolCallOutput::success(pending_response.response_json)
            .with_extra(format!(
                "The MCP client cancelled or dismissed elicitation for pending judgment `{}`. Volicord did not record an answer; the judgment remains pending.",
                pending.judgment_id.as_str()
            ))),
        ElicitationReply::Invalid(message) => Ok(ToolCallOutput::success(
            pending_response.response_json,
        )
        .with_extra(format!(
            "Volicord rejected the MCP elicitation response: {message}. The pending judgment remains unresolved."
        ))),
        ElicitationReply::Unavailable(message) => {
            let fallback = user_judgment_fallback(adapter, &pending)?;
            Ok(ToolCallOutput::success(response_json_with_inbox_capture(
                &pending_response,
                &fallback,
            )?)
            .with_extra(format!(
                "MCP elicitation was unavailable after the client advertised support: {message}."
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
    Recorded(PipelineResponse),
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
            "MCP elicitation selected unknown option_id `{selected_option_id}` for pending judgment `{}`.",
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
            "MCP elicitation",
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
    adapter
        .core
        .record_user_judgment(request, invocation)
        .map(ElicitedRecordOutcome::Recorded)
        .map_err(McpAdapterError::Core)
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
        "MCP elicitation is unavailable. The pending judgment `{}` remains unresolved. To use chat prompt capture, ask the user to send one exact command in chat: {options}. To defer with a note, use `Volicord: note {chat_id} \"text\" {verification_code}`. Do not ask the user to include secrets, credentials, tokens, or private keys.",
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
    })
}

pub(crate) fn local_web_consent_fallback(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
) -> Result<UserJudgmentFallback, McpAdapterError> {
    let Some(context) = adapter.local_web_consent.as_ref() else {
        return Err(McpAdapterError::Environment(
            "local web consent is not available".to_owned(),
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
        "MCP elicitation and prompt-capture chat commands are unavailable. The pending judgment `{}` remains unresolved. Open this local Volicord consent link before {}: {}",
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
    })
}

pub(crate) fn cli_recovery_fallback(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
    prompt_capture_status: &str,
    local_web_reason: &'static str,
) -> UserJudgmentFallback {
    let human_text = format!(
        "MCP elicitation is unavailable. The pending judgment `{}` remains unresolved. Prompt-capture chat commands are not available for this connection (prompt_capture_status={prompt_capture_status}). Local web consent is unavailable ({local_web_reason}). Use `volicord inbox` and `volicord inbox answer` as the local CLI recovery path.",
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
    }
}

pub(crate) fn prompt_capture_path_json() -> Value {
    json!({
        "kind": "prompt_capture",
        "label": "Prompt capture",
        "available": true,
        "command": null,
        "url": null,
        "capture_basis": VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
        "expires_at": null,
        "detail": "Use the displayed prompt-capture answer command with the current verification code."
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
        "label": "Local web consent",
        "available": true,
        "command": null,
        "url": url,
        "capture_basis": capture_basis,
        "expires_at": expires_at,
        "detail": format!(
            "Open the loopback consent link to answer pending judgment {}.",
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

pub(crate) fn tool_execution_error_result(error: &McpAdapterError) -> Value {
    let text = match error {
        McpAdapterError::InvalidParams { tool_name, source } => {
            format!("Invalid arguments for {tool_name}: {source}. Check the tool input schema and retry.")
        }
        McpAdapterError::ToolExecution { tool_name, message } if tool_name == "project routing" => {
            format!("{message}. Use volicord.list_projects when project selection is unclear.")
        }
        McpAdapterError::ToolExecution { tool_name, message } => {
            format!("{tool_name} failed before reaching Core: {message}")
        }
        _ => "Tool execution failed before reaching Core.".to_owned(),
    };

    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
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
