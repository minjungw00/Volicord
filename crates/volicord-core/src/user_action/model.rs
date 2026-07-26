use std::path::PathBuf;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, EffectiveUserActionRecord, ProjectStateHeader, TaskRecord,
};
use volicord_types::ids::{
    ChangeUnitId, ProjectId, RiskId, TaskId, UserActionOptionId, UserActionRequestId,
    UserActionResolutionId,
};
use volicord_types::schema::{
    ArtifactRef, EvidenceTarget, RequiredNullable, StateRecordRef, ToolEnvelope, UserActionBasis,
    UserActionBasisCoordinates, UserActionDraft, UserActionRequest, UserActionRequestBody,
};
use volicord_types::values::{
    EvidenceRelevanceStatus, JudgmentResolutionOutcome, UserActionChannelKind, UserActionKind,
    UserActionOptionAction, UserActionRequiredFor, UserActionStatus, UtcTimestamp,
};

/// Semantic intent supplied by a Core operation that needs one current UserAction.
#[derive(Debug, Clone)]
pub(crate) struct UserActionIntent {
    pub(crate) task_id: TaskId,
    pub(crate) change_unit_id: Option<ChangeUnitId>,
    pub(crate) action: UserActionDraft,
    pub(crate) required_for: Vec<UserActionRequiredFor>,
    pub(crate) expires_at: RequiredNullable<UtcTimestamp>,
}

/// Current domain facts used to validate and construct one canonical UserAction.
pub(crate) struct UserActionConstructionInput<'a> {
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) project_state: &'a ProjectStateHeader,
    pub(crate) envelope: &'a ToolEnvelope,
    pub(crate) task: &'a TaskRecord,
    pub(crate) current_change_unit: Option<&'a ChangeUnitRecord>,
    pub(crate) operation_now: &'a UtcTimestamp,
    pub(crate) intent: UserActionIntent,
}

/// Store-acquired facts needed by canonical body construction.
#[derive(Debug, Clone)]
pub(super) enum UserActionBodyFacts {
    Choice {
        close_basis_revision: Option<u64>,
        result_refs: Vec<StateRecordRef>,
        residual_risk_ids: Vec<RiskId>,
    },
    EvidenceObservation {
        artifact_candidates: Vec<ArtifactRef>,
    },
}

/// Pure validation input assembled from semantic intent and current facts.
pub(super) struct UserActionValidationInput {
    pub(super) project_id: ProjectId,
    pub(super) repository_root: PathBuf,
    pub(super) actual_task_id: String,
    pub(super) task_scope_revision: u64,
    pub(super) baseline_ref: Option<String>,
    pub(super) current_change_unit_id: Option<ChangeUnitId>,
    pub(super) requested_change_unit_exists: bool,
    pub(super) state_version: u64,
    pub(super) operation_now: UtcTimestamp,
    pub(super) intent: UserActionIntent,
}

/// Semantic intent after pure current-fact validation and normalization.
#[derive(Debug)]
pub(super) struct ValidatedUserActionIntent {
    pub(super) task_id: TaskId,
    pub(super) coordinate_change_unit_id: Option<ChangeUnitId>,
    pub(super) action: UserActionDraft,
    pub(super) coordinates: UserActionBasisCoordinates,
    pub(super) required_for: Vec<UserActionRequiredFor>,
    pub(super) expires_at: RequiredNullable<UtcTimestamp>,
    pub(super) created_at: UtcTimestamp,
}

/// Validated semantic intent with its canonical typed body and authority basis.
#[derive(Debug, Clone)]
pub(crate) struct ValidatedUserAction {
    pub(crate) task_id: TaskId,
    pub(crate) coordinate_change_unit_id: Option<ChangeUnitId>,
    pub(crate) body: UserActionRequestBody,
    pub(crate) basis: UserActionBasis,
    pub(crate) required_for: Vec<UserActionRequiredFor>,
    pub(crate) expires_at: RequiredNullable<UtcTimestamp>,
    pub(crate) created_at: UtcTimestamp,
}

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
