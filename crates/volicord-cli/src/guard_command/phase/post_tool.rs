use serde_json::json;
use volicord_platform_fs::{
    InvocationObservationPaths, ObserverLimits, RepositoryObserver, SemanticObserverContractDigest,
};
use volicord_store::{
    bootstrap::ProjectRecord,
    guards::{
        repository_observation_for_invocation, repository_observation_id,
        PostToolRepositoryObservationOutcome, RepositoryObservationState,
        RepositoryObservationUnavailableReason,
    },
    RuntimeHomeMutationContext,
};
use volicord_types::guard_outcome::GuardPolicyDecision;

use super::{GuardPhaseResult, RepositoryObservationMutation};
use crate::guard_command::{
    args::GuardInput,
    context::guard_state_summary,
    envelope::GuardEnvelope,
    render::{context_json, reasons_json, tool_observation_json},
    tool_observation::tool_observation,
    GuardCommandError,
};

pub(in crate::guard_command) fn handle_post_tool(
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
    let observation_id = repository_observation_id(
        &project.project_id,
        &envelope.connection_id,
        envelope.session_id.as_deref().ok_or_else(|| {
            GuardCommandError::Runtime("Guard event has no session identity".to_owned())
        })?,
        &envelope.correlation,
    )?;
    let observer_contract_digest =
        SemanticObserverContractDigest::for_limits(&ObserverLimits::default())
            .as_str()
            .to_owned();
    let stored = repository_observation_for_invocation(
        context.runtime_home().as_path(),
        &project.project_id,
        &envelope.connection_id,
        envelope.session_id.as_deref().ok_or_else(|| {
            GuardCommandError::Runtime("Guard event has no session identity".to_owned())
        })?,
        &envelope.correlation,
    )?;
    let outcome = match stored {
        Some(record) if record.state == RepositoryObservationState::Open => {
            capture_post_outcome(project, record, &observer_contract_digest)
        }
        Some(record) if record.state == RepositoryObservationState::Unavailable => {
            PostToolRepositoryObservationOutcome::Unavailable {
                reason: record.unavailable_reason.ok_or_else(|| {
                    GuardCommandError::Runtime(
                        "unavailable repository observation has no closed reason".to_owned(),
                    )
                })?,
            }
        }
        Some(_) => {
            return Err(GuardCommandError::Runtime(
                "repository observation is already complete".to_owned(),
            ))
        }
        None => PostToolRepositoryObservationOutcome::Unavailable {
            reason: RepositoryObservationUnavailableReason::MissingOpenObservation,
        },
    };
    let unavailable_reason = match &outcome {
        PostToolRepositoryObservationOutcome::Complete { .. } => None,
        PostToolRepositoryObservationOutcome::Unavailable { reason } => Some(*reason),
    };
    let reasons = unavailable_reason
        .map(|reason| {
            vec![crate::guard_command::context::GuardReason {
                code: "repository_observation_unavailable",
                message: format!(
                    "The exact invocation repository observation is unavailable: {}.",
                    reason.as_str()
                ),
                severity: "warn",
            }]
        })
        .unwrap_or_default();
    let decision = if unavailable_reason.is_some() {
        GuardPolicyDecision::ContinueWithWarning
    } else {
        GuardPolicyDecision::Continue
    };
    Ok(GuardPhaseResult::with_repository_observation(
        decision,
        json!({
            "decision": decision.as_str(),
            "allowed": true,
            "reasons": reasons_json(&reasons),
            "tool": tool_observation_json(&tool),
            "repository_observation": {
                "observation_state": if unavailable_reason.is_some() {
                    "unavailable"
                } else {
                    "complete"
                },
                "repository_observation_id": observation_id,
                "unavailable_reason": unavailable_reason.map(|reason| reason.as_str()),
            },
            "context": context_json(&summary),
            "enforcement_level": "cooperative_guard"
        }),
        RepositoryObservationMutation::Post {
            repository_observation_id: observation_id,
            observer_contract_digest,
            outcome,
            task_id: summary.active_task_id.clone(),
            metadata: json!({
                "source": "volicord_guard_post_tool",
                "raw_event_sha256": input.raw_sha256,
                "effect": tool.prospective_effect.as_str(),
            })
            .as_object()
            .expect("repository-observation metadata is an object")
            .clone(),
        },
    ))
}

fn capture_post_outcome(
    project: &ProjectRecord,
    record: volicord_store::guards::RepositoryObservationRecord,
    observer_contract_digest: &str,
) -> PostToolRepositoryObservationOutcome {
    let Some(checkpoint) = record.pre_snapshot else {
        return PostToolRepositoryObservationOutcome::Unavailable {
            reason: RepositoryObservationUnavailableReason::MissingOpenObservation,
        };
    };
    let observer = match RepositoryObserver::new(&project.repo_root, ObserverLimits::default()) {
        Ok(observer) if observer.contract_digest().as_str() == observer_contract_digest => observer,
        Ok(_) => {
            return PostToolRepositoryObservationOutcome::Unavailable {
                reason:
                    volicord_platform_fs::ObservationUnavailableReason::ObserverContractMismatch
                        .into(),
            }
        }
        Err(error) => {
            return PostToolRepositoryObservationOutcome::Unavailable {
                reason: error.reason().into(),
            }
        }
    };
    let invocation_paths = checkpoint
        .invocation_paths()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let before = match observer.restore_checkpoint(checkpoint) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return PostToolRepositoryObservationOutcome::Unavailable {
                reason: error.reason().into(),
            }
        }
    };
    let after = match observer.snapshot(&InvocationObservationPaths::new(
        invocation_paths,
        Vec::new(),
    )) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return PostToolRepositoryObservationOutcome::Unavailable {
                reason: error.reason().into(),
            }
        }
    };
    match observer.delta(&before, &after) {
        Ok(delta) => PostToolRepositoryObservationOutcome::Complete {
            post_snapshot: Box::new(after.checkpoint()),
            delta,
        },
        Err(error) => PostToolRepositoryObservationOutcome::Unavailable {
            reason: error.reason().into(),
        },
    }
}
