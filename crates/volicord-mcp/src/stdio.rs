use crate::adapter::*;
use crate::diagnostics::{
    data_for_diagnostic, production_supported_revisions, JsonRpcDiagnostic, McpDiagnostic,
    McpDiagnosticContext, McpLifecycleDiagnostic, McpProtocolDiagnostic, McpToolCallDiagnostic,
    McpToolDiscoveryDiagnostic,
};
use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError, McpHostError};
use crate::prelude::*;
use crate::routing::*;
use crate::schema_validation::validate_mcp_tool_output;
use crate::tool_registry::{CanonicalContent, CanonicalToolResult};
use crate::util::*;
use crate::VOLICORD_HOME_ENV;
use sha2::{Digest, Sha256};
use volicord_host_contract::{CodexMcpCorrelation, CodexMcpTurnMetadataV1, HostContractErrorCode};
use volicord_types::{HostKind, ManagedMcpClientInfo};

const CODEX_THREAD_BINDING_DOMAIN: &[u8] = b"volicord.codex-mcp-thread-binding\0";
pub(crate) const MAX_MCP_COMPACT_MUTATION_RESULT_BYTES: usize = 65_536;
pub(crate) const MAX_MCP_FULL_MUTATION_RESULT_BYTES: usize = 256 * 1024;
pub(crate) const MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES: usize = 512;
pub(crate) const MAX_MCP_REQUEST_LINE_BYTES: usize = 1024 * 1024;

pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    run_stdio_with_options(adapter, reader, writer, StdioRunOptions::default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StdioRunOptions {
    session_source: McpRuntimeSessionSource,
    managed_lease: Option<ManagedMcpLaunchLeaseConsumption>,
    observed_host_executable_version: Option<String>,
}

impl Default for StdioRunOptions {
    fn default() -> Self {
        Self {
            session_source: McpRuntimeSessionSource::ManualCli,
            managed_lease: None,
            observed_host_executable_version: None,
        }
    }
}

fn run_stdio_with_options<R, W>(
    adapter: McpAdapter,
    mut reader: R,
    mut writer: W,
    options: StdioRunOptions,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let mut state = ConnectionState::for_session_source(options.session_source);
    let process_started_at = authoritative_observation_timestamp();
    let runtime_start = McpRuntimeSessionStart {
        connection_internal_id: adapter.context.connection_internal_id.as_str().to_owned(),
        session_source: options.session_source,
        observed_host_executable_version: options.observed_host_executable_version,
        process_id: std::process::id(),
        process_started_at,
    };
    let runtime_session = if let Some(lease) = options.managed_lease {
        if options.session_source != McpRuntimeSessionSource::ManagedHost {
            return Err(McpAdapterError::Environment(
                "managed launch lease requires session_source=managed_host".to_owned(),
            ));
        }
        consume_managed_mcp_launch_lease_and_start_runtime(
            &adapter.runtime_home,
            lease,
            runtime_start,
        )
    } else {
        start_mcp_runtime_session(&adapter.runtime_home, runtime_start)
    };
    let runtime_session = runtime_session.map_err(McpAdapterError::Store)?;
    state.runtime_session_id = runtime_session.runtime_session_id;

    let transport_result = (|| {
        validate_managed_stdio_session_ownership(&adapter, &state)?;
        if !state.codex_binding.is_pending() && !state.managed_stdio_binding_active {
            let _ = start_transport_diagnostic_session(&adapter, &state);
        }
        loop {
            let line = match read_bounded_json_line(&mut reader)? {
                BoundedJsonLine::Eof => break,
                BoundedJsonLine::Line(line) => line,
                BoundedJsonLine::InvalidUtf8 => {
                    record_current_session_finding(
                        &adapter,
                        &mut state,
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::ParseError),
                        Some(-32700),
                        Some("input was not valid UTF-8".to_owned()),
                        None,
                        Vec::new(),
                        false,
                    )?;
                    write_json_line(
                        &mut writer,
                        json_rpc_error(Value::Null, -32700, "Parse error", None),
                    )?;
                    continue;
                }
                BoundedJsonLine::TooLong => {
                    record_current_session_finding(
                        &adapter,
                        &mut state,
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::MessageSizeExceeded),
                        Some(-32600),
                        Some(format!(
                            "request exceeded the {MAX_MCP_REQUEST_LINE_BYTES}-byte limit"
                        )),
                        None,
                        Vec::new(),
                        false,
                    )?;
                    write_json_line(
                        &mut writer,
                        invalid_request_response(&Value::Null, "request message is too large"),
                    )?;
                    continue;
                }
                BoundedJsonLine::Incomplete => {
                    record_current_session_finding(
                        &adapter,
                        &mut state,
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::FramingFailure),
                        Some(-32600),
                        Some("request ended without newline framing".to_owned()),
                        None,
                        Vec::new(),
                        true,
                    )?;
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }

            let message: Value = match serde_json::from_str(&line) {
                Ok(message) => message,
                Err(error) => {
                    record_current_session_finding(
                        &adapter,
                        &mut state,
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::ParseError),
                        Some(-32700),
                        Some(format!(
                            "invalid JSON at line {} column {}",
                            error.line(),
                            error.column()
                        )),
                        None,
                        Vec::new(),
                        false,
                    )?;
                    write_json_line(
                        &mut writer,
                        json_rpc_error(Value::Null, -32700, "Parse error", None),
                    )?;
                    continue;
                }
            };

            if let Some(response) = handle_json_rpc_message(&adapter, &mut state, message)? {
                write_json_line(&mut writer, response)?;
            }
        }

        writer.flush().map_err(McpAdapterError::Io)
    })();

    match transport_result {
        Ok(()) => {
            if !state.terminal_finding_recorded {
                let incomplete = match state.phase {
                    ConnectionPhase::Ready => None,
                    ConnectionPhase::AwaitingInitialize => Some(McpDiagnostic::Lifecycle(
                        McpLifecycleDiagnostic::InvalidShutdownSequence,
                    )),
                    ConnectionPhase::AwaitingInitialized
                        if state.mcp_session.as_ref().is_some_and(|session| {
                            session.outcome == McpNegotiationOutcome::ServerCounterOffer
                        }) =>
                    {
                        Some(McpDiagnostic::Protocol(
                            McpProtocolDiagnostic::CounterOfferRejectedOrDisconnected,
                        ))
                    }
                    ConnectionPhase::AwaitingInitialized => Some(McpDiagnostic::Lifecycle(
                        McpLifecycleDiagnostic::InitializedNotificationMissing,
                    )),
                };
                if let Some(diagnostic) = incomplete {
                    record_current_session_finding(
                        &adapter,
                        &mut state,
                        diagnostic,
                        None,
                        None,
                        None,
                        Vec::new(),
                        true,
                    )?;
                } else {
                    record_mcp_graceful_close(
                        &adapter.runtime_home,
                        &state.runtime_session_id,
                        &authoritative_observation_timestamp(),
                    )
                    .map_err(McpAdapterError::Store)?;
                }
            }
            Ok(())
        }
        Err(error) => {
            if !state.terminal_finding_recorded {
                record_current_session_finding(
                    &adapter,
                    &mut state,
                    McpDiagnostic::from(&error),
                    None,
                    None,
                    None,
                    Vec::new(),
                    true,
                )?;
            }
            Err(error)
        }
    }
}

enum BoundedJsonLine {
    Eof,
    Line(String),
    InvalidUtf8,
    TooLong,
    Incomplete,
}

fn read_bounded_json_line(reader: &mut impl BufRead) -> Result<BoundedJsonLine, McpAdapterError> {
    let mut bytes = Vec::with_capacity(8 * 1024);
    let mut exceeded = false;
    loop {
        let available = reader.fill_buf().map_err(McpAdapterError::Io)?;
        if available.is_empty() {
            return if bytes.is_empty() && !exceeded {
                Ok(BoundedJsonLine::Eof)
            } else {
                Ok(BoundedJsonLine::Incomplete)
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        let content_end = newline.unwrap_or(available.len());
        if !exceeded {
            let remaining = MAX_MCP_REQUEST_LINE_BYTES.saturating_sub(bytes.len());
            let retained = remaining.min(content_end);
            bytes.extend_from_slice(&available[..retained]);
            exceeded = retained < content_end;
        }
        reader.consume(consumed);
        if newline.is_some() {
            if exceeded {
                return Ok(BoundedJsonLine::TooLong);
            }
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            return String::from_utf8(bytes)
                .map(BoundedJsonLine::Line)
                .or(Ok(BoundedJsonLine::InvalidUtf8));
        }
    }
}

fn authoritative_observation_timestamp() -> String {
    UtcTimestamp::from_datetime(chrono::DateTime::<chrono::Utc>::from(SystemTime::now()))
        .to_canonical_string()
}

/// Runs the MCP stdio adapter from process environment and stdin/stdout.
pub fn run_stdio_from_env(
    connection_id: &str,
    project_id: Option<&str>,
    observed_host_executable_version: Option<String>,
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
    let adapter = McpAdapter::new(runtime_home, context);
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManualCli,
            managed_lease: None,
            observed_host_executable_version,
        },
    )
}

/// Runs stdio from a clone-portable shared managed launch.
///
/// The launch carries only the host selector. Connection and project
/// identities are resolved from the current Git repository and the selected
/// local Runtime Home before the transport starts.
pub fn run_stdio_discover_repository_from_env(
    host: HostKind,
    observed_host_executable_version: Option<String>,
) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_repository_discovery_runtime_home(process_env_var, &current_dir)?;
    let resolution = RepositoryDiscoveryResolution::resolve(&runtime_home, &current_dir, host)?;
    let adapter = McpAdapter::new(runtime_home, resolution.context);
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManualCli,
            managed_lease: None,
            observed_host_executable_version,
        },
    )
}

/// Runs managed stdio only through an in-memory one-time launch-lease claim.
pub fn run_managed_stdio(
    adapter: McpAdapter,
    managed_lease: ManagedMcpLaunchLeaseConsumption,
    observed_host_executable_version: Option<String>,
) -> Result<(), McpAdapterError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManagedHost,
            managed_lease: Some(managed_lease),
            observed_host_executable_version,
        },
    )
}

/// Resolves the explicit Runtime Home required by repository discovery.
pub fn resolve_repository_discovery_runtime_home<F>(
    env_var: F,
    current_dir: &Path,
) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = env_var(VOLICORD_HOME_ENV).ok_or_else(|| {
        McpAdapterError::Environment(
            "repository discovery MCP startup requires VOLICORD_HOME; refusing to substitute the platform default Runtime Home"
            .to_owned(),
        )
    })?;
    if runtime_home.is_empty() {
        return Err(RuntimeHomeResolutionError::EmptyVolicordHome.into());
    }
    if !Path::new(&runtime_home).is_absolute() {
        return Err(McpAdapterError::Environment(
            "repository discovery MCP startup requires an absolute VOLICORD_HOME; refusing current-directory-relative Runtime Home selection"
                .to_owned(),
        ));
    }
    resolve_runtime_home(
        |name| (name == VOLICORD_HOME_ENV).then(|| runtime_home.clone()),
        current_dir,
    )
}

#[cfg(test)]
pub(crate) fn run_managed_stdio_with_test_lease<R, W>(
    adapter: McpAdapter,
    reader: R,
    writer: W,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| McpAdapterError::Environment("test Connection disappeared".to_owned()))?;
    let revision = connection_integration_revision(&connection).map_err(McpAdapterError::Store)?;
    let lease = issue_managed_mcp_launch_lease(
        &adapter.runtime_home,
        ManagedMcpLaunchLeaseIssue {
            connection_internal_id: connection.connection_internal_id.clone(),
            host_kind: HostKind::Codex,
            expected_integration_revision: revision.as_str().to_owned(),
            expected_launch_fingerprint: connection.managed_fingerprint.clone(),
        },
    )
    .map_err(McpAdapterError::Store)?;
    run_stdio_with_options(
        adapter,
        reader,
        writer,
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManagedHost,
            managed_lease: Some(ManagedMcpLaunchLeaseConsumption {
                launch_lease_id: lease.launch_lease_id,
                connection_internal_id: lease.connection_internal_id,
                host_kind: lease.host_kind,
                expected_integration_revision: lease.expected_integration_revision,
                expected_launch_fingerprint: lease.expected_launch_fingerprint,
            }),
            observed_host_executable_version: None,
        },
    )
}

#[cfg(test)]
pub(crate) fn run_manual_stdio_with_ignored_env_marker<R, W, F>(
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
    let _ = env_var;
    run_stdio(adapter, reader, writer)
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
    let started_at = authoritative_observation_timestamp();
    let session = start_mcp_runtime_session(
        &runtime_home,
        McpRuntimeSessionStart {
            connection_internal_id: connection_id.to_owned(),
            session_source: McpRuntimeSessionSource::CliPreflight,
            observed_host_executable_version: None,
            process_id: std::process::id(),
            process_started_at: started_at,
        },
    )
    .map_err(McpAdapterError::Store)?;
    record_mcp_graceful_close(
        &runtime_home,
        &session.runtime_session_id,
        &authoritative_observation_timestamp(),
    )
    .map_err(McpAdapterError::Store)?;
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

#[cfg(test)]
mod repository_discovery_runtime_home_tests {
    use super::*;

    #[test]
    fn missing_runtime_home_fails_before_default_home_lookup() {
        let error = resolve_repository_discovery_runtime_home(
            |name| {
                if name == "VOLICORD_HOME" {
                    None
                } else {
                    panic!("repository discovery must not inspect default-home variable {name}")
                }
            },
            Path::new("/repo"),
        )
        .expect_err("missing VOLICORD_HOME must fail closed");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error.to_string().contains("requires VOLICORD_HOME"));
        assert!(error.to_string().contains("refusing to substitute"));
    }

    #[test]
    fn empty_runtime_home_fails_before_repository_discovery() {
        let error = resolve_repository_discovery_runtime_home(
            |name| (name == "VOLICORD_HOME").then(OsString::new),
            Path::new("/repo"),
        )
        .expect_err("empty VOLICORD_HOME must fail closed");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error
            .to_string()
            .contains("VOLICORD_HOME must not be empty"));
    }

    #[test]
    fn relative_runtime_home_fails_before_repository_discovery() {
        let error = resolve_repository_discovery_runtime_home(
            |name| (name == "VOLICORD_HOME").then(|| OsString::from("runtime")),
            Path::new("/repo"),
        )
        .expect_err("relative VOLICORD_HOME must fail closed");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error.to_string().contains("absolute VOLICORD_HOME"));
        assert!(error.to_string().contains("current-directory-relative"));
    }

    #[test]
    fn explicit_absolute_runtime_home_is_used_as_supplied() {
        let absolute = std::env::current_dir()
            .expect("current directory")
            .join("runtime-home");
        let runtime_home = resolve_repository_discovery_runtime_home(
            |name| (name == "VOLICORD_HOME").then(|| absolute.clone().into_os_string()),
            Path::new("ignored"),
        )
        .expect("explicit absolute repository discovery Runtime Home");

        assert_eq!(runtime_home, absolute);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionPhase {
    AwaitingInitialize,
    AwaitingInitialized,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CodexManagedBinding {
    NotApplicable,
    Pending,
    Bound {
        thread_digest: [u8; 32],
        correlation: CodexMcpCorrelation,
    },
}

impl CodexManagedBinding {
    const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionState {
    pub(crate) phase: ConnectionPhase,
    pub(crate) mcp_session: Option<NegotiatedMcpSession>,
    pub(crate) runtime_session_id: String,
    pub(crate) managed_stdio_binding_active: bool,
    pub(crate) launch_origin: &'static str,
    status_method_call_count: u64,
    terminal_finding_recorded: bool,
    pending_finding: Option<McpDiagnostic>,
    codex_binding: CodexManagedBinding,
    deferred_tools_list_serialized_bytes: Option<u64>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::AwaitingInitialize,
            mcp_session: None,
            runtime_session_id: String::new(),
            managed_stdio_binding_active: false,
            launch_origin: "unknown",
            status_method_call_count: 0,
            terminal_finding_recorded: false,
            pending_finding: None,
            codex_binding: CodexManagedBinding::NotApplicable,
            deferred_tools_list_serialized_bytes: None,
        }
    }
}

/// Session-scoped MCP selection and handshake state.
///
/// The selected profile is available after a valid initialize request, while
/// `initialized_notification_completed` remains false until the client
/// completes the required lifecycle step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NegotiatedMcpSession {
    pub(crate) requested_protocol_version: String,
    pub(crate) requested_protocol_version_well_formed: bool,
    pub(crate) selected_profile: &'static McpProtocolProfile,
    pub(crate) outcome: McpNegotiationOutcome,
    pub(crate) client_capabilities: Map<String, Value>,
    pub(crate) attempted_client_name: String,
    pub(crate) attempted_client_version: String,
    pub(crate) initialized_notification_completed: bool,
}

impl NegotiatedMcpSession {
    const fn requires_initialized_notification(&self) -> bool {
        matches!(
            self.selected_profile.messages().initialized_notification(),
            InitializedNotification::AfterInitialize
        )
    }
}

impl ConnectionState {
    fn managed_agent_session_binding(&self) -> Option<ManagedAgentSessionBinding> {
        let CodexManagedBinding::Bound { correlation, .. } = &self.codex_binding else {
            return None;
        };
        Some(ManagedAgentSessionBinding {
            runtime_session_id: self.runtime_session_id.clone(),
            correlation: correlation.clone(),
        })
    }

    fn for_session_source(session_source: McpRuntimeSessionSource) -> Self {
        let pending_codex = session_source == McpRuntimeSessionSource::ManagedHost;
        Self {
            launch_origin: session_source.as_str(),
            codex_binding: if pending_codex {
                CodexManagedBinding::Pending
            } else {
                CodexManagedBinding::NotApplicable
            },
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
    pub(crate) diagnostic: JsonRpcDiagnostic,
}

pub(crate) fn handle_json_rpc_message(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    message: Value,
) -> Result<Option<Value>, McpAdapterError> {
    if let Value::Array(entries) = message {
        return handle_json_rpc_batch(adapter, state, entries);
    }
    handle_single_json_rpc_message(adapter, state, message)
}

fn handle_single_json_rpc_message(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    message: Value,
) -> Result<Option<Value>, McpAdapterError> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            handle_json_rpc_request(adapter, state, request).map(Some)
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(adapter, state, notification)?;
            Ok(None)
        }
        Err(error) => Ok(Some(json_rpc_error(
            error.id.clone(),
            error.code,
            error.message,
            error.data.clone(),
        )))
        .and_then(|response| {
            record_current_session_finding(
                adapter,
                state,
                McpDiagnostic::JsonRpc(error.diagnostic),
                Some(error.code),
                error.data,
                None,
                Vec::new(),
                false,
            )?;
            Ok(response)
        }),
    }
}

fn handle_json_rpc_batch(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    entries: Vec<Value>,
) -> Result<Option<Value>, McpAdapterError> {
    if entries.is_empty() {
        record_current_session_finding(
            adapter,
            state,
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
            adapter,
            state,
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
        if let Some(response) = handle_single_json_rpc_message(adapter, state, entry)? {
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
            Self::OperationBeforeReady => {
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
        }
    }
}

fn admit_json_rpc_batch(
    state: &ConnectionState,
    entries: &[Value],
) -> Result<(), JsonRpcBatchAdmissionError> {
    if entries.iter().any(batch_entry_is_initialization_message) {
        return Err(JsonRpcBatchAdmissionError::InitializationMessage);
    }

    let session = match state.phase {
        ConnectionPhase::AwaitingInitialize => {
            return Err(JsonRpcBatchAdmissionError::InitializeRequired)
        }
        ConnectionPhase::AwaitingInitialized => {
            return Err(JsonRpcBatchAdmissionError::OperationBeforeReady)
        }
        ConnectionPhase::Ready => state
            .mcp_session
            .as_ref()
            .filter(|session| session.initialized_notification_completed)
            .ok_or(JsonRpcBatchAdmissionError::OperationBeforeReady)?,
    };

    if session.selected_profile.messages().json_rpc_batching() != JsonRpcBatching::Allowed {
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

pub(crate) fn parse_client_message(message: Value) -> Result<ClientMessage, JsonRpcFailure> {
    let object = match message {
        Value::Object(object) => object,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            return Err(invalid_request_with_diagnostic(
                Value::Null,
                "message must be a JSON object",
                JsonRpcDiagnostic::InvalidRequest,
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
            return Err(invalid_request_with_diagnostic(
                response_id,
                "jsonrpc must be exactly \"2.0\"",
                JsonRpcDiagnostic::InvalidRequest,
            ));
        }
    }

    let Some(Value::String(method)) = object.get("method") else {
        return Err(invalid_request_with_diagnostic(
            response_id,
            "method must be a string",
            JsonRpcDiagnostic::InvalidRequest,
        ));
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
            Err(invalid_request_with_diagnostic(
                Value::Null,
                "id must be a string or integer",
                JsonRpcDiagnostic::InvalidId,
            ))
        }
    }
}

pub(crate) fn handle_json_rpc_notification(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    notification: JsonRpcNotification,
) -> Result<(), McpAdapterError> {
    let completes_selected_handshake = state
        .mcp_session
        .as_ref()
        .is_some_and(NegotiatedMcpSession::requires_initialized_notification);
    if notification.method == "notifications/initialized"
        && state.phase == ConnectionPhase::AwaitingInitialized
        && completes_selected_handshake
        && notification_params_are_object_or_absent(notification.params.as_ref())
    {
        let selected_revision = state
            .mcp_session
            .as_ref()
            .expect("awaiting initialized has a selected MCP session")
            .selected_profile
            .revision()
            .as_str();
        if !state.runtime_session_id.is_empty() {
            record_mcp_initialized_notification(
                &adapter.runtime_home,
                &state.runtime_session_id,
                selected_revision,
                &authoritative_observation_timestamp(),
            )
            .map_err(McpAdapterError::Store)?;
        }
        state
            .mcp_session
            .as_mut()
            .expect("awaiting initialized has a selected MCP session")
            .initialized_notification_completed = true;
        state.phase = ConnectionPhase::Ready;
    } else if notification.method == "notifications/initialized"
        && !(state.phase == ConnectionPhase::Ready
            && notification_params_are_object_or_absent(notification.params.as_ref()))
    {
        record_current_session_finding(
            adapter,
            state,
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationInvalid),
            Some(-32600),
            Some("notifications/initialized did not match the selected lifecycle state".to_owned()),
            None,
            Vec::new(),
            false,
        )?;
    }
    Ok(())
}

pub(crate) fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

pub(crate) fn handle_json_rpc_request(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
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
    let response = handle_json_rpc_request_inner(adapter, state, request)?;
    if method == "tools/call" && state.pending_finding.is_none() {
        state.pending_finding = response
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
    let pending_finding = state.pending_finding.take();
    let safe_tool_failed = safe_tool_name.is_some()
        && state.managed_stdio_binding_active
        && safe_tool_call_response_failed(&response);
    if safe_tool_failed {
        if let Some(failure) = pending_finding {
            record_current_session_finding(
                adapter,
                state,
                failure,
                json_rpc_code,
                safe_error_data.clone(),
                safe_tool_name.clone(),
                Vec::new(),
                false,
            )?;
        }
        record_current_session_finding(
            adapter,
            state,
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::SafeReadOnlyToolFailure),
            json_rpc_code,
            safe_error_data,
            safe_tool_name,
            Vec::new(),
            true,
        )?;
    } else if let Some(failure) = pending_finding.or(fallback_failure) {
        record_current_session_finding(
            adapter,
            state,
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
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    request: JsonRpcRequest,
) -> Result<Value, McpAdapterError> {
    if let Some((error, diagnostic)) = lifecycle_error_with_diagnostic(state, &request) {
        state.pending_finding = Some(diagnostic);
        return Ok(error);
    }

    let response_id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => {
            if let Some((client_info, requested_protocol_version)) =
                parsed_initialize_attempt(request.params.as_ref())
            {
                if !state.runtime_session_id.is_empty() {
                    record_mcp_initialize_attempt(
                        &adapter.runtime_home,
                        &state.runtime_session_id,
                        &client_info,
                        &requested_protocol_version,
                        &authoritative_observation_timestamp(),
                    )
                    .map_err(McpAdapterError::Store)?;
                }
            }
            let mcp_session = match validate_initialize_params(&response_id, request.params) {
                Ok(mcp_session) => mcp_session,
                Err((error, diagnostic)) => {
                    state.pending_finding = Some(diagnostic);
                    return Ok(error);
                }
            };
            if !state.runtime_session_id.is_empty() {
                record_mcp_initialize_completion(
                    &adapter.runtime_home,
                    &state.runtime_session_id,
                    mcp_session.selected_profile.revision().as_str(),
                    &authoritative_observation_timestamp(),
                )
                .map_err(McpAdapterError::Store)?;
            }
            let result = initialize_result(&mcp_session);
            let counter_offer = mcp_session.outcome == McpNegotiationOutcome::ServerCounterOffer;
            let requested_revision_well_formed = mcp_session.requested_protocol_version_well_formed;
            state.mcp_session = Some(mcp_session);
            state.phase = ConnectionPhase::AwaitingInitialized;
            if counter_offer {
                record_current_session_finding(
                    adapter,
                    state,
                    McpDiagnostic::Protocol(if requested_revision_well_formed {
                        McpProtocolDiagnostic::UnsupportedVersion
                    } else {
                        McpProtocolDiagnostic::MalformedVersion
                    }),
                    None,
                    Some("requested revision is outside the production-supported set".to_owned()),
                    None,
                    Vec::new(),
                    false,
                )?;
                record_current_session_finding(
                    adapter,
                    state,
                    McpDiagnostic::Protocol(McpProtocolDiagnostic::CounterOffer),
                    None,
                    Some("server selected its preferred production revision".to_owned()),
                    None,
                    Vec::new(),
                    false,
                )?;
            }
            if !state.codex_binding.is_pending() && state.managed_stdio_binding_active {
                let _ = start_transport_diagnostic_session(adapter, state);
            }
            result
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
                state.pending_finding = Some(McpDiagnostic::ToolDiscovery(
                    McpToolDiscoveryDiagnostic::ProtocolError,
                ));
                return Ok(error);
            }
            match adapter.tools() {
                Ok(canonical_tools) => {
                    let required_tools_present =
                        required_tool_set_present(adapter, &canonical_tools)?;
                    let returned_tool_identities = canonical_tools
                        .iter()
                        .map(|tool| tool.id.wire_name().to_owned())
                        .collect::<Vec<_>>();
                    if !required_tools_present {
                        state.pending_finding = Some(McpDiagnostic::ToolDiscovery(
                            McpToolDiscoveryDiagnostic::RequiredToolMissing,
                        ));
                    }
                    let profile = state
                        .mcp_session
                        .as_ref()
                        .expect("tools/list lifecycle requires a selected MCP profile")
                        .selected_profile;
                    let tools = canonical_tools
                        .iter()
                        .map(|tool| tool.project(profile))
                        .collect::<Vec<_>>();
                    let result = json!({ "tools": tools });
                    if !state.runtime_session_id.is_empty() {
                        record_mcp_tools_list(
                            &adapter.runtime_home,
                            &state.runtime_session_id,
                            &returned_tool_identities,
                            required_tools_present,
                            &authoritative_observation_timestamp(),
                        )
                        .map_err(McpAdapterError::Store)?;
                    }
                    let serialized_bytes = serde_json::to_vec(&result)
                        .ok()
                        .and_then(|bytes| u64::try_from(bytes.len()).ok());
                    if state.codex_binding.is_pending() {
                        if state.deferred_tools_list_serialized_bytes.is_none() {
                            state.deferred_tools_list_serialized_bytes = serialized_bytes;
                        }
                    } else {
                        if let Some(serialized_bytes) = serialized_bytes {
                            record_tools_list_metric_best_effort(adapter, state, serialized_bytes);
                        }
                    }
                    result
                }
                Err(error) => {
                    state.pending_finding = Some(McpDiagnostic::from(&error));
                    return Ok(json_rpc_error_for_adapter(response_id, error));
                }
            }
        }
        "tools/call" => match call_tool_result(adapter, &response_id, request.params, state)? {
            Ok(result) => result,
            Err(error) => return Ok(error),
        },
        _ => {
            state.pending_finding = Some(McpDiagnostic::JsonRpc(JsonRpcDiagnostic::UnknownMethod));
            return Ok(json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(request.method),
            ));
        }
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": response_id,
        "result": result
    }))
}

fn safe_tool_call_response_failed(response: &Value) -> bool {
    if response.get("error").is_some() {
        return true;
    }
    let Some(result) = response.get("result") else {
        return false;
    };
    let error_code = projected_tool_error_code(result);
    let is_error = result.get("isError").and_then(Value::as_bool) == Some(true)
        || result
            .pointer("/toolResult/code")
            .and_then(Value::as_str)
            .is_some_and(|code| code.starts_with("MCP_"));
    is_error && error_code.as_deref() != Some("MCP_INVALID_ARGUMENTS")
}

fn projected_tool_error_code(result: &Value) -> Option<String> {
    if let Some(code) = result
        .pointer("/structuredContent/code")
        .or_else(|| result.pointer("/toolResult/code"))
        .and_then(Value::as_str)
    {
        return Some(code.to_owned());
    }
    result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|value| value.get("code").and_then(Value::as_str).map(str::to_owned))
}

#[allow(clippy::too_many_arguments)]
fn record_current_session_finding(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    diagnostic: McpDiagnostic,
    json_rpc_error_code: Option<i64>,
    safe_error_data: Option<String>,
    tool_name: Option<String>,
    missing_tools: Vec<String>,
    terminal: bool,
) -> Result<(), McpAdapterError> {
    if state.runtime_session_id.is_empty() || (terminal && state.terminal_finding_recorded) {
        return Ok(());
    }
    let runtime = mcp_runtime_session(&adapter.runtime_home, &state.runtime_session_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| McpAdapterError::Protocol("MCP runtime session disappeared".to_owned()))?;
    let data = data_for_diagnostic(
        diagnostic,
        &McpDiagnosticContext {
            observed_at: UtcTimestamp::parse(&authoritative_observation_timestamp()).map_err(
                |_| McpAdapterError::Protocol("diagnostic timestamp is invalid".to_owned()),
            )?,
            connection_id: Some(runtime.connection_internal_id),
            integration_revision: Some(runtime.connection_integration_revision),
            runtime_session_id: Some(state.runtime_session_id.clone()),
            requested_revision: runtime.requested_protocol_version,
            selected_revision: runtime.selected_protocol_version,
            negotiated_revision: runtime.negotiated_protocol_version,
            supported_revisions: production_supported_revisions(),
            attempted_client_name: runtime.attempted_client_name,
            attempted_client_version: runtime.attempted_client_version,
            json_rpc_error_code,
            safe_error_data,
            tool_name,
            missing_tools,
        },
    )
    .map_err(|error| {
        McpAdapterError::Protocol(format!("structured diagnostic finding is invalid: {error}"))
    })?;
    let finding = OccurrenceDiagnosticFinding::try_new(
        data,
        Some(AgentRuntimeSessionId::new(state.runtime_session_id.clone())),
    )
    .map_err(|error| {
        McpAdapterError::Protocol(format!(
            "structured diagnostic occurrence is invalid: {error}"
        ))
    })?;
    if terminal {
        record_mcp_terminal_finding(&adapter.runtime_home, &finding)
            .map_err(McpAdapterError::Store)?;
        state.terminal_finding_recorded = true;
    } else {
        insert_occurrence_finding(&adapter.runtime_home, &finding)
            .map_err(McpAdapterError::Store)?;
    }
    Ok(())
}

fn required_tool_set_present(
    adapter: &McpAdapter,
    tools: &[crate::tool_registry::CanonicalToolDefinition],
) -> Result<bool, McpAdapterError> {
    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| {
        McpAdapterError::Environment("tools/list Agent Connection disappeared".to_owned())
    })?;
    let mode = match connection.mode.as_str() {
        CONNECTION_MODE_WORKFLOW => AgentConnectionMode::Workflow,
        CONNECTION_MODE_READ_ONLY => AgentConnectionMode::ReadOnly,
        _ => {
            return Err(McpAdapterError::Environment(
                "tools/list Agent Connection has an invalid mode".to_owned(),
            ))
        }
    };
    let actual = tools
        .iter()
        .map(|tool| tool.id.wire_name())
        .collect::<BTreeSet<_>>();
    Ok(AgentToolId::ALL
        .iter()
        .filter(|tool| tool.available_in(mode))
        .all(|tool| actual.contains(tool.wire_name())))
}

fn lifecycle_error_with_diagnostic(
    state: &ConnectionState,
    request: &JsonRpcRequest,
) -> Option<(Value, McpDiagnostic)> {
    match state.phase {
        ConnectionPhase::AwaitingInitialize if request.method != "initialize" => Some((
            invalid_request_response(&request.id, "initialize must be the first request"),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializeRequired),
        )),
        ConnectionPhase::AwaitingInitialize => None,
        ConnectionPhase::AwaitingInitialized => match request.method.as_str() {
            "initialize" => Some((
                invalid_request_response(&request.id, "initialize has already completed"),
                McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize),
            )),
            "tools/list" => None,
            "tools/call"
                if state
                    .mcp_session
                    .as_ref()
                    .is_none_or(NegotiatedMcpSession::requires_initialized_notification) =>
            {
                Some((
                    invalid_request_response(
                        &request.id,
                        "tools/call requires notifications/initialized",
                    ),
                    McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady),
                ))
            }
            _ => None,
        },
        ConnectionPhase::Ready if request.method == "initialize" => Some((
            invalid_request_response(&request.id, "initialize has already completed"),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize),
        )),
        ConnectionPhase::Ready => None,
    }
}

fn bind_codex_managed_tool_call(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    params: &Map<String, Value>,
) -> Result<(), McpHostError> {
    if matches!(state.codex_binding, CodexManagedBinding::NotApplicable) {
        return Ok(());
    }
    let binding =
        codex_managed_call_binding(params, adapter.context.connection_internal_id.as_str())?;
    match &mut state.codex_binding {
        CodexManagedBinding::Pending => {
            let mut candidate = state.clone();
            candidate.codex_binding = CodexManagedBinding::Bound {
                thread_digest: binding.thread_digest,
                correlation: binding.correlation,
            };
            candidate.managed_stdio_binding_active = true;
            validate_managed_stdio_session_ownership(adapter, &candidate)
                .map_err(|_| McpHostError::RegisteredSessionCorrelationMismatch)?;
            *state = candidate;
            Ok(())
        }
        CodexManagedBinding::Bound {
            thread_digest,
            correlation,
            ..
        } if correlation.session_id == binding.correlation.session_id
            && *thread_digest == binding.thread_digest =>
        {
            correlation.turn_id = binding.correlation.turn_id;
            Ok(())
        }
        CodexManagedBinding::Bound { .. } => {
            Err(McpHostError::RegisteredSessionCorrelationMismatch)
        }
        CodexManagedBinding::NotApplicable => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexManagedCallBinding {
    thread_digest: [u8; 32],
    correlation: CodexMcpCorrelation,
}

fn codex_managed_call_binding(
    params: &Map<String, Value>,
    connection_internal_id: &str,
) -> Result<CodexManagedCallBinding, McpHostError> {
    let correlation = CodexMcpTurnMetadataV1
        .parse_tools_call_params(params)
        .map_err(|error| match error.code() {
            HostContractErrorCode::InconsistentCorrelation => {
                McpHostError::SessionThreadTurnInconsistent
            }
            _ => McpHostError::MalformedNativeMetadata,
        })?;
    let thread_digest = codex_thread_correlation_digest(
        connection_internal_id,
        correlation.session_id.as_str(),
        correlation.thread_id.as_str(),
    );
    Ok(CodexManagedCallBinding {
        thread_digest,
        correlation,
    })
}

fn codex_thread_correlation_digest(
    connection_internal_id: &str,
    native_session_id: &str,
    native_thread_id: &str,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(CODEX_THREAD_BINDING_DOMAIN);
    digest.update(connection_internal_id.as_bytes());
    digest.update([0]);
    digest.update(native_session_id.as_bytes());
    digest.update([0]);
    digest.update(native_thread_id.as_bytes());
    digest.finalize().into()
}

#[cfg(test)]
mod codex_call_binding_tests {
    use super::*;

    const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";

    fn valid_params() -> Map<String, Value> {
        json!({
            "name": "volicord.status",
            "arguments": {},
            "_meta": {
                "threadId": "thread.alpha",
                "x-codex-turn-metadata": {
                    "session_id": "session.alpha",
                    "thread_id": "thread.alpha",
                    "turn_id": "turn.alpha",
                    "future_field": "allowed"
                },
                "future_meta": true
            }
        })
        .as_object()
        .expect("fixture object")
        .clone()
    }

    #[test]
    fn exact_codex_call_metadata_accepts_extensions_and_has_stable_bindings() {
        let first = codex_managed_call_binding(&valid_params(), "connection.alpha")
            .expect("exact metadata should bind");
        assert_eq!(first.correlation.session_id.as_str(), "session.alpha");
        let replay = codex_managed_call_binding(&valid_params(), "connection.alpha")
            .expect("same metadata should replay");
        assert_eq!(first, replay);

        let mut next_turn = valid_params();
        next_turn["_meta"][CODEX_TURN_METADATA_KEY]["turn_id"] = json!("turn.beta");
        let next = codex_managed_call_binding(&next_turn, "connection.alpha")
            .expect("turn changes do not change the session/thread binding");
        assert_eq!(next.correlation.session_id, first.correlation.session_id);
        assert_eq!(next.thread_digest, first.thread_digest);
        assert_eq!(next.correlation.thread_id, first.correlation.thread_id);
        assert_eq!(next.correlation.turn_id.as_str(), "turn.beta");
    }

    #[test]
    fn malformed_codex_call_metadata_is_rejected_table_driven() {
        let mut cases = Vec::<(&str, Map<String, Value>)>::new();
        let mut push = |label, value: Value| {
            cases.push((
                label,
                value.as_object().expect("negative fixture object").clone(),
            ));
        };
        push(
            "missing meta",
            json!({"name":"volicord.status","arguments":{}}),
        );
        push(
            "non-object meta",
            json!({"name":"volicord.status","arguments":{},"_meta":null}),
        );
        for (label, metadata) in [
            (
                "missing flat thread",
                json!({CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t","turn_id":"u"}}),
            ),
            (
                "non-string flat thread",
                json!({"threadId":1,CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t","turn_id":"u"}}),
            ),
            ("missing nested metadata", json!({"threadId":"t"})),
            (
                "non-object nested metadata",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:null}),
            ),
            (
                "missing session",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"thread_id":"t","turn_id":"u"}}),
            ),
            (
                "missing thread",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"s","turn_id":"u"}}),
            ),
            (
                "missing turn",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t"}}),
            ),
            (
                "invalid session",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"bad session","thread_id":"t","turn_id":"u"}}),
            ),
            (
                "invalid thread",
                json!({"threadId":"bad/thread",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"bad/thread","turn_id":"u"}}),
            ),
            (
                "invalid turn",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t","turn_id":""}}),
            ),
            (
                "thread mismatch",
                json!({"threadId":"t1",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t2","turn_id":"u"}}),
            ),
            (
                "oversized thread",
                json!({"threadId":"x".repeat(257),CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"x".repeat(257),"turn_id":"u"}}),
            ),
        ] {
            push(
                label,
                json!({"name":"volicord.status","arguments":{},"_meta":metadata}),
            );
        }

        for (label, params) in cases {
            assert!(
                codex_managed_call_binding(&params, "connection.alpha").is_err(),
                "{label}"
            );
        }
    }
}

pub(crate) fn initialize_result(session: &NegotiatedMcpSession) -> Value {
    let build = crate::build_info();
    let package_version = build.package_version;
    let capabilities = if session
        .selected_profile
        .schema()
        .server_capability_fields()
        .contains(&ServerCapabilityField::Tools)
    {
        json!({ "tools": {} })
    } else {
        json!({})
    };
    let mut result = json!({
        "_meta": {
            "io.volicord/build": build
        },
        "protocolVersion": session.selected_profile.revision().as_str(),
        "capabilities": capabilities,
        "serverInfo": {
            "name": SERVER_NAME,
            "version": package_version
        }
    });
    if session
        .selected_profile
        .messages()
        .initialize_result_instructions()
    {
        result["instructions"] = Value::String(server_instructions());
    }
    result
}

fn validate_initialize_params(
    id: &Value,
    params: Option<Value>,
) -> Result<NegotiatedMcpSession, (Value, McpDiagnostic)> {
    let object = required_object_params(id, params, "initialize").map_err(|response| {
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
    let selection = ProtocolRegistry::production()
        .negotiate_initialize(requested_protocol_version)
        .map_err(|mismatch| {
            (
                json_rpc_error(
                    id.clone(),
                    -32601,
                    "Method not found",
                    Some(format!(
                        "protocolVersion {} does not use the initialize handshake",
                        mismatch.revision()
                    )),
                ),
                McpDiagnostic::Protocol(McpProtocolDiagnostic::GenerationMismatch),
            )
        })?;

    Ok(NegotiatedMcpSession {
        requested_protocol_version: requested_protocol_version.clone(),
        requested_protocol_version_well_formed: protocol_revision_is_well_formed(
            requested_protocol_version,
        ),
        selected_profile: selection.profile(),
        outcome: selection.outcome(),
        client_capabilities: client_capabilities.clone(),
        attempted_client_name,
        attempted_client_version,
        initialized_notification_completed: false,
    })
}

fn protocol_revision_is_well_formed(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
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

pub(crate) fn call_tool_result(
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
    state: &mut ConnectionState,
) -> Result<Result<Value, Value>, McpAdapterError> {
    let selected_profile = state
        .mcp_session
        .as_ref()
        .expect("tools/call lifecycle requires a selected MCP profile")
        .selected_profile;
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
        .and_then(|tool_name| AgentToolId::from_wire_name(tool_name).ok())
        .map(|tool| tool.wire_name().to_owned());
    let object = match required_object_params(id, params, "tools/call") {
        Ok(object) => object,
        Err(error) => {
            state.pending_finding = Some(McpDiagnostic::ToolCall(
                McpToolCallDiagnostic::InvalidArguments,
            ));
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
        state.pending_finding = Some(McpDiagnostic::ToolCall(
            McpToolCallDiagnostic::InvalidArguments,
        ));
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

    let requested_tool_name = match object.get("name").and_then(Value::as_str) {
        Some(tool_name) => tool_name,
        None => {
            let error = invalid_params_response(id, "tools/call params.name must be a string");
            state.pending_finding = Some(McpDiagnostic::ToolCall(
                McpToolCallDiagnostic::InvalidArguments,
            ));
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
    let tool = match AgentToolId::from_wire_name(requested_tool_name) {
        Ok(tool) => tool,
        Err(_) => {
            let error = json_rpc_error(
                id.clone(),
                -32602,
                "Invalid params",
                Some(format!("unknown MCP tool: {requested_tool_name}")),
            );
            state.pending_finding =
                Some(McpDiagnostic::ToolCall(McpToolCallDiagnostic::UnknownTool));
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
    let tool_name = tool.wire_name();
    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            let error =
                invalid_params_response(id, "tools/call params.arguments must be an object");
            state.pending_finding = Some(McpDiagnostic::ToolCall(
                McpToolCallDiagnostic::InvalidArguments,
            ));
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
    let codex_was_pending = state.codex_binding.is_pending();
    if let Err(error) = bind_codex_managed_tool_call(adapter, state, &object) {
        state.pending_finding = Some(McpDiagnostic::Host(error));
        return Ok(Err(invalid_params_response(id, error.to_string())));
    }
    if codex_was_pending {
        let _ = start_transport_diagnostic_session(adapter, state);
        if let Some(serialized_bytes) = state.deferred_tools_list_serialized_bytes.take() {
            record_tools_list_metric_best_effort(adapter, state, serialized_bytes);
        }
    }
    if tool == AgentToolId::STATUS {
        state.status_method_call_count = state.status_method_call_count.saturating_add(1);
    }
    let mutation_detail = mutation_detail_for_tool(tool, &arguments);

    let binding = state.managed_agent_session_binding();
    let coordinates = binding
        .as_ref()
        .map(|binding| adapter.ensure_agent_session_binding_for_tool(tool, &arguments, binding))
        .transpose()?
        .flatten();

    let output = if matches!(tool.owner(), AgentToolOwner::CoreMethod(_)) {
        let call_result = if let Some(coordinates) = coordinates.as_ref() {
            adapter.call_tool_for_session(tool, arguments, Some(coordinates.borrowed()))
        } else if adapter.has_default_agent_session() {
            adapter.call_tool(tool_name, arguments)
        } else {
            adapter.call_tool_for_session(tool, arguments, None)
        };
        match call_result {
            Ok(response) if tool == AgentToolId::REQUEST_USER_ACTION => {
                let pending_response = response.clone();
                match user_action_tool_output(adapter, response) {
                    Ok(output) => output,
                    Err(_) => ToolCallOutput::from_pipeline_response(&pending_response)?
                        .with_post_effect_failure(
                            McpPostEffectFailureCode::McpPostEffectAdapterFailed,
                        ),
                }
            }
            Ok(response) if tool == AgentToolId::GET_OPERATION_RESULT => {
                ToolCallOutput::from_operation_result_response_for_profile(
                    &response,
                    selected_profile,
                )?
            }
            Ok(response) => ToolCallOutput::from_pipeline_response(&response)?,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_profile(tool_name, &error, selected_profile);
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
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_profile(tool_name, &error, selected_profile);
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
                state.pending_finding = Some(McpDiagnostic::from(&error));
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
        let response = match adapter.call_adapter_tool(tool, arguments, None) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_profile(tool_name, &error, selected_profile);
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
                state.pending_finding = Some(McpDiagnostic::from(&error));
                let response =
                    tool_execution_error_result_for_profile(tool_name, &error, selected_profile);
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
                state.pending_finding = Some(McpDiagnostic::from(&error));
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

    let diagnostic_facts = output.diagnostic_facts();
    let diagnostic_outcome =
        if response_kind_from_structured_content(&output.structured_content) == Some("rejected") {
            DiagnosticOutcome::Rejected
        } else if output.is_error {
            DiagnosticOutcome::ToolError
        } else {
            DiagnosticOutcome::Success
        };
    let verification_tool = ToolVerificationRole::ManagedHostRoundTrip.tool();
    if diagnostic_outcome == DiagnosticOutcome::Success
        && tool == verification_tool
        && state.managed_stdio_binding_active
        && !state.runtime_session_id.is_empty()
    {
        validate_managed_stdio_session_ownership(adapter, state)?;
        record_mcp_verification_tool_observation(
            &adapter.runtime_home,
            &state.runtime_session_id,
            &authoritative_observation_timestamp(),
        )
        .map_err(McpAdapterError::Store)?;
    }
    let response = tool_call_result_from_output_for_profile(tool_name, output, selected_profile)?;
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

fn mutation_detail_for_tool(tool: AgentToolId, arguments: &Value) -> Option<MutationDetailLevel> {
    matches!(
        tool.category(),
        AgentToolCategory::NonDestructiveMutation | AgentToolCategory::DestructiveMutation
    )
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
    effect_anchor: Option<String>,
    fallback_kind: Option<DiagnosticFallbackKind>,
    product_file_write_count: u64,
    authoritative_refresh_failure: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct CanonicalMcpMutationOutcome {
    tool_name: String,
    profile: &'static McpProtocolProfile,
    requested_detail: MutationDetailLevel,
    facts: ToolDiagnosticFacts,
    exact_method_result: Option<Value>,
    compact_method_result: Option<Value>,
    operation_result_ref: Option<OperationResultRef>,
    authority_receipt: Option<AuthorityReceipt>,
    next_actions: Vec<NextActionSummary>,
}

impl CanonicalMcpMutationOutcome {
    fn new(
        tool_name: &str,
        profile: &'static McpProtocolProfile,
        requested_detail: MutationDetailLevel,
        facts: ToolDiagnosticFacts,
        exact_method_result: Option<Value>,
        operation_result_ref: Option<OperationResultRef>,
    ) -> Self {
        let compact_method_result = exact_method_result
            .as_ref()
            .and_then(|result| compact_mutation_method_result(tool_name, result).ok());
        Self {
            tool_name: tool_name.to_owned(),
            profile,
            requested_detail,
            facts,
            exact_method_result,
            compact_method_result,
            operation_result_ref,
            authority_receipt: None,
            next_actions: Vec::new(),
        }
    }

    fn set_authority_refresh(
        &mut self,
        authority_receipt: AuthorityReceipt,
        next_actions: Vec<NextActionSummary>,
    ) {
        self.authority_receipt = Some(authority_receipt);
        self.next_actions = next_actions;
    }

    fn recovery_candidates(
        &self,
        include_exact: bool,
    ) -> [Option<MutationRecoveryCandidate<'_>>; 5] {
        let receipt_and_exact = if include_exact {
            self.authority_receipt
                .as_ref()
                .zip(self.exact_method_result.as_ref())
                .map(|(receipt, method_result)| MutationRecoveryCandidate {
                    authority_receipt: Some(receipt),
                    method_result: Some(method_result),
                })
        } else {
            None
        };
        let receipt_and_compact = self
            .authority_receipt
            .as_ref()
            .zip(self.compact_method_result.as_ref())
            .map(|(receipt, method_result)| MutationRecoveryCandidate {
                authority_receipt: Some(receipt),
                method_result: Some(method_result),
            });
        let receipt_only =
            self.authority_receipt
                .as_ref()
                .map(|receipt| MutationRecoveryCandidate {
                    authority_receipt: Some(receipt),
                    method_result: None,
                });
        let compact_only =
            self.compact_method_result
                .as_ref()
                .map(|method_result| MutationRecoveryCandidate {
                    authority_receipt: None,
                    method_result: Some(method_result),
                });
        [
            receipt_and_exact,
            receipt_and_compact,
            receipt_only,
            compact_only,
            Some(MutationRecoveryCandidate {
                authority_receipt: None,
                method_result: None,
            }),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MutationRecoveryCandidate<'a> {
    authority_receipt: Option<&'a AuthorityReceipt>,
    method_result: Option<&'a Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCallOutput {
    primary_text: String,
    structured_content: Value,
    extra_texts: Vec<String>,
    is_error: bool,
    diagnostic_facts: ToolDiagnosticFacts,
    operation_result_ref: Option<OperationResultRef>,
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
            operation_result_ref: None,
            mutation_refresh_context: None,
            post_effect_failure: None,
        })
    }

    fn from_pipeline_response(response: &PipelineResponse) -> Result<Self, McpAdapterError> {
        let mut output = Self::success(response.response_json.clone())?;
        output.operation_result_ref = response.operation_result_ref.clone();
        output.apply_pipeline_diagnostics(response);
        Ok(output)
    }

    #[cfg(test)]
    fn from_operation_result_response(
        response: &PipelineResponse,
    ) -> Result<Self, McpAdapterError> {
        Self::from_operation_result_response_for_profile(
            response,
            ProtocolRegistry::production().preferred_server_profile(),
        )
    }

    fn from_operation_result_response_for_profile(
        response: &PipelineResponse,
        profile: &McpProtocolProfile,
    ) -> Result<Self, McpAdapterError> {
        let mut output = Self::from_pipeline_response(response)?;
        if output.structured_content["base"]["response_kind"].as_str() == Some("result") {
            let start = output.structured_content["start_offset_bytes"]
                .as_u64()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include start_offset_bytes".to_owned(),
                    )
                })?;
            let end = output.structured_content["end_offset_bytes"]
                .as_u64()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include end_offset_bytes".to_owned(),
                    )
                })?;
            let complete = output.structured_content["complete"]
                .as_bool()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include complete".to_owned(),
                    )
                })?;
            output.primary_text = bounded_mutation_compatibility_text(format!(
                "Volicord returned historical operation-result bytes [{start}, {end}); complete={complete}. Inspect chunk_utf8 in the authoritative result and do not treat historical bytes as current authority."
            ));
            if rendered_tool_call_output_size_for_profile(&output, profile)?
                > MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            {
                return Err(McpAdapterError::Protocol(
                    "operation-result page exceeded its fixed MCP output budget".to_owned(),
                ));
            }
        }
        Ok(output)
    }

    fn with_operation_result_ref(
        mut self,
        operation_result_ref: Option<OperationResultRef>,
    ) -> Self {
        self.operation_result_ref = operation_result_ref;
        self
    }

    fn with_pipeline_diagnostics(mut self, response: &PipelineResponse) -> Self {
        self.operation_result_ref = response.operation_result_ref.clone();
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
        self.diagnostic_facts.effect_anchor = mutation_effect_anchor(response);
        self.diagnostic_facts.replayed = response.replayed;
        self.diagnostic_facts.product_file_write_count = response
            .response_value
            .pointer("/run_summary/observed_changes/product_file_write_observed")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            as u64;
        self.mutation_refresh_context = MutationRefreshContext::from_pipeline_response(response);
    }

    fn with_post_effect_failure(mut self, code: McpPostEffectFailureCode) -> Self {
        self.post_effect_failure = Some(code);
        self
    }

    fn with_user_action_fallback(mut self, fallback: UserActionFallback) -> Self {
        self.diagnostic_facts.fallback_kind = Some(fallback.kind);
        self.extra_texts.extend(fallback.texts);
        self
    }

    fn diagnostic_facts(&self) -> ToolDiagnosticFacts {
        self.diagnostic_facts.clone()
    }
}

fn mutation_effect_anchor(response: &PipelineResponse) -> Option<String> {
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
    let binding = state.managed_agent_session_binding();
    let profile = state
        .mcp_session
        .as_ref()
        .expect("tools/call lifecycle requires a selected MCP profile")
        .selected_profile;
    finalize_mutation_output_with_refresh_for_profile(
        tool_name,
        profile,
        detail,
        output,
        |context| {
            let coordinates = binding
                .as_ref()
                .map(|binding| adapter.ensure_agent_session_binding(&context.project_id, binding))
                .transpose()?;
            adapter.refresh_authority_status(
                &context.project_id,
                &context.task_id,
                coordinates.as_ref().map(|value| value.borrowed()),
            )
        },
    )
}

#[cfg(test)]
fn finalize_mutation_output_with_refresh<F>(
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    output: ToolCallOutput,
    refresh: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: FnOnce(&MutationRefreshContext) -> Result<PipelineResponse, McpAdapterError>,
{
    finalize_mutation_output_with_refresh_for_profile(
        tool_name,
        ProtocolRegistry::production().preferred_server_profile(),
        detail,
        output,
        refresh,
    )
}

fn finalize_mutation_output_with_refresh_for_profile<F>(
    tool_name: &str,
    profile: &'static McpProtocolProfile,
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
    if response_kind_from_structured_content(&output.structured_content) != Some("result") {
        output.primary_text = bounded_mutation_compatibility_text(format!(
            "Volicord {tool_name} returned response_kind={}; inspect the authoritative result carrier.",
            response_kind_from_structured_content(&output.structured_content)
                .unwrap_or("unknown")
        ));
        return Ok(output);
    }

    let original_method_result = std::mem::take(&mut output.structured_content);
    let operation_result_ref = output.operation_result_ref.clone();
    let mut outcome = CanonicalMcpMutationOutcome::new(
        tool_name,
        profile,
        detail,
        output.diagnostic_facts.clone(),
        Some(original_method_result),
        operation_result_ref,
    );
    let Some(context) = output.mutation_refresh_context.clone() else {
        return authoritative_refresh_failure_output(&outcome);
    };
    let (receipt, next_actions) = match refresh(&context) {
        Ok(response) => match validated_authority_refresh(&context, &response) {
            Ok(refreshed) => refreshed,
            Err(()) => return authoritative_refresh_failure_output(&outcome),
        },
        Err(_) => return authoritative_refresh_failure_output(&outcome),
    };
    outcome.set_authority_refresh(receipt, next_actions);
    let authority_receipt = outcome
        .authority_receipt
        .as_ref()
        .expect("validated canonical mutation outcome requires an authority receipt");

    if let Some(code) = output.post_effect_failure {
        return mutation_post_effect_failure_output(&outcome, code);
    }
    output.primary_text = match authority_receipt_compatibility_text(tool_name, authority_receipt) {
        Ok(text) => text,
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };
    output.mutation_refresh_context = None;
    let Some(compact_method_result) = outcome.compact_method_result.clone() else {
        return mutation_post_effect_failure_output(
            &outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        );
    };
    let method_result = match detail {
        MutationDetailLevel::Full => outcome
            .exact_method_result
            .clone()
            .expect("canonical mutation outcome requires an exact result"),
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => compact_method_result,
    };
    let projected = match detail {
        MutationDetailLevel::Summary => serde_json::to_value(McpMutationSummaryResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
        }),
        MutationDetailLevel::Workflow => serde_json::to_value(McpMutationWorkflowResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
            next_actions: outcome.next_actions.clone(),
        }),
        MutationDetailLevel::Full => serde_json::to_value(McpMutationFullResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
        }),
    };
    output.structured_content = match projected {
        Ok(projected) => projected,
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };

    let result = tool_call_result_from_output_for_profile(tool_name, output.clone(), profile)?;
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
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };
    if rendered_size > response_budget {
        return mutation_response_budget_exceeded_output(&outcome);
    }
    Ok(output)
}

fn response_kind_from_structured_content(value: &Value) -> Option<&str> {
    value
        .pointer("/agent_workflow_result/base/response_kind")
        .or_else(|| value.pointer("/base/response_kind"))
        .and_then(Value::as_str)
}

fn compact_mutation_method_result(
    tool_name: &str,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let effect = compact_mutation_effect(method_result)?;
    let tool = AgentToolId::from_wire_name(tool_name)
        .map_err(|_| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    match tool {
        AgentToolId::PREPARE_EVIDENCE_CAPTURE => {
            let result: PrepareEvidenceCaptureResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpPrepareEvidenceCaptureCompactResult {
                effect,
                capture_intent_ref: result.capture_intent_ref,
                capture_intent: result.capture_intent,
                expires_at: result.expires_at,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::PREPARE_WRITE => {
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
                user_action_draft: result.user_action_draft,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::STAGE_ARTIFACT => {
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
        AgentToolId::RECORD_RUN => {
            let result: RecordRunResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let evidence_observation_refs = result
                .evidence_observations
                .iter()
                .map(|observation| StateRecordRef {
                    record_kind: StateRecordKind::EvidenceObservation,
                    record_id: RecordId::new(observation.observation_id.as_str()),
                    project_id: observation.project_id.clone(),
                    task_id: Some(observation.task_id.clone()).into(),
                    produced_at_state_version: effect.state_version.into(),
                })
                .collect();
            let evidence_producer_refs = result
                .evidence_producers
                .iter()
                .map(|producer| StateRecordRef {
                    record_kind: StateRecordKind::EvidenceProducer,
                    record_id: RecordId::new(producer.evidence_producer_id.as_str()),
                    project_id: producer.project_id.clone(),
                    task_id: Some(producer.task_id.clone()).into(),
                    produced_at_state_version: effect.state_version.into(),
                })
                .collect();
            let close_basis_anchor =
                result
                    .current_close_basis
                    .map(|basis| McpRecordRunCloseBasisAnchor {
                        close_basis_revision: basis.close_basis_revision,
                        scope_revision: basis.scope_revision,
                        source_run_ref: basis.source_run_ref,
                        evidence_summary_ref: basis.evidence_summary_ref,
                    });
            serde_json::to_value(McpRecordRunCompactResult {
                effect,
                run_ref: result.run_summary.run_ref,
                registered_artifact_refs: result.registered_artifacts,
                evidence_observation_refs,
                evidence_producer_refs,
                close_basis_anchor: close_basis_anchor.into(),
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::REQUEST_USER_ACTION => {
            compact_request_user_action_result(effect, method_result)
        }
        AgentToolId::RECONCILE_CHANGES => {
            let result: ReconcileChangesResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpReconcileChangesCompactResult {
                effect,
                unresolved_changes: result.unresolved_changes,
                resolved_changes: result.resolved_changes,
                pending_user_action_summaries: result.pending_user_action_summaries,
                rejected_resolution_requests: result.rejected_resolution_requests,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::INTAKE | AgentToolId::UPDATE_SCOPE | AgentToolId::CLOSE_TASK => {
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
    let method_result = method_result
        .get("agent_workflow_result")
        .unwrap_or(method_result);
    let base: ToolResultBase =
        serde_json::from_value(method_result["base"].clone()).map_err(McpAdapterError::Json)?;
    Ok(McpMutationEffectSummary {
        effect_kind: base.effect_kind,
        state_version: base.state_version,
        events: base.events,
    })
}

fn compact_request_user_action_result(
    effect: McpMutationEffectSummary,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let compound: McpRequestUserActionResponse =
        serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
    let agent_result = match compound.agent_workflow_result {
        volicord_types::ToolResponse::Result(result) => result,
        _ => {
            return Err(McpAdapterError::Protocol(
                "request-user-action compact projection requires a result branch".to_owned(),
            ))
        }
    };
    let resolution_summary = compound
        .user_channel_resolution
        .as_ref()
        .map(|resolution| resolution.resolution_summary.clone());
    serde_json::to_value(McpRequestUserActionCompactResult {
        effect,
        agent_workflow_result_replayed: compound.agent_workflow_result_replayed,
        user_action_request_summary: agent_result.user_action_request_summary,
        current_projection_state_version: compound.current_projection_state_version,
        current_projection_observed_at: compound.current_projection_observed_at,
        user_action_resolution_ref: compound.user_channel_resolution_ref,
        status: compound.current_status,
        resolution_summary: resolution_summary.into(),
        derived_refs: compound.derived_refs,
    })
    .map_err(McpAdapterError::Json)
}

fn validated_authority_refresh(
    context: &MutationRefreshContext,
    response: &PipelineResponse,
) -> Result<(AuthorityReceipt, Vec<NextActionSummary>), ()> {
    validate_authority_status(
        &response.response_value,
        &AuthorityStatusExpectation::new(context.project_id.clone(), context.task_id.clone()),
    )
    .map_err(|_| ())
    .map(|validated| validated.into_authority_projection())
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
        "Volicord {tool_name} refreshed Task {} at state_version {}; close_state={close_state}; next_actor={next_actor}. Inspect the authoritative result for the authority receipt.",
        receipt.task_ref.record_id.as_str(),
        receipt.state_version,
    )))
}

fn select_bounded_mutation_recovery<F>(
    outcome: &CanonicalMcpMutationOutcome,
    include_exact: bool,
    exhausted_message: &'static str,
    build_output: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: Fn(&MutationRecoveryCandidate<'_>) -> Result<ToolCallOutput, McpAdapterError>,
{
    for candidate in outcome
        .recovery_candidates(include_exact)
        .into_iter()
        .flatten()
    {
        let output = build_output(&candidate)?;
        if rendered_tool_call_output_size_for_profile(&output, outcome.profile)?
            <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        {
            return Ok(output);
        }
    }
    Err(McpAdapterError::Protocol(exhausted_message.to_owned()))
}

fn mutation_response_budget_exceeded_output(
    outcome: &CanonicalMcpMutationOutcome,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let requested_detail = outcome.requested_detail;
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
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = false;
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let receipt_preserved = candidate.authority_receipt.is_some();
            let method_result_preserved = candidate.method_result.is_some();
            let structured_content =
                serde_json::to_value(McpMutationResponseBudgetExceeded::<Value> {
                    code: McpMutationProjectionErrorCode::McpResponseBudgetExceeded,
                    tool_name: method_name,
                    requested_detail,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
                    effect_anchor: facts.effect_anchor.clone().into(),
                    operation_result_ref: outcome.operation_result_ref.clone().into(),
                    authority_receipt: candidate.authority_receipt.cloned().into(),
                    method_result: RequiredNullable::new(candidate.method_result.cloned()),
                    authoritative_refresh_succeeded: true,
                    response_projection_omitted: true,
                    status_read_required: true,
                    completion_claim_withheld: true,
                })
                .map_err(McpAdapterError::Json)?;
            let preserved_guidance = match (receipt_preserved, method_result_preserved) {
                (true, true) => {
                    "The fresh authority receipt and compact method_result are preserved"
                }
                (true, false) => {
                    "The fresh authority receipt is preserved; the compact method_result exceeded the recovery budget"
                }
                (false, true) => {
                    "The compact method_result is preserved; the fresh authority receipt exceeded the recovery budget"
                }
                (false, false) => {
                    "The fresh authority receipt and compact method_result exceeded the recovery budget"
                }
            };
            let exact_result_guidance = if outcome.operation_result_ref.is_some() {
                " Retrieve the exact historical result with volicord.get_operation_result."
            } else {
                ""
            };
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord {tool_name} reached Core (effect_applied={}, committed={}) and refreshed current authority, but the requested {requested_detail_label} projection exceeded the MCP response budget. {preserved_guidance}.{exact_result_guidance} Read volicord.status before making an authority claim. Do not retry this mutation.",
                    facts.effect_applied, facts.core_committed
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        false,
        "bounded mutation budget recovery exceeded its fixed output budget",
        build_output,
    )
}

fn mutation_post_effect_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
    code: McpPostEffectFailureCode,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let requested_detail = outcome.requested_detail;
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = false;
    let exact_result_guidance = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact historical result with volicord.get_operation_result."
    } else {
        ""
    };
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let method_result = candidate
                .method_result
                .map(|method_result| {
                    method_result.as_object().cloned().ok_or_else(|| {
                        McpAdapterError::Protocol(
                            "post-effect method_result must remain a JSON object".to_owned(),
                        )
                    })
                })
                .transpose()?;
            let structured_content = serde_json::to_value(McpMutationPostEffectFailure {
                code,
                tool_name: method_name,
                requested_detail,
                retryable: false,
                reached_core: facts.core_reached,
                committed: facts.core_committed,
                effect_kind: facts.effect_kind.into(),
                effect_applied: facts.effect_applied,
                effect_anchor: facts.effect_anchor.clone().into(),
                operation_result_ref: outcome.operation_result_ref.clone().into(),
                authority_receipt: candidate.authority_receipt.cloned().into(),
                method_result: method_result.into(),
                authoritative_refresh_succeeded: true,
                response_projection_omitted: true,
                status_read_required: true,
                completion_claim_withheld: true,
            })
            .map_err(McpAdapterError::Json)?;
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord {tool_name} observed an applied mutation effect and refreshed current authority, but post-effect adapter work could not produce the normal response projection. Do not retry this mutation; inspect {exact_result_guidance} in the authoritative result. Read volicord.status before acting."
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        true,
        "bounded post-effect recovery exceeded its fixed output budget",
        build_output,
    )
}

#[cfg(test)]
fn rendered_tool_call_output_size(output: &ToolCallOutput) -> Result<usize, McpAdapterError> {
    rendered_tool_call_output_size_for_profile(
        output,
        ProtocolRegistry::production().preferred_server_profile(),
    )
}

fn rendered_tool_call_output_size_for_profile(
    output: &ToolCallOutput,
    profile: &McpProtocolProfile,
) -> Result<usize, McpAdapterError> {
    let canonical = canonical_tool_result_from_output(output);
    let projected = canonical.project(profile).map_err(McpAdapterError::Json)?;
    serde_json::to_vec(projected.as_value())
        .map(|rendered| rendered.len())
        .map_err(McpAdapterError::Json)
}

fn authoritative_refresh_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = true;
    let exact_result_guidance = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact historical result with volicord.get_operation_result, then"
    } else {
        ""
    };
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let method_result_preserved = candidate.method_result.is_some();
            let structured_content =
                serde_json::to_value(McpAuthoritativeRefreshFailure::<Value> {
                    code: ErrorCode::McpUnavailable,
                    tool_name: method_name,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
                    effect_anchor: facts.effect_anchor.clone().into(),
                    operation_result_ref: outcome.operation_result_ref.clone().into(),
                    method_result: RequiredNullable::new(candidate.method_result.cloned()),
                    status_read_required: true,
                    completion_claim_withheld: true,
                })
                .map_err(McpAdapterError::Json)?;
            let method_result_guidance = if method_result_preserved {
                "The compact method_result is preserved"
            } else {
                "The compact method_result could not be included"
            };
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord withheld the {tool_name} success or completion claim because authoritative status refresh was unavailable. {method_result_guidance}.{exact_result_guidance} Read volicord.status before acting. Do not retry this mutation."
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        false,
        "bounded authoritative refresh recovery exceeded its fixed output budget",
        build_output,
    )
}

fn method_name_for_tool(tool_name: &str) -> Option<MethodName> {
    AgentToolId::from_wire_name(tool_name).ok()?.method()
}

fn bounded_mutation_compatibility_text(text: String) -> String {
    if text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES {
        return text;
    }
    "Volicord omitted an oversized compatibility summary without truncating it. Inspect the authoritative result carrier for the complete result."
        .to_owned()
}

fn validate_managed_stdio_session_ownership(
    adapter: &McpAdapter,
    state: &ConnectionState,
) -> Result<(), McpAdapterError> {
    if !state.managed_stdio_binding_active {
        return Ok(());
    }
    let CodexManagedBinding::Bound { correlation, .. } = &state.codex_binding else {
        return Err(McpAdapterError::Environment(
            "managed_host_session_correlation_invalid: active managed stdio binding has no host-native session correlation"
                .to_owned(),
        ));
    };
    let _connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| {
        McpAdapterError::Environment(
            "managed_stdio_session_ownership_unavailable: managed stdio connection is unavailable"
                .to_owned(),
        )
    })?;
    let _validated_correlation = correlation;

    Ok(())
}

fn stdio_diagnostic_project_id(adapter: &McpAdapter) -> Option<String> {
    adapter
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
        })
}

fn start_transport_diagnostic_session(
    adapter: &McpAdapter,
    state: &ConnectionState,
) -> Result<(), StoreError> {
    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .ok()
    .flatten();
    let host_kind = connection
        .as_ref()
        .and_then(|record| DiagnosticHostKind::from_connection_host_kind(&record.host_kind));
    let project_id = stdio_diagnostic_project_id(adapter);
    let build = crate::build_info();
    start_diagnostic_session(
        &adapter.runtime_home,
        DiagnosticSessionStart {
            session_id: &state.runtime_session_id,
            connection_id: Some(adapter.context.connection_internal_id.as_str()),
            project_id: project_id.as_deref(),
            transport: DiagnosticTransport::McpStdio,
            host_kind,
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    )
}

fn record_tools_list_metric_best_effort(
    adapter: &McpAdapter,
    state: &ConnectionState,
    serialized_bytes: u64,
) {
    if state.codex_binding.is_pending()
        || start_transport_diagnostic_session(adapter, state).is_err()
    {
        return;
    }
    let _ = record_workflow_metric_event(
        &adapter.runtime_home,
        &WorkflowMetricEvent {
            session_id: state.runtime_session_id.clone(),
            metric_kind: WorkflowMetricKind::ToolsListSerializedBytes,
            value: serialized_bytes,
            method_name: None,
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: Some(WorkflowMetricOutcome::Success),
        },
    );
}

fn record_public_method_metrics_best_effort(
    adapter: &McpAdapter,
    state: &ConnectionState,
    tool_name: Option<&str>,
    outcome: DiagnosticOutcome,
) {
    let Some(method_name) = tool_name.and_then(method_name_for_tool) else {
        return;
    };
    let outcome = workflow_metric_outcome(outcome);
    let _ = record_workflow_metric_event(
        &adapter.runtime_home,
        &WorkflowMetricEvent {
            session_id: state.runtime_session_id.clone(),
            metric_kind: WorkflowMetricKind::McpMethodCall,
            value: 1,
            method_name: Some(method_name),
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: Some(outcome),
        },
    );
    if method_name == MethodName::Status && state.status_method_call_count > 1 {
        let _ = record_workflow_metric_event(
            &adapter.runtime_home,
            &WorkflowMetricEvent {
                session_id: state.runtime_session_id.clone(),
                metric_kind: WorkflowMetricKind::StatusReread,
                value: 1,
                method_name: None,
                integration_profile: None,
                decision: None,
                observation_confidence: None,
                outcome: Some(outcome),
            },
        );
    }
}

const fn workflow_metric_outcome(outcome: DiagnosticOutcome) -> WorkflowMetricOutcome {
    match outcome {
        DiagnosticOutcome::Success => WorkflowMetricOutcome::Success,
        DiagnosticOutcome::Rejected => WorkflowMetricOutcome::Rejected,
        DiagnosticOutcome::ValidationFailure => WorkflowMetricOutcome::ValidationFailure,
        DiagnosticOutcome::ToolError => WorkflowMetricOutcome::ToolError,
        DiagnosticOutcome::TransportError => WorkflowMetricOutcome::TransportError,
        DiagnosticOutcome::Unavailable => WorkflowMetricOutcome::Unavailable,
    }
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
    if state.codex_binding.is_pending() {
        return;
    }
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let response_bytes = response
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    if start_transport_diagnostic_session(adapter, state).is_err() {
        return;
    }
    let _ = record_diagnostic_event(
        &adapter.runtime_home,
        DiagnosticEvent {
            session_id: &state.runtime_session_id,
            event_kind: DiagnosticEventKind::McpToolCall,
            tool_name,
            latency_micros: elapsed,
            request_bytes,
            response_bytes,
            validation_failure,
            core_reached: facts.core_reached,
            core_committed: facts.core_committed,
            replayed: facts.replayed,
            user_channel_kind: None,
            fallback_kind: facts.fallback_kind,
            product_file_write_count: facts.product_file_write_count,
            authoritative_refresh_failure: facts.authoritative_refresh_failure,
            outcome,
        },
    );
    record_public_method_metrics_best_effort(adapter, state, tool_name, outcome);
}

#[cfg(test)]
pub(crate) fn tool_call_result_from_output(output: ToolCallOutput) -> Value {
    tool_call_result_from_output_for_profile(
        "volicord.test",
        output,
        ProtocolRegistry::production().preferred_server_profile(),
    )
    .expect("canonical test tool result should project")
}

fn canonical_tool_result_from_output(output: &ToolCallOutput) -> CanonicalToolResult {
    let mut content = Vec::with_capacity(1 + output.extra_texts.len());
    content.push(CanonicalContent::Text(output.primary_text.clone()));
    content.extend(
        output
            .extra_texts
            .iter()
            .cloned()
            .map(CanonicalContent::Text),
    );
    CanonicalToolResult {
        metadata: None,
        content,
        structured_content: output.structured_content.clone(),
        is_error: output.is_error,
    }
}

fn tool_call_result_from_output_for_profile(
    tool_name: &str,
    output: ToolCallOutput,
    profile: &McpProtocolProfile,
) -> Result<Value, McpAdapterError> {
    if profile.tools().structured_content() && is_known_mcp_tool(tool_name) {
        validate_mcp_tool_output(tool_name, &output.structured_content)?;
    }
    canonical_tool_result_from_output(&output)
        .project(profile)
        .map(|projected| projected.into_value())
        .map_err(McpAdapterError::Json)
}

pub(crate) fn user_action_tool_output(
    adapter: &McpAdapter,
    pending_response: PipelineResponse,
) -> Result<ToolCallOutput, McpAdapterError> {
    let Some(coordinate) = pending_user_action_coordinate_from_response(&pending_response)? else {
        return ToolCallOutput::from_pipeline_response(&pending_response);
    };
    let current = current_user_action_projection_for_coordinate(adapter, &coordinate)?;
    let mut output = compound_user_action_output(&pending_response, &current)?;
    if current.status == UserActionStatus::Pending {
        output = output.with_user_action_fallback(cli_recovery_fallback());
    }
    Ok(output)
}

fn compound_user_action_output(
    pending_response: &PipelineResponse,
    current: &CurrentUserActionProjection,
) -> Result<ToolCallOutput, McpAdapterError> {
    let compound = McpRequestUserActionResponse {
        agent_workflow_result: serde_json::from_value::<RequestUserActionResponse>(
            pending_response.response_value.clone(),
        )
        .map_err(McpAdapterError::Json)?,
        agent_workflow_result_replayed: pending_response.replayed,
        current_projection_state_version: current.observed_state_version,
        current_projection_observed_at: current.observed_at.clone(),
        current_status: current.status,
        user_channel_resolution_ref: current.user_action_resolution_ref.clone().into(),
        user_channel_resolution: current.user_action_resolution.clone().into(),
        derived_refs: current.derived_refs.clone(),
    };
    let response_value = serde_json::to_value(compound).map_err(McpAdapterError::Json)?;
    let response_json = serde_json::to_string(&response_value).map_err(McpAdapterError::Json)?;
    Ok(ToolCallOutput::success(response_json)?
        .with_operation_result_ref(pending_response.operation_result_ref.clone())
        .with_pipeline_diagnostics(pending_response))
}

fn current_user_action_projection_for_coordinate(
    adapter: &McpAdapter,
    coordinate: &PendingUserActionCoordinate,
) -> Result<CurrentUserActionProjection, McpAdapterError> {
    let current = adapter
        .core
        .current_user_action_projection(&coordinate.project_id, &coordinate.user_action_request_id)
        .map_err(McpAdapterError::Core)?
        .ok_or_else(|| {
            McpAdapterError::Protocol(
                "committed user-action request disappeared during current-state reread".to_owned(),
            )
        })?;
    if current.project_id != coordinate.project_id
        || current.user_action_request_id != coordinate.user_action_request_id
    {
        return Err(McpAdapterError::Protocol(
            "current user-action projection does not match the original request".to_owned(),
        ));
    }
    Ok(current)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingUserActionCoordinate {
    project_id: ProjectId,
    user_action_request_id: UserActionRequestId,
}

fn pending_user_action_coordinate_from_response(
    response: &PipelineResponse,
) -> Result<Option<PendingUserActionCoordinate>, McpAdapterError> {
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return Ok(None);
    }
    let result = serde_json::from_value::<RequestUserActionResult>(response.response_value.clone())
        .map_err(McpAdapterError::Json)?;
    let invocation = response.verified_invocation.as_ref().ok_or_else(|| {
        McpAdapterError::Protocol(
            "successful request_user_action response omitted verified invocation facts".to_owned(),
        )
    })?;
    Ok(Some(PendingUserActionCoordinate {
        project_id: invocation.project_id.clone(),
        user_action_request_id: result.user_action_request_summary.user_action_request_id,
    }))
}

pub(crate) struct UserActionFallback {
    texts: Vec<String>,
    kind: DiagnosticFallbackKind,
}

pub(crate) fn cli_recovery_fallback() -> UserActionFallback {
    UserActionFallback {
        texts: vec![cli_inbox_fallback_text()],
        kind: DiagnosticFallbackKind::CliInbox,
    }
}

fn cli_inbox_fallback_text() -> String {
    "A pending UserAction requires the user. Open `volicord inbox` and resume the existing request after the user completes it."
        .to_owned()
}

pub(crate) fn is_known_mcp_tool(tool_name: &str) -> bool {
    AgentToolId::from_wire_name(tool_name).is_ok()
}

#[cfg(test)]
pub(crate) fn tool_execution_error_result(
    requested_tool_name: &str,
    error: &McpAdapterError,
) -> Value {
    tool_execution_error_result_for_profile(
        requested_tool_name,
        error,
        ProtocolRegistry::production().preferred_server_profile(),
    )
}

fn tool_execution_error_result_for_profile(
    requested_tool_name: &str,
    error: &McpAdapterError,
    profile: &McpProtocolProfile,
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
                        "{message}. Use {} when project selection is unclear.",
                        AgentToolId::LIST_PROJECTS.wire_name()
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
    bounded_tool_error_result(structured, profile)
}

fn bounded_tool_error_result(
    mut structured: McpToolErrorResponse,
    profile: &McpProtocolProfile,
) -> Value {
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
        let result = serialize_tool_error_result(&structured, profile);
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
        let fallback = serialize_tool_error_result(&structured, profile);
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

fn serialize_tool_error_result(
    structured: &McpToolErrorResponse,
    profile: &McpProtocolProfile,
) -> Value {
    let structured_content =
        serde_json::to_value(structured).expect("MCP tool error should serialize");
    let text = serde_json::to_string(&structured_content)
        .expect("MCP tool error compatibility text should serialize");
    if profile.tools().structured_content() {
        validate_mcp_tool_output(&structured.tool_name, &structured_content)
            .expect("bounded MCP tool error should match advertised output schema");
    }
    CanonicalToolResult {
        metadata: None,
        content: vec![CanonicalContent::Text(text)],
        structured_content,
        is_error: true,
    }
    .project(profile)
    .expect("bounded MCP tool error should project")
    .into_value()
}

pub(crate) fn json_rpc_error_for_adapter(id: Value, error: McpAdapterError) -> Value {
    let (code, message) = match error {
        McpAdapterError::UnknownTool(_) | McpAdapterError::InvalidParams { .. } => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Protocol(_)
        | McpAdapterError::Environment(_)
        | McpAdapterError::Host(_)
        | McpAdapterError::ToolExecution { .. }
        | McpAdapterError::ToolOutputSchema { .. } => (-32602, "Invalid params"),
        McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

fn invalid_request_with_diagnostic(
    id: Value,
    data: impl Into<String>,
    diagnostic: JsonRpcDiagnostic,
) -> JsonRpcFailure {
    JsonRpcFailure {
        id,
        code: -32600,
        message: "Invalid Request",
        data: Some(data.into()),
        diagnostic,
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
    use volicord_store::evidence_capture::EvidenceCaptureReceiptInsert;
    use volicord_test_support::core_fixtures::{
        CoreFixture, UpdateScopeFixture, UserActionFixture,
    };
    use volicord_types::{
        canonical_json_bare_sha256, AcceptanceCriterionId, BaselineRef, ChangeUnitId,
        ChangeUnitOperation, EvidenceAssuranceLevel, EvidenceCaptureSpec, EvidenceObservationInput,
        EvidenceProducer, EvidenceSourceKind, EvidenceTarget, JudgmentKind, RecordId,
        StateRecordKind, UtcTimestamp, EVIDENCE_CAPTURE_COMMAND_LIMITATION,
        MAX_OPERATION_RESULT_PAGE_BYTES,
    };

    fn test_agent_invocation(
        fixture: &CoreFixture,
        operation_category: OperationCategory,
    ) -> InvocationContext {
        let guard = guard_health_record(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
        )
        .expect("guard authority fixture must load");
        let session = volicord_test_support::seed_test_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
            guard
                .guard_installation
                .as_ref()
                .map(|installation| installation.guard_installation_id.as_str()),
        )
        .expect("managed Agent Session fixture must seed");
        let validated = CoreService::new(fixture.runtime_home_path())
            .validate_agent_session(
                AgentConnectionId::new(fixture.connection_id()),
                ProjectId::new(fixture.project_id()),
                session.runtime_session_id,
                session.project_session_id,
                operation_category,
            )
            .expect("managed Agent Session fixture must validate");
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            operation_category,
            "",
        )
        .with_validated_agent_session(validated)
    }

    fn committed_intake_with_receipt(
        prefix: &str,
    ) -> Result<(CoreFixture, PipelineResponse, AuthorityReceipt), Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_recovery_order",
                "idem_mcp_recovery_order",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .as_ref()
            .expect("committed intake resolves a Task");
        let status = core.status(
            fixture.status_request("req_mcp_recovery_order_status", Some(task_id.as_str())),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let receipt = serde_json::from_value(status.response_value["authority_receipt"].clone())?;
        Ok((fixture, committed, receipt))
    }

    fn committed_record_run_with_capture_producer(
        prefix: &str,
    ) -> Result<
        (
            CoreFixture,
            PipelineResponse,
            PipelineResponse,
            StateRecordRef,
        ),
        Box<dyn Error>,
    > {
        let fixture = CoreFixture::new(prefix)?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workspace = GitWorkspaceContext {
            git_common_dir: fixture
                .product_repo_path()
                .join(".git")
                .to_string_lossy()
                .into_owned(),
            worktree_id: format!("sha256:{}", "1".repeat(64)),
            branch_ref: Some("refs/heads/mcp-producer-recovery".to_owned()),
            head_sha: Some("2".repeat(40)),
            workspace_fingerprint: format!("sha256:{}", "3".repeat(64)),
        };
        let workflow_invocation = || {
            test_agent_invocation(&fixture, OperationCategory::AgentWorkflow)
                .with_git_workspace_context(workspace.clone())
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_producer_recovery_intake",
                "idem_mcp_producer_recovery_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let scope = core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_producer_recovery_scope",
                idempotency_key: "idem_mcp_producer_recovery_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Bind an actual evidence producer to compact recovery.",
            }),
            workflow_invocation(),
        )?;
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or("scope should expose the current Change Unit")?;
        let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
            ["acceptance_criterion_id"]
            .as_str()
            .ok_or("scope should expose the current acceptance criterion")?;
        let target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(criterion_id),
        };
        let prepared = core.prepare_evidence_capture(
            PrepareEvidenceCaptureRequest {
                envelope: fixture.envelope(
                    "req_mcp_producer_recovery_prepare",
                    Some("idem_mcp_producer_recovery_prepare"),
                    false,
                    Some(2),
                    Some(task_id.as_str()),
                ),
                task_id: task_id.clone(),
                change_unit_id: ChangeUnitId::new(change_unit_id),
                baseline_ref: BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                ),
                target: target.clone(),
                capture: EvidenceCaptureSpec::VerifiedCommandExecution {
                    command_sha256: "4".repeat(64),
                    command_label: "actual compact producer fixture".to_owned(),
                    expected_exit_code: RequiredNullable::null(),
                },
            },
            workflow_invocation(),
        )?;
        let capture_intent_ref: StateRecordRef =
            serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;

        let mut store = fixture.store()?;
        let intent = store
            .evidence_capture_intent_record(capture_intent_ref.record_id.as_str())?
            .expect("committed capture intent should be readable");
        let observed_outcome = json!({
            "exit_code": 0,
            "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "stdout_size_bytes": 0,
            "stderr_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "stderr_size_bytes": 0
        });
        let result_sha256 = canonical_json_bare_sha256(&observed_outcome)?;
        let source = json!({
            "connection_id": fixture.connection_id(),
            "host_invocation_id": "host_invocation_mcp_producer_recovery"
        });
        let expected_outcome: Value = serde_json::from_str(&intent.expected_outcome_json)?;
        let safe_receipt = json!({
            "contract_id": volicord_types::EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID,
            "capture_kind": "verified_command_execution",
            "capture_intent_id": capture_intent_ref.record_id,
            "input_sha256": intent.input_sha256,
            "result_sha256": result_sha256,
            "expected_outcome": expected_outcome,
            "observed_outcome": observed_outcome,
            "source": source,
            "complete": true,
            "limitations": [EVIDENCE_CAPTURE_COMMAND_LIMITATION],
            "redaction_state": "redacted",
            "observed_by_actor_source": fixture.actor_source(),
            "observed_at": intent.created_at
        });
        store.fulfill_evidence_capture_source(EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: "evidence_capture_receipt_mcp_producer_recovery"
                .to_owned(),
            evidence_capture_intent_id: capture_intent_ref.record_id.as_str().to_owned(),
            staging_handle_id: "staged_capture_receipt_mcp_producer_recovery".to_owned(),
            task_id: intent.task_id.clone(),
            capture_kind: intent.capture_kind.clone(),
            input_sha256: intent.input_sha256.clone(),
            result_sha256: result_sha256.clone(),
            expected_outcome_json: intent.expected_outcome_json.clone(),
            observed_outcome_json: serde_json::to_string(&observed_outcome)?,
            source_refs_json: "[]".to_owned(),
            observed_by_actor_source: fixture.actor_source(),
            observed_at: intent.created_at.clone(),
            limitations_json: serde_json::to_string(&json!([EVIDENCE_CAPTURE_COMMAND_LIMITATION]))?,
            safe_receipt_json: serde_json::to_string(&safe_receipt)?,
            created_at: intent.created_at.clone(),
            staging_expires_at: intent.expires_at.clone(),
            metadata_json: serde_json::to_string(&json!({ "source": source }))?,
        })?;
        drop(store);

        let mut record_request = fixture.record_run_request(
            "req_mcp_producer_recovery_record",
            "idem_mcp_producer_recovery_record",
            false,
            Some(3),
            task_id.as_str(),
            change_unit_id,
        );
        record_request.evidence_observations = vec![EvidenceObservationInput {
            target,
            source_kind: EvidenceSourceKind::ExternalTool,
            assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
            observed_by_actor_source: RequiredNullable::null(),
            tool_name: RequiredNullable::null(),
            tool_invocation_id: RequiredNullable::null(),
            tool_metadata: Map::new(),
            input_refs: vec![capture_intent_ref.clone()],
            source_refs: Vec::new(),
            output_artifact_refs: Vec::new(),
            limitations: Vec::new(),
            observed_at: UtcTimestamp::parse("2000-01-01T00:00:00Z")?,
        }];
        let recorded = core.record_run(record_request, workflow_invocation())?;
        let producer: EvidenceProducer =
            serde_json::from_value(recorded.response_value["evidence_producers"][0].clone())?;
        let producer_id = producer.evidence_producer_id.as_str().to_owned();
        let producer_row = fixture
            .store()?
            .evidence_producer_record(&producer_id)?
            .expect("record_run producer should be immediately readable");
        assert_eq!(
            producer_row.evidence_capture_intent_id,
            capture_intent_ref.record_id.as_str()
        );
        let state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .ok_or("record_run should expose its committed state version")?;
        let producer_ref = StateRecordRef {
            record_kind: StateRecordKind::EvidenceProducer,
            record_id: RecordId::new(producer_id),
            project_id: producer.project_id,
            task_id: Some(producer.task_id).into(),
            produced_at_state_version: Some(state_version).into(),
        };

        let refreshed = core.status(
            fixture.status_request("req_mcp_producer_recovery_status", Some(task_id.as_str())),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        Ok((fixture, recorded, refreshed, producer_ref))
    }

    fn receipt_with_message_padding(
        receipt: &AuthorityReceipt,
        padding_bytes: usize,
    ) -> AuthorityReceipt {
        let mut value = serde_json::to_value(receipt).expect("receipt should serialize");
        value["close_blockers"][0]["message"] = Value::String("x".repeat(padding_bytes));
        serde_json::from_value(value).expect("padded receipt should remain valid")
    }

    fn recovery_facts() -> ToolDiagnosticFacts {
        ToolDiagnosticFacts {
            core_reached: true,
            core_committed: true,
            effect_kind: Some(EffectKind::CoreCommitted),
            effect_applied: true,
            effect_anchor: Some("authority_event:event_recovery_order".to_owned()),
            ..ToolDiagnosticFacts::default()
        }
    }

    fn recovery_operation_result_ref() -> OperationResultRef {
        OperationResultRef {
            project_id: ProjectId::new("project_mcp_recovery_order"),
            source_method: MethodName::Intake,
            source_idempotency_key: IdempotencyKey::new("idem_mcp_recovery_order"),
            committed_state_version: 1,
            response_sha256: format!("sha256:{}", "a".repeat(64)),
            response_size_bytes: 1_024,
        }
    }

    fn recovery_outcome(
        tool_name: &str,
        requested_detail: MutationDetailLevel,
        authority_receipt: Option<AuthorityReceipt>,
        exact_method_result: Option<Value>,
        compact_method_result: Option<Value>,
    ) -> CanonicalMcpMutationOutcome {
        CanonicalMcpMutationOutcome {
            tool_name: tool_name.to_owned(),
            profile: ProtocolRegistry::production().preferred_server_profile(),
            requested_detail,
            facts: recovery_facts(),
            exact_method_result,
            compact_method_result,
            operation_result_ref: Some(recovery_operation_result_ref()),
            authority_receipt,
            next_actions: Vec::new(),
        }
    }

    fn assert_compact_budget(output: ToolCallOutput) -> Result<(), Box<dyn Error>> {
        assert!(!output.is_error);
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["effect_anchor"],
            "authority_event:event_recovery_order"
        );
        assert_eq!(
            output.structured_content["operation_result_ref"],
            serde_json::to_value(recovery_operation_result_ref())?
        );
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
    fn operation_result_page_keeps_escape_heavy_chunk_out_of_bounded_compatibility_text(
    ) -> Result<(), Box<dyn Error>> {
        let chunk_utf8 = "\\\"".repeat(MAX_OPERATION_RESULT_PAGE_BYTES / 2);
        assert_eq!(chunk_utf8.len(), MAX_OPERATION_RESULT_PAGE_BYTES);
        let response_value = json!({
            "base": { "response_kind": "result" },
            "start_offset_bytes": 0,
            "end_offset_bytes": MAX_OPERATION_RESULT_PAGE_BYTES,
            "chunk_utf8": chunk_utf8,
            "complete": false
        });
        let response = PipelineResponse {
            response_json: serde_json::to_string(&response_value)?,
            response_value,
            operation_result_ref: None,
            verified_invocation: None,
            resolved_task_id: None,
            replayed: false,
        };

        let output = ToolCallOutput::from_operation_result_response(&response)?;

        assert!(output.primary_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
        assert!(!output.primary_text.contains(&chunk_utf8));
        assert!(rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        Ok(())
    }

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
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);

        let committed = core.intake(request.clone(), workflow_invocation())?;
        assert!(!committed.replayed);
        let replayed = core.intake(request, workflow_invocation())?;
        assert!(replayed.replayed);
        let task_id = replayed
            .resolved_task_id
            .clone()
            .expect("replay preserves the resolved Task identity");

        let detail = mutation_detail_for_tool(AgentToolId::INTAKE, &json!({}));
        assert_eq!(detail, Some(MutationDetailLevel::Summary));
        let output = ToolCallOutput::from_pipeline_response(&replayed)?;
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            detail,
            output,
            |context| {
                assert_eq!(context.project_id.as_str(), fixture.project_id());
                assert_eq!(context.task_id, task_id);
                core.status(
                    fixture.status_request(
                        "req_mcp_mutation_replay_summary_refresh",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

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
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
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
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&intake)?,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_full_fresh_receipt_status",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
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
        output.diagnostic_facts.effect_anchor =
            Some("authority_event:event_refresh_failure".to_owned());
        output.mutation_refresh_context = Some(MutationRefreshContext {
            project_id: ProjectId::new("project_refresh_failure"),
            task_id: TaskId::new("task_refresh_failure"),
        });

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
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
            output.structured_content["effect_anchor"],
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
    fn refresh_freshness_mismatch_uses_same_non_retryable_failure_boundary(
    ) -> Result<(), Box<dyn Error>> {
        let (fixture, committed, _) =
            committed_intake_with_receipt("mcp-refresh-freshness-mismatch")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let expected_compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &committed.response_value,
        )?;
        let mut mismatched_refresh = core.status(
            fixture.status_request(
                "req_mcp_refresh_freshness_mismatch_status",
                Some(task_id.as_str()),
            ),
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        mismatched_refresh.response_value["authority_receipt"]["state_version"] = json!(999);

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
            Some(MutationDetailLevel::Summary),
            ToolCallOutput::from_pipeline_response(&committed)?,
            |_| Ok(mismatched_refresh),
        )?;

        assert!(!output.is_error);
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["method_result"], expected_compact);
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        assert!(output.diagnostic_facts.authoritative_refresh_failure);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn post_effect_adapter_failure_refreshes_authority_without_recommending_replay(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-adapter-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
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
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let expected_compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &committed.response_value,
        )?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        output.structured_content["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let output =
            output.with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
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
        assert_eq!(output.structured_content["method_result"], expected_compact);
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
            true
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn superseded_user_action_projects_latest_state_and_preserves_the_origin_effect(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-user-action-record-race")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_user_action_record_race_intake",
                "idem_mcp_user_action_record_race_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let pending_response = core.request_user_action(
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_user_action_record_race_pending",
                idempotency_key: "idem_mcp_user_action_record_race_pending",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_user_action_record_race_scope",
                idempotency_key: "idem_mcp_user_action_record_race_scope",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance state while the user action remains pending.",
            }),
            workflow_invocation(),
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);

        let output = user_action_tool_output(&adapter, pending_response)?;
        assert_eq!(output.post_effect_failure, None);
        assert_eq!(
            output.structured_content["agent_workflow_result"]["user_action_request_summary"]
                ["status"],
            "pending"
        );
        assert_eq!(output.structured_content["current_status"], "superseded");
        assert!(output.structured_content["user_channel_resolution"].is_null());
        assert_eq!(output.structured_content["derived_refs"], json!([]));
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_user_action_record_race_status",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert!(output.structured_content.get("code").is_none());
        assert_eq!(
            output.structured_content["operation_result_ref"]["source_method"],
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
        assert_eq!(
            output.structured_content["method_result"]["status"],
            "superseded"
        );
        assert_eq!(
            output.structured_content["method_result"]["agent_workflow_result_replayed"],
            false
        );
        assert_eq!(
            output.structured_content["method_result"]["derived_refs"],
            json!([])
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            3
        );
        assert!(output
            .structured_content
            .get("completion_claim_withheld")
            .is_none());
        Ok(())
    }

    #[test]
    fn mismatched_safe_summary_routes_to_closed_post_effect_recovery() -> Result<(), Box<dyn Error>>
    {
        let fixture = CoreFixture::new("mcp-noncanonical-pending-post-effect")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_noncanonical_pending_intake",
                "idem_mcp_noncanonical_pending_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let mut pending_response = core.request_user_action(
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_noncanonical_pending_action",
                idempotency_key: "idem_mcp_noncanonical_pending_action",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        let before = fixture.counts()?;
        pending_response.response_value["user_action_request_summary"]["user_action_request_id"] =
            json!("uar_not_in_trusted_projection");
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
        let error = user_action_tool_output(&adapter, pending_response.clone())
            .expect_err("mismatched public summary must fail during current-state reread");
        assert!(matches!(error, McpAdapterError::Protocol(_)));
        assert_eq!(fixture.counts()?, before);

        let output = ToolCallOutput::from_pipeline_response(&pending_response)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_noncanonical_pending_status",
                        Some(context.task_id.as_str()),
                    ),
                    test_agent_invocation(&fixture, OperationCategory::Read),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;
        let recovery: McpMutationPostEffectFailure =
            serde_json::from_value(output.structured_content.clone())?;
        assert_eq!(
            recovery.code,
            McpPostEffectFailureCode::McpPostEffectAdapterFailed
        );
        assert!(!recovery.retryable);
        assert!(recovery.reached_core);
        assert!(recovery.committed);
        assert!(recovery.effect_applied);
        assert!(recovery.response_projection_omitted);
        assert!(recovery.status_read_required);
        assert!(recovery.completion_claim_withheld);
        assert_eq!(
            output.structured_content["method_result"]["user_action_request_summary"]["status"],
            "pending"
        );
        assert!(output
            .structured_content
            .get("agent_workflow_result")
            .is_none());
        assert_eq!(fixture.counts()?, before);
        Ok(())
    }

    #[test]
    fn projection_failure_preserves_effect_facts_and_exact_method_result(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-projection-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
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
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        output.structured_content["base"]["effect_kind"] = json!("invalid_projection_fixture");
        let exact_unprojectable_result = output.structured_content.clone();
        let output = finalize_mutation_output_with_refresh(
            AgentToolId::INTAKE.wire_name(),
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
            true
        );
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn staging_refresh_failure_reports_applied_handle_as_non_retryable_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-staging-refresh-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
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
        let effect_anchor = format!("staged_artifact:{handle_id}");
        assert_eq!(
            output.diagnostic_facts.effect_anchor.as_deref(),
            Some(effect_anchor.as_str())
        );

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::STAGE_ARTIFACT.wire_name(),
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
        assert_eq!(output.structured_content["effect_anchor"], effect_anchor);
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
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
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
            test_agent_invocation(&fixture, OperationCategory::Read),
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
        refreshed.response_value["active_task"]["close_blockers"] = oversized_blockers.clone();
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
                AgentToolId::INTAKE.wire_name(),
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
            assert!(output.structured_content["effect_anchor"]
                .as_str()
                .is_some_and(|token| token.starts_with("authority_event:")));
            assert!(output.structured_content["operation_result_ref"].is_object());
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
            AgentToolId::INTAKE.wire_name(),
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
    fn post_effect_recovery_preserves_receipt_then_compact_result_then_effect_facts(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, committed, receipt) =
            committed_intake_with_receipt("mcp-post-effect-recovery-order")?;

        let compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &committed.response_value,
        )?;
        let both_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt.clone()),
            Some(committed.response_value.clone()),
            Some(compact),
        );
        let both = mutation_post_effect_failure_output(
            &both_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert!(both.structured_content["authority_receipt"].is_object());
        assert!(both.structured_content["method_result"].is_object());
        assert_compact_budget(both)?;

        let mut oversized_exact_result = committed.response_value.clone();
        oversized_exact_result["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let expected_compact = compact_mutation_method_result(
            AgentToolId::INTAKE.wire_name(),
            &oversized_exact_result,
        )?;
        let receipt_and_compact_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt.clone()),
            Some(oversized_exact_result.clone()),
            Some(expected_compact.clone()),
        );
        let receipt_and_compact = mutation_post_effect_failure_output(
            &receipt_and_compact_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert!(receipt_and_compact.structured_content["authority_receipt"].is_object());
        assert_eq!(
            receipt_and_compact.structured_content["method_result"],
            expected_compact
        );
        assert_compact_budget(receipt_and_compact)?;

        let compact_only_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(oversized_exact_result),
            Some(expected_compact),
        );
        let compact_only = mutation_post_effect_failure_output(
            &compact_only_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert_eq!(
            compact_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            compact_only.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(compact_only)?;

        let mut unprojectable_result = committed.response_value;
        unprojectable_result["base"] = Value::String("invalid".to_owned());
        unprojectable_result["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let effect_facts_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(unprojectable_result),
            None,
        );
        let effect_facts_only = mutation_post_effect_failure_output(
            &effect_facts_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert_eq!(
            effect_facts_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            effect_facts_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(effect_facts_only)?;
        Ok(())
    }

    #[test]
    fn post_effect_recovery_budget_table_uses_canonical_candidate_priority(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-post-effect-recovery-budget-table")?;
        let small_exact = json!({"projection_marker": "exact"});
        let large_exact = json!({
            "projection_marker": "exact",
            "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
        });
        let small_compact = json!({"projection_marker": "compact"});
        let large_compact = json!({
            "projection_marker": "compact",
            "padding": "한".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
        });
        let oversized_receipt =
            receipt_with_message_padding(&receipt, MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        let cases = vec![
            (
                "receipt_exact",
                receipt.clone(),
                small_exact.clone(),
                small_compact.clone(),
                true,
                Some("exact"),
            ),
            (
                "receipt_compact",
                receipt.clone(),
                large_exact.clone(),
                small_compact.clone(),
                true,
                Some("compact"),
            ),
            (
                "receipt_only",
                receipt,
                large_exact.clone(),
                large_compact.clone(),
                true,
                None,
            ),
            (
                "compact_only",
                oversized_receipt.clone(),
                large_exact.clone(),
                small_compact,
                false,
                Some("compact"),
            ),
            (
                "effect_facts_only",
                oversized_receipt,
                large_exact,
                large_compact,
                false,
                None,
            ),
        ];

        for (name, receipt, exact, compact, expect_receipt, expected_marker) in cases {
            let outcome = recovery_outcome(
                AgentToolId::INTAKE.wire_name(),
                MutationDetailLevel::Summary,
                Some(receipt),
                Some(exact),
                Some(compact),
            );
            let output = mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )?;
            assert_eq!(
                output.structured_content["authority_receipt"].is_object(),
                expect_receipt,
                "{name} receipt preservation"
            );
            assert_eq!(
                output.structured_content["method_result"]["projection_marker"].as_str(),
                expected_marker,
                "{name} method-result preservation"
            );
            if expected_marker.is_none() {
                assert!(
                    output.structured_content["method_result"].is_null(),
                    "{name} must omit the complete method result"
                );
            }
            assert_compact_budget(output)?;
        }
        Ok(())
    }

    #[test]
    fn response_budget_recovery_preserves_receipt_then_compact_result_then_effect_facts(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-response-budget-recovery-order")?;
        let small_result = json!({"effect_kind": "core_committed"});

        let both_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Full,
            Some(receipt.clone()),
            None,
            Some(small_result.clone()),
        );
        let both = mutation_response_budget_exceeded_output(&both_outcome)?;
        assert!(both.structured_content["authority_receipt"].is_object());
        assert_eq!(
            both.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(both)?;

        let receipt_only_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(&receipt, 36 * 1024)),
            None,
            Some(json!({
                "effect_kind": "core_committed",
                "padding": "x".repeat(36 * 1024),
            })),
        );
        let receipt_only = mutation_response_budget_exceeded_output(&receipt_only_outcome)?;
        assert!(receipt_only.structured_content["authority_receipt"].is_object());
        assert_eq!(
            receipt_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(receipt_only)?;

        let compact_only_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            None,
            Some(small_result),
        );
        let compact_only = mutation_response_budget_exceeded_output(&compact_only_outcome)?;
        assert_eq!(
            compact_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            compact_only.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(compact_only)?;

        let effect_facts_outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            None,
            Some(json!({
                "effect_kind": "core_committed",
                "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
            })),
        );
        let effect_facts_only = mutation_response_budget_exceeded_output(&effect_facts_outcome)?;
        assert_eq!(
            effect_facts_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            effect_facts_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(effect_facts_only)?;
        Ok(())
    }

    #[test]
    fn record_run_actual_producer_ref_survives_default_compact_and_bounded_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, recorded, refreshed, producer_ref) =
            committed_record_run_with_capture_producer("mcp-record-run-producer-recovery")?;
        let default_detail = MutationDetailLevel::default();
        assert_eq!(default_detail, MutationDetailLevel::Summary);
        let decode_producer_refs =
            |value: Value, label: &str| -> Result<Vec<StateRecordRef>, Box<dyn Error>> {
                if !value.is_array() {
                    return Err(format!("{label} did not preserve producer refs: {value}").into());
                }
                Ok(serde_json::from_value(value)?)
            };

        let normal = finalize_mutation_output_with_refresh(
            AgentToolId::RECORD_RUN.wire_name(),
            Some(default_detail),
            ToolCallOutput::from_pipeline_response(&recorded)?,
            |_| Ok(refreshed.clone()),
        )?;
        let default_refs = decode_producer_refs(
            normal.structured_content["method_result"]["evidence_producer_refs"].clone(),
            "default compact finalizer",
        )?;
        assert_eq!(default_refs, vec![producer_ref.clone()]);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(normal))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );

        let mut oversized_recorded = recorded.clone();
        oversized_recorded.response_value["run_summary"]["summary"] =
            Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES));
        oversized_recorded.response_json =
            serde_json::to_string(&oversized_recorded.response_value)?;

        let mut projection_output = ToolCallOutput::from_pipeline_response(&oversized_recorded)?;
        projection_output.post_effect_failure =
            Some(McpPostEffectFailureCode::McpResponseProjectionFailed);
        let projection_recovery = finalize_mutation_output_with_refresh(
            AgentToolId::RECORD_RUN.wire_name(),
            Some(default_detail),
            projection_output,
            |_| Ok(refreshed.clone()),
        )?;

        let budget_recovery = finalize_mutation_output_with_refresh(
            AgentToolId::RECORD_RUN.wire_name(),
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&oversized_recorded)?,
            |_| Ok(refreshed.clone()),
        )?;

        let recoveries = [
            (projection_recovery, "MCP_RESPONSE_PROJECTION_FAILED"),
            (budget_recovery, "MCP_RESPONSE_BUDGET_EXCEEDED"),
        ];
        for (recovery, expected_code) in recoveries {
            assert_eq!(recovery.structured_content["code"], expected_code);
            assert!(recovery.structured_content["authority_receipt"].is_object());
            let producer_refs = decode_producer_refs(
                recovery.structured_content["method_result"]["evidence_producer_refs"].clone(),
                expected_code,
            )?;
            assert_eq!(producer_refs, vec![producer_ref.clone()]);
            assert_eq!(recovery.structured_content["effect_applied"], true);
            assert_eq!(recovery.structured_content["committed"], true);
            assert_eq!(recovery.structured_content["retryable"], false);
            assert!(
                serde_json::to_vec(&tool_call_result_from_output(recovery))?.len()
                    <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            );
        }
        Ok(())
    }

    #[test]
    fn user_action_derived_refs_survive_compact_only_recovery_paths() -> Result<(), Box<dyn Error>>
    {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-user-action-derived-ref-recovery")?;
        let derived_ref = json!({
            "record_kind": "project_continuity_record",
            "record_id": "continuity_user_action_derived",
            "project_id": "project_mcp_recovery_order",
            "task_id": "task_mcp_recovery_order",
            "produced_at_state_version": 3
        });
        let compact = json!({
            "effect": {
                "effect_kind": "core_committed",
                "state_version": 3,
                "events": []
            },
            "agent_workflow_result_replayed": true,
            "user_action_request_summary": {
                "user_action_request_id": "user_action_request_recovery",
                "status": "pending",
                "next_actor": "user"
            },
            "user_action_resolution_ref": {
                "record_kind": "user_action_resolution",
                "record_id": "user_action_resolution_recovery",
                "project_id": "project_mcp_recovery_order",
                "task_id": "task_mcp_recovery_order",
                "produced_at_state_version": 3
            },
            "current_projection_state_version": 4,
            "current_projection_observed_at": "2026-07-13T12:00:00Z",
            "status": "resolved",
            "resolution_summary": {
                "resolution_type": "choice",
                "selected_option_id": "accept",
                "selected_option_label": "Accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted"
            },
            "derived_refs": [derived_ref.clone()]
        });
        let outcome = recovery_outcome(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(json!({
                "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES)
            })),
            Some(compact),
        );

        let outputs = [
            mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )?,
            mutation_response_budget_exceeded_output(&outcome)?,
            authoritative_refresh_failure_output(&outcome)?,
        ];
        for output in outputs {
            assert_eq!(output.structured_content["authority_receipt"], Value::Null);
            assert_eq!(
                output.structured_content["method_result"]["derived_refs"][0],
                derived_ref
            );
            assert_eq!(
                output.structured_content["method_result"]["agent_workflow_result_replayed"],
                true
            );
            assert_eq!(
                output.structured_content["method_result"]["current_projection_state_version"],
                4
            );
            assert_eq!(
                output.structured_content["method_result"]["current_projection_observed_at"],
                "2026-07-13T12:00:00Z"
            );
            assert_eq!(
                output.structured_content["method_result"]["status"],
                "resolved"
            );
            assert!(
                serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                    <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            );
        }
        Ok(())
    }

    #[test]
    fn oversized_compact_method_result_is_omitted_from_authoritative_refresh_failure(
    ) -> Result<(), Box<dyn Error>> {
        let facts = recovery_facts();
        let oversized_method_result = json!({
            "effect_kind": "core_committed",
            "oversized": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES * 2),
        });
        let outcome = recovery_outcome(
            AgentToolId::INTAKE.wire_name(),
            MutationDetailLevel::Summary,
            None,
            None,
            Some(oversized_method_result),
        );
        let mut outcome = outcome;
        outcome.facts = facts;
        let output = authoritative_refresh_failure_output(&outcome)?;
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
        let workflow_invocation =
            || test_agent_invocation(&fixture, OperationCategory::AgentWorkflow);
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
            test_agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("status exposes a close blocker");
        blocker["message"] = Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2));
        let blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] = blockers.clone();
        refreshed.response_value["active_task"]["close_blockers"] = blockers.clone();
        refreshed.response_value["close_blockers"] = blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        let output = finalize_mutation_output_with_refresh(
            AgentToolId::STAGE_ARTIFACT.wire_name(),
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
