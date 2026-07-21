use std::sync::atomic::AtomicU64;

pub use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, CHECK_CLOSE_TOOL_NAME, CLOSE_TASK_TOOL_NAME,
    GET_OPERATION_RESULT_TOOL_NAME, INTAKE_TOOL_NAME, LIST_PROJECTS_TOOL_NAME,
    PREPARE_EVIDENCE_CAPTURE_TOOL_NAME, PREPARE_WRITE_TOOL_NAME, READ_ONLY_METHOD_TOOL_NAMES,
    RECONCILE_CHANGES_TOOL_NAME, RECORD_RUN_TOOL_NAME, REQUEST_USER_ACTION_TOOL_NAME,
    STAGE_ARTIFACT_TOOL_NAME, STATUS_TOOL_NAME, UPDATE_SCOPE_TOOL_NAME,
    WORKFLOW_METHOD_TOOL_NAMES as PUBLIC_METHOD_TOOL_NAMES,
};

pub(crate) const SERVER_NAME: &str = "volicord-mcp";
pub(crate) const DEFAULT_LOCALE: &str = "en-US";
pub(crate) static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) const SERVER_INSTRUCTIONS: &str = "Volicord records task scope, write tickets, evidence, runs, user-action requests, evidence attachments, and Close Status for explicitly registered Product Repositories. Preserve the user's requested outcome when choosing Task scope: when analysis or shaping is a step toward an implementation outcome, keep one work Task and record that step as a shaping_update; use advisor only when the requested outcome itself is read-only advice. If the broader outcome is unclear, keep the known boundary in shaping state or ask the user instead of expanding it. If project selection is unclear, call volicord.list_projects and use one listed project_selector; do not guess from folders, roots, labels, or memory. Mutation tools default to a fresh compact authority receipt plus the method outcome needed for the next step; request detail=workflow for current next actions or detail=full only when the bounded full method result is needed. When a mutation returns a non-null operation_result_ref, use volicord.get_operation_result for omitted exact historical bytes and volicord.status separately for current authority; never retry an applied mutation. Volicord state management is separate from product-file edit authority: product-file edits still require the host/user path and any required write ticket. A write ticket records intended product-file changes; it is not OS permission, review bypass, access control, or a promise of automatic tool use.";
pub(crate) const TRANSPORT_DISCLOSURE_TEXT: &str = "Does not prove: public API availability, authentication service status, security boundary, OS sandboxing, network isolation, write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion";

#[cfg(test)]
mod tests {
    use super::SERVER_INSTRUCTIONS;

    #[test]
    fn server_instructions_preserve_outcome_across_shaping_and_implementation() {
        assert!(SERVER_INSTRUCTIONS.contains("keep one work Task"));
        assert!(SERVER_INSTRUCTIONS.contains("record that step as a shaping_update"));
        assert!(SERVER_INSTRUCTIONS.contains("use advisor only"));
        assert!(SERVER_INSTRUCTIONS.contains("instead of expanding it"));
        assert!(SERVER_INSTRUCTIONS.contains("fresh compact authority receipt"));
        assert!(SERVER_INSTRUCTIONS.contains("volicord.get_operation_result"));
        assert!(SERVER_INSTRUCTIONS.contains("never retry an applied mutation"));
    }
}
