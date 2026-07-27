//! Semantic identities and exact identifier catalogs for public JSON contracts.

use std::collections::{BTreeMap, BTreeSet};

use schemars::{schema_for, JsonSchema};
use serde_json::{json, Value};

use crate::methods::{
    public_request_schema, public_response_schema, public_result_schema, OperationResultRef,
};
use crate::schema::{
    AcceptanceCriterion, AcceptanceCriterionInput, AcceptanceCriterionReplacement,
    AcceptedRiskInput, AgentSafeUserActionRequestSummary, AgentSession, ArtifactInput, ArtifactRef,
    AuthorityReceipt, CarryForwardDisposition, ChangeUnitEffectContract, CloseAssessmentInput,
    CloseReadinessBlocker, ContinuityCursor, ContinuityPageInfo, ContinuityPageRequest,
    CurrentCloseBasis, DryRunSummary, EventRef, EvidenceCaptureIntent, EvidenceCaptureReceipt,
    EvidenceCaptureSpec, EvidenceCoverageItem, EvidenceCoverageUpdate, EvidenceGateSummary,
    EvidenceObservation, EvidenceObservationInput, EvidenceProducer, EvidenceProducerAnchor,
    EvidenceRelevanceAssessment, EvidenceSummary, EvidenceTarget, EvidenceUpdateProvenance,
    GuaranteeDisclosure, GuaranteeDisplay, GuardEvent, GuardInstallation, MutationAssessment,
    NextActionSummary, ObservedChanges, PathAssessment, PlannedBlocker, PlannedEffect,
    PlatformDiagnosticDetail, ProjectContinuityPage, ProjectContinuityRecord,
    ProjectContinuitySummary, ProjectEnforcementProfile, ProjectWorkflowPolicySummary,
    PromptCapture, ResidualRisk, ResidualRiskInput, RiskAcceptanceCoverage, RunSummary,
    SensitiveActionRequirement, SensitiveActionScope, ShapingGap, ShapingReadiness, SourceRef,
    StagedArtifactHandle, StateRecordRef, StateSummary, SummaryCard, TaskFlowItem,
    TaskLifecycleState, TaskLineageInput, TaskLineageSummary, ToolDryRunResponse, ToolEnvelope,
    ToolError, ToolRejectedResponse, ToolResultBase, UnrecordedChange, UnrecordedChangeFinding,
    UnrecordedChangeResolutionSummary, UserActionBasis, UserActionBasisCoordinates,
    UserActionChoiceBasis, UserActionChoiceDraft, UserActionChoiceRequestBody, UserActionContext,
    UserActionDraft, UserActionEvidenceObservation, UserActionEvidenceObservationBasis,
    UserActionEvidenceObservationDraft, UserActionEvidenceObservationRequestBody, UserActionOption,
    UserActionOptionInput, UserActionRequest, UserActionRequestBody, UserActionResolution,
    UserActionResolutionBody, UserActionResolutionChoice, UserActionResolutionForm,
    UserActionResolutionInput, ValidatorResult, WorkspaceContext, WriteDecisionReason, WriteTicket,
    WriteTicketAttemptScope, WriteTicketPathPatterns, WriteTicketScope, WriteTicketStateSummary,
    WriteTicketValidityBasis,
};
use crate::values::{MethodName, UserActionStatus};

/// One exact JSON instance shape exposed by a semantic contract descriptor.
#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum JsonExampleShape {
    /// A method name and its complete `params` object.
    CompleteMethodRequest,
    /// A method's `params` object without the method-name wrapper.
    MethodParams,
    /// A complete successful public method response.
    CompleteMethodResponse,
    /// The successful result body for one public method.
    MethodResultBody,
    /// The rejected response body accepted for one public method.
    MethodRejection,
    /// One exact named public schema object.
    PublicSchemaObject(String),
    /// One complete structured CLI output document.
    CliOutput,
    /// One exact MCP wire request.
    McpWireRequest(String),
    /// One exact MCP wire response.
    McpWireResponse(String),
    /// One exact persisted object.
    PersistedObject(String),
    /// One exact diagnostic object.
    DiagnosticObject(String),
}

impl JsonExampleShape {
    /// Returns the stable Markdown fence value for this shape.
    pub fn id(&self) -> String {
        match self {
            Self::CompleteMethodRequest => "complete_request".to_owned(),
            Self::MethodParams => "params".to_owned(),
            Self::CompleteMethodResponse => "complete_response".to_owned(),
            Self::MethodResultBody => "result_body".to_owned(),
            Self::MethodRejection => "rejection".to_owned(),
            Self::PublicSchemaObject(name) => format!("schema_object.{name}"),
            Self::CliOutput => "cli_output".to_owned(),
            Self::McpWireRequest(name) => format!("mcp_request.{name}"),
            Self::McpWireResponse(name) => format!("mcp_response.{name}"),
            Self::PersistedObject(name) => format!("persisted_object.{name}"),
            Self::DiagnosticObject(name) => format!("diagnostic_object.{name}"),
        }
    }
}

/// Exact identifier categories extracted from an owner schema.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsonContractIdentifiers {
    properties: BTreeSet<String>,
    values: BTreeSet<String>,
    schema_names: BTreeSet<String>,
}

impl JsonContractIdentifiers {
    /// Returns exact JSON property identifiers.
    pub const fn properties(&self) -> &BTreeSet<String> {
        &self.properties
    }

    /// Returns exact closed string values.
    pub const fn values(&self) -> &BTreeSet<String> {
        &self.values
    }

    /// Returns exact named schema identifiers.
    pub const fn schema_names(&self) -> &BTreeSet<String> {
        &self.schema_names
    }

    fn extend(&mut self, other: Self) {
        self.properties.extend(other.properties);
        self.values.extend(other.values);
        self.schema_names.extend(other.schema_names);
    }
}

/// One stable semantic contract owned by a public JSON-schema source.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonContractDescriptor {
    id: String,
    identifiers: JsonContractIdentifiers,
    related_contracts: Vec<String>,
    schema: Option<Value>,
    example_schemas: BTreeMap<String, Value>,
}

impl JsonContractDescriptor {
    fn from_schema(id: impl Into<String>, schema: Value, related_contracts: Vec<String>) -> Self {
        let mut example_schemas = BTreeMap::new();
        if let Some(title) = schema.get("title").and_then(Value::as_str) {
            example_schemas.insert(
                JsonExampleShape::PublicSchemaObject(title.to_owned()).id(),
                schema.clone(),
            );
        }
        Self {
            id: id.into(),
            identifiers: identifiers_from_json_schema(&schema),
            related_contracts,
            schema: Some(schema),
            example_schemas,
        }
    }

    /// Constructs a descriptor for another crate that owns a JSON schema.
    pub fn from_owner_schema(
        id: impl Into<String>,
        schema: Value,
        identifiers: JsonContractIdentifiers,
        related_contracts: Vec<String>,
    ) -> Self {
        let mut example_schemas = BTreeMap::new();
        if let Some(title) = schema.get("title").and_then(Value::as_str) {
            example_schemas.insert(
                JsonExampleShape::PublicSchemaObject(title.to_owned()).id(),
                schema.clone(),
            );
        }
        Self {
            id: id.into(),
            identifiers,
            related_contracts,
            schema: Some(schema),
            example_schemas,
        }
    }

    fn from_schemas(
        id: impl Into<String>,
        schemas: impl IntoIterator<Item = Value>,
        related_contracts: Vec<String>,
    ) -> Self {
        let mut identifiers = JsonContractIdentifiers::default();
        let mut example_schemas = BTreeMap::new();
        for schema in schemas {
            identifiers.extend(identifiers_from_json_schema(&schema));
            if let Some(title) = schema.get("title").and_then(Value::as_str) {
                let previous = example_schemas.insert(
                    JsonExampleShape::PublicSchemaObject(title.to_owned()).id(),
                    schema,
                );
                assert!(
                    previous.is_none(),
                    "a semantic contract must not expose duplicate named schema-object shapes"
                );
            }
        }
        Self {
            id: id.into(),
            identifiers,
            related_contracts,
            schema: None,
            example_schemas,
        }
    }

    fn identifiers_only(
        id: impl Into<String>,
        identifiers: JsonContractIdentifiers,
        related_contracts: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            identifiers,
            related_contracts,
            schema: None,
            example_schemas: BTreeMap::new(),
        }
    }

    /// Returns the stable semantic contract identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the exact owner-derived identifier catalog.
    pub const fn identifiers(&self) -> &JsonContractIdentifiers {
        &self.identifiers
    }

    /// Returns deliberate semantic relationships to adjacent contracts.
    pub fn related_contracts(&self) -> &[String] {
        &self.related_contracts
    }

    /// Returns the exact generated schema when this descriptor has one root.
    pub const fn schema(&self) -> Option<&Value> {
        self.schema.as_ref()
    }

    /// Returns every exact instance shape exposed by this descriptor.
    pub const fn example_schemas(&self) -> &BTreeMap<String, Value> {
        &self.example_schemas
    }

    /// Returns the exact JSON Schema for one supported example shape.
    pub fn example_schema(&self, shape: &str) -> Option<&Value> {
        self.example_schemas.get(shape)
    }

    /// Exposes one exact instance shape owned by another crate.
    pub fn with_example_schema(mut self, shape: JsonExampleShape, schema: Value) -> Self {
        let previous = self.example_schemas.insert(shape.id(), schema);
        assert!(
            previous.is_none(),
            "a semantic contract must expose each example shape exactly once"
        );
        self
    }
}

/// Returns the current public JSON contract descriptors.
pub fn public_json_contract_descriptors() -> Vec<JsonContractDescriptor> {
    let mut descriptors = Vec::new();
    let method_ids = MethodName::ALL
        .into_iter()
        .map(|method| {
            (
                method,
                method_contract_id(method, "request"),
                method_contract_id(method, "response"),
            )
        })
        .collect::<Vec<_>>();

    let method_names = MethodName::ALL
        .into_iter()
        .map(|method| method.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    descriptors.push(JsonContractDescriptor::identifiers_only(
        "api.methods",
        JsonContractIdentifiers {
            values: method_names,
            ..JsonContractIdentifiers::default()
        },
        method_ids
            .iter()
            .flat_map(|(_, request, response)| [request.clone(), response.clone()])
            .collect(),
    ));

    for (method, request_id, response_id) in &method_ids {
        let request = public_request_schema(method.as_str())
            .unwrap_or_else(|| panic!("public method {} has a request schema", method.as_str()));
        let response = public_response_schema(method.as_str())
            .unwrap_or_else(|| panic!("public method {} has a response schema", method.as_str()));
        let result = public_result_schema(method.as_str())
            .unwrap_or_else(|| panic!("public method {} has a result schema", method.as_str()));
        let mut request_descriptor = JsonContractDescriptor::from_schema(
            request_id,
            request.clone(),
            vec![response_id.clone()],
        );
        request_descriptor
            .example_schemas
            .insert(JsonExampleShape::MethodParams.id(), request.clone());
        request_descriptor.example_schemas.insert(
            JsonExampleShape::CompleteMethodRequest.id(),
            complete_method_request_schema(method.as_str(), request),
        );
        request_descriptor
            .identifiers
            .values
            .insert(method.as_str().to_owned());
        descriptors.push(request_descriptor);
        let mut response_descriptor = JsonContractDescriptor::from_schema(
            response_id,
            response.clone(),
            vec![request_id.clone()],
        );
        response_descriptor.example_schemas.insert(
            JsonExampleShape::CompleteMethodResponse.id(),
            result.clone(),
        );
        response_descriptor
            .example_schemas
            .insert(JsonExampleShape::MethodResultBody.id(), result);
        response_descriptor.example_schemas.insert(
            JsonExampleShape::MethodRejection.id(),
            schema::<ToolRejectedResponse>(),
        );
        response_descriptor
            .identifiers
            .values
            .insert(method.as_str().to_owned());
        descriptors.push(response_descriptor);
    }

    descriptors.extend(schema_family_descriptors());
    descriptors.extend(value_contract_descriptors());
    descriptors
}

fn complete_method_request_schema(method: &str, mut params: Value) -> Value {
    let definitions = params
        .as_object_mut()
        .and_then(|schema| schema.remove("definitions"));
    if let Some(schema) = params.as_object_mut() {
        schema.remove("$schema");
    }
    let mut complete = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "CompleteMethodRequest",
        "type": "object",
        "required": ["method", "params"],
        "additionalProperties": false,
        "properties": {
            "method": {"const": method, "type": "string"},
            "params": params
        }
    });
    if let Some(definitions) = definitions {
        complete
            .as_object_mut()
            .expect("complete request schema is an object")
            .insert("definitions".to_owned(), definitions);
    }
    complete
}

fn method_contract_id(method: MethodName, shape: &str) -> String {
    let method = method
        .as_str()
        .strip_prefix("volicord.")
        .expect("public method uses the volicord namespace");
    format!("api.method.{method}.{shape}")
}

fn schema_family_descriptors() -> Vec<JsonContractDescriptor> {
    vec![
        JsonContractDescriptor::from_schemas(
            "api.schema.core",
            [
                schema::<ToolEnvelope>(),
                schema::<ToolResultBase>(),
                schema::<ToolError>(),
                schema::<ToolRejectedResponse>(),
                schema::<ToolDryRunResponse>(),
                schema::<OperationResultRef>(),
                schema::<EventRef>(),
                schema::<GuaranteeDisclosure>(),
                schema::<DryRunSummary>(),
                schema::<PlannedEffect>(),
                schema::<PlannedBlocker>(),
                schema::<PlatformDiagnosticDetail>(),
            ],
            vec!["api.schema.state".to_owned()],
        ),
        JsonContractDescriptor::from_schemas(
            "api.schema.state",
            [
                schema::<StateRecordRef>(),
                schema::<SourceRef>(),
                schema::<GuardInstallation>(),
                schema::<AgentSession>(),
                schema::<GuardEvent>(),
                schema::<PromptCapture>(),
                schema::<UnrecordedChange>(),
                schema::<UnrecordedChangeFinding>(),
                schema::<UnrecordedChangeResolutionSummary>(),
                schema::<ProjectContinuityRecord>(),
                schema::<ProjectContinuitySummary>(),
                schema::<ContinuityCursor>(),
                schema::<ContinuityPageRequest>(),
                schema::<ContinuityPageInfo>(),
                schema::<ProjectContinuityPage>(),
                schema::<ProjectEnforcementProfile>(),
                schema::<ProjectWorkflowPolicySummary>(),
                schema::<StateSummary>(),
                schema::<TaskLineageInput>(),
                schema::<CarryForwardDisposition>(),
                schema::<TaskLineageSummary>(),
                schema::<TaskFlowItem>(),
                schema::<WorkspaceContext>(),
                schema::<AuthorityReceipt>(),
                schema::<ChangeUnitEffectContract>(),
                schema::<TaskLifecycleState>(),
                schema::<ShapingReadiness>(),
                schema::<ShapingGap>(),
                schema::<NextActionSummary>(),
                schema::<SummaryCard>(),
                schema::<WriteTicketStateSummary>(),
                schema::<WriteTicketPathPatterns>(),
                schema::<WriteTicketValidityBasis>(),
                schema::<WriteTicketScope>(),
                schema::<WriteTicket>(),
                schema::<WriteTicketAttemptScope>(),
                schema::<PathAssessment>(),
                schema::<MutationAssessment>(),
                schema::<WriteDecisionReason>(),
                schema::<AcceptanceCriterionInput>(),
                schema::<AcceptanceCriterionReplacement>(),
                schema::<AcceptanceCriterion>(),
                schema::<EvidenceTarget>(),
                schema::<EvidenceCaptureSpec>(),
                schema::<EvidenceCaptureIntent>(),
                schema::<EvidenceCaptureReceipt>(),
                schema::<EvidenceProducer>(),
                schema::<EvidenceSummary>(),
                schema::<EvidenceGateSummary>(),
                schema::<EvidenceCoverageItem>(),
                schema::<EvidenceCoverageUpdate>(),
                schema::<EvidenceUpdateProvenance>(),
                schema::<EvidenceObservation>(),
                schema::<EvidenceProducerAnchor>(),
                schema::<EvidenceRelevanceAssessment>(),
                schema::<UserActionEvidenceObservation>(),
                schema::<EvidenceObservationInput>(),
                schema::<RunSummary>(),
                schema::<ObservedChanges>(),
                schema::<CloseAssessmentInput>(),
                schema::<ResidualRiskInput>(),
                schema::<CurrentCloseBasis>(),
                schema::<SensitiveActionRequirement>(),
                schema::<ResidualRisk>(),
                schema::<RiskAcceptanceCoverage>(),
                schema::<CloseReadinessBlocker>(),
                schema::<ValidatorResult>(),
                schema::<GuaranteeDisplay>(),
            ],
            vec![
                "api.schema.core".to_owned(),
                "api.schema.artifact".to_owned(),
                "api.schema.user_action".to_owned(),
            ],
        ),
        JsonContractDescriptor::from_schemas(
            "api.schema.artifact",
            [
                schema::<ArtifactRef>(),
                schema::<StagedArtifactHandle>(),
                schema::<ArtifactInput>(),
            ],
            vec!["api.schema.state".to_owned()],
        ),
        JsonContractDescriptor::from_schemas(
            "api.schema.judgment",
            [
                schema::<AcceptedRiskInput>(),
                schema::<SensitiveActionScope>(),
                schema::<UserActionBasis>(),
            ],
            vec!["api.schema.user_action".to_owned()],
        ),
        JsonContractDescriptor::from_schemas(
            "api.schema.user_action",
            [
                schema::<UserActionDraft>(),
                schema::<UserActionChoiceDraft>(),
                schema::<UserActionEvidenceObservationDraft>(),
                schema::<UserActionChoiceRequestBody>(),
                schema::<UserActionEvidenceObservationRequestBody>(),
                schema::<UserActionRequestBody>(),
                schema::<UserActionRequest>(),
                schema::<UserActionResolution>(),
                schema::<UserActionResolutionInput>(),
                schema::<UserActionResolutionBody>(),
                schema::<UserActionResolutionForm>(),
                schema::<AgentSafeUserActionRequestSummary>(),
                schema::<UserActionBasisCoordinates>(),
                schema::<UserActionChoiceBasis>(),
                schema::<UserActionEvidenceObservationBasis>(),
                schema::<UserActionResolutionChoice>(),
                schema::<UserActionOptionInput>(),
                schema::<UserActionOption>(),
                schema::<UserActionContext>(),
            ],
            vec![
                "api.schema.state".to_owned(),
                "api.schema.artifact".to_owned(),
                "api.schema.judgment".to_owned(),
            ],
        ),
    ]
}

fn value_contract_descriptors() -> Vec<JsonContractDescriptor> {
    vec![JsonContractDescriptor::from_schema(
        "api.values.user_action_status",
        schema::<UserActionStatus>(),
        vec!["api.schema.user_action".to_owned()],
    )]
}

fn schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("public JSON schema should serialize")
}

/// Extracts exact properties, closed strings, and named schemas from JSON Schema.
pub fn identifiers_from_json_schema(schema: &Value) -> JsonContractIdentifiers {
    let mut identifiers = JsonContractIdentifiers::default();
    collect_json_schema_identifiers(schema, &mut identifiers);
    identifiers
}

fn collect_json_schema_identifiers(value: &Value, identifiers: &mut JsonContractIdentifiers) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                match key.as_str() {
                    "properties" => {
                        if let Some(entries) = value.as_object() {
                            identifiers.properties.extend(entries.keys().cloned());
                        }
                    }
                    "definitions" | "$defs" => {
                        if let Some(entries) = value.as_object() {
                            identifiers.schema_names.extend(entries.keys().cloned());
                        }
                    }
                    "enum" => identifiers.values.extend(
                        value
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .filter(|identifier| !identifier.is_empty())
                            .map(str::to_owned),
                    ),
                    "const" => {
                        if let Some(identifier) =
                            value.as_str().filter(|identifier| !identifier.is_empty())
                        {
                            identifiers.values.insert(identifier.to_owned());
                        }
                    }
                    "title" => {
                        if let Some(identifier) =
                            value.as_str().filter(|identifier| !identifier.is_empty())
                        {
                            identifiers.schema_names.insert(identifier.to_owned());
                        }
                    }
                    _ => {}
                }
                collect_json_schema_identifiers(value, identifiers);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_json_schema_identifiers(value, identifiers);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_descriptors_have_stable_semantic_ids_and_exact_schemas() {
        let descriptors = public_json_contract_descriptors();
        let intake = descriptors
            .iter()
            .find(|descriptor| descriptor.id() == "api.method.intake.request")
            .expect("intake request descriptor");

        assert!(intake.schema().is_some());
        assert!(intake.identifiers().properties().contains("requested_mode"));
        assert!(intake.identifiers().values().contains("direct"));
        assert_eq!(intake.related_contracts(), ["api.method.intake.response"]);
        assert!(intake.example_schema("params").is_some());
        assert!(intake.example_schema("complete_request").is_some());
        assert!(intake.example_schema("result_body").is_none());

        let response = descriptors
            .iter()
            .find(|descriptor| descriptor.id() == "api.method.intake.response")
            .expect("intake response descriptor");
        assert!(response.example_schema("complete_response").is_some());
        assert!(response.example_schema("result_body").is_some());
        assert!(response.example_schema("rejection").is_some());
        assert!(response.example_schema("params").is_none());
        assert_eq!(
            response.example_schema("complete_response"),
            response.example_schema("result_body")
        );
        assert_ne!(
            response.schema(),
            response.example_schema("complete_response")
        );
    }

    #[test]
    fn method_contracts_do_not_share_unrelated_method_fields() {
        let descriptors = public_json_contract_descriptors();
        let intake = descriptors
            .iter()
            .find(|descriptor| descriptor.id() == "api.method.intake.request")
            .expect("intake request descriptor");
        let stage = descriptors
            .iter()
            .find(|descriptor| descriptor.id() == "api.method.stage_artifact.request")
            .expect("stage-artifact request descriptor");

        assert!(intake
            .identifiers()
            .properties()
            .contains("plain_language_request"));
        assert!(!stage
            .identifiers()
            .properties()
            .contains("plain_language_request"));
    }

    #[test]
    fn schema_family_descriptors_expose_each_named_object_as_an_exact_shape() {
        let descriptors = public_json_contract_descriptors();
        let core = descriptors
            .iter()
            .find(|descriptor| descriptor.id() == "api.schema.core")
            .expect("core schema descriptor");

        assert!(core.example_schema("schema_object.ToolEnvelope").is_some());
        assert!(core
            .example_schema("schema_object.ToolRejectedResponse")
            .is_some());
        assert!(core.example_schema("complete_request").is_none());
    }
}
