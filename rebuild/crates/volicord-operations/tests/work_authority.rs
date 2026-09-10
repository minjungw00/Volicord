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
    CommandVerificationDraft, CoupledArtifactAssessment, CoupledArtifactCategory,
    CoupledArtifactDisposition, CoupledArtifactReview, DiscoveredAlternativeAccounting,
    DiscoveredAlternativeResolution, EngineeringAlternative, EngineeringChoice,
    EngineeringChoiceDiscoveryDraft, EngineeringChoiceEvidenceState, EngineeringChoiceRelationship,
    EngineeringEffectCategory, ExactAuthoritySufficiency, ExplicitDelegationEvidence,
    ExploratoryDisposition, GroundedCheckpointDraft, LearningAlternativeSelection,
    LearningDeliberationDraft, LearningDeliberationState, LearningFeedbackDraft,
    LearningInitialResponse, LearningParticipation, LearningRecommendation,
    LearningReconsiderationDraft, LearningResponseDraft, LearningValueAssessment,
    LearningValueRevisionBasis, LearningValueRevisionRequest, LocalOperations,
    MaterialBoundaryConclusion, MaterialBoundaryReview, MaterialOutcomeOwnershipAssessment,
    MaterialOutcomeSignal, MaterialityDimension, MaterialityDisposition, MaterialityReviewDraft,
    MaterialityReviewRevisionDraft, RuntimeLayout, WorkAuthorityBasis, WorkAuthorityBasisKind,
    WorkAuthorityDisposition, WorkAuthorityStage, WorkflowDisposition, WorkflowStage,
};

fn coupled_artifact_review(paths: &[&str]) -> CoupledArtifactReview {
    categorized_coupled_artifact_review(&[(CoupledArtifactCategory::Implementation, paths)])
}

fn categorized_coupled_artifact_review(
    included: &[(CoupledArtifactCategory, &[&str])],
) -> CoupledArtifactReview {
    let categories = [
        CoupledArtifactCategory::Implementation,
        CoupledArtifactCategory::FocusedTests,
        CoupledArtifactCategory::PublicOrInternalDocumentation,
        CoupledArtifactCategory::ChangelogOrReleaseNotes,
        CoupledArtifactCategory::SchemaSnapshotOrGeneratedArtifact,
        CoupledArtifactCategory::OtherRepositoryOwnedArtifact,
    ];
    CoupledArtifactReview {
        assessments: categories
            .into_iter()
            .map(|category| CoupledArtifactAssessment {
                category,
                disposition: included
                    .iter()
                    .find(|(included_category, _)| *included_category == category)
                    .map_or(
                        CoupledArtifactDisposition::NoCoupledArtifact,
                        |(_, paths)| CoupledArtifactDisposition::Included {
                            repository_paths: paths.iter().map(|path| (*path).to_owned()).collect(),
                        },
                    ),
                basis_summary: "fixture repository inspection accounts for this category".into(),
            })
            .collect(),
        materiality_closure: volicord_inquiry::PreWriteMaterialityClosure::NoNewMaterialOutcome {
            commitments: vec![volicord_inquiry::PlannedCommitment {
                temporal_effect: volicord_inquiry::PlannedTemporalEffect::NoTemporalChange { rationale: "This fixture commitment preserves temporal behavior and makes no timestamp or lifetime selection.".into() },
                commitment_id: "fixture-private-preservation".into(),
                description: "Private fixture change preserves every current reviewed material outcome".into(),
                repository_paths: included.iter().flat_map(|(_, paths)| paths.iter().map(|p| (*p).to_owned())).collect(),
                outcome_binding: volicord_inquiry::PlannedOutcomeBinding::PrivateEquivalent {
                    equivalence_rationale: "The fixture changes implementation privately while preserving the complete current server-bound outcome and authority graph".into(),
                },
            }],
            rationale: "fixture scope introduces no material outcome beyond the current dimensions"
                .into(),
        },
    }
}

fn authority_evidence(
    source: volicord_context::SourceId,
    contract: bool,
) -> volicord_operations::AuthoritySourceEvidence {
    volicord_operations::AuthoritySourceEvidence {
        source_id: source,
        role: if contract { volicord_operations::AuthoritySourceRole::AcceptedContract { contract_reference: "fixture accepted contract".into() } }
            else { volicord_operations::AuthoritySourceRole::UniqueMechanicalFact },
        rationale: "The maintained fixture source explicitly requires this exact outcome; alternative accounting states how each other outcome violates that requirement.".into(),
    }
}

fn discretion_proof(
    id: &str,
    alternatives: &[&str],
    source: volicord_context::SourceId,
) -> Vec<volicord_operations::ImplementationDiscretionCounterfactual> {
    alternatives.iter().map(|alternative| volicord_operations::ImplementationDiscretionCounterfactual {
        choice_id: id.into(), alternative_id: (*alternative).into(), externally_observable: false,
        observation_rationale: "The fixture alternatives preserve caller results; only private organization differs.".into(),
        source_id: source,
        source_supported_boundary: "The current fixture source defines the same result for either internal structure; private organization is unconstrained.".into(),
    }).collect()
}

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
        source_evidence: vec![authority_evidence(
            source,
            kinds.contains(&WorkAuthorityBasisKind::AcceptedContract),
        )],
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
        learning_authority: volicord_inquiry::LearningAuthorityAssessment::Inactive,
        dimension_id: id.to_owned(),
        discovered_choice_ids: vec![id.to_owned()],
        summary: format!("material outcome {id}"),
        affected_scope: vec!["src/lib.rs".to_owned()],
        material_consequences: vec!["changes externally observable behavior".to_owned()],
        observable_signals: vec![MaterialOutcomeSignal::PublicApiSemantics],
        ownership: MaterialOutcomeOwnershipAssessment {
            discretion_counterfactuals: if contains_user_owned_outcome {
                Vec::new()
            } else {
                discretion_proof(id, &["approach-a", "approach-b"], source)
            },
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
fn successor_review_cannot_reset_learning_value_or_participation(
) -> Result<(), Box<dyn std::error::Error>> {
    for (inactive, completed) in [(false, false), (true, false), (false, true), (true, true)] {
        let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
        let source = fixture.baseline.repository_source.identity();
        let choices = vec![engineering_choice(
            "stable-choice",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        )];
        let dimension = agent_owned_dimension("stable-choice", source, deliberation_worthy());
        let first = review_with_learning(
            &fixture,
            choices.clone(),
            vec![dimension.clone()],
            active_learning(&fixture),
        )?;
        if completed {
            complete_learning_selection(&fixture, &first, "stable-choice")?;
        }
        let mut successor = dimension;
        if !inactive {
            successor.learning_value = LearningValueAssessment::Routine {
                rationale: "Already selected and completed deliberation; do not interrupt again"
                    .into(),
            };
        }
        let rejected = record_review_with_learning(
            &fixture,
            choices,
            vec![successor],
            if inactive {
                LearningParticipation::Inactive
            } else {
                active_learning(&fixture)
            },
        );
        assert!(
            rejected.is_err(),
            "equivalent rediscovery cannot reset learning (inactive={inactive})"
        );
    }
    Ok(())
}

#[test]
fn successor_review_reuses_terminal_learning_without_decision_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
    let source = fixture.baseline.repository_source.identity();
    let choices = vec![engineering_choice(
        "stable-choice",
        EngineeringEffectCategory::ImplementationInternal,
        source,
    )];
    let dimension = agent_owned_dimension("stable-choice", source, deliberation_worthy());
    let first = review_with_learning(
        &fixture,
        choices.clone(),
        vec![dimension.clone()],
        active_learning(&fixture),
    )?;
    complete_learning_selection(&fixture, &first, "stable-choice")?;
    let satisfied = readiness(&fixture, &first)?;
    let second = review_with_learning(
        &fixture,
        choices,
        vec![dimension],
        active_learning(&fixture),
    )?;
    let continued = readiness(&fixture, &second)?;
    assert_eq!(continued.stage, WorkAuthorityStage::ReadyForWork);
    assert_eq!(
        continued.learning_deliberation_candidate_ids,
        satisfied.learning_deliberation_candidate_ids
    );
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_decisions
        .is_empty());
    Ok(())
}

#[test]
fn successor_learning_requires_current_typed_meaning_but_allows_coupled_artifact_expansion(
) -> Result<(), Box<dyn std::error::Error>> {
    for change in [
        "scope",
        "alternative",
        "consequence",
        "outcome",
        "choice",
        "learning",
    ] {
        let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
        let source = fixture.baseline.repository_source.identity();
        let mut choices = vec![engineering_choice(
            "stable-choice",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        )];
        let mut dimension = agent_owned_dimension("stable-choice", source, deliberation_worthy());
        let first = review_with_learning(
            &fixture,
            choices.clone(),
            vec![dimension.clone()],
            active_learning(&fixture),
        )?;
        complete_learning_selection(&fixture, &first, "stable-choice")?;
        match change {
            "scope" => {
                dimension.affected_scope.push("tests/learning.rs".into());
                choices[0].affected_scope.push("tests/learning.rs".into());
            }
            "alternative" => {
                let mut alternative = choices[0].alternatives[1].clone();
                alternative.alternative_id = "approach-c".into();
                choices[0].alternatives.push(alternative);
                dimension.alternative_accounting =
                    unresolved_accounts_for_choices(&choices, source);
                dimension.ownership.discretion_counterfactuals = discretion_proof(
                    "stable-choice",
                    &["approach-a", "approach-b", "approach-c"],
                    source,
                );
            }
            "consequence" => choices[0].alternatives[0]
                .technical_consequences
                .push("a newly material allocation trade-off".into()),
            "learning" => {
                if let LearningValueAssessment::DeliberationWorthy {
                    transferable_principles,
                    ..
                } = &mut dimension.learning_value
                {
                    transferable_principles.push("A distinct requested learning principle".into());
                }
            }
            "outcome" => dimension
                .ownership
                .materially_varying_outcomes
                .push("a distinct resource lifetime outcome".into()),
            _ => {
                choices[0] = engineering_choice(
                    "new-choice",
                    EngineeringEffectCategory::ImplementationInternal,
                    source,
                );
                dimension = agent_owned_dimension("new-choice", source, deliberation_worthy());
                dimension.dimension_id = "stable-choice".into();
            }
        }
        if change != "scope" && change != "learning" {
            let mut routine = dimension.clone();
            routine.learning_value = LearningValueAssessment::Routine {
                rationale: "The distinct choice has independently routine learning value.".into(),
            };
            let unrelated = review_with_learning(
                &fixture,
                choices.clone(),
                vec![routine],
                active_learning(&fixture),
            )?;
            assert_eq!(
                readiness(&fixture, &unrelated)?.stage,
                WorkAuthorityStage::ReadyForWork
            );
        }
        let second = review_with_learning(
            &fixture,
            choices.clone(),
            vec![dimension.clone()],
            active_learning(&fixture),
        )?;
        let state = readiness(&fixture, &second)?;
        assert_eq!(
            state.stage,
            if change == "scope" {
                WorkAuthorityStage::ReadyForWork
            } else {
                WorkAuthorityStage::LearningDeliberation
            },
            "{change}"
        );
        if change != "scope" {
            // Unrelated learning meaning is free to be independently assessed as routine.
            dimension.learning_value = LearningValueAssessment::Routine {
                rationale: "The distinct learning meaning is routine.".into(),
            };
            // Revise the current worthy review first is still required; creating it
            // made this NEW meaning worthy, so it cannot reset either.
            assert!(record_review_with_learning(
                &fixture,
                choices,
                vec![dimension],
                active_learning(&fixture)
            )
            .is_err());
        }
        assert!(fixture
            .operations
            .canonical_basis(fixture.project_id)?
            .active_decisions
            .is_empty());
    }
    Ok(())
}

#[test]
fn successor_learning_pending_and_reconsidered_branches_override_older_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    for reconsider in [false, true] {
        let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
        let source = fixture.baseline.repository_source.identity();
        let choices = vec![engineering_choice(
            "stable-choice",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        )];
        let dimension = agent_owned_dimension("stable-choice", source, deliberation_worthy());
        let first = review_with_learning(
            &fixture,
            choices.clone(),
            vec![dimension.clone()],
            active_learning(&fixture),
        )?;
        complete_learning_selection(&fixture, &first, "stable-choice")?;
        let second = review_with_learning(
            &fixture,
            choices.clone(),
            vec![dimension.clone()],
            active_learning(&fixture),
        )?;
        if reconsider {
            let learning_id = readiness(&fixture, &first)?.learning_deliberation_candidate_ids[0];
            fixture
                .operations
                .reconsider_learning_deliberation(LearningReconsiderationDraft {
                    project_id: fixture.project_id,
                    deliberation_candidate_id: learning_id,
                    host: "codex".into(),
                    session: "reconsider".into(),
                    user_turn: "Reopen this choice after considering the feedback.".into(),
                    rationale: "I want to reconsider the trade-off.".into(),
                })?;
        } else {
            begin_learning(&fixture, &second, "stable-choice")?;
        }
        let third = review_with_learning(
            &fixture,
            choices,
            vec![dimension],
            active_learning(&fixture),
        )?;
        assert_eq!(
            readiness(&fixture, &third)?.stage,
            WorkAuthorityStage::LearningDeliberation
        );
    }
    Ok(())
}

#[test]
fn successor_review_inherits_only_supported_effective_downgrade(
) -> Result<(), Box<dyn std::error::Error>> {
    for evidence in ["research", "prototype", "withdrawal"] {
        let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
        let source = fixture.baseline.repository_source.identity();
        let choices = vec![engineering_choice(
            "stable-choice",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        )];
        let mut dimension = agent_owned_dimension("stable-choice", source, deliberation_worthy());
        let first = review_with_learning(
            &fixture,
            choices.clone(),
            vec![dimension.clone()],
            active_learning(&fixture),
        )?;
        let unsupported =
            fixture
                .operations
                .revise_materiality_review(MaterialityReviewRevisionDraft {
                    project_id: fixture.project_id,
                    review_candidate_id: first.review_candidate_id,
                    rationale: "Avoid another learning interruption".into(),
                    learning_participation: LearningParticipation::Inactive,
                    dimensions: vec![dimension.clone()],
                    learning_value_revision_bases: vec![],
                });
        assert!(unsupported.is_err());
        let basis = match evidence {
            "research" => LearningValueRevisionBasis::ResearchEvidence {
                source_basis: vec![source],
                evidence_basis: vec!["The repository eliminates the credible trade-off.".into()],
                rationale: "Current research resolves the uncertainty.".into(),
            },
            "prototype" => LearningValueRevisionBasis::PrototypeEvidence {
                source_basis: vec![source],
                evidence_basis: vec![
                    "The source-backed prototype resolves the uncertain trade-off.".into(),
                ],
                rationale: "Current prototype evidence makes this routine.".into(),
            },
            _ => {
                let withdrawal = fixture.operations.record_current_host_user_context(
                    fixture.project_id,
                    "codex".into(),
                    "withdraw".into(),
                    "Withdraw learning for this choice.".into(),
                    ContextItemRole::Preference,
                    "Withdraw learning for this choice.".into(),
                )?;
                LearningValueRevisionBasis::CurrentUserWithdrawal {
                    user_turn_source_id: withdrawal.source_id,
                    verbatim_statement: "Withdraw learning for this choice.".into(),
                    rationale: "Exact current user withdrawal.".into(),
                }
            }
        };
        let participation = if evidence == "withdrawal" {
            LearningParticipation::Inactive
        } else {
            active_learning(&fixture)
        };
        assess_learning_authority(&mut dimension);
        dimension.learning_value = LearningValueAssessment::Routine {
            rationale:
                "Supported current evidence or withdrawal supersedes the learning assessment."
                    .into(),
        };
        fixture
            .operations
            .revise_materiality_review(MaterialityReviewRevisionDraft {
                project_id: fixture.project_id,
                review_candidate_id: first.review_candidate_id,
                rationale: "Revise the prior review through the supported owner path.".into(),
                learning_participation: participation.clone(),
                dimensions: vec![dimension.clone()],
                learning_value_revision_bases: vec![LearningValueRevisionRequest {
                    dimension_id: "stable-choice".into(),
                    basis,
                }],
            })?;
        let second = review_with_learning(&fixture, choices, vec![dimension], participation)?;
        assert_eq!(
            readiness(&fixture, &second)?.stage,
            WorkAuthorityStage::ReadyForWork,
            "{evidence}"
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
fn successor_learning_fails_closed_without_matching_project_goal_baseline_and_retained_history(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal("Implement it. I want to learn while we work.")?;
    let other = fixture_with_goal("Another bounded learning Goal.")?;
    let source = fixture.baseline.repository_source.identity();
    let choices = vec![engineering_choice(
        "stable-choice",
        EngineeringEffectCategory::ImplementationInternal,
        source,
    )];
    let dimension = agent_owned_dimension("stable-choice", source, deliberation_worthy());
    let first = review_with_learning(
        &fixture,
        choices.clone(),
        vec![dimension.clone()],
        active_learning(&fixture),
    )?;
    complete_learning_selection(&fixture, &first, "stable-choice")?;
    let second = review_with_learning(
        &fixture,
        choices,
        vec![dimension],
        active_learning(&fixture),
    )?;
    let basis = fixture.operations.candidate_basis(fixture.project_id)?;
    let current = basis
        .candidates
        .iter()
        .find(|c| c.id == second.review_candidate_id)
        .expect("current review");
    let discovery_id = current
        .content
        .as_ref()
        .expect("content")
        .materiality_review
        .as_ref()
        .expect("review")
        .engineering_choice_discovery_candidate_id;
    let discovery = basis
        .candidates
        .iter()
        .find(|c| c.id == discovery_id)
        .expect("discovery");
    let canonical = fixture.operations.canonical_basis(fixture.project_id)?;
    for missing in [
        "project",
        "goal",
        "baseline",
        "review",
        "discovery",
        "choices",
    ] {
        let mut candidates = basis.candidates.clone();
        if missing == "review" {
            candidates.retain(|c| c.id != first.review_candidate_id);
        } else if missing == "discovery" {
            candidates.retain(|c| {
                c.kind != CandidateKind::EngineeringChoiceDiscovery || c.id == discovery_id
            });
        } else {
            let candidate = candidates
                .iter_mut()
                .find(|c| c.kind == CandidateKind::LearningDeliberation)
                .expect("learning");
            let learning = candidate
                .content
                .as_mut()
                .expect("content")
                .learning_deliberation
                .as_mut()
                .expect("learning");
            match missing {
                "project" => candidate.project_id = other.project_id,
                "goal" => learning.goal_context_id = other.goal_id,
                "baseline" => learning.baseline_analysis_snapshot_id = other.baseline.identity,
                _ => learning.choices.clear(),
            }
        }
        let result = volicord_inquiry::evaluate_work_authority(
            &canonical,
            volicord_inquiry::WorkAuthorityCandidateBasis {
                review: Some(current),
                discovery: Some(discovery),
                learning_deliberations: &candidates,
            },
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            &volicord_inquiry::ApplicabilityQuery {
                project_id: fixture.project_id,
                paths: vec!["src/lib.rs".into()],
                components: vec![],
                work_contexts: vec![],
                current_assumptions: vec![],
                met_revisit_triggers: vec![],
            },
        );
        assert_eq!(
            result.stage,
            WorkAuthorityStage::LearningDeliberation,
            "{missing}"
        );
        assert!(
            result.learning_deliberation_candidate_ids.is_empty(),
            "{missing}"
        );
    }
    Ok(())
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
    assess_learning_authority(&mut dimension);
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
    bind_current_review_scope(&fixture, &revised)?;
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
    assess_learning_authority(&mut dimension);
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
    bind_current_review_scope(&fixture, &revised)?;
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

fn complete_learning_selection(
    fixture: &Fixture,
    review: &volicord_operations::MaterialityReviewOutcome,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let learning = begin_learning(fixture, review, id)?;
    fixture
        .operations
        .record_learning_response(LearningResponseDraft {
            project_id: fixture.project_id,
            deliberation_candidate_id: learning.deliberation_candidate_id,
            host: "codex".into(),
            session: "learning-selection".into(),
            user_turn: "For learning I select approach-a after comparing the representation costs."
                .into(),
            response: LearningInitialResponse::Select {
                selections: selection(id, "approach-a"),
            },
            user_rationale: Some(
                "The representation makes the invariant easier to explain.".into(),
            ),
        })?;
    fixture.operations.provide_learning_feedback(LearningFeedbackDraft {
        project_id: fixture.project_id, deliberation_candidate_id: learning.deliberation_candidate_id,
        feedback: "Both representations preserve the externally fixed behavior; the selected structure centralizes provenance at an allocation cost.".into(),
        recommendation: LearningRecommendation { selections: selection(id, "approach-a"), rationale: "The bounded implementation trade-off favors explicit provenance.".into() },
    })?;
    fixture
        .operations
        .complete_learning_deliberation(fixture.project_id, learning.deliberation_candidate_id)?;
    Ok(())
}

#[test]
fn provenance_representation_learning_selection_never_becomes_product_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal("Preserve externally fixed/default behavior. I want to learn. Help me compare a tagged provenance record with a side table and think through the representation for learning.")?;
    let source = fixture.baseline.repository_source.identity();
    let mut choice = engineering_choice(
        "provenance-representation",
        EngineeringEffectCategory::ImplementationInternal,
        source,
    );
    choice.alternatives[0].summary = "A tagged record stores provenance beside the value".into();
    choice.alternatives[1].summary = "A side table stores provenance separately".into();
    let dimension =
        agent_owned_dimension("provenance-representation", source, deliberation_worthy());
    let review = review_with_learning(
        &fixture,
        vec![choice],
        vec![dimension],
        active_learning(&fixture),
    )?;
    let retained = fixture
        .operations
        .candidate_basis(fixture.project_id)?
        .candidates
        .into_iter()
        .find(|c| c.id == review.review_candidate_id)
        .ok_or("missing review")?;
    let current = retained
        .content
        .as_ref()
        .and_then(|c| c.materiality_review.as_ref())
        .ok_or("missing content")?;
    let assessed = current.dimensions[0].clone();
    assert!(matches!(
        assessed.learning_authority,
        volicord_inquiry::LearningAuthorityAssessment::Assessed {
            independent_user_authority: false,
            ..
        }
    ));
    for mutation in 0..6 {
        let mut invalid = assessed.clone();
        match mutation {
            0 => {
                invalid.learning_authority = volicord_inquiry::LearningAuthorityAssessment::Inactive
            }
            1 => {
                if let volicord_inquiry::LearningAuthorityAssessment::Assessed {
                    independent_user_authority,
                    ..
                } = &mut invalid.learning_authority
                {
                    *independent_user_authority = true;
                }
            }
            2 => {
                if let volicord_inquiry::LearningAuthorityAssessment::Assessed {
                    choice_ids, ..
                } = &mut invalid.learning_authority
                {
                    choice_ids[0] = "different-choice".into();
                }
            }
            3 => {
                if let volicord_inquiry::LearningAuthorityAssessment::Assessed {
                    material_outcomes,
                    ..
                } = &mut invalid.learning_authority
                {
                    material_outcomes[0] = "different-outcome".into();
                }
            }
            4 => {
                if let volicord_inquiry::LearningAuthorityAssessment::Assessed {
                    source_basis,
                    ..
                } = &mut invalid.learning_authority
                {
                    source_basis.clear();
                }
            }
            _ => {
                invalid.disposition = MaterialityDisposition::UnresolvedUserOwnedOutcome {
                    resolution_decision_id: None,
                };
                invalid.ownership.contains_user_owned_outcome = true;
                invalid.ownership.user_owned_outcomes = vec!["learning selection".into()];
            }
        }
        assert!(
            fixture
                .operations
                .revise_materiality_review(MaterialityReviewRevisionDraft {
                    project_id: fixture.project_id,
                    review_candidate_id: review.review_candidate_id,
                    rationale: "An educational request cannot manufacture product ownership".into(),
                    learning_participation: active_learning(&fixture),
                    dimensions: vec![invalid],
                    learning_value_revision_bases: vec![],
                })
                .is_err(),
            "invalid learning assessment {mutation}"
        );
    }
    complete_learning_selection(&fixture, &review, "provenance-representation")?;
    let reopened = LocalOperations::new(fixture.operations.layout().clone());
    assert_eq!(
        readiness(&fixture, &review)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    let canonical = reopened.canonical_basis(fixture.project_id)?;
    assert!(canonical.active_decisions.is_empty());
    assert!(canonical.active_questions.is_empty());
    assert!(canonical.terminal_question_history.is_empty());
    assert!(reopened
        .candidate_basis(fixture.project_id)?
        .candidates
        .iter()
        .any(|c| c
            .content
            .as_ref()
            .and_then(|c| c.learning_deliberation.as_ref())
            .is_some_and(|l| matches!(l.state, LearningDeliberationState::Completed { .. }))));
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
    fixture_with_repository(goal_statement, "pub fn value() -> u32 { 1 }\n")
}

fn fixture_with_repository(
    goal_statement: &str,
    source: &str,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let temporary = tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(repository.join("src/lib.rs"), source)?;
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
                    material_decomposition: volicord_inquiry::MaterialDecomposition::MateriallyAtomic { rationale: "The fixture Source bounds this alternative to its stated outcome; no further product choice remains.".into(), residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![], fixed_outcome: "The fixture alternative's stated observable consequence".into(), credible_implementations: vec!["Direct implementation preserving the stated consequence".into(), "Private helper implementation preserving the same consequence".into()], remaining_material_outcomes: vec![], source_basis: dimension.basis.source_basis.clone() } },
                    alternative_id: "approach-a".into(),
                    summary: "first credible approach".into(),
                    technical_consequences: vec!["first bounded consequence".into()],
                },
                EngineeringAlternative {
                    material_decomposition: volicord_inquiry::MaterialDecomposition::MateriallyAtomic { rationale: "The fixture Source bounds this alternative to its stated outcome; no further product choice remains.".into(), residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![], fixed_outcome: "The fixture alternative's stated observable consequence".into(), credible_implementations: vec!["Direct implementation preserving the stated consequence".into(), "Private helper implementation preserving the same consequence".into()], remaining_material_outcomes: vec![], source_basis: dimension.basis.source_basis.clone() } },
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
    let artifact_review =
        coupled_artifact_review(&paths.iter().map(String::as_str).collect::<Vec<_>>());
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
        artifact_review,
    )
}

fn assess_learning_authority(dimension: &mut MaterialityDimension) {
    dimension.learning_authority = volicord_inquiry::LearningAuthorityAssessment::Assessed {
        independent_user_authority: dimension.ownership.contains_user_owned_outcome,
        choice_ids: dimension.discovered_choice_ids.clone(),
        material_outcomes: dimension.ownership.materially_varying_outcomes.clone(),
        rationale: "Without the educational request, the cited fixture contract grants the same product ownership or private discretion described in this dimension's ownership assessment.".into(),
        source_basis: dimension.ownership.source_basis.clone(),
    };
}

fn record_review_with_learning(
    fixture: &Fixture,
    choices: Vec<EngineeringChoice>,
    mut dimensions: Vec<MaterialityDimension>,
    learning_participation: LearningParticipation,
) -> Result<volicord_operations::MaterialityReviewOutcome, volicord_operations::Error> {
    if matches!(learning_participation, LearningParticipation::Active { .. }) {
        for dimension in &mut dimensions {
            assess_learning_authority(dimension);
        }
    }
    let material_boundary_review =
        complete_material_boundary_review(&choices, fixture.baseline.repository_source.identity());
    let discovery = fixture.operations.record_engineering_choice_discovery(
        EngineeringChoiceDiscoveryDraft {
            interaction_review: outside_interactions(fixture.baseline.repository_source.identity()),
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
                reviewed_outcomes: vec![format!(
                    "Fixture observable behavior within {effect_category:?}"
                )],
                effect_category,
                conclusion: if choice_ids.is_empty() {
                    MaterialBoundaryConclusion::NoIndependentFork {
                        basis: volicord_inquiry::NoIndependentForkBasis::OutsideAffectedScope,
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
                    material_decomposition: volicord_inquiry::MaterialDecomposition::MateriallyAtomic { rationale: "The fixture Source bounds this alternative to its stated outcome; no further product choice remains.".into(), residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![], fixed_outcome: "The fixture alternative's stated observable consequence".into(), credible_implementations: vec!["Direct implementation preserving the stated consequence".into(), "Private helper implementation preserving the same consequence".into()], remaining_material_outcomes: vec![], source_basis: vec![source] } },
                alternative_id: "approach-a".into(),
                summary: "first credible approach".into(),
                technical_consequences: vec!["first observable consequence".into()],
            },
            EngineeringAlternative {
                    material_decomposition: volicord_inquiry::MaterialDecomposition::MateriallyAtomic { rationale: "The fixture Source bounds this alternative to its stated outcome; no further product choice remains.".into(), residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![], fixed_outcome: "The fixture alternative's stated observable consequence".into(), credible_implementations: vec!["Direct implementation preserving the stated consequence".into(), "Private helper implementation preserving the same consequence".into()], remaining_material_outcomes: vec![], source_basis: vec![source] } },
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
        verification_basis: volicord_inquiry::CheckpointVerificationBasis::OrdinaryChange,
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
            interaction_review: outside_interactions(fixture.baseline.repository_source.identity()),
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
        basis: volicord_inquiry::NoIndependentForkBasis::OutsideAffectedScope,
        rationale: "incorrectly collapse the open result shape into settled failure behavior"
            .into(),
    };
    let rejected = fixture
        .operations
        .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
            interaction_review: outside_interactions(fixture.baseline.repository_source.identity()),
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
                        basis: volicord_inquiry::NoIndependentForkBasis::OutsideAffectedScope,
        rationale:
            "private helper naming and test fixture selection do not create independent product outcomes"
                .into(),
    };
    let accepted = fixture.operations.record_engineering_choice_discovery(
        EngineeringChoiceDiscoveryDraft {
            interaction_review: outside_interactions(fixture.baseline.repository_source.identity()),
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
    coupled
        .basis
        .exact_authority
        .as_mut()
        .ok_or("authority missing")?
        .source_evidence[0]
        .role = volicord_operations::AuthoritySourceRole::AcceptedContract {
        contract_reference: "accepted protocol response contract".into(),
    };
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
        coupled_artifact_review(&["src/serializer"]),
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
        let workflow = fixture.operations.workflow_for_work_basis(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            paths.clone(),
            components.clone(),
            work_contexts.clone(),
            Vec::new(),
        )?;
        assert!(workflow.blocks_ordinary_work);
        let next = workflow
            .required_next_action
            .ok_or("scope inspection action missing")?;
        assert_eq!(next.tool, "materiality_review");
        assert_eq!(next.action.as_deref(), Some("inspect"));
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
fn binds_predictable_coupled_changelog_before_first_write() -> Result<(), Box<dyn std::error::Error>>
{
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
                    material_decomposition: volicord_inquiry::MaterialDecomposition::MateriallyAtomic { rationale: "The fixture Source bounds this alternative to its stated outcome; no further product choice remains.".into(), residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![], fixed_outcome: "The fixture alternative's stated observable consequence".into(), credible_implementations: vec!["Direct implementation preserving the stated consequence".into(), "Private helper implementation preserving the same consequence".into()], remaining_material_outcomes: vec![], source_basis: delegated.basis.source_basis.clone() } },
                alternative_id: "one".into(),
                summary: "one layout".into(),
                technical_consequences: delegated.material_consequences.clone(),
            },
            EngineeringAlternative {
                    material_decomposition: volicord_inquiry::MaterialDecomposition::MateriallyAtomic { rationale: "The fixture Source bounds this alternative to its stated outcome; no further product choice remains.".into(), residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![], fixed_outcome: "The fixture alternative's stated observable consequence".into(), credible_implementations: vec!["Direct implementation preserving the stated consequence".into(), "Private helper implementation preserving the same consequence".into()], remaining_material_outcomes: vec![], source_basis: delegated.basis.source_basis.clone() } },
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
            paths: vec![
                "src".into(),
                "tests".into(),
                "docs".into(),
                "CHANGES.rst".into(),
            ],
            components: Vec::new(),
            work_contexts: Vec::new(),
        },
        categorized_coupled_artifact_review(&[
            (CoupledArtifactCategory::Implementation, &["src"]),
            (CoupledArtifactCategory::FocusedTests, &["tests"]),
            (
                CoupledArtifactCategory::PublicOrInternalDocumentation,
                &["docs"],
            ),
            (
                CoupledArtifactCategory::ChangelogOrReleaseNotes,
                &["CHANGES.rst"],
            ),
        ]),
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
    fs::write(
        fixture.repository.join("CHANGES.rst"),
        "Coupled structural behavior\n",
    )?;

    let checkpoint = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(checkpoint.changed_paths.len(), 4);
    Ok(())
}

#[test]
fn later_coupled_artifact_can_be_added_prospectively_before_its_first_write(
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

    let workflow = |paths: Vec<String>| {
        fixture.operations.workflow_for_work_basis(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            paths,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    };
    let covered = workflow(vec!["src/lib.rs".into()])?;
    assert!(!covered.blocks_ordinary_work);
    assert!(covered
        .reason
        .contains("ready_for_work covers only current authority and bound executable scope"));
    // Continuation begins with valid covered work, then discovers a coupled path.
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;
    assert!(!workflow(vec!["src/lib.rs".into()])?.blocks_ordinary_work);
    let expanded_paths = vec!["src/lib.rs".into(), "tests/prospective.rs".into()];
    let blocked = workflow(expanded_paths.clone())?;
    assert!(blocked.blocks_ordinary_work);
    let next = blocked
        .required_next_action
        .ok_or("scope inspection action missing")?;
    assert_eq!(next.tool, "materiality_review");
    assert_eq!(next.action.as_deref(), Some("inspect"));
    assert!(!fixture.repository.join("tests/prospective.rs").exists());

    let rebound = fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        ApplicabilityScope {
            paths: vec!["src/lib.rs".into(), "tests".into()],
            components: Vec::new(),
            work_contexts: Vec::new(),
        },
        categorized_coupled_artifact_review(&[
            (CoupledArtifactCategory::Implementation, &["src/lib.rs"]),
            (CoupledArtifactCategory::FocusedTests, &["tests"]),
        ]),
    )?;
    assert!(rebound.review_revision > recorded.review_revision);
    assert!(!workflow(expanded_paths.clone())?.blocks_ordinary_work);

    fs::create_dir_all(fixture.repository.join("tests"))?;
    fs::write(
        fixture.repository.join("tests/prospective.rs"),
        "#[test] fn prospective() { assert!(true); }\n",
    )?;
    assert!(!workflow(expanded_paths)?.blocks_ordinary_work);
    let checkpoint = fixture
        .operations
        .record_grounded_checkpoint(checkpoint_draft(&fixture, Vec::new()))?;
    assert_eq!(
        checkpoint.changed_paths,
        ["src/lib.rs", "tests/prospective.rs"]
    );
    Ok(())
}

#[test]
fn continuation_rechecks_changed_authority_and_retains_pre_work_baseline(
) -> Result<(), Box<dyn std::error::Error>> {
    for changed_before_revision in [false, true] {
        let fixture =
            fixture_with_goal("Implement the bounded change; choose the bounded implementation.")?;
        let original = review(&fixture, vec![delegated_dimension(&fixture, "bounded")])?;
        let current = || {
            fixture
                .operations
                .workflow_for_review_candidate(fixture.project_id, original.review_candidate_id)
        };
        assert!(!current()?.blocks_ordinary_work);
        if changed_before_revision {
            fs::write(
                fixture.repository.join("src/lib.rs"),
                "pub fn value() -> u32 { 2 }\n",
            )?;
        }
        let revised =
            fixture
                .operations
                .revise_materiality_review(MaterialityReviewRevisionDraft {
                    project_id: fixture.project_id,
                    review_candidate_id: original.review_candidate_id,
                    rationale:
                        "Current evidence changes the authority basis to bounded private discretion"
                            .into(),
                    learning_participation: LearningParticipation::Inactive,
                    dimensions: vec![agent_owned_dimension(
                        "bounded",
                        fixture.baseline.repository_source.identity(),
                        LearningValueAssessment::Routine {
                            rationale: "private equivalent implementation".into(),
                        },
                    )],
                    learning_value_revision_bases: vec![],
                })?;
        assert!(current()?.blocks_ordinary_work);
        if !changed_before_revision {
            let next = current()?
                .required_next_action
                .ok_or("current inspection action missing")?;
            assert_eq!(next.tool, "materiality_review");
            assert_eq!(next.action.as_deref(), Some("inspect"));
            bind_current_review_scope(&fixture, &revised)?;
            assert!(!current()?.blocks_ordinary_work);
            fs::write(
                fixture.repository.join("src/lib.rs"),
                "pub fn value() -> u32 { 2 }\n",
            )?;
        }

        // Repository analysis remains useful after work, but its identity cannot
        // replace the original review's pre-work authority for that work.
        let later = fixture
            .operations
            .analyze(fixture.project_id, vec![])?
            .value
            .ok_or("post-work analysis missing")?
            .analysis;
        assert_ne!(later.identity, fixture.baseline.identity);
        let substituted = fixture.operations.work_readiness(
            fixture.project_id,
            fixture.goal_id,
            later.identity,
            revised.review_candidate_id,
            vec!["src/lib.rs".into()],
            vec![],
            vec![],
            vec![],
        )?;
        assert!(substituted.blocking);
        assert!(substituted.reason.contains("baseline is stale"));
        let mut rebased_checkpoint = checkpoint_draft(&fixture, vec![]);
        rebased_checkpoint.baseline_analysis_snapshot_id = later.identity;
        assert!(fixture
            .operations
            .record_grounded_checkpoint(rebased_checkpoint)
            .is_err());
        let restarted = LocalOperations::new(fixture.operations.layout().clone());
        let workflow = restarted
            .workflow_for_review_candidate(fixture.project_id, revised.review_candidate_id)?;
        assert_eq!(workflow.blocks_ordinary_work, changed_before_revision);
        assert!(workflow
            .satisfied_basis_identities
            .iter()
            .any(|basis| basis.kind == "baseline_analysis_snapshot"
                && basis.identity == fixture.baseline.identity.to_string()));
        let checkpoint = restarted.record_grounded_checkpoint(checkpoint_draft(&fixture, vec![]));
        if changed_before_revision {
            assert!(workflow.reason.contains("cannot certify the earlier work"));
            assert!(checkpoint.is_err());
            let choice = engineering_choice(
                "bounded",
                EngineeringEffectCategory::ImplementationInternal,
                later.repository_source.identity(),
            );
            let attempt =
                restarted.record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
                    project_id: fixture.project_id,
                    goal_context_id: fixture.goal_id,
                    baseline_analysis_snapshot_id: later.identity,
                    session: "continuation".into(),
                    source_operation: "attempt post-work replacement authority".into(),
                    summary: "Attempt to replace the retained pre-work chain".into(),
                    material_boundary_review: complete_material_boundary_review(
                        std::slice::from_ref(&choice),
                        later.repository_source.identity(),
                    ),
                    interaction_review: outside_interactions(later.repository_source.identity()),
                    choices: vec![choice],
                });
            assert!(attempt
                .err()
                .ok_or("late authority was rebased")?
                .message()
                .contains("baseline cannot be replaced"));
        } else {
            assert_eq!(checkpoint?.changed_paths, ["src/lib.rs"]);
        }
    }
    Ok(())
}

#[test]
fn executable_scope_requires_complete_coupled_artifact_accounting(
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
    let mut incomplete = coupled_artifact_review(&["src/lib.rs"]);
    incomplete.assessments.retain(|assessment| {
        assessment.category != CoupledArtifactCategory::ChangelogOrReleaseNotes
    });

    let error = fixture
        .operations
        .bind_executable_work_scope(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            recorded.review_candidate_id,
            ApplicabilityScope {
                paths: vec!["src/lib.rs".into()],
                components: Vec::new(),
                work_contexts: Vec::new(),
            },
            incomplete,
        )
        .expect_err("every coupled-artifact category must be reviewed");
    assert!(std::error::Error::source(&error)
        .expect("coupled-artifact source error")
        .to_string()
        .contains("assess every maintained artifact category exactly once"));
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
            coupled_artifact_review(&["src/lib.rs", "tests"]),
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
    corrected.ownership.discretion_counterfactuals.clear();
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
    repository_fact.ownership.discretion_counterfactuals = discretion_proof(
        &repository_fact.dimension_id,
        &["approach-a", "approach-b"],
        repository_fact.ownership.source_basis[0],
    );
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
        source_evidence: vec![authority_evidence(
            repository_fact.basis.source_basis[0],
            false,
        )],
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
    agent_owned.ownership.discretion_counterfactuals = discretion_proof(
        &agent_owned.dimension_id,
        &["approach-a", "approach-b"],
        agent_owned.ownership.source_basis[0],
    );
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
    let rejected = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: recorded.review_candidate_id,
            rationale: "bounded research resolved the uncertainty".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions: vec![exploratory],
            learning_value_revision_bases: Vec::new(),
        })
        .expect_err("repository mutation cannot become qualifying prototype evidence");
    assert!(rejected
        .message()
        .contains("unchanged original repository baseline"));
    assert_eq!(
        readiness(&fixture, &recorded)?.stage,
        WorkAuthorityStage::ResearchOrPrototype
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
    repository_fact.ownership.discretion_counterfactuals = discretion_proof(
        &repository_fact.dimension_id,
        &["approach-a", "approach-b"],
        repository_fact.ownership.source_basis[0],
    );
    repository_fact.ownership.user_owned_outcomes.clear();
    repository_fact
        .ownership
        .bounded_implementation_discretion_rationale = Some(
        "repository evidence mechanically fixes the outcome without a product-policy choice".into(),
    );
    repository_fact.basis.kinds = vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact];
    repository_fact.basis.explicit_delegation = None;
    repository_fact.basis.exact_authority = Some(ExactAuthoritySufficiency {
        source_evidence: vec![authority_evidence(
            repository_fact.basis.source_basis[0],
            false,
        )],
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
    bind_current_review_scope(&fixture, &revised)?;
    assert_eq!(
        readiness(&fixture, &revised)?.stage,
        WorkAuthorityStage::ReadyForWork
    );

    let mut agent_owned = repository_fact;
    agent_owned.disposition = MaterialityDisposition::AgentOwnedImplementationChoice;
    agent_owned.ownership.contains_user_owned_outcome = false;
    agent_owned.ownership.discretion_counterfactuals = discretion_proof(
        &agent_owned.dimension_id,
        &["approach-a", "approach-b"],
        agent_owned.ownership.source_basis[0],
    );
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
    bind_current_review_scope(&fixture, &revised)?;
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
    bind_current_review_scope(&fixture, &ready)?;
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
    agent_owned.ownership.discretion_counterfactuals = discretion_proof(
        &agent_owned.dimension_id,
        &["approach-a", "approach-b"],
        agent_owned.ownership.source_basis[0],
    );
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
    bind_current_review_scope(&unrelated, &revised)?;
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
    bind_current_review_scope(&metadata, &revised)?;
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
    bind_current_review_scope(&fixture, &revised)?;
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
    bind_current_review_scope(&delegated_fixture, &revised)?;
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
    exploratory.ownership.discretion_counterfactuals.clear();
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
        discretion_counterfactuals: Vec::new(),
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
    run_user_owned_policy_with_learning(false)
}

#[test]
fn learning_and_independent_public_policy_keep_separate_lifecycles(
) -> Result<(), Box<dyn std::error::Error>> {
    run_user_owned_policy_with_learning(true)
}

fn run_user_owned_policy_with_learning(mixed: bool) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with_goal("Implement the public failure policy, which I retain control over independently of learning. I want to learn the private provenance representation trade-off.")?;
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
    let mut dimensions = vec![user_owned.clone(), coupled.clone()];
    let mut technical =
        agent_owned_dimension("provenance-representation", source, deliberation_worthy());
    if mixed {
        assess_learning_authority(&mut technical);
        dimensions.push(technical.clone());
    }
    let review_outcome = if mixed {
        let choices = dimensions
            .iter()
            .map(|d| {
                engineering_choice(
                    &d.dimension_id,
                    EngineeringEffectCategory::ImplementationInternal,
                    source,
                )
            })
            .collect();
        review_with_learning(&fixture, choices, dimensions, active_learning(&fixture))?
    } else {
        review(&fixture, dimensions)?
    };
    if mixed {
        complete_learning_selection(&fixture, &review_outcome, "provenance-representation")?;
        assert!(fixture
            .operations
            .canonical_basis(fixture.project_id)?
            .active_decisions
            .is_empty());
    }
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
    if mixed {
        assert!(bind_question_candidate_to_materiality(
            &review_record,
            "provenance-representation",
            question_draft.clone()
        )
        .is_err());
    }
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
    let mut revised_dimensions = vec![resolved, resolved_coupled];
    if mixed {
        for dimension in &mut revised_dimensions {
            assess_learning_authority(dimension);
        }
        revised_dimensions.push(technical);
    }
    let revised = fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: review_outcome.review_candidate_id,
            rationale: "the exact current-host response produced an applicable Decision".to_owned(),
            learning_participation: if mixed {
                active_learning(&fixture)
            } else {
                LearningParticipation::Inactive
            },
            dimensions: revised_dimensions,
            learning_value_revision_bases: Vec::new(),
        })?;
    bind_current_review_scope(&fixture, &revised)?;
    let ready = readiness(&fixture, &revised)?;
    assert_eq!(ready.disposition, WorkAuthorityDisposition::ReadyForWork);
    assert_eq!(
        ready.satisfied_requirements.len(),
        if mixed { 4 } else { 2 }
    );
    assert!(ready
        .satisfied_requirements
        .iter()
        .filter(
            |requirement| requirement.dimension_id.as_deref() != Some("provenance-representation")
        )
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

#[test]
fn material_decomposition_closes_hidden_public_contracts_before_materiality(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::MaterialDecomposition;
    for (parent_id, child_id, alternative_summary) in [
        (
            "response-file",
            "relative-path-policy",
            "Accept response-file arguments",
        ),
        (
            "direct-scalar",
            "configuration-source-precedence",
            "Accept a scalar compatibility setting",
        ),
        (
            "control-semantics",
            "public-status-contract",
            "Expose control and status",
        ),
        (
            "compatible-existing-result",
            "public-error-metadata",
            "Keep compatible results with error metadata",
        ),
    ] {
        let fixture = fixture()?;
        let source = fixture.baseline.repository_source.identity();
        let mut parent = engineering_choice(
            parent_id,
            EngineeringEffectCategory::PublicApiShapeOrSemantics,
            source,
        );
        parent.alternatives[0].summary = alternative_summary.into();
        parent.alternatives[0].material_decomposition = MaterialDecomposition::Decomposed {
            choice_ids: vec![child_id.into()],
        };
        let parent_dimension = agent_owned_dimension(
            parent_id,
            source,
            LearningValueAssessment::Routine {
                rationale: "bounded parent implementation".into(),
            },
        );
        let error = review_with_choices(
            &fixture,
            vec![parent.clone()],
            vec![parent_dimension.clone()],
        )
        .expect_err("hidden public subchoice must be explicit");
        assert!(error.message().contains("missing choice"));
        let child = engineering_choice(
            child_id,
            EngineeringEffectCategory::PublicApiShapeOrSemantics,
            source,
        );
        let child_dimension = dimension(
            child_id,
            MaterialityDisposition::UnresolvedUserOwnedOutcome {
                resolution_decision_id: None,
            },
            vec![WorkAuthorityBasisKind::AgentRecommendation],
            source,
        );
        let recorded = review_with_choices(
            &fixture,
            vec![parent, child],
            vec![parent_dimension, child_dimension],
        )?;
        let result = readiness(&fixture, &recorded)?;
        assert_eq!(result.stage, WorkAuthorityStage::QuestionRequired);
        assert!(result.blocking);
    }
    Ok(())
}

#[test]
fn material_decomposition_rejects_open_cyclic_and_duplicate_graphs(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::MaterialDecomposition;
    for invalid in [
        "empty",
        "self",
        "cycle",
        "duplicate-child",
        "duplicate-choice",
        "duplicate-alternative",
        "empty-rationale",
    ] {
        let fixture = fixture()?;
        let source = fixture.baseline.repository_source.identity();
        let mut parent = engineering_choice(
            "parent",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        );
        let mut child = engineering_choice(
            "child",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        );
        parent.alternatives[0].material_decomposition = MaterialDecomposition::Decomposed {
            choice_ids: match invalid {
                "empty" => vec![],
                "self" => vec!["parent".into()],
                "duplicate-child" => vec!["child".into(), "child".into()],
                _ => vec!["child".into()],
            },
        };
        match invalid {
            "cycle" => {
                child.alternatives[0].material_decomposition = MaterialDecomposition::Decomposed {
                    choice_ids: vec!["parent".into()],
                }
            }
            "duplicate-choice" => child.choice_id = "parent".into(),
            "duplicate-alternative" => {
                parent.alternatives[1].alternative_id =
                    parent.alternatives[0].alternative_id.clone()
            }
            "empty-rationale" => {
                if let MaterialDecomposition::MateriallyAtomic { rationale, .. } =
                    &mut child.alternatives[0].material_decomposition
                {
                    rationale.clear();
                }
            }
            _ => {}
        }
        let choices = vec![parent, child];
        let result = fixture.operations.record_engineering_choice_discovery(
            EngineeringChoiceDiscoveryDraft {
                interaction_review: outside_interactions(
                    fixture.baseline.repository_source.identity(),
                ),
                project_id: fixture.project_id,
                goal_context_id: fixture.goal_id,
                baseline_analysis_snapshot_id: fixture.baseline.identity,
                session: "work-authority-session".into(),
                source_operation: "decomposition".into(),
                summary: "review subordinate choices".into(),
                material_boundary_review: complete_material_boundary_review(&choices, source),
                choices,
            },
        );
        assert!(result.is_err(), "invalid graph accepted: {invalid}");
    }
    Ok(())
}

#[test]
fn materially_atomic_private_details_terminate_without_question(
) -> Result<(), Box<dyn std::error::Error>> {
    for detail in [
        "helper-naming",
        "private-function-extraction",
        "equivalent-internal-structure",
        "local-test-helper",
    ] {
        let fixture = fixture()?;
        let source = fixture.baseline.repository_source.identity();
        let mut choice = engineering_choice(
            detail,
            EngineeringEffectCategory::ImplementationInternal,
            source,
        );
        for alternative in &mut choice.alternatives {
            alternative.material_decomposition = volicord_inquiry::MaterialDecomposition::MateriallyAtomic {
                residual_fork_closure: volicord_inquiry::ResidualForkClosure { interaction_comparisons: vec![],
                    fixed_outcome: "The public value contract remains identical".into(),
                    credible_implementations: vec!["Inline value construction".into(), "Private helper returns the identical value".into()],
                    remaining_material_outcomes: vec![], source_basis: vec![source],
                },
                rationale: "Inspection of src/lib.rs establishes that this local representation preserves the public value contract and introduces no additional observable outcome.".into(),
            };
        }
        let bounded = agent_owned_dimension(
            detail,
            source,
            LearningValueAssessment::Routine {
                rationale: "private mechanically equivalent detail".into(),
            },
        );
        let recorded = review_with_choices(&fixture, vec![choice], vec![bounded])?;
        assert_eq!(
            readiness(&fixture, &recorded)?.stage,
            WorkAuthorityStage::ReadyForWork
        );
    }
    Ok(())
}

#[test]
fn blocked_prototype_cannot_rebase_tracked_fixture_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        EngineeringChoiceEvidenceState::PrototypeRequired,
        EngineeringChoiceEvidenceState::ResearchRequired,
    ] {
        let mut fixture = fixture()?;
        fs::create_dir_all(fixture.repository.join("tests"))?;
        fs::write(fixture.repository.join("tests/prototype.txt"), "original\n")?;
        for args in [
            vec!["init", "-q"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(std::process::Command::new("git")
                .args(args)
                .current_dir(&fixture.repository)
                .status()?
                .success());
        }
        fixture.baseline = fixture
            .operations
            .analyze(fixture.project_id, vec![])?
            .value
            .ok_or("analysis")?
            .analysis;
        let source = fixture.baseline.repository_source.identity();
        let mut choice = engineering_choice(
            "prototype",
            EngineeringEffectCategory::ImplementationInternal,
            source,
        );
        choice.evidence_state = evidence;
        let bounded = agent_owned_dimension(
            "prototype",
            source,
            LearningValueAssessment::Routine {
                rationale: "bounded implementation after evidence".into(),
            },
        );
        let original = review_with_choices(&fixture, vec![choice.clone()], vec![bounded.clone()])?;
        assert!(readiness(&fixture, &original)?.blocking);
        fs::write(
            fixture.repository.join("tests/prototype.txt"),
            "unauthorized experiment\n",
        )?;
        let later = fixture
            .operations
            .analyze(fixture.project_id, vec![])?
            .value
            .ok_or("analysis")?
            .analysis;
        choice.evidence_state = EngineeringChoiceEvidenceState::Sufficient;
        choice.source_basis = vec![later.repository_source.identity()];
        let attempt = fixture.operations.record_engineering_choice_discovery(
            EngineeringChoiceDiscoveryDraft {
                interaction_review: outside_interactions(
                    fixture.baseline.repository_source.identity(),
                ),
                project_id: fixture.project_id,
                goal_context_id: fixture.goal_id,
                baseline_analysis_snapshot_id: later.identity,
                session: "fresh-resume".into(),
                source_operation: "post-write rebaseline".into(),
                summary: "attempt fresh readiness".into(),
                material_boundary_review: complete_material_boundary_review(
                    &[choice.clone()],
                    later.repository_source.identity(),
                ),
                choices: vec![choice],
            },
        );
        assert!(
            attempt.is_err(),
            "post-write baseline must not replace blocked authority"
        );
        let mut resolved = bounded;
        resolved.basis.kinds.push(match evidence {
            EngineeringChoiceEvidenceState::ResearchRequired => {
                WorkAuthorityBasisKind::ResearchEvidence
            }
            _ => WorkAuthorityBasisKind::PrototypeEvidence,
        });
        resolved.basis.research_basis = vec!["bounded experiment result".into()];
        assert!(
            fixture
                .operations
                .revise_materiality_review(MaterialityReviewRevisionDraft {
                    project_id: fixture.project_id,
                    review_candidate_id: original.review_candidate_id,
                    rationale: "attempt resolution after tracked fixture mutation".into(),
                    learning_participation: LearningParticipation::Inactive,
                    dimensions: vec![resolved.clone()],
                    learning_value_revision_bases: vec![],
                })
                .is_err(),
            "tracked evidence must be restored before resolving the original chain"
        );
        fs::write(fixture.repository.join("tests/prototype.txt"), "original\n")?;
        let scratch = tempdir()?;
        fs::write(
            scratch.path().join("prototype.txt"),
            "scratch experiment evidence\n",
        )?;
        let ready =
            fixture
                .operations
                .revise_materiality_review(MaterialityReviewRevisionDraft {
                    project_id: fixture.project_id,
                    review_candidate_id: original.review_candidate_id,
                    rationale: "scratch evidence resolves the original pre-work chain".into(),
                    learning_participation: LearningParticipation::Inactive,
                    dimensions: vec![resolved],
                    learning_value_revision_bases: vec![],
                })?;
        assert_eq!(
            ready.baseline_analysis_snapshot_id,
            fixture.baseline.identity
        );
        bind_current_review_scope(&fixture, &ready)?;
        assert_eq!(
            readiness(&fixture, &ready)?.stage,
            WorkAuthorityStage::ReadyForWork
        );
    }
    Ok(())
}

#[test]
fn preserving_refactor_requires_override_and_default_propagation_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::{CheckpointVerificationBasis, CompatibilitySurfaceReview};
    let mut fixture = fixture()?;
    fs::create_dir_all(fixture.repository.join("tests"))?;
    fs::write(
        fixture.repository.join("src/signer.py"),
        include_str!("../../../validation/inquiry/compatibility-preservation/signer.py"),
    )?;
    fixture.baseline = fixture
        .operations
        .analyze(fixture.project_id, vec![])?
        .value
        .ok_or("analysis")?
        .analysis;
    let source = fixture.baseline.repository_source.identity();
    let mut choice = engineering_choice(
        "preserving-signer",
        EngineeringEffectCategory::Compatibility,
        source,
    );
    let mut bounded = agent_owned_dimension(
        "preserving-signer",
        source,
        LearningValueAssessment::Routine {
            rationale: "preserve the established signer contract".into(),
        },
    );
    choice.affected_scope = vec!["src/signer.py".into()];
    bounded.affected_scope = choice.affected_scope.clone();
    let _review = review_with_choices(&fixture, vec![choice], vec![bounded])?;
    let original_signer = fs::read_to_string(fixture.repository.join("src/signer.py"))?;
    fs::write(
        fixture.repository.join("src/signer.py"),
        original_signer.replace(
            "class ActiveExtension(Extension, Signer):",
            "class ActiveExtension(Extension, RefactoredSigner):",
        ),
    )?;
    let run = |case: &str| -> Result<CommandVerificationDraft, Box<dyn std::error::Error>> {
        let result = std::process::Command::new("python3")
            .args(["src/signer.py", case])
            .current_dir(&fixture.repository)
            .output()?;
        Ok(CommandVerificationDraft {
            state: if result.status.success() {
                VerificationState::Passed
            } else {
                VerificationState::Failed
            },
            command_label: Some(format!("signer {case}")),
            command_invocation: Some(format!("python3 src/signer.py {case}")),
            exit_code: result.status.code(),
            termination: Some(volicord_context::CommandTermination::Exited),
            outcome: Some(format!("{case}: {}", result.status)),
        })
    };
    let base = run("BaseContract")?;
    let broken = run("ExtensionContract")?;
    assert_eq!(base.state, VerificationState::Passed);
    assert_eq!(broken.state, VerificationState::Failed);
    let mut draft = checkpoint_draft(&fixture, vec![]);
    draft.kind = CheckpointKind::Completion;
    draft.work_state = WorkState::Completed;
    draft.handoff_to = None;
    draft.verification = vec![base];
    assert!(
        fixture
            .operations
            .record_grounded_checkpoint(draft.clone())
            .is_err(),
        "compatibility impact cannot omit the review"
    );
    let mut surfaces = vec![
        CompatibilitySurfaceReview {
            surface_id: "base-signer".into(),
            inspected_paths: vec!["src/signer.py".into()],
            preserved_contract: "base default and explicit salt behavior".into(),
            verification_indices: vec![0],
            coverage_rationale: "BaseContract exercises direct calls".into(),
        },
        CompatibilitySurfaceReview {
            surface_id: "override-default".into(),
            inspected_paths: vec!["src/signer.py".into()],
            preserved_contract:
                "make_signer override receives None unchanged and retains explicit salt".into(),
            verification_indices: vec![],
            coverage_rationale: "ExtensionContract exercises dispatch and default propagation"
                .into(),
        },
    ];
    let basis = |surfaces| {
        CheckpointVerificationBasis::BehaviorPreserving { surfaces, preservation_rationale: "The affected extension hook and default propagation are exercised alongside direct calls.".into() }
    };
    draft.verification_basis = basis(surfaces.clone());
    assert!(
        fixture
            .operations
            .record_grounded_checkpoint(draft.clone())
            .is_err(),
        "base tests alone do not cover the known override"
    );
    surfaces[1].verification_indices = vec![1];
    draft.verification_basis = basis(surfaces);
    draft.verification.push(broken);
    assert!(
        fixture
            .operations
            .record_grounded_checkpoint(draft.clone())
            .is_err(),
        "broken extension test blocks completion despite passing base tests"
    );
    fs::write(
        fixture.repository.join("src/signer.py"),
        format!("{original_signer}\n# Preserve virtual dispatch and default propagation.\n"),
    )?;
    draft.verification[0] = run("BaseContract")?;
    draft.verification[1] = run("ExtensionContract")?;
    let outcome = fixture.operations.record_grounded_checkpoint(draft)?;
    let fresh = LocalOperations::new(fixture.operations.layout().clone());
    let canonical = fresh.canonical_basis(fixture.project_id)?;
    assert!(canonical
        .latest_checkpoint
        .iter()
        .any(|checkpoint| checkpoint.id == outcome.checkpoint_id
            && checkpoint
                .verification
                .iter()
                .any(|fact| fact.outcome.as_deref().is_some_and(|text| text
                    .contains("override-default")
                    && text.contains("make_signer")))));
    Ok(())
}

#[test]
fn public_failure_semantics_rejects_bare_agent_owned_assertion(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let mut failure = dimension(
        "failure-contract",
        MaterialityDisposition::AgentOwnedImplementationChoice,
        vec![WorkAuthorityBasisKind::ImplementationPreference],
        fixture.baseline.repository_source.identity(),
    );
    failure.material_consequences = vec![
        "returned discriminated failure requires caller result inspection".into(),
        "thrown failure requires caller exception handling".into(),
    ];
    failure.observable_signals = vec![MaterialOutcomeSignal::ObservableFailurePolicy];
    failure.ownership.materially_varying_outcomes = failure.material_consequences.clone();
    failure.ownership.discretion_counterfactuals.clear();
    failure.ownership.rationale = "the agent implements the error handling internally".into();
    failure
        .ownership
        .bounded_implementation_discretion_rationale =
        Some("implementation is the agent's responsibility".into());
    assert!(
        review(&fixture, vec![failure]).is_err(),
        "caller-visible failure semantics need source-grounded discretion proof"
    );
    Ok(())
}

#[test]
fn discretion_counterfactuals_require_complete_linked_evidence_without_category_classifier(
) -> Result<(), Box<dyn std::error::Error>> {
    for defect in [
        "missing",
        "duplicate",
        "foreign-alternative",
        "ungrounded",
        "empty-observation",
    ] {
        let fixture = fixture()?;
        let mut choice = agent_owned_dimension(
            "private-index",
            fixture.baseline.repository_source.identity(),
            LearningValueAssessment::Routine {
                rationale: "private organization".into(),
            },
        );
        match defect {
            "missing" => {
                choice.ownership.discretion_counterfactuals.pop();
            }
            "duplicate" => {
                choice.ownership.discretion_counterfactuals[1] =
                    choice.ownership.discretion_counterfactuals[0].clone();
            }
            "foreign-alternative" => {
                choice.ownership.discretion_counterfactuals[0].alternative_id = "unrelated".into();
            }
            "ungrounded" => {
                choice.ownership.discretion_counterfactuals[0]
                    .source_supported_boundary
                    .clear();
            }
            _ => {
                choice.ownership.discretion_counterfactuals[0]
                    .observation_rationale
                    .clear();
            }
        }
        assert!(review(&fixture, vec![choice]).is_err(), "{defect}");
    }
    let fixture = fixture()?;
    let mut choice = agent_owned_dimension(
        "private-index",
        fixture.baseline.repository_source.identity(),
        LearningValueAssessment::Routine {
            rationale: "bounded resource discretion".into(),
        },
    );
    // Observable differences can still be implementation discretion; typed evidence
    // is checked, without promoting the observation boolean to an ownership rule.
    choice.ownership.discretion_counterfactuals[0].externally_observable = true;
    choice.ownership.discretion_counterfactuals[0].observation_rationale =
        "The two private indexes use different amounts of memory within the fixed resource budget."
            .into();
    choice.ownership.discretion_counterfactuals[0].source_supported_boundary = "The fixture resource boundary permits either allocation inside the fixed budget without changing result semantics.".into();
    let recorded = review(&fixture, vec![choice])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.disposition,
        WorkAuthorityDisposition::ReadyForWork
    );
    Ok(())
}

#[test]
fn tuple_precedent_does_not_normatively_select_new_public_result(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let mut representation = dimension(
        "result-representation",
        MaterialityDisposition::SettledAuthority,
        vec![WorkAuthorityBasisKind::AcceptedContract],
        fixture.baseline.repository_source.identity(),
    );
    if let Some(authority) = &mut representation.basis.exact_authority {
        authority.source_evidence[0].role =
            volicord_operations::AuthoritySourceRole::RepositoryPrecedent;
        authority.source_evidence[0].rationale = "decode_unchecked demonstrates a tuple convention, without requiring that shape for the new API.".into();
    }
    representation.material_consequences = vec!["An optional tuple and a structured result both preserve existing callers but give new callers different public representations.".into()];
    representation.basis.summary = "The existing decode_unchecked API returns a tuple; follow that convention for the new API.".into();
    representation.basis.authority_counterfactual = "The existing tuple convention is a compatibility precedent, with no accepted requirement selecting the new result shape.".into();
    assert!(review(&fixture, vec![representation]).is_err(),
        "a descriptive tuple precedent needs an exact normative source before settling the new public contract");
    Ok(())
}

#[test]
fn exact_authority_requires_normative_source_linkage_for_each_alternative(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_operations::AuthoritySourceRole;
    for role in [
        AuthoritySourceRole::RepositoryPrecedent,
        AuthoritySourceRole::CompatibilityConstraint,
        AuthoritySourceRole::RecommendationOrPreference,
    ] {
        let fixture = fixture()?;
        let mut choice = dimension(
            "new-result",
            MaterialityDisposition::SettledAuthority,
            vec![WorkAuthorityBasisKind::AcceptedContract],
            fixture.baseline.repository_source.identity(),
        );
        choice
            .basis
            .exact_authority
            .as_mut()
            .ok_or("authority missing")?
            .source_evidence[0]
            .role = role;
        assert!(review(&fixture, vec![choice]).is_err());
    }
    for defect in ["missing", "wrong-contract", "unlinked", "duplicate"] {
        let fixture = fixture()?;
        let mut choice = dimension(
            "new-result",
            MaterialityDisposition::SettledAuthority,
            vec![WorkAuthorityBasisKind::AcceptedContract],
            fixture.baseline.repository_source.identity(),
        );
        let evidence = &mut choice
            .basis
            .exact_authority
            .as_mut()
            .ok_or("authority missing")?
            .source_evidence;
        match defect {
            "missing" => evidence.clear(),
            "wrong-contract" => {
                evidence[0].role = AuthoritySourceRole::AcceptedContract {
                    contract_reference: "unrelated convention".into(),
                }
            }
            "unlinked" => evidence[0].source_id = fixture.goal_source_id,
            _ => evidence.push(evidence[0].clone()),
        }
        assert!(review(&fixture, vec![choice]).is_err(), "{defect}");
    }
    // A precedent remains useful when an actual accepted requirement adopts it.
    let fixture = fixture()?;
    let mut choice = dimension(
        "new-result",
        MaterialityDisposition::SettledAuthority,
        vec![WorkAuthorityBasisKind::AcceptedContract],
        fixture.baseline.repository_source.identity(),
    );
    let mut precedent = authority_evidence(fixture.baseline.repository_source.identity(), true);
    precedent.role = AuthoritySourceRole::RepositoryPrecedent;
    precedent.rationale = "The existing tuple example supports the recommendation; the separately identified accepted requirement explicitly mandates that same shape for this new API.".into();
    choice
        .basis
        .exact_authority
        .as_mut()
        .ok_or("authority missing")?
        .source_evidence
        .push(precedent);
    let recorded = review(&fixture, vec![choice])?;
    assert_eq!(
        readiness(&fixture, &recorded)?.disposition,
        WorkAuthorityDisposition::ReadyForWork
    );
    assert!(fixture
        .operations
        .canonical_basis(fixture.project_id)?
        .active_questions
        .is_empty());
    Ok(())
}

#[test]
fn residual_material_outcomes_cannot_close_as_atomic() -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::MaterialDecomposition;
    for (feature, residual, implementations) in [
        (
            "response-file",
            "relative-path authority",
            [
                "Resolve paths from the process directory",
                "Resolve paths from the response-file directory",
            ],
        ),
        (
            "direct-scalar-compatibility",
            "configuration-source precedence",
            [
                "The direct scalar overrides the alias source",
                "The alias source overrides the direct scalar",
            ],
        ),
    ] {
        for defect in [
            "remaining-outcome",
            "missing-source",
            "foreign-source",
            "no-comparison",
            "missing-fixed-outcome",
            "missing-boundary-outcomes",
        ] {
            let fixture = fixture()?;
            let source = fixture.baseline.repository_source.identity();
            let mut choice =
                engineering_choice(feature, EngineeringEffectCategory::Compatibility, source);
            if let MaterialDecomposition::MateriallyAtomic {
                residual_fork_closure,
                ..
            } = &mut choice.alternatives[0].material_decomposition
            {
                residual_fork_closure.fixed_outcome = format!("Support {feature}");
                residual_fork_closure.credible_implementations =
                    implementations.iter().map(|s| (*s).into()).collect();
                match defect {
                    "remaining-outcome" => residual_fork_closure
                        .remaining_material_outcomes
                        .push(residual.into()),
                    "missing-source" => residual_fork_closure.source_basis.clear(),
                    "foreign-source" => {
                        residual_fork_closure.source_basis = vec![fixture.goal_source_id]
                    }
                    "no-comparison" => residual_fork_closure.credible_implementations.truncate(1),
                    "missing-fixed-outcome" => residual_fork_closure.fixed_outcome.clear(),
                    _ => {}
                }
            }
            let choices = vec![choice];
            let mut boundaries = complete_material_boundary_review(&choices, source);
            if defect == "missing-boundary-outcomes" {
                boundaries[0].reviewed_outcomes.clear();
            }
            let result = fixture.operations.record_engineering_choice_discovery(
                EngineeringChoiceDiscoveryDraft {
                    interaction_review: outside_interactions(
                        fixture.baseline.repository_source.identity(),
                    ),
                    project_id: fixture.project_id,
                    goal_context_id: fixture.goal_id,
                    baseline_analysis_snapshot_id: fixture.baseline.identity,
                    session: "residual-challenge".into(),
                    source_operation: "bounded counterfactual review".into(),
                    summary: feature.into(),
                    choices,
                    material_boundary_review: boundaries,
                },
            );
            assert!(result.is_err(), "{feature}: {defect} must be rejected");
        }
    }
    // Current wire/persisted atomic shape has no rationale-only decoder.
    assert!(
        serde_json::from_value::<MaterialDecomposition>(serde_json::json!({
            "state":"materially_atomic", "rationale":"prose alone"
        }))
        .is_err()
    );
    Ok(())
}

#[test]
fn new_pre_write_outcome_revokes_scope_and_requires_rediscovery(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let bounded = agent_owned_dimension(
        "bounded",
        source,
        LearningValueAssessment::Routine {
            rationale: "private work".into(),
        },
    );
    let recorded = review(&fixture, vec![bounded.clone()])?;
    let before = fixture
        .operations
        .inspect_workflow_candidate(fixture.project_id, recorded.review_candidate_id)?;
    let before_review = before
        .content
        .as_ref()
        .and_then(|content| content.materiality_review.as_ref())
        .ok_or("review missing")?;
    let binding = before_review
        .executable_work_scope
        .as_ref()
        .ok_or("scope missing")?;
    assert_eq!(binding.authority_basis.review_candidate_id, before.id);
    assert!(binding.authority_basis.review_revision < before.revision);
    assert!(binding.authority_basis.source_basis.contains(&source));
    assert_eq!(binding.scope.paths, ["src/lib.rs"]);
    let mut report = coupled_artifact_review(&["src/lib.rs"]);
    report.materiality_closure = volicord_inquiry::PreWriteMaterialityClosure::NewMaterialOutcome {
        outcomes: vec!["Configuration-source precedence changes when both sources are present".into()],
        rationale: "The planned compatibility adapter introduces an independently material precedence branch".into(),
    };
    fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        recorded.review_candidate_id,
        binding.scope.clone(),
        report,
    )?;
    let reopened = LocalOperations::new(fixture.operations.layout().clone());
    let pending =
        reopened.inspect_workflow_candidate(fixture.project_id, recorded.review_candidate_id)?;
    let pending_review = pending
        .content
        .as_ref()
        .and_then(|content| content.materiality_review.as_ref())
        .ok_or("pending missing")?;
    assert!(pending_review.executable_work_scope.is_none());
    assert!(pending_review.pending_pre_write_reassessment.is_some());
    assert!(readiness(&fixture, &recorded)?.blocking);
    let workflow =
        reopened.workflow_for_review_candidate(fixture.project_id, recorded.review_candidate_id)?;
    assert_eq!(workflow.stage, WorkflowStage::EngineeringChoiceDiscovery);
    assert!(reopened
        .bind_executable_work_scope(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            recorded.review_candidate_id,
            binding.scope.clone(),
            coupled_artifact_review(&["src/lib.rs"])
        )
        .is_err());
    // Rediscovery represents the newly observed material branch independently.
    let new_material = dimension(
        "source-precedence",
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None,
        },
        vec![WorkAuthorityBasisKind::AgentRecommendation],
        source,
    );
    let mut choices = reopened
        .inspect_workflow_candidate(
            fixture.project_id,
            before_review.engineering_choice_discovery_candidate_id,
        )?
        .content
        .and_then(|content| content.engineering_choice_discovery)
        .ok_or("discovery missing")?
        .choices;
    choices.push(engineering_choice(
        "source-precedence",
        EngineeringEffectCategory::PublicApiShapeOrSemantics,
        source,
    ));
    let discovery =
        reopened.record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
            interaction_review: outside_interactions(fixture.baseline.repository_source.identity()),
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "rediscovery".into(),
            source_operation: "pre-write material outcome reassessment".into(),
            summary: "Represent source precedence independently".into(),
            material_boundary_review: complete_material_boundary_review(&choices, source),
            choices,
        })?;
    let next =
        reopened.workflow_for_review_candidate(fixture.project_id, recorded.review_candidate_id)?;
    assert_eq!(next.stage, WorkflowStage::MaterialityReview);
    assert!(next.blocks_ordinary_work);
    assert_eq!(
        next.required_next_action
            .as_ref()
            .map(|action| action.tool.as_str()),
        Some("materiality_review")
    );
    assert!(next
        .satisfied_basis_identities
        .iter()
        .any(
            |basis| basis.kind == "engineering_choice_discovery_candidate"
                && basis.identity == discovery.discovery_candidate_id.to_string()
        ));
    assert!(!next
        .satisfied_basis_identities
        .iter()
        .any(|basis| basis.kind == "materiality_review_candidate"));
    let reassessment = MaterialityReviewDraft {
        project_id: fixture.project_id,
        goal_context_id: fixture.goal_id,
        baseline_analysis_snapshot_id: fixture.baseline.identity,
        session: "rediscovery".into(),
        source_operation: "review rediscovered outcomes".into(),
        rationale: "The new precedence policy needs user authority".into(),
        behavioral_context_basis: volicord_operations::BehavioralContextBasis {
            context_item_ids: vec![],
            completeness_rationale: "No non-Goal behavioral context in this fixture".into(),
        },
        learning_participation: LearningParticipation::Inactive,
        engineering_choice_discovery_candidate_id: discovery.discovery_candidate_id,
        dimensions: vec![bounded, new_material],
    };
    // A review after a premature write cannot certify that write, even though R1
    // was timely. Restore the baseline before recording legitimate R2 authority.
    let original = fs::read(fixture.repository.join("src/lib.rs"))?;
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn value() -> u32 { 2 }\n",
    )?;
    let late = reopened
        .record_materiality_review(reassessment.clone())
        .expect_err("R2 cannot authorize an earlier mutation");
    assert!(late.message().contains("first Materiality Review is late"));
    fs::write(fixture.repository.join("src/lib.rs"), original)?;
    let reassessed = reopened.record_materiality_review(reassessment)?;
    assert_ne!(reassessed.review_candidate_id, recorded.review_candidate_id);
    assert_eq!(
        readiness(&fixture, &reassessed)?.stage,
        WorkAuthorityStage::QuestionRequired
    );
    let next =
        reopened.workflow_for_review_candidate(fixture.project_id, recorded.review_candidate_id)?;
    assert_eq!(next.stage, WorkflowStage::QuestionCandidate);
    assert!(next.blocks_ordinary_work);
    for (kind, identity) in [
        (
            "engineering_choice_discovery_candidate",
            discovery.discovery_candidate_id.to_string(),
        ),
        (
            "materiality_review_candidate",
            reassessed.review_candidate_id.to_string(),
        ),
    ] {
        assert!(next
            .satisfied_basis_identities
            .iter()
            .any(|basis| basis.kind == kind && basis.identity == identity));
    }
    assert_eq!(
        reopened.inspect_workflow_candidate(fixture.project_id, recorded.review_candidate_id)?,
        pending
    );
    assert!(readiness(&fixture, &recorded)?.blocking);
    Ok(())
}

#[test]
fn workflow_selects_reviews_only_within_the_latest_discovery_frontier(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let dimensions = vec![agent_owned_dimension(
        "bounded",
        source,
        LearningValueAssessment::Routine {
            rationale: "private work".into(),
        },
    )];
    let first = review(&fixture, dimensions.clone())?;
    let second = review(&fixture, dimensions.clone())?;
    let first_discovery = readiness(&fixture, &first)?
        .engineering_choice_discovery_candidate_id
        .ok_or("D1 missing")?;
    let second_discovery = readiness(&fixture, &second)?
        .engineering_choice_discovery_candidate_id
        .ok_or("D2 missing")?;
    let draft = |discovery| MaterialityReviewDraft {
        project_id: fixture.project_id,
        goal_context_id: fixture.goal_id,
        baseline_analysis_snapshot_id: fixture.baseline.identity,
        session: "candidate ordering".into(),
        source_operation: "review candidate ordering".into(),
        rationale: "Reassess the same bounded dimensions".into(),
        behavioral_context_basis: volicord_operations::BehavioralContextBasis {
            context_item_ids: vec![],
            completeness_rationale: "No other behavioral context".into(),
        },
        learning_participation: LearningParticipation::Inactive,
        engineering_choice_discovery_candidate_id: discovery,
        dimensions: dimensions.clone(),
    };
    // A later review of D1 cannot grant or withhold authority over D2.
    let older_frontier_review = fixture
        .operations
        .record_materiality_review(draft(first_discovery))?;
    let assert_frontier =
        |review_id: String, stage, blocked| -> Result<(), volicord_operations::Error> {
            let reopened = LocalOperations::new(fixture.operations.layout().clone());
            for requested in [
                first.review_candidate_id,
                second.review_candidate_id,
                older_frontier_review.review_candidate_id,
            ] {
                let workflow =
                    reopened.workflow_for_review_candidate(fixture.project_id, requested)?;
                assert_eq!(workflow.stage, stage);
                assert_eq!(workflow.blocks_ordinary_work, blocked);
                for (kind, identity) in [
                    (
                        "engineering_choice_discovery_candidate",
                        second_discovery.to_string(),
                    ),
                    ("materiality_review_candidate", review_id.clone()),
                ] {
                    assert!(workflow
                        .satisfied_basis_identities
                        .iter()
                        .any(|basis| basis.kind == kind && basis.identity == identity));
                }
            }
            Ok(())
        };
    assert_frontier(
        second.review_candidate_id.to_string(),
        WorkflowStage::ReadyForWork,
        false,
    )?;
    // Among reviews of D2, the newest candidate wins and still needs its own closure.
    let current = fixture
        .operations
        .record_materiality_review(draft(second_discovery))?;
    assert_frontier(
        current.review_candidate_id.to_string(),
        WorkflowStage::MaterialityReview,
        true,
    )?;
    fixture
        .operations
        .revise_materiality_review(MaterialityReviewRevisionDraft {
            project_id: fixture.project_id,
            review_candidate_id: second.review_candidate_id,
            rationale: "A later revision does not change this candidate's creation order".into(),
            learning_participation: LearningParticipation::Inactive,
            dimensions,
            learning_value_revision_bases: vec![],
        })?;
    assert_frontier(
        current.review_candidate_id.to_string(),
        WorkflowStage::MaterialityReview,
        true,
    )?;
    bind_current_review_scope(&fixture, &current)?;
    assert_frontier(
        current.review_candidate_id.to_string(),
        WorkflowStage::ReadyForWork,
        false,
    )?;
    Ok(())
}

#[test]
fn pre_write_closure_rejects_superseded_or_contradictory_serialized_shapes() {
    use volicord_inquiry::PreWriteMaterialityClosure;
    for value in [
        serde_json::json!({"materiality_reassessment":"no new outcome"}),
        serde_json::json!({"state":"no_new_material_outcome","rationale":"no new outcome"}),
        serde_json::json!({"state":"no_new_material_outcome","reviewed_outcomes":["scalar support"],"outcomes":["source precedence"],"rationale":"prose cannot hide the new outcome"}),
    ] {
        assert!(serde_json::from_value::<PreWriteMaterialityClosure>(value).is_err());
    }
}

fn bind_current_review_scope(
    fixture: &Fixture,
    review: &volicord_operations::MaterialityReviewOutcome,
) -> Result<(), volicord_operations::Error> {
    fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        review.review_candidate_id,
        volicord_context::ApplicabilityScope {
            paths: vec!["src/lib.rs".into()],
            components: vec![],
            work_contexts: vec![],
        },
        coupled_artifact_review(&["src/lib.rs"]),
    )?;
    Ok(())
}

#[test]
fn persisted_pre_write_closure_is_validated_before_restart_readiness(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let source = fixture.baseline.repository_source.identity();
    let recorded = review(
        &fixture,
        vec![agent_owned_dimension(
            "private",
            source,
            LearningValueAssessment::Routine {
                rationale: "bounded helper".into(),
            },
        )],
    )?;
    let connection = rusqlite::Connection::open(fixture.operations.layout().candidate_store())?;
    let original: String = connection.query_row(
        "SELECT record_json FROM candidates WHERE id = ?1",
        [recorded.review_candidate_id.as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    for defect in ["foreign-review", "new-outcome-as-scope", "missing-source"] {
        let mut value: serde_json::Value = serde_json::from_str(&original)?;
        let binding = &mut value["content"]["materiality_review"]["executable_work_scope"];
        match defect {
            "foreign-review" => {
                binding["authority_basis"]["review_candidate_id"] = serde_json::json!(vec![0; 16])
            }
            "missing-source" => binding["authority_basis"]["source_basis"] = serde_json::json!([]),
            _ => {
                binding["coupled_artifact_review"]["materiality_closure"] = serde_json::json!({
                    "state":"new_material_outcome","outcomes":["relative-path authority"],"rationale":"discovery is required"
                })
            }
        }
        connection.execute(
            "UPDATE candidates SET record_json = ?1 WHERE id = ?2",
            rusqlite::params![
                value.to_string(),
                recorded.review_candidate_id.as_bytes().as_slice()
            ],
        )?;
        let reopened = LocalOperations::new(fixture.operations.layout().clone());
        assert!(
            reopened
                .inspect_workflow_candidate(fixture.project_id, recorded.review_candidate_id)
                .is_err(),
            "{defect}"
        );
        connection.execute(
            "UPDATE candidates SET record_json = ?1 WHERE id = ?2",
            rusqlite::params![original, recorded.review_candidate_id.as_bytes().as_slice()],
        )?;
    }
    assert_eq!(
        readiness(&fixture, &recorded)?.stage,
        WorkAuthorityStage::ReadyForWork
    );
    Ok(())
}

fn outside_interactions(
    source: volicord_context::SourceId,
) -> Vec<volicord_inquiry::InteractionReview> {
    volicord_inquiry::InteractionAxis::ALL.into_iter().map(|axis| volicord_inquiry::InteractionReview {
        axis,
        outcomes: vec![volicord_inquiry::InteractionOutcome {
            outcome_id: format!("fixture-{axis:?}"),
            scenario: format!("The isolated fixture does not change {axis:?}; its tested authority dimension is bounded separately"),
            credible_outcomes: vec![volicord_inquiry::InteractionResult { result_id: "unchanged".into(), description: "Existing fixture interaction behavior is preserved".into() }],
            affected_choice_ids: vec![],
            conclusion: volicord_inquiry::InteractionConclusion::NoIndependentFork {
                result_id: "unchanged".into(),
                basis: volicord_inquiry::NoIndependentForkBasis::OutsideAffectedScope,
                rationale: "The fixture Source bounds this test to its declared isolated choice; interactions are unchanged".into(),
            },
            source_basis: vec![source],
        }],
    }).collect()
}

#[test]
fn interaction_partial_durability_requires_decomposition_or_source_settlement(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::{
        InteractionAxis, InteractionConclusion, InteractionOutcome, InteractionResult,
        MaterialDecomposition, ResidualInteractionComparison,
    };
    let fixture = fixture_with_repository("Add statement rejection with current documented contracts", "// Accepted contract: validate the complete input before execution; invalid input makes no durable changes.\npub fn value() -> u32 { 1 }\n")?;
    let source = fixture.baseline.repository_source.identity();
    let parent = engineering_choice(
        "rejection-policy",
        EngineeringEffectCategory::FailureOrErrorSemantics,
        source,
    );
    let mut child = engineering_choice(
        "batch-durability",
        EngineeringEffectCategory::PersistenceOrLifetime,
        source,
    );
    child.alternatives[0].summary =
        "Execute each validated statement, retaining earlier writes when a later statement fails"
            .into();
    child.alternatives[1].summary =
        "Prevalidate complete input; a later invalid statement prevents every write".into();
    let mut interactions = outside_interactions(source);
    interactions
        .iter_mut()
        .find(|r| r.axis == InteractionAxis::MultiItemEffects)
        .ok_or("axis")?
        .outcomes = vec![InteractionOutcome {
        outcome_id: "durable-prefix".into(),
        scenario: "A safe statement is followed by an unsafe statement".into(),
        credible_outcomes: vec![
            InteractionResult {
                result_id: "prefix-committed".into(),
                description: "Safe statement persists; unsafe statement rejected".into(),
            },
            InteractionResult {
                result_id: "nothing-committed".into(),
                description: "Whole input rejected with no durable change".into(),
            },
        ],
        affected_choice_ids: vec!["rejection-policy".into(), "batch-durability".into()],
        conclusion: InteractionConclusion::RepresentedByChoices {
            choice_ids: vec!["batch-durability".into()],
        },
        source_basis: vec![source],
    }];
    let compare = |result: &str| {
        ResidualInteractionComparison { outcome_id: "durable-prefix".into(), implementation_outcome_ids: vec![result.into(), result.into()], equivalence_rationale: "Inline and helper implementations preserve the same documented durable database result for this scenario".into() }
    };
    for (index, alt) in child.alternatives.iter_mut().enumerate() {
        if let MaterialDecomposition::MateriallyAtomic {
            residual_fork_closure,
            ..
        } = &mut alt.material_decomposition
        {
            residual_fork_closure.interaction_comparisons = vec![compare(if index == 0 {
                "prefix-committed"
            } else {
                "nothing-committed"
            })];
        }
    }
    let record = |choices: Vec<EngineeringChoice>, interaction_review| {
        fixture
            .operations
            .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
                project_id: fixture.project_id,
                goal_context_id: fixture.goal_id,
                baseline_analysis_snapshot_id: fixture.baseline.identity,
                session: "interaction-review".into(),
                source_operation: "partial-durability challenge".into(),
                summary: "Challenge durable results independently of rejection".into(),
                material_boundary_review: complete_material_boundary_review(&choices, source),
                choices,
                interaction_review,
            })
    };
    let mut broad = parent.clone();
    for alt in &mut broad.alternatives {
        if let MaterialDecomposition::MateriallyAtomic {
            residual_fork_closure,
            ..
        } = &mut alt.material_decomposition
        {
            residual_fork_closure.interaction_comparisons = vec![compare("nothing-committed")];
        }
    }
    let error = record(vec![broad.clone(), child.clone()], interactions.clone())
        .expect_err("hard rejection alone cannot absorb batch durability");
    assert!(error.message().contains("independent interaction outcome"));
    let mut swallowed = interactions.clone();
    swallowed[2].outcomes[0].affected_choice_ids = vec![parent.choice_id.clone()];
    swallowed[2].outcomes[0].conclusion = InteractionConclusion::RepresentedByChoices {
        choice_ids: vec![parent.choice_id.clone()],
    };
    let missing_result = record(vec![broad.clone()], swallowed).expect_err(
        "both rejection alternatives cannot silently omit the credible partial-durability result",
    );
    assert!(missing_result
        .message()
        .contains("every declared independent interaction result"));
    let mut decomposed = parent.clone();
    for alt in &mut decomposed.alternatives {
        alt.material_decomposition = MaterialDecomposition::Decomposed {
            choice_ids: vec![child.choice_id.clone()],
        };
    }
    let mut research = child.clone();
    research.alternatives.clear();
    research.evidence_state = EngineeringChoiceEvidenceState::ResearchRequired;
    record(vec![decomposed.clone(), research], interactions.clone())?;
    assert!(
        fixture
            .operations
            .workflow_after_analysis(fixture.project_id, fixture.baseline.identity)?
            .blocks_ordinary_work
    );
    let accepted = record(vec![decomposed, child.clone()], interactions.clone())?;
    let retained = fixture
        .operations
        .inspect_workflow_candidate(fixture.project_id, accepted.discovery_candidate_id)?;
    assert_eq!(
        retained
            .content
            .ok_or("content")?
            .engineering_choice_discovery
            .ok_or("discovery")?
            .interaction_review,
        interactions
    );
    // A discovery represents the unresolved dimension but does not grant work authority.
    assert!(
        fixture
            .operations
            .workflow_after_analysis(fixture.project_id, fixture.baseline.identity)?
            .blocks_ordinary_work
    );
    let mut divergent = child.clone();
    if let MaterialDecomposition::MateriallyAtomic {
        residual_fork_closure,
        ..
    } = &mut divergent.alternatives[0].material_decomposition
    {
        residual_fork_closure.interaction_comparisons[0].implementation_outcome_ids[1] =
            "nothing-committed".into();
    }
    let mut child_only = interactions.clone();
    child_only[2].outcomes[0].affected_choice_ids = vec![child.choice_id.clone()];
    assert!(
        record(vec![divergent], child_only).is_err(),
        "materially different durable results cannot be atomic"
    );
    // Current repository Source explicitly fixes the same result; no artificial subordinate Question.
    let settled = &mut interactions[2].outcomes[0];
    settled.affected_choice_ids = vec![parent.choice_id.clone()];
    settled.conclusion = InteractionConclusion::NoIndependentFork {
        basis: volicord_inquiry::NoIndependentForkBasis::SettledByCurrentSources,
        result_id: "nothing-committed".into(), rationale: "The current repository contract requires complete validation before execution and no durable changes on invalid input".into(),
    };
    let settled_record = record(vec![broad.clone()], interactions.clone())?;
    assert_eq!(
        fixture
            .operations
            .inspect_workflow_candidate(fixture.project_id, settled_record.discovery_candidate_id)?
            .kind,
        volicord_inquiry::CandidateKind::EngineeringChoiceDiscovery
    );
    let reviewed = fixture
        .operations
        .record_materiality_review(MaterialityReviewDraft {
            project_id: fixture.project_id,
            goal_context_id: fixture.goal_id,
            baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "interaction-review".into(),
            source_operation: "source-settled interaction authority".into(),
            rationale: "The repository contract explicitly requires whole-input atomic validation"
                .into(),
            behavioral_context_basis: volicord_operations::BehavioralContextBasis {
                context_item_ids: vec![],
                completeness_rationale: "No additional behavioral context in this fixture".into(),
            },
            learning_participation: LearningParticipation::Inactive,
            engineering_choice_discovery_candidate_id: settled_record.discovery_candidate_id,
            dimensions: vec![dimension(
                "rejection-policy",
                MaterialityDisposition::SettledAuthority,
                vec![WorkAuthorityBasisKind::AcceptedContract],
                source,
            )],
        })?;
    let mut atomic_plan = coupled_artifact_review(&["src/lib.rs"]);
    atomic_plan.materiality_closure = volicord_inquiry::PreWriteMaterialityClosure::NoNewMaterialOutcome {
        commitments: vec![volicord_inquiry::PlannedCommitment {
                temporal_effect: volicord_inquiry::PlannedTemporalEffect::NoTemporalChange { rationale: "This fixture commitment preserves temporal behavior and makes no timestamp or lifetime selection.".into() },
            commitment_id: "atomic-input".into(), description: "Whole-input prevalidation preserves zero durable writes for safe-then-unsafe input".into(), repository_paths: vec!["src/lib.rs".into()],
            outcome_binding: volicord_inquiry::PlannedOutcomeBinding::ReviewedInteraction { outcome_id: "durable-prefix".into(), result_id: "nothing-committed".into() },
        }], rationale: "Current repository Source explicitly fixes the durable result".into(),
    };
    fixture.operations.bind_executable_work_scope(
        fixture.project_id,
        fixture.goal_id,
        fixture.baseline.identity,
        reviewed.review_candidate_id,
        ApplicabilityScope {
            paths: vec!["src/lib.rs".into()],
            components: vec![],
            work_contexts: vec![],
        },
        atomic_plan,
    )?;
    assert!(!readiness(&fixture, &reviewed)?.blocking);
    for defect in [
        "missing-axis",
        "duplicate-outcome",
        "unknown-source",
        "missing-comparison",
        "wrong-result",
    ] {
        let mut reviews = interactions.clone();
        let mut choice = broad.clone();
        match defect {
            "missing-axis" => {
                reviews.pop();
            }
            "duplicate-outcome" => {
                reviews[0].outcomes[0].outcome_id = "durable-prefix".into();
            }
            "unknown-source" => {
                reviews[2].outcomes[0].source_basis =
                    vec![volicord_context::SourceId::from_bytes([255; 16])];
            }
            "missing-comparison" => {
                if let MaterialDecomposition::MateriallyAtomic {
                    residual_fork_closure,
                    ..
                } = &mut choice.alternatives[0].material_decomposition
                {
                    residual_fork_closure.interaction_comparisons.clear();
                }
            }
            _ => {
                if let MaterialDecomposition::MateriallyAtomic {
                    residual_fork_closure,
                    ..
                } = &mut choice.alternatives[0].material_decomposition
                {
                    residual_fork_closure.interaction_comparisons =
                        vec![compare("prefix-committed")];
                }
            }
        }
        assert!(record(vec![choice], reviews).is_err(), "{defect}");
    }
    // Equivalent private implementation and outside-scope axes remain closed without user ownership classification.
    interactions[2].outcomes[0].conclusion = InteractionConclusion::NoIndependentFork { basis: volicord_inquiry::NoIndependentForkBasis::MechanicallyEquivalent, result_id: "nothing-committed".into(), rationale: "Direct prevalidation and a private validation helper both preserve the documented no-write result".into() };
    record(vec![broad], interactions)?;
    Ok(())
}

#[test]
fn planned_commitment_graph_binding_is_prospective_and_unmapped_durability_rediscoverable(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::{PlannedCommitment, PlannedOutcomeBinding, PreWriteMaterialityClosure};
    for case in [
        "mapped",
        "private",
        "unmapped-durability",
        "unreviewed-temporal",
        "wrong-axis-temporal",
        "temporal-private",
        "eliminated-alternative",
        "wrong-dimension",
        "unreviewed-source-result",
        "uncovered-path",
    ] {
        let fixture = fixture()?;
        let source = fixture.baseline.repository_source.identity();
        let reviewed = review(
            &fixture,
            vec![dimension(
                "rejection-policy",
                MaterialityDisposition::SettledAuthority,
                vec![WorkAuthorityBasisKind::AcceptedContract],
                source,
            )],
        )?;
        let mut plan = coupled_artifact_review(&["src/lib.rs"]);
        let mut commitment = PlannedCommitment {
                temporal_effect: volicord_inquiry::PlannedTemporalEffect::NoTemporalChange { rationale: "This fixture commitment preserves temporal behavior and makes no timestamp or lifetime selection.".into() },
            commitment_id: "planned-result".into(),
            description:
                "Preserve the reviewed hard rejection policy in implementation and its assertions"
                    .into(),
            repository_paths: vec!["src/lib.rs".into()],
            outcome_binding: PlannedOutcomeBinding::ReviewedChoice {
                dimension_id: "rejection-policy".into(),
                choice_id: "rejection-policy".into(),
                alternative_id: "approach-a".into(),
            },
        };
        match case {
            "private" => commitment.outcome_binding = PlannedOutcomeBinding::PrivateEquivalent { equivalence_rationale: "A private helper extracts the same predicate; all current reviewed results remain identical".into() },
            "unmapped-durability" => { commitment.description = "Prevalidate a whole input containing safe then unsafe statements, leaving no durable database changes".into(); commitment.outcome_binding = PlannedOutcomeBinding::ReviewedInteraction { outcome_id: "batch-durability".into(), result_id: "nothing-committed".into() }; }
            "eliminated-alternative" => if let PlannedOutcomeBinding::ReviewedChoice { alternative_id, .. } = &mut commitment.outcome_binding { *alternative_id = "approach-b".into(); },
            "wrong-dimension" => if let PlannedOutcomeBinding::ReviewedChoice { dimension_id, .. } = &mut commitment.outcome_binding { *dimension_id = "unrelated-activation".into(); },
            "unreviewed-source-result" => commitment.outcome_binding = PlannedOutcomeBinding::ReviewedInteraction { outcome_id: "fixture-MultiItemEffects".into(), result_id: "unchanged".into() },
            "unreviewed-temporal" | "wrong-axis-temporal" | "temporal-private" => {
                commitment.description = "Preserve the original timestamp and lifetime during reissue".into();
                commitment.temporal_effect = volicord_inquiry::PlannedTemporalEffect::ReviewedTemporalOutcome {
                    outcome_id: if case == "wrong-axis-temporal" { "fixture-MultiItemEffects".into() } else { "unreviewed-lifetime".into() }, result_id: "preserved".into(),
                };
                if case == "temporal-private" { commitment.outcome_binding = PlannedOutcomeBinding::PrivateEquivalent { equivalence_rationale: "A private label cannot authorize this temporal commitment".into() }; }
            },
            "uncovered-path" => commitment.repository_paths.clear(),
            _ => (),
        }
        plan.materiality_closure = PreWriteMaterialityClosure::NoNewMaterialOutcome {
            commitments: vec![commitment],
            rationale: "Concrete plan closure against the current review".into(),
        };
        let result = fixture.operations.bind_executable_work_scope(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            reviewed.review_candidate_id,
            ApplicabilityScope {
                paths: vec!["src/lib.rs".into()],
                components: vec![],
                work_contexts: vec![],
            },
            plan,
        );
        if case == "uncovered-path" {
            assert!(result.is_err());
            continue;
        }
        result?;
        let reopened = LocalOperations::new(fixture.operations.layout().clone());
        let retained = reopened
            .inspect_workflow_candidate(fixture.project_id, reviewed.review_candidate_id)?
            .content
            .ok_or("content")?
            .materiality_review
            .ok_or("review")?;
        if matches!(case, "mapped" | "private") {
            assert!(retained.executable_work_scope.is_some());
            assert!(!readiness(&fixture, &reviewed)?.blocking);
        } else {
            assert!(retained.executable_work_scope.is_none(), "{case}");
            let pending = retained.pending_pre_write_reassessment.ok_or("pending")?;
            assert!(matches!(
                pending.coupled_artifact_review.materiality_closure,
                PreWriteMaterialityClosure::NewMaterialOutcome { .. }
            ));
            let workflow = reopened
                .workflow_for_review_candidate(fixture.project_id, reviewed.review_candidate_id)?;
            assert_eq!(workflow.stage, WorkflowStage::EngineeringChoiceDiscovery);
            assert!(workflow.blocks_ordinary_work);
            if case == "unreviewed-temporal" {
                fs::write(fixture.repository.join("src/lib.rs"), "// A later contract requires original timestamp preservation.\npub fn value() -> u32 { 2 }\n")?;
                assert!(
                    review(
                        &fixture,
                        vec![dimension(
                            "lifetime",
                            MaterialityDisposition::SettledAuthority,
                            vec![WorkAuthorityBasisKind::AcceptedContract],
                            source
                        )]
                    )
                    .is_err(),
                    "later temporal authority cannot retroactively cover earlier baseline writes"
                );
            }
        }
    }
    assert!(serde_json::from_value::<PreWriteMaterialityClosure>(serde_json::json!({"state":"no_new_material_outcome","reviewed_outcomes":["hard reject"],"rationale":"old prose-only shape"})).is_err());
    Ok(())
}

#[test]
fn timed_key_rotation_reviews_independent_lifetime_and_binds_temporal_commitments(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_inquiry::{
        InteractionAxis, InteractionConclusion, InteractionOutcome, InteractionResult,
        MaterialDecomposition, NoIndependentForkBasis, PlannedOutcomeBinding,
        PlannedTemporalEffect, PreWriteMaterialityClosure, ResidualInteractionComparison,
    };
    for case in [
        "unresolved",
        "source-settled",
        "private-equivalent",
        "no-change-bypass",
    ] {
        let contract = if case == "unresolved" {
            "// Rotation: an old key validates; the newest key re-signs. Timestamp/lifetime policy is unspecified.\npub fn value() -> u32 { 1 }\n"
        } else {
            "// Rotation: an old key validates; the newest key re-signs. Preserve the original timestamp and expiration: rotation must not renew lifetime. Decode/re-encode and byte-copy techniques produce identical age/expiry.\npub fn value() -> u32 { 1 }\n"
        };
        let fixture = fixture_with_repository(
            "Add timed-key rotation under the current accepted contract",
            contract,
        )?;
        let source = fixture.baseline.repository_source.identity();
        let mut trigger = engineering_choice(
            "replacement-trigger",
            EngineeringEffectCategory::Security,
            source,
        );
        trigger.alternatives[0].summary = "Old key validates and newest key re-signs".into();
        trigger.alternatives[1].summary = "Re-sign only when the newest key validates".into();
        if case == "private-equivalent" {
            trigger.alternatives[0].summary =
                "Decode and re-encode the original timestamp with the new signature".into();
            trigger.alternatives[1].summary =
                "Copy the verified timestamp bytes with the new signature".into();
        }
        let mut lifetime = engineering_choice(
            "rotation-lifetime",
            EngineeringEffectCategory::PersistenceOrLifetime,
            source,
        );
        lifetime.alternatives[0].summary =
            "Preserve the original timestamp, effective age, and expiration".into();
        lifetime.alternatives[1].summary =
            "Reset the timestamp to rotation time, renewing the expiration".into();
        let comparison = |result: &str| {
            ResidualInteractionComparison {
            outcome_id: "rotation-age".into(), implementation_outcome_ids: vec![result.into(), result.into()],
            equivalence_rationale: "Inline and helper implementations of this alternative give the same effective age and expiration on old-key validation and re-signing, including retry.".into(),
        }
        };
        for (index, alternative) in lifetime.alternatives.iter_mut().enumerate() {
            if let MaterialDecomposition::MateriallyAtomic {
                residual_fork_closure,
                ..
            } = &mut alternative.material_decomposition
            {
                residual_fork_closure.fixed_outcome = alternative.summary.clone();
                residual_fork_closure.interaction_comparisons =
                    vec![comparison(if index == 0 { "preserved" } else { "renewed" })];
            }
        }
        let mut interactions = outside_interactions(source);
        let temporal = interactions
            .iter_mut()
            .find(|review| review.axis == InteractionAxis::TemporalAndLifetime)
            .ok_or("temporal axis")?;
        temporal.outcomes = vec![InteractionOutcome {
            outcome_id: "rotation-age".into(),
            scenario: "An old-key token near expiry validates and is re-signed by the newest key; rotation and retries may preserve or reset its timestamp and effective lifetime independently of the replacement trigger.".into(),
            credible_outcomes: vec![InteractionResult { result_id: "preserved".into(), description: "Original timestamp/age/expiry survive replacement and retries".into() }, InteractionResult { result_id: "renewed".into(), description: "Rotation/retry resets issuance time and extends validity".into() }],
            affected_choice_ids: if case == "unresolved" { vec![trigger.choice_id.clone(), lifetime.choice_id.clone()] } else { vec![trigger.choice_id.clone()] },
            conclusion: if case == "unresolved" { InteractionConclusion::RepresentedByChoices { choice_ids: vec![lifetime.choice_id.clone()] } } else { InteractionConclusion::NoIndependentFork {
                basis: if case == "source-settled" { NoIndependentForkBasis::SettledByCurrentSources } else { NoIndependentForkBasis::MechanicallyEquivalent },
                rationale: "The current contract requires the original timestamp and expiration; decoding/re-encoding the original time and copying its bytes preserve identical validity.".into(), result_id: "preserved".into(),
            } }, source_basis: vec![source],
        }];
        let record = |choices: Vec<EngineeringChoice>, interaction_review| {
            fixture.operations.record_engineering_choice_discovery(
                EngineeringChoiceDiscoveryDraft {
                    project_id: fixture.project_id,
                    goal_context_id: fixture.goal_id,
                    baseline_analysis_snapshot_id: fixture.baseline.identity,
                    session: "timed-rotation".into(),
                    source_operation: "temporal completeness challenge".into(),
                    summary: "Review trigger and lifetime separately".into(),
                    material_boundary_review: complete_material_boundary_review(&choices, source),
                    choices,
                    interaction_review,
                },
            )
        };
        // A broad trigger cannot claim atomicity while a separate lifetime fork remains.
        for alternative in &mut trigger.alternatives {
            if let MaterialDecomposition::MateriallyAtomic {
                residual_fork_closure,
                ..
            } = &mut alternative.material_decomposition
            {
                residual_fork_closure.interaction_comparisons = vec![comparison("preserved")];
            }
        }
        if case == "unresolved" {
            assert!(record(
                vec![trigger.clone(), lifetime.clone()],
                interactions.clone()
            )
            .is_err());
            let mut swallowed = interactions.clone();
            let outcome = &mut swallowed[4].outcomes[0];
            outcome.affected_choice_ids = vec![trigger.choice_id.clone()];
            outcome.conclusion = InteractionConclusion::RepresentedByChoices {
                choice_ids: vec![trigger.choice_id.clone()],
            };
            assert!(
                record(vec![trigger.clone()], swallowed).is_err(),
                "trigger alternatives omit the credible renewed result"
            );
            let mut divergent = lifetime.clone();
            if let MaterialDecomposition::MateriallyAtomic {
                residual_fork_closure,
                ..
            } = &mut divergent.alternatives[0].material_decomposition
            {
                residual_fork_closure.interaction_comparisons[0].implementation_outcome_ids[1] =
                    "renewed".into();
            }
            let mut child_only = interactions.clone();
            child_only[4].outcomes[0].affected_choice_ids = vec![lifetime.choice_id.clone()];
            assert!(
                record(vec![divergent], child_only).is_err(),
                "preserve and reset are not materially atomic"
            );
            for alternative in &mut trigger.alternatives {
                alternative.material_decomposition = MaterialDecomposition::Decomposed {
                    choice_ids: vec![lifetime.choice_id.clone()],
                };
            }
        }
        let mut choices = vec![trigger.clone()];
        let mut dimensions = vec![dimension(
            "replacement-trigger",
            MaterialityDisposition::SettledAuthority,
            vec![WorkAuthorityBasisKind::AcceptedContract],
            source,
        )];
        if case == "unresolved" {
            choices.push(lifetime);
            dimensions.push(dimension(
                "rotation-lifetime",
                MaterialityDisposition::UnresolvedUserOwnedOutcome {
                    resolution_decision_id: None,
                },
                vec![WorkAuthorityBasisKind::NoSettlingAuthority],
                source,
            ));
        }
        if case == "private-equivalent" {
            dimensions[0] = agent_owned_dimension(
                "replacement-trigger",
                source,
                LearningValueAssessment::Routine {
                    rationale: "Equivalent ways to preserve the fixed temporal behavior".into(),
                },
            );
        }
        let discovery = record(choices.clone(), interactions.clone())?;
        let reviewed = fixture.operations.record_materiality_review(MaterialityReviewDraft {
            project_id: fixture.project_id, goal_context_id: fixture.goal_id, baseline_analysis_snapshot_id: fixture.baseline.identity,
            session: "timed-rotation".into(), source_operation: "temporal authority".into(), rationale: "Trigger authority does not choose lifetime; only exact current Sources or a separate product Decision can settle that outcome.".into(),
            behavioral_context_basis: volicord_operations::BehavioralContextBasis { context_item_ids: vec![], completeness_rationale: "No learning or other behavioral Context".into() },
            learning_participation: LearningParticipation::Inactive, engineering_choice_discovery_candidate_id: discovery.discovery_candidate_id, dimensions,
        })?;
        let mut plan = coupled_artifact_review(&["src/lib.rs"]);
        let commitment = volicord_inquiry::PlannedCommitment {
            commitment_id: "preserve-original-time".into(), description: "Preserve the original timestamp and expiration while re-signing with the newest key".into(), repository_paths: vec!["src/lib.rs".into()],
            temporal_effect: PlannedTemporalEffect::ReviewedTemporalOutcome { outcome_id: "rotation-age".into(), result_id: "preserved".into() },
            outcome_binding: PlannedOutcomeBinding::ReviewedInteraction { outcome_id: "rotation-age".into(), result_id: "preserved".into() },
        };
        plan.materiality_closure = PreWriteMaterialityClosure::NoNewMaterialOutcome {
            commitments: vec![commitment.clone()],
            rationale: "This planned temporal result must use the reviewed temporal authority"
                .into(),
        };
        fixture.operations.bind_executable_work_scope(
            fixture.project_id,
            fixture.goal_id,
            fixture.baseline.identity,
            reviewed.review_candidate_id,
            ApplicabilityScope {
                paths: vec!["src/lib.rs".into()],
                components: vec![],
                work_contexts: vec![],
            },
            plan.clone(),
        )?;
        let ready = readiness(&fixture, &reviewed)?;
        if case == "unresolved" {
            assert!(ready.blocking);
            assert_eq!(ready.stage, WorkAuthorityStage::QuestionRequired);
            assert!(ready
                .unresolved_requirements
                .iter()
                .any(
                    |requirement| requirement.dimension_id.as_deref() == Some("rotation-lifetime")
                ));
        } else {
            assert!(!ready.blocking, "{case}: {ready:?}");
            assert!(fixture
                .operations
                .canonical_basis(fixture.project_id)?
                .active_questions
                .is_empty());
            // An exact choice binding is valid only when its atomic temporal comparison agrees.
            let mut by_choice = commitment.clone();
            by_choice.outcome_binding = PlannedOutcomeBinding::ReviewedChoice {
                dimension_id: "replacement-trigger".into(),
                choice_id: "replacement-trigger".into(),
                alternative_id: "approach-a".into(),
            };
            if let PreWriteMaterialityClosure::NoNewMaterialOutcome { commitments, .. } =
                &mut plan.materiality_closure
            {
                *commitments = vec![by_choice];
            }
            fixture.operations.bind_executable_work_scope(
                fixture.project_id,
                fixture.goal_id,
                fixture.baseline.identity,
                reviewed.review_candidate_id,
                ApplicabilityScope {
                    paths: vec!["src/lib.rs".into()],
                    components: vec![],
                    work_contexts: vec![],
                },
                plan.clone(),
            )?;
            assert!(!readiness(&fixture, &reviewed)?.blocking);
            // Reset contradicts the source-settled preserve result and revokes executable authority.
            let mut reset = commitment;
            reset.temporal_effect = PlannedTemporalEffect::ReviewedTemporalOutcome {
                outcome_id: "rotation-age".into(),
                result_id: "renewed".into(),
            };
            if case == "no-change-bypass" {
                reset.temporal_effect = PlannedTemporalEffect::NoTemporalChange { rationale: "A known temporal commitment cannot bypass its reviewed result by claiming no temporal change".into() };
            }
            if let PreWriteMaterialityClosure::NoNewMaterialOutcome { commitments, .. } =
                &mut plan.materiality_closure
            {
                *commitments = vec![reset];
            }
            fixture.operations.bind_executable_work_scope(
                fixture.project_id,
                fixture.goal_id,
                fixture.baseline.identity,
                reviewed.review_candidate_id,
                ApplicabilityScope {
                    paths: vec!["src/lib.rs".into()],
                    components: vec![],
                    work_contexts: vec![],
                },
                plan,
            )?;
            let restarted = LocalOperations::new(fixture.operations.layout().clone());
            let current = restarted
                .inspect_workflow_candidate(fixture.project_id, reviewed.review_candidate_id)?
                .content
                .ok_or("content")?
                .materiality_review
                .ok_or("review")?;
            assert!(current.executable_work_scope.is_none());
            assert!(current.pending_pre_write_reassessment.is_some());
        }
        let mut missing = interactions.clone();
        missing.pop();
        assert!(
            record(choices, missing).is_err(),
            "the temporal axis is mandatory even when its conclusion is closed"
        );
    }
    Ok(())
}
