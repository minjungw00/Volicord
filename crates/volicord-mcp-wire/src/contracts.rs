//! Semantic contract descriptors derived from canonical MCP wire schemas.

use std::collections::{BTreeMap, BTreeSet};

use schemars::schema_for;
use serde_json::Value;
use volicord_types::contracts::{identifiers_from_json_schema, JsonExampleShape};

use crate::tool_contracts::mcp_tool_contracts;
use crate::{McpToolAnnotations, SemanticSchemaNode};

/// Exact identifiers for one stable semantic MCP wire contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireContractDescriptor {
    id: &'static str,
    identifiers: BTreeSet<String>,
    related_contracts: Vec<&'static str>,
    example_schemas: BTreeMap<String, Value>,
}

impl WireContractDescriptor {
    pub const fn id(&self) -> &'static str {
        self.id
    }

    pub const fn identifiers(&self) -> &BTreeSet<String> {
        &self.identifiers
    }

    pub fn related_contracts(&self) -> &[&'static str] {
        &self.related_contracts
    }

    /// Returns every exact MCP request or response shape exposed by this owner.
    pub const fn example_schemas(&self) -> &BTreeMap<String, Value> {
        &self.example_schemas
    }
}

/// Returns the MCP wire catalog projected from the canonical tool descriptors.
pub fn wire_contract_descriptors() -> Vec<WireContractDescriptor> {
    let mut example_schemas = BTreeMap::new();
    let mut identifiers = BTreeSet::from([
        "jsonrpc".to_owned(),
        "protocolVersion".to_owned(),
        "serverInfo".to_owned(),
        "clientInfo".to_owned(),
        "_meta".to_owned(),
        "annotations".to_owned(),
        "inputSchema".to_owned(),
        "outputSchema".to_owned(),
        "toolResult".to_owned(),
        "structuredContent".to_owned(),
        "isError".to_owned(),
        "readOnlyHint".to_owned(),
        "destructiveHint".to_owned(),
        "idempotentHint".to_owned(),
        "openWorldHint".to_owned(),
        "McpToolStructuredContent".to_owned(),
        "McpReadOnlyToolStructuredContent".to_owned(),
        "McpOperationalFailure".to_owned(),
        "McpToolDefinitionEnvelope".to_owned(),
        "McpToolResultEnvelope".to_owned(),
    ]);

    for contract in mcp_tool_contracts() {
        let input_schema = contract.input_schema();
        identifiers.extend(
            contract
                .input_descriptor()
                .definitions()
                .keys()
                .filter(|name| name.starts_with("Mcp"))
                .cloned(),
        );
        extend_semantic_identifiers(&mut identifiers, contract.input_descriptor().node());
        for node in contract.input_descriptor().definitions().values() {
            extend_semantic_identifiers(&mut identifiers, node);
        }
        example_schemas.insert(
            JsonExampleShape::McpWireRequest(contract.tool().wire_name().to_owned()).id(),
            input_schema.clone(),
        );
        extend_wire_identifiers(&mut identifiers, &input_schema);

        let output_schema = contract.output_schema();
        identifiers.extend(
            contract
                .output_descriptor()
                .definitions()
                .keys()
                .filter(|name| name.starts_with("Mcp"))
                .cloned(),
        );
        extend_semantic_identifiers(&mut identifiers, contract.output_descriptor().node());
        for node in contract.output_descriptor().definitions().values() {
            extend_semantic_identifiers(&mut identifiers, node);
        }
        example_schemas.insert(
            JsonExampleShape::McpWireResponse(contract.tool().wire_name().to_owned()).id(),
            output_schema.clone(),
        );
        extend_wire_identifiers(&mut identifiers, &output_schema);
    }

    let annotations = serde_json::to_value(schema_for!(McpToolAnnotations))
        .expect("MCP tool annotation schema should serialize");
    let annotation_identifiers = identifiers_from_json_schema(&annotations);
    identifiers.extend(
        annotation_identifiers
            .schema_names()
            .iter()
            .filter(|name| name.starts_with("Mcp"))
            .cloned(),
    );

    vec![WireContractDescriptor {
        id: "mcp.wire",
        identifiers,
        related_contracts: vec!["mcp.protocol"],
        example_schemas,
    }]
}

fn extend_semantic_identifiers(identifiers: &mut BTreeSet<String>, node: &SemanticSchemaNode) {
    let semantic_type = node.semantic_type_name();
    if semantic_type.starts_with("Mcp") {
        identifiers.insert(semantic_type);
    }
    match node {
        SemanticSchemaNode::Object(object) => {
            for field in &object.fields {
                if field.semantic_type.starts_with("Mcp") {
                    identifiers.insert(field.semantic_type.clone());
                }
                extend_semantic_identifiers(identifiers, &field.schema);
            }
        }
        SemanticSchemaNode::Array(array) => {
            extend_semantic_identifiers(identifiers, &array.items);
        }
        SemanticSchemaNode::Nullable(nullable) => {
            extend_semantic_identifiers(identifiers, &nullable.schema);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            for variant in &union.variants {
                if variant.semantic_type.starts_with("Mcp") {
                    identifiers.insert(variant.semantic_type.clone());
                }
                extend_semantic_identifiers(identifiers, &variant.schema);
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                extend_semantic_identifiers(identifiers, variant);
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                extend_semantic_identifiers(identifiers, schema);
            }
        }
        SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_)
        | SemanticSchemaNode::Reference(_) => {}
    }
}

fn extend_wire_identifiers(identifiers: &mut BTreeSet<String>, schema: &Value) {
    let schema_identifiers = identifiers_from_json_schema(schema);
    identifiers.extend(
        schema_identifiers
            .values()
            .iter()
            .filter(|value| value.starts_with("MCP_"))
            .cloned(),
    );
    identifiers.extend(
        schema_identifiers
            .schema_names()
            .iter()
            .filter(|name| name.starts_with("Mcp"))
            .cloned(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_resolves_wire_identifiers_from_the_wire_owner() {
        let descriptor = wire_contract_descriptors()
            .into_iter()
            .find(|descriptor| descriptor.id() == "mcp.wire")
            .expect("MCP wire descriptor");

        for identifier in [
            "MCP_UNAVAILABLE",
            "McpOperationalFailure",
            "McpReadOnlyToolStructuredContent",
            "structuredContent",
            "readOnlyHint",
        ] {
            assert!(
                descriptor.identifiers().contains(identifier),
                "MCP wire descriptor should own {identifier}"
            );
        }
        for shape in [
            "mcp_request.volicord.status",
            "mcp_response.volicord.status",
        ] {
            assert!(
                descriptor.example_schemas().contains_key(shape),
                "MCP wire descriptor should expose {shape}"
            );
        }
    }
}
