use crate::prelude::*;
use crate::routing::McpStorageCapability;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub fn public_method_tools() -> Vec<McpToolDefinition> {
    method_tools(PUBLIC_METHOD_TOOL_NAMES)
}

/// Returns adapter utility tool definitions.
pub fn adapter_utility_tools() -> Vec<McpToolDefinition> {
    ADAPTER_UTILITY_TOOL_NAMES
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        })
        .collect()
}

/// Returns workflow-mode MCP-visible tools.
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    mcp_tools_for_mode(AgentConnectionMode::Workflow)
}

/// Returns MCP-visible tools for the supplied Agent Connection mode.
pub fn mcp_tools_for_mode(mode: AgentConnectionMode) -> Vec<McpToolDefinition> {
    let mut tools = match mode {
        AgentConnectionMode::ReadOnly => method_tools(READ_ONLY_METHOD_TOOL_NAMES),
        AgentConnectionMode::Workflow => public_method_tools(),
    };
    tools.extend(adapter_utility_tools());
    tools
}

/// Returns MCP-visible tools for the effective connection and storage capability.
pub fn mcp_tools_for_mode_and_storage(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
) -> Vec<McpToolDefinition> {
    let mut tools = match (mode, storage_capability) {
        (_, McpStorageCapability::Unavailable) => Vec::new(),
        (AgentConnectionMode::ReadOnly, _) => method_tools(READ_ONLY_METHOD_TOOL_NAMES),
        (AgentConnectionMode::Workflow, McpStorageCapability::ReadWrite) => public_method_tools(),
        (AgentConnectionMode::Workflow, McpStorageCapability::ReadOnly)
        | (AgentConnectionMode::Workflow, McpStorageCapability::Unknown) => {
            method_tools(READ_ONLY_METHOD_TOOL_NAMES)
        }
    };
    tools.extend(adapter_utility_tools());
    tools
}

pub(crate) fn method_tools<const N: usize>(names: [&'static str; N]) -> Vec<McpToolDefinition> {
    names
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: mcp_request_schema(name).expect("MCP tool schema should exist"),
        })
        .collect()
}

pub(crate) fn tool_description(name: &str) -> &'static str {
    match name {
        INTAKE_TOOL_NAME => "Start, resume, supersede, or reject an ordinary user work loop.",
        UPDATE_SCOPE_TOOL_NAME => "Update current Task scope and Change Unit state.",
        STATUS_TOOL_NAME => "Read the current Core status view.",
        PREPARE_WRITE_TOOL_NAME => "Check one proposed product-file write against Core state.",
        STAGE_ARTIFACT_TOOL_NAME => {
            "Prepare an Evidence attachment input; staging alone is not recorded Evidence."
        }
        RECORD_RUN_TOOL_NAME => {
            "Record Evidence for a run, observation, or result, linking attachments when supplied."
        }
        REQUEST_USER_JUDGMENT_TOOL_NAME => "Create one pending focused user-owned judgment.",
        RECONCILE_CHANGES_TOOL_NAME => {
            "Reconcile unresolved unrecorded Product Repository changes."
        }
        CHECK_CLOSE_TOOL_NAME => "Check Close Status for a selected Task.",
        CLOSE_TASK_TOOL_NAME => "Perform a selected Task close path.",
        LIST_PROJECTS_TOOL_NAME => "List projects explicitly allowed for this MCP connection.",
        _ => "Unsupported Volicord method.",
    }
}
