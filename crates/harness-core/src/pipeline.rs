use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use harness_store::{
    bootstrap::SurfaceRecord,
    core_pipeline::{
        commit_input, CommitMutationInput, CommittedEventRef, CoreProjectStore,
        CoreStorageMutation, MutationCommitOutcome, PendingTaskEvent, ProjectStateHeader,
        VerifiedReplayContext,
    },
    StoreError,
};
use harness_types::{
    canonical_request_hash, AccessClass, ChangeUnitId, DryRunSummary, DurableIdError,
    DurableIdGenerator, DurableIdKind, EffectKind, ErrorCode, EventId, EventRef, IdempotencyKey,
    JsonObject, MethodName, ProjectId, RandomDurableIdGenerator, RequestHash, ResponseKind,
    SurfaceId, SurfaceInstanceId, TaskId, ToolDryRunResponse, ToolEnvelope, ToolError,
    ToolRejectedResponse, ToolResultBase, DURABLE_ID_RETRY_LIMIT,
};
use serde_json::{Map, Value};

/// Result type for Core pipeline execution errors.
pub type CoreResult<T> = Result<T, CorePipelineError>;

/// Errors that indicate implementation or storage failure outside public API rejection routing.
#[derive(Debug)]
pub enum CorePipelineError {
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
}

impl fmt::Display for CorePipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl Error for CorePipelineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Store(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::DurableId(error) => Some(error),
            Self::GeneratedIdCollision { .. } | Self::InvalidDispatch { .. } => None,
        }
    }
}

impl From<StoreError> for CorePipelineError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
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

/// Local invocation facts supplied by an adapter outside `ToolEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationContext {
    pub surface_instance_id: Option<SurfaceInstanceId>,
    pub requested_access_class: AccessClass,
    pub invocation_binding_basis: String,
}

/// Internal verified surface context derived for one invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedSurfaceContext {
    pub project_id: ProjectId,
    pub surface_id: SurfaceId,
    pub surface_instance_id: SurfaceInstanceId,
    pub access_class: AccessClass,
    pub capability_profile: Value,
    pub verification_basis: String,
}

/// Task selector behavior required by the owner-selected branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRequirement {
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

/// Method access behavior after the invocation has a verified registered grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodAccessPolicy {
    Exact(AccessClass),
    VerifiedGrantOnly,
}

/// Storage/effect family selected before method-specific planning runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodEffectPolicy {
    ReadOnly,
    NoEffect,
    Staging,
    DryRunPreview,
    CoreMutation,
}

/// Authoritative preflight policy for a public method branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodPolicy {
    pub(crate) access: MethodAccessPolicy,
    pub(crate) task: TaskRequirement,
    pub(crate) replay: ReplayPolicy,
    pub(crate) freshness: FreshnessPolicy,
    pub(crate) effect: MethodEffectPolicy,
}

impl MethodPolicy {
    pub(crate) fn exact(
        access_class: AccessClass,
        task: TaskRequirement,
        replay: ReplayPolicy,
        freshness: FreshnessPolicy,
        effect: MethodEffectPolicy,
    ) -> Self {
        Self {
            access: MethodAccessPolicy::Exact(access_class),
            task,
            replay,
            freshness,
            effect,
        }
    }

    pub(crate) fn verified_grant_only(
        task: TaskRequirement,
        replay: ReplayPolicy,
        freshness: FreshnessPolicy,
        effect: MethodEffectPolicy,
    ) -> Self {
        Self {
            access: MethodAccessPolicy::VerifiedGrantOnly,
            task,
            replay,
            freshness,
            effect,
        }
    }

    fn for_branch(
        access_class: AccessClass,
        task: TaskRequirement,
        branch: &OwnerPipelineBranch,
    ) -> Self {
        match branch {
            OwnerPipelineBranch::ReadOnly { .. } => Self::exact(
                access_class,
                task,
                ReplayPolicy::None,
                FreshnessPolicy::None,
                MethodEffectPolicy::ReadOnly,
            ),
            OwnerPipelineBranch::NoEffectResult { .. } => Self::exact(
                access_class,
                task,
                ReplayPolicy::None,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::NoEffect,
            ),
            OwnerPipelineBranch::DryRunPreview { .. } => Self::exact(
                access_class,
                task,
                ReplayPolicy::None,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::DryRunPreview,
            ),
            OwnerPipelineBranch::CommitMutation { .. } => Self::exact(
                access_class,
                task,
                ReplayPolicy::Committed,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::CoreMutation,
            ),
        }
    }
}

/// Owner-selected branch shape used by the shared pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum OwnerPipelineBranch {
    ReadOnly {
        result_fields: JsonObject,
    },
    NoEffectResult {
        result_fields: JsonObject,
    },
    DryRunPreview {
        dry_run_summary: DryRunSummary,
    },
    CommitMutation {
        result_fields: JsonObject,
        event_kind: String,
        event_payload: JsonObject,
        task_id: Option<TaskId>,
        change_unit_id: Option<ChangeUnitId>,
        storage_mutations: Vec<CoreStorageMutation>,
    },
}

/// Input to the shared Core request pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineRequest {
    pub method_name: MethodName,
    pub envelope: ToolEnvelope,
    pub request_json: Value,
    pub invocation: InvocationContext,
    pub required_access_class: AccessClass,
    pub task_requirement: TaskRequirement,
    pub branch: OwnerPipelineBranch,
}

/// Input to the shared preflight boundary before method-specific planning.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PipelinePreflightRequest {
    pub method_name: MethodName,
    pub envelope: ToolEnvelope,
    pub request_json: Value,
    pub invocation: InvocationContext,
    pub policy: MethodPolicy,
}

/// Verified request context produced by the shared preflight boundary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerifiedRequestContext {
    pub project_state: ProjectStateHeader,
    pub verified_surface: VerifiedSurfaceContext,
    pub resolved_task_id: Option<TaskId>,
}

/// Store-backed request prepared for method-specific planning or effect routing.
pub(crate) struct PreparedRequest {
    pub method_name: MethodName,
    pub envelope: ToolEnvelope,
    pub request_hash: RequestHash,
    pub store: CoreProjectStore,
    pub context: VerifiedRequestContext,
}

/// Preflight may either prepare a request or return an authoritative response.
pub(crate) enum PipelinePreflightOutcome {
    Prepared(Box<PreparedRequest>),
    Response(Box<PipelineResponse>),
}

/// Shared pipeline response with exact stored JSON when replayed.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineResponse {
    pub response_json: String,
    pub response_value: Value,
    pub verified_surface: Option<VerifiedSurfaceContext>,
    pub resolved_task_id: Option<TaskId>,
    pub replayed: bool,
}

/// Core request pipeline service bound to a local Runtime Home.
#[derive(Clone)]
pub struct CoreService {
    runtime_home: PathBuf,
    id_generator: Arc<dyn DurableIdGenerator>,
}

impl fmt::Debug for CoreService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreService")
            .field("runtime_home", &self.runtime_home)
            .field("id_generator", &self.id_generator)
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
    /// Creates a service that reads and writes Core records under `runtime_home`.
    pub fn new(runtime_home: impl AsRef<Path>) -> Self {
        Self::with_id_generator(runtime_home, RandomDurableIdGenerator)
    }

    /// Creates a service with an injected durable ID generator.
    pub fn with_id_generator(
        runtime_home: impl AsRef<Path>,
        id_generator: impl DurableIdGenerator + 'static,
    ) -> Self {
        Self {
            runtime_home: runtime_home.as_ref().to_path_buf(),
            id_generator: Arc::new(id_generator),
        }
    }

    pub(crate) fn allocate_generated_id(
        &self,
        kind: DurableIdKind,
        mut exists: impl FnMut(&str) -> CoreResult<bool>,
    ) -> CoreResult<String> {
        for _ in 0..DURABLE_ID_RETRY_LIMIT {
            let candidate = self.id_generator.generate(kind)?;
            if !exists(&candidate)? {
                return Ok(candidate);
            }
        }

        Err(CorePipelineError::GeneratedIdCollision {
            kind,
            attempts: DURABLE_ID_RETRY_LIMIT,
        })
    }

    /// Runs the shared envelope, context, freshness, replay, and effect pipeline.
    pub fn execute_pipeline(&self, request: PipelineRequest) -> CoreResult<PipelineResponse> {
        validate_branch_shape(&request.branch, request.envelope.dry_run)?;
        let policy = MethodPolicy::for_branch(
            request.required_access_class,
            request.task_requirement,
            &request.branch,
        );
        let preflight = PipelinePreflightRequest {
            method_name: request.method_name,
            envelope: request.envelope,
            request_json: request.request_json,
            invocation: request.invocation,
            policy,
        };
        match self.prepare_request(preflight)? {
            PipelinePreflightOutcome::Prepared(prepared) => {
                self.execute_prepared_request(*prepared, request.branch)
            }
            PipelinePreflightOutcome::Response(response) => Ok(*response),
        }
    }

    /// Runs the authoritative preflight sequence before method-specific planning.
    pub(crate) fn prepare_request(
        &self,
        request: PipelinePreflightRequest,
    ) -> CoreResult<PipelinePreflightOutcome> {
        let envelope_errors = validate_envelope(&request.envelope, &request.request_json);
        if !envelope_errors.is_empty() {
            return response_outcome_from_rejected(
                rejected_response(request.envelope.dry_run, None, envelope_errors),
                None,
                None,
            );
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

        let request_hash = canonical_request_hash(&request.request_json)?;

        let store = match CoreProjectStore::open(&self.runtime_home, &request.envelope.project_id) {
            Ok(store) => store,
            Err(error) => {
                return response_outcome_from_rejected(
                    rejected_response(
                        request.envelope.dry_run,
                        None,
                        vec![store_unavailable_error(error)],
                    ),
                    None,
                    None,
                );
            }
        };

        let project_state = match store.project_state() {
            Ok(project_state) => project_state,
            Err(error) => {
                return response_outcome_from_rejected(
                    rejected_response(
                        request.envelope.dry_run,
                        None,
                        vec![store_unavailable_error(error)],
                    ),
                    None,
                    None,
                );
            }
        };

        let verified_surface = match derive_verified_surface(
            &store,
            &project_state,
            &request.envelope,
            &request.invocation,
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

        if let Some(replay_response) = replay_preflight_response(
            &store,
            &request,
            &request_hash,
            &project_state,
            &verified_surface,
        )? {
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
            Err(error) => {
                return response_outcome_from_rejected(
                    rejected_response(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        vec![error],
                    ),
                    Some(verified_surface),
                    None,
                );
            }
        };

        if let Some(freshness_response) = freshness_preflight_response(
            &request,
            &project_state,
            Some(verified_surface.clone()),
            resolved_task_id.clone(),
        )? {
            return Ok(PipelinePreflightOutcome::Response(Box::new(
                freshness_response,
            )));
        }

        if let Some(error) = method_access_error(request.policy.access, &verified_surface) {
            return response_outcome_from_rejected(
                rejected_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    vec![error],
                ),
                Some(verified_surface),
                resolved_task_id,
            );
        }

        Ok(PipelinePreflightOutcome::Prepared(Box::new(
            PreparedRequest {
                method_name: request.method_name,
                envelope: request.envelope,
                request_hash,
                store,
                context: VerifiedRequestContext {
                    project_state,
                    verified_surface,
                    resolved_task_id,
                },
            },
        )))
    }

    /// Routes a prepared request to the selected storage/effect branch.
    pub(crate) fn execute_prepared_request(
        &self,
        mut prepared: PreparedRequest,
        branch: OwnerPipelineBranch,
    ) -> CoreResult<PipelineResponse> {
        validate_branch_shape(&branch, prepared.envelope.dry_run)?;
        let project_state = prepared.context.project_state.clone();
        let verified_surface = prepared.context.verified_surface.clone();
        let resolved_task_id = prepared.context.resolved_task_id.clone();

        match branch {
            OwnerPipelineBranch::ReadOnly { result_fields } => {
                let base = method_result_base(
                    EffectKind::ReadOnly,
                    prepared.envelope.dry_run,
                    Some(project_state.state_version),
                    Vec::new(),
                );
                response_from_value(
                    method_result_value(base, result_fields)?,
                    Some(verified_surface),
                    resolved_task_id,
                    false,
                )
            }
            OwnerPipelineBranch::NoEffectResult { result_fields } => {
                let base = method_result_base(
                    EffectKind::NoEffect,
                    false,
                    Some(project_state.state_version),
                    Vec::new(),
                );
                response_from_value(
                    method_result_value(base, result_fields)?,
                    Some(verified_surface),
                    resolved_task_id,
                    false,
                )
            }
            OwnerPipelineBranch::DryRunPreview { dry_run_summary } => response_from_dry_run(
                dry_run_response(Some(project_state.state_version), dry_run_summary),
                Some(verified_surface),
                resolved_task_id,
            ),
            OwnerPipelineBranch::CommitMutation {
                result_fields,
                event_kind,
                event_payload,
                task_id: branch_task_id,
                change_unit_id,
                storage_mutations,
            } => {
                let task_id = match branch_task_id.or(resolved_task_id) {
                    Some(task_id) => task_id,
                    None => {
                        return response_from_rejected(
                            rejected_response(
                                false,
                                Some(project_state.state_version),
                                vec![no_active_task_error()],
                            ),
                            Some(verified_surface),
                            None,
                        );
                    }
                };
                let event_id = self.allocate_generated_id(DurableIdKind::Event, |candidate| {
                    prepared
                        .store
                        .event_id_exists(candidate)
                        .map_err(CorePipelineError::from)
                })?;
                commit_mutation(
                    &mut prepared.store,
                    CommitPipelineArgs {
                        envelope: &prepared.envelope,
                        method_name: prepared.method_name,
                        request_hash: &prepared.request_hash,
                        event_id,
                        result_fields,
                        event_kind,
                        event_payload,
                        change_unit_id,
                        storage_mutations,
                        task_id: &task_id,
                        verified_surface,
                    },
                )
            }
        }
    }
}

/// Builds a common method-result base.
pub fn method_result_base(
    effect_kind: EffectKind,
    dry_run: bool,
    state_version: Option<u64>,
    events: Vec<EventRef>,
) -> ToolResultBase {
    ToolResultBase {
        response_kind: ResponseKind::Result,
        effect_kind,
        dry_run,
        state_version,
        events,
    }
}

/// Builds a method result JSON object by adding the common `base` field.
pub fn method_result_value(
    base: ToolResultBase,
    mut result_fields: JsonObject,
) -> CoreResult<Value> {
    if result_fields.contains_key("base") {
        return Err(CorePipelineError::InvalidDispatch {
            detail: "method result fields must not contain base".to_owned(),
        });
    }
    result_fields.insert("base".to_owned(), serde_json::to_value(base)?);
    Ok(Value::Object(result_fields))
}

/// Builds a rejected response and applies public error precedence.
pub fn rejected_response(
    dry_run: bool,
    state_version: Option<u64>,
    mut errors: Vec<ToolError>,
) -> ToolRejectedResponse {
    errors.sort_by_key(|error| error_precedence(error.code));
    ToolRejectedResponse {
        base: ToolResultBase {
            response_kind: ResponseKind::Rejected,
            effect_kind: EffectKind::NoEffect,
            dry_run,
            state_version,
            events: Vec::new(),
        },
        errors,
    }
}

/// Builds a dry-run preview response.
pub fn dry_run_response(
    state_version: Option<u64>,
    dry_run_summary: DryRunSummary,
) -> ToolDryRunResponse {
    ToolDryRunResponse {
        base: ToolResultBase {
            response_kind: ResponseKind::DryRun,
            effect_kind: EffectKind::NoEffect,
            dry_run: true,
            state_version,
            events: Vec::new(),
        },
        dry_run_summary,
    }
}

/// Builds a public API error item.
pub fn tool_error(
    code: ErrorCode,
    message: impl Into<String>,
    retryable: bool,
    details: Option<JsonObject>,
) -> ToolError {
    ToolError {
        code,
        message: message.into(),
        retryable,
        details,
    }
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
    if let Some(task_id) = &envelope.task_id {
        if task_id.as_str().trim().is_empty() {
            errors.push(validation_error("task_id", "task_id must not be empty"));
        }
    }
    if envelope.surface_id.as_str().trim().is_empty() {
        errors.push(validation_error(
            "surface_id",
            "surface_id must not be empty",
        ));
    }
    if envelope.request_id.as_str().trim().is_empty() {
        errors.push(validation_error(
            "request_id",
            "request_id must not be empty",
        ));
    }
    if let Some(idempotency_key) = &envelope.idempotency_key {
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
    if envelope.dry_run || policy.effect != MethodEffectPolicy::CoreMutation {
        return Vec::new();
    }
    if envelope.idempotency_key.is_none() {
        return vec![validation_error(
            "idempotency_key",
            "committed mutations require idempotency_key",
        )];
    }
    if envelope.expected_state_version.is_none() {
        return vec![validation_error(
            "expected_state_version",
            "committed mutations require expected_state_version",
        )];
    }
    Vec::new()
}

fn validate_branch_shape(branch: &OwnerPipelineBranch, dry_run: bool) -> CoreResult<()> {
    match (branch, dry_run) {
        (OwnerPipelineBranch::ReadOnly { result_fields }, _) => ensure_no_base_field(result_fields),
        (OwnerPipelineBranch::NoEffectResult { result_fields }, false) => {
            ensure_no_base_field(result_fields)
        }
        (OwnerPipelineBranch::NoEffectResult { .. }, true) => {
            Err(CorePipelineError::InvalidDispatch {
                detail: "no-effect result branch requires ToolEnvelope.dry_run=false".to_owned(),
            })
        }
        (OwnerPipelineBranch::DryRunPreview { .. }, true) => Ok(()),
        (OwnerPipelineBranch::DryRunPreview { .. }, false) => {
            Err(CorePipelineError::InvalidDispatch {
                detail: "dry-run preview branch requires ToolEnvelope.dry_run=true".to_owned(),
            })
        }
        (
            OwnerPipelineBranch::CommitMutation {
                result_fields,
                event_kind,
                ..
            },
            false,
        ) => {
            ensure_no_base_field(result_fields)?;
            if event_kind.trim().is_empty() {
                return Err(CorePipelineError::InvalidDispatch {
                    detail: "committed mutation event_kind must not be empty".to_owned(),
                });
            }
            Ok(())
        }
        (OwnerPipelineBranch::CommitMutation { .. }, true) => {
            Err(CorePipelineError::InvalidDispatch {
                detail: "commit branch requires ToolEnvelope.dry_run=false".to_owned(),
            })
        }
    }
}

fn ensure_no_base_field(result_fields: &JsonObject) -> CoreResult<()> {
    if result_fields.contains_key("base") {
        Err(CorePipelineError::InvalidDispatch {
            detail: "method result fields must not contain base".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn replay_preflight_response(
    store: &CoreProjectStore,
    request: &PipelinePreflightRequest,
    request_hash: &RequestHash,
    project_state: &ProjectStateHeader,
    verified_surface: &VerifiedSurfaceContext,
) -> CoreResult<Option<PipelineResponse>> {
    if request.policy.replay != ReplayPolicy::Committed || request.envelope.dry_run {
        return Ok(None);
    }
    let Some(idempotency_key) = &request.envelope.idempotency_key else {
        return Ok(None);
    };
    let Some(record) = store.tool_invocation(request.method_name, idempotency_key)? else {
        return Ok(None);
    };

    let replay_context = replay_context_from_verified_surface(verified_surface);
    if !record.matches_verified_replay_context(&replay_context) {
        return Ok(Some(response_from_rejected(
            replay_context_mismatch_response(request.envelope.dry_run, project_state.state_version),
            Some(verified_surface.clone()),
            None,
        )?));
    }
    if record.request_hash == request_hash.as_str() {
        return Ok(Some(response_from_json_string(
            record.response_json,
            Some(verified_surface.clone()),
            None,
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
        Some(verified_surface.clone()),
        None,
    )?))
}

fn freshness_preflight_response(
    request: &PipelinePreflightRequest,
    project_state: &ProjectStateHeader,
    verified_surface: Option<VerifiedSurfaceContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<Option<PipelineResponse>> {
    if request.policy.freshness == FreshnessPolicy::None {
        return Ok(None);
    }
    let Some(expected_state_version) = request.envelope.expected_state_version else {
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
        verified_surface,
        resolved_task_id,
    )?))
}

fn method_access_error(
    policy: MethodAccessPolicy,
    verified_surface: &VerifiedSurfaceContext,
) -> Option<ToolError> {
    match policy {
        MethodAccessPolicy::Exact(required_access_class)
            if verified_surface.access_class != required_access_class =>
        {
            Some(capability_insufficient_error(
                required_access_class,
                verified_surface.access_class,
            ))
        }
        MethodAccessPolicy::Exact(_) | MethodAccessPolicy::VerifiedGrantOnly => None,
    }
}

fn derive_verified_surface(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    invocation: &InvocationContext,
) -> Result<VerifiedSurfaceContext, ToolError> {
    let surface = if let Some(surface_instance_id) = &invocation.surface_instance_id {
        store
            .surface(&envelope.surface_id, surface_instance_id.as_str())
            .map_err(|_| mcp_unavailable_error("surface lookup failed"))?
            .ok_or_else(|| local_access_mismatch_error("surface_instance_id"))?
    } else if project_state.default_surface_id.as_deref() == Some(envelope.surface_id.as_str()) {
        let default_instance = project_state
            .default_surface_instance_id
            .as_deref()
            .ok_or_else(|| local_access_mismatch_error("default_surface_instance_id"))?;
        store
            .surface(&envelope.surface_id, default_instance)
            .map_err(|_| mcp_unavailable_error("surface lookup failed"))?
            .ok_or_else(|| local_access_mismatch_error("default_surface_instance_id"))?
    } else {
        let candidates = store
            .surface_candidates(&envelope.surface_id)
            .map_err(|_| mcp_unavailable_error("surface lookup failed"))?;
        if candidates.len() == 1 {
            candidates
                .into_iter()
                .next()
                .ok_or_else(|| local_access_mismatch_error("surface_id"))?
        } else {
            return Err(local_access_mismatch_error("surface_id"));
        }
    };

    let capability_profile = serde_json::from_str::<Value>(&surface.capability_profile_json)
        .map_err(|_| mcp_unavailable_error("surface capability profile is invalid"))?;

    verified_surface_from_registered_surface(surface, invocation, capability_profile)
        .map_err(|_| local_access_mismatch_error("surfaces.local_access_json"))
}

pub(crate) fn verified_surface_from_registered_surface(
    surface: SurfaceRecord,
    invocation: &InvocationContext,
    capability_profile: Value,
) -> Result<VerifiedSurfaceContext, ()> {
    let grant = parse_registered_local_access_grant(&surface.local_access_json)?;
    if !grant
        .authorized_access_classes
        .contains(&invocation.requested_access_class)
    {
        return Err(());
    }

    Ok(VerifiedSurfaceContext {
        project_id: ProjectId::new(surface.project_id),
        surface_id: SurfaceId::new(surface.surface_id),
        surface_instance_id: SurfaceInstanceId::new(surface.surface_instance_id),
        access_class: invocation.requested_access_class,
        capability_profile,
        verification_basis: verified_surface_basis(
            &grant.verification_basis,
            &invocation.invocation_binding_basis,
        ),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredLocalAccessGrant {
    pub authorized_access_classes: Vec<AccessClass>,
    pub verification_basis: String,
}

pub(crate) fn parse_registered_local_access_grant(
    text: &str,
) -> Result<RegisteredLocalAccessGrant, ()> {
    let value = serde_json::from_str::<Value>(text).map_err(|_| ())?;
    let object = value.as_object().ok_or(())?;
    let authorized_access_classes = if let Some(value) = object.get("authorized_access_classes") {
        parse_authorized_access_classes(value)?
    } else {
        vec![parse_access_class_field(
            object.get("access_class").ok_or(())?,
        )?]
    };

    if let Some(value) = object.get("access_class") {
        let fallback_access_class = parse_access_class_field(value)?;
        if !authorized_access_classes.contains(&fallback_access_class) {
            return Err(());
        }
    }

    let verification_basis = match object.get("verification_basis") {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        Some(_) => return Err(()),
        None => "registered_local_access".to_owned(),
    };

    Ok(RegisteredLocalAccessGrant {
        authorized_access_classes,
        verification_basis,
    })
}

fn parse_authorized_access_classes(value: &Value) -> Result<Vec<AccessClass>, ()> {
    let values = value.as_array().ok_or(())?;
    if values.is_empty() {
        return Err(());
    }

    let mut access_classes = Vec::new();
    for value in values {
        let access_class = parse_access_class_field(value)?;
        if !access_classes.contains(&access_class) {
            access_classes.push(access_class);
        }
    }
    if access_classes.is_empty() {
        return Err(());
    }
    Ok(access_classes)
}

fn parse_access_class_field(value: &Value) -> Result<AccessClass, ()> {
    let value = value.as_str().ok_or(())?;
    if value.trim().is_empty() {
        return Err(());
    }
    match value {
        "read_status" => Ok(AccessClass::ReadStatus),
        "core_mutation" => Ok(AccessClass::CoreMutation),
        "write_authorization" => Ok(AccessClass::WriteAuthorization),
        "run_recording" => Ok(AccessClass::RunRecording),
        "artifact_registration" => Ok(AccessClass::ArtifactRegistration),
        "artifact_read" => Ok(AccessClass::ArtifactRead),
        _ => Err(()),
    }
}

fn verified_surface_basis(registered_basis: &str, invocation_binding_basis: &str) -> String {
    let invocation_binding_basis = invocation_binding_basis.trim();
    if invocation_binding_basis.is_empty() {
        registered_basis.to_owned()
    } else {
        format!("{registered_basis}; invocation_binding_basis={invocation_binding_basis}")
    }
}

pub(crate) fn replay_context_from_verified_surface(
    verified_surface: &VerifiedSurfaceContext,
) -> VerifiedReplayContext {
    VerifiedReplayContext {
        surface_id: verified_surface.surface_id.as_str().to_owned(),
        surface_instance_id: verified_surface.surface_instance_id.as_str().to_owned(),
        access_class: access_class_value(verified_surface.access_class).to_owned(),
        verification_basis: (!verified_surface.verification_basis.trim().is_empty())
            .then(|| verified_surface.verification_basis.clone()),
    }
}

pub(crate) fn access_class_value(access_class: AccessClass) -> &'static str {
    match access_class {
        AccessClass::ReadStatus => "read_status",
        AccessClass::CoreMutation => "core_mutation",
        AccessClass::WriteAuthorization => "write_authorization",
        AccessClass::RunRecording => "run_recording",
        AccessClass::ArtifactRegistration => "artifact_registration",
        AccessClass::ArtifactRead => "artifact_read",
    }
}

fn resolve_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    requirement: TaskRequirement,
) -> Result<Option<TaskId>, ToolError> {
    match requirement {
        TaskRequirement::None => Ok(None),
        TaskRequirement::Optional => match &envelope.task_id {
            Some(task_id) => ensure_task_exists(store, task_id).map(Some),
            None => Ok(None),
        },
        TaskRequirement::Required => {
            if let Some(task_id) = &envelope.task_id {
                return ensure_task_exists(store, task_id).map(Some);
            }

            let active_task_id = project_state
                .active_task_id
                .as_ref()
                .ok_or_else(no_active_task_error)?;
            let task_id = TaskId::new(active_task_id.clone());
            ensure_task_exists(store, &task_id).map(Some)
        }
        TaskRequirement::Exact(task_id) => ensure_task_exists(store, &task_id).map(Some),
    }
}

fn ensure_task_exists(store: &CoreProjectStore, task_id: &TaskId) -> Result<TaskId, ToolError> {
    match store.task_exists(task_id) {
        Ok(true) => Ok(task_id.clone()),
        Ok(false) => Err(no_active_task_error()),
        Err(_) => Err(mcp_unavailable_error("task lookup failed")),
    }
}

struct CommitPipelineArgs<'a> {
    envelope: &'a ToolEnvelope,
    method_name: MethodName,
    request_hash: &'a RequestHash,
    event_id: String,
    result_fields: JsonObject,
    event_kind: String,
    event_payload: JsonObject,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    task_id: &'a TaskId,
    verified_surface: VerifiedSurfaceContext,
}

fn commit_mutation(
    store: &mut CoreProjectStore,
    args: CommitPipelineArgs<'_>,
) -> CoreResult<PipelineResponse> {
    let CommitPipelineArgs {
        envelope,
        method_name,
        request_hash,
        event_id,
        result_fields,
        event_kind,
        event_payload,
        change_unit_id,
        storage_mutations,
        task_id,
        verified_surface,
    } = args;

    let input = commit_input(
        &envelope.project_id,
        method_name,
        envelope.idempotency_key.as_ref(),
        request_hash,
        envelope
            .idempotency_key
            .as_ref()
            .map(|_| replay_context_from_verified_surface(&verified_surface)),
        envelope.expected_state_version,
        vec![PendingTaskEvent {
            event_id,
            task_id: task_id.as_str().to_owned(),
            change_unit_id: change_unit_id.map(|id| id.into_inner()),
            event_kind,
            event_payload_json: serde_json::to_string(&Value::Object(event_payload))?,
        }],
    );

    let outcome = store.commit_mutation(
        input,
        |mutation, facts| {
            for storage_mutation in &storage_mutations {
                storage_mutation.apply(mutation, facts.committed_state_version)?;
            }
            Ok(())
        },
        |facts| {
            committed_response_json(result_fields, facts.committed_state_version, facts.events)
                .map_err(store_invalid_input)
        },
    )?;

    match outcome {
        MutationCommitOutcome::Replayed { response_json, .. } => response_from_json_string(
            response_json,
            Some(verified_surface),
            Some(task_id.clone()),
            true,
        ),
        MutationCommitOutcome::ReplayContextMismatch {
            current_state_version,
            ..
        } => response_from_rejected(
            replay_context_mismatch_response(false, current_state_version),
            Some(verified_surface),
            Some(task_id.clone()),
        ),
        MutationCommitOutcome::IdempotencyConflict {
            current_state_version,
            idempotency_key,
            stored_request_hash,
            attempted_request_hash,
        } => response_from_rejected(
            rejected_response(
                false,
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
            Some(verified_surface),
            Some(task_id.clone()),
        ),
        MutationCommitOutcome::StaleExpectedState {
            current_state_version,
            expected_state_version,
        } => response_from_rejected(
            rejected_response(
                false,
                Some(current_state_version),
                vec![stale_expected_state_error(
                    current_state_version,
                    expected_state_version,
                    &envelope.project_id,
                    envelope.task_id.as_ref(),
                )],
            ),
            Some(verified_surface),
            Some(task_id.clone()),
        ),
        MutationCommitOutcome::Committed { response_json, .. } => response_from_json_string(
            response_json,
            Some(verified_surface),
            Some(task_id.clone()),
            false,
        ),
    }
}

fn committed_response_json(
    result_fields: JsonObject,
    committed_state_version: u64,
    events: Vec<CommittedEventRef>,
) -> CoreResult<String> {
    let event_refs = events
        .into_iter()
        .map(|event| EventRef {
            event_id: EventId::new(event.event_id),
            event_kind: event.event_kind,
        })
        .collect();
    let base = method_result_base(
        EffectKind::CoreCommitted,
        false,
        Some(committed_state_version),
        event_refs,
    );
    let response = method_result_value(base, result_fields)?;
    serde_json::to_string(&response).map_err(CorePipelineError::from)
}

fn response_from_rejected(
    response: ToolRejectedResponse,
    verified_surface: Option<VerifiedSurfaceContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelineResponse> {
    response_from_value(
        serde_json::to_value(response)?,
        verified_surface,
        resolved_task_id,
        false,
    )
}

fn response_outcome_from_rejected(
    response: ToolRejectedResponse,
    verified_surface: Option<VerifiedSurfaceContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelinePreflightOutcome> {
    response_from_rejected(response, verified_surface, resolved_task_id)
        .map(|response| PipelinePreflightOutcome::Response(Box::new(response)))
}

fn response_from_dry_run(
    response: ToolDryRunResponse,
    verified_surface: Option<VerifiedSurfaceContext>,
    resolved_task_id: Option<TaskId>,
) -> CoreResult<PipelineResponse> {
    response_from_value(
        serde_json::to_value(response)?,
        verified_surface,
        resolved_task_id,
        false,
    )
}

fn response_from_value(
    response_value: Value,
    verified_surface: Option<VerifiedSurfaceContext>,
    resolved_task_id: Option<TaskId>,
    replayed: bool,
) -> CoreResult<PipelineResponse> {
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        verified_surface,
        resolved_task_id,
        replayed,
    })
}

fn response_from_json_string(
    response_json: String,
    verified_surface: Option<VerifiedSurfaceContext>,
    resolved_task_id: Option<TaskId>,
    replayed: bool,
) -> CoreResult<PipelineResponse> {
    let response_value = serde_json::from_str(&response_json)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        verified_surface,
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

pub(crate) fn replay_context_mismatch_response(
    dry_run: bool,
    current_state_version: u64,
) -> ToolRejectedResponse {
    rejected_response(
        dry_run,
        Some(current_state_version),
        vec![local_access_mismatch_error("idempotency_replay_context")],
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

fn store_unavailable_error(error: StoreError) -> ToolError {
    tool_error(
        match error {
            StoreError::NotFound { .. } => ErrorCode::LocalAccessMismatch,
            StoreError::InvalidInput { .. }
            | StoreError::Io(_)
            | StoreError::Sqlite(_)
            | StoreError::MigrationConflict { .. }
            | StoreError::SchemaInvariant { .. } => ErrorCode::McpUnavailable,
        },
        "Core storage or project binding is unavailable",
        true,
        None,
    )
}

fn mcp_unavailable_error(message: &'static str) -> ToolError {
    tool_error(ErrorCode::McpUnavailable, message, true, None)
}

fn local_access_mismatch_error(field: &'static str) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(
        ErrorCode::LocalAccessMismatch,
        "local surface context does not match the registered surface",
        false,
        Some(details),
    )
}

fn no_active_task_error() -> ToolError {
    tool_error(
        ErrorCode::NoActiveTask,
        "a Task is required but no addressed or current Task is available",
        false,
        None,
    )
}

fn capability_insufficient_error(
    required_access_class: AccessClass,
    actual_access_class: AccessClass,
) -> ToolError {
    let mut details = Map::new();
    details.insert(
        "required_access_class".to_owned(),
        serde_json::to_value(required_access_class).unwrap_or(Value::Null),
    );
    details.insert(
        "actual_access_class".to_owned(),
        serde_json::to_value(actual_access_class).unwrap_or(Value::Null),
    );
    tool_error(
        ErrorCode::CapabilityInsufficient,
        "surface access class is insufficient for this request",
        false,
        Some(details),
    )
}

fn error_precedence(code: ErrorCode) -> u8 {
    match code {
        ErrorCode::ValidationFailed => 1,
        ErrorCode::StateVersionConflict => 2,
        ErrorCode::McpUnavailable => 3,
        ErrorCode::LocalAccessMismatch => 4,
        ErrorCode::NoActiveTask => 5,
        ErrorCode::NoActiveChangeUnit => 6,
        ErrorCode::BaselineStale => 7,
        ErrorCode::ScopeRequired => 8,
        ErrorCode::ScopeViolation => 9,
        ErrorCode::WriteAuthorizationRequired => 10,
        ErrorCode::WriteAuthorizationInvalid => 11,
        ErrorCode::ApprovalDenied => 12,
        ErrorCode::ApprovalExpired => 13,
        ErrorCode::ApprovalRequired => 14,
        ErrorCode::DecisionUnresolved => 15,
        ErrorCode::AutonomyBoundaryExceeded => 16,
        ErrorCode::DecisionRequired => 17,
        ErrorCode::CapabilityInsufficient => 18,
        ErrorCode::EvidenceInsufficient => 19,
        ErrorCode::ResidualRiskNotVisible => 20,
        ErrorCode::AcceptanceRequired => 21,
        ErrorCode::ProjectionStale => 22,
        ErrorCode::ArtifactMissing => 23,
        ErrorCode::ValidatorFailed => 24,
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
    use std::{error::Error, path::PathBuf};

    use harness_store::{
        bootstrap::{
            initialize_runtime_home, register_project, register_surface, ProjectRegistration,
            SurfaceRegistration, ACTIVE_PROJECT_STATUS,
        },
        core_pipeline::{CoreProjectStore, StorageEffectCounts},
        sqlite::open_project_state_database,
    };
    use harness_test_support::TempRuntimeHome;
    use harness_types::{
        ActorKind, IdempotencyKey, PlannedEffect, ProjectId, RequestId, SurfaceId,
        SurfaceInstanceId,
    };
    use serde_json::{json, Map, Value};

    use super::*;

    const PROJECT_ID: &str = "project_a";
    const TASK_ID: &str = "task_a";
    const SURFACE_ID: &str = "surface_main";
    const SURFACE_INSTANCE_ID: &str = "surface_instance_1";

    struct PipelineHarness {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
        service: CoreService,
    }

    impl PipelineHarness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("core-pipeline")?;
            initialize_runtime_home(runtime_home.path(), "runtime_home_a", "{}")?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root: runtime_home.path().join("repo"),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_surface(
                runtime_home.path(),
                SurfaceRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    surface_id: SURFACE_ID.to_owned(),
                    surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                    surface_kind: "local_test".to_owned(),
                    display_name: Some("Pipeline Test Surface".to_owned()),
                    capability_profile_json: "{}".to_owned(),
                    local_access_json: json!({
                        "access_class": "core_mutation",
                        "authorized_access_classes": ["read_status", "core_mutation"],
                        "verification_basis": "pipeline_test_registration"
                    })
                    .to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;

            let conn = open_project_state_database(runtime_home.project_state_db_path(PROJECT_ID))?;
            conn.execute(
                "INSERT INTO tasks (
                    project_id,
                    task_id,
                    created_by_surface_id,
                    created_by_surface_instance_id,
                    mode,
                    lifecycle_phase,
                    created_at,
                    updated_at
                )
                VALUES (
                    'project_a',
                    'task_a',
                    'surface_main',
                    'surface_instance_1',
                    'work',
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

            let runtime_home_path = runtime_home.path().to_path_buf();
            let service = CoreService::new(&runtime_home_path);
            Ok(Self {
                _runtime_home: runtime_home,
                runtime_home_path,
                service,
            })
        }

        fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
            let store =
                CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
            Ok(store.effect_counts()?)
        }

        fn register_surface_instance(
            &self,
            surface_instance_id: &str,
            authorized_access_classes: Vec<&str>,
        ) -> Result<(), Box<dyn Error>> {
            register_surface(
                &self.runtime_home_path,
                SurfaceRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    surface_id: SURFACE_ID.to_owned(),
                    surface_instance_id: surface_instance_id.to_owned(),
                    surface_kind: "local_test".to_owned(),
                    display_name: Some(format!("Pipeline Test Surface {surface_instance_id}")),
                    capability_profile_json: "{}".to_owned(),
                    local_access_json: json!({
                        "authorized_access_classes": authorized_access_classes,
                        "verification_basis": "pipeline_test_registration"
                    })
                    .to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(())
        }

        fn conn(&self) -> Result<rusqlite::Connection, StoreError> {
            open_project_state_database(
                self.runtime_home_path
                    .join("projects")
                    .join(PROJECT_ID)
                    .join("state.sqlite"),
            )
        }

        fn execute(&self, request: PipelineRequest) -> CoreResult<PipelineResponse> {
            self.service.execute_pipeline(request)
        }
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: OwnerPipelineBranch::DryRunPreview {
                dry_run_summary: dry_run_summary(),
            },
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
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
            invocation: invocation(AccessClass::ReadStatus, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::ReadStatus,
            task_requirement: TaskRequirement::Optional,
            branch: OwnerPipelineBranch::ReadOnly {
                result_fields: result_fields("read_only"),
            },
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        assert_eq!(harness.counts()?, before);
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
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
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        assert_eq!(after.tasks, before.tasks);
        Ok(())
    }

    #[test]
    fn idempotency_replay_returns_stored_response() -> Result<(), Box<dyn Error>> {
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("replay"),
        };

        let first = harness.execute(request.clone())?;
        let after_first = harness.counts()?;
        let second = harness.execute(request)?;
        let after_second = harness.counts()?;

        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(after_second, after_first);
        Ok(())
    }

    #[test]
    fn idempotency_replay_rejects_other_surface_instance_without_stored_response(
    ) -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        harness.register_surface_instance(
            "surface_instance_other",
            vec!["read_status", "core_mutation"],
        )?;
        let envelope = envelope(
            "req_replay_surface",
            Some("idem_replay_surface"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(MethodName::UpdateScope, &envelope, "surface-secret");
        let first_request = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json.clone(),
            envelope: envelope.clone(),
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("surface-secret"),
        };
        let first = harness.execute(first_request)?;
        let after_first = harness.counts()?;

        let mismatch = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json,
            envelope,
            invocation: invocation(AccessClass::CoreMutation, Some("surface_instance_other")),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("surface-secret"),
        })?;

        assert!(!mismatch.replayed);
        assert_eq!(mismatch.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            mismatch.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert!(!mismatch.response_json.contains("surface-secret"));
        assert_ne!(mismatch.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn idempotency_replay_rejects_other_access_class() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_replay_access",
            Some("idem_replay_access"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(MethodName::UpdateScope, &envelope, "access-secret");
        let first_request = PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json: request_json.clone(),
            envelope: envelope.clone(),
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("access-secret"),
        };
        harness.execute(first_request)?;
        let after_first = harness.counts()?;

        let mismatch = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json,
            envelope,
            invocation: invocation(AccessClass::ReadStatus, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("access-secret"),
        })?;

        assert_eq!(
            mismatch.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert!(!mismatch.response_json.contains("access-secret"));
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn replay_context_mismatch_precedes_request_hash_conflict() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        harness.register_surface_instance(
            "surface_instance_hash_mismatch",
            vec!["read_status", "core_mutation"],
        )?;
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
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
                AccessClass::CoreMutation,
                Some("surface_instance_hash_mismatch"),
            ),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("different"),
        })?;

        assert_eq!(
            mismatch.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert!(!mismatch.response_json.contains("stored-secret"));
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn legacy_replay_row_is_preserved_but_not_replayed() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let envelope = envelope(
            "req_legacy_replay",
            Some("idem_legacy_replay"),
            false,
            Some(0),
            Some(TASK_ID),
        );
        let request_json = request_json(MethodName::UpdateScope, &envelope, "legacy-attempt");
        let request_hash = canonical_request_hash(&request_json)?;
        harness.conn()?.execute(
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
                'harness.update_scope',
                'idem_legacy_replay',
                ?2,
                0,
                1,
                '{\"stored\":\"legacy-secret\"}',
                't0'
            )",
            rusqlite::params![PROJECT_ID, request_hash.as_str()],
        )?;
        let before = harness.counts()?;

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::UpdateScope,
            request_json,
            envelope,
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("legacy-attempt"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert!(!response.response_json.contains("legacy-secret"));
        assert_eq!(harness.counts()?, before);
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
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
            invocation: invocation(AccessClass::CoreMutation, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
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
    fn surface_instance_mismatch_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = PipelineHarness::new()?;
        let before = harness.counts()?;
        let envelope = envelope("req_surface", None, false, None, Some(TASK_ID));

        let response = harness.execute(PipelineRequest {
            method_name: MethodName::Status,
            request_json: request_json(MethodName::Status, &envelope, "surface-mismatch"),
            envelope,
            invocation: invocation(AccessClass::ReadStatus, Some("unknown_surface_instance")),
            required_access_class: AccessClass::ReadStatus,
            task_requirement: TaskRequirement::Optional,
            branch: OwnerPipelineBranch::ReadOnly {
                result_fields: result_fields("surface_mismatch"),
            },
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn access_class_mismatch_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
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
            invocation: invocation(AccessClass::ReadStatus, Some(SURFACE_INSTANCE_ID)),
            required_access_class: AccessClass::CoreMutation,
            task_requirement: TaskRequirement::Required,
            branch: commit_branch("access_mismatch"),
        })?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "CAPABILITY_INSUFFICIENT"
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
            task_id: task_id.map(TaskId::new),
            actor_kind: ActorKind::Agent,
            surface_id: SurfaceId::new(SURFACE_ID),
            request_id: RequestId::new(request_id),
            idempotency_key: idempotency_key.map(IdempotencyKey::new),
            expected_state_version,
            dry_run,
            locale: None,
        }
    }

    fn invocation(
        access_class: AccessClass,
        surface_instance_id: Option<&str>,
    ) -> InvocationContext {
        InvocationContext {
            surface_instance_id: surface_instance_id.map(SurfaceInstanceId::new),
            requested_access_class: access_class,
            invocation_binding_basis: "pipeline_test_invocation".to_owned(),
        }
    }

    fn request_json(method_name: MethodName, envelope: &ToolEnvelope, marker: &str) -> Value {
        json!({
            "method": method_name.as_str(),
            "envelope": envelope,
            "pipeline_placeholder": marker
        })
    }

    fn result_fields(marker: &str) -> JsonObject {
        let mut fields = Map::new();
        fields.insert(
            "pipeline_placeholder".to_owned(),
            Value::String(marker.to_owned()),
        );
        fields
    }

    fn commit_branch(marker: &str) -> OwnerPipelineBranch {
        OwnerPipelineBranch::CommitMutation {
            result_fields: result_fields(marker),
            event_kind: "core.pipeline_placeholder_commit".to_owned(),
            event_payload: result_fields(marker),
            task_id: None,
            change_unit_id: None,
            storage_mutations: Vec::new(),
        }
    }

    fn dry_run_summary() -> DryRunSummary {
        DryRunSummary {
            planned_effects: Vec::<PlannedEffect>::new(),
            would_blockers: Vec::new(),
            would_errors: Vec::new(),
            next_actions: Vec::new(),
            diagnostics: vec!["pipeline placeholder dry-run".to_owned()],
        }
    }
}
