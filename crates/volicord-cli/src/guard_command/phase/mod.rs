use serde_json::Value;
use volicord_platform_fs::RepositoryObservationCheckpoint;
use volicord_store::guards::{
    PostToolRepositoryObservationOutcome, RepositoryExpectedWriteInsert,
    RepositoryObservationUnavailableReason,
};
use volicord_types::guard_outcome::GuardPolicyDecision;
use volicord_types::schema::JsonObject;

pub(super) mod post_tool;
pub(super) mod pre_tool;

#[derive(Debug, Clone)]
pub(super) enum RepositoryObservationMutation {
    Pre {
        repository_observation_id: String,
        observer_contract_digest: String,
        checkpoint: Option<Box<RepositoryObservationCheckpoint>>,
        unavailable_reason: Option<RepositoryObservationUnavailableReason>,
        expected_write: Option<RepositoryExpectedWriteInsert>,
        metadata: JsonObject,
    },
    Post {
        repository_observation_id: String,
        observer_contract_digest: String,
        outcome: PostToolRepositoryObservationOutcome,
        task_id: Option<String>,
        metadata: JsonObject,
    },
}

#[derive(Debug, Clone)]
pub(super) struct GuardPhaseResult {
    pub(super) decision: GuardPolicyDecision,
    pub(super) result: Value,
    pub(super) repository_observation: Option<RepositoryObservationMutation>,
}

impl GuardPhaseResult {
    pub(super) fn new(decision: GuardPolicyDecision, result: Value) -> Self {
        Self {
            decision,
            result,
            repository_observation: None,
        }
    }

    pub(super) fn with_repository_observation(
        decision: GuardPolicyDecision,
        result: Value,
        repository_observation: RepositoryObservationMutation,
    ) -> Self {
        Self {
            decision,
            result,
            repository_observation: Some(repository_observation),
        }
    }
}
