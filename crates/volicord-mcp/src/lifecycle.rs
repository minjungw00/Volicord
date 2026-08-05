//! Initialize/initialized lifecycle and state-valid message admission.
//!
//! The closed [`SessionState`] model keeps initialization data in only the
//! states where it is valid. Request and notification handlers return the next
//! state together with protocol output or with the failure that stopped the
//! transition.

use crate::adapter::McpAdapter;
use crate::binding::{validate_managed_stdio_session_ownership, CodexManagedBinding};
use crate::constants::{server_instructions, SERVER_NAME};
use crate::diagnostics::{
    JsonRpcDiagnostic, McpDiagnostic, McpLifecycleDiagnostic, McpProtocolDiagnostic,
    McpToolCallDiagnostic, McpToolDiscoveryDiagnostic,
};
use crate::errors::McpAdapterError;
use crate::json_rpc::{
    self, diagnostic_for_failure, invalid_params_response, invalid_request_response,
    json_rpc_error, notification_params_are_object_or_absent, parse_client_message, response_id,
    success_response, validate_optional_object_params, ClientMessage, JsonRpcNotification,
    JsonRpcRequest,
};
use crate::mutation_admission::with_mcp_runtime_home_mutation;
use crate::session_metrics::start_transport_diagnostic_session;
use crate::telemetry::{
    authoritative_observation_timestamp, record_current_session_finding,
    record_current_session_finding_with_admission,
};
use crate::tool_dispatch::{
    call_tool_result, list_tools_result, projected_tool_error_code, safe_tool_call_response_failed,
};
use serde_json::{json, Map, Value};
use volicord_mcp_protocol::{
    ClientCapabilitiesShape, InitializedNotification, JsonRpcBatching, McpProtocolProfile,
    McpProtocolRevisionError, ProtocolRegistry,
};
use volicord_store::managed_launch_leases::{
    consume_managed_mcp_launch_lease_and_start_runtime, ManagedMcpLaunchLeaseConsumption,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_store::operational_sessions::{
    record_mcp_graceful_close, record_mcp_initialize_attempt, record_mcp_initialize_completion,
    record_mcp_initialized_notification, recover_terminal_managed_runtime_repository_observations,
    start_mcp_runtime_session, McpRuntimeSessionStart,
};
use volicord_types::integration_revision::McpRuntimeSessionSource;
use volicord_types::managed_mcp_client_info::ManagedMcpClientInfo;
use volicord_types::tool_names::{AgentToolCategory, AgentToolId};
use volicord_types::values::UtcTimestamp;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionPhase {
    AwaitingInitialization,
    AwaitingInitializedNotification,
    InitializedAndReady,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitializationSelection {
    pub(crate) requested_protocol_version: String,
    pub(crate) selected_profile: &'static McpProtocolProfile,
    pub(crate) client_capabilities: Map<String, Value>,
    pub(crate) attempted_client_name: String,
    pub(crate) attempted_client_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitializedSession {
    selection: InitializationSelection,
}

impl InitializedSession {
    pub(crate) const fn selected_profile(&self) -> &'static McpProtocolProfile {
        self.selection.selected_profile
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionTermination {
    GracefulEof,
    TerminalFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedSession {
    pub(crate) runtime: SessionRuntime,
    pub(crate) termination: SessionTermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRuntime {
    pub(crate) runtime_session_id: String,
    observation_floor: Option<UtcTimestamp>,
    pub(crate) launch_origin: &'static str,
    pub(crate) status_method_call_count: u64,
    pub(crate) terminal_finding_recorded: bool,
    pub(crate) pending_finding: Option<McpDiagnostic>,
    pub(crate) codex_binding: CodexManagedBinding,
    pub(crate) deferred_tools_list_serialized_bytes: Option<u64>,
}

impl SessionRuntime {
    pub(crate) fn for_session_source(session_source: McpRuntimeSessionSource) -> Self {
        Self {
            runtime_session_id: String::new(),
            observation_floor: None,
            launch_origin: session_source.as_str(),
            status_method_call_count: 0,
            terminal_finding_recorded: false,
            pending_finding: None,
            codex_binding: CodexManagedBinding::for_session_source(session_source),
            deferred_tools_list_serialized_bytes: None,
        }
    }

    pub(crate) fn next_observation_timestamp(&mut self) -> String {
        let sampled = UtcTimestamp::parse(&authoritative_observation_timestamp())
            .expect("the adapter clock always produces a canonical UTC timestamp");
        self.advance_observation_floor(sampled)
    }

    fn advance_observation_floor(&mut self, sampled: UtcTimestamp) -> String {
        if self
            .observation_floor
            .as_ref()
            .is_none_or(|floor| sampled > *floor)
        {
            self.observation_floor = Some(sampled);
        }
        self.observation_floor
            .as_ref()
            .expect("an observation sample always establishes the runtime clock floor")
            .to_canonical_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionState {
    AwaitingInitialization(SessionRuntime),
    AwaitingInitializedNotification {
        runtime: SessionRuntime,
        selection: InitializationSelection,
    },
    InitializedAndReady {
        runtime: SessionRuntime,
        session: InitializedSession,
    },
    Closed(ClosedSession),
}

impl SessionState {
    pub(crate) fn new(session_source: McpRuntimeSessionSource) -> Self {
        Self::AwaitingInitialization(SessionRuntime::for_session_source(session_source))
    }

    #[cfg(test)]
    pub(crate) const fn phase(&self) -> SessionPhase {
        match self {
            Self::AwaitingInitialization(_) => SessionPhase::AwaitingInitialization,
            Self::AwaitingInitializedNotification { .. } => {
                SessionPhase::AwaitingInitializedNotification
            }
            Self::InitializedAndReady { .. } => SessionPhase::InitializedAndReady,
            Self::Closed(_) => SessionPhase::Closed,
        }
    }

    pub(crate) fn runtime(&self) -> &SessionRuntime {
        match self {
            Self::AwaitingInitialization(runtime)
            | Self::AwaitingInitializedNotification { runtime, .. }
            | Self::InitializedAndReady { runtime, .. } => runtime,
            Self::Closed(closed) => &closed.runtime,
        }
    }

    pub(crate) fn runtime_mut(&mut self) -> &mut SessionRuntime {
        match self {
            Self::AwaitingInitialization(runtime)
            | Self::AwaitingInitializedNotification { runtime, .. }
            | Self::InitializedAndReady { runtime, .. } => runtime,
            Self::Closed(closed) => &mut closed.runtime,
        }
    }

    pub(crate) const fn selected_profile(&self) -> Option<&'static McpProtocolProfile> {
        match self {
            Self::AwaitingInitializedNotification { selection, .. } => {
                Some(selection.selected_profile)
            }
            Self::InitializedAndReady { session, .. } => Some(session.selected_profile()),
            Self::AwaitingInitialization(_) | Self::Closed(_) => None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn initialization_selection(&self) -> Option<&InitializationSelection> {
        match self {
            Self::AwaitingInitializedNotification { selection, .. } => Some(selection),
            Self::InitializedAndReady { session, .. } => Some(&session.selection),
            Self::AwaitingInitialization(_) | Self::Closed(_) => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct MessageTransition {
    pub(crate) state: SessionState,
    pub(crate) output: Option<Value>,
}

#[derive(Debug)]
pub(crate) struct SessionTransitionFailure {
    pub(crate) state: Box<SessionState>,
    pub(crate) error: Box<McpAdapterError>,
}

impl SessionTransitionFailure {
    fn new(state: SessionState, error: McpAdapterError) -> Self {
        Self {
            state: Box::new(state),
            error: Box::new(error),
        }
    }
}

pub(crate) struct SessionStart {
    pub(crate) session_source: McpRuntimeSessionSource,
    pub(crate) managed_lease: Option<ManagedMcpLaunchLeaseConsumption>,
    pub(crate) observed_host_executable_version: Option<String>,
    pub(crate) process_started_at: String,
}

pub(crate) fn start_session(
    adapter: &McpAdapter,
    start: SessionStart,
) -> Result<SessionState, McpAdapterError> {
    let runtime_start = McpRuntimeSessionStart {
        connection_internal_id: adapter.context.connection_internal_id.as_str().to_owned(),
        session_source: start.session_source,
        observed_host_executable_version: start.observed_host_executable_version,
        process_id: std::process::id(),
        process_started_at: start.process_started_at,
    };
    let runtime_session = with_mcp_runtime_home_mutation(
        &adapter.runtime_home,
        "mcp.runtime_session.start",
        |context| {
            recover_terminal_managed_runtime_repository_observations(context)
                .map_err(McpAdapterError::Store)?;
            let runtime_session = if let Some(lease) = start.managed_lease {
                if start.session_source != McpRuntimeSessionSource::ManagedHost {
                    return Err(McpAdapterError::Environment(
                        "managed launch lease requires session_source=managed_host".to_owned(),
                    ));
                }
                consume_managed_mcp_launch_lease_and_start_runtime(context, lease, runtime_start)
            } else {
                start_mcp_runtime_session(context, runtime_start)
            };
            runtime_session.map_err(McpAdapterError::Store)
        },
    )?;
    let mut state = SessionState::new(start.session_source);
    state.runtime_mut().runtime_session_id = runtime_session.runtime_session_id;
    state.runtime_mut().observation_floor = Some(
        UtcTimestamp::parse(&runtime_session.last_observed_at).map_err(|_| {
            McpAdapterError::Protocol(
                "new MCP runtime session returned an invalid observation timestamp".to_owned(),
            )
        })?,
    );
    if let Err(error) =
        validate_managed_stdio_session_ownership(adapter, &state.runtime().codex_binding)
    {
        let diagnostic = McpDiagnostic::from(&error);
        record_current_session_finding_with_admission(
            adapter,
            state.runtime_mut(),
            diagnostic,
            None,
            None,
            None,
            Vec::new(),
            true,
        )?;
        return Err(error);
    }
    if !state.runtime().codex_binding.is_pending() {
        let _ = with_mcp_runtime_home_mutation(
            &adapter.runtime_home,
            "mcp.diagnostic_session.start",
            |context| {
                start_transport_diagnostic_session(context, adapter, state.runtime())
                    .map_err(McpAdapterError::Store)
            },
        );
    }
    Ok(state)
}

pub(crate) fn handle_json_rpc_message(
    adapter: &McpAdapter,
    mut state: SessionState,
    message: Value,
) -> Result<MessageTransition, SessionTransitionFailure> {
    let response_id = response_id(&message);
    let handled =
        with_mcp_runtime_home_mutation(&adapter.runtime_home, "mcp.lifecycle_message", |context| {
            handle_json_rpc_message_admitted(context, adapter, &mut state, message)
        });
    match handled {
        Ok(output) => Ok(MessageTransition { state, output }),
        Err(error @ McpAdapterError::MutationAdmission(_)) => Ok(MessageTransition {
            state,
            output: response_id.map(|id| json_rpc::json_rpc_error_for_adapter(id, error)),
        }),
        Err(error) => Err(SessionTransitionFailure::new(state, error)),
    }
}

fn handle_json_rpc_message_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &mut SessionState,
    message: Value,
) -> Result<Option<Value>, McpAdapterError> {
    if let Value::Array(entries) = message {
        return handle_json_rpc_batch(context, adapter, state, entries);
    }
    handle_single_json_rpc_message(context, adapter, state, message)
}

fn handle_single_json_rpc_message(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &mut SessionState,
    message: Value,
) -> Result<Option<Value>, McpAdapterError> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            handle_json_rpc_request(context, adapter, state, request).map(Some)
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(context, adapter, state, notification)?;
            Ok(None)
        }
        Err(error) => {
            let response = json_rpc_error(
                error.id.clone(),
                error.code,
                error.message,
                error.data.clone(),
            );
            record_current_session_finding(
                context,
                adapter,
                state.runtime_mut(),
                McpDiagnostic::JsonRpc(diagnostic_for_failure(&error)),
                Some(error.code),
                error.data,
                None,
                Vec::new(),
                false,
            )?;
            Ok(Some(response))
        }
    }
}

fn handle_json_rpc_batch(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &mut SessionState,
    entries: Vec<Value>,
) -> Result<Option<Value>, McpAdapterError> {
    if entries.is_empty() {
        record_current_session_finding(
            context,
            adapter,
            state.runtime_mut(),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::InvalidRequest),
            Some(-32600),
            Some("JSON-RPC batch was empty".to_owned()),
            None,
            Vec::new(),
            false,
        )?;
        return Ok(Some(invalid_request_response(
            &Value::Null,
            "JSON-RPC batch must not be empty",
        )));
    }

    if let Err(rejection) = admit_json_rpc_batch(state, &entries) {
        record_current_session_finding(
            context,
            adapter,
            state.runtime_mut(),
            rejection.diagnostic(),
            Some(-32600),
            Some(rejection.detail().to_owned()),
            None,
            Vec::new(),
            false,
        )?;
        return Ok(Some(invalid_request_response(
            &Value::Null,
            rejection.detail(),
        )));
    }

    let mut responses = Vec::new();
    for entry in entries {
        if let Some(response) = handle_single_json_rpc_message(context, adapter, state, entry)? {
            responses.push(response);
        }
    }
    Ok((!responses.is_empty()).then_some(Value::Array(responses)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonRpcBatchAdmissionError {
    InitializationMessage,
    InitializeRequired,
    OperationBeforeReady,
    SelectedProfileDisallowsBatching,
    SessionClosed,
}

impl JsonRpcBatchAdmissionError {
    const fn diagnostic(self) -> McpDiagnostic {
        match self {
            Self::InitializationMessage => {
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializationBatchForbidden)
            }
            Self::InitializeRequired => {
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializeRequired)
            }
            Self::OperationBeforeReady | Self::SessionClosed => {
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady)
            }
            Self::SelectedProfileDisallowsBatching => {
                McpDiagnostic::JsonRpc(JsonRpcDiagnostic::InvalidRequest)
            }
        }
    }

    const fn detail(self) -> &'static str {
        match self {
            Self::InitializationMessage => {
                "initialize and notifications/initialized must be standalone messages"
            }
            Self::InitializeRequired => {
                "JSON-RPC batching requires standalone initialization to complete first"
            }
            Self::OperationBeforeReady => {
                "JSON-RPC batching requires an operation-ready MCP session"
            }
            Self::SelectedProfileDisallowsBatching => {
                "JSON-RPC batching is not permitted by the selected protocol profile"
            }
            Self::SessionClosed => "JSON-RPC batching is not valid after session close",
        }
    }
}

fn admit_json_rpc_batch(
    state: &SessionState,
    entries: &[Value],
) -> Result<(), JsonRpcBatchAdmissionError> {
    if entries.iter().any(batch_entry_is_initialization_message) {
        return Err(JsonRpcBatchAdmissionError::InitializationMessage);
    }
    let profile = match state {
        SessionState::AwaitingInitialization(_) => {
            return Err(JsonRpcBatchAdmissionError::InitializeRequired);
        }
        SessionState::AwaitingInitializedNotification { .. } => {
            return Err(JsonRpcBatchAdmissionError::OperationBeforeReady);
        }
        SessionState::InitializedAndReady { session, .. } => session.selected_profile(),
        SessionState::Closed(_) => return Err(JsonRpcBatchAdmissionError::SessionClosed),
    };
    if profile.messages().json_rpc_batching() != JsonRpcBatching::Allowed {
        return Err(JsonRpcBatchAdmissionError::SelectedProfileDisallowsBatching);
    }
    Ok(())
}

fn batch_entry_is_initialization_message(entry: &Value) -> bool {
    entry
        .as_object()
        .and_then(|entry| entry.get("method"))
        .and_then(Value::as_str)
        .is_some_and(|method| matches!(method, "initialize" | "notifications/initialized"))
}

fn handle_json_rpc_notification(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &mut SessionState,
    notification: JsonRpcNotification,
) -> Result<(), McpAdapterError> {
    if notification.method != "notifications/initialized" {
        return Ok(());
    }
    let valid_params = notification_params_are_object_or_absent(notification.params.as_ref());
    match state {
        SessionState::AwaitingInitializedNotification { runtime, selection }
            if valid_params
                && matches!(
                    selection
                        .selected_profile
                        .messages()
                        .initialized_notification(),
                    InitializedNotification::AfterInitialize
                ) =>
        {
            if !runtime.runtime_session_id.is_empty() {
                let observed_at = runtime.next_observation_timestamp();
                record_mcp_initialized_notification(
                    context,
                    &runtime.runtime_session_id,
                    selection.selected_profile.revision().as_str(),
                    &observed_at,
                )
                .map_err(McpAdapterError::Store)?;
            }
            let next_runtime = runtime.clone();
            let session = InitializedSession {
                selection: selection.clone(),
            };
            *state = SessionState::InitializedAndReady {
                runtime: next_runtime,
                session,
            };
        }
        SessionState::InitializedAndReady { .. } if valid_params => {
            // Duplicate valid initialized notifications are idempotent.
        }
        _ => {
            record_current_session_finding(
                context,
                adapter,
                state.runtime_mut(),
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationInvalid),
                Some(-32600),
                Some(
                    "notifications/initialized did not match the selected lifecycle state"
                        .to_owned(),
                ),
                None,
                Vec::new(),
                false,
            )?;
        }
    }
    Ok(())
}

fn handle_json_rpc_request(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &mut SessionState,
    request: JsonRpcRequest,
) -> Result<Value, McpAdapterError> {
    let method = request.method.clone();
    let safe_tool_name = if method == "tools/call" {
        request
            .params
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            .and_then(|name| AgentToolId::from_wire_name(name).ok())
            .filter(|tool| matches!(tool.category(), AgentToolCategory::ReadOnly))
            .map(|tool| tool.wire_name().to_owned())
    } else {
        None
    };
    let response = handle_json_rpc_request_inner(context, adapter, state, request)?;
    if method == "tools/call" && state.runtime().pending_finding.is_none() {
        state.runtime_mut().pending_finding = response
            .get("result")
            .and_then(projected_tool_error_code)
            .and_then(|code| match code.as_str() {
                "MCP_INVALID_ARGUMENTS" => Some(McpDiagnostic::ToolCall(
                    McpToolCallDiagnostic::InvalidArguments,
                )),
                "MCP_RESPONSE_BUDGET_EXCEEDED" => Some(McpDiagnostic::ToolCall(
                    McpToolCallDiagnostic::ResponseBudgetFailure,
                )),
                "MCP_POST_EFFECT_ADAPTER_FAILED" => Some(McpDiagnostic::ToolCall(
                    McpToolCallDiagnostic::AdapterExecutionError,
                )),
                _ => None,
            });
    }
    let fallback_failure = if method == "initialize" && response.get("error").is_some() {
        Some(McpDiagnostic::Protocol(
            McpProtocolDiagnostic::CapabilityShapeFailure,
        ))
    } else if method == "tools/list" && response.get("error").is_some() {
        Some(McpDiagnostic::ToolDiscovery(
            McpToolDiscoveryDiagnostic::ProtocolError,
        ))
    } else {
        None
    };
    let error = response.get("error");
    let json_rpc_code = error
        .and_then(|error| error.get("code"))
        .and_then(Value::as_i64);
    let safe_error_data = error
        .and_then(|error| error.get("data"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let pending_finding = state.runtime_mut().pending_finding.take();
    let safe_tool_failed = safe_tool_name.is_some()
        && state.runtime().codex_binding.is_bound()
        && safe_tool_call_response_failed(&response);
    if safe_tool_failed {
        if let Some(failure) = pending_finding {
            record_current_session_finding(
                context,
                adapter,
                state.runtime_mut(),
                failure,
                json_rpc_code,
                safe_error_data.clone(),
                safe_tool_name.clone(),
                Vec::new(),
                false,
            )?;
        }
        record_current_session_finding(
            context,
            adapter,
            state.runtime_mut(),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::SafeReadOnlyToolFailure),
            json_rpc_code,
            safe_error_data,
            safe_tool_name,
            Vec::new(),
            true,
        )?;
    } else if let Some(failure) = pending_finding.or(fallback_failure) {
        record_current_session_finding(
            context,
            adapter,
            state.runtime_mut(),
            failure,
            json_rpc_code,
            safe_error_data,
            safe_tool_name,
            Vec::new(),
            method == "initialize" || method == "tools/list",
        )?;
    }
    Ok(response)
}

fn handle_json_rpc_request_inner(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &mut SessionState,
    request: JsonRpcRequest,
) -> Result<Value, McpAdapterError> {
    if let Some((error, diagnostic)) = lifecycle_error_with_diagnostic(state, &request) {
        state.runtime_mut().pending_finding = Some(diagnostic);
        return Ok(error);
    }

    let response_id = request.id.clone();
    if request.method == "initialize" {
        return match handle_initialize(context, adapter, state, &response_id, request.params)? {
            Ok(result) => Ok(success_response(response_id, result)),
            Err(error) => Ok(error),
        };
    }

    let selected_profile = state
        .selected_profile()
        .expect("lifecycle admission requires a selected profile");
    let result = match request.method.as_str() {
        "ping" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "ping")
            {
                return Ok(error);
            }
            json!({})
        }
        "tools/list" => match list_tools_result(
            context,
            adapter,
            &response_id,
            request.params,
            state.runtime_mut(),
            selected_profile.capabilities(),
        )? {
            Ok(result) => result,
            Err(error) => return Ok(error),
        },
        "tools/call" => match call_tool_result(
            context,
            adapter,
            &response_id,
            request.params,
            state.runtime_mut(),
            selected_profile.capabilities(),
        )? {
            Ok(result) => result,
            Err(error) => return Ok(error),
        },
        _ => {
            state.runtime_mut().pending_finding =
                Some(McpDiagnostic::JsonRpc(JsonRpcDiagnostic::UnknownMethod));
            return Ok(json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(request.method),
            ));
        }
    };
    Ok(success_response(response_id, result))
}

fn lifecycle_error_with_diagnostic(
    state: &SessionState,
    request: &JsonRpcRequest,
) -> Option<(Value, McpDiagnostic)> {
    match state {
        SessionState::AwaitingInitialization(_) if request.method != "initialize" => Some((
            invalid_request_response(&request.id, "initialize must be the first request"),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializeRequired),
        )),
        SessionState::AwaitingInitialization(_) => None,
        SessionState::AwaitingInitializedNotification { .. } => match request.method.as_str() {
            "initialize" => Some((
                invalid_request_response(&request.id, "initialize has already completed"),
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize),
            )),
            "tools/call" => Some((
                invalid_request_response(
                    &request.id,
                    "tools/call requires notifications/initialized",
                ),
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady),
            )),
            _ => None,
        },
        SessionState::InitializedAndReady { .. } if request.method == "initialize" => Some((
            invalid_request_response(&request.id, "initialize has already completed"),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize),
        )),
        SessionState::InitializedAndReady { .. } => None,
        SessionState::Closed(_) => Some((
            invalid_request_response(&request.id, "MCP session is closed"),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InvalidShutdownSequence),
        )),
    }
}

fn handle_initialize(
    context: &RuntimeHomeMutationContext<'_>,
    _adapter: &McpAdapter,
    state: &mut SessionState,
    id: &Value,
    params: Option<Value>,
) -> Result<Result<Value, Value>, McpAdapterError> {
    if let Some((client_info, requested_protocol_version)) =
        parsed_initialize_attempt(params.as_ref())
    {
        if !state.runtime().runtime_session_id.is_empty() {
            let observed_at = state.runtime_mut().next_observation_timestamp();
            record_mcp_initialize_attempt(
                context,
                &state.runtime().runtime_session_id,
                &client_info,
                &requested_protocol_version,
                &observed_at,
            )
            .map_err(McpAdapterError::Store)?;
        }
    }
    let selection = match validate_initialize_params(id, params) {
        Ok(selection) => selection,
        Err((error, diagnostic)) => {
            state.runtime_mut().pending_finding = Some(diagnostic);
            return Ok(Err(error));
        }
    };
    if !state.runtime().runtime_session_id.is_empty() {
        let observed_at = state.runtime_mut().next_observation_timestamp();
        record_mcp_initialize_completion(
            context,
            &state.runtime().runtime_session_id,
            selection.selected_profile.revision().as_str(),
            &observed_at,
        )
        .map_err(McpAdapterError::Store)?;
    }
    let result = initialize_result(&selection);
    let runtime = state.runtime().clone();
    *state = match selection
        .selected_profile
        .messages()
        .initialized_notification()
    {
        InitializedNotification::AfterInitialize => {
            SessionState::AwaitingInitializedNotification { runtime, selection }
        }
        InitializedNotification::Absent => SessionState::InitializedAndReady {
            runtime,
            session: InitializedSession { selection },
        },
    };
    Ok(Ok(result))
}

pub(crate) fn initialize_result(selection: &InitializationSelection) -> Value {
    let build = crate::build_info();
    let initialize = selection.selected_profile.capabilities().initialize();
    let capabilities = if initialize.tools_capability() {
        json!({ "tools": {} })
    } else {
        json!({})
    };
    let mut result = Map::new();
    if initialize.metadata() {
        result.insert(
            "_meta".to_owned(),
            json!({
                "io.volicord/build": build
            }),
        );
    }
    if initialize.protocol_version() {
        result.insert(
            "protocolVersion".to_owned(),
            Value::String(selection.selected_profile.revision().as_str().to_owned()),
        );
    }
    if initialize.capabilities() {
        result.insert("capabilities".to_owned(), capabilities);
    }
    if initialize.server_info() {
        result.insert(
            "serverInfo".to_owned(),
            json!({
                "name": SERVER_NAME,
                "version": build.package_version
            }),
        );
    }
    if initialize.instructions() {
        result.insert(
            "instructions".to_owned(),
            Value::String(server_instructions()),
        );
    }
    Value::Object(result)
}

fn validate_initialize_params(
    id: &Value,
    params: Option<Value>,
) -> Result<InitializationSelection, (Value, McpDiagnostic)> {
    let object =
        json_rpc::required_object_params(id, params, "initialize").map_err(|response| {
            (
                response,
                McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
            )
        })?;
    let Some(Value::String(requested_protocol_version)) = object.get("protocolVersion") else {
        return Err((
            invalid_params_response(id, "initialize params.protocolVersion must be a string"),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::MalformedVersion),
        ));
    };
    let Some(Value::Object(client_capabilities)) = object.get("capabilities") else {
        return Err((
            invalid_params_response(id, "initialize params.capabilities must be an object"),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
        ));
    };
    let Some(Value::Object(client_info)) = object.get("clientInfo") else {
        return Err((
            invalid_params_response(id, "initialize params.clientInfo must be an object"),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
        ));
    };
    let Some(Value::String(client_name)) = client_info.get("name") else {
        return Err((
            invalid_params_response(id, "initialize params.clientInfo.name must be a string"),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
        ));
    };
    let Some(Value::String(client_version)) = client_info.get("version") else {
        return Err((
            invalid_params_response(id, "initialize params.clientInfo.version must be a string"),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
        ));
    };
    let client_info = ManagedMcpClientInfo::new(client_name.clone(), client_version.clone())
        .map_err(|error| {
            (
                invalid_params_response(id, error.to_string()),
                McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
            )
        })?;
    let (attempted_client_name, attempted_client_version) = client_info.into_parts();
    let selected_profile = ProtocolRegistry::production()
        .select_initialize(requested_protocol_version)
        .map_err(|error| {
            let safe_detail = match error {
                McpProtocolRevisionError::Unknown => {
                    "protocolVersion is not a supported MCP revision".to_owned()
                }
                McpProtocolRevisionError::NotProductionSupported(revision) => {
                    format!("protocolVersion {revision} is not production-supported")
                }
            };
            (
                invalid_params_response(id, safe_detail),
                McpDiagnostic::Protocol(McpProtocolDiagnostic::UnsupportedVersion),
            )
        })?;
    let client_capabilities = match selected_profile.capabilities().client().shape() {
        ClientCapabilitiesShape::OpenObject => client_capabilities,
    };

    Ok(InitializationSelection {
        requested_protocol_version: requested_protocol_version.clone(),
        selected_profile,
        client_capabilities: client_capabilities.clone(),
        attempted_client_name,
        attempted_client_version,
    })
}

fn parsed_initialize_attempt(params: Option<&Value>) -> Option<(ManagedMcpClientInfo, String)> {
    let object = params?.as_object()?;
    let requested_protocol_version = object.get("protocolVersion")?.as_str()?.to_owned();
    let client_info = object.get("clientInfo")?.as_object()?;
    let client_name = client_info.get("name")?.as_str()?;
    let client_version = client_info.get("version")?.as_str()?;
    ManagedMcpClientInfo::new(client_name, client_version)
        .ok()
        .map(|client_info| (client_info, requested_protocol_version))
}

pub(crate) fn close_session(
    adapter: &McpAdapter,
    state: SessionState,
) -> Result<SessionState, SessionTransitionFailure> {
    if matches!(state, SessionState::Closed(_)) {
        return Ok(state);
    }
    let mut runtime = state.runtime().clone();
    if !runtime.terminal_finding_recorded {
        let incomplete = match &state {
            SessionState::InitializedAndReady { .. } => None,
            SessionState::AwaitingInitialization(_) => Some(McpDiagnostic::Lifecycle(
                McpLifecycleDiagnostic::InvalidShutdownSequence,
            )),
            SessionState::AwaitingInitializedNotification { .. } => Some(McpDiagnostic::Lifecycle(
                McpLifecycleDiagnostic::InitializedNotificationMissing,
            )),
            SessionState::Closed(_) => None,
        };
        if let Some(diagnostic) = incomplete {
            if let Err(error) = record_current_session_finding_with_admission(
                adapter,
                &mut runtime,
                diagnostic,
                None,
                None,
                None,
                Vec::new(),
                true,
            ) {
                let failed = SessionState::Closed(ClosedSession {
                    runtime,
                    termination: SessionTermination::TerminalFailure,
                });
                return Err(SessionTransitionFailure::new(failed, error));
            }
        } else if let Err(error) = with_mcp_runtime_home_mutation(
            &adapter.runtime_home,
            "mcp.runtime_session.close",
            |context| {
                let observed_at = runtime.next_observation_timestamp();
                record_mcp_graceful_close(context, &runtime.runtime_session_id, &observed_at)
                    .map_err(McpAdapterError::Store)
            },
        ) {
            let failed = SessionState::Closed(ClosedSession {
                runtime,
                termination: SessionTermination::TerminalFailure,
            });
            return Err(SessionTransitionFailure::new(failed, error));
        }
    }
    let termination = if runtime.terminal_finding_recorded {
        SessionTermination::TerminalFailure
    } else {
        SessionTermination::GracefulEof
    };
    Ok(SessionState::Closed(ClosedSession {
        runtime,
        termination,
    }))
}

pub(crate) fn terminate_session(
    adapter: &McpAdapter,
    mut state: SessionState,
    error: &McpAdapterError,
) -> SessionState {
    if !state.runtime().terminal_finding_recorded {
        let diagnostic = McpDiagnostic::from(error);
        let _ = record_current_session_finding_with_admission(
            adapter,
            state.runtime_mut(),
            diagnostic,
            None,
            None,
            None,
            Vec::new(),
            true,
        );
    }
    SessionState::Closed(ClosedSession {
        runtime: state.runtime().clone(),
        termination: SessionTermination::TerminalFailure,
    })
}

#[cfg(test)]
mod observation_clock_tests {
    use super::*;

    #[test]
    fn runtime_observation_floor_never_moves_backward() {
        let mut runtime = SessionRuntime::for_session_source(McpRuntimeSessionSource::ManagedHost);
        let later = UtcTimestamp::parse("2026-08-06T01:02:03.500Z").expect("valid timestamp");
        let earlier = UtcTimestamp::parse("2026-08-06T01:02:03.499Z").expect("valid timestamp");

        assert_eq!(
            runtime.advance_observation_floor(later.clone()),
            later.to_canonical_string()
        );
        assert_eq!(
            runtime.advance_observation_floor(earlier),
            later.to_canonical_string()
        );
    }
}
