#![forbid(unsafe_code)]

//! Core-facing services for owner-defined Volicord behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

use volicord_store::{
    artifacts::ArtifactStoreBoundary, sqlite::SqliteStoreBoundary,
    user_action_channel::UserActionChannelTokenRecord,
};
use volicord_types::{
    AgentSafeUserActionResolution, ProjectId, ResolveUserActionRequest, StateRecordRef, TaskId,
    TypeBoundary, UserActionInboxForm, UserActionInboxItem, UserActionRequest, UserActionRequestId,
    UserActionStatus, UserChannelAvailability, UtcTimestamp,
};

mod authority_status;

/// Closed adapter-owned completion metadata included in local-web replay identity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalWebConsentCompletionMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_recording: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

mod methods;
pub mod pipeline;
mod policy;

pub use authority_status::{
    validate_authority_status, AuthorityStatusExpectation, AuthorityStatusValidationError,
    ValidatedAuthorityStatus,
};
pub use pipeline::{
    dry_run_response, method_result_base, method_result_value, rejected_response, tool_error,
    Clock, CorePipelineError, CoreResult, CoreService, GitWorkspaceContext, InvocationContext,
    PipelineResponse, SystemClock, VerifiedInvocationContext,
};

/// Internal Core request for local web consent user capture.
///
/// The raw token is adapter-supplied capture context. It is validated and
/// hashed for lookup but is not stored as public method request data.
#[derive(Clone, PartialEq)]
pub struct LocalWebConsentUserActionRequest {
    pub request: ResolveUserActionRequest,
    pub token: String,
    pub expected_connection_internal_id: String,
    pub completion_metadata_json: String,
}

/// Internal input for projecting the canonical local-web consent form after
/// the adapter has validated the presented bearer token.
///
/// Core rereads and exact-matches the supplied token record inside the same
/// project snapshot used for session, request, creator, and form validation.
/// This boundary is intentionally not serializable.
#[derive(Clone, PartialEq, Eq)]
pub struct LocalWebConsentUserActionProjectionRequest {
    pub token: String,
    pub validated_token: UserActionChannelTokenRecord,
    pub allow_resolved_replay: bool,
}

/// Complete Core-owned request and form for the loopback User Channel.
///
/// This value can contain private user-facing context and candidates. It is a
/// nonserialized Core-to-User-Channel boundary and must not enter MCP tool
/// output, diagnostics, replay bytes, or public method results.
#[derive(Clone, PartialEq)]
pub struct LocalWebConsentUserActionProjection {
    pub request: UserActionRequest,
    pub form: UserActionInboxForm,
}

/// Fail-closed classification for the local-web projection boundary.
///
/// `FormMismatch` is kept distinct so the loopback renderer can preserve its
/// specific user-facing conflict response without learning or rebuilding the
/// canonical form in the adapter.
#[derive(Clone, PartialEq)]
pub enum LocalWebConsentUserActionProjectionOutcome {
    Projected(Box<LocalWebConsentUserActionProjection>),
    Invalid,
    FormMismatch,
}

impl std::fmt::Debug for LocalWebConsentUserActionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalWebConsentUserActionRequest")
            .field("request", &self.request)
            .field("token", &"[REDACTED]")
            .field(
                "expected_connection_internal_id",
                &self.expected_connection_internal_id,
            )
            .field("completion_metadata_json", &self.completion_metadata_json)
            .finish()
    }
}

/// Derives the only accepted local-web channel submission id for one bearer
/// token and user-action authority coordinate.
///
/// The returned id contains only a digest; the raw bearer token is never part
/// of public request data, durable replay rows, or method responses.
pub fn local_web_channel_submission_id(
    project_id: &ProjectId,
    user_action_request_id: &UserActionRequestId,
    token: &str,
    expected_connection_internal_id: &str,
    completion_metadata: &LocalWebConsentCompletionMetadata,
) -> Result<String, serde_json::Error> {
    let digest = volicord_types::canonical_json_bare_sha256(&serde_json::json!({
        "project_id": project_id,
        "user_action_request_id": user_action_request_id,
        "token": token,
        "expected_connection_internal_id": expected_connection_internal_id,
        "completion_metadata": completion_metadata,
    }))?;
    Ok(format!("local_web:{digest}"))
}

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
