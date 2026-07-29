use std::{error::Error, fmt};

use volicord_store::StoreError;
use volicord_types::values::UserActionStatus;

/// Focused validation failure for one semantic UserAction field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserActionValidationError {
    field: &'static str,
    message: &'static str,
}

impl UserActionValidationError {
    pub const fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }

    pub const fn message(&self) -> &'static str {
        self.message
    }
}

/// Typed semantic reason why a requested UserAction transition is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionUnavailable {
    TaskNotFound,
    CurrentCloseBasisRequired,
    ResolutionRequestNotFound,
    ResolutionStatus(UserActionStatus),
    BasisNotCurrent,
    CloseBasisNotCurrent,
    ChannelSubmissionConflict,
    ChannelSubmissionAlreadyCommitted,
    SelectedArtifactChanged,
    CurrentChangeUnitRequired,
    CurrentBaselineRequired,
    StoredResolutionOptionMissing,
    AcceptedRiskIdentityMismatch,
}

impl UserActionUnavailable {
    pub const fn message(self) -> &'static str {
        match self {
            Self::TaskNotFound => "the UserAction Task does not exist",
            Self::CurrentCloseBasisRequired => {
                "a current close basis is required for this user action"
            }
            Self::ResolutionRequestNotFound => {
                "user_action_request_id does not identify a current user action"
            }
            Self::ResolutionStatus(UserActionStatus::Expired) => {
                "user action expired at or before this resolution"
            }
            Self::ResolutionStatus(UserActionStatus::Stale) => "user action basis is stale",
            Self::ResolutionStatus(UserActionStatus::Superseded) => {
                "user action basis is superseded"
            }
            Self::ResolutionStatus(UserActionStatus::Resolved) => {
                "user action is already resolved"
            }
            Self::ResolutionStatus(UserActionStatus::Pending) => {
                "pending user action is available for resolution"
            }
            Self::BasisNotCurrent => "user-action basis is not current for this resolution",
            Self::CloseBasisNotCurrent => "user-action close basis is no longer current",
            Self::ChannelSubmissionConflict => {
                "channel_submission_id conflicts with an immutable stored resolution"
            }
            Self::ChannelSubmissionAlreadyCommitted => {
                "channel submission is already committed; pipeline replay must use its original request identity"
            }
            Self::SelectedArtifactChanged => {
                "selected observation artifact changed after the request was created"
            }
            Self::CurrentChangeUnitRequired => {
                "evidence observation resolution requires the current Change Unit"
            }
            Self::CurrentBaselineRequired => {
                "evidence observation resolution requires a current baseline"
            }
            Self::StoredResolutionOptionMissing => {
                "stored user-action resolution does not select a request option"
            }
            Self::AcceptedRiskIdentityMismatch => {
                "accepted residual-risk identities do not match the current close basis"
            }
        }
    }
}

/// Typed identity planning failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionIdentityError {
    MissingOperationIdentity,
    InvalidOriginMetadata,
}

/// Internal invariant failure that is not a caller validation rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionInvariantError {
    TaskIdentityMismatch,
    ActionFactsMismatch,
    Serialization,
}

/// Failures produced by UserAction semantic services.
#[derive(Debug)]
pub enum UserActionServiceError {
    Validation(UserActionValidationError),
    CorruptStoredState(StoreError),
    Store(StoreError),
    Identity(UserActionIdentityError),
    Invariant(UserActionInvariantError),
    Unavailable(UserActionUnavailable),
}

impl UserActionServiceError {
    pub fn from_store(error: StoreError) -> Self {
        match error {
            error @ (StoreError::CorruptStoredJson { .. }
            | StoreError::CorruptOwnerStateJson { .. }
            | StoreError::CorruptOwnerStateValue { .. }
            | StoreError::CorruptOwnerStateInvariant { .. }
            | StoreError::CorruptStoredValue { .. }
            | StoreError::SchemaInvariant { .. }) => Self::CorruptStoredState(error),
            error => Self::Store(error),
        }
    }

    pub const fn validation(field: &'static str, message: &'static str) -> Self {
        Self::Validation(UserActionValidationError::new(field, message))
    }
}

impl fmt::Display for UserActionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => {
                write!(formatter, "{}: {}", error.field(), error.message())
            }
            Self::CorruptStoredState(error) => write!(formatter, "corrupt stored state: {error}"),
            Self::Store(error) => write!(formatter, "store error: {error}"),
            Self::Identity(error) => write!(formatter, "user-action identity error: {error:?}"),
            Self::Invariant(error) => write!(formatter, "user-action invariant error: {error:?}"),
            Self::Unavailable(error) => formatter.write_str(error.message()),
        }
    }
}

impl Error for UserActionServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CorruptStoredState(error) | Self::Store(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StoreError> for UserActionServiceError {
    fn from(error: StoreError) -> Self {
        Self::from_store(error)
    }
}
