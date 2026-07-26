#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! Core-facing services for owner-defined Volicord behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

use volicord_store::{
    artifacts::ArtifactStoreBoundary, core_pipeline::EffectiveUserActionRecord,
    sqlite::SqliteStoreBoundary,
};
use volicord_types::ids::{ProjectId, TaskId, UserActionRequestId};
use volicord_types::methods::AgentSafeUserActionResolution;
use volicord_types::schema::{
    StateRecordRef, UserActionInboxItem, UserActionRequest, UserChannelAvailability,
};
use volicord_types::values::{UserActionStatus, UtcTimestamp};
use volicord_types::TypeBoundary;

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
    CoreResult, CoreService, GitWorkspaceContext, InvocationContext, PipelineResponse, SystemClock,
    VerifiedInvocationContext,
};

/// Current Core-owned, agent-safe view of one user-action request.
///
/// This internal adapter boundary exposes only the effective lifecycle status,
/// the immutable resolution reference and safe summary, and public records
/// derived by that resolution. User-authored private text is intentionally
/// excluded.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentUserActionProjection {
    pub project_id: ProjectId,
    pub user_action_request_id: UserActionRequestId,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub status: UserActionStatus,
    pub user_action_resolution_ref: Option<StateRecordRef>,
    pub user_action_resolution: Option<AgentSafeUserActionResolution>,
    pub derived_refs: Vec<StateRecordRef>,
}

/// Internal request for the user-visible inbox projection.
///
/// This boundary is intentionally not serializable and is not part of the
/// public method or MCP schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserChannelInboxProjectionRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
}

/// Current user-visible inbox projection for one Task.
///
/// The projection may contain private question, form, path, and command data.
/// It therefore remains a typed, nonserialized Core-to-User-Channel boundary.
#[derive(Clone, PartialEq)]
pub struct UserChannelInboxProjection {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub user_channel_availability: UserChannelAvailability,
    pub items: Vec<UserChannelInboxProjectionItem>,
}

/// One trusted User Channel item with both its immutable machine request and
/// its user-facing presentation.
///
/// Adapters use the request body for closed resolution semantics and the inbox
/// item for user presentation. Neither value crosses the public result
/// boundary through this container.
#[derive(Clone, PartialEq)]
pub struct UserChannelInboxProjectionItem {
    pub request: UserActionRequest,
    pub inbox_item: UserActionInboxItem,
}

/// One coherent Store snapshot used to plan a local User Channel resolution.
///
/// This internal, nonserialized boundary keeps the exact effective record and
/// its pending inbox projection on the same project SQLite snapshot. Terminal
/// records have no pending projection.
#[derive(Clone, PartialEq)]
pub struct UserChannelInboxResolutionSnapshot {
    pub project_id: ProjectId,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub record: EffectiveUserActionRecord,
    pub pending_projection: Option<UserChannelInboxProjection>,
}

/// Minimal Core service marker for validating crate boundaries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CoreBoundary {
    store: SqliteStoreBoundary,
    artifacts: ArtifactStoreBoundary,
}

impl CoreBoundary {
    /// Creates a Core boundary marker.
    pub const fn new() -> Self {
        Self {
            store: SqliteStoreBoundary,
            artifacts: ArtifactStoreBoundary,
        }
    }

    /// Identifies the shared type boundary used by Core-facing APIs.
    pub const fn api_type_boundary(self) -> TypeBoundary {
        let _ = self.store;
        let _ = self.artifacts;
        TypeBoundary::Api
    }
}

#[cfg(test)]
mod tests {
    use super::CoreBoundary;
    use volicord_types::TypeBoundary;

    #[test]
    fn core_boundary_points_to_api_types() {
        assert_eq!(CoreBoundary::new().api_type_boundary(), TypeBoundary::Api);
    }
}
