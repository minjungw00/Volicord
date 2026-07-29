use volicord_store::core_pipeline::VerifiedReplayContext;
use volicord_types::schema::ToolRejectedResponse;

use crate::{
    pipeline::{rejected_response, VerifiedInvocationContext},
    policy::access::invocation_context_mismatch_error,
};

pub(crate) fn replay_context_from_verified_invocation(
    verified_invocation: &VerifiedInvocationContext,
) -> Result<VerifiedReplayContext, serde_json::Error> {
    Ok(VerifiedReplayContext {
        actor_source: verified_invocation.actor_source.clone(),
        operation_category: verified_invocation.operation_category,
        verification_basis: (!verified_invocation.verification_basis.trim().is_empty())
            .then(|| verified_invocation.verification_basis.clone()),
        git_workspace_context: verified_invocation
            .git_workspace_context
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?
            .map(|value| {
                value
                    .as_object()
                    .cloned()
                    .expect("GitWorkspaceContext serializes as an object")
            }),
    })
}

pub(crate) fn replay_context_mismatch_response(
    dry_run: volicord_types::schema::DryRunIntent,
    current_state_version: u64,
) -> ToolRejectedResponse {
    rejected_response(
        dry_run,
        Some(current_state_version),
        vec![invocation_context_mismatch_error(
            "idempotency_replay_context",
        )],
    )
}
