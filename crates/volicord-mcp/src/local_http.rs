use crate::adapter::*;
use crate::errors::{LocalHttpError, McpAdapterError};
use crate::http::*;
use crate::local_web_consent::*;
use crate::prelude::*;
use crate::routing::*;
use crate::stdio::*;

pub const LOCAL_HTTP_MCP_ENDPOINT_PATH: &str = "/mcp";

/// Source of the bearer token used for the local HTTP MCP endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalHttpTokenSource {
    Supplied,
    TokenFile,
    Generated,
}

/// Listener policy for the local HTTP transport process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalHttpListenScope {
    NativeLoopback,
    ContainerPublishedHostLoopback,
}

/// Configuration for the token-authenticated MCP endpoint over local HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHttpServerConfig {
    pub runtime_home: PathBuf,
    pub connection_id: String,
    pub listen_addr: SocketAddr,
    pub listen_scope: LocalHttpListenScope,
    pub bearer_token: String,
    pub token_source: LocalHttpTokenSource,
    pub project_allowlist: Vec<ProjectId>,
    pub allowed_origins: Vec<String>,
}

/// Generates a bearer token from operating-system randomness.
pub fn generate_bearer_token() -> Result<String, McpAdapterError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        McpAdapterError::Environment(format!(
            "local HTTP bearer token random source unavailable: {error}"
        ))
    })?;
    Ok(hex_encode(&bytes))
}

/// Returns whether a listen address is loopback-only.
pub fn local_http_listen_is_loopback(addr: &SocketAddr) -> bool {
    matches!(
        addr.ip(),
        IpAddr::V4(address) if address == Ipv4Addr::LOCALHOST
    ) || matches!(
        addr.ip(),
        IpAddr::V6(address) if address == Ipv6Addr::LOCALHOST
    )
}

/// Returns whether a listen address is a container wildcard bind.
pub fn local_http_listen_is_container_wildcard(addr: &SocketAddr) -> bool {
    matches!(addr.ip(), IpAddr::V4(address) if address == Ipv4Addr::UNSPECIFIED)
        || matches!(addr.ip(), IpAddr::V6(address) if address == Ipv6Addr::UNSPECIFIED)
}

/// Runs the token-authenticated MCP endpoint over local HTTP until the process exits.
pub fn run_local_http_server(config: LocalHttpServerConfig) -> Result<(), LocalHttpError> {
    validate_local_http_server_config(&config)?;
    let context = McpConnectionContext::resolve(&config.runtime_home, &config.connection_id)
        .map_err(LocalHttpError::Adapter)?
        .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING)
        .with_project_allowlist(config.project_allowlist.clone());
    validate_local_http_project_allowlist(
        &config.runtime_home,
        &config.connection_id,
        &config.project_allowlist,
    )?;
    let listen_scope = config.listen_scope;
    let listener = TcpListener::bind(config.listen_addr).map_err(LocalHttpError::Io)?;
    let actual_addr = listener.local_addr().map_err(LocalHttpError::Io)?;
    validate_local_http_listen_addr(&actual_addr, listen_scope)?;
    let mut adapter = McpAdapter::new(&config.runtime_home, context);
    adapter = adapter.with_local_web_consent(LocalWebConsentContext {
        base_url: local_web_consent_base_url(actual_addr, listen_scope),
    });

    eprintln!("volicord serve listening on http://{actual_addr}{LOCAL_HTTP_MCP_ENDPOINT_PATH}");
    eprintln!("{}", local_http_transport_summary(listen_scope));
    eprintln!("authentication: bearer token required");
    eprintln!("{LOCAL_HTTP_EXPOSURE_WARNING}");
    if listen_scope == LocalHttpListenScope::ContainerPublishedHostLoopback {
        eprintln!("{LOCAL_HTTP_CONTAINER_WARNING}");
    }
    eprintln!("{TRANSPORT_DISCLOSURE_TEXT}");
    if config.token_source == LocalHttpTokenSource::Generated {
        eprintln!("generated_bearer_token: {}", config.bearer_token);
        eprintln!("{LOCAL_HTTP_GENERATED_TOKEN_WARNING}");
    }

    let mut server = LocalHttpServer::new(adapter, config);
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = stream.set_read_timeout(Some(HTTP_READ_TIMEOUT)) {
                    eprintln!("warning: failed to set HTTP read timeout: {error}");
                }
                if let Err(error) = server.handle_stream(&mut stream) {
                    eprintln!("warning: HTTP request handling failed: {error}");
                }
            }
            Err(error) => return Err(LocalHttpError::Io(error)),
        }
    }
    Ok(())
}

pub(crate) fn validate_local_http_server_config(
    config: &LocalHttpServerConfig,
) -> Result<(), LocalHttpError> {
    validate_bearer_token_text(&config.bearer_token).map_err(|message| LocalHttpError::Config {
        code: "AUTH_TOKEN_INVALID",
        message,
    })?;
    validate_local_http_listen_addr(&config.listen_addr, config.listen_scope)?;
    for origin in &config.allowed_origins {
        validate_origin_text(origin).map_err(|message| LocalHttpError::Config {
            code: "ORIGIN_INVALID",
            message,
        })?;
    }
    Ok(())
}

pub(crate) fn validate_local_http_listen_addr(
    addr: &SocketAddr,
    scope: LocalHttpListenScope,
) -> Result<(), LocalHttpError> {
    match scope {
        LocalHttpListenScope::NativeLoopback => {
            if local_http_listen_is_loopback(addr) {
                return Ok(());
            }
            Err(LocalHttpError::Config {
                code: "NONLOCAL_LISTEN_REJECTED",
                message: format!(
                    "listen address {addr} is not allowed; native local HTTP transport only supports 127.0.0.1 or [::1]"
                ),
            })
        }
        LocalHttpListenScope::ContainerPublishedHostLoopback => {
            if !local_http_listen_is_container_wildcard(addr) {
                return Err(LocalHttpError::Config {
                    code: "CONTAINER_LISTEN_REJECTED",
                    message: format!(
                        "container listen address {addr} is not allowed; use 0.0.0.0:<port> or [::]:<port> and publish only to host loopback"
                    ),
                });
            }
            if addr.port() == 0 {
                return Err(LocalHttpError::Config {
                    code: "CONTAINER_LISTEN_REJECTED",
                    message: "container listen address must use a fixed port for host-loopback publishing"
                        .to_owned(),
                });
            }
            Ok(())
        }
    }
}

fn local_http_transport_summary(scope: LocalHttpListenScope) -> &'static str {
    match scope {
        LocalHttpListenScope::NativeLoopback => {
            "transport: local-http; loopback-only MCP-over-HTTP endpoint; full MCP Streamable HTTP compatibility is not claimed"
        }
        LocalHttpListenScope::ContainerPublishedHostLoopback => {
            "transport: local-http; Docker/container MCP-over-HTTP endpoint for host-loopback publishing only; full MCP Streamable HTTP compatibility is not claimed"
        }
    }
}

pub(crate) const LOCAL_HTTP_EXPOSURE_WARNING: &str = "warning: local HTTP endpoint is for host loopback or intended Docker host-loopback publishing only; do not expose it on public interfaces or remote networks";

pub(crate) const LOCAL_HTTP_CONTAINER_WARNING: &str = "warning: --container-listen is intended only for Docker host-loopback publishing; do not publish the container port on public interfaces or remote hosts";

pub(crate) const LOCAL_HTTP_GENERATED_TOKEN_WARNING: &str = "warning: generated_bearer_token is a local secret for this serve process; keep the endpoint on host loopback or the intended Docker host-loopback boundary and do not expose it publicly";

fn local_web_consent_base_url(addr: SocketAddr, scope: LocalHttpListenScope) -> String {
    match scope {
        LocalHttpListenScope::NativeLoopback => format!("http://{addr}"),
        LocalHttpListenScope::ContainerPublishedHostLoopback => match addr {
            SocketAddr::V4(address) => format!("http://127.0.0.1:{}", address.port()),
            SocketAddr::V6(address) => format!("http://[::1]:{}", address.port()),
        },
    }
}

pub(crate) fn validate_local_http_project_allowlist(
    runtime_home: &Path,
    connection_id: &str,
    project_ids: &[ProjectId],
) -> Result<(), LocalHttpError> {
    for project_id in project_ids {
        let access =
            agent_connection_project_access(runtime_home, connection_id, project_id.as_str())
                .map_err(|error| LocalHttpError::Adapter(McpAdapterError::Store(error)))?
                .ok_or_else(|| LocalHttpError::Config {
                    code: "PROJECT_NOT_ALLOWED",
                    message: format!(
                        "connection {connection_id} is not registered for project {}",
                        project_id.as_str()
                    ),
                })?;
        if !access.connection_enabled {
            return Err(LocalHttpError::Config {
                code: "PROJECT_NOT_ALLOWED",
                message: format!("connection {connection_id} is disabled"),
            });
        }
        if !access.project_allowed {
            return Err(LocalHttpError::Config {
                code: "PROJECT_NOT_ALLOWED",
                message: format!(
                    "project {} is outside connection {connection_id} project allowlist",
                    project_id.as_str()
                ),
            });
        }
        let Some(project) = access.project else {
            return Err(LocalHttpError::Config {
                code: "PROJECT_NOT_ALLOWED",
                message: format!("project {} is not registered", project_id.as_str()),
            });
        };
        let availability = inspect_allowed_project(&ConnectionProjectRecord {
            connection_internal_id: connection_id.to_owned(),
            project_internal_id: project.project_internal_id.clone(),
            project_id: project.project_id.clone(),
            created_at: String::new(),
            project,
        });
        if !availability.available {
            return Err(LocalHttpError::Config {
                code: "PROJECT_NOT_ALLOWED",
                message: format!(
                    "project {} is unavailable: {}",
                    availability.project_id,
                    availability
                        .unavailable_reason
                        .unwrap_or_else(|| "unavailable".to_owned())
                ),
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_bearer_token_text(token: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("bearer token must not be empty".to_owned());
    }
    if token.chars().any(|character| {
        character.is_ascii_whitespace() || character == '\0' || !character.is_ascii()
    }) {
        return Err("bearer token must use visible ASCII characters without whitespace".to_owned());
    }
    Ok(())
}

pub(crate) fn validate_origin_text(origin: &str) -> Result<(), String> {
    if origin.trim().is_empty() {
        return Err("allowed origin must not be empty".to_owned());
    }
    if origin.contains('\r') || origin.contains('\n') || origin.contains('\0') {
        return Err("allowed origin must not contain control characters".to_owned());
    }
    Ok(())
}

pub(crate) struct LocalHttpServer {
    adapter: McpAdapter,
    bearer_token: String,
    allowed_origins: Vec<String>,
    sessions: HashMap<String, ConnectionState>,
}

impl LocalHttpServer {
    pub(crate) fn new(adapter: McpAdapter, config: LocalHttpServerConfig) -> Self {
        Self {
            adapter,
            bearer_token: config.bearer_token,
            allowed_origins: config.allowed_origins,
            sessions: HashMap::new(),
        }
    }

    pub(crate) fn handle_stream(&mut self, stream: &mut TcpStream) -> Result<(), LocalHttpError> {
        let response = match read_http_request(stream) {
            Ok(request) => self.handle_request(request),
            Err(response) => response,
        };
        write_http_response(stream, response).map_err(LocalHttpError::Io)
    }

    pub(crate) fn handle_request(&mut self, request: HttpRequest) -> HttpResponse {
        let origin = request.header("origin").map(str::to_owned);
        if http_request_path(&request.target) == LOCAL_WEB_CONSENT_PATH {
            return handle_local_web_consent_http_request(
                &self.adapter,
                request,
                origin.as_deref(),
            );
        }
        if let Err(response) = self.validate_origin(origin.as_deref()) {
            return response;
        }
        if request.method == "OPTIONS" {
            return self.handle_options(&request, origin.as_deref());
        }
        if let Err(response) = self.validate_auth(&request) {
            return response;
        }

        match (request.method.as_str(), request.target.as_str()) {
            ("GET", "/healthz") => HttpResponse::json(
                200,
                "OK",
                json!({
                    "status": "ok",
                    "disclosure": detective_observation_disclosure_json()
                }),
                self.cors_headers(origin.as_deref()),
            ),
            ("POST", LOCAL_HTTP_MCP_ENDPOINT_PATH) => {
                self.handle_mcp_post(request, origin.as_deref())
            }
            ("GET", LOCAL_HTTP_MCP_ENDPOINT_PATH) => structured_http_error_with_headers(
                405,
                "Method Not Allowed",
                "SSE_UNSUPPORTED",
                "server-sent event streams are not implemented by this local HTTP endpoint",
                self.cors_headers(origin.as_deref()),
            )
            .with_header("Allow", "POST, GET, DELETE, OPTIONS"),
            ("DELETE", LOCAL_HTTP_MCP_ENDPOINT_PATH) => {
                self.handle_mcp_delete(&request, origin.as_deref())
            }
            (_, LOCAL_HTTP_MCP_ENDPOINT_PATH) => structured_http_error_with_headers(
                405,
                "Method Not Allowed",
                "METHOD_NOT_ALLOWED",
                "method is not supported for the MCP endpoint",
                self.cors_headers(origin.as_deref()),
            )
            .with_header("Allow", "POST, GET, DELETE, OPTIONS"),
            _ => structured_http_error_with_headers(
                404,
                "Not Found",
                "NOT_FOUND",
                "HTTP path is not a Volicord MCP endpoint",
                self.cors_headers(origin.as_deref()),
            ),
        }
    }

    fn handle_options(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        if request.target != LOCAL_HTTP_MCP_ENDPOINT_PATH {
            return structured_http_error(
                404,
                "Not Found",
                "NOT_FOUND",
                "HTTP path is not a Volicord MCP endpoint",
            );
        }
        if origin.is_none() || self.allowed_origins.is_empty() {
            return structured_http_error(
                403,
                "Forbidden",
                "CORS_DENIED",
                "CORS is denied unless an allowed Origin is configured",
            );
        }
        HttpResponse::empty(204, "No Content", self.cors_headers(origin))
            .with_header("Access-Control-Max-Age", "600")
    }

    fn handle_mcp_post(&mut self, request: HttpRequest, origin: Option<&str>) -> HttpResponse {
        let mut cors_headers = self.cors_headers(origin);
        if !accepts_content_type(request.header("accept"), "application/json")
            || !accepts_content_type(request.header("accept"), "text/event-stream")
        {
            return structured_http_error_with_headers(
                406,
                "Not Acceptable",
                "ACCEPT_UNSUPPORTED",
                "Accept header must include application/json and text/event-stream",
                cors_headers,
            );
        }
        if !content_type_is_json(request.header("content-type")) {
            return structured_http_error_with_headers(
                415,
                "Unsupported Media Type",
                "CONTENT_TYPE_UNSUPPORTED",
                "Content-Type must be application/json",
                cors_headers,
            );
        }
        let message: Value = match serde_json::from_slice(&request.body) {
            Ok(value) => value,
            Err(error) => {
                return HttpResponse::json(
                    400,
                    "Bad Request",
                    json_rpc_error(Value::Null, -32700, "Parse error", Some(error.to_string())),
                    cors_headers,
                )
            }
        };

        if json_rpc_method(&message) == Some("initialize") {
            if request.header("mcp-session-id").is_some() {
                return structured_http_error_with_headers(
                    400,
                    "Bad Request",
                    "SESSION_ALREADY_SUPPLIED",
                    "initialize requests must not include Mcp-Session-Id",
                    cors_headers,
                );
            }
            let mut state = ConnectionState::default();
            let dispatch = dispatch_http_json_rpc_message(&self.adapter, &mut state, message);
            state.client_supports_elicitation = false;
            match dispatch {
                Ok(HttpMcpDispatch::Response(response)) => {
                    if response.get("result").is_some() {
                        match generate_http_session_id() {
                            Ok(session_id) => {
                                state.session_id = session_id.clone();
                                let _startup_observation = self
                                    .adapter
                                    .startup_session_watch_observation_best_effort(&session_id);
                                self.sessions.insert(session_id.clone(), state);
                                cors_headers.push(("Mcp-Session-Id".to_owned(), session_id));
                            }
                            Err(error) => {
                                return structured_http_error_with_headers(
                                    500,
                                    "Internal Server Error",
                                    "SESSION_GENERATION_FAILED",
                                    &error.to_string(),
                                    cors_headers,
                                )
                            }
                        }
                    }
                    HttpResponse::json(200, "OK", response, cors_headers)
                }
                Ok(HttpMcpDispatch::Accepted) => HttpResponse::empty(202, "Accepted", cors_headers),
                Ok(HttpMcpDispatch::Invalid(response)) => {
                    HttpResponse::json(400, "Bad Request", response, cors_headers)
                }
                Err(error) => structured_http_error_with_headers(
                    500,
                    "Internal Server Error",
                    "MCP_DISPATCH_FAILED",
                    &error.to_string(),
                    cors_headers,
                ),
            }
        } else {
            let Some(session_id) = request.header("mcp-session-id").map(str::to_owned) else {
                return structured_http_error_with_headers(
                    400,
                    "Bad Request",
                    "SESSION_REQUIRED",
                    "Mcp-Session-Id is required after initialize",
                    cors_headers,
                );
            };
            let Some(state) = self.sessions.get_mut(&session_id) else {
                return structured_http_error_with_headers(
                    404,
                    "Not Found",
                    "SESSION_NOT_FOUND",
                    "Mcp-Session-Id does not name an active Volicord HTTP MCP session",
                    cors_headers,
                );
            };
            match dispatch_http_json_rpc_message(&self.adapter, state, message) {
                Ok(HttpMcpDispatch::Response(response)) => {
                    HttpResponse::json(200, "OK", response, cors_headers)
                }
                Ok(HttpMcpDispatch::Accepted) => HttpResponse::empty(202, "Accepted", cors_headers),
                Ok(HttpMcpDispatch::Invalid(response)) => {
                    HttpResponse::json(400, "Bad Request", response, cors_headers)
                }
                Err(error) => structured_http_error_with_headers(
                    500,
                    "Internal Server Error",
                    "MCP_DISPATCH_FAILED",
                    &error.to_string(),
                    cors_headers,
                ),
            }
        }
    }

    fn handle_mcp_delete(&mut self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(session_id) = request.header("mcp-session-id") else {
            return structured_http_error_with_headers(
                400,
                "Bad Request",
                "SESSION_REQUIRED",
                "Mcp-Session-Id is required to delete a session",
                self.cors_headers(origin),
            );
        };
        if self.sessions.remove(session_id).is_some() {
            HttpResponse::empty(202, "Accepted", self.cors_headers(origin))
        } else {
            structured_http_error_with_headers(
                404,
                "Not Found",
                "SESSION_NOT_FOUND",
                "Mcp-Session-Id does not name an active Volicord HTTP MCP session",
                self.cors_headers(origin),
            )
        }
    }

    fn validate_origin(&self, origin: Option<&str>) -> Result<(), HttpResponse> {
        let Some(origin) = origin else {
            return Ok(());
        };
        if self.allowed_origins.iter().any(|allowed| allowed == origin) {
            return Ok(());
        }
        Err(structured_http_error(
            403,
            "Forbidden",
            "ORIGIN_NOT_ALLOWED",
            "Origin header is not in the configured allowlist",
        ))
    }

    fn validate_auth(&self, request: &HttpRequest) -> Result<(), HttpResponse> {
        let Some(header) = request.header("authorization") else {
            return Err(structured_http_error(
                401,
                "Unauthorized",
                "AUTH_REQUIRED",
                "Authorization: Bearer token is required",
            )
            .with_header("WWW-Authenticate", "Bearer"));
        };
        let Some(token) = header.strip_prefix("Bearer ") else {
            return Err(structured_http_error(
                401,
                "Unauthorized",
                "AUTH_REQUIRED",
                "Authorization header must use Bearer authentication",
            )
            .with_header("WWW-Authenticate", "Bearer"));
        };
        if constant_time_eq(token.as_bytes(), self.bearer_token.as_bytes()) {
            Ok(())
        } else {
            Err(structured_http_error(
                401,
                "Unauthorized",
                "AUTH_INVALID",
                "Bearer token is not valid for this Volicord serve process",
            )
            .with_header("WWW-Authenticate", "Bearer"))
        }
    }

    fn cors_headers(&self, origin: Option<&str>) -> Vec<(String, String)> {
        let Some(origin) = origin else {
            return Vec::new();
        };
        if !self.allowed_origins.iter().any(|allowed| allowed == origin) {
            return Vec::new();
        }
        vec![
            ("Access-Control-Allow-Origin".to_owned(), origin.to_owned()),
            ("Vary".to_owned(), "Origin".to_owned()),
            (
                "Access-Control-Allow-Methods".to_owned(),
                "POST, GET, DELETE, OPTIONS".to_owned(),
            ),
            (
                "Access-Control-Allow-Headers".to_owned(),
                "Authorization, Content-Type, Accept, MCP-Protocol-Version, Mcp-Session-Id"
                    .to_owned(),
            ),
        ]
    }
}

enum HttpMcpDispatch {
    Response(Value),
    Accepted,
    Invalid(Value),
}

fn dispatch_http_json_rpc_message(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    message: Value,
) -> Result<HttpMcpDispatch, McpAdapterError> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            let mut empty_lines = io::BufReader::new(io::empty()).lines();
            let mut sink = io::sink();
            state.client_supports_elicitation = false;
            handle_json_rpc_request(adapter, state, request, &mut empty_lines, &mut sink)
                .map(HttpMcpDispatch::Response)
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(state, notification);
            Ok(HttpMcpDispatch::Accepted)
        }
        Err(error) => Ok(HttpMcpDispatch::Invalid(json_rpc_error(
            error.id,
            error.code,
            error.message,
            error.data,
        ))),
    }
}

pub(crate) fn json_rpc_method(value: &Value) -> Option<&str> {
    value.as_object()?.get("method")?.as_str()
}

pub(crate) fn generate_http_session_id() -> Result<String, McpAdapterError> {
    generate_bearer_token().map(|token| format!("mcp_session_{token}"))
}
