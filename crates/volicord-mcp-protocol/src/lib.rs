//! Closed MCP protocol-revision profiles used by Volicord production adapters.
//!
//! This crate owns exact protocol-revision parsing, reviewed feature declarations,
//! deterministic production ordering, and the preferred server revision. It has
//! no host-specific behavior and no registration path for arbitrary revisions.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fmt, str::FromStr};

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

/// Behavior of operation-phase JSON-RPC batching in a protocol revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JsonRpcBatching {
    /// Operation-phase batch request and response messages are not admitted.
    Disallowed,
    /// Operation-phase batch request and response messages may be admitted.
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

/// Wire carrier used for one successful or failed tool result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToolResultCarrier {
    /// The authoritative JSON object is carried by the top-level `toolResult`
    /// field.
    DirectToolResult,
    /// The authoritative JSON object is serialized into the first text item in
    /// `content`.
    JsonTextContent,
    /// The authoritative JSON object is carried by `structuredContent` while
    /// compatibility text remains in `content`.
    StructuredContentWithText,
}

/// Accepted top-level shape for `InitializeRequest.capabilities`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClientCapabilitiesShape {
    /// Any JSON object is accepted. Known fields remain recorded for schema
    /// parity, but unknown extension fields are not rejected.
    OpenObject,
}

/// Recovery behavior for a committed mutation whose ordinary result cannot be
/// projected within the adapter result budget.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommittedResultRecovery {
    /// Preserve fresh authority first, then the compact method result, then
    /// stable effect facts. The mutation is never retried.
    PreserveAuthorityThenCompactResult,
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

    /// Returns whether the selected profile allows operation-phase batch messages.
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
    title: bool,
    definition_metadata: bool,
    result_metadata: bool,
    result_carrier: ToolResultCarrier,
    is_error: bool,
}

impl ToolFeatures {
    const fn new(
        annotations: bool,
        output_schema: bool,
        title: bool,
        definition_metadata: bool,
        result_metadata: bool,
        result_carrier: ToolResultCarrier,
        is_error: bool,
    ) -> Self {
        Self {
            annotations,
            output_schema,
            title,
            definition_metadata,
            result_metadata,
            result_carrier,
            is_error,
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

    /// Returns whether tool definitions may contain `title`.
    pub const fn title(self) -> bool {
        self.title
    }

    /// Returns whether tool definitions may contain `_meta`.
    pub const fn definition_metadata(self) -> bool {
        self.definition_metadata
    }

    /// Returns whether tool results may contain `_meta`.
    pub const fn result_metadata(self) -> bool {
        self.result_metadata
    }

    /// Returns the selected result carrier form.
    pub const fn result_carrier(self) -> ToolResultCarrier {
        self.result_carrier
    }

    /// Returns whether tool results may contain `structuredContent`.
    pub const fn structured_content(self) -> bool {
        matches!(
            self.result_carrier,
            ToolResultCarrier::StructuredContentWithText
        )
    }

    /// Returns whether tool results may contain `isError`.
    pub const fn is_error(self) -> bool {
        self.is_error
    }
}

/// Initialize-result fields declared by a reviewed profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitializeFeatures {
    metadata: bool,
    protocol_version: bool,
    capabilities: bool,
    server_info: bool,
    instructions: bool,
    tools_capability: bool,
}

impl InitializeFeatures {
    const fn new(instructions: bool) -> Self {
        Self {
            metadata: true,
            protocol_version: true,
            capabilities: true,
            server_info: true,
            instructions,
            tools_capability: true,
        }
    }

    /// Returns whether `InitializeResult` may contain `_meta`.
    pub const fn metadata(self) -> bool {
        self.metadata
    }

    /// Returns whether `InitializeResult` contains `protocolVersion`.
    pub const fn protocol_version(self) -> bool {
        self.protocol_version
    }

    /// Returns whether `InitializeResult` contains `capabilities`.
    pub const fn capabilities(self) -> bool {
        self.capabilities
    }

    /// Returns whether `InitializeResult` contains `serverInfo`.
    pub const fn server_info(self) -> bool {
        self.server_info
    }

    /// Returns whether `InitializeResult` may contain `instructions`.
    pub const fn instructions(self) -> bool {
        self.instructions
    }

    /// Returns whether the server advertises the `tools` capability.
    pub const fn tools_capability(self) -> bool {
        self.tools_capability
    }
}

/// Client-capability admission declared by a reviewed profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientFeatures {
    shape: ClientCapabilitiesShape,
    known_fields: &'static [ClientCapabilityField],
}

impl ClientFeatures {
    const fn new(known_fields: &'static [ClientCapabilityField]) -> Self {
        Self {
            shape: ClientCapabilitiesShape::OpenObject,
            known_fields,
        }
    }

    /// Returns the accepted top-level client capability shape.
    pub const fn shape(self) -> ClientCapabilitiesShape {
        self.shape
    }

    /// Returns the known fields from the pinned schema.
    pub const fn known_fields(self) -> &'static [ClientCapabilityField] {
        self.known_fields
    }
}

/// Result-budget and committed-mutation recovery behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResultRecoveryFeatures {
    committed_result_recovery: CommittedResultRecovery,
}

impl ResultRecoveryFeatures {
    const fn authority_preserving() -> Self {
        Self {
            committed_result_recovery: CommittedResultRecovery::PreserveAuthorityThenCompactResult,
        }
    }

    /// Returns the recovery behavior after a committed mutation.
    pub const fn committed_result_recovery(self) -> CommittedResultRecovery {
        self.committed_result_recovery
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

/// A top-level field in the pinned `CallToolResult` schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToolResultField {
    Meta,
    ToolResult,
    Content,
    IsError,
    StructuredContent,
}

/// Exact revision-specific capability, tool-definition, and tool-result fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemaFeatures {
    client_capability_fields: &'static [ClientCapabilityField],
    server_capability_fields: &'static [ServerCapabilityField],
    tool_definition_fields: &'static [ToolDefinitionField],
    tool_result_fields: &'static [ToolResultField],
}

/// Complete semantic capability bundle consumed by protocol adapters.
///
/// Revision identifiers do not appear in this type, so adapter projection can
/// select behavior only through reviewed semantic declarations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpProtocolCapabilities {
    messages: MessageFeatures,
    tools: ToolFeatures,
    initialize: InitializeFeatures,
    client: ClientFeatures,
    result_recovery: ResultRecoveryFeatures,
    schema: SchemaFeatures,
}

impl McpProtocolCapabilities {
    const fn new(
        messages: MessageFeatures,
        tools: ToolFeatures,
        client_capability_fields: &'static [ClientCapabilityField],
        schema: SchemaFeatures,
    ) -> Self {
        Self {
            messages,
            tools,
            initialize: InitializeFeatures::new(messages.initialize_result_instructions()),
            client: ClientFeatures::new(client_capability_fields),
            result_recovery: ResultRecoveryFeatures::authority_preserving(),
            schema,
        }
    }

    /// Returns message-admission capabilities.
    pub const fn messages(self) -> MessageFeatures {
        self.messages
    }

    /// Returns tool-definition and tool-result capabilities.
    pub const fn tools(self) -> ToolFeatures {
        self.tools
    }

    /// Returns initialize-result capabilities.
    pub const fn initialize(self) -> InitializeFeatures {
        self.initialize
    }

    /// Returns client-capability admission.
    pub const fn client(self) -> ClientFeatures {
        self.client
    }

    /// Returns result-budget and committed-mutation recovery behavior.
    pub const fn result_recovery(self) -> ResultRecoveryFeatures {
        self.result_recovery
    }

    /// Returns exact pinned schema fields used by schema parity checks.
    pub const fn schema(self) -> SchemaFeatures {
        self.schema
    }
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
    capabilities: McpProtocolCapabilities,
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
            capabilities: McpProtocolCapabilities::new(
                messages,
                tools,
                schema.client_capability_fields(),
                schema,
            ),
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
        self.capabilities.messages()
    }

    /// Returns this profile's tool features.
    pub const fn tools(self) -> ToolFeatures {
        self.capabilities.tools()
    }

    /// Returns this profile's initialize-result features.
    pub const fn initialize(self) -> InitializeFeatures {
        self.capabilities.initialize()
    }

    /// Returns this profile's accepted client-capability shape and known fields.
    pub const fn client(self) -> ClientFeatures {
        self.capabilities.client()
    }

    /// Returns this profile's result recovery behavior.
    pub const fn result_recovery(self) -> ResultRecoveryFeatures {
        self.capabilities.result_recovery()
    }

    /// Returns this profile's revision-specific schema fields.
    pub const fn schema(self) -> SchemaFeatures {
        self.capabilities.schema()
    }

    /// Returns the semantic capability bundle consumed by adapters.
    pub const fn capabilities(self) -> McpProtocolCapabilities {
        self.capabilities
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
const INITIALIZE_MESSAGES_WITH_OPERATION_BATCHING: MessageFeatures = MessageFeatures::new(
    JsonRpcBatching::Allowed,
    InitializedNotification::AfterInitialize,
    true,
);
const TOOLS_DIRECT_RESULT: ToolFeatures = ToolFeatures::new(
    false,
    false,
    false,
    false,
    true,
    ToolResultCarrier::DirectToolResult,
    false,
);
const TOOLS_TEXT_RESULT: ToolFeatures = ToolFeatures::new(
    false,
    false,
    false,
    false,
    true,
    ToolResultCarrier::JsonTextContent,
    true,
);
const TOOLS_ANNOTATED_TEXT_RESULT: ToolFeatures = ToolFeatures::new(
    true,
    false,
    false,
    false,
    true,
    ToolResultCarrier::JsonTextContent,
    true,
);
const TOOLS_STRUCTURED_RESULT: ToolFeatures = ToolFeatures::new(
    true,
    true,
    true,
    true,
    true,
    ToolResultCarrier::StructuredContentWithText,
    true,
);

const PRODUCTION_PROFILES: [McpProtocolProfile; 5] = [
    McpProtocolProfile::new(
        McpProtocolRevision::V20241007,
        INITIALIZE_MESSAGES_2024_10,
        TOOLS_DIRECT_RESULT,
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
        TOOLS_TEXT_RESULT,
        SchemaFeatures::new(
            CLIENT_CAPABILITIES_2024,
            SERVER_CAPABILITIES_2024,
            TOOL_DEFINITION_2024,
            TOOL_RESULT_2024_11,
        ),
    ),
    McpProtocolProfile::new(
        McpProtocolRevision::V20250326,
        INITIALIZE_MESSAGES_WITH_OPERATION_BATCHING,
        TOOLS_ANNOTATED_TEXT_RESULT,
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
        TOOLS_STRUCTURED_RESULT,
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
        TOOLS_STRUCTURED_RESULT,
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

    /// Selects an exact production-supported initialization profile.
    ///
    /// Unknown and tracked-but-unsupported identifiers are rejected. The
    /// registry never substitutes another profile.
    pub fn select_initialize(
        &self,
        requested: &str,
    ) -> Result<&'static McpProtocolProfile, McpProtocolRevisionError> {
        self.parse(requested)
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

/// Exact identifiers for one stable semantic MCP contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolContractDescriptor {
    id: &'static str,
    identifiers: BTreeSet<String>,
    related_contracts: Vec<&'static str>,
}

impl ProtocolContractDescriptor {
    /// Returns the stable semantic contract identity.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Returns exact public protocol revision identifiers.
    pub const fn identifiers(&self) -> &BTreeSet<String> {
        &self.identifiers
    }

    /// Returns deliberate semantic relationships to adjacent contracts.
    pub fn related_contracts(&self) -> &[&'static str] {
        &self.related_contracts
    }
}

/// Returns the semantic MCP contract derived from the production registry.
pub fn protocol_contract_descriptors() -> Vec<ProtocolContractDescriptor> {
    let identifiers = PRODUCTION_PROFILES
        .iter()
        .map(|profile| profile.revision().as_str().to_owned())
        .collect();
    vec![ProtocolContractDescriptor {
        id: "mcp.protocol",
        identifiers,
        related_contracts: Vec::new(),
    }]
}
