//! Lifecycle-specific structured diagnostic persistence in the Runtime Home Registry.

mod current_state;
mod graph;
mod occurrence;
mod queries;
mod row;

pub use current_state::{resolve_current_finding, upsert_current_snapshot};
pub use graph::{
    bounded_stored_diagnostic_graph_from_seeds, diagnostic_root_cause_ids,
    MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH, MAX_DIAGNOSTIC_CAUSE_CHAIN_FINDINGS,
};
pub use occurrence::{
    insert_and_link_runtime_terminal_occurrence, insert_occurrence_finding,
    insert_occurrence_finding_graph,
};
pub use queries::{
    active_current_findings_for_scope, diagnostic_occurrences_for_runtime_session,
    reportable_diagnostic_findings_by_ids, stored_diagnostic_finding_by_id,
    stored_diagnostic_findings_by_ids,
};
