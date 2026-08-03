use std::sync::atomic::AtomicU64;

use volicord_types::integration_verification::IntegrationVerificationWorkflowState;
use volicord_types::tool_names::AgentToolId;

pub(crate) const SERVER_NAME: &str = "volicord-mcp";
pub(crate) const DEFAULT_LOCALE: &str = "en-US";
pub(crate) static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn server_instructions() -> String {
    format!(
        "Volicord records task scope, shaping checkpoints, write tickets, evidence, runs, user-action requests, evidence attachments, and Close Status for explicitly registered Product Repositories. Preserve the user's requested outcome when choosing Task scope: when shaping is a step toward implementation, keep one work Task, call volicord.record_shaping, create or update its Change Unit without changing phase, and call volicord.advance_task only when the returned workflow requires it; use advisor only when the requested outcome itself is read-only advice. If the broader outcome is unclear, keep the known boundary in shaping state or ask the user instead of expanding it. If project selection is unclear, call {} and use one listed project_selector; do not guess from folders, roots, labels, or memory. For the canonical request `Run the Volicord integration verification.`, call {} and then {}. Follow the returned `workflow` tagged state: `{}` calls its exact `{}` tool once; `{}` calls its exact `{}` status tool once; `{}` and `{}` are terminal and call no verification tool. Do not use shell sleep or poll loops, make repeated status calls, or automatically restart the workflow in the same turn. Begin, probe, and status expose this same state contract. Only that first-party state-directed workflow proves current managed MCP and Guard correlation. If Volicord tools are not exposed, report the managed MCP connection as unavailable; do not substitute raw stdio, hand-author Codex `_meta`, or treat resources/list or resource templates as proof of tool availability. `volicord connection verify` is optional active diagnostics only and does not replace the managed-host workflow. Read-only connection status and CLI MCP preflight are diagnostic only and are not managed-host evidence. Hook trust remains user/host owned. Treat the current shaping checkpoint and linked UserAction authority as authoritative. Follow the tagged workflow's required_action; do not call workflow tools speculatively or select progression from top-level array order. Never replace the current checkpoint to remove a pending or accepted-but-unapplied decision. Inspect the exact User Channel resolution outcome. Resolution does not apply a shaping decision: apply only accepted, current, compatible authority through its application_owner with the exact current resolution refs. After rejection, deferral, or expiration, follow decision_recovery_required and revise shaping. Never retry resolution of a terminal or expired request. If the revised plan still needs that judgment, create a successor UserActionRequest with an independent identity; chat text cannot replace it. A rejected, deferred, or expired decision grants no authority and keeps Product Repository mutation unavailable; surface that outcome and do not hide it as success. Do not invent a scope decision or pass a scope-decision ref for product-only or technical-only work. Change Unit creation does not advance phase. For work, call volicord.advance_task only when the tagged workflow requires explicit advance and never while a UserAction is pending; do not call volicord.prepare_write before implementation. Advisor work uses only a non-write Change Unit. On ready_to_finalize_advice, finalize the current advisor result with volicord.record_shaping; do not use volicord.record_run, volicord.advance_task, or volicord.prepare_write for advisor. Create current UserAction requests before presenting user-owned choices; a chat reply is not a User Channel resolution. Never present a rejected mutation as success; surface the tagged workflow and every structured presentation.must_surface fact. Evaluate close readiness only during close review. Close blockers do not replace workflow progression. Mutation tools default to a fresh compact authority receipt plus the method outcome needed for the next step; request detail=workflow for current workflow authority or detail=full only when the bounded full method result is needed. When a mutation returns a non-null operation_result_ref, use {} for omitted exact historical bytes and {} separately for current authority; never retry an applied mutation. Volicord state management is separate from product-file edit authority: product-file edits still require the host/user path and any required write ticket. A write ticket records intended product-file changes; it is not OS permission, review bypass, access control, or a promise of automatic tool use.",
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
    .replacen(
        "Inspect the exact User Channel resolution outcome.",
        "When revising a checkpoint, carry every current compatible applied decision explicitly through carry_forward_application_refs; never replace a checkpoint to discard applied authority. Inspect the exact User Channel resolution outcome.",
        1,
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
        assert!(instructions.contains("volicord.record_shaping"));
        assert!(instructions.contains("volicord.advance_task"));
        assert!(instructions.contains("use advisor only"));
        assert!(instructions.contains("instead of expanding it"));
        assert!(instructions.contains("fresh compact authority receipt"));
        assert!(instructions.contains(AgentToolId::GET_OPERATION_RESULT.wire_name()));
        assert!(instructions.contains("never retry an applied mutation"));
        for required in [
            "tagged workflow's required_action",
            "current shaping checkpoint and linked UserAction authority",
            "accepted-but-unapplied decision",
            "carry every current compatible applied decision explicitly",
            "carry_forward_application_refs",
            "never replace a checkpoint to discard applied authority",
            "Inspect the exact User Channel resolution outcome",
            "apply only accepted, current, compatible authority",
            "through its application_owner",
            "follow decision_recovery_required and revise shaping",
            "Never retry resolution of a terminal or expired request",
            "successor UserActionRequest with an independent identity",
            "keeps Product Repository mutation unavailable",
            "product-only or technical-only work",
            "Change Unit creation does not advance phase",
            "UserAction requests before presenting user-owned choices",
            "chat reply is not a User Channel resolution",
            "tagged workflow requires explicit advance",
            "never while a UserAction is pending",
            "Advisor work uses only a non-write Change Unit",
            "ready_to_finalize_advice",
            "finalize the current advisor result with volicord.record_shaping",
            "volicord.prepare_write before implementation",
            "rejected mutation as success",
            "presentation.must_surface",
            "Close blockers do not replace workflow progression",
            "close readiness only during close review",
        ] {
            assert!(instructions.contains(required));
        }
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
