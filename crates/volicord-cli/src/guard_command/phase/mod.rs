use serde_json::Value;
use volicord_types::guard_outcome::GuardPolicyDecision;

use self::pre_tool::ExpectedWriteCandidate;

pub(super) mod post_tool;
pub(super) mod pre_tool;

#[derive(Debug, Clone)]
pub(super) struct GuardPhaseResult {
    pub(super) decision: GuardPolicyDecision,
    pub(super) result: Value,
    pub(super) expected_write: Option<ExpectedWriteCandidate>,
}

impl GuardPhaseResult {
    pub(super) fn new(decision: GuardPolicyDecision, result: Value) -> Self {
        Self {
            decision,
            result,
            expected_write: None,
        }
    }

    pub(super) fn with_expected_write(
        decision: GuardPolicyDecision,
        result: Value,
        expected_write: Option<ExpectedWriteCandidate>,
    ) -> Self {
        Self {
            decision,
            result,
            expected_write,
        }
    }
}
