use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;
use volicord_mcp_protocol::{
    ClientCapabilitiesShape, ClientCapabilityField, CommittedResultRecovery,
    InitializedNotification, JsonRpcBatching, McpProtocolGeneration, McpProtocolRevision,
    McpProtocolRevisionError, McpRevisionStatus, ProtocolRegistry, ServerCapabilityField,
    ToolDefinitionField, ToolResultCarrier, ToolResultField,
};

const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/manifest.toml"
));

const SCHEMA_2024_10_07: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2024-10-07/schema.json"
));
const SCHEMA_2024_11_05: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2024-11-05/schema.json"
));
const SCHEMA_2025_03_26: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2025-03-26/schema.json"
));
const SCHEMA_2025_06_18: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2025-06-18/schema.json"
));
const SCHEMA_2025_11_25: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2025-11-25/schema.json"
));
const SCHEMA_2026_07_28: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/draft/2026-07-28/schema.json"
));

#[derive(Debug, Deserialize)]
struct Manifest {
    revision: Vec<ManifestRevision>,
}

#[derive(Debug, Deserialize)]
struct ManifestRevision {
    protocol_version: String,
    release_status: String,
    handshake_family: String,
    production_supported: bool,
    pre_release_only: bool,
}

fn manifest() -> Manifest {
    toml_edit::de::from_str(MANIFEST).expect("pinned MCP manifest should parse")
}

fn schema(revision: McpProtocolRevision) -> Value {
    let source = match revision {
        McpProtocolRevision::V20241007 => SCHEMA_2024_10_07,
        McpProtocolRevision::V20241105 => SCHEMA_2024_11_05,
        McpProtocolRevision::V20250326 => SCHEMA_2025_03_26,
        McpProtocolRevision::V20250618 => SCHEMA_2025_06_18,
        McpProtocolRevision::V20251125 => SCHEMA_2025_11_25,
        McpProtocolRevision::V20260728 => SCHEMA_2026_07_28,
    };
    serde_json::from_str(source).expect("pinned MCP schema should parse")
}

fn definitions(schema: &Value) -> &Map<String, Value> {
    schema
        .get("definitions")
        .or_else(|| schema.get("$defs"))
        .and_then(Value::as_object)
        .expect("pinned schema definitions")
}

fn property_names<'a>(definitions: &'a Map<String, Value>, definition: &str) -> BTreeSet<&'a str> {
    definitions
        .get(definition)
        .and_then(|value| value.get("properties"))
        .and_then(Value::as_object)
        .expect("definition properties")
        .keys()
        .map(String::as_str)
        .collect()
}

fn names<T: Copy>(values: &[T], name: impl Fn(T) -> &'static str) -> BTreeSet<&'static str> {
    values.iter().copied().map(name).collect()
}

fn client_capability_name(field: ClientCapabilityField) -> &'static str {
    match field {
        ClientCapabilityField::Elicitation => "elicitation",
        ClientCapabilityField::Experimental => "experimental",
        ClientCapabilityField::Roots => "roots",
        ClientCapabilityField::Sampling => "sampling",
        ClientCapabilityField::Tasks => "tasks",
    }
}

fn server_capability_name(field: ServerCapabilityField) -> &'static str {
    match field {
        ServerCapabilityField::Completions => "completions",
        ServerCapabilityField::Experimental => "experimental",
        ServerCapabilityField::Logging => "logging",
        ServerCapabilityField::Prompts => "prompts",
        ServerCapabilityField::Resources => "resources",
        ServerCapabilityField::Tasks => "tasks",
        ServerCapabilityField::Tools => "tools",
    }
}

fn tool_definition_name(field: ToolDefinitionField) -> &'static str {
    match field {
        ToolDefinitionField::Meta => "_meta",
        ToolDefinitionField::Annotations => "annotations",
        ToolDefinitionField::Description => "description",
        ToolDefinitionField::Execution => "execution",
        ToolDefinitionField::Icons => "icons",
        ToolDefinitionField::InputSchema => "inputSchema",
        ToolDefinitionField::Name => "name",
        ToolDefinitionField::OutputSchema => "outputSchema",
        ToolDefinitionField::Title => "title",
    }
}

fn tool_result_name(field: ToolResultField) -> &'static str {
    match field {
        ToolResultField::Meta => "_meta",
        ToolResultField::ToolResult => "toolResult",
        ToolResultField::Content => "content",
        ToolResultField::IsError => "isError",
        ToolResultField::StructuredContent => "structuredContent",
    }
}

fn json_rpc_message_allows_batch(definitions: &Map<String, Value>) -> bool {
    definitions
        .get("JSONRPCMessage")
        .and_then(|value| value.get("anyOf"))
        .and_then(Value::as_array)
        .is_some_and(|variants| {
            variants
                .iter()
                .any(|variant| variant.get("type").and_then(Value::as_str) == Some("array"))
        })
}

#[test]
fn every_production_revision_parses_exactly() {
    let registry = ProtocolRegistry::production();

    for profile in registry.oldest_to_newest() {
        let revision = profile.revision();
        assert_eq!(
            McpProtocolRevision::from_str(revision.as_str()),
            Ok(revision)
        );
        assert_eq!(registry.parse(revision.as_str()), Ok(profile));
    }
}

#[test]
fn every_production_initialize_revision_is_selected_exactly() {
    let registry = ProtocolRegistry::production();

    for profile in registry.oldest_to_newest() {
        assert_eq!(
            registry.select_initialize(profile.revision().as_str()),
            Ok(profile)
        );
    }
}

#[test]
fn unknown_initialize_identifiers_are_rejected_without_profile_substitution() {
    let registry = ProtocolRegistry::production();

    for requested in ["", "unsupported-revision", "2025-11-25-preview"] {
        assert_eq!(
            registry.select_initialize(requested),
            Err(McpProtocolRevisionError::Unknown)
        );
    }
}

#[test]
fn tracked_nonproduction_identifier_is_rejected_without_profile_substitution() {
    let registry = ProtocolRegistry::production();
    let revision = McpProtocolRevision::V20260728;

    assert_eq!(
        registry.select_initialize(revision.as_str()),
        Err(McpProtocolRevisionError::NotProductionSupported(revision))
    );
}

#[test]
fn unknown_revisions_are_rejected() {
    let registry = ProtocolRegistry::production();

    for value in [
        "",
        "2024-10-08",
        "2025-01-01",
        "2025-11-26",
        "2025-11-25-preview",
        " 2025-11-25",
        "9999-12-31",
    ] {
        assert_eq!(
            McpProtocolRevision::from_str(value),
            Err(McpProtocolRevisionError::Unknown),
            "{value}"
        );
        assert_eq!(
            registry.parse(value),
            Err(McpProtocolRevisionError::Unknown),
            "{value}"
        );
    }
}

#[test]
fn production_ordering_is_explicit_and_deterministic() {
    let registry = ProtocolRegistry::production();
    let expected = [
        McpProtocolRevision::V20241007,
        McpProtocolRevision::V20241105,
        McpProtocolRevision::V20250326,
        McpProtocolRevision::V20250618,
        McpProtocolRevision::V20251125,
    ];
    let oldest_to_newest = registry
        .oldest_to_newest()
        .map(|profile| profile.revision())
        .collect::<Vec<_>>();
    let newest_to_oldest = registry
        .newest_to_oldest()
        .map(|profile| profile.revision())
        .collect::<Vec<_>>();

    assert_eq!(oldest_to_newest, expected);
    assert_eq!(
        newest_to_oldest,
        expected.into_iter().rev().collect::<Vec<_>>()
    );
}

#[test]
fn production_registry_has_no_duplicate_profiles() {
    let profiles = ProtocolRegistry::production()
        .oldest_to_newest()
        .collect::<Vec<_>>();
    let revisions = profiles
        .iter()
        .map(|profile| profile.revision())
        .collect::<HashSet<_>>();

    assert_eq!(profiles.len(), revisions.len());
}

#[test]
fn production_registry_matches_the_pinned_manifest() {
    let manifest = manifest();
    let manifest_profiles = manifest
        .revision
        .iter()
        .filter(|revision| revision.production_supported)
        .collect::<Vec<_>>();
    let registry_profiles = ProtocolRegistry::production()
        .oldest_to_newest()
        .collect::<Vec<_>>();

    assert_eq!(registry_profiles.len(), manifest_profiles.len());
    for (profile, pinned) in registry_profiles.into_iter().zip(manifest_profiles) {
        assert_eq!(profile.revision().as_str(), pinned.protocol_version);
        assert_eq!(profile.status(), McpRevisionStatus::Released);
        assert_eq!(pinned.release_status, "released");
        assert_eq!(
            profile.generation(),
            McpProtocolGeneration::InitializeHandshake
        );
        assert_eq!(pinned.handshake_family, "initialization-based");
        assert!(!pinned.pre_release_only);
    }
}

#[test]
fn pre_release_discover_generation_is_tracked_but_excluded() {
    let registry = ProtocolRegistry::production();
    let revision = McpProtocolRevision::V20260728;
    let pinned = manifest()
        .revision
        .into_iter()
        .find(|candidate| candidate.protocol_version == revision.as_str())
        .expect("draft manifest entry");
    let schema = schema(revision);
    let definitions = definitions(&schema);

    assert_eq!(revision.status(), McpRevisionStatus::ReleaseCandidate);
    assert_eq!(revision.generation(), McpProtocolGeneration::Discover);
    assert_eq!(
        McpProtocolRevision::from_str(revision.as_str()),
        Ok(revision)
    );
    assert!(!pinned.production_supported);
    assert!(pinned.pre_release_only);
    assert_eq!(pinned.release_status, "release-candidate");
    assert_eq!(pinned.handshake_family, "per-request-metadata");
    assert!(definitions.contains_key("DiscoverRequest"));
    assert!(definitions.contains_key("DiscoverResult"));
    assert!(!definitions.contains_key("InitializeRequest"));
    assert!(!definitions.contains_key("InitializedNotification"));
    assert_eq!(registry.profile(revision), None);
    assert_eq!(
        registry.parse(revision.as_str()),
        Err(McpProtocolRevisionError::NotProductionSupported(revision))
    );
}

#[test]
fn profile_feature_differences_match_the_pinned_schemas() {
    for profile in ProtocolRegistry::production().oldest_to_newest() {
        let schema = schema(profile.revision());
        let definitions = definitions(&schema);
        let messages = profile.messages();
        let tools = profile.tools();
        let initialize = profile.initialize();
        let client = profile.client();
        let schema_features = profile.schema();

        let batch_definitions_present = definitions.contains_key("JSONRPCBatchRequest")
            && definitions.contains_key("JSONRPCBatchResponse");
        assert_eq!(
            batch_definitions_present,
            json_rpc_message_allows_batch(definitions),
            "{} batch schema",
            profile.revision()
        );
        assert_eq!(
            messages.json_rpc_batching() == JsonRpcBatching::Allowed,
            batch_definitions_present,
            "{} batching",
            profile.revision()
        );
        assert_eq!(
            messages.initialized_notification(),
            InitializedNotification::AfterInitialize
        );
        assert_eq!(
            definitions
                .get("InitializedNotification")
                .and_then(|value| value.get("properties"))
                .and_then(|value| value.get("method"))
                .and_then(|value| value.get("const"))
                .and_then(Value::as_str),
            Some("notifications/initialized")
        );
        assert_eq!(
            messages.initialize_result_instructions(),
            property_names(definitions, "InitializeResult").contains("instructions")
        );
        let initialize_fields = property_names(definitions, "InitializeResult");
        assert_eq!(initialize.metadata(), initialize_fields.contains("_meta"));
        assert_eq!(
            initialize.protocol_version(),
            initialize_fields.contains("protocolVersion")
        );
        assert_eq!(
            initialize.capabilities(),
            initialize_fields.contains("capabilities")
        );
        assert_eq!(
            initialize.server_info(),
            initialize_fields.contains("serverInfo")
        );
        assert_eq!(
            initialize.instructions(),
            initialize_fields.contains("instructions")
        );
        assert!(initialize.tools_capability());

        let tool_fields = property_names(definitions, "Tool");
        let tool_result_fields = property_names(definitions, "CallToolResult");
        assert_eq!(tools.annotations(), tool_fields.contains("annotations"));
        assert_eq!(tools.output_schema(), tool_fields.contains("outputSchema"));
        assert_eq!(tools.title(), tool_fields.contains("title"));
        assert_eq!(tools.definition_metadata(), tool_fields.contains("_meta"));
        assert_eq!(
            tools.result_metadata(),
            tool_result_fields.contains("_meta")
        );
        assert_eq!(tools.is_error(), tool_result_fields.contains("isError"));
        assert_eq!(
            tools.structured_content(),
            tool_result_fields.contains("structuredContent")
        );
        match tools.result_carrier() {
            ToolResultCarrier::DirectToolResult => {
                assert!(tool_result_fields.contains("toolResult"));
                assert!(!tool_result_fields.contains("content"));
                assert!(!tool_result_fields.contains("structuredContent"));
            }
            ToolResultCarrier::JsonTextContent => {
                assert!(!tool_result_fields.contains("toolResult"));
                assert!(tool_result_fields.contains("content"));
                assert!(!tool_result_fields.contains("structuredContent"));
            }
            ToolResultCarrier::StructuredContentWithText => {
                assert!(!tool_result_fields.contains("toolResult"));
                assert!(tool_result_fields.contains("content"));
                assert!(tool_result_fields.contains("structuredContent"));
            }
        }

        assert_eq!(client.shape(), ClientCapabilitiesShape::OpenObject);
        assert_eq!(
            names(client.known_fields(), client_capability_name),
            property_names(definitions, "ClientCapabilities")
        );
        assert_eq!(
            client.known_fields(),
            schema_features.client_capability_fields()
        );
        assert_eq!(
            names(
                schema_features.server_capability_fields(),
                server_capability_name
            ),
            property_names(definitions, "ServerCapabilities")
        );
        assert_eq!(
            names(
                schema_features.tool_definition_fields(),
                tool_definition_name
            ),
            tool_fields
        );
        assert_eq!(
            names(schema_features.tool_result_fields(), tool_result_name),
            tool_result_fields
        );
        assert_eq!(
            profile.result_recovery().committed_result_recovery(),
            CommittedResultRecovery::PreserveAuthorityThenCompactResult
        );
    }
}

#[test]
fn preferred_server_revision_is_in_the_supported_set() {
    let registry = ProtocolRegistry::production();
    let preferred = registry.preferred_server_profile();

    assert_eq!(
        preferred.revision(),
        ProtocolRegistry::PREFERRED_SERVER_REVISION
    );
    assert_eq!(
        registry.profile(ProtocolRegistry::PREFERRED_SERVER_REVISION),
        Some(preferred)
    );
}

#[test]
fn production_parsing_accepts_only_exact_registry_keys() {
    let registry = ProtocolRegistry::production();
    let supported = registry
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str())
        .collect::<BTreeSet<_>>();

    for value in [
        "2024-10-07",
        "2024-11-05",
        "2025-03-26",
        "2025-06-18",
        "2025-11-25",
        "2026-07-28",
        "unsupported-revision",
        "2025-11-25-preview",
    ] {
        assert_eq!(
            registry.parse(value).is_ok(),
            supported.contains(value),
            "production support must use exact registry membership for {value}"
        );
    }
}
