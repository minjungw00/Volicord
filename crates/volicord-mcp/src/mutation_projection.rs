//! Normal mutation result projection after authoritative refresh.

use crate::adapter::McpAdapter;
use crate::authority_refresh::{
    refresh_authority_status, validated_authority_refresh, MutationRefreshContext,
};
use crate::committed_result_recovery::{
    authoritative_refresh_failure_output, bounded_mutation_compatibility_text,
    mutation_post_effect_failure_output, mutation_response_budget_exceeded_output,
    CanonicalMcpMutationOutcome,
};
use crate::errors::McpAdapterError;
use crate::lifecycle::SessionRuntime;
use crate::tool_dispatch::{tool_call_result_from_output_for_capabilities, ToolCallOutput};
use serde_json::Value;
use volicord_core::pipeline::PipelineResponse;
use volicord_mcp_protocol::McpProtocolCapabilities;
#[cfg(test)]
use volicord_mcp_protocol::ProtocolRegistry;
use volicord_mcp_wire::{
    McpMutationEffectSummary, McpMutationFullResponse, McpMutationSummaryResponse,
    McpMutationWorkflowResponse, McpPostEffectFailureCode, McpPrepareEvidenceCaptureCompactResult,
    McpPrepareWriteCompactResult, McpReconcileChangesCompactResult, McpRecordRunCloseBasisAnchor,
    McpRecordRunCompactResult, McpRequestUserActionCompactResult, McpRequestUserActionResponse,
    McpStageArtifactCompactResult,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::RecordId;
use volicord_types::methods::{
    PrepareEvidenceCaptureResult, PrepareWriteResult, ReconcileChangesResult, RecordRunResult,
    StageArtifactResult,
};
use volicord_types::schema::{AuthorityReceipt, StateRecordRef, ToolResultBase};
use volicord_types::tool_names::{AgentToolCategory, AgentToolId, AgentToolOwner};
use volicord_types::values::{MutationDetailLevel, StateRecordKind};

pub(crate) const MAX_MCP_COMPACT_MUTATION_RESULT_BYTES: usize = 65_536;
pub(crate) const MAX_MCP_FULL_MUTATION_RESULT_BYTES: usize = 256 * 1024;

pub(crate) fn mutation_detail_for_tool(
    tool: AgentToolId,
    arguments: &Value,
) -> Option<MutationDetailLevel> {
    (matches!(tool.owner(), AgentToolOwner::CoreMethod(_))
        && matches!(
            tool.category(),
            AgentToolCategory::NonDestructiveMutation | AgentToolCategory::DestructiveMutation
        ))
    .then(|| {
        arguments
            .get("detail")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    })
}

pub(crate) fn mutation_effect_anchor(response: &PipelineResponse) -> Option<String> {
    if let Some(event_id) = response
        .response_value
        .pointer("/base/events/0/event_id")
        .and_then(Value::as_str)
    {
        return Some(format!("authority_event:{event_id}"));
    }
    if let Some(handle_id) = response
        .response_value
        .pointer("/staged_artifact_handle/handle_id")
        .and_then(Value::as_str)
    {
        return Some(format!("staged_artifact:{handle_id}"));
    }
    let effect_kind = response
        .response_value
        .pointer("/base/effect_kind")
        .and_then(Value::as_str)?;
    if !matches!(effect_kind, "core_committed" | "staging_created") {
        return None;
    }
    let project_id = response.verified_invocation.as_ref()?.project_id.as_str();
    let state_version = response
        .response_value
        .pointer("/base/state_version")
        .and_then(Value::as_u64)?;
    Some(format!("state_effect:{project_id}:{state_version}"))
}

pub(crate) fn finalize_mutation_output(
    mutation_context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &SessionRuntime,
    capabilities: McpProtocolCapabilities,
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    output: ToolCallOutput,
) -> Result<ToolCallOutput, McpAdapterError> {
    finalize_mutation_output_with_refresh_for_capabilities(
        tool_name,
        capabilities,
        detail,
        output,
        |context| refresh_authority_status(mutation_context, adapter, state, context),
    )
}

#[cfg(test)]
pub(crate) fn finalize_mutation_output_with_refresh<F>(
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    output: ToolCallOutput,
    refresh: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: FnOnce(&MutationRefreshContext) -> Result<PipelineResponse, McpAdapterError>,
{
    finalize_mutation_output_with_refresh_for_capabilities(
        tool_name,
        ProtocolRegistry::production()
            .preferred_server_profile()
            .capabilities(),
        detail,
        output,
        refresh,
    )
}

fn finalize_mutation_output_with_refresh_for_capabilities<F>(
    tool_name: &str,
    capabilities: McpProtocolCapabilities,
    detail: Option<MutationDetailLevel>,
    mut output: ToolCallOutput,
    refresh: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: FnOnce(&MutationRefreshContext) -> Result<PipelineResponse, McpAdapterError>,
{
    let Some(detail) = detail else {
        return Ok(output);
    };
    if output.is_error {
        return Ok(output);
    }
    if response_kind_from_structured_content(&output.structured_content) != Some("result") {
        output.primary_text = bounded_mutation_compatibility_text(format!(
            "Volicord {tool_name} returned response_kind={}; inspect the authoritative result carrier.",
            response_kind_from_structured_content(&output.structured_content)
                .unwrap_or("unknown")
        ));
        return Ok(output);
    }

    let original_method_result = std::mem::take(&mut output.structured_content);
    let operation_result_ref = output.operation_result_ref.clone();
    let mut outcome = CanonicalMcpMutationOutcome::new(
        tool_name,
        capabilities,
        detail,
        output.diagnostic_facts.clone(),
        Some(original_method_result),
        operation_result_ref,
    );
    let Some(context) = output.mutation_refresh_context.clone() else {
        return authoritative_refresh_failure_output(&outcome);
    };
    let (receipt, next_actions) = match refresh(&context) {
        Ok(response) => match validated_authority_refresh(&context, &response) {
            Ok(refreshed) => refreshed,
            Err(()) => return authoritative_refresh_failure_output(&outcome),
        },
        Err(_) => return authoritative_refresh_failure_output(&outcome),
    };
    outcome.set_authority_refresh(receipt, next_actions);
    let authority_receipt = outcome
        .authority_receipt
        .as_ref()
        .expect("validated canonical mutation outcome requires an authority receipt");

    if let Some(code) = output.post_effect_failure {
        return mutation_post_effect_failure_output(&outcome, code);
    }
    output.primary_text = match authority_receipt_compatibility_text(tool_name, authority_receipt) {
        Ok(text) => text,
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };
    output.mutation_refresh_context = None;
    let Some(compact_method_result) = outcome.compact_method_result.clone() else {
        return mutation_post_effect_failure_output(
            &outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        );
    };
    let method_result = match detail {
        MutationDetailLevel::Full => outcome
            .exact_method_result
            .clone()
            .expect("canonical mutation outcome requires an exact result"),
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => compact_method_result,
    };
    let projected = match detail {
        MutationDetailLevel::Summary => serde_json::to_value(McpMutationSummaryResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
        }),
        MutationDetailLevel::Workflow => serde_json::to_value(McpMutationWorkflowResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
            next_actions: outcome.next_actions.clone(),
        }),
        MutationDetailLevel::Full => serde_json::to_value(McpMutationFullResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
        }),
    };
    output.structured_content = match projected {
        Ok(projected) => projected,
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };

    let result =
        tool_call_result_from_output_for_capabilities(tool_name, output.clone(), capabilities)?;
    let response_budget = match detail {
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => {
            MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        }
        MutationDetailLevel::Full => MAX_MCP_FULL_MUTATION_RESULT_BYTES,
    };
    let rendered_size = match serde_json::to_vec(&result) {
        Ok(rendered) => rendered.len(),
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };
    if rendered_size > response_budget {
        return mutation_response_budget_exceeded_output(&outcome);
    }
    Ok(output)
}

pub(crate) fn response_kind_from_structured_content(value: &Value) -> Option<&str> {
    value
        .pointer("/agent_workflow_result/base/response_kind")
        .or_else(|| value.pointer("/base/response_kind"))
        .and_then(Value::as_str)
}

pub(crate) fn compact_mutation_method_result(
    tool_name: &str,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let effect = compact_mutation_effect(method_result)?;
    let tool = AgentToolId::from_wire_name(tool_name)
        .map_err(|_| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    match tool {
        AgentToolId::PREPARE_EVIDENCE_CAPTURE => {
            let result: PrepareEvidenceCaptureResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpPrepareEvidenceCaptureCompactResult {
                effect,
                capture_intent_ref: result.capture_intent_ref,
                capture_intent: result.capture_intent,
                expires_at: result.expires_at,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::PREPARE_WRITE => {
            let result: PrepareWriteResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpPrepareWriteCompactResult {
                effect,
                decision: result.decision,
                write_ticket_id: result.write_ticket_id,
                write_ticket_ref: result.write_ticket_ref,
                write_ticket: result.write_ticket,
                write_ticket_effect: result.write_ticket_effect,
                allowed_path_patterns: result.allowed_path_patterns,
                denied_path_patterns: result.denied_path_patterns,
                write_decision_reasons: result.write_decision_reasons,
                user_action_draft: result.user_action_draft,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::STAGE_ARTIFACT => {
            let result: StageArtifactResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpStageArtifactCompactResult {
                effect,
                evidence_state: result.evidence_state,
                staged_artifact_handle: result.staged_artifact_handle,
                expires_at: result.expires_at,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::RECORD_RUN => {
            let result: RecordRunResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let evidence_observation_refs = result
                .evidence_observations
                .iter()
                .map(|observation| StateRecordRef {
                    record_kind: StateRecordKind::EvidenceObservation,
                    record_id: RecordId::new(observation.observation_id.as_str()),
                    project_id: observation.project_id.clone(),
                    task_id: Some(observation.task_id.clone()).into(),
                    produced_at_state_version: effect.state_version.into(),
                })
                .collect();
            let evidence_producer_refs = result
                .evidence_producers
                .iter()
                .map(|producer| StateRecordRef {
                    record_kind: StateRecordKind::EvidenceProducer,
                    record_id: RecordId::new(producer.evidence_producer_id.as_str()),
                    project_id: producer.project_id.clone(),
                    task_id: Some(producer.task_id.clone()).into(),
                    produced_at_state_version: effect.state_version.into(),
                })
                .collect();
            let close_basis_anchor =
                result
                    .current_close_basis
                    .map(|basis| McpRecordRunCloseBasisAnchor {
                        close_basis_revision: basis.close_basis_revision,
                        scope_revision: basis.scope_revision,
                        source_run_ref: basis.source_run_ref,
                        evidence_summary_ref: basis.evidence_summary_ref,
                    });
            serde_json::to_value(McpRecordRunCompactResult {
                effect,
                run_ref: result.run_summary.run_ref,
                registered_artifact_refs: result.registered_artifacts,
                evidence_observation_refs,
                evidence_producer_refs,
                close_basis_anchor: close_basis_anchor.into(),
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::REQUEST_USER_ACTION => {
            compact_request_user_action_result(effect, method_result)
        }
        AgentToolId::RECONCILE_CHANGES => {
            let result: ReconcileChangesResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpReconcileChangesCompactResult {
                effect,
                unresolved_changes: result.unresolved_changes,
                resolved_changes: result.resolved_changes,
                pending_user_action_summaries: result.pending_user_action_summaries,
                rejected_resolution_requests: result.rejected_resolution_requests,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::INTAKE | AgentToolId::UPDATE_SCOPE | AgentToolId::CLOSE_TASK => {
            serde_json::to_value(effect).map_err(McpAdapterError::Json)
        }
        _ => Err(McpAdapterError::Protocol(format!(
            "missing compact mutation result projection for {tool_name}"
        ))),
    }
}

fn compact_mutation_effect(
    method_result: &Value,
) -> Result<McpMutationEffectSummary, McpAdapterError> {
    let method_result = method_result
        .get("agent_workflow_result")
        .unwrap_or(method_result);
    let base: ToolResultBase =
        serde_json::from_value(method_result["base"].clone()).map_err(McpAdapterError::Json)?;
    Ok(McpMutationEffectSummary {
        effect_kind: base.effect_kind,
        state_version: base.state_version,
        events: base.events,
    })
}

fn compact_request_user_action_result(
    effect: McpMutationEffectSummary,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let compound: McpRequestUserActionResponse =
        serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
    let agent_result = match compound.agent_workflow_result {
        volicord_types::schema::ToolResponse::Result(result) => result,
        _ => {
            return Err(McpAdapterError::Protocol(
                "request-user-action compact projection requires a result branch".to_owned(),
            ))
        }
    };
    let resolution_summary = compound
        .user_channel_resolution
        .as_ref()
        .map(|resolution| resolution.resolution_summary.clone());
    serde_json::to_value(McpRequestUserActionCompactResult {
        effect,
        agent_workflow_result_replayed: compound.agent_workflow_result_replayed,
        user_action_request_summary: agent_result.user_action_request_summary,
        current_projection_state_version: compound.current_projection_state_version,
        current_projection_observed_at: compound.current_projection_observed_at,
        user_action_resolution_ref: compound.user_channel_resolution_ref,
        status: compound.current_status,
        resolution_summary: resolution_summary.into(),
        derived_refs: compound.derived_refs,
    })
    .map_err(McpAdapterError::Json)
}

fn authority_receipt_compatibility_text(
    tool_name: &str,
    receipt: &AuthorityReceipt,
) -> Result<String, McpAdapterError> {
    let close_state = serde_json::to_value(receipt.close_state)
        .map_err(McpAdapterError::Json)?
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    let next_actor = serde_json::to_value(receipt.next_actor)
        .map_err(McpAdapterError::Json)?
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    Ok(bounded_mutation_compatibility_text(format!(
        "Volicord {tool_name} refreshed Task {} at state_version {}; close_state={close_state}; next_actor={next_actor}. Inspect the authoritative result for the authority receipt.",
        receipt.task_ref.record_id.as_str(),
        receipt.state_version,
    )))
}
