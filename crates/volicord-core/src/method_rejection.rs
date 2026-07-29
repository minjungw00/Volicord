use chrono::Duration;
use serde_json::{Map, Value};
use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::{
    ids::BaselineRef,
    schema::{DryRunSummary, NextActionSummary, PlannedEffect, ToolEnvelope},
    values::{ErrorCode, UtcTimestamp},
};

use crate::{
    method_execution::PlanError,
    pipeline::{rejected_response, tool_error, CoreResult, PipelineResponse},
};

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
