//! Versioned, dependency-safe contracts for host-native Codex wire data.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use volicord_types::{validate_managed_host_native_session_id, AgentToolId};

const MAX_TOOL_NAME_BYTES: usize = 256;
const MAX_PRESENTATION_TEXT_BYTES: usize = 4_096;
const MAX_SAFE_PAYLOAD_BYTES: usize = 65_536;
const MAX_SAFE_PAYLOAD_DEPTH: usize = 32;
const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";

const CODEX_MCP_CONTRACT_CANONICAL: &str = concat!(
    "profile=codex-mcp-2025-06-18-v1\n",
    "required=params._meta.threadId:string\n",
    "required=params._meta.x-codex-turn-metadata.session_id:string\n",
    "required=params._meta.x-codex-turn-metadata.thread_id:string\n",
    "required=params._meta.x-codex-turn-metadata.turn_id:string\n",
    "invariant=threadId==x-codex-turn-metadata.thread_id\n",
    "unknown_fields=allowed\n",
);

const CODEX_HOOKS_CONTRACT_CANONICAL: &str = concat!(
    "profile=codex-hooks-v1\n",
    "common=session_id:string,turn_id:string,hook_event_name:string\n",
    "UserPromptSubmit=prompt:string\n",
    "PreToolUse=tool_use_id:string,tool_name:string,tool_input:bounded-json\n",
    "PostToolUse=tool_use_id:string,tool_name:string,tool_input:bounded-json,tool_response:bounded-json\n",
    "presentation=cwd?:string,transcript_path?:string\n",
    "unknown_fields=allowed\n",
);

/// Closed identifiers for reviewed host-wire contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HostContractProfileId {
    #[serde(rename = "codex-mcp-2025-06-18-v1")]
    CodexMcpTurnMetadataV1,
    #[serde(rename = "codex-hooks-v1")]
    CodexHooksV1,
}

impl HostContractProfileId {
    pub const ALL: [Self; 2] = [Self::CodexMcpTurnMetadataV1, Self::CodexHooksV1];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CodexMcpTurnMetadataV1 => "codex-mcp-2025-06-18-v1",
            Self::CodexHooksV1 => "codex-hooks-v1",
        }
    }

    pub fn contract_digest(self) -> String {
        let canonical = match self {
            Self::CodexMcpTurnMetadataV1 => CODEX_MCP_CONTRACT_CANONICAL,
            Self::CodexHooksV1 => CODEX_HOOKS_CONTRACT_CANONICAL,
        };
        format!("sha256:{:x}", Sha256::digest(canonical.as_bytes()))
    }
}

impl fmt::Display for HostContractProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One validated host-native session identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostSessionId(String);

/// One validated Codex MCP thread identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostThreadId(String);

/// One validated host-native turn identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostTurnId(String);

/// One validated command-hook tool-use identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostToolUseId(String);

macro_rules! native_id {
    ($name:ident, $label:literal) => {
        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, HostContractError> {
                let value = value.into();
                validate_managed_host_native_session_id(&value)
                    .map_err(|_| HostContractError::invalid_field($label))?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }
        }
    };
}

native_id!(HostSessionId, "session_id");
native_id!(HostThreadId, "thread_id");
native_id!(HostTurnId, "turn_id");
native_id!(HostToolUseId, "tool_use_id");

/// A validated canonical host tool name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalToolName(String);

impl CanonicalToolName {
    pub fn parse(value: impl Into<String>) -> Result<Self, HostContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_TOOL_NAME_BYTES
            && value.trim() == value
            && value.chars().all(|character| !character.is_control());
        valid
            .then_some(Self(value))
            .ok_or_else(|| HostContractError::invalid_field("tool_name"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Projects one canonical Volicord MCP tool identity into the Codex hook tool-name form.
pub fn codex_hook_tool_name(tool: AgentToolId) -> CanonicalToolName {
    let (server, method) = tool
        .wire_name()
        .split_once('.')
        .expect("canonical AgentToolId wire names contain one namespace separator");
    CanonicalToolName::parse(format!("mcp__{server}__{method}"))
        .expect("canonical AgentToolId projects to a valid Codex tool name")
}

/// A JSON value admitted only after applying the host payload bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BoundedHostValue(Value);

impl BoundedHostValue {
    pub fn parse(field: &'static str, value: &Value) -> Result<Self, HostContractError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|_| HostContractError::invalid_field(field))?
            .len();
        if bytes > MAX_SAFE_PAYLOAD_BYTES || value_depth(value, 0) > MAX_SAFE_PAYLOAD_DEPTH {
            return Err(HostContractError::payload_too_large(field));
        }
        Ok(Self(value.clone()))
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

fn value_depth(value: &Value, depth: usize) -> usize {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| value_depth(value, depth.saturating_add(1)))
            .max()
            .unwrap_or(depth),
        Value::Object(values) => values
            .values()
            .map(|value| value_depth(value, depth.saturating_add(1)))
            .max()
            .unwrap_or(depth),
        _ => depth,
    }
}

/// Correlation carried by managed Codex MCP `tools/call` metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexMcpCorrelation {
    pub session_id: HostSessionId,
    pub thread_id: HostThreadId,
    pub turn_id: HostTurnId,
}

/// Correlation carried by a Codex `UserPromptSubmit` hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexHookPromptCorrelation {
    pub session_id: HostSessionId,
    pub turn_id: HostTurnId,
}

/// Correlation carried by a Codex tool hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexHookToolCorrelation {
    pub session_id: HostSessionId,
    pub turn_id: HostTurnId,
    pub tool_use_id: HostToolUseId,
    pub tool_name: CanonicalToolName,
}

/// Source-specific host-native correlation. Variants cannot be interchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostNativeCorrelation {
    CodexMcp(CodexMcpCorrelation),
    CodexHookPrompt(CodexHookPromptCorrelation),
    CodexHookTool(CodexHookToolCorrelation),
}

impl HostNativeCorrelation {
    pub fn session_id(&self) -> &HostSessionId {
        match self {
            Self::CodexMcp(value) => &value.session_id,
            Self::CodexHookPrompt(value) => &value.session_id,
            Self::CodexHookTool(value) => &value.session_id,
        }
    }

    pub fn turn_id(&self) -> &HostTurnId {
        match self {
            Self::CodexMcp(value) => &value.turn_id,
            Self::CodexHookPrompt(value) => &value.turn_id,
            Self::CodexHookTool(value) => &value.turn_id,
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::CodexMcp(_) => "codex_mcp",
            Self::CodexHookPrompt(_) => "codex_hook_prompt",
            Self::CodexHookTool(_) => "codex_hook_tool",
        }
    }
}

/// Non-identity presentation context owned by the Codex hook contract.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CodexHookContext {
    pub cwd: Option<String>,
    pub transcript_path: Option<String>,
}

/// A parsed event from the `CodexHooksV1` wire profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodexHookEvent {
    UserPromptSubmit {
        correlation: CodexHookPromptCorrelation,
        prompt: String,
        context: CodexHookContext,
    },
    PreToolUse {
        correlation: CodexHookToolCorrelation,
        tool_input: BoundedHostValue,
        context: CodexHookContext,
    },
    PostToolUse {
        correlation: CodexHookToolCorrelation,
        tool_input: BoundedHostValue,
        tool_response: BoundedHostValue,
        context: CodexHookContext,
    },
}

impl CodexHookEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::UserPromptSubmit { .. } => "UserPromptSubmit",
            Self::PreToolUse { .. } => "PreToolUse",
            Self::PostToolUse { .. } => "PostToolUse",
        }
    }

    pub fn correlation(&self) -> HostNativeCorrelation {
        match self {
            Self::UserPromptSubmit { correlation, .. } => {
                HostNativeCorrelation::CodexHookPrompt(correlation.clone())
            }
            Self::PreToolUse { correlation, .. } | Self::PostToolUse { correlation, .. } => {
                HostNativeCorrelation::CodexHookTool(correlation.clone())
            }
        }
    }
}

/// Marker for the managed MCP turn-metadata contract.
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexMcpTurnMetadataV1;

impl CodexMcpTurnMetadataV1 {
    pub const PROFILE_ID: HostContractProfileId = HostContractProfileId::CodexMcpTurnMetadataV1;

    pub fn parse_tools_call(
        &self,
        message: &Value,
    ) -> Result<CodexMcpCorrelation, HostContractError> {
        let object = required_object(message, "message")?;
        if required_string(object, "method")? != "tools/call" {
            return Err(HostContractError::unexpected_value("method"));
        }
        let params = required_object_field(object, "params")?;
        self.parse_tools_call_params(params)
    }

    pub fn parse_tools_call_params(
        &self,
        params: &Map<String, Value>,
    ) -> Result<CodexMcpCorrelation, HostContractError> {
        let metadata = required_object_field(params, "_meta")?;
        let flat_thread = HostThreadId::parse(required_string(metadata, "threadId")?)?;
        let turn_metadata = required_object_field(metadata, CODEX_TURN_METADATA_KEY)?;
        let session_id = HostSessionId::parse(required_string(turn_metadata, "session_id")?)?;
        let thread_id = HostThreadId::parse(required_string(turn_metadata, "thread_id")?)?;
        let turn_id = HostTurnId::parse(required_string(turn_metadata, "turn_id")?)?;
        if flat_thread != thread_id {
            return Err(HostContractError::inconsistent("thread_id"));
        }
        Ok(CodexMcpCorrelation {
            session_id,
            thread_id,
            turn_id,
        })
    }
}

/// Marker for the current Codex command-hook contract.
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexHooksV1;

impl CodexHooksV1 {
    pub const PROFILE_ID: HostContractProfileId = HostContractProfileId::CodexHooksV1;

    pub fn parse(&self, payload: &Value) -> Result<CodexHookEvent, HostContractError> {
        let object = required_object(payload, "payload")?;
        let event_name = required_string(object, "hook_event_name")?;
        let session_id = HostSessionId::parse(required_string(object, "session_id")?)?;
        let turn_id = HostTurnId::parse(required_string(object, "turn_id")?)?;
        let context = CodexHookContext {
            cwd: optional_bounded_string(object, "cwd")?,
            transcript_path: optional_bounded_string(object, "transcript_path")?,
        };
        match event_name {
            "UserPromptSubmit" => Ok(CodexHookEvent::UserPromptSubmit {
                correlation: CodexHookPromptCorrelation {
                    session_id,
                    turn_id,
                },
                prompt: bounded_string(object, "prompt")?,
                context,
            }),
            "PreToolUse" | "PostToolUse" => {
                let correlation = CodexHookToolCorrelation {
                    session_id,
                    turn_id,
                    tool_use_id: HostToolUseId::parse(required_string(object, "tool_use_id")?)?,
                    tool_name: CanonicalToolName::parse(required_string(object, "tool_name")?)?,
                };
                let tool_input = BoundedHostValue::parse(
                    "tool_input",
                    object
                        .get("tool_input")
                        .ok_or_else(|| HostContractError::missing("tool_input"))?,
                )?;
                if event_name == "PreToolUse" {
                    Ok(CodexHookEvent::PreToolUse {
                        correlation,
                        tool_input,
                        context,
                    })
                } else {
                    let tool_response = BoundedHostValue::parse(
                        "tool_response",
                        object
                            .get("tool_response")
                            .ok_or_else(|| HostContractError::missing("tool_response"))?,
                    )?;
                    Ok(CodexHookEvent::PostToolUse {
                        correlation,
                        tool_input,
                        tool_response,
                        context,
                    })
                }
            }
            _ => Err(HostContractError::unexpected_value("hook_event_name")),
        }
    }
}

fn required_object<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a Map<String, Value>, HostContractError> {
    value
        .as_object()
        .ok_or_else(|| HostContractError::invalid_field(field))
}

fn required_object_field<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, HostContractError> {
    object
        .get(field)
        .ok_or_else(|| HostContractError::missing(field))?
        .as_object()
        .ok_or_else(|| HostContractError::invalid_field(field))
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a str, HostContractError> {
    object
        .get(field)
        .ok_or_else(|| HostContractError::missing(field))?
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| HostContractError::invalid_field(field))
}

fn bounded_string(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<String, HostContractError> {
    let value = required_string(object, field)?;
    if value.len() > MAX_SAFE_PAYLOAD_BYTES {
        return Err(HostContractError::payload_too_large(field));
    }
    Ok(value.to_owned())
}

fn optional_bounded_string(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<String>, HostContractError> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| HostContractError::invalid_field(field))?;
    if value.len() > MAX_PRESENTATION_TEXT_BYTES {
        return Err(HostContractError::payload_too_large(field));
    }
    Ok(Some(value.to_owned()))
}

/// Bounded host-contract parsing error. It stores only a code and field label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostContractError {
    code: HostContractErrorCode,
    field: &'static str,
}

impl HostContractError {
    const fn missing(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::MissingRequiredField,
            field,
        }
    }

    const fn invalid_field(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::InvalidField,
            field,
        }
    }

    const fn unexpected_value(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::UnexpectedValue,
            field,
        }
    }

    const fn inconsistent(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::InconsistentCorrelation,
            field,
        }
    }

    const fn payload_too_large(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::PayloadTooLarge,
            field,
        }
    }

    pub const fn code(self) -> HostContractErrorCode {
        self.code
    }

    pub const fn field(self) -> &'static str {
        self.field
    }
}

impl fmt::Display for HostContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.field)
    }
}

impl Error for HostContractError {}

/// Closed error codes for host-wire decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostContractErrorCode {
    MissingRequiredField,
    InvalidField,
    UnexpectedValue,
    InconsistentCorrelation,
    PayloadTooLarge,
}

impl HostContractErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRequiredField => "missing_required_field",
            Self::InvalidField => "invalid_field",
            Self::UnexpectedValue => "unexpected_value",
            Self::InconsistentCorrelation => "inconsistent_correlation",
            Self::PayloadTooLarge => "payload_too_large",
        }
    }
}
