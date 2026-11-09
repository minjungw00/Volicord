use crate::{Error, LocalOperations};
use serde_json::{json, Value};
use volicord_context::ProjectId;
use volicord_inquiry::{CandidateKind, CandidateStore, CollectionOptOut, CollectionOptOutScope};

pub const COLLECTION_KINDS: &[&str] = &[
    "observation",
    "hypothesis",
    "semantic_claim",
    "question_candidate",
    "checkpoint_candidate",
    "promotion_proposal",
    "engineering_choice_discovery",
    "materiality_review",
    "learning_deliberation",
];

pub fn parse_collection_kind(value: &str) -> Result<CandidateKind, Error> {
    Ok(match value {
        "observation" => CandidateKind::Observation,
        "hypothesis" => CandidateKind::Hypothesis,
        "semantic_claim" => CandidateKind::SemanticClaim,
        "question_candidate" => CandidateKind::QuestionCandidate,
        "checkpoint_candidate" => CandidateKind::CheckpointCandidate,
        "promotion_proposal" => CandidateKind::PromotionProposal,
        "engineering_choice_discovery" => CandidateKind::EngineeringChoiceDiscovery,
        "materiality_review" => CandidateKind::MaterialityReview,
        "learning_deliberation" => CandidateKind::LearningDeliberation,
        _ => return Err(Error::new("unsupported Candidate collection kind")),
    })
}

pub fn collection_policy_json(policy: CollectionOptOut) -> Value {
    let kind = policy.scope.candidate_kind.and_then(|kind| {
        COLLECTION_KINDS
            .iter()
            .find(|name| parse_collection_kind(name).ok() == Some(kind))
            .copied()
    });
    json!({"project_id":policy.scope.project_id.to_string(),
        "session":policy.scope.session,"source_operation":policy.scope.source_operation,
        "candidate_kind":kind,"opted_out":policy.opted_out,
        "effective_at_unix_micros":policy.effective_at.as_unix_micros(),"basis":policy.basis})
}

impl LocalOperations {
    pub fn collection_policies(&self, project: ProjectId) -> Result<Vec<CollectionOptOut>, Error> {
        self.canonical_basis(project)?;
        CandidateStore::open(self.layout().candidate_store())
            .and_then(|store| store.collection_policies(project))
            .map_err(|error| {
                Error::with_source("Candidate collection policy inspection failed", error)
            })
    }

    pub fn set_collection_opt_out(
        &self,
        scope: CollectionOptOutScope,
        opted_out: bool,
        basis: impl Into<String>,
    ) -> Result<CollectionOptOut, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        self.canonical_basis(scope.project_id)?;
        CandidateStore::open(self.layout().candidate_store())
            .and_then(|mut store| store.set_collection_opt_out(scope, opted_out, basis))
            .map_err(|error| Error::with_source("Candidate collection policy change failed", error))
    }
}
