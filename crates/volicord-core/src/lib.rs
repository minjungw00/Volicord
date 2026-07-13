#![forbid(unsafe_code)]

//! Core-facing services for owner-defined Volicord behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

use volicord_store::{artifacts::ArtifactStoreBoundary, sqlite::SqliteStoreBoundary};
use volicord_types::{
    AgentSafeUserActionResolution, ProjectId, ResolveUserActionRequest, StateRecordRef,
    TypeBoundary, UserActionRequestId, UserActionStatus, UtcTimestamp,
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
