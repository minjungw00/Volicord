//! Public connection-integration verification request and result shapes.

use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
    JsonSchema,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{AgentToolId, GuardEventId, GuardIntegrationVerificationId, UtcTimestamp};

/// Maximum lifetime of one active in-chat Guard integration verification.
pub const GUARD_INTEGRATION_VERIFICATION_TTL_SECONDS: i64 = 300;

/// Closed durable lifecycle for one Guard integration-verification run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardIntegrationVerificationStatus {
    Active,
    Passed,
    Failed,
    Expired,
}

/// Closed observation state for one correlated verification phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardIntegrationVerificationPhaseStatus {
    Pending,
    Matched,
}

/// Closed acquisition stage for one bounded Guard-probe observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardProbeObservationStage {
    ProbeAcknowledged,
    HookEventNotObserved,
    HookPayloadIncompatible,
    CallableIdentityUnknown,
    CallableIdentityMismatch,
    VerificationIdMismatch,
    SessionMismatch,
    TurnMismatch,
    ToolUseMismatch,
    PreToolMatched,
    PostToolMatched,
}

impl GuardProbeObservationStage {
    pub const ALL: [Self; 11] = [
        Self::ProbeAcknowledged,
        Self::HookEventNotObserved,
        Self::HookPayloadIncompatible,
        Self::CallableIdentityUnknown,
        Self::CallableIdentityMismatch,
        Self::VerificationIdMismatch,
        Self::SessionMismatch,
        Self::TurnMismatch,
        Self::ToolUseMismatch,
        Self::PreToolMatched,
        Self::PostToolMatched,
    ];

    /// Returns the exact storage and diagnostic spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProbeAcknowledged => "probe_acknowledged",
            Self::HookEventNotObserved => "hook_event_not_observed",
            Self::HookPayloadIncompatible => "hook_payload_incompatible",
            Self::CallableIdentityUnknown => "callable_identity_unknown",
            Self::CallableIdentityMismatch => "callable_identity_mismatch",
            Self::VerificationIdMismatch => "verification_id_mismatch",
            Self::SessionMismatch => "session_mismatch",
            Self::TurnMismatch => "turn_mismatch",
            Self::ToolUseMismatch => "tool_use_mismatch",
            Self::PreToolMatched => "pre_tool_matched",
            Self::PostToolMatched => "post_tool_matched",
        }
    }

    /// Parses the exact storage and diagnostic spelling.
    pub fn from_storage_str(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.as_str() == value)
    }
}

/// Bounded terminal or current verification finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardIntegrationVerificationFinding {
    pub code: String,
    pub summary: String,
}

macro_rules! fixed_agent_tool_reference {
    ($name:ident, $tool:expr) => {
        #[doc = concat!("Exact public reference to `", stringify!($tool), "`.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl $name {
            /// Creates the only valid value of this fixed tool reference.
            pub const fn new() -> Self {
                Self
            }

            /// Returns the canonical Agent Connection tool identity.
            pub const fn tool_id(self) -> AgentToolId {
                $tool
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                $tool.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let tool = AgentToolId::deserialize(deserializer)?;
                if tool == $tool {
                    Ok(Self)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected canonical integration-verification tool {}",
                        $tool.wire_name()
                    )))
                }
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_owned()
            }

            fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
                Schema::Object(SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                    enum_values: Some(vec![serde_json::Value::String(
                        $tool.wire_name().to_owned(),
                    )]),
                    ..Default::default()
                })
            }
        }
    };
}

fixed_agent_tool_reference!(GuardProbeToolReference, AgentToolId::GUARD_PROBE);
fixed_agent_tool_reference!(
    IntegrationVerificationStatusToolReference,
    AgentToolId::GET_INTEGRATION_VERIFICATION
);
fixed_agent_tool_reference!(
    BeginIntegrationVerificationToolReference,
    AgentToolId::BEGIN_INTEGRATION_VERIFICATION
);

/// Closed reason that requires a new bounded verification run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationVerificationRestartReason {
    Failed,
    Expired,
}

/// One authoritative, state-directed integration-verification workflow state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegrationVerificationWorkflowState {
    AwaitingProbe {
        tool: GuardProbeToolReference,
        expires_at: UtcTimestamp,
    },
    AwaitingHookCompletion {
        tool: IntegrationVerificationStatusToolReference,
        acknowledged_at: UtcTimestamp,
        expires_at: UtcTimestamp,
    },
    Complete {
        completed_at: UtcTimestamp,
    },
    RestartRequired {
        reason: IntegrationVerificationRestartReason,
        tool: BeginIntegrationVerificationToolReference,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finding: Option<GuardIntegrationVerificationFinding>,
    },
}

impl IntegrationVerificationWorkflowState {
    pub const AWAITING_PROBE_KIND: &'static str = "awaiting_probe";
    pub const AWAITING_HOOK_COMPLETION_KIND: &'static str = "awaiting_hook_completion";
    pub const COMPLETE_KIND: &'static str = "complete";
    pub const RESTART_REQUIRED_KIND: &'static str = "restart_required";

    /// Returns the exact next tool owned by this state, when one exists.
    pub const fn directed_tool(&self) -> Option<AgentToolId> {
        match self {
            Self::AwaitingProbe { tool, .. } => Some(tool.tool_id()),
            Self::AwaitingHookCompletion { tool, .. } => Some(tool.tool_id()),
            Self::Complete { .. } => None,
            Self::RestartRequired { tool, .. } => Some(tool.tool_id()),
        }
    }

    /// Returns the stable serialized tag for this state.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::AwaitingProbe { .. } => Self::AWAITING_PROBE_KIND,
            Self::AwaitingHookCompletion { .. } => Self::AWAITING_HOOK_COMPLETION_KIND,
            Self::Complete { .. } => Self::COMPLETE_KIND,
            Self::RestartRequired { .. } => Self::RESTART_REQUIRED_KIND,
        }
    }
}

/// Arguments for `volicord.begin_integration_verification`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BeginIntegrationVerificationArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
}

/// Arguments shared by the probe and result lookup tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntegrationVerificationIdArguments {
    pub verification_id: GuardIntegrationVerificationId,
}

/// Result of beginning or resuming an in-chat verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BeginIntegrationVerificationResult {
    pub verification_id: GuardIntegrationVerificationId,
    pub workflow: IntegrationVerificationWorkflowState,
    pub matched_prompt_event_id: GuardEventId,
}

/// Result of the exact MCP probe call observed by Guard tool hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardProbeResult {
    pub verification_id: GuardIntegrationVerificationId,
    pub workflow: IntegrationVerificationWorkflowState,
}

/// Correlated phase projection returned by verification lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardIntegrationVerificationPhases {
    pub prompt_capture: GuardIntegrationVerificationPhaseStatus,
    pub pre_tool: GuardIntegrationVerificationPhaseStatus,
    pub post_tool: GuardIntegrationVerificationPhaseStatus,
}

/// Current correlated result for one in-chat integration verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetIntegrationVerificationResult {
    pub verification_id: GuardIntegrationVerificationId,
    pub workflow: IntegrationVerificationWorkflowState,
    pub guard_phases: GuardIntegrationVerificationPhases,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_prompt_event_id: Option<GuardEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_pre_tool_event_id: Option<GuardEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_post_tool_event_id: Option<GuardEventId>,
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;
    use serde_json::json;

    use super::*;
    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::parse(value).expect("timestamp")
    }

    #[test]
    fn every_workflow_variant_serializes_with_its_exact_state_directed_tool() {
        let awaiting_probe = IntegrationVerificationWorkflowState::AwaitingProbe {
            tool: GuardProbeToolReference::new(),
            expires_at: timestamp("2026-07-23T00:05:00Z"),
        };
        assert_eq!(
            serde_json::to_value(&awaiting_probe).expect("awaiting-probe state"),
            json!({
                "kind": "awaiting_probe",
                "tool": AgentToolId::GUARD_PROBE.wire_name(),
                "expires_at": "2026-07-23T00:05:00Z",
            })
        );
        let awaiting_hooks = IntegrationVerificationWorkflowState::AwaitingHookCompletion {
            tool: IntegrationVerificationStatusToolReference::new(),
            acknowledged_at: timestamp("2026-07-23T00:00:04Z"),
            expires_at: timestamp("2026-07-23T00:05:00Z"),
        };
        assert_eq!(
            serde_json::to_value(&awaiting_hooks).expect("awaiting-hook state"),
            json!({
                "kind": "awaiting_hook_completion",
                "tool": AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
                "acknowledged_at": "2026-07-23T00:00:04Z",
                "expires_at": "2026-07-23T00:05:00Z",
            })
        );
        let complete = IntegrationVerificationWorkflowState::Complete {
            completed_at: timestamp("2026-07-23T00:00:05Z"),
        };
        assert_eq!(
            serde_json::to_value(&complete).expect("complete state"),
            json!({
                "kind": "complete",
                "completed_at": "2026-07-23T00:00:05Z",
            })
        );
        for reason in [
            IntegrationVerificationRestartReason::Failed,
            IntegrationVerificationRestartReason::Expired,
        ] {
            let restart = IntegrationVerificationWorkflowState::RestartRequired {
                reason,
                tool: BeginIntegrationVerificationToolReference::new(),
                finding: Some(GuardIntegrationVerificationFinding {
                    code: "verification_restart_required".to_owned(),
                    summary: "Begin a new bounded verification.".to_owned(),
                }),
            };
            let value = serde_json::to_value(&restart).expect("restart-required state");
            assert_eq!(value["kind"], "restart_required");
            assert_eq!(
                value["tool"],
                AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name()
            );
            assert_eq!(
                restart.directed_tool(),
                Some(AgentToolId::BEGIN_INTEGRATION_VERIFICATION)
            );
        }
        assert_eq!(
            awaiting_probe.directed_tool(),
            Some(AgentToolId::GUARD_PROBE)
        );
        assert_eq!(
            awaiting_hooks.directed_tool(),
            Some(AgentToolId::GET_INTEGRATION_VERIFICATION)
        );
        assert_eq!(complete.directed_tool(), None);
    }

    #[test]
    fn public_results_require_one_shared_tagged_workflow_state() {
        let schemas = [
            (
                "begin",
                serde_json::to_value(schema_for!(BeginIntegrationVerificationResult))
                    .expect("begin schema"),
                &["verification_id", "workflow", "matched_prompt_event_id"][..],
            ),
            (
                "probe",
                serde_json::to_value(schema_for!(GuardProbeResult)).expect("probe schema"),
                &["verification_id", "workflow"][..],
            ),
            (
                "get",
                serde_json::to_value(schema_for!(GetIntegrationVerificationResult))
                    .expect("get schema"),
                &["verification_id", "workflow", "guard_phases"][..],
            ),
        ];
        for (result_name, schema, required_fields) in schemas {
            let required = schema["required"].as_array().expect("required fields");
            for field in required_fields {
                assert!(
                    required.contains(&json!(field)),
                    "{result_name} is missing required {field}"
                );
            }
            assert_eq!(
                schema["properties"]["workflow"]["$ref"],
                "#/definitions/IntegrationVerificationWorkflowState"
            );
            for removed in [
                "status",
                "mcp_probe_acknowledged",
                "next_probe_tool",
                "next_action",
                "acknowledged_at",
                "completed_at",
                "finding",
            ] {
                assert!(
                    schema["properties"].get(removed).is_none(),
                    "{result_name} retained independent field {removed}"
                );
            }
        }
    }

    #[test]
    fn fixed_tool_references_reject_every_other_canonical_tool() {
        for (value, expected) in [
            (
                json!(AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name()),
                AgentToolId::GUARD_PROBE,
            ),
            (
                json!(AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name()),
                AgentToolId::GET_INTEGRATION_VERIFICATION,
            ),
        ] {
            if expected == AgentToolId::GUARD_PROBE {
                assert!(serde_json::from_value::<GuardProbeToolReference>(value).is_err());
            } else {
                assert!(
                    serde_json::from_value::<IntegrationVerificationStatusToolReference>(value)
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn guard_probe_observation_stages_round_trip_exactly() {
        for stage in GuardProbeObservationStage::ALL {
            assert_eq!(
                serde_json::to_value(stage).expect("stage serializes"),
                json!(stage.as_str())
            );
            assert_eq!(
                GuardProbeObservationStage::from_storage_str(stage.as_str()),
                Some(stage)
            );
        }
        assert_eq!(
            GuardProbeObservationStage::from_storage_str("matcher_failed"),
            None
        );
    }
}
