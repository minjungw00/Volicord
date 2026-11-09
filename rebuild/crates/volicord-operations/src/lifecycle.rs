use crate::{Error, LocalOperations};
use serde_json::{json, Value};
use volicord_context::{Clock, ProjectId, SystemClock};
use volicord_inquiry::{CandidateKind, CandidateStore, CollectionOptOut, CollectionOptOutScope};
use volicord_privacy::{ManagedDerivedState, PrivacyStore};

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

/// Per-owner receipt: a failed batch may have committed a prefix, so identities
/// are unknown until inspection/retry rather than falsely reported as empty.
#[derive(Debug, serde::Serialize)]
pub struct ExpiryPartition {
    pub state: &'static str,
    pub removed_ids: Option<Vec<String>>,
    pub remaining_eligible: Option<usize>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ExpiryCleanupOutcome {
    pub project_id: String,
    pub limit_per_store: usize,
    pub candidates: ExpiryPartition,
    pub managed: ExpiryPartition,
    pub local_guarantee: &'static str,
    pub provider_outcome: &'static str,
}

fn expiry_partition<T: ToString>(
    result: Result<Vec<T>, Error>,
    remaining: Result<usize, Error>,
) -> ExpiryPartition {
    let (removed_ids, mut diagnostic) = match result {
        Ok(ids) => (
            Some(ids.into_iter().map(|id| id.to_string()).collect()),
            None,
        ),
        Err(error) => (None, Some(error.to_string())),
    };
    let remaining_eligible = match remaining {
        Ok(count) => Some(count),
        Err(error) => {
            diagnostic = Some(error.to_string());
            None
        }
    };
    ExpiryPartition {
        state: if diagnostic.is_some() {
            "incomplete"
        } else {
            "completed"
        },
        removed_ids,
        remaining_eligible,
        diagnostic,
    }
}

impl LocalOperations {
    /// Bounded mutation, never called by reading/rendering/export. Each owner
    /// retains its receipts before the next record, so restart needs no journal.
    pub fn cleanup_expired(
        &self,
        project: ProjectId,
        limit: usize,
    ) -> Result<ExpiryCleanupOutcome, Error> {
        if !(1..=256).contains(&limit) {
            return Err(Error::new("expiry cleanup limit must be 1..256 per store"));
        }
        let _mutation = self.layout().acquire_mutation_lock()?;
        self.canonical_basis(project)?;
        let candidate_result = CandidateStore::open(self.layout().candidate_store())
            .and_then(|mut store| store.cleanup_expired_bounded(project, limit))
            .map_err(|error| {
                Error::with_source(
                    "Candidate expiry batch incomplete; inspect and retry cleanup",
                    error,
                )
            });
        let managed_result = PrivacyStore::open(self.layout().privacy_store())
            .and_then(|mut store| store.cleanup_expired_bounded(project, limit))
            .map_err(|error| {
                Error::with_source(
                    "managed expiry batch incomplete; inspect and retry cleanup",
                    error,
                )
            });
        let now = SystemClock
            .now()
            .map_err(|error| Error::with_source("cleanup clock unavailable", error))?;
        let candidate_remaining = CandidateStore::open(self.layout().candidate_store())
            .and_then(|store| store.read_basis(project))
            .map(|basis| {
                basis
                    .candidates
                    .iter()
                    .filter(|c| {
                        c.content.is_some()
                            && c.cleanup.is_none()
                            && c.retention.retained_until.is_some_and(|t| t <= now)
                    })
                    .count()
            })
            .map_err(|error| Error::with_source("Candidate cleanup inspection failed", error));
        let managed_remaining = PrivacyStore::open(self.layout().privacy_store())
            .and_then(|store| store.inspect_project(project))
            .map(|basis| {
                basis
                    .managed_derived
                    .iter()
                    .filter(|r| {
                        r.state != ManagedDerivedState::Deleted
                            && r.retained_until.is_some_and(|t| t <= now)
                            && !r.source_computed_explanation_deadline(&basis.managed_derived)
                    })
                    .count()
            })
            .map_err(|error| Error::with_source("managed cleanup inspection failed", error));
        Ok(ExpiryCleanupOutcome {
            project_id: project.to_string(),
            limit_per_store: limit,
            candidates: expiry_partition(candidate_result, candidate_remaining),
            managed: expiry_partition(managed_result, managed_remaining),
            local_guarantee: "logical_content_removal",
            provider_outcome: "not_requested",
        })
    }
}

/// Metadata only: lifecycle inspection never grants retained body access.
pub fn managed_lifecycle_json(record: &volicord_privacy::ManagedDerivedRecord) -> Value {
    json!({"id":record.id.to_string(),"kind":record.kind,"state":record.state,
        "provider":record.provider,"model":record.model,"purpose":record.purpose,
        "analysis_snapshot":record.analysis_snapshot.map(|id|id.to_string()),
        "source_ids":record.included_sources.iter().map(|s|s.identity().to_string()).collect::<Vec<_>>(),
        "created_at_unix_micros":record.created_at.as_unix_micros(),
        "retained_until_unix_micros":record.retained_until.map(|t|t.as_unix_micros()),
        "retention_basis":record.retention_basis,"local_deletion":record.local_deletion,
        "expired_by_cleanup":record.expired_by_cleanup(),"provider_deletion":record.provider_deletion})
}

#[derive(Debug, serde::Serialize)]
pub struct AnnotationDeletionPlan {
    pub project_id: String,
    pub ids: Vec<String>,
    pub fingerprint: String,
    pub request_provider_deletion: bool,
    pub records: Vec<Value>,
}

pub struct AnnotationDeletionRequest {
    pub project_id: ProjectId,
    pub ids: Vec<volicord_privacy::ManagedDerivedId>,
    pub expected_fingerprint: String,
    pub basis: String,
    pub request_provider_deletion: bool,
}

pub fn parse_managed_id(value: &str) -> Result<volicord_privacy::ManagedDerivedId, Error> {
    crate::operations::parse_identity(value).map(volicord_privacy::ManagedDerivedId::from_bytes)
}

impl LocalOperations {
    pub fn annotation_deletion_plan(
        &self,
        project: ProjectId,
        ids: &[volicord_privacy::ManagedDerivedId],
        request_provider_deletion: bool,
    ) -> Result<AnnotationDeletionPlan, Error> {
        use sha2::{Digest, Sha256};
        use std::collections::BTreeSet;
        use volicord_privacy::ManagedDerivedKind;
        if ids.is_empty()
            || ids.len() > 64
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err(Error::new("select 1..64 distinct semantic annotation IDs"));
        }
        self.canonical_basis(project)?;
        let store = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|error| Error::with_source("annotation selection unavailable", error))?;
        let mut ids = ids.to_vec();
        ids.sort();
        let records = ids
            .iter()
            .map(|id| {
                store.get_derived(project, *id).map_err(|_| {
                    Error::new("annotation selection is missing or belongs to another Project")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if records
            .iter()
            .any(|r| r.kind != ManagedDerivedKind::SemanticAnnotation)
        {
            return Err(Error::new(
                "selection must contain only semantic annotations",
            ));
        }
        let records = records
            .iter()
            .map(managed_lifecycle_json)
            .collect::<Vec<_>>();
        let basis = serde_json::to_vec(&(project.to_string(), request_provider_deletion, &records))
            .map_err(|_| Error::new("annotation selection fingerprint failed"))?;
        Ok(AnnotationDeletionPlan {
            project_id: project.to_string(),
            ids: ids.iter().map(ToString::to_string).collect(),
            fingerprint: format!("sha256:{:x}", Sha256::digest(basis)),
            request_provider_deletion,
            records,
        })
    }

    pub fn delete_semantic_annotations(
        &self,
        request: AnnotationDeletionRequest,
    ) -> Result<Value, Error> {
        // The current production adapter's delete method is a local unsupported
        // result. It never probes credentials, spawns Codex or transmits data.
        let mut provider = crate::CodexCliSemanticProvider::new(
            volicord_privacy::ProviderIdentity {
                provider: crate::CODEX_CLI_PROVIDER.into(),
                model: "deletion-capability".into(),
            },
            crate::CodexCliProviderConfig::production(self.layout().artifacts_dir()),
        );
        self.delete_semantic_annotations_with_provider(request, Some(&mut provider))
    }

    /// The caller supplies current exact deletion intent through the request;
    /// adapter injection itself supplies no permission. No generation is invoked.
    pub fn delete_semantic_annotations_with_provider(
        &self,
        request: AnnotationDeletionRequest,
        provider: Option<&mut dyn volicord_privacy::BackgroundSemanticProvider>,
    ) -> Result<Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let plan = self.annotation_deletion_plan(
            request.project_id,
            &request.ids,
            request.request_provider_deletion,
        )?;
        if plan.fingerprint != request.expected_fingerprint {
            return Err(Error::new(
                "annotation selection changed; inspect and prepare the same IDs before retrying",
            ));
        }
        let mut store = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|error| Error::with_source("annotation deletion unavailable", error))?;
        let result = store
            .delete_semantic_annotations(
                request.project_id,
                &request.ids,
                &request.basis,
                request.request_provider_deletion,
                provider,
            )
            .map_err(|error| {
                Error::with_source("annotation deletion failed; inspect before retry", error)
            })?;
        Ok(
            json!({"project_id":request.project_id.to_string(),"ids":result.ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "logical_deletion_committed":result.logical_deletion_committed,
            "local_cleanup_completed":result.local_cleanup_completed,"local_diagnostic":result.local_diagnostic,
            "local_guarantee":"sqlite_content_and_wal_sanitized_when_completed",
            "provider_deletion_requested":result.provider_deletion_requested,
            "provider_step":if !request.request_provider_deletion {"not_requested"} else if !result.local_cleanup_completed {"blocked_by_local_cleanup"} else {"evaluated"},
            "provider_outcomes":result.provider_outcomes.iter().map(|r|json!({"id":r.id.to_string(),"retained_outcome":r.outcome})).collect::<Vec<_>>() }),
        )
    }
}
