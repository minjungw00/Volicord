#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! Core-facing services for owner-defined Volicord behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

mod agent_session;
mod authority_status;

mod methods;
pub mod pipeline;
mod policy;
mod user_action;

pub use agent_session::{AgentSessionValidationError, ValidatedAgentSession};
pub use authority_status::{
    validate_authority_status, AuthorityStatusExpectation, AuthorityStatusValidationError,
    ValidatedAuthorityStatus,
};
pub use pipeline::{
    dry_run_response, method_result_base, rejected_response, tool_error, Clock,
    CoreOperationalOperation, CoreOperationalResource, CoreOperationalUnavailable,
    CorePipelineError, CoreResult, CoreService, GitWorkspaceContext, InvocationAuthority,
    InvocationContext, PipelineResponse, SystemClock, VerifiedInvocationContext,
};
pub use user_action::{
    CurrentUserActionFacts, CurrentUserActionRead, CurrentUserActionUnavailableReason,
    PendingUserAction, PendingUserActionFacts, PendingUserActionFactsRequest,
    PendingUserActionResolutionSnapshot, UserActionResolutionAvailability,
    UserActionResolutionFacts, UserActionResolutionFactsBody,
    UserActionResolutionUnavailableReason,
};
