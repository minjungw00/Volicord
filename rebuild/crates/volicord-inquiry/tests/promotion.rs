use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, Availability, CanonicalReadOptions, DeterministicIdGenerator, FixedClock,
    NonUserQuestionOutcome, OperationId, Principal, PrincipalKind, Project, QuestionAlternative,
    QuestionDraft, QuestionEstablishedFact, QuestionEvidenceFreshness, QuestionMateriality,
    QuestionResearchState, Source, SourceDraft, SourcePayload, Store as ContextStore,
    TimestampMicros,
};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDisposition,
    CandidateDraft, CandidateFreshness, CandidateKind, CandidateObservationBasis, CandidateOrigin,
    CandidateRetention, CandidateStore, DuplicateAssessment, MaterialityAssessment,
    MaterialityStatus, QuestionCandidate, RepositoryResearchBasis, SubmissionOutcome,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn context(path: &Path, ids: &[u8]) -> Result<ContextStore, volicord_context::Error> {
    ContextStore::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )
}

fn setup_context(store: &mut ContextStore) -> Result<(Project, Source), volicord_context::Error> {
    let project = store.create_project(operation(1), "Inquiry")?.value;
    let source = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "commit-a".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "local-repository".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".to_owned(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    Ok((project, source))
}

fn question_candidate(source: &Source) -> QuestionCandidate {
    QuestionCandidate {
        prompt_basis: "Which storage boundary should the feature use?".to_owned(),
        known_facts: vec![QuestionEstablishedFact {
            statement: "Canonical Context is local".to_owned(),
            source_basis: vec![source.id],
            capability: Some("inventory".to_owned()),
            freshness: QuestionEvidenceFreshness::Current,
        }],
        assumptions: vec!["the Project remains local-first".to_owned()],
        uncertainty: vec!["future provider availability".to_owned()],
        affected_scope: vec!["storage".to_owned()],
        possible_prerequisites: vec![],
        source_basis: vec![source.id],
        repository_basis: vec![],
        freshness: CandidateFreshness::Current,
        duplicate_assessment: DuplicateAssessment::NoDuplicate {
            basis: "canonical read contained no matching Question".to_owned(),
        },
        materiality: MaterialityAssessment {
            status: MaterialityStatus::Material,
            rationale: Some("the choice changes retained data".to_owned()),
            source_basis: vec![source.id],
            assessed_by: Some(Principal {
                kind: PrincipalKind::Agent,
                identity: "inquiry".to_owned(),
            }),
            assessed_at: Some(TimestampMicros::from_unix_micros(900)),
        },
        presentation_order: Some(7),
        why_it_matters_now: "the implementation cannot choose a durable boundary safely".to_owned(),
        alternatives: vec![QuestionAlternative {
            key: "local".to_owned(),
            label: "Local".to_owned(),
            consequence: "No background transmission".to_owned(),
        }],
        recommendation: AgentRecommendation {
            alternative_key: Some("local".to_owned()),
            rationale: "matches the local-first contract".to_owned(),
            source_basis: vec![source.id],
        },
        trade_offs: vec!["less remote semantic capability".to_owned()],
        known_limits: vec!["provider behavior is not evaluated".to_owned()],
        what_the_answer_unlocks: vec!["storage implementation".to_owned()],
        allowed_non_choice_dispositions: vec![
            NonUserQuestionOutcome::ResolvedByResearch,
            NonUserQuestionOutcome::RequiresPrototype,
            NonUserQuestionOutcome::Deferred,
            NonUserQuestionOutcome::OutOfScope,
            NonUserQuestionOutcome::Superseded,
        ],
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

fn candidate_draft(project: &Project, source: &Source) -> CandidateDraft {
    CandidateDraft {
        project_id: project.id,
        kind: CandidateKind::QuestionCandidate,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "codex".to_owned(),
            },
            subsystem: "inquiry".to_owned(),
            session: Some("session-a".to_owned()),
            provenance_summary: "material uncertainty observation".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id: project.id,
            session: Some("session-a".to_owned()),
            source_operation: Some("design-review".to_owned()),
            candidate_kind: CandidateKind::QuestionCandidate,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source.id],
            repository_snapshot: Some("commit-a".to_owned()),
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(800),
        retention: CandidateRetention {
            retained_until: None,
            basis: "retain through inquiry review".to_owned(),
        },
        content: CandidateContent {
            bounded_summary: "storage boundary is material".to_owned(),
            question: Some(question_candidate(source)),
        },
    }
}

fn canonical_draft(project: &Project, candidate: &QuestionCandidate) -> QuestionDraft {
    QuestionDraft {
        expected_project_revision: project.revision,
        prompt_basis: candidate.prompt_basis.clone(),
        source_basis: candidate.source_basis.clone(),
        dependencies: candidate.possible_prerequisites.clone(),
        alternatives: candidate.alternatives.clone(),
        recommendation: candidate.recommendation.clone(),
        trade_offs: candidate.trade_offs.clone(),
        uncertainty: candidate.uncertainty.clone(),
        material_scope: candidate.affected_scope.clone(),
        materiality: QuestionMateriality::Material,
        presentation_order: candidate.presentation_order.unwrap_or_default(),
        why_it_matters_now: candidate.why_it_matters_now.clone(),
        established_facts: candidate.known_facts.clone(),
        assumptions: candidate.assumptions.clone(),
        known_limits: candidate.known_limits.clone(),
        what_the_answer_unlocks: candidate.what_the_answer_unlocks.clone(),
        allowed_non_choice_dispositions: candidate.allowed_non_choice_dispositions.clone(),
        research_state: candidate.research_state,
    }
}

#[test]
fn promotion_reconciles_a_canonical_commit_without_creating_a_duplicate(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut canonical = context(&root.path().join("canonical.sqlite3"), &[1, 2, 3])?;
    let (project, source) = setup_context(&mut canonical)?;
    let mut candidates = CandidateStore::open_with(
        root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[9; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let candidate = match candidates.submit(candidate_draft(&project, &source))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("Question Candidate collection was disabled".into()),
    };
    let question = candidate
        .content
        .as_ref()
        .and_then(|content| content.question.as_ref())
        .ok_or("Question Candidate content missing")?;
    let committed = canonical.create_question(
        OperationId::from_bytes(*candidate.id.as_bytes()),
        project.id,
        canonical_draft(&project, question),
    )?;
    assert_eq!(
        candidates.get(project.id, candidate.id)?.disposition,
        CandidateDisposition::PendingOrRetained
    );

    let basis = canonical.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let promoted = candidates.promote_question(&mut canonical, &basis, project.id, candidate.id)?;
    assert!(promoted.canonical_replayed);
    assert!(promoted.candidate_reconciled);
    assert_eq!(promoted.question_id, committed.value.id);
    let repeated = candidates.promote_question(&mut canonical, &basis, project.id, candidate.id)?;
    assert_eq!(repeated.question_id, committed.value.id);
    let cleaned = candidates.delete_candidate(
        project.id,
        candidate.id,
        "explicit Candidate retention cleanup",
    )?;
    assert!(cleaned.content.is_none());
    assert_eq!(cleaned.promotion_target, Some(committed.value.id));
    assert_eq!(
        canonical
            .read_canonical_basis(project.id, CanonicalReadOptions::default())?
            .active_questions
            .len(),
        1
    );

    let bundle = root.path().join("bundle.json");
    canonical.export_bundle(project.id, &bundle)?;
    let bundle_text = std::fs::read_to_string(bundle)?;
    assert!(!bundle_text.contains("storage boundary is material"));
    let retained_basis = candidates.read_basis(project.id)?;
    assert!(retained_basis.candidates[0].content.is_none());
    assert_eq!(
        retained_basis.candidates[0].promotion_target,
        Some(committed.value.id)
    );
    Ok(())
}

#[test]
fn research_attachment_is_source_grounded_and_does_not_promote(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut canonical = context(&root.path().join("canonical.sqlite3"), &[1, 2])?;
    let (project, source) = setup_context(&mut canonical)?;
    let basis = canonical.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let mut candidates = CandidateStore::open_with(
        root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[4; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let candidate = match candidates.submit(candidate_draft(&project, &source))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("Question Candidate collection was disabled".into()),
    };
    candidates.set_research_state(
        project.id,
        candidate.id,
        QuestionResearchState::ResearchRequired,
    )?;
    candidates.attach_research_basis(
        project.id,
        candidate.id,
        &basis,
        RepositoryResearchBasis {
            repository_snapshot: "commit-a".to_owned(),
            analysis_snapshot: None,
            capability: "inventory".to_owned(),
            coverage: "repository root".to_owned(),
            freshness: CandidateFreshness::Current,
            source_basis: vec![source.id],
            sufficient: true,
            limits: vec!["runtime behavior excluded".to_owned()],
        },
    )?;
    let updated = candidates.set_research_state(
        project.id,
        candidate.id,
        QuestionResearchState::ReadyToAsk,
    )?;
    assert_eq!(updated.disposition, CandidateDisposition::PendingOrRetained);
    assert_eq!(
        updated
            .content
            .as_ref()
            .and_then(|content| content.question.as_ref())
            .map(|question| question.repository_basis.len()),
        Some(1)
    );
    assert!(basis.active_questions.is_empty());
    Ok(())
}

#[test]
fn promotion_requires_an_explicit_materiality_transition() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;
    let mut canonical = context(&root.path().join("canonical.sqlite3"), &[1, 2, 3])?;
    let (project, source) = setup_context(&mut canonical)?;
    let basis = canonical.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let mut candidates = CandidateStore::open_with(
        root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[5; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let mut draft = candidate_draft(&project, &source);
    if let Some(question) = draft.content.question.as_mut() {
        question.materiality = MaterialityAssessment::default();
    }
    let candidate = match candidates.submit(draft)? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("Question Candidate collection was disabled".into()),
    };
    assert_eq!(
        candidates
            .promote_question(&mut canonical, &basis, project.id, candidate.id)
            .err()
            .ok_or("unassessed Candidate was promoted")?
            .kind(),
        volicord_inquiry::ErrorKind::DomainConflict
    );
    let actor = Principal {
        kind: PrincipalKind::Agent,
        identity: "inquiry".to_owned(),
    };
    candidates.assess_materiality(
        project.id,
        candidate.id,
        MaterialityAssessment {
            status: MaterialityStatus::NeedsEvidence,
            rationale: Some("repository evidence is still incomplete".to_owned()),
            source_basis: vec![source.id],
            assessed_by: Some(actor.clone()),
            assessed_at: None,
        },
    )?;
    assert!(candidates
        .promote_question(&mut canonical, &basis, project.id, candidate.id)
        .is_err());
    candidates.assess_materiality(
        project.id,
        candidate.id,
        MaterialityAssessment {
            status: MaterialityStatus::Material,
            rationale: Some("the storage result changes materially".to_owned()),
            source_basis: vec![source.id],
            assessed_by: Some(actor),
            assessed_at: None,
        },
    )?;
    let promoted = candidates.promote_question(&mut canonical, &basis, project.id, candidate.id)?;
    assert!(promoted.candidate_reconciled);
    Ok(())
}
