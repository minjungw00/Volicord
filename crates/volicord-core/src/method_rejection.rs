use chrono::Duration;
use serde_json::{Map, Value};
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_types::{
    ids::{BaselineRef, TaskId},
    schema::{
        AuthorityBasisMismatch, AuthorityBasisValue, DryRunSummary, FalseValue, NextActionSummary,
        PlannedEffect, ToolEnvelope, TransitionRejection, WorkflowActionKey, WorkflowProjection,
    },
    values::{
        ErrorCode, MethodName, RunKind, TransitionRejectionReason, UtcTimestamp,
        WorkflowActionSemanticVariant,
    },
};

use crate::{
    json_object::object_from_value,
    method_execution::PlanError,
    pipeline::{rejected_response, tool_error, CoreResult, PipelineResponse},
};

/// Exact or statically single-variant action identity retained by a method planner.
pub(crate) enum AttemptedWorkflowAction {
    SingleVariantMethod(MethodName),
    Exact(WorkflowActionKey),
}

impl From<MethodName> for AttemptedWorkflowAction {
    fn from(method: MethodName) -> Self {
        Self::SingleVariantMethod(method)
    }
}

impl From<WorkflowActionKey> for AttemptedWorkflowAction {
    fn from(action_key: WorkflowActionKey) -> Self {
        Self::Exact(action_key)
    }
}

/// Builds one closed workflow rejection from current Store authority.
#[allow(clippy::too_many_arguments)]
pub(crate) fn workflow_rejected_response<A: Into<AttemptedWorkflowAction>>(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    code: ErrorCode,
    message: &'static str,
    received_action: A,
    received_run_kind: Option<RunKind>,
    allowed_run_kinds: Vec<RunKind>,
    corrected_retry_allowed: bool,
) -> CoreResult<PipelineResponse> {
    let received_action = match received_action.into() {
        AttemptedWorkflowAction::Exact(action_key) => action_key,
        AttemptedWorkflowAction::SingleVariantMethod(method) => {
            WorkflowActionSemanticVariant::for_single_variant_method(method)
                .and_then(|variant| WorkflowActionKey::new(method, variant).ok())
                .ok_or_else(|| crate::pipeline::CorePipelineError::Invariant {
                    detail:
                        "a multi-variant method rejection must retain its exact workflow action key"
                            .to_owned(),
                })?
        }
    };
    workflow_rejected_response_from_current_catalog(
        store,
        project_state,
        envelope,
        task_id,
        code,
        message,
        received_action,
        received_run_kind,
        allowed_run_kinds,
        corrected_retry_allowed,
    )
}

/// Builds a rejection from an exact attempted action and the same current catalog
/// that is used to validate any recovery action.
#[allow(clippy::too_many_arguments)]
pub(crate) fn transition_rejected_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    workflow: &WorkflowProjection,
    code: ErrorCode,
    message: &'static str,
    attempted_action_key: WorkflowActionKey,
    reason: TransitionRejectionReason,
    retryable: bool,
    recovery_action_key: Option<WorkflowActionKey>,
) -> CoreResult<PipelineResponse> {
    let details = TransitionRejection::new(
        attempted_action_key,
        reason,
        retryable,
        recovery_action_key,
        workflow.required_refs().to_vec(),
        workflow.kind(),
        workflow.transition_catalog(),
    )
    .map_err(|detail| crate::pipeline::CorePipelineError::Invariant {
        detail: detail.to_owned(),
    })?;
    let details = object_from_value(serde_json::to_value(details)?)?;
    rejected_pipeline_response(
        envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(code, message, retryable, Some(details))],
    )
}

#[allow(clippy::too_many_arguments)]
fn workflow_rejected_response_from_current_catalog(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    code: ErrorCode,
    message: &'static str,
    attempted_action_key: WorkflowActionKey,
    received_run_kind: Option<RunKind>,
    allowed_run_kinds: Vec<RunKind>,
    corrected_retry_allowed: bool,
) -> CoreResult<PipelineResponse> {
    if !TransitionRejection::is_required_for(code) {
        return Err(crate::pipeline::CorePipelineError::InvalidDispatch {
            detail: format!("{} is not a workflow-rejection error code", code.as_str()),
        });
    }
    let workflow =
        crate::workflow_projection::current_workflow_projection(store, project_state, task_id)?;
    let catalog = workflow.transition_catalog();
    let recovery_action_key = catalog
        .required_transition()
        .map(|transition| transition.action_key);
    let reason = transition_reason_for_code(code);
    let _ = (received_run_kind, allowed_run_kinds);
    transition_rejected_response(
        envelope,
        project_state,
        &workflow,
        code,
        message,
        attempted_action_key,
        reason,
        corrected_retry_allowed,
        recovery_action_key,
    )
}

fn transition_reason_for_code(code: ErrorCode) -> TransitionRejectionReason {
    match code {
        ErrorCode::ShapingCheckpointStale => TransitionRejectionReason::CheckpointStale,
        ErrorCode::ChangeUnitStale => TransitionRejectionReason::ChangeUnitStale,
        ErrorCode::WorkspaceBasisStale => TransitionRejectionReason::WorkspaceBasisStale,
        ErrorCode::UserDecisionUnresolved => TransitionRejectionReason::UserAuthorityMissing,
        ErrorCode::TaskPhaseTransitionRequired => {
            TransitionRejectionReason::ImplementationAuthorityWouldBeInvalidated
        }
        ErrorCode::WorkflowActionNotAllowed => TransitionRejectionReason::ActionNotCurrent,
        ErrorCode::RunKindIncompatible
        | ErrorCode::ShapingCheckpointRequired
        | ErrorCode::ChangeUnitRequired => TransitionRejectionReason::AuthorityBasisMismatch,
        _ => TransitionRejectionReason::AuthorityBasisMismatch,
    }
}

/// Returns a typed workflow rejection from a method planning branch.
#[allow(clippy::too_many_arguments)]
pub(crate) fn workflow_rejection_plan_error<T, A: Into<AttemptedWorkflowAction>>(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    code: ErrorCode,
    message: &'static str,
    received_action: A,
    received_run_kind: Option<RunKind>,
    allowed_run_kinds: Vec<RunKind>,
    corrected_retry_allowed: bool,
) -> Result<T, PlanError> {
    let response = workflow_rejected_response(
        store,
        project_state,
        envelope,
        task_id,
        code,
        message,
        received_action,
        received_run_kind,
        allowed_run_kinds,
        corrected_retry_allowed,
    )
    .map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

pub(crate) fn validation_plan_error<T>(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    let response =
        validation_rejected(dry_run, state_version, field, message).map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

pub(crate) fn authority_basis_mismatch_plan_error<T>(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    expected: AuthorityBasisValue,
    received: AuthorityBasisValue,
    message: impl Into<String>,
) -> Result<T, PlanError> {
    let details = object_from_value(serde_json::to_value(AuthorityBasisMismatch {
        field: field.to_owned(),
        expected,
        received,
        state_change_applied: FalseValue,
    })?)?;
    let response = rejected_pipeline_response(
        dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::ValidationFailed,
            message,
            true,
            Some(details),
        )],
    )
    .map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

pub(crate) fn checked_derived_expiration(
    created_at: &UtcTimestamp,
    duration: Duration,
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
) -> Result<UtcTimestamp, PlanError> {
    match created_at.checked_add(duration) {
        Ok(expires_at) => Ok(expires_at),
        Err(_) => validation_plan_error(
            dry_run,
            state_version,
            field,
            "derived expiration exceeds the supported canonical RFC 3339 range",
        ),
    }
}

pub(crate) fn baseline_stale_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    baseline_ref: &BaselineRef,
) -> PipelineResponse {
    let mut details = Map::new();
    details.insert(
        "baseline_ref".to_owned(),
        Value::String(baseline_ref.as_str().to_owned()),
    );
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::BaselineStale,
            "baseline_ref does not match the current Change Unit basis",
            true,
            Some(details),
        )],
    )
}

pub(crate) fn workspace_stale_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
) -> PipelineResponse {
    let mut details = Map::new();
    details.insert(
        "workspace_reason".to_owned(),
        Value::String("workspace_context_mismatch".to_owned()),
    );
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::BaselineStale,
            "current Git workspace context does not match the current Change Unit basis",
            true,
            Some(details),
        )],
    )
}

pub(crate) fn no_active_change_unit_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::NoActiveChangeUnit,
            message,
            false,
            None,
        )],
    )
}

pub(crate) fn decision_rejected_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::DecisionUnresolved,
            message,
            false,
            None,
        )],
    )
}

pub(crate) fn validation_rejected(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> CoreResult<PipelineResponse> {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    rejected_pipeline_response(
        dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::ValidationFailed,
            message,
            false,
            Some(details),
        )],
    )
}

pub(crate) fn rejected_pipeline_response(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    errors: Vec<volicord_types::schema::ToolError>,
) -> CoreResult<PipelineResponse> {
    let response = rejected_response(dry_run, state_version, errors);
    let response_value = serde_json::to_value(response)?;
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        operation_result_ref: None,
        verified_invocation: None,
        resolved_task_id: None,
        replayed: false,
    })
}

pub(crate) fn infallible_rejected_pipeline_response(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    errors: Vec<volicord_types::schema::ToolError>,
) -> PipelineResponse {
    rejected_pipeline_response(dry_run, state_version, errors)
        .expect("rejected response serialization should succeed")
}

pub(crate) fn no_active_task_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(
            ErrorCode::NoActiveTask,
            "a Task is required but no addressed or current Task is available",
            false,
            None,
        )],
    )
}

pub(crate) fn dry_run_summary(
    target_kind: &str,
    action: &str,
    description: &str,
    next_actions: Vec<NextActionSummary>,
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: vec![PlannedEffect {
            target_kind: target_kind.to_owned(),
            action: action.to_owned(),
            description: description.to_owned(),
        }],
        would_blockers: Vec::new(),
        would_errors: Vec::new(),
        next_actions,
        diagnostics: Vec::new(),
    }
}
