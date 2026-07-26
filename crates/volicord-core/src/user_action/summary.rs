use volicord_types::ids::UserActionRequestId;
use volicord_types::schema::{AgentSafeUserActionRequestSummary, StateRecordRef};

/// Returns adapter-neutral application guidance for pending actions.
pub(crate) fn pending_user_action_instruction() -> String {
    "Resolve pending user actions through the User Channel.".to_owned()
}

/// Reduces pending request refs to the only projection allowed in agent results.
pub(crate) fn agent_safe_pending_user_action_summaries(
    refs: impl IntoIterator<Item = StateRecordRef>,
) -> Vec<AgentSafeUserActionRequestSummary> {
    refs.into_iter()
        .map(|record_ref| {
            AgentSafeUserActionRequestSummary::pending(UserActionRequestId::new(
                record_ref.record_id.as_str(),
            ))
        })
        .collect()
}
