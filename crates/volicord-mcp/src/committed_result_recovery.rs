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
    McpAuthoritativeRefreshFailure, McpMutationPostEffectFailure, McpMutationProjectionErrorCode,
    McpMutationResponseBudgetExceeded, McpOperationalErrorCode, McpPostEffectFailureCode,
};
use volicord_types::methods::OperationResultRef;
use volicord_types::schema::{AuthorityReceipt, NextActionSummary, RequiredNullable};
use volicord_types::values::MutationDetailLevel;

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
    pub(crate) next_actions: Vec<NextActionSummary>,
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
            next_actions: Vec::new(),
        }
    }

    pub(crate) fn set_authority_refresh(
        &mut self,
        authority_receipt: AuthorityReceipt,
        next_actions: Vec<NextActionSummary>,
    ) {
        self.authority_receipt = Some(authority_receipt);
        self.next_actions = next_actions;
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

pub(crate) fn mutation_response_budget_exceeded_output(
    outcome: &CanonicalMcpMutationOutcome,
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
                    code: McpMutationProjectionErrorCode::McpResponseBudgetExceeded,
                    tool_name: method_name,
                    requested_detail,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
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

pub(crate) fn mutation_post_effect_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
    code: McpPostEffectFailureCode,
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
    let exact_result_guidance = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact historical result with volicord.get_operation_result."
    } else {
        ""
    };
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
                effect_kind: facts.effect_kind.into(),
                effect_applied: facts.effect_applied,
                effect_anchor: facts.effect_anchor.clone().into(),
                operation_result_ref: outcome.operation_result_ref.clone().into(),
                authority_receipt: candidate.authority_receipt.cloned().into(),
                method_result: method_result.into(),
                authoritative_refresh_succeeded: true,
                response_projection_omitted: true,
                status_read_required: true,
                completion_claim_withheld: true,
            })
            .map_err(McpAdapterError::Json)?;
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord {tool_name} observed an applied mutation effect and refreshed current authority, but post-effect adapter work could not produce the normal response projection. Do not retry this mutation; inspect {exact_result_guidance} in the authoritative result. Read volicord.status before acting."
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
        true,
        "bounded post-effect recovery exceeded its fixed output budget",
        build_output,
    )
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
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
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
