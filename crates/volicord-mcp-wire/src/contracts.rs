//! Semantic contract descriptors derived from canonical MCP wire schemas.

use std::collections::BTreeSet;

use schemars::schema_for;
use volicord_types::contracts::identifiers_from_json_schema;
use volicord_types::tool_names::AgentToolId;

use crate::methods::{mcp_request_schema, mcp_response_schema};
use crate::tools::McpToolAnnotations;

/// Exact identifiers for one stable semantic MCP wire contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireContractDescriptor {
    id: &'static str,
    identifiers: BTreeSet<String>,
    related_contracts: Vec<&'static str>,
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
}

/// Returns the MCP wire contract derived from its generated schemas.
pub fn wire_contract_descriptors() -> Vec<WireContractDescriptor> {
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
        "McpToolDefinitionEnvelope".to_owned(),
        "McpToolResultEnvelope".to_owned(),
    ]);

    for tool in AgentToolId::ALL {
        for schema in [mcp_request_schema(tool), mcp_response_schema(tool)]
            .into_iter()
            .flatten()
        {
            let schema_identifiers = identifiers_from_json_schema(&schema);
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
    }]
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
    }
}
