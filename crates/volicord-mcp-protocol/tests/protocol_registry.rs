use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;
use volicord_mcp_protocol::{
    ClientCapabilityField, InitializedNotification, JsonRpcBatching, McpNegotiationOutcome,
    McpProtocolGeneration, McpProtocolRevision, McpProtocolRevisionError, McpRevisionStatus,
    ProtocolRegistry, ServerCapabilityField, ToolDefinitionField, ToolResultField,
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
    volicord_conformance_covered: bool,
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
        let selection = registry
            .negotiate_initialize(profile.revision().as_str())
            .expect("production initialize revision should negotiate");

        assert_eq!(selection.profile(), profile);
        assert_eq!(selection.outcome(), McpNegotiationOutcome::ExactMatch);
    }
}

#[test]
fn unknown_initialize_revision_receives_the_preferred_server_counter_offer() {
    let registry = ProtocolRegistry::production();

    for requested in ["", "2025-01-01", "future-initialize-revision"] {
        let selection = registry
            .negotiate_initialize(requested)
            .expect("a string protocol version is negotiated by server selection");

        assert_eq!(selection.profile(), registry.preferred_server_profile());
        assert_eq!(
            selection.outcome(),
            McpNegotiationOutcome::ServerCounterOffer
        );
    }
}

#[test]
fn discover_generation_is_not_counter_offered_as_initialize() {
    let registry = ProtocolRegistry::production();
    let mismatch = registry
        .negotiate_initialize(McpProtocolRevision::V20260728.as_str())
        .expect_err("discover-based traffic must not enter initialize negotiation");

    assert_eq!(mismatch.revision(), McpProtocolRevision::V20260728);
    assert_eq!(mismatch.actual(), McpProtocolGeneration::Discover);
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
        assert!(pinned.volicord_conformance_covered);
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
    assert!(!pinned.volicord_conformance_covered);
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

        let tool_fields = property_names(definitions, "Tool");
        let tool_result_fields = property_names(definitions, "CallToolResult");
        assert_eq!(tools.annotations(), tool_fields.contains("annotations"));
        assert_eq!(tools.output_schema(), tool_fields.contains("outputSchema"));
        assert_eq!(
            tools.structured_content(),
            tool_result_fields.contains("structuredContent")
        );

        assert_eq!(
            names(
                schema_features.client_capability_fields(),
                ClientCapabilityField::as_str
            ),
            property_names(definitions, "ClientCapabilities")
        );
        assert_eq!(
            names(
                schema_features.server_capability_fields(),
                ServerCapabilityField::as_str
            ),
            property_names(definitions, "ServerCapabilities")
        );
        assert_eq!(
            names(
                schema_features.tool_definition_fields(),
                ToolDefinitionField::as_str
            ),
            tool_fields
        );
        assert_eq!(
            names(
                schema_features.tool_result_fields(),
                ToolResultField::as_str
            ),
            tool_result_fields
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
fn property_production_parsing_never_accepts_revision_date_ranges() {
    let registry = ProtocolRegistry::production();
    let supported = registry
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str())
        .collect::<BTreeSet<_>>();

    for year in 2023..=2027 {
        for month in 1..=12 {
            for day in 1..=31 {
                let value = format!("{year:04}-{month:02}-{day:02}");
                assert_eq!(
                    registry.parse(&value).is_ok(),
                    supported.contains(value.as_str()),
                    "production support must use exact registry membership for {value}"
                );
            }
        }
    }
}
