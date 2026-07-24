//! Dependency-safe contracts for host-native Codex wire data.

use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use volicord_types::{validate_managed_host_native_session_id, AgentToolId};

const MAX_TOOL_NAME_BYTES: usize = 256;
const MAX_HOST_CALLABLE_NAME_BYTES: usize = 64;
const CALLABLE_NAME_HASH_LEN: usize = 12;
const MCP_TOOL_NAME_DELIMITER: &str = "__";
const CODEX_MCP_TOOL_PREFIX: &str = "mcp__";
const CODEX_GUARD_HOST_TOOLS: [&str; 4] = ["Bash", "apply_patch", "Edit", "Write"];
const MAX_PRESENTATION_TEXT_BYTES: usize = 4_096;
const MAX_SAFE_PAYLOAD_BYTES: usize = 65_536;
const MAX_SAFE_PAYLOAD_DEPTH: usize = 32;
const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";

const CODEX_MCP_CONTRACT_CANONICAL: &str = concat!(
    "profile=codex-mcp-turn-metadata\n",
    "required=params._meta.threadId:string\n",
    "required=params._meta.x-codex-turn-metadata.session_id:string\n",
    "required=params._meta.x-codex-turn-metadata.thread_id:string\n",
    "required=params._meta.x-codex-turn-metadata.turn_id:string\n",
    "invariant=threadId==x-codex-turn-metadata.thread_id\n",
    "unknown_fields=allowed\n",
);

const CODEX_HOOKS_CONTRACT_CANONICAL: &str = concat!(
    "profile=codex-command-hooks\n",
    "common=session_id:string,turn_id:string,hook_event_name:string\n",
    "UserPromptSubmit=prompt:string\n",
    "PreToolUse=tool_use_id:string,tool_name:string,tool_input:bounded-json\n",
    "PostToolUse=tool_use_id:string,tool_name:string,tool_input:bounded-json,tool_response:bounded-json\n",
    "tool-matcher=union(host-tools,semantic-mcp-routing)\n",
    "host-tools=Bash,apply_patch,Edit,Write\n",
    "mcp-server-namespace=mcp__<normalized-server-key>__.*\n",
    "mcp-routing-fallback=exact-canonical-callables\n",
    "presentation=cwd?:string,transcript_path?:string\n",
    "unknown_fields=allowed\n",
);

const CODEX_MCP_CALLABLE_NAMES_CONTRACT_CANONICAL: &str = concat!(
    "profile=codex-mcp-callable-names\n",
    "source=mcp-server-key,mcp-raw-tool-name\n",
    "namespace=mcp__<normalized-server-key>\n",
    "callable=<normalized-complete-raw-tool-name>\n",
    "separator=__\n",
    "allowed=ascii-alphanumeric-or-underscore\n",
    "maximum_bytes=64\n",
    "overflow=sha1-source-identity-suffix-12\n",
    "collision=reject-catalog-construction\n",
);

/// Closed identifiers for reviewed host-wire contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HostContractProfileId {
    #[serde(rename = "codex-mcp-turn-metadata")]
    CodexMcpTurnMetadata,
    #[serde(rename = "codex-command-hooks")]
    CodexCommandHooks,
    #[serde(rename = "codex-mcp-callable-names")]
    CodexMcpCallableNames,
}

impl HostContractProfileId {
    pub const ALL: [Self; 3] = [
        Self::CodexMcpTurnMetadata,
        Self::CodexCommandHooks,
        Self::CodexMcpCallableNames,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CodexMcpTurnMetadata => "codex-mcp-turn-metadata",
            Self::CodexCommandHooks => "codex-command-hooks",
            Self::CodexMcpCallableNames => "codex-mcp-callable-names",
        }
    }

    pub fn contract_digest(self) -> String {
        let canonical = match self {
            Self::CodexMcpTurnMetadata => CODEX_MCP_CONTRACT_CANONICAL,
            Self::CodexCommandHooks => CODEX_HOOKS_CONTRACT_CANONICAL,
            Self::CodexMcpCallableNames => CODEX_MCP_CALLABLE_NAMES_CONTRACT_CANONICAL,
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

/// An explicit MCP server registration key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpServerKey(String);

impl McpServerKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, HostContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_TOOL_NAME_BYTES
            && value.trim() == value
            && value.chars().all(|character| !character.is_control());
        valid
            .then_some(Self(value))
            .ok_or_else(|| HostContractError::invalid_field("mcp_server_key"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// The complete raw MCP tool name owned by one [`AgentToolId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpRawToolName(String);

impl McpRawToolName {
    pub fn for_tool(tool: AgentToolId) -> Self {
        Self(tool.wire_name().to_owned())
    }

    pub fn parse_for_tool(
        value: impl Into<String>,
        tool: AgentToolId,
    ) -> Result<Self, HostContractError> {
        let value = value.into();
        if value == tool.wire_name() {
            Ok(Self(value))
        } else {
            Err(HostContractError::unexpected_value("mcp_raw_tool_name"))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// A canonical MCP tool identity with explicit registration and raw-name coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpToolIdentity {
    server: McpServerKey,
    tool: AgentToolId,
    raw_tool_name: McpRawToolName,
}

impl McpToolIdentity {
    pub fn new(server: McpServerKey, tool: AgentToolId) -> Self {
        Self {
            server,
            tool,
            raw_tool_name: McpRawToolName::for_tool(tool),
        }
    }

    pub fn server(&self) -> &McpServerKey {
        &self.server
    }

    pub const fn tool(&self) -> AgentToolId {
        self.tool
    }

    pub fn raw_tool_name(&self) -> &McpRawToolName {
        &self.raw_tool_name
    }
}

/// A validated flattened name emitted by the Codex MCP callable-name contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostCallableName(String);

impl HostCallableName {
    pub fn parse(value: impl Into<String>) -> Result<Self, HostContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_HOST_CALLABLE_NAME_BYTES
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_');
        valid
            .then_some(Self(value))
            .ok_or_else(|| HostContractError::invalid_field("host_callable_name"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// One source MCP identity and its semantic Codex host-callable projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCallableIdentity {
    profile: HostContractProfileId,
    source: McpToolIdentity,
    callable_name: HostCallableName,
}

impl HostCallableIdentity {
    pub const fn profile(&self) -> HostContractProfileId {
        self.profile
    }

    pub fn source(&self) -> &McpToolIdentity {
        &self.source
    }

    pub fn callable_name(&self) -> &HostCallableName {
        &self.callable_name
    }
}

/// One exact host-native tool identity admitted by a command-hook matcher.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostToolIdentity(String);

impl HostToolIdentity {
    pub fn parse(value: impl Into<String>) -> Result<Self, HostContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_HOST_CALLABLE_NAME_BYTES
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_');
        valid
            .then_some(Self(value))
            .ok_or_else(|| HostContractError::invalid_field("host_tool_identity"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed host-level routing for one command-hook event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostHookMatcherStrategy {
    HostTools {
        identities: Vec<HostToolIdentity>,
    },
    McpServerNamespace {
        server: McpServerKey,
    },
    ExactCallables {
        callables: Vec<HostCallableIdentity>,
    },
    Union(Vec<HostHookMatcherStrategy>),
}

impl HostHookMatcherStrategy {
    /// Builds the current Codex Guard tool-routing strategy.
    pub fn codex_guard(server: &McpServerKey) -> Result<Self, HostContractError> {
        let identities = CODEX_GUARD_HOST_TOOLS
            .into_iter()
            .map(HostToolIdentity::parse)
            .collect::<Result<Vec<_>, _>>()?;
        let catalog = McpToolCatalog::for_server(server, AgentToolId::ALL)?;
        let namespace_prefix = codex_mcp_server_namespace_prefix(server);
        let mcp_routing = if catalog.identities().iter().all(|identity| {
            identity
                .callable_name()
                .as_str()
                .starts_with(&namespace_prefix)
        }) {
            Self::McpServerNamespace {
                server: server.clone(),
            }
        } else {
            Self::ExactCallables {
                callables: catalog.identities().to_vec(),
            }
        };
        Ok(Self::Union(vec![
            Self::HostTools { identities },
            mcp_routing,
        ]))
    }

    /// Renders the reviewed Codex matcher representation.
    pub fn codex_matcher(&self) -> Result<String, HostContractError> {
        let mut tokens = Vec::new();
        self.append_codex_tokens(&mut tokens)?;
        if tokens.is_empty() {
            return Err(HostContractError::invalid_field(
                "host_hook_matcher_strategy",
            ));
        }
        let mut unique = HashSet::new();
        if tokens.iter().any(|token| !unique.insert(token.clone())) {
            return Err(HostContractError::duplicate_tool(
                "host_hook_matcher_strategy",
            ));
        }
        Ok(tokens.join("|"))
    }

    /// Reconstructs the current typed strategy from one generated Codex matcher.
    pub fn parse_codex_guard(
        value: &str,
        server: &McpServerKey,
    ) -> Result<Self, HostContractError> {
        let expected = Self::codex_guard(server)?;
        let expected_mcp = match &expected {
            Self::Union(strategies) => strategies.get(1),
            _ => None,
        }
        .ok_or_else(|| HostContractError::invalid_field("host_hook_matcher"))?;
        let mut host_tools = Vec::new();
        let mut mcp_tokens = Vec::new();
        let mut unique = HashSet::new();
        for token in value.split('|') {
            if token.is_empty() || !unique.insert(token) {
                return Err(HostContractError::invalid_field("host_hook_matcher"));
            }
            if expected_mcp.matches_codex_token(token) {
                mcp_tokens.push(token);
            } else {
                host_tools.push(HostToolIdentity::parse(token)?);
            }
        }
        let reconstructed_mcp = match expected_mcp {
            Self::McpServerNamespace { server } if mcp_tokens.len() == 1 => {
                Self::McpServerNamespace {
                    server: server.clone(),
                }
            }
            Self::ExactCallables { callables } if mcp_tokens.len() == callables.len() => {
                Self::ExactCallables {
                    callables: callables.clone(),
                }
            }
            _ => return Err(HostContractError::invalid_field("host_hook_matcher")),
        };
        let reconstructed = Self::Union(vec![
            Self::HostTools {
                identities: host_tools,
            },
            reconstructed_mcp,
        ]);
        (reconstructed.codex_matcher()? == value)
            .then_some(reconstructed)
            .ok_or_else(|| HostContractError::invalid_field("host_hook_matcher"))
    }

    /// Returns whether the host-level strategy routes one bounded observed tool name.
    pub fn routes(&self, observed: &CanonicalToolName) -> bool {
        match self {
            Self::HostTools { identities } => identities
                .iter()
                .any(|identity| identity.as_str() == observed.as_str()),
            Self::McpServerNamespace { server } => {
                let prefix = codex_mcp_server_namespace_prefix(server);
                observed.as_str().starts_with(&prefix)
            }
            Self::ExactCallables { callables } => callables
                .iter()
                .any(|identity| identity.callable_name().as_str() == observed.as_str()),
            Self::Union(strategies) => strategies.iter().any(|strategy| strategy.routes(observed)),
        }
    }

    fn matches_codex_token(&self, token: &str) -> bool {
        match self {
            Self::McpServerNamespace { server } => {
                token == codex_mcp_server_namespace_matcher(server)
            }
            Self::ExactCallables { callables } => callables
                .iter()
                .any(|identity| identity.callable_name().as_str() == token),
            _ => false,
        }
    }

    fn append_codex_tokens(&self, tokens: &mut Vec<String>) -> Result<(), HostContractError> {
        match self {
            Self::HostTools { identities } => {
                tokens.extend(
                    identities
                        .iter()
                        .map(|identity| identity.as_str().to_owned()),
                );
            }
            Self::McpServerNamespace { server } => {
                tokens.push(codex_mcp_server_namespace_matcher(server));
            }
            Self::ExactCallables { callables } => {
                tokens.extend(
                    callables
                        .iter()
                        .map(|identity| identity.callable_name().as_str().to_owned()),
                );
            }
            Self::Union(strategies) => {
                if strategies.is_empty() {
                    return Err(HostContractError::invalid_field(
                        "host_hook_matcher_strategy",
                    ));
                }
                for strategy in strategies {
                    strategy.append_codex_tokens(tokens)?;
                }
            }
        }
        Ok(())
    }
}

/// The current semantic Codex MCP callable-name contract.
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexMcpCallableNames;

impl CodexMcpCallableNames {
    pub const PROFILE_ID: HostContractProfileId = HostContractProfileId::CodexMcpCallableNames;

    pub fn project_mcp_tool(
        &self,
        server: &McpServerKey,
        tool: AgentToolId,
    ) -> Result<HostCallableIdentity, HostContractError> {
        let source = McpToolIdentity::new(server.clone(), tool);
        let namespace = format!(
            "{CODEX_MCP_TOOL_PREFIX}{}",
            sanitize_callable_part(server.as_str())
        );
        let raw_tool_name = source.raw_tool_name().as_str();
        let callable = sanitize_callable_part(raw_tool_name);
        let raw_identity = codex_raw_tool_identity(server.as_str(), raw_tool_name);
        let (namespace, callable) = fit_callable_parts(
            &namespace,
            &callable,
            &raw_identity,
            MCP_TOOL_NAME_DELIMITER.len(),
        );
        let callable_name =
            HostCallableName::parse(format!("{namespace}{MCP_TOOL_NAME_DELIMITER}{callable}"))?;
        Ok(HostCallableIdentity {
            profile: Self::PROFILE_ID,
            source,
            callable_name,
        })
    }

    pub fn parse_callable_name(
        &self,
        value: &HostCallableName,
        catalog: &McpToolCatalog,
    ) -> Result<McpToolIdentity, HostContractError> {
        catalog
            .by_callable_name
            .get(value)
            .cloned()
            .ok_or_else(|| HostContractError::unknown_callable("host_callable_name"))
    }
}

/// Projects one explicitly registered MCP tool through the semantic Codex contract.
pub fn project_mcp_tool(
    server: &McpServerKey,
    tool: AgentToolId,
) -> Result<HostCallableIdentity, HostContractError> {
    CodexMcpCallableNames.project_mcp_tool(server, tool)
}

/// Resolves one callable name only through an explicit canonical catalog.
pub fn parse_callable_name(
    value: &HostCallableName,
    catalog: &McpToolCatalog,
) -> Result<McpToolIdentity, HostContractError> {
    CodexMcpCallableNames.parse_callable_name(value, catalog)
}

/// A collision-checked catalog of MCP source identities and host-callable projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolCatalog {
    identities: Vec<HostCallableIdentity>,
    by_callable_name: BTreeMap<HostCallableName, McpToolIdentity>,
}

impl McpToolCatalog {
    pub fn new<I>(registrations: I) -> Result<Self, HostContractError>
    where
        I: IntoIterator<Item = (McpServerKey, AgentToolId)>,
    {
        let mut sources = HashSet::new();
        let mut by_callable_name = BTreeMap::new();
        let mut identities = Vec::new();
        for (server, tool) in registrations {
            let identity = project_mcp_tool(&server, tool)?;
            if !sources.insert(identity.source.clone()) {
                return Err(HostContractError::duplicate_tool("mcp_tool_identity"));
            }
            if by_callable_name
                .insert(identity.callable_name.clone(), identity.source.clone())
                .is_some()
            {
                return Err(HostContractError::callable_collision("host_callable_name"));
            }
            identities.push(identity);
        }
        Ok(Self {
            identities,
            by_callable_name,
        })
    }

    pub fn for_server<I>(server: &McpServerKey, tools: I) -> Result<Self, HostContractError>
    where
        I: IntoIterator<Item = AgentToolId>,
    {
        Self::new(tools.into_iter().map(|tool| (server.clone(), tool)))
    }

    pub fn identities(&self) -> &[HostCallableIdentity] {
        &self.identities
    }

    pub fn find(&self, server: &McpServerKey, tool: AgentToolId) -> Option<&HostCallableIdentity> {
        self.identities
            .iter()
            .find(|identity| identity.source.server() == server && identity.source.tool() == tool)
    }

    pub fn require(
        &self,
        server: &McpServerKey,
        tool: AgentToolId,
    ) -> Result<&HostCallableIdentity, HostContractError> {
        self.find(server, tool)
            .ok_or_else(|| HostContractError::unknown_tool("mcp_tool_identity"))
    }
}

fn sanitize_callable_part(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            sanitized.push(character);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "_".to_owned()
    } else {
        sanitized
    }
}

fn codex_mcp_server_namespace(server: &McpServerKey) -> String {
    format!(
        "{CODEX_MCP_TOOL_PREFIX}{}",
        sanitize_callable_part(server.as_str())
    )
}

fn codex_mcp_server_namespace_prefix(server: &McpServerKey) -> String {
    format!(
        "{}{MCP_TOOL_NAME_DELIMITER}",
        codex_mcp_server_namespace(server)
    )
}

fn codex_mcp_server_namespace_matcher(server: &McpServerKey) -> String {
    format!("{}.*", codex_mcp_server_namespace_prefix(server))
}

fn codex_raw_tool_identity(server: &str, raw_tool_name: &str) -> String {
    format!("{server}\0{server}\0\0{raw_tool_name}\0{raw_tool_name}")
}

fn callable_name_hash_suffix(raw_identity: &str) -> String {
    let hash = format!("{:x}", Sha1::digest(raw_identity.as_bytes()));
    format!("_{}", &hash[..CALLABLE_NAME_HASH_LEN])
}

fn fit_callable_parts(
    namespace: &str,
    callable: &str,
    raw_identity: &str,
    reserved_len: usize,
) -> (String, String) {
    if namespace.len() + callable.len() + reserved_len <= MAX_HOST_CALLABLE_NAME_BYTES {
        return (namespace.to_owned(), callable.to_owned());
    }
    let suffix = callable_name_hash_suffix(raw_identity);
    let max_callable_len =
        MAX_HOST_CALLABLE_NAME_BYTES.saturating_sub(namespace.len() + reserved_len);
    if max_callable_len >= suffix.len() {
        let prefix_len = max_callable_len - suffix.len();
        return (
            namespace.to_owned(),
            format!("{}{}", truncate_ascii(callable, prefix_len), suffix),
        );
    }
    let max_namespace_len =
        MAX_HOST_CALLABLE_NAME_BYTES.saturating_sub(suffix.len() + reserved_len);
    (truncate_ascii(namespace, max_namespace_len), suffix)
}

fn truncate_ascii(value: &str, max_len: usize) -> String {
    value.chars().take(max_len).collect()
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

/// A parsed event from the `CodexCommandHooks` wire profile.
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
pub struct CodexMcpTurnMetadata;

impl CodexMcpTurnMetadata {
    pub const PROFILE_ID: HostContractProfileId = HostContractProfileId::CodexMcpTurnMetadata;

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
pub struct CodexCommandHooks;

impl CodexCommandHooks {
    pub const PROFILE_ID: HostContractProfileId = HostContractProfileId::CodexCommandHooks;

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

    const fn duplicate_tool(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::DuplicateMcpToolIdentity,
            field,
        }
    }

    const fn callable_collision(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::CallableNameCollision,
            field,
        }
    }

    const fn unknown_callable(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::UnknownCallableName,
            field,
        }
    }

    const fn unknown_tool(field: &'static str) -> Self {
        Self {
            code: HostContractErrorCode::UnknownMcpToolIdentity,
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
    DuplicateMcpToolIdentity,
    CallableNameCollision,
    UnknownCallableName,
    UnknownMcpToolIdentity,
}

impl HostContractErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRequiredField => "missing_required_field",
            Self::InvalidField => "invalid_field",
            Self::UnexpectedValue => "unexpected_value",
            Self::InconsistentCorrelation => "inconsistent_correlation",
            Self::PayloadTooLarge => "payload_too_large",
            Self::DuplicateMcpToolIdentity => "duplicate_mcp_tool_identity",
            Self::CallableNameCollision => "callable_name_collision",
            Self::UnknownCallableName => "unknown_callable_name",
            Self::UnknownMcpToolIdentity => "unknown_mcp_tool_identity",
        }
    }
}
