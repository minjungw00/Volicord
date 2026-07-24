//! Durable, session-coherent Guard integration verification.

mod begin;
mod coordinate;
mod correlation;
mod observation;
mod probe;
mod row;
mod status;

pub use begin::{
    begin_guard_integration_verification, begin_guard_integration_verification_with_generator,
};
pub use correlation::refresh_guard_integration_verification_for_event;
pub use observation::{
    guard_probe_observations, observe_guard_probe_hook_event,
    observe_unbound_guard_probe_hook_event, GuardProbeHookEvidence, GuardProbeObservationRecord,
    UnboundGuardProbeHookObservation,
};
pub use probe::acknowledge_guard_integration_probe;
pub use status::{
    current_guard_integration_verification_workflow, get_guard_integration_verification,
    latest_guard_integration_verification_for_connection,
};

/// Exact managed caller coordinate supplied by the MCP session boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardIntegrationVerificationCaller {
    pub connection_internal_id: String,
    pub runtime_session_id: String,
    pub host_session_id: String,
    pub host_turn_id: String,
}

/// Input used to create or resume one verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeginGuardIntegrationVerificationInput {
    pub caller: GuardIntegrationVerificationCaller,
    pub project_id: String,
    pub project_session_id: String,
    pub observed_at: String,
}

/// Registry-owned durable verification row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardIntegrationVerificationRunRecord {
    pub verification_id: String,
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub runtime_session_id: String,
    pub host_session_id: String,
    pub host_turn_id: String,
    pub guard_installation_id: String,
    pub integration_revision: String,
    pub policy_hash: String,
    pub hook_contract_digest: String,
    pub expected_probe_tool: String,
    pub expected_host_callable_name: String,
    pub created_at: String,
    pub expires_at: String,
    pub status: String,
    pub probe_acknowledged_at: Option<String>,
    pub completed_at: Option<String>,
    pub matched_prompt_event_id: Option<String>,
    pub matched_pre_tool_event_id: Option<String>,
    pub matched_post_tool_event_id: Option<String>,
    pub terminal_finding_code: Option<String>,
    pub terminal_finding_summary: Option<String>,
}

#[cfg(test)]
mod tests;
