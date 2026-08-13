use volicord_context::TimestampMicros;
use volicord_inquiry::{
    CandidateCleanup, CandidateDisposition, CandidateId, CandidateReadBasis, CandidateRecord,
    CollectionOptOut, CollectionOptOutScope,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateContentAccess {
    AllowBoundedSummary,
    PolicyWithheld,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectionHealth {
    Complete,
    Partial,
    Degraded,
    NotFound,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateContentOmission {
    PolicyWithheld,
    RetentionCleaned,
    ContentUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetentionInspection {
    RetainedIndefinitely {
        basis: String,
    },
    RetainedUntil {
        retained_until: TimestampMicros,
        expired_at_observation: bool,
        basis: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateInspection {
    pub candidate_id: CandidateId,
    pub exists: bool,
    pub health: InspectionHealth,
    pub revision: Option<u64>,
    pub kind: Option<volicord_inquiry::CandidateKind>,
    pub origin: Option<volicord_inquiry::CandidateOrigin>,
    pub collection_scope: Option<volicord_inquiry::CandidateCollectionScope>,
    pub observation_basis: Option<volicord_inquiry::CandidateObservationBasis>,
    pub created_at: Option<TimestampMicros>,
    pub observed_at: Option<TimestampMicros>,
    pub retention: Option<RetentionInspection>,
    pub promotion_disposition: Option<CandidateDisposition>,
    pub promotion_target: Option<volicord_context::QuestionId>,
    pub content_cleaned: bool,
    pub cleanup: Option<CandidateCleanup>,
    pub current_applicable_opt_out: Vec<CollectionOptOut>,
    pub bounded_summary: Option<String>,
    pub content_omission: Option<CandidateContentOmission>,
}

/// Reads one named Candidate from an owned immutable basis. No store or
/// lifecycle handle is accepted, so inspection and failure cannot mutate it.
pub fn inspect_candidate(
    basis: &CandidateReadBasis,
    candidate_id: CandidateId,
    content_access: CandidateContentAccess,
    observed_at: TimestampMicros,
) -> CandidateInspection {
    let Some(candidate) = basis
        .candidates
        .iter()
        .find(|candidate| candidate.id == candidate_id)
    else {
        return CandidateInspection {
            candidate_id,
            exists: false,
            health: InspectionHealth::NotFound,
            revision: None,
            kind: None,
            origin: None,
            collection_scope: None,
            observation_basis: None,
            created_at: None,
            observed_at: None,
            retention: None,
            promotion_disposition: None,
            promotion_target: None,
            content_cleaned: false,
            cleanup: None,
            current_applicable_opt_out: Vec::new(),
            bounded_summary: None,
            content_omission: None,
        };
    };
    inspect_existing(basis, candidate, content_access, observed_at)
}

fn inspect_existing(
    basis: &CandidateReadBasis,
    candidate: &CandidateRecord,
    content_access: CandidateContentAccess,
    observed_at: TimestampMicros,
) -> CandidateInspection {
    let current_applicable_opt_out = basis
        .collection_policies
        .iter()
        .filter(|policy| scope_matches(&policy.scope, &candidate.collection_scope))
        .cloned()
        .collect();
    let cleaned = candidate.cleanup.is_some();
    let retention = if let Some(retained_until) = candidate.retention.retained_until {
        RetentionInspection::RetainedUntil {
            retained_until,
            expired_at_observation: retained_until <= observed_at,
            basis: candidate.retention.basis.clone(),
        }
    } else {
        RetentionInspection::RetainedIndefinitely {
            basis: candidate.retention.basis.clone(),
        }
    };
    let (health, bounded_summary, content_omission) = match content_access {
        CandidateContentAccess::PolicyWithheld => (
            InspectionHealth::Partial,
            None,
            Some(CandidateContentOmission::PolicyWithheld),
        ),
        CandidateContentAccess::AllowBoundedSummary => match candidate.content.as_ref() {
            Some(content) => (
                InspectionHealth::Complete,
                Some(content.bounded_summary.clone()),
                None,
            ),
            None if cleaned => (
                InspectionHealth::Partial,
                None,
                Some(CandidateContentOmission::RetentionCleaned),
            ),
            None => (
                InspectionHealth::Degraded,
                None,
                Some(CandidateContentOmission::ContentUnavailable),
            ),
        },
    };
    CandidateInspection {
        candidate_id: candidate.id,
        exists: true,
        health,
        revision: Some(candidate.revision),
        kind: Some(candidate.kind),
        origin: Some(candidate.origin.clone()),
        collection_scope: Some(candidate.collection_scope.clone()),
        observation_basis: Some(candidate.observation_basis.clone()),
        created_at: Some(candidate.created_at),
        observed_at: Some(candidate.observed_at),
        retention: Some(retention),
        promotion_disposition: Some(candidate.disposition.clone()),
        promotion_target: candidate.promotion_target,
        content_cleaned: cleaned,
        cleanup: candidate.cleanup.clone(),
        current_applicable_opt_out,
        bounded_summary,
        content_omission,
    }
}

fn scope_matches(
    policy: &CollectionOptOutScope,
    candidate: &volicord_inquiry::CandidateCollectionScope,
) -> bool {
    policy.project_id == candidate.project_id
        && policy
            .session
            .as_ref()
            .is_none_or(|value| candidate.session.as_ref() == Some(value))
        && policy
            .source_operation
            .as_ref()
            .is_none_or(|value| candidate.source_operation.as_ref() == Some(value))
        && policy
            .candidate_kind
            .is_none_or(|value| candidate.candidate_kind == value)
}
