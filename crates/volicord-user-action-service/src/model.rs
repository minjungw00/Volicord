use volicord_store::core_pipeline::{
    ChangeUnitRecord, StoredUserActionRecordSet, TaskRecord, UserActionStoreReader,
};
use volicord_types::ids::{
    ChangeUnitId, IdempotencyKey, ProjectId, RiskId, TaskId, UserActionOptionId,
    UserActionRequestId, UserActionResolutionId,
};
use volicord_types::schema::{
    ArtifactRef, EvidenceTarget, RequiredNullable, StateRecordRef, UserActionBasis,
    UserActionBasisCoordinates, UserActionDraft, UserActionRequest, UserActionRequestBody,
    UserActionResolutionBody,
};
use volicord_types::values::{
    ActorSource, EvidenceRelevanceStatus, JudgmentResolutionOutcome, UserActionBasisStatus,
    UserActionChannelKind, UserActionKind, UserActionOptionAction, UserActionRequiredFor,
    UserActionStatus, UserActionVerificationBasis, UtcTimestamp,
};

/// Semantic intent supplied by a Core operation that needs one current UserAction.
#[derive(Debug, Clone)]
pub struct UserActionIntent {
    pub task_id: TaskId,
    pub change_unit_id: Option<ChangeUnitId>,
    pub action: UserActionDraft,
    pub required_for: Vec<UserActionRequiredFor>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

/// Semantic operation facts needed to validate and construct a UserAction.
#[derive(Debug, Clone)]
pub struct UserActionConstructionContext {
    pub project_id: ProjectId,
    pub observed_state_version: u64,
    pub observed_at: UtcTimestamp,
    pub locale: Option<String>,
}

/// Current domain facts used to validate and construct one canonical UserAction.
pub struct UserActionConstructionInput<'a> {
    pub store: &'a dyn UserActionStoreReader,
    pub task: &'a TaskRecord,
    pub current_change_unit: Option<&'a ChangeUnitRecord>,
    pub context: UserActionConstructionContext,
    pub intent: UserActionIntent,
}

/// Durable identity and actor facts used to materialize one request.
#[derive(Debug, Clone)]
pub struct UserActionPersistenceContext {
    pub project_id: ProjectId,
    pub actor_source: ActorSource,
    pub operation_identity: IdempotencyKey,
    pub planned_state_version: u64,
    pub user_action_request_id: UserActionRequestId,
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
pub struct ValidatedUserAction {
    pub task_id: TaskId,
    pub coordinate_change_unit_id: Option<ChangeUnitId>,
    pub body: UserActionRequestBody,
    pub basis: UserActionBasis,
    pub required_for: Vec<UserActionRequiredFor>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
    pub created_at: UtcTimestamp,
}

/// Normalized authority facts decoded from one current UserAction record.
#[derive(Debug, Clone)]
pub struct UserActionAuthority {
    pub user_action_request_id: String,
    pub user_action_resolution_id: Option<String>,
    pub task_id: TaskId,
    pub action_kind: UserActionKind,
    pub status: UserActionStatus,
    pub required_for: Vec<UserActionRequiredFor>,
    pub affected_refs: Vec<StateRecordRef>,
    pub machine_action: Option<UserActionOptionAction>,
    pub resolution_outcome: Option<JudgmentResolutionOutcome>,
    pub resolved_by_actor_source: Option<ActorSource>,
    pub resolved_verification_basis: Option<UserActionVerificationBasis>,
    pub resolved_assurance_level: Option<String>,
    pub basis_status: UserActionBasisStatus,
    pub basis: Option<UserActionBasis>,
    pub resolution: Option<UserActionResolutionBody>,
    pub expires_at: Option<UtcTimestamp>,
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
    pub record: StoredUserActionRecordSet,
    pub resolution_availability: UserActionResolutionAvailability,
    pub pending_actions: Option<PendingUserActionFacts>,
}
