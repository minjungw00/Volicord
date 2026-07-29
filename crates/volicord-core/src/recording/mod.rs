mod artifact;
mod authority;
mod context;
mod evidence;
mod model;
mod plan;
mod projection;
mod state;

use crate::artifact::ArtifactPolicyError;
use crate::close_readiness::CloseReadinessError;
use crate::pipeline::CorePipelineError;
use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_store::error::StoreError;
use volicord_types::schema::{DryRunIntent, ToolEnvelope};
use volicord_user_action_service::UserActionServiceError;

pub(crate) use plan::plan_record_run;

#[derive(Debug)]
pub(crate) enum RecordingError {
    Core(CorePipelineError),
    UserAction(UserActionServiceError),
    Artifact(ArtifactPolicyError),
    CloseReadiness(CloseReadinessError),
    Rejected(RecordingRejection),
}

#[derive(Debug)]
pub(crate) enum RecordingRejection {
    Validation {
        field: &'static str,
        message: &'static str,
    },
    NoActiveTask,
    NoActiveChangeUnit {
        message: &'static str,
    },
    BaselineStale,
    WorkspaceStale,
    ProductPathContainment {
        message: &'static str,
    },
    DecisionRejected {
        message: &'static str,
    },
    WriteTicketRequired,
    WriteTicketInvalid {
        reason: &'static str,
        message: &'static str,
    },
    EvidenceInsufficient {
        message: &'static str,
    },
    ArtifactInput {
        artifact_input_id: String,
        reason: &'static str,
        message: &'static str,
    },
    ArtifactMissing {
        message: &'static str,
    },
}

impl From<CorePipelineError> for RecordingError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<serde_json::Error> for RecordingError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

impl From<UserActionServiceError> for RecordingError {
    fn from(error: UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

pub(super) fn recording_validation_error<T>(
    _dry_run: DryRunIntent,
    _state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, RecordingError> {
    Err(RecordingError::Rejected(RecordingRejection::Validation {
        field,
        message,
    }))
}

pub(super) fn recording_store_error(
    _envelope: &ToolEnvelope,
    _project_state: &ProjectStateHeader,
    error: StoreError,
) -> RecordingError {
    RecordingError::Core(CorePipelineError::from(error))
}

pub(super) fn recording_user_action_error(
    _envelope: &ToolEnvelope,
    _project_state: &ProjectStateHeader,
    error: UserActionServiceError,
) -> RecordingError {
    RecordingError::UserAction(error)
}
