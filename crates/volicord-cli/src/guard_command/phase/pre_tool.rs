use std::collections::BTreeSet;

use serde_json::{json, Value};
use volicord_platform_fs::{
    InvocationObservationPaths, ObservationUnavailableReason, ObserverLimits,
    RepositoryObservationCheckpoint, RepositoryObserver, SemanticObserverContractDigest,
};
use volicord_store::{
    bootstrap::ProjectRecord,
    guards::{
        repository_observation_id, RepositoryExpectedWriteInsert,
        RepositoryObservationUnavailableReason,
    },
    RuntimeHomeMutationContext,
};
use volicord_types::guard_outcome::GuardPolicyDecision;
use volicord_types::product_path::ProductRelativePath;
use volicord_types::tool_names::ProductRepositoryEffect;
use volicord_types::values::UtcTimestamp;

use super::{GuardPhaseResult, RepositoryObservationMutation};
use crate::guard_command::{
    args::GuardInput,
    context::{guard_state_summary, ActiveWriteTicketSummary, GuardReason, GuardStateSummary},
    envelope::{event_time, GuardEnvelope},
    render::{context_json, reasons_json, tool_observation_json, write_ticket_backing_json},
    stable_id,
    tool_observation::{tool_observation, ToolObservation},
    write_ticket::{normalized_observed_paths, write_ticket_coverage, WriteTicketCoverage},
    GuardCommandError,
};

#[derive(Debug)]
struct ObservationPreparation {
    observer_contract_digest: String,
    checkpoint: Option<RepositoryObservationCheckpoint>,
    unavailable_reason: Option<RepositoryObservationUnavailableReason>,
}

#[derive(Debug, Clone)]
struct ExpectedWriteCandidate {
    insert: RepositoryExpectedWriteInsert,
    expected_paths: Vec<String>,
    write_ticket: ActiveWriteTicketSummary,
}

pub(in crate::guard_command) fn handle_pre_tool(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardPhaseResult, GuardCommandError> {
    let summary = guard_state_summary(context, project, envelope, input)?;
    let server = envelope.mcp_server.as_ref().ok_or_else(|| {
        GuardCommandError::Runtime("Guard event has no typed MCP server binding".to_owned())
    })?;
    let tool = tool_observation(&input.raw_value, &project.repo_root, server)?;
    let observation = prepare_repository_observation(project, &tool);
    let (decision, reasons) =
        pre_tool_decision(&summary, &tool, observation.unavailable_reason.is_some());
    let observation_id = repository_observation_id(
        &project.project_id,
        &envelope.connection_id,
        envelope.session_id.as_deref().ok_or_else(|| {
            GuardCommandError::Runtime("Guard event has no session identity".to_owned())
        })?,
        &envelope.correlation,
    )?;
    let expected_write =
        expected_write_candidate(&observation_id, &summary, &tool, envelope, input, decision)?;
    let unavailable_reason = match (
        observation.unavailable_reason,
        decision == GuardPolicyDecision::Deny,
    ) {
        (Some(reason), _) => Some(reason),
        (None, true) => Some(RepositoryObservationUnavailableReason::InvocationDenied),
        (None, false) => None,
    };
    let expected_write_json = expected_write
        .as_ref()
        .map(expected_write_candidate_json)
        .unwrap_or(Value::Null);
    let repository_observation_json = if let Some(reason) = unavailable_reason {
        json!({
            "observation_state": "unavailable",
            "repository_observation_id": observation_id,
            "unavailable_reason": reason.as_str(),
        })
    } else {
        json!({
            "observation_state": "open",
            "repository_observation_id": observation_id,
            "observer_contract_digest": observation.observer_contract_digest,
        })
    };
    let write_ticket_backing =
        if tool.prospective_effect == ProductRepositoryEffect::MayWriteProduct {
            write_ticket_backing_json(write_ticket_coverage(&summary, &tool))
        } else {
            write_ticket_backing_json(WriteTicketCoverage::NotWriteLike)
        };
    Ok(GuardPhaseResult::with_repository_observation(
        decision,
        json!({
            "decision": decision.as_str(),
            "allowed": decision != GuardPolicyDecision::Deny,
            "reasons": reasons_json(&reasons),
            "tool": tool_observation_json(&tool),
            "write_ticket_backing": write_ticket_backing,
            "expected_write": expected_write_json,
            "repository_observation": repository_observation_json,
            "context": context_json(&summary),
            "enforcement_level": "cooperative_guard"
        }),
        RepositoryObservationMutation::Pre {
            repository_observation_id: observation_id,
            observer_contract_digest: observation.observer_contract_digest,
            checkpoint: observation.checkpoint.map(Box::new),
            unavailable_reason,
            expected_write: expected_write.map(|candidate| candidate.insert),
            metadata: json!({
                "source": "volicord_guard_pre_tool",
                "raw_event_sha256": input.raw_sha256,
                "effect": tool.prospective_effect.as_str(),
            })
            .as_object()
            .expect("repository-observation metadata is an object")
            .clone(),
        },
    ))
}

fn pre_tool_decision(
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    repository_observation_unavailable: bool,
) -> (GuardPolicyDecision, Vec<GuardReason>) {
    let mut reasons = Vec::new();
    let may_write = observation.prospective_effect == ProductRepositoryEffect::MayWriteProduct;
    if may_write
        && observation
            .structured_paths
            .iter()
            .any(|path| !path.inside_repo)
    {
        reasons.push(GuardReason {
            code: "target_outside_project_allowlist",
            message: "One or more target paths are outside the selected Product Repository."
                .to_owned(),
            severity: "deny",
        });
    }
    if may_write {
        if summary.active_task_id.is_none() {
            reasons.push(GuardReason {
                code: "no_active_task",
                message: "Product-file writes require an active Volicord task.".to_owned(),
                severity: "deny",
            });
        } else {
            match write_ticket_coverage(summary, observation) {
                WriteTicketCoverage::PolicyAuthorityStale { .. } => reasons.push(GuardReason {
                    code: "write_ticket_policy_changed",
                    message: "Project workflow policy changed after the covering write ticket was issued. Run `volicord.prepare_write` again before this Product Repository write.".to_owned(),
                    severity: "deny",
                }),
                _ if summary.policy_control_reevaluation.is_some() => reasons.push(GuardReason {
                    code: "policy_control_reevaluation_required",
                    message: "Project policy requires the active Task control level or acceptance policy to be reevaluated before this Product Repository write.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::NotWriteLike => {}
                WriteTicketCoverage::TicketBacked { ticket, .. } => {
                    if ticket.workspace_validity_uncertain {
                        reasons.push(GuardReason {
                            code: "write_ticket_workspace_unverified",
                            message: "The covering write ticket is current, but its Git workspace coordinate could not be refreshed.".to_owned(),
                            severity: "warn",
                        });
                    }
                }
                WriteTicketCoverage::NoObservedPaths => reasons.push(GuardReason {
                    code: "write_ticket_scope_indeterminate",
                    message: "The exact Product Repository target path set is unavailable."
                        .to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::NoActiveTickets { .. } => reasons.push(GuardReason {
                    code: "write_ticket_missing",
                    message: "No active write ticket covers this Product Repository write."
                        .to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::OutOfScope { .. } => reasons.push(GuardReason {
                    code: "write_ticket_path_scope_violation",
                    message: "The exact Product Repository path set is outside the active write ticket scope.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::Ambiguous { .. } => reasons.push(GuardReason {
                    code: "write_ticket_ambiguous",
                    message: "More than one active write ticket covers the exact path set."
                        .to_owned(),
                    severity: "warn",
                }),
            }
        }
    }
    if observation.prospective_effect == ProductRepositoryEffect::UnknownProductEffect {
        reasons.push(GuardReason {
            code: "unknown_effect_warning",
            message: "The invocation has an unknown Product Repository effect and will be checked with an exact repository observation.".to_owned(),
            severity: "warn",
        });
    }
    if repository_observation_unavailable {
        let deny = observation.prospective_effect != ProductRepositoryEffect::NoProductWrite;
        reasons.push(GuardReason {
            code: "repository_observation_unavailable",
            message: if deny {
                "A stable pre-tool Product Repository snapshot is unavailable, so this write-capable or unknown-effect invocation is denied.".to_owned()
            } else {
                "A stable pre-tool Product Repository snapshot is unavailable; the read-only-declared invocation may continue with an explicit unavailable observation.".to_owned()
            },
            severity: if deny { "deny" } else { "warn" },
        });
    }
    let decision = if reasons.iter().any(|reason| reason.severity == "deny") {
        GuardPolicyDecision::Deny
    } else if reasons.iter().any(|reason| reason.severity == "warn") {
        GuardPolicyDecision::ContinueWithWarning
    } else {
        GuardPolicyDecision::Continue
    };
    (decision, reasons)
}

fn prepare_repository_observation(
    project: &ProjectRecord,
    observation: &ToolObservation,
) -> ObservationPreparation {
    let limits = ObserverLimits::default();
    let observer_contract_digest = SemanticObserverContractDigest::for_limits(&limits)
        .as_str()
        .to_owned();
    let invocation_paths = observation
        .structured_paths
        .iter()
        .filter(|path| path.inside_repo)
        .filter_map(|path| path.normalized.as_deref())
        .map(|path| ProductRelativePath::parse(path.to_owned()))
        .collect::<Result<BTreeSet<_>, _>>();
    let invocation_paths = match invocation_paths {
        Ok(paths) => paths.into_iter().collect(),
        Err(_) => {
            return ObservationPreparation {
                observer_contract_digest,
                checkpoint: None,
                unavailable_reason: Some(ObservationUnavailableReason::InvalidRelativePath.into()),
            }
        }
    };
    let observer = match RepositoryObserver::new(&project.repo_root, limits) {
        Ok(observer) => observer,
        Err(error) => {
            return ObservationPreparation {
                observer_contract_digest,
                checkpoint: None,
                unavailable_reason: Some(error.reason().into()),
            }
        }
    };
    match observer.snapshot(&InvocationObservationPaths::new(
        invocation_paths,
        Vec::new(),
    )) {
        Ok(snapshot) => ObservationPreparation {
            observer_contract_digest,
            checkpoint: Some(snapshot.checkpoint()),
            unavailable_reason: None,
        },
        Err(error) => ObservationPreparation {
            observer_contract_digest,
            checkpoint: None,
            unavailable_reason: Some(error.reason().into()),
        },
    }
}

fn expected_write_candidate(
    repository_observation_id: &str,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    envelope: &GuardEnvelope,
    input: &GuardInput,
    decision: GuardPolicyDecision,
) -> Result<Option<ExpectedWriteCandidate>, GuardCommandError> {
    if decision == GuardPolicyDecision::Deny
        || observation.prospective_effect != ProductRepositoryEffect::MayWriteProduct
        || observation
            .structured_paths
            .iter()
            .any(|path| !path.inside_repo)
    {
        return Ok(None);
    }
    let expected_paths = normalized_observed_paths(observation.structured_paths.iter());
    if expected_paths.is_empty() {
        return Ok(None);
    }
    let typed_expected_paths = expected_paths
        .iter()
        .map(|path| {
            ProductRelativePath::parse(path)
                .map_err(|error| GuardCommandError::Runtime(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let write_ticket = match write_ticket_coverage(summary, observation) {
        WriteTicketCoverage::TicketBacked { ticket, .. } => ticket,
        _ => return Ok(None),
    };
    let task_id = summary.active_task_id.clone().ok_or_else(|| {
        GuardCommandError::Runtime("expected write has no active task".to_owned())
    })?;
    let created_at = event_time(&envelope.occurred_at)?;
    let expected_write_id = stable_id(
        "expected_write",
        &[
            repository_observation_id,
            &expected_paths.join("|"),
            &write_ticket.write_ticket_id,
        ],
    );
    let write_ticket_ids = vec![write_ticket.write_ticket_id.clone()];
    Ok(Some(ExpectedWriteCandidate {
        insert: RepositoryExpectedWriteInsert {
            expected_write_id,
            command_kind: observation.prospective_effect.as_str().to_owned(),
            expected_paths: typed_expected_paths,
            task_id,
            change_unit_id: write_ticket.change_unit_id.clone(),
            write_ticket_ids: write_ticket_ids.clone(),
            basis_state_version: summary.state_version,
            created_at: UtcTimestamp::from(created_at),
            metadata: json!({
                "source": "volicord_guard_pre_tool",
                "raw_event_sha256": input.raw_sha256,
                "ticket_backed": true,
                "write_ticket_ids": write_ticket_ids
            })
            .as_object()
            .expect("expected-write metadata is an object")
            .clone(),
        },
        expected_paths,
        write_ticket,
    }))
}

fn expected_write_candidate_json(candidate: &ExpectedWriteCandidate) -> Value {
    json!({
        "expected_write_id": candidate.insert.expected_write_id,
        "command_kind": candidate.insert.command_kind,
        "path_policy": "exact_paths",
        "expected_paths": candidate.expected_paths,
        "task_id": candidate.insert.task_id,
        "change_unit_id": candidate.insert.change_unit_id,
        "ticket_backed": true,
        "write_ticket_id": candidate.write_ticket.write_ticket_id,
        "write_ticket_ids": candidate.insert.write_ticket_ids,
        "basis_state_version": candidate.insert.basis_state_version
    })
}
