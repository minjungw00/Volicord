//! Host-neutral Guard observation and policy outcomes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::DiagnosticFactSource;

/// Maximum number of bounded diagnostics carried by one hook outcome.
pub const MAX_GUARD_HOOK_DIAGNOSTICS: usize = 8;

/// The durable result of attempting to record one Guard observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardObservationOutcome {
    CompatibleRecorded,
    IncompatibleRecorded,
    PersistenceUnavailable,
}

impl GuardObservationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CompatibleRecorded => "compatible_recorded",
            Self::IncompatibleRecorded => "incompatible_recorded",
            Self::PersistenceUnavailable => "persistence_unavailable",
        }
    }
}

/// A Guard policy result, independent of host output and process exit behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardPolicyDecision {
    Continue,
    ContinueWithContext,
    ContinueWithWarning,
    Deny,
}

impl GuardPolicyDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::ContinueWithContext => "continue_with_context",
            Self::ContinueWithWarning => "continue_with_warning",
            Self::Deny => "deny",
        }
    }
}

/// Stable, bounded Guard hook diagnostic identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GuardHookDiagnosticCode {
    #[serde(rename = "guard.observation.incompatible")]
    HostContractIncompatible,
    #[serde(rename = "guard.event.persistence_unavailable")]
    EventPersistenceUnavailable,
    #[serde(rename = "guard.policy.denied")]
    PolicyDenied,
    #[serde(rename = "guard.host_output.projection_failure")]
    HostOutputProjectionFailure,
    #[serde(rename = "guard.internal.unexpected_failure")]
    UnexpectedInternalFailure,
}

impl GuardHookDiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostContractIncompatible => "guard.observation.incompatible",
            Self::EventPersistenceUnavailable => "guard.event.persistence_unavailable",
            Self::PolicyDenied => "guard.policy.denied",
            Self::HostOutputProjectionFailure => "guard.host_output.projection_failure",
            Self::UnexpectedInternalFailure => "guard.internal.unexpected_failure",
        }
    }
}

/// Bounded, safe coordinates for one Guard hook diagnostic.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GuardHookDiagnosticFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_event_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_installation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_event_id: Option<String>,
}

impl DiagnosticFactSource for GuardHookDiagnosticFacts {}

/// One typed diagnostic attached to a Guard hook outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GuardHookDiagnostic {
    pub code: GuardHookDiagnosticCode,
    pub facts: GuardHookDiagnosticFacts,
}

/// Host-neutral feedback that an adapter may safely project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardHostFeedback {
    Context,
    Warning,
}

/// Complete host-neutral result of one Guard hook invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GuardHookOutcome {
    pub observation: GuardObservationOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<GuardPolicyDecision>,
    pub diagnostics: Vec<GuardHookDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_host_feedback: Option<GuardHostFeedback>,
}

impl GuardHookOutcome {
    pub fn new(
        observation: GuardObservationOutcome,
        policy: Option<GuardPolicyDecision>,
        diagnostics: impl IntoIterator<Item = GuardHookDiagnostic>,
        safe_host_feedback: Option<GuardHostFeedback>,
    ) -> Self {
        Self {
            observation,
            policy,
            diagnostics: diagnostics
                .into_iter()
                .take(MAX_GUARD_HOOK_DIAGNOSTICS)
                .collect(),
            safe_host_feedback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_outcome_keeps_observation_and_policy_independent_and_bounded() {
        let diagnostic = GuardHookDiagnostic {
            code: GuardHookDiagnosticCode::HostContractIncompatible,
            facts: GuardHookDiagnosticFacts::default(),
        };
        let outcome = GuardHookOutcome::new(
            GuardObservationOutcome::IncompatibleRecorded,
            None,
            std::iter::repeat_n(diagnostic, MAX_GUARD_HOOK_DIAGNOSTICS + 2),
            Some(GuardHostFeedback::Warning),
        );
        assert_eq!(outcome.policy, None);
        assert_eq!(outcome.diagnostics.len(), MAX_GUARD_HOOK_DIAGNOSTICS);
        assert_eq!(
            serde_json::to_value(&outcome).unwrap()["diagnostics"][0]["code"],
            GuardHookDiagnosticCode::HostContractIncompatible.as_str()
        );
    }
}
