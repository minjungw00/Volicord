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
    CommandVerificationDraft, DiscoveredAlternativeAccounting, DiscoveredAlternativeResolution,
    EngineeringAlternative, EngineeringChoice, EngineeringChoiceDiscoveryDraft,
    EngineeringChoiceEvidenceState, EngineeringChoiceRelationship, EngineeringEffectCategory,
    ExactAuthoritySufficiency, ExplicitDelegationEvidence, ExploratoryDisposition,
    GroundedCheckpointDraft, LearningAlternativeSelection, LearningDeliberationDraft,
    LearningDeliberationState, LearningFeedbackDraft, LearningInitialResponse,
    LearningParticipation, LearningRecommendation, LearningReconsiderationDraft,
    LearningResponseDraft, LearningValueAssessment, LearningValueRevisionBasis,
    LearningValueRevisionRequest, LocalOperations, MaterialBoundaryConclusion,
    MaterialBoundaryReview, MaterialOutcomeOwnershipAssessment, MaterialOutcomeSignal,
    MaterialityDimension, MaterialityDisposition, MaterialityReviewDraft,
    MaterialityReviewRevisionDraft, RuntimeLayout, WorkAuthorityBasis, WorkAuthorityBasisKind,
    WorkAuthorityDisposition, WorkAuthorityStage, WorkflowDisposition, WorkflowStage,
};

fn dimension(
    id: &str,
    disposition: MaterialityDisposition,
    kinds: Vec<WorkAuthorityBasisKind>,
    source: volicord_context::SourceId,
) -> MaterialityDimension {
    let exact_authority = matches!(
        &disposition,
        MaterialityDisposition::RepositoryOrEnvironmentFact
            | MaterialityDisposition::SettledAuthority
    )
    .then(|| ExactAuthoritySufficiency {
        covered_outcome: format!("the complete {id} material dimension"),
        unique_outcome_rationale:
            "the cited fixture authority leaves one mechanically or normatively selected outcome"
                .into(),
    });
    let alternative_accounting = match &disposition {
        MaterialityDisposition::RepositoryOrEnvironmentFact => vec![
            alternative_account(
                id,
                "approach-a",
                DiscoveredAlternativeResolution::Selected,
                source,
            ),
            alternative_account(
                id,
                "approach-b",
                DiscoveredAlternativeResolution::EliminatedByRepositoryOrEnvironmentFact,
                source,
            ),
        ],
        MaterialityDisposition::SettledAuthority
            if kinds.contains(&WorkAuthorityBasisKind::AcceptedContract) =>
        {
            vec![
                alternative_account(
                    id,
                    "approach-a",
                    DiscoveredAlternativeResolution::Selected,
                    source,
                ),
                alternative_account(
                    id,
                    "approach-b",
                    DiscoveredAlternativeResolution::EliminatedByAcceptedContract {
                        contract_reference: "fixture accepted contract".into(),
                    },
                    source,
                ),
            ]
        }
        _ => ["approach-a", "approach-b"]
            .into_iter()
            .map(|alternative_id| {
                alternative_account(
                    id,
                    alternative_id,
                    DiscoveredAlternativeResolution::Unresolved,
                    source,
                )
            })
            .collect(),
    };
    let contains_user_owned_outcome = matches!(
        disposition,
        MaterialityDisposition::SettledAuthority
            | MaterialityDisposition::DelegatedImplementationChoice
            | MaterialityDisposition::UnresolvedUserOwnedOutcome { .. }
    );
    let contract_basis = kinds
        .contains(&WorkAuthorityBasisKind::AcceptedContract)
        .then(|| "fixture accepted contract".into())
        .into_iter()
        .collect();
    MaterialityDimension {
        dimension_id: id.to_owned(),
        discovered_choice_ids: vec![id.to_owned()],
        summary: format!("material outcome {id}"),
        affected_scope: vec!["src/lib.rs".to_owned()],
        material_consequences: vec!["changes externally observable behavior".to_owned()],
        observable_signals: vec![MaterialOutcomeSignal::PublicApiSemantics],
        ownership: MaterialOutcomeOwnershipAssessment {
            materially_varying_outcomes: vec![
                "the exact externally observable behavior selected by the alternatives".into(),
            ],
            contains_user_owned_outcome,
            user_owned_outcomes: contains_user_owned_outcome
                .then(|| "the externally observable product policy".into())
                .into_iter()
                .collect(),
            rationale: if contains_user_owned_outcome {
                "the alternatives select externally observable product policy".into()
            } else {
                "the alternatives do not alter settled user-observable behavior".into()
            },
            bounded_implementation_discretion_rationale: (!contains_user_owned_outcome).then(
                || {
                    "every alternative remains inside the fixture's settled observable boundary"
                        .into()
                },
            ),
            source_basis: vec![source],
        },
        alternative_accounting,
        disposition,
        basis: WorkAuthorityBasis {
            kinds,
            summary: "bounded repository and owner-contract evidence".to_owned(),
            authority_counterfactual: "The fixture names the exact outcome and authority basis."
                .to_owned(),
            exact_authority,
            source_basis: vec![source],
            contract_basis,
            decision_basis: Vec::new(),
            research_basis: Vec::new(),
            explicit_delegation: None,
        },
        learning_value: volicord_operations::LearningValueAssessment::Routine {
            rationale: "normal-mode authority regression fixture".into(),
        },
    }
}

fn alternative_account(
    choice_id: &str,
    alternative_id: &str,
    resolution: DiscoveredAlternativeResolution,
    source: volicord_context::SourceId,
) -> DiscoveredAlternativeAccounting {
    DiscoveredAlternativeAccounting {
        choice_id: choice_id.into(),
        alternative_id: alternative_id.into(),
        resolution,
        rationale: "the fixture accounts for this exact discovered alternative".into(),
        source_basis: vec![source],
    }
}

fn unresolved_accounts_for_choices(
    choices: &[EngineeringChoice],
    source: volicord_context::SourceId,
) -> Vec<DiscoveredAlternativeAccounting> {
    choices
        .iter()
        .flat_map(|choice| {
            choice.alternatives.iter().map(move |alternative| {
                alternative_account(
                    &choice.choice_id,
                    &alternative.alternative_id,
                    DiscoveredAlternativeResolution::Unresolved,
                    source,
                )
            })
        })
        .collect()
}

fn settled_contract_accounts_for_choices(
    choices: &[EngineeringChoice],
    contract_reference: &str,
    source: volicord_context::SourceId,
) -> Vec<DiscoveredAlternativeAccounting> {
    choices
        .iter()
        .flat_map(|choice| {
            choice
                .alternatives
                .iter()
                .enumerate()
                .map(move |(index, alternative)| {
                    alternative_account(
                        &choice.choice_id,
                        &alternative.alternative_id,
                        if index == 0 {
                            DiscoveredAlternativeResolution::Selected
                        } else {
                            DiscoveredAlternativeResolution::EliminatedByAcceptedContract {
                                contract_reference: contract_reference.into(),
                            }
                        },
                        source,
                    )
                })
        })
        .collect()
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
        semantic_rationale: "The fixture names the exact outcome and authority basis.".to_owned(),
    }
}

fn delegated_dimension(fixture: &Fixture, dimension_id: &str) -> MaterialityDimension {
    let mut delegated = dimension(
        dimension_id,
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    delegated.basis.explicit_delegation = Some(delegation_evidence(
        fixture,
        dimension_id,
        "choose the bounded implementation",
        vec!["src/lib.rs".to_owned()],
    ));
    delegated
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
    let mut paths = dimensions
        .iter()
        .flat_map(|dimension| dimension.affected_scope.iter().cloned())
        .collect::<Vec<_>>();
    paths.push("src/lib.rs".into());
    let review = record_review_with_learning(fixture, choices, dimensions, learning_participation)?;
    fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        review.review_candidate_id,
        volicord_context::ApplicabilityScope {
            paths,
            components: Vec::new(),
            work_contexts: Vec::new(),
        },
    )
}

fn record_review_with_learning(
    fixture: &Fixture,
    choices: Vec<EngineeringChoice>,
    dimensions: Vec<MaterialityDimension>,
    learning_participation: LearningParticipation,
) -> Result<volicord_operations::MaterialityReviewOutcome, volicord_operations::Error> {
    let material_boundary_review =
        complete_material_boundary_review(&choices, fixture.baseline.repository_source.identity());
    let discovery = fixture.operations.record_engineering_choice_discovery(
        EngineeringChoiceDiscoveryDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "work-authority-session".to_owned(),
            source_operation: "engineering-choice-discovery".to_owned(),
            summary: "discover meaningful technical forks before authority assessment".to_owned(),
            choices,
            material_boundary_review,
        },
    )?;
    let behavioral_context_ids = match &learning_participation {
        LearningParticipation::Active {
            verbatim_statement, ..
        } => vec![
            fixture
                .operations
                .record_current_host_user_context(
                    fixture.project_id,
                    "codex".into(),
                    "work-authority-session".into(),
                    verbatim_statement.clone(),
                    volicord_context::ContextItemRole::Learning,
                    verbatim_statement.clone(),
                )?
                .context_item_id,
        ],
        LearningParticipation::Inactive => Vec::new(),
    };
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
            behavioral_context_basis: volicord_operations::BehavioralContextBasis {
                context_item_ids: behavioral_context_ids,
                completeness_rationale:
                    "all behaviorally relevant non-Goal Context is bound for this fixture".into(),
            },
            learning_participation,
            engineering_choice_discovery_candidate_id: discovery.discovery_candidate_id,
            dimensions,
        })
}

fn complete_material_boundary_review(
    choices: &[EngineeringChoice],
    source: volicord_context::SourceId,
) -> Vec<MaterialBoundaryReview> {
    EngineeringEffectCategory::ALL
        .into_iter()
        .map(|effect_category| {
            let choice_ids = choices
                .iter()
                .filter(|choice| choice.effect_categories.contains(&effect_category))
                .map(|choice| choice.choice_id.clone())
                .collect::<Vec<_>>();
            MaterialBoundaryReview {
                effect_category,
                conclusion: if choice_ids.is_empty() {
                    MaterialBoundaryConclusion::NoIndependentFork {
                        rationale:
                            "the fixture review found no separate material outcome in this category"
                                .into(),
                    }
                } else {
                    MaterialBoundaryConclusion::RepresentedByChoices { choice_ids }
                },
                source_basis: vec![source],
            }
        })
        .collect()
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
        interruption_counterfactual:
            "without participation, the requested understanding of boundary ownership would be lost"
                .into(),
        participation_scope_alignment:
            "the Goal requests reasoning about meaningful architecture choices, including this boundary"
                .into(),
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
    settled
        .basis
        .contract_basis
        .push("rebuild/docs/design/inquiry-and-decision.md".to_owned());
    let fact = dimension(
        "repository-fact",
        MaterialityDisposition::RepositoryOrEnvironmentFact,
        vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        source,
    );
    let recorded = review(&fixture, vec![settled, fact])?;
    let result = readiness(&fixture, &recorded)?;
    assert_eq!(result.stage, WorkAuthorityStage::ReadyForWork, "{result:?}");
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
    assert_eq!(resumed.review_revision, Some(2));
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;
    let checkpoint = reopened.record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(checkpoint.changed_paths, ["src/lib.rs"]);
    Ok(())
}

#[test]
fn relevant_evidence_cannot_claim_exact_authority_while_credible_alternatives_remain(
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, disposition, kinds) in [
        (
            "candidate-expiry-cleanup-trigger",
            MaterialityDisposition::SettledAuthority,
            vec![WorkAuthorityBasisKind::AcceptedContract],
        ),
        (
            "project-local-token-file-contract",
            MaterialityDisposition::RepositoryOrEnvironmentFact,
            vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        ),
    ] {
        let fixture = fixture()?;
        let mut overclaimed = dimension(
            label,
            disposition,
            kinds,
            fixture.baseline.repository_source.identity(),
        );
        if matches!(
            overclaimed.disposition,
            MaterialityDisposition::SettledAuthority
        ) {
            overclaimed.basis.contract_basis =
                vec!["a related subsystem owner constrains the design".to_owned()];
        }
        overclaimed.alternative_accounting[1].resolution =
            DiscoveredAlternativeResolution::Unresolved;

        let error = review(&fixture, vec![overclaimed])
            .expect_err("constraining evidence cannot uniquely select a material outcome");
        assert!(
            error.message().contains("credible alternative unresolved"),
            "{label}: {}",
            error.message()
        );
    }
    Ok(())
}

#[test]
fn settling_dispositions_require_explicit_exact_authority_sufficiency(
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, disposition, kinds) in [
        (
            "settled-without-coverage",
            MaterialityDisposition::SettledAuthority,
            vec![WorkAuthorityBasisKind::AcceptedContract],
        ),
        (
            "fact-without-coverage",
            MaterialityDisposition::RepositoryOrEnvironmentFact,
            vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact],
        ),
    ] {
        let fixture = fixture()?;
        let mut dimension = dimension(
            label,
            disposition,
            kinds,
            fixture.baseline.repository_source.identity(),
        );
        if matches!(
            dimension.disposition,
            MaterialityDisposition::SettledAuthority
        ) {
            dimension.basis.contract_basis = vec!["an accepted exact contract".to_owned()];
        }
        dimension.basis.exact_authority = None;

        let error = review(&fixture, vec![dimension])
            .expect_err("a settling label without exact coverage must be rejected");
        assert!(
            error.message().contains("requires exact coverage"),
            "{label}"
        );
    }
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
    coarse.alternative_accounting =
        settled_contract_accounts_for_choices(&choices, "the requested feature Goal", source);
    let error = review_with_choices(&fixture, choices, vec![coarse])
        .expect_err("independent API and failure choices must remain separate");
    assert!(error
        .message()
        .contains("independent discovered choices cannot be collapsed"));
    Ok(())
}

#[test]
fn discovery_requires_explicit_complete_material_boundary_review_with_real_choice_links(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let choices = vec![engineering_choice(
        "structured-result-shape",
        EngineeringEffectCategory::PublicApiShapeOrSemantics,
        source,
    )];
    let mut incomplete = complete_material_boundary_review(&choices, source);
    incomplete.retain(|review| {
        review.effect_category != EngineeringEffectCategory::PublicApiShapeOrSemantics
    });
    let rejected = fixture
        .operations
        .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "material-boundary-review".into(),
            source_operation: "structured-result discovery".into(),
            summary: "review public result shape independently from settled failures".into(),
            choices: choices.clone(),
            material_boundary_review: incomplete,
        })
        .expect_err("omitted public API review cannot declare discovery complete");
    assert!(rejected
        .message()
        .contains("Engineering Choice Discovery failed"));

    let mut false_negative = complete_material_boundary_review(&choices, source);
    let public = false_negative
        .iter_mut()
        .find(|review| {
            review.effect_category == EngineeringEffectCategory::PublicApiShapeOrSemantics
        })
        .ok_or("public API boundary review missing")?;
    public.conclusion = MaterialBoundaryConclusion::NoIndependentFork {
        rationale: "incorrectly collapse the open result shape into settled failure behavior"
            .into(),
    };
    let rejected = fixture
        .operations
        .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "material-boundary-review".into(),
            source_operation: "structured-result discovery".into(),
            summary: "review public result shape independently from settled failures".into(),
            choices: choices.clone(),
            material_boundary_review: false_negative,
        })
        .expect_err("a discovered public choice must be linked by the public boundary review");
    assert!(rejected
        .message()
        .contains("Engineering Choice Discovery failed"));

    let mut valid = complete_material_boundary_review(&choices, source);
    let internal = valid
        .iter_mut()
        .find(|review| review.effect_category == EngineeringEffectCategory::ImplementationInternal)
        .ok_or("implementation-internal boundary review missing")?;
    internal.conclusion = MaterialBoundaryConclusion::NoIndependentFork {
        rationale:
            "private helper naming and test fixture selection do not create independent product outcomes"
                .into(),
    };
    let accepted = fixture.operations.record_engineering_choice_discovery(
        EngineeringChoiceDiscoveryDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "material-boundary-review".into(),
            source_operation: "structured-result discovery".into(),
            summary: "surface the public result contract without fake private choices".into(),
            choices,
            material_boundary_review: valid,
        },
    )?;
    let persisted = fixture
        .operations
        .inspect_workflow_candidate(fixture.project_id, accepted.discovery_candidate_id)?;
    let discovery = persisted
        .content
        .and_then(|content| content.engineering_choice_discovery)
        .ok_or("Engineering Choice Discovery content missing")?;
    assert_eq!(discovery.choices.len(), 1);
    assert_eq!(discovery.choices[0].choice_id, "structured-result-shape");
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
    coarse.alternative_accounting =
        settled_contract_accounts_for_choices(&choices, "the custom parser reload Goal", source);
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
    let choices = vec![response, status];
    coupled.alternative_accounting = settled_contract_accounts_for_choices(
        &choices,
        "accepted protocol response contract",
        source,
    );
    let recorded = review_with_choices(&fixture, choices, vec![coupled])?;
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
fn broad_current_task_delegation_covers_discovered_child_path_at_first_checkpoint(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the serializer module; choose the bounded internal representation.",
    )?;
    let mut delegated = dimension(
        "serializer-representation",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    delegated.affected_scope = vec!["src".to_owned()];
    delegated.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "serializer-representation",
        "choose the bounded internal representation",
        vec!["src".to_owned()],
    ));
    let recorded = review(&fixture, vec![delegated])?;
    let ready = fixture.operations.work_readiness(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        vec!["src".to_owned()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    assert_eq!(ready.disposition, WorkAuthorityDisposition::ReadyForWork);

    fs::write(
        fixture.repository.join("src/serializer.rs"),
        "pub fn encode(value: u32) -> String { value.to_string() }\n",
    )?;
    let checkpoint = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(checkpoint.changed_paths, ["src/serializer.rs"]);
    Ok(())
}

#[test]
fn materiality_inspection_blocks_scope_that_checkpoint_authority_would_reject(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Implement the serializer module; choose the bounded internal representation.",
    )?;
    let reviewed_scope = vec![
        "src/serializer".to_owned(),
        "serializer-core".to_owned(),
        "checkpoint-publication".to_owned(),
    ];
    let mut delegated = dimension(
        "serializer-representation",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    delegated.affected_scope = reviewed_scope.clone();
    delegated.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "serializer-representation",
        "choose the bounded internal representation",
        reviewed_scope,
    ));
    let recorded = review(&fixture, vec![delegated])?;
    let recorded = fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        volicord_context::ApplicabilityScope {
            paths: vec!["src/serializer".into()],
            components: vec!["serializer-core".into()],
            work_contexts: vec!["checkpoint-publication".into()],
        },
    )?;

    let ready = fixture.operations.work_readiness(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        vec!["src/serializer/encode.rs".to_owned()],
        vec!["serializer-core".to_owned()],
        vec!["checkpoint-publication".to_owned()],
        Vec::new(),
    )?;
    assert_eq!(ready.disposition, WorkAuthorityDisposition::ReadyForWork);

    for (paths, components, work_contexts, expected_path, expected_component, expected_context) in [
        (
            vec!["src/transport.rs".to_owned()],
            vec!["serializer-core".to_owned()],
            vec!["checkpoint-publication".to_owned()],
            Some("src/transport.rs"),
            None,
            None,
        ),
        (
            vec!["src/serializer/encode.rs".to_owned()],
            vec!["transport-core".to_owned()],
            vec!["checkpoint-publication".to_owned()],
            None,
            Some("transport-core"),
            None,
        ),
        (
            vec!["src/serializer/encode.rs".to_owned()],
            vec!["serializer-core".to_owned()],
            vec!["release-publication".to_owned()],
            None,
            None,
            Some("release-publication"),
        ),
    ] {
        let blocked = fixture.operations.work_readiness(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            recorded.review_candidate_id,
            paths,
            components,
            work_contexts,
            Vec::new(),
        )?;
        assert_eq!(blocked.disposition, WorkAuthorityDisposition::ReviewInvalid);
        assert_eq!(
            blocked.next_action,
            Some(volicord_operations::WorkAuthorityAction::BindExecutableWorkScope)
        );
        let mismatch = blocked.scope_mismatch.expect("typed scope mismatch");
        assert_eq!(
            mismatch.uncovered_paths.first().map(String::as_str),
            expected_path
        );
        assert_eq!(
            mismatch.uncovered_components.first().map(String::as_str),
            expected_component
        );
        assert_eq!(
            mismatch.uncovered_work_contexts.first().map(String::as_str),
            expected_context
        );
    }

    fs::write(
        fixture.repository.join("src/transport.rs"),
        "pub fn send(value: u32) -> u32 { value }\n",
    )?;
    let checkpoint_error = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("Checkpoint cannot accept work outside the reviewed authority scope");
    assert!(checkpoint_error.message().contains("src/transport.rs"));
    assert!(checkpoint_error.checkpoint_scope_violation().is_some());
    Ok(())
}

#[test]
fn binds_parent_roots_before_work_and_accepts_a_multi_file_checkpoint(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal(
        "Add bounded Repository Intelligence regression coverage; choose the test structure.",
    )?;
    let semantic_scope = vec!["Repository Intelligence regression coverage".to_owned()];
    let mut delegated = dimension(
        "regression-test-structure",
        MaterialityDisposition::DelegatedImplementationChoice,
        vec![WorkAuthorityBasisKind::ExplicitDelegation],
        fixture.goal_source_id,
    );
    delegated.affected_scope = semantic_scope.clone();
    delegated.basis.explicit_delegation = Some(delegation_evidence(
        &fixture,
        "regression-test-structure",
        "choose the test structure",
        semantic_scope,
    ));
    let choices = vec![EngineeringChoice {
        choice_id: "regression-test-structure".into(),
        summary: "implementation structure".into(),
        affected_scope: delegated.affected_scope.clone(),
        alternatives: vec![
            EngineeringAlternative {
                alternative_id: "one".into(),
                summary: "one layout".into(),
                technical_consequences: delegated.material_consequences.clone(),
            },
            EngineeringAlternative {
                alternative_id: "two".into(),
                summary: "another layout".into(),
                technical_consequences: delegated.material_consequences.clone(),
            },
        ],
        technical_consequences: delegated.material_consequences.clone(),
        source_basis: delegated.basis.source_basis.clone(),
        effect_categories: vec![EngineeringEffectCategory::PublicApiShapeOrSemantics],
        relationship: EngineeringChoiceRelationship::Independent,
        evidence_state: EngineeringChoiceEvidenceState::Sufficient,
    }];
    delegated.alternative_accounting =
        unresolved_accounts_for_choices(&choices, fixture.goal_source_id);
    let recorded = record_review_with_learning(
        &fixture,
        choices,
        vec![delegated],
        LearningParticipation::Inactive,
    )?;

    let blocked = fixture
        .operations
        .workflow_for_review_candidate(fixture.project_id, recorded.review_candidate_id)?;
    assert_eq!(
        blocked.disposition,
        WorkflowDisposition::ExecutableScopeRequired
    );
    assert!(blocked.blocks_ordinary_work);
    let bound = fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        volicord_context::ApplicabilityScope {
            paths: vec!["src".into(), "tests".into(), "docs".into()],
            components: Vec::new(),
            work_contexts: Vec::new(),
        },
    )?;
    let advertised = fixture
        .operations
        .workflow_for_review_candidate(fixture.project_id, bound.review_candidate_id)?;
    assert_eq!(advertised.stage, WorkflowStage::ReadyForWork);
    assert!(!advertised.blocks_ordinary_work);

    fs::create_dir_all(fixture.repository.join("tests"))?;
    fs::create_dir_all(fixture.repository.join("docs"))?;
    fs::write(
        fixture.repository.join("src/structural.rs"),
        "pub fn bounded() -> bool { true }\n",
    )?;
    fs::write(
        fixture.repository.join("tests/structural.rs"),
        "#[test] fn bounded() { assert!(true); }\n",
    )?;
    fs::write(
        fixture.repository.join("docs/structural.md"),
        "# Bounded structural behavior\n",
    )?;

    let checkpoint = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(checkpoint.changed_paths.len(), 3);
    Ok(())
}

#[test]
fn executable_scope_expansion_cannot_retroactively_cover_changed_work(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let recorded = review(
        &fixture,
        vec![dimension(
            "bounded-implementation",
            MaterialityDisposition::AgentOwnedImplementationChoice,
            vec![WorkAuthorityBasisKind::ImplementationPreference],
            source,
        )],
    )?;
    fs::create_dir_all(fixture.repository.join("tests"))?;
    fs::write(
        fixture.repository.join("tests/late_scope.rs"),
        "#[test] fn late() { assert!(true); }\n",
    )?;

    let error = fixture
        .operations
        .bind_executable_work_scope(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            recorded.review_candidate_id,
            volicord_context::ApplicabilityScope {
                paths: vec!["src/lib.rs".into(), "tests".into()],
                components: Vec::new(),
                work_contexts: Vec::new(),
            },
        )
        .expect_err("late scope expansion cannot authorize an already-changed path");
    assert!(std::error::Error::source(&error)
        .expect("late-scope source error")
        .to_string()
        .contains("cannot retroactively authorize already-changed paths: tests/late_scope.rs"));

    let checkpoint_error = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("the original executable scope remains authoritative");
    assert!(checkpoint_error.to_string().contains("tests/late_scope.rs"));
    Ok(())
}

#[test]
fn checkpoint_reports_every_scope_violation_with_current_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let recorded = review(
        &fixture,
        vec![dimension(
            "bounded-implementation",
            MaterialityDisposition::AgentOwnedImplementationChoice,
            vec![WorkAuthorityBasisKind::ImplementationPreference],
            source,
        )],
    )?;
    fs::create_dir_all(fixture.repository.join("docs"))?;
    fs::create_dir_all(fixture.repository.join("tests"))?;
    fs::write(fixture.repository.join("docs/z.md"), "# uncovered\n")?;
    fs::write(
        fixture.repository.join("tests/a.rs"),
        "#[test] fn uncovered() {}\n",
    )?;
    let mut draft = checkpoint_draft(&fixture, Vec::new());
    draft.decision_components = vec!["transport-core".into(), "release-core".into()];
    draft.work_contexts = vec!["release".into(), "transport".into()];

    let error = fixture
        .operations
        .record_grounded_checkpoint(draft)
        .expect_err("all uncovered scope dimensions must reject together");
    let violation = error
        .checkpoint_scope_violation()
        .expect("typed Checkpoint scope violation");
    assert_eq!(
        violation.mismatch.uncovered_paths,
        ["docs/z.md", "tests/a.rs"]
    );
    assert_eq!(
        violation.mismatch.uncovered_components,
        ["release-core", "transport-core"]
    );
    assert_eq!(
        violation.mismatch.uncovered_work_contexts,
        ["release", "transport"]
    );
    assert_eq!(violation.mismatch.executable_scope.paths, ["src/lib.rs"]);
    assert_eq!(
        violation.review_candidate_id,
        Some(recorded.review_candidate_id)
    );
    assert_eq!(violation.review_revision, Some(recorded.review_revision));
    assert_eq!(
        violation.workflow.required_next_action,
        Some(volicord_operations::WorkflowAction {
            tool: "materiality_review".into(),
            action: Some("inspect".into()),
        })
    );
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

    let mut missing_counterfactual = delegated_dimension(&fixture, "missing-counterfactual");
    missing_counterfactual
        .basis
        .explicit_delegation
        .as_mut()
        .expect("delegation evidence")
        .verbatim_statement = "choose the internal module naming and structure".to_owned();
    missing_counterfactual
        .basis
        .authority_counterfactual
        .clear();
    assert!(review(&fixture, vec![missing_counterfactual]).is_err());

    let mut missing_semantic_rationale =
        delegated_dimension(&fixture, "missing-semantic-rationale");
    missing_semantic_rationale
        .basis
        .explicit_delegation
        .as_mut()
        .expect("delegation evidence")
        .verbatim_statement = "choose the internal module naming and structure".to_owned();
    missing_semantic_rationale
        .basis
        .explicit_delegation
        .as_mut()
        .expect("delegation evidence")
        .semantic_rationale
        .clear();
    assert!(review(&fixture, vec![missing_semantic_rationale]).is_err());

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
        semantic_rationale: "The fixture names the exact outcome and authority basis.".to_owned(),
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
    for account in &mut error_type.alternative_accounting {
        account.choice_id = "internal-error-type".into();
    }
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
    corrected.ownership.contains_user_owned_outcome = true;
    corrected.ownership.user_owned_outcomes = vec!["the public network exposure default".into()];
    corrected
        .ownership
        .bounded_implementation_discretion_rationale = None;
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
    assert_eq!(review.late_work_authority_revisions.len(), 1);
    assert_eq!(
        review.late_work_authority_revisions[0].affected_changed_paths,
        ["src/lib.rs"]
    );
    Ok(())
}

#[test]
fn late_delegated_to_repository_fact_revision_cannot_certify_affected_work_after_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture =
        fixture_with_goal("Implement the bounded change; choose the bounded implementation.")?;
    let delegated = delegated_dimension(&fixture, "implementation-boundary");
    let recorded = review(&fixture, vec![delegated.clone()])?;
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;

    let mut repository_fact = delegated;
    repository_fact.disposition = MaterialityDisposition::RepositoryOrEnvironmentFact;
    repository_fact.ownership.contains_user_owned_outcome = false;
    repository_fact.ownership.user_owned_outcomes.clear();
    repository_fact
        .ownership
        .bounded_implementation_discretion_rationale = Some(
        "repository evidence mechanically fixes the outcome without a product-policy choice".into(),
    );
    repository_fact.basis.kinds = vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact];
    repository_fact.basis.explicit_delegation = None;
    repository_fact.basis.summary = "current repository evidence fixes the value".into();
    repository_fact.basis.exact_authority = Some(ExactAuthoritySufficiency {
        covered_outcome: "the complete implementation-boundary dimension".into(),
        unique_outcome_rationale: "current repository evidence mechanically fixes one value".into(),
    });
    repository_fact.alternative_accounting[0].resolution =
        DiscoveredAlternativeResolution::Selected;
    repository_fact.alternative_accounting[1].resolution =
        DiscoveredAlternativeResolution::EliminatedByRepositoryOrEnvironmentFact;
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "record the current repository-fact disposition".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![repository_fact],
            learning_value_revision_bases: Vec::new(),
        })?;

    let reopened = LocalOperations::new(fixture.operations.layout().clone());
    let blocked = reopened.work_readiness(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        revised.review_candidate_id,
        vec!["src/lib.rs".into()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    assert_eq!(blocked.disposition, WorkAuthorityDisposition::ReviewInvalid);
    assert!(blocked.reason.contains("prospective"));
    let persisted = reopened
        .inspect_workflow_candidate(fixture.project_id, revised.review_candidate_id)?
        .content
        .and_then(|content| content.materiality_review)
        .ok_or("Materiality Review content missing")?;
    assert_eq!(persisted.late_work_authority_revisions.len(), 1);
    assert_eq!(
        persisted.late_work_authority_revisions[0].affected_changed_paths,
        ["src/lib.rs"]
    );
    let error = reopened
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("late repository-fact revision cannot certify earlier work");
    assert!(error.message().contains("work authority is not resolved"));
    Ok(())
}

#[test]
fn late_delegated_to_agent_owned_revision_cannot_certify_affected_work(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture =
        fixture_with_goal("Implement the bounded change; choose the bounded implementation.")?;
    let delegated = delegated_dimension(&fixture, "implementation-boundary");
    let recorded = review(&fixture, vec![delegated.clone()])?;
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;

    let mut agent_owned = delegated;
    agent_owned.disposition = MaterialityDisposition::AgentOwnedImplementationChoice;
    agent_owned.ownership.contains_user_owned_outcome = false;
    agent_owned.ownership.user_owned_outcomes.clear();
    agent_owned
        .ownership
        .bounded_implementation_discretion_rationale =
        Some("all remaining alternatives preserve the settled product behavior".into());
    agent_owned.basis.kinds = vec![WorkAuthorityBasisKind::ImplementationPreference];
    agent_owned.basis.explicit_delegation = None;
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "record bounded agent-owned implementation discretion".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![agent_owned],
            learning_value_revision_bases: Vec::new(),
        })?;

    assert_eq!(
        readiness(&fixture, &revised)?.disposition,
        WorkAuthorityDisposition::ReviewInvalid
    );
    let error = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))
        .expect_err("late agent-owned revision cannot certify earlier work");
    assert!(error.message().contains("work authority is not resolved"));
    Ok(())
}

#[test]
fn late_exploratory_resolution_cannot_certify_affected_work(
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
    exploratory.basis.research_basis = vec!["inspect the bounded parser behavior".into()];
    let recorded = review(&fixture, vec![exploratory.clone()])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.stage,
        WorkAuthorityStage::ResearchOrPrototype
    );
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;

    exploratory.disposition = MaterialityDisposition::ExploratoryUncertainty {
        disposition: ExploratoryDisposition::ResolvedByResearch,
    };
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "bounded research resolved the uncertainty".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![exploratory],
            learning_value_revision_bases: Vec::new(),
        })?;
    assert_eq!(
        readiness(&fixture, &revised)?.disposition,
        WorkAuthorityDisposition::ReviewInvalid
    );
    Ok(())
}

#[test]
fn equivalent_work_authority_revisions_before_affected_work_remain_allowed(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture =
        fixture_with_goal("Implement the bounded change; choose the bounded implementation.")?;
    let delegated = delegated_dimension(&fixture, "implementation-boundary");
    let recorded = review(&fixture, vec![delegated.clone()])?;

    let mut repository_fact = delegated;
    repository_fact.disposition = MaterialityDisposition::RepositoryOrEnvironmentFact;
    repository_fact.ownership.contains_user_owned_outcome = false;
    repository_fact.ownership.user_owned_outcomes.clear();
    repository_fact
        .ownership
        .bounded_implementation_discretion_rationale = Some(
        "repository evidence mechanically fixes the outcome without a product-policy choice".into(),
    );
    repository_fact.basis.kinds = vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact];
    repository_fact.basis.explicit_delegation = None;
    repository_fact.basis.exact_authority = Some(ExactAuthoritySufficiency {
        covered_outcome: "the complete implementation-boundary dimension".into(),
        unique_outcome_rationale: "pre-work repository evidence mechanically fixes one value"
            .into(),
    });
    repository_fact.alternative_accounting[0].resolution =
        DiscoveredAlternativeResolution::Selected;
    repository_fact.alternative_accounting[1].resolution =
        DiscoveredAlternativeResolution::EliminatedByRepositoryOrEnvironmentFact;
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "pre-work repository evidence fixed the value".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![repository_fact.clone()],
            learning_value_revision_bases: Vec::new(),
        })?;
    assert_eq!(
        readiness(&fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let mut agent_owned = repository_fact;
    agent_owned.disposition = MaterialityDisposition::AgentOwnedImplementationChoice;
    agent_owned.ownership.contains_user_owned_outcome = false;
    agent_owned.ownership.user_owned_outcomes.clear();
    agent_owned
        .ownership
        .bounded_implementation_discretion_rationale =
        Some("all remaining alternatives preserve the settled product behavior".into());
    agent_owned.basis.kinds = vec![WorkAuthorityBasisKind::ImplementationPreference];
    agent_owned.basis.exact_authority = None;
    for account in &mut agent_owned.alternative_accounting {
        account.resolution = DiscoveredAlternativeResolution::Unresolved;
    }
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "pre-work evidence leaves bounded implementation discretion".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![agent_owned.clone()],
            learning_value_revision_bases: Vec::new(),
        })?;
    assert_eq!(
        readiness(&fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let mut exploratory = agent_owned;
    exploratory.disposition = MaterialityDisposition::ExploratoryUncertainty {
        disposition: ExploratoryDisposition::ResearchRequired,
    };
    exploratory.basis.kinds = vec![WorkAuthorityBasisKind::ResearchEvidence];
    exploratory.basis.research_basis = vec!["inspect the parser behavior".into()];
    let pending = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "pre-work research remains necessary".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![exploratory.clone()],
            learning_value_revision_bases: Vec::new(),
        })?;
    assert_eq!(
        readiness(&fixture, &pending)?.stage,
        WorkAuthorityStage::ResearchOrPrototype
    );
    exploratory.disposition = MaterialityDisposition::ExploratoryUncertainty {
        disposition: ExploratoryDisposition::ResolvedByResearch,
    };
    let ready = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "pre-work research resolved the uncertainty".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![exploratory],
            learning_value_revision_bases: Vec::new(),
        })?;
    assert_eq!(
        readiness(&fixture, &ready)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    let review = fixture
        .operations
        .inspect_workflow_candidate(fixture.project_id, recorded.review_candidate_id)?
        .content
        .and_then(|content| content.materiality_review)
        .ok_or("Materiality Review content missing")?;
    assert!(review.late_work_authority_revisions.is_empty());
    Ok(())
}

#[test]
fn unrelated_paths_and_metadata_only_revisions_do_not_create_late_blockers(
) -> Result<(), Box<dyn std::error::Error>> {
    let unrelated =
        fixture_with_goal("Implement the bounded change; choose the bounded implementation.")?;
    let delegated = delegated_dimension(&unrelated, "implementation-boundary");
    let recorded = review(&unrelated, vec![delegated.clone()])?;
    fs::create_dir_all(unrelated.repository.join("docs"))?;
    fs::write(
        unrelated.repository.join("docs/notes.md"),
        "unrelated notes\n",
    )?;
    let mut agent_owned = delegated;
    agent_owned.disposition = MaterialityDisposition::AgentOwnedImplementationChoice;
    agent_owned.ownership.contains_user_owned_outcome = false;
    agent_owned.ownership.user_owned_outcomes.clear();
    agent_owned
        .ownership
        .bounded_implementation_discretion_rationale =
        Some("all remaining alternatives preserve the settled product behavior".into());
    agent_owned.basis.kinds = vec![WorkAuthorityBasisKind::ImplementationPreference];
    agent_owned.basis.explicit_delegation = None;
    let revised =
        unrelated
            .operations
            .revise_materiality_review(MaterialityReviewRevisionDraft {
                project_id: unrelated.project_id,
                review_candidate_id: recorded.review_candidate_id,
                rationale: "revise only the src/lib.rs authority meaning".into(),
                learning_participation: LearningParticipation::Inactive,
                dimensions: vec![agent_owned],
                learning_value_revision_bases: Vec::new(),
            })?;
    assert_eq!(
        readiness(&unrelated, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let metadata = fixture()?;
    let source = metadata.baseline.repository_source.identity();
    let initial = agent_owned_dimension(
        "internal-boundary",
        source,
        LearningValueAssessment::Routine {
            rationale: "bounded internal choice".into(),
        },
    );
    let recorded = review(&metadata, vec![initial.clone()])?;
    fs::write(
        metadata.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;
    let mut clarified = initial;
    clarified.summary = "clarified bounded internal boundary description".into();
    clarified.basis.summary = "clarified evidence description without changing authority".into();
    let revised =
        metadata
            .operations
            .revise_materiality_review(MaterialityReviewRevisionDraft {
                project_id: metadata.project_id,
                review_candidate_id: recorded.review_candidate_id,
                rationale: "clarify review prose after work".into(),
                learning_participation: LearningParticipation::Inactive,
                dimensions: vec![clarified],
                learning_value_revision_bases: Vec::new(),
            })?;
    assert_eq!(
        readiness(&metadata, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    let review = metadata
        .operations
        .inspect_workflow_candidate(metadata.project_id, recorded.review_candidate_id)?
        .content
        .and_then(|content| content.materiality_review)
        .ok_or("Materiality Review content missing")?;
    assert!(review.late_work_authority_revisions.is_empty());
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
    assert_eq!(revised.review_revision, 3);
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
fn discovery_evidence_precedes_delegated_or_agent_owned_implementation_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    let delegated_fixture = fixture_with_goal(
        "Investigate the behavior, then choose the bounded implementation for me.",
    )?;
    let source = delegated_fixture.baseline.repository_source.identity();
    let mut prototype_choice = engineering_choice(
        "parser-shape",
        EngineeringEffectCategory::PublicApiShapeOrSemantics,
        source,
    );
    prototype_choice.evidence_state = EngineeringChoiceEvidenceState::PrototypeRequired;
    let delegated = delegated_dimension(&delegated_fixture, "parser-shape");
    let recorded =
        review_with_choices(&delegated_fixture, vec![prototype_choice], vec![delegated])?;
    let blocked = readiness(&delegated_fixture, &recorded)?;
    assert_eq!(blocked.stage, WorkAuthorityStage::ResearchOrPrototype);
    assert_eq!(
        blocked.disposition,
        WorkAuthorityDisposition::ResearchRequired
    );
    assert!(blocked.reason.contains("research or prototype evidence"));
    let persisted = delegated_fixture
        .operations
        .inspect_workflow_candidate(delegated_fixture.project_id, recorded.review_candidate_id)?
        .content
        .and_then(|content| content.materiality_review)
        .ok_or("Materiality Review content missing")?;
    assert!(matches!(
        persisted.dimensions[0].disposition,
        MaterialityDisposition::DelegatedImplementationChoice
    ));

    let agent_fixture = fixture()?;
    let source = agent_fixture.baseline.repository_source.identity();
    let mut research_choice = engineering_choice(
        "internal-cache",
        EngineeringEffectCategory::ImplementationInternal,
        source,
    );
    research_choice.evidence_state = EngineeringChoiceEvidenceState::ResearchRequired;
    let agent_owned = agent_owned_dimension(
        "internal-cache",
        source,
        LearningValueAssessment::Routine {
            rationale: "routine implementation choice".into(),
        },
    );
    let recorded = review_with_choices(&agent_fixture, vec![research_choice], vec![agent_owned])?;
    assert_eq!(
        readiness(&agent_fixture, &recorded)?.stage,
        WorkAuthorityStage::ResearchOrPrototype
    );
    Ok(())
}

#[test]
fn completed_discovery_evidence_restores_prospective_authority_or_reveals_a_question(
) -> Result<(), Box<dyn std::error::Error>> {
    let delegated_fixture = fixture_with_goal(
        "Investigate the behavior, then choose the bounded implementation for me.",
    )?;
    let source = delegated_fixture.baseline.repository_source.identity();
    let mut prototype_choice = engineering_choice(
        "parser-shape",
        EngineeringEffectCategory::PublicApiShapeOrSemantics,
        source,
    );
    prototype_choice.evidence_state = EngineeringChoiceEvidenceState::PrototypeRequired;
    let mut delegated = delegated_dimension(&delegated_fixture, "parser-shape");
    let recorded = review_with_choices(
        &delegated_fixture,
        vec![prototype_choice],
        vec![delegated.clone()],
    )?;
    delegated
        .basis
        .kinds
        .push(WorkAuthorityBasisKind::PrototypeEvidence);
    delegated.basis.research_basis =
        vec!["the bounded prototype establishes both viable parser result shapes".into()];
    let revised =
        delegated_fixture
            .operations
            .revise_materiality_review(MaterialityReviewRevisionDraft {
                project_id: delegated_fixture.project_id,
                review_candidate_id: recorded.review_candidate_id,
                rationale: "prototype evidence now makes the delegated selection actionable".into(),
                learning_participation: LearningParticipation::Inactive,
                dimensions: vec![delegated],
                learning_value_revision_bases: Vec::new(),
            })?;
    assert_eq!(
        readiness(&delegated_fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let user_fixture = fixture()?;
    let source = user_fixture.baseline.repository_source.identity();
    let mut research_choice = engineering_choice(
        "public-result-contract",
        EngineeringEffectCategory::PublicApiShapeOrSemantics,
        source,
    );
    research_choice.evidence_state = EngineeringChoiceEvidenceState::ResearchRequired;
    let mut exploratory = dimension(
        "public-result-contract",
        MaterialityDisposition::ExploratoryUncertainty {
            disposition: ExploratoryDisposition::ResearchRequired,
        },
        vec![WorkAuthorityBasisKind::ResearchEvidence],
        source,
    );
    exploratory.basis.research_basis = vec!["inspect current public callers".into()];
    let recorded = review_with_choices(
        &user_fixture,
        vec![research_choice],
        vec![exploratory.clone()],
    )?;
    assert_eq!(
        readiness(&user_fixture, &recorded)?.stage,
        WorkAuthorityStage::ResearchOrPrototype
    );
    exploratory.disposition = MaterialityDisposition::UnresolvedUserOwnedOutcome {
        resolution_decision_id: None,
    };
    exploratory.ownership.contains_user_owned_outcome = true;
    exploratory.ownership.user_owned_outcomes = vec!["the public result contract".into()];
    exploratory
        .ownership
        .bounded_implementation_discretion_rationale = None;
    exploratory.basis.kinds = vec![
        WorkAuthorityBasisKind::NoSettlingAuthority,
        WorkAuthorityBasisKind::ResearchEvidence,
    ];
    exploratory.basis.research_basis =
        vec!["repository research confirms two public result contracts remain viable".into()];
    let revised =
        user_fixture
            .operations
            .revise_materiality_review(MaterialityReviewRevisionDraft {
                project_id: user_fixture.project_id,
                review_candidate_id: recorded.review_candidate_id,
                rationale: "research reveals an unresolved public contract policy".into(),
                learning_participation: LearningParticipation::Inactive,
                dimensions: vec![exploratory],
                learning_value_revision_bases: Vec::new(),
            })?;
    assert_eq!(
        readiness(&user_fixture, &revised)?.stage,
        WorkAuthorityStage::QuestionRequired
    );
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
fn public_path_exclusion_policy_cannot_escape_through_agent_owned_disposition(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let mut path_policy = agent_owned_dimension(
        "public-path-exclusion-policy",
        source,
        LearningValueAssessment::Routine {
            rationale: "normal mode does not add a learning interruption".into(),
        },
    );
    path_policy.observable_signals = vec![
        MaterialOutcomeSignal::PublicApiSemantics,
        MaterialOutcomeSignal::UserVisibleDefault,
        MaterialOutcomeSignal::MaintenanceOrSupportPolicy,
    ];
    path_policy.material_consequences = vec![
        "prefix semantics exclude descendants".into(),
        "glob semantics expose a public pattern language".into(),
        "exact-path semantics retain descendants by default".into(),
    ];
    path_policy.ownership = MaterialOutcomeOwnershipAssessment {
        materially_varying_outcomes: vec![
            "which repository paths callers can include or exclude".into(),
            "the compatibility lifetime of the public matching syntax".into(),
        ],
        contains_user_owned_outcome: true,
        user_owned_outcomes: vec!["the public path-exclusion contract and default".into()],
        rationale: "each credible alternative changes caller-observable product policy".into(),
        bounded_implementation_discretion_rationale: None,
        source_basis: vec![source],
    };
    assert!(review(&fixture, vec![path_policy.clone()]).is_err());

    path_policy.disposition = MaterialityDisposition::UnresolvedUserOwnedOutcome {
        resolution_decision_id: None,
    };
    path_policy.basis.kinds = vec![WorkAuthorityBasisKind::NoSettlingAuthority];
    let recorded = review(&fixture, vec![path_policy])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.disposition,
        WorkAuthorityDisposition::QuestionRequired
    );
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
        );
        match recorded {
            Ok(recorded) => assert_eq!(
                readiness(&fixture, &recorded)?.disposition,
                WorkAuthorityDisposition::ReviewInvalid,
                "{label}"
            ),
            Err(error) => assert!(
                error.message().contains("settling authority")
                    || error.message().contains("exact fact")
                    || error.message().contains("explicit delegation"),
                "{label}: {}",
                error.message()
            ),
        }
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
