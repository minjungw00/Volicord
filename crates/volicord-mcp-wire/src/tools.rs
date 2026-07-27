//! MCP tool-definition and tool-result envelope values.

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;
use volicord_mcp_protocol::{
    ClientCapabilityField, ServerCapabilityField, ToolDefinitionField, ToolResultField,
};

/// Returns the exact MCP field name for a semantic client capability.
pub const fn client_capability_field_name(field: ClientCapabilityField) -> &'static str {
    match field {
        ClientCapabilityField::Elicitation => "elicitation",
        ClientCapabilityField::Experimental => "experimental",
        ClientCapabilityField::Roots => "roots",
        ClientCapabilityField::Sampling => "sampling",
        ClientCapabilityField::Tasks => "tasks",
    }
}

/// Returns the exact MCP field name for a semantic server capability.
pub const fn server_capability_field_name(field: ServerCapabilityField) -> &'static str {
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

/// Returns the exact MCP field name for a semantic tool-definition capability.
pub const fn tool_definition_field_name(field: ToolDefinitionField) -> &'static str {
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

/// Returns the exact MCP field name for a semantic tool-result capability.
pub const fn tool_result_field_name(field: ToolResultField) -> &'static str {
    match field {
        ToolResultField::Meta => "_meta",
        ToolResultField::ToolResult => "toolResult",
        ToolResultField::Content => "content",
        ToolResultField::IsError => "isError",
        ToolResultField::StructuredContent => "structuredContent",
    }
}

/// Exact MCP annotation fields projected for one tool definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpToolAnnotations {
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

impl McpToolAnnotations {
    pub const fn read_only() -> Self {
        Self {
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }
    }

    pub const fn non_destructive_mutation() -> Self {
        Self {
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }
    }

    pub const fn destructive_mutation() -> Self {
        Self {
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: false,
        }
    }
}

/// Capability-selected MCP tool-definition envelope.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct McpToolDefinitionEnvelope(Value);

impl McpToolDefinitionEnvelope {
    pub fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

/// Capability-selected MCP tool-result envelope.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct McpToolResultEnvelope(Value);

impl McpToolResultEnvelope {
    pub fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}
