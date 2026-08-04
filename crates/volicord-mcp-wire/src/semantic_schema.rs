//! Canonical semantic schema descriptors for MCP wire contracts.

use std::collections::{BTreeMap, BTreeSet};

use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde_json::{Map, Value};

/// One typed canonical input example owned by a semantic descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalSchemaExample {
    id: &'static str,
    description: &'static str,
    value: Value,
    expected_variants: Vec<ExpectedTaggedVariant>,
}

impl CanonicalSchemaExample {
    /// Serializes a typed Rust value into one canonical JSON example.
    pub fn from_typed<T: Serialize>(
        id: &'static str,
        description: &'static str,
        value: &T,
        expected_variants: Vec<ExpectedTaggedVariant>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id,
            description,
            value: serde_json::to_value(value)?,
            expected_variants,
        })
    }

    pub const fn id(&self) -> &'static str {
        self.id
    }

    pub const fn description(&self) -> &'static str {
        self.description
    }

    pub const fn value(&self) -> &Value {
        &self.value
    }

    pub fn expected_variants(&self) -> &[ExpectedTaggedVariant] {
        &self.expected_variants
    }
}

/// Expected tagged-union selection within one typed example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedTaggedVariant {
    pub instance_path: &'static str,
    pub discriminator_path: &'static str,
    pub discriminator_value: &'static str,
    pub semantic_type: &'static str,
}

/// Canonical semantic schema descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticSchemaDescriptor {
    semantic_type: String,
    node: SemanticSchemaNode,
    definitions: BTreeMap<String, SemanticSchemaNode>,
    canonical_examples: Vec<CanonicalSchemaExample>,
    dialect: Option<String>,
}

impl SemanticSchemaDescriptor {
    /// Projects a Rust wire type into the closed semantic representation.
    pub fn for_type<T: JsonSchema>(canonical_examples: Vec<CanonicalSchemaExample>) -> Self {
        let schema = serde_json::to_value(schema_for!(T))
            .unwrap_or_else(|error| panic!("{} schema must serialize: {error}", T::schema_name()));
        Self::from_json_schema(T::schema_name(), schema, canonical_examples)
    }

    /// Projects an MCP structured output whose JSON root is always an object.
    pub fn for_object_output<T: JsonSchema>(
        canonical_examples: Vec<CanonicalSchemaExample>,
    ) -> Self {
        let mut descriptor = Self::for_type::<T>(canonical_examples);
        descriptor
            .node_metadata_mut()
            .validation
            .insert("type".to_owned(), Value::String("object".to_owned()));
        descriptor
    }

    fn node_metadata_mut(&mut self) -> &mut SemanticNodeMetadata {
        match &mut self.node {
            SemanticSchemaNode::Object(schema) => &mut schema.metadata,
            SemanticSchemaNode::Array(schema) => &mut schema.metadata,
            SemanticSchemaNode::String(metadata)
            | SemanticSchemaNode::Integer(metadata)
            | SemanticSchemaNode::Number(metadata)
            | SemanticSchemaNode::Boolean(metadata)
            | SemanticSchemaNode::Null(metadata) => metadata,
            SemanticSchemaNode::Nullable(schema) => &mut schema.metadata,
            SemanticSchemaNode::Enum(schema) => &mut schema.metadata,
            SemanticSchemaNode::Reference(schema) => &mut schema.metadata,
            SemanticSchemaNode::TaggedUnion(schema) => &mut schema.metadata,
            SemanticSchemaNode::Union(schema) => &mut schema.metadata,
            SemanticSchemaNode::AllOf(schema) => &mut schema.metadata,
        }
    }

    fn from_json_schema(
        semantic_type: String,
        mut schema: Value,
        canonical_examples: Vec<CanonicalSchemaExample>,
    ) -> Self {
        let dialect = schema
            .as_object_mut()
            .and_then(|object| object.remove("$schema"))
            .and_then(|value| value.as_str().map(str::to_owned));
        let raw_definitions = schema
            .as_object_mut()
            .and_then(|object| object.remove("definitions"))
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let definitions = raw_definitions
            .iter()
            .map(|(name, schema)| {
                (
                    name.clone(),
                    parse_schema_node(schema, name, &raw_definitions),
                )
            })
            .collect();
        let node = parse_schema_node(&schema, &semantic_type, &raw_definitions);
        let mut descriptor = Self {
            semantic_type,
            node,
            definitions,
            canonical_examples,
            dialect,
        };
        let variant_examples = descriptor
            .canonical_examples
            .iter()
            .flat_map(|example| {
                example.expected_variants.iter().filter_map(|expected| {
                    example
                        .value
                        .pointer(expected.instance_path)
                        .cloned()
                        .map(|value| (expected.clone(), value))
                })
            })
            .collect::<Vec<_>>();
        for (expected, value) in variant_examples {
            attach_tagged_variant_example(&mut descriptor.node, &expected, &value);
            for node in descriptor.definitions.values_mut() {
                attach_tagged_variant_example(node, &expected, &value);
            }
        }
        descriptor
    }

    pub fn semantic_type(&self) -> &str {
        &self.semantic_type
    }

    pub const fn node(&self) -> &SemanticSchemaNode {
        &self.node
    }

    pub const fn definitions(&self) -> &BTreeMap<String, SemanticSchemaNode> {
        &self.definitions
    }

    pub fn canonical_examples(&self) -> &[CanonicalSchemaExample] {
        &self.canonical_examples
    }

    /// Generates deterministic MCP JSON Schema from this descriptor.
    pub fn json_schema(&self) -> Value {
        let mut schema = self.node.to_json_schema();
        let object = schema
            .as_object_mut()
            .expect("semantic schema root must render as an object");
        if let Some(dialect) = &self.dialect {
            object.insert("$schema".to_owned(), Value::String(dialect.clone()));
        }
        if !self.definitions.is_empty() {
            object.insert(
                "definitions".to_owned(),
                Value::Object(
                    self.definitions
                        .iter()
                        .map(|(name, node)| (name.clone(), node.to_json_schema()))
                        .collect(),
                ),
            );
        }
        if !self.canonical_examples.is_empty() {
            object.insert(
                "examples".to_owned(),
                Value::Array(
                    self.canonical_examples
                        .iter()
                        .map(|example| example.value.clone())
                        .collect(),
                ),
            );
        }
        schema
    }

    /// Generates the bounded root-object projection used by runtime tool discovery.
    pub fn compact_root_object_schema(&self) -> Value {
        let mut object = Map::new();
        object.insert("type".to_owned(), Value::String("object".to_owned()));
        Value::Object(object)
    }

    /// Validates one JSON value against the semantic validator tree.
    pub fn validate(&self, value: &Value) -> SemanticValidationResult {
        let mut result = SemanticValidationResult::default();
        validate_node(
            &self.node,
            &self.definitions,
            value,
            "",
            0,
            None,
            &mut result,
        );
        result
    }

    /// Resolves descriptor-owned metadata for one instance JSON Pointer.
    pub fn metadata_at_instance_path(&self, path: &str) -> Option<SemanticPathMetadata> {
        metadata_at_path(&self.node, &self.definitions, path)
    }

    /// Checks structural integrity without selecting a best-effort branch.
    pub fn integrity_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        validate_node_integrity(
            &self.node,
            &self.definitions,
            "#",
            &BTreeSet::new(),
            &mut errors,
        );
        for (name, node) in &self.definitions {
            validate_node_integrity(
                node,
                &self.definitions,
                &format!("#/definitions/{name}"),
                &BTreeSet::new(),
                &mut errors,
            );
        }
        for example in &self.canonical_examples {
            let validation = self.validate(example.value());
            if !validation.issues.is_empty() {
                errors.push(format!(
                    "example `{}` does not validate against `{}`: {}",
                    example.id,
                    self.semantic_type,
                    validation
                        .issues
                        .iter()
                        .map(|issue| format!("{} {}", issue.path, issue.message))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            for expected in &example.expected_variants {
                match selected_tagged_variant(
                    &self.node,
                    &self.definitions,
                    example.value(),
                    expected.instance_path,
                    expected.discriminator_path,
                ) {
                    Some((value, semantic_type))
                        if value == expected.discriminator_value
                            && semantic_type == expected.semantic_type => {}
                    Some((value, semantic_type)) => errors.push(format!(
                        "example `{}` selected `{value}`/`{semantic_type}` instead of `{}`/`{}`",
                        example.id, expected.discriminator_value, expected.semantic_type
                    )),
                    None => errors.push(format!(
                        "example `{}` did not select tagged union {} at {}",
                        example.id, expected.discriminator_path, expected.instance_path
                    )),
                }
            }
        }
        errors
    }
}

/// Closed semantic schema node vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticSchemaNode {
    Object(SemanticObjectSchema),
    Array(SemanticArraySchema),
    String(SemanticNodeMetadata),
    Integer(SemanticNodeMetadata),
    Number(SemanticNodeMetadata),
    Boolean(SemanticNodeMetadata),
    Null(SemanticNodeMetadata),
    Nullable(SemanticNullableSchema),
    Enum(SemanticEnumSchema),
    Reference(SemanticReferenceSchema),
    TaggedUnion(SemanticTaggedUnionSchema),
    Union(SemanticUnionSchema),
    AllOf(SemanticAllOfSchema),
}

impl SemanticSchemaNode {
    fn metadata(&self) -> &SemanticNodeMetadata {
        match self {
            Self::Object(schema) => &schema.metadata,
            Self::Array(schema) => &schema.metadata,
            Self::String(metadata)
            | Self::Integer(metadata)
            | Self::Number(metadata)
            | Self::Boolean(metadata)
            | Self::Null(metadata) => metadata,
            Self::Nullable(schema) => &schema.metadata,
            Self::Enum(schema) => &schema.metadata,
            Self::Reference(schema) => &schema.metadata,
            Self::TaggedUnion(schema) => &schema.metadata,
            Self::Union(schema) => &schema.metadata,
            Self::AllOf(schema) => &schema.metadata,
        }
    }

    fn metadata_mut(&mut self) -> &mut SemanticNodeMetadata {
        match self {
            Self::Object(schema) => &mut schema.metadata,
            Self::Array(schema) => &mut schema.metadata,
            Self::String(metadata)
            | Self::Integer(metadata)
            | Self::Number(metadata)
            | Self::Boolean(metadata)
            | Self::Null(metadata) => metadata,
            Self::Nullable(schema) => &mut schema.metadata,
            Self::Enum(schema) => &mut schema.metadata,
            Self::Reference(schema) => &mut schema.metadata,
            Self::TaggedUnion(schema) => &mut schema.metadata,
            Self::Union(schema) => &mut schema.metadata,
            Self::AllOf(schema) => &mut schema.metadata,
        }
    }

    pub fn semantic_type_name(&self) -> String {
        match self {
            Self::Reference(reference) => reference.semantic_type.clone(),
            Self::Array(array) => format!("array<{}>", array.items.semantic_type_name()),
            Self::Nullable(nullable) => nullable.schema.semantic_type_name(),
            Self::Object(schema) => schema
                .metadata
                .title
                .clone()
                .unwrap_or_else(|| "object".to_owned()),
            Self::TaggedUnion(schema) => schema
                .metadata
                .title
                .clone()
                .unwrap_or_else(|| "tagged_union".to_owned()),
            Self::Enum(schema) => schema
                .metadata
                .title
                .clone()
                .unwrap_or_else(|| schema.value_type.clone()),
            Self::String(metadata) => metadata
                .title
                .clone()
                .unwrap_or_else(|| "string".to_owned()),
            Self::Integer(metadata) => metadata
                .title
                .clone()
                .unwrap_or_else(|| "integer".to_owned()),
            Self::Number(metadata) => metadata
                .title
                .clone()
                .unwrap_or_else(|| "number".to_owned()),
            Self::Boolean(metadata) => metadata
                .title
                .clone()
                .unwrap_or_else(|| "boolean".to_owned()),
            Self::Null(_) => "null".to_owned(),
            Self::Union(schema) => schema
                .metadata
                .title
                .clone()
                .unwrap_or_else(|| "union".to_owned()),
            Self::AllOf(schema) => schema
                .metadata
                .title
                .clone()
                .unwrap_or_else(|| "intersection".to_owned()),
        }
    }

    fn is_nullable(&self) -> bool {
        matches!(self, Self::Nullable(_))
    }

    fn to_json_schema(&self) -> Value {
        let mut object = self.metadata().to_json_object();
        match self {
            Self::Object(schema) => {
                object.insert("type".to_owned(), Value::String("object".to_owned()));
                object.insert(
                    "properties".to_owned(),
                    Value::Object(
                        schema
                            .fields
                            .iter()
                            .map(|field| {
                                let mut value = field.schema.to_json_schema();
                                if let Some(property) = value.as_object_mut() {
                                    property.insert(
                                        "description".to_owned(),
                                        Value::String(field.description.clone()),
                                    );
                                }
                                (field.field_name.clone(), value)
                            })
                            .collect(),
                    ),
                );
                let required = schema
                    .fields
                    .iter()
                    .filter(|field| field.required)
                    .map(|field| Value::String(field.field_name.clone()))
                    .collect::<Vec<_>>();
                if !required.is_empty() {
                    object.insert("required".to_owned(), Value::Array(required));
                }
                match &schema.additional_properties {
                    SemanticAdditionalProperties::Allowed => {}
                    SemanticAdditionalProperties::Forbidden => {
                        object.insert("additionalProperties".to_owned(), Value::Bool(false));
                    }
                    SemanticAdditionalProperties::Schema(schema) => {
                        object.insert("additionalProperties".to_owned(), schema.to_json_schema());
                    }
                }
            }
            Self::Array(schema) => {
                object.insert("type".to_owned(), Value::String("array".to_owned()));
                object.insert("items".to_owned(), schema.items.to_json_schema());
            }
            Self::String(_) => {
                object.insert("type".to_owned(), Value::String("string".to_owned()));
            }
            Self::Integer(_) => {
                object.insert("type".to_owned(), Value::String("integer".to_owned()));
            }
            Self::Number(_) => {
                object.insert("type".to_owned(), Value::String("number".to_owned()));
            }
            Self::Boolean(_) => {
                object.insert("type".to_owned(), Value::String("boolean".to_owned()));
            }
            Self::Null(_) => {
                object.insert("type".to_owned(), Value::String("null".to_owned()));
            }
            Self::Nullable(schema) => {
                let inner = schema.schema.to_json_schema();
                let primitive_type = inner
                    .get("type")
                    .and_then(Value::as_str)
                    .filter(|value_type| {
                        matches!(*value_type, "string" | "integer" | "number" | "boolean")
                    })
                    .map(str::to_owned);
                if let Some(primitive_type) = primitive_type {
                    let mut inner = inner
                        .as_object()
                        .expect("primitive schema has an object root")
                        .clone();
                    inner.remove("type");
                    object.extend(inner);
                    object.insert(
                        "type".to_owned(),
                        Value::Array(vec![
                            Value::String(primitive_type),
                            Value::String("null".to_owned()),
                        ]),
                    );
                } else {
                    object.insert(
                        "anyOf".to_owned(),
                        Value::Array(vec![
                            inner,
                            Value::Object(Map::from_iter([(
                                "type".to_owned(),
                                Value::String("null".to_owned()),
                            )])),
                        ]),
                    );
                }
            }
            Self::Enum(schema) => {
                object.insert("type".to_owned(), Value::String(schema.value_type.clone()));
                object.insert("enum".to_owned(), Value::Array(schema.values.clone()));
            }
            Self::Reference(schema) => {
                object.insert("$ref".to_owned(), Value::String(schema.reference.clone()));
            }
            Self::TaggedUnion(schema) => {
                object.insert(
                    "oneOf".to_owned(),
                    Value::Array(
                        schema
                            .variants
                            .iter()
                            .map(|variant| variant.schema.to_json_schema())
                            .collect(),
                    ),
                );
            }
            Self::Union(schema) => {
                object.insert(
                    schema.keyword.to_owned(),
                    Value::Array(
                        schema
                            .variants
                            .iter()
                            .map(SemanticSchemaNode::to_json_schema)
                            .collect(),
                    ),
                );
            }
            Self::AllOf(schema) => {
                object.insert(
                    "allOf".to_owned(),
                    Value::Array(
                        schema
                            .schemas
                            .iter()
                            .map(SemanticSchemaNode::to_json_schema)
                            .collect(),
                    ),
                );
            }
        }
        Value::Object(object)
    }
}

/// Object node and its closed field descriptors.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticObjectSchema {
    pub fields: Vec<SemanticObjectField>,
    pub additional_properties: SemanticAdditionalProperties,
    pub metadata: SemanticNodeMetadata,
}

/// Canonical field descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticObjectField {
    pub field_name: String,
    pub required: bool,
    pub nullable: bool,
    pub semantic_type: String,
    pub description: String,
    pub schema: Box<SemanticSchemaNode>,
}

/// Object additional-property policy.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticAdditionalProperties {
    Allowed,
    Forbidden,
    Schema(Box<SemanticSchemaNode>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticArraySchema {
    pub items: Box<SemanticSchemaNode>,
    pub metadata: SemanticNodeMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticNullableSchema {
    pub schema: Box<SemanticSchemaNode>,
    pub metadata: SemanticNodeMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticEnumSchema {
    pub values: Vec<Value>,
    pub value_type: String,
    pub metadata: SemanticNodeMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticReferenceSchema {
    pub reference: String,
    pub semantic_type: String,
    pub metadata: SemanticNodeMetadata,
}

/// Tagged union and its explicit discriminator metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTaggedUnionSchema {
    pub discriminator_path: String,
    pub variants: Vec<SemanticTaggedUnionVariant>,
    pub metadata: SemanticNodeMetadata,
}

/// One exact tagged-union branch.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTaggedUnionVariant {
    pub discriminator_value: String,
    pub semantic_type: String,
    pub schema: Box<SemanticSchemaNode>,
    pub canonical_examples: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticUnionSchema {
    pub keyword: &'static str,
    pub variants: Vec<SemanticSchemaNode>,
    pub metadata: SemanticNodeMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticAllOfSchema {
    pub schemas: Vec<SemanticSchemaNode>,
    pub metadata: SemanticNodeMetadata,
}

/// Descriptive annotations and validation constraints owned by one node.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticNodeMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub validation: BTreeMap<String, Value>,
}

impl SemanticNodeMetadata {
    fn to_json_object(&self) -> Map<String, Value> {
        let mut object = Map::new();
        if let Some(title) = &self.title {
            object.insert("title".to_owned(), Value::String(title.clone()));
        }
        if let Some(description) = &self.description {
            object.insert("description".to_owned(), Value::String(description.clone()));
        }
        object.extend(self.validation.clone());
        object
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticValidationIssueCode {
    Required,
    Unknown,
    TypeMismatch,
    EnumValue,
    AmbiguousUnion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticValidationIssue {
    pub path: String,
    pub code: SemanticValidationIssueCode,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticValidationResult {
    pub issues: Vec<SemanticValidationIssue>,
    pub truncated: bool,
}

impl SemanticValidationResult {
    pub const MAX_ISSUES: usize = 32;

    fn push(&mut self, issue: SemanticValidationIssue) {
        if self.issues.contains(&issue) {
            return;
        }
        if self.issues.len() < Self::MAX_ISSUES {
            self.issues.push(issue);
        } else {
            self.truncated = true;
        }
    }
}

/// Descriptor-owned metadata used to enrich one validation issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPathMetadata {
    pub semantic_type: String,
    pub required_fields: Vec<String>,
    pub allowed_enum_values: Vec<String>,
    pub description: Option<String>,
}

fn parse_schema_node(
    schema: &Value,
    fallback_type: &str,
    definitions: &Map<String, Value>,
) -> SemanticSchemaNode {
    let Some(object) = schema.as_object() else {
        return SemanticSchemaNode::Object(SemanticObjectSchema {
            fields: Vec::new(),
            additional_properties: SemanticAdditionalProperties::Allowed,
            metadata: SemanticNodeMetadata::default(),
        });
    };
    let metadata = node_metadata(object);
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        return SemanticSchemaNode::Reference(SemanticReferenceSchema {
            reference: reference.to_owned(),
            semantic_type: reference
                .rsplit('/')
                .next()
                .unwrap_or(fallback_type)
                .to_owned(),
            metadata,
        });
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return SemanticSchemaNode::Enum(SemanticEnumSchema {
            values: values.clone(),
            value_type: object
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| infer_enum_type(values).to_owned()),
            metadata,
        });
    }
    for (keyword, branches) in [
        ("oneOf", object.get("oneOf")),
        ("anyOf", object.get("anyOf")),
    ] {
        let Some(branches) = branches.and_then(Value::as_array) else {
            continue;
        };
        let non_null = branches
            .iter()
            .filter(|branch| !is_null_schema(branch))
            .collect::<Vec<_>>();
        if non_null.len() + 1 == branches.len() {
            let inner = if non_null.len() == 1 {
                parse_schema_node(non_null[0], fallback_type, definitions)
            } else {
                parse_union_node(keyword, &non_null, fallback_type, definitions)
            };
            return nullable_node(inner, metadata);
        }
        if let Some(tagged) =
            parse_tagged_union(branches, fallback_type, definitions, metadata.clone())
        {
            return with_union_siblings(
                SemanticSchemaNode::TaggedUnion(tagged),
                object,
                keyword,
                fallback_type,
                definitions,
            );
        }
        return with_union_siblings(
            parse_union_node(
                keyword,
                &branches.iter().collect::<Vec<_>>(),
                fallback_type,
                definitions,
            ),
            object,
            keyword,
            fallback_type,
            definitions,
        );
    }
    if let Some(schemas) = object.get("allOf").and_then(Value::as_array) {
        let mut parsed = schemas
            .iter()
            .map(|schema| parse_schema_node(schema, fallback_type, definitions))
            .collect::<Vec<_>>();
        if object.contains_key("properties")
            || object.contains_key("required")
            || object.contains_key("additionalProperties")
        {
            let mut sibling = object.clone();
            sibling.remove("allOf");
            let sibling = parse_schema_node(&Value::Object(sibling), fallback_type, definitions);
            let mut combined = sibling;
            while let Some(schema) = parsed.pop() {
                combined = merge_semantic_nodes(schema, combined, definitions, 0).unwrap_or_else(
                    |nodes| {
                        let (left, right) = *nodes;
                        SemanticSchemaNode::AllOf(SemanticAllOfSchema {
                            schemas: vec![left, right],
                            metadata: SemanticNodeMetadata::default(),
                        })
                    },
                );
            }
            return combined;
        }
        return SemanticSchemaNode::AllOf(SemanticAllOfSchema {
            schemas: parsed,
            metadata,
        });
    }

    if let Some(types) = object.get("type").and_then(Value::as_array) {
        let non_null = types
            .iter()
            .filter_map(Value::as_str)
            .filter(|value_type| *value_type != "null")
            .collect::<Vec<_>>();
        let includes_null = types.iter().any(|value_type| value_type == "null");
        if includes_null && non_null.len() == 1 {
            let mut inner = schema.clone();
            inner
                .as_object_mut()
                .expect("schema object was already established")
                .insert("type".to_owned(), Value::String(non_null[0].to_owned()));
            return nullable_node(
                parse_schema_node(&inner, fallback_type, definitions),
                metadata,
            );
        }
    }

    match object.get("type").and_then(Value::as_str) {
        Some("array") => SemanticSchemaNode::Array(SemanticArraySchema {
            items: Box::new(parse_schema_node(
                object.get("items").unwrap_or(&Value::Object(Map::new())),
                &format!("{fallback_type}Item"),
                definitions,
            )),
            metadata,
        }),
        Some("string") => SemanticSchemaNode::String(metadata),
        Some("integer") => SemanticSchemaNode::Integer(metadata),
        Some("number") => SemanticSchemaNode::Number(metadata),
        Some("boolean") => SemanticSchemaNode::Boolean(metadata),
        Some("null") => SemanticSchemaNode::Null(metadata),
        Some("object") | None if object.contains_key("properties") => {
            let required = object
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<BTreeSet<_>>();
            let fields = object
                .get("properties")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(name, field_schema)| {
                    let mut node = parse_schema_node(field_schema, name, definitions);
                    let semantic_type = if name == "baseline_ref" {
                        "BaselineRef".to_owned()
                    } else {
                        node.semantic_type_name()
                    };
                    if required.contains(name.as_str())
                        && node.is_nullable()
                        && semantic_type == "BaselineRef"
                    {
                        node.metadata_mut().validation.insert(
                            "not".to_owned(),
                            Value::Object(Map::from_iter([(
                                "const".to_owned(),
                                Value::String("null".to_owned()),
                            )])),
                        );
                    }
                    let description = field_schema
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            format!("`{name}` carries the `{semantic_type}` wire value.")
                        });
                    SemanticObjectField {
                        field_name: name.clone(),
                        required: required.contains(name.as_str()),
                        nullable: node.is_nullable(),
                        semantic_type,
                        description,
                        schema: Box::new(node),
                    }
                })
                .collect();
            let additional_properties =
                match object.get("additionalProperties") {
                    Some(Value::Bool(false)) => SemanticAdditionalProperties::Forbidden,
                    Some(Value::Bool(true)) | None => SemanticAdditionalProperties::Allowed,
                    Some(schema) => SemanticAdditionalProperties::Schema(Box::new(
                        parse_schema_node(schema, "AdditionalProperty", definitions),
                    )),
                };
            SemanticSchemaNode::Object(SemanticObjectSchema {
                fields,
                additional_properties,
                metadata,
            })
        }
        _ => SemanticSchemaNode::Object(SemanticObjectSchema {
            fields: Vec::new(),
            additional_properties: SemanticAdditionalProperties::Allowed,
            metadata,
        }),
    }
}

fn nullable_node(node: SemanticSchemaNode, metadata: SemanticNodeMetadata) -> SemanticSchemaNode {
    match node {
        SemanticSchemaNode::Nullable(mut nullable) => {
            if nullable.metadata.title.is_none() {
                nullable.metadata.title = metadata.title;
            }
            if nullable.metadata.description.is_none() {
                nullable.metadata.description = metadata.description;
            }
            nullable.metadata.validation.extend(metadata.validation);
            SemanticSchemaNode::Nullable(nullable)
        }
        node => SemanticSchemaNode::Nullable(SemanticNullableSchema {
            schema: Box::new(node),
            metadata,
        }),
    }
}

fn with_union_siblings(
    union: SemanticSchemaNode,
    object: &Map<String, Value>,
    keyword: &str,
    fallback_type: &str,
    definitions: &Map<String, Value>,
) -> SemanticSchemaNode {
    if !object.contains_key("properties")
        && !object.contains_key("required")
        && !object.contains_key("additionalProperties")
    {
        return union;
    }
    let mut sibling = object.clone();
    sibling.remove(keyword);
    let sibling = parse_schema_node(&Value::Object(sibling), fallback_type, definitions);
    merge_semantic_nodes(union, sibling, definitions, 0).unwrap_or_else(|nodes| {
        let (left, right) = *nodes;
        SemanticSchemaNode::AllOf(SemanticAllOfSchema {
            schemas: vec![left, right],
            metadata: SemanticNodeMetadata::default(),
        })
    })
}

fn merge_semantic_nodes(
    left: SemanticSchemaNode,
    right: SemanticSchemaNode,
    definitions: &Map<String, Value>,
    depth: usize,
) -> Result<SemanticSchemaNode, Box<(SemanticSchemaNode, SemanticSchemaNode)>> {
    if depth >= 32 {
        return Err(Box::new((left, right)));
    }
    match (left, right) {
        (SemanticSchemaNode::Reference(reference), right) => {
            let Some(target) = reference
                .reference
                .strip_prefix("#/definitions/")
                .and_then(|name| definitions.get(name).map(|schema| (name, schema)))
            else {
                return Err(Box::new((SemanticSchemaNode::Reference(reference), right)));
            };
            merge_semantic_nodes(
                parse_schema_node(target.1, target.0, definitions),
                right,
                definitions,
                depth + 1,
            )
        }
        (left, SemanticSchemaNode::Reference(reference)) => {
            let Some(target) = reference
                .reference
                .strip_prefix("#/definitions/")
                .and_then(|name| definitions.get(name).map(|schema| (name, schema)))
            else {
                return Err(Box::new((left, SemanticSchemaNode::Reference(reference))));
            };
            merge_semantic_nodes(
                left,
                parse_schema_node(target.1, target.0, definitions),
                definitions,
                depth + 1,
            )
        }
        (SemanticSchemaNode::Object(left), SemanticSchemaNode::Object(right)) => Ok(
            SemanticSchemaNode::Object(merge_object_schemas(left, right)),
        ),
        (SemanticSchemaNode::TaggedUnion(mut union), right) => {
            for index in 0..union.variants.len() {
                let schema = std::mem::replace(
                    union.variants[index].schema.as_mut(),
                    SemanticSchemaNode::Object(SemanticObjectSchema {
                        fields: Vec::new(),
                        additional_properties: SemanticAdditionalProperties::Allowed,
                        metadata: SemanticNodeMetadata::default(),
                    }),
                );
                let Ok(merged) =
                    merge_semantic_nodes(schema, right.clone(), definitions, depth + 1)
                else {
                    return Err(Box::new((SemanticSchemaNode::TaggedUnion(union), right)));
                };
                *union.variants[index].schema = merged;
            }
            Ok(SemanticSchemaNode::TaggedUnion(union))
        }
        (left, SemanticSchemaNode::TaggedUnion(mut union)) => {
            for index in 0..union.variants.len() {
                let schema = std::mem::replace(
                    union.variants[index].schema.as_mut(),
                    SemanticSchemaNode::Object(SemanticObjectSchema {
                        fields: Vec::new(),
                        additional_properties: SemanticAdditionalProperties::Allowed,
                        metadata: SemanticNodeMetadata::default(),
                    }),
                );
                let Ok(merged) = merge_semantic_nodes(left.clone(), schema, definitions, depth + 1)
                else {
                    return Err(Box::new((left, SemanticSchemaNode::TaggedUnion(union))));
                };
                *union.variants[index].schema = merged;
            }
            Ok(SemanticSchemaNode::TaggedUnion(union))
        }
        (SemanticSchemaNode::Union(mut union), right) => {
            for index in 0..union.variants.len() {
                let schema = std::mem::replace(
                    &mut union.variants[index],
                    SemanticSchemaNode::Object(SemanticObjectSchema {
                        fields: Vec::new(),
                        additional_properties: SemanticAdditionalProperties::Allowed,
                        metadata: SemanticNodeMetadata::default(),
                    }),
                );
                let Ok(merged) =
                    merge_semantic_nodes(schema, right.clone(), definitions, depth + 1)
                else {
                    return Err(Box::new((SemanticSchemaNode::Union(union), right)));
                };
                union.variants[index] = merged;
            }
            Ok(SemanticSchemaNode::Union(union))
        }
        (left, SemanticSchemaNode::Union(mut union)) => {
            for index in 0..union.variants.len() {
                let schema = std::mem::replace(
                    &mut union.variants[index],
                    SemanticSchemaNode::Object(SemanticObjectSchema {
                        fields: Vec::new(),
                        additional_properties: SemanticAdditionalProperties::Allowed,
                        metadata: SemanticNodeMetadata::default(),
                    }),
                );
                let Ok(merged) = merge_semantic_nodes(left.clone(), schema, definitions, depth + 1)
                else {
                    return Err(Box::new((left, SemanticSchemaNode::Union(union))));
                };
                union.variants[index] = merged;
            }
            Ok(SemanticSchemaNode::Union(union))
        }
        (left, right) => Err(Box::new((left, right))),
    }
}

fn merge_object_schemas(
    mut left: SemanticObjectSchema,
    right: SemanticObjectSchema,
) -> SemanticObjectSchema {
    for mut right_field in right.fields {
        if let Some(left_field) = left
            .fields
            .iter_mut()
            .find(|field| field.field_name == right_field.field_name)
        {
            left_field.required |= right_field.required;
            left_field.nullable &= right_field.nullable;
            if left_field.schema != right_field.schema {
                *left_field.schema = SemanticSchemaNode::AllOf(SemanticAllOfSchema {
                    schemas: vec![
                        std::mem::replace(
                            left_field.schema.as_mut(),
                            SemanticSchemaNode::Object(SemanticObjectSchema {
                                fields: Vec::new(),
                                additional_properties: SemanticAdditionalProperties::Allowed,
                                metadata: SemanticNodeMetadata::default(),
                            }),
                        ),
                        *right_field.schema,
                    ],
                    metadata: SemanticNodeMetadata::default(),
                });
            }
            if !right_field.description.trim().is_empty() {
                left_field.description = std::mem::take(&mut right_field.description);
            }
            if left_field.semantic_type == "object" {
                left_field.semantic_type = right_field.semantic_type;
            }
        } else {
            left.fields.push(right_field);
        }
    }
    left.additional_properties = match (left.additional_properties, right.additional_properties) {
        (SemanticAdditionalProperties::Forbidden, _)
        | (_, SemanticAdditionalProperties::Forbidden) => SemanticAdditionalProperties::Forbidden,
        (SemanticAdditionalProperties::Allowed, right) => right,
        (left, SemanticAdditionalProperties::Allowed) => left,
        (
            SemanticAdditionalProperties::Schema(left),
            SemanticAdditionalProperties::Schema(right),
        ) => SemanticAdditionalProperties::Schema(Box::new(SemanticSchemaNode::AllOf(
            SemanticAllOfSchema {
                schemas: vec![*left, *right],
                metadata: SemanticNodeMetadata::default(),
            },
        ))),
    };
    if left.metadata.title.is_none() {
        left.metadata.title = right.metadata.title;
    }
    if left.metadata.description.is_none() {
        left.metadata.description = right.metadata.description;
    }
    left.metadata.validation.extend(right.metadata.validation);
    left
}

fn parse_union_node(
    keyword: &str,
    branches: &[&Value],
    fallback_type: &str,
    definitions: &Map<String, Value>,
) -> SemanticSchemaNode {
    SemanticSchemaNode::Union(SemanticUnionSchema {
        keyword: if keyword == "oneOf" { "oneOf" } else { "anyOf" },
        variants: branches
            .iter()
            .map(|branch| parse_schema_node(branch, fallback_type, definitions))
            .collect(),
        metadata: SemanticNodeMetadata::default(),
    })
}

fn parse_tagged_union(
    branches: &[Value],
    fallback_type: &str,
    definitions: &Map<String, Value>,
    metadata: SemanticNodeMetadata,
) -> Option<SemanticTaggedUnionSchema> {
    let branch_constants = branches
        .iter()
        .map(|branch| discriminator_constants(branch, definitions, 0))
        .collect::<Vec<_>>();
    let first = branch_constants.first()?;
    let mut candidates = first.keys().cloned().collect::<Vec<_>>();
    candidates.retain(|path| {
        let values = branch_constants
            .iter()
            .filter_map(|constants| constants.get(path))
            .collect::<BTreeSet<_>>();
        values.len() == branches.len()
    });
    candidates.sort_by_key(|path| (path.matches('/').count(), path.clone()));
    let discriminator_path = candidates.into_iter().next()?;
    let variants = branches
        .iter()
        .zip(branch_constants)
        .map(|(branch, constants)| {
            let discriminator_value = constants.get(&discriminator_path)?.clone();
            let semantic_type = branch_semantic_type(branch)
                .unwrap_or_else(|| format!("{fallback_type}::{discriminator_value}"));
            Some(SemanticTaggedUnionVariant {
                discriminator_value,
                semantic_type: semantic_type.clone(),
                schema: Box::new(parse_schema_node(branch, &semantic_type, definitions)),
                canonical_examples: Vec::new(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SemanticTaggedUnionSchema {
        discriminator_path,
        variants,
        metadata,
    })
}

fn discriminator_constants(
    schema: &Value,
    definitions: &Map<String, Value>,
    depth: usize,
) -> BTreeMap<String, String> {
    if depth > 16 {
        return BTreeMap::new();
    }
    let Some(object) = schema.as_object() else {
        return BTreeMap::new();
    };
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        return reference
            .strip_prefix("#/definitions/")
            .and_then(|name| definitions.get(name))
            .map(|schema| discriminator_constants(schema, definitions, depth + 1))
            .unwrap_or_default();
    }
    let mut constants = BTreeMap::new();
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (name, property) in properties {
            if let Some(value) = singleton_string_enum(property, definitions, depth + 1) {
                constants.insert(format!("/{name}"), value);
            }
            for (path, value) in discriminator_constants(property, definitions, depth + 1) {
                constants.insert(format!("/{name}{path}"), value);
            }
        }
    }
    if let Some(schemas) = object.get("allOf").and_then(Value::as_array) {
        for schema in schemas {
            constants.extend(discriminator_constants(schema, definitions, depth + 1));
        }
    }
    for keyword in ["oneOf", "anyOf"] {
        let Some(branches) = object.get(keyword).and_then(Value::as_array) else {
            continue;
        };
        let branch_constants = branches
            .iter()
            .map(|branch| discriminator_constants(branch, definitions, depth + 1))
            .collect::<Vec<_>>();
        let Some(first) = branch_constants.first() else {
            continue;
        };
        for (path, value) in first {
            if branch_constants
                .iter()
                .skip(1)
                .all(|branch| branch.get(path) == Some(value))
            {
                constants.insert(path.clone(), value.clone());
            }
        }
    }
    constants
}

fn singleton_string_enum(
    schema: &Value,
    definitions: &Map<String, Value>,
    depth: usize,
) -> Option<String> {
    if depth > 16 {
        return None;
    }
    let object = schema.as_object()?;
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        return reference
            .strip_prefix("#/definitions/")
            .and_then(|name| definitions.get(name))
            .and_then(|schema| singleton_string_enum(schema, definitions, depth + 1));
    }
    if let Some(schemas) = object.get("allOf").and_then(Value::as_array) {
        let values = schemas
            .iter()
            .filter_map(|schema| singleton_string_enum(schema, definitions, depth + 1))
            .collect::<BTreeSet<_>>();
        if values.len() == 1 {
            return values.into_iter().next();
        }
    }
    let values = object.get("enum")?.as_array()?;
    match values.as_slice() {
        [Value::String(value)] => Some(value.clone()),
        _ => None,
    }
}

fn branch_semantic_type(branch: &Value) -> Option<String> {
    branch
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.rsplit('/').next())
        .or_else(|| branch.get("title").and_then(Value::as_str))
        .map(str::to_owned)
        .or_else(|| {
            branch
                .get("allOf")
                .and_then(Value::as_array)
                .and_then(|schemas| schemas.iter().find_map(branch_semantic_type))
        })
}

fn node_metadata(object: &Map<String, Value>) -> SemanticNodeMetadata {
    let handled = [
        "$ref",
        "allOf",
        "anyOf",
        "description",
        "enum",
        "items",
        "oneOf",
        "properties",
        "required",
        "title",
        "type",
        "additionalProperties",
    ];
    SemanticNodeMetadata {
        title: object
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_owned),
        description: object
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        validation: object
            .iter()
            .filter(|(key, _)| !handled.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    }
}

fn infer_enum_type(values: &[Value]) -> &'static str {
    if values.iter().all(Value::is_string) {
        "string"
    } else if values.iter().all(Value::is_boolean) {
        "boolean"
    } else if values
        .iter()
        .all(|value| value.as_i64().is_some() || value.as_u64().is_some())
    {
        "integer"
    } else {
        "number"
    }
}

fn is_null_schema(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("null")
}

fn validate_node(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    instance: &Value,
    path: &str,
    depth: usize,
    expected_type: Option<&str>,
    result: &mut SemanticValidationResult,
) {
    if result.issues.len() >= SemanticValidationResult::MAX_ISSUES {
        result.truncated = true;
        return;
    }
    if depth >= 64 {
        result.truncated = true;
        return;
    }
    if node
        .metadata()
        .validation
        .get("not")
        .and_then(|constraint| constraint.get("const"))
        == Some(instance)
    {
        let semantic_type = expected_type
            .map(str::to_owned)
            .unwrap_or_else(|| node.semantic_type_name());
        result.push(type_issue(
            path,
            &semantic_type,
            instance,
            "Value matches a forbidden semantic literal.",
        ));
        return;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            let Some(target) = reference_target(definitions, &reference.reference) else {
                result.push(type_issue(
                    path,
                    &reference.semantic_type,
                    instance,
                    "Referenced semantic type is unavailable.",
                ));
                return;
            };
            validate_node(
                target,
                definitions,
                instance,
                path,
                depth + 1,
                Some(&reference.semantic_type),
                result,
            );
        }
        SemanticSchemaNode::Nullable(nullable) => {
            if !instance.is_null() {
                validate_node(
                    &nullable.schema,
                    definitions,
                    instance,
                    path,
                    depth + 1,
                    expected_type,
                    result,
                );
            }
        }
        SemanticSchemaNode::Object(schema) => {
            let Some(instance) = instance.as_object() else {
                result.push(type_issue(
                    path,
                    expected_type.unwrap_or("object"),
                    instance,
                    "Expected an object.",
                ));
                return;
            };
            for field in schema.fields.iter().filter(|field| field.required) {
                if !instance.contains_key(&field.field_name) {
                    result.push(SemanticValidationIssue {
                        path: pointer_child(path, &field.field_name),
                        code: SemanticValidationIssueCode::Required,
                        message: format!("Required argument `{}` is missing.", field.field_name),
                    });
                }
            }
            for (name, value) in instance {
                if let Some(field) = schema.fields.iter().find(|field| field.field_name == *name) {
                    validate_node(
                        &field.schema,
                        definitions,
                        value,
                        &pointer_child(path, name),
                        depth + 1,
                        Some(&field.semantic_type),
                        result,
                    );
                    continue;
                }
                match &schema.additional_properties {
                    SemanticAdditionalProperties::Allowed => {}
                    SemanticAdditionalProperties::Forbidden => {
                        result.push(SemanticValidationIssue {
                            path: pointer_child(path, name),
                            code: SemanticValidationIssueCode::Unknown,
                            message: format!("Unknown argument `{name}` is not allowed."),
                        });
                    }
                    SemanticAdditionalProperties::Schema(schema) => validate_node(
                        schema,
                        definitions,
                        value,
                        &pointer_child(path, name),
                        depth + 1,
                        None,
                        result,
                    ),
                }
            }
        }
        SemanticSchemaNode::Array(schema) => {
            let Some(items) = instance.as_array() else {
                result.push(type_issue(
                    path,
                    expected_type.unwrap_or("array"),
                    instance,
                    "Expected an array.",
                ));
                return;
            };
            if !array_length_valid(&schema.metadata, items.len()) {
                result.push(type_issue(
                    path,
                    expected_type.unwrap_or("array"),
                    instance,
                    "Array length does not satisfy the semantic schema.",
                ));
            }
            for (index, value) in items.iter().enumerate() {
                validate_node(
                    &schema.items,
                    definitions,
                    value,
                    &pointer_child(path, &index.to_string()),
                    depth + 1,
                    Some(&schema.items.semantic_type_name()),
                    result,
                );
            }
        }
        SemanticSchemaNode::String(metadata) => validate_primitive(
            instance,
            path,
            expected_type.unwrap_or("string"),
            "string",
            metadata,
            result,
        ),
        SemanticSchemaNode::Integer(metadata) => validate_primitive(
            instance,
            path,
            expected_type.unwrap_or("integer"),
            "integer",
            metadata,
            result,
        ),
        SemanticSchemaNode::Number(metadata) => validate_primitive(
            instance,
            path,
            expected_type.unwrap_or("number"),
            "number",
            metadata,
            result,
        ),
        SemanticSchemaNode::Boolean(metadata) => validate_primitive(
            instance,
            path,
            expected_type.unwrap_or("boolean"),
            "boolean",
            metadata,
            result,
        ),
        SemanticSchemaNode::Null(metadata) => validate_primitive(
            instance,
            path,
            expected_type.unwrap_or("null"),
            "null",
            metadata,
            result,
        ),
        SemanticSchemaNode::Enum(schema) => {
            if !schema.values.contains(instance) {
                result.push(SemanticValidationIssue {
                    path: path.to_owned(),
                    code: SemanticValidationIssueCode::EnumValue,
                    message: format!(
                        "Expected one of [{}], but received {}.",
                        schema
                            .values
                            .iter()
                            .map(Value::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        instance
                    ),
                });
            }
        }
        SemanticSchemaNode::TaggedUnion(schema) => {
            let Some(value) = instance.pointer(&schema.discriminator_path) else {
                result.push(SemanticValidationIssue {
                    path: format!("{path}{}", schema.discriminator_path),
                    code: SemanticValidationIssueCode::Required,
                    message: format!(
                        "Tagged union discriminator `{}` is required.",
                        schema.discriminator_path
                    ),
                });
                return;
            };
            let Some(value) = value.as_str() else {
                result.push(type_issue(
                    &format!("{path}{}", schema.discriminator_path),
                    "string discriminator",
                    value,
                    "Expected a string discriminator.",
                ));
                return;
            };
            let Some(variant) = schema
                .variants
                .iter()
                .find(|variant| variant.discriminator_value == value)
            else {
                result.push(SemanticValidationIssue {
                    path: format!("{path}{}", schema.discriminator_path),
                    code: SemanticValidationIssueCode::EnumValue,
                    message: format!(
                        "Expected one of [{}], but received {}.",
                        schema
                            .variants
                            .iter()
                            .map(|variant| format!("\"{}\"", variant.discriminator_value))
                            .collect::<Vec<_>>()
                            .join(", "),
                        Value::String(value.to_owned())
                    ),
                });
                return;
            };
            validate_node(
                &variant.schema,
                definitions,
                instance,
                path,
                depth + 1,
                Some(&variant.semantic_type),
                result,
            );
        }
        SemanticSchemaNode::Union(schema) => {
            validate_union(&schema.variants, definitions, instance, path, depth, result)
        }
        SemanticSchemaNode::AllOf(schema) => {
            for child in &schema.schemas {
                validate_node(
                    child,
                    definitions,
                    instance,
                    path,
                    depth + 1,
                    expected_type,
                    result,
                );
            }
        }
    }
}

fn validate_union(
    variants: &[SemanticSchemaNode],
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    instance: &Value,
    path: &str,
    depth: usize,
    result: &mut SemanticValidationResult,
) {
    let attempts = variants
        .iter()
        .map(|variant| {
            let mut attempt = SemanticValidationResult::default();
            validate_node(
                variant,
                definitions,
                instance,
                path,
                depth + 1,
                None,
                &mut attempt,
            );
            attempt
        })
        .collect::<Vec<_>>();
    let matches = attempts
        .iter()
        .filter(|attempt| attempt.issues.is_empty())
        .count();
    match matches {
        1 => {}
        0 => {
            if let Some(best) = attempts.iter().min_by_key(|attempt| attempt.issues.len()) {
                for issue in &best.issues {
                    result.push(issue.clone());
                }
                result.truncated |= best.truncated;
            }
        }
        _ => result.push(SemanticValidationIssue {
            path: path.to_owned(),
            code: SemanticValidationIssueCode::AmbiguousUnion,
            message: "Value matched more than one semantic union variant.".to_owned(),
        }),
    }
}

fn validate_primitive(
    instance: &Value,
    path: &str,
    expected_semantic_type: &str,
    primitive: &str,
    metadata: &SemanticNodeMetadata,
    result: &mut SemanticValidationResult,
) {
    if !value_matches_type(instance, primitive) {
        result.push(type_issue(
            path,
            expected_semantic_type,
            instance,
            &format!("Expected {primitive}."),
        ));
        return;
    }
    if primitive == "string" {
        if let Some(value) = instance.as_str() {
            let length = value.chars().count() as u64;
            let minimum = metadata.validation.get("minLength").and_then(Value::as_u64);
            let maximum = metadata.validation.get("maxLength").and_then(Value::as_u64);
            if minimum.is_some_and(|minimum| length < minimum)
                || maximum.is_some_and(|maximum| length > maximum)
            {
                result.push(type_issue(
                    path,
                    expected_semantic_type,
                    instance,
                    "String length does not satisfy the semantic schema.",
                ));
            }
        }
    }
    if matches!(primitive, "integer" | "number") {
        let value = instance.as_f64();
        let minimum = metadata.validation.get("minimum").and_then(Value::as_f64);
        let maximum = metadata.validation.get("maximum").and_then(Value::as_f64);
        if value.is_some_and(|value| {
            minimum.is_some_and(|minimum| value < minimum)
                || maximum.is_some_and(|maximum| value > maximum)
        }) {
            result.push(type_issue(
                path,
                expected_semantic_type,
                instance,
                "Number does not satisfy the semantic schema range.",
            ));
        }
    }
}

fn array_length_valid(metadata: &SemanticNodeMetadata, length: usize) -> bool {
    let length = length as u64;
    let minimum = metadata.validation.get("minItems").and_then(Value::as_u64);
    let maximum = metadata.validation.get("maxItems").and_then(Value::as_u64);
    !minimum.is_some_and(|minimum| length < minimum)
        && !maximum.is_some_and(|maximum| length > maximum)
}

fn type_issue(
    path: &str,
    expected_semantic_type: &str,
    instance: &Value,
    detail: &str,
) -> SemanticValidationIssue {
    SemanticValidationIssue {
        path: path.to_owned(),
        code: SemanticValidationIssueCode::TypeMismatch,
        message: format!(
            "{detail} Expected `{expected_semantic_type}`, but received {}.",
            instance_type_name(instance)
        ),
    }
}

fn value_matches_type(value: &Value, expected: &str) -> bool {
    match expected {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "number" => value.is_number(),
        "string" => value.is_string(),
        "integer" => value
            .as_number()
            .is_some_and(|number| number.is_i64() || number.is_u64()),
        _ => false,
    }
}

fn instance_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn pointer_child(path: &str, segment: &str) -> String {
    format!("{path}/{}", segment.replace('~', "~0").replace('/', "~1"))
}

fn reference_target<'a>(
    definitions: &'a BTreeMap<String, SemanticSchemaNode>,
    reference: &str,
) -> Option<&'a SemanticSchemaNode> {
    reference
        .strip_prefix("#/definitions/")
        .and_then(|name| definitions.get(name))
}

fn metadata_at_path(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    path: &str,
) -> Option<SemanticPathMetadata> {
    let mut current = node;
    let mut parent_required = Vec::new();
    for segment in path
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
    {
        current = resolve_node(current, definitions);
        match current {
            SemanticSchemaNode::Nullable(nullable) => current = &nullable.schema,
            SemanticSchemaNode::TaggedUnion(union) => {
                current = union
                    .variants
                    .first()
                    .map(|variant| variant.schema.as_ref())?;
            }
            _ => {}
        }
        current = resolve_node(current, definitions);
        match current {
            SemanticSchemaNode::Object(object) => {
                parent_required = object
                    .fields
                    .iter()
                    .filter(|field| field.required)
                    .map(|field| field.field_name.clone())
                    .collect();
                let Some(field) = object
                    .fields
                    .iter()
                    .find(|field| field.field_name == segment)
                else {
                    return Some(path_metadata(
                        current,
                        current,
                        parent_required,
                        definitions,
                    ));
                };
                current = &field.schema;
            }
            SemanticSchemaNode::Array(array) if segment.parse::<usize>().is_ok() => {
                current = &array.items;
            }
            SemanticSchemaNode::Union(union) => {
                current = union.variants.first()?;
            }
            SemanticSchemaNode::AllOf(all_of) => {
                current = all_of.schemas.first()?;
            }
            _ => return None,
        }
    }
    let resolved = resolve_node(current, definitions);
    if let SemanticSchemaNode::Object(object) = resolved {
        parent_required = object
            .fields
            .iter()
            .filter(|field| field.required)
            .map(|field| field.field_name.clone())
            .collect();
    }
    Some(path_metadata(
        current,
        resolved,
        parent_required,
        definitions,
    ))
}

fn path_metadata(
    semantic_node: &SemanticSchemaNode,
    resolved_node: &SemanticSchemaNode,
    required_fields: Vec<String>,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
) -> SemanticPathMetadata {
    SemanticPathMetadata {
        semantic_type: semantic_node.semantic_type_name(),
        required_fields,
        allowed_enum_values: allowed_enum_values(semantic_node, definitions),
        description: semantic_node
            .metadata()
            .description
            .clone()
            .or_else(|| resolved_node.metadata().description.clone()),
    }
}

fn resolve_node<'a>(
    mut node: &'a SemanticSchemaNode,
    definitions: &'a BTreeMap<String, SemanticSchemaNode>,
) -> &'a SemanticSchemaNode {
    for _ in 0..32 {
        let SemanticSchemaNode::Reference(reference) = node else {
            break;
        };
        let Some(target) = reference_target(definitions, &reference.reference) else {
            break;
        };
        node = target;
    }
    node
}

fn allowed_enum_values(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
) -> Vec<String> {
    match resolve_node(node, definitions) {
        SemanticSchemaNode::Enum(schema) => schema
            .values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        SemanticSchemaNode::TaggedUnion(schema) => schema
            .variants
            .iter()
            .map(|variant| variant.discriminator_value.clone())
            .collect(),
        SemanticSchemaNode::Nullable(schema) => allowed_enum_values(&schema.schema, definitions),
        _ => Vec::new(),
    }
}

fn validate_node_integrity(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    path: &str,
    required_context: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if reference_target(definitions, &reference.reference).is_none() {
                errors.push(format!(
                    "{path} has unresolved reference `{}`",
                    reference.reference
                ));
            }
        }
        SemanticSchemaNode::Nullable(nullable) => {
            if matches!(
                nullable.schema.as_ref(),
                SemanticSchemaNode::Nullable(_) | SemanticSchemaNode::Null(_)
            ) {
                errors.push(format!("{path} has invalid nested nullable construction"));
            }
            validate_node_integrity(
                &nullable.schema,
                definitions,
                &format!("{path}/nullable"),
                required_context,
                errors,
            );
        }
        SemanticSchemaNode::Object(object) => {
            let mut names = BTreeSet::new();
            for field in &object.fields {
                if !names.insert(&field.field_name) {
                    errors.push(format!("{path} duplicates field `{}`", field.field_name));
                }
                if field.required && field.description.trim().is_empty() {
                    errors.push(format!(
                        "{path}/properties/{} is required but undocumented",
                        field.field_name
                    ));
                }
                if field.nullable != field.schema.is_nullable() {
                    errors.push(format!(
                        "{path}/properties/{} has inconsistent nullable metadata",
                        field.field_name
                    ));
                }
                validate_node_integrity(
                    &field.schema,
                    definitions,
                    &format!("{path}/properties/{}", field.field_name),
                    required_context,
                    errors,
                );
            }
            if let SemanticAdditionalProperties::Schema(schema) = &object.additional_properties {
                validate_node_integrity(
                    schema,
                    definitions,
                    &format!("{path}/additionalProperties"),
                    required_context,
                    errors,
                );
            }
        }
        SemanticSchemaNode::Array(array) => validate_node_integrity(
            &array.items,
            definitions,
            &format!("{path}/items"),
            required_context,
            errors,
        ),
        SemanticSchemaNode::TaggedUnion(union) => {
            let mut values = BTreeSet::new();
            let mut variant_types = BTreeMap::<&str, &SemanticSchemaNode>::new();
            for variant in &union.variants {
                if !values.insert(&variant.discriminator_value) {
                    errors.push(format!(
                        "{path} duplicates discriminator value `{}`",
                        variant.discriminator_value
                    ));
                }
                if let Some(previous) =
                    variant_types.insert(&variant.semantic_type, &variant.schema)
                {
                    if previous != variant.schema.as_ref() {
                        errors.push(format!(
                            "{path} reuses semantic type `{}` for different variants",
                            variant.semantic_type
                        ));
                    }
                }
                let constants = rendered_discriminator_constants(&variant.schema, definitions);
                if constants.get(&union.discriminator_path) != Some(&variant.discriminator_value) {
                    errors.push(format!(
                        "{path} variant `{}` is missing discriminator `{}`",
                        variant.semantic_type, union.discriminator_path
                    ));
                }
                if !required_context.contains(&union.discriminator_path)
                    && !discriminator_path_is_required(
                        &variant.schema,
                        definitions,
                        &union.discriminator_path,
                        0,
                    )
                {
                    errors.push(format!(
                        "{path} variant `{}` does not require discriminator `{}`",
                        variant.semantic_type, union.discriminator_path
                    ));
                }
                for (index, example) in variant.canonical_examples.iter().enumerate() {
                    let mut validation = SemanticValidationResult::default();
                    validate_node(
                        &variant.schema,
                        definitions,
                        example,
                        "",
                        0,
                        Some(&variant.semantic_type),
                        &mut validation,
                    );
                    if !validation.issues.is_empty() {
                        errors.push(format!(
                            "{path} variant `{}` canonical example {index} is invalid: {}",
                            variant.semantic_type,
                            validation
                                .issues
                                .iter()
                                .map(|issue| format!("{} {}", issue.path, issue.message))
                                .collect::<Vec<_>>()
                                .join("; ")
                        ));
                    }
                }
                validate_node_integrity(
                    &variant.schema,
                    definitions,
                    &format!("{path}/variants/{}", variant.discriminator_value),
                    required_context,
                    errors,
                );
            }
        }
        SemanticSchemaNode::Union(union) => {
            if union.variants.len() > 1
                && union
                    .variants
                    .iter()
                    .all(|variant| is_object_shape(variant, definitions, 0))
            {
                errors.push(format!(
                    "{path} is an object union without an explicit discriminator"
                ));
            }
            for (index, variant) in union.variants.iter().enumerate() {
                validate_node_integrity(
                    variant,
                    definitions,
                    &format!("{path}/{}/{}", union.keyword, index),
                    required_context,
                    errors,
                );
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            let mut all_of_required = required_context.clone();
            for schema in &all_of.schemas {
                collect_required_paths(schema, definitions, "", 0, &mut all_of_required);
            }
            for (index, schema) in all_of.schemas.iter().enumerate() {
                validate_node_integrity(
                    schema,
                    definitions,
                    &format!("{path}/allOf/{index}"),
                    &all_of_required,
                    errors,
                );
            }
        }
        SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => {}
    }
}

fn collect_required_paths(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    prefix: &str,
    depth: usize,
    paths: &mut BTreeSet<String>,
) {
    if depth >= 32 {
        return;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if let Some(target) = reference_target(definitions, &reference.reference) {
                collect_required_paths(target, definitions, prefix, depth + 1, paths);
            }
        }
        SemanticSchemaNode::Object(object) => {
            for field in object.fields.iter().filter(|field| field.required) {
                let field_path = format!("{prefix}/{}", field.field_name);
                paths.insert(field_path.clone());
                collect_required_paths(&field.schema, definitions, &field_path, depth + 1, paths);
            }
        }
        SemanticSchemaNode::Nullable(nullable) => {
            collect_required_paths(&nullable.schema, definitions, prefix, depth + 1, paths);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            let mut common = None::<BTreeSet<String>>;
            for variant in &union.variants {
                let mut variant_paths = BTreeSet::new();
                collect_required_paths(
                    &variant.schema,
                    definitions,
                    prefix,
                    depth + 1,
                    &mut variant_paths,
                );
                common = Some(match common {
                    Some(common) => common.intersection(&variant_paths).cloned().collect(),
                    None => variant_paths,
                });
            }
            paths.extend(common.unwrap_or_default());
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                collect_required_paths(schema, definitions, prefix, depth + 1, paths);
            }
        }
        SemanticSchemaNode::Array(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_)
        | SemanticSchemaNode::Union(_) => {}
    }
}

fn is_object_shape(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    depth: usize,
) -> bool {
    if depth >= 32 {
        return false;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            reference_target(definitions, &reference.reference)
                .is_some_and(|target| is_object_shape(target, definitions, depth + 1))
        }
        SemanticSchemaNode::Object(_) | SemanticSchemaNode::TaggedUnion(_) => true,
        SemanticSchemaNode::Nullable(nullable) => {
            is_object_shape(&nullable.schema, definitions, depth + 1)
        }
        SemanticSchemaNode::AllOf(all_of) => all_of
            .schemas
            .iter()
            .any(|schema| is_object_shape(schema, definitions, depth + 1)),
        _ => false,
    }
}

fn discriminator_path_is_required(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    discriminator_path: &str,
    depth: usize,
) -> bool {
    if depth >= 32 {
        return false;
    }
    let Some(path) = discriminator_path.strip_prefix('/') else {
        return false;
    };
    let (segment, remainder) = path
        .split_once('/')
        .map_or((path, None), |(segment, remainder)| {
            (segment, Some(remainder))
        });
    match node {
        SemanticSchemaNode::Reference(reference) => {
            reference_target(definitions, &reference.reference).is_some_and(|target| {
                discriminator_path_is_required(target, definitions, discriminator_path, depth + 1)
            })
        }
        SemanticSchemaNode::Object(object) => object
            .fields
            .iter()
            .find(|field| field.field_name == segment)
            .is_some_and(|field| {
                field.required
                    && remainder.is_none_or(|remainder| {
                        discriminator_path_is_required(
                            &field.schema,
                            definitions,
                            &format!("/{remainder}"),
                            depth + 1,
                        )
                    })
            }),
        SemanticSchemaNode::Nullable(nullable) => discriminator_path_is_required(
            &nullable.schema,
            definitions,
            discriminator_path,
            depth + 1,
        ),
        SemanticSchemaNode::TaggedUnion(union) => union.variants.iter().all(|variant| {
            discriminator_path_is_required(
                &variant.schema,
                definitions,
                discriminator_path,
                depth + 1,
            )
        }),
        SemanticSchemaNode::Union(union) => union.variants.iter().all(|variant| {
            discriminator_path_is_required(variant, definitions, discriminator_path, depth + 1)
        }),
        SemanticSchemaNode::AllOf(all_of) => all_of.schemas.iter().any(|schema| {
            discriminator_path_is_required(schema, definitions, discriminator_path, depth + 1)
        }),
        _ => false,
    }
}

fn attach_tagged_variant_example(
    node: &mut SemanticSchemaNode,
    expected: &ExpectedTaggedVariant,
    value: &Value,
) {
    match node {
        SemanticSchemaNode::Object(object) => {
            for field in &mut object.fields {
                attach_tagged_variant_example(&mut field.schema, expected, value);
            }
            if let SemanticAdditionalProperties::Schema(schema) = &mut object.additional_properties
            {
                attach_tagged_variant_example(schema, expected, value);
            }
        }
        SemanticSchemaNode::Array(array) => {
            attach_tagged_variant_example(&mut array.items, expected, value);
        }
        SemanticSchemaNode::Nullable(nullable) => {
            attach_tagged_variant_example(&mut nullable.schema, expected, value);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            if union.discriminator_path == expected.discriminator_path {
                if let Some(variant) = union.variants.iter_mut().find(|variant| {
                    variant.discriminator_value == expected.discriminator_value
                        && variant.semantic_type == expected.semantic_type
                }) {
                    if !variant.canonical_examples.contains(value) {
                        variant.canonical_examples.push(value.clone());
                    }
                }
            }
            for variant in &mut union.variants {
                attach_tagged_variant_example(&mut variant.schema, expected, value);
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &mut union.variants {
                attach_tagged_variant_example(variant, expected, value);
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &mut all_of.schemas {
                attach_tagged_variant_example(schema, expected, value);
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

fn rendered_discriminator_constants(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
) -> BTreeMap<String, String> {
    let raw_definitions = definitions
        .iter()
        .map(|(name, node)| (name.clone(), node.to_json_schema()))
        .collect::<Map<_, _>>();
    discriminator_constants(&node.to_json_schema(), &raw_definitions, 0)
}

fn selected_tagged_variant<'a>(
    root: &'a SemanticSchemaNode,
    definitions: &'a BTreeMap<String, SemanticSchemaNode>,
    value: &Value,
    instance_path: &str,
    discriminator_path: &str,
) -> Option<(&'a str, &'a str)> {
    let instance = value.pointer(instance_path)?;
    find_tagged_union(root, definitions, discriminator_path).and_then(|union| {
        let discriminator = instance.pointer(discriminator_path)?.as_str()?;
        union
            .variants
            .iter()
            .find(|variant| variant.discriminator_value == discriminator)
            .map(|variant| {
                (
                    variant.discriminator_value.as_str(),
                    variant.semantic_type.as_str(),
                )
            })
    })
}

fn find_tagged_union<'a>(
    node: &'a SemanticSchemaNode,
    definitions: &'a BTreeMap<String, SemanticSchemaNode>,
    discriminator_path: &str,
) -> Option<&'a SemanticTaggedUnionSchema> {
    match resolve_node(node, definitions) {
        SemanticSchemaNode::TaggedUnion(union)
            if union.discriminator_path == discriminator_path =>
        {
            Some(union)
        }
        SemanticSchemaNode::TaggedUnion(union) => union.variants.iter().find_map(|variant| {
            find_tagged_union(&variant.schema, definitions, discriminator_path)
        }),
        SemanticSchemaNode::Object(object) => object
            .fields
            .iter()
            .find_map(|field| find_tagged_union(&field.schema, definitions, discriminator_path)),
        SemanticSchemaNode::Array(array) => {
            find_tagged_union(&array.items, definitions, discriminator_path)
        }
        SemanticSchemaNode::Nullable(nullable) => {
            find_tagged_union(&nullable.schema, definitions, discriminator_path)
        }
        SemanticSchemaNode::Union(union) => union
            .variants
            .iter()
            .find_map(|variant| find_tagged_union(variant, definitions, discriminator_path)),
        SemanticSchemaNode::AllOf(all_of) => all_of
            .schemas
            .iter()
            .find_map(|schema| find_tagged_union(schema, definitions, discriminator_path)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum ExampleUnion {
        Alpha { value: String },
        Beta { count: u64 },
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct ExampleRoot {
        required_nullable: Option<String>,
        tagged: ExampleUnion,
    }

    #[test]
    fn descriptor_projects_nullable_and_tagged_union_nodes() {
        let descriptor = SemanticSchemaDescriptor::for_type::<ExampleRoot>(Vec::new());
        let schema = descriptor.json_schema();
        assert_eq!(
            schema.pointer("/properties/required_nullable/type"),
            Some(&serde_json::json!(["string", "null"]))
        );
        let tagged = descriptor
            .definitions()
            .get("ExampleUnion")
            .expect("tagged union definition");
        assert!(matches!(tagged, SemanticSchemaNode::TaggedUnion(_)));
    }

    #[test]
    fn integrity_rejects_duplicate_discriminators_and_semantic_type_collisions() {
        let mut descriptor = SemanticSchemaDescriptor::for_type::<ExampleRoot>(Vec::new());
        let SemanticSchemaNode::TaggedUnion(union) = descriptor
            .definitions
            .get_mut("ExampleUnion")
            .expect("tagged union definition")
        else {
            panic!("ExampleUnion must be tagged");
        };
        union.variants[1].discriminator_value = union.variants[0].discriminator_value.clone();
        union.variants[1].semantic_type = union.variants[0].semantic_type.clone();

        let errors = descriptor.integrity_errors();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicates discriminator value")),
            "{errors:#?}"
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("reuses semantic type")),
            "{errors:#?}"
        );
    }

    #[test]
    fn integrity_rejects_optional_discriminator_fields() {
        let mut descriptor = SemanticSchemaDescriptor::for_type::<ExampleRoot>(Vec::new());
        let SemanticSchemaNode::TaggedUnion(union) = descriptor
            .definitions
            .get_mut("ExampleUnion")
            .expect("tagged union definition")
        else {
            panic!("ExampleUnion must be tagged");
        };
        let SemanticSchemaNode::Object(branch) = union.variants[0].schema.as_mut() else {
            panic!("ExampleUnion branch must be an object");
        };
        branch
            .fields
            .iter_mut()
            .find(|field| field.field_name == "kind")
            .expect("kind discriminator field")
            .required = false;

        let errors = descriptor.integrity_errors();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("does not require discriminator")),
            "{errors:#?}"
        );
    }
}
