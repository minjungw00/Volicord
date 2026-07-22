use std::sync::atomic::AtomicU64;

pub use volicord_types::{AgentToolCategory, AgentToolId, AgentToolOwner, ToolVerificationRole};

pub(crate) const SERVER_NAME: &str = "volicord-mcp";
pub(crate) const DEFAULT_LOCALE: &str = "en-US";
pub(crate) static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn server_instructions() -> String {
    format!(
        "Volicord records task scope, write tickets, evidence, runs, user-action requests, evidence attachments, and Close Status for explicitly registered Product Repositories. Preserve the user's requested outcome when choosing Task scope: when analysis or shaping is a step toward an implementation outcome, keep one work Task and record that step as a shaping_update; use advisor only when the requested outcome itself is read-only advice. If the broader outcome is unclear, keep the known boundary in shaping state or ask the user instead of expanding it. If project selection is unclear, call {} and use one listed project_selector; do not guess from folders, roots, labels, or memory. Mutation tools default to a fresh compact authority receipt plus the method outcome needed for the next step; request detail=workflow for current next actions or detail=full only when the bounded full method result is needed. When a mutation returns a non-null operation_result_ref, use {} for omitted exact historical bytes and {} separately for current authority; never retry an applied mutation. Volicord state management is separate from product-file edit authority: product-file edits still require the host/user path and any required write ticket. A write ticket records intended product-file changes; it is not OS permission, review bypass, access control, or a promise of automatic tool use.",
        AgentToolId::LIST_PROJECTS.wire_name(),
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
}
