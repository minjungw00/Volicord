use std::fs;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointDraft,
    CheckpointKind, ContextItemDraft, ContextItemRole, DecisionChoice, DecisionSupersessionDraft,
    DeterministicIdGenerator, ExplicitQuestionResponse, FixedClock, NonUserQuestionOutcome,
    OperationId, Principal, PrincipalKind, Project, QuestionAlternative, QuestionDraft,
    QuestionMateriality, QuestionResearchState, QuestionResponseDraft, Source, SourceDraft,
    SourceFreshness, SourcePayload, StatementProvenanceRole, Store, TimestampMicros,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource,
    VerificationFact, VerificationState, WorkState,
};
use volicord_inquiry::{
    ApplicabilityQuery, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDisposition, CandidateDraft, CandidateKind, CandidateObservationBasis,
    CandidateOrigin, CandidateRetention, CandidateStore, CollectionOptOutScope, SubmissionOutcome,
};
use volicord_projections::{
    build_resume_brief, inspect_candidate, BriefDecisionState, CandidateContentAccess,
    CandidateContentOmission, InspectionHealth, OmissionReason, RecallBound, RecallInputs,
    RecallTriggerOutcome, RetentionInspection, SessionRecallTrigger,
};
use volicord_repository_intelligence::{
    inventory_repository, CanonicalGrounding, CapabilityState, InventoryRequest,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn source_draft(
    project: &Project,
    payload: SourcePayload,
    availability: Availability,
) -> SourceDraft {
    SourceDraft {
        expected_project_revision: project.revision,
        payload,
        actor: principal(PrincipalKind::Repository, "repository"),
        observer: Some(principal(PrincipalKind::Agent, "codex")),
        availability,
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
                consequence: "keep local state".to_owned(),
            },
            QuestionAlternative {
                key: "remote".to_owned(),
                label: "Remote".to_owned(),
                consequence: "use remote state".to_owned(),
            },
        ],
        recommendation: AgentRecommendation {
            alternative_key: Some("local".to_owned()),
            rationale: "local-first is safer".to_owned(),
            source_basis: vec![source.id],
        },
        trade_offs: vec!["remote scale is deferred".to_owned()],
        uncertainty: vec!["future load is unknown".to_owned()],
        material_scope: vec!["storage".to_owned()],
        materiality: QuestionMateriality::Material,
        presentation_order: order,
        why_it_matters_now: "the implementation needs a boundary".to_owned(),
        established_facts: Vec::new(),
        assumptions: vec!["local-first".to_owned()],
        known_limits: vec!["runtime behavior is not measured".to_owned()],
        what_the_answer_unlocks: vec!["storage implementation".to_owned()],
        allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

#[test]
fn trigger_ignores_unrelated_requests_and_runs_once_per_session() {
    let project = volicord_context::ProjectId::from_bytes([7; 16]);
    let mut trigger = SessionRecallTrigger::new();
    assert_eq!(
        trigger.observe(None),
        RecallTriggerOutcome::UnrelatedRequest
    );
    assert_eq!(
        trigger.observe(Some(project)),
        RecallTriggerOutcome::FirstProjectScoped {
            project_id: project
        }
    );
    assert_eq!(
        trigger.observe(Some(project)),
        RecallTriggerOutcome::LaterProjectScoped {
            project_id: project
        }
    );
}

#[test]
fn candidate_inspection_is_complete_or_explicitly_partial_and_never_mutates(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let project = volicord_context::ProjectId::from_bytes([1; 16]);
    let source = volicord_context::SourceId::from_bytes([2; 16]);
    let mut store = CandidateStore::open_with(
        root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[3; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let candidate = match store.submit(CandidateDraft {
        project_id: project,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: principal(PrincipalKind::Agent, "codex"),
            subsystem: "repository-intelligence".to_owned(),
            session: Some("session-a".to_owned()),
            provenance_summary: "bounded observation".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id: project,
            session: Some("session-a".to_owned()),
            source_operation: Some("inventory".to_owned()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source],
            repository_snapshot: Some("snapshot-a".to_owned()),
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(900),
        retention: CandidateRetention {
            retained_until: Some(TimestampMicros::from_unix_micros(3_000)),
            basis: "session retention".to_owned(),
        },
        content: CandidateContent {
            bounded_summary: "one bounded observation".to_owned(),
            question: None,
        },
    })? {
        SubmissionOutcome::Stored(value) => value,
        SubmissionOutcome::CollectionDisabled { .. } => return Err("collection disabled".into()),
    };
    store.set_collection_opt_out(
        CollectionOptOutScope {
            project_id: project,
            session: Some("session-a".to_owned()),
            source_operation: None,
            candidate_kind: Some(CandidateKind::Observation),
        },
        true,
        "user disabled later collection",
    )?;
    let basis = store.read_basis(project)?;
    let unchanged = basis.clone();
    let complete = inspect_candidate(
        &basis,
        candidate.id,
        CandidateContentAccess::AllowBoundedSummary,
        TimestampMicros::from_unix_micros(2_000),
    );
    assert!(complete.exists);
    assert_eq!(complete.health, InspectionHealth::Complete);
    assert_eq!(complete.kind, Some(CandidateKind::Observation));
    assert_eq!(complete.current_applicable_opt_out.len(), 1);
    assert_eq!(
        complete.promotion_disposition,
        Some(CandidateDisposition::PendingOrRetained)
    );
    let withheld = inspect_candidate(
        &basis,
        candidate.id,
        CandidateContentAccess::PolicyWithheld,
        TimestampMicros::from_unix_micros(2_000),
    );
    assert_eq!(withheld.health, InspectionHealth::Partial);
    assert_eq!(
        withheld.content_omission,
        Some(CandidateContentOmission::PolicyWithheld)
    );
    assert_eq!(basis, unchanged);
    assert_eq!(store.read_basis(project)?, unchanged);

    store.dismiss(project, candidate.id, "not material")?;
    store.delete_candidate(project, candidate.id, "explicit cleanup")?;
    let cleaned_basis = store.read_basis(project)?;
    let cleaned_unchanged = cleaned_basis.clone();
    let cleaned = inspect_candidate(
        &cleaned_basis,
        candidate.id,
        CandidateContentAccess::AllowBoundedSummary,
        TimestampMicros::from_unix_micros(4_000),
    );
    assert_eq!(cleaned.health, InspectionHealth::Partial);
    assert!(cleaned.content_cleaned);
    assert!(matches!(
        cleaned.promotion_disposition,
        Some(CandidateDisposition::Dismissed { ref reason, .. }) if reason == "not material"
    ));
    assert_eq!(cleaned.promotion_target, None);
    assert_eq!(cleaned.current_applicable_opt_out.len(), 1);
    assert!(matches!(
        cleaned.retention,
        Some(RetentionInspection::RetainedUntil {
            retained_until,
            expired_at_observation: true,
            ref basis,
        }) if retained_until == TimestampMicros::from_unix_micros(3_000)
            && basis == "session retention"
    ));
    assert_eq!(
        cleaned.cleanup.as_ref().map(|cleanup| cleanup.kind),
        Some(volicord_inquiry::CandidateCleanupKind::ExplicitDeletion)
    );
    assert_eq!(
        cleaned
            .cleanup
            .as_ref()
            .map(|cleanup| cleanup.basis.as_str()),
        Some("explicit cleanup")
    );
    assert_eq!(
        cleaned.cleanup.as_ref().map(|cleanup| cleanup.cleaned_at),
        Some(TimestampMicros::from_unix_micros(1_000))
    );
    assert_eq!(
        cleaned.content_omission,
        Some(CandidateContentOmission::RetentionCleaned)
    );
    assert_eq!(cleaned_basis, cleaned_unchanged);
    assert_eq!(store.read_basis(project)?, cleaned_unchanged);

    let canonical_question = volicord_context::QuestionId::from_bytes([8; 16]);
    let mut promoted_basis = cleaned_basis.clone();
    promoted_basis.candidates[0].disposition = CandidateDisposition::Promoted {
        canonical_question_id: canonical_question,
        promoted_at: TimestampMicros::from_unix_micros(2_500),
    };
    promoted_basis.candidates[0].promotion_target = Some(canonical_question);
    let promoted_cleaned = inspect_candidate(
        &promoted_basis,
        candidate.id,
        CandidateContentAccess::AllowBoundedSummary,
        TimestampMicros::from_unix_micros(4_000),
    );
    assert_eq!(promoted_cleaned.promotion_target, Some(canonical_question));
    assert!(promoted_cleaned.content_cleaned);
    assert_eq!(promoted_cleaned.cleanup, cleaned.cleanup);
    assert!(matches!(
        promoted_cleaned.promotion_disposition,
        Some(CandidateDisposition::Promoted {
            canonical_question_id,
            ..
        }) if canonical_question_id == canonical_question
    ));
    let mut unavailable_basis = cleaned_basis.clone();
    unavailable_basis.candidates[0].disposition = CandidateDisposition::PendingOrRetained;
    unavailable_basis.candidates[0].cleanup = None;
    let degraded = inspect_candidate(
        &unavailable_basis,
        candidate.id,
        CandidateContentAccess::AllowBoundedSummary,
        TimestampMicros::from_unix_micros(4_000),
    );
    assert_eq!(degraded.health, InspectionHealth::Degraded);
    assert_eq!(
        degraded.content_omission,
        Some(CandidateContentOmission::ContentUnavailable)
    );
    Ok(())
}

#[test]
fn resume_brief_is_deterministic_bounded_grounded_and_read_only(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=100).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let project = store
        .create_project(operation(101), "Recall Project")?
        .value;
    let repository = store
        .record_source(
            operation(102),
            project.id,
            source_draft(
                &project,
                SourcePayload::RepositorySnapshot {
                    revision: "snapshot-a".to_owned(),
                },
                Availability::Available,
            ),
        )?
        .value;
    let unavailable = store
        .record_source(
            operation(103),
            project.id,
            source_draft(
                &project,
                SourcePayload::File {
                    locator: "src/unavailable.rs".to_owned(),
                    snapshot: "snapshot-a".to_owned(),
                },
                Availability::Unavailable,
            ),
        )?
        .value;
    let stale = store
        .record_source(
            operation(118),
            project.id,
            source_draft(
                &project,
                SourcePayload::File {
                    locator: "src/stale.rs".to_owned(),
                    snapshot: "snapshot-old".to_owned(),
                },
                Availability::Stale,
            ),
        )?
        .value;
    let user_turn = store
        .record_source(
            operation(104),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "session-a".to_owned(),
                    turn: "turn-a".to_owned(),
                },
                actor: principal(PrincipalKind::User, "owner"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    for (operation_id, role, statement) in [
        (105, ContextItemRole::Goal, "ship reliable resumption"),
        (106, ContextItemRole::Goal, "teach the decision rationale"),
        (107, ContextItemRole::Risk, "source may be unavailable"),
        (
            108,
            ContextItemRole::KnownLimit,
            "runtime behavior is unobserved",
        ),
    ] {
        store.record_context_item(
            operation(operation_id),
            project.id,
            ContextItemDraft {
                expected_project_revision: project.revision,
                role,
                statement: statement.to_owned(),
                provenance_role: StatementProvenanceRole::AgentStatement,
                author: principal(PrincipalKind::Agent, "codex"),
                source_basis: vec![repository.id],
                applicability: ApplicabilityScope::default(),
            },
        )?;
    }
    let current_question = store
        .create_question(
            operation(109),
            project.id,
            question_draft(&project, &repository, "Current Decision?", 1),
        )?
        .value;
    let unavailable_question = store
        .create_question(
            operation(110),
            project.id,
            question_draft(&project, &unavailable, "Unavailable basis?", 2),
        )?
        .value;
    let historical_question = store
        .create_question(
            operation(111),
            project.id,
            question_draft(&project, &repository, "Historical Decision?", 3),
        )?
        .value;
    let stale_question = store
        .create_question(
            operation(119),
            project.id,
            question_draft(&project, &stale, "Stale basis?", 5),
        )?
        .value;
    let open_question = store
        .create_question(
            operation(112),
            project.id,
            question_draft(&project, &repository, "What remains open?", 4),
        )?
        .value;
    let response = |question: &volicord_context::Question, key: &str| QuestionResponseDraft {
        expected_project_revision: project.revision,
        question_id: question.id,
        question_revision: question.revision,
        user_turn_source: UserTurnSource::Existing(user_turn.id),
        displayed_alternative_keys: vec!["local".to_owned(), "remote".to_owned()],
        displayed_recommendation_key: Some("local".to_owned()),
        response: ExplicitQuestionResponse::Choice {
            alternative_key: key.to_owned(),
            user_rationale: Some("preserve continuity".to_owned()),
        },
        applicability: ApplicabilityScope {
            paths: Vec::new(),
            components: vec!["storage".to_owned()],
            work_contexts: vec!["phase-6".to_owned()],
        },
        assumptions: vec!["local-first".to_owned()],
        revisit_triggers: vec!["source changes".to_owned()],
    };
    let current_decision = store
        .record_question_response(
            operation(113),
            project.id,
            response(&current_question, "local"),
        )?
        .value
        .decision
        .ok_or("current decision missing")?;
    store.record_question_response(
        operation(114),
        project.id,
        response(&unavailable_question, "local"),
    )?;
    store.record_question_response(
        operation(120),
        project.id,
        response(&stale_question, "local"),
    )?;
    let historical = store
        .record_question_response(
            operation(115),
            project.id,
            response(&historical_question, "local"),
        )?
        .value
        .decision
        .ok_or("historical decision missing")?;
    store.supersede_decision(
        operation(116),
        project.id,
        DecisionSupersessionDraft {
            expected_project_revision: project.revision,
            previous_decision_id: historical.id,
            user_turn_source: UserTurnSource::Existing(user_turn.id),
            choice: DecisionChoice::Alternative {
                alternative_key: "remote".to_owned(),
            },
            user_rationale: Some("the semantic meaning changed".to_owned()),
            applicability: historical.applicability.clone(),
            assumptions: historical.assumptions.clone(),
            revisit_triggers: historical.revisit_triggers.clone(),
        },
    )?;
    store.record_checkpoint(
        operation(117),
        project.id,
        CheckpointDraft {
            expected_project_revision: project.revision,
            kind: CheckpointKind::Pause,
            goal: "ship reliable resumption".to_owned(),
            work_state: WorkState::Paused,
            state_change: None,
            source_basis: vec![repository.id],
            changed_source_basis: Vec::new(),
            changed_paths: Vec::new(),
            applied_decisions: vec![current_decision.id],
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
            non_goals: vec!["no viewer".to_owned()],
            open_questions: vec![volicord_context::QuestionReference {
                question_id: open_question.id,
                revision: open_question.revision,
            }],
            next_step: "answer the open Question".to_owned(),
            handoff_to: None,
        },
    )?;

    let repository_root = root.path().join("repo");
    fs::create_dir_all(repository_root.join("src"))?;
    fs::write(repository_root.join("src/lib.rs"), "pub fn recall() {}\n")?;
    let canonical = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, mut analysis) = inventory_repository(InventoryRequest::new(
        &repository_root,
        &grounding,
        repository.id,
        2_000,
    )?)?;
    analysis.capabilities[0].state = CapabilityState::Failed;
    analysis.capabilities[0].reason = Some("injected bounded capability failure".to_owned());
    let before = canonical.clone();
    let analyses = [&analysis];
    let build = || {
        build_resume_brief(RecallInputs {
            canonical: &canonical,
            analyses: &analyses,
            scope: volicord_inquiry::ApplicabilityQuery {
                project_id: project.id,
                paths: Vec::new(),
                components: vec!["storage".to_owned()],
                work_contexts: vec!["phase-6".to_owned()],
                current_assumptions: vec!["local-first".to_owned()],
                met_revisit_triggers: Vec::new(),
            },
            bound: RecallBound {
                max_items_per_section: 8,
            },
        })
    };
    let first = build();
    assert_eq!(first, build());
    assert_eq!(first.project_id, project.id);
    assert!(!first.goals_and_why.is_empty());
    assert!(first.latest_meaningful_checkpoint.is_some());
    assert_eq!(
        first.next_meaningful_step.as_deref(),
        Some("answer the open Question")
    );
    assert!(first
        .open_questions
        .iter()
        .any(|question| question.question_id == open_question.id && question.on_current_frontier));
    assert!(first
        .decisions
        .iter()
        .any(|decision| decision.state == BriefDecisionState::Current));
    assert!(first
        .decisions
        .iter()
        .any(|decision| decision.state == BriefDecisionState::UnavailableBasis));
    assert!(first
        .decisions
        .iter()
        .any(|decision| decision.state == BriefDecisionState::StaleBasis));
    assert!(first
        .decisions
        .iter()
        .any(|decision| decision.state == BriefDecisionState::Superseded));
    assert_eq!(first.snapshots[0].analysis_snapshot, analysis.identity);
    assert!(first.snapshots[0]
        .capabilities
        .iter()
        .any(|capability| capability.state == CapabilityState::Failed));
    assert!(first
        .declared_assumptions
        .contains(&"local-first".to_owned()));
    assert!(first
        .known_limits
        .contains(&"verification not run".to_owned()));
    assert!(!first.proposals.is_empty());
    assert_eq!(canonical, before);
    assert_eq!(
        store.read_canonical_basis(
            project.id,
            CanonicalReadOptions {
                include_checkpoint_history: true,
            },
        )?,
        before
    );

    let bounded = build_resume_brief(RecallInputs {
        canonical: &canonical,
        analyses: &analyses,
        scope: ApplicabilityQuery {
            project_id: project.id,
            paths: Vec::new(),
            components: vec!["storage".to_owned()],
            work_contexts: vec!["phase-6".to_owned()],
            current_assumptions: vec!["local-first".to_owned()],
            met_revisit_triggers: Vec::new(),
        },
        bound: RecallBound {
            max_items_per_section: 1,
        },
    });
    assert!(bounded.omitted_count > 0);
    assert_eq!(bounded.omitted_count, bounded.omissions.len());
    assert!(bounded.omissions.iter().any(|omission| {
        omission.reason == OmissionReason::Bound && !omission.expandable_basis.is_empty()
    }));
    Ok(())
}

#[test]
fn historical_checkpoint_remains_readable_with_non_current_source_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("historical-checkpoint.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=8).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(3_000)),
    )?;
    let project = store
        .create_project(operation(121), "Historical Checkpoint")?
        .value;
    let repository = store
        .record_source(
            operation(122),
            project.id,
            source_draft(
                &project,
                SourcePayload::RepositorySnapshot {
                    revision: "snapshot-current-at-creation".to_owned(),
                },
                Availability::Available,
            ),
        )?
        .value;
    let checkpoint = store
        .record_checkpoint(
            operation(123),
            project.id,
            CheckpointDraft {
                expected_project_revision: project.revision,
                kind: CheckpointKind::Pause,
                goal: "preserve historical work context".to_owned(),
                work_state: WorkState::Paused,
                state_change: None,
                source_basis: vec![repository.id],
                changed_source_basis: Vec::new(),
                changed_paths: Vec::new(),
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
                known_limits: Vec::new(),
                non_goals: Vec::new(),
                open_questions: Vec::new(),
                next_step: "restore current grounding before new work".to_owned(),
                handoff_to: None,
            },
        )?
        .value;
    let historical = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;

    for (freshness, availability) in [
        (SourceFreshness::Unavailable, Availability::Unavailable),
        (SourceFreshness::Stale, Availability::Stale),
    ] {
        let mut degraded = historical.clone();
        let source = degraded
            .sources
            .iter_mut()
            .find(|source| source.source.id == repository.id)
            .ok_or("historical Source basis missing")?;
        source.freshness = freshness;
        source.availability = availability;
        source.source.availability = availability;
        let brief = build_resume_brief(RecallInputs {
            canonical: &degraded,
            analyses: &[],
            scope: ApplicabilityQuery {
                project_id: project.id,
                paths: Vec::new(),
                components: Vec::new(),
                work_contexts: Vec::new(),
                current_assumptions: Vec::new(),
                met_revisit_triggers: Vec::new(),
            },
            bound: RecallBound::default(),
        });
        assert_eq!(
            brief
                .latest_meaningful_checkpoint
                .as_ref()
                .map(|value| value.id),
            Some(checkpoint.id)
        );
        assert_eq!(degraded.checkpoint_history, vec![checkpoint.clone()]);
        assert!(brief
            .used_sources
            .iter()
            .any(|basis| basis.source.id == repository.id && basis.freshness == freshness));
        assert!(brief
            .proposals
            .iter()
            .any(|proposal| proposal.source_ids.contains(&repository.id)));
    }
    assert_eq!(
        store.read_canonical_basis(
            project.id,
            CanonicalReadOptions {
                include_checkpoint_history: true,
            },
        )?,
        historical
    );
    Ok(())
}
