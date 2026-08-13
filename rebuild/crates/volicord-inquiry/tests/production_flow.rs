use std::fs;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointKind,
    DeterministicIdGenerator, FixedClock, NonUserQuestionOutcome, OperationId, Principal,
    PrincipalKind, Project, QuestionAlternative, QuestionDraft, QuestionMateriality,
    QuestionResearchState, Source, SourceDraft, SourcePayload, Store, TimestampMicros,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, VerificationFact,
    VerificationState, WorkState,
};
use volicord_inquiry::{
    attribute_repository_changes, evaluate_checkpoint_candidate, evaluate_decision_applicability,
    interpret_current_host_response, propose_requestioning, recompute_frontier_for_resume,
    record_checkpoint, record_response_batch, ApplicabilityIssue, ApplicabilityQuery,
    BatchResponseItem, BatchResponseOutcome, CandidateRetention, CandidateStore, ChangeAttribution,
    CheckpointCandidate, CheckpointEvaluation, CheckpointRejection, CurrentHostResponse,
    DecisionApplicabilityState, DisplayedQuestion, InquiryScope, RepositoryWorkBasis,
    RequestioningProposal, ResponseInterpretation, ResponseMapping, ResponseRejection,
    SubmissionOutcome,
};
use volicord_repository_intelligence::{
    inventory_repository, CanonicalGrounding, InventoryRequest,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn source_draft(project: &Project, payload: SourcePayload, actor: Principal) -> SourceDraft {
    SourceDraft {
        expected_project_revision: project.revision,
        payload,
        actor,
        observer: Some(Principal {
            kind: PrincipalKind::Agent,
            identity: "codex-host-adapter".to_owned(),
        }),
        availability: Availability::Available,
    }
}

fn question_draft(project: &Project, source: &Source, prompt: &str, order: u64) -> QuestionDraft {
    QuestionDraft {
        expected_project_revision: project.revision,
        prompt_basis: prompt.to_owned(),
        source_basis: vec![source.id],
        dependencies: Vec::new(),
        alternatives: vec![
            QuestionAlternative {
                key: "local".to_owned(),
                label: "Local".to_owned(),
                consequence: "keep the boundary local".to_owned(),
            },
            QuestionAlternative {
                key: "remote".to_owned(),
                label: "Remote".to_owned(),
                consequence: "use a remote boundary".to_owned(),
            },
        ],
        recommendation: AgentRecommendation {
            alternative_key: Some("local".to_owned()),
            rationale: "the Project is local-first".to_owned(),
            source_basis: vec![source.id],
        },
        trade_offs: vec!["remote capability remains unavailable".to_owned()],
        uncertainty: vec!["future scale is unknown".to_owned()],
        material_scope: vec!["storage".to_owned()],
        materiality: QuestionMateriality::Material,
        presentation_order: order,
        why_it_matters_now: "implementation is blocked".to_owned(),
        established_facts: Vec::new(),
        assumptions: vec!["local-first".to_owned()],
        known_limits: vec!["provider behavior is out of scope".to_owned()],
        what_the_answer_unlocks: vec!["implementation".to_owned()],
        allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

struct Setup {
    root: tempfile::TempDir,
    store: Store,
    project: Project,
    repository: Source,
    user_turn: Source,
}

fn setup() -> Result<Setup, Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=80).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let project = store
        .create_project(operation(90), "Production Inquiry")?
        .value;
    let repository = store
        .record_source(
            operation(91),
            project.id,
            source_draft(
                &project,
                SourcePayload::RepositorySnapshot {
                    revision: "baseline".to_owned(),
                },
                Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository".to_owned(),
                },
            ),
        )?
        .value;
    let user_turn = store
        .record_source(
            operation(92),
            project.id,
            source_draft(
                &project,
                SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "session-a".to_owned(),
                    turn: "turn-7".to_owned(),
                },
                Principal {
                    kind: PrincipalKind::User,
                    identity: "project-owner".to_owned(),
                },
            ),
        )?
        .value;
    Ok(Setup {
        root,
        store,
        project,
        repository,
        user_turn,
    })
}

fn response(
    project: &Project,
    source: &Source,
    question: &volicord_context::Question,
    mapping: ResponseMapping,
) -> CurrentHostResponse {
    CurrentHostResponse {
        project_id: project.id,
        source_id: source.id,
        host: "codex".to_owned(),
        session: "session-a".to_owned(),
        turn: "turn-7".to_owned(),
        displayed: DisplayedQuestion {
            question_id: question.id,
            revision: question.revision,
            alternative_keys: question
                .alternatives
                .iter()
                .map(|alternative| alternative.key.clone())
                .collect(),
            recommendation_key: question.recommendation.alternative_key.clone(),
        },
        mapping,
        applicability: ApplicabilityScope {
            paths: vec!["src".to_owned()],
            components: vec!["context".to_owned()],
            work_contexts: vec!["phase-6".to_owned()],
        },
        assumptions: vec!["local-first".to_owned()],
        revisit_triggers: vec!["source boundary changes".to_owned()],
    }
}

#[test]
fn exact_response_rejections_do_not_partially_transition_and_batch_is_truthful(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root: _root,
        mut store,
        project,
        repository,
        user_turn,
    } = setup()?;
    let first = store
        .create_question(
            operation(93),
            project.id,
            question_draft(&project, &repository, "First?", 1),
        )?
        .value;
    let second = store
        .create_question(
            operation(94),
            project.id,
            question_draft(&project, &repository, "Second?", 2),
        )?
        .value;
    let before = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    for (mapping, expected) in [
        (
            ResponseMapping::Ambiguous,
            ResponseRejection::AmbiguousResponse,
        ),
        (
            ResponseMapping::RecommendationEcho,
            ResponseRejection::RecommendationWithoutUserChoice,
        ),
    ] {
        assert!(matches!(
            interpret_current_host_response(&before, &response(&project, &user_turn, &first, mapping)),
            ResponseInterpretation::Rejected { reason, .. } if reason == expected
        ));
    }
    let mut stale = response(
        &project,
        &user_turn,
        &second,
        ResponseMapping::ExplicitAlternative {
            alternative_key: "local".to_owned(),
            user_rationale: None,
        },
    );
    stale.displayed.revision += 1;
    let batch = record_response_batch(
        &mut store,
        project.id,
        vec![
            BatchResponseItem {
                operation_id: operation(95),
                response: response(
                    &project,
                    &user_turn,
                    &first,
                    ResponseMapping::ExplicitAlternative {
                        alternative_key: "local".to_owned(),
                        user_rationale: Some("keep source local".to_owned()),
                    },
                ),
            },
            BatchResponseItem {
                operation_id: operation(96),
                response: stale,
            },
        ],
    );
    assert!(!batch.all_succeeded());
    assert!(matches!(
        batch.items[0].2,
        BatchResponseOutcome::Succeeded(_)
    ));
    assert!(matches!(
        batch.items[1].2,
        BatchResponseOutcome::Rejected {
            reason: ResponseRejection::StaleDisplayedRevision,
            ..
        }
    ));
    let after = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    assert_eq!(after.active_decisions.len(), 1);
    assert_eq!(after.active_questions.len(), 1);
    assert_eq!(after.sources.len(), before.sources.len());
    let resumed = recompute_frontier_for_resume(
        &after,
        &InquiryScope {
            project_id: project.id,
            material_scope: vec!["storage".to_owned()],
        },
    );
    assert_eq!(resumed.recomputed.questions.len(), 1);
    assert_eq!(resumed.recomputed.questions[0].question_id, second.id);
    assert!(resumed.differs_from_checkpoint_observation);

    let replay = record_response_batch(
        &mut store,
        project.id,
        vec![BatchResponseItem {
            operation_id: operation(95),
            response: response(
                &project,
                &user_turn,
                &first,
                ResponseMapping::ExplicitAlternative {
                    alternative_key: "local".to_owned(),
                    user_rationale: Some("keep source local".to_owned()),
                },
            ),
        }],
    );
    assert!(matches!(
        replay.items[0].2,
        BatchResponseOutcome::Replayed(_)
    ));
    Ok(())
}

#[test]
fn applicability_is_evidence_based_and_review_starts_as_candidate(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root,
        mut store,
        project,
        repository,
        user_turn,
    } = setup()?;
    let question = store
        .create_question(
            operation(93),
            project.id,
            question_draft(&project, &repository, "Boundary?", 1),
        )?
        .value;
    let accepted = record_response_batch(
        &mut store,
        project.id,
        vec![BatchResponseItem {
            operation_id: operation(94),
            response: response(
                &project,
                &user_turn,
                &question,
                ResponseMapping::ExplicitDelegation {
                    delegate_to: "implementation-owner".to_owned(),
                    user_rationale: None,
                },
            ),
        }],
    );
    assert!(accepted.all_succeeded());
    let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let lifecycle = &canonical.active_decisions[0];
    let current = evaluate_decision_applicability(
        &canonical,
        lifecycle,
        &ApplicabilityQuery {
            project_id: project.id,
            paths: vec!["src/store.rs".to_owned()],
            components: vec!["context".to_owned()],
            work_contexts: vec!["phase-6".to_owned()],
            current_assumptions: vec!["local-first".to_owned()],
            met_revisit_triggers: Vec::new(),
        },
    );
    assert_eq!(current.state, DecisionApplicabilityState::ReusableCurrent);

    let review = evaluate_decision_applicability(
        &canonical,
        lifecycle,
        &ApplicabilityQuery {
            project_id: project.id,
            paths: vec!["src/store.rs".to_owned()],
            components: vec!["context".to_owned()],
            work_contexts: vec!["phase-6".to_owned()],
            current_assumptions: vec!["local-first".to_owned()],
            met_revisit_triggers: vec!["source boundary changes".to_owned()],
        },
    );
    assert_eq!(
        review.state,
        DecisionApplicabilityState::ReviewRequiredUncertain
    );
    assert!(review.issues.iter().any(|issue| matches!(
        issue,
        ApplicabilityIssue::RevisitTriggerMet(value) if value == "source boundary changes"
    )));
    let mut candidates = CandidateStore::open_with(
        root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[70; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(2_000)),
    )?;
    let proposed = propose_requestioning(
        &mut candidates,
        &canonical,
        &review,
        RequestioningProposal {
            session: "session-a".to_owned(),
            source_operation: "decision-review".to_owned(),
            observed_at: TimestampMicros::from_unix_micros(1_900),
            retention: CandidateRetention {
                retained_until: None,
                basis: "retain for explicit review".to_owned(),
            },
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "inquiry".to_owned(),
            },
            review_explanation: "declared revisit trigger is met".to_owned(),
        },
    )?;
    assert!(matches!(proposed, SubmissionOutcome::Stored(_)));
    assert_eq!(
        canonical,
        store.read_canonical_basis(project.id, CanonicalReadOptions::default())?
    );
    Ok(())
}

#[test]
fn checkpoint_uses_snapshot_delta_and_preserves_independent_dimensions(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root,
        mut store,
        project,
        repository,
        user_turn: _user_turn,
    } = setup()?;
    let repository_root = root.path().join("repo");
    fs::create_dir_all(repository_root.join("src"))?;
    fs::write(repository_root.join("src/lib.rs"), "pub fn old() {}\n")?;
    fs::write(repository_root.join("pre.txt"), "already dirty\n")?;
    let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, baseline) = inventory_repository(InventoryRequest::new(
        &repository_root,
        &grounding,
        repository.id,
        1_000,
    )?)?;
    fs::write(repository_root.join("src/lib.rs"), "pub fn current() {}\n")?;
    let (_, current) = inventory_repository(InventoryRequest::new(
        &repository_root,
        &grounding,
        repository.id,
        2_000,
    )?)?;
    let work = RepositoryWorkBasis {
        baseline: &baseline,
        current: &current,
        pre_existing_dirty_paths: vec!["pre.txt".to_owned()],
    };
    assert_eq!(
        attribute_repository_changes(project.id, &work),
        ChangeAttribution::Attributed {
            pre_existing_paths: vec!["pre.txt".to_owned()],
            changed_paths: vec!["src/lib.rs".to_owned()],
        }
    );
    let evaluation = evaluate_checkpoint_candidate(
        &canonical,
        CheckpointCandidate {
            project_id: project.id,
            kind: CheckpointKind::Completion,
            goal: "implement bounded work".to_owned(),
            work_state: WorkState::Completed,
            state_change: Some("implemented the bounded change".to_owned()),
            repository_work: Some(work),
            supporting_sources: vec![repository.id],
            applied_decisions: Vec::new(),
            verification: vec![VerificationFact {
                state: VerificationState::NotRun,
                source_id: None,
                outcome: None,
            }],
            user_review: UserReviewFact {
                state: UserReviewState::NotRequested,
                source_id: None,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::NotRequested,
                source_id: None,
            },
            known_limits: vec!["verification not run".to_owned()],
            non_goals: vec!["no process execution".to_owned()],
            next_step: "run focused validation".to_owned(),
            handoff_to: None,
            status_only: false,
        },
    );
    let recorded = record_checkpoint(&mut store, operation(95), project.id, evaluation)?.value;
    assert_eq!(recorded.changed_paths, vec!["src/lib.rs"]);
    assert_eq!(recorded.verification[0].state, VerificationState::NotRun);
    assert_eq!(recorded.user_review.state, UserReviewState::NotRequested);
    assert_eq!(
        recorded.user_acceptance.state,
        UserAcceptanceState::NotRequested
    );

    let current_basis = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let rejected = evaluate_checkpoint_candidate(
        &current_basis,
        CheckpointCandidate {
            project_id: project.id,
            kind: CheckpointKind::Pause,
            goal: "read status".to_owned(),
            work_state: WorkState::Paused,
            state_change: None,
            repository_work: None,
            supporting_sources: vec![repository.id],
            applied_decisions: Vec::new(),
            verification: Vec::new(),
            user_review: UserReviewFact {
                state: UserReviewState::NotRequested,
                source_id: None,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::NotRequested,
                source_id: None,
            },
            known_limits: Vec::new(),
            non_goals: Vec::new(),
            next_step: "continue".to_owned(),
            handoff_to: None,
            status_only: true,
        },
    );
    assert!(matches!(
        rejected,
        CheckpointEvaluation::Rejected {
            reason: CheckpointRejection::StatusOnly,
            ..
        }
    ));
    assert!(record_checkpoint(&mut store, operation(96), project.id, rejected).is_err());
    assert_eq!(
        store
            .read_canonical_basis(
                project.id,
                CanonicalReadOptions {
                    include_checkpoint_history: true
                },
            )?
            .checkpoint_history
            .len(),
        1
    );
    Ok(())
}
