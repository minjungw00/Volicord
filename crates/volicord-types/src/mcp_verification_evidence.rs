//! Typed MCP preflight and active-verification evidence.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::UtcTimestamp;

/// Result of one bounded MCP verification observation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum McpEvidenceCheckStatus {
    Passed,
    Failed,
}

impl McpEvidenceCheckStatus {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

/// Active operation required to observe Store writeability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpActiveVerificationKind {
    ConnectionVerify,
}

/// Read-only preflight observation of writeability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpWriteabilityObservation {
    NotChecked { requires: McpActiveVerificationKind },
}

impl McpWriteabilityObservation {
    /// Creates the only supported preflight writeability observation.
    pub const fn requires_connection_verify() -> Self {
        Self::NotChecked {
            requires: McpActiveVerificationKind::ConnectionVerify,
        }
    }
}

/// One project-state read observed during read-only preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpProjectReadEvidence {
    project_id: String,
    state_read: McpEvidenceCheckStatus,
}

impl McpProjectReadEvidence {
    pub fn new(project_id: impl Into<String>, state_read: McpEvidenceCheckStatus) -> Self {
        Self {
            project_id: project_id.into(),
            state_read,
        }
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub const fn state_read(&self) -> McpEvidenceCheckStatus {
        self.state_read
    }
}

/// Immutable evidence produced by one read-only MCP preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct McpPreflightEvidence {
    configuration: McpEvidenceCheckStatus,
    registry_read: McpEvidenceCheckStatus,
    project_reads: Vec<McpProjectReadEvidence>,
    schema_validation: McpEvidenceCheckStatus,
    protocol_profiles: McpEvidenceCheckStatus,
    host_contracts: McpEvidenceCheckStatus,
    writeability: McpWriteabilityObservation,
    #[schemars(length(max = 0))]
    side_effects: Vec<McpSideEffectKind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpPreflightEvidenceWire {
    configuration: McpEvidenceCheckStatus,
    registry_read: McpEvidenceCheckStatus,
    project_reads: Vec<McpProjectReadEvidence>,
    schema_validation: McpEvidenceCheckStatus,
    protocol_profiles: McpEvidenceCheckStatus,
    host_contracts: McpEvidenceCheckStatus,
    writeability: McpWriteabilityObservation,
    side_effects: Vec<McpSideEffectKind>,
}

impl<'de> Deserialize<'de> for McpPreflightEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = McpPreflightEvidenceWire::deserialize(deserializer)?;
        if wire.writeability != McpWriteabilityObservation::requires_connection_verify() {
            return Err(serde::de::Error::custom(
                "MCP preflight writeability must require connection_verify",
            ));
        }
        if !wire.side_effects.is_empty() {
            return Err(serde::de::Error::custom(
                "MCP preflight evidence cannot contain side effects",
            ));
        }
        Ok(Self::new(
            wire.configuration,
            wire.registry_read,
            wire.project_reads,
            wire.schema_validation,
            wire.protocol_profiles,
            wire.host_contracts,
        ))
    }
}

impl McpPreflightEvidence {
    /// Constructs immutable preflight evidence with fixed read-only facts.
    pub fn new(
        configuration: McpEvidenceCheckStatus,
        registry_read: McpEvidenceCheckStatus,
        project_reads: Vec<McpProjectReadEvidence>,
        schema_validation: McpEvidenceCheckStatus,
        protocol_profiles: McpEvidenceCheckStatus,
        host_contracts: McpEvidenceCheckStatus,
    ) -> Self {
        Self {
            configuration,
            registry_read,
            project_reads,
            schema_validation,
            protocol_profiles,
            host_contracts,
            writeability: McpWriteabilityObservation::requires_connection_verify(),
            side_effects: Vec::new(),
        }
    }

    pub const fn writeability(&self) -> McpWriteabilityObservation {
        self.writeability
    }

    pub fn side_effects(&self) -> &[McpSideEffectKind] {
        &self.side_effects
    }
}

/// One project-database writeability result from active verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpProjectWriteEvidence {
    project_id: String,
    state_write: McpEvidenceCheckStatus,
}

impl McpProjectWriteEvidence {
    pub fn new(project_id: impl Into<String>, state_write: McpEvidenceCheckStatus) -> Self {
        Self {
            project_id: project_id.into(),
            state_write,
        }
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub const fn state_write(&self) -> McpEvidenceCheckStatus {
        self.state_write
    }
}

/// One executable MCP probe result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpProbeEvidence {
    status: McpEvidenceCheckStatus,
    requested_revision: Option<String>,
    negotiated_revision: Option<String>,
    initialize: bool,
    initialized_notification: bool,
    schema_validation: bool,
    tools_list_observed: bool,
    tools_returned: Option<usize>,
    required_tools_validated: bool,
    safe_read_only_tool: String,
    safe_read_only_tool_completed: bool,
    shutdown_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finding_id: Option<String>,
}

impl McpProbeEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status: McpEvidenceCheckStatus,
        requested_revision: Option<String>,
        negotiated_revision: Option<String>,
        initialize: bool,
        initialized_notification: bool,
        schema_validation: bool,
        tools_list_observed: bool,
        tools_returned: Option<usize>,
        required_tools_validated: bool,
        safe_read_only_tool: impl Into<String>,
        safe_read_only_tool_completed: bool,
        shutdown_completed: bool,
        diagnostic_code: Option<String>,
        failure_stage: Option<String>,
        finding_id: Option<String>,
    ) -> Self {
        Self {
            status,
            requested_revision,
            negotiated_revision,
            initialize,
            initialized_notification,
            schema_validation,
            tools_list_observed,
            tools_returned,
            required_tools_validated,
            safe_read_only_tool: safe_read_only_tool.into(),
            safe_read_only_tool_completed,
            shutdown_completed,
            diagnostic_code,
            failure_stage,
            finding_id,
        }
    }
}

/// One production-revision conformance result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRevisionConformance {
    revision: String,
    #[serde(flatten)]
    probe: McpProbeEvidence,
}

impl McpRevisionConformance {
    pub fn new(revision: impl Into<String>, probe: McpProbeEvidence) -> Self {
        Self {
            revision: revision.into(),
            probe,
        }
    }
}

/// One independently pinned host-compatibility result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpHostCompatibilityEvidence {
    profile: String,
    fixture: String,
    #[serde(flatten)]
    probe: McpProbeEvidence,
}

impl McpHostCompatibilityEvidence {
    pub fn new(
        profile: impl Into<String>,
        fixture: impl Into<String>,
        probe: McpProbeEvidence,
    ) -> Self {
        Self {
            profile: profile.into(),
            fixture: fixture.into(),
            probe,
        }
    }
}

/// Source of active MCP verification evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpActiveVerificationSource {
    ConnectionVerify,
}

/// Closed side effects that can produce active MCP verification evidence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum McpSideEffectKind {
    RollbackOnlyRegistryWriteProbe,
    RollbackOnlyProjectWriteProbe,
    DisposableProtocolConformance,
    DisposableHostCompatibility,
}

/// Evidence produced by one active MCP verification operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpActiveVerificationEvidence {
    registry_write: McpEvidenceCheckStatus,
    project_writes: Vec<McpProjectWriteEvidence>,
    protocol_conformance: Vec<McpRevisionConformance>,
    host_compatibility: Vec<McpHostCompatibilityEvidence>,
    observed_at: UtcTimestamp,
    source: McpActiveVerificationSource,
    side_effects: Vec<McpSideEffectKind>,
}

impl McpActiveVerificationEvidence {
    pub fn new(
        registry_write: McpEvidenceCheckStatus,
        project_writes: Vec<McpProjectWriteEvidence>,
        protocol_conformance: Vec<McpRevisionConformance>,
        host_compatibility: Vec<McpHostCompatibilityEvidence>,
        observed_at: UtcTimestamp,
        mut side_effects: Vec<McpSideEffectKind>,
    ) -> Self {
        side_effects.sort();
        side_effects.dedup();
        Self {
            registry_write,
            project_writes,
            protocol_conformance,
            host_compatibility,
            observed_at,
            source: McpActiveVerificationSource::ConnectionVerify,
            side_effects,
        }
    }

    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    pub const fn source(&self) -> McpActiveVerificationSource {
        self.source
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn preflight_evidence_has_only_not_checked_writeability_and_no_side_effects() {
        let evidence = McpPreflightEvidence::new(
            McpEvidenceCheckStatus::Passed,
            McpEvidenceCheckStatus::Passed,
            vec![McpProjectReadEvidence::new(
                "project_1",
                McpEvidenceCheckStatus::Passed,
            )],
            McpEvidenceCheckStatus::Passed,
            McpEvidenceCheckStatus::Passed,
            McpEvidenceCheckStatus::Passed,
        );

        assert_eq!(
            serde_json::to_value(&evidence).expect("preflight JSON"),
            json!({
                "configuration": "passed",
                "registry_read": "passed",
                "project_reads": [{
                    "project_id": "project_1",
                    "state_read": "passed"
                }],
                "schema_validation": "passed",
                "protocol_profiles": "passed",
                "host_contracts": "passed",
                "writeability": {
                    "status": "not_checked",
                    "requires": "connection_verify"
                },
                "side_effects": []
            })
        );
    }

    #[test]
    fn preflight_evidence_decoder_rejects_side_effects() {
        let invalid = json!({
            "configuration": "passed",
            "registry_read": "passed",
            "project_reads": [],
            "schema_validation": "passed",
            "protocol_profiles": "passed",
            "host_contracts": "passed",
            "writeability": {
                "status": "not_checked",
                "requires": "connection_verify"
            },
            "side_effects": ["rollback_only_registry_write_probe"]
        });
        assert!(serde_json::from_value::<McpPreflightEvidence>(invalid).is_err());
    }

    #[test]
    fn active_evidence_has_its_own_timestamp_source_and_side_effects() {
        let evidence = McpActiveVerificationEvidence::new(
            McpEvidenceCheckStatus::Passed,
            vec![McpProjectWriteEvidence::new(
                "project_1",
                McpEvidenceCheckStatus::Passed,
            )],
            Vec::new(),
            Vec::new(),
            UtcTimestamp::parse("2026-07-25T01:02:03Z").expect("timestamp"),
            vec![
                McpSideEffectKind::RollbackOnlyProjectWriteProbe,
                McpSideEffectKind::RollbackOnlyRegistryWriteProbe,
            ],
        );
        let value = serde_json::to_value(evidence).expect("active JSON");
        assert_eq!(value["observed_at"], "2026-07-25T01:02:03Z");
        assert_eq!(value["source"], "connection_verify");
        assert_eq!(
            value["side_effects"],
            json!([
                "rollback_only_registry_write_probe",
                "rollback_only_project_write_probe"
            ])
        );
        assert!(value.get("preflight").is_none());
    }
}
