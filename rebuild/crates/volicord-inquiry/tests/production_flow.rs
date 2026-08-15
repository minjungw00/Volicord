use std::fs;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadBasis,
    CanonicalReadOptions, CheckpointKind, CommandOutcome, CommandTermination,
    DeterministicIdGenerator, FixedClock, NonUserQuestionOutcome, OperationId, Principal,
    PrincipalKind, Project, ProjectId, QuestionAlternative, QuestionDraft, QuestionMateriality,
    QuestionResearchState, Source, SourceDraft, SourceFreshness, SourceId, SourcePayload, Store,
    TimestampMicros, UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState,
    VerificationFact, VerificationState, WorkState,
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
    inventory_repository, AnalysisSnapshot, CanonicalGrounding, CanonicalSourceRef, FreshnessState,
    InventoryRequest,
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

fn pause_candidate<'a>(
    project_id: ProjectId,
    supporting_sources: Vec<SourceId>,
) -> CheckpointCandidate<'a> {
    CheckpointCandidate {
        project_id,
        kind: CheckpointKind::Pause,
        goal: "pause with grounded context".to_owned(),
        work_state: WorkState::Paused,
        state_change: None,
        repository_work: None,
        supporting_sources,
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
        next_step: "continue from current grounding".to_owned(),
        handoff_to: None,
        status_only: false,
    }
}

fn repository_checkpoint_candidate<'a>(
    project_id: ProjectId,
    baseline: &'a AnalysisSnapshot,
    current: &'a AnalysisSnapshot,
) -> CheckpointCandidate<'a> {
    CheckpointCandidate {
        project_id,
        kind: CheckpointKind::Completion,
        goal: "record attributable repository work".to_owned(),
        work_state: WorkState::Completed,
        state_change: None,
        repository_work: Some(RepositoryWorkBasis {
            baseline,
            current,
            pre_existing_dirty_paths: Vec::new(),
        }),
        supporting_sources: Vec::new(),
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
        next_step: "verify the attributed change".to_owned(),
        handoff_to: None,
        status_only: false,
    }
}

fn set_source_freshness(
    canonical: &mut CanonicalReadBasis,
    source_id: SourceId,
    freshness: SourceFreshness,
) {
    let basis = canonical
        .sources
        .iter_mut()
        .find(|basis| basis.source.id == source_id)
        .unwrap_or_else(|| panic!("test Source {source_id} is missing"));
    basis.freshness = freshness;
    basis.availability = match freshness {
        SourceFreshness::Current => Availability::Available,
        SourceFreshness::Stale => Availability::Stale,
        SourceFreshness::Unavailable => Availability::Unavailable,
        SourceFreshness::Unknown => Availability::Unknown,
    };
    basis.source.availability = basis.availability;
}

fn assert_checkpoint_rejection(
    evaluation: CheckpointEvaluation,
    expected: CheckpointRejection,
    detail_fragment: &str,
) {
    match evaluation {
        CheckpointEvaluation::Rejected { reason, detail, .. } => {
            assert_eq!(reason, expected);
            assert!(
                detail.contains(detail_fragment),
                "rejection detail {detail:?} did not contain {detail_fragment:?}"
            );
        }
        CheckpointEvaluation::Ready { .. } => {
            panic!("expected {expected:?} Checkpoint rejection")
        }
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
        user_turn,
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
    let compatible_reference = |identity: SourceId, scope: &str, observed_at: i64| {
        serde_json::from_value::<CanonicalSourceRef>(serde_json::json!({
            "project": project.id.to_string(),
            "identity": identity.to_string(),
            "basis": {
                "kind": "snapshot",
                "value": format!("local-observation:sha256:{scope}:at:{observed_at}")
            }
        }))
    };
    let scope = "ab".repeat(32);
    let mut distinct_baseline = baseline.clone();
    let mut distinct_current = current.clone();
    distinct_baseline.repository_source = compatible_reference(repository.id, &scope, 1_000)?;
    distinct_current.repository_source = compatible_reference(user_turn.id, &scope, 2_000)?;
    assert_eq!(
        attribute_repository_changes(
            project.id,
            &RepositoryWorkBasis {
                baseline: &distinct_baseline,
                current: &distinct_current,
                pre_existing_dirty_paths: vec!["pre.txt".into()],
            }
        ),
        ChangeAttribution::Attributed {
            pre_existing_paths: vec!["pre.txt".into()],
            changed_paths: vec!["src/lib.rs".into()],
        }
    );
    distinct_current.repository_source =
        compatible_reference(user_turn.id, &"cd".repeat(32), 2_000)?;
    assert!(matches!(
        attribute_repository_changes(
            project.id,
            &RepositoryWorkBasis {
                baseline: &distinct_baseline,
                current: &distinct_current,
                pre_existing_dirty_paths: Vec::new(),
            }
        ),
        ChangeAttribution::Unavailable { .. }
    ));
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

#[test]
fn checkpoint_rejects_unavailable_supporting_source() -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root: _root,
        mut store,
        project,
        repository: _repository,
        user_turn: _user_turn,
    } = setup()?;
    let unavailable = store
        .record_source(
            operation(93),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::File {
                    locator: "src/unavailable.rs".to_owned(),
                    snapshot: "snapshot-a".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex-host-adapter".to_owned(),
                }),
                availability: Availability::Unavailable,
            },
        )?
        .value;
    let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let evaluation = evaluate_checkpoint_candidate(
        &canonical,
        pause_candidate(project.id, vec![unavailable.id]),
    );
    assert_checkpoint_rejection(
        evaluation.clone(),
        CheckpointRejection::SourceUnavailable,
        "availability=Unavailable",
    );
    assert!(record_checkpoint(&mut store, operation(94), project.id, evaluation).is_err());
    assert_eq!(
        store.read_canonical_basis(project.id, CanonicalReadOptions::default())?,
        canonical
    );
    Ok(())
}

#[test]
fn checkpoint_support_requires_every_source_to_be_current_and_project_owned(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root: _root,
        store,
        project,
        repository,
        user_turn,
    } = setup()?;
    let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;

    assert!(matches!(
        evaluate_checkpoint_candidate(&canonical, pause_candidate(project.id, vec![repository.id])),
        CheckpointEvaluation::Ready { .. }
    ));
    assert!(matches!(
        evaluate_checkpoint_candidate(
            &canonical,
            pause_candidate(project.id, vec![repository.id, user_turn.id])
        ),
        CheckpointEvaluation::Ready { .. }
    ));

    for (freshness, expected, detail) in [
        (
            SourceFreshness::Unavailable,
            CheckpointRejection::SourceUnavailable,
            "Unavailable",
        ),
        (
            SourceFreshness::Stale,
            CheckpointRejection::SourceStale,
            "Stale",
        ),
        (
            SourceFreshness::Unknown,
            CheckpointRejection::SourceFreshnessUnknown,
            "Unknown",
        ),
    ] {
        let mut degraded = canonical.clone();
        set_source_freshness(&mut degraded, repository.id, freshness);
        assert_checkpoint_rejection(
            evaluate_checkpoint_candidate(
                &degraded,
                pause_candidate(project.id, vec![repository.id]),
            ),
            expected,
            detail,
        );
        assert_checkpoint_rejection(
            evaluate_checkpoint_candidate(
                &degraded,
                pause_candidate(project.id, vec![repository.id, user_turn.id]),
            ),
            expected,
            detail,
        );
    }

    let missing = SourceId::from_bytes([99; 16]);
    assert_checkpoint_rejection(
        evaluate_checkpoint_candidate(&canonical, pause_candidate(project.id, vec![missing])),
        CheckpointRejection::MissingSourceBasis,
        "missing from the canonical read basis",
    );

    let mut wrong_project = canonical.clone();
    let repository_basis = wrong_project
        .sources
        .iter_mut()
        .find(|basis| basis.source.id == repository.id)
        .ok_or("repository Source basis missing")?;
    repository_basis.source.project_id = ProjectId::from_bytes([98; 16]);
    assert_checkpoint_rejection(
        evaluate_checkpoint_candidate(
            &wrong_project,
            pause_candidate(project.id, vec![repository.id]),
        ),
        CheckpointRejection::WrongProject,
        "different Project",
    );
    Ok(())
}

#[test]
fn repository_checkpoint_basis_requires_current_analysis_and_canonical_source(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root,
        store,
        project,
        repository,
        user_turn: _user_turn,
    } = setup()?;
    let repository_root = root.path().join("repository-grounding");
    fs::create_dir_all(repository_root.join("src"))?;
    fs::write(repository_root.join("src/lib.rs"), "pub fn baseline() {}\n")?;
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

    let ready = evaluate_checkpoint_candidate(
        &canonical,
        repository_checkpoint_candidate(project.id, &baseline, &current),
    );
    let CheckpointEvaluation::Ready { draft, .. } = ready else {
        return Err("current repository grounding was unexpectedly rejected".into());
    };
    assert_eq!(draft.source_basis, vec![repository.id]);
    assert_eq!(draft.changed_source_basis, vec![repository.id]);
    assert_eq!(draft.changed_paths, vec!["src/lib.rs"]);

    for (freshness, expected) in [
        (
            SourceFreshness::Unavailable,
            CheckpointRejection::SourceUnavailable,
        ),
        (SourceFreshness::Stale, CheckpointRejection::SourceStale),
        (
            SourceFreshness::Unknown,
            CheckpointRejection::SourceFreshnessUnknown,
        ),
    ] {
        let mut degraded = canonical.clone();
        set_source_freshness(&mut degraded, repository.id, freshness);
        let evaluation = evaluate_checkpoint_candidate(
            &degraded,
            repository_checkpoint_candidate(project.id, &baseline, &current),
        );
        assert_checkpoint_rejection(evaluation, expected, "baseline repository work");
    }

    for (freshness, expected) in [
        (FreshnessState::Stale, CheckpointRejection::SourceStale),
        (
            FreshnessState::Unknown,
            CheckpointRejection::SourceFreshnessUnknown,
        ),
    ] {
        let mut non_current = current.clone();
        non_current.freshness.state = freshness;
        let evaluation = evaluate_checkpoint_candidate(
            &canonical,
            repository_checkpoint_candidate(project.id, &baseline, &non_current),
        );
        assert_checkpoint_rejection(evaluation, expected, "Repository Intelligence basis");
    }

    let mismatched_reference: CanonicalSourceRef = serde_json::from_value(serde_json::json!({
        "project": project.id.to_string(),
        "identity": repository.id.to_string(),
        "basis": {
            "kind": "snapshot",
            "value": "different-canonical-snapshot"
        }
    }))?;
    let mut mismatched_baseline = baseline.clone();
    let mut mismatched_current = current.clone();
    mismatched_baseline.repository_source = mismatched_reference.clone();
    mismatched_current.repository_source = mismatched_reference;
    assert_checkpoint_rejection(
        evaluate_checkpoint_candidate(
            &canonical,
            repository_checkpoint_candidate(project.id, &mismatched_baseline, &mismatched_current),
        ),
        CheckpointRejection::SourceUnavailable,
        "snapshot basis disagrees",
    );

    let mut disagreeing_current = current.clone();
    disagreeing_current.repository_source = mismatched_baseline.repository_source.clone();
    assert_checkpoint_rejection(
        evaluate_checkpoint_candidate(
            &canonical,
            repository_checkpoint_candidate(project.id, &baseline, &disagreeing_current),
        ),
        CheckpointRejection::SourceUnavailable,
        "do not identify the same Project Source",
    );
    Ok(())
}

#[test]
fn verification_checkpoint_basis_is_current_and_keeps_command_kind_semantics(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root: _root,
        mut store,
        project,
        repository,
        user_turn: _user_turn,
    } = setup()?;
    let command = store
        .record_source(
            operation(93),
            project.id,
            source_draft(
                &project,
                SourcePayload::CommandExecution {
                    command_label: "cargo test".to_owned(),
                    outcome: CommandOutcome {
                        exit_code: Some(0),
                        termination: CommandTermination::Exited,
                    },
                },
                Principal {
                    kind: PrincipalKind::Command,
                    identity: "focused-validation".to_owned(),
                },
            ),
        )?
        .value;
    let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let executed = |source_id| VerificationFact {
        state: VerificationState::Passed,
        source_id: Some(source_id),
        outcome: Some("focused validation passed".to_owned()),
    };
    let mut current = pause_candidate(project.id, vec![repository.id]);
    current.verification = vec![executed(command.id)];
    assert!(matches!(
        evaluate_checkpoint_candidate(&canonical, current),
        CheckpointEvaluation::Ready { .. }
    ));

    for (freshness, expected) in [
        (
            SourceFreshness::Unavailable,
            CheckpointRejection::SourceUnavailable,
        ),
        (SourceFreshness::Stale, CheckpointRejection::SourceStale),
        (
            SourceFreshness::Unknown,
            CheckpointRejection::SourceFreshnessUnknown,
        ),
    ] {
        let mut degraded = canonical.clone();
        set_source_freshness(&mut degraded, command.id, freshness);
        let mut candidate = pause_candidate(project.id, vec![repository.id]);
        candidate.verification = vec![executed(command.id)];
        assert_checkpoint_rejection(
            evaluate_checkpoint_candidate(&degraded, candidate),
            expected,
            "executed verification",
        );
    }

    let mut not_run = pause_candidate(project.id, vec![repository.id]);
    not_run.verification = vec![VerificationFact {
        state: VerificationState::NotRun,
        source_id: None,
        outcome: None,
    }];
    assert!(matches!(
        evaluate_checkpoint_candidate(&canonical, not_run),
        CheckpointEvaluation::Ready { .. }
    ));
    let mut wrong_kind = pause_candidate(project.id, vec![repository.id]);
    wrong_kind.verification = vec![executed(repository.id)];
    assert_checkpoint_rejection(
        evaluate_checkpoint_candidate(&canonical, wrong_kind),
        CheckpointRejection::InvalidSourceKind,
        "command-execution Source",
    );
    Ok(())
}

#[test]
fn review_and_acceptance_require_current_user_turn_sources_only_when_observed(
) -> Result<(), Box<dyn std::error::Error>> {
    let Setup {
        root: _root,
        store,
        project,
        repository,
        user_turn,
    } = setup()?;
    let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;

    let mut reviewed = pause_candidate(project.id, vec![repository.id]);
    reviewed.user_review = UserReviewFact {
        state: UserReviewState::Reviewed,
        source_id: Some(user_turn.id),
    };
    assert!(matches!(
        evaluate_checkpoint_candidate(&canonical, reviewed),
        CheckpointEvaluation::Ready { .. }
    ));
    for acceptance in [UserAcceptanceState::Accepted, UserAcceptanceState::Rejected] {
        let mut observed = pause_candidate(project.id, vec![repository.id]);
        observed.user_acceptance = UserAcceptanceFact {
            state: acceptance,
            source_id: Some(user_turn.id),
        };
        assert!(matches!(
            evaluate_checkpoint_candidate(&canonical, observed),
            CheckpointEvaluation::Ready { .. }
        ));
    }

    for (freshness, expected) in [
        (
            SourceFreshness::Unavailable,
            CheckpointRejection::SourceUnavailable,
        ),
        (SourceFreshness::Stale, CheckpointRejection::SourceStale),
        (
            SourceFreshness::Unknown,
            CheckpointRejection::SourceFreshnessUnknown,
        ),
    ] {
        let mut degraded = canonical.clone();
        set_source_freshness(&mut degraded, user_turn.id, freshness);
        let mut review = pause_candidate(project.id, vec![repository.id]);
        review.user_review = UserReviewFact {
            state: UserReviewState::Reviewed,
            source_id: Some(user_turn.id),
        };
        assert_checkpoint_rejection(
            evaluate_checkpoint_candidate(&degraded, review),
            expected,
            "observed user review",
        );
        for acceptance in [UserAcceptanceState::Accepted, UserAcceptanceState::Rejected] {
            let mut observed = pause_candidate(project.id, vec![repository.id]);
            observed.user_acceptance = UserAcceptanceFact {
                state: acceptance,
                source_id: Some(user_turn.id),
            };
            assert_checkpoint_rejection(
                evaluate_checkpoint_candidate(&degraded, observed),
                expected,
                "observed user acceptance",
            );
        }
    }

    for (review, acceptance) in [
        (
            UserReviewState::NotRequested,
            UserAcceptanceState::NotRequested,
        ),
        (UserReviewState::Pending, UserAcceptanceState::Pending),
    ] {
        let mut unobserved = pause_candidate(project.id, vec![repository.id]);
        unobserved.user_review.state = review;
        unobserved.user_acceptance.state = acceptance;
        assert!(matches!(
            evaluate_checkpoint_candidate(&canonical, unobserved),
            CheckpointEvaluation::Ready { .. }
        ));
    }

    let mut wrong_kind = pause_candidate(project.id, vec![repository.id]);
    wrong_kind.user_review = UserReviewFact {
        state: UserReviewState::Reviewed,
        source_id: Some(repository.id),
    };
    assert_checkpoint_rejection(
        evaluate_checkpoint_candidate(&canonical, wrong_kind),
        CheckpointRejection::InvalidSourceKind,
        "current-host user-turn Source",
    );
    Ok(())
}
