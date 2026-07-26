#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! Core-facing services for owner-defined Volicord behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

use volicord_store::core_pipeline::EffectiveUserActionRecord;
use volicord_types::ids::{
    ProjectId, TaskId, UserActionOptionId, UserActionRequestId, UserActionResolutionId,
};
use volicord_types::schema::{ArtifactRef, EvidenceTarget, StateRecordRef, UserActionRequest};
use volicord_types::values::{
    EvidenceRelevanceStatus, JudgmentResolutionOutcome, UserActionChannelKind, UserActionKind,
    UserActionOptionAction, UserActionStatus, UtcTimestamp,
};

mod agent_session;
mod authority_status;

mod methods;
pub mod pipeline;
mod policy;

pub use agent_session::{AgentSessionValidationError, ValidatedAgentSession};
pub use authority_status::{
    validate_authority_status, AuthorityStatusExpectation, AuthorityStatusValidationError,
    ValidatedAuthorityStatus,
};
pub use pipeline::{
    dry_run_response, method_result_base, rejected_response, tool_error, Clock, CorePipelineError,
    CoreResult, CoreService, GitWorkspaceContext, InvocationAuthority, InvocationContext,
    PipelineResponse, SystemClock, VerifiedInvocationContext,
};

/// Current adapter-neutral semantic facts for one user-action request.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentUserActionFacts {
    pub project_id: ProjectId,
    pub user_action_request_id: UserActionRequestId,
    pub action_kind: UserActionKind,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub status: UserActionStatus,
    pub resolution_availability: UserActionResolutionAvailability,
    pub user_action_resolution_ref: Option<StateRecordRef>,
    pub user_action_resolution: Option<UserActionResolutionFacts>,
    pub derived_refs: Vec<StateRecordRef>,
}

/// Result of reading current user-action facts at an adapter boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum CurrentUserActionRead {
    Available(Box<CurrentUserActionFacts>),
    Unavailable(CurrentUserActionUnavailableReason),
}

/// Neutral reason why current user-action facts are unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentUserActionUnavailableReason {
    NotFound,
}

/// Semantic availability of the user-owned resolution transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionResolutionAvailability {
    Available,
    Unavailable(UserActionResolutionUnavailableReason),
}

impl UserActionResolutionAvailability {
    /// Derives semantic resolution availability from the effective lifecycle status.
    pub const fn from_status(status: UserActionStatus) -> Self {
        match status {
            UserActionStatus::Pending => Self::Available,
            UserActionStatus::Resolved => {
                Self::Unavailable(UserActionResolutionUnavailableReason::AlreadyResolved)
            }
            UserActionStatus::Stale => {
                Self::Unavailable(UserActionResolutionUnavailableReason::Stale)
            }
            UserActionStatus::Superseded => {
                Self::Unavailable(UserActionResolutionUnavailableReason::Superseded)
            }
            UserActionStatus::Expired => {
                Self::Unavailable(UserActionResolutionUnavailableReason::Expired)
            }
        }
    }

    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Neutral reason why a user-owned resolution transition is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionResolutionUnavailableReason {
    AlreadyResolved,
    Stale,
    Superseded,
    Expired,
}

/// Adapter-neutral safe facts for one immutable user-action resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct UserActionResolutionFacts {
    pub user_action_resolution_id: UserActionResolutionId,
    pub user_action_request_id: UserActionRequestId,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub resolved_at: UtcTimestamp,
    pub resolution: UserActionResolutionFactsBody,
}

/// Closed adapter-neutral resolution facts without private user-authored text.
#[derive(Debug, Clone, PartialEq)]
pub enum UserActionResolutionFactsBody {
    Choice {
        selected_option_id: UserActionOptionId,
        selected_option_label: String,
        machine_action: UserActionOptionAction,
        resolution_outcome: JudgmentResolutionOutcome,
    },
    EvidenceObservation {
        target: EvidenceTarget,
        artifact_refs: Vec<ArtifactRef>,
        relevance_status: EvidenceRelevanceStatus,
    },
}

/// Internal request for current pending UserAction facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUserActionFactsRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
}

/// Current adapter-neutral pending UserAction facts for one Task.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingUserActionFacts {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub actions: Vec<PendingUserAction>,
}

/// One pending UserAction and its typed resolution availability.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingUserAction {
    pub request_ref: StateRecordRef,
    pub request: UserActionRequest,
    pub resolution_availability: UserActionResolutionAvailability,
}

/// One coherent Store snapshot used to plan a local User Channel resolution.
///
/// The exact effective record and pending semantic facts come from the same
/// project SQLite snapshot. Terminal records have no pending action set.
#[derive(Clone, PartialEq)]
pub struct PendingUserActionResolutionSnapshot {
    pub project_id: ProjectId,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub record: EffectiveUserActionRecord,
    pub resolution_availability: UserActionResolutionAvailability,
    pub pending_actions: Option<PendingUserActionFacts>,
}
