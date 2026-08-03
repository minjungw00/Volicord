#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! UserAction validation, authority, lifecycle, persistence, and semantic services.

mod authority;
mod body;
mod continuity;
mod error;
mod identity;
mod lifecycle;
mod materialization;
mod model;
mod persistence;
mod projection;
mod relevance;
mod resolution;
mod service;
mod summary;
#[cfg(test)]
mod tests;
mod validation;

pub use authority::{
    user_action_authority_from_record, user_action_authority_from_state, user_action_from_record,
};
pub use continuity::{
    derive_user_action_continuity, UserActionContinuityDraft, UserActionContinuityInput,
};
pub use error::{
    UserActionIdentityError, UserActionInvariantError, UserActionServiceError,
    UserActionUnavailable, UserActionValidationError,
};
pub use identity::UserActionOrigin;
pub use lifecycle::projected_user_action_lifecycle_phase;
pub use materialization::{
    materialize_user_action_request, materialize_user_action_resolution,
    MaterializedUserActionRequest, MaterializedUserActionResolution,
    UserActionMaterializationInput, UserActionResolutionMaterializationInput,
};
pub use model::{
    CurrentUserActionFacts, CurrentUserActionRead, CurrentUserActionUnavailableReason,
    PendingUserAction, PendingUserActionFacts, PendingUserActionFactsRequest,
    PendingUserActionResolutionSnapshot, UserActionAuthority, UserActionConstructionContext,
    UserActionConstructionInput, UserActionIntent, UserActionPersistenceContext,
    UserActionResolutionAvailability, UserActionResolutionFacts, UserActionResolutionFactsBody,
    UserActionResolutionUnavailableReason, ValidatedUserAction,
};
pub use projection::{pending_user_action_facts_from_records, user_action_resolution_facts};
pub use relevance::{
    accepted_current_user_authority, current_cancellation_authority, current_sensitive_approval,
    sensitive_action_scope_matches_requirement, user_action_blocks_operation,
    user_action_has_current_basis, user_action_keeps_task_waiting, user_action_required_for,
    verified_user_channel_provenance, CancellationAuthorityRequirement,
    SensitiveApprovalRequirement, UserActionOperation, UserActionOperationContext,
};
pub use resolution::{
    construct_user_action_resolution, resolution_input_matches_body,
    user_action_resolution_from_record, validate_current_resolution_basis,
};
pub use service::{
    construct_user_action, pending_user_action_authorities, pending_user_action_refs_for_operation,
    projected_pending_user_action_refs, resolved_user_action_facts,
    resolved_user_action_facts_for_all_kinds,
};
pub use summary::{agent_safe_pending_user_action_summaries, pending_user_action_instruction};
