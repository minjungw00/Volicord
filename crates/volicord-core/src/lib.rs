#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! Core-facing services for owner-defined Volicord behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

mod acceptance_facts;
mod agent_session;
mod artifact;
mod authority_status;
mod change_unit_planning;
mod close_readiness;
mod continuity;
mod enforcement_facts;
mod error_boundary;
mod evidence_facts;
mod evidence_projection;
mod guarantee_projection;
mod guidance;
mod identity;
mod json_object;
mod method_execution;
mod method_rejection;
mod methods;
mod operation_plan;
pub mod pipeline;
mod policy;
mod product_path;
mod record_refs;
mod recording;
mod state_summary;
mod summary_text;
mod task_facts;
mod task_policy;
mod task_state;
mod workflow_diagnostics;
mod write_ticket;

pub use agent_session::{AgentSessionValidationError, ValidatedAgentSession};
pub use authority_status::{
    validate_authority_status, AuthorityStatusExpectation, AuthorityStatusValidationError,
    ValidatedAuthorityStatus,
};
pub use pipeline::{
    dry_run_response, rejected_response, tool_error, Clock, CoreOperationalOperation,
    CoreOperationalResource, CoreOperationalUnavailable, CorePipelineError, CoreResult,
    CoreService, GitWorkspaceContext, InvocationAuthority, InvocationContext, PipelineResponse,
    SystemClock, VerifiedInvocationContext,
};
pub use write_ticket::current_validity::EvaluatedWriteTicket;
pub use write_ticket::service::load_evaluated_write_tickets;
