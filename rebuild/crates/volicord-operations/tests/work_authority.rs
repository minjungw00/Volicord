use std::fs;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, CheckpointKind, ContextItemRole,
    NonUserQuestionOutcome, OperationId, Principal, PrincipalKind, QuestionAlternative,
    QuestionResearchState, VerificationState, WorkState,
};
use volicord_inquiry::{
    bind_question_candidate_to_materiality, BatchResponseItem, CandidateCollectionMode,
    CandidateCollectionScope, CandidateContent, CandidateDraft, CandidateFreshness, CandidateKind,
    CandidateObservationBasis, CandidateOrigin, CandidateRetention, CurrentHostResponse,
    DisplayedQuestion, DuplicateAssessment, MaterialityAssessment, MaterialityStatus,
    QuestionCandidate, ResponseMapping, SubmissionOutcome,
};
use volicord_operations::{
    CommandVerificationDraft, EngineeringAlternative, EngineeringChoice,
    EngineeringChoiceDiscoveryDraft, EngineeringChoiceEvidenceState, EngineeringChoiceRelationship,
    EngineeringEffectCategory, ExplicitDelegationEvidence, ExploratoryDisposition,
    GroundedCheckpointDraft, LearningAlternativeSelection, LearningDeliberationDraft,
    LearningDeliberationState, LearningFeedbackDraft, LearningInitialResponse,
    LearningParticipation, LearningRecommendation, LearningReconsiderationDraft,
    LearningResponseDraft, LearningValueAssessment, LearningValueRevisionBasis,
    LearningValueRevisionRequest, LocalOperations, MaterialOutcomeSignal, MaterialityDimension,
    MaterialityDisposition, MaterialityReviewDraft, MaterialityReviewRevisionDraft, RuntimeLayout,
    WorkAuthorityBasis, WorkAuthorityBasisKind, WorkAuthorityDisposition, WorkAuthorityStage,
    WorkflowStage,
};

fn dimension(
    id: &str,
    disposition: MaterialityDisposition,
    kinds: Vec<WorkAuthorityBasisKind>,
    source: volicord_context::SourceId,
) -> MaterialityDimension {
    MaterialityDimension {
        dimension_id: id.to_owned(),
        discovered_choice_ids: vec![id.to_owned()],
        summary: format!("material outcome {id}"),
        affected_scope: vec!["src/lib.rs".to_owned()],
        material_consequences: vec!["changes externally observable behavior".to_owned()],
        observable_signals: vec![MaterialOutcomeSignal::PublicApiSemantics],
        disposition,
        basis: WorkAuthorityBasis {
            kinds,
            summary: "bounded repository and owner-contract evidence".to_owned(),
            source_basis: vec![source],
            contract_basis: Vec::new(),
            decision_basis: Vec::new(),
            research_basis: Vec::new(),
            explicit_delegation: None,
        },
        learning_value: volicord_operations::LearningValueAssessment::Routine {
            rationale: "normal-mode authority regression fixture".into(),
        },
    }
}

#[test]
fn learning_worthy_agent_choice_is_non_blocking_in_normal_mode_but_blocks_when_explicitly_active(
) -> Result<(), Box<dyn std::error::Error>> {
    let normal = fixture()?;
    let normal_source = normal.baseline.repository_source.identity();
    let normal_dimension =
        agent_owned_dimension("error-boundary", normal_source, deliberation_worthy());
    let normal_review = review(&normal, vec![normal_dimension])?;
    assert_eq!(
        readiness(&normal, &normal_review)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let active = fixture_with_goal(
        "Implement the error boundary. I want to learn before meaningful engineering choices.",
    )?;
    let active_source = active.baseline.repository_source.identity();
    let active_dimension =
        agent_owned_dimension("error-boundary", active_source, deliberation_worthy());
    let active_review = review_with_learning(
        &active,
        vec![engineering_choice(
            "error-boundary",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            active_source,
        )],
        vec![active_dimension.clone()],
        active_learning(&active),
    )?;
    let pending = readiness(&active, &active_review)?;
    assert_eq!(pending.stage, WorkAuthorityStage::LearningDeliberation);
    assert_eq!(
        pending.disposition,
        WorkAuthorityDisposition::LearningDeliberationPending
    );
    assert!(pending.blocking);

    let mut unsupported_routine = active_dimension;
    unsupported_routine.learning_value = LearningValueAssessment::Routine {
        rationale: "the agent selected an implementation".into(),
    };
    let rejected = active
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: active.project_id,
            review_candidate_id: active_review.review_candidate_id,
            rationale: "attempt to bypass the learning fork".into(),
            learning_participation: active_learning(&active),
            dimensions: vec![unsupported_routine],
            learning_value_revision_bases: Vec::new(),
        })
        .expect_err("agent preference cannot downgrade deliberation-worthy learning");
    assert!(rejected
        .message()
        .contains("Materiality Review revision failed"));
    let unchanged = readiness(&active, &active_review)?;
    assert_eq!(unchanged.stage, WorkAuthorityStage::LearningDeliberation);
    assert!(unchanged.blocking);
    Ok(())
}

#[test]
fn source_backed_research_can_make_a_prior_learning_fork_routine(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the error boundary. I want to learn before meaningful engineering choices.",
    )?;
    let source = fixture.baseline.repository_source.identity();
    let mut dimension = agent_owned_dimension("error-boundary", source, deliberation_worthy());
    let review = review_with_learning(
        &fixture,
        vec![engineering_choice(
            "error-boundary",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            source,
        )],
        vec![dimension.clone()],
        active_learning(&fixture),
    )?;
    dimension.learning_value = LearningValueAssessment::Routine {
        rationale: "repository evidence proves both alternatives use the same fixed boundary"
            .into(),
    };
    let rejected = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: review.review_candidate_id,
            rationale: "a user Goal Source is not repository research evidence".into(),
            learning_participation: active_learning(&fixture),
            dimensions: vec![dimension.clone()],
            learning_value_revision_bases: vec![LearningValueRevisionRequest {
                dimension_id: "error-boundary".into(),
                basis: LearningValueRevisionBasis::ResearchEvidence {
                    source_basis: vec![fixture.goal_source_id],
                    evidence_basis: vec![
                        "the original user request cannot remove a repository trade-off".into(),
                    ],
                    rationale: "reject a user turn relabeled as research".into(),
                },
            }],
        })
        .expect_err("current user-turn Source cannot be relabeled as research evidence");
    assert!(rejected
        .message()
        .contains("current non-user evidence Sources"));
    assert_eq!(
        readiness(&fixture, &review)?.stage,
        WorkAuthorityStage::LearningDeliberation
    );
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: review.review_candidate_id,
            rationale: "new repository evidence removes the prior trade-off".into(),
            learning_participation: active_learning(&fixture),
            dimensions: vec![dimension],
            learning_value_revision_bases: vec![LearningValueRevisionRequest {
                dimension_id: "error-boundary".into(),
                basis: LearningValueRevisionBasis::ResearchEvidence {
                    source_basis: vec![source],
                    evidence_basis: vec![
                        "both credible implementations share the same enforced boundary".into(),
                    ],
                    rationale: "the previously credible trade-off is no longer real".into(),
                },
            }],
        })?;
    assert_eq!(
        readiness(&fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    let persisted = fixture
        .operations
        .inspect_workflow_candidate(fixture.project_id, review.review_candidate_id)?
        .content
        .and_then(|content| content.materiality_review)
        .ok_or("Materiality Review content missing")?;
    assert_eq!(persisted.learning_value_revisions.len(), 1);
    assert!(matches!(
        persisted.learning_value_revisions[0].basis,
        LearningValueRevisionBasis::ResearchEvidence { .. }
    ));
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    Ok(())
}

#[test]
fn current_user_can_withdraw_learning_without_creating_a_decision(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the error boundary. I want to learn before meaningful engineering choices.",
    )?;
    let source = fixture.baseline.repository_source.identity();
    let mut dimension = agent_owned_dimension("error-boundary", source, deliberation_worthy());
    let review = review_with_learning(
        &fixture,
        vec![engineering_choice(
            "error-boundary",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            source,
        )],
        vec![dimension.clone()],
        active_learning(&fixture),
    )?;
    let withdrawal = fixture.operations.record_current_host_user_context(
        fixture.project_id,
        "codex".into(),
        "learning-withdrawal".into(),
        "I no longer want to deliberate this choice; proceed routinely.".into(),
        ContextItemRole::Preference,
        "I no longer want to deliberate this choice; proceed routinely.".into(),
    )?;
    dimension.learning_value = LearningValueAssessment::Routine {
        rationale: "the current user withdrew this bounded learning interaction".into(),
    };
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: review.review_candidate_id,
            rationale: "apply the exact current-user learning withdrawal".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![dimension],
            learning_value_revision_bases: vec![LearningValueRevisionRequest {
                dimension_id: "error-boundary".into(),
                basis: LearningValueRevisionBasis::CurrentUserWithdrawal {
                    user_turn_source_id: withdrawal.source_id,
                    verbatim_statement:
                        "I no longer want to deliberate this choice; proceed routinely.".into(),
                    rationale: "the user narrowed participation for this exact choice".into(),
                },
            }],
        })?;
    assert_eq!(
        readiness(&fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    Ok(())
}

#[test]
fn learning_deliberation_orders_response_before_feedback_survives_restart_and_retains_selection(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the error boundary. I want to learn before meaningful engineering choices.",
    )?;
    let source = fixture.baseline.repository_source.identity();
    let review = review_with_learning(
        &fixture,
        vec![engineering_choice(
            "error-boundary",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            source,
        )],
        vec![agent_owned_dimension(
            "error-boundary",
            source,
            deliberation_worthy(),
        )],
        active_learning(&fixture),
    )?;
    let deliberation = begin_learning(&fixture, &review, "error-boundary")?;
    assert_eq!(
        deliberation.state,
        LearningDeliberationState::AwaitingInitialResponse
    );
    let stored = fixture
        .operations
        .candidate_basis(fixture.project_id)?
        .candidates
        .into_iter()
        .find(|candidate| candidate.id == deliberation.deliberation_candidate_id)
        .ok_or("Learning Deliberation missing")?;
    let pre_response = stored
        .content
        .and_then(|content| content.learning_deliberation)
        .ok_or("Learning Deliberation content missing")?;
    assert!(pre_response.rounds.is_empty());
    let premature_feedback = fixture
        .operations
        .provide_learning_feedback(LearningFeedbackDraft {
            project_id: fixture.project_id,
            deliberation_candidate_id: deliberation.deliberation_candidate_id,
            feedback: "premature feedback".into(),
            recommendation: LearningRecommendation {
                selections: selection("error-boundary", "approach-a"),
                rationale: "premature recommendation".into(),
            },
        })
        .expect_err("feedback cannot anchor the initial response");
    assert_eq!(premature_feedback.message(), "Learning feedback failed");
    let unchanged = fixture
        .operations
        .candidate_basis(fixture.project_id)?
        .candidates
        .into_iter()
        .find(|candidate| candidate.id == deliberation.deliberation_candidate_id)
        .and_then(|candidate| candidate.content)
        .and_then(|content| content.learning_deliberation)
        .ok_or("Learning Deliberation missing after rejected feedback")?;
    assert_eq!(
        unchanged.state,
        LearningDeliberationState::AwaitingInitialResponse
    );
    assert!(unchanged.rounds.is_empty());

    let reopened = LocalOperations::new(fixture.operations.layout().clone());
    let bypass = reopened
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("Checkpoint cannot bypass pending learning deliberation");
    assert!(bypass.message().contains("work authority is not resolved"));
    assert_eq!(
        reopened
            .work_readiness(
                fixture.project_id,
                fixture.goal_id,
                fixture.baseline.identity,
                review.review_candidate_id,
                vec!["src/lib.rs".into()],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )?
            .stage,
        WorkAuthorityStage::LearningDeliberation
    );
    let response = reopened.record_learning_response(LearningResponseDraft {
        project_id: fixture.project_id,
        deliberation_candidate_id: deliberation.deliberation_candidate_id,
        host: "codex".into(),
        session: "learning-response".into(),
        user_turn: "I choose approach-a because it isolates failures.".into(),
        response: LearningInitialResponse::Select {
            selections: selection("error-boundary", "approach-a"),
        },
        user_rationale: Some("it isolates failures at the boundary".into()),
    })?;
    assert!(matches!(
        response.state,
        LearningDeliberationState::AwaitingAgentFeedback { .. }
    ));
    let feedback = reopened.provide_learning_feedback(LearningFeedbackDraft {
        project_id: fixture.project_id,
        deliberation_candidate_id: deliberation.deliberation_candidate_id,
        feedback: "The boundary isolates failures, while adding one translation layer.".into(),
        recommendation: LearningRecommendation {
            selections: selection("error-boundary", "approach-a"),
            rationale: "the containment benefit outweighs the local layer".into(),
        },
    })?;
    assert!(matches!(
        feedback.state,
        LearningDeliberationState::FeedbackProvided { .. }
    ));
    let completed = reopened.complete_learning_deliberation(
        fixture.project_id,
        deliberation.deliberation_candidate_id,
    )?;
    assert!(matches!(
        completed.state,
        LearningDeliberationState::Completed {
            ref selected_alternatives,
            ..
        } if selected_alternatives == &selection("error-boundary", "approach-a")
    ));
    assert_eq!(
        readiness(&fixture, &review)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    let learning_statement = "I learned why error-boundary uses approach-a.";
    let learning_context = fixture.operations.record_current_host_user_context(
        fixture.project_id,
        "codex".into(),
        "learning-trail".into(),
        learning_statement.into(),
        ContextItemRole::Learning,
        learning_statement.into(),
    )?;
    assert_eq!(learning_context.role, ContextItemRole::Learning);
    Ok(())
}

#[test]
fn active_learning_keeps_routine_and_user_owned_choices_on_their_existing_paths(
) -> Result<(), Box<dyn std::error::Error>> {
    let routine = fixture_with_goal("Implement it. I want to learn while we work.")?;
    let routine_source = routine.baseline.repository_source.identity();
    let routine_review = review_with_learning(
        &routine,
        vec![engineering_choice(
            "private-helper-name",
            EngineeringEffectCategory::ImplementationInternal,
            routine_source,
        )],
        vec![agent_owned_dimension(
            "private-helper-name",
            routine_source,
            LearningValueAssessment::Routine {
                rationale: "a private helper name has no transferable trade-off".into(),
            },
        )],
        active_learning(&routine),
    )?;
    assert_eq!(
        readiness(&routine, &routine_review)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let user_owned = fixture_with_goal("Implement it. I want to learn while we work.")?;
    let user_source = user_owned.baseline.repository_source.identity();
    let mut outcome = dimension(
        "public-failure-policy",
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None,
        },
        vec![WorkAuthorityBasisKind::AgentRecommendation],
        user_source,
    );
    outcome.learning_value = deliberation_worthy();
    let user_review = review_with_learning(
        &user_owned,
        vec![engineering_choice(
            "public-failure-policy",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            user_source,
        )],
        vec![outcome],
        active_learning(&user_owned),
    )?;
    let blocked = readiness(&user_owned, &user_review)?;
    assert_eq!(blocked.stage, WorkAuthorityStage::QuestionRequired);
    assert_eq!(
        blocked.disposition,
        WorkAuthorityDisposition::QuestionRequired
    );
    assert!(begin_learning(&user_owned, &user_review, "public-failure-policy").is_err());
    Ok(())
}

#[test]
fn delegate_skip_and_prototype_learning_responses_are_non_decision_transitions(
) -> Result<(), Box<dyn std::error::Error>> {
    for (response, expected_ready) in [
        (LearningInitialResponse::DelegateToAgent, true),
        (LearningInitialResponse::Skip, true),
        (
            LearningInitialResponse::RequestResearchOrPrototype {
                evidence_state: EngineeringChoiceEvidenceState::PrototypeRequired,
            },
            false,
        ),
    ] {
        let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
        let source = fixture.baseline.repository_source.identity();
        let review = review_with_learning(
            &fixture,
            vec![engineering_choice(
                "storage-strategy",
                EngineeringEffectCategory::PersistenceOrLifetime,
                source,
            )],
            vec![agent_owned_dimension(
                "storage-strategy",
                source,
                deliberation_worthy(),
            )],
            active_learning(&fixture),
        )?;
        let deliberation = begin_learning(&fixture, &review, "storage-strategy")?;
        fixture
            .operations
            .record_learning_response(LearningResponseDraft {
                project_id: fixture.project_id,
                deliberation_candidate_id: deliberation.deliberation_candidate_id,
                host: "codex".into(),
                session: "learning-terminal".into(),
                user_turn: "Use the requested learning disposition.".into(),
                response,
                user_rationale: None,
            })?;
        let state = readiness(&fixture, &review)?;
        assert_eq!(
            state.stage,
            if expected_ready {
                WorkAuthorityStage::ReadyForWork
            } else {
                WorkAuthorityStage::ResearchOrPrototype
            }
        );
        assert!(fixture
            .operations
            .canonical_basis(fixture.project_id)?
            .active_decisions
            .is_empty());
    }
    Ok(())
}

#[test]
fn reconsideration_reopens_learning_without_changing_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
    let source = fixture.baseline.repository_source.identity();
    let review = review_with_learning(
        &fixture,
        vec![engineering_choice(
            "cache-lifetime",
            EngineeringEffectCategory::PersistenceOrLifetime,
            source,
        )],
        vec![agent_owned_dimension(
            "cache-lifetime",
            source,
            deliberation_worthy(),
        )],
        active_learning(&fixture),
    )?;
    let deliberation = begin_learning(&fixture, &review, "cache-lifetime")?;
    fixture
        .operations
        .record_learning_response(LearningResponseDraft {
            project_id: fixture.project_id,
            deliberation_candidate_id: deliberation.deliberation_candidate_id,
            host: "codex".into(),
            session: "learning-choice".into(),
            user_turn: "I select approach-a.".into(),
            response: LearningInitialResponse::Select {
                selections: selection("cache-lifetime", "approach-a"),
            },
            user_rationale: Some("it is initially simpler".into()),
        })?;
    fixture
        .operations
        .provide_learning_feedback(LearningFeedbackDraft {
            project_id: fixture.project_id,
            deliberation_candidate_id: deliberation.deliberation_candidate_id,
            feedback: "The simpler lifetime may increase repeated work.".into(),
            recommendation: LearningRecommendation {
                selections: selection("cache-lifetime", "approach-b"),
                rationale: "the bounded cache amortizes repeated work".into(),
            },
        })?;
    let reconsidered =
        fixture
            .operations
            .reconsider_learning_deliberation(LearningReconsiderationDraft {
                project_id: fixture.project_id,
                deliberation_candidate_id: deliberation.deliberation_candidate_id,
                host: "codex".into(),
                session: "learning-reconsideration".into(),
                user_turn: "I want to reconsider after that feedback.".into(),
                rationale: "the repeated-work cost changes my reasoning".into(),
            })?;
    assert!(matches!(
        reconsidered.state,
        LearningDeliberationState::ReconsiderationRequested { .. }
    ));
    assert_eq!(
        readiness(&fixture, &review)?.stage,
        WorkAuthorityStage::LearningDeliberation
    );
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    Ok(())
}

fn delegation_evidence(
    fixture: &Fixture,
    dimension_id: &str,
    verbatim_statement: &str,
    affected_scope: Vec<String>,
) -> ExplicitDelegationEvidence {
    ExplicitDelegationEvidence {
        goal_context_id: fixture.goal_id,
        user_turn_source_id: fixture.goal_source_id,
        verbatim_statement: verbatim_statement.to_owned(),
        dimension_id: dimension_id.to_owned(),
        discovered_choice_ids: vec![dimension_id.to_owned()],
        affected_scope,
        material_consequences: vec!["changes externally observable behavior".to_owned()],
        effect_categories: vec![EngineeringEffectCategory::PublicApiShapeOrSemantics],
    }
}

struct Fixture {
    _temporary: tempfile::TempDir,
    operations: LocalOperations,
    repository: std::path::PathBuf,
    project_id: volicord_context::ProjectId,
    goal_id: volicord_context::ContextItemId,
    goal_source_id: volicord_context::SourceId,
    baseline: volicord_repository_intelligence::AnalysisSnapshot,
}

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    fixture_with_goal("Implement the bounded work-authority fixture.")
}

fn fixture_with_goal(goal_statement: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let temporary = tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 1 }\n",
    )?;
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    let project = operations
        .initialize_project("Work authority fixture", Some(&repository))?
        .project;
    let goal = operations.record_current_host_user_context(
        project.id,
        "codex".to_owned(),
        "work-authority-session".to_owned(),
        goal_statement.to_owned(),
        ContextItemRole::Goal,
        goal_statement.to_owned(),
    )?;
    let baseline = operations
        .analyze(project.id, Vec::new())?
        .value
        .ok_or("baseline analysis is unavailable")?
        .analysis;
    Ok(Fixture {
        _temporary: temporary,
        operations,
        repository,
        project_id: project.id,
        goal_id: goal.context_item_id,
        goal_source_id: goal.source_id,
        baseline,
    })
}

fn review(
    fixture: &Fixture,
    dimensions: Vec<MaterialityDimension>,
) -> Result<volicord_operations::MaterialityReviewOutcome, volicord_operations::Error> {
    let choices = dimensions
        .iter()
        .map(|dimension| EngineeringChoice {
            choice_id: dimension.discovered_choice_ids[0].clone(),
            summary: dimension.summary.clone(),
            affected_scope: dimension.affected_scope.clone(),
            alternatives: vec![
                EngineeringAlternative {
                    alternative_id: "approach-a".into(),
                    summary: "first credible approach".into(),
                    technical_consequences: vec!["first bounded consequence".into()],
                },
                EngineeringAlternative {
                    alternative_id: "approach-b".into(),
                    summary: "second credible approach".into(),
                    technical_consequences: vec!["second bounded consequence".into()],
                },
            ],
            technical_consequences: dimension.material_consequences.clone(),
            source_basis: dimension.basis.source_basis.clone(),
            effect_categories: vec![EngineeringEffectCategory::PublicApiShapeOrSemantics],
            relationship: EngineeringChoiceRelationship::Independent,
            evidence_state: EngineeringChoiceEvidenceState::Sufficient,
        })
        .collect();
    review_with_choices(fixture, choices, dimensions)
}

fn review_with_choices(
    fixture: &Fixture,
    choices: Vec<EngineeringChoice>,
    dimensions: Vec<MaterialityDimension>,
) -> Result<volicord_operations::MaterialityReviewOutcome, volicord_operations::Error> {
    review_with_learning(
        fixture,
        choices,
        dimensions,
        LearningParticipation::Inactive,
    )
}

fn review_with_learning(
    fixture: &Fixture,
    choices: Vec<EngineeringChoice>,
    dimensions: Vec<MaterialityDimension>,
    learning_participation: LearningParticipation,
) -> Result<volicord_operations::MaterialityReviewOutcome, volicord_operations::Error> {
    let discovery = fixture.operations.record_engineering_choice_discovery(
        EngineeringChoiceDiscoveryDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "work-authority-session".to_owned(),
            source_operation: "engineering-choice-discovery".to_owned(),
            summary: "discover meaningful technical forks before authority assessment".to_owned(),
            choices,
        },
    )?;
    fixture
        .operations
        .record_materiality_review(MaterialityReviewDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "work-authority-session".to_owned(),
            source_operation: "pre-work-review".to_owned(),
            rationale: "review every independently material outcome before ordinary work"
                .to_owned(),
            learning_participation,
            engineering_choice_discovery_candidate_id: discovery.discovery_candidate_id,
            dimensions,
        })
}

fn agent_owned_dimension(
    id: &str,
    source: volicord_context::SourceId,
    learning_value: LearningValueAssessment,
) -> MaterialityDimension {
    let mut value = dimension(
        id,
        MaterialityDisposition::AgentOwnedImplementationChoice,
        vec![WorkAuthorityBasisKind::ImplementationPreference],
        source,
    );
    value.observable_signals = Vec::new();
    value.material_consequences = vec!["changes a transferable implementation trade-off".into()];
    value.learning_value = learning_value;
    value
}

fn deliberation_worthy() -> LearningValueAssessment {
    LearningValueAssessment::DeliberationWorthy {
        rationale: "the fork is meaningful and transferable".into(),
        consequence_significance: vec!["changes failure containment and maintenance cost".into()],
        transferable_principles: vec!["separate policy from mechanism".into()],
        non_obvious_trade_offs: vec!["simpler code can reduce later observability".into()],
    }
}

fn active_learning(fixture: &Fixture) -> LearningParticipation {
    LearningParticipation::Active {
        user_turn_source_id: fixture.goal_source_id,
        verbatim_statement: "I want to learn".into(),
    }
}

fn begin_learning(
    fixture: &Fixture,
    review: &volicord_operations::MaterialityReviewOutcome,
    dimension_id: &str,
) -> Result<volicord_operations::LearningDeliberationOutcome, volicord_operations::Error> {
    fixture
        .operations
        .begin_learning_deliberation(LearningDeliberationDraft {
            project_id: fixture.project_id,
            review_candidate_id: review.review_candidate_id,
            dimension_id: dimension_id.into(),
            session: "work-authority-session".into(),
            source_operation: "pre-work-learning".into(),
            problem: format!("reason about {dimension_id} before implementation"),
            established_facts: vec!["two credible alternatives are source-grounded".into()],
        })
}

fn selection(choice_id: &str, alternative_id: &str) -> Vec<LearningAlternativeSelection> {
    vec![LearningAlternativeSelection {
        choice_id: choice_id.into(),
        alternative_id: alternative_id.into(),
    }]
}

fn engineering_choice(
    id: &str,
    effect: EngineeringEffectCategory,
    source: volicord_context::SourceId,
) -> EngineeringChoice {
    EngineeringChoice {
        choice_id: id.into(),
        summary: format!("meaningful engineering fork {id}"),
        affected_scope: vec!["src/lib.rs".into()],
        alternatives: vec![
            EngineeringAlternative {
                alternative_id: "approach-a".into(),
                summary: "first credible approach".into(),
                technical_consequences: vec!["first observable consequence".into()],
            },
            EngineeringAlternative {
                alternative_id: "approach-b".into(),
                summary: "second credible approach".into(),
                technical_consequences: vec!["second observable consequence".into()],
            },
        ],
        technical_consequences: vec!["the alternatives produce different behavior".into()],
        source_basis: vec![source],
        effect_categories: vec![effect],
        relationship: EngineeringChoiceRelationship::Independent,
        evidence_state: EngineeringChoiceEvidenceState::Sufficient,
    }
}

fn readiness(
    fixture: &Fixture,
    review: &volicord_operations::MaterialityReviewOutcome,
) -> Result<volicord_operations::WorkAuthorityResult, volicord_operations::Error> {
    fixture.operations.work_readiness(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        review.review_candidate_id,
        vec!["src/lib.rs".to_owned()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

fn checkpoint_draft(
    fixture: &Fixture,
    applied_decisions: Vec<volicord_context::DecisionId>,
) -> GroundedCheckpointDraft {
    GroundedCheckpointDraft {
        project_id: fixture.project_id,
        goal_context_id: fixture.goal_id,
        baseline_analysis_snapshot_id: fixture.baseline.identity,
        kind: CheckpointKind::Handoff,
        work_state: WorkState::Paused,
        state_change: Some("completed the bounded authority-backed work".to_owned()),
        applied_decisions,
        decision_components: Vec::new(),
        work_contexts: Vec::new(),
        met_revisit_triggers: Vec::new(),
        verification: vec![CommandVerificationDraft {
            state: VerificationState::NotRun,
            command_label: None,
            command_invocation: None,
            exit_code: None,
            termination: None,
            outcome: None,
        }],
        known_limits: Vec::new(),
        non_goals: Vec::new(),
        next_step: "resume from the grounded Checkpoint".to_owned(),
        handoff_to: Some("next session".to_owned()),
    }
}

#[test]
fn settled_contract_and_repository_fact_are_ready_without_question_and_survive_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let mut settled = dimension(
        "public-contract",
        MaterialityDisposition::SettledAuthority,
        vec![WorkAuthorityBasisKind::AcceptedContract],
        source,
    );
    settled.basis.contract_basis = vec!["rebuild/docs/design/inquiry-and-decision.md".to_owned()];
    let fact = dimension(
        "repository-fact",
        MaterialityDisposition::RepositoryOrEnvironmentFact,
        vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        source,
    );
    let recorded = review(&fixture, vec![settled, fact])?;
    let result = readiness(&fixture, &recorded)?;
    assert_eq!(result.stage, WorkAuthorityStage::ReadyForWork);
    assert_eq!(result.disposition, WorkAuthorityDisposition::ReadyForWork);
    assert!(!result.blocking);
    assert_eq!(result.satisfied_requirements.len(), 2);
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_questions
        .is_empty());
    let stored_review = fixture
        .operations
        .candidate_basis(fixture.project_id)?
        .candidates
        .into_iter()
        .find(|candidate| candidate.id == recorded.review_candidate_id)
        .and_then(|candidate| candidate.content)
        .and_then(|content| content.materiality_review)
        .ok_or("stored Materiality Review missing")?;
    assert!(matches!(
        stored_review.dimensions[0].disposition,
        MaterialityDisposition::SettledAuthority
    ));

    let reopened = LocalOperations::new(fixture.operations.layout().clone());
    let resumed = reopened.work_readiness(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        vec!["src/lib.rs".to_owned()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    assert_eq!(resumed.disposition, WorkAuthorityDisposition::ReadyForWork);
    assert_eq!(resumed.review_revision, Some(1));
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;
    let checkpoint = reopened.record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(checkpoint.changed_paths, ["src/lib.rs"]);
    Ok(())
}

#[test]
fn hidden_public_api_and_failure_choices_cannot_be_swallowed_by_one_feature_dimension(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let choices = vec![
        engineering_choice(
            "public-api-shape",
            EngineeringEffectCategory::PublicApiShapeOrSemantics,
            source,
        ),
        engineering_choice(
            "failure-semantics",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            source,
        ),
    ];
    let mut coarse = dimension(
        "requested-feature",
        MaterialityDisposition::SettledAuthority,
        vec![WorkAuthorityBasisKind::AcceptedContract],
        source,
    );
    coarse.discovered_choice_ids = vec!["public-api-shape".into(), "failure-semantics".into()];
    coarse.basis.contract_basis = vec!["the requested feature Goal".into()];
    let error = review_with_choices(&fixture, choices, vec![coarse])
        .expect_err("independent API and failure choices must remain separate");
    assert!(error
        .message()
        .contains("independent discovered choices cannot be collapsed"));
    Ok(())
}

#[test]
fn hidden_persistence_and_reload_choices_cannot_be_swallowed_by_one_feature_dimension(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let choices = vec![
        engineering_choice(
            "persistence-lifetime",
            EngineeringEffectCategory::PersistenceOrLifetime,
            source,
        ),
        engineering_choice(
            "reload-failure-semantics",
            EngineeringEffectCategory::FailureOrErrorSemantics,
            source,
        ),
    ];
    let mut coarse = dimension(
        "custom-parser-reload",
        MaterialityDisposition::SettledAuthority,
        vec![WorkAuthorityBasisKind::AcceptedContract],
        source,
    );
    coarse.discovered_choice_ids = vec![
        "persistence-lifetime".into(),
        "reload-failure-semantics".into(),
    ];
    coarse.basis.contract_basis = vec!["the custom parser reload Goal".into()];
    let error = review_with_choices(&fixture, choices, vec![coarse])
        .expect_err("independent persistence and reload semantics must remain separate");
    assert!(error
        .message()
        .contains("independent discovered choices cannot be collapsed"));
    Ok(())
}

#[test]
fn necessarily_coupled_choices_may_share_one_authority_dimension(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let mut response = engineering_choice(
        "response-shape",
        EngineeringEffectCategory::PublicApiShapeOrSemantics,
        source,
    );
    response.relationship = EngineeringChoiceRelationship::Coupled {
        choice_ids: vec!["status-code".into()],
        rationale: "the selected protocol response necessarily fixes both together".into(),
    };
    let mut status = engineering_choice(
        "status-code",
        EngineeringEffectCategory::Compatibility,
        source,
    );
    status.relationship = EngineeringChoiceRelationship::Coupled {
        choice_ids: vec!["response-shape".into()],
        rationale: "the selected protocol response necessarily fixes both together".into(),
    };
    let mut coupled = dimension(
        "protocol-response",
        MaterialityDisposition::SettledAuthority,
        vec![WorkAuthorityBasisKind::AcceptedContract],
        source,
    );
    coupled.discovered_choice_ids = vec!["response-shape".into(), "status-code".into()];
    coupled.basis.contract_basis = vec!["accepted protocol response contract".into()];
    let recorded = review_with_choices(&fixture, vec![response, status], vec![coupled])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.disposition,
        WorkAuthorityDisposition::ReadyForWork
    );
    Ok(())
}

#[test]
fn current_goal_explicit_delegation_is_ready_and_checkpoints_without_a_decision(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the bounded change; choose the internal module naming and structure.",
    )?;
    let mut delegated = dimension(
        "internal-implementation-structure",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    delegated.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "internal-implementation-structure",
        "choose the internal module naming and structure",
        vec!["src/lib.rs".to_owned()],
    ));
    let recorded = review(&fixture, vec![delegated])?;
    assert_eq!(
        fixture
            .operations
            .workflow_for_review_candidate(fixture.project_id, recorded.review_candidate_id)?
            .stage,
        WorkflowStage::ReadyForWork
    );
    let ready = readiness(&fixture, &recorded)?;
    assert_eq!(ready.stage, WorkAuthorityStage::ReadyForWork);
    assert_eq!(ready.disposition, WorkAuthorityDisposition::ReadyForWork);
    assert!(!ready.blocking);
    assert_eq!(ready.satisfied_requirements[0].decision_basis, []);
    let canonical = fixture.operations.canonical_basis(fixture.project_id)?;
    assert!(canonical.active_questions.is_empty());
    assert!(canonical.active_decisions.is_empty());

    fs::write(
        fixture.repository.join("src/lib.rs"),
        "mod internal_name { pub fn value() -> u32 { 2 } }\n",
    )?;
    let checkpoint = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(checkpoint.changed_paths, ["src/lib.rs"]);
    assert!(checkpoint.applied_decisions.is_empty());
    Ok(())
}

#[test]
fn current_task_delegation_rejects_unrelated_goal_missing_and_out_of_scope_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the bounded change; choose the internal module naming and structure.",
    )?;
    let unrelated = fixture.operations.record_current_host_user_context(
        fixture.project_id,
        "codex".to_owned(),
        "work-authority-session".to_owned(),
        "Use concise diagnostics in a different follow-up task.".to_owned(),
        ContextItemRole::Preference,
        "Use concise diagnostics in a different follow-up task.".to_owned(),
    )?;
    for (label, source, affected_scope) in [
        (
            "unrelated-turn",
            unrelated.source_id,
            vec!["src/lib.rs".to_owned()],
        ),
        (
            "goal-source-missing",
            fixture.baseline.repository_source.identity(),
            vec!["src/lib.rs".to_owned()],
        ),
        (
            "outside-work-scope",
            fixture.goal_source_id,
            vec!["public/observable-policy".to_owned()],
        ),
    ] {
        let mut delegated = dimension(
            label,
            MaterialityDisposition::DelegatedImplementationChoice,
            vec![WorkAuthorityBasisKind::ExplicitDelegation],
            source,
        );
        delegated.affected_scope = affected_scope;
        let recorded = review(&fixture, vec![delegated])?;
        assert_eq!(
            readiness(&fixture, &recorded)?.disposition,
            WorkAuthorityDisposition::ReviewInvalid,
            "{label}"
        );
    }
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    Ok(())
}

#[test]
fn current_task_delegation_rejects_nonverbatim_wrong_goal_and_excess_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the bounded change; choose the internal module naming and structure.",
    )?;
    let unrelated = fixture.operations.record_current_host_user_context(
        fixture.project_id,
        "codex".to_owned(),
        "work-authority-session".to_owned(),
        "You may choose the unrelated logging format.".to_owned(),
        ContextItemRole::Preference,
        "You may choose the unrelated logging format.".to_owned(),
    )?;

    let mut nonverbatim = dimension(
        "nonverbatim",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    nonverbatim.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "nonverbatim",
        "the user delegates every implementation choice",
        vec!["src/lib.rs".to_owned()],
    ));
    assert!(review(&fixture, vec![nonverbatim]).is_err());

    let mut wrong_turn = dimension(
        "wrong-turn",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        unrelated.source_id,
    );
    wrong_turn.basis.explicit_delegation = Some(ExplicitDelegationEvidence {
        goal_context_id: fixture.goal_id,
        user_turn_source_id: unrelated.source_id,
        verbatim_statement: "You may choose the unrelated logging format.".to_owned(),
        dimension_id: "wrong-turn".to_owned(),
        discovered_choice_ids: vec!["wrong-turn".to_owned()],
        affected_scope: vec!["src/lib.rs".to_owned()],
        material_consequences: vec!["changes externally observable behavior".to_owned()],
        effect_categories: vec![EngineeringEffectCategory::PublicApiShapeOrSemantics],
    });
    assert!(review(&fixture, vec![wrong_turn]).is_err());

    let mut excess_scope = dimension(
        "excess-scope",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    excess_scope.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "excess-scope",
        "choose the internal module naming and structure",
        vec!["src".to_owned(), "public/observable-policy".to_owned()],
    ));
    let recorded = review(&fixture, vec![excess_scope])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.disposition,
        WorkAuthorityDisposition::ReviewInvalid
    );
    Ok(())
}

#[test]
fn current_task_delegation_is_per_dimension_and_independent_of_research(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Choose the internal module naming and error type; keep public behavior unchanged.",
    )?;
    let mut naming = dimension(
        "module-naming",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    naming.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "module-naming",
        "Choose the internal module naming and error type",
        vec!["src/lib.rs".to_owned()],
    ));
    let user_owned = dimension(
        "public-failure-policy",
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None,
        },
        vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        fixture.baseline.repository_source.identity(),
    );
    let mixed = review(&fixture, vec![naming.clone(), user_owned])?;
    let blocked = readiness(&fixture, &mixed)?;
    assert_eq!(
        blocked.disposition,
        WorkAuthorityDisposition::QuestionRequired
    );
    assert_eq!(blocked.satisfied_requirements.len(), 1);
    assert_eq!(blocked.unresolved_requirements.len(), 1);

    let mut error_type = naming.clone();
    error_type.dimension_id = "internal-error-type".to_owned();
    error_type.discovered_choice_ids = vec!["internal-error-type".to_owned()];
    error_type.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "internal-error-type",
        "Choose the internal module naming and error type",
        vec!["src/lib.rs".to_owned()],
    ));
    let delegated = review(&fixture, vec![naming.clone(), error_type])?;
    assert_eq!(
        readiness(&fixture, &delegated)?.disposition,
        WorkAuthorityDisposition::ReadyForWork
    );
    assert!(naming.basis.research_basis.is_empty());

    naming
        .basis
        .kinds
        .push(WorkAuthorityBasisKind::ResearchEvidence);
    naming.basis.research_basis = vec!["independent implementation research".to_owned()];
    let researched = review(&fixture, vec![naming])?;
    assert_eq!(
        readiness(&fixture, &researched)?.disposition,
        WorkAuthorityDisposition::ReadyForWork
    );
    Ok(())
}

#[test]
fn delegation_binding_cannot_omit_or_borrow_another_material_dimension(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Choose the internal module naming and error type; keep public behavior unchanged.",
    )?;
    let mut naming = dimension(
        "module-naming",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    naming.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "module-naming",
        "Choose the internal module naming and error type",
        vec!["src/lib.rs".to_owned()],
    ));
    naming
        .basis
        .explicit_delegation
        .as_mut()
        .ok_or("delegation evidence missing")?
        .dimension_id = "public-network-default".to_owned();
    let error = review(&fixture, vec![naming])
        .expect_err("delegation of another dimension cannot settle this dimension");
    assert!(error
        .message()
        .contains("explicit delegation evidence must name the exact dimension"));
    Ok(())
}

#[test]
fn late_user_authority_correction_preserves_prospective_only_work_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let initially_agent_owned = agent_owned_dimension(
        "network-exposure-default",
        source,
        LearningValueAssessment::Routine {
            rationale: "initial authority assessment treated the choice as implementation-owned"
                .into(),
        },
    );
    let recorded = review(&fixture, vec![initially_agent_owned.clone()])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn exposure_default() -> bool { true }\n",
    )?;

    let mut corrected = initially_agent_owned;
    corrected.disposition = MaterialityDisposition::UnresolvedUserOwnedOutcome {
        resolution_decision_id: None,
    };
    corrected.basis.kinds = vec![WorkAuthorityBasisKind::NoSettlingAuthority];
    corrected.basis.summary =
        "credible exposure alternatives change an external security outcome and no exact authority settles it"
            .into();
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "correct the hidden material authority boundary".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![corrected],
            learning_value_revision_bases: Vec::new(),
        })?;
    let prospective = readiness(&fixture, &revised)?;
    assert_eq!(prospective.stage, WorkAuthorityStage::QuestionRequired);
    assert!(prospective.reason.contains("prospective"));
    let review = fixture
        .operations
        .inspect_workflow_candidate(fixture.project_id, recorded.review_candidate_id)?
        .content
        .and_then(|content| content.materiality_review)
        .ok_or("Materiality Review content missing")?;
    assert_eq!(review.late_authority_corrections.len(), 1);
    assert_eq!(
        review.late_authority_corrections[0].affected_changed_paths,
        ["src/lib.rs"]
    );
    Ok(())
}

#[test]
fn exploratory_uncertainty_loops_through_research_without_manufacturing_decision(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let mut exploratory = dimension(
        "parser-behavior",
        MaterialityDisposition::ExploratoryUncertainty {
            disposition: ExploratoryDisposition::ResearchRequired,
        },
        vec![WorkAuthorityBasisKind::ResearchEvidence],
        source,
    );
    exploratory.basis.research_basis = vec!["inspect the bounded parser behavior".to_owned()];
    let recorded = review(&fixture, vec![exploratory.clone()])?;
    let pending = readiness(&fixture, &recorded)?;
    assert_eq!(pending.stage, WorkAuthorityStage::ResearchOrPrototype);
    assert!(pending.blocking);

    exploratory.disposition = MaterialityDisposition::ExploratoryUncertainty {
        disposition: ExploratoryDisposition::ResolvedByResearch,
    };
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "bounded research resolved the implementation uncertainty".to_owned(),
            learning_participation: volicord_operations::LearningParticipation::Inactive,
            dimensions: vec![exploratory],
            learning_value_revision_bases: Vec::new(),
        })?;
    assert_eq!(revised.review_revision, 2);
    assert_eq!(
        readiness(&fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    Ok(())
}

#[test]
fn user_owned_and_hidden_material_signals_require_question_lifecycle(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let mut api = dimension(
        "api-failure-policy",
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None,
        },
        vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        source,
    );
    api.observable_signals = vec![
        MaterialOutcomeSignal::PublicApiSemantics,
        MaterialOutcomeSignal::ObservableFailurePolicy,
        MaterialOutcomeSignal::PrivacyOrExternalDisclosure,
    ];
    api.material_consequences = vec![
        "fail closed and preserve privacy".to_owned(),
        "degrade and disclose a bounded external request".to_owned(),
    ];
    let recorded = review(&fixture, vec![api])?;
    let result = readiness(&fixture, &recorded)?;
    assert_eq!(result.stage, WorkAuthorityStage::QuestionRequired);
    assert_eq!(
        result.disposition,
        WorkAuthorityDisposition::QuestionRequired
    );
    assert!(result.blocking);
    assert_eq!(result.unresolved_requirements.len(), 1);
    Ok(())
}

#[test]
fn recommendation_library_convention_and_fake_delegation_never_establish_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, disposition, kind) in [
        (
            "recommendation",
            MaterialityDisposition::SettledAuthority,
            WorkAuthorityBasisKind::AgentRecommendation,
        ),
        (
            "library-default",
            MaterialityDisposition::SettledAuthority,
            WorkAuthorityBasisKind::LibraryOrConvention,
        ),
        (
            "implementation-preference",
            MaterialityDisposition::SettledAuthority,
            WorkAuthorityBasisKind::ImplementationPreference,
        ),
        (
            "fake-delegation",
            MaterialityDisposition::DelegatedImplementationChoice,
            WorkAuthorityBasisKind::ExplicitDelegation,
        ),
    ] {
        let fixture = fixture()?;
        let source = fixture.baseline.repository_source.identity();
        let recorded = review(
            &fixture,
            vec![dimension(label, disposition, vec![kind], source)],
        )?;
        assert_eq!(
            readiness(&fixture, &recorded)?.disposition,
            WorkAuthorityDisposition::ReviewInvalid,
            "{label}"
        );
    }
    for kind in [
        WorkAuthorityBasisKind::AcceptedContract,
        WorkAuthorityBasisKind::AgentRecommendation,
        WorkAuthorityBasisKind::LibraryOrConvention,
        WorkAuthorityBasisKind::ImplementationPreference,
    ] {
        let fixture = fixture_with_goal(
            "Implement the bounded change; choose the internal module naming and structure.",
        )?;
        let mut delegated = dimension(
            "masquerading-authority",
            MaterialityDisposition::DelegatedImplementationChoice,
            vec![WorkAuthorityBasisKind::ExplicitDelegation, kind],
            fixture.goal_source_id,
        );
        delegated.basis.explicit_delegation = Some(delegation_evidence(
            &fixture,
            "masquerading-authority",
            "choose the internal module naming and structure",
            vec!["src/lib.rs".to_owned()],
        ));
        if kind == WorkAuthorityBasisKind::AcceptedContract {
            delegated.basis.contract_basis = vec!["accepted owner text".to_owned()];
        }
        let recorded = review(&fixture, vec![delegated])?;
        assert_eq!(
            readiness(&fixture, &recorded)?.disposition,
            WorkAuthorityDisposition::ReviewInvalid,
            "{kind:?}"
        );
    }
    let fixture = fixture_with_goal(
        "Implement the bounded change; choose the internal module naming and structure.",
    )?;
    let mut relabeled_contract = dimension(
        "relabeled-contract",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    relabeled_contract.basis.contract_basis = vec!["accepted owner text".to_owned()];
    relabeled_contract.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "relabeled-contract",
        "choose the internal module naming and structure",
        vec!["src/lib.rs".to_owned()],
    ));
    let recorded = review(&fixture, vec![relabeled_contract])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.disposition,
        WorkAuthorityDisposition::ReviewInvalid
    );
    Ok(())
}

#[test]
fn first_review_after_meaningful_mutation_is_rejected_and_trivial_details_do_not_explode(
) -> Result<(), Box<dyn std::error::Error>> {
    let late_fixture = fixture()?;
    let source = late_fixture.baseline.repository_source.identity();
    fs::write(
        late_fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;
    let late = review(
        &late_fixture,
        vec![dimension(
            "implementation-detail",
            MaterialityDisposition::RepositoryOrEnvironmentFact,
            vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
            source,
        )],
    )
    .expect_err("late review must not be accepted");
    assert!(late.message().contains("first Materiality Review is late"));
    let refused = late_fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&late_fixture, Vec::new()))
        .expect_err("a rejected late review cannot validate a Checkpoint");
    assert!(refused.message().contains("work authority is not resolved"));

    let clean = fixture()?;
    let source = clean.baseline.repository_source.identity();
    let recorded = review(
        &clean,
        vec![dimension(
            "bounded-task-outcome",
            MaterialityDisposition::RepositoryOrEnvironmentFact,
            vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
            source,
        )],
    )?;
    assert_eq!(
        readiness(&clean, &recorded)?.disposition,
        WorkAuthorityDisposition::ReadyForWork
    );
    assert!(clean
        .operations
        .canonical_basis(clean.project_id)?
        .active_questions
        .is_empty());
    Ok(())
}

#[test]
fn checkpoint_rejects_missing_and_unresolved_materiality_without_recording_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    let missing = fixture()?;
    let before = missing.operations.canonical_basis(missing.project_id)?;
    let error = missing
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&missing, Vec::new()))
        .expect_err("missing review must block Checkpoint");
    assert!(error.message().contains("Materiality Review is required"));
    assert!(missing
        .operations
        .canonical_basis(missing.project_id)?
        .checkpoint_history
        .is_empty());
    assert!(before.latest_checkpoint.is_none());

    let unresolved = fixture()?;
    let source = unresolved.baseline.repository_source.identity();
    review(
        &unresolved,
        vec![dimension(
            "public-default",
            MaterialityDisposition::UnresolvedUserOwnedOutcome {
                resolution_decision_id: None,
            },
            vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
            source,
        )],
    )?;
    let error = unresolved
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&unresolved, Vec::new()))
        .expect_err("unresolved owner outcome must block Checkpoint");
    assert!(error
        .message()
        .contains("material user-owned outcomes still require explicit authority"));
    assert!(unresolved
        .operations
        .canonical_basis(unresolved.project_id)?
        .checkpoint_history
        .is_empty());
    Ok(())
}

#[test]
fn user_owned_dimension_can_be_explicitly_delegated_and_reused_without_requestioning(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let user_owned = dimension(
        "failure-policy",
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None,
        },
        vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        source,
    );
    let coupled = dimension(
        "cli-exit-policy",
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None,
        },
        vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        source,
    );
    let review_outcome = review(&fixture, vec![user_owned.clone(), coupled.clone()])?;
    assert_eq!(
        readiness(&fixture, &review_outcome)?.stage,
        WorkAuthorityStage::QuestionRequired
    );
    let refused = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("unresolved user-owned work cannot produce a Checkpoint");
    assert!(refused.message().contains("work authority is not resolved"));
    let review_record = fixture
        .operations
        .candidate_basis(fixture.project_id)?
        .candidates
        .into_iter()
        .find(|candidate| candidate.id == review_outcome.review_candidate_id)
        .ok_or("review Candidate missing")?;
    let question_draft = CandidateDraft {
        project_id: fixture.project_id,
        kind: CandidateKind::QuestionCandidate,
        collection_mode: CandidateCollectionMode::ExplicitUserDirected,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "codex".to_owned(),
            },
            subsystem: "inquiry".to_owned(),
            session: Some("work-authority-session".to_owned()),
            provenance_summary: "materiality dimension Question draft".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id: fixture.project_id,
            session: Some("work-authority-session".to_owned()),
            source_operation: Some("materiality-question".to_owned()),
            candidate_kind: CandidateKind::QuestionCandidate,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source],
            analysis_snapshot: Some(fixture.baseline.identity.to_string()),
            ..CandidateObservationBasis::default()
        },
        observed_at: volicord_context::TimestampMicros::from_unix_micros(1),
        retention: CandidateRetention {
            retained_until: None,
            basis: "retain through explicit Question lifecycle".to_owned(),
        },
        content: CandidateContent {
            bounded_summary: "choose the externally observable failure policy".to_owned(),
            question: Some(QuestionCandidate {
                prompt_basis: "Which failure policy should the public API use?".to_owned(),
                known_facts: Vec::new(),
                assumptions: Vec::new(),
                uncertainty: Vec::new(),
                affected_scope: vec!["src/lib.rs".to_owned()],
                possible_prerequisites: Vec::new(),
                source_basis: vec![source],
                repository_basis: Vec::new(),
                freshness: CandidateFreshness::Current,
                duplicate_assessment: DuplicateAssessment::NoDuplicate {
                    basis: "no applicable Decision exists".to_owned(),
                },
                materiality: MaterialityAssessment {
                    status: MaterialityStatus::Material,
                    rationale: Some(
                        "public callers observe the selected failure policy".to_owned(),
                    ),
                    source_basis: vec![source],
                    assessed_by: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "codex".to_owned(),
                    }),
                    assessed_at: Some(volicord_context::TimestampMicros::from_unix_micros(1)),
                },
                presentation_order: Some(1),
                why_it_matters_now: "implementation would otherwise choose user-owned behavior"
                    .to_owned(),
                alternatives: vec![
                    QuestionAlternative {
                        key: "strict".to_owned(),
                        label: "Strict".to_owned(),
                        consequence: "return an explicit error".to_owned(),
                    },
                    QuestionAlternative {
                        key: "degraded".to_owned(),
                        label: "Degraded".to_owned(),
                        consequence: "continue with an explicit degraded result".to_owned(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("strict".to_owned()),
                    rationale: "preserves a clear failure boundary".to_owned(),
                    source_basis: vec![source],
                },
                trade_offs: vec!["availability versus strictness".to_owned()],
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["public API implementation".to_owned()],
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state: QuestionResearchState::ReadyToAsk,
            }),
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
        },
    };
    let bound =
        bind_question_candidate_to_materiality(&review_record, "failure-policy", question_draft)?;
    let bound = bind_question_candidate_to_materiality(&review_record, "cli-exit-policy", bound)?;
    let coupled_scope = bound
        .content
        .question
        .as_ref()
        .ok_or("bound Question content missing")?
        .affected_scope
        .clone();
    assert!(coupled_scope.contains(&"work-authority:failure-policy".to_owned()));
    assert!(coupled_scope.contains(&"work-authority:cli-exit-policy".to_owned()));
    let question_candidate_id = match fixture.operations.submit_candidate(bound)? {
        SubmissionOutcome::Stored(candidate) => candidate.id,
        SubmissionOutcome::CollectionDisabled { .. } => {
            return Err("explicit Question Candidate was disabled".into())
        }
    };
    let promoted = fixture
        .operations
        .promote_question_candidate(fixture.project_id, question_candidate_id)?;
    let user_response = fixture.operations.record_current_host_user_context(
        fixture.project_id,
        "codex".to_owned(),
        "work-authority-session".to_owned(),
        "Choose strict for the public failure policy.".to_owned(),
        ContextItemRole::Preference,
        "Choose strict".to_owned(),
    )?;
    let response = fixture.operations.record_inquiry_responses(
        fixture.project_id,
        vec![BatchResponseItem {
            operation_id: OperationId::from_bytes([91; 16]),
            response: CurrentHostResponse {
                project_id: fixture.project_id,
                source_id: user_response.source_id,
                host: "codex".to_owned(),
                session: "work-authority-session".to_owned(),
                turn: "Choose strict for the public failure policy.".to_owned(),
                displayed: DisplayedQuestion {
                    question_id: promoted.question_id,
                    revision: 1,
                    alternative_keys: vec!["strict".to_owned(), "degraded".to_owned()],
                    recommendation_key: Some("strict".to_owned()),
                },
                mapping: ResponseMapping::ExplicitDelegation {
                    delegate_to: "implementation-owner".to_owned(),
                    user_rationale: Some(
                        "choose within the displayed failure-policy scope".to_owned(),
                    ),
                },
                applicability: ApplicabilityScope {
                    paths: vec!["src/lib.rs".to_owned()],
                    components: Vec::new(),
                    work_contexts: Vec::new(),
                },
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        }],
    )?;
    assert!(response.all_succeeded());
    let decision_id = fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions[0]
        .decision
        .id;
    let mut resolved = user_owned;
    resolved.disposition = MaterialityDisposition::DelegatedImplementationChoice;
    resolved
        .basis
        .kinds
        .push(WorkAuthorityBasisKind::ExplicitDelegation);
    resolved.basis.decision_basis.push(decision_id);
    let mut resolved_coupled = coupled;
    resolved_coupled.disposition = MaterialityDisposition::DelegatedImplementationChoice;
    resolved_coupled
        .basis
        .kinds
        .push(WorkAuthorityBasisKind::ExplicitDelegation);
    resolved_coupled.basis.decision_basis.push(decision_id);
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: review_outcome.review_candidate_id,
            rationale: "the exact current-host response produced an applicable Decision".to_owned(),
            learning_participation: volicord_operations::LearningParticipation::Inactive,
            dimensions: vec![resolved, resolved_coupled],
            learning_value_revision_bases: Vec::new(),
        })?;
    let ready = readiness(&fixture, &revised)?;
    assert_eq!(ready.disposition, WorkAuthorityDisposition::ReadyForWork);
    assert_eq!(ready.satisfied_requirements.len(), 2);
    assert!(ready
        .satisfied_requirements
        .iter()
        .all(|requirement| requirement.decision_basis == [decision_id]));
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_questions
        .is_empty());
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> Result<u32, &'static str> { Ok(2) }\n",
    )?;
    let reopened = LocalOperations::new(fixture.operations.layout().clone());
    let missing_decision = reopened
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("Checkpoint must name the authority Decision");
    assert!(missing_decision
        .message()
        .contains("must name every Decision"));
    let checkpoint =
        reopened.record_grounded_checkpoint(checkpoint_draft(&fixture, vec![decision_id]))?;
    assert_eq!(checkpoint.applied_decisions, [decision_id]);
    assert_eq!(checkpoint.changed_paths, ["src/lib.rs"]);
    Ok(())
}
