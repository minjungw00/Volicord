use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, Availability, NonUserQuestionOutcome, OperationId, Principal,
    PrincipalKind, QuestionAlternative, QuestionResearchState, SourceDraft, SourcePayload, Store,
};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDisposition,
    CandidateDraft, CandidateFreshness, CandidateKind, CandidateObservationBasis, CandidateOrigin,
    CandidateRetention, DuplicateAssessment, MaterialityAssessment, MaterialityStatus,
    QuestionCandidate, SubmissionOutcome,
};
use volicord_operations::{LocalOperations, RuntimeLayout};

#[test]
fn local_operations_orchestrate_candidate_lifecycle_without_owning_domain_semantics(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let operations = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = operations
        .initialize_project("Candidate operations", None)?
        .project;
    let mut canonical = Store::open(operations.layout().canonical_store())?;
    let source = canonical
        .record_source(
            OperationId::from_bytes([71; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "candidate-operations-fixture".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "fixture".into(),
                },
                observer: Some(agent()),
                availability: Availability::Available,
            },
        )?
        .value;
    drop(canonical);

    let submitted = operations.submit_candidate(question_candidate(project.id, source.id, 1))?;
    let candidate = match submitted {
        SubmissionOutcome::Stored(candidate) => candidate,
        SubmissionOutcome::CollectionDisabled { .. } => {
            return Err("Candidate collection unexpectedly disabled".into())
        }
    };
    assert_eq!(
        candidate.disposition,
        CandidateDisposition::PendingOrRetained
    );
    assert!(operations
        .canonical_basis(project.id)?
        .active_questions
        .is_empty());

    let read_only_basis = operations.candidate_basis(project.id)?;
    assert_eq!(read_only_basis.candidates[0].revision, candidate.revision);
    assert!(operations
        .canonical_basis(project.id)?
        .active_questions
        .is_empty());

    let promoted = operations.promote_question_candidate(project.id, candidate.id)?;
    assert_eq!(
        operations
            .inquiry_frontier(project.id, Vec::new())?
            .questions[0]
            .question_id,
        promoted.question_id
    );
    assert!(operations
        .canonical_basis(project.id)?
        .active_decisions
        .is_empty());

    let second = match operations.submit_candidate(question_candidate(project.id, source.id, 2))? {
        SubmissionOutcome::Stored(candidate) => candidate,
        SubmissionOutcome::CollectionDisabled { .. } => {
            return Err("Candidate collection unexpectedly disabled".into())
        }
    };
    let dismissed = operations.dismiss_candidate(project.id, second.id, "duplicate concern")?;
    assert!(matches!(
        dismissed.disposition,
        CandidateDisposition::Dismissed { ref reason, .. } if reason == "duplicate concern"
    ));
    assert_eq!(
        operations
            .canonical_basis(project.id)?
            .active_questions
            .len(),
        1
    );
    Ok(())
}

fn agent() -> Principal {
    Principal {
        kind: PrincipalKind::Agent,
        identity: "codex".into(),
    }
}

fn question_candidate(
    project_id: volicord_context::ProjectId,
    source_id: volicord_context::SourceId,
    order: u64,
) -> CandidateDraft {
    CandidateDraft {
        project_id,
        kind: CandidateKind::QuestionCandidate,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: agent(),
            subsystem: "inquiry".into(),
            session: Some("operations-test".into()),
            provenance_summary: "explicit Question Candidate submission".into(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some("operations-test".into()),
            source_operation: Some("design-review".into()),
            candidate_kind: CandidateKind::QuestionCandidate,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source_id],
            repository_snapshot: Some("candidate-operations-fixture".into()),
            ..CandidateObservationBasis::default()
        },
        observed_at: volicord_context::TimestampMicros::from_unix_micros(1),
        retention: CandidateRetention {
            retained_until: None,
            basis: "retain through inquiry disposition".into(),
        },
        content: CandidateContent {
            bounded_summary: "material storage choice".into(),
            question: Some(QuestionCandidate {
                prompt_basis: format!("Choose storage boundary {order}"),
                known_facts: Vec::new(),
                assumptions: vec!["local-first".into()],
                uncertainty: Vec::new(),
                affected_scope: vec![format!("storage-{order}")],
                possible_prerequisites: Vec::new(),
                source_basis: vec![source_id],
                repository_basis: Vec::new(),
                freshness: CandidateFreshness::Current,
                duplicate_assessment: DuplicateAssessment::NoDuplicate {
                    basis: "no matching canonical Question".into(),
                },
                materiality: MaterialityAssessment {
                    status: MaterialityStatus::Material,
                    rationale: Some("the implementation changes".into()),
                    source_basis: vec![source_id],
                    assessed_by: Some(agent()),
                    assessed_at: Some(volicord_context::TimestampMicros::from_unix_micros(1)),
                },
                presentation_order: Some(order),
                why_it_matters_now: "implementation is blocked".into(),
                alternatives: vec![QuestionAlternative {
                    key: "local".into(),
                    label: "Local".into(),
                    consequence: "keep state local".into(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("local".into()),
                    rationale: "matches local-first".into(),
                    source_basis: vec![source_id],
                },
                trade_offs: Vec::new(),
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["implementation".into()],
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state: QuestionResearchState::ReadyToAsk,
            }),
        },
    }
}
