//! Bounded recovery for committed mutations whose ordinary MCP result cannot
//! be projected.

use crate::errors::McpAdapterError;
use crate::mutation_projection::{
    compact_mutation_method_result, MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
};
use crate::telemetry::ToolDiagnosticFacts;
use crate::tool_dispatch::{rendered_tool_call_output_size_for_capabilities, ToolCallOutput};
use crate::tool_registry::method_name_for_tool;
use serde_json::Value;
use volicord_mcp_protocol::{CommittedResultRecovery, McpProtocolCapabilities};
use volicord_mcp_wire::{
    McpAuthoritativeRefreshFailure, McpMutationFinalizationStage, McpMutationPostEffectFailure,
    McpMutationProjectionErrorCode, McpMutationResponseBudgetExceeded,
    McpMutationStructuredContent, McpOperationalErrorCode, McpPostEffectFailureCode,
    McpToolErrorCode, McpToolErrorIssue, McpToolErrorResponse, McpToolIssueCode,
    McpWorkflowContractDiagnostics,
};
use volicord_types::methods::OperationResultRef;
use volicord_types::schema::{
    AuthorityReceipt, RequiredNullable, TransitionRejection, WorkflowProjection,
};
use volicord_types::values::{EffectKind, MutationDetailLevel};

pub(crate) const MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CanonicalMcpMutationOutcome {
    pub(crate) tool_name: String,
    pub(crate) capabilities: McpProtocolCapabilities,
    pub(crate) requested_detail: MutationDetailLevel,
    pub(crate) facts: ToolDiagnosticFacts,
    pub(crate) exact_method_result: Option<Value>,
    pub(crate) compact_method_result: Option<Value>,
    pub(crate) operation_result_ref: Option<OperationResultRef>,
    pub(crate) authority_receipt: Option<AuthorityReceipt>,
    pub(crate) workflow: Option<WorkflowProjection>,
}

impl CanonicalMcpMutationOutcome {
    pub(crate) fn new(
        tool_name: &str,
        capabilities: McpProtocolCapabilities,
        requested_detail: MutationDetailLevel,
        facts: ToolDiagnosticFacts,
        exact_method_result: Option<Value>,
        operation_result_ref: Option<OperationResultRef>,
    ) -> Self {
        let compact_method_result = exact_method_result
            .as_ref()
            .and_then(|result| compact_mutation_method_result(tool_name, result).ok());
        Self {
            tool_name: tool_name.to_owned(),
            capabilities,
            requested_detail,
            facts,
            exact_method_result,
            compact_method_result,
            operation_result_ref,
            authority_receipt: None,
            workflow: None,
        }
    }

    pub(crate) fn set_authority_refresh(
        &mut self,
        authority_receipt: AuthorityReceipt,
        workflow: WorkflowProjection,
    ) {
        self.authority_receipt = Some(authority_receipt);
        self.workflow = Some(workflow);
    }

    fn recovery_candidates(
        &self,
        include_exact: bool,
    ) -> [Option<MutationRecoveryCandidate<'_>>; 5] {
        let receipt_and_exact = if include_exact {
            self.authority_receipt
                .as_ref()
                .zip(self.exact_method_result.as_ref())
                .map(|(receipt, method_result)| MutationRecoveryCandidate {
                    authority_receipt: Some(receipt),
                    method_result: Some(method_result),
                })
        } else {
            None
        };
        let receipt_and_compact = self
            .authority_receipt
            .as_ref()
            .zip(self.compact_method_result.as_ref())
            .map(|(receipt, method_result)| MutationRecoveryCandidate {
                authority_receipt: Some(receipt),
                method_result: Some(method_result),
            });
        let receipt_only =
            self.authority_receipt
                .as_ref()
                .map(|receipt| MutationRecoveryCandidate {
                    authority_receipt: Some(receipt),
                    method_result: None,
                });
        let compact_only =
            self.compact_method_result
                .as_ref()
                .map(|method_result| MutationRecoveryCandidate {
                    authority_receipt: None,
                    method_result: Some(method_result),
                });
        [
            receipt_and_exact,
            receipt_and_compact,
            receipt_only,
            compact_only,
            Some(MutationRecoveryCandidate {
                authority_receipt: None,
                method_result: None,
            }),
        ]
    }

    fn state_change_applied(&self) -> bool {
        self.facts.core_committed
    }

    fn validate_effect_facts(&self) -> Result<(), McpAdapterError> {
        let valid = match self.facts.effect_kind {
            Some(EffectKind::CoreCommitted) => {
                self.facts.effect_applied && (self.facts.core_committed ^ self.facts.replayed)
            }
            Some(EffectKind::StagingCreated) => {
                self.facts.effect_applied && !self.facts.core_committed && !self.facts.replayed
            }
            Some(EffectKind::NoEffect | EffectKind::ReadOnly) | None => {
                !self.facts.effect_applied && !self.facts.core_committed && !self.facts.replayed
            }
        };
        if valid {
            Ok(())
        } else {
            Err(McpAdapterError::Protocol(
                "canonical mutation effect facts are inconsistent".to_owned(),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MutationFinalizationFailureKind {
    Projection {
        post_effect_code: McpPostEffectFailureCode,
        transition_rejection: Option<TransitionRejection>,
    },
    ResponseBudgetExceeded(McpMutationProjectionErrorCode),
}

/// One phase-aware failure observed after an exact mutation method result exists.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MutationFinalizationFailure {
    pub(crate) stage: McpMutationFinalizationStage,
    pub(crate) failure_kind: MutationFinalizationFailureKind,
    pub(crate) workflow_contract_diagnostics: Option<McpWorkflowContractDiagnostics>,
}

impl MutationFinalizationFailure {
    pub(crate) fn workflow_contract(
        stage: McpMutationFinalizationStage,
        diagnostics: McpWorkflowContractDiagnostics,
        transition_rejection: Option<TransitionRejection>,
    ) -> Self {
        Self {
            stage,
            failure_kind: MutationFinalizationFailureKind::Projection {
                post_effect_code: McpPostEffectFailureCode::McpWorkflowContractProjectionFailed,
                transition_rejection,
            },
            workflow_contract_diagnostics: Some(diagnostics),
        }
    }

    pub(crate) fn response_projection(stage: McpMutationFinalizationStage) -> Self {
        Self {
            stage,
            failure_kind: MutationFinalizationFailureKind::Projection {
                post_effect_code: McpPostEffectFailureCode::McpResponseProjectionFailed,
                transition_rejection: None,
            },
            workflow_contract_diagnostics: None,
        }
    }

    pub(crate) fn post_effect_marker(code: McpPostEffectFailureCode) -> Self {
        let stage = match code {
            McpPostEffectFailureCode::McpWorkflowContractProjectionFailed => {
                McpMutationFinalizationStage::WorkflowPresentation
            }
            McpPostEffectFailureCode::McpResponseProjectionFailed => {
                McpMutationFinalizationStage::ResponseProjection
            }
            McpPostEffectFailureCode::McpPostEffectAdapterFailed => {
                McpMutationFinalizationStage::PostEffectAdapter
            }
        };
        Self {
            stage,
            failure_kind: MutationFinalizationFailureKind::Projection {
                post_effect_code: code,
                transition_rejection: None,
            },
            workflow_contract_diagnostics: None,
        }
    }

    pub(crate) fn response_budget_exceeded() -> Self {
        Self {
            stage: McpMutationFinalizationStage::ResponseProjection,
            failure_kind: MutationFinalizationFailureKind::ResponseBudgetExceeded(
                McpMutationProjectionErrorCode::McpResponseBudgetExceeded,
            ),
            workflow_contract_diagnostics: None,
        }
    }

    fn post_effect_code(&self) -> Option<McpPostEffectFailureCode> {
        match self.failure_kind {
            MutationFinalizationFailureKind::Projection {
                post_effect_code, ..
            } => Some(post_effect_code),
            MutationFinalizationFailureKind::ResponseBudgetExceeded(_) => None,
        }
    }

    fn transition_rejection(&self) -> Option<&TransitionRejection> {
        match &self.failure_kind {
            MutationFinalizationFailureKind::Projection {
                transition_rejection,
                ..
            } => transition_rejection.as_ref(),
            MutationFinalizationFailureKind::ResponseBudgetExceeded(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MutationRecoveryCandidate<'a> {
    authority_receipt: Option<&'a AuthorityReceipt>,
    method_result: Option<&'a Value>,
}

fn select_bounded_mutation_recovery<F>(
    outcome: &CanonicalMcpMutationOutcome,
    include_exact: bool,
    exhausted_message: &'static str,
    build_output: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: Fn(&MutationRecoveryCandidate<'_>) -> Result<ToolCallOutput, McpAdapterError>,
{
    match outcome
        .capabilities
        .result_recovery()
        .committed_result_recovery()
    {
        CommittedResultRecovery::PreserveAuthorityThenCompactResult => {}
    }
    for candidate in outcome
        .recovery_candidates(include_exact)
        .into_iter()
        .flatten()
    {
        let output = build_output(&candidate)?;
        if rendered_tool_call_output_size_for_capabilities(&output, outcome.capabilities)?
            <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        {
            return Ok(output);
        }
    }
    Err(McpAdapterError::Protocol(exhausted_message.to_owned()))
}

fn aligned_workflow_contract_diagnostics(
    outcome: &CanonicalMcpMutationOutcome,
    failure: &MutationFinalizationFailure,
) -> Option<McpWorkflowContractDiagnostics> {
    failure
        .workflow_contract_diagnostics
        .as_ref()
        .cloned()
        .map(|mut diagnostics| {
            diagnostics.committed = outcome.facts.core_committed;
            diagnostics.state_change_applied = outcome.state_change_applied();
            diagnostics
        })
}

pub(crate) fn project_mutation_finalization_failure(
    outcome: &CanonicalMcpMutationOutcome,
    failure: &MutationFinalizationFailure,
) -> Result<ToolCallOutput, McpAdapterError> {
    outcome.validate_effect_facts()?;
    match (&failure.failure_kind, outcome.facts.effect_applied) {
        (_, false) => pre_effect_internal_contract_rejection(outcome, failure),
        (MutationFinalizationFailureKind::ResponseBudgetExceeded(code), true) => {
            mutation_response_budget_exceeded_output(outcome, *code)
        }
        (MutationFinalizationFailureKind::Projection { .. }, true) => {
            mutation_post_effect_failure_output(outcome, failure)
        }
    }
}

fn pre_effect_internal_contract_rejection(
    outcome: &CanonicalMcpMutationOutcome,
    failure: &MutationFinalizationFailure,
) -> Result<ToolCallOutput, McpAdapterError> {
    if outcome.facts.effect_applied {
        return Err(McpAdapterError::Protocol(
            "pre-effect internal contract rejection cannot carry an applied effect".to_owned(),
        ));
    }
    let diagnostics = aligned_workflow_contract_diagnostics(outcome, failure);
    let transition_rejection = failure.transition_rejection().cloned();
    let workflow_contract_failed = diagnostics.is_some();
    let mut structured = McpToolErrorResponse {
        code: McpToolErrorCode::InternalContractInconsistent,
        tool_name: outcome.tool_name.clone(),
        selected_variant: RequiredNullable::new(transition_rejection.as_ref().map(|rejection| {
            rejection
                .attempted_action_key
                .semantic_variant
                .as_str()
                .to_owned()
        })),
        canonical_example: RequiredNullable::null(),
        retryable: false,
        reached_core: outcome.facts.core_reached,
        committed: false,
        failed_action_key: diagnostics
            .as_ref()
            .map(|value| value.failed_action_key.clone())
            .unwrap_or_else(RequiredNullable::null),
        failed_stage: diagnostics
            .as_ref()
            .map(|value| value.failed_stage.clone())
            .unwrap_or_else(RequiredNullable::null),
        method_error_code: diagnostics
            .as_ref()
            .map(|value| value.method_error_code.clone())
            .unwrap_or_else(RequiredNullable::null),
        method_error_details: diagnostics
            .as_ref()
            .map(|value| value.method_error_details.clone())
            .unwrap_or_else(RequiredNullable::null),
        state_change_applied: false,
        reported_issue_count: 1,
        truncated: false,
        issues: vec![McpToolErrorIssue::new(
            String::new(),
            McpToolIssueCode::InternalContractInconsistent,
            if workflow_contract_failed {
                "The current workflow form contract is internally inconsistent; the rejected witness is not executable and must not be retried."
            } else {
                "Mutation finalization encountered an internal contract failure before any authoritative effect; the call must not be retried."
            },
        )],
        authoritative_context: RequiredNullable::null(),
        retry_contract: RequiredNullable::null(),
        failure: RequiredNullable::null(),
        workflow_admission: RequiredNullable::null(),
        action_form_argument_mismatches: Vec::new(),
        transition_rejection: RequiredNullable::new(transition_rejection),
        contract_diagnostics: RequiredNullable::new(diagnostics),
    };
    if serde_json::to_vec(&structured)
        .map_err(McpAdapterError::Json)?
        .len()
        > MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
    {
        structured.contract_diagnostics = RequiredNullable::null();
        structured.truncated = true;
    }
    let structured_content = serde_json::to_value(
        McpMutationStructuredContent::<Value, Value>::AdapterError(Box::new(structured)),
    )
    .map_err(McpAdapterError::Json)?;
    let primary_text = if workflow_contract_failed {
        "Volicord rejected the mutation because the current workflow form contract is internally inconsistent; Core state is unchanged, the rejected witness is not executable, and no retry is suggested.".to_owned()
    } else {
        format!(
            "Volicord could not finalize {} because an internal contract is inconsistent. No authoritative effect was applied, no operation result is claimed, and this call must not be retried.",
            outcome.tool_name
        )
    };
    Ok(ToolCallOutput {
        primary_text: bounded_mutation_compatibility_text(primary_text),
        structured_content,
        extra_texts: Vec::new(),
        is_error: true,
        diagnostic_facts: outcome.facts.clone(),
        operation_result_ref: None,
        mutation_refresh_context: None,
        post_effect_failure: None,
    })
}

fn mutation_response_budget_exceeded_output(
    outcome: &CanonicalMcpMutationOutcome,
    code: McpMutationProjectionErrorCode,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let requested_detail = outcome.requested_detail;
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let requested_detail_label = match requested_detail {
        MutationDetailLevel::Summary => "summary",
        MutationDetailLevel::Workflow => "workflow",
        MutationDetailLevel::Full => "full",
    };
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = false;
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let receipt_preserved = candidate.authority_receipt.is_some();
            let method_result_preserved = candidate.method_result.is_some();
            let structured_content =
                serde_json::to_value(McpMutationResponseBudgetExceeded::<Value> {
                    code,
                    tool_name: method_name,
                    requested_detail,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    replayed: facts.replayed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
                    state_change_applied: facts.core_committed,
                    effect_anchor: facts.effect_anchor.clone().into(),
                    operation_result_ref: outcome.operation_result_ref.clone().into(),
                    authority_receipt: candidate.authority_receipt.cloned().into(),
                    method_result: RequiredNullable::new(candidate.method_result.cloned()),
                    authoritative_refresh_succeeded: true,
                    response_projection_omitted: true,
                    status_read_required: true,
                    completion_claim_withheld: true,
                })
                .map_err(McpAdapterError::Json)?;
            let preserved_guidance = match (receipt_preserved, method_result_preserved) {
                (true, true) => {
                    "The fresh authority receipt and compact method_result are preserved"
                }
                (true, false) => {
                    "The fresh authority receipt is preserved; the compact method_result exceeded the recovery budget"
                }
                (false, true) => {
                    "The compact method_result is preserved; the fresh authority receipt exceeded the recovery budget"
                }
                (false, false) => {
                    "The fresh authority receipt and compact method_result exceeded the recovery budget"
                }
            };
            let exact_result_guidance = if outcome.operation_result_ref.is_some() {
                " Retrieve the exact historical result with volicord.get_operation_result."
            } else {
                ""
            };
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord {tool_name} reached Core (effect_applied={}, committed={}) and refreshed current authority, but the requested {requested_detail_label} projection exceeded the MCP response budget. {preserved_guidance}.{exact_result_guidance} Read volicord.status before making an authority claim. Do not retry this mutation.",
                    facts.effect_applied, facts.core_committed
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        false,
        "bounded mutation budget recovery exceeded its fixed output budget",
        build_output,
    )
}

fn mutation_post_effect_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
    failure: &MutationFinalizationFailure,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let requested_detail = outcome.requested_detail;
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = false;
    let code = failure.post_effect_code().ok_or_else(|| {
        McpAdapterError::Protocol(
            "post-effect recovery requires a post-effect failure code".to_owned(),
        )
    })?;
    let contract_diagnostics = aligned_workflow_contract_diagnostics(outcome, failure);
    let failed_action_key = contract_diagnostics
        .as_ref()
        .map(|diagnostics| diagnostics.failed_action_key.clone())
        .unwrap_or_else(RequiredNullable::null);
    let method_error_code = contract_diagnostics
        .as_ref()
        .map(|diagnostics| diagnostics.method_error_code.clone())
        .unwrap_or_else(RequiredNullable::null);
    let method_error_details = contract_diagnostics
        .as_ref()
        .map(|diagnostics| diagnostics.method_error_details.clone())
        .unwrap_or_else(RequiredNullable::null);
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let method_result = candidate
                .method_result
                .map(|method_result| {
                    method_result.as_object().cloned().ok_or_else(|| {
                        McpAdapterError::Protocol(
                            "post-effect method_result must remain a JSON object".to_owned(),
                        )
                    })
                })
                .transpose()?;
            let structured_content = serde_json::to_value(McpMutationPostEffectFailure {
                code,
                tool_name: method_name,
                requested_detail,
                retryable: false,
                reached_core: facts.core_reached,
                committed: facts.core_committed,
                replayed: facts.replayed,
                effect_kind: facts.effect_kind.into(),
                effect_applied: facts.effect_applied,
                state_change_applied: facts.core_committed,
                effect_anchor: facts.effect_anchor.clone().into(),
                operation_result_ref: outcome.operation_result_ref.clone().into(),
                authority_receipt: candidate.authority_receipt.cloned().into(),
                method_result: method_result.into(),
                failed_action_key: failed_action_key.clone(),
                failed_stage: failure.stage,
                method_error_code: method_error_code.clone(),
                method_error_details: method_error_details.clone(),
                contract_diagnostics: RequiredNullable::new(contract_diagnostics.clone()),
                authoritative_refresh_succeeded: true,
                response_projection_omitted: true,
                status_read_required: true,
                completion_claim_withheld: true,
            })
            .map_err(McpAdapterError::Json)?;
            let primary_text = post_effect_failure_compatibility_text(outcome, code);
            Ok(ToolCallOutput {
                primary_text,
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        true,
        "bounded post-effect recovery exceeded its fixed output budget",
        build_output,
    )
}

fn post_effect_failure_compatibility_text(
    outcome: &CanonicalMcpMutationOutcome,
    code: McpPostEffectFailureCode,
) -> String {
    let retrieval = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact operation result with volicord.get_operation_result."
    } else {
        ""
    };
    let message = if code == McpPostEffectFailureCode::McpWorkflowContractProjectionFailed {
        match (
            outcome.facts.effect_kind,
            outcome.facts.core_committed,
            outcome.facts.replayed,
        ) {
            (Some(EffectKind::CoreCommitted), true, false) => format!(
                "The mutation was committed, but the next-state workflow form contract could not be projected. Do not retry this mutation.{retrieval} Read volicord.status before continuing."
            ),
            (Some(EffectKind::StagingCreated), false, false) => {
                "The staging effect was created, but the next-state workflow form contract could not be projected. Do not stage the same artifact again. Read volicord.status before continuing."
                    .to_owned()
            }
            (Some(EffectKind::CoreCommitted), false, true) => format!(
                "The exact committed result was replayed, but the next-state workflow form contract could not be projected. No new commit was created.{retrieval} Read volicord.status before continuing."
            ),
            _ => format!(
                "An authoritative effect was applied, but the next-state workflow form contract could not be projected. Do not retry this mutation.{retrieval} Read volicord.status before continuing."
            ),
        }
    } else {
        format!(
            "Volicord {} preserved an applied mutation effect and refreshed current authority, but finalization failed before the normal response could be projected. Do not retry this mutation.{retrieval} Read volicord.status before acting.",
            outcome.tool_name
        )
    };
    bounded_mutation_compatibility_text(message)
}

pub(crate) fn authoritative_refresh_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = true;
    let exact_result_guidance = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact historical result with volicord.get_operation_result, then"
    } else {
        ""
    };
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let method_result_preserved = candidate.method_result.is_some();
            let structured_content =
                serde_json::to_value(McpAuthoritativeRefreshFailure::<Value> {
                    code: McpOperationalErrorCode::Unavailable,
                    tool_name: method_name,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    replayed: facts.replayed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
                    state_change_applied: facts.core_committed,
                    effect_anchor: facts.effect_anchor.clone().into(),
                    operation_result_ref: outcome.operation_result_ref.clone().into(),
                    method_result: RequiredNullable::new(candidate.method_result.cloned()),
                    status_read_required: true,
                    completion_claim_withheld: true,
                })
                .map_err(McpAdapterError::Json)?;
            let method_result_guidance = if method_result_preserved {
                "The compact method_result is preserved"
            } else {
                "The compact method_result could not be included"
            };
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord withheld the {tool_name} success or completion claim because authoritative status refresh was unavailable. {method_result_guidance}.{exact_result_guidance} Read volicord.status before acting. Do not retry this mutation."
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        false,
        "bounded authoritative refresh recovery exceeded its fixed output budget",
        build_output,
    )
}

pub(crate) fn bounded_mutation_compatibility_text(text: String) -> String {
    if text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES {
        return text;
    }
    "Volicord omitted an oversized compatibility summary without truncating it. Inspect the authoritative result carrier for the complete result."
        .to_owned()
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use volicord_mcp_protocol::ProtocolRegistry;
    use volicord_mcp_wire::{McpMutationFinalizationStage, McpWorkflowContractDiagnostics};
    use volicord_types::schema::RequiredNullable;
    use volicord_types::tool_names::AgentToolId;
    use volicord_types::values::{EffectKind, MutationDetailLevel};

    use super::{
        pre_effect_internal_contract_rejection, project_mutation_finalization_failure,
        CanonicalMcpMutationOutcome, MutationFinalizationFailure,
    };
    use crate::telemetry::ToolDiagnosticFacts;

    fn outcome(facts: ToolDiagnosticFacts) -> CanonicalMcpMutationOutcome {
        let effect_kind =
            serde_json::to_value(facts.effect_kind).expect("test effect kind should serialize");
        let replayed = facts.replayed;
        CanonicalMcpMutationOutcome {
            tool_name: AgentToolId::INTAKE.wire_name().to_owned(),
            capabilities: ProtocolRegistry::production()
                .preferred_server_profile()
                .capabilities(),
            requested_detail: MutationDetailLevel::Summary,
            facts,
            exact_method_result: Some(json!({
                "base": {
                    "response_kind": "result",
                    "effect_kind": effect_kind.clone()
                },
                "agent_workflow_result_replayed": replayed
            })),
            compact_method_result: Some(json!({
                "effect_kind": effect_kind,
                "agent_workflow_result_replayed": replayed
            })),
            operation_result_ref: None,
            authority_receipt: None,
            workflow: None,
        }
    }

    fn finalization_failure() -> MutationFinalizationFailure {
        MutationFinalizationFailure::response_projection(
            McpMutationFinalizationStage::ResponseProjection,
        )
    }

    #[test]
    fn staging_contract_projection_failure_preserves_staging_effect_facts() {
        let outcome = outcome(ToolDiagnosticFacts {
            core_reached: true,
            effect_kind: Some(EffectKind::StagingCreated),
            effect_applied: true,
            effect_anchor: Some("staged_artifact:handle_test".to_owned()),
            ..ToolDiagnosticFacts::default()
        });

        let projected = project_mutation_finalization_failure(
            &outcome,
            &MutationFinalizationFailure::workflow_contract(
                McpMutationFinalizationStage::ActionFormCatalog,
                test_contract_diagnostics(),
                None,
            ),
        )
        .expect("staging recovery");

        assert!(!projected.is_error);
        assert_eq!(
            projected.structured_content["code"],
            "MCP_WORKFLOW_CONTRACT_PROJECTION_FAILED"
        );
        assert_eq!(projected.structured_content["committed"], false);
        assert_eq!(projected.structured_content["replayed"], false);
        assert_eq!(
            projected.structured_content["effect_kind"],
            "staging_created"
        );
        assert_eq!(projected.structured_content["effect_applied"], true);
        assert_eq!(projected.structured_content["state_change_applied"], false);
        assert_eq!(
            projected.structured_content["effect_anchor"],
            "staged_artifact:handle_test"
        );
        assert!(projected
            .primary_text
            .contains("staging effect was created"));
        assert!(projected.primary_text.contains("Do not stage"));
    }

    #[test]
    fn replay_contract_projection_failure_preserves_replayed_effect_facts() {
        let outcome = outcome(ToolDiagnosticFacts {
            core_reached: true,
            replayed: true,
            effect_kind: Some(EffectKind::CoreCommitted),
            effect_applied: true,
            effect_anchor: Some("authority_event:event_replayed".to_owned()),
            ..ToolDiagnosticFacts::default()
        });

        let projected = project_mutation_finalization_failure(
            &outcome,
            &MutationFinalizationFailure::workflow_contract(
                McpMutationFinalizationStage::WorkflowPresentation,
                test_contract_diagnostics(),
                None,
            ),
        )
        .expect("replay recovery");

        assert!(!projected.is_error);
        assert_eq!(projected.structured_content["committed"], false);
        assert_eq!(projected.structured_content["replayed"], true);
        assert_eq!(
            projected.structured_content["effect_kind"],
            "core_committed"
        );
        assert_eq!(projected.structured_content["effect_applied"], true);
        assert_eq!(projected.structured_content["state_change_applied"], false);
        assert_eq!(
            projected.structured_content["effect_anchor"],
            "authority_event:event_replayed"
        );
        assert!(projected
            .primary_text
            .contains("exact committed result was replayed"));
        assert!(projected.primary_text.contains("No new commit was created"));
    }

    #[test]
    fn no_effect_projection_failure_remains_an_internal_error_not_a_method_rejection() {
        let facts = ToolDiagnosticFacts {
            core_reached: true,
            effect_kind: Some(EffectKind::NoEffect),
            ..ToolDiagnosticFacts::default()
        };
        let outcome = outcome(facts.clone());

        let projected = project_mutation_finalization_failure(&outcome, &finalization_failure())
            .expect("no-effect internal error");

        assert!(projected.is_error);
        assert_eq!(
            projected.structured_content["code"],
            "INTERNAL_CONTRACT_INCONSISTENT"
        );
        assert_eq!(projected.structured_content["committed"], false);
        assert_eq!(projected.structured_content["state_change_applied"], false);
        assert_eq!(projected.structured_content["retryable"], false);
        assert_eq!(
            projected.structured_content["transition_rejection"],
            Value::Null
        );
        assert!(projected.operation_result_ref.is_none());
        assert_eq!(projected.diagnostic_facts, facts);
        assert!(projected
            .primary_text
            .contains("No authoritative effect was applied"));
        assert!(!projected.primary_text.contains("mutation was rejected"));
    }

    #[test]
    fn pre_effect_helper_refuses_an_applied_effect_without_mutating_facts() {
        let facts = ToolDiagnosticFacts {
            core_reached: true,
            core_committed: true,
            effect_kind: Some(EffectKind::CoreCommitted),
            effect_applied: true,
            effect_anchor: Some("authority_event:event_committed".to_owned()),
            ..ToolDiagnosticFacts::default()
        };
        let outcome = outcome(facts.clone());

        let failure = pre_effect_internal_contract_rejection(&outcome, &finalization_failure())
            .expect_err("applied effects cannot enter pre-effect projection");

        assert!(failure
            .to_string()
            .contains("cannot carry an applied effect"));
        assert_eq!(outcome.facts, facts);
    }

    fn test_contract_diagnostics() -> McpWorkflowContractDiagnostics {
        let workflow = volicord_types::schema::WorkflowProjection::NoActiveTask {
            next_actor: volicord_types::values::AuthorityNextActor::Agent,
            required_refs: Vec::new(),
            expected_state_version: 0,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::null(),
            transition_catalog: volicord_types::schema::WorkflowTransitionCatalog::new(Vec::new())
                .expect("empty transition catalog"),
            close_readiness: volicord_types::schema::WorkflowCloseReadiness {
                assessment_required: false,
                current_close_basis_present: false,
            },
        };
        McpWorkflowContractDiagnostics {
            normalized_workflow_snapshot: workflow.clone(),
            current_transition_catalog: workflow.transition_catalog().clone(),
            current_action_forms: RequiredNullable::null(),
            attempted_action_key: RequiredNullable::null(),
            typed_rejection_reason: RequiredNullable::null(),
            recovery_action_key: RequiredNullable::null(),
            failed_action_key: RequiredNullable::null(),
            failed_stage: RequiredNullable::null(),
            planned_branch: RequiredNullable::null(),
            method_error_code: RequiredNullable::null(),
            method_error_details: RequiredNullable::null(),
            basis_state_version: RequiredNullable::null(),
            state_change_applied: false,
            committed: false,
            workflow_contract_digest:
                volicord_types::managed_guidance::workflow_contract_semantic_digest(),
            submission_contract_digest:
                volicord_types::managed_guidance::submission_contract_semantic_digest(),
            action_form_contract_digest:
                volicord_types::managed_guidance::action_form_contract_semantic_digest(),
            semantic_schema_digest: volicord_types::managed_guidance::mcp_semantic_schema_digest(),
            scalar_contract_digest:
                volicord_types::canonical_scalar::baseline_ref_scalar_contract_digest(),
        }
    }
}
