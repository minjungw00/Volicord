use crate::routing::{
    effective_tool_mode_for_mode_and_storage, McpEffectiveToolMode, McpStorageCapability,
};
use serde::Serialize;
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use volicord_host_contract::{HostContractError, McpServerKey, McpToolCatalog};
use volicord_mcp_protocol::{McpProtocolCapabilities, ToolResultCarrier};
use volicord_mcp_wire::{
    mcp_tool_contract, McpToolAnnotations, McpToolDefinitionEnvelope, McpToolResultEnvelope,
};
#[cfg(test)]
use volicord_mcp_wire::{CanonicalSchemaExample, McpToolContractDescriptor};
use volicord_types::tool_names::{AgentToolCategory, AgentToolId, AgentToolOwner};
use volicord_types::values::{AgentConnectionMode, MethodName};

pub(crate) fn method_name_for_tool(tool_name: &str) -> Option<MethodName> {
    AgentToolId::from_wire_name(tool_name).ok()?.method()
}

#[cfg(test)]
pub(crate) const MAX_RUNTIME_TOOLS_LIST_BYTES: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CanonicalToolDefinition {
    #[serde(rename = "name")]
    pub id: AgentToolId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'static str>,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Value,
    pub annotations: McpToolAnnotations,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalContent {
    Text(String),
}

impl CanonicalContent {
    fn to_wire_value(&self) -> Value {
        match self {
            Self::Text(text) => json!({
                "type": "text",
                "text": text,
            }),
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Text(text) => text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalToolResult {
    pub metadata: Option<Map<String, Value>>,
    pub content: Vec<CanonicalContent>,
    pub structured_content: Value,
    pub is_error: bool,
}

impl CanonicalToolDefinition {
    pub(crate) fn project(
        &self,
        capabilities: McpProtocolCapabilities,
    ) -> McpToolDefinitionEnvelope {
        let tool_capabilities = capabilities.tools();
        let mut projected = Map::from_iter([
            (
                "description".to_owned(),
                Value::String(self.description.to_owned()),
            ),
            ("inputSchema".to_owned(), self.input_schema.clone()),
            (
                "name".to_owned(),
                Value::String(self.id.wire_name().to_owned()),
            ),
        ]);
        if tool_capabilities.definition_metadata() {
            if let Some(metadata) = &self.metadata {
                projected.insert("_meta".to_owned(), Value::Object(metadata.clone()));
            }
        }
        if tool_capabilities.annotations() {
            projected.insert(
                "annotations".to_owned(),
                serde_json::to_value(self.annotations)
                    .expect("canonical tool annotations should serialize"),
            );
        }
        if tool_capabilities.output_schema() {
            projected.insert("outputSchema".to_owned(), self.output_schema.clone());
        }
        if tool_capabilities.title() {
            if let Some(title) = self.title {
                projected.insert("title".to_owned(), Value::String(title.to_owned()));
            }
        }
        McpToolDefinitionEnvelope::new(Value::Object(projected))
    }
}

/// Builds the collision-checked host catalog for the complete canonical MCP registry.
pub fn canonical_mcp_tool_catalog(
    server: &McpServerKey,
) -> Result<McpToolCatalog, HostContractError> {
    McpToolCatalog::for_server(server, AgentToolId::ALL)
}

/// Builds the collision-checked host catalog for an effective `tools/list` projection.
pub fn effective_mcp_tool_catalog(
    server: &McpServerKey,
    tools: &[CanonicalToolDefinition],
) -> Result<McpToolCatalog, HostContractError> {
    McpToolCatalog::for_server(server, tools.iter().map(|tool| tool.id))
}

impl CanonicalToolResult {
    pub(crate) fn project(
        &self,
        capabilities: McpProtocolCapabilities,
    ) -> Result<McpToolResultEnvelope, serde_json::Error> {
        let tool_capabilities = capabilities.tools();
        let mut projected = Map::new();

        if tool_capabilities.result_metadata() {
            if let Some(metadata) = &self.metadata {
                projected.insert("_meta".to_owned(), Value::Object(metadata.clone()));
            }
        }

        match tool_capabilities.result_carrier() {
            ToolResultCarrier::DirectToolResult => {
                projected.insert("toolResult".to_owned(), self.structured_content.clone());
            }
            ToolResultCarrier::JsonTextContent => {
                let authoritative_text = serde_json::to_string(&self.structured_content)?;
                let mut content = vec![json!({
                    "type": "text",
                    "text": authoritative_text,
                })];
                content.extend(
                    self.content
                        .iter()
                        .filter(|item| item.text() != authoritative_text)
                        .map(CanonicalContent::to_wire_value),
                );
                projected.insert("content".to_owned(), Value::Array(content));
            }
            ToolResultCarrier::StructuredContentWithText => {
                projected.insert(
                    "content".to_owned(),
                    Value::Array(
                        self.content
                            .iter()
                            .map(CanonicalContent::to_wire_value)
                            .collect(),
                    ),
                );
                projected.insert(
                    "structuredContent".to_owned(),
                    self.structured_content.clone(),
                );
            }
        }
        if tool_capabilities.is_error() {
            projected.insert("isError".to_owned(), Value::Bool(self.is_error));
        }

        Ok(McpToolResultEnvelope::new(Value::Object(projected)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolSchemaDetail {
    RuntimeCompact,
    Documentation,
}

#[cfg(test)]
pub(crate) fn canonical_tool_examples(tool: AgentToolId) -> &'static [CanonicalSchemaExample] {
    mcp_tool_contract(tool)
        .map(McpToolContractDescriptor::canonical_examples)
        .unwrap_or_default()
}
pub fn public_method_tools() -> Vec<CanonicalToolDefinition> {
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| matches!(tool.owner(), AgentToolOwner::CoreMethod(_))),
        ToolSchemaDetail::Documentation,
    )
}

/// Returns adapter utility tool definitions.
pub fn adapter_utility_tools() -> Vec<CanonicalToolDefinition> {
    adapter_utility_tools_with_detail(ToolSchemaDetail::Documentation)
}

fn adapter_utility_tools_with_detail(detail: ToolSchemaDetail) -> Vec<CanonicalToolDefinition> {
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| matches!(tool.owner(), AgentToolOwner::AdapterUtility)),
        detail,
    )
}

/// Returns workflow-mode MCP-visible tools.
pub fn mcp_tools() -> Vec<CanonicalToolDefinition> {
    mcp_tools_for_mode(AgentConnectionMode::Workflow)
}

/// Returns MCP-visible tools for the supplied Agent Connection mode.
pub fn mcp_tools_for_mode(mode: AgentConnectionMode) -> Vec<CanonicalToolDefinition> {
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| tool.available_in(mode)),
        ToolSchemaDetail::Documentation,
    )
}

/// Returns MCP-visible tools for the effective connection and storage capability.
#[cfg(test)]
pub(crate) fn mcp_tools_for_mode_and_storage(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
) -> Vec<CanonicalToolDefinition> {
    mcp_tools_for_mode_and_storage_with_detail(
        mode,
        storage_capability,
        ToolSchemaDetail::Documentation,
    )
}

pub(crate) fn mcp_tools_for_mode_and_storage_with_detail(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
    detail: ToolSchemaDetail,
) -> Vec<CanonicalToolDefinition> {
    let effective_mode = effective_tool_mode_for_mode_and_storage(mode, storage_capability);
    tool_definitions(
        AgentToolId::ALL
            .iter()
            .copied()
            .filter(|tool| match effective_mode {
                McpEffectiveToolMode::Unavailable => {
                    matches!(tool.owner(), AgentToolOwner::AdapterUtility)
                }
                McpEffectiveToolMode::ReadOnly => tool.available_in(AgentConnectionMode::ReadOnly),
                McpEffectiveToolMode::ReadOnlyDegraded => {
                    matches!(tool.category(), AgentToolCategory::ReadOnly)
                        || matches!(tool.owner(), AgentToolOwner::ConnectionIntegration)
                        || *tool == AgentToolId::REQUEST_USER_ACTION
                }
                McpEffectiveToolMode::Workflow => tool.available_in(AgentConnectionMode::Workflow),
            }),
        detail,
    )
}

pub(crate) fn tools_list_schema_validation_status(
    tools: &[CanonicalToolDefinition],
) -> &'static str {
    if validate_tools_list_schema_compatibility(tools).is_ok() {
        "passed"
    } else {
        "failed"
    }
}

pub(crate) fn mcp_tool_naming_style(tools: &[CanonicalToolDefinition]) -> &'static str {
    if tools.is_empty() {
        return "empty";
    }
    if tools.iter().all(|tool| tool.id.wire_name().contains('.')) {
        "dotted_namespace"
    } else if tools.iter().all(|tool| !tool.id.wire_name().contains('.')) {
        "plain"
    } else {
        "mixed"
    }
}

pub(crate) fn validate_tools_list_schema_compatibility(
    tools: &[CanonicalToolDefinition],
) -> Result<(), Vec<String>> {
    let values = tools
        .iter()
        .map(|tool| serde_json::to_value(tool).expect("tool definition should serialize"))
        .collect::<Vec<_>>();
    validate_tools_list_json_compatibility(&values)
}

pub(crate) fn validate_tools_list_json_compatibility(tools: &[Value]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut names = BTreeSet::new();

    for (index, tool) in tools.iter().enumerate() {
        let Some(object) = tool.as_object() else {
            errors.push(format!("tool[{index}] is not an object"));
            continue;
        };

        let Some(name) = object.get("name").and_then(Value::as_str) else {
            errors.push(format!("tool[{index}].name is not a string"));
            continue;
        };
        if !valid_mcp_tool_name(name) {
            errors.push(format!("tool `{name}` has an MCP-incompatible name"));
        }
        if !names.insert(name.to_owned()) {
            errors.push(format!("tool `{name}` is duplicated"));
        }
        if object
            .get("description")
            .is_none_or(|description| description.as_str().is_none_or(|text| text.is_empty()))
        {
            errors.push(format!("tool `{name}` description is missing or empty"));
        }

        match object.get("inputSchema") {
            Some(input_schema) => {
                validate_root_object_schema(name, "inputSchema", input_schema, &mut errors)
            }
            None => errors.push(format!("tool `{name}` is missing inputSchema")),
        }
        match object.get("outputSchema") {
            Some(output_schema) => {
                validate_root_object_schema(name, "outputSchema", output_schema, &mut errors)
            }
            None => errors.push(format!("tool `{name}` is missing outputSchema")),
        }
        match object.get("annotations") {
            Some(annotations) => validate_annotations(name, annotations, &mut errors),
            None => errors.push(format!("tool `{name}` is missing annotations")),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn tool_definitions(
    tools: impl IntoIterator<Item = AgentToolId>,
    detail: ToolSchemaDetail,
) -> Vec<CanonicalToolDefinition> {
    tools
        .into_iter()
        .map(|id| {
            let contract = mcp_tool_contract(id).expect("MCP semantic contract should exist");
            CanonicalToolDefinition {
                id,
                title: None,
                description: match detail {
                    ToolSchemaDetail::RuntimeCompact => contract.compact_description(),
                    ToolSchemaDetail::Documentation => contract.documentation_description(),
                },
                input_schema: mcp_tool_input_schema_with_detail(id, detail)
                    .expect("MCP tool schema should exist"),
                output_schema: match detail {
                    ToolSchemaDetail::RuntimeCompact => contract.compact_output_schema(),
                    ToolSchemaDetail::Documentation => contract.output_schema(),
                },
                annotations: tool_annotations(id),
                metadata: None,
            }
        })
        .collect()
}

fn mcp_tool_input_schema_with_detail(tool: AgentToolId, detail: ToolSchemaDetail) -> Option<Value> {
    let mut schema = mcp_tool_contract(tool)?.input_schema();
    match detail {
        ToolSchemaDetail::RuntimeCompact => compact_runtime_schema(&mut schema),
        ToolSchemaDetail::Documentation => {}
    }
    Some(schema)
}

pub(crate) fn compact_runtime_schema(schema: &mut Value) {
    // Keep the draft marker and validation semantics. Runtime compaction
    // removes annotations and redundant constraints, drops unreachable
    // definitions, and rewrites only local definition references.
    strip_schema_presentation_annotations(schema);
    prune_unreferenced_definitions(schema);
    inline_single_use_definitions(schema);
    compact_definition_names(schema);
}

fn strip_schema_presentation_annotations(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    for annotation in [
        "$comment",
        "default",
        "deprecated",
        "description",
        "examples",
        "readOnly",
        "title",
        "writeOnly",
    ] {
        object.remove(annotation);
    }
    if enum_makes_type_redundant(object) {
        object.remove("type");
    }

    for keyword in [
        "additionalItems",
        "additionalProperties",
        "contains",
        "contentSchema",
        "else",
        "if",
        "not",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    ] {
        if let Some(child) = object.get_mut(keyword) {
            strip_schema_presentation_annotations(child);
        }
    }
    if let Some(items) = object.get_mut("items") {
        match items {
            Value::Array(items) => {
                for item in items {
                    strip_schema_presentation_annotations(item);
                }
            }
            item => strip_schema_presentation_annotations(item),
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(items) = object.get_mut(keyword).and_then(Value::as_array_mut) {
            for item in items {
                strip_schema_presentation_annotations(item);
            }
        }
    }
    for keyword in [
        "$defs",
        "definitions",
        "dependentSchemas",
        "patternProperties",
        "properties",
    ] {
        if let Some(children) = object.get_mut(keyword).and_then(Value::as_object_mut) {
            for child in children.values_mut() {
                strip_schema_presentation_annotations(child);
            }
        }
    }
    if let Some(dependencies) = object
        .get_mut("dependencies")
        .and_then(Value::as_object_mut)
    {
        for dependency in dependencies.values_mut() {
            if dependency.is_object() {
                strip_schema_presentation_annotations(dependency);
            }
        }
    }
}

fn enum_makes_type_redundant(schema: &Map<String, Value>) -> bool {
    let Some(values) = schema.get("enum").and_then(Value::as_array) else {
        return false;
    };
    if values.is_empty() {
        return false;
    }
    let schema_types = match schema.get("type") {
        Some(Value::String(schema_type)) if recognized_schema_type(schema_type) => {
            vec![schema_type.as_str()]
        }
        Some(Value::Array(schema_types)) => {
            let schema_type_names = schema_types
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            if schema_type_names.len() != schema_types.len() {
                return false;
            }
            schema_type_names
        }
        _ => return false,
    };
    !schema_types.is_empty()
        && schema_types
            .iter()
            .all(|schema_type| recognized_schema_type(schema_type))
        && values.iter().all(|value| {
            schema_types
                .iter()
                .any(|schema_type| value_matches_schema_type(value, schema_type))
        })
}

fn recognized_schema_type(schema_type: &str) -> bool {
    matches!(
        schema_type,
        "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
    )
}

fn value_matches_schema_type(value: &Value, schema_type: &str) -> bool {
    match schema_type {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => false,
    }
}

fn prune_unreferenced_definitions(schema: &mut Value) {
    let Some(definitions) = schema
        .get("definitions")
        .and_then(Value::as_object)
        .cloned()
    else {
        return;
    };
    let mut pending = Vec::new();
    if let Some(root) = schema.as_object() {
        for (keyword, child) in root {
            if keyword != "definitions" {
                collect_definition_refs(child, &mut pending);
            }
        }
    }
    let mut reachable = BTreeSet::new();
    let mut index = 0;
    while index < pending.len() {
        let name = pending[index].clone();
        index += 1;
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(definition) = definitions.get(&name) {
            collect_definition_refs(definition, &mut pending);
        }
    }

    if let Some(definitions) = schema
        .as_object_mut()
        .and_then(|object| object.get_mut("definitions"))
        .and_then(Value::as_object_mut)
    {
        definitions.retain(|name, _| reachable.contains(name));
    }
    remove_empty_definitions(schema);
}

fn collect_definition_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
            {
                refs.push(name.to_owned());
            }
            for child in object.values() {
                collect_definition_refs(child, refs);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_definition_refs(child, refs);
            }
        }
        _ => {}
    }
}

fn inline_single_use_definitions(schema: &mut Value) {
    loop {
        let Some(definitions) = schema
            .get("definitions")
            .and_then(Value::as_object)
            .cloned()
        else {
            return;
        };
        let mut counts = BTreeMap::<String, usize>::new();
        count_definition_refs(schema, &mut counts);
        let candidate = definitions
            .iter()
            .find(|(name, definition)| {
                counts.get(*name).copied() == Some(1)
                    && !value_references_definition(definition, name)
            })
            .map(|(name, definition)| (name.clone(), definition.clone()));
        let Some((name, definition)) = candidate else {
            break;
        };

        let replaced = replace_one_definition_ref(schema, &name, &definition);
        debug_assert!(replaced);
        if !replaced {
            break;
        }
        if let Some(definitions) = schema
            .as_object_mut()
            .and_then(|object| object.get_mut("definitions"))
            .and_then(Value::as_object_mut)
        {
            definitions.remove(&name);
        }
    }
    remove_empty_definitions(schema);
}

fn count_definition_refs(value: &Value, counts: &mut BTreeMap<String, usize>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
            {
                *counts.entry(name.to_owned()).or_default() += 1;
            }
            for child in object.values() {
                count_definition_refs(child, counts);
            }
        }
        Value::Array(items) => {
            for child in items {
                count_definition_refs(child, counts);
            }
        }
        _ => {}
    }
}

fn value_references_definition(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
                == Some(name)
                || object
                    .values()
                    .any(|child| value_references_definition(child, name))
        }
        Value::Array(items) => items
            .iter()
            .any(|child| value_references_definition(child, name)),
        _ => false,
    }
}

fn replace_one_definition_ref(value: &mut Value, name: &str, definition: &Value) -> bool {
    match value {
        Value::Object(object) => {
            if object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(definition_name_from_ref)
                == Some(name)
            {
                *value = definition.clone();
                return true;
            }
            for child in object.values_mut() {
                if replace_one_definition_ref(child, name, definition) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => {
            for child in items {
                if replace_one_definition_ref(child, name, definition) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn compact_definition_names(schema: &mut Value) {
    let names = schema
        .get("definitions")
        .and_then(Value::as_object)
        .map(|definitions| definitions.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let aliases = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, base36(index)))
        .collect::<BTreeMap<_, _>>();
    replace_definition_refs(schema, &aliases);

    let Some(definitions) = schema
        .as_object_mut()
        .and_then(|object| object.get_mut("definitions"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let original = std::mem::take(definitions);
    for (name, definition) in original {
        definitions.insert(
            aliases
                .get(&name)
                .expect("every retained definition should have a compact alias")
                .clone(),
            definition,
        );
    }
}

fn replace_definition_refs(value: &mut Value, aliases: &BTreeMap<String, String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                if let Some(name) = definition_name_from_ref(reference) {
                    if let Some(alias) = aliases.get(name) {
                        *reference = format!("#/definitions/{alias}");
                    }
                }
            }
            for child in object.values_mut() {
                replace_definition_refs(child, aliases);
            }
        }
        Value::Array(items) => {
            for child in items {
                replace_definition_refs(child, aliases);
            }
        }
        _ => {}
    }
}

fn definition_name_from_ref(reference: &str) -> Option<&str> {
    reference.strip_prefix("#/definitions/")
}

fn remove_empty_definitions(schema: &mut Value) {
    if schema
        .get("definitions")
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty)
    {
        schema
            .as_object_mut()
            .expect("generated schema should be an object")
            .remove("definitions");
    }
}

fn base36(mut value: usize) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let mut digits = Vec::new();
    while value > 0 {
        let digit = value % 36;
        digits.push(if digit < 10 {
            char::from(b'0' + digit as u8)
        } else {
            char::from(b'a' + (digit - 10) as u8)
        });
        value /= 36;
    }
    digits.iter().rev().collect()
}

fn tool_annotations(tool: AgentToolId) -> McpToolAnnotations {
    let mut annotations = match tool.category() {
        AgentToolCategory::ReadOnly => McpToolAnnotations::read_only(),
        AgentToolCategory::NonDestructiveMutation => McpToolAnnotations::non_destructive_mutation(),
        AgentToolCategory::DestructiveMutation => McpToolAnnotations::destructive_mutation(),
    };
    if tool.is_idempotent() {
        annotations.idempotent_hint = true;
    }
    annotations
}

fn valid_mcp_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn validate_root_object_schema(
    tool_name: &str,
    schema_name: &str,
    schema: &Value,
    errors: &mut Vec<String>,
) {
    let Some(object) = schema.as_object() else {
        errors.push(format!("tool `{tool_name}` {schema_name} is not an object"));
        return;
    };

    match object.get("type") {
        Some(Value::String(schema_type)) if schema_type == "object" => {}
        Some(_) => errors.push(format!(
            "tool `{tool_name}` {schema_name} root type is not object"
        )),
        None => errors.push(format!(
            "tool `{tool_name}` {schema_name} root type is missing"
        )),
    }

    validate_schema_fragment(tool_name, schema_name, schema, errors);
}

fn validate_annotations(tool_name: &str, annotations: &Value, errors: &mut Vec<String>) {
    let Some(object) = annotations.as_object() else {
        errors.push(format!("tool `{tool_name}` annotations is not an object"));
        return;
    };
    for field in [
        "readOnlyHint",
        "destructiveHint",
        "idempotentHint",
        "openWorldHint",
    ] {
        if object.get(field).is_none_or(|value| !value.is_boolean()) {
            errors.push(format!(
                "tool `{tool_name}` annotations.{field} is not a boolean"
            ));
        }
    }
    for field in object.keys() {
        if !matches!(
            field.as_str(),
            "readOnlyHint" | "destructiveHint" | "idempotentHint" | "openWorldHint"
        ) {
            errors.push(format!(
                "tool `{tool_name}` annotations contains unsupported field `{field}`"
            ));
        }
    }
}

fn validate_schema_fragment(tool_name: &str, path: &str, schema: &Value, errors: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        errors.push(format!("tool `{tool_name}` {path} schema is not an object"));
        return;
    };

    for keyword in [
        "patternProperties",
        "unevaluatedProperties",
        "dependentSchemas",
        "$dynamicRef",
        "contains",
    ] {
        if object.contains_key(keyword) {
            errors.push(format!(
                "tool `{tool_name}` {path} uses unsupported schema keyword `{keyword}`"
            ));
        }
    }

    if let Some(schema_uri) = object.get("$schema") {
        if schema_uri.as_str().is_none_or(|uri| uri.is_empty()) {
            errors.push(format!("tool `{tool_name}` {path} has invalid $schema"));
        }
    }
    if let Some(reference) = object.get("$ref") {
        match reference.as_str() {
            Some(value) if value.starts_with("#/") => {}
            _ => errors.push(format!("tool `{tool_name}` {path} uses a non-local $ref")),
        }
    }
    if let Some(schema_type) = object.get("type") {
        validate_schema_type(tool_name, path, schema_type, errors);
    }
    if let Some(enum_values) = object.get("enum") {
        if enum_values
            .as_array()
            .is_none_or(|values| values.is_empty())
        {
            errors.push(format!(
                "tool `{tool_name}` {path} enum is not a non-empty array"
            ));
        }
    }
    if let Some(format) = object.get("format") {
        if format.as_str().is_none_or(|value| value.is_empty()) {
            errors.push(format!("tool `{tool_name}` {path} format is not a string"));
        }
    }
    if let Some(required) = object.get("required") {
        validate_required_fields(tool_name, path, object, required, errors);
    }
    if let Some(properties) = object.get("properties") {
        validate_properties(tool_name, path, properties, errors);
    }
    if let Some(items) = object.get("items") {
        validate_items(tool_name, path, items, errors);
    }
    if let Some(additional_properties) = object.get("additionalProperties") {
        validate_additional_properties(tool_name, path, additional_properties, errors);
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(combinator) {
            validate_schema_branches(tool_name, path, combinator, branches, errors);
        }
    }
    for definitions_key in ["definitions", "$defs"] {
        if let Some(definitions) = object.get(definitions_key) {
            validate_definitions(tool_name, path, definitions_key, definitions, errors);
        }
    }
}

fn validate_schema_branches(
    tool_name: &str,
    path: &str,
    combinator: &str,
    branches: &Value,
    errors: &mut Vec<String>,
) {
    let Some(branches) = branches.as_array() else {
        errors.push(format!(
            "tool `{tool_name}` {path}.{combinator} is not an array"
        ));
        return;
    };
    if branches.is_empty() {
        errors.push(format!("tool `{tool_name}` {path}.{combinator} is empty"));
    }
    for (index, branch) in branches.iter().enumerate() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.{combinator}[{index}]"),
            branch,
            errors,
        );
    }
}

fn validate_schema_type(
    tool_name: &str,
    path: &str,
    schema_type: &Value,
    errors: &mut Vec<String>,
) {
    match schema_type {
        Value::String(value) if valid_json_schema_type(value) => {}
        Value::Array(values)
            if !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(valid_json_schema_type)) => {}
        _ => errors.push(format!("tool `{tool_name}` {path} has invalid type")),
    }
}

fn valid_json_schema_type(value: &str) -> bool {
    matches!(
        value,
        "null" | "boolean" | "object" | "array" | "number" | "string" | "integer"
    )
}

fn validate_required_fields(
    tool_name: &str,
    path: &str,
    object: &Map<String, Value>,
    required: &Value,
    errors: &mut Vec<String>,
) {
    let Some(required_values) = required.as_array() else {
        errors.push(format!(
            "tool `{tool_name}` {path} required is not an array"
        ));
        return;
    };
    let properties = object.get("properties").and_then(Value::as_object);
    let mut seen = BTreeSet::new();
    for value in required_values {
        let Some(field) = value.as_str() else {
            errors.push(format!(
                "tool `{tool_name}` {path} required contains a non-string value"
            ));
            continue;
        };
        if !seen.insert(field.to_owned()) {
            errors.push(format!(
                "tool `{tool_name}` {path} required duplicates `{field}`"
            ));
        }
        if !properties.is_some_and(|properties| properties.contains_key(field)) {
            errors.push(format!(
                "tool `{tool_name}` {path} requires unknown property `{field}`"
            ));
        }
    }
}

fn validate_properties(tool_name: &str, path: &str, properties: &Value, errors: &mut Vec<String>) {
    let Some(properties) = properties.as_object() else {
        errors.push(format!(
            "tool `{tool_name}` {path} properties is not an object"
        ));
        return;
    };
    for (property_name, property_schema) in properties {
        if property_name.is_empty() {
            errors.push(format!(
                "tool `{tool_name}` {path} has an empty property name"
            ));
        }
        validate_schema_fragment(
            tool_name,
            &format!("{path}.properties.{property_name}"),
            property_schema,
            errors,
        );
    }
}

fn validate_items(tool_name: &str, path: &str, items: &Value, errors: &mut Vec<String>) {
    if let Some(item_schema) = items.as_object() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.items"),
            &Value::Object(item_schema.clone()),
            errors,
        );
    } else if let Some(item_schemas) = items.as_array() {
        for (index, item_schema) in item_schemas.iter().enumerate() {
            validate_schema_fragment(
                tool_name,
                &format!("{path}.items[{index}]"),
                item_schema,
                errors,
            );
        }
    } else {
        errors.push(format!("tool `{tool_name}` {path} items is not a schema"));
    }
}

fn validate_additional_properties(
    tool_name: &str,
    path: &str,
    additional_properties: &Value,
    errors: &mut Vec<String>,
) {
    if additional_properties.is_boolean() {
        return;
    }
    if additional_properties.is_object() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.additionalProperties"),
            additional_properties,
            errors,
        );
        return;
    }
    errors.push(format!(
        "tool `{tool_name}` {path} additionalProperties is not boolean or schema"
    ));
}

fn validate_definitions(
    tool_name: &str,
    path: &str,
    definitions_key: &str,
    definitions: &Value,
    errors: &mut Vec<String>,
) {
    let Some(definitions) = definitions.as_object() else {
        errors.push(format!(
            "tool `{tool_name}` {path}.{definitions_key} is not an object"
        ));
        return;
    };
    for (definition_name, definition_schema) in definitions {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.{definitions_key}.{definition_name}"),
            definition_schema,
            errors,
        );
    }
}
