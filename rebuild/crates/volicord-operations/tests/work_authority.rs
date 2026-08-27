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
    CommandVerificationDraft, ExplicitDelegationEvidence, ExploratoryDisposition,
    GroundedCheckpointDraft, LocalOperations, MaterialOutcomeSignal, MaterialityDimension,
    MaterialityDisposition, MaterialityReviewDraft, MaterialityReviewRevisionDraft, RuntimeLayout,
    WorkAuthorityBasis, WorkAuthorityBasisKind, WorkAuthorityDisposition, WorkAuthorityStage,
};

fn dimension(
    id: &str,
    disposition: MaterialityDisposition,
    kinds: Vec<WorkAuthorityBasisKind>,
    source: volicord_context::SourceId,
) -> MaterialityDimension {
    MaterialityDimension {
        dimension_id: id.to_owned(),
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
    }
}

fn delegation_evidence(
    fixture: &Fixture,
    verbatim_statement: &str,
    affected_scope: Vec<String>,
) -> ExplicitDelegationEvidence {
    ExplicitDelegationEvidence {
        goal_context_id: fixture.goal_id,
        user_turn_source_id: fixture.goal_source_id,
        verbatim_statement: verbatim_statement.to_owned(),
        affected_scope,
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
            dimensions,
        })
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
        "choose the internal module naming and structure",
        vec!["src/lib.rs".to_owned()],
    ));
    let recorded = review(&fixture, vec![delegated])?;
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
        affected_scope: vec!["src/lib.rs".to_owned()],
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
            dimensions: vec![exploratory],
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
            materiality_review: None,
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
            dimensions: vec![resolved, resolved_coupled],
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
