use harness_store::core_pipeline::VerifiedReplayContext;
use harness_types::ToolRejectedResponse;

use crate::{
    pipeline::{rejected_response, VerifiedSurfaceContext},
    policy::access::{access_class_value, local_access_mismatch_error},
};

pub(crate) fn replay_context_from_verified_surface(
    verified_surface: &VerifiedSurfaceContext,
) -> VerifiedReplayContext {
    VerifiedReplayContext {
        surface_id: verified_surface.surface_id.as_str().to_owned(),
        surface_instance_id: verified_surface.surface_instance_id.as_str().to_owned(),
        access_class: access_class_value(verified_surface.access_class).to_owned(),
        verification_basis: (!verified_surface.verification_basis.trim().is_empty())
            .then(|| verified_surface.verification_basis.clone()),
    }
}

pub(crate) fn replay_context_mismatch_response(
    dry_run: bool,
    current_state_version: u64,
) -> ToolRejectedResponse {
    rejected_response(
        dry_run,
        Some(current_state_version),
        vec![local_access_mismatch_error("idempotency_replay_context")],
    )
}
