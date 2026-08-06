use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use crate::identity::allocate_durable_id;
use chrono::{DateTime, Timelike, Utc};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_platform_fs::{PlatformBoundaryError, PlatformDiagnosticKind};
use volicord_store::{
    core_pipeline::{
        commit_input, CommitMutationInput, CommittedEventRef, CoreProjectStore,
        CoreStorageMutation, MutationCommitOutcome, PendingTaskEvent, ProjectStateHeader,
        TransitionCommitExpectation,
    },
    CanonicalRuntimeHomePath, RuntimeHomeMutationContext, StoreCorruptionLocation, StoreError,
    StoreFailureRoute, StoreResult,
};
use volicord_types::canonical::canonical_request_hash;
use volicord_types::ids::{
    BaselineRef, ChangeUnitId, DurableIdError, DurableIdGenerator, DurableIdKind, EventId,
    IdempotencyKey, ProjectId, RandomDurableIdGenerator, RequestHash, ShapingCheckpointId, TaskId,
    UserActionRequestId, UserActionResolutionId,
};
use volicord_types::methods::{
    public_method_contract, AdvanceTaskRequest, AssembleMethodResult, CheckCloseRequest,
    CloseTaskRequest, DryRunRequestRoute, FinalizeAdviceRequest, MethodResponseBranch,
    MethodResponseContract, OperationResultRef, PrepareEvidenceCaptureRequest, PrepareWriteRequest,
    ReconcileChangesRequest, RecordRunRequest, RecordShapingCheckpointRequest,
    RequestUserActionRequest, ResultBaseConstructionError, ResultEffectContract,
    StageArtifactRequest, SupportsCoreCommittedResult, SupportsNoEffectResult,
    SupportsReadOnlyResult, UpdateScopeRequest, WorkflowActionAdmissionClass,
};
use volicord_types::schema::{
    DryRunIntent, DryRunSummary, EventRef, GuaranteeDisclosure, JsonObject, RequiredNullable,
    StateRecordRef, ToolDryRunResponse, ToolEnvelope, ToolError, ToolRejectedResponse,
    TransitionDescriptor, WorkflowActionAuthorityCoordinates, WorkflowActionKey,
    WorkflowCheckpointActionCoordinates, WorkflowProjection,
};
use volicord_types::values::{
    ActorSource, ChangeUnitOperation, EffectKind, ErrorCode, MethodName, OperationCategory,
    RunKind, UserActionChannelKind, UtcTimestamp, WorkflowActionSemanticVariant,
    WorkflowExpectedResultState, WorkflowStateKind, WorkflowTransitionEffectClass,
};

use crate::policy::{
    access::derive_verified_invocation,
    replay::{replay_context_from_verified_invocation, replay_context_mismatch_response},
};

/// Result type for Core pipeline execution errors.
pub type CoreResult<T> = Result<T, CorePipelineError>;

/// Exact typed request submitted to the current-transition no-commit planner.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionSubmission {
    RecordShapingCheckpoint(RecordShapingCheckpointRequest),
    UpdateScope(UpdateScopeRequest),
    FinalizeAdvice(FinalizeAdviceRequest),
    AdvanceTask(AdvanceTaskRequest),
    PrepareEvidenceCapture(PrepareEvidenceCaptureRequest),
    PrepareWrite(PrepareWriteRequest),
    StageArtifact(StageArtifactRequest),
    RecordRun(RecordRunRequest),
    RequestUserAction(RequestUserActionRequest),
    ReconcileChanges(ReconcileChangesRequest),
    CheckClose(CheckCloseRequest),
    CloseTask(CloseTaskRequest),
}

impl TransitionSubmission {
    fn action_key(&self) -> CoreResult<WorkflowActionKey> {
        let (method, variant) = match self {
            Self::RecordShapingCheckpoint(request) => (
                MethodName::RecordShapingCheckpoint,
                match request.checkpoint_operation {
                    volicord_types::schema::ShapingCheckpointOperation::CreateInitial => {
                        WorkflowActionSemanticVariant::CreateInitial
                    }
                    volicord_types::schema::ShapingCheckpointOperation::ReplaceCurrent {
                        ..
                    } => WorkflowActionSemanticVariant::ReplaceCurrent,
                },
            ),
            Self::UpdateScope(request) => (
                MethodName::UpdateScope,
                match request.change_unit.operation {
                    ChangeUnitOperation::KeepCurrent => {
                        WorkflowActionSemanticVariant::KeepCurrentChangeUnit
                    }
                    ChangeUnitOperation::CreateCurrent => {
                        WorkflowActionSemanticVariant::CreateCurrentChangeUnit
                    }
                    ChangeUnitOperation::ReplaceCurrent => {
                        WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit
                    }
                },
            ),
            Self::FinalizeAdvice(_) => (
                MethodName::FinalizeAdvice,
                WorkflowActionSemanticVariant::FinalizeAdvice,
            ),
            Self::AdvanceTask(_) => (
                MethodName::AdvanceTask,
                WorkflowActionSemanticVariant::AdvanceTask,
            ),
            Self::PrepareEvidenceCapture(_) => (
                MethodName::PrepareEvidenceCapture,
                WorkflowActionSemanticVariant::PrepareEvidenceCapture,
            ),
            Self::PrepareWrite(_) => (
                MethodName::PrepareWrite,
                WorkflowActionSemanticVariant::PrepareWrite,
            ),
            Self::StageArtifact(_) => (
                MethodName::StageArtifact,
                WorkflowActionSemanticVariant::StageArtifact,
            ),
            Self::RecordRun(_) => (
                MethodName::RecordRun,
                WorkflowActionSemanticVariant::RecordRun,
            ),
            Self::RequestUserAction(_) => (
                MethodName::RequestUserAction,
                WorkflowActionSemanticVariant::RequestUserAction,
            ),
            Self::ReconcileChanges(_) => (
                MethodName::ReconcileChanges,
                WorkflowActionSemanticVariant::ReconcileChanges,
            ),
            Self::CheckClose(_) => (
                MethodName::CheckClose,
                WorkflowActionSemanticVariant::CheckClose,
            ),
            Self::CloseTask(_) => (
                MethodName::CloseTask,
                WorkflowActionSemanticVariant::CloseTask,
            ),
        };
        WorkflowActionKey::new(method, variant).map_err(|error| CorePipelineError::Invariant {
            detail: format!("typed transition submission has an invalid action key: {error}"),
        })
    }
}

/// Verified output of the exact current-transition planner before any effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoCommitTransitionPlan {
    pub action_key: WorkflowActionKey,
    pub basis_state_version: u64,
    pub effect_class: WorkflowTransitionEffectClass,
    pub expected_result_state: WorkflowExpectedResultState,
}

/// Typed Core operation that could not produce a method result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreOperationalOperation {
    ProductPathObservation,
    StoreAccess,
}

impl CoreOperationalOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductPathObservation => "product_path_observation",
            Self::StoreAccess => "store_access",
        }
    }
}

/// Typed infrastructure resource that was unavailable to Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreOperationalResource {
    ProductRepository,
    Store,
    RegistryStore,
    ProjectStore,
    RuntimeHome,
    PlatformEnvironment,
}

impl CoreOperationalResource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductRepository => "product_repository",
            Self::Store => "store",
            Self::RegistryStore => "registry_store",
            Self::ProjectStore => "project_store",
            Self::RuntimeHome => "runtime_home",
            Self::PlatformEnvironment => "platform_environment",
        }
    }
}

/// Neutral Core failure produced when infrastructure cannot provide a method result.
#[derive(Debug)]
pub struct CoreOperationalUnavailable {
    operation: CoreOperationalOperation,
    resource: CoreOperationalResource,
    retryable: bool,
    source: CoreOperationalSource,
}

#[derive(Debug)]
enum CoreOperationalSource {
    Platform(PlatformBoundaryError),
    Store(StoreError),
}

impl CoreOperationalUnavailable {
    fn from_store(source: StoreError) -> Self {
        let classification = source.classification();
        let resource = if source.platform_diagnostic().is_some() {
            CoreOperationalResource::PlatformEnvironment
        } else if classification.entity == Some("runtime_home") {
            CoreOperationalResource::RuntimeHome
        } else {
            match classification.database_kind {
                Some("registry") => CoreOperationalResource::RegistryStore,
                Some("project_state") => CoreOperationalResource::ProjectStore,
                _ => CoreOperationalResource::Store,
            }
        };
        Self {
            operation: CoreOperationalOperation::StoreAccess,
            resource,
            retryable: classification.retryable,
            source: CoreOperationalSource::Store(source),
        }
    }

    fn from_platform(source: PlatformBoundaryError) -> Self {
        let (operation, resource, retryable) = product_path_operational_route(source.kind());
        Self {
            operation,
            resource,
            retryable,
            source: CoreOperationalSource::Platform(source),
        }
    }

    pub const fn operation(&self) -> CoreOperationalOperation {
        self.operation
    }

    pub const fn resource(&self) -> CoreOperationalResource {
        self.resource
    }

    pub const fn retryable(&self) -> bool {
        self.retryable
    }

    /// Returns the platform diagnostic kind when the unavailable resource was
    /// observed by the platform owner.
    pub const fn platform_diagnostic_kind(&self) -> Option<PlatformDiagnosticKind> {
        match &self.source {
            CoreOperationalSource::Platform(error) => Some(error.kind()),
            CoreOperationalSource::Store(_) => None,
        }
    }
}

fn product_path_operational_route(
    kind: PlatformDiagnosticKind,
) -> (CoreOperationalOperation, CoreOperationalResource, bool) {
    (
        CoreOperationalOperation::ProductPathObservation,
        CoreOperationalResource::ProductRepository,
        kind.retryable(),
    )
}

impl fmt::Display for CoreOperationalUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Core operation unavailable: operation={}, resource={}, retryable={}",
            self.operation.as_str(),
            self.resource.as_str(),
            self.retryable
        )
    }
}

impl Error for CoreOperationalUnavailable {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.source {
            CoreOperationalSource::Platform(error) => Some(error),
            CoreOperationalSource::Store(error) => Some(error),
        }
    }
}

/// Errors that indicate implementation or storage failure outside public API rejection routing.
#[derive(Debug)]
pub enum CorePipelineError {
    OperationalUnavailable(CoreOperationalUnavailable),
    Store(StoreError),
    Json(serde_json::Error),
    DurableId(DurableIdError),
    GeneratedIdCollision {
        kind: DurableIdKind,
        attempts: usize,
    },
    InvalidDispatch {
        detail: String,
    },
    Invariant {
        detail: String,
    },
}

impl fmt::Display for CorePipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationalUnavailable(error) => error.fmt(formatter),
            Self::Store(error) => write!(formatter, "store error: {error}"),
            Self::Json(error) => write!(formatter, "json error: {error}"),
            Self::DurableId(error) => write!(formatter, "{error}"),
            Self::GeneratedIdCollision { kind, attempts } => write!(
                formatter,
                "could not allocate unique generated {kind} id after {attempts} attempts"
            ),
            Self::InvalidDispatch { detail } => {
                write!(formatter, "invalid pipeline dispatch: {detail}")
            }
            Self::Invariant { detail } => write!(formatter, "Core invariant failed: {detail}"),
        }
    }
}

impl Error for CorePipelineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OperationalUnavailable(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::DurableId(error) => Some(error),
            Self::GeneratedIdCollision { .. }
            | Self::InvalidDispatch { .. }
            | Self::Invariant { .. } => None,
        }
    }
}

impl From<StoreError> for CorePipelineError {
    fn from(error: StoreError) -> Self {
        if error.classification().route == StoreFailureRoute::OperationalUnavailable {
            Self::OperationalUnavailable(CoreOperationalUnavailable::from_store(error))
        } else {
            Self::Store(error)
        }
    }
}

impl From<PlatformBoundaryError> for CorePipelineError {
    fn from(error: PlatformBoundaryError) -> Self {
        Self::OperationalUnavailable(CoreOperationalUnavailable::from_platform(error))
    }
}

impl From<serde_json::Error> for CorePipelineError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<DurableIdError> for CorePipelineError {
    fn from(error: DurableIdError) -> Self {
        Self::DurableId(error)
    }
}

impl From<volicord_user_action_service::UserActionServiceError> for CorePipelineError {
    fn from(error: volicord_user_action_service::UserActionServiceError) -> Self {
        use volicord_user_action_service::UserActionServiceError;
        match error {
            UserActionServiceError::CorruptStoredState(error)
            | UserActionServiceError::Store(error) => Self::from(error),
            UserActionServiceError::Validation(error) => Self::InvalidDispatch {
                detail: format!(
                    "user-action service validation escaped method mapping: {}: {}",
                    error.field(),
                    error.message()
                ),
            },
            UserActionServiceError::Identity(error) => Self::InvalidDispatch {
                detail: format!("user-action service identity invariant failed: {error:?}"),
            },
            UserActionServiceError::Invariant(error) => Self::InvalidDispatch {
                detail: format!("user-action service invariant failed: {error:?}"),
            },
            UserActionServiceError::Unavailable(error) => Self::InvalidDispatch {
                detail: format!(
                    "user-action service availability escaped method mapping: {}",
                    error.message()
                ),
            },
        }
    }
}

/// Adapter-captured Git coordinate for the selected Product Repository.
///
/// Core never discovers Git state itself. The selected adapter captures this
/// coordinate at the method boundary and Core treats it as verified invocation
/// context only after the same structural and actor checks as the surrounding
/// invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitWorkspaceContext {
    pub git_common_dir: String,
    pub worktree_id: String,
    pub branch_ref: Option<String>,
    pub head_sha: Option<String>,
    pub workspace_fingerprint: String,
}

/// Host-neutral authority capability supplied to Core outside `ToolEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationAuthority {
    /// A local user acting through a supported User Channel.
    LocalUser { channel: UserActionChannelKind },
    /// A current Agent Connection session validated for this invocation.
    AgentConnection(crate::ValidatedAgentSession),
}

impl InvocationAuthority {
    /// Returns the semantic actor represented by this authority capability.
    pub fn actor_source(&self) -> ActorSource {
        match self {
            Self::LocalUser { .. } => ActorSource::LocalUser,
            Self::AgentConnection(session) => {
                ActorSource::AgentConnection(session.connection_id().clone())
            }
        }
    }

    /// Returns the local User Channel when this is local-user authority.
    pub const fn user_channel(&self) -> Option<UserActionChannelKind> {
        match self {
            Self::LocalUser { channel } => Some(*channel),
            Self::AgentConnection(_) => None,
        }
    }
}

/// Local host-neutral invocation facts supplied by an adapter outside `ToolEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationContext {
    pub project_id: ProjectId,
    pub operation_category: OperationCategory,
    authority: InvocationAuthority,
    pub session_id: Option<String>,
    pub git_workspace_context: Option<GitWorkspaceContext>,
}

impl InvocationContext {
    /// Creates local-user invocation facts from a typed User Channel.
    pub fn local_user(
        project_id: ProjectId,
        operation_category: OperationCategory,
        channel: UserActionChannelKind,
    ) -> Self {
        Self {
            project_id,
            operation_category,
            authority: InvocationAuthority::LocalUser { channel },
            session_id: None,
            git_workspace_context: None,
        }
    }

    /// Creates Agent Connection invocation facts from a validated authority capability.
    pub fn agent_connection(
        operation_category: OperationCategory,
        session: crate::ValidatedAgentSession,
    ) -> Self {
        Self {
            project_id: session.project_id().clone(),
            operation_category,
            authority: InvocationAuthority::AgentConnection(session),
            session_id: None,
            git_workspace_context: None,
        }
    }

    /// Returns the semantic actor represented by the typed authority capability.
    pub fn actor_source(&self) -> ActorSource {
        self.authority.actor_source()
    }

    /// Returns the local User Channel when this invocation carries local-user authority.
    pub const fn user_channel(&self) -> Option<UserActionChannelKind> {
        self.authority.user_channel()
    }

    pub(crate) const fn authority(&self) -> &InvocationAuthority {
        &self.authority
    }

    /// Adds the adapter-owned session correlation coordinate when the transport has one.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        self.session_id = (!session_id.trim().is_empty()).then_some(session_id);
        self
    }

    /// Adds the adapter-captured Git workspace coordinate for this invocation.
    pub fn with_git_workspace_context(mut self, context: GitWorkspaceContext) -> Self {
        self.git_workspace_context = Some(context);
        self
    }
}

/// Internal verified invocation context derived for one Core call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedInvocationContext {
    pub project_id: ProjectId,
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub verification_basis: String,
    pub assurance_level: String,
    pub session_id: Option<String>,
    pub git_workspace_context: Option<GitWorkspaceContext>,
}

/// Internal verified actor-provenance context derived for authority-bearing resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedActorContext {
    pub actor_source: ActorSource,
    pub verification_basis: String,
    pub assurance_level: String,
}

impl VerifiedActorContext {
    /// Derives actor provenance from the verified invocation context.
    pub fn from_verified_invocation(invocation: &VerifiedInvocationContext) -> Self {
        Self {
            actor_source: invocation.actor_source.clone(),
            verification_basis: invocation.verification_basis.clone(),
            assurance_level: invocation.assurance_level.clone(),
        }
    }
}

/// Task selector behavior required by the owner-selected branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskRequirement {
    None,
    Optional,
    Required,
    Exact(TaskId),
}

/// Idempotency replay behavior for the selected method/effect branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplayPolicy {
    None,
    Committed,
}

/// State-version freshness behavior for the selected method/effect branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FreshnessPolicy {
    None,
    IfPresent,
}

/// Storage/effect family selected before method-specific planning runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodEffectPolicy {
    ReadOnly,
    #[cfg(test)]
    NoEffect,
    Staging,
    DryRunPreview,
    CoreMutation,
}

/// Authoritative preflight policy for a public method branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodPolicy {
    pub(crate) operation_category: OperationCategory,
    pub(crate) task: TaskRequirement,
    pub(crate) replay: ReplayPolicy,
    pub(crate) freshness: FreshnessPolicy,
    pub(crate) effect: MethodEffectPolicy,
    pub(crate) current_state_default: bool,
}

impl MethodPolicy {
    pub(crate) fn exact(
        operation_category: OperationCategory,
        task: TaskRequirement,
        replay: ReplayPolicy,
        freshness: FreshnessPolicy,
        effect: MethodEffectPolicy,
    ) -> Self {
        Self {
            operation_category,
            task,
            replay,
            freshness,
            effect,
            current_state_default: false,
        }
    }

    /// Allows an owner-defined mutation to pin the current project state when
    /// its public request deliberately omits `expected_state_version`.
    pub(crate) fn with_current_state_default(mut self) -> Self {
        self.current_state_default = true;
        self
    }

    #[cfg(test)]
    fn for_branch<F, B>(
        operation_category: OperationCategory,
        task: TaskRequirement,
        branch: &OwnerPipelineBranch<F, B>,
    ) -> Self {
        match &branch.kind {
            OwnerPipelineBranchKind::ReadOnly { .. } => Self::exact(
                operation_category,
                task,
                ReplayPolicy::None,
                FreshnessPolicy::None,
                MethodEffectPolicy::ReadOnly,
            ),
            OwnerPipelineBranchKind::NoEffectResult { .. } => Self::exact(
                operation_category,
                task,
                ReplayPolicy::None,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::NoEffect,
            ),
            OwnerPipelineBranchKind::DryRunPreview { .. } => Self::exact(
                operation_category,
                task,
                ReplayPolicy::None,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::DryRunPreview,
            ),
            OwnerPipelineBranchKind::CommitMutation { .. } => Self::exact(
                operation_category,
                task,
                ReplayPolicy::Committed,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::CoreMutation,
            ),
        }
    }
}

/// Owner-selected branch shape used by the shared pipeline.
#[derive(Debug, Clone)]
pub(crate) struct OwnerPipelineBranch<F, B> {
    kind: OwnerPipelineBranchKind<F, B>,
}

#[derive(Debug, Clone)]
enum OwnerPipelineBranchKind<F, B> {
    ReadOnly {
        result_fields: F,
        build_base: fn(DryRunIntent, u64) -> Result<B, ResultBaseConstructionError>,
    },
    NoEffectResult {
        result_fields: F,
        build_base: fn(u64) -> B,
    },
    DryRunPreview {
        dry_run_summary: DryRunSummary,
    },
    CommitMutation {
        result_fields: F,
        event_kind: String,
        event_payload: JsonObject,
        task_id: Option<TaskId>,
        change_unit_id: Option<ChangeUnitId>,
        storage_mutations: Vec<CoreStorageMutation>,
        build_base: fn(u64, Vec<EventRef>) -> Result<B, ResultBaseConstructionError>,
    },
}

pub(crate) fn read_only_branch<M>(
    result_fields: M::ResultFields,
) -> OwnerPipelineBranch<M::ResultFields, M::ResultBase>
where
    M: SupportsReadOnlyResult,
{
    OwnerPipelineBranch {
        kind: OwnerPipelineBranchKind::ReadOnly {
            result_fields,
            build_base: M::read_only_result_base,
        },
    }
}

pub(crate) fn no_effect_result_branch<M>(
    result_fields: M::ResultFields,
) -> OwnerPipelineBranch<M::ResultFields, M::ResultBase>
where
    M: SupportsNoEffectResult,
{
    OwnerPipelineBranch {
        kind: OwnerPipelineBranchKind::NoEffectResult {
            result_fields,
            build_base: M::no_effect_result_base,
        },
    }
}

pub(crate) fn dry_run_preview_branch<M>(
    dry_run_summary: DryRunSummary,
) -> OwnerPipelineBranch<M::ResultFields, M::ResultBase>
where
    M: MethodResponseContract,
{
    OwnerPipelineBranch {
        kind: OwnerPipelineBranchKind::DryRunPreview { dry_run_summary },
    }
}

pub(crate) struct CommitMutationBranch<M>
where
    M: MethodResponseContract,
{
    pub result_fields: M::ResultFields,
    pub event_kind: String,
    pub event_payload: JsonObject,
    pub task_id: Option<TaskId>,
    pub change_unit_id: Option<ChangeUnitId>,
    pub storage_mutations: Vec<CoreStorageMutation>,
}

pub(crate) fn commit_mutation_branch<M>(
    branch: CommitMutationBranch<M>,
) -> OwnerPipelineBranch<M::ResultFields, M::ResultBase>
where
    M: SupportsCoreCommittedResult,
{
    OwnerPipelineBranch {
        kind: OwnerPipelineBranchKind::CommitMutation {
            result_fields: branch.result_fields,
            event_kind: branch.event_kind,
            event_payload: branch.event_payload,
            task_id: branch.task_id,
            change_unit_id: branch.change_unit_id,
            storage_mutations: branch.storage_mutations,
            build_base: M::core_committed_result_base,
        },
    }
}

/// Input to the shared Core request pipeline.
#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct PipelineRequest<F, B> {
    pub method_name: MethodName,
    pub envelope: ToolEnvelope,
    pub request_json: Value,
    pub invocation: InvocationContext,
    pub operation_category: OperationCategory,
    pub task_requirement: TaskRequirement,
    pub branch: OwnerPipelineBranch<F, B>,
}

/// Input to the shared preflight boundary before method-specific planning.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PipelinePreflightRequest {
    pub method_name: MethodName,
    pub attempted_action_key: Option<WorkflowActionKey>,
    pub envelope: ToolEnvelope,
    pub request_json: Value,
    pub invocation: InvocationContext,
    pub policy: MethodPolicy,
}

/// Exact current machine transition consumed by a state-bound request.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AdmittedTransition {
    pub descriptor: TransitionDescriptor,
    pub workflow: WorkflowProjection,
}

/// Verified request context produced by the shared preflight boundary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerifiedRequestContext {
    pub project_state: ProjectStateHeader,
    pub verified_invocation: VerifiedInvocationContext,
    pub verified_actor: VerifiedActorContext,
    pub resolved_task_id: Option<TaskId>,
}

/// Store-backed request prepared for method-specific planning or effect routing.
pub(crate) struct PreparedRequest<'mutation> {
    pub method_name: MethodName,
    pub envelope: ToolEnvelope,
    pub request_hash: RequestHash,
    pub store: CoreProjectStore<'mutation>,
    pub context: VerifiedRequestContext,
    pub operation_now: UtcTimestamp,
    pub admitted_transition: Option<AdmittedTransition>,
}

/// Preflight may either prepare a request or return an authoritative response.
pub(crate) enum PipelinePreflightOutcome<'mutation> {
    Prepared(Box<PreparedRequest<'mutation>>),
    Response(Box<PipelineResponse>),
}

/// Shared pipeline response with exact stored JSON when replayed.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineResponse {
    pub response_json: String,
    pub response_value: Value,
    pub operation_result_ref: Option<OperationResultRef>,
    pub verified_invocation: Option<VerifiedInvocationContext>,
    pub resolved_task_id: Option<TaskId>,
    pub replayed: bool,
}

impl PipelineResponse {
    /// Attaches the verified authority coordinates retained by a prepared
    /// request to a method-planning response.
    pub(crate) fn with_prepared_context(mut self, prepared: &PreparedRequest<'_>) -> Self {
        self.verified_invocation = Some(prepared.context.verified_invocation.clone());
        self.resolved_task_id = prepared.context.resolved_task_id.clone();
        self
    }

    /// Creates the adapter-internal marker returned after a verified no-commit plan.
    /// This marker is not a public method response and must not be emitted on a transport.
    pub fn from_no_commit_transition_plan(plan: NoCommitTransitionPlan) -> Self {
        no_commit_pipeline_response(plan)
    }
}

/// Runtime Home identity used by one Core service.
#[derive(Clone, PartialEq, Eq)]
enum CoreRuntimeHome {
    ReadOnly(PathBuf),
    Admitted(CanonicalRuntimeHomePath),
}

impl CoreRuntimeHome {
    fn as_path(&self) -> &Path {
        match self {
            Self::ReadOnly(path) => path,
            Self::Admitted(path) => path.as_path(),
        }
    }

    fn admitted(&self) -> Option<&CanonicalRuntimeHomePath> {
        match self {
            Self::ReadOnly(_) => None,
            Self::Admitted(path) => Some(path),
        }
    }
}

impl fmt::Debug for CoreRuntimeHome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnly(path) => formatter.debug_tuple("ReadOnly").field(path).finish(),
            Self::Admitted(path) => formatter.debug_tuple("Admitted").field(path).finish(),
        }
    }
}

/// Core request pipeline service bound to a local Runtime Home identity.
#[derive(Clone)]
pub struct CoreService {
    runtime_home: CoreRuntimeHome,
    id_generator: Arc<dyn DurableIdGenerator>,
    clock: Arc<dyn Clock>,
    execution_mode: CoreExecutionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreExecutionMode {
    Execute,
    PlanNoCommit(WorkflowActionKey),
}

impl fmt::Debug for CoreService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreService")
            .field("runtime_home", &self.runtime_home)
            .field("id_generator", &self.id_generator)
            .field("clock", &self.clock)
            .field("execution_mode", &self.execution_mode)
            .finish()
    }
}

impl PartialEq for CoreService {
    fn eq(&self, other: &Self) -> bool {
        self.runtime_home == other.runtime_home
    }
}

impl Eq for CoreService {}

impl CoreService {
    /// Creates a service for read-only Core work under a selected Runtime Home path.
    pub fn for_read_only(runtime_home: impl AsRef<Path>) -> Self {
        Self::for_read_only_with_id_generator_and_clock(
            runtime_home,
            RandomDurableIdGenerator,
            SystemClock,
        )
    }

    /// Creates a service for admitted Core work from the context's sole Runtime Home identity.
    ///
    /// ```compile_fail
    /// use std::path::Path;
    /// use volicord_core::CoreService;
    ///
    /// fn cannot_admit_from_path(path: &Path) {
    ///     let _ = CoreService::for_mutation(path);
    /// }
    /// ```
    pub fn for_mutation(context: &RuntimeHomeMutationContext<'_>) -> Self {
        Self::for_mutation_with_id_generator_and_clock(
            context,
            RandomDurableIdGenerator,
            SystemClock,
        )
    }

    /// Creates a read-only service with an injected durable ID generator.
    pub fn for_read_only_with_id_generator(
        runtime_home: impl AsRef<Path>,
        id_generator: impl DurableIdGenerator + 'static,
    ) -> Self {
        Self::for_read_only_with_id_generator_and_clock(runtime_home, id_generator, SystemClock)
    }

    /// Creates an admitted service with an injected durable ID generator.
    pub fn for_mutation_with_id_generator(
        context: &RuntimeHomeMutationContext<'_>,
        id_generator: impl DurableIdGenerator + 'static,
    ) -> Self {
        Self::for_mutation_with_id_generator_and_clock(context, id_generator, SystemClock)
    }

    /// Creates a read-only service with an injected UTC clock.
    pub fn for_read_only_with_clock(
        runtime_home: impl AsRef<Path>,
        clock: impl Clock + 'static,
    ) -> Self {
        Self::for_read_only_with_id_generator_and_clock(
            runtime_home,
            RandomDurableIdGenerator,
            clock,
        )
    }

    /// Creates an admitted service with an injected UTC clock.
    pub fn for_mutation_with_clock(
        context: &RuntimeHomeMutationContext<'_>,
        clock: impl Clock + 'static,
    ) -> Self {
        Self::for_mutation_with_id_generator_and_clock(context, RandomDurableIdGenerator, clock)
    }

    /// Creates a read-only service with injected durable ID generation and UTC time.
    pub fn for_read_only_with_id_generator_and_clock(
        runtime_home: impl AsRef<Path>,
        id_generator: impl DurableIdGenerator + 'static,
        clock: impl Clock + 'static,
    ) -> Self {
        Self {
            runtime_home: CoreRuntimeHome::ReadOnly(runtime_home.as_ref().to_path_buf()),
            id_generator: Arc::new(id_generator),
            clock: Arc::new(clock),
            execution_mode: CoreExecutionMode::Execute,
        }
    }

    /// Creates an admitted service with injected durable ID generation and UTC time.
    pub fn for_mutation_with_id_generator_and_clock(
        context: &RuntimeHomeMutationContext<'_>,
        id_generator: impl DurableIdGenerator + 'static,
        clock: impl Clock + 'static,
    ) -> Self {
        Self {
            runtime_home: CoreRuntimeHome::Admitted(context.runtime_home().clone()),
            id_generator: Arc::new(id_generator),
            clock: Arc::new(clock),
            execution_mode: CoreExecutionMode::Execute,
        }
    }

    /// Runs one exact typed current-transition request through its real method
    /// planner while retaining a read-only Store handle and stopping before all
    /// durable and transient effects.
    pub fn plan_transition_submission_no_commit(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        action_key: WorkflowActionKey,
        submission: TransitionSubmission,
        invocation: InvocationContext,
    ) -> CoreResult<NoCommitTransitionPlan> {
        let submitted_key = submission.action_key()?;
        if submitted_key != action_key {
            return Err(CorePipelineError::Invariant {
                detail: format!(
                    "no-commit transition submission key {} {} differs from requested key {} {}",
                    submitted_key.method.as_str(),
                    submitted_key.semantic_variant.as_str(),
                    action_key.method.as_str(),
                    action_key.semantic_variant.as_str()
                ),
            });
        }
        self.ensure_mutation_context(context)?;
        let mut planner = self.clone();
        planner.execution_mode = CoreExecutionMode::PlanNoCommit(action_key);
        let response = match submission {
            TransitionSubmission::RecordShapingCheckpoint(request) => {
                planner.record_shaping_checkpoint(context, request, invocation)
            }
            TransitionSubmission::UpdateScope(request) => {
                planner.update_scope(context, request, invocation)
            }
            TransitionSubmission::FinalizeAdvice(request) => {
                planner.finalize_advice(context, request, invocation)
            }
            TransitionSubmission::AdvanceTask(request) => {
                planner.advance_task(context, request, invocation)
            }
            TransitionSubmission::PrepareEvidenceCapture(request) => {
                planner.prepare_evidence_capture(context, request, invocation)
            }
            TransitionSubmission::PrepareWrite(request) => {
                planner.prepare_write(context, request, invocation)
            }
            TransitionSubmission::StageArtifact(request) => {
                planner.stage_artifact(context, request, invocation)
            }
            TransitionSubmission::RecordRun(request) => {
                planner.record_run(context, request, invocation)
            }
            TransitionSubmission::RequestUserAction(request) => {
                planner.request_user_action(context, request, invocation)
            }
            TransitionSubmission::ReconcileChanges(request) => {
                planner.reconcile_changes(context, request, invocation)
            }
            TransitionSubmission::CheckClose(request) => planner.check_close(request, invocation),
            TransitionSubmission::CloseTask(request) => {
                planner.close_task(context, request, invocation)
            }
        }?;
        if let Some(plan) = decode_no_commit_transition_plan(&response)? {
            return Ok(plan);
        }
        Err({
            let code = response
                .response_value
                .pointer("/errors/0/code")
                .and_then(Value::as_str)
                .unwrap_or("method_planner_rejected");
            let message = response
                .response_value
                .pointer("/errors/0/message")
                .and_then(Value::as_str)
                .unwrap_or("no diagnostic message");
            let field = response
                .response_value
                .pointer("/errors/0/details/field")
                .and_then(Value::as_str)
                .unwrap_or("unknown_field");
            CorePipelineError::Invariant {
                detail: format!(
                    "no-commit planner did not reach a valid exact method plan for {} {} at {code}:{field}:{}",
                    action_key.method.as_str(),
                    action_key.semantic_variant.as_str(),
                    message.chars().take(160).collect::<String>()
                ),
            }
        })
    }

    pub(crate) fn project_now(&self, store: &CoreProjectStore) -> CoreResult<UtcTimestamp> {
        self.canonical_project_now(store)
            .map_err(CorePipelineError::from)
    }

    pub(crate) fn project_store_now(&self, store: &CoreProjectStore) -> StoreResult<UtcTimestamp> {
        self.canonical_project_now(store)
    }

    fn canonical_project_now(&self, store: &CoreProjectStore) -> StoreResult<UtcTimestamp> {
        let sampled = self.clock.project_now(store)?;
        sampled
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| StoreError::InvalidInput {
                detail:
                    "Core clock sample must have a canonical four-digit RFC 3339 representation"
                        .to_owned(),
            })?;
        let floor = store.current_clock_floor()?;
        let canonical = std::cmp::max(sampled, floor);
        store.remember_clock_sample(&canonical);
        Ok(canonical)
    }

    pub(crate) fn runtime_home(&self) -> &Path {
        self.runtime_home.as_path()
    }

    pub(crate) fn admitted_runtime_home(&self) -> Option<&CanonicalRuntimeHomePath> {
        self.runtime_home.admitted()
    }

    fn ensure_mutation_context(&self, context: &RuntimeHomeMutationContext<'_>) -> StoreResult<()> {
        match self.admitted_runtime_home() {
            Some(runtime_home) if runtime_home == context.runtime_home() => Ok(()),
            Some(runtime_home) => Err(StoreError::InvalidInput {
                detail: format!(
                    "Core service admitted for {} cannot use mutation context for {}",
                    runtime_home.as_path().display(),
                    context.runtime_home().as_path().display()
                ),
            }),
            None => Err(StoreError::InvalidInput {
                detail:
                    "Core mutation requires a service constructed from Runtime Home mutation admission"
                        .to_owned(),
            }),
        }
    }

    pub(crate) fn durable_id_generator(&self) -> &dyn DurableIdGenerator {
        self.id_generator.as_ref()
    }

    /// Runs the shared envelope, context, freshness, replay, and effect pipeline.
    #[cfg(test)]
    pub(crate) fn execute_pipeline<F, B>(
        &self,
        context: Option<&RuntimeHomeMutationContext<'_>>,
        request: PipelineRequest<F, B>,
    ) -> CoreResult<PipelineResponse>
    where
        F: AssembleMethodResult<B> + Serialize,
        F::Result: Serialize,
    {
        validate_branch_shape(&request.branch, request.envelope.dry_run)?;
        let policy = MethodPolicy::for_branch(
            request.operation_category,
            request.task_requirement,
            &request.branch,
        );
        let semantic_variant = match request.method_name {
            MethodName::UpdateScope => {
                Some(volicord_types::values::WorkflowActionSemanticVariant::CreateCurrentChangeUnit)
            }
            method => {
                volicord_types::values::WorkflowActionSemanticVariant::for_single_variant_method(
                    method,
                )
            }
        };
        let preflight = PipelinePreflightRequest {
            method_name: request.method_name,
            attempted_action_key: semantic_variant
                .and_then(|variant| WorkflowActionKey::new(request.method_name, variant).ok()),
            envelope: request.envelope,
            request_json: request.request_json,
            invocation: request.invocation,
            policy,
        };
        match self.prepare_request(context, preflight)? {
            PipelinePreflightOutcome::Prepared(prepared) => {
                self.execute_prepared_request(*prepared, request.branch)
            }
            PipelinePreflightOutcome::Response(response) => Ok(*response),
        }
    }

    /// Runs the authoritative preflight sequence before method-specific planning.
    pub(crate) fn prepare_request<'mutation>(
        &self,
        context: Option<&'mutation RuntimeHomeMutationContext<'mutation>>,
        request: PipelinePreflightRequest,
    ) -> CoreResult<PipelinePreflightOutcome<'mutation>> {
        let envelope_errors = validate_envelope(&request.envelope, &request.request_json);
        if !envelope_errors.is_empty() {
            return response_outcome_from_rejected(
                rejected_response(request.envelope.dry_run, None, envelope_errors),
                None,
                None,
            );
        }

        if let Some(response) =
            dry_run_policy_rejection(request.method_name, request.envelope.dry_run, None)
        {
            return response_outcome_from_rejected(response, None, None);
        }

        let committed_envelope_errors =
            validate_committed_effect_envelope(&request.envelope, &request.policy);
        if !committed_envelope_errors.is_empty() {
            return response_outcome_from_rejected(
                rejected_response(request.envelope.dry_run, None, committed_envelope_errors),
                None,
                None,
            );
        }

        let request_hash = match request.attempted_action_key.as_ref() {
            Some(action_key) => canonical_request_hash(&(action_key, &request.request_json))?,
            None => canonical_request_hash(&request.request_json)?,
        };

        let store = match open_store_for_policy(
            self,
            context,
            &request.invocation.project_id,
            &request.policy,
        ) {
            Ok(store) => store,
            Err(error) => match CorePipelineError::from(error) {
                CorePipelineError::Store(error) => {
                    return response_outcome_from_rejected(
                        rejected_response(
                            request.envelope.dry_run,
                            None,
                            vec![store_failure_error(error)],
                        ),
                        None,
                        None,
                    );
                }
                error => return Err(error),
            },
        };

        let project_state = match store.project_state() {
            Ok(project_state) => project_state,
            Err(error) => match CorePipelineError::from(error) {
                CorePipelineError::Store(error) => {
                    return response_outcome_from_rejected(
                        rejected_response(
                            request.envelope.dry_run,
                            None,
                            vec![store_failure_error(error)],
                        ),
                        None,
                        None,
                    );
                }
                error => return Err(error),
            },
        };

        let verified_invocation = match derive_verified_invocation(
            &project_state,
            &request.envelope,
            &request.invocation,
            &request.policy,
        ) {
            Ok(context) => context,
            Err(error) => {
                return response_outcome_from_rejected(
                    rejected_response(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        vec![error],
                    ),
                    None,
                    None,
                );
            }
        };

        let replay_response = if matches!(self.execution_mode, CoreExecutionMode::Execute) {
            match replay_preflight_response(
                &store,
                &request,
                &request_hash,
                &project_state,
                &verified_invocation,
            ) {
                Ok(response) => response,
                Err(CorePipelineError::Store(error)) => {
                    return response_outcome_from_rejected(
                        rejected_response(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            vec![store_failure_error(error)],
                        ),
                        Some(verified_invocation),
                        None,
                    );
                }
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        if let Some(replay_response) = replay_response {
            return Ok(PipelinePreflightOutcome::Response(Box::new(
                replay_response,
            )));
        }

        let resolved_task_id = match resolve_task(
            &store,
            &project_state,
            &request.envelope,
            request.policy.task.clone(),
        ) {
            Ok(task_id) => task_id,
            Err(TaskResolutionError::Rejection(error)) => {
                return response_outcome_from_rejected(
                    rejected_response(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        vec![error],
                    ),
                    Some(verified_invocation),
                    None,
                );
            }
            Err(TaskResolutionError::Pipeline(error)) => return Err(error),
        };

        if let Some(freshness_response) = freshness_preflight_response(
            &request,
            &project_state,
            Some(verified_invocation.clone()),
            resolved_task_id.clone(),
        )? {
            return Ok(PipelinePreflightOutcome::Response(Box::new(
                freshness_response,
            )));
        }

        let verified_actor = VerifiedActorContext::from_verified_invocation(&verified_invocation);
        let mut prepared_envelope = request.envelope;
        if request.policy.current_state_default
            && prepared_envelope.dry_run.is_not_requested()
            && prepared_envelope.expected_state_version.is_none()
        {
            prepared_envelope.expected_state_version = Some(project_state.state_version).into();
        }

        let operation_now = match self.project_now(&store) {
            Ok(operation_now) => operation_now,
            Err(CorePipelineError::Store(error)) => {
                return response_outcome_from_rejected(
                    rejected_response(
                        prepared_envelope.dry_run,
                        Some(project_state.state_version),
                        vec![store_failure_error(error)],
                    ),
                    Some(verified_invocation),
                    resolved_task_id,
                )
            }
            Err(error) => return Err(error),
        };

        let admitted_transition =
            match public_method_contract(request.method_name).workflow_action_admission() {
                WorkflowActionAdmissionClass::TaskStateBound => {
                    let attempted_action_key = request.attempted_action_key.ok_or_else(|| {
                        CorePipelineError::InvalidDispatch {
                            detail: format!(
                                "{} requires an exact workflow action key",
                                request.method_name.as_str()
                            ),
                        }
                    })?;
                    let task_id =
                        resolved_task_id
                            .as_ref()
                            .ok_or_else(|| CorePipelineError::Invariant {
                                detail: "task-state-bound admission requires a resolved Task"
                                    .to_owned(),
                            })?;
                    let admission = match crate::workflow_projection::current_transition_admission(
                        &store,
                        &project_state,
                        task_id,
                        attempted_action_key,
                        &operation_now,
                    ) {
                        Ok(admission) => admission,
                        Err(CorePipelineError::Store(error)) => {
                            return response_outcome_from_rejected(
                                rejected_response(
                                    prepared_envelope.dry_run,
                                    Some(project_state.state_version),
                                    vec![store_failure_error(error)],
                                ),
                                Some(verified_invocation),
                                resolved_task_id,
                            )
                        }
                        Err(error) => return Err(error),
                    };
                    let Some(descriptor) = admission.descriptor else {
                        let reason = admission.rejection_reason.ok_or_else(|| {
                            CorePipelineError::Invariant {
                                detail: "rejected transition admission has no typed reason"
                                    .to_owned(),
                            }
                        })?;
                        let code = admission.rejection_code.ok_or_else(|| {
                            CorePipelineError::Invariant {
                                detail: "rejected transition admission has no public error code"
                                    .to_owned(),
                            }
                        })?;
                        let mut response = crate::method_rejection::transition_rejected_response(
                            &prepared_envelope,
                            &project_state,
                            &admission.workflow,
                            code,
                            "exact workflow action is not current",
                            attempted_action_key,
                            reason,
                            admission.retryable,
                            admission.recovery_action_key,
                        )?;
                        response.verified_invocation = Some(verified_invocation);
                        response.resolved_task_id = resolved_task_id;
                        return Ok(PipelinePreflightOutcome::Response(Box::new(response)));
                    };
                    if descriptor.expected_state_version != project_state.state_version
                        || descriptor.fixed_authority_coordinates.task_id() != task_id
                    {
                        return Err(CorePipelineError::Invariant {
                            detail:
                                "admitted transition coordinates contradict preflight authority"
                                    .to_owned(),
                        });
                    }
                    if !request_authority_coordinates_match(
                        request.method_name,
                        &request.request_json,
                        &descriptor.fixed_authority_coordinates,
                    )? {
                        let (code, reason) = request_coordinate_mismatch_rejection(
                            request.method_name,
                            &request.request_json,
                            &descriptor.fixed_authority_coordinates,
                        )?;
                        let mut response = crate::method_rejection::transition_rejected_response(
                            &prepared_envelope,
                            &project_state,
                            &admission.workflow,
                            code,
                            "request authority coordinates do not match the current transition",
                            attempted_action_key,
                            reason,
                            true,
                            Some(descriptor.action_key),
                        )?;
                        response.verified_invocation = Some(verified_invocation);
                        response.resolved_task_id = resolved_task_id;
                        return Ok(PipelinePreflightOutcome::Response(Box::new(response)));
                    }
                    Some(AdmittedTransition {
                        descriptor,
                        workflow: admission.workflow,
                    })
                }
                WorkflowActionAdmissionClass::ReadOnly
                | WorkflowActionAdmissionClass::NotTaskStateBound
                | WorkflowActionAdmissionClass::UserChannelAuthority => {
                    if request.attempted_action_key.is_some()
                        && public_method_contract(request.method_name).workflow_action_admission()
                            != WorkflowActionAdmissionClass::UserChannelAuthority
                    {
                        return Err(CorePipelineError::InvalidDispatch {
                            detail: format!(
                                "{} is not admitted by a task workflow transition",
                                request.method_name.as_str()
                            ),
                        });
                    }
                    None
                }
            };
        Ok(PipelinePreflightOutcome::Prepared(Box::new(
            PreparedRequest {
                method_name: request.method_name,
                envelope: prepared_envelope,
                request_hash,
                store,
                context: VerifiedRequestContext {
                    project_state,
                    verified_invocation,
                    verified_actor,
                    resolved_task_id,
                },
                operation_now,
                admitted_transition,
            },
        )))
    }

    /// Routes a prepared request to the selected storage/effect branch.
    pub(crate) fn execute_prepared_request<F, B>(
        &self,
        mut prepared: PreparedRequest<'_>,
        branch: OwnerPipelineBranch<F, B>,
    ) -> CoreResult<PipelineResponse>
    where
        F: AssembleMethodResult<B> + Serialize,
        F::Result: Serialize,
    {
        validate_branch_shape(&branch, prepared.envelope.dry_run)?;
        validate_method_response_branch(prepared.method_name, prepared.envelope.dry_run, &branch)?;
        validate_admitted_transition_branch(&prepared, &branch)?;
        if let CoreExecutionMode::PlanNoCommit(action_key) = self.execution_mode {
            let planned_fields = match &branch.kind {
                OwnerPipelineBranchKind::ReadOnly { result_fields, .. }
                | OwnerPipelineBranchKind::NoEffectResult { result_fields, .. }
                | OwnerPipelineBranchKind::CommitMutation { result_fields, .. } => {
                    Some(serde_json::to_value(result_fields)?)
                }
                OwnerPipelineBranchKind::DryRunPreview { .. } => None,
            };
            let plan = validate_no_commit_transition_plan(
                action_key,
                &prepared,
                &branch,
                planned_fields.as_ref(),
            )?;
            return Ok(no_commit_pipeline_response(plan));
        }
        let project_state = prepared.context.project_state.clone();
        let verified_invocation = prepared.context.verified_invocation.clone();
        let resolved_task_id = prepared.context.resolved_task_id.clone();

        match branch.kind {
            OwnerPipelineBranchKind::ReadOnly {
                result_fields,
                build_base,
            } => {
                let base = build_base(prepared.envelope.dry_run, project_state.state_version)
                    .map_err(result_base_construction_error)?;
                response_from_value(
                    serde_json::to_value(result_fields.assemble(base))?,
                    Some(verified_invocation),
                    resolved_task_id,
                    false,
                )
            }
            OwnerPipelineBranchKind::NoEffectResult {
                result_fields,
                build_base,
            } => {
                let base = build_base(project_state.state_version);
                response_from_value(
                    serde_json::to_value(result_fields.assemble(base))?,
                    Some(verified_invocation),
                    resolved_task_id,
                    false,
                )
            }
            OwnerPipelineBranchKind::DryRunPreview { dry_run_summary } => response_from_dry_run(
                dry_run_response(Some(project_state.state_version), dry_run_summary),
                Some(verified_invocation),
                resolved_task_id,
            ),
            OwnerPipelineBranchKind::CommitMutation {
                result_fields,
                event_kind,
                mut event_payload,
                task_id: branch_task_id,
                change_unit_id,
                storage_mutations,
                build_base,
            } => {
                if let Some(admission) = prepared.admitted_transition.as_ref() {
                    if event_payload.contains_key("transition") {
                        return Err(CorePipelineError::Invariant {
                            detail: "method event payload shadows the admitted transition"
                                .to_owned(),
                        });
                    }
                    event_payload.insert(
                        "transition".to_owned(),
                        serde_json::to_value(&admission.descriptor)?,
                    );
                }
                let task_id = match branch_task_id.or(resolved_task_id) {
                    Some(task_id) => task_id,
                    None => {
                        return response_from_rejected(
                            rejected_response(
                                prepared.envelope.dry_run,
                                Some(project_state.state_version),
                                vec![no_active_task_error()],
                            ),
                            Some(verified_invocation),
                            None,
                        );
                    }
                };
                let event_id = match allocate_durable_id(
                    self.durable_id_generator(),
                    DurableIdKind::Event,
                    |candidate| {
                        prepared
                            .store
                            .event_id_exists(candidate)
                            .map_err(CorePipelineError::from)
                    },
                ) {
                    Ok(event_id) => event_id,
                    Err(CorePipelineError::Store(error)) => {
                        return response_from_rejected(
                            rejected_response(
                                prepared.envelope.dry_run,
                                Some(project_state.state_version),
                                vec![store_failure_error(error)],
                            ),
                            Some(verified_invocation),
                            Some(task_id),
                        );
                    }
                    Err(error) => return Err(error),
                };
                match commit_mutation(
                    &mut prepared.store,
                    CommitPipelineArgs {
                        envelope: &prepared.envelope,
                        method_name: prepared.method_name,
                        request_hash: &prepared.request_hash,
                        event_id,
                        result_fields,
                        build_base,
                        event_kind,
                        event_payload,
                        change_unit_id,
                        storage_mutations,
                        task_id: &task_id,
                        verified_invocation: verified_invocation.clone(),
                        clock_floor: &prepared.operation_now,
                        include_live_storage_time: self.clock.include_live_storage_time_at_commit(),
                        transition_descriptor: prepared
                            .admitted_transition
                            .as_ref()
                            .map(|admission| admission.descriptor.clone()),
                    },
                ) {
                    Ok(response) => Ok(response),
                    Err(CorePipelineError::Store(error)) => response_from_rejected(
                        rejected_response(
                            prepared.envelope.dry_run,
                            Some(project_state.state_version),
                            vec![store_failure_error(error)],
                        ),
                        Some(verified_invocation),
                        Some(task_id),
                    ),
                    Err(error) => Err(error),
                }
            }
        }
    }

    /// Completes a method-owned response produced after exact transition
    /// admission. During no-commit planning, an exact method-owned rejection is
    /// a valid no-effect plan: the advertised transition remains current while
    /// the method's present policy or semantic state blocks an effect.
    pub(crate) fn complete_prepared_response(
        &self,
        response: PipelineResponse,
        prepared: &PreparedRequest<'_>,
    ) -> CoreResult<PipelineResponse> {
        let response = response.with_prepared_context(prepared);
        let CoreExecutionMode::PlanNoCommit(action_key) = self.execution_mode else {
            return Ok(response);
        };
        if response
            .response_value
            .pointer("/base/response_kind")
            .and_then(Value::as_str)
            != Some("rejected")
        {
            return Ok(response);
        }
        let rejected =
            serde_json::from_value::<ToolRejectedResponse>(response.response_value.clone())?;
        if rejected.errors().is_empty() {
            return Err(CorePipelineError::Invariant {
                detail: "no-commit method-owned rejection has no typed error".to_owned(),
            });
        }
        let admission =
            prepared
                .admitted_transition
                .as_ref()
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "no-commit policy rejection has no exact transition admission"
                        .to_owned(),
                })?;
        if admission.descriptor.action_key != action_key
            || admission.descriptor.expected_state_version
                != prepared.context.project_state.state_version
            || rejected.base().dry_run_intent() != DryRunIntent::NotRequested
            || rejected.base().state_version() != Some(prepared.context.project_state.state_version)
            || !rejected.base().events().is_empty()
            || response.operation_result_ref.is_some()
            || response.replayed
        {
            return Err(CorePipelineError::Invariant {
                detail: "no-commit policy rejection contradicts its admitted no-effect plan"
                    .to_owned(),
            });
        }
        validate_planned_expected_result(
            true,
            None,
            &admission.workflow,
            admission.descriptor.expected_result_state,
        )?;
        Ok(no_commit_pipeline_response(NoCommitTransitionPlan {
            action_key,
            basis_state_version: prepared.context.project_state.state_version,
            effect_class: admission.descriptor.effect_class,
            expected_result_state: admission.descriptor.expected_result_state,
        }))
    }

    pub(crate) fn complete_stage_artifact_no_commit(
        &self,
        prepared: &PreparedRequest<'_>,
    ) -> CoreResult<Option<PipelineResponse>> {
        let CoreExecutionMode::PlanNoCommit(action_key) = self.execution_mode else {
            return Ok(None);
        };
        let admission =
            prepared
                .admitted_transition
                .as_ref()
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "no-commit artifact planner lost exact transition admission".to_owned(),
                })?;
        if admission.descriptor.action_key != action_key
            || admission.descriptor.effect_class != WorkflowTransitionEffectClass::ArtifactStaging
        {
            return Err(CorePipelineError::Invariant {
                detail: "planned artifact branch contradicts current transition effect".to_owned(),
            });
        }
        if !workflow_matches_expected_result(
            &admission.workflow,
            admission.descriptor.expected_result_state,
        ) {
            return Err(CorePipelineError::Invariant {
                detail: "planned artifact authority contradicts expected-result family".to_owned(),
            });
        }
        Ok(Some(no_commit_pipeline_response(NoCommitTransitionPlan {
            action_key,
            basis_state_version: prepared.context.project_state.state_version,
            effect_class: admission.descriptor.effect_class,
            expected_result_state: admission.descriptor.expected_result_state,
        })))
    }
}

fn open_store_for_policy<'mutation>(
    service: &CoreService,
    context: Option<&'mutation RuntimeHomeMutationContext<'mutation>>,
    project_id: &ProjectId,
    policy: &MethodPolicy,
) -> Result<CoreProjectStore<'mutation>, StoreError> {
    if policy.effect == MethodEffectPolicy::ReadOnly {
        CoreProjectStore::open_read_only(service.runtime_home(), project_id)
    } else if matches!(service.execution_mode, CoreExecutionMode::PlanNoCommit(_)) {
        let context = context.ok_or_else(|| StoreError::InvalidInput {
            detail: "Core no-commit planning requires Runtime Home mutation admission".to_owned(),
        })?;
        service.ensure_mutation_context(context)?;
        CoreProjectStore::open_read_only(service.runtime_home(), project_id)
    } else {
        let context = context.ok_or_else(|| StoreError::InvalidInput {
            detail: "Core mutation requires Runtime Home mutation admission".to_owned(),
        })?;
        service.ensure_mutation_context(context)?;
        CoreProjectStore::open_for_mutation(context, project_id)
    }
}

fn validate_no_commit_transition_plan<F, B>(
    action_key: WorkflowActionKey,
    prepared: &PreparedRequest<'_>,
    branch: &OwnerPipelineBranch<F, B>,
    planned_fields: Option<&Value>,
) -> CoreResult<NoCommitTransitionPlan> {
    let admission =
        prepared
            .admitted_transition
            .as_ref()
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "no-commit planner reached a method without exact transition admission"
                    .to_owned(),
            })?;
    if admission.descriptor.action_key != action_key {
        return Err(CorePipelineError::Invariant {
            detail: "no-commit planner admitted a different current action key".to_owned(),
        });
    }
    match &branch.kind {
        OwnerPipelineBranchKind::CommitMutation {
            storage_mutations, ..
        } if !volicord_store::core_pipeline::transition_effect_matches_mutations(
            action_key,
            admission.descriptor.effect_class,
            storage_mutations,
        ) =>
        {
            return Err(CorePipelineError::Invariant {
                detail: "planned mutation batch contradicts transition effect class".to_owned(),
            });
        }
        OwnerPipelineBranchKind::DryRunPreview { .. } => {
            return Err(CorePipelineError::Invariant {
                detail: "no-commit transition planning cannot stop at a dry-run preview".to_owned(),
            });
        }
        OwnerPipelineBranchKind::ReadOnly { .. }
        | OwnerPipelineBranchKind::NoEffectResult { .. }
        | OwnerPipelineBranchKind::CommitMutation { .. } => {}
    }

    let projected_workflow = planned_fields
        .and_then(|fields| {
            fields
                .pointer("/state/workflow")
                .or_else(|| fields.pointer("/workflow"))
        })
        .map(|workflow| serde_json::from_value::<WorkflowProjection>(workflow.clone()))
        .transpose()?
        .unwrap_or_else(|| admission.workflow.clone());
    let no_effect_result = matches!(&branch.kind, OwnerPipelineBranchKind::NoEffectResult { .. });
    validate_planned_expected_result(
        no_effect_result,
        planned_fields,
        &projected_workflow,
        admission.descriptor.expected_result_state,
    )?;
    Ok(NoCommitTransitionPlan {
        action_key,
        basis_state_version: prepared.context.project_state.state_version,
        effect_class: admission.descriptor.effect_class,
        expected_result_state: admission.descriptor.expected_result_state,
    })
}

fn validate_planned_expected_result(
    no_effect_result: bool,
    planned_fields: Option<&Value>,
    workflow: &WorkflowProjection,
    expected: WorkflowExpectedResultState,
) -> CoreResult<()> {
    if no_effect_result || planned_fields_match_expected_result(planned_fields, workflow, expected)
    {
        return Ok(());
    }
    let observed_phase = planned_fields
        .and_then(|fields| fields.pointer("/state/lifecycle/lifecycle_phase"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    Err(CorePipelineError::Invariant {
        detail: format!(
            "planned authority state contradicts transition expected-result family (workflow={:?}, lifecycle={observed_phase})",
            workflow.kind()
        ),
    })
}

fn workflow_matches_expected_result(
    workflow: &WorkflowProjection,
    expected: WorkflowExpectedResultState,
) -> bool {
    match expected {
        WorkflowExpectedResultState::ReevaluateCurrentAuthority => {
            workflow.kind() != WorkflowStateKind::Terminal
        }
        WorkflowExpectedResultState::AwaitingUserAction => {
            workflow.kind() == WorkflowStateKind::AwaitingUserAction
        }
        WorkflowExpectedResultState::Implementation => {
            workflow.kind() == WorkflowStateKind::Implementation
        }
        WorkflowExpectedResultState::CloseReview => workflow_close_basis_present(workflow),
        WorkflowExpectedResultState::Terminal => workflow.kind() == WorkflowStateKind::Terminal,
    }
}

fn planned_fields_match_expected_result(
    planned_fields: Option<&Value>,
    workflow: &WorkflowProjection,
    expected: WorkflowExpectedResultState,
) -> bool {
    let state = planned_fields.and_then(|fields| fields.get("state"));
    match expected {
        WorkflowExpectedResultState::Terminal => {
            state
                .and_then(|state| state.pointer("/lifecycle/lifecycle_phase"))
                .and_then(Value::as_str)
                .is_some_and(|phase| matches!(phase, "completed" | "cancelled" | "superseded"))
                || workflow.kind() == WorkflowStateKind::Terminal
        }
        WorkflowExpectedResultState::Implementation => {
            state
                .and_then(|state| state.get("work_phase"))
                .and_then(Value::as_str)
                .is_some_and(|phase| phase == "implementation")
                || workflow.kind() == WorkflowStateKind::Implementation
        }
        WorkflowExpectedResultState::AwaitingUserAction => {
            state
                .and_then(|state| state.get("pending_user_action_summaries"))
                .and_then(Value::as_array)
                .is_some_and(|pending| !pending.is_empty())
                || workflow.kind() == WorkflowStateKind::AwaitingUserAction
        }
        WorkflowExpectedResultState::CloseReview => {
            planned_fields
                .and_then(|fields| fields.get("current_close_basis"))
                .is_some_and(|basis| !basis.is_null())
                || workflow_close_basis_present(workflow)
        }
        WorkflowExpectedResultState::ReevaluateCurrentAuthority => state
            .and_then(|state| state.pointer("/lifecycle/lifecycle_phase"))
            .and_then(Value::as_str)
            .is_none_or(|phase| !matches!(phase, "completed" | "cancelled" | "superseded")),
    }
}

fn workflow_close_basis_present(workflow: &WorkflowProjection) -> bool {
    match workflow {
        WorkflowProjection::NoActiveTask {
            close_readiness, ..
        }
        | WorkflowProjection::ShapingRequired {
            close_readiness, ..
        }
        | WorkflowProjection::AwaitingUserAction {
            close_readiness, ..
        }
        | WorkflowProjection::DecisionRecoveryRequired {
            close_readiness, ..
        }
        | WorkflowProjection::ReadyToApplyDecisions {
            close_readiness, ..
        }
        | WorkflowProjection::ReadyForChangeUnit {
            close_readiness, ..
        }
        | WorkflowProjection::ReadyToFinalizeAdvice {
            close_readiness, ..
        }
        | WorkflowProjection::ReadyForImplementation {
            close_readiness, ..
        }
        | WorkflowProjection::Implementation {
            close_readiness, ..
        }
        | WorkflowProjection::CloseReview {
            close_readiness, ..
        }
        | WorkflowProjection::Terminal {
            close_readiness, ..
        } => close_readiness.current_close_basis_present,
    }
}

fn no_commit_pipeline_response(plan: NoCommitTransitionPlan) -> PipelineResponse {
    let response_value = serde_json::to_value(NoCommitPipelineMarker {
        no_commit_transition_plan: plan,
    })
    .expect("typed no-commit marker serialization must succeed");
    PipelineResponse {
        response_json: "null".to_owned(),
        response_value,
        operation_result_ref: None,
        verified_invocation: None,
        resolved_task_id: None,
        replayed: false,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoCommitPipelineMarker {
    no_commit_transition_plan: NoCommitTransitionPlan,
}

fn decode_no_commit_transition_plan(
    response: &PipelineResponse,
) -> CoreResult<Option<NoCommitTransitionPlan>> {
    if response
        .response_value
        .get("no_commit_transition_plan")
        .is_none()
    {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_value::<NoCommitPipelineMarker>(response.response_value.clone())?
            .no_commit_transition_plan,
    ))
}

/// Injectable UTC clock used by Core authority checks.
pub trait Clock: fmt::Debug + Send + Sync {
    /// Returns the current UTC timestamp.
    fn now(&self) -> DateTime<Utc>;

    /// Samples the current UTC timestamp for one opened project Store.
    fn project_now(&self, _store: &CoreProjectStore) -> StoreResult<UtcTimestamp> {
        Ok(UtcTimestamp::from_datetime(self.now()))
    }

    /// Whether mutation commit must also include SQLite's live UTC candidate.
    /// Injected clocks replace that source unless they explicitly opt in.
    fn include_live_storage_time_at_commit(&self) -> bool {
        false
    }
}

/// Production UTC clock backed by the system clock.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        canonical_core_utc_timestamp(DateTime::<Utc>::from(SystemTime::now()))
    }

    fn project_now(&self, store: &CoreProjectStore) -> StoreResult<UtcTimestamp> {
        store.current_timestamp()
    }

    fn include_live_storage_time_at_commit(&self) -> bool {
        true
    }
}

fn canonical_core_utc_timestamp(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    let millisecond_nanos = timestamp.timestamp_subsec_millis() * 1_000_000;
    timestamp
        .with_nanosecond(millisecond_nanos)
        .expect("millisecond precision is always a valid UTC nanosecond")
}

fn result_base_construction_error(error: ResultBaseConstructionError) -> CorePipelineError {
    CorePipelineError::InvalidDispatch {
        detail: error.detail().to_owned(),
    }
}

/// Builds a rejected response and applies public error precedence.
pub fn rejected_response(
    dry_run: DryRunIntent,
    state_version: Option<u64>,
    mut errors: Vec<ToolError>,
) -> ToolRejectedResponse {
    errors.sort_by_key(|error| error_precedence(error.code()));
    ToolRejectedResponse::new(
        dry_run,
        state_version,
        GuaranteeDisclosure::authority_record(),
        errors,
    )
}

/// Builds the canonical rejection for a decoded dry-run request that the
/// method contract forbids.
pub(crate) fn dry_run_policy_rejection(
    method_name: MethodName,
    dry_run: DryRunIntent,
    state_version: Option<u64>,
) -> Option<ToolRejectedResponse> {
    (public_method_contract(method_name)
        .dry_run_policy()
        .route(dry_run)
        == DryRunRequestRoute::Rejected)
        .then(|| {
            rejected_response(
                dry_run,
                state_version,
                vec![validation_error(
                    "dry_run",
                    "dry_run=true is forbidden for this method",
                )],
            )
        })
}

/// Builds a dry-run preview response.
pub fn dry_run_response(
    state_version: Option<u64>,
    dry_run_summary: DryRunSummary,
) -> ToolDryRunResponse {
    ToolDryRunResponse::new(
        state_version,
        GuaranteeDisclosure::authority_record(),
        dry_run_summary,
    )
}

/// Builds a public API error item.
pub fn tool_error(
    code: ErrorCode,
    message: impl Into<String>,
    retryable: bool,
    details: Option<JsonObject>,
) -> ToolError {
    ToolError::new(code, message, retryable, details)
}

fn validate_envelope(envelope: &ToolEnvelope, request_json: &Value) -> Vec<ToolError> {
    let mut errors = Vec::new();
    if !request_json.is_object() {
        errors.push(validation_error(
            "request_json",
            "request must be a JSON object",
        ));
    }
    if envelope.project_id.as_str().trim().is_empty() {
        errors.push(validation_error(
            "project_id",
            "project_id must not be empty",
        ));
    }
    if let Some(task_id) = envelope.task_id.as_ref() {
        if task_id.as_str().trim().is_empty() {
            errors.push(validation_error("task_id", "task_id must not be empty"));
        }
    }
    if envelope.request_id.as_str().trim().is_empty() {
        errors.push(validation_error(
            "request_id",
            "request_id must not be empty",
        ));
    }
    if let Some(idempotency_key) = envelope.idempotency_key.as_ref() {
        if idempotency_key.as_str().trim().is_empty() {
            errors.push(validation_error(
                "idempotency_key",
                "idempotency_key must not be empty",
            ));
        }
    }
    errors
}

fn validate_committed_effect_envelope(
    envelope: &ToolEnvelope,
    policy: &MethodPolicy,
) -> Vec<ToolError> {
    if envelope.dry_run.is_requested() || policy.effect != MethodEffectPolicy::CoreMutation {
        return Vec::new();
    }
    if envelope.idempotency_key.is_none() {
        return vec![validation_error(
            "idempotency_key",
            "committed mutations require idempotency_key",
        )];
    }
    if envelope.expected_state_version.is_none() && !policy.current_state_default {
        return vec![validation_error(
            "expected_state_version",
            "committed mutations require expected_state_version",
        )];
    }
    Vec::new()
}

fn validate_branch_shape<F, B>(
    branch: &OwnerPipelineBranch<F, B>,
    dry_run: DryRunIntent,
) -> CoreResult<()> {
    match (&branch.kind, dry_run) {
        (OwnerPipelineBranchKind::ReadOnly { .. }, _) => Ok(()),
        (OwnerPipelineBranchKind::NoEffectResult { .. }, DryRunIntent::NotRequested) => Ok(()),
        (OwnerPipelineBranchKind::NoEffectResult { .. }, DryRunIntent::Requested) => {
            Err(CorePipelineError::InvalidDispatch {
                detail: "no-effect result branch requires ToolEnvelope.dry_run=false".to_owned(),
            })
        }
        (OwnerPipelineBranchKind::DryRunPreview { .. }, DryRunIntent::Requested) => Ok(()),
        (OwnerPipelineBranchKind::DryRunPreview { .. }, DryRunIntent::NotRequested) => {
            Err(CorePipelineError::InvalidDispatch {
                detail: "dry-run preview branch requires ToolEnvelope.dry_run=true".to_owned(),
            })
        }
        (
            OwnerPipelineBranchKind::CommitMutation { event_kind, .. },
            DryRunIntent::NotRequested,
        ) => {
            if event_kind.trim().is_empty() {
                return Err(CorePipelineError::InvalidDispatch {
                    detail: "committed mutation event_kind must not be empty".to_owned(),
                });
            }
            Ok(())
        }
        (OwnerPipelineBranchKind::CommitMutation { .. }, DryRunIntent::Requested) => {
            Err(CorePipelineError::InvalidDispatch {
                detail: "commit branch requires ToolEnvelope.dry_run=false".to_owned(),
            })
        }
    }
}

fn validate_method_response_branch<F, B>(
    method_name: MethodName,
    dry_run: DryRunIntent,
    branch: &OwnerPipelineBranch<F, B>,
) -> CoreResult<()> {
    let (response_branch, result_effect) = match &branch.kind {
        OwnerPipelineBranchKind::ReadOnly { .. } => (
            MethodResponseBranch::Result,
            Some(ResultEffectContract::ReadOnly),
        ),
        OwnerPipelineBranchKind::NoEffectResult { .. } => (
            MethodResponseBranch::Result,
            Some(ResultEffectContract::NoEffect),
        ),
        OwnerPipelineBranchKind::CommitMutation { .. } => (
            MethodResponseBranch::Result,
            Some(ResultEffectContract::CoreCommitted),
        ),
        OwnerPipelineBranchKind::DryRunPreview { .. } => (MethodResponseBranch::DryRun, None),
    };
    let contract = public_method_contract(method_name);
    let policy_route = contract.dry_run_policy().route(dry_run);
    let route_matches_branch = matches!(
        (policy_route, response_branch),
        (DryRunRequestRoute::Result, MethodResponseBranch::Result)
            | (DryRunRequestRoute::Preview, MethodResponseBranch::DryRun)
    );
    let effect_matches = result_effect.is_none_or(|effect| contract.supports_result_effect(effect));
    if contract.supports_response_branch(response_branch) && route_matches_branch && effect_matches
    {
        return Ok(());
    }
    Err(CorePipelineError::InvalidDispatch {
        detail: format!(
            "{} cannot produce the {response_branch:?} response branch for dry_run={}",
            method_name.as_str(),
            dry_run.as_wire_bool()
        ),
    })
}

fn validate_admitted_transition_branch<F, B>(
    prepared: &PreparedRequest<'_>,
    branch: &OwnerPipelineBranch<F, B>,
) -> CoreResult<()> {
    let Some(admission) = prepared.admitted_transition.as_ref() else {
        return Ok(());
    };
    if admission.descriptor.action_key.method != prepared.method_name {
        return Err(CorePipelineError::Invariant {
            detail: "prepared method does not consume its admitted transition".to_owned(),
        });
    }
    match &branch.kind {
        OwnerPipelineBranchKind::ReadOnly { .. } => Ok(()),
        OwnerPipelineBranchKind::DryRunPreview { .. }
        | OwnerPipelineBranchKind::NoEffectResult { .. } => Ok(()),
        OwnerPipelineBranchKind::CommitMutation {
            storage_mutations, ..
        } if volicord_store::core_pipeline::transition_effect_matches_mutations(
            admission.descriptor.action_key,
            admission.descriptor.effect_class,
            storage_mutations,
        ) =>
        {
            Ok(())
        }
        _ => Err(CorePipelineError::Invariant {
            detail: "method effect branch contradicts its admitted transition descriptor"
                .to_owned(),
        }),
    }
}

#[derive(Deserialize)]
struct SubmittedTaskCoordinates {
    task_id: TaskId,
}

#[derive(Deserialize)]
struct SubmittedRecordShapingCoordinates {
    task_id: TaskId,
    checkpoint_operation: SubmittedCheckpointOperation,
    scope_revision: u64,
    baseline_ref: RequiredNullable<BaselineRef>,
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum SubmittedCheckpointOperation {
    CreateInitial,
    ReplaceCurrent {
        expected_current_checkpoint_id: ShapingCheckpointId,
        retired_non_authorizing_request_refs: Vec<StateRecordRef>,
        carry_forward_application_refs: Vec<StateRecordRef>,
        stale_authority_actions: Vec<SubmittedStaleAuthorityAction>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum SubmittedStaleAuthorityAction {
    Retire {
        stale_application_ref: StateRecordRef,
    },
    Reauthorize {
        stale_application_ref: StateRecordRef,
    },
}

#[derive(Deserialize)]
struct SubmittedUpdateScopeCoordinates {
    task_id: TaskId,
    change_unit: SubmittedChangeUnitCoordinates,
}

#[derive(Deserialize)]
struct SubmittedChangeUnitCoordinates {
    operation: ChangeUnitOperation,
}

#[derive(Deserialize)]
struct SubmittedCheckpointApplicationCoordinates {
    task_id: TaskId,
    shaping_checkpoint_id: ShapingCheckpointId,
    change_unit_id: ChangeUnitId,
    scope_revision: u64,
    baseline_ref: BaselineRef,
    user_action_resolution_ids: Vec<UserActionResolutionId>,
}

#[derive(Deserialize)]
struct SubmittedExecutionCoordinates {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    baseline_ref: BaselineRef,
}

#[derive(Deserialize)]
struct SubmittedPrepareWriteCoordinates {
    task_id: RequiredNullable<TaskId>,
    change_unit_id: RequiredNullable<ChangeUnitId>,
}

#[derive(Deserialize)]
struct SubmittedRecordRunCoordinates {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    baseline_ref: BaselineRef,
    kind: RunKind,
}

#[derive(Deserialize)]
struct SubmittedRequestUserActionCoordinates {
    task_id: TaskId,
    change_unit_id: RequiredNullable<ChangeUnitId>,
}

#[derive(Deserialize)]
struct SubmittedResolveUserActionCoordinates {
    user_action_request_id: UserActionRequestId,
}

fn request_authority_coordinates_match(
    method: MethodName,
    request_json: &Value,
    coordinates: &WorkflowActionAuthorityCoordinates,
) -> CoreResult<bool> {
    fn decode<T: serde::de::DeserializeOwned>(request_json: &Value) -> CoreResult<T> {
        serde_json::from_value(request_json.clone()).map_err(|error| CorePipelineError::Invariant {
            detail: format!("typed method request failed transition-coordinate decoding: {error}"),
        })
    }

    fn normalized_refs(refs: &[StateRecordRef]) -> Vec<StateRecordRef> {
        let mut refs = refs.to_vec();
        refs.sort();
        refs.dedup();
        refs
    }

    fn normalized_ids<T: Ord + Clone>(ids: &[T]) -> Vec<T> {
        let mut ids = ids.to_vec();
        ids.sort();
        ids.dedup();
        ids
    }

    let matches = match (method, coordinates) {
        (
            MethodName::RecordShapingCheckpoint,
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                task_id,
                checkpoint_operation,
                scope_revision,
                baseline_ref,
            },
        ) => {
            let request: SubmittedRecordShapingCoordinates = decode(request_json)?;
            let operation_matches = match (&request.checkpoint_operation, checkpoint_operation) {
                (
                    SubmittedCheckpointOperation::CreateInitial,
                    WorkflowCheckpointActionCoordinates::CreateInitial,
                ) => true,
                (
                    SubmittedCheckpointOperation::ReplaceCurrent {
                        expected_current_checkpoint_id,
                        retired_non_authorizing_request_refs,
                        carry_forward_application_refs,
                        stale_authority_actions,
                    },
                    WorkflowCheckpointActionCoordinates::ReplaceCurrent {
                        current_checkpoint_ref,
                        retired_non_authorizing_request_refs: current_retired_refs,
                        carry_forward_application_refs: current_carry_refs,
                        stale_application_refs,
                        ..
                    },
                ) => {
                    let requested_stale_refs = stale_authority_actions
                        .iter()
                        .map(|action| match action {
                            SubmittedStaleAuthorityAction::Retire {
                                stale_application_ref,
                            }
                            | SubmittedStaleAuthorityAction::Reauthorize {
                                stale_application_ref,
                            } => stale_application_ref.clone(),
                        })
                        .collect::<Vec<_>>();
                    expected_current_checkpoint_id.as_str()
                        == current_checkpoint_ref.record_id.as_str()
                        && normalized_refs(retired_non_authorizing_request_refs)
                            == normalized_refs(current_retired_refs)
                        && normalized_refs(carry_forward_application_refs)
                            == normalized_refs(current_carry_refs)
                        && normalized_refs(&requested_stale_refs)
                            == normalized_refs(stale_application_refs)
                }
                _ => false,
            };
            request.task_id == *task_id
                && request.scope_revision == *scope_revision
                && request.baseline_ref == *baseline_ref
                && operation_matches
        }
        (
            MethodName::UpdateScope,
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id,
                selected_change_unit_operation,
                ..
            },
        ) => {
            let request: SubmittedUpdateScopeCoordinates = decode(request_json)?;
            request.task_id == *task_id
                && request.change_unit.operation == *selected_change_unit_operation
        }
        (
            MethodName::FinalizeAdvice,
            WorkflowActionAuthorityCoordinates::FinalizeAdvice {
                task_id,
                shaping_checkpoint_id,
                change_unit_id,
                scope_revision,
                baseline_ref,
                user_action_resolution_ids,
            },
        ) => {
            let request: SubmittedCheckpointApplicationCoordinates = decode(request_json)?;
            request.task_id == *task_id
                && request.shaping_checkpoint_id == *shaping_checkpoint_id
                && request.change_unit_id == *change_unit_id
                && request.scope_revision == *scope_revision
                && baseline_ref.as_ref() == Some(&request.baseline_ref)
                && normalized_ids(&request.user_action_resolution_ids)
                    == normalized_ids(user_action_resolution_ids)
        }
        (
            MethodName::AdvanceTask,
            WorkflowActionAuthorityCoordinates::AdvanceTask {
                task_id,
                shaping_checkpoint_id,
                change_unit_id,
                scope_revision,
                baseline_ref,
                user_action_resolution_ids,
            },
        ) => {
            let request: SubmittedCheckpointApplicationCoordinates = decode(request_json)?;
            request.task_id == *task_id
                && request.shaping_checkpoint_id == *shaping_checkpoint_id
                && request.change_unit_id == *change_unit_id
                && request.scope_revision == *scope_revision
                && baseline_ref.as_ref() == Some(&request.baseline_ref)
                && normalized_ids(&request.user_action_resolution_ids)
                    == normalized_ids(user_action_resolution_ids)
        }
        (
            MethodName::PrepareEvidenceCapture,
            WorkflowActionAuthorityCoordinates::PrepareEvidenceCapture {
                task_id,
                change_unit_id,
                baseline_ref,
            },
        ) => {
            let request: SubmittedExecutionCoordinates = decode(request_json)?;
            request.task_id == *task_id
                && request.change_unit_id == *change_unit_id
                && request.baseline_ref == *baseline_ref
        }
        (
            MethodName::PrepareWrite,
            WorkflowActionAuthorityCoordinates::PrepareWrite {
                task_id,
                change_unit_id,
                ..
            },
        ) => {
            let request: SubmittedPrepareWriteCoordinates = decode(request_json)?;
            request
                .task_id
                .as_ref()
                .is_none_or(|value| value == task_id)
                && request
                    .change_unit_id
                    .as_ref()
                    .is_none_or(|value| value == change_unit_id)
        }
        (
            MethodName::StageArtifact,
            WorkflowActionAuthorityCoordinates::StageArtifact { task_id },
        ) => {
            let request: SubmittedTaskCoordinates = decode(request_json)?;
            request.task_id == *task_id
        }
        (
            MethodName::RecordRun,
            WorkflowActionAuthorityCoordinates::RecordRun {
                task_id,
                change_unit_id,
                baseline_ref,
                run_kind,
            },
        ) => {
            let request: SubmittedRecordRunCoordinates = decode(request_json)?;
            request.task_id == *task_id
                && request.change_unit_id == *change_unit_id
                && request.baseline_ref == *baseline_ref
                && request.kind == *run_kind
        }
        (
            MethodName::RequestUserAction,
            WorkflowActionAuthorityCoordinates::RequestUserAction {
                task_id,
                change_unit_id,
            },
        ) => {
            let request: SubmittedRequestUserActionCoordinates = decode(request_json)?;
            request.task_id == *task_id
                && request
                    .change_unit_id
                    .as_ref()
                    .is_none_or(|value| change_unit_id.as_ref() == Some(value))
        }
        (
            MethodName::ResolveUserAction,
            WorkflowActionAuthorityCoordinates::ResolveUserAction {
                user_action_request_refs,
                ..
            },
        ) => {
            let request: SubmittedResolveUserActionCoordinates = decode(request_json)?;
            user_action_request_refs.iter().any(|reference| {
                reference.record_kind == volicord_types::values::StateRecordKind::UserActionRequest
                    && reference.record_id.as_str() == request.user_action_request_id.as_str()
            })
        }
        (
            MethodName::ReconcileChanges,
            WorkflowActionAuthorityCoordinates::ReconcileChanges { task_id },
        ) => {
            let request: SubmittedTaskCoordinates = decode(request_json)?;
            request.task_id == *task_id
        }
        (MethodName::CheckClose, WorkflowActionAuthorityCoordinates::CheckClose { task_id }) => {
            let request: SubmittedTaskCoordinates = decode(request_json)?;
            request.task_id == *task_id
        }
        (MethodName::CloseTask, WorkflowActionAuthorityCoordinates::CloseTask { task_id }) => {
            let request: SubmittedTaskCoordinates = decode(request_json)?;
            request.task_id == *task_id
        }
        _ => false,
    };
    Ok(matches)
}

fn request_coordinate_mismatch_rejection(
    method: MethodName,
    request_json: &Value,
    coordinates: &WorkflowActionAuthorityCoordinates,
) -> CoreResult<(ErrorCode, volicord_types::values::TransitionRejectionReason)> {
    use volicord_types::values::TransitionRejectionReason;

    fn decode<T: serde::de::DeserializeOwned>(request_json: &Value) -> CoreResult<T> {
        serde_json::from_value(request_json.clone()).map_err(|error| CorePipelineError::Invariant {
            detail: format!("typed method request failed mismatch classification: {error}"),
        })
    }

    let rejection = match (method, coordinates) {
        (
            MethodName::RecordShapingCheckpoint,
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                checkpoint_operation:
                    WorkflowCheckpointActionCoordinates::ReplaceCurrent {
                        current_checkpoint_ref,
                        ..
                    },
                ..
            },
        ) => {
            let request: SubmittedRecordShapingCoordinates = decode(request_json)?;
            if matches!(
                request.checkpoint_operation,
                SubmittedCheckpointOperation::ReplaceCurrent {
                    expected_current_checkpoint_id,
                    ..
                } if expected_current_checkpoint_id.as_str()
                    != current_checkpoint_ref.record_id.as_str()
            ) {
                (
                    ErrorCode::ShapingCheckpointStale,
                    TransitionRejectionReason::CheckpointStale,
                )
            } else {
                (
                    ErrorCode::WorkflowActionNotAllowed,
                    TransitionRejectionReason::AuthorityBasisMismatch,
                )
            }
        }
        (
            MethodName::FinalizeAdvice,
            WorkflowActionAuthorityCoordinates::FinalizeAdvice {
                shaping_checkpoint_id,
                change_unit_id,
                baseline_ref,
                ..
            },
        )
        | (
            MethodName::AdvanceTask,
            WorkflowActionAuthorityCoordinates::AdvanceTask {
                shaping_checkpoint_id,
                change_unit_id,
                baseline_ref,
                ..
            },
        ) => {
            let request: SubmittedCheckpointApplicationCoordinates = decode(request_json)?;
            if request.shaping_checkpoint_id != *shaping_checkpoint_id {
                (
                    ErrorCode::ShapingCheckpointStale,
                    TransitionRejectionReason::CheckpointStale,
                )
            } else if request.change_unit_id != *change_unit_id {
                (
                    ErrorCode::ChangeUnitStale,
                    TransitionRejectionReason::ChangeUnitStale,
                )
            } else if baseline_ref.as_ref() != Some(&request.baseline_ref) {
                (
                    ErrorCode::WorkspaceBasisStale,
                    TransitionRejectionReason::WorkspaceBasisStale,
                )
            } else {
                (
                    ErrorCode::WorkflowActionNotAllowed,
                    TransitionRejectionReason::AuthorityBasisMismatch,
                )
            }
        }
        (
            MethodName::PrepareEvidenceCapture,
            WorkflowActionAuthorityCoordinates::PrepareEvidenceCapture {
                change_unit_id,
                baseline_ref,
                ..
            },
        ) => {
            let request: SubmittedExecutionCoordinates = decode(request_json)?;
            if request.change_unit_id != *change_unit_id {
                (
                    ErrorCode::ChangeUnitStale,
                    TransitionRejectionReason::ChangeUnitStale,
                )
            } else if request.baseline_ref != *baseline_ref {
                (
                    ErrorCode::WorkspaceBasisStale,
                    TransitionRejectionReason::WorkspaceBasisStale,
                )
            } else {
                (
                    ErrorCode::WorkflowActionNotAllowed,
                    TransitionRejectionReason::AuthorityBasisMismatch,
                )
            }
        }
        (
            MethodName::RecordRun,
            WorkflowActionAuthorityCoordinates::RecordRun {
                change_unit_id,
                baseline_ref,
                ..
            },
        ) => {
            let request: SubmittedRecordRunCoordinates = decode(request_json)?;
            if request.change_unit_id != *change_unit_id {
                (
                    ErrorCode::ChangeUnitStale,
                    TransitionRejectionReason::ChangeUnitStale,
                )
            } else if request.baseline_ref != *baseline_ref {
                (
                    ErrorCode::WorkspaceBasisStale,
                    TransitionRejectionReason::WorkspaceBasisStale,
                )
            } else {
                (
                    ErrorCode::RunKindIncompatible,
                    TransitionRejectionReason::AuthorityBasisMismatch,
                )
            }
        }
        _ => (
            ErrorCode::WorkflowActionNotAllowed,
            TransitionRejectionReason::AuthorityBasisMismatch,
        ),
    };
    Ok(rejection)
}

fn replay_preflight_response(
    store: &CoreProjectStore,
    request: &PipelinePreflightRequest,
    request_hash: &RequestHash,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
) -> CoreResult<Option<PipelineResponse>> {
    if request.policy.replay != ReplayPolicy::Committed || request.envelope.dry_run.is_requested() {
        return Ok(None);
    }
    let Some(idempotency_key) = request.envelope.idempotency_key.as_ref() else {
        return Ok(None);
    };
    let Some(record) = store.tool_invocation(request.method_name, idempotency_key)? else {
        return Ok(None);
    };

    let replay_context = replay_context_from_verified_invocation(verified_invocation)?;
    if !record.matches_verified_replay_context(&replay_context) {
        return Ok(Some(response_from_rejected(
            replay_context_mismatch_response(request.envelope.dry_run, project_state.state_version),
            Some(verified_invocation.clone()),
            None,
        )?));
    }
    if record.request_hash == request_hash.as_str() {
        if !stored_public_response_is_current(
            request.method_name,
            &record.response_json,
            record.committed_state_version,
        ) {
            return Ok(Some(stored_response_corrupt_response(
                request.envelope.dry_run,
                project_state.state_version,
                Some(verified_invocation.clone()),
                request.envelope.task_id.as_ref().cloned(),
            )?));
        }
        let resolved_task_id =
            replay_resolved_task_id(&record.response_json, request, project_state)?;
        let operation_result_ref = operation_result_ref(
            &record.response_json,
            &request.envelope.project_id,
            request.method_name,
            Some(idempotency_key),
            record.committed_state_version,
            verified_invocation,
        );
        return Ok(Some(response_from_json_string_with_operation_result(
            record.response_json,
            operation_result_ref,
            Some(verified_invocation.clone()),
            resolved_task_id,
            true,
        )?));
    }
    Ok(Some(response_from_rejected(
        rejected_response(
            request.envelope.dry_run,
            Some(project_state.state_version),
            vec![idempotency_conflict_error(
                project_state.state_version,
                &request.envelope.project_id,
                request.envelope.task_id.as_ref(),
                idempotency_key,
                &record.request_hash,
                request_hash.as_str(),
            )],
        ),
        Some(verified_invocation.clone()),
        None,
    )?))
}

pub(crate) fn stored_public_response_is_current(
    method_name: MethodName,
    response_json: &str,
    committed_state_version: u64,
) -> bool {
    let contract = public_method_contract(method_name);
    if !contract.has_committed_result_replay() || !raw_json_has_unique_object_members(response_json)
    {
        return false;
    }
    let Ok(response_value) = serde_json::from_str::<Value>(response_json) else {
        return false;
    };
    if !response_value.is_object() {
        return false;
    }
    let Some(metadata) = contract.decode_result_json(response_json, &response_value) else {
        return false;
    };
    if metadata.effect_kind != EffectKind::CoreCommitted
        || metadata.dry_run != DryRunIntent::NotRequested
        || metadata.state_version != committed_state_version
        || metadata.event_count == 0
    {
        return false;
    }
    true
}

fn raw_json_has_unique_object_members(response_json: &str) -> bool {
    let mut deserializer = serde_json::Deserializer::from_str(response_json);
    UniqueJsonDocument::deserialize(&mut deserializer)
        .and_then(|_| deserializer.end())
        .is_ok()
}

struct UniqueJsonDocument;

impl<'de> Deserialize<'de> for UniqueJsonDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonDocumentVisitor)
    }
}

struct UniqueJsonDocumentVisitor;

impl<'de> Visitor<'de> for UniqueJsonDocumentVisitor {
    type Value = UniqueJsonDocument;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with unique object members")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJsonDocument)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJsonDocument)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJsonDocument)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(UniqueJsonDocument)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJsonDocument)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJsonDocument)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJsonDocument)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<UniqueJsonDocument>()?.is_some() {}
        Ok(UniqueJsonDocument)
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut members = BTreeSet::new();
        while let Some(member) = object.next_key::<String>()? {
            if !members.insert(member) {
                return Err(de::Error::custom("duplicate JSON object member"));
            }
            object.next_value::<UniqueJsonDocument>()?;
        }
        Ok(UniqueJsonDocument)
    }
}

pub(crate) fn stored_response_corrupt_response(
    dry_run: DryRunIntent,
    state_version: u64,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelineResponse> {
    response_from_rejected(
        rejected_response(
            dry_run,
            Some(state_version),
            vec![tool_error(
                ErrorCode::PersistedDataCorrupt,
                "stored operation result violates the current response contract",
                false,
                Some(serde_json::Map::from_iter([(
                    "reason".to_owned(),
                    Value::String("stored_response_contract_violation".to_owned()),
                )])),
            )],
        ),
        verified_invocation,
        resolved_task_id,
    )
}

fn replay_resolved_task_id(
    response_json: &str,
    request: &PipelinePreflightRequest,
    project_state: &ProjectStateHeader,
) -> CoreResult<Option<TaskId>> {
    let response: Value = serde_json::from_str(response_json)?;
    for pointer in [
        "/task_ref/record_id",
        "/state/task_ref/record_id",
        "/active_task/task_ref/record_id",
    ] {
        if let Some(task_id) = response
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|task_id| !task_id.is_empty())
        {
            return Ok(Some(TaskId::new(task_id)));
        }
    }
    if let Some(task_id) = request.envelope.task_id.as_ref() {
        return Ok(Some(task_id.clone()));
    }
    Ok(match &request.policy.task {
        TaskRequirement::Exact(task_id) => Some(task_id.clone()),
        TaskRequirement::Required => project_state.active_task_id.as_ref().map(TaskId::new),
        TaskRequirement::None | TaskRequirement::Optional => None,
    })
}

fn freshness_preflight_response(
    request: &PipelinePreflightRequest,
    project_state: &ProjectStateHeader,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<Option<PipelineResponse>> {
    if request.policy.freshness == FreshnessPolicy::None {
        return Ok(None);
    }
    let Some(expected_state_version) = request.envelope.expected_state_version.as_ref().copied()
    else {
        return Ok(None);
    };
    if expected_state_version == project_state.state_version {
        return Ok(None);
    }

    Ok(Some(response_from_rejected(
        rejected_response(
            request.envelope.dry_run,
            Some(project_state.state_version),
            vec![stale_expected_state_error(
                project_state.state_version,
                expected_state_version,
                &request.envelope.project_id,
                request.envelope.task_id.as_ref(),
            )],
        ),
        verified_invocation,
        resolved_task_id,
    )?))
}

fn resolve_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    requirement: TaskRequirement,
) -> Result<Option<TaskId>, TaskResolutionError> {
    match requirement {
        TaskRequirement::None => Ok(None),
        TaskRequirement::Optional => match envelope.task_id.as_ref() {
            Some(task_id) => ensure_task_exists(store, task_id).map(Some),
            None => Ok(None),
        },
        TaskRequirement::Required => {
            if let Some(task_id) = envelope.task_id.as_ref() {
                return ensure_task_exists(store, task_id).map(Some);
            }

            let active_task_id = project_state
                .active_task_id
                .as_ref()
                .ok_or_else(|| TaskResolutionError::Rejection(no_active_task_error()))?;
            let task_id = TaskId::new(active_task_id.clone());
            ensure_task_exists(store, &task_id).map(Some)
        }
        TaskRequirement::Exact(task_id) => ensure_task_exists(store, &task_id).map(Some),
    }
}

enum TaskResolutionError {
    Rejection(ToolError),
    Pipeline(CorePipelineError),
}

fn ensure_task_exists(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> Result<TaskId, TaskResolutionError> {
    match store.task_exists(task_id) {
        Ok(true) => Ok(task_id.clone()),
        Ok(false) => Err(TaskResolutionError::Rejection(no_active_task_error())),
        Err(error) => Err(TaskResolutionError::Pipeline(CorePipelineError::from(
            error,
        ))),
    }
}

struct CommitPipelineArgs<'a, F, B> {
    envelope: &'a ToolEnvelope,
    method_name: MethodName,
    request_hash: &'a RequestHash,
    event_id: String,
    result_fields: F,
    build_base: fn(u64, Vec<EventRef>) -> Result<B, ResultBaseConstructionError>,
    event_kind: String,
    event_payload: JsonObject,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    task_id: &'a TaskId,
    verified_invocation: VerifiedInvocationContext,
    clock_floor: &'a UtcTimestamp,
    include_live_storage_time: bool,
    transition_descriptor: Option<TransitionDescriptor>,
}

fn commit_mutation<F, B>(
    store: &mut CoreProjectStore,
    args: CommitPipelineArgs<'_, F, B>,
) -> CoreResult<PipelineResponse>
where
    F: AssembleMethodResult<B>,
    F::Result: Serialize,
{
    let CommitPipelineArgs {
        envelope,
        method_name,
        request_hash,
        event_id,
        result_fields,
        build_base,
        event_kind,
        event_payload,
        change_unit_id,
        storage_mutations,
        task_id,
        verified_invocation,
        clock_floor,
        include_live_storage_time,
        transition_descriptor,
    } = args;

    let replay_context = replay_context_from_verified_invocation(&verified_invocation)?;
    let mut input = commit_input(
        &envelope.project_id,
        method_name,
        envelope.idempotency_key.as_ref(),
        request_hash,
        Some(replay_context),
        envelope.expected_state_version.as_ref().copied(),
        vec![PendingTaskEvent {
            event_id,
            task_id: Some(task_id.as_str().to_owned()),
            change_unit_id: change_unit_id.map(|id| id.into_inner()),
            event_kind,
            event_payload_json: serde_json::to_string(&Value::Object(event_payload))?,
        }],
    );
    input.clock_floor = Some(clock_floor.clone());
    input.include_live_storage_time = include_live_storage_time;
    input.transition_expectation =
        transition_descriptor
            .clone()
            .map(|descriptor| TransitionCommitExpectation {
                project_id: envelope.project_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                action_key: descriptor.action_key,
                effect_class: descriptor.effect_class,
                expected_result_state: descriptor.expected_result_state,
                basis_state_version: descriptor.expected_state_version,
            });

    let outcome = store.commit_mutation(input, &storage_mutations, |facts| {
        committed_response_json(
            result_fields,
            build_base,
            facts.committed_state_version,
            facts.events,
            transition_descriptor.as_ref(),
        )
        .map_err(store_invalid_input)
    })?;

    match outcome {
        MutationCommitOutcome::Replayed {
            response_json,
            committed_state_version,
            ..
        } => {
            if !stored_public_response_is_current(
                method_name,
                &response_json,
                committed_state_version,
            ) {
                let current_state_version = store.project_state()?.state_version;
                return stored_response_corrupt_response(
                    envelope.dry_run,
                    current_state_version,
                    Some(verified_invocation),
                    Some(task_id.clone()),
                );
            }
            let operation_result_ref = operation_result_ref(
                &response_json,
                &envelope.project_id,
                method_name,
                envelope.idempotency_key.as_ref(),
                committed_state_version,
                &verified_invocation,
            );
            response_from_json_string_with_operation_result(
                response_json,
                operation_result_ref,
                Some(verified_invocation),
                Some(task_id.clone()),
                true,
            )
        }
        MutationCommitOutcome::ReplayContextMismatch {
            current_state_version,
            ..
        } => response_from_rejected(
            replay_context_mismatch_response(envelope.dry_run, current_state_version),
            Some(verified_invocation),
            Some(task_id.clone()),
        ),
        MutationCommitOutcome::IdempotencyConflict {
            current_state_version,
            idempotency_key,
            stored_request_hash,
            attempted_request_hash,
        } => response_from_rejected(
            rejected_response(
                envelope.dry_run,
                Some(current_state_version),
                vec![idempotency_conflict_error(
                    current_state_version,
                    &envelope.project_id,
                    envelope.task_id.as_ref(),
                    &IdempotencyKey::new(idempotency_key),
                    &stored_request_hash,
                    &attempted_request_hash,
                )],
            ),
            Some(verified_invocation),
            Some(task_id.clone()),
        ),
        MutationCommitOutcome::StaleExpectedState {
            current_state_version,
            expected_state_version,
        } => response_from_rejected(
            rejected_response(
                envelope.dry_run,
                Some(current_state_version),
                vec![stale_expected_state_error(
                    current_state_version,
                    expected_state_version,
                    &envelope.project_id,
                    envelope.task_id.as_ref(),
                )],
            ),
            Some(verified_invocation),
            Some(task_id.clone()),
        ),
        MutationCommitOutcome::Committed {
            response_json,
            committed_state_version,
            ..
        } => {
            let operation_result_ref = operation_result_ref(
                &response_json,
                &envelope.project_id,
                method_name,
                envelope.idempotency_key.as_ref(),
                committed_state_version,
                &verified_invocation,
            );
            response_from_json_string_with_operation_result(
                response_json,
                operation_result_ref,
                Some(verified_invocation),
                Some(task_id.clone()),
                false,
            )
        }
    }
}

pub(crate) fn operation_result_ref(
    response_json: &str,
    project_id: &ProjectId,
    source_method: MethodName,
    source_idempotency_key: Option<&IdempotencyKey>,
    committed_state_version: u64,
    verified_invocation: &VerifiedInvocationContext,
) -> Option<OperationResultRef> {
    if !matches!(
        verified_invocation.actor_source,
        ActorSource::AgentConnection(_)
    ) || verified_invocation.operation_category != OperationCategory::AgentWorkflow
    {
        return None;
    }
    let source_idempotency_key = source_idempotency_key?.clone();
    Some(OperationResultRef {
        project_id: project_id.clone(),
        source_method,
        source_idempotency_key,
        committed_state_version,
        response_sha256: format!("sha256:{:x}", Sha256::digest(response_json.as_bytes())),
        response_size_bytes: response_json.len() as u64,
    })
}

fn committed_response_json<F, B>(
    result_fields: F,
    build_base: fn(u64, Vec<EventRef>) -> Result<B, ResultBaseConstructionError>,
    committed_state_version: u64,
    events: Vec<CommittedEventRef>,
    transition_descriptor: Option<&TransitionDescriptor>,
) -> CoreResult<String>
where
    F: AssembleMethodResult<B>,
    F::Result: Serialize,
{
    let event_refs = events
        .into_iter()
        .map(|event| EventRef {
            event_id: EventId::new(event.event_id),
            event_kind: event.event_kind,
        })
        .collect();
    let base =
        build_base(committed_state_version, event_refs).map_err(result_base_construction_error)?;
    let mut response = serde_json::to_value(result_fields.assemble(base))?;
    if let Some(descriptor) = transition_descriptor {
        let transition = response
            .get_mut("base")
            .and_then(Value::as_object_mut)
            .and_then(|base| base.get_mut("transition"))
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "committed result base has no transition projection".to_owned(),
            })?;
        *transition = serde_json::to_value(descriptor)?;
    }
    serde_json::to_string(&response).map_err(CorePipelineError::from)
}

fn response_from_rejected(
    response: ToolRejectedResponse,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelineResponse> {
    response_from_value(
        serde_json::to_value(response)?,
        verified_invocation,
        resolved_task_id,
        false,
    )
}

fn response_outcome_from_rejected<'mutation>(
    response: ToolRejectedResponse,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelinePreflightOutcome<'mutation>> {
    response_from_rejected(response, verified_invocation, resolved_task_id)
        .map(|response| PipelinePreflightOutcome::Response(Box::new(response)))
}

fn response_from_dry_run(
    response: ToolDryRunResponse,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelineResponse> {
    response_from_value(
        serde_json::to_value(response)?,
        verified_invocation,
        resolved_task_id,
        false,
    )
}

fn response_from_value(
    response_value: Value,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
    replayed: bool,
) -> CoreResult<PipelineResponse> {
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        operation_result_ref: None,
        verified_invocation,
        resolved_task_id,
        replayed,
    })
}

fn response_from_json_string_with_operation_result(
    response_json: String,
    operation_result_ref: Option<OperationResultRef>,
    verified_invocation: Option<VerifiedInvocationContext>,
    resolved_task_id: Option<TaskId>,
    replayed: bool,
) -> CoreResult<PipelineResponse> {
    let response_value = serde_json::from_str(&response_json)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        operation_result_ref,
        verified_invocation,
        resolved_task_id,
        replayed,
    })
}

fn validation_error(field: &'static str, message: &'static str) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(ErrorCode::ValidationFailed, message, false, Some(details))
}

fn stale_expected_state_error(
    current_state_version: u64,
    expected_state_version: u64,
    project_id: &ProjectId,
    task_id: Option<&TaskId>,
) -> ToolError {
    let mut details = state_conflict_details(current_state_version, project_id, task_id);
    details.insert(
        "expected_state_version".to_owned(),
        Value::from(expected_state_version),
    );
    tool_error(
        ErrorCode::StateVersionConflict,
        "expected_state_version is stale",
        true,
        Some(details),
    )
}

fn idempotency_conflict_error(
    current_state_version: u64,
    project_id: &ProjectId,
    task_id: Option<&TaskId>,
    idempotency_key: &IdempotencyKey,
    stored_request_hash: &str,
    attempted_request_hash: &str,
) -> ToolError {
    let mut details = state_conflict_details(current_state_version, project_id, task_id);
    details.insert(
        "idempotency_key".to_owned(),
        Value::String(idempotency_key.as_str().to_owned()),
    );
    details.insert(
        "stored_request_hash".to_owned(),
        Value::String(stored_request_hash.to_owned()),
    );
    details.insert(
        "attempted_request_hash".to_owned(),
        Value::String(attempted_request_hash.to_owned()),
    );
    tool_error(
        ErrorCode::StateVersionConflict,
        "idempotency_key was reused with a different request hash",
        false,
        Some(details),
    )
}

fn state_conflict_details(
    current_state_version: u64,
    project_id: &ProjectId,
    task_id: Option<&TaskId>,
) -> JsonObject {
    let mut details = Map::new();
    details.insert(
        "state_clock".to_owned(),
        Value::String("project_state.state_version".to_owned()),
    );
    details.insert(
        "current_state_version".to_owned(),
        Value::from(current_state_version),
    );
    details.insert(
        "project_id".to_owned(),
        Value::String(project_id.as_str().to_owned()),
    );
    if let Some(task_id) = task_id {
        details.insert(
            "task_id".to_owned(),
            Value::String(task_id.as_str().to_owned()),
        );
    }
    details
}

pub(crate) fn store_failure_error(error: StoreError) -> ToolError {
    let classification = error.classification();
    assert_ne!(
        classification.route,
        StoreFailureRoute::OperationalUnavailable,
        "operational Store failures must remain on the Core error path"
    );
    let mut details = Map::new();
    if let Some(diagnostic) = error.platform_diagnostic() {
        details.insert(
            "diagnostic_code".to_owned(),
            Value::String(diagnostic.code().to_owned()),
        );
    } else {
        details.insert(
            "store_failure_category".to_owned(),
            Value::String(classification.category.to_owned()),
        );
    }
    if let Some(database_kind) = classification.database_kind {
        details.insert(
            "database_kind".to_owned(),
            Value::String(database_kind.to_owned()),
        );
    }
    if let Some(entity) = classification.entity {
        details.insert("entity".to_owned(), Value::String(entity.to_owned()));
    }
    if let Some(field) = classification.field {
        details.insert("field".to_owned(), Value::String(field.to_owned()));
    }
    if let StoreError::InvalidProjectRegistration {
        project_id,
        relationship,
        ..
    } = &error
    {
        details.insert(
            "project_id".to_owned(),
            Value::String(project_id.to_owned()),
        );
        details.insert(
            "path_relationship".to_owned(),
            Value::String((*relationship).to_owned()),
        );
    }
    if let Some(owner_state_error) = classification.owner_state_error {
        let projected = match owner_state_error {
            StoreCorruptionLocation::Field(field) => json!({
                "table": field.table,
                "record_ref": field.record_ref,
                "logical_column": field.logical_column,
                "corruption_category": field.corruption_category
            }),
            StoreCorruptionLocation::AggregateInvariant {
                aggregate,
                record_ref,
                invariant,
                corruption_category,
            } => json!({
                "aggregate": aggregate.as_str(),
                "record_ref": record_ref,
                "invariant": invariant.as_str(),
                "corruption_category": corruption_category
            }),
        };
        details.insert("owner_state_error".to_owned(), projected);
    }
    let code = match classification.route {
        StoreFailureRoute::OperationalUnavailable => {
            unreachable!("operational Store failures must remain on the Core error path")
        }
        StoreFailureRoute::InvalidEnvironment => ErrorCode::ValidationFailed,
        StoreFailureRoute::InvocationContextMismatch => ErrorCode::InvocationContextMismatch,
        StoreFailureRoute::PersistedDataCorrupt => ErrorCode::PersistedDataCorrupt,
    };
    let message = match code {
        ErrorCode::ValidationFailed => "platform environment is invalid",
        ErrorCode::InvocationContextMismatch => {
            "project binding or invocation context does not match registration"
        }
        ErrorCode::PersistedDataCorrupt => "persisted owner data violates its declared contract",
        _ => "Core storage is unavailable",
    };
    tool_error(code, message, classification.retryable, Some(details))
}

fn no_active_task_error() -> ToolError {
    tool_error(
        ErrorCode::NoActiveTask,
        "a Task is required but no addressed or current Task is available",
        false,
        None,
    )
}

fn error_precedence(code: ErrorCode) -> u8 {
    match code {
        ErrorCode::ValidationFailed => 1,
        ErrorCode::RunKindIncompatible => 2,
        ErrorCode::TaskPhaseTransitionRequired => 3,
        ErrorCode::ShapingCheckpointRequired => 4,
        ErrorCode::ShapingCheckpointStale => 5,
        ErrorCode::UserDecisionUnresolved => 6,
        ErrorCode::ChangeUnitRequired => 7,
        ErrorCode::ChangeUnitStale => 8,
        ErrorCode::WorkspaceBasisStale => 9,
        ErrorCode::WorkflowActionNotAllowed => 10,
        ErrorCode::PersistedDataCorrupt => 11,
        ErrorCode::StateVersionConflict => 12,
        ErrorCode::InvocationContextMismatch => 13,
        ErrorCode::NoActiveTask => 14,
        ErrorCode::NoActiveChangeUnit => 15,
        ErrorCode::BaselineStale => 16,
        ErrorCode::ScopeRequired => 17,
        ErrorCode::ScopeViolation => 18,
        ErrorCode::WriteTicketRequired => 19,
        ErrorCode::WriteTicketInvalid => 20,
        ErrorCode::ApprovalDenied => 21,
        ErrorCode::ApprovalExpired => 22,
        ErrorCode::ApprovalRequired => 23,
        ErrorCode::DecisionUnresolved => 24,
        ErrorCode::AutonomyBoundaryExceeded => 25,
        ErrorCode::DecisionRequired => 26,
        ErrorCode::CapabilityInsufficient => 27,
        ErrorCode::EvidenceInsufficient => 28,
        ErrorCode::ResidualRiskNotVisible => 29,
        ErrorCode::AcceptanceRequired => 30,
        ErrorCode::ProjectionStale => 31,
        ErrorCode::ArtifactMissing => 32,
        ErrorCode::ValidatorFailed => 33,
        ErrorCode::OperationResultUnavailable => 34,
    }
}

fn store_invalid_input(error: CorePipelineError) -> StoreError {
    StoreError::InvalidInput {
        detail: error.to_string(),
    }
}

#[allow(dead_code)]
fn _assert_commit_input_sendable(_: CommitMutationInput) {}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
    };

    use serde::{Deserialize, Serialize};
    use serde_json::{json, Map, Value};
    use volicord_store::{
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
        core_pipeline::{
            ChangeUnitInsert, CoreProjectStore, StorageEffectCounts, StoredChangeUnitLifecycle,
            StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis, TaskMutation,
            TaskScopeUpdate,
        },
        sqlite::registry_db_path,
        StoreAggregateInvariant, WriteTicketInvariant,
    };
    use volicord_test_support::{
        open_project_fixture_database, open_registry_fixture_database,
        with_test_runtime_home_setup, TempRuntimeHome, TestRuntimeHomeMutation,
    };
    use volicord_types::ids::{EventId, IdempotencyKey, ProjectId, RequestId};
    use volicord_types::methods::{
        CloseTaskRequest, CloseTaskResultBase, IntakeRequest, IntakeResultBase,
        ReconcileChangesResultBase, StageArtifactRequest, StatusRequest, StatusResultBase,
        SupportsStagingCreatedResult,
    };
    use volicord_types::schema::PlannedEffect;
    use volicord_types::values::{ActorSource, OperationCategory, UserActionChannelKind};

    use super::*;

    const PROJECT_ID: &str = "project_a";
    const TASK_ID: &str = "task_a";
    const CONNECTION_ID: &str = "connection_main";

    #[test]
    fn no_commit_expected_result_mismatch_fails_closed() {
        let workflow = WorkflowProjection::Implementation {
            next_actor: volicord_types::values::AuthorityNextActor::Agent,
            required_refs: Vec::new(),
            expected_state_version: 7,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::null(),
            transition_catalog: volicord_types::schema::WorkflowTransitionCatalog::new(Vec::new())
                .expect("empty test catalog"),
            close_readiness: volicord_types::schema::WorkflowCloseReadiness {
                assessment_required: true,
                current_close_basis_present: false,
            },
        };
        let planned_fields = json!({
            "state": {
                "work_phase": "implementation",
                "lifecycle": { "lifecycle_phase": "executing" }
            }
        });

        assert!(matches!(
            validate_planned_expected_result(
                false,
                Some(&planned_fields),
                &workflow,
                WorkflowExpectedResultState::Terminal,
            ),
            Err(CorePipelineError::Invariant { .. })
        ));
        assert!(validate_planned_expected_result(
            false,
            Some(&planned_fields),
            &workflow,
            WorkflowExpectedResultState::Implementation,
        )
        .is_ok());
    }

    #[test]
    fn write_ticket_invariant_corruption_projects_without_a_physical_column() {
        let error = store_failure_error(StoreError::CorruptOwnerStateInvariant {
            database_kind: "project_state",
            record_ref: "ticket".to_owned(),
            invariant: StoreAggregateInvariant::WriteTicket(
                WriteTicketInvariant::TaskIdentityAgreement,
            ),
        });
        let details = error.details().expect("corruption details");
        let location = details["owner_state_error"]
            .as_object()
            .expect("aggregate location");

        assert_eq!(location["aggregate"], "write_ticket");
        assert_eq!(location["record_ref"], "ticket");
        assert_eq!(location["invariant"], "task_identity_agreement");
        assert_eq!(
            location["corruption_category"],
            "corrupt_aggregate_invariant"
        );
        assert!(!location.contains_key("table"));
        assert!(!location.contains_key("logical_column"));
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    struct PipelineTestResultFields {
        pipeline_marker: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct PipelineTestResult<B> {
        base: B,
        pipeline_marker: String,
    }

    impl<B> AssembleMethodResult<B> for PipelineTestResultFields {
        type Result = PipelineTestResult<B>;

        fn assemble(self, base: B) -> Self::Result {
            PipelineTestResult {
                base,
                pipeline_marker: self.pipeline_marker,
            }
        }
    }

    #[test]
    fn pipeline_accepts_preview_only_for_declared_method_policies() {
        use volicord_types::methods::{DryRunRequestPolicy, PUBLIC_METHOD_CONTRACTS};

        let preview_branch = test_preview_branch();
        for contract in PUBLIC_METHOD_CONTRACTS {
            let preview_accepted = validate_method_response_branch(
                contract.method(),
                DryRunIntent::Requested,
                &preview_branch,
            )
            .is_ok();
            assert_eq!(
                preview_accepted,
                contract.dry_run_policy().route(DryRunIntent::Requested)
                    == DryRunRequestRoute::Preview,
                "{} pipeline branch disagrees with its response declaration",
                contract.method().as_str()
            );

            let read_only = test_read_only_branch(contract.method().as_str());
            assert_eq!(
                validate_method_response_branch(
                    contract.method(),
                    DryRunIntent::NotRequested,
                    &read_only,
                )
                .is_ok(),
                contract.supports_result_effect(ResultEffectContract::ReadOnly),
                "{} read-only route disagrees with its result effects",
                contract.method().as_str()
            );
            assert_eq!(
                validate_method_response_branch(
                    contract.method(),
                    DryRunIntent::Requested,
                    &read_only,
                )
                .is_ok(),
                contract.supports_result_effect(ResultEffectContract::ReadOnly)
                    && contract.dry_run_policy() == DryRunRequestPolicy::RegularResult,
                "{} requested dry-run result route disagrees with policy",
                contract.method().as_str()
            );
            assert_eq!(
                validate_method_response_branch(
                    contract.method(),
                    DryRunIntent::NotRequested,
                    &commit_branch(contract.method().as_str()),
                )
                .is_ok(),
                contract.supports_result_effect(ResultEffectContract::CoreCommitted),
                "{} Core-committed route disagrees with its result effects",
                contract.method().as_str()
            );
            assert_eq!(
                validate_method_response_branch(
                    contract.method(),
                    DryRunIntent::NotRequested,
                    &test_no_effect_branch(contract.method().as_str()),
                )
                .is_ok(),
                contract.supports_result_effect(ResultEffectContract::NoEffect),
                "{} no-effect route disagrees with its result effects",
                contract.method().as_str()
            );

            let rejection =
                dry_run_policy_rejection(contract.method(), DryRunIntent::Requested, None);
            assert_eq!(
                rejection.is_some(),
                contract.dry_run_policy() == DryRunRequestPolicy::Forbidden,
                "{} policy rejection disagrees with declaration",
                contract.method().as_str()
            );
            if let Some(rejection) = rejection {
                let value =
                    serde_json::to_value(rejection).expect("policy rejection should serialize");
                assert_eq!(value["base"]["response_kind"], "rejected");
                assert_eq!(value["base"]["dry_run"], true);
            }
        }
    }

    #[test]
    fn semantic_response_constructors_set_every_fixed_branch_fact() {
        let event = EventRef {
            event_id: EventId::new("event_contract_test"),
            event_kind: "contract_test".to_owned(),
        };
        for (base, expected_effect, expected_dry_run) in [
            (
                serde_json::to_value(
                    StatusRequest::read_only_result_base(DryRunIntent::NotRequested, 7)
                        .expect("status read-only base"),
                )
                .expect("status base serializes"),
                "read_only",
                false,
            ),
            (
                serde_json::to_value(
                    StatusRequest::read_only_result_base(DryRunIntent::Requested, 7)
                        .expect("status requested-intent base"),
                )
                .expect("status base serializes"),
                "read_only",
                true,
            ),
            (
                serde_json::to_value(
                    IntakeRequest::core_committed_result_base(8, vec![event])
                        .expect("nonempty commit events"),
                )
                .expect("committed base serializes"),
                "core_committed",
                false,
            ),
            (
                serde_json::to_value(StageArtifactRequest::staging_created_result_base(7))
                    .expect("staging base serializes"),
                "staging_created",
                false,
            ),
            (
                serde_json::to_value(CloseTaskRequest::no_effect_result_base(7))
                    .expect("no-effect base serializes"),
                "no_effect",
                false,
            ),
        ] {
            assert_eq!(base["response_kind"], "result");
            assert_eq!(base["effect_kind"], expected_effect);
            assert_eq!(base["dry_run"], expected_dry_run);
        }

        let rejected = serde_json::to_value(rejected_response(
            DryRunIntent::Requested,
            Some(7),
            Vec::new(),
        ))
        .expect("rejection should serialize");
        assert_eq!(rejected["base"]["response_kind"], "rejected");
        assert_eq!(rejected["base"]["effect_kind"], "no_effect");
        assert_eq!(rejected["base"]["dry_run"], true);

        let preview = serde_json::to_value(dry_run_response(Some(7), dry_run_summary()))
            .expect("preview should serialize");
        assert_eq!(preview["base"]["response_kind"], "dry_run");
        assert_eq!(preview["base"]["effect_kind"], "no_effect");
        assert_eq!(preview["base"]["dry_run"], true);
    }

    #[test]
    fn admitted_core_service_accepts_only_its_exact_mutation_identity() -> Result<(), Box<dyn Error>>
    {
        let first = TempRuntimeHome::new("core-service-admitted-first")?;
        let second = TempRuntimeHome::new("core-service-admitted-second")?;
        let first_mutation = TestRuntimeHomeMutation::acquire(first.path())?;
        let second_mutation = TestRuntimeHomeMutation::acquire(second.path())?;
        let first_context = first_mutation.context()?;
        let second_context = second_mutation.context()?;

        let admitted = CoreService::for_mutation(&first_context);
        admitted.ensure_mutation_context(&first_context)?;
        let mismatch = admitted
            .ensure_mutation_context(&second_context)
            .expect_err("a Core service admitted for A must reject context B");
        assert!(matches!(mismatch, StoreError::InvalidInput { .. }));

        let read_only = CoreService::for_read_only(first.path());
        let missing_admission = read_only
            .ensure_mutation_context(&first_context)
            .expect_err("a read-only Core service must not accept mutation context");
        assert!(matches!(missing_admission, StoreError::InvalidInput { .. }));
        Ok(())
    }

    #[test]
    fn system_clock_creation_is_not_after_a_same_millisecond_core_utc_read() {
        let system_sample = DateTime::parse_from_rfc3339("2026-07-13T12:34:56.123456789Z")
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);
        let sqlite_core_read = DateTime::parse_from_rfc3339("2026-07-13T12:34:56.123Z")
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);

        let created_at = canonical_core_utc_timestamp(system_sample);

        assert_eq!(created_at, sqlite_core_read);
        assert!(created_at <= sqlite_core_read);
    }

    #[test]
    fn operation_result_unavailable_uses_public_error_precedence() {
        let response = rejected_response(
            DryRunIntent::NotRequested,
            Some(7),
            vec![
                tool_error(
                    ErrorCode::OperationResultUnavailable,
                    "stored operation result is unavailable",
                    false,
                    None,
                ),
                tool_error(ErrorCode::ValidatorFailed, "validator failed", false, None),
                tool_error(
                    ErrorCode::ValidationFailed,
                    "request validation failed",
                    false,
                    None,
                ),
            ],
        );

        assert_eq!(
            response
                .errors()
                .iter()
                .map(ToolError::code)
                .collect::<Vec<_>>(),
            vec![
                ErrorCode::ValidationFailed,
                ErrorCode::ValidatorFailed,
                ErrorCode::OperationResultUnavailable,
            ]
        );
    }

    #[test]
    fn semantic_error_without_details_serializes_required_null() {
        let response = rejected_response(
            DryRunIntent::NotRequested,
            Some(7),
            vec![tool_error(
                ErrorCode::ValidationFailed,
                "request validation failed",
                false,
                None,
            )],
        );

        let serialized = serde_json::to_value(response).expect("rejection should serialize");
        assert_eq!(serialized["errors"][0]["details"], Value::Null);
        assert!(serialized["errors"][0]
            .as_object()
            .expect("ToolError should serialize as an object")
            .contains_key("details"));
    }

    #[test]
    fn stored_response_gate_rejects_shared_non_result_branches_for_every_method() {
        let rejected = serde_json::to_string(&rejected_response(
            DryRunIntent::NotRequested,
            Some(7),
            vec![tool_error(
                ErrorCode::ValidationFailed,
                "stored response fixture",
                false,
                None,
            )],
        ))
        .expect("rejected response should serialize");
        let dry_run = serde_json::to_string(&dry_run_response(
            Some(7),
            DryRunSummary {
                planned_effects: Vec::new(),
                would_blockers: Vec::new(),
                would_errors: Vec::new(),
                next_actions: Vec::new(),
                diagnostics: Vec::new(),
            },
        ))
        .expect("dry-run response should serialize");
        let methods = [
            MethodName::Intake,
            MethodName::UpdateScope,
            MethodName::Status,
            MethodName::GetOperationResult,
            MethodName::CheckClose,
            MethodName::PrepareEvidenceCapture,
            MethodName::PrepareWrite,
            MethodName::StageArtifact,
            MethodName::RecordRun,
            MethodName::RequestUserAction,
            MethodName::ResolveUserAction,
            MethodName::ReconcileChanges,
            MethodName::CloseTask,
        ];

        for method in methods {
            for (branch, response_json) in [("rejected", &rejected), ("dry_run", &dry_run)] {
                assert!(
                    !stored_public_response_is_current(method, response_json, 7),
                    "{method:?} must reject the shared {branch} branch"
                );
            }
        }
    }

    struct PipelineHarness {
        mutation: TestRuntimeHomeMutation,
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
        service: CoreService,
    }

    impl PipelineHarness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("core-pipeline")?;
            let repo_root = runtime_home.create_product_repo("repo")?;
            with_test_runtime_home_setup(runtime_home.path(), |context| {
                initialize_runtime_home(context, "runtime_home_a", "{}")?;
                register_project(
                    context,
                    ProjectRegistration {
                        project_id: PROJECT_ID.to_owned(),
                        repo_root,
                        project_home: None,
                        status: ACTIVE_PROJECT_STATUS.to_owned(),
                        metadata_json: "{}".to_owned(),
                    },
                )?;

                let conn =
                    open_project_fixture_database(runtime_home.project_state_db_path(PROJECT_ID))?;
                conn.execute(
                    "INSERT INTO tasks (
                    project_id,
                    task_id,
                    created_by_actor_source,
                    mode,
                    requested_control_level,
                    effective_control_level,
                    control_level_reason,
                    work_phase,
                    acceptance_policy,
                    acceptance_policy_reason,
                    carry_forward_json,
                    lifecycle_phase,
                    created_at,
                    updated_at
                )
                VALUES (
                    'project_a',
                    'task_a',
                    'agent_connection:connection_main',
                    'work',
                    'tracked',
                    'tracked',
                    'Pipeline fixture uses tracked control.',
                    'shaping',
                    'required',
                    'Pipeline fixture requires explicit acceptance.',
                    '[]',
                    'shaping',
                    't0',
                    't0'
                )",
                    [],
                )?;
                conn.execute(
                    "UPDATE project_state
                    SET active_task_id = 'task_a'
                  WHERE project_id = 'project_a'",
                    [],
                )?;
                Ok(())
            })?;

            let runtime_home_path = runtime_home.path().to_path_buf();
            let mutation = TestRuntimeHomeMutation::acquire(&runtime_home_path)?;
            let service = CoreService::for_mutation(&mutation.context()?);
            Ok(Self {
                mutation,
                _runtime_home: runtime_home,
                runtime_home_path,
                service,
            })
        }

        fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
            let store = CoreProjectStore::open_read_only(
                &self.runtime_home_path,
                &ProjectId::new(PROJECT_ID),
            )?;
            Ok(store.effect_counts()?)
        }

        fn conn(&self) -> Result<rusqlite::Connection, StoreError> {
            open_project_fixture_database(
                self.runtime_home_path
                    .join("projects")
                    .join(PROJECT_ID)
                    .join("state.sqlite"),
            )
        }

        fn execute<B>(
            &self,
            request: PipelineRequest<PipelineTestResultFields, B>,
        ) -> CoreResult<PipelineResponse>
        where
            PipelineTestResultFields: AssembleMethodResult<B>,
            <PipelineTestResultFields as AssembleMethodResult<B>>::Result: Serialize,
        {
            let context = self.mutation.context().map_err(CorePipelineError::from)?;
            self.service.execute_pipeline(Some(&context), request)
        }

        fn execute_with_action<B>(
            &self,
            request: PipelineRequest<PipelineTestResultFields, B>,
            semantic_variant: volicord_types::values::WorkflowActionSemanticVariant,
        ) -> CoreResult<PipelineResponse>
        where
            PipelineTestResultFields: AssembleMethodResult<B>,
            <PipelineTestResultFields as AssembleMethodResult<B>>::Result: Serialize,
        {
            let context = self.mutation.context().map_err(CorePipelineError::from)?;
            let policy = MethodPolicy::for_branch(
                request.operation_category,
                request.task_requirement,
                &request.branch,
            );
            let preflight = PipelinePreflightRequest {
                method_name: request.method_name,
                attempted_action_key: WorkflowActionKey::new(request.method_name, semantic_variant)
                    .ok(),
                envelope: request.envelope,
                request_json: request.request_json,
                invocation: request.invocation,
                policy,
            };
            match self.service.prepare_request(Some(&context), preflight)? {
                PipelinePreflightOutcome::Prepared(prepared) => self
                    .service
                    .execute_prepared_request(*prepared, request.branch),
                PipelinePreflightOutcome::Response(response) => Ok(*response),
            }
        }

        fn state_db_path(&self) -> PathBuf {
            self.runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite")
        }

        fn replace_project_repo_root(&self, repo_root: &Path) -> Result<(), Box<dyn Error>> {
            let conn = open_registry_fixture_database(registry_db_path(&self.runtime_home_path))?;
            conn.execute(
                "UPDATE projects SET repo_root = ?2 WHERE project_internal_id = ?1",
                rusqlite::params![PROJECT_ID, repo_root.to_string_lossy().as_ref()],
            )?;
            Ok(())
        }
    }

    #[test]
    fn every_task_state_bound_method_requires_an_exact_action_key() -> Result<(), Box<dyn Error>> {
        use volicord_types::methods::{WorkflowActionAdmissionClass, PUBLIC_METHOD_CONTRACTS};

        let harness = PipelineHarness::new()?;
        let context = harness.mutation.context()?;
        for contract in PUBLIC_METHOD_CONTRACTS.iter().filter(|contract| {
            contract.workflow_action_admission() == WorkflowActionAdmissionClass::TaskStateBound
        }) {
            let method = contract.method();
            let request_id = format!("req_missing_action_{}", method.as_str().replace('.', "_"));
            let idempotency_key =
                format!("idem_missing_action_{}", method.as_str().replace('.', "_"));
            let envelope = envelope(
                &request_id,
                Some(&idempotency_key),
                false,
                Some(0),
                Some(TASK_ID),
            );
            let outcome = harness.service.prepare_request(
                Some(&context),
                PipelinePreflightRequest {
                    method_name: method,
                    attempted_action_key: None,
                    request_json: request_json(method, &envelope, "missing-action-key"),
                    envelope,
                    invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
                    policy: MethodPolicy::exact(
                        OperationCategory::AgentWorkflow,
                        TaskRequirement::Required,
                        ReplayPolicy::Committed,
                        FreshnessPolicy::IfPresent,
                        MethodEffectPolicy::CoreMutation,
                    ),
                },
            );
            match outcome {
                Err(CorePipelineError::InvalidDispatch { detail })
                    if detail.contains(method.as_str())
                        && detail.contains("exact workflow action key") => {}
                Ok(PipelinePreflightOutcome::Response(response)) => panic!(
                    "{} returned a response before exact-key admission: {}",
                    method.as_str(),
                    response.response_json
                ),
                Ok(PipelinePreflightOutcome::Prepared(_)) => {
                    panic!("{} prepared without its exact action key", method.as_str())
                }
                Err(error) => panic!(
                    "{} returned the wrong exact-key failure: {error}",
                    method.as_str()
                ),
            }
        }
        Ok(())
    }

    #[test]
    fn exact_method_with_noncurrent_variant_rejects_before_method_semantics(
    ) -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_wrong_transition_variant",
            Some("idem_wrong_transition_variant"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let context = harness.mutation.context()?;
        let outcome = harness.service.prepare_request(
            Some(&context),
            PipelinePreflightRequest {
                method_name: MethodName::UpdateScope,
                attempted_action_key: WorkflowActionKey::new(
                    MethodName::UpdateScope,
                    volicord_types::values::WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
                )
                .ok(),
                request_json: update_scope_request_json(
                    &envelope,
                    "wrong transition variant",
                    "keep_current",
                ),
                envelope,
                invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
                policy: MethodPolicy::exact(
                    OperationCategory::AgentWorkflow,
                    TaskRequirement::Required,
                    ReplayPolicy::Committed,
                    FreshnessPolicy::IfPresent,
                    MethodEffectPolicy::CoreMutation,
                ),
            },
        )?;
        let PipelinePreflightOutcome::Response(response) = outcome else {
            panic!("a noncurrent exact variant must not prepare");
        };
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "WORKFLOW_ACTION_NOT_ALLOWED"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["reason"],
            "variant_not_current"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn submitted_authority_coordinates_must_match_the_admitted_transition(
    ) -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_transition_coordinate_mismatch",
            Some("idem_transition_coordinate_mismatch"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let mut request_json = update_scope_request_json(
            &envelope,
            "transition coordinate mismatch",
            "create_current",
        );
        request_json["task_id"] = Value::String("task_other".to_owned());
        let context = harness.mutation.context()?;
        let outcome = harness.service.prepare_request(
            Some(&context),
            PipelinePreflightRequest {
                method_name: MethodName::UpdateScope,
                attempted_action_key: WorkflowActionKey::new(
                    MethodName::UpdateScope,
                    volicord_types::values::WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
                )
                .ok(),
                request_json,
                envelope,
                invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
                policy: MethodPolicy::exact(
                    OperationCategory::AgentWorkflow,
                    TaskRequirement::Required,
                    ReplayPolicy::Committed,
                    FreshnessPolicy::IfPresent,
                    MethodEffectPolicy::CoreMutation,
                ),
            },
        )?;
        let PipelinePreflightOutcome::Response(response) = outcome else {
            panic!("mismatched transition coordinates must not prepare");
        };
        assert_eq!(
            response.response_value["errors"][0]["details"]["reason"],
            "authority_basis_mismatch"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["recovery_action_key"],
            json!({
                "method": "volicord.update_scope",
                "semantic_variant": "create_current_change_unit"
            })
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn rejected_branch_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_missing_task",
            Some("idem_missing_task"),
            false,
            Some(0),
            Some("missing_task"),
        );

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &envelope, "missing-task"),
            envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("missing-task"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "NO_ACTIVE_TASK"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn dry_run_branch_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_dry_run",
            Some("idem_dry_run"),
            true,
            Some(0),
            Some(TASK_ID),
        );

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &envelope, "dry-run"),
            envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: test_preview_branch(),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(response.response_value["base"]["state_version"], 0);
        assert_eq!(response.response_value["base"]["events"], json!([]));
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn read_only_branch_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope("req_read_only", None, false, None, Some(TASK_ID));

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::Status,
            request_json: request_json(MethodName::Status, &envelope, "read-only"),
            envelope,
            invocation: invocation(OperationCategory::Read, Some(CONNECTION_ID)),
            operation_category: OperationCategory::Read,
            task_requirement: TaskRequirement::Optional,
            branch: test_read_only_branch("read_only"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        assert_eq!(response.response_value["base"]["state_version"], 0);
        assert_eq!(response.response_value["base"]["events"], json!([]));
        let typed: PipelineTestResult<StatusResultBase> =
            serde_json::from_str(&response.response_json)?;
        assert_eq!(typed.pipeline_marker, "read_only");
        assert_eq!(serde_json::to_value(typed)?, response.response_value);
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn no_effect_result_branch_composes_typed_fields_without_storage_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_no_effect",
            Some("idem_no_effect"),
            false,
            Some(0),
            Some(TASK_ID),
        );

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::CloseTask,
            request_json: request_json(MethodName::CloseTask, &envelope, "no-effect"),
            envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: test_no_effect_branch("no_effect"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(response.response_value["base"]["state_version"], 0);
        assert_eq!(response.response_value["base"]["events"], json!([]));
        let typed: PipelineTestResult<CloseTaskResultBase> =
            serde_json::from_str(&response.response_json)?;
        assert_eq!(typed.pipeline_marker, "no_effect");
        assert_eq!(serde_json::to_value(typed)?, response.response_value);
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn invalid_project_registration_is_an_operational_core_error() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        harness.replace_project_repo_root(&harness.runtime_home_path)?;
        let envelope = envelope("req_invalid_project_path", None, false, None, None);

        let error = harness
            .execute(PipelineRequest {
                method_name: MethodName::Status,
                request_json: request_json(MethodName::Status, &envelope, "invalid-project-path"),
                envelope,
                invocation: invocation(OperationCategory::Read, Some(CONNECTION_ID)),
                operation_category: OperationCategory::Read,
                task_requirement: TaskRequirement::Optional,
                branch: test_read_only_branch("invalid_project_path"),
            })
            .expect_err("invalid project registration must not produce a method response");

        assert_operational_unavailable(error, CoreOperationalResource::RegistryStore, false);
        Ok(())
    }

    #[test]
    fn missing_project_state_database_is_an_operational_core_error() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        fs::remove_file(harness.state_db_path())?;

        let error = harness
            .execute(PipelineRequest {
                method_name: MethodName::Status,
                request_json: request_json(
                    MethodName::Status,
                    &envelope("req_missing_db", None, false, None, None),
                    "missing-db",
                ),
                envelope: envelope("req_missing_db", None, false, None, None),
                invocation: invocation(OperationCategory::Read, Some(CONNECTION_ID)),
                operation_category: OperationCategory::Read,
                task_requirement: TaskRequirement::Optional,
                branch: test_read_only_branch("missing_db"),
            })
            .expect_err("missing project Store must not produce a method response");

        assert_operational_unavailable(error, CoreOperationalResource::ProjectStore, true);
        Ok(())
    }

    #[test]
    fn unexpected_schema_relation_routes_to_persisted_data_corruption() -> Result<(), Box<dyn Error>>
    {
        let harness = PipelineHarness::new()?;
        harness
            .conn()?
            .execute("CREATE TABLE unexpected_relation (value TEXT NOT NULL)", [])?;

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::Status,
            request_json: request_json(
                MethodName::Status,
                &envelope("req_unexpected_schema_relation", None, false, None, None),
                "unexpected-schema-relation",
            ),
            envelope: envelope("req_unexpected_schema_relation", None, false, None, None),
            invocation: invocation(OperationCategory::Read, Some(CONNECTION_ID)),
            operation_category: OperationCategory::Read,
            task_requirement: TaskRequirement::Optional,
            branch: test_read_only_branch("unexpected_schema_relation"),
        })?;

        assert_store_rejection(&response, "PERSISTED_DATA_CORRUPT", "schema_invariant");
        assert_eq!(response.response_value["errors"][0]["category"], "corrupt");
        assert_public_response_has_no_internal_leak(&response, &harness.runtime_home_path);
        Ok(())
    }

    #[test]
    fn committed_mutation_increments_state_version_once() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_commit",
            Some("idem_commit"),
            false,
            Some(0),
            Some(TASK_ID),
        );

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &envelope, "commit"),
            envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("commit"),
        })?;

        let after = harness.counts()?;
        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(
            response.response_value["base"]["effect_kind"],
            "core_committed"
        );
        assert_eq!(response.response_value["base"]["state_version"], 1);
        assert_eq!(
            response.response_value["base"]["events"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.authority_events, before.authority_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        assert_eq!(after.tasks, before.tasks);
        let typed: PipelineTestResult<IntakeResultBase> =
            serde_json::from_str(&response.response_json)?;
        assert_eq!(typed.pipeline_marker, "commit");
        assert_eq!(serde_json::to_value(typed)?, response.response_value);
        Ok(())
    }

    #[test]
    fn owner_current_state_default_is_pinned_by_core_preflight() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_current_state_default",
            Some("idem_current_state_default"),
            false,
            None,
            Some(TASK_ID),
        );
        let request_json = request_json(
            MethodName::ResolveUserAction,
            &envelope,
            "current-state-default",
        );
        let context = harness.mutation.context()?;
        let prepared = match harness.service.prepare_request(
            Some(&context),
            PipelinePreflightRequest {
                method_name: MethodName::ResolveUserAction,
                attempted_action_key: WorkflowActionKey::new(
                    MethodName::ResolveUserAction,
                    volicord_types::values::WorkflowActionSemanticVariant::ResolveUserAction,
                )
                .ok(),
                envelope,
                request_json,
                invocation: invocation_with_actor(
                    ActorSource::LocalUser,
                    OperationCategory::UserOnly,
                ),
                policy: MethodPolicy::exact(
                    OperationCategory::UserOnly,
                    TaskRequirement::Required,
                    ReplayPolicy::Committed,
                    FreshnessPolicy::IfPresent,
                    MethodEffectPolicy::CoreMutation,
                )
                .with_current_state_default(),
            },
        )? {
            PipelinePreflightOutcome::Prepared(prepared) => *prepared,
            PipelinePreflightOutcome::Response(response) => {
                panic!(
                    "current-state default unexpectedly rejected: {}",
                    response.response_json
                )
            }
        };

        assert_eq!(prepared.envelope.expected_state_version.as_ref(), Some(&0));
        let response = harness
            .service
            .execute_prepared_request(prepared, commit_branch("current-state-default"))?;
        assert_eq!(response.response_value["base"]["state_version"], 1);
        Ok(())
    }

    #[test]
    fn idempotency_replay_rejects_untyped_stored_response() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_replay",
            Some("idem_replay"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(MethodName::UpdateScope, &envelope, "replay");
        let request = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json.clone(),
            envelope: envelope.clone(),
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("replay"),
        };

        let first = harness.execute(request.clone())?;
        let after_first = harness.counts()?;
        let second = harness.execute(request)?;
        let after_second = harness.counts()?;

        assert!(!second.replayed);
        assert_eq!(second.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            second.response_value["errors"][0]["code"],
            "PERSISTED_DATA_CORRUPT"
        );
        assert_ne!(second.response_json, first.response_json);
        assert_eq!(after_second, after_first);
        Ok(())
    }

    #[test]
    fn concurrent_replay_outcome_rechecks_current_stored_result_contract(
    ) -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_concurrent_replay_gate",
            Some("idem_concurrent_replay_gate"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(
            MethodName::UpdateScope,
            &envelope,
            "concurrent-replay-private-sentinel",
        );
        let invocation = invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID));
        let branch = commit_branch("concurrent-replay-private-sentinel");
        let policy = MethodPolicy::for_branch(
            OperationCategory::AgentWorkflow,
            TaskRequirement::Required,
            &branch,
        );
        let context = harness.mutation.context()?;
        let prepared = match harness.service.prepare_request(
            Some(&context),
            PipelinePreflightRequest {
                method_name: MethodName::UpdateScope,
                attempted_action_key: WorkflowActionKey::new(
                    MethodName::UpdateScope,
                    volicord_types::values::WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
                )
                .ok(),
                envelope: envelope.clone(),
                request_json: request_json.clone(),
                invocation: invocation.clone(),
                policy,
            },
        )? {
            PipelinePreflightOutcome::Prepared(prepared) => *prepared,
            PipelinePreflightOutcome::Response(response) => {
                panic!("preflight unexpectedly returned {}", response.response_json)
            }
        };

        let concurrent = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json,
            envelope,
            invocation,
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: branch.clone(),
        })?;
        assert_eq!(concurrent.response_value["base"]["response_kind"], "result");
        let after_concurrent = harness.counts()?;

        let replay = harness.service.execute_prepared_request(prepared, branch)?;
        assert_eq!(replay.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            replay.response_value["errors"][0]["code"],
            "PERSISTED_DATA_CORRUPT"
        );
        assert!(!replay.replayed);
        assert!(replay.operation_result_ref.is_none());
        assert!(!replay
            .response_json
            .contains("concurrent-replay-private-sentinel"));
        assert_eq!(harness.counts()?, after_concurrent);
        Ok(())
    }

    #[test]
    fn idempotency_replay_rejects_other_agent_connection_without_stored_response(
    ) -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_replay_connection",
            Some("idem_replay_connection"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(MethodName::UpdateScope, &envelope, "connection-secret");
        let first_request = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json.clone(),
            envelope: envelope.clone(),
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("connection-secret"),
        };
        let first = harness.execute(first_request)?;
        let after_first = harness.counts()?;

        let mismatch = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json,
            envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some("connection_other")),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("connection-secret"),
        })?;

        assert!(!mismatch.replayed);
        assert_eq!(mismatch.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            mismatch.response_value["errors"][0]["code"],
            "INVOCATION_CONTEXT_MISMATCH"
        );
        assert!(!mismatch.response_json.contains("connection-secret"));
        assert_ne!(mismatch.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn idempotency_replay_rejects_other_operation_category() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_replay_category",
            Some("idem_replay_category"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(MethodName::UpdateScope, &envelope, "category-secret");
        let first_request = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json.clone(),
            envelope: envelope.clone(),
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("category-secret"),
        };
        harness.execute(first_request)?;
        let after_first = harness.counts()?;

        let mismatch = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json,
            envelope,
            invocation: invocation(OperationCategory::Read, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("access-secret"),
        })?;

        assert_eq!(
            mismatch.response_value["errors"][0]["code"],
            "INVOCATION_CONTEXT_MISMATCH"
        );
        assert!(!mismatch.response_json.contains("category-secret"));
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn replay_context_mismatch_precedes_request_hash_conflict() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let first_envelope = envelope(
            "req_context_precedence_first",
            Some("idem_context_precedence"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &first_envelope, "stored-secret"),
            envelope: first_envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("stored-secret"),
        })?;
        let after_first = harness.counts()?;

        let second_envelope = envelope(
            "req_context_precedence_second",
            Some("idem_context_precedence"),
            false,
            Some(1),
            Some(TASK_ID),
        );
        let mismatch = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &second_envelope, "different"),
            envelope: second_envelope,
            invocation: invocation(
                OperationCategory::AgentWorkflow,
                Some("connection_hash_mismatch"),
            ),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("different"),
        })?;

        assert_eq!(
            mismatch.response_value["errors"][0]["code"],
            "INVOCATION_CONTEXT_MISMATCH"
        );
        assert!(!mismatch.response_json.contains("stored-secret"));
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn replay_row_without_identity_is_rejected_by_storage() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let error = harness
            .conn()?
            .execute(
                "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                response_json,
                created_at
            )
            VALUES (
                ?1,
                'volicord.update_scope',
                'idem_missing_identity_replay',
                'sha256:missing-identity-replay',
                0,
                1,
                '{\"stored\":\"missing-identity\"}',
                't0'
            )",
                rusqlite::params![PROJECT_ID],
            )
            .expect_err("replay rows require invocation context identity");
        assert_constraint_error(error);
        Ok(())
    }

    #[test]
    fn conflicting_idempotency_key_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let first_envelope = envelope(
            "req_conflict_first",
            Some("idem_conflict"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let first = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &first_envelope, "first"),
            envelope: first_envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("first"),
        };
        harness.execute(first)?;
        let before_conflict = harness.counts()?;

        let second_envelope = envelope(
            "req_conflict_second",
            Some("idem_conflict"),
            false,
            Some(1),
            Some(TASK_ID),
        );
        let second = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &second_envelope, "second"),
            envelope: second_envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("second"),
        };

        let response = harness.execute(second)?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert_eq!(harness.counts()?, before_conflict);
        Ok(())
    }

    #[test]
    fn unexpected_uniqueness_failure_is_an_operational_core_error() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let first_envelope = envelope(
            "req_unique_first",
            Some("idem_unique_first"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &first_envelope, "unique-first"),
            envelope: first_envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: change_unit_commit_branch("change_unit_unique_first", "unique_first"),
        })?;
        let after_first = harness.counts()?;

        let second_envelope = envelope(
            "req_unique_second",
            Some("idem_unique_second"),
            false,
            Some(1),
            Some(TASK_ID),
        );
        let error = harness
            .execute_with_action(
                PipelineRequest {
                    method_name: MethodName::UpdateScope,
                    request_json: update_scope_request_json(
                        &second_envelope,
                        "unique-second",
                        "keep_current",
                    ),
                    envelope: second_envelope,
                    invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
                    operation_category: OperationCategory::AgentWorkflow,
                    task_requirement: TaskRequirement::Required,
                    branch: change_unit_commit_branch("change_unit_unique_second", "unique_second"),
                },
                volicord_types::values::WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
            )
            .expect_err("Store constraint failure must not produce a method response");

        assert_operational_unavailable(error, CoreOperationalResource::Store, false);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn stale_expected_state_version_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_stale",
            Some("idem_stale"),
            false,
            Some(7),
            Some(TASK_ID),
        );

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &envelope, "stale"),
            envelope,
            invocation: invocation(OperationCategory::AgentWorkflow, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("stale"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn operation_category_mismatch_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope(
            "req_access",
            Some("idem_access"),
            false,
            Some(0),
            Some(TASK_ID),
        );

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json(MethodName::UpdateScope, &envelope, "access-mismatch"),
            envelope,
            invocation: invocation(OperationCategory::Read, Some(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("access_mismatch"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "INVOCATION_CONTEXT_MISMATCH"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    fn envelope(
        request_id: &str,
        idempotency_key: Option<&str>,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: Option<&str>,
    ) -> ToolEnvelope {
        ToolEnvelope {
            project_id: ProjectId::new(PROJECT_ID),
            task_id: task_id.map(TaskId::new).into(),
            request_id: RequestId::new(request_id),
            idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
            expected_state_version: expected_state_version.into(),
            dry_run: DryRunIntent::from_wire_bool(dry_run),
            locale: None.into(),
        }
    }

    fn invocation(
        operation_category: OperationCategory,
        connection_id: Option<&str>,
    ) -> InvocationContext {
        invocation_with_actor(
            ActorSource::agent_connection(connection_id.unwrap_or(CONNECTION_ID)),
            operation_category,
        )
    }

    fn invocation_with_actor(
        actor_source: ActorSource,
        operation_category: OperationCategory,
    ) -> InvocationContext {
        match actor_source {
            ActorSource::AgentConnection(connection_id) => InvocationContext::agent_connection(
                operation_category,
                crate::agent_session::validated_agent_session_for_test(
                    connection_id.as_str(),
                    PROJECT_ID,
                ),
            ),
            ActorSource::LocalUser => InvocationContext::local_user(
                ProjectId::new(PROJECT_ID),
                operation_category,
                UserActionChannelKind::Cli,
            ),
            ActorSource::System => panic!("system authority is not a public Core invocation input"),
        }
    }

    fn request_json(method_name: MethodName, envelope: &ToolEnvelope, marker: &str) -> Value {
        match method_name {
            MethodName::UpdateScope => {
                update_scope_request_json(envelope, marker, "create_current")
            }
            MethodName::CloseTask => json!({
                "envelope": envelope,
                "task_id": TASK_ID,
                "intent": "cancel",
                "close_reason": "cancelled",
                "superseding_task_id": null,
                "user_note": marker,
            }),
            _ => json!({
                "method": method_name.as_str(),
                "envelope": envelope,
                "pipeline_marker": marker
            }),
        }
    }

    fn update_scope_request_json(envelope: &ToolEnvelope, marker: &str, operation: &str) -> Value {
        json!({
            "envelope": envelope,
            "task_id": TASK_ID,
            "goal_summary": marker,
            "scope_update": null,
            "scope_boundary": null,
            "non_goals": null,
            "acceptance_criteria": null,
            "autonomy_boundary": null,
            "baseline_ref": null,
            "change_unit": { "operation": operation },
            "related_scope_decision_refs": [],
        })
    }

    fn result_fields(marker: &str) -> PipelineTestResultFields {
        PipelineTestResultFields {
            pipeline_marker: marker.to_owned(),
        }
    }

    fn event_payload(marker: &str) -> JsonObject {
        let mut fields = Map::new();
        fields.insert(
            "pipeline_marker".to_owned(),
            Value::String(marker.to_owned()),
        );
        fields
    }

    fn test_preview_branch(
    ) -> OwnerPipelineBranch<PipelineTestResultFields, ReconcileChangesResultBase> {
        OwnerPipelineBranch {
            kind: OwnerPipelineBranchKind::DryRunPreview {
                dry_run_summary: dry_run_summary(),
            },
        }
    }

    fn test_read_only_branch(
        marker: &str,
    ) -> OwnerPipelineBranch<PipelineTestResultFields, StatusResultBase> {
        OwnerPipelineBranch {
            kind: OwnerPipelineBranchKind::ReadOnly {
                result_fields: result_fields(marker),
                build_base: StatusRequest::read_only_result_base,
            },
        }
    }

    fn test_no_effect_branch(
        marker: &str,
    ) -> OwnerPipelineBranch<PipelineTestResultFields, CloseTaskResultBase> {
        OwnerPipelineBranch {
            kind: OwnerPipelineBranchKind::NoEffectResult {
                result_fields: result_fields(marker),
                build_base: CloseTaskRequest::no_effect_result_base,
            },
        }
    }

    fn commit_branch(
        marker: &str,
    ) -> OwnerPipelineBranch<PipelineTestResultFields, IntakeResultBase> {
        change_unit_commit_branch(&format!("change_unit_{marker}"), marker)
    }

    fn change_unit_commit_branch(
        change_unit_id: &str,
        marker: &str,
    ) -> OwnerPipelineBranch<PipelineTestResultFields, IntakeResultBase> {
        OwnerPipelineBranch {
            kind: OwnerPipelineBranchKind::CommitMutation {
                result_fields: result_fields(marker),
                event_kind: "core.pipeline_test_commit".to_owned(),
                event_payload: event_payload(marker),
                task_id: None,
                change_unit_id: None,
                storage_mutations: vec![
                    CoreStorageMutation::Task(TaskMutation::UpdateScope(TaskScopeUpdate {
                        task_id: TASK_ID.to_owned(),
                        work_phase: None,
                        lifecycle_phase: None,
                        result: None,
                        title: None,
                        summary: None,
                        shaping: None,
                        bounded_context: None,
                        autonomy_boundary: None,
                        close_summary: None,
                    })),
                    CoreStorageMutation::ChangeUnit(
                        volicord_store::core_pipeline::ChangeUnitMutation::InsertCurrent(
                            ChangeUnitInsert {
                                change_unit_id: change_unit_id.to_owned(),
                                task_id: TASK_ID.to_owned(),
                                scope_summary: StoredChangeUnitScopeSummary {
                                    scope_summary: Some(marker.to_owned()),
                                    affected_areas: Vec::new(),
                                    constraints: Vec::new(),
                                },
                                bounded_paths: Vec::new(),
                                write_basis: StoredChangeUnitWriteBasis {
                                    baseline_ref: None,
                                    git_workspace_context: None,
                                },
                                effect_contract: None,
                                lifecycle: StoredChangeUnitLifecycle {
                                    recovery_required: false,
                                },
                            },
                        ),
                    ),
                ],
                build_base: IntakeRequest::core_committed_result_base,
            },
        }
    }

    fn assert_store_rejection(
        response: &PipelineResponse,
        expected_code: &str,
        expected_category: &str,
    ) {
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(response.response_value["errors"][0]["code"], expected_code);
        assert_eq!(
            response.response_value["errors"][0]["details"]["store_failure_category"],
            expected_category
        );
    }

    fn assert_operational_unavailable(
        error: CorePipelineError,
        expected_resource: CoreOperationalResource,
        expected_retryable: bool,
    ) {
        let CorePipelineError::OperationalUnavailable(failure) = error else {
            panic!("expected operational unavailability, got {error}");
        };
        assert_eq!(failure.operation(), CoreOperationalOperation::StoreAccess);
        assert_eq!(failure.resource(), expected_resource);
        assert_eq!(failure.retryable(), expected_retryable);
    }

    #[test]
    fn typed_product_path_diagnostic_selects_the_core_operational_route() {
        assert_eq!(
            product_path_operational_route(PlatformDiagnosticKind::ProductRepositoryNotFound),
            (
                CoreOperationalOperation::ProductPathObservation,
                CoreOperationalResource::ProductRepository,
                true,
            )
        );
        assert_eq!(
            product_path_operational_route(PlatformDiagnosticKind::ProductPathContainmentFailure),
            (
                CoreOperationalOperation::ProductPathObservation,
                CoreOperationalResource::ProductRepository,
                false,
            )
        );
    }

    fn assert_constraint_error(error: rusqlite::Error) {
        match error {
            rusqlite::Error::SqliteFailure(err, _) => assert_eq!(
                err.code,
                rusqlite::ErrorCode::ConstraintViolation,
                "expected SQLite constraint error, got {err:?}"
            ),
            other => panic!("expected SQLite constraint error, got {other:?}"),
        }
    }

    fn assert_public_response_has_no_internal_leak(
        response: &PipelineResponse,
        runtime_home_path: &Path,
    ) {
        let body = &response.response_json;
        let runtime_home = runtime_home_path.to_string_lossy();
        assert!(!body.contains(runtime_home.as_ref()));
        for fragment in [
            "SELECT ",
            "INSERT INTO",
            "UPDATE ",
            "DELETE ",
            "constraint failed",
            "state.sqlite",
        ] {
            assert!(
                !body.contains(fragment),
                "public response leaked internal fragment {fragment}: {body}"
            );
        }
    }

    fn dry_run_summary() -> DryRunSummary {
        DryRunSummary {
            planned_effects: Vec::<PlannedEffect>::new(),
            would_blockers: Vec::new(),
            would_errors: Vec::new(),
            next_actions: Vec::new(),
            diagnostics: vec!["pipeline test dry-run".to_owned()],
        }
    }
}
