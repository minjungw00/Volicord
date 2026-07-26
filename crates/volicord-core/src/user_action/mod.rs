//! Shared UserAction domain and application services.

pub(crate) mod authority;
mod body;
pub(crate) mod continuity;
pub(crate) mod identity;
pub(crate) mod lifecycle;
pub(crate) mod materialization;
pub(crate) mod model;
pub(crate) mod persistence;
mod projection;
mod reader;
pub(crate) mod resolution;
pub(crate) mod service;
pub(crate) mod summary;
#[cfg(test)]
mod tests;
mod validation;

pub use model::{
    CurrentUserActionFacts, CurrentUserActionRead, CurrentUserActionUnavailableReason,
    PendingUserAction, PendingUserActionFacts, PendingUserActionFactsRequest,
    PendingUserActionResolutionSnapshot, UserActionResolutionAvailability,
    UserActionResolutionFacts, UserActionResolutionFactsBody,
    UserActionResolutionUnavailableReason,
};
