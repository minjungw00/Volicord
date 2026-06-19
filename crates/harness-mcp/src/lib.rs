#![forbid(unsafe_code)]

//! Local MCP adapter for public Harness method calls.
//!
//! This crate owns only transport dispatch: it registers the documented tools,
//! decodes tool arguments into `harness-types` request structs, derives local
//! invocation facts from adapter context, and hands execution to `harness-core`.

use std::{
    error::Error,
    ffi::OsString,
    fmt,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use harness_core::{
    rejected_response, tool_error, AdapterSessionBinding, CoreBoundary, CorePipelineError,
    CoreService, InvocationContext, PipelineResponse,
};
use harness_store::{
    bootstrap::{list_surfaces, project_record, SurfaceRecord, ACTIVE_PROJECT_STATUS},
    core_pipeline::CoreProjectStore,
    StoreError,
};
use harness_types::{
    public_request_schema, AccessClass, CloseTaskRequest, ErrorCode, IntakeRequest,
    MethodAccessClass, PrepareWriteRequest, ProjectId, RecordRunRequest, RecordUserJudgmentRequest,
    RequestUserJudgmentRequest, StageArtifactRequest, StatusRequest, SurfaceId, SurfaceInstanceId,
    ToolEnvelope, ToolError, UpdateScopeRequest, VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};
use serde::Serialize;
use serde_json::{json, Map, Value};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "harness-mcp";
const DEFAULT_INVOCATION_BINDING_BASIS: &str = VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING;

/// The exact public Harness method tools exposed through MCP.
pub const PUBLIC_METHOD_TOOL_NAMES: [&str; 9] = [
    "harness.intake",
    "harness.update_scope",
    "harness.status",
    "harness.prepare_write",
    "harness.stage_artifact",
    "harness.record_run",
    "harness.request_user_judgment",
    "harness.record_user_judgment",
    "harness.close_task",
];

/// Minimal MCP adapter marker for validating dependency direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpAdapterBoundary {
    core: CoreBoundary,
}

impl McpAdapterBoundary {
    /// Creates an inert MCP adapter boundary marker.
    pub const fn new(core: CoreBoundary) -> Self {
        Self { core }
    }

    /// Returns the adapter boundary label.
    pub const fn label(self) -> &'static str {
        let _ = self.core;
        "mcp-adapter"
    }
}

/// Tool metadata returned by `tools/list`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Local adapter session facts that are not accepted from tool arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSessionContext {
    pub project_id: ProjectId,
    pub surface_id: SurfaceId,
    pub surface_instance_id: SurfaceInstanceId,
    pub invocation_binding_basis: String,
}

impl McpSessionContext {
    /// Creates a local session context for an already resolved surface binding.
    pub fn new(
        project_id: ProjectId,
        surface_id: SurfaceId,
        surface_instance_id: SurfaceInstanceId,
    ) -> Self {
        Self {
            project_id,
            surface_id,
            surface_instance_id,
            invocation_binding_basis: DEFAULT_INVOCATION_BINDING_BASIS.to_owned(),
        }
    }

    /// Replaces the controlled adapter-binding basis carried into Core.
    pub fn with_invocation_binding_basis(mut self, basis: impl Into<String>) -> Self {
        let basis = basis.into();
        self.invocation_binding_basis = controlled_invocation_binding_basis(&basis).to_owned();
        self
    }

    /// Builds session context from process environment.
    pub fn from_env<F>(runtime_home: impl AsRef<Path>, env_var: F) -> Result<Self, McpAdapterError>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let project_id = required_env_string(&env_var, "HARNESS_PROJECT_ID")?;
        let surface_id = required_env_string(&env_var, "HARNESS_SURFACE_ID")?;
        let surface_instance_id = env_string(&env_var, "HARNESS_SURFACE_INSTANCE_ID")?;

        Self::resolve(
            runtime_home,
            ProjectId::new(project_id),
            SurfaceId::new(surface_id),
            surface_instance_id.map(SurfaceInstanceId::new),
        )
    }

    /// Resolves and validates one configured MCP session binding.
    pub fn resolve(
        runtime_home: impl AsRef<Path>,
        project_id: ProjectId,
        surface_id: SurfaceId,
        configured_surface_instance_id: Option<SurfaceInstanceId>,
    ) -> Result<Self, McpAdapterError> {
        let runtime_home = runtime_home.as_ref();
        let project = project_record(runtime_home, project_id.as_str())
            .map_err(McpAdapterError::Store)?
            .ok_or_else(|| {
                McpAdapterError::Environment("configured project is not registered".to_owned())
            })?;
        if project.status != ACTIVE_PROJECT_STATUS {
            return Err(McpAdapterError::Environment(
                "configured project is not active".to_owned(),
            ));
        }

        let store =
            CoreProjectStore::open(runtime_home, &project_id).map_err(McpAdapterError::Store)?;
        let project_state = store.project_state().map_err(McpAdapterError::Store)?;
        let surfaces =
            list_surfaces(runtime_home, project_id.as_str()).map_err(McpAdapterError::Store)?;
        let candidates = surfaces
            .into_iter()
            .filter(|surface| surface.surface_id == surface_id.as_str())
            .map(valid_startup_surface)
            .collect::<Result<Vec<_>, _>>()?;

        let selected = if let Some(configured) = configured_surface_instance_id {
            candidates
                .into_iter()
                .find(|surface| surface.surface_instance_id == configured.as_str())
                .ok_or_else(|| {
                    McpAdapterError::Environment(
                        "configured surface instance is not registered".to_owned(),
                    )
                })?
        } else {
            let default_candidate =
                if project_state.default_surface_id.as_deref() == Some(surface_id.as_str()) {
                    project_state
                        .default_surface_instance_id
                        .as_deref()
                        .and_then(|default_instance| {
                            candidates
                                .iter()
                                .find(|surface| surface.surface_instance_id == default_instance)
                                .cloned()
                        })
                } else {
                    None
                };
            match default_candidate {
                Some(surface) => surface,
                None => select_single_startup_candidate(candidates)?,
            }
        };

        Ok(Self::new(
            ProjectId::new(selected.project_id),
            SurfaceId::new(selected.surface_id),
            SurfaceInstanceId::new(selected.surface_instance_id),
        ))
    }
}

/// Invocation context derived for one tool call before entering Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDerivedInvocationContext {
    pub project_id: ProjectId,
    pub surface_id: SurfaceId,
    pub surface_instance_id: SurfaceInstanceId,
    pub requested_access_class: AccessClass,
    pub invocation_binding_basis: String,
}

impl McpDerivedInvocationContext {
    fn core_invocation(&self) -> InvocationContext {
        InvocationContext {
            binding: AdapterSessionBinding::new(
                self.project_id.clone(),
                self.surface_id.clone(),
                self.surface_instance_id.clone(),
                self.invocation_binding_basis.clone(),
            ),
            requested_access_class: self.requested_access_class,
        }
    }
}

/// Local MCP adapter bound to a Core service and one local session context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAdapter {
    core: CoreService,
    session: McpSessionContext,
}

impl McpAdapter {
    /// Creates an adapter for a Runtime Home and local session context.
    pub fn new(runtime_home: impl AsRef<Path>, session: McpSessionContext) -> Self {
        Self {
            core: CoreService::new(runtime_home),
            session,
        }
    }

    /// Returns the exact public Harness method tools exposed by this adapter.
    pub fn tools(&self) -> Vec<McpToolDefinition> {
        public_method_tools()
    }

    /// Derives local invocation facts for one request envelope.
    pub fn derive_invocation_context(
        &self,
        envelope: &ToolEnvelope,
        requested_access_class: AccessClass,
    ) -> Result<McpDerivedInvocationContext, ToolError> {
        if envelope.project_id != self.session.project_id {
            return Err(local_access_mismatch_error("envelope.project_id"));
        }
        if envelope.surface_id != self.session.surface_id {
            return Err(local_access_mismatch_error("envelope.surface_id"));
        }

        Ok(McpDerivedInvocationContext {
            project_id: self.session.project_id.clone(),
            surface_id: self.session.surface_id.clone(),
            surface_instance_id: self.session.surface_instance_id.clone(),
            requested_access_class,
            invocation_binding_basis: self.session.invocation_binding_basis.clone(),
        })
    }

    /// Calls one public Harness method tool and returns Core's response.
    pub fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<PipelineResponse, McpAdapterError> {
        match tool_name {
            "harness.intake" => {
                let request: IntakeRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .intake(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.update_scope" => {
                let request: UpdateScopeRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .update_scope(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.status" => {
                let request: StatusRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .status(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.prepare_write" => {
                let request: PrepareWriteRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .prepare_write(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.stage_artifact" => {
                let request: StageArtifactRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .stage_artifact(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.record_run" => {
                let request: RecordRunRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .record_run(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.request_user_judgment" => {
                let request: RequestUserJudgmentRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .request_user_judgment(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.record_user_judgment" => {
                let request: RecordUserJudgmentRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .record_user_judgment(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            "harness.close_task" => {
                let request: CloseTaskRequest = self.decode_params(tool_name, params)?;
                let invocation = match self.typed_invocation(&request) {
                    Ok(invocation) => invocation.core_invocation(),
                    Err(error) => {
                        return rejected_pipeline_response(request.envelope.dry_run, error)
                    }
                };
                self.core
                    .close_task(request, invocation)
                    .map_err(McpAdapterError::Core)
            }
            other => Err(McpAdapterError::UnknownTool(other.to_owned())),
        }
    }

    fn decode_params<T>(&self, tool_name: &str, params: Value) -> Result<T, McpAdapterError>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value(params).map_err(|source| McpAdapterError::InvalidParams {
            tool_name: tool_name.to_owned(),
            source,
        })
    }

    fn typed_invocation<T>(&self, request: &T) -> Result<McpDerivedInvocationContext, ToolError>
    where
        T: MethodAccessClass + HasEnvelope,
    {
        self.derive_invocation_context(request.envelope(), request.requested_access_class())
    }
}

trait HasEnvelope {
    fn envelope(&self) -> &ToolEnvelope;
}

macro_rules! impl_has_envelope {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl HasEnvelope for $ty {
                fn envelope(&self) -> &ToolEnvelope {
                    &self.envelope
                }
            }
        )+
    };
}

impl_has_envelope!(
    IntakeRequest,
    UpdateScopeRequest,
    StatusRequest,
    PrepareWriteRequest,
    StageArtifactRequest,
    RecordRunRequest,
    RequestUserJudgmentRequest,
    RecordUserJudgmentRequest,
    CloseTaskRequest,
);

/// Returns the exact public Harness method tool definitions.
pub fn public_method_tools() -> Vec<McpToolDefinition> {
    PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: public_request_schema(name).expect("public method schema should exist"),
        })
        .collect()
}

/// Runs a line-delimited JSON-RPC MCP stdio loop.
pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, mut writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
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

        if let Some(responses) = handle_json_rpc_message(&adapter, message) {
            for response in responses {
                write_json_line(&mut writer, response)?;
            }
        }
    }

    writer.flush().map_err(McpAdapterError::Io)
}

/// Runs the MCP stdio adapter from process environment and stdin/stdout.
pub fn run_stdio_from_env() -> Result<(), McpAdapterError> {
    let runtime_home = resolve_runtime_home_from_env(|name| std::env::var_os(name))?;
    let session = McpSessionContext::from_env(&runtime_home, |name| std::env::var_os(name))?;
    let adapter = McpAdapter::new(runtime_home, session);
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio(adapter, stdin.lock(), stdout.lock())
}

/// Resolves the Runtime Home used by the stdio entry point.
pub fn resolve_runtime_home_from_env<F>(env_var: F) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    if let Some(path) = env_string(&env_var, "HARNESS_HOME")? {
        return Ok(PathBuf::from(path));
    }

    let home = env_string(&env_var, "HOME")?
        .ok_or_else(|| McpAdapterError::Environment("HOME is not set".to_owned()))?;
    Ok(PathBuf::from(home).join(".harness"))
}

fn valid_startup_surface(surface: SurfaceRecord) -> Result<SurfaceRecord, McpAdapterError> {
    match surface.interaction_role.as_str() {
        "agent" | "user_interaction" => (),
        _ => {
            return Err(McpAdapterError::Environment(
                "registered surface interaction role is not recognized".to_owned(),
            ));
        }
    }
    match serde_json::from_str::<Value>(&surface.capability_profile_json) {
        Ok(Value::Object(_)) => (),
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered surface capability profile is not an object".to_owned(),
            ));
        }
        Err(error) => return Err(McpAdapterError::Json(error)),
    };
    match serde_json::from_str::<Value>(&surface.metadata_json) {
        Ok(Value::Object(_)) => (),
        Ok(_) => {
            return Err(McpAdapterError::Environment(
                "registered surface metadata is not an object".to_owned(),
            ));
        }
        Err(error) => return Err(McpAdapterError::Json(error)),
    };
    match startup_authorized_access_classes(&surface.local_access_json) {
        Ok(access_classes) if !access_classes.is_empty() => Ok(surface),
        Ok(_) => Err(McpAdapterError::Environment(
            "registered surface local access grant is empty".to_owned(),
        )),
        Err(error) => Err(error),
    }
}

fn startup_authorized_access_classes(text: &str) -> Result<Vec<AccessClass>, McpAdapterError> {
    let value = serde_json::from_str::<Value>(text).map_err(McpAdapterError::Json)?;
    let object = value.as_object().ok_or_else(|| {
        McpAdapterError::Environment("registered surface local access is not an object".to_owned())
    })?;
    let mut access_classes = Vec::new();
    if let Some(value) = object.get("authorized_access_classes") {
        let values = value.as_array().ok_or_else(|| {
            McpAdapterError::Environment(
                "registered surface authorized access classes are not an array".to_owned(),
            )
        })?;
        for value in values {
            let access_class = startup_access_class(value)?;
            if !access_classes.contains(&access_class) {
                access_classes.push(access_class);
            }
        }
    } else if let Some(value) = object.get("access_class") {
        access_classes.push(startup_access_class(value)?);
    } else {
        return Err(McpAdapterError::Environment(
            "registered surface local access grant is missing".to_owned(),
        ));
    }

    if let Some(value) = object.get("access_class") {
        let fallback_access_class = startup_access_class(value)?;
        if !access_classes.contains(&fallback_access_class) {
            return Err(McpAdapterError::Environment(
                "registered surface local access fallback grant is inconsistent".to_owned(),
            ));
        }
    }

    if let Some(value) = object.get("verification_basis") {
        match value {
            Value::String(text) if !text.trim().is_empty() => (),
            _ => {
                return Err(McpAdapterError::Environment(
                    "registered surface verification basis is invalid".to_owned(),
                ));
            }
        }
    }

    Ok(access_classes)
}

fn startup_access_class(value: &Value) -> Result<AccessClass, McpAdapterError> {
    serde_json::from_value(value.clone()).map_err(McpAdapterError::Json)
}

fn select_single_startup_candidate(
    candidates: Vec<SurfaceRecord>,
) -> Result<SurfaceRecord, McpAdapterError> {
    match candidates.as_slice() {
        [candidate] => Ok(candidate.clone()),
        [] => Err(McpAdapterError::Environment(
            "configured surface has no usable registered instance".to_owned(),
        )),
        _ => Err(McpAdapterError::Environment(
            "configured surface has multiple usable registered instances".to_owned(),
        )),
    }
}

fn rejected_pipeline_response(
    dry_run: bool,
    error: ToolError,
) -> Result<PipelineResponse, McpAdapterError> {
    let response = rejected_response(dry_run, None, vec![error]);
    let response_value = serde_json::to_value(&response).map_err(McpAdapterError::Json)?;
    let response_json = serde_json::to_string(&response_value).map_err(McpAdapterError::Json)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        verified_surface: None,
        resolved_task_id: None,
        replayed: false,
    })
}

fn local_access_mismatch_error(field: &'static str) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(
        ErrorCode::LocalAccessMismatch,
        "local surface context does not match the registered surface",
        false,
        Some(details),
    )
}

fn controlled_invocation_binding_basis(value: &str) -> &'static str {
    match value.trim() {
        VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING => {
            VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING
        }
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING => VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        _ => DEFAULT_INVOCATION_BINDING_BASIS,
    }
}

fn handle_json_rpc_message(adapter: &McpAdapter, message: Value) -> Option<Vec<Value>> {
    if let Value::Array(messages) = message {
        let responses = messages
            .into_iter()
            .filter_map(|message| handle_json_rpc_request(adapter, message))
            .collect::<Vec<_>>();
        if responses.is_empty() {
            None
        } else {
            Some(responses)
        }
    } else {
        handle_json_rpc_request(adapter, message).map(|response| vec![response])
    }
}

fn handle_json_rpc_request(adapter: &McpAdapter, message: Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let is_notification = id.is_none();
    let response_id = id.unwrap_or(Value::Null);

    let Some(method) = message.get("method").and_then(Value::as_str) else {
        if is_notification {
            return None;
        }
        return Some(json_rpc_error(
            response_id,
            -32600,
            "Invalid Request",
            Some("method must be a string".to_owned()),
        ));
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    if is_notification {
        return None;
    }

    let result = match method {
        "initialize" => initialize_result(params),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": adapter.tools() }),
        "tools/call" => match call_tool_result(adapter, params) {
            Ok(result) => result,
            Err(error) => return Some(json_rpc_error_for_adapter(response_id, error)),
        },
        _ => {
            return Some(json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(method.to_owned()),
            ))
        }
    };

    Some(json!({
        "jsonrpc": "2.0",
        "id": response_id,
        "result": result
    }))
}

fn initialize_result(params: Value) -> Value {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn call_tool_result(adapter: &McpAdapter, params: Value) -> Result<Value, McpAdapterError> {
    let tool_name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
        McpAdapterError::Protocol("tools/call params.name is required".to_owned())
    })?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let response = adapter.call_tool(tool_name, arguments)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": response.response_json
            }
        ],
        "isError": false
    }))
}

fn json_rpc_error_for_adapter(id: Value, error: McpAdapterError) -> Value {
    let (code, message) = match error {
        McpAdapterError::UnknownTool(_) | McpAdapterError::InvalidParams { .. } => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Protocol(_) | McpAdapterError::Environment(_) => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<String>) -> Value {
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

fn write_json_line(writer: &mut impl Write, value: Value) -> Result<(), McpAdapterError> {
    serde_json::to_writer(&mut *writer, &value).map_err(McpAdapterError::Json)?;
    writer.write_all(b"\n").map_err(McpAdapterError::Io)
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "harness.intake" => "Start, resume, supersede, or reject an ordinary user work loop.",
        "harness.update_scope" => "Update current Task scope and Change Unit state.",
        "harness.status" => "Read the current Core status view.",
        "harness.prepare_write" => "Check one proposed product-file write against Core state.",
        "harness.stage_artifact" => "Stage safe artifact bytes or a safe notice.",
        "harness.record_run" => "Record shaping, direct, or implementation work.",
        "harness.request_user_judgment" => "Create one pending focused user-owned judgment.",
        "harness.record_user_judgment" => "Record the user's answer to one pending judgment.",
        "harness.close_task" => "Check or perform a selected Task close path.",
        _ => "Unsupported Harness method.",
    }
}

fn env_string<F>(env_var: &F, name: &str) -> Result<Option<String>, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    env_var(name)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| McpAdapterError::Environment(format!("{name} is not valid UTF-8")))
        })
        .transpose()
}

fn required_env_string<F>(env_var: &F, name: &str) -> Result<String, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    env_string(env_var, name)?.ok_or_else(|| {
        McpAdapterError::Environment(format!("{name} is required for MCP session binding"))
    })
}

/// Adapter and stdio errors that occur before or outside public Core responses.
#[derive(Debug)]
pub enum McpAdapterError {
    UnknownTool(String),
    InvalidParams {
        tool_name: String,
        source: serde_json::Error,
    },
    Core(CorePipelineError),
    Store(StoreError),
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Environment(String),
}

impl fmt::Display for McpAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(tool_name) => write!(formatter, "unknown MCP tool: {tool_name}"),
            Self::InvalidParams { tool_name, source } => {
                write!(formatter, "invalid params for {tool_name}: {source}")
            }
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Store(error) => write!(formatter, "store error: {error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Protocol(message) | Self::Environment(message) => formatter.write_str(message),
        }
    }
}

impl Error for McpAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidParams { source, .. } => Some(source),
            Self::Core(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnknownTool(_) | Self::Protocol(_) | Self::Environment(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fs,
        io::{BufReader, Cursor},
        path::{Path, PathBuf},
    };

    use harness_core::{AdapterSessionBinding, CoreBoundary, CoreService, InvocationContext};
    use harness_store::{
        bootstrap::{
            initialize_runtime_home, register_project, register_surface, ProjectRegistration,
            SurfaceRegistration, ACTIVE_PROJECT_STATUS,
        },
        core_pipeline::{CoreProjectStore, StorageEffectCounts},
    };
    use harness_test_support::TempRuntimeHome;
    use harness_types::{
        ActorKind, ChangeUnitOperation, InitialScope, RedactionState, RequestedMode, ResumePolicy,
        StatusInclude, SurfaceInteractionRole, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    };
    use serde_json::json;

    use super::*;

    const PROJECT_ID: &str = "project_mcp";
    const SURFACE_ID: &str = "surface_mcp";
    const SURFACE_INSTANCE_ID: &str = "surface_instance_mcp";

    struct TestHarness {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
    }

    impl TestHarness {
        fn new(capability_profile: Value) -> Result<Self, Box<dyn Error>> {
            Self::with_local_access(
                capability_profile,
                json!({
                    "access_class": "core_mutation",
                    "authorized_access_classes": [
                        "read_status",
                        "core_mutation",
                        "write_authorization",
                        "run_recording",
                        "artifact_registration",
                        "artifact_read"
                    ],
                    "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
                }),
            )
        }

        fn with_local_access(
            capability_profile: Value,
            local_access: Value,
        ) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("mcp")?;
            let repo_root = runtime_home.path().join("repo");
            fs::create_dir_all(&repo_root)?;
            initialize_runtime_home(runtime_home.path(), "runtime_home_mcp", "{}")?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_surface(
                runtime_home.path(),
                SurfaceRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    surface_id: SURFACE_ID.to_owned(),
                    surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                    surface_kind: "mcp_test".to_owned(),
                    interaction_role: SurfaceInteractionRole::UserInteraction,
                    display_name: Some("MCP Test Surface".to_owned()),
                    capability_profile_json: capability_profile.to_string(),
                    local_access_json: local_access.to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;

            Ok(Self {
                runtime_home_path: runtime_home.path().to_path_buf(),
                _runtime_home: runtime_home,
            })
        }

        fn adapter(&self) -> McpAdapter {
            McpAdapter::new(
                &self.runtime_home_path,
                McpSessionContext::new(
                    ProjectId::new(PROJECT_ID),
                    SurfaceId::new(SURFACE_ID),
                    SurfaceInstanceId::new(SURFACE_INSTANCE_ID),
                )
                .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING),
            )
        }

        fn core(&self) -> CoreService {
            CoreService::new(&self.runtime_home_path)
        }

        fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
            Ok(
                CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?
                    .effect_counts()?,
            )
        }
    }

    #[test]
    fn mcp_boundary_wraps_core_boundary() {
        assert_eq!(
            McpAdapterBoundary::new(CoreBoundary::new()).label(),
            "mcp-adapter"
        );
    }

    #[test]
    fn registers_exactly_documented_public_method_tools() {
        let tools = public_method_tools();
        let names = tools.iter().map(|tool| tool.name).collect::<Vec<_>>();
        let unique_names = names.iter().copied().collect::<BTreeSet<_>>();

        assert_eq!(names, PUBLIC_METHOD_TOOL_NAMES);
        assert_eq!(tools.len(), 9);
        assert_eq!(unique_names.len(), 9);
    }

    #[test]
    fn public_method_tool_schemas_are_closed_request_shapes() {
        let tools = public_method_tools();

        for tool in &tools {
            assert_eq!(
                tool.input_schema["additionalProperties"], false,
                "{} should reject additional top-level properties",
                tool.name
            );
            assert_required_fields(tool.name, &tool.input_schema);
            for forbidden in [
                "verified",
                "surface_instance_id",
                "verified_surface_context",
                "access_class",
                "capability_profile",
                "verification_basis",
            ] {
                assert!(
                    !schema_has_property(&tool.input_schema, forbidden),
                    "{} schema should not expose invocation-only field {forbidden}",
                    tool.name
                );
            }
        }
        let request_judgment = tools
            .iter()
            .find(|tool| tool.name == "harness.request_user_judgment")
            .expect("request_user_judgment tool should be registered");
        for forbidden in ["machine_action", "resolution_outcome"] {
            assert!(
                !schema_has_property(&request_judgment.input_schema, forbidden),
                "request_user_judgment schema should not expose caller option {forbidden}"
            );
        }
    }

    #[test]
    fn stdio_tools_list_exposes_exactly_public_method_tools() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({}))?;
        let adapter = harness.adapter();
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut output = Vec::new();

        run_stdio(adapter, BufReader::new(input), &mut output)?;

        let response: Value = serde_json::from_slice(&output)?;
        let names = response["result"]["tools"]
            .as_array()
            .expect("tools should be an array")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect::<Vec<_>>();
        assert_eq!(names, PUBLIC_METHOD_TOOL_NAMES);
        Ok(())
    }

    #[test]
    fn bootstrap_registered_surface_can_call_status_through_adapter() -> Result<(), Box<dyn Error>>
    {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        let adapter = harness.adapter();
        let request = status_request("req_status_adapter");

        let response = adapter.call_tool("harness.status", serde_json::to_value(request)?)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        let verified = response
            .verified_surface
            .as_ref()
            .expect("Core should return verified surface context");
        assert_eq!(verified.project_id.as_str(), PROJECT_ID);
        assert_eq!(verified.surface_id.as_str(), SURFACE_ID);
        assert_eq!(verified.surface_instance_id.as_str(), SURFACE_INSTANCE_ID);
        assert_eq!(
            verified.verification_basis,
            "local_admin_registration:test_fixture_binding"
        );
        Ok(())
    }

    #[test]
    fn adapter_preserves_structured_core_store_rejection() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        fs::remove_file(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
        let adapter = harness.adapter();

        let response = adapter.call_tool(
            "harness.status",
            serde_json::to_value(status_request("req_status_missing_db_adapter"))?,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "MCP_UNAVAILABLE"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["store_failure_category"],
            "project_state_database_missing"
        );
        let body = &response.response_json;
        let runtime_home = harness.runtime_home_path.to_string_lossy();
        assert!(!body.contains(runtime_home.as_ref()));
        assert!(!body.contains("state.sqlite"));
        assert!(!body.contains("SELECT "));
        Ok(())
    }

    #[test]
    fn adapter_and_direct_core_status_have_equivalent_response_meaning(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        let task_id = create_task_with_change_unit(&harness, "status_equiv")?;
        let adapter = harness.adapter();
        let mut request = status_request("req_status_equiv");
        request.envelope.task_id = Some(harness_types::TaskId::new(&task_id)).into();
        let direct = harness
            .core()
            .status(request.clone(), invocation(AccessClass::ReadStatus))?;
        let adapted = adapter.call_tool("harness.status", serde_json::to_value(request)?)?;

        assert_eq!(adapted.response_value, direct.response_value);
        assert_eq!(adapted.response_json, direct.response_json);
        assert_eq!(adapted.response_value["close_state"], "blocked");
        assert!(adapted.response_value["close_blockers"]
            .as_array()
            .expect("close blockers")
            .iter()
            .any(|blocker| blocker["code"] == "missing_current_close_basis"));
        assert_eq!(
            adapted.response_value["guarantee_display"]["level"],
            "cooperative"
        );
        Ok(())
    }

    #[test]
    fn adapter_and_direct_core_intake_dry_run_have_equivalent_response_meaning(
    ) -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "core_mutation",
            "supported_access_classes": ["core_mutation"]
        }))?;
        let adapter = harness.adapter();
        let request = intake_request("req_intake_equiv", true, None);
        let direct = harness
            .core()
            .intake(request.clone(), invocation(AccessClass::CoreMutation))?;
        let adapted = adapter.call_tool("harness.intake", serde_json::to_value(request)?)?;

        assert_eq!(
            normalize_dry_run_required_ref_ids(adapted.response_value),
            normalize_dry_run_required_ref_ids(direct.response_value)
        );
        Ok(())
    }

    #[test]
    fn adapter_derives_access_class_per_method_call() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "read_status",
            "supported_access_classes": ["read_status"]
        }))?;
        let adapter = harness.adapter();
        let response = adapter.call_tool(
            "harness.status",
            serde_json::to_value(status_request("req_status_derived_read"))?,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        Ok(())
    }

    #[test]
    fn env_requested_access_class_cannot_elevate_registered_grant() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::with_local_access(
            json!({
                "access_class": "core_mutation",
                "supported_access_classes": ["core_mutation"]
            }),
            json!({
                "access_class": "read_status",
                "authorized_access_classes": ["read_status"],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
        )?;
        let session = McpSessionContext::from_env(&harness.runtime_home_path, |name| match name {
            "HARNESS_PROJECT_ID" => Some(OsString::from(PROJECT_ID)),
            "HARNESS_SURFACE_ID" => Some(OsString::from(SURFACE_ID)),
            "HARNESS_ACCESS_CLASS" => Some(OsString::from("core_mutation")),
            "HARNESS_SURFACE_INSTANCE_ID" => Some(OsString::from(SURFACE_INSTANCE_ID)),
            _ => None,
        })?;
        let adapter = McpAdapter::new(&harness.runtime_home_path, session);

        let response = adapter.call_tool(
            "harness.intake",
            serde_json::to_value(intake_request("req_env_elevate", true, None))?,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert!(response.verified_surface.is_none());
        Ok(())
    }

    #[test]
    fn adapter_does_not_bypass_artifact_registration_capability() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"]
        }))?;
        let task_id = create_task(&harness, "req_stage_task", "idem_stage_task")?;
        let adapter = harness.adapter();

        let response = adapter.call_tool(
            "harness.stage_artifact",
            serde_json::to_value(stage_artifact_request(
                "req_stage_missing_capability",
                &task_id,
            ))?,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "CAPABILITY_INSUFFICIENT"
        );
        Ok(())
    }

    #[test]
    fn caller_submitted_invocation_context_is_rejected_before_core() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"],
            "manual_artifact_attachment_supported": true
        }))?;
        let task_id = create_task(&harness, "req_stage_task_forged", "idem_stage_task_forged")?;
        let adapter = harness.adapter();
        let mut params =
            serde_json::to_value(stage_artifact_request("req_stage_forged", &task_id))?;
        params["surface_instance_id"] = json!("forged_surface_instance");
        let before = harness.counts()?;

        let error = adapter
            .call_tool("harness.stage_artifact", params)
            .expect_err("forged invocation context should be invalid params");

        assert!(matches!(error, McpAdapterError::InvalidParams { .. }));
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn missing_required_nullable_param_is_rejected_before_core() -> Result<(), Box<dyn Error>> {
        let harness = TestHarness::new(json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"],
            "manual_artifact_attachment_supported": true
        }))?;
        let task_id = create_task(
            &harness,
            "req_stage_missing_required_task",
            "idem_stage_missing_required_task",
        )?;
        let adapter = harness.adapter();
        let mut params = serde_json::to_value(stage_artifact_request(
            "req_stage_missing_required_nullable",
            &task_id,
        ))?;
        params
            .as_object_mut()
            .expect("params should be an object")
            .remove("expected_sha256");
        let before = harness.counts()?;

        let error = adapter
            .call_tool("harness.stage_artifact", params)
            .expect_err("missing required nullable field should be invalid params");

        assert!(matches!(error, McpAdapterError::InvalidParams { .. }));
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn replay_hash_uses_decoded_typed_request_not_raw_property_order() -> Result<(), Box<dyn Error>>
    {
        let harness = TestHarness::new(json!({
            "access_class": "core_mutation",
            "supported_access_classes": ["core_mutation"]
        }))?;
        let adapter = harness.adapter();
        let first = serde_json::to_value(intake_request(
            "req_intake_order_replay",
            false,
            Some("idem_intake_order_replay"),
        ))?;
        let second: Value = serde_json::from_str(
            r#"{
                "initial_context_refs": [],
                "initial_scope": {
                    "acceptance_criteria": ["Adapter calls return Core responses."],
                    "non_goals": ["Changing Core method semantics."],
                    "boundary": "Local MCP adapter behavior."
                },
                "resume_policy": "create_new",
                "requested_mode": "work",
                "plain_language_request": "Prepare a local MCP adapter test task.",
                "envelope": {
                    "locale": "en-US",
                    "dry_run": false,
                    "expected_state_version": 0,
                    "idempotency_key": "idem_intake_order_replay",
                    "request_id": "req_intake_order_replay",
                    "surface_id": "surface_mcp",
                    "actor_kind": "agent",
                    "task_id": null,
                    "project_id": "project_mcp"
                }
            }"#,
        )?;

        let first = adapter.call_tool("harness.intake", first)?;
        let after_first = harness.counts()?;
        let second = adapter.call_tool("harness.intake", second)?;

        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    fn assert_required_fields(tool_name: &str, schema: &Value) {
        let required = schema["required"]
            .as_array()
            .expect("schema should have required fields")
            .iter()
            .map(|value| value.as_str().expect("required field name"))
            .collect::<BTreeSet<_>>();
        let expected = expected_required_fields(tool_name)
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(required, expected, "{tool_name} required fields");
    }

    fn expected_required_fields(tool_name: &str) -> &'static [&'static str] {
        match tool_name {
            "harness.intake" => &[
                "envelope",
                "plain_language_request",
                "requested_mode",
                "resume_policy",
                "initial_scope",
                "initial_context_refs",
            ],
            "harness.update_scope" => &[
                "envelope",
                "task_id",
                "goal_summary",
                "scope_update",
                "scope_boundary",
                "non_goals",
                "acceptance_criteria",
                "autonomy_boundary",
                "baseline_ref",
                "change_unit",
                "related_scope_decision_refs",
            ],
            "harness.status" => &["envelope", "include"],
            "harness.prepare_write" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "intended_operation",
                "intended_paths",
                "product_file_write_intended",
                "sensitive_categories",
                "baseline_ref",
            ],
            "harness.stage_artifact" => &[
                "envelope",
                "task_id",
                "display_name",
                "content_type",
                "redaction_state",
                "safe_bytes_or_notice",
                "expected_sha256",
                "expected_size_bytes",
                "relation_hint",
            ],
            "harness.record_run" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "kind",
                "run_id",
                "baseline_ref",
                "write_authorization_id",
                "summary",
                "observed_changes",
                "artifact_inputs",
                "evidence_updates",
                "close_assessment",
            ],
            "harness.request_user_judgment" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "judgment_kind",
                "presentation",
                "question",
                "context",
                "affected_refs",
                "required_for",
                "expires_at",
            ],
            "harness.record_user_judgment" => &[
                "envelope",
                "user_judgment_id",
                "judgment_kind",
                "selected_option_id",
                "answer",
                "note",
                "accepted_risks",
            ],
            "harness.close_task" => &[
                "envelope",
                "task_id",
                "intent",
                "close_reason",
                "superseding_task_id",
                "user_note",
            ],
            other => panic!("unexpected tool name: {other}"),
        }
    }

    fn schema_has_property(schema: &Value, property_name: &str) -> bool {
        match schema {
            Value::Object(object) => {
                object
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|properties| properties.contains_key(property_name))
                    || object
                        .values()
                        .any(|child| schema_has_property(child, property_name))
            }
            Value::Array(items) => items
                .iter()
                .any(|child| schema_has_property(child, property_name)),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
        }
    }

    fn create_task(
        harness: &TestHarness,
        request_id: &str,
        idempotency_key: &str,
    ) -> Result<String, Box<dyn Error>> {
        let response = harness.core().intake(
            intake_request(request_id, false, Some(idempotency_key)),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");
        Ok(response.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task id should be present")
            .to_owned())
    }

    fn create_task_with_change_unit(
        harness: &TestHarness,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let task_id = create_task(
            harness,
            &format!("req_{suffix}_task"),
            &format!("idem_{suffix}_task"),
        )?;
        let response = harness.core().update_scope(
            update_scope_request(
                &format!("req_{suffix}_scope"),
                &format!("idem_{suffix}_scope"),
                &task_id,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert!(response.response_value["change_unit_ref"].is_object());
        Ok(task_id)
    }

    fn normalize_dry_run_required_ref_ids(mut value: Value) -> Value {
        let Some(next_actions) = value["dry_run_summary"]["next_actions"].as_array_mut() else {
            return value;
        };
        for action in next_actions {
            let Some(required_refs) = action["required_refs"].as_array_mut() else {
                continue;
            };
            for required_ref in required_refs {
                if required_ref["record_id"].is_string() {
                    required_ref["record_id"] = json!("<opaque-record-id>");
                }
                if required_ref["task_id"].is_string() {
                    required_ref["task_id"] = json!("<opaque-task-id>");
                }
            }
        }
        value
    }

    fn status_request(request_id: &str) -> StatusRequest {
        StatusRequest {
            envelope: envelope(request_id, None, false, None, None),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_authority: false,
                evidence: false,
                close: true,
                guarantees: true,
            },
        }
    }

    fn intake_request(
        request_id: &str,
        dry_run: bool,
        idempotency_key: Option<&str>,
    ) -> IntakeRequest {
        IntakeRequest {
            envelope: envelope(request_id, idempotency_key, dry_run, Some(0), None),
            plain_language_request: "Prepare a local MCP adapter test task.".to_owned(),
            requested_mode: RequestedMode::Work,
            resume_policy: ResumePolicy::CreateNew,
            initial_scope: InitialScope {
                boundary: "Local MCP adapter behavior.".to_owned(),
                non_goals: vec!["Changing Core method semantics.".to_owned()],
                acceptance_criteria: vec!["Adapter calls return Core responses.".to_owned()],
            },
            initial_context_refs: Vec::new(),
        }
    }

    fn update_scope_request(
        request_id: &str,
        idempotency_key: &str,
        task_id: &str,
    ) -> UpdateScopeRequest {
        let fields = match json!({
            "scope_summary": "MCP adapter status parity Change Unit.",
            "affected_paths": ["src/mcp_adapter.rs"]
        }) {
            Value::Object(object) => object,
            _ => unreachable!("literal object"),
        };
        UpdateScopeRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                false,
                Some(1),
                Some(task_id),
            ),
            task_id: harness_types::TaskId::new(task_id),
            goal_summary: None.into(),
            scope_update: None.into(),
            scope_boundary: Some("MCP adapter status parity scope.".to_owned()).into(),
            non_goals: None.into(),
            acceptance_criteria: None.into(),
            autonomy_boundary: None.into(),
            baseline_ref: Some(harness_types::BaselineRef::new("baseline_mcp")).into(),
            change_unit: harness_types::ChangeUnitUpdate {
                operation: ChangeUnitOperation::CreateCurrent,
                fields,
            },
            related_scope_decision_refs: Vec::new(),
        }
    }

    fn stage_artifact_request(request_id: &str, task_id: &str) -> StageArtifactRequest {
        StageArtifactRequest {
            envelope: envelope(request_id, None, false, None, Some(task_id)),
            task_id: harness_types::TaskId::new(task_id),
            display_name: "adapter-note.txt".to_owned(),
            content_type: "text/plain".to_owned(),
            redaction_state: RedactionState::None,
            safe_bytes_or_notice: "Adapter staging test note.".to_owned(),
            expected_sha256: None.into(),
            expected_size_bytes: None.into(),
            relation_hint: Some("adapter_test".to_owned()).into(),
        }
    }

    fn envelope(
        request_id: &str,
        idempotency_key: Option<&str>,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: Option<&str>,
    ) -> ToolEnvelope {
        ToolEnvelope {
            project_id: ProjectId::new(PROJECT_ID),
            task_id: task_id.map(harness_types::TaskId::new).into(),
            actor_kind: ActorKind::Agent,
            surface_id: SurfaceId::new(SURFACE_ID),
            request_id: harness_types::RequestId::new(request_id),
            idempotency_key: idempotency_key
                .map(harness_types::IdempotencyKey::new)
                .into(),
            expected_state_version: expected_state_version.into(),
            dry_run,
            locale: Some("en-US".to_owned()).into(),
        }
    }

    fn invocation(access_class: AccessClass) -> InvocationContext {
        InvocationContext {
            binding: AdapterSessionBinding::new(
                ProjectId::new(PROJECT_ID),
                SurfaceId::new(SURFACE_ID),
                SurfaceInstanceId::new(SURFACE_INSTANCE_ID),
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
            requested_access_class: access_class,
        }
    }

    #[test]
    fn runtime_home_env_resolution_uses_harness_home_then_home() -> Result<(), Box<dyn Error>> {
        let explicit = resolve_runtime_home_from_env(|name| {
            if name == "HARNESS_HOME" {
                Some(OsString::from("/tmp/harness-explicit"))
            } else {
                None
            }
        })?;
        assert_eq!(explicit, Path::new("/tmp/harness-explicit"));

        let default = resolve_runtime_home_from_env(|name| {
            if name == "HOME" {
                Some(OsString::from("/tmp/harness-home"))
            } else {
                None
            }
        })?;
        assert_eq!(default, Path::new("/tmp/harness-home/.harness"));
        Ok(())
    }
}
