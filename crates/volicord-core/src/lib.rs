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
mod product_path;

pub use agent_session::{AgentSessionValidationError, ValidatedAgentSession};
pub use authority_status::{
    validate_authority_status, AuthorityStatusExpectation, AuthorityStatusValidationError,
    ValidatedAuthorityStatus,
};
pub use pipeline::{
    committed_result_base, dry_run_response, no_effect_result_base, read_only_result_base,
    rejected_response, staging_created_result_base, tool_error, Clock, CoreOperationalOperation,
    CoreOperationalResource, CoreOperationalUnavailable, CorePipelineError, CoreResult,
    CoreService, GitWorkspaceContext, InvocationAuthority, InvocationContext, PipelineResponse,
    SystemClock, VerifiedInvocationContext,
};
