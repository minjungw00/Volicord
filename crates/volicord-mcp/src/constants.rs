use std::sync::atomic::AtomicU64;

use volicord_types::integration_verification::IntegrationVerificationWorkflowState;
use volicord_types::tool_names::AgentToolId;

pub(crate) const SERVER_NAME: &str = "volicord-mcp";
pub(crate) const DEFAULT_LOCALE: &str = "en-US";
pub(crate) static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn server_instructions() -> String {
    format!(
        "Volicord records task scope, write tickets, evidence, runs, user-action requests, evidence attachments, and Close Status for explicitly registered Product Repositories. Preserve the user's requested outcome when choosing Task scope: when analysis or shaping is a step toward an implementation outcome, keep one work Task and record that step as a shaping_update; use advisor only when the requested outcome itself is read-only advice. If the broader outcome is unclear, keep the known boundary in shaping state or ask the user instead of expanding it. If project selection is unclear, call {} and use one listed project_selector; do not guess from folders, roots, labels, or memory. For the canonical request `Run the Volicord integration verification.`, call {} and then {}. Follow the returned `workflow` tagged state: `{}` calls its exact `{}` tool once; `{}` calls its exact `{}` status tool once; `{}` and `{}` are terminal and call no verification tool. Do not use shell sleep or poll loops, make repeated status calls, or automatically restart the workflow in the same turn. Begin, probe, and status expose this same state contract. Only that first-party state-directed workflow proves current managed MCP and Guard correlation. If Volicord tools are not exposed, report the managed MCP connection as unavailable; do not substitute raw stdio, hand-author Codex `_meta`, or treat resources/list or resource templates as proof of tool availability. `volicord connection verify` is optional active diagnostics only and does not replace the managed-host workflow. Read-only connection status and CLI MCP preflight are diagnostic only and are not managed-host evidence. Hook trust remains user/host owned. Mutation tools default to a fresh compact authority receipt plus the method outcome needed for the next step; request detail=workflow for current next actions or detail=full only when the bounded full method result is needed. When a mutation returns a non-null operation_result_ref, use {} for omitted exact historical bytes and {} separately for current authority; never retry an applied mutation. Volicord state management is separate from product-file edit authority: product-file edits still require the host/user path and any required write ticket. A write ticket records intended product-file changes; it is not OS permission, review bypass, access control, or a promise of automatic tool use.",
        AgentToolId::LIST_PROJECTS.wire_name(),
        AgentToolId::LIST_PROJECTS.wire_name(),
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
        IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND,
        AgentToolId::GUARD_PROBE.wire_name(),
        IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND,
        AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        IntegrationVerificationWorkflowState::REPAIR_REQUIRED_KIND,
        IntegrationVerificationWorkflowState::COMPLETE_KIND,
        AgentToolId::GET_OPERATION_RESULT.wire_name(),
        AgentToolId::STATUS.wire_name(),
    )
}
pub(crate) const TRANSPORT_DISCLOSURE_TEXT: &str = "Does not prove: public API availability, authentication service status, security boundary, OS sandboxing, network isolation, write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_instructions_preserve_outcome_across_shaping_and_implementation() {
        let instructions = server_instructions();
        assert!(instructions.contains("keep one work Task"));
        assert!(instructions.contains("record that step as a shaping_update"));
        assert!(instructions.contains("use advisor only"));
        assert!(instructions.contains("instead of expanding it"));
        assert!(instructions.contains("fresh compact authority receipt"));
        assert!(instructions.contains(AgentToolId::GET_OPERATION_RESULT.wire_name()));
        assert!(instructions.contains("never retry an applied mutation"));
    }

    #[test]
    fn server_instructions_define_the_only_managed_integration_proof_sequence() {
        let instructions = server_instructions();
        assert!(instructions.contains("Run the Volicord integration verification."));
        let sequence = [
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        ];
        let mut prior = 0;
        for tool in sequence {
            let offset = instructions[prior..]
                .find(tool)
                .expect("canonical verification tool should be named in order");
            prior += offset + tool.len();
        }
        for unavailable_boundary in [
            "managed MCP connection as unavailable",
            "raw stdio",
            "Codex `_meta`",
            "resources/list",
            "resource templates",
            "diagnostic only",
            "Hook trust remains user/host owned",
        ] {
            assert!(instructions.contains(unavailable_boundary));
        }
        for kind in [
            IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND,
            IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND,
            IntegrationVerificationWorkflowState::REPAIR_REQUIRED_KIND,
            IntegrationVerificationWorkflowState::COMPLETE_KIND,
        ] {
            assert!(instructions.contains(kind));
        }
        assert!(instructions.contains("same state contract"));
        assert!(instructions.contains("state-directed workflow"));
        assert!(instructions.contains("Do not use shell sleep or poll loops"));
        assert!(instructions.contains("automatically restart the workflow in the same turn"));
        assert!(instructions
            .contains("`volicord connection verify` is optional active diagnostics only"));
    }
}
