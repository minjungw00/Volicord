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
    pub next_probe_tool: String,
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
