//! Public connection-integration verification request and result shapes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{GuardEventId, GuardIntegrationVerificationId, UtcTimestamp};

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

/// State-directed operation returned when beginning or resuming verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationVerificationNextAction {
    CallGuardProbe,
    ReadVerificationStatus,
    NoFurtherAction,
}

/// Bounded terminal or current verification finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardIntegrationVerificationFinding {
    pub code: String,
    pub summary: String,
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
    pub status: GuardIntegrationVerificationStatus,
    pub expires_at: UtcTimestamp,
    pub mcp_probe_acknowledged: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_probe_tool: Option<String>,
    pub next_action: IntegrationVerificationNextAction,
    pub matched_prompt_event_id: GuardEventId,
}

/// Result of the exact MCP probe call observed by Guard tool hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardProbeResult {
    pub verification_id: GuardIntegrationVerificationId,
    pub status: GuardIntegrationVerificationStatus,
    pub acknowledged_at: UtcTimestamp,
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
    pub status: GuardIntegrationVerificationStatus,
    pub mcp_probe_acknowledged: bool,
    pub guard_phases: GuardIntegrationVerificationPhases,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_prompt_event_id: Option<GuardEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_pre_tool_event_id: Option<GuardEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_post_tool_event_id: Option<GuardEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<UtcTimestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<GuardIntegrationVerificationFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;
    use serde_json::json;

    use super::*;
    use crate::AgentToolId;

    fn begin_result(
        status: GuardIntegrationVerificationStatus,
        acknowledged: bool,
        next_probe_tool: Option<String>,
        next_action: IntegrationVerificationNextAction,
    ) -> BeginIntegrationVerificationResult {
        BeginIntegrationVerificationResult {
            verification_id: GuardIntegrationVerificationId::new("guard_verification_test"),
            status,
            expires_at: UtcTimestamp::parse("2026-07-23T00:05:00Z").expect("timestamp"),
            mcp_probe_acknowledged: acknowledged,
            next_probe_tool,
            next_action,
            matched_prompt_event_id: GuardEventId::new("guard_event_prompt"),
        }
    }

    #[test]
    fn begin_result_projects_probe_requirement_and_state_directed_action() {
        let active = serde_json::to_value(begin_result(
            GuardIntegrationVerificationStatus::Active,
            false,
            Some(AgentToolId::GUARD_PROBE.wire_name().to_owned()),
            IntegrationVerificationNextAction::CallGuardProbe,
        ))
        .expect("active begin result");
        assert_eq!(
            active,
            json!({
                "verification_id": "guard_verification_test",
                "status": "active",
                "expires_at": "2026-07-23T00:05:00Z",
                "mcp_probe_acknowledged": false,
                "next_probe_tool": AgentToolId::GUARD_PROBE.wire_name(),
                "next_action": "call_guard_probe",
                "matched_prompt_event_id": "guard_event_prompt",
            })
        );

        let passed = serde_json::to_value(begin_result(
            GuardIntegrationVerificationStatus::Passed,
            true,
            None,
            IntegrationVerificationNextAction::NoFurtherAction,
        ))
        .expect("passed begin result");
        assert!(passed.get("next_probe_tool").is_none());
        assert_eq!(passed["mcp_probe_acknowledged"], true);
        assert_eq!(passed["next_action"], "no_further_action");
    }

    #[test]
    fn begin_schema_requires_state_and_allows_omitted_probe_tool() {
        let schema = serde_json::to_value(schema_for!(BeginIntegrationVerificationResult))
            .expect("begin schema");
        let required = schema["required"].as_array().expect("required fields");
        for field in ["status", "mcp_probe_acknowledged", "next_action"] {
            assert!(required.contains(&json!(field)), "missing required {field}");
        }
        assert!(!required.contains(&json!("next_probe_tool")));
        assert!(schema["properties"].get("next_probe_tool").is_some());
    }
}
