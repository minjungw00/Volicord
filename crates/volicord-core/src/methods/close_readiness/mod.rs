mod acceptance;
mod blockers;
mod change_control;
mod evidence;
mod facts;
mod guidance;
mod policy;
mod service;
mod summary;

#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;

pub(crate) use blockers::{normalize_close_blockers, open_write_ticket_close_blocker};
pub(crate) use facts::{
    facts_from_projection, facts_with_pending_authorities,
    facts_with_projected_acceptance_criteria, facts_with_record_run_projection,
    facts_with_resolved_authorities, facts_with_resolved_unrecorded_changes, CloseReadinessFacts,
};
pub(crate) use guidance::close_next_action;
pub(crate) use service::{
    assess_close_readiness, plan_close_readiness, plan_projected_close_readiness,
    CloseReadinessRequest,
};
pub(crate) use summary::{CloseReadinessAssessment, CloseReadinessSummary};
