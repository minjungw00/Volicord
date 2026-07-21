//! Closed MCP protocol-revision profiles used by Volicord production adapters.
//!
//! This crate owns exact protocol-revision parsing, reviewed feature declarations,
//! deterministic production ordering, and the preferred server revision. It has
//! no host-specific behavior and no registration path for arbitrary revisions.

#![forbid(unsafe_code)]

use std::{fmt, str::FromStr};

const TRACKED_REVISIONS: [McpProtocolRevision; 6] = [
    McpProtocolRevision::V20241007,
    McpProtocolRevision::V20241105,
    McpProtocolRevision::V20250326,
    McpProtocolRevision::V20250618,
    McpProtocolRevision::V20251125,
    McpProtocolRevision::V20260728,
];

/// A pinned MCP protocol revision tracked by this workspace.
///
/// Released revisions and the pre-release discover generation are represented
/// explicitly. Production support remains a separate property of
/// [`ProtocolRegistry`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum McpProtocolRevision {
    /// Released initialization-based revision `2024-10-07`.
    V20241007,
    /// Released initialization-based revision `2024-11-05`.
    V20241105,
    /// Released initialization-based revision `2025-03-26`.
    V20250326,
    /// Released initialization-based revision `2025-06-18`.
    V20250618,
    /// Released initialization-based revision `2025-11-25`.
    V20251125,
    /// Tracked pre-release discover-based revision `2026-07-28`.
    V20260728,
}

impl McpProtocolRevision {
    /// Returns the exact protocol-version string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V20241007 => "2024-10-07",
            Self::V20241105 => "2024-11-05",
            Self::V20250326 => "2025-03-26",
            Self::V20250618 => "2025-06-18",
            Self::V20251125 => "2025-11-25",
            Self::V20260728 => "2026-07-28",
        }
    }

    /// Returns the handshake generation recorded for this pinned revision.
    pub const fn generation(self) -> McpProtocolGeneration {
        match self {
            Self::V20241007
            | Self::V20241105
            | Self::V20250326
            | Self::V20250618
            | Self::V20251125 => McpProtocolGeneration::InitializeHandshake,
            Self::V20260728 => McpProtocolGeneration::Discover,
        }
    }

    /// Returns the pinned release classification for this revision.
    pub const fn status(self) -> McpRevisionStatus {
        match self {
            Self::V20241007
            | Self::V20241105
            | Self::V20250326
            | Self::V20250618
            | Self::V20251125 => McpRevisionStatus::Released,
            Self::V20260728 => McpRevisionStatus::ReleaseCandidate,
        }
    }
}

impl fmt::Display for McpProtocolRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for McpProtocolRevision {
    type Err = McpProtocolRevisionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        TRACKED_REVISIONS
            .iter()
            .copied()
            .find(|revision| revision.as_str() == value)
            .ok_or(McpProtocolRevisionError::Unknown)
    }
}

/// The handshake family for a pinned MCP protocol revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum McpProtocolGeneration {
    /// The client and server negotiate through `initialize` and then the client
    /// sends `notifications/initialized`.
    InitializeHandshake,
    /// The future generation begins with `server/discover` and per-request
    /// protocol metadata.
    Discover,
}

/// Release classification copied from the pinned MCP specification manifest.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum McpRevisionStatus {
    /// A finalized upstream revision.
    Released,
    /// A tracked upstream release candidate that is not production-supported.
    ReleaseCandidate,
}

/// Failure to parse or select an MCP protocol revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpProtocolRevisionError {
    /// The string is not one of the exact pinned protocol-version strings.
    Unknown,
    /// The revision is pinned for review but has no production profile.
    NotProductionSupported(McpProtocolRevision),
}

impl fmt::Display for McpProtocolRevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => formatter.write_str("unknown MCP protocol revision"),
            Self::NotProductionSupported(revision) => {
                write!(
                    formatter,
                    "MCP protocol revision {revision} is not production-supported"
                )
            }
        }
    }
}

impl std::error::Error for McpProtocolRevisionError {}

/// Behavior of JSON-RPC batching in a protocol revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JsonRpcBatching {
    /// Batch request and response messages are absent from the revision schema.
    Disallowed,
    /// Batch request and response messages are present in the revision schema.
    Allowed,
}

/// Behavior of the initialized notification in a protocol revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InitializedNotification {
    /// The notification follows a successful initialize handshake.
    AfterInitialize,
    /// The generation has no initialized notification.
    Absent,
}

/// Message-level protocol features declared by a reviewed profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MessageFeatures {
    json_rpc_batching: JsonRpcBatching,
    initialized_notification: InitializedNotification,
    initialize_result_instructions: bool,
}

impl MessageFeatures {
    const fn new(
        json_rpc_batching: JsonRpcBatching,
        initialized_notification: InitializedNotification,
        initialize_result_instructions: bool,
    ) -> Self {
        Self {
            json_rpc_batching,
            initialized_notification,
            initialize_result_instructions,
        }
    }

    /// Returns whether the revision schema allows JSON-RPC batch messages.
    pub const fn json_rpc_batching(self) -> JsonRpcBatching {
        self.json_rpc_batching
    }

    /// Returns the revision's initialized-notification behavior.
    pub const fn initialized_notification(self) -> InitializedNotification {
        self.initialized_notification
    }

    /// Returns whether `InitializeResult` may contain `instructions`.
    pub const fn initialize_result_instructions(self) -> bool {
        self.initialize_result_instructions
    }
}

/// Tool-specific feature availability declared by a reviewed profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToolFeatures {
    annotations: bool,
    output_schema: bool,
    structured_content: bool,
}

impl ToolFeatures {
    const fn new(annotations: bool, output_schema: bool, structured_content: bool) -> Self {
        Self {
            annotations,
            output_schema,
            structured_content,
        }
    }

    /// Returns whether tool definitions may contain `annotations`.
    pub const fn annotations(self) -> bool {
        self.annotations
    }

    /// Returns whether tool definitions may contain `outputSchema`.
    pub const fn output_schema(self) -> bool {
        self.output_schema
    }

    /// Returns whether tool results may contain `structuredContent`.
    pub const fn structured_content(self) -> bool {
        self.structured_content
    }
}

/// A top-level field in the pinned `ClientCapabilities` schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClientCapabilityField {
    Elicitation,
    Experimental,
    Roots,
    Sampling,
    Tasks,
}

impl ClientCapabilityField {
    /// Returns the exact wire field name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Elicitation => "elicitation",
            Self::Experimental => "experimental",
            Self::Roots => "roots",
            Self::Sampling => "sampling",
            Self::Tasks => "tasks",
        }
    }
}

/// A top-level field in the pinned `ServerCapabilities` schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServerCapabilityField {
    Completions,
    Experimental,
    Logging,
    Prompts,
    Resources,
    Tasks,
    Tools,
}

impl ServerCapabilityField {
    /// Returns the exact wire field name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completions => "completions",
            Self::Experimental => "experimental",
            Self::Logging => "logging",
            Self::Prompts => "prompts",
            Self::Resources => "resources",
            Self::Tasks => "tasks",
            Self::Tools => "tools",
        }
    }
}

/// A top-level field in the pinned `Tool` definition schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToolDefinitionField {
    Meta,
    Annotations,
    Description,
    Execution,
    Icons,
    InputSchema,
    Name,
    OutputSchema,
    Title,
}

impl ToolDefinitionField {
    /// Returns the exact wire field name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Meta => "_meta",
            Self::Annotations => "annotations",
            Self::Description => "description",
            Self::Execution => "execution",
            Self::Icons => "icons",
            Self::InputSchema => "inputSchema",
            Self::Name => "name",
            Self::OutputSchema => "outputSchema",
            Self::Title => "title",
        }
    }
}

/// A top-level field in the pinned `CallToolResult` schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToolResultField {
    Meta,
    ToolResult,
    Content,
    IsError,
    StructuredContent,
}

impl ToolResultField {
    /// Returns the exact wire field name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Meta => "_meta",
            Self::ToolResult => "toolResult",
            Self::Content => "content",
            Self::IsError => "isError",
            Self::StructuredContent => "structuredContent",
        }
    }
}

/// Exact revision-specific capability, tool-definition, and tool-result fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemaFeatures {
    client_capability_fields: &'static [ClientCapabilityField],
    server_capability_fields: &'static [ServerCapabilityField],
    tool_definition_fields: &'static [ToolDefinitionField],
    tool_result_fields: &'static [ToolResultField],
}

impl SchemaFeatures {
    const fn new(
        client_capability_fields: &'static [ClientCapabilityField],
        server_capability_fields: &'static [ServerCapabilityField],
        tool_definition_fields: &'static [ToolDefinitionField],
        tool_result_fields: &'static [ToolResultField],
    ) -> Self {
        Self {
            client_capability_fields,
            server_capability_fields,
            tool_definition_fields,
            tool_result_fields,
        }
    }

    /// Returns the exact top-level `ClientCapabilities` fields.
    pub const fn client_capability_fields(self) -> &'static [ClientCapabilityField] {
        self.client_capability_fields
    }

    /// Returns the exact top-level `ServerCapabilities` fields.
    pub const fn server_capability_fields(self) -> &'static [ServerCapabilityField] {
        self.server_capability_fields
    }

    /// Returns the exact top-level `Tool` fields.
    pub const fn tool_definition_fields(self) -> &'static [ToolDefinitionField] {
        self.tool_definition_fields
    }

    /// Returns the exact top-level `CallToolResult` fields.
    pub const fn tool_result_fields(self) -> &'static [ToolResultField] {
        self.tool_result_fields
    }
}

/// One reviewed production-supported MCP protocol profile.
///
/// Fields and construction remain private so callers can only obtain profiles
/// that are part of the statically reviewed production registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpProtocolProfile {
    revision: McpProtocolRevision,
    generation: McpProtocolGeneration,
    status: McpRevisionStatus,
    messages: MessageFeatures,
    tools: ToolFeatures,
    schema: SchemaFeatures,
}

impl McpProtocolProfile {
    const fn new(
        revision: McpProtocolRevision,
        messages: MessageFeatures,
        tools: ToolFeatures,
        schema: SchemaFeatures,
    ) -> Self {
        Self {
            revision,
            generation: revision.generation(),
            status: revision.status(),
            messages,
            tools,
            schema,
        }
    }

    /// Returns this profile's exact protocol revision.
    pub const fn revision(self) -> McpProtocolRevision {
        self.revision
    }

    /// Returns this profile's handshake generation.
    pub const fn generation(self) -> McpProtocolGeneration {
        self.generation
    }

    /// Returns this profile's pinned release status.
    pub const fn status(self) -> McpRevisionStatus {
        self.status
    }

    /// Returns this profile's message features.
    pub const fn messages(self) -> MessageFeatures {
        self.messages
    }

    /// Returns this profile's tool features.
    pub const fn tools(self) -> ToolFeatures {
        self.tools
    }

    /// Returns this profile's revision-specific schema fields.
    pub const fn schema(self) -> SchemaFeatures {
        self.schema
    }
}

use ClientCapabilityField as Client;
use ServerCapabilityField as Server;
use ToolDefinitionField as ToolDefinition;
use ToolResultField as ToolResult;

const CLIENT_CAPABILITIES_2024: &[ClientCapabilityField] =
    &[Client::Experimental, Client::Roots, Client::Sampling];
const CLIENT_CAPABILITIES_2025_06: &[ClientCapabilityField] = &[
    Client::Elicitation,
    Client::Experimental,
    Client::Roots,
    Client::Sampling,
];
const CLIENT_CAPABILITIES_2025_11: &[ClientCapabilityField] = &[
    Client::Elicitation,
    Client::Experimental,
    Client::Roots,
    Client::Sampling,
    Client::Tasks,
];

const SERVER_CAPABILITIES_2024: &[ServerCapabilityField] = &[
    Server::Experimental,
    Server::Logging,
    Server::Prompts,
    Server::Resources,
    Server::Tools,
];
const SERVER_CAPABILITIES_2025_03: &[ServerCapabilityField] = &[
    Server::Completions,
    Server::Experimental,
    Server::Logging,
    Server::Prompts,
    Server::Resources,
    Server::Tools,
];
const SERVER_CAPABILITIES_2025_11: &[ServerCapabilityField] = &[
    Server::Completions,
    Server::Experimental,
    Server::Logging,
    Server::Prompts,
    Server::Resources,
    Server::Tasks,
    Server::Tools,
];

const TOOL_DEFINITION_2024: &[ToolDefinitionField] = &[
    ToolDefinition::Description,
    ToolDefinition::InputSchema,
    ToolDefinition::Name,
];
const TOOL_DEFINITION_2025_03: &[ToolDefinitionField] = &[
    ToolDefinition::Annotations,
    ToolDefinition::Description,
    ToolDefinition::InputSchema,
    ToolDefinition::Name,
];
const TOOL_DEFINITION_2025_06: &[ToolDefinitionField] = &[
    ToolDefinition::Meta,
    ToolDefinition::Annotations,
    ToolDefinition::Description,
    ToolDefinition::InputSchema,
    ToolDefinition::Name,
    ToolDefinition::OutputSchema,
    ToolDefinition::Title,
];
const TOOL_DEFINITION_2025_11: &[ToolDefinitionField] = &[
    ToolDefinition::Meta,
    ToolDefinition::Annotations,
    ToolDefinition::Description,
    ToolDefinition::Execution,
    ToolDefinition::Icons,
    ToolDefinition::InputSchema,
    ToolDefinition::Name,
    ToolDefinition::OutputSchema,
    ToolDefinition::Title,
];

const TOOL_RESULT_2024_10: &[ToolResultField] = &[ToolResult::Meta, ToolResult::ToolResult];
const TOOL_RESULT_2024_11: &[ToolResultField] =
    &[ToolResult::Meta, ToolResult::Content, ToolResult::IsError];
const TOOL_RESULT_2025_06: &[ToolResultField] = &[
    ToolResult::Meta,
    ToolResult::Content,
    ToolResult::IsError,
    ToolResult::StructuredContent,
];

const INITIALIZE_MESSAGES_2024_10: MessageFeatures = MessageFeatures::new(
    JsonRpcBatching::Disallowed,
    InitializedNotification::AfterInitialize,
    false,
);
const INITIALIZE_MESSAGES_WITHOUT_BATCHING: MessageFeatures = MessageFeatures::new(
    JsonRpcBatching::Disallowed,
    InitializedNotification::AfterInitialize,
    true,
);
const INITIALIZE_MESSAGES_WITH_BATCHING: MessageFeatures = MessageFeatures::new(
    JsonRpcBatching::Allowed,
    InitializedNotification::AfterInitialize,
    true,
);
const TOOLS_BASE: ToolFeatures = ToolFeatures::new(false, false, false);
const TOOLS_WITH_ANNOTATIONS: ToolFeatures = ToolFeatures::new(true, false, false);
const TOOLS_WITH_STRUCTURED_OUTPUT: ToolFeatures = ToolFeatures::new(true, true, true);

const PRODUCTION_PROFILES: [McpProtocolProfile; 5] = [
    McpProtocolProfile::new(
        McpProtocolRevision::V20241007,
        INITIALIZE_MESSAGES_2024_10,
        TOOLS_BASE,
        SchemaFeatures::new(
            CLIENT_CAPABILITIES_2024,
            SERVER_CAPABILITIES_2024,
            TOOL_DEFINITION_2024,
            TOOL_RESULT_2024_10,
        ),
    ),
    McpProtocolProfile::new(
        McpProtocolRevision::V20241105,
        INITIALIZE_MESSAGES_WITHOUT_BATCHING,
        TOOLS_BASE,
        SchemaFeatures::new(
            CLIENT_CAPABILITIES_2024,
            SERVER_CAPABILITIES_2024,
            TOOL_DEFINITION_2024,
            TOOL_RESULT_2024_11,
        ),
    ),
    McpProtocolProfile::new(
        McpProtocolRevision::V20250326,
        INITIALIZE_MESSAGES_WITH_BATCHING,
        TOOLS_WITH_ANNOTATIONS,
        SchemaFeatures::new(
            CLIENT_CAPABILITIES_2024,
            SERVER_CAPABILITIES_2025_03,
            TOOL_DEFINITION_2025_03,
            TOOL_RESULT_2024_11,
        ),
    ),
    McpProtocolProfile::new(
        McpProtocolRevision::V20250618,
        INITIALIZE_MESSAGES_WITHOUT_BATCHING,
        TOOLS_WITH_STRUCTURED_OUTPUT,
        SchemaFeatures::new(
            CLIENT_CAPABILITIES_2025_06,
            SERVER_CAPABILITIES_2025_03,
            TOOL_DEFINITION_2025_06,
            TOOL_RESULT_2025_06,
        ),
    ),
    McpProtocolProfile::new(
        McpProtocolRevision::V20251125,
        INITIALIZE_MESSAGES_WITHOUT_BATCHING,
        TOOLS_WITH_STRUCTURED_OUTPUT,
        SchemaFeatures::new(
            CLIENT_CAPABILITIES_2025_11,
            SERVER_CAPABILITIES_2025_11,
            TOOL_DEFINITION_2025_11,
            TOOL_RESULT_2025_06,
        ),
    ),
];

const PREFERRED_SERVER_PROFILE_INDEX: usize = 4;
static PRODUCTION_REGISTRY: ProtocolRegistry = ProtocolRegistry { _private: () };

/// The closed registry of reviewed production-supported MCP profiles.
///
/// The registry cannot be constructed or extended outside this crate. Its
/// explicit iteration order is independent of string comparison.
#[derive(Debug)]
pub struct ProtocolRegistry {
    _private: (),
}

impl ProtocolRegistry {
    /// The server revision preferred independently of the supported set.
    pub const PREFERRED_SERVER_REVISION: McpProtocolRevision = McpProtocolRevision::V20251125;

    /// Returns the single production registry.
    pub const fn production() -> &'static Self {
        &PRODUCTION_REGISTRY
    }

    /// Parses an exact production-supported protocol-version string.
    pub fn parse(
        &self,
        value: &str,
    ) -> Result<&'static McpProtocolProfile, McpProtocolRevisionError> {
        let revision = value.parse::<McpProtocolRevision>()?;
        self.profile(revision)
            .ok_or(McpProtocolRevisionError::NotProductionSupported(revision))
    }

    /// Looks up a production profile by its typed revision.
    pub fn profile(&self, revision: McpProtocolRevision) -> Option<&'static McpProtocolProfile> {
        PRODUCTION_PROFILES
            .iter()
            .find(|profile| profile.revision == revision)
    }

    /// Iterates production profiles in reviewed oldest-to-newest order.
    pub fn oldest_to_newest(
        &self,
    ) -> impl DoubleEndedIterator<Item = &'static McpProtocolProfile> + ExactSizeIterator {
        PRODUCTION_PROFILES.iter()
    }

    /// Iterates production profiles in reviewed newest-to-oldest order.
    pub fn newest_to_oldest(
        &self,
    ) -> impl DoubleEndedIterator<Item = &'static McpProtocolProfile> + ExactSizeIterator {
        PRODUCTION_PROFILES.iter().rev()
    }

    /// Returns the profile for the separately selected preferred server revision.
    pub fn preferred_server_profile(&self) -> &'static McpProtocolProfile {
        &PRODUCTION_PROFILES[PREFERRED_SERVER_PROFILE_INDEX]
    }
}
