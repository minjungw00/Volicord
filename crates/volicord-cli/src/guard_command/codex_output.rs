use serde_json::{json, Value};
use volicord_types::guard_outcome::{
    GuardHookOutcome, GuardObservationOutcome, GuardPolicyDecision,
};
use volicord_types::values::{GuardHookPhase, HostKind};

use crate::{
    disclosure::COOPERATIVE_DECISION_DISCLOSURE_TEXT,
    host_integration::contracts::{contract_for, hook_event_for_phase},
};

use super::{json_error, render::RenderedGuardOutput, GuardCommandError};

/// Codex-specific projection of a host-neutral Guard outcome.
pub(super) fn render_codex_output(
    phase: GuardHookPhase,
    outcome: &GuardHookOutcome,
    result: &Value,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    let event_name = contract_for(HostKind::Codex)
        .and_then(|contract| hook_event_for_phase(contract, phase))
        .map(|event| event.event_name)
        .ok_or_else(|| {
            GuardCommandError::Runtime(
                "Codex host output projection is unavailable for this Guard phase".to_owned(),
            )
        })?;

    let value = if phase == GuardHookPhase::PreTool
        && outcome.policy == Some(GuardPolicyDecision::Deny)
    {
        Some(json!({
            "hookSpecificOutput": {
                "hookEventName": event_name,
                "permissionDecision": "deny",
                "permissionDecisionReason": native_message(
                    &format!(
                        "[{}] {}",
                        volicord_types::guard_outcome::GuardHookDiagnosticCode::PolicyDenied.as_str(),
                        first_reason_message(result).unwrap_or_else(||
                            "Volicord policy denied this write attempt".to_owned()
                        )
                    )
                )
            }
        }))
    } else {
        codex_context(phase, outcome, result).map(|message| {
            json!({
                "hookSpecificOutput": {
                    "hookEventName": event_name,
                    "additionalContext": native_message(&message)
                }
            })
        })
    };

    Ok(RenderedGuardOutput {
        stdout: value
            .map(|value| serde_json::to_string(&value).map(|text| format!("{text}\n")))
            .transpose()
            .map_err(json_error)?
            .unwrap_or_default(),
        stderr: String::new(),
        exit_code: 0,
    })
}

/// Minimal infallible Codex projection used when the richer projection fails.
pub(super) fn render_codex_projection_failure(
    phase: GuardHookPhase,
    policy: Option<GuardPolicyDecision>,
) -> RenderedGuardOutput {
    let event_name = match phase {
        GuardHookPhase::PromptCapture => "UserPromptSubmit",
        GuardHookPhase::PreTool => "PreToolUse",
        GuardHookPhase::PostTool => "PostToolUse",
    };
    let value = if phase == GuardHookPhase::PreTool && policy == Some(GuardPolicyDecision::Deny) {
        json!({
            "hookSpecificOutput": {
                "hookEventName": event_name,
                "permissionDecision": "deny",
                "permissionDecisionReason": native_message("[guard.host_output.projection_failure] Volicord policy denied this write attempt; detailed host output projection was unavailable")
            }
        })
    } else {
        json!({
            "hookSpecificOutput": {
                "hookEventName": event_name,
                "additionalContext": native_message("[guard.host_output.projection_failure] Volicord could not project detailed Guard host output; the action continues and the typed projection-failure finding should be inspected")
            }
        })
    };
    RenderedGuardOutput {
        stdout: format!(
            "{}\n",
            serde_json::to_string(&value).expect("the bounded static Codex fallback is valid JSON")
        ),
        stderr: String::new(),
        exit_code: 0,
    }
}

fn codex_context(
    phase: GuardHookPhase,
    outcome: &GuardHookOutcome,
    result: &Value,
) -> Option<String> {
    if outcome.diagnostics.first().is_some_and(|diagnostic| {
        diagnostic.code
            == volicord_types::guard_outcome::GuardHookDiagnosticCode::UnexpectedInternalFailure
    }) {
        return Some("[guard.internal.unexpected_failure] Volicord encountered an unexpected Guard failure. The action continues; inspect the typed finding before relying on this observation".to_owned());
    }
    match outcome.observation {
        GuardObservationOutcome::IncompatibleRecorded => Some(match phase {
            GuardHookPhase::PromptCapture => "[guard.observation.incompatible] Volicord recorded an incompatible prompt observation. The prompt continues, but this event does not satisfy prompt-observation readiness",
            GuardHookPhase::PreTool => "[guard.observation.incompatible] Volicord recorded an incompatible pre-tool observation. The tool request continues because no policy decision was produced, and this event does not satisfy pre-tool observation readiness",
            GuardHookPhase::PostTool => "[guard.observation.incompatible] Volicord recorded an incompatible post-tool observation after the tool action. The completed action was not prevented or reversed, and this event does not satisfy post-tool observation readiness",
        }.to_owned()),
        GuardObservationOutcome::PersistenceUnavailable => Some(match phase {
            GuardHookPhase::PromptCapture => "[guard.event.persistence_unavailable] Volicord could not persist the prompt observation. The prompt continues; inspect Volicord diagnostics before relying on Guard readiness",
            GuardHookPhase::PreTool => "[guard.event.persistence_unavailable] Volicord could not persist the required pre-tool repository observation, so the invocation is denied",
            GuardHookPhase::PostTool => "[guard.event.persistence_unavailable] Volicord could not persist the post-tool observation after the tool action. The completed action was not prevented or reversed",
        }.to_owned()),
        GuardObservationOutcome::CompatibleRecorded => match (phase, outcome.policy) {
            (GuardHookPhase::PromptCapture, Some(GuardPolicyDecision::ContinueWithContext)) => {
                result.get("model_context").and_then(Value::as_str).map(str::to_owned)
            }
            (GuardHookPhase::PreTool, Some(GuardPolicyDecision::ContinueWithWarning)) => {
                first_reason_message(result)
            }
            (GuardHookPhase::PostTool, Some(GuardPolicyDecision::ContinueWithWarning)) => Some(
                "Volicord recorded a post-tool Guard warning. The tool action has already completed; inspect the hook result and reconcile unrecorded Product Repository changes before close"
                    .to_owned(),
            ),
            (GuardHookPhase::PostTool, Some(GuardPolicyDecision::Deny)) => Some(
                "Volicord recorded a post-tool policy finding after the tool action already completed. The completed action was not prevented or reversed"
                    .to_owned(),
            ),
            _ => None,
        },
    }
}

fn first_reason_message(result: &Value) -> Option<String> {
    result
        .get("reasons")
        .and_then(Value::as_array)
        .and_then(|reasons| reasons.first())
        .and_then(|reason| reason.get("message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn native_message(message: &str) -> String {
    format!("{message}. {COOPERATIVE_DECISION_DISCLOSURE_TEXT}.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::guard_outcome::{
        GuardHookDiagnostic, GuardHookDiagnosticCode, GuardHookDiagnosticFacts, GuardHostFeedback,
    };

    fn outcome(
        observation: GuardObservationOutcome,
        policy: Option<GuardPolicyDecision>,
    ) -> GuardHookOutcome {
        GuardHookOutcome::new(
            observation,
            policy,
            [GuardHookDiagnostic {
                code: GuardHookDiagnosticCode::HostContractIncompatible,
                facts: GuardHookDiagnosticFacts::default(),
            }],
            Some(GuardHostFeedback::Warning),
        )
    }

    #[test]
    fn incompatible_observations_continue_for_every_codex_phase() {
        for phase in [
            GuardHookPhase::PromptCapture,
            GuardHookPhase::PreTool,
            GuardHookPhase::PostTool,
        ] {
            let output = render_codex_output(
                phase,
                &outcome(GuardObservationOutcome::IncompatibleRecorded, None),
                &json!({}),
            )
            .expect("Codex projection");
            assert_eq!(output.exit_code, 0);
            assert!(output.stderr.is_empty());
            assert!(output.stdout.contains("additionalContext"));
            assert!(!output.stdout.contains("permissionDecision\":\"deny"));
        }
    }

    #[test]
    fn only_pre_tool_policy_deny_projects_codex_deny() {
        let denied = outcome(
            GuardObservationOutcome::CompatibleRecorded,
            Some(GuardPolicyDecision::Deny),
        );
        let pre = render_codex_output(GuardHookPhase::PreTool, &denied, &json!({})).unwrap();
        assert!(pre.stdout.contains("permissionDecision\":\"deny"));
        let post_warning = outcome(
            GuardObservationOutcome::CompatibleRecorded,
            Some(GuardPolicyDecision::ContinueWithWarning),
        );
        let post =
            render_codex_output(GuardHookPhase::PostTool, &post_warning, &json!({})).unwrap();
        assert!(!post.stdout.contains("permissionDecision"));
        assert!(post.stdout.contains("already completed"));
    }
}
