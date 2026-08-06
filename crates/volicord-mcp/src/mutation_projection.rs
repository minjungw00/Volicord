//! Normal mutation result projection after authoritative refresh.

#[cfg(test)]
use crate::action_form::workflow_action_form_catalog;
use crate::action_form::{retry_contract, ActionFormCatalogError};
use crate::adapter::McpAdapter;
use crate::authority_refresh::{
    refresh_authority_status, validated_authority_refresh, MutationRefreshContext,
    ValidatedMutationAuthority,
};
use crate::binding::managed_agent_session_binding;
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
    McpAdvanceTaskCompactResult, McpAgentStateChange, McpArgumentFailurePresentation,
    McpFinalizeAdviceCompactResult, McpMustSurfaceFact, McpMutationEffectSummary,
    McpMutationFullResponse, McpMutationStructuredContent, McpMutationSummaryResponse,
    McpMutationWorkflowResponse, McpPostEffectFailureCode, McpPrepareEvidenceCaptureCompactResult,
    McpPrepareWriteCompactResult, McpReconcileChangesCompactResult, McpRecordRunCloseBasisAnchor,
    McpRecordRunCompactResult, McpRecordShapingCheckpointCompactResult,
    McpRequestUserActionCompactResult, McpRequestUserActionResponse, McpStageArtifactCompactResult,
    McpTaskPhasePresentation, McpToolErrorCode, McpToolErrorIssue, McpToolErrorResponse,
    McpToolIssueCode, McpUpdateScopeCompactResult, McpUserChannelInstructions,
    McpWorkflowBlockerSummary, McpWorkflowContractDiagnostics, McpWorkflowDryRunResponse,
    McpWorkflowPresentation, McpWorkflowRejectedResponse, RetryContract,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::RecordId;
use volicord_types::methods::{
    AdvanceTaskResult, CloseTaskResult, FinalizeAdviceResult, IntakeResult, MethodResultBase,
    PrepareEvidenceCaptureResult, PrepareWriteResult, PublicMethodResult, ReconcileChangesResult,
    RecordRunResult, RecordShapingCheckpointResult, StageArtifactResult, UpdateScopeResult,
};
use volicord_types::schema::{
    AuthorityReceipt, PreviewableToolResponse, RequiredNullable, StateRecordRef,
    ToolDryRunResponse, ToolRejectedResponse, TransitionAttemptDetails, TransitionRejection,
};
use volicord_types::tool_names::{AgentToolCategory, AgentToolId, AgentToolOwner};
use volicord_types::values::{EffectKind, MethodName, MutationDetailLevel, StateRecordKind};
use volicord_user_action_presentation::canonical_user_channel_instructions;

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
        |context, authority| {
            let binding =
                managed_agent_session_binding(&state.codex_binding, &state.runtime_session_id);
            let coordinates = binding
                .as_ref()
                .map(|binding| {
                    adapter.ensure_agent_session_binding(
                        mutation_context,
                        &context.project_id,
                        binding,
                    )
                })
                .transpose()
                .map_err(|error| ActionFormCatalogError {
                    action_key: None,
                    stage: volicord_mcp_wire::McpWorkflowContractStage::AdapterProjection,
                    detail: error.to_string(),
                })?;
            adapter.validated_workflow_action_form_catalog(
                mutation_context,
                &authority.receipt.project_id,
                &authority.workflow,
                coordinates.as_ref().map(|value| value.borrowed()),
            )
        },
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
        |_, authority| {
            workflow_action_form_catalog(
                &authority.receipt.project_id,
                &authority.workflow,
                |_, _| Ok(()),
            )
        },
    )
}

fn finalize_mutation_output_with_refresh_for_capabilities<F, G>(
    tool_name: &str,
    capabilities: McpProtocolCapabilities,
    detail: Option<MutationDetailLevel>,
    mut output: ToolCallOutput,
    refresh: F,
    action_forms: G,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: FnOnce(&MutationRefreshContext) -> Result<PipelineResponse, McpAdapterError>,
    G: FnOnce(
        &MutationRefreshContext,
        &ValidatedMutationAuthority,
    ) -> Result<volicord_mcp_wire::WorkflowActionFormCatalog, ActionFormCatalogError>,
{
    let Some(detail) = detail else {
        return Ok(output);
    };
    if output.is_error {
        return Ok(output);
    }
    let response_kind = response_kind_from_structured_content(&output.structured_content)
        .ok_or_else(|| {
            McpAdapterError::Protocol(format!(
                "mutation tool {tool_name} returned no response_kind"
            ))
        })?
        .to_owned();
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
    let authority = match refresh(&context) {
        Ok(response) => match validated_authority_refresh(&context, &response) {
            Ok(refreshed) => refreshed,
            Err(()) => return authoritative_refresh_failure_output(&outcome),
        },
        Err(_) => return authoritative_refresh_failure_output(&outcome),
    };
    outcome.set_authority_refresh(authority.receipt.clone(), authority.workflow.clone());
    let authority_receipt = outcome
        .authority_receipt
        .as_ref()
        .expect("validated canonical mutation outcome requires an authority receipt");

    if let Some(code) = output.post_effect_failure {
        return mutation_post_effect_failure_output(&outcome, code);
    }

    let rejected_method_result = if response_kind == "rejected" {
        let method_result: ToolRejectedResponse = serde_json::from_value(
            outcome
                .exact_method_result
                .clone()
                .expect("rejected mutation requires an exact result"),
        )
        .map_err(McpAdapterError::Json)?;
        let transition_rejection = method_result.errors().iter().find_map(|error| {
            error.details().and_then(|details| {
                serde_json::from_value::<TransitionRejection>(Value::Object(details.clone())).ok()
            })
        });
        Some((method_result, transition_rejection))
    } else {
        None
    };

    let action_form_catalog = match action_forms(&context, &authority) {
        Ok(catalog) => catalog,
        Err(failure) => {
            let diagnostics = workflow_contract_diagnostics_with_failure(
                &authority.workflow,
                failure.action_key,
                failure.stage,
            );
            return internal_contract_inconsistent_rejection(output, tool_name, None, diagnostics);
        }
    };

    let presentation = match workflow_presentation(
        tool_name,
        &response_kind,
        outcome.facts.replayed,
        outcome
            .exact_method_result
            .as_ref()
            .expect("canonical mutation outcome requires an exact result"),
        &authority,
        action_form_catalog,
    ) {
        Ok(presentation) => presentation,
        Err(McpAdapterError::SchemaContractFailure { .. }) if rejected_method_result.is_some() => {
            let transition_rejection = rejected_method_result
                .as_ref()
                .and_then(|(_, rejection)| rejection.clone());
            let diagnostics = workflow_contract_diagnostics(
                &authority.workflow,
                None,
                transition_rejection.as_ref(),
            );
            return internal_contract_inconsistent_rejection(
                output,
                tool_name,
                transition_rejection,
                diagnostics,
            );
        }
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };

    if let Some((method_result, transition_rejection)) = rejected_method_result {
        let diagnostics = workflow_contract_diagnostics(
            &authority.workflow,
            Some(&presentation.action_form_catalog),
            transition_rejection.as_ref(),
        );
        let retry = match transition_rejection.as_ref().and_then(|rejection| {
            rejection
                .recovery_action_key
                .as_ref()
                .map(|recovery_action_key| {
                    retry_contract(
                        rejection.attempted_action_key,
                        *recovery_action_key,
                        &authority.workflow,
                        &presentation.action_form_catalog,
                        rejection.attempt_details.clone(),
                        rejection.incompatible_submitted_paths.clone(),
                    )
                })
        }) {
            Some(Ok(retry)) => Some(retry),
            Some(Err(_)) => {
                return internal_contract_inconsistent_rejection(
                    output,
                    tool_name,
                    transition_rejection,
                    diagnostics,
                )
            }
            None => None,
        };
        let baseline_compatibility = transition_rejection
            .as_ref()
            .and_then(TransitionRejection::baseline_compatibility);
        output.primary_text = rejected_compatibility_text(
            tool_name,
            &presentation,
            transition_rejection.as_ref(),
            retry.as_ref(),
        );
        output.structured_content = serde_json::to_value(McpWorkflowRejectedResponse {
            method_result,
            authority_receipt: authority.receipt.clone(),
            workflow: authority.workflow.clone(),
            transition_rejection: RequiredNullable::new(transition_rejection.clone()),
            retry_contract: RequiredNullable::new(retry),
            failure: McpArgumentFailurePresentation {
                method_committed: false,
                reached_core: true,
                current_task_phase: RequiredNullable::some(authority.work_phase),
                current_state_version: RequiredNullable::some(authority.receipt.state_version),
                checkpoint_recorded: false,
                user_action_created: false,
                product_repository_changed: false,
                core_state_unchanged: true,
                current_baseline_canonical: RequiredNullable::new(
                    baseline_compatibility.map(|facts| facts.current_baseline_canonical),
                ),
                submitted_baseline_canonical: RequiredNullable::new(
                    baseline_compatibility.map(|facts| facts.submitted_baseline_canonical),
                ),
                submitted_baseline_matches_current: RequiredNullable::new(
                    baseline_compatibility.map(|facts| facts.submitted_baseline_matches_current),
                ),
                submitted_baseline_compatible_with_transition: RequiredNullable::new(
                    baseline_compatibility
                        .map(|facts| facts.submitted_baseline_compatible_with_transition),
                ),
                exact_retry_action: RequiredNullable::null(),
                repair_required: false,
            },
            contract_diagnostics: diagnostics,
            presentation,
        })
        .map_err(McpAdapterError::Json)?;
        output.mutation_refresh_context = None;
        return Ok(output);
    }
    if response_kind == "dry_run" {
        let method_result: ToolDryRunResponse = serde_json::from_value(
            outcome
                .exact_method_result
                .clone()
                .expect("dry-run mutation requires an exact result"),
        )
        .map_err(McpAdapterError::Json)?;
        output.primary_text = dry_run_compatibility_text(tool_name, &presentation);
        output.structured_content = serde_json::to_value(McpWorkflowDryRunResponse {
            method_result,
            authority_receipt: authority.receipt,
            workflow: authority.workflow,
            presentation,
        })
        .map_err(McpAdapterError::Json)?;
        output.mutation_refresh_context = None;
        return Ok(output);
    }
    if response_kind != "result" {
        return Err(McpAdapterError::Protocol(format!(
            "mutation tool {tool_name} returned unsupported response_kind={response_kind}"
        )));
    }

    output.primary_text = match authority_receipt_compatibility_text(
        tool_name,
        authority_receipt,
        presentation.state_change,
    ) {
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
            presentation: presentation.clone(),
        }),
        MutationDetailLevel::Workflow => serde_json::to_value(McpMutationWorkflowResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
            workflow: outcome
                .workflow
                .clone()
                .expect("validated canonical mutation outcome requires a workflow projection"),
            presentation: presentation.clone(),
        }),
        MutationDetailLevel::Full => serde_json::to_value(McpMutationFullResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
            presentation,
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

fn workflow_presentation(
    tool_name: &str,
    response_kind: &str,
    replayed: bool,
    method_result: &Value,
    authority: &ValidatedMutationAuthority,
    action_form_catalog: volicord_mcp_wire::WorkflowActionFormCatalog,
) -> Result<McpWorkflowPresentation, McpAdapterError> {
    let method = AgentToolId::from_wire_name(tool_name)
        .ok()
        .and_then(AgentToolId::method)
        .ok_or_else(|| {
            McpAdapterError::Protocol(format!(
                "missing MethodName mapping for mutation tool {tool_name}"
            ))
        })?;
    let state_change = mutation_state_change(response_kind, replayed, method_result)?;
    let task_phase = McpTaskPhasePresentation {
        mode: authority.task_mode,
        work_phase: authority.work_phase,
    };
    let mut must_surface = Vec::new();
    let mut blocker_summary = Vec::new();
    if response_kind == "rejected" {
        must_surface.push(McpMustSurfaceFact::MethodRejected {
            method,
            core_state_unchanged: volicord_types::schema::TrueValue,
        });
        must_surface.push(McpMustSurfaceFact::CurrentTaskPhase {
            mode: authority.task_mode,
            work_phase: authority.work_phase,
        });
        let rejected: ToolRejectedResponse =
            serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
        if let Some(details) = rejected.errors().iter().find_map(|error| {
            error.details().and_then(|details| {
                serde_json::from_value::<TransitionRejection>(Value::Object(details.clone())).ok()
            })
        }) {
            if let TransitionAttemptDetails::RecordRunKind {
                received_run_kind,
                allowed_run_kinds,
            } = &details.attempt_details
            {
                must_surface.push(McpMustSurfaceFact::RecordRunKindRejected {
                    received_run_kind: *received_run_kind,
                    allowed_run_kinds: allowed_run_kinds.clone(),
                });
            }
            if let Some(recovery) = details.recovery_action_key.as_ref() {
                must_surface.push(McpMustSurfaceFact::RecoveryAction {
                    action_key: *recovery,
                });
            }
            blocker_summary.push(McpWorkflowBlockerSummary {
                code: rejected
                    .errors()
                    .first()
                    .map(|error| RequiredNullable::some(error.code()))
                    .unwrap_or_else(RequiredNullable::null),
                owner_method: details
                    .recovery_action_key
                    .as_ref()
                    .map_or(details.attempted_action_key.method, |key| key.method),
                required_refs: details.blocking_refs,
                user_actions: Vec::new(),
            });
        } else {
            blocker_summary.extend(rejected.errors().iter().map(|error| {
                McpWorkflowBlockerSummary {
                    code: RequiredNullable::some(error.code()),
                    owner_method: method,
                    required_refs: authority.workflow.required_refs().to_vec(),
                    user_actions: Vec::new(),
                }
            }));
        }
    }

    let mut authority_request_refs = blocker_summary
        .iter()
        .flat_map(|blocker| blocker.user_actions.iter())
        .map(|user_action| user_action.user_action_request_ref.clone())
        .collect::<Vec<_>>();
    authority_request_refs.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    authority_request_refs.dedup();
    if !authority_request_refs.is_empty() {
        must_surface.push(
            McpMustSurfaceFact::ImplementationBlockedUntilUserActionAuthoritySatisfied {
                request_refs: authority_request_refs,
            },
        );
    }

    if let Some(checkpoint) = workflow_checkpoint(&authority.workflow) {
        for gap in &checkpoint.gaps {
            let (Some(disposition), Some(request_ref)) = (
                gap.decision_authority_state.as_ref().copied(),
                gap.user_action_request_ref.as_ref().cloned(),
            ) else {
                continue;
            };
            let authority_granted = matches!(
                disposition,
                volicord_types::values::ShapingDecisionAuthorityState::AcceptedUnapplied
                    | volicord_types::values::ShapingDecisionAuthorityState::Applied
            );
            must_surface.push(McpMustSurfaceFact::ShapingDecisionOutcome {
                request_ref: request_ref.clone(),
                resolution_ref: gap.user_action_resolution_ref.clone(),
                disposition,
                authority_granted,
            });
            if disposition.recovery_reason().is_some() {
                must_surface.push(McpMustSurfaceFact::NonAuthorizingShapingDecision {
                    request_ref,
                    resolution_ref: gap.user_action_resolution_ref.clone(),
                    disposition,
                    recovery_action_key: RequiredNullable::new(
                        authority
                            .workflow
                            .transition_catalog()
                            .required_transition()
                            .map(|transition| transition.action_key),
                    ),
                    authority_granted: volicord_types::schema::FalseValue,
                    terminal_request_cannot_be_retried: volicord_types::schema::TrueValue,
                    successor_request_required_if_still_needed: volicord_types::schema::TrueValue,
                    chat_text_cannot_replace_successor: volicord_types::schema::TrueValue,
                    product_repository_mutation_available: volicord_types::schema::FalseValue,
                });
            }
        }
    }

    let required_user_action = if authority.workflow.next_actor()
        == volicord_types::values::AuthorityNextActor::User
    {
        let task_id = authority
            .receipt
            .task_ref
            .task_id
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                McpAdapterError::Protocol(
                    "authoritative User Channel presentation requires a Task coordinate".to_owned(),
                )
            })?;
        let instruction = canonical_user_channel_instructions(
            &authority.receipt.project_id,
            &task_id,
            authority.pending_user_action_refs.clone(),
        )
        .map_err(|error| McpAdapterError::Protocol(error.to_string()))?;
        must_surface.push(McpMustSurfaceFact::UserActionRequestExists {
            request_refs: instruction.request_refs.clone(),
        });
        must_surface.push(McpMustSurfaceFact::NextActorIsUser);
        must_surface.push(McpMustSurfaceFact::ChatReplyIsNotResolution);
        must_surface
            .push(McpMustSurfaceFact::ProductRepositoryMutationBlockedUntilUserChannelResolution);
        must_surface.push(
            McpMustSurfaceFact::ImplementationBlockedUntilUserActionAuthoritySatisfied {
                request_refs: instruction.request_refs.clone(),
            },
        );
        Some(McpUserChannelInstructions {
            channel_kind: instruction.channel_kind,
            list_command: instruction.list_command,
            request_refs: instruction.request_refs,
            chat_reply_is_resolution: instruction.chat_reply_is_resolution,
        })
    } else {
        None
    };

    if response_kind == "result"
        && method == MethodName::AdvanceTask
        && authority.work_phase == volicord_types::values::WorkPhase::Implementation
    {
        must_surface.push(McpMustSurfaceFact::EnteredImplementation);
        must_surface.push(McpMustSurfaceFact::PhaseTransitionCreatedNoWriteTicket);
        must_surface.push(
            McpMustSurfaceFact::ProductRepositoryWritesRequirePrepareWrite {
                owner_method: MethodName::PrepareWrite,
            },
        );
    }

    let headline = match state_change {
        McpAgentStateChange::Rejected => format!("{tool_name} was rejected by current workflow"),
        McpAgentStateChange::DryRun => format!("{tool_name} returned a dry-run preview"),
        McpAgentStateChange::CoreCommitted => format!("{tool_name} committed Core authority"),
        McpAgentStateChange::StagingCreated => format!("{tool_name} created staging state"),
        McpAgentStateChange::ReadOnlyResume => {
            format!("{tool_name} resumed current authority without mutation")
        }
        McpAgentStateChange::NoEffect => format!("{tool_name} returned without a state change"),
    };
    Ok(McpWorkflowPresentation {
        headline,
        state_change,
        task_phase,
        next_actor: authority.workflow.next_actor(),
        blocker_summary,
        required_user_action: required_user_action.into(),
        must_surface,
        action_form_catalog,
    })
}

fn mutation_state_change(
    response_kind: &str,
    replayed: bool,
    method_result: &Value,
) -> Result<McpAgentStateChange, McpAdapterError> {
    if response_kind == "rejected" {
        return Ok(McpAgentStateChange::Rejected);
    }
    if response_kind == "dry_run" {
        return Ok(McpAgentStateChange::DryRun);
    }
    if replayed
        || method_result
            .get("agent_workflow_result_replayed")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return Ok(McpAgentStateChange::ReadOnlyResume);
    }
    let effect_kind = method_result
        .pointer("/agent_workflow_result/base/effect_kind")
        .or_else(|| method_result.pointer("/base/effect_kind"))
        .cloned()
        .ok_or_else(|| {
            McpAdapterError::Protocol("mutation result is missing effect_kind".to_owned())
        })?;
    match serde_json::from_value::<EffectKind>(effect_kind).map_err(McpAdapterError::Json)? {
        EffectKind::CoreCommitted => Ok(McpAgentStateChange::CoreCommitted),
        EffectKind::StagingCreated => Ok(McpAgentStateChange::StagingCreated),
        EffectKind::ReadOnly => Ok(McpAgentStateChange::ReadOnlyResume),
        EffectKind::NoEffect => Ok(McpAgentStateChange::NoEffect),
    }
}

fn workflow_contract_diagnostics(
    workflow: &volicord_types::schema::WorkflowProjection,
    action_forms: Option<&volicord_mcp_wire::WorkflowActionFormCatalog>,
    rejection: Option<&TransitionRejection>,
) -> McpWorkflowContractDiagnostics {
    McpWorkflowContractDiagnostics {
        normalized_workflow_snapshot: workflow.clone(),
        current_transition_catalog: workflow.transition_catalog().clone(),
        current_action_forms: RequiredNullable::new(action_forms.cloned()),
        attempted_action_key: RequiredNullable::new(
            rejection.map(|rejection| rejection.attempted_action_key),
        ),
        typed_rejection_reason: RequiredNullable::new(rejection.map(|rejection| rejection.reason)),
        recovery_action_key: RequiredNullable::new(
            rejection.and_then(|rejection| rejection.recovery_action_key.as_ref().copied()),
        ),
        failed_action_key: RequiredNullable::null(),
        failed_stage: RequiredNullable::null(),
        workflow_contract_digest: action_forms.map_or_else(
            volicord_types::managed_guidance::workflow_contract_semantic_digest,
            |forms| forms.workflow_contract_digest.clone(),
        ),
        action_form_contract_digest: action_forms.map_or_else(
            volicord_types::managed_guidance::action_form_contract_semantic_digest,
            |forms| forms.action_form_contract_digest.clone(),
        ),
        semantic_schema_digest: action_forms.map_or_else(
            volicord_types::managed_guidance::mcp_semantic_schema_digest,
            |forms| forms.semantic_schema_digest.clone(),
        ),
        scalar_contract_digest: action_forms.map_or_else(
            volicord_types::canonical_scalar::baseline_ref_scalar_contract_digest,
            |forms| forms.scalar_contract_digest.clone(),
        ),
    }
}

fn workflow_contract_diagnostics_with_failure(
    workflow: &volicord_types::schema::WorkflowProjection,
    action_key: Option<volicord_types::schema::WorkflowActionKey>,
    stage: volicord_mcp_wire::McpWorkflowContractStage,
) -> McpWorkflowContractDiagnostics {
    let mut diagnostics = workflow_contract_diagnostics(workflow, None, None);
    diagnostics.failed_action_key = RequiredNullable::new(action_key);
    diagnostics.failed_stage = RequiredNullable::some(stage);
    diagnostics
}

fn internal_contract_inconsistent_rejection(
    mut output: ToolCallOutput,
    tool_name: &str,
    transition_rejection: Option<TransitionRejection>,
    diagnostics: McpWorkflowContractDiagnostics,
) -> Result<ToolCallOutput, McpAdapterError> {
    let mut structured = McpToolErrorResponse {
        code: McpToolErrorCode::InternalContractInconsistent,
        tool_name: tool_name.to_owned(),
        selected_variant: RequiredNullable::null(),
        canonical_example: RequiredNullable::null(),
        retryable: false,
        reached_core: true,
        committed: false,
        failed_action_key: diagnostics.failed_action_key.clone(),
        failed_stage: diagnostics.failed_stage.clone(),
        reported_issue_count: 1,
        truncated: false,
        issues: vec![McpToolErrorIssue::new(
            String::new(),
            McpToolIssueCode::InternalContractInconsistent,
            "Core named a recovery action that has no exact executable form in the current MCP projection.",
        )],
        authoritative_context: RequiredNullable::null(),
        retry_contract: RequiredNullable::null(),
        failure: RequiredNullable::null(),
        workflow_admission: RequiredNullable::null(),
        action_form_argument_mismatches: Vec::new(),
        transition_rejection: RequiredNullable::new(transition_rejection),
        contract_diagnostics: RequiredNullable::some(diagnostics),
    };
    if serde_json::to_vec(&structured)
        .map_err(McpAdapterError::Json)?
        .len()
        > MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
    {
        structured.contract_diagnostics = RequiredNullable::null();
        structured.truncated = true;
    }
    output.primary_text = "Volicord rejected the mutation and found an internal workflow-contract inconsistency; Core state is unchanged and no retry is suggested.".to_owned();
    output.structured_content = serde_json::to_value(
        McpMutationStructuredContent::<Value, Value>::AdapterError(Box::new(structured)),
    )
    .map_err(McpAdapterError::Json)?;
    output.is_error = true;
    output.diagnostic_facts.core_reached = true;
    output.diagnostic_facts.core_committed = false;
    output.diagnostic_facts.effect_applied = false;
    output.mutation_refresh_context = None;
    Ok(output)
}

fn rejected_compatibility_text(
    tool_name: &str,
    presentation: &McpWorkflowPresentation,
    rejection: Option<&TransitionRejection>,
    retry: Option<&RetryContract>,
) -> String {
    let recovery = presentation
        .must_surface
        .iter()
        .find_map(|fact| match fact {
            McpMustSurfaceFact::RecoveryAction { action_key } => Some(format!(
                "{}/{}",
                action_key.method.as_str(),
                serde_json::to_value(action_key.semantic_variant)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "unknown_variant".to_owned())
            )),
            _ => None,
        });
    let recovery_text = recovery.map_or_else(
        || "no current recovery transition is cataloged".to_owned(),
        |action_key| {
            if retry.is_some_and(|contract| contract.retry_possible_in_current_task) {
                format!("retry with the exact current form for {action_key}")
            } else {
                format!(
                    "current recovery {action_key} is not retryable through this attempted action"
                )
            }
        },
    );
    let attempt_text = rejection
        .and_then(|rejection| match &rejection.attempt_details {
            TransitionAttemptDetails::RecordRunKind {
                received_run_kind,
                allowed_run_kinds,
            } => Some(format!(
                "received Run kind={}; allowed Run kinds=[{}]; ",
                run_kind_text(*received_run_kind),
                allowed_run_kinds
                    .iter()
                    .map(|kind| run_kind_text(*kind))
                    .collect::<Vec<_>>()
                    .join(",")
            )),
            TransitionAttemptDetails::None
            | TransitionAttemptDetails::BaselineTransition { .. } => None,
        })
        .unwrap_or_default();
    bounded_mutation_compatibility_text(format!(
        "Volicord {tool_name} rejected the mutation; {attempt_text}current Task phase={}/{}; Core state is unchanged; {recovery_text}.",
        serde_json::to_value(presentation.task_phase.mode)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned()),
        serde_json::to_value(presentation.task_phase.work_phase)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned()),
    ))
}

const fn run_kind_text(kind: volicord_types::values::RunKind) -> &'static str {
    match kind {
        volicord_types::values::RunKind::Direct => "direct",
        volicord_types::values::RunKind::Implementation => "implementation",
    }
}

fn dry_run_compatibility_text(tool_name: &str, presentation: &McpWorkflowPresentation) -> String {
    bounded_mutation_compatibility_text(format!(
        "Volicord {tool_name} returned a dry-run preview; Core state is unchanged; current Task phase={}/{}.",
        serde_json::to_value(presentation.task_phase.mode)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned()),
        serde_json::to_value(presentation.task_phase.work_phase)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned()),
    ))
}

pub(crate) fn compact_mutation_method_result(
    tool_name: &str,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let tool = AgentToolId::from_wire_name(tool_name)
        .map_err(|_| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    match tool {
        AgentToolId::PREPARE_EVIDENCE_CAPTURE => {
            let result: PrepareEvidenceCaptureResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let effect = compact_mutation_effect(&result);
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
            let effect = compact_mutation_effect(&result);
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
            let effect = compact_mutation_effect(&result);
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
            let effect = compact_mutation_effect(&result);
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
        AgentToolId::REQUEST_USER_ACTION => compact_request_user_action_result(method_result),
        AgentToolId::RECONCILE_CHANGES => {
            let result: ReconcileChangesResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let effect = compact_mutation_effect(&result);
            serde_json::to_value(McpReconcileChangesCompactResult {
                effect,
                unresolved_changes: result.unresolved_changes,
                resolved_changes: result.resolved_changes,
                pending_user_action_summaries: result.pending_user_action_summaries,
                rejected_resolution_requests: result.rejected_resolution_requests,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::INTAKE => {
            let result: IntakeResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(compact_mutation_effect(&result)).map_err(McpAdapterError::Json)
        }
        AgentToolId::UPDATE_SCOPE => {
            let result: UpdateScopeResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpUpdateScopeCompactResult {
                effect: compact_mutation_effect(&result),
                applied_shaping_gap_refs: result.applied_shaping_gap_refs,
                applied_scope_decision_refs: result.applied_scope_decision_refs,
                applied_shaping_decision_application_refs: result
                    .applied_shaping_decision_application_refs,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::RECORD_SHAPING_CHECKPOINT => {
            let result: RecordShapingCheckpointResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let unresolved_application_owners = workflow_checkpoint(&result.workflow)
                .map(|checkpoint| checkpoint.unresolved_application_owners.clone())
                .unwrap_or_default();
            let decision_recovery_requirements = workflow_checkpoint(&result.workflow)
                .map(|checkpoint| checkpoint.decision_recovery_requirements.clone())
                .unwrap_or_default();
            serde_json::to_value(McpRecordShapingCheckpointCompactResult {
                effect: compact_mutation_effect(&result),
                shaping_checkpoint_id: result.shaping_checkpoint.shaping_checkpoint_id,
                readiness: result.shaping_checkpoint.readiness,
                unresolved_application_owners,
                decision_recovery_requirements,
                created_user_action_request_refs: result.created_user_action_request_refs,
                shaping_authority_reauthorization_refs: result
                    .shaping_authority_reauthorization_refs,
                workflow_kind: workflow_state_kind(&result.workflow),
                close_state: result.state.close_state,
                close_blocker_count: result.state.close_blockers.len(),
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::FINALIZE_ADVICE => {
            let result: FinalizeAdviceResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let unresolved_application_owners = workflow_checkpoint(&result.workflow)
                .map(|checkpoint| checkpoint.unresolved_application_owners.clone())
                .unwrap_or_default();
            let decision_recovery_requirements = workflow_checkpoint(&result.workflow)
                .map(|checkpoint| checkpoint.decision_recovery_requirements.clone())
                .unwrap_or_default();
            serde_json::to_value(McpFinalizeAdviceCompactResult {
                effect: compact_mutation_effect(&result),
                shaping_checkpoint_id: result.shaping_checkpoint.shaping_checkpoint_id,
                readiness: result.shaping_checkpoint.readiness,
                unresolved_application_owners,
                decision_recovery_requirements,
                applied_shaping_decision_application_refs: result
                    .applied_shaping_decision_application_refs,
                workflow_kind: workflow_state_kind(&result.workflow),
                close_state: result.state.close_state,
                close_blocker_count: result.state.close_blockers.len(),
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::ADVANCE_TASK => {
            let result: AdvanceTaskResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpAdvanceTaskCompactResult {
                effect: compact_mutation_effect(&result),
                applied_shaping_gap_refs: result.applied_shaping_gap_refs,
                applied_user_action_resolution_refs: result.applied_user_action_resolution_refs,
                applied_shaping_decision_application_refs: result
                    .applied_shaping_decision_application_refs,
            })
            .map_err(McpAdapterError::Json)
        }
        AgentToolId::CLOSE_TASK => {
            let result: CloseTaskResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(compact_mutation_effect(&result)).map_err(McpAdapterError::Json)
        }
        _ => Err(McpAdapterError::Protocol(format!(
            "missing compact mutation result projection for {tool_name}"
        ))),
    }
}

fn workflow_checkpoint(
    workflow: &volicord_types::schema::WorkflowProjection,
) -> Option<&volicord_types::schema::ShapingCheckpointSummary> {
    use volicord_types::schema::WorkflowProjection;

    match workflow {
        WorkflowProjection::NoActiveTask { checkpoint, .. }
        | WorkflowProjection::ShapingRequired { checkpoint, .. }
        | WorkflowProjection::AwaitingUserAction { checkpoint, .. }
        | WorkflowProjection::DecisionRecoveryRequired { checkpoint, .. }
        | WorkflowProjection::ReadyToApplyDecisions { checkpoint, .. }
        | WorkflowProjection::ReadyForChangeUnit { checkpoint, .. }
        | WorkflowProjection::ReadyToFinalizeAdvice { checkpoint, .. }
        | WorkflowProjection::ReadyForImplementation { checkpoint, .. }
        | WorkflowProjection::Implementation { checkpoint, .. }
        | WorkflowProjection::CloseReview { checkpoint, .. }
        | WorkflowProjection::Terminal { checkpoint, .. } => checkpoint.as_ref(),
    }
}

fn workflow_state_kind(
    workflow: &volicord_types::schema::WorkflowProjection,
) -> volicord_types::values::WorkflowStateKind {
    use volicord_types::schema::WorkflowProjection;
    use volicord_types::values::WorkflowStateKind;

    match workflow {
        WorkflowProjection::NoActiveTask { .. } => WorkflowStateKind::NoActiveTask,
        WorkflowProjection::ShapingRequired { .. } => WorkflowStateKind::ShapingRequired,
        WorkflowProjection::AwaitingUserAction { .. } => WorkflowStateKind::AwaitingUserAction,
        WorkflowProjection::DecisionRecoveryRequired { .. } => {
            WorkflowStateKind::DecisionRecoveryRequired
        }
        WorkflowProjection::ReadyToApplyDecisions { .. } => {
            WorkflowStateKind::ReadyToApplyDecisions
        }
        WorkflowProjection::ReadyForChangeUnit { .. } => WorkflowStateKind::ReadyForChangeUnit,
        WorkflowProjection::ReadyToFinalizeAdvice { .. } => {
            WorkflowStateKind::ReadyToFinalizeAdvice
        }
        WorkflowProjection::ReadyForImplementation { .. } => {
            WorkflowStateKind::ReadyForImplementation
        }
        WorkflowProjection::Implementation { .. } => WorkflowStateKind::Implementation,
        WorkflowProjection::CloseReview { .. } => WorkflowStateKind::CloseReview,
        WorkflowProjection::Terminal { .. } => WorkflowStateKind::Terminal,
    }
}

fn compact_mutation_effect<R>(method_result: &R) -> McpMutationEffectSummary
where
    R: PublicMethodResult,
{
    let base = method_result.base();
    McpMutationEffectSummary {
        effect_kind: base.effect_kind(),
        state_version: Some(base.state_version()),
        events: base.events().to_vec(),
    }
}

fn compact_request_user_action_result(method_result: &Value) -> Result<Value, McpAdapterError> {
    let compound: McpRequestUserActionResponse =
        serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
    let agent_result = match compound.agent_workflow_result {
        PreviewableToolResponse::Result(result) => result,
        _ => {
            return Err(McpAdapterError::Protocol(
                "request-user-action compact projection requires a result branch".to_owned(),
            ))
        }
    };
    let effect = compact_mutation_effect(&agent_result);
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
    state_change: McpAgentStateChange,
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
    let effect = match state_change {
        McpAgentStateChange::CoreCommitted => "committed Core authority",
        McpAgentStateChange::StagingCreated => "created staging state",
        McpAgentStateChange::ReadOnlyResume => "resumed current authority without mutation",
        McpAgentStateChange::NoEffect => "returned without a Core state change",
        McpAgentStateChange::DryRun => "returned a dry-run preview",
        McpAgentStateChange::Rejected => "rejected the mutation",
    };
    Ok(bounded_mutation_compatibility_text(format!(
        "Volicord {tool_name} {effect} for Task {} at state_version {}; close_state={close_state}; next_actor={next_actor}. Inspect the authority receipt and presentation.must_surface before reporting the result.",
        receipt.task_ref.record_id.as_str(),
        receipt.state_version,
    )))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use volicord_mcp_wire::{
        McpAgentStateChange, McpMustSurfaceFact, McpTaskPhasePresentation, McpWorkflowPresentation,
        RetryContract, WorkflowActionForm, WorkflowActionFormCatalog,
    };
    use volicord_types::ids::{RequestHash, TaskId};
    use volicord_types::schema::{
        RequiredNullable, TransitionAttemptDetails, TransitionDescriptor, TransitionRejection,
        WorkflowActionAuthorityCoordinates, WorkflowActionKey, WorkflowActionRole,
        WorkflowCloseReadiness, WorkflowProjection, WorkflowTransitionCatalog,
        WorkflowTransitionSubmissionContract,
    };
    use volicord_types::values::{
        AuthorityNextActor, MethodName, RunKind, TaskMode, TransitionRejectionReason, WorkPhase,
        WorkflowActionSemanticVariant, WorkflowAuthorityInvalidationPolicy,
        WorkflowExpectedResultState, WorkflowTransitionActor, WorkflowTransitionEffectClass,
    };

    use super::{
        internal_contract_inconsistent_rejection, rejected_compatibility_text,
        response_kind_from_structured_content, workflow_contract_diagnostics_with_failure,
    };
    use crate::tool_dispatch::ToolCallOutput;

    #[test]
    fn response_branch_projection_uses_response_kind_not_dry_run_metadata() {
        let regular_result_with_requested_intent = json!({
            "base": {
                "response_kind": "result",
                "dry_run": true
            }
        });
        assert_eq!(
            response_kind_from_structured_content(&regular_result_with_requested_intent),
            Some("result")
        );

        let compound_result_with_requested_intent = json!({
            "agent_workflow_result": {
                "base": {
                    "response_kind": "result",
                    "dry_run": true
                }
            }
        });
        assert_eq!(
            response_kind_from_structured_content(&compound_result_with_requested_intent),
            Some("result")
        );
    }

    #[test]
    fn missing_recovery_form_projects_bounded_internal_error_and_preserves_rejection() {
        let task_id = TaskId::new("task_contract_inconsistent");
        let action_key = WorkflowActionKey {
            method: MethodName::CloseTask,
            semantic_variant: WorkflowActionSemanticVariant::CloseTask,
        };
        let coordinates = WorkflowActionAuthorityCoordinates::CloseTask { task_id };
        let transition_catalog = WorkflowTransitionCatalog::new(vec![TransitionDescriptor {
            action_key,
            actor: WorkflowTransitionActor::Agent,
            role: WorkflowActionRole::Required,
            expected_state_version: 7,
            submission_contract: WorkflowTransitionSubmissionContract::for_current_transition(
                TaskMode::Work,
                &coordinates,
            ),
            fixed_authority_coordinates: coordinates,
            effect_class: WorkflowTransitionEffectClass::TerminalMutation,
            expected_result_state: WorkflowExpectedResultState::Terminal,
            authority_invalidation: WorkflowAuthorityInvalidationPolicy::Permitted,
            required_refs: Vec::new(),
        }])
        .expect("valid test transition catalog");
        let workflow = WorkflowProjection::ShapingRequired {
            next_actor: AuthorityNextActor::Agent,
            required_refs: Vec::new(),
            expected_state_version: 7,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::null(),
            transition_catalog: transition_catalog.clone(),
            close_readiness: WorkflowCloseReadiness {
                assessment_required: false,
                current_close_basis_present: false,
            },
        };
        let rejection = TransitionRejection::new(
            action_key,
            TransitionRejectionReason::ClosePreconditionMissing,
            volicord_types::schema::TransitionAttemptDetails::None,
            true,
            Some(action_key),
            Vec::new(),
            workflow.kind(),
            &transition_catalog,
        )
        .expect("valid typed rejection");
        let diagnostics = workflow_contract_diagnostics_with_failure(
            &workflow,
            Some(action_key),
            volicord_mcp_wire::McpWorkflowContractStage::CatalogTotality,
        );
        let output = internal_contract_inconsistent_rejection(
            ToolCallOutput::success("{}".to_owned()).expect("empty test output"),
            MethodName::CloseTask.as_str(),
            Some(rejection.clone()),
            diagnostics,
        )
        .expect("bounded internal error projection");

        assert!(output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "INTERNAL_CONTRACT_INCONSISTENT"
        );
        assert_eq!(output.structured_content["committed"], false);
        assert_eq!(output.structured_content["retry_contract"], Value::Null);
        assert_eq!(
            serde_json::from_value::<TransitionRejection>(
                output.structured_content["transition_rejection"].clone()
            )
            .expect("preserved transition rejection"),
            rejection
        );
        assert_eq!(
            output.structured_content["contract_diagnostics"]["current_action_forms"],
            Value::Null
        );
        assert_eq!(
            output.structured_content["failed_action_key"],
            json!(action_key)
        );
        assert_eq!(
            output.structured_content["failed_stage"],
            "catalog_totality"
        );
        assert_eq!(
            output.structured_content["contract_diagnostics"]["failed_action_key"],
            json!(action_key)
        );
        assert_eq!(
            output.structured_content["contract_diagnostics"]["failed_stage"],
            "catalog_totality"
        );
        assert!(output.primary_text.len() <= 512);
    }

    #[test]
    fn record_run_kind_rejection_structured_and_compact_projections_agree() {
        let task_id = TaskId::new("task_record_run_kind_rejected");
        let action_key = WorkflowActionKey {
            method: MethodName::RecordRun,
            semantic_variant: WorkflowActionSemanticVariant::RecordRun,
        };
        let coordinates = WorkflowActionAuthorityCoordinates::RecordRun {
            task_id,
            change_unit_id: volicord_types::ids::ChangeUnitId::new(
                "change_unit_record_run_kind_rejected",
            ),
            baseline_ref: volicord_types::ids::BaselineRef::parse(
                "baseline_record_run_kind_rejected",
            )
            .expect("canonical test baseline"),
            run_kind: RunKind::Direct,
        };
        let transition_catalog = WorkflowTransitionCatalog::new(vec![TransitionDescriptor {
            action_key,
            actor: WorkflowTransitionActor::Agent,
            role: WorkflowActionRole::Required,
            expected_state_version: 7,
            submission_contract: WorkflowTransitionSubmissionContract::for_current_transition(
                TaskMode::Direct,
                &coordinates,
            ),
            fixed_authority_coordinates: coordinates,
            effect_class: WorkflowTransitionEffectClass::ExecutionRecording,
            expected_result_state: WorkflowExpectedResultState::Implementation,
            authority_invalidation: WorkflowAuthorityInvalidationPolicy::Permitted,
            required_refs: Vec::new(),
        }])
        .expect("valid current RecordRun transition");
        let attempt_details = TransitionAttemptDetails::record_run_kind(
            RunKind::Implementation,
            vec![RunKind::Direct, RunKind::Direct],
        )
        .expect("canonical Run-kind rejection details");
        let rejection = TransitionRejection::new(
            action_key,
            TransitionRejectionReason::RunKindIncompatible,
            attempt_details.clone(),
            true,
            Some(action_key),
            Vec::new(),
            volicord_types::values::WorkflowStateKind::Implementation,
            &transition_catalog,
        )
        .expect("valid typed Run-kind rejection");

        let form = WorkflowActionForm {
            action_key,
            form_ref: RequestHash::new("form_record_run_kind_rejected"),
            expected_state_version: 7,
            fixed_arguments: serde_json::Map::new(),
            fixed_argument_paths: vec!["/kind".to_owned()],
            agent_authored_inputs: Vec::new(),
            canonical_minimal_request: serde_json::Map::new(),
        };
        let action_form_catalog = WorkflowActionFormCatalog {
            required_action_key: RequiredNullable::some(action_key),
            workflow_contract_digest: RequestHash::new("workflow_digest"),
            action_form_contract_digest: RequestHash::new("action_form_digest"),
            semantic_schema_digest: RequestHash::new("semantic_schema_digest"),
            scalar_contract_digest: RequestHash::new("scalar_contract_digest"),
            forms: vec![form.clone()],
        };
        let presentation = McpWorkflowPresentation {
            headline: "volicord.record_run was rejected by current workflow".to_owned(),
            state_change: McpAgentStateChange::Rejected,
            task_phase: McpTaskPhasePresentation {
                mode: TaskMode::Direct,
                work_phase: WorkPhase::Implementation,
            },
            next_actor: AuthorityNextActor::Agent,
            blocker_summary: Vec::new(),
            required_user_action: RequiredNullable::null(),
            must_surface: vec![
                McpMustSurfaceFact::MethodRejected {
                    method: MethodName::RecordRun,
                    core_state_unchanged: volicord_types::schema::TrueValue,
                },
                McpMustSurfaceFact::CurrentTaskPhase {
                    mode: TaskMode::Direct,
                    work_phase: WorkPhase::Implementation,
                },
                McpMustSurfaceFact::RecordRunKindRejected {
                    received_run_kind: RunKind::Implementation,
                    allowed_run_kinds: vec![RunKind::Direct],
                },
                McpMustSurfaceFact::RecoveryAction { action_key },
            ],
            action_form_catalog,
        };
        let retry = RetryContract {
            recovery_action_key: RequiredNullable::some(action_key),
            recovery_form: RequiredNullable::some(form),
            attempt_details,
            invalid_or_incompatible_submitted_paths: vec!["/kind".to_owned()],
            retry_possible_in_current_task: true,
        };

        let structured = serde_json::to_value(json!({
            "transition_rejection": rejection,
            "retry_contract": retry,
            "presentation": presentation,
        }))
        .expect("structured MCP rejection");
        assert_eq!(
            structured["transition_rejection"]["attempt_details"],
            structured["retry_contract"]["attempt_details"]
        );
        assert!(structured["presentation"]["must_surface"]
            .as_array()
            .expect("must-surface facts")
            .iter()
            .any(|fact| {
                fact == &json!({
                    "fact_kind": "record_run_kind_rejected",
                    "received_run_kind": "implementation",
                    "allowed_run_kinds": ["direct"]
                })
            }));

        let rejection: TransitionRejection =
            serde_json::from_value(structured["transition_rejection"].clone())
                .expect("typed structured rejection");
        let retry: RetryContract = serde_json::from_value(structured["retry_contract"].clone())
            .expect("typed structured retry contract");
        let presentation: McpWorkflowPresentation =
            serde_json::from_value(structured["presentation"].clone())
                .expect("typed structured presentation");
        let compact = rejected_compatibility_text(
            MethodName::RecordRun.as_str(),
            &presentation,
            Some(&rejection),
            Some(&retry),
        );
        assert!(compact.contains("received Run kind=implementation"));
        assert!(compact.contains("allowed Run kinds=[direct]"));
        assert!(compact.contains("current Task phase=direct/implementation"));
        assert!(compact.contains("Core state is unchanged"));
        assert!(compact
            .contains("retry with the exact current form for volicord.record_run/record_run"));
    }
}
