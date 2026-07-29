use chrono::{DateTime, Utc};
use serde_json::Value;
use volicord_store::RuntimeHomeMutationContext;
use volicord_types::{
    methods::{public_method_contract, DryRunRequestRoute},
    schema::ToolEnvelope,
    values::{MethodName, UtcTimestamp},
};

use crate::pipeline::{
    CorePipelineError, CoreResult, CoreService, FreshnessPolicy, InvocationContext,
    MethodEffectPolicy, MethodPolicy, PipelinePreflightOutcome, PipelinePreflightRequest,
    PipelineResponse, PreparedRequest, ReplayPolicy, TaskRequirement,
};

pub(crate) enum PlanError {
    Core(CorePipelineError),
    Response(Box<PipelineResponse>),
}

impl From<CorePipelineError> for PlanError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<serde_json::Error> for PlanError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

pub(crate) fn prepare_or_response<'mutation>(
    service: &CoreService,
    context: Option<&'mutation RuntimeHomeMutationContext<'mutation>>,
    method_name: MethodName,
    envelope: ToolEnvelope,
    request_json: Value,
    invocation: InvocationContext,
    policy: MethodPolicy,
) -> CoreResult<Result<PreparedRequest<'mutation>, PipelineResponse>> {
    match service.prepare_request(
        context,
        PipelinePreflightRequest {
            method_name,
            envelope,
            request_json,
            invocation,
            policy,
        },
    )? {
        PipelinePreflightOutcome::Prepared(prepared) => Ok(Ok(*prepared)),
        PipelinePreflightOutcome::Response(response) => Ok(Err(*response)),
    }
}

pub(crate) fn utc_timestamp(timestamp: DateTime<Utc>) -> UtcTimestamp {
    UtcTimestamp::from_datetime(timestamp)
}

pub(crate) fn decode_semantic_replay_result<T>(replay_identity: &str, raw: &str) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw).map_err(|_| CorePipelineError::Invariant {
        detail: format!(
            "stored semantic replay result `{replay_identity}` does not match its method result contract"
        ),
    })
}

pub(crate) fn storage_value<T>(value: T) -> CoreResult<String>
where
    T: serde::Serialize,
{
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "storage value must serialize to a string".to_owned(),
        }),
    }
}

pub(crate) fn mutation_method_policy(
    method_name: MethodName,
    operation_category: volicord_types::values::OperationCategory,
    task: TaskRequirement,
    dry_run: volicord_types::schema::DryRunIntent,
) -> MethodPolicy {
    if public_method_contract(method_name)
        .dry_run_policy()
        .route(dry_run)
        == DryRunRequestRoute::Preview
    {
        MethodPolicy::exact(
            operation_category,
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            operation_category,
            task,
            ReplayPolicy::Committed,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::CoreMutation,
        )
    }
}
