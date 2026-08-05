//! Canonical semantic schema descriptors for MCP wire contracts.

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde_json::{Map, Value};
use volicord_types::values::UtcTimestamp;

/// Type-owned entry point for an MCP semantic schema.
///
/// Rust wire types own their semantic name and leaf representation through
/// `JsonSchema`; the MCP descriptor layer adds the closed object-union catalog
/// below instead of inferring discriminators from incidental constants.
pub trait McpSemanticSchema: JsonSchema {
    fn mcp_semantic_type_name() -> String {
        Self::schema_name()
    }

    fn mcp_schema_document() -> Value {
        serde_json::to_value(schema_for!(Self)).unwrap_or_else(|error| {
            panic!(
                "{} schema must serialize: {error}",
                Self::mcp_semantic_type_name()
            )
        })
    }

    fn mcp_semantic_descriptor(
        canonical_examples: Vec<CanonicalSchemaExample>,
    ) -> SemanticSchemaDescriptor {
        SemanticSchemaDescriptor::from_json_schema(
            Self::mcp_semantic_type_name(),
            Self::mcp_schema_document(),
            canonical_examples,
        )
    }
}

impl<T: JsonSchema> McpSemanticSchema for T {}

/// One explicitly declared discriminator value and its caller-facing meaning.
///
/// Its ordinal in [`McpTaggedUnionContract::variants`] selects the schema at
/// the same ordinal in the wire type's declared union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpTaggedUnionVariantContract {
    pub discriminator_value: &'static str,
    pub semantic_type_suffix: &'static str,
    pub meaning: &'static str,
}

/// One explicitly declared public object union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpTaggedUnionContract {
    pub semantic_type_patterns: &'static [&'static str],
    pub discriminator_path: &'static str,
    pub variants: &'static [McpTaggedUnionVariantContract],
}

macro_rules! semantic_variants {
    ($($value:literal => $meaning:literal),+ $(,)?) => {
        &[$(
            McpTaggedUnionVariantContract {
                discriminator_value: $value,
                semantic_type_suffix: $value,
                meaning: $meaning,
            },
        )+]
    };
    ($($value:literal),+ $(,)?) => {
        &[$(
            McpTaggedUnionVariantContract {
                discriminator_value: $value,
                semantic_type_suffix: $value,
                meaning: concat!("Selects the `", $value, "` semantic branch."),
            },
        )+]
    };
}

/// Closed discriminator catalog used by production MCP descriptors.
///
/// A declaration matches its named semantic owner and exact branch count. Each
/// declared variant owns the same-position branch schema, and integrity checks
/// verify the declared discriminator without using constants for selection.
pub const MCP_TAGGED_UNION_CONTRACTS: &[McpTaggedUnionContract] = &[
    McpTaggedUnionContract {
        semantic_type_patterns: &["StaleShapingAuthorityAction"],
        discriminator_path: "/action",
        variants: semantic_variants!(
            "retire" => "Retires stale shaping authority without replacement authority.",
            "reauthorize" => "Creates a successor user-owned authority request."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &[
            "UserActionBasis",
            "UserActionDraft",
            "UserActionRequestBody",
        ],
        discriminator_path: "/action_type",
        variants: semantic_variants!(
            "choice" => "Requests one typed choice from a closed option set.",
            "evidence_observation" => "Requests one typed evidence observation."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &[
            "ToolResultOrRejected_for_*",
            "McpReadOnlyToolStructuredContent_for_*::response",
        ],
        discriminator_path: "/base/effect_kind",
        variants: semantic_variants!("read_only", "no_effect"),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["PreviewableToolResponse_for_*"],
        discriminator_path: "/base/response_kind",
        variants: semantic_variants!(
            "result" => "Returns the method's applied or read-only result.",
            "rejected" => "Returns an owner-defined rejection with no committed method effect.",
            "dry_run" => "Returns the validated preview without committing the method."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["EvidenceCaptureSpec", "McpEvidenceCaptureSpec"],
        discriminator_path: "/capture_kind",
        variants: semantic_variants!(
            "verified_command_execution" => "Captures a registered command execution.",
            "verified_tool_invocation" => "Captures a registered tool invocation."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["ToolError"],
        discriminator_path: "/code",
        variants: semantic_variants!(
            "VALIDATION_FAILED",
            "RUN_KIND_INCOMPATIBLE",
            "TASK_PHASE_TRANSITION_REQUIRED",
            "SHAPING_CHECKPOINT_REQUIRED",
            "SHAPING_CHECKPOINT_STALE",
            "USER_DECISION_UNRESOLVED",
            "CHANGE_UNIT_REQUIRED",
            "CHANGE_UNIT_STALE",
            "WORKSPACE_BASIS_STALE",
            "WORKFLOW_ACTION_NOT_ALLOWED",
            "PERSISTED_DATA_CORRUPT",
            "STATE_VERSION_CONFLICT",
            "INVOCATION_CONTEXT_MISMATCH",
            "NO_ACTIVE_TASK",
            "NO_ACTIVE_CHANGE_UNIT",
            "BASELINE_STALE",
            "SCOPE_REQUIRED",
            "SCOPE_VIOLATION",
            "WRITE_TICKET_REQUIRED",
            "WRITE_TICKET_INVALID",
            "APPROVAL_DENIED",
            "APPROVAL_EXPIRED",
            "APPROVAL_REQUIRED",
            "DECISION_UNRESOLVED",
            "AUTONOMY_BOUNDARY_EXCEEDED",
            "DECISION_REQUIRED",
            "CAPABILITY_INSUFFICIENT",
            "EVIDENCE_INSUFFICIENT",
            "RESIDUAL_RISK_NOT_VISIBLE",
            "ACCEPTANCE_REQUIRED",
            "PROJECTION_STALE",
            "ARTIFACT_MISSING",
            "VALIDATOR_FAILED",
            "OPERATION_RESULT_UNAVAILABLE"
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["WorkflowActionAuthorityCoordinates"],
        discriminator_path: "/coordinate_kind",
        variants: semantic_variants!(
            "record_shaping_checkpoint",
            "update_scope",
            "finalize_advice",
            "advance_task",
            "prepare_evidence_capture",
            "prepare_write",
            "stage_artifact",
            "record_run",
            "request_user_action",
            "reconcile_changes",
            "check_close",
            "close_task"
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &[
            "AdvanceTaskResultBase",
            "FinalizeAdviceResultBase",
            "IntakeResultBase",
            "PrepareEvidenceCaptureResultBase",
            "PrepareWriteResultBase",
            "RecordRunResultBase",
            "RecordShapingCheckpointResultBase",
            "RequestUserActionResultBase",
            "UpdateScopeResultBase",
        ],
        discriminator_path: "/effect_kind",
        variants: semantic_variants!("core_committed"),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["CloseTaskResultBase"],
        discriminator_path: "/effect_kind",
        variants: semantic_variants!("core_committed", "no_effect"),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &[
            "CheckCloseResultBase",
            "GetOperationResultResultBase",
            "StatusResultBase",
        ],
        discriminator_path: "/effect_kind",
        variants: semantic_variants!("read_only"),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["ReconcileChangesResultBase"],
        discriminator_path: "/effect_kind",
        variants: semantic_variants!("read_only", "core_committed"),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["StageArtifactResultBase"],
        discriminator_path: "/effect_kind",
        variants: semantic_variants!("staging_created"),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["McpMustSurfaceFact"],
        discriminator_path: "/fact_kind",
        variants: semantic_variants!(
            "method_rejected",
            "current_task_phase",
            "recovery_method",
            "shaping_decision_outcome",
            "non_authorizing_shaping_decision",
            "user_action_request_exists",
            "next_actor_is_user",
            "chat_reply_is_not_resolution",
            "product_repository_mutation_blocked_until_user_channel_resolution",
            "implementation_blocked_until_user_action_authority_satisfied",
            "entered_implementation",
            "phase_transition_created_no_write_ticket",
            "product_repository_writes_require_prepare_write"
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["UserActionResolutionForm"],
        discriminator_path: "/form_type",
        variants: semantic_variants!(
            "choice" => "Presents the choices accepted by a choice-action resolution.",
            "evidence_observation" => "Presents evidence targets and artifacts for an observation resolution."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["IntegrationVerificationWorkflowState"],
        discriminator_path: "/kind",
        variants: semantic_variants!(
            "awaiting_probe" => "Requires the declared integration probe.",
            "awaiting_observation" => "Requires observation of the declared integration result.",
            "complete" => "Confirms the integration-verification workflow is complete.",
            "repair_required" => "Requires the declared integration repair action."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["WorkflowProjection"],
        discriminator_path: "/kind",
        variants: semantic_variants!(
            "no_active_task" => "No Task is currently active.",
            "shaping_required" => "The active Task requires shaping.",
            "awaiting_user_action" => "The active Task is waiting for User Channel authority.",
            "decision_recovery_required" => "Stale shaping authority requires explicit recovery.",
            "ready_to_apply_decisions" => "Current user decisions are ready for application.",
            "ready_for_change_unit" => "The shaped Task is ready for a current Change Unit.",
            "ready_to_finalize_advice" => "Advisor work is ready for finalization.",
            "ready_for_implementation" => "The Task is ready to enter implementation.",
            "implementation" => "The Task is in implementation.",
            "close_review" => "The Task is undergoing close-readiness review.",
            "terminal" => "The Task is in a terminal state."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["McpRequestUserActionOperation"],
        discriminator_path: "/operation",
        variants: semantic_variants!(
            "create" => "Creates one new user-action request.",
            "resume" => "Resumes one existing user-action request."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &[
            "ShapingCheckpointOperation",
            "WorkflowCheckpointActionCoordinates",
        ],
        discriminator_path: "/operation",
        variants: semantic_variants!(
            "create_initial" => "Creates the first shaping checkpoint.",
            "replace_current" => "Replaces the exact current shaping checkpoint."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["McpUserActionResolutionSummary"],
        discriminator_path: "/resolution_type",
        variants: semantic_variants!(
            "choice" => "Summarizes the selected user-owned choice.",
            "evidence_observation" => "Summarizes the submitted evidence observation."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["McpMutationStructuredContent_for_*"],
        discriminator_path: "/result_type",
        variants: semantic_variants!(
            "rejected" => "Method rejection.",
            "dry_run" => "Non-committing preview.",
            "full" => "Full result and receipt.",
            "summary" => "Compact result and receipt.",
            "workflow" => "Result with refreshed workflow.",
            "operational_failure" => "Pre-effect operational failure.",
            "refresh_failure" => "Post-effect refresh failure.",
            "response_budget_exceeded" => "Applied result too large to inline.",
            "post_effect_failure" => "Post-effect failure.",
            "adapter_error" => "Pre-Core adapter failure."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["McpToolStructuredContent_for_*"],
        discriminator_path: "/result_type",
        variants: semantic_variants!(
            "response" => "Successful adapter-owned response.",
            "adapter_error" => "MCP adapter failure."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["McpReadOnlyToolStructuredContent_for_*"],
        discriminator_path: "/result_type",
        variants: semantic_variants!(
            "response" => "Read-only Core response.",
            "operational_failure" => "Operational read failure.",
            "adapter_error" => "Adapter failure before Core execution."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["SourceRef"],
        discriminator_path: "/source_kind",
        variants: semantic_variants!(
            "repository_file" => "Identifies an exact repository-file source.",
            "git_commit" => "Identifies an exact Git commit source.",
            "git_diff" => "Identifies an exact Git diff source.",
            "command" => "Identifies an exact command source.",
            "external_uri" => "Identifies an external URI source.",
            "user_context" => "Identifies explicit user-provided context."
        ),
    },
    McpTaggedUnionContract {
        semantic_type_patterns: &["EvidenceTarget"],
        discriminator_path: "/target_kind",
        variants: semantic_variants!(
            "acceptance_criterion" => "Targets one acceptance criterion.",
            "supplemental_claim" => "Targets one supplemental evidence claim."
        ),
    },
    #[cfg(test)]
    McpTaggedUnionContract {
        semantic_type_patterns: &["ExampleUnion", "ExplicitUnionWithIncidentalConstants"],
        discriminator_path: "/kind",
        variants: semantic_variants!("alpha", "beta"),
    },
    #[cfg(test)]
    McpTaggedUnionContract {
        semantic_type_patterns: &["LeftUnion"],
        discriminator_path: "/kind",
        variants: semantic_variants!("text"),
    },
    #[cfg(test)]
    McpTaggedUnionContract {
        semantic_type_patterns: &["RightUnion"],
        discriminator_path: "/kind",
        variants: semantic_variants!("count"),
    },
];

/// Validates the explicit discriminator catalog independently of generated schemas.
pub fn mcp_tagged_union_contract_integrity_errors() -> Vec<String> {
    let mut errors = Vec::new();
    let mut declarations = BTreeSet::new();
    let mut owners = BTreeSet::new();
    for contract in MCP_TAGGED_UNION_CONTRACTS {
        if contract.semantic_type_patterns.is_empty() {
            errors.push(format!(
                "tagged union `{}` has no semantic type owner",
                contract.discriminator_path
            ));
        }
        for pattern in contract.semantic_type_patterns {
            if pattern.is_empty() || pattern.matches('*').count() > 1 {
                errors.push(format!(
                    "tagged-union semantic owner pattern `{pattern}` is invalid"
                ));
            }
            if !owners.insert(*pattern) {
                errors.push(format!(
                    "duplicate tagged-union semantic owner pattern `{pattern}`"
                ));
            }
        }
        if !contract.discriminator_path.starts_with('/') {
            errors.push(format!(
                "tagged-union discriminator path `{}` is not an absolute JSON Pointer",
                contract.discriminator_path
            ));
        }
        let mut values = BTreeSet::new();
        for variant in contract.variants {
            if !values.insert(variant.discriminator_value) {
                errors.push(format!(
                    "tagged union `{}` duplicates discriminator value `{}`",
                    contract.discriminator_path, variant.discriminator_value
                ));
            }
            if variant.meaning.trim().is_empty() {
                errors.push(format!(
                    "tagged union `{}` value `{}` has no semantic meaning",
                    contract.discriminator_path, variant.discriminator_value
                ));
            }
            if variant.semantic_type_suffix.trim().is_empty() {
                errors.push(format!(
                    "tagged union `{}` value `{}` has no semantic variant type",
                    contract.discriminator_path, variant.discriminator_value
                ));
            }
        }
        let signature = format!(
            "{}:{}",
            contract.discriminator_path,
            values.into_iter().collect::<Vec<_>>().join(",")
        );
        if !declarations.insert(signature.clone()) {
            errors.push(format!("duplicate tagged-union declaration `{signature}`"));
        }
    }
    errors
}

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

/// One field contract resolved from the descriptor tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticFieldContract {
    pub semantic_type: String,
    pub required: bool,
    pub nullable: bool,
}

impl SemanticSchemaDescriptor {
    /// Projects a Rust wire type into the closed semantic representation.
    pub fn for_type<T: McpSemanticSchema>(canonical_examples: Vec<CanonicalSchemaExample>) -> Self {
        T::mcp_semantic_descriptor(canonical_examples)
    }

    /// Projects an MCP structured output whose JSON root is always an object.
    pub fn for_object_output<T: McpSemanticSchema>(
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
        Self {
            semantic_type,
            node,
            definitions,
            canonical_examples,
            dialect,
        }
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

    /// Resolves one JSON Pointer pattern through the semantic request tree.
    ///
    /// `*` selects an array item. Tagged-union and ordinary union branches are
    /// searched without guessing an instance branch, and the returned semantic
    /// type names are sorted and deduplicated.
    pub fn semantic_types_at_pointer_pattern(&self, pattern: &str) -> Vec<String> {
        let Some(segments) = pointer_pattern_segments(pattern) else {
            return Vec::new();
        };
        let mut resolved = BTreeSet::new();
        resolve_pointer_pattern(
            &self.node,
            &self.definitions,
            &segments,
            0,
            &mut BTreeSet::new(),
            &mut resolved,
        );
        resolved.into_iter().collect()
    }

    /// Resolves requiredness, nullability, and semantic type for one field path.
    pub fn field_contracts_at_pointer_pattern(&self, pattern: &str) -> Vec<SemanticFieldContract> {
        let Some(segments) = pointer_pattern_segments(pattern) else {
            return Vec::new();
        };
        let mut resolved = BTreeSet::new();
        resolve_field_contracts(
            &self.node,
            &self.definitions,
            &segments,
            0,
            &mut BTreeSet::new(),
            &mut resolved,
        );
        resolved.into_iter().collect()
    }

    /// Checks whether one concrete pointer selects a schema that accepts the value.
    pub fn accepts_value_at_pointer(&self, pointer: &str, value: &Value) -> bool {
        let Some(segments) = pointer_pattern_segments(pointer) else {
            return false;
        };
        pointer_accepts_value(
            &self.node,
            &self.definitions,
            &segments,
            value,
            0,
            &mut BTreeSet::new(),
        )
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

    /// Returns a deterministic digest of the generated documentation schema.
    pub fn schema_digest(&self) -> String {
        volicord_types::canonical::canonical_json_bare_sha256(&self.json_schema())
            .expect("semantic JSON Schema must have a canonical digest")
    }

    /// Returns a deterministic digest that also binds explicit union meanings.
    pub fn descriptor_digest(&self) -> String {
        let mut tagged_unions = Vec::new();
        collect_tagged_union_contracts(&self.node, "#", &mut tagged_unions);
        for (name, node) in &self.definitions {
            collect_tagged_union_contracts(
                node,
                &format!("#/definitions/{name}"),
                &mut tagged_unions,
            );
        }
        volicord_types::canonical::canonical_json_bare_sha256(&serde_json::json!({
            "semantic_type": self.semantic_type,
            "schema": self.json_schema(),
            "tagged_unions": tagged_unions,
        }))
        .expect("semantic descriptor must have a canonical digest")
    }

    /// Generates the annotation-preserving input projection used by discovery.
    pub fn runtime_json_schema(&self) -> Value {
        const MAX_SUMMARY_CHARS: usize = 320;

        let mut schema = self.node.to_runtime_json_schema(0);
        let object = schema
            .as_object_mut()
            .expect("semantic schema root must render as an object");
        if let Some(dialect) = &self.dialect {
            object.insert("$schema".to_owned(), Value::String(dialect.clone()));
        }
        if !self.definitions.is_empty() {
            let definition_depths = runtime_definition_depths(&self.node, &self.definitions);
            object.insert(
                "definitions".to_owned(),
                Value::Object(
                    self.definitions
                        .iter()
                        .map(|(name, node)| {
                            (
                                name.clone(),
                                node.to_runtime_json_schema(
                                    definition_depths.get(name).copied().unwrap_or(3),
                                ),
                            )
                        })
                        .collect(),
                ),
            );
        }
        let summary = bounded_chars(&runtime_semantic_summary(self), MAX_SUMMARY_CHARS);
        object.insert("description".to_owned(), Value::String(summary));
        schema
    }

    /// Generates the bounded root-object projection used by runtime tool discovery.
    pub fn compact_root_object_schema(&self) -> Value {
        let mut object = Map::new();
        object.insert("type".to_owned(), Value::String("object".to_owned()));
        if let Some(union) = root_tagged_union(&self.node, &self.definitions, 0) {
            let meanings = union
                .variants
                .iter()
                .map(|variant| format!("{}={}", variant.discriminator_value, variant.meaning))
                .collect::<Vec<_>>()
                .join("; ");
            object.insert(
                "properties".to_owned(),
                Value::Object(Map::from_iter([(
                    union.discriminator_path.trim_start_matches('/').to_owned(),
                    serde_json::json!({
                        "type": "string",
                        "enum": union
                            .variants
                            .iter()
                            .map(|variant| variant.discriminator_value.clone())
                            .collect::<Vec<_>>(),
                        "description": format!("Allowed result variants: {meanings}"),
                    }),
                )])),
            );
            object.insert(
                "required".to_owned(),
                Value::Array(vec![Value::String(
                    union.discriminator_path.trim_start_matches('/').to_owned(),
                )]),
            );
        }
        Value::Object(object)
    }

    /// Validates one JSON value against the semantic validator tree.
    pub fn validate(&self, value: &Value) -> SemanticValidationResult {
        let mut result = SemanticValidationResult::default();
        let context = ValidationContext {
            schema_node_id: "#".to_owned(),
            selected_variant: None,
            selected_variant_instance_path: None,
            semantic_type: Some(self.semantic_type.clone()),
            field_description: self.node.metadata().description.clone(),
            nested_item: false,
        };
        validate_node(
            &self.node,
            &self.definitions,
            value,
            "",
            0,
            &context,
            &mut result,
        );
        result.finish(self);
        result
    }

    /// Checks descriptor structural integrity independently of instance validation.
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
                match validation.selections.iter().find(|selection| {
                    selection.instance_path == expected.instance_path
                        && selection.discriminator_path == expected.discriminator_path
                }) {
                    Some(selection)
                        if selection.discriminator_value == expected.discriminator_value
                            && selection.semantic_type == expected.semantic_type => {}
                    Some(selection) => errors.push(format!(
                        "example `{}` selected `{}`/`{}` instead of `{}`/`{}`",
                        example.id,
                        selection.discriminator_value,
                        selection.semantic_type,
                        expected.discriminator_value,
                        expected.semantic_type
                    )),
                    None => errors.push(format!(
                        "example `{}` did not select tagged union {} at {}",
                        example.id, expected.discriminator_path, expected.instance_path
                    )),
                }
            }
        }
        errors.extend(runtime_semantic_parity_errors(self));
        errors
    }
}

fn runtime_semantic_parity_errors(descriptor: &SemanticSchemaDescriptor) -> Vec<String> {
    let mut errors = Vec::new();
    let documentation = descriptor.json_schema();
    let runtime = descriptor.runtime_json_schema();
    let documentation_properties = documentation
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().collect::<BTreeSet<_>>());
    let runtime_properties = runtime
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().collect::<BTreeSet<_>>());
    if runtime_properties != documentation_properties {
        errors.push(format!(
            "runtime/documentation root property semantics differ for `{}`",
            descriptor.semantic_type
        ));
    }
    for keyword in ["required", "additionalProperties"] {
        if runtime.get(keyword) != documentation.get(keyword) {
            errors.push(format!(
                "runtime/documentation root `{keyword}` semantics differ for `{}`",
                descriptor.semantic_type
            ));
        }
    }

    let mut expected_variants = Vec::new();
    collect_descriptor_runtime_variants(&descriptor.node, &mut expected_variants);
    for definition in descriptor.definitions.values() {
        collect_descriptor_runtime_variants(definition, &mut expected_variants);
    }
    expected_variants.sort();
    let mut actual_variants = Vec::new();
    collect_projected_runtime_variants(&runtime, &mut actual_variants, &mut errors);
    actual_variants.sort();
    if actual_variants != expected_variants {
        let first_mismatch = expected_variants
            .iter()
            .zip(&actual_variants)
            .find(|(expected, actual)| expected != actual)
            .map(|(expected, actual)| format!("expected {expected:?}, actual {actual:?}"))
            .unwrap_or_else(|| "variant counts differ".to_owned());
        errors.push(format!(
            "runtime/documentation tagged-union semantics differ for `{}` ({} expected, {} actual; {first_mismatch})",
            descriptor.semantic_type,
            expected_variants.len(),
            actual_variants.len(),
        ));
    }
    errors
}

fn collect_descriptor_runtime_variants(
    node: &SemanticSchemaNode,
    variants: &mut Vec<(String, String, String)>,
) {
    match node {
        SemanticSchemaNode::Object(object) => {
            for field in &object.fields {
                collect_descriptor_runtime_variants(&field.schema, variants);
            }
            if let SemanticAdditionalProperties::Schema(schema) = &object.additional_properties {
                collect_descriptor_runtime_variants(schema, variants);
            }
        }
        SemanticSchemaNode::Array(array) => {
            collect_descriptor_runtime_variants(&array.items, variants);
        }
        SemanticSchemaNode::Nullable(nullable) => {
            collect_descriptor_runtime_variants(&nullable.schema, variants);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            for variant in &union.variants {
                variants.push((
                    variant.semantic_type.clone(),
                    variant.discriminator_value.clone(),
                    variant.meaning.clone(),
                ));
                collect_descriptor_runtime_variants(&variant.schema, variants);
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                collect_descriptor_runtime_variants(variant, variants);
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                collect_descriptor_runtime_variants(schema, variants);
            }
        }
        SemanticSchemaNode::Reference(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => {}
    }
}

fn collect_projected_runtime_variants(
    value: &Value,
    variants: &mut Vec<(String, String, String)>,
    errors: &mut Vec<String>,
) {
    match value {
        Value::Object(object) => {
            if let Some(meaning) = object
                .get("x-volicord-variant-meaning")
                .and_then(Value::as_str)
            {
                let semantic_type = object
                    .get("x-volicord-semantic-type")
                    .and_then(Value::as_str);
                let discriminator = object
                    .get("enum")
                    .and_then(Value::as_array)
                    .filter(|values| values.len() == 1)
                    .and_then(|values| values[0].as_str());
                match (semantic_type, discriminator) {
                    (Some(semantic_type), Some(discriminator)) => variants.push((
                        semantic_type.to_owned(),
                        discriminator.to_owned(),
                        meaning.to_owned(),
                    )),
                    _ => errors.push(
                        "runtime tagged-union meaning lacks one semantic type and discriminator"
                            .to_owned(),
                    ),
                }
            }
            for child in object.values() {
                collect_projected_runtime_variants(child, variants, errors);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_projected_runtime_variants(item, variants, errors);
            }
        }
        _ => {}
    }
}

fn bounded_chars(value: &str, maximum: usize) -> String {
    if value.chars().count() <= maximum {
        return value.to_owned();
    }
    value
        .chars()
        .take(maximum.saturating_sub(1))
        .collect::<String>()
        + "…"
}

fn runtime_definition_depths(
    root: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    let mut pending = Vec::new();
    collect_runtime_definition_refs(root, 1, &mut pending);
    let mut index = 0;
    while index < pending.len() {
        let (name, depth) = pending[index].clone();
        index += 1;
        if depths.get(&name).is_some_and(|current| *current <= depth) {
            continue;
        }
        depths.insert(name.clone(), depth);
        if let Some(definition) = definitions.get(&name) {
            collect_runtime_definition_refs(definition, depth + 1, &mut pending);
        }
    }
    depths
}

fn collect_runtime_definition_refs(
    node: &SemanticSchemaNode,
    depth: usize,
    references: &mut Vec<(String, usize)>,
) {
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if let Some(name) = reference.reference.strip_prefix("#/definitions/") {
                references.push((name.to_owned(), depth));
            }
        }
        SemanticSchemaNode::Object(object) => {
            for field in &object.fields {
                collect_runtime_definition_refs(&field.schema, depth, references);
            }
            if let SemanticAdditionalProperties::Schema(schema) = &object.additional_properties {
                collect_runtime_definition_refs(schema, depth, references);
            }
        }
        SemanticSchemaNode::Array(array) => {
            collect_runtime_definition_refs(&array.items, depth, references);
        }
        SemanticSchemaNode::Nullable(nullable) => {
            collect_runtime_definition_refs(&nullable.schema, depth, references);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            for variant in &union.variants {
                collect_runtime_definition_refs(&variant.schema, depth, references);
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                collect_runtime_definition_refs(variant, depth, references);
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                collect_runtime_definition_refs(schema, depth, references);
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

fn runtime_semantic_summary(descriptor: &SemanticSchemaDescriptor) -> String {
    let mut required_nullable = Vec::new();
    let mut unions = Vec::new();
    collect_runtime_semantics(
        &descriptor.node,
        &descriptor.definitions,
        "",
        0,
        &mut BTreeSet::new(),
        &mut required_nullable,
        &mut unions,
    );
    required_nullable.sort();
    required_nullable.dedup();
    unions.sort();
    unions.dedup();

    let mut sections = vec![format!("Semantic type `{}`.", descriptor.semantic_type)];
    if !unions.is_empty() {
        sections.push(format!("Tagged unions: {}.", unions.join(" | ")));
    }
    if !required_nullable.is_empty() {
        sections.push(format!(
            "Required-nullable fields (must be present; JSON null or the named type): {}.",
            required_nullable.join(", ")
        ));
    }
    if let Some(example) = descriptor.canonical_examples.first() {
        sections.push(format!(
            "Canonical example `{}`: {}",
            example.id, example.description
        ));
    }
    sections.join(" ")
}

fn collect_runtime_semantics(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    path: &str,
    depth: usize,
    references: &mut BTreeSet<String>,
    required_nullable: &mut Vec<String>,
    unions: &mut Vec<String>,
) {
    if depth >= 32 {
        return;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if references.insert(reference.reference.clone()) {
                if let Some(target) = reference_target(definitions, &reference.reference) {
                    collect_runtime_semantics(
                        target,
                        definitions,
                        path,
                        depth + 1,
                        references,
                        required_nullable,
                        unions,
                    );
                }
                references.remove(&reference.reference);
            }
        }
        SemanticSchemaNode::Object(object) => {
            for field in &object.fields {
                let field_path = format!("{path}/{}", field.field_name);
                if field.required && field.nullable {
                    required_nullable
                        .push(format!("`{field_path}` (`{} | null`)", field.semantic_type));
                }
                collect_runtime_semantics(
                    &field.schema,
                    definitions,
                    &field_path,
                    depth + 1,
                    references,
                    required_nullable,
                    unions,
                );
            }
        }
        SemanticSchemaNode::Array(array) => collect_runtime_semantics(
            &array.items,
            definitions,
            &format!("{path}/*"),
            depth + 1,
            references,
            required_nullable,
            unions,
        ),
        SemanticSchemaNode::Nullable(nullable) => collect_runtime_semantics(
            &nullable.schema,
            definitions,
            path,
            depth + 1,
            references,
            required_nullable,
            unions,
        ),
        SemanticSchemaNode::TaggedUnion(union) => {
            unions.push(format!(
                "`{path}{}` = {}",
                union.discriminator_path,
                union
                    .variants
                    .iter()
                    .map(|variant| format!(
                        "`{}` ({})",
                        variant.discriminator_value, variant.meaning
                    ))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
            for variant in &union.variants {
                collect_runtime_semantics(
                    &variant.schema,
                    definitions,
                    path,
                    depth + 1,
                    references,
                    required_nullable,
                    unions,
                );
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                collect_runtime_semantics(
                    variant,
                    definitions,
                    path,
                    depth + 1,
                    references,
                    required_nullable,
                    unions,
                );
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                collect_runtime_semantics(
                    schema,
                    definitions,
                    path,
                    depth + 1,
                    references,
                    required_nullable,
                    unions,
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

fn collect_tagged_union_contracts(
    node: &SemanticSchemaNode,
    path: &str,
    contracts: &mut Vec<Value>,
) {
    match node {
        SemanticSchemaNode::Object(object) => {
            for field in &object.fields {
                collect_tagged_union_contracts(
                    &field.schema,
                    &format!("{path}/properties/{}", field.field_name),
                    contracts,
                );
            }
        }
        SemanticSchemaNode::Array(array) => {
            collect_tagged_union_contracts(&array.items, &format!("{path}/items"), contracts);
        }
        SemanticSchemaNode::Nullable(nullable) => {
            collect_tagged_union_contracts(&nullable.schema, path, contracts);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            contracts.push(serde_json::json!({
                "schema_path": path,
                "discriminator_path": union.discriminator_path,
                "variants": union.variants.iter().map(|variant| serde_json::json!({
                    "value": variant.discriminator_value,
                    "semantic_type": variant.semantic_type,
                    "meaning": variant.meaning,
                })).collect::<Vec<_>>(),
            }));
            for variant in &union.variants {
                collect_tagged_union_contracts(
                    &variant.schema,
                    &format!("{path}/variants/{}", variant.discriminator_value),
                    contracts,
                );
            }
        }
        SemanticSchemaNode::Union(union) => {
            for (index, variant) in union.variants.iter().enumerate() {
                collect_tagged_union_contracts(
                    variant,
                    &format!("{path}/{}/{index}", union.keyword),
                    contracts,
                );
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for (index, schema) in all_of.schemas.iter().enumerate() {
                collect_tagged_union_contracts(schema, &format!("{path}/allOf/{index}"), contracts);
            }
        }
        SemanticSchemaNode::Reference(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => {}
    }
}

fn root_tagged_union<'a>(
    node: &'a SemanticSchemaNode,
    definitions: &'a BTreeMap<String, SemanticSchemaNode>,
    depth: usize,
) -> Option<&'a SemanticTaggedUnionSchema> {
    if depth >= 16 {
        return None;
    }
    match node {
        SemanticSchemaNode::TaggedUnion(union) => Some(union),
        SemanticSchemaNode::Reference(reference) => {
            reference_target(definitions, &reference.reference)
                .and_then(|target| root_tagged_union(target, definitions, depth + 1))
        }
        SemanticSchemaNode::Nullable(nullable) => {
            root_tagged_union(&nullable.schema, definitions, depth + 1)
        }
        SemanticSchemaNode::AllOf(all_of) => all_of
            .schemas
            .iter()
            .find_map(|schema| root_tagged_union(schema, definitions, depth + 1)),
        _ => None,
    }
}

fn pointer_pattern_segments(pattern: &str) -> Option<Vec<String>> {
    if pattern.is_empty() {
        return Some(Vec::new());
    }
    pattern.strip_prefix('/').map(|path| {
        path.split('/')
            .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
            .collect()
    })
}

fn resolve_pointer_pattern(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    segments: &[String],
    depth: usize,
    references: &mut BTreeSet<String>,
    resolved: &mut BTreeSet<String>,
) {
    if depth > 64 {
        return;
    }
    if segments.is_empty() {
        resolved.insert(node.semantic_type_name());
        return;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if references.insert(reference.reference.clone()) {
                if let Some(target) = reference_target(definitions, &reference.reference) {
                    resolve_pointer_pattern(
                        target,
                        definitions,
                        segments,
                        depth + 1,
                        references,
                        resolved,
                    );
                }
                references.remove(&reference.reference);
            }
        }
        SemanticSchemaNode::Nullable(nullable) => resolve_pointer_pattern(
            &nullable.schema,
            definitions,
            segments,
            depth + 1,
            references,
            resolved,
        ),
        SemanticSchemaNode::Object(object) => {
            if let Some(field) = object
                .fields
                .iter()
                .find(|field| field.field_name == segments[0])
            {
                resolve_pointer_pattern(
                    &field.schema,
                    definitions,
                    &segments[1..],
                    depth + 1,
                    references,
                    resolved,
                );
            } else {
                match &object.additional_properties {
                    SemanticAdditionalProperties::Allowed if segments.len() == 1 => {
                        resolved.insert("json".to_owned());
                    }
                    SemanticAdditionalProperties::Schema(schema) => resolve_pointer_pattern(
                        schema,
                        definitions,
                        &segments[1..],
                        depth + 1,
                        references,
                        resolved,
                    ),
                    SemanticAdditionalProperties::Allowed
                    | SemanticAdditionalProperties::Forbidden => {}
                }
            }
        }
        SemanticSchemaNode::Array(array)
            if segments[0] == "*" || segments[0].parse::<usize>().is_ok() =>
        {
            resolve_pointer_pattern(
                &array.items,
                definitions,
                &segments[1..],
                depth + 1,
                references,
                resolved,
            );
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            for variant in &union.variants {
                resolve_pointer_pattern(
                    &variant.schema,
                    definitions,
                    segments,
                    depth + 1,
                    references,
                    resolved,
                );
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                resolve_pointer_pattern(
                    variant,
                    definitions,
                    segments,
                    depth + 1,
                    references,
                    resolved,
                );
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                resolve_pointer_pattern(
                    schema,
                    definitions,
                    segments,
                    depth + 1,
                    references,
                    resolved,
                );
            }
        }
        SemanticSchemaNode::Array(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => {}
    }
}

fn resolve_field_contracts(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    segments: &[String],
    depth: usize,
    references: &mut BTreeSet<String>,
    resolved: &mut BTreeSet<SemanticFieldContract>,
) {
    if depth > 64 || segments.is_empty() {
        return;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if references.insert(reference.reference.clone()) {
                if let Some(target) = reference_target(definitions, &reference.reference) {
                    resolve_field_contracts(
                        target,
                        definitions,
                        segments,
                        depth + 1,
                        references,
                        resolved,
                    );
                }
                references.remove(&reference.reference);
            }
        }
        SemanticSchemaNode::Nullable(nullable) => resolve_field_contracts(
            &nullable.schema,
            definitions,
            segments,
            depth + 1,
            references,
            resolved,
        ),
        SemanticSchemaNode::Object(object) => {
            if let Some(field) = object
                .fields
                .iter()
                .find(|field| field.field_name == segments[0])
            {
                if segments.len() == 1 {
                    resolved.insert(SemanticFieldContract {
                        semantic_type: if field.nullable {
                            format!("{} | null", field.semantic_type)
                        } else {
                            field.semantic_type.clone()
                        },
                        required: field.required,
                        nullable: field.nullable,
                    });
                } else {
                    resolve_field_contracts(
                        &field.schema,
                        definitions,
                        &segments[1..],
                        depth + 1,
                        references,
                        resolved,
                    );
                }
            }
        }
        SemanticSchemaNode::Array(array)
            if segments[0] == "*" || segments[0].parse::<usize>().is_ok() =>
        {
            resolve_field_contracts(
                &array.items,
                definitions,
                &segments[1..],
                depth + 1,
                references,
                resolved,
            );
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            for variant in &union.variants {
                resolve_field_contracts(
                    &variant.schema,
                    definitions,
                    segments,
                    depth + 1,
                    references,
                    resolved,
                );
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                resolve_field_contracts(
                    variant,
                    definitions,
                    segments,
                    depth + 1,
                    references,
                    resolved,
                );
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                resolve_field_contracts(
                    schema,
                    definitions,
                    segments,
                    depth + 1,
                    references,
                    resolved,
                );
            }
        }
        SemanticSchemaNode::Array(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => {}
    }
}

fn pointer_accepts_value(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    segments: &[String],
    value: &Value,
    depth: usize,
    references: &mut BTreeSet<String>,
) -> bool {
    if depth > 64 {
        return false;
    }
    if segments.is_empty() {
        let mut result = SemanticValidationResult::default();
        let context = ValidationContext {
            schema_node_id: "#fixed_argument".to_owned(),
            selected_variant: None,
            selected_variant_instance_path: None,
            semantic_type: Some(node.semantic_type_name()),
            field_description: node.metadata().description.clone(),
            nested_item: false,
        };
        validate_node(node, definitions, value, "", 0, &context, &mut result);
        return result.issues.is_empty();
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            if !references.insert(reference.reference.clone()) {
                return false;
            }
            let accepted =
                reference_target(definitions, &reference.reference).is_some_and(|target| {
                    pointer_accepts_value(
                        target,
                        definitions,
                        segments,
                        value,
                        depth + 1,
                        references,
                    )
                });
            references.remove(&reference.reference);
            accepted
        }
        SemanticSchemaNode::Nullable(nullable) => pointer_accepts_value(
            &nullable.schema,
            definitions,
            segments,
            value,
            depth + 1,
            references,
        ),
        SemanticSchemaNode::Object(object) => {
            if let Some(field) = object
                .fields
                .iter()
                .find(|field| field.field_name == segments[0])
            {
                pointer_accepts_value(
                    &field.schema,
                    definitions,
                    &segments[1..],
                    value,
                    depth + 1,
                    references,
                )
            } else {
                match &object.additional_properties {
                    SemanticAdditionalProperties::Allowed => segments.len() == 1,
                    SemanticAdditionalProperties::Forbidden => false,
                    SemanticAdditionalProperties::Schema(schema) => pointer_accepts_value(
                        schema,
                        definitions,
                        &segments[1..],
                        value,
                        depth + 1,
                        references,
                    ),
                }
            }
        }
        SemanticSchemaNode::Array(array)
            if segments[0] == "*" || segments[0].parse::<usize>().is_ok() =>
        {
            pointer_accepts_value(
                &array.items,
                definitions,
                &segments[1..],
                value,
                depth + 1,
                references,
            )
        }
        SemanticSchemaNode::TaggedUnion(union) => union.variants.iter().any(|variant| {
            pointer_accepts_value(
                &variant.schema,
                definitions,
                segments,
                value,
                depth + 1,
                references,
            )
        }),
        SemanticSchemaNode::Union(union) => union.variants.iter().any(|variant| {
            pointer_accepts_value(variant, definitions, segments, value, depth + 1, references)
        }),
        SemanticSchemaNode::AllOf(all_of) => all_of.schemas.iter().any(|schema| {
            pointer_accepts_value(schema, definitions, segments, value, depth + 1, references)
        }),
        SemanticSchemaNode::Array(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => false,
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

    fn to_runtime_json_schema(&self, depth: usize) -> Value {
        let mut object = self
            .metadata()
            .validation
            .clone()
            .into_iter()
            .collect::<Map<_, _>>();
        match self {
            Self::Object(schema) => {
                object.insert("type".to_owned(), Value::String("object".to_owned()));
                if depth < 2 {
                    object.insert(
                        "properties".to_owned(),
                        Value::Object(
                            schema
                                .fields
                                .iter()
                                .map(|field| {
                                    (
                                        field.field_name.clone(),
                                        field.schema.to_runtime_json_schema(depth + 1),
                                    )
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
                            object.insert(
                                "additionalProperties".to_owned(),
                                schema.to_runtime_json_schema(depth + 1),
                            );
                        }
                    }
                } else if let Some(semantic_type) = schema.metadata.title.as_ref() {
                    object.insert(
                        "x-volicord-semantic-type".to_owned(),
                        Value::String(semantic_type.clone()),
                    );
                }
            }
            Self::Array(schema) => {
                object.insert("type".to_owned(), Value::String("array".to_owned()));
                object.insert(
                    "items".to_owned(),
                    schema.items.to_runtime_json_schema(depth + 1),
                );
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
                let inner = schema.schema.to_runtime_json_schema(depth + 1);
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
                        .expect("primitive runtime schema has an object root")
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
                        Value::Array(vec![inner, serde_json::json!({"type": "null"})]),
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
                object.insert("type".to_owned(), Value::String("object".to_owned()));
                object.insert(
                    "oneOf".to_owned(),
                    Value::Array(
                        schema
                            .variants
                            .iter()
                            .map(|variant| runtime_discriminator_branch(schema, variant))
                            .collect(),
                    ),
                );
                let mut nested_variants = Vec::new();
                for variant in &schema.variants {
                    collect_runtime_variant_annotations(&variant.schema, &mut nested_variants);
                }
                if !nested_variants.is_empty() {
                    object.insert(
                        "x-volicord-nested-variants".to_owned(),
                        Value::Array(nested_variants),
                    );
                }
            }
            Self::Union(schema) => {
                object.insert(
                    schema.keyword.to_owned(),
                    Value::Array(
                        schema
                            .variants
                            .iter()
                            .map(|variant| variant.to_runtime_json_schema(depth + 1))
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
                            .map(|schema| schema.to_runtime_json_schema(depth + 1))
                            .collect(),
                    ),
                );
            }
        }
        Value::Object(object)
    }
}

fn runtime_discriminator_branch(
    union: &SemanticTaggedUnionSchema,
    variant: &SemanticTaggedUnionVariant,
) -> Value {
    let mut leaf = serde_json::json!({
        "enum": [variant.discriminator_value.clone()],
        "x-volicord-semantic-type": variant.semantic_type,
        "x-volicord-variant-meaning": variant.meaning,
    });
    for segment in union
        .discriminator_path
        .trim_start_matches('/')
        .split('/')
        .rev()
    {
        leaf = serde_json::json!({
            "type": "object",
            "properties": { (segment): leaf },
            "required": [segment],
        });
    }
    leaf
}

fn collect_runtime_variant_annotations(node: &SemanticSchemaNode, variants: &mut Vec<Value>) {
    match node {
        SemanticSchemaNode::Object(object) => {
            for field in &object.fields {
                collect_runtime_variant_annotations(&field.schema, variants);
            }
            if let SemanticAdditionalProperties::Schema(schema) = &object.additional_properties {
                collect_runtime_variant_annotations(schema, variants);
            }
        }
        SemanticSchemaNode::Array(array) => {
            collect_runtime_variant_annotations(&array.items, variants);
        }
        SemanticSchemaNode::Nullable(nullable) => {
            collect_runtime_variant_annotations(&nullable.schema, variants);
        }
        SemanticSchemaNode::TaggedUnion(union) => {
            for variant in &union.variants {
                variants.push(serde_json::json!({
                    "enum": [variant.discriminator_value.clone()],
                    "x-volicord-semantic-type": variant.semantic_type,
                    "x-volicord-variant-meaning": variant.meaning,
                }));
                collect_runtime_variant_annotations(&variant.schema, variants);
            }
        }
        SemanticSchemaNode::Union(union) => {
            for variant in &union.variants {
                collect_runtime_variant_annotations(variant, variants);
            }
        }
        SemanticSchemaNode::AllOf(all_of) => {
            for schema in &all_of.schemas {
                collect_runtime_variant_annotations(schema, variants);
            }
        }
        SemanticSchemaNode::Reference(_)
        | SemanticSchemaNode::String(_)
        | SemanticSchemaNode::Integer(_)
        | SemanticSchemaNode::Number(_)
        | SemanticSchemaNode::Boolean(_)
        | SemanticSchemaNode::Null(_)
        | SemanticSchemaNode::Enum(_) => {}
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
    pub meaning: String,
    pub schema: Box<SemanticSchemaNode>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticValidationIssue {
    pub path: String,
    pub code: SemanticValidationIssueCode,
    pub message: String,
    pub schema_node_id: String,
    pub selected_variant: Option<String>,
    pub expected_semantic_type: Option<String>,
    pub field_description: Option<String>,
    pub allowed_values: Vec<String>,
    discriminator: bool,
    nested_item: bool,
    selected_variant_instance_path: Option<String>,
    variant_summary: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticValidationResult {
    pub issues: Vec<SemanticValidationIssue>,
    pub truncated: bool,
    pub selected_variant: Option<String>,
    pub canonical_example: Option<Map<String, Value>>,
    selections: Vec<SemanticTaggedSelection>,
}

impl SemanticValidationResult {
    pub const MAX_ISSUES: usize = 32;

    fn push(&mut self, issue: SemanticValidationIssue) {
        if self.issues.contains(&issue) {
            return;
        }
        self.issues.push(issue);
        self.issues.sort_by(issue_order);
        if self.issues.len() > Self::MAX_ISSUES {
            self.issues.truncate(Self::MAX_ISSUES);
            self.truncated = true;
        }
    }

    fn select_variant(&mut self, selection: SemanticTaggedSelection) {
        if !self.selections.contains(&selection) {
            self.selections.push(selection);
        }
    }

    fn finish(&mut self, descriptor: &SemanticSchemaDescriptor) {
        self.issues.sort_by(issue_order);
        if self.issues.iter().any(|issue| issue.discriminator) {
            self.issues.retain(|issue| issue.discriminator);
        }
        let primary = self.issues.first();
        let selected = primary
            .and_then(|issue| {
                issue.selected_variant.as_ref().map(|value| {
                    (
                        issue.selected_variant_instance_path.as_deref(),
                        value.as_str(),
                    )
                })
            })
            .and_then(|(instance_path, value)| {
                self.selections.iter().find(|selection| {
                    selection.discriminator_value == value
                        && instance_path == Some(selection.instance_path.as_str())
                })
            })
            .or_else(|| self.selections.first());
        self.selected_variant = selected.map(|selection| selection.discriminator_value.clone());
        self.canonical_example = primary
            .and_then(|issue| issue.variant_summary.clone())
            .or_else(|| {
                selected.and_then(|selection| {
                    descriptor.canonical_examples.iter().find_map(|example| {
                        example
                            .expected_variants
                            .iter()
                            .any(|expected| {
                                expected.instance_path == selection.instance_path
                                    && expected.discriminator_value == selection.discriminator_value
                            })
                            .then(|| example.value.as_object().cloned())
                            .flatten()
                    })
                })
            })
            .or_else(|| {
                descriptor
                    .canonical_examples
                    .first()
                    .and_then(|example| example.value.as_object().cloned())
            });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticTaggedSelection {
    instance_path: String,
    discriminator_path: String,
    discriminator_value: String,
    semantic_type: String,
}

#[derive(Debug, Clone)]
struct ValidationContext {
    schema_node_id: String,
    selected_variant: Option<String>,
    selected_variant_instance_path: Option<String>,
    semantic_type: Option<String>,
    field_description: Option<String>,
    nested_item: bool,
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
            declared_tagged_union(branches, fallback_type, definitions, metadata.clone())
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
        Some("object") | None => {
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
                    let node = parse_schema_node(field_schema, name, definitions);
                    let semantic_type = node.semantic_type_name();
                    let required = required.contains(name.as_str());
                    let description = field_schema
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            format!("`{name}` carries the `{semantic_type}` wire value.")
                        });
                    SemanticObjectField {
                        field_name: name.clone(),
                        required,
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

fn declared_tagged_union(
    branches: &[Value],
    fallback_type: &str,
    definitions: &Map<String, Value>,
    mut metadata: SemanticNodeMetadata,
) -> Option<SemanticTaggedUnionSchema> {
    let contract = MCP_TAGGED_UNION_CONTRACTS.iter().find(|contract| {
        contract.variants.len() == branches.len()
            && contract
                .semantic_type_patterns
                .iter()
                .any(|pattern| semantic_type_pattern_matches(pattern, fallback_type))
    })?;
    let variants = contract
        .variants
        .iter()
        .zip(branches)
        .map(|(variant, branch)| {
            let semantic_type = format!("{fallback_type}::{}", variant.semantic_type_suffix);
            SemanticTaggedUnionVariant {
                discriminator_value: variant.discriminator_value.to_owned(),
                semantic_type: semantic_type.clone(),
                meaning: variant.meaning.to_owned(),
                schema: Box::new(parse_schema_node(branch, &semantic_type, definitions)),
            }
        })
        .collect::<Vec<_>>();
    if metadata.description.is_none() {
        metadata.description = Some(format!(
            "Discriminator `{}`: {}",
            contract.discriminator_path,
            contract
                .variants
                .iter()
                .map(|variant| format!("`{}` — {}", variant.discriminator_value, variant.meaning))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    Some(SemanticTaggedUnionSchema {
        discriminator_path: contract.discriminator_path.to_owned(),
        variants,
        metadata,
    })
}

fn semantic_type_pattern_matches(pattern: &str, semantic_type: &str) -> bool {
    let Some((prefix, suffix)) = pattern.split_once('*') else {
        return pattern == semantic_type;
    };
    semantic_type.starts_with(prefix)
        && semantic_type.ends_with(suffix)
        && semantic_type.len() >= prefix.len() + suffix.len()
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
    context: &ValidationContext,
    result: &mut SemanticValidationResult,
) {
    if depth >= 64 {
        result.truncated = true;
        return;
    }
    let nullable_constraint_context;
    let constraint_context = if let SemanticSchemaNode::Nullable(nullable) = node {
        nullable_constraint_context = context_for_nullable(context, &nullable.schema);
        &nullable_constraint_context
    } else {
        context
    };
    if node
        .metadata()
        .validation
        .get("not")
        .and_then(|constraint| constraint.get("const"))
        == Some(instance)
    {
        result.push(type_issue(
            path,
            constraint_context,
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
                    context,
                    instance,
                    "Referenced semantic type is unavailable.",
                ));
                return;
            };
            let mut target_context = context.clone();
            target_context.schema_node_id = reference.reference.clone();
            target_context.semantic_type = context
                .semantic_type
                .clone()
                .or_else(|| Some(reference.semantic_type.clone()));
            target_context.field_description = context
                .field_description
                .clone()
                .or_else(|| reference.metadata.description.clone())
                .or_else(|| target.metadata().description.clone());
            validate_node(
                target,
                definitions,
                instance,
                path,
                depth + 1,
                &target_context,
                result,
            );
        }
        SemanticSchemaNode::Nullable(nullable) => {
            if !instance.is_null() {
                let nullable_context = context_for_nullable(context, &nullable.schema);
                validate_node(
                    &nullable.schema,
                    definitions,
                    instance,
                    path,
                    depth + 1,
                    &nullable_context,
                    result,
                );
            }
        }
        SemanticSchemaNode::Object(schema) => {
            let Some(instance) = instance.as_object() else {
                result.push(type_issue(path, context, instance, "Expected an object."));
                return;
            };
            if !object_length_valid(&schema.metadata, instance.len()) {
                result.push(type_issue(
                    path,
                    context,
                    &Value::Object(instance.clone()),
                    "Object member count does not satisfy the semantic schema.",
                ));
            }
            for field in schema.fields.iter().filter(|field| field.required) {
                if !instance.contains_key(&field.field_name) {
                    let field_context = context_for_field(context, field);
                    result.push(validation_issue(
                        pointer_child(path, &field.field_name),
                        SemanticValidationIssueCode::Required,
                        format!("Required argument `{}` is missing.", field.field_name),
                        &field_context,
                    ));
                }
            }
            for (name, value) in instance {
                if let Some(field) = schema.fields.iter().find(|field| field.field_name == *name) {
                    let field_context = context_for_field(context, field);
                    validate_node(
                        &field.schema,
                        definitions,
                        value,
                        &pointer_child(path, name),
                        depth + 1,
                        &field_context,
                        result,
                    );
                    continue;
                }
                match &schema.additional_properties {
                    SemanticAdditionalProperties::Allowed => {}
                    SemanticAdditionalProperties::Forbidden => {
                        result.push(validation_issue(
                            pointer_child(path, name),
                            SemanticValidationIssueCode::Unknown,
                            format!("Unknown argument `{name}` is not allowed."),
                            context,
                        ));
                    }
                    SemanticAdditionalProperties::Schema(schema) => {
                        let mut additional_context = context.clone();
                        additional_context.schema_node_id =
                            format!("{}/additionalProperties", context.schema_node_id);
                        additional_context.semantic_type = Some(schema.semantic_type_name());
                        validate_node(
                            schema,
                            definitions,
                            value,
                            &pointer_child(path, name),
                            depth + 1,
                            &additional_context,
                            result,
                        );
                    }
                }
            }
        }
        SemanticSchemaNode::Array(schema) => {
            let Some(items) = instance.as_array() else {
                result.push(type_issue(path, context, instance, "Expected an array."));
                return;
            };
            if !array_length_valid(&schema.metadata, items.len()) {
                result.push(type_issue(
                    path,
                    context,
                    instance,
                    "Array length does not satisfy the semantic schema.",
                ));
            }
            if schema
                .metadata
                .validation
                .get("uniqueItems")
                .and_then(Value::as_bool)
                == Some(true)
                && items
                    .iter()
                    .enumerate()
                    .any(|(index, value)| items[..index].contains(value))
            {
                result.push(type_issue(
                    path,
                    context,
                    instance,
                    "Array items must be unique.",
                ));
            }
            for (index, value) in items.iter().enumerate() {
                let mut item_context = context.clone();
                item_context.schema_node_id = format!("{}/items", context.schema_node_id);
                item_context.semantic_type = Some(schema.items.semantic_type_name());
                item_context.field_description = schema
                    .metadata
                    .description
                    .clone()
                    .or_else(|| context.field_description.clone());
                item_context.nested_item = true;
                validate_node(
                    &schema.items,
                    definitions,
                    value,
                    &pointer_child(path, &index.to_string()),
                    depth + 1,
                    &item_context,
                    result,
                );
            }
        }
        SemanticSchemaNode::String(metadata) => {
            validate_primitive(instance, path, "string", metadata, context, result)
        }
        SemanticSchemaNode::Integer(metadata) => {
            validate_primitive(instance, path, "integer", metadata, context, result)
        }
        SemanticSchemaNode::Number(metadata) => {
            validate_primitive(instance, path, "number", metadata, context, result)
        }
        SemanticSchemaNode::Boolean(metadata) => {
            validate_primitive(instance, path, "boolean", metadata, context, result)
        }
        SemanticSchemaNode::Null(metadata) => {
            validate_primitive(instance, path, "null", metadata, context, result)
        }
        SemanticSchemaNode::Enum(schema) => {
            if !value_matches_type(instance, &schema.value_type) {
                let mut issue = type_issue(
                    path,
                    context,
                    instance,
                    &format!("Expected {} enum value.", schema.value_type),
                );
                issue.allowed_values = schema.values.iter().map(enum_value_text).collect();
                result.push(issue);
            } else if !schema.values.contains(instance) {
                let mut issue = validation_issue(
                    path.to_owned(),
                    SemanticValidationIssueCode::EnumValue,
                    format!(
                        "Expected one of [{}], but received {}.",
                        schema
                            .values
                            .iter()
                            .map(Value::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        instance,
                    ),
                    context,
                );
                issue.allowed_values = schema.values.iter().map(enum_value_text).collect();
                result.push(issue);
            }
        }
        SemanticSchemaNode::TaggedUnion(schema) => {
            let discriminator_path = format!("{path}{}", schema.discriminator_path);
            let allowed_values = schema
                .variants
                .iter()
                .map(|variant| variant.discriminator_value.clone())
                .collect::<Vec<_>>();
            let meanings = schema
                .variants
                .iter()
                .map(|variant| format!("`{}` — {}", variant.discriminator_value, variant.meaning))
                .collect::<Vec<_>>()
                .join("; ");
            let summary = tagged_variant_summary(&discriminator_path, schema);
            let mut discriminator_context = context.clone();
            discriminator_context.schema_node_id = format!(
                "{}/discriminator{}",
                context.schema_node_id, schema.discriminator_path
            );
            discriminator_context.semantic_type = Some(
                context
                    .semantic_type
                    .clone()
                    .unwrap_or_else(|| node.semantic_type_name()),
            );
            discriminator_context.field_description = schema
                .metadata
                .description
                .clone()
                .or_else(|| context.field_description.clone());
            let Some(value) = instance.pointer(&schema.discriminator_path) else {
                let mut issue = validation_issue(
                    discriminator_path,
                    SemanticValidationIssueCode::Required,
                    format!(
                        "Tagged union discriminator `{}` is required. Allowed variants: {meanings}",
                        schema.discriminator_path,
                    ),
                    &discriminator_context,
                );
                issue.allowed_values = allowed_values;
                issue.discriminator = true;
                issue.variant_summary = Some(summary);
                result.push(issue);
                return;
            };
            let Some(value) = value.as_str() else {
                let mut issue = type_issue(
                    &discriminator_path,
                    &discriminator_context,
                    value,
                    "Expected a string discriminator.",
                );
                issue.allowed_values = allowed_values;
                issue.discriminator = true;
                issue.variant_summary = Some(summary);
                result.push(issue);
                return;
            };
            let Some(variant) = schema
                .variants
                .iter()
                .find(|variant| variant.discriminator_value == value)
            else {
                let mut issue = validation_issue(
                    discriminator_path,
                    SemanticValidationIssueCode::EnumValue,
                    format!(
                        "Received discriminator value {}. Allowed variants: {meanings}",
                        Value::String(value.to_owned()),
                    ),
                    &discriminator_context,
                );
                issue.allowed_values = allowed_values;
                issue.discriminator = true;
                issue.variant_summary = Some(summary);
                result.push(issue);
                return;
            };
            result.select_variant(SemanticTaggedSelection {
                instance_path: path.to_owned(),
                discriminator_path: schema.discriminator_path.clone(),
                discriminator_value: variant.discriminator_value.clone(),
                semantic_type: variant.semantic_type.clone(),
            });
            let mut variant_context = context.clone();
            variant_context.schema_node_id = format!(
                "{}/variants/{}",
                context.schema_node_id,
                variant
                    .discriminator_value
                    .replace('~', "~0")
                    .replace('/', "~1")
            );
            variant_context.selected_variant = Some(variant.discriminator_value.clone());
            variant_context.selected_variant_instance_path = Some(path.to_owned());
            variant_context.semantic_type = Some(variant.semantic_type.clone());
            validate_node(
                &variant.schema,
                definitions,
                instance,
                path,
                depth + 1,
                &variant_context,
                result,
            );
        }
        SemanticSchemaNode::Union(schema) => validate_union(
            &schema.variants,
            definitions,
            instance,
            path,
            depth,
            context,
            result,
        ),
        SemanticSchemaNode::AllOf(schema) => {
            for child in &schema.schemas {
                validate_node(
                    child,
                    definitions,
                    instance,
                    path,
                    depth + 1,
                    context,
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
    context: &ValidationContext,
    result: &mut SemanticValidationResult,
) {
    let matching = variants
        .iter()
        .filter(|variant| node_matches_instance_kind(variant, definitions, instance, 0))
        .collect::<Vec<_>>();
    if let [variant] = matching.as_slice() {
        let mut variant_context = context.clone();
        variant_context.schema_node_id = format!("{}/type_variant", context.schema_node_id);
        variant_context.semantic_type = Some(variant.semantic_type_name());
        validate_node(
            variant,
            definitions,
            instance,
            path,
            depth + 1,
            &variant_context,
            result,
        );
        return;
    }
    let expected = variants
        .iter()
        .map(SemanticSchemaNode::semantic_type_name)
        .collect::<Vec<_>>()
        .join(" | ");
    let mut union_context = context.clone();
    union_context.semantic_type = Some(expected);
    let detail = if matching.is_empty() {
        "Value does not match any type-distinct semantic union branch."
    } else {
        "Value matches overlapping semantic union branches; an explicit discriminator is required."
    };
    result.push(type_issue(path, &union_context, instance, detail));
}

fn validate_primitive(
    instance: &Value,
    path: &str,
    primitive: &str,
    metadata: &SemanticNodeMetadata,
    context: &ValidationContext,
    result: &mut SemanticValidationResult,
) {
    if !value_matches_type(instance, primitive) {
        result.push(type_issue(
            path,
            context,
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
                    context,
                    instance,
                    "String length does not satisfy the semantic schema.",
                ));
            }
            if let Some(pattern) = metadata.validation.get("pattern").and_then(Value::as_str) {
                if Regex::new(pattern).is_ok_and(|pattern| !pattern.is_match(value)) {
                    result.push(type_issue(
                        path,
                        context,
                        instance,
                        "String does not satisfy the semantic schema pattern.",
                    ));
                }
            }
            if metadata.validation.get("format").and_then(Value::as_str) == Some("date-time")
                && UtcTimestamp::parse(value).is_err()
            {
                result.push(type_issue(
                    path,
                    context,
                    instance,
                    "String is not a valid RFC 3339 date-time.",
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
                context,
                instance,
                "Number does not satisfy the semantic schema range.",
            ));
        }
        let exclusive_minimum = metadata
            .validation
            .get("exclusiveMinimum")
            .and_then(Value::as_f64);
        let exclusive_maximum = metadata
            .validation
            .get("exclusiveMaximum")
            .and_then(Value::as_f64);
        let multiple_of = metadata
            .validation
            .get("multipleOf")
            .and_then(Value::as_f64);
        if value.is_some_and(|value| {
            exclusive_minimum.is_some_and(|minimum| value <= minimum)
                || exclusive_maximum.is_some_and(|maximum| value >= maximum)
                || multiple_of.is_some_and(|multiple| {
                    multiple <= 0.0 || (value / multiple - (value / multiple).round()).abs() > 1e-9
                })
        }) {
            result.push(type_issue(
                path,
                context,
                instance,
                "Number does not satisfy the semantic schema constraint.",
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

fn object_length_valid(metadata: &SemanticNodeMetadata, length: usize) -> bool {
    let length = length as u64;
    let minimum = metadata
        .validation
        .get("minProperties")
        .and_then(Value::as_u64);
    let maximum = metadata
        .validation
        .get("maxProperties")
        .and_then(Value::as_u64);
    !minimum.is_some_and(|minimum| length < minimum)
        && !maximum.is_some_and(|maximum| length > maximum)
}

fn type_issue(
    path: &str,
    context: &ValidationContext,
    instance: &Value,
    detail: &str,
) -> SemanticValidationIssue {
    let expected = context
        .semantic_type
        .as_deref()
        .unwrap_or("the declared semantic type");
    validation_issue(
        path.to_owned(),
        SemanticValidationIssueCode::TypeMismatch,
        format!(
            "{detail} Expected `{expected}`, but received {}.",
            instance_type_name(instance)
        ),
        context,
    )
}

fn validation_issue(
    path: String,
    code: SemanticValidationIssueCode,
    message: String,
    context: &ValidationContext,
) -> SemanticValidationIssue {
    SemanticValidationIssue {
        path,
        code,
        message,
        schema_node_id: context.schema_node_id.clone(),
        selected_variant: context.selected_variant.clone(),
        expected_semantic_type: context.semantic_type.clone(),
        field_description: context.field_description.clone(),
        allowed_values: Vec::new(),
        discriminator: false,
        nested_item: context.nested_item,
        selected_variant_instance_path: context.selected_variant_instance_path.clone(),
        variant_summary: None,
    }
}

fn context_for_field(
    context: &ValidationContext,
    field: &SemanticObjectField,
) -> ValidationContext {
    ValidationContext {
        schema_node_id: format!(
            "{}/properties/{}",
            context.schema_node_id,
            field.field_name.replace('~', "~0").replace('/', "~1")
        ),
        selected_variant: context.selected_variant.clone(),
        selected_variant_instance_path: context.selected_variant_instance_path.clone(),
        semantic_type: Some(field.semantic_type.clone()),
        field_description: Some(field.description.clone()),
        nested_item: context.nested_item,
    }
}

fn context_for_nullable(
    context: &ValidationContext,
    schema: &SemanticSchemaNode,
) -> ValidationContext {
    let mut nullable_context = context.clone();
    let non_null_type = context
        .semantic_type
        .clone()
        .unwrap_or_else(|| schema.semantic_type_name());
    nullable_context.semantic_type = Some(if non_null_type.ends_with(" | null") {
        non_null_type
    } else {
        format!("{non_null_type} | null")
    });
    nullable_context
}

fn tagged_variant_summary(
    discriminator_path: &str,
    schema: &SemanticTaggedUnionSchema,
) -> Map<String, Value> {
    Map::from_iter([
        (
            "discriminator_path".to_owned(),
            Value::String(discriminator_path.to_owned()),
        ),
        (
            "variants".to_owned(),
            Value::Array(
                schema
                    .variants
                    .iter()
                    .map(|variant| {
                        Value::Object(Map::from_iter([
                            (
                                "value".to_owned(),
                                Value::String(variant.discriminator_value.clone()),
                            ),
                            (
                                "semantic_type".to_owned(),
                                Value::String(variant.semantic_type.clone()),
                            ),
                            ("meaning".to_owned(), Value::String(variant.meaning.clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
    ])
}

fn enum_value_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn issue_order(
    left: &SemanticValidationIssue,
    right: &SemanticValidationIssue,
) -> std::cmp::Ordering {
    issue_rank(left)
        .cmp(&issue_rank(right))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| format!("{:?}", left.code).cmp(&format!("{:?}", right.code)))
        .then_with(|| left.message.cmp(&right.message))
        .then_with(|| left.schema_node_id.cmp(&right.schema_node_id))
}

fn issue_rank(issue: &SemanticValidationIssue) -> u8 {
    if issue.discriminator {
        return 0;
    }
    if issue.nested_item {
        return 5;
    }
    match issue.code {
        SemanticValidationIssueCode::TypeMismatch => 1,
        SemanticValidationIssueCode::Required => 2,
        SemanticValidationIssueCode::Unknown => 3,
        SemanticValidationIssueCode::EnumValue => 4,
    }
}

fn node_matches_instance_kind(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    instance: &Value,
    depth: usize,
) -> bool {
    if depth >= 32 {
        return false;
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            reference_target(definitions, &reference.reference).is_some_and(|target| {
                node_matches_instance_kind(target, definitions, instance, depth + 1)
            })
        }
        SemanticSchemaNode::Nullable(nullable) => {
            instance.is_null()
                || node_matches_instance_kind(&nullable.schema, definitions, instance, depth + 1)
        }
        SemanticSchemaNode::Object(_) | SemanticSchemaNode::TaggedUnion(_) => instance.is_object(),
        SemanticSchemaNode::Array(_) => instance.is_array(),
        SemanticSchemaNode::String(_) => instance.is_string(),
        SemanticSchemaNode::Integer(_) => instance
            .as_number()
            .is_some_and(|number| number.is_i64() || number.is_u64()),
        SemanticSchemaNode::Number(_) => instance.is_number(),
        SemanticSchemaNode::Boolean(_) => instance.is_boolean(),
        SemanticSchemaNode::Null(_) => instance.is_null(),
        SemanticSchemaNode::Enum(schema) => value_matches_type(instance, &schema.value_type),
        SemanticSchemaNode::Union(union) => union
            .variants
            .iter()
            .any(|variant| node_matches_instance_kind(variant, definitions, instance, depth + 1)),
        SemanticSchemaNode::AllOf(all_of) => all_of
            .schemas
            .iter()
            .all(|schema| node_matches_instance_kind(schema, definitions, instance, depth + 1)),
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

fn validate_node_integrity(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    path: &str,
    required_context: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    validate_constraint_type_compatibility(node, path, errors);
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
                let declared_value = rendered_discriminator_value(
                    &variant.schema,
                    definitions,
                    &union.discriminator_path,
                );
                if declared_value.as_ref() != Some(&variant.discriminator_value) {
                    errors.push(format!(
                        "{path} variant `{}` does not declare discriminator `{}` as `{}` (found {})",
                        variant.semantic_type,
                        union.discriminator_path,
                        variant.discriminator_value,
                        declared_value.as_deref().unwrap_or("no exact value")
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
            } else {
                for left in 0..union.variants.len() {
                    for right in (left + 1)..union.variants.len() {
                        let left_kinds = instance_kinds(&union.variants[left], definitions, 0);
                        let right_kinds = instance_kinds(&union.variants[right], definitions, 0);
                        if left_kinds.iter().any(|kind| right_kinds.contains(kind)) {
                            errors.push(format!(
                                "{path} has overlapping union variants {left} and {right} without an explicit discriminator"
                            ));
                        }
                    }
                }
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

fn validate_constraint_type_compatibility(
    node: &SemanticSchemaNode,
    path: &str,
    errors: &mut Vec<String>,
) {
    let node_kind = match node {
        SemanticSchemaNode::Object(_) => Some("object"),
        SemanticSchemaNode::Array(_) => Some("array"),
        SemanticSchemaNode::String(_) => Some("string"),
        SemanticSchemaNode::Integer(_) => Some("integer"),
        SemanticSchemaNode::Number(_) => Some("number"),
        SemanticSchemaNode::Boolean(_) => Some("boolean"),
        SemanticSchemaNode::Null(_) => Some("null"),
        SemanticSchemaNode::Enum(schema) => Some(schema.value_type.as_str()),
        SemanticSchemaNode::Nullable(_)
        | SemanticSchemaNode::Reference(_)
        | SemanticSchemaNode::TaggedUnion(_)
        | SemanticSchemaNode::Union(_)
        | SemanticSchemaNode::AllOf(_) => None,
    };
    let Some(node_kind) = node_kind else {
        return;
    };
    for constraint in node.metadata().validation.keys() {
        let allowed = match constraint.as_str() {
            "minLength" | "maxLength" | "pattern" => node_kind == "string",
            "format" => matches!(node_kind, "string" | "integer" | "number"),
            "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" | "multipleOf" => {
                matches!(node_kind, "integer" | "number")
            }
            "minItems" | "maxItems" | "uniqueItems" => node_kind == "array",
            "minProperties" | "maxProperties" => node_kind == "object",
            _ => true,
        };
        if !allowed {
            errors.push(format!(
                "{path} applies `{constraint}` to incompatible semantic type `{node_kind}`"
            ));
        }
    }
}

fn instance_kinds(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    depth: usize,
) -> BTreeSet<&'static str> {
    if depth >= 32 {
        return BTreeSet::new();
    }
    match node {
        SemanticSchemaNode::Reference(reference) => {
            reference_target(definitions, &reference.reference)
                .map(|target| instance_kinds(target, definitions, depth + 1))
                .unwrap_or_default()
        }
        SemanticSchemaNode::Nullable(nullable) => {
            let mut kinds = instance_kinds(&nullable.schema, definitions, depth + 1);
            kinds.insert("null");
            kinds
        }
        SemanticSchemaNode::Object(_) | SemanticSchemaNode::TaggedUnion(_) => {
            BTreeSet::from(["object"])
        }
        SemanticSchemaNode::Array(_) => BTreeSet::from(["array"]),
        SemanticSchemaNode::String(_) => BTreeSet::from(["string"]),
        SemanticSchemaNode::Integer(_) => BTreeSet::from(["integer"]),
        SemanticSchemaNode::Number(_) => BTreeSet::from(["integer", "number"]),
        SemanticSchemaNode::Boolean(_) => BTreeSet::from(["boolean"]),
        SemanticSchemaNode::Null(_) => BTreeSet::from(["null"]),
        SemanticSchemaNode::Enum(schema) => match schema.value_type.as_str() {
            "number" => BTreeSet::from(["integer", "number"]),
            "integer" => BTreeSet::from(["integer"]),
            "boolean" => BTreeSet::from(["boolean"]),
            _ => BTreeSet::from(["string"]),
        },
        SemanticSchemaNode::Union(union) => union
            .variants
            .iter()
            .flat_map(|variant| instance_kinds(variant, definitions, depth + 1))
            .collect(),
        SemanticSchemaNode::AllOf(all_of) => {
            let mut schemas = all_of.schemas.iter();
            let Some(first) = schemas.next() else {
                return BTreeSet::new();
            };
            let mut kinds = instance_kinds(first, definitions, depth + 1);
            for schema in schemas {
                let child = instance_kinds(schema, definitions, depth + 1);
                kinds.retain(|kind| child.contains(kind));
            }
            kinds
        }
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

fn rendered_discriminator_value(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    discriminator_path: &str,
) -> Option<String> {
    exact_semantic_string_at_path(node, definitions, discriminator_path, 0)
}

fn exact_semantic_string_at_path(
    node: &SemanticSchemaNode,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    path: &str,
    depth: usize,
) -> Option<String> {
    if depth > 32 {
        return None;
    }
    if path.is_empty() {
        return match node {
            SemanticSchemaNode::Reference(reference) => {
                reference_target(definitions, &reference.reference).and_then(|target| {
                    exact_semantic_string_at_path(target, definitions, path, depth + 1)
                })
            }
            SemanticSchemaNode::Enum(schema) => match schema.values.as_slice() {
                [Value::String(value)] => Some(value.clone()),
                _ => None,
            },
            SemanticSchemaNode::String(metadata) => metadata
                .validation
                .get("const")
                .and_then(Value::as_str)
                .map(str::to_owned),
            SemanticSchemaNode::AllOf(all_of) => {
                let values = all_of
                    .schemas
                    .iter()
                    .filter_map(|schema| {
                        exact_semantic_string_at_path(schema, definitions, path, depth + 1)
                    })
                    .collect::<BTreeSet<_>>();
                (values.len() == 1)
                    .then(|| values.into_iter().next())
                    .flatten()
            }
            SemanticSchemaNode::TaggedUnion(union) => common_exact_semantic_string(
                union.variants.iter().map(|variant| variant.schema.as_ref()),
                definitions,
                path,
                depth,
            ),
            SemanticSchemaNode::Union(union) => {
                common_exact_semantic_string(union.variants.iter(), definitions, path, depth)
            }
            _ => None,
        };
    }
    let remaining = path.strip_prefix('/')?;
    let (segment, tail) = remaining
        .split_once('/')
        .map_or((remaining, None), |(segment, tail)| (segment, Some(tail)));
    match node {
        SemanticSchemaNode::Reference(reference) => {
            reference_target(definitions, &reference.reference).and_then(|target| {
                exact_semantic_string_at_path(target, definitions, path, depth + 1)
            })
        }
        SemanticSchemaNode::Object(object) => object
            .fields
            .iter()
            .find(|field| field.field_name == segment)
            .and_then(|field| {
                tail.map_or_else(
                    || exact_semantic_string_at_path(&field.schema, definitions, "", depth + 1),
                    |tail| {
                        exact_semantic_string_at_path(
                            &field.schema,
                            definitions,
                            &format!("/{tail}"),
                            depth + 1,
                        )
                    },
                )
            }),
        SemanticSchemaNode::Nullable(nullable) => {
            exact_semantic_string_at_path(&nullable.schema, definitions, path, depth + 1)
        }
        SemanticSchemaNode::AllOf(all_of) => {
            let values = all_of
                .schemas
                .iter()
                .filter_map(|schema| {
                    exact_semantic_string_at_path(schema, definitions, path, depth + 1)
                })
                .collect::<BTreeSet<_>>();
            (values.len() == 1)
                .then(|| values.into_iter().next())
                .flatten()
        }
        SemanticSchemaNode::TaggedUnion(union) => common_exact_semantic_string(
            union.variants.iter().map(|variant| variant.schema.as_ref()),
            definitions,
            path,
            depth,
        ),
        SemanticSchemaNode::Union(union) => {
            common_exact_semantic_string(union.variants.iter(), definitions, path, depth)
        }
        _ => None,
    }
}

fn common_exact_semantic_string<'a>(
    nodes: impl Iterator<Item = &'a SemanticSchemaNode>,
    definitions: &BTreeMap<String, SemanticSchemaNode>,
    path: &str,
    depth: usize,
) -> Option<String> {
    let values = nodes
        .map(|node| exact_semantic_string_at_path(node, definitions, path, depth + 1))
        .collect::<Option<BTreeSet<_>>>()?;
    (values.len() == 1)
        .then(|| values.into_iter().next())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use volicord_types::{
        ids::BaselineRef,
        schema::{
            RequiredNullable, UserActionBasis, UserActionRequestBody, UserActionResolutionForm,
        },
    };

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

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum LeftUnion {
        Text { shared: String },
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum RightUnion {
        Count { shared: u64 },
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct SameNameSiblings {
        left: LeftUnion,
        right: RightUnion,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(untagged)]
    enum UntaggedObjectUnion {
        Alpha { alpha: String },
        Beta { beta: u64 },
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct UntaggedRoot {
        value: UntaggedObjectUnion,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct ConstraintRoot {
        #[schemars(regex(pattern = "^[a-z]+$"))]
        patterned: String,
        timestamp: UtcTimestamp,
        #[schemars(range(min = 2, max = 4))]
        count: u64,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    enum OrderEnum {
        Allowed,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct OrderedIssues {
        typed: String,
        required: String,
        enum_value: OrderEnum,
        items: Vec<u64>,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct NestedBaseline {
        authority_baseline: RequiredNullable<BaselineRef>,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct TypeOwnedBaselineRoot {
        current_baseline: RequiredNullable<BaselineRef>,
        nested: NestedBaseline,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct MisleadingFieldName {
        baseline_ref: RequiredNullable<String>,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    enum AlphaConstant {
        AlphaFixed,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    enum BetaConstant {
        BetaFixed,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum ExplicitUnionWithIncidentalConstants {
        Alpha { a_constant: AlphaConstant },
        Beta { a_constant: BetaConstant },
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum UndeclaredLookalikeUnion {
        Alpha { value: String },
        Beta { count: u64 },
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct ExplicitUnionRoot {
        value: ExplicitUnionWithIncidentalConstants,
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

    #[test]
    fn same_name_siblings_keep_branch_local_issue_context() {
        let descriptor = SemanticSchemaDescriptor::for_type::<SameNameSiblings>(Vec::new());
        let validation = descriptor.validate(&serde_json::json!({
            "left": {"kind": "text", "shared": 7},
            "right": {"kind": "count", "shared": "seven"}
        }));
        let left = validation
            .issues
            .iter()
            .find(|issue| issue.path == "/left/shared")
            .expect("left issue");
        let right = validation
            .issues
            .iter()
            .find(|issue| issue.path == "/right/shared")
            .expect("right issue");

        assert_eq!(left.selected_variant.as_deref(), Some("text"));
        assert_eq!(left.expected_semantic_type.as_deref(), Some("string"));
        assert_eq!(right.selected_variant.as_deref(), Some("count"));
        assert_eq!(right.expected_semantic_type.as_deref(), Some("integer"));
        assert_ne!(left.schema_node_id, right.schema_node_id);
    }

    #[test]
    fn untagged_object_union_reports_one_non_guessing_issue() {
        let descriptor = SemanticSchemaDescriptor::for_type::<UntaggedRoot>(Vec::new());
        let validation = descriptor.validate(&serde_json::json!({
            "value": {"unrelated": true}
        }));

        assert_eq!(validation.issues.len(), 1);
        assert_eq!(validation.issues[0].path, "/value");
        assert!(validation.issues[0]
            .message
            .contains("explicit discriminator is required"));
        assert!(descriptor
            .integrity_errors()
            .iter()
            .any(|error| error.contains("object union without an explicit discriminator")));
    }

    #[test]
    fn semantic_constraints_reject_pattern_format_and_range_mismatches() {
        let descriptor = SemanticSchemaDescriptor::for_type::<ConstraintRoot>(Vec::new());
        let validation = descriptor.validate(&serde_json::json!({
            "patterned": "UPPER",
            "timestamp": "not-a-timestamp",
            "count": 1
        }));

        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.path == "/patterned" && issue.message.contains("pattern")));
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.path == "/timestamp" && issue.message.contains("date-time")));
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.path == "/count" && issue.message.contains("range")));
    }

    #[test]
    fn issues_have_stable_semantic_precedence() {
        let descriptor = SemanticSchemaDescriptor::for_type::<OrderedIssues>(Vec::new());
        let validation = descriptor.validate(&serde_json::json!({
            "typed": false,
            "enum_value": "invalid",
            "items": ["invalid"],
            "unknown": true
        }));

        assert_eq!(
            validation
                .issues
                .iter()
                .map(|issue| issue.path.as_str())
                .collect::<Vec<_>>(),
            ["/typed", "/required", "/unknown", "/enum_value", "/items/0",]
        );
    }

    #[test]
    fn baseline_ref_and_required_nullable_are_type_owned_at_every_field_name() {
        let descriptor = SemanticSchemaDescriptor::for_type::<TypeOwnedBaselineRoot>(Vec::new());
        assert_eq!(
            descriptor.semantic_types_at_pointer_pattern("/current_baseline"),
            ["BaselineRef"]
        );
        assert_eq!(
            descriptor.semantic_types_at_pointer_pattern("/nested/authority_baseline"),
            ["BaselineRef"]
        );

        for valid in [
            serde_json::json!({
                "current_baseline": null,
                "nested": {"authority_baseline": null}
            }),
            serde_json::json!({
                "current_baseline": "baseline_current_001",
                "nested": {"authority_baseline": "baseline_authority_001"}
            }),
        ] {
            assert!(descriptor.validate(&valid).issues.is_empty());
            assert!(serde_json::from_value::<TypeOwnedBaselineRoot>(valid).is_ok());
        }

        for invalid in [
            serde_json::json!({
                "current_baseline": "null",
                "nested": {"authority_baseline": null}
            }),
            serde_json::json!({
                "current_baseline": "",
                "nested": {"authority_baseline": null}
            }),
            serde_json::json!({
                "current_baseline": null,
                "nested": {"authority_baseline": "null"}
            }),
            serde_json::json!({
                "nested": {"authority_baseline": null}
            }),
        ] {
            assert!(
                !descriptor.validate(&invalid).issues.is_empty(),
                "{invalid}"
            );
            assert!(serde_json::from_value::<TypeOwnedBaselineRoot>(invalid).is_err());
        }

        let misleading = SemanticSchemaDescriptor::for_type::<MisleadingFieldName>(Vec::new());
        for ordinary_string in [
            serde_json::json!({"baseline_ref": "null"}),
            serde_json::json!({"baseline_ref": ""}),
        ] {
            assert!(misleading.validate(&ordinary_string).issues.is_empty());
            assert!(serde_json::from_value::<MisleadingFieldName>(ordinary_string).is_ok());
        }
        assert_eq!(
            misleading.semantic_types_at_pointer_pattern("/baseline_ref"),
            ["string"]
        );
    }

    #[test]
    fn declared_discriminator_ignores_unrelated_singleton_constants() {
        let descriptor = SemanticSchemaDescriptor::for_type::<ExplicitUnionRoot>(Vec::new());
        let SemanticSchemaNode::TaggedUnion(union) = descriptor
            .definitions()
            .get("ExplicitUnionWithIncidentalConstants")
            .expect("explicit union definition")
        else {
            panic!("union must use the explicit tagged representation");
        };
        assert_eq!(union.discriminator_path, "/kind");
        assert_eq!(
            union
                .variants
                .iter()
                .map(|variant| variant.discriminator_value.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta"]
        );
    }

    #[test]
    fn discriminator_contract_does_not_attach_to_an_unrelated_lookalike_type() {
        let descriptor = SemanticSchemaDescriptor::for_type::<UndeclaredLookalikeUnion>(Vec::new());
        assert!(matches!(descriptor.node(), SemanticSchemaNode::Union(_)));
        assert!(descriptor
            .integrity_errors()
            .iter()
            .any(|error| error.contains("object union without an explicit discriminator")));
    }

    #[test]
    fn public_user_action_body_unions_have_explicit_type_owned_discriminators() {
        fn assert_tagged<T: McpSemanticSchema>(path: &str, values: &[&str]) {
            let descriptor = SemanticSchemaDescriptor::for_type::<T>(Vec::new());
            let SemanticSchemaNode::TaggedUnion(union) = descriptor.node() else {
                panic!(
                    "{} must be an explicit tagged union",
                    descriptor.semantic_type()
                );
            };
            assert_eq!(union.discriminator_path, path);
            assert_eq!(
                union
                    .variants
                    .iter()
                    .map(|variant| variant.discriminator_value.as_str())
                    .collect::<Vec<_>>(),
                values
            );
        }

        assert_tagged::<UserActionRequestBody>("/action_type", &["choice", "evidence_observation"]);
        assert_tagged::<UserActionBasis>("/action_type", &["choice", "evidence_observation"]);
        assert_tagged::<UserActionResolutionForm>(
            "/form_type",
            &["choice", "evidence_observation"],
        );
    }

    #[test]
    fn catalog_and_descriptor_digests_are_deterministic() {
        assert!(mcp_tagged_union_contract_integrity_errors().is_empty());
        let first = SemanticSchemaDescriptor::for_type::<ExampleRoot>(Vec::new());
        let second = SemanticSchemaDescriptor::for_type::<ExampleRoot>(Vec::new());
        assert_eq!(first.schema_digest(), second.schema_digest());
        assert_eq!(first.descriptor_digest(), second.descriptor_digest());
        assert_ne!(first.schema_digest(), first.descriptor_digest());
    }

    #[test]
    fn integrity_rejects_constraint_and_type_mismatches() {
        let mut descriptor = SemanticSchemaDescriptor::for_type::<ConstraintRoot>(Vec::new());
        let SemanticSchemaNode::Object(root) = &mut descriptor.node else {
            panic!("constraint fixture must be an object");
        };
        let patterned = root
            .fields
            .iter_mut()
            .find(|field| field.field_name == "patterned")
            .expect("patterned field");
        let SemanticSchemaNode::String(metadata) = patterned.schema.as_mut() else {
            panic!("patterned field must be a string");
        };
        metadata
            .validation
            .insert("minimum".to_owned(), serde_json::json!(1));

        assert!(descriptor
            .integrity_errors()
            .iter()
            .any(|error| error.contains("incompatible semantic type `string`")));
    }
}
