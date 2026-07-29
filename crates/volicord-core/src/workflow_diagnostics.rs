use volicord_types::ids::TaskId;
use volicord_types::values::UtcTimestamp;

use serde_json::Value;
use volicord_store::{
    core_pipeline::CoreProjectStore,
    diagnostics::{record_workflow_metric_event, WorkflowMetricEvent, WorkflowMetricKind},
    RuntimeHomeMutationContext,
};

use crate::pipeline::PipelineResponse;

pub(crate) fn elapsed_micros(start: &UtcTimestamp, end: &UtcTimestamp) -> Option<u64> {
    end.as_datetime()
        .signed_duration_since(start.as_datetime())
        .num_microseconds()
        .and_then(|value| u64::try_from(value).ok())
}

pub(crate) fn first_product_write_duration_micros(
    store: &CoreProjectStore,
    task_id: &TaskId,
    observed_no_later_than: &UtcTimestamp,
) -> Option<u64> {
    let task_created_at = store.task_created_at(task_id).ok()??;
    store
        .product_write_observation_candidates_for_task(task_id)
        .ok()?
        .into_iter()
        .filter_map(|candidate| {
            if candidate.observed_paths.is_empty() {
                return None;
            }
            let observed_at = candidate.observed_at;
            if observed_at.as_datetime() < task_created_at.as_datetime()
                || observed_at.as_datetime() > observed_no_later_than.as_datetime()
            {
                return None;
            }
            elapsed_micros(&task_created_at, &observed_at)
        })
        .min()
}

pub(crate) fn record_core_workflow_metric_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    session_id: Option<&str>,
    metric_kind: WorkflowMetricKind,
    value: u64,
) {
    let Some(session_id) = session_id else {
        return;
    };
    let _ = record_workflow_metric_event(
        context,
        &WorkflowMetricEvent {
            session_id: session_id.to_owned(),
            metric_kind,
            value,
            method_name: None,
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: None,
        },
    );
}

pub(crate) fn response_committed_fresh_effect(response: &PipelineResponse) -> bool {
    !response.replayed
        && response
            .response_value
            .pointer("/base/effect_kind")
            .and_then(Value::as_str)
            == Some("core_committed")
}
