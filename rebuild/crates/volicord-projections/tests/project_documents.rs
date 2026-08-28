use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadBasis,
    CanonicalReadOptions, CheckpointDraft, CheckpointKind, ContextItemDraft, ContextItemRole,
    DecisionSupersessionDraft, DeterministicIdGenerator, ExplicitQuestionResponse, FixedClock,
    NonUserQuestionOutcome, OperationId, Principal, PrincipalKind, Project, ProjectId,
    QuestionAlternative, QuestionDraft, QuestionMateriality, QuestionResearchState,
    QuestionResponseDraft, Source, SourceDraft, SourcePayload, StatementProvenanceRole, Store,
    TimestampMicros, UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState,
    UserTurnSource, VerificationFact, VerificationState, WorkState,
};
use volicord_inquiry::{
    ApplicabilityQuery, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDraft, CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateReadBasis,
    CandidateRetention, CandidateStore, SubmissionOutcome,
};
use volicord_projections::{
    build_project_projection, build_project_understanding, generate_documents,
    prepare_narrative_plan, realize_narrative, BriefDecisionState, CandidateContentAccess,
    CandidateDependencyFailure, CandidateDependencyFailureKind, CandidateDependencyState,
    CandidateProjectionInput, CanonicalInspectionKind, ClaimClass, DocumentKind, DocumentRequest,
    FixedLocale, GeneratorIdentity, MapRelationClass, NarrativeRealization,
    NarrativeRealizationState, OutputFormat, ProjectProjection, ProjectProjectionInputs,
    ProjectionBound, ProjectionHealth, ProjectionIssueKind, RealizedNarrativeClaim,
    RealizedNarrativeSection, RequestedDestination, UnderstandingBound, UnderstandingEvidenceClass,
    UnderstandingExplanationKind, GENERATED_DOCUMENT_METADATA_VERSION,
    NARRATIVE_PLAN_PROTECTED_TERM_BYTE_LIMIT, NARRATIVE_PLAN_PROTECTED_TERM_LIMIT,
    NARRATIVE_PLAN_SOURCE_TEXT_BYTE_LIMIT, RENDERED_DOCUMENT_FIELD_BYTE_LIMIT,
    RENDERED_HTML_BYTE_LIMIT, RENDERED_MARKDOWN_BYTE_LIMIT,
};
use volicord_repository_intelligence::{
    analyze_repository_semantics, AgentInterpretation, AnalysisSnapshot, CanonicalGrounding,
    Capability, CapabilityState, InventoryRequest, ProvenanceClass, SemanticAnalysisRequest,
    StructuralAnalysisRequest, Uncertainty, UncertaintyLevel,
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

fn source_draft(project: &Project, payload: SourcePayload) -> SourceDraft {
    SourceDraft {
        expected_project_revision: project.revision,
        payload,
        actor: principal(PrincipalKind::Repository, "repository"),
        observer: Some(principal(PrincipalKind::Agent, "codex")),
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
                consequence: "keep analysis local".to_owned(),
            },
            QuestionAlternative {
                key: "remote".to_owned(),
                label: "Remote".to_owned(),
                consequence: "use a remote service".to_owned(),
            },
        ],
        recommendation: AgentRecommendation {
            alternative_key: Some("local".to_owned()),
            rationale: "local structural evidence remains available".to_owned(),
            source_basis: vec![source.id],
        },
        trade_offs: vec!["remote semantic depth may differ".to_owned()],
        uncertainty: vec!["runtime behavior is not observed".to_owned()],
        material_scope: vec!["analysis".to_owned()],
        materiality: QuestionMateriality::Material,
        presentation_order: order,
        why_it_matters_now: "the implementation needs a stable analysis boundary".to_owned(),
        established_facts: Vec::new(),
        assumptions: vec!["local-first".to_owned()],
        known_limits: vec!["runtime behavior is outside static analysis".to_owned()],
        what_the_answer_unlocks: vec!["document generation".to_owned()],
        allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(path);
            }
        }
    }
    files
}

fn build_projection_fixture(
    canonical: &CanonicalReadBasis,
    candidates: &CandidateReadBasis,
    analysis: &AnalysisSnapshot,
    project_id: ProjectId,
    bound: ProjectionBound,
) -> ProjectProjection {
    build_project_projection(ProjectProjectionInputs {
        canonical,
        analyses: &[analysis],
        applicability: ApplicabilityQuery {
            project_id,
            paths: vec!["src".to_owned()],
            components: vec!["MarkdownGuide".to_owned()],
            work_contexts: vec!["documents".to_owned()],
            current_assumptions: vec!["local-first".to_owned()],
            met_revisit_triggers: Vec::new(),
        },
        candidates: CandidateProjectionInput::Available(candidates),
        candidate_content_access: CandidateContentAccess::AllowBoundedSummary,
        observed_at: TimestampMicros::from_unix_micros(30_000),
        bound,
    })
}

#[test]
fn completed_project_documents_are_human_first_and_keep_resolved_ambiguity_in_audit(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("completed-context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=40).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(50_000)),
    )?;
    let project = store
        .create_project(operation(201), "Completed Project")?
        .value;
    let user_turn = store
        .record_source(
            operation(202),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "completed-session".to_owned(),
                    turn: "Choose the local renderer".to_owned(),
                },
                actor: principal(PrincipalKind::User, "owner"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    store.record_context_item(
        operation(203),
        project.id,
        ContextItemDraft {
            expected_project_revision: project.revision,
            role: ContextItemRole::Goal,
            statement: "Ship a readable local project summary".to_owned(),
            provenance_role: StatementProvenanceRole::UserStatement,
            author: principal(PrincipalKind::User, "owner"),
            source_basis: vec![user_turn.id],
            applicability: ApplicabilityScope::default(),
        },
    )?;
    let mut draft = question_draft(
        &project,
        &user_turn,
        "Should the summary use the local or remote renderer?",
        1,
    );
    draft.uncertainty = vec!["the renderer choice is unresolved".to_owned()];
    draft.known_limits = vec!["manual readability review remains".to_owned()];
    let question = store
        .create_question(operation(204), project.id, draft)?
        .value;
    let decision = store
        .record_question_response(
            operation(205),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: project.revision,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Existing(user_turn.id),
                displayed_alternative_keys: vec!["local".to_owned(), "remote".to_owned()],
                displayed_recommendation_key: Some("local".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "local".to_owned(),
                    user_rationale: Some("the result must remain local and shareable".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["local-first".to_owned()],
                revisit_triggers: vec!["local output becomes unusable".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or("Decision missing")?;
    let verification = store
        .record_source(
            operation(206),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CommandExecution {
                    command_label: "cargo test -p completed-fixture".to_owned(),
                    invocation_fingerprint: format!("sha256:{}", "0".repeat(64)),
                    outcome: volicord_context::CommandOutcome {
                        exit_code: Some(0),
                        termination: volicord_context::CommandTermination::Exited,
                    },
                },
                actor: principal(PrincipalKind::Command, "cargo"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    store.record_checkpoint(
        operation(207),
        project.id,
        CheckpointDraft {
            expected_project_revision: project.revision,
            kind: CheckpointKind::Completion,
            goal: "Ship a readable local project summary".to_owned(),
            work_state: WorkState::Completed,
            state_change: Some("summary renderer completed".to_owned()),
            source_basis: vec![user_turn.id, verification.id],
            changed_source_basis: vec![verification.id],
            changed_paths: vec!["src/summary.rs".to_owned()],
            applied_decisions: vec![decision.id],
            verification: vec![VerificationFact {
                state: VerificationState::Passed,
                source_id: Some(verification.id),
                outcome: Some("all focused checks passed".to_owned()),
            }],
            user_review: UserReviewFact {
                state: UserReviewState::Reviewed,
                source_id: Some(user_turn.id),
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::Accepted,
                source_id: Some(user_turn.id),
            },
            known_limits: vec!["manual readability review remains".to_owned()],
            non_goals: Vec::new(),
            open_questions: Vec::new(),
            next_step: "No further work is planned for this goal".to_owned(),
            handoff_to: None,
        },
    )?;

    let canonical = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    let candidate_store = CandidateStore::open(root.path().join("completed-candidates.sqlite3"))?;
    let candidates = candidate_store.read_basis(project.id)?;
    let projection = build_project_projection(ProjectProjectionInputs {
        canonical: &canonical,
        analyses: &[],
        applicability: ApplicabilityQuery {
            project_id: project.id,
            paths: Vec::new(),
            components: Vec::new(),
            work_contexts: Vec::new(),
            current_assumptions: vec!["local-first".to_owned()],
            met_revisit_triggers: Vec::new(),
        },
        candidates: CandidateProjectionInput::Available(&candidates),
        candidate_content_access: CandidateContentAccess::AllowBoundedSummary,
        observed_at: TimestampMicros::from_unix_micros(60_000),
        bound: ProjectionBound::default(),
    });
    assert!(projection.resume.open_questions.is_empty());
    let current = projection
        .resume
        .decisions
        .iter()
        .find(|value| value.decision_id == decision.id)
        .ok_or("projected Decision missing")?;
    assert_eq!(
        current.question_uncertainty,
        ["the renderer choice is unresolved"]
    );
    assert_eq!(current.known_limits, ["manual readability review remains"]);

    let documents = generate_documents(
        &projection,
        &DocumentRequest {
            requested_language: "en".to_owned(),
            fixed_locale: FixedLocale::English,
            generated_at: TimestampMicros::from_unix_micros(70_000),
            generator: GeneratorIdentity {
                generator: "volicord-projections".to_owned(),
                agent: None,
                model: None,
            },
            requested_destinations: Vec::new(),
        },
    )?;
    for document in [
        &documents.project_architecture_guide,
        &documents.decision_report,
        &documents.implementation_plan,
        &documents.handoff_resume,
    ] {
        let markdown = &document.markdown.content;
        let appendix = markdown
            .find("## Grounding and audit appendix")
            .ok_or("Markdown audit appendix missing")?;
        assert!(markdown
            .find("Ship a readable local project summary")
            .is_some_and(|position| position < appendix));
        assert!(markdown.find("Grounding metadata").is_none());
        assert!(!markdown[..appendix].contains(&project.id.to_string()));
        assert!(!markdown[..appendix].contains("the renderer choice is unresolved"));
        assert!(markdown[..appendix].contains("manual readability review remains"));
        assert!(markdown[appendix..].contains("the renderer choice is unresolved"));
        assert!(markdown[appendix..].contains("claim="));

        let html = &document.html.content;
        let audit = html
            .find("<details class=\"audit\" data-section=\"grounding-audit\">")
            .ok_or("HTML audit disclosure missing")?;
        assert!(!html[audit..].starts_with("<details open"));
        assert!(!html[..audit].contains(&project.id.to_string()));
        assert!(!html[..audit].contains("the renderer choice is unresolved"));
        assert!(html[audit..].contains("the renderer choice is unresolved"));
        assert!(html[audit..].contains("data-claim-id"));
    }
    Ok(())
}

#[test]
fn project_surface_and_four_documents_are_grounded_equivalent_and_read_only(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let repository_root = root.path().join("repository");
    fs::create_dir_all(repository_root.join("src"))?;
    fs::write(
        repository_root.join("Cargo.toml"),
        "[package]\nname='guide-fixture'\nversion='0.1.0'\nedition='2021'\n",
    )?;
    fs::write(
        repository_root.join("src/lib.rs"),
        "pub trait Guide { fn render(&self) -> String; }\npub struct MarkdownGuide;\nimpl Guide for MarkdownGuide { fn render(&self) -> String { String::from(\"guide\") } }\n",
    )?;

    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=100).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(10_000)),
    )?;
    let project = store
        .create_project(operation(101), "Grounded Documents")?
        .value;
    let repository = store
        .record_source(
            operation(102),
            project.id,
            source_draft(
                &project,
                SourcePayload::RepositorySnapshot {
                    revision: "snapshot-guide".to_owned(),
                },
            ),
        )?
        .value;
    let user_turn = store
        .record_source(
            operation(103),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "session-docs".to_owned(),
                    turn: "turn-1".to_owned(),
                },
                actor: principal(PrincipalKind::User, "owner"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    store.record_context_item(
        operation(104),
        project.id,
        ContextItemDraft {
            expected_project_revision: project.revision,
            role: ContextItemRole::Goal,
            statement: "Explain the architecture without losing source identity".to_owned(),
            provenance_role: StatementProvenanceRole::UserStatement,
            author: principal(PrincipalKind::User, "owner"),
            source_basis: vec![user_turn.id],
            applicability: ApplicabilityScope::default(),
        },
    )?;
    let first_question = store
        .create_question(
            operation(105),
            project.id,
            question_draft(&project, &repository, "Where should rendering live?", 1),
        )?
        .value;
    let first_decision = store
        .record_question_response(
            operation(106),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: project.revision,
                question_id: first_question.id,
                question_revision: first_question.revision,
                user_turn_source: UserTurnSource::Existing(user_turn.id),
                displayed_alternative_keys: vec!["local".to_owned(), "remote".to_owned()],
                displayed_recommendation_key: Some("local".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "local".to_owned(),
                    user_rationale: Some("preserve a read-only boundary".to_owned()),
                },
                applicability: ApplicabilityScope {
                    paths: vec!["src".to_owned()],
                    components: vec!["MarkdownGuide".to_owned()],
                    work_contexts: vec!["documents".to_owned()],
                },
                assumptions: vec!["local-first".to_owned()],
                revisit_triggers: vec!["source basis changes".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or("first Decision missing")?;
    let active_decision = store
        .supersede_decision(
            operation(107),
            project.id,
            DecisionSupersessionDraft {
                expected_project_revision: project.revision,
                previous_decision_id: first_decision.id,
                user_turn_source: UserTurnSource::Existing(user_turn.id),
                choice: volicord_context::DecisionChoice::Alternative {
                    alternative_key: "local".to_owned(),
                },
                user_rationale: Some("render both portable formats from one body".to_owned()),
                applicability: first_decision.applicability.clone(),
                assumptions: first_decision.assumptions.clone(),
                revisit_triggers: first_decision.revisit_triggers.clone(),
            },
        )?
        .value;
    let open_question = store
        .create_question(
            operation(108),
            project.id,
            question_draft(&project, &repository, "Which review gap remains?", 2),
        )?
        .value;
    let verification_source = store
        .record_source(
            operation(109),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CommandExecution {
                    command_label: "cargo test -p volicord-projections".to_owned(),
                    invocation_fingerprint: format!("sha256:{}", "0".repeat(64)),
                    outcome: volicord_context::CommandOutcome {
                        exit_code: Some(0),
                        termination: volicord_context::CommandTermination::Exited,
                    },
                },
                actor: principal(PrincipalKind::Command, "cargo"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    store.record_checkpoint(
        operation(110),
        project.id,
        CheckpointDraft {
            expected_project_revision: project.revision,
            kind: CheckpointKind::Handoff,
            goal: "Generate four grounded documents".to_owned(),
            work_state: WorkState::Completed,
            state_change: Some("read projection implemented".to_owned()),
            source_basis: vec![repository.id],
            changed_source_basis: vec![repository.id],
            changed_paths: vec!["src/lib.rs".to_owned()],
            applied_decisions: vec![active_decision.id],
            verification: vec![VerificationFact {
                state: VerificationState::Passed,
                source_id: Some(verification_source.id),
                outcome: Some("focused tests passed".to_owned()),
            }],
            user_review: UserReviewFact {
                state: UserReviewState::Pending,
                source_id: None,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::NotRequested,
                source_id: None,
            },
            known_limits: vec!["runtime-only behavior is not analyzed".to_owned()],
            non_goals: vec!["no viewer server".to_owned()],
            open_questions: vec![volicord_context::QuestionReference {
                question_id: open_question.id,
                revision: open_question.revision,
            }],
            next_step: "review source-grounding gaps".to_owned(),
            handoff_to: Some("next agent".to_owned()),
        },
    )?;

    let canonical = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    let canonical_before = canonical.clone();
    let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, mut analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(InventoryRequest::new(
            &repository_root,
            &grounding,
            repository.id,
            20_000,
        )?),
    ))?;
    let semantic_report = analysis
        .capabilities
        .iter_mut()
        .find(|report| report.capability == Capability::Semantic)
        .ok_or("semantic report missing")?;
    semantic_report.state = CapabilityState::Partial;
    semantic_report.reason = Some("one analyzer scope is intentionally partial".to_owned());
    semantic_report.usable_remainder = Some("local definitions remain available".to_owned());
    analysis.agent_interpretations.push(AgentInterpretation {
        identity: "interpretation:guide-boundary".to_owned(),
        analysis_snapshot: analysis.identity,
        agent: "codex".to_owned(),
        host: "codex".to_owned(),
        session: "session-docs".to_owned(),
        source_basis: vec![analysis.repository_source.clone()],
        analysis_basis: analysis
            .structural_facts
            .iter()
            .take(1)
            .map(|fact| fact.entity.identity.clone())
            .collect(),
        text: "The Guide boundary appears to separate rendering from canonical state.".to_owned(),
        generated_at_unix_micros: 20_000,
        known_gaps: vec!["runtime mutation behavior was not observed".to_owned()],
        uncertainty: Uncertainty {
            level: UncertaintyLevel::Medium,
            reasons: vec!["architecture meaning is inferred from source structure".to_owned()],
        },
        provenance_class: ProvenanceClass::AgentInterpretation,
    });

    let mut candidate_store = CandidateStore::open_with(
        root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[111; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(30_000)),
    )?;
    match candidate_store.submit(CandidateDraft {
        project_id: project.id,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: principal(PrincipalKind::Agent, "codex"),
            subsystem: "repository-intelligence".to_owned(),
            session: Some("session-docs".to_owned()),
            provenance_summary: "bounded architecture observation".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id: project.id,
            session: Some("session-docs".to_owned()),
            source_operation: Some("analysis".to_owned()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![repository.id],
            repository_snapshot: Some(analysis.repository_snapshot.to_string()),
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(25_000),
        retention: CandidateRetention {
            retained_until: None,
            basis: "explicit test retention".to_owned(),
        },
        content: CandidateContent {
            bounded_summary: "architecture candidate".to_owned(),
            question: None,
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
        },
    })? {
        SubmissionOutcome::Stored(_) => {}
        SubmissionOutcome::CollectionDisabled { .. } => return Err("collection disabled".into()),
    }
    let candidates = candidate_store.read_basis(project.id)?;
    let candidates_before = candidates.clone();
    let projection = build_projection_fixture(
        &canonical,
        &candidates,
        &analysis,
        project.id,
        ProjectionBound::default(),
    );
    assert_eq!(
        projection,
        build_projection_fixture(
            &canonical,
            &candidates,
            &analysis,
            project.id,
            ProjectionBound::default(),
        )
    );
    assert_eq!(projection.overview.project_id, project.id);
    assert_ne!(projection.health, ProjectionHealth::Complete);
    assert!(!projection.repository_map.entities.is_empty());
    assert!(projection
        .repository_map
        .relations
        .iter()
        .any(|relation| relation.class == MapRelationClass::StructuralFact));
    assert!(projection
        .repository_map
        .relations
        .iter()
        .any(|relation| relation.class == MapRelationClass::SemanticResult));
    assert!(projection
        .resume
        .decisions
        .iter()
        .any(|decision| decision.state == BriefDecisionState::Current));
    assert!(projection
        .resume
        .decisions
        .iter()
        .any(|decision| decision.state == BriefDecisionState::Superseded));
    assert!(projection
        .decision_context_code
        .iter()
        .any(|link| !link.related_code_entities.is_empty()));
    let timeline = projection
        .checkpoint_timeline
        .last()
        .ok_or("timeline is empty")?;
    assert_eq!(timeline.work_state, WorkState::Completed);
    assert_eq!(timeline.verification[0].state, VerificationState::Passed);
    assert_eq!(timeline.user_review.state, UserReviewState::Pending);
    assert_eq!(
        timeline.user_acceptance.state,
        UserAcceptanceState::NotRequested
    );
    assert!(projection
        .canonical_inspection
        .iter()
        .any(|item| item.kind == CanonicalInspectionKind::ContextItem));
    assert_eq!(projection.candidate_inspection.len(), 1);

    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 64,
        },
    );
    assert_eq!(understanding.project_id, project.id);
    assert_eq!(understanding.goals_and_why.len(), 1);
    assert_eq!(
        understanding.current_work.as_ref().map(|work| work.state),
        Some(WorkState::Completed)
    );
    assert_eq!(understanding.completed_work.len(), 1);
    assert!(understanding.remaining_work.is_empty());
    assert!(understanding
        .next_steps
        .iter()
        .any(|step| step.text == "review source-grounding gaps"));
    assert!(understanding.active_decisions.iter().any(|decision| {
        decision.decision.decision_id == active_decision.id
            && !decision.affected_code_entities.is_empty()
            && !decision.link_basis.is_empty()
    }));
    assert!(understanding
        .open_questions
        .iter()
        .any(|question| question.question_id == open_question.id));
    assert!(!understanding.architecture.components.is_empty());
    assert!(!understanding.architecture.relationships.is_empty());
    assert!(!understanding.deterministic_explanations.is_empty());
    assert_eq!(understanding.generated_interpretations.len(), 1);
    assert!(!understanding.evidence.snapshots.is_empty());
    assert_eq!(canonical, canonical_before);
    assert_eq!(candidates, candidates_before);

    let mut local_analysis = analysis.clone();
    local_analysis.agent_interpretations.clear();
    let local_projection = build_projection_fixture(
        &canonical,
        &candidates,
        &local_analysis,
        project.id,
        ProjectionBound::default(),
    );
    let local_understanding = build_project_understanding(
        &local_projection,
        UnderstandingBound {
            max_items_per_section: 64,
        },
    );
    assert!(local_understanding.generated_interpretations.is_empty());
    assert!(local_understanding
        .deterministic_explanations
        .iter()
        .any(|item| item.kind == UnderstandingExplanationKind::Component));
    assert!(local_understanding
        .deterministic_explanations
        .iter()
        .any(|item| item.kind == UnderstandingExplanationKind::Flow));
    let decision_effect = local_understanding
        .deterministic_explanations
        .iter()
        .find(|item| {
            item.kind == UnderstandingExplanationKind::DecisionImpact
                && item.decision_basis.contains(&active_decision.id)
        })
        .ok_or("deterministic Decision effect missing")?;
    assert!(!decision_effect.entity_basis.is_empty());
    assert!(!decision_effect.source_basis.is_empty());
    assert!(decision_effect
        .evidence_classes
        .contains(&UnderstandingEvidenceClass::CanonicalDecision));
    let inspectable_relations = local_understanding
        .architecture
        .relationships
        .iter()
        .chain(&local_understanding.evidence.unresolved_relationships)
        .map(|relation| relation.identity.as_str())
        .collect::<BTreeSet<_>>();
    for explanation in &local_understanding.deterministic_explanations {
        assert!(!explanation.identity.is_empty());
        assert!(!explanation.english.is_empty());
        assert!(!explanation.korean.is_empty());
        assert!(!explanation.evidence_classes.is_empty());
        assert!(
            !explanation.entity_basis.is_empty() || !explanation.decision_basis.is_empty(),
            "explanation lacks inspectable entity/Decision basis: {}",
            explanation.identity
        );
        if explanation.kind == UnderstandingExplanationKind::Flow {
            assert!(!explanation.relation_basis.is_empty());
        }
        assert!(explanation
            .relation_basis
            .iter()
            .all(|relation| inspectable_relations.contains(relation.as_str())));
    }
    assert_eq!(canonical, canonical_before);
    assert_eq!(candidates, candidates_before);

    let tightly_bounded = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 1,
        },
    );
    assert_eq!(
        tightly_bounded,
        build_project_understanding(
            &projection,
            UnderstandingBound {
                max_items_per_section: 1,
            }
        )
    );
    assert!(tightly_bounded
        .omissions
        .iter()
        .all(|omission| omission.omitted_count > 0));

    let explicit_destination = repository_root.join("generated/guide.md");
    let before_files = files_under(&repository_root);
    let request = DocumentRequest {
        requested_language: "fr-CA".to_owned(),
        fixed_locale: FixedLocale::English,
        generated_at: TimestampMicros::from_unix_micros(40_000),
        generator: GeneratorIdentity {
            generator: "volicord-projections".to_owned(),
            agent: Some("codex".to_owned()),
            model: Some("fixture-model".to_owned()),
        },
        requested_destinations: vec![RequestedDestination {
            document_kind: DocumentKind::ProjectArchitectureGuide,
            output_format: OutputFormat::Markdown,
            path: explicit_destination.display().to_string(),
        }],
    };
    let candidate_failures = [
        (
            CandidateDependencyState::Unavailable,
            CandidateDependencyFailureKind::Unavailable,
            ProjectionIssueKind::CandidateUnavailable,
            "Candidate storage is unavailable",
        ),
        (
            CandidateDependencyState::Unsupported,
            CandidateDependencyFailureKind::Unsupported,
            ProjectionIssueKind::CandidateUnsupported,
            "Candidate storage version is unsupported",
        ),
        (
            CandidateDependencyState::Corrupt,
            CandidateDependencyFailureKind::Corrupt,
            ProjectionIssueKind::CandidateCorrupt,
            "Candidate storage is corrupt",
        ),
        (
            CandidateDependencyState::RepairRequired,
            CandidateDependencyFailureKind::RepairRequired,
            ProjectionIssueKind::CandidateRepairRequired,
            "Candidate cleanup requires repair",
        ),
    ];
    for (state, failure_kind, issue_kind, reason) in candidate_failures {
        let degraded = build_project_projection(ProjectProjectionInputs {
            canonical: &canonical,
            analyses: &[&analysis],
            applicability: ApplicabilityQuery {
                project_id: project.id,
                paths: Vec::new(),
                components: Vec::new(),
                work_contexts: Vec::new(),
                current_assumptions: Vec::new(),
                met_revisit_triggers: Vec::new(),
            },
            candidates: CandidateProjectionInput::Degraded {
                usable_basis: (state == CandidateDependencyState::RepairRequired)
                    .then_some(&candidates),
                failure: CandidateDependencyFailure {
                    kind: failure_kind,
                    affected_scope: "candidate_inspection".to_owned(),
                    reason: reason.to_owned(),
                },
            },
            candidate_content_access: CandidateContentAccess::AllowBoundedSummary,
            observed_at: TimestampMicros::from_unix_micros(30_000),
            bound: ProjectionBound::default(),
        });
        assert_eq!(degraded.candidate_dependency, state);
        assert_eq!(degraded.health, ProjectionHealth::Degraded);
        assert!(!degraded.canonical_inspection.is_empty());
        assert!(degraded.issues.iter().any(|issue| {
            issue.kind == issue_kind
                && issue.affected_scope == "candidate_inspection"
                && issue.reason == reason
        }));
        if state != CandidateDependencyState::RepairRequired {
            assert!(degraded.candidate_inspection.is_empty());
        }
        let degraded_documents = generate_documents(&degraded, &request)?;
        let document = &degraded_documents.handoff_resume;
        assert!(document
            .metadata
            .omissions
            .iter()
            .any(|issue| issue.kind == issue_kind && issue.reason == reason));
        assert!(document.markdown.content.contains(reason));
        assert!(document.html.content.contains(reason));
    }
    let documents = generate_documents(&projection, &request)?;
    assert_eq!(documents, generate_documents(&projection, &request)?);
    let all = [
        &documents.project_architecture_guide,
        &documents.decision_report,
        &documents.implementation_plan,
        &documents.handoff_resume,
    ];
    let architecture_classes = documents
        .project_architecture_guide
        .body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .map(|claim| claim.class)
        .collect::<BTreeSet<_>>();
    assert!(architecture_classes.contains(&ClaimClass::StructuralFact));
    assert!(architecture_classes.contains(&ClaimClass::SemanticResult));
    assert!(architecture_classes.contains(&ClaimClass::AgentInterpretation));
    for document in all {
        assert_eq!(document.metadata.requested_language, "fr-CA");
        assert_eq!(document.metadata.html_language_tag, "fr-CA");
        assert!(matches!(
            document.metadata.narrative_realization,
            NarrativeRealizationState::Unavailable { .. }
        ));
        assert_eq!(document.metadata.project_id, project.id);
        assert!(!document.metadata.included_decisions.is_empty());
        assert!(!document.metadata.capability_coverage.is_empty());
        assert!(!document.metadata.capability_gaps.is_empty());
        assert!(document
            .html
            .content
            .starts_with("<!doctype html><html lang=\"fr-CA\">"));
        assert!(!document.html.content.contains("<script"));
        assert!(!document.html.content.contains(" href="));
        assert!(!document.html.content.contains(" src="));
        for claim in document
            .body
            .sections
            .iter()
            .flat_map(|section| &section.claims)
        {
            assert!(
                !claim.source_basis.is_empty()
                    || !claim.decision_basis.is_empty()
                    || !claim.analysis_basis.is_empty()
                    || claim.explicit_inference
            );
            assert!(document
                .markdown
                .content
                .contains(&claim.identity.replace('_', "\\_")));
            assert!(document.html.content.contains(&claim.identity));
            if claim.class == ClaimClass::AgentInterpretation {
                assert!(claim.explicit_inference);
            }
        }
    }

    let plan = prepare_narrative_plan(
        &projection,
        &request,
        DocumentKind::ProjectArchitectureGuide,
    )?;
    let realization = NarrativeRealization {
        plan_fingerprint: plan.plan_fingerprint.clone(),
        title: "Comprendre le projet et son architecture".to_owned(),
        sections: plan
            .sections
            .iter()
            .map(|section| RealizedNarrativeSection {
                key: section.key.clone(),
                title: format!("Explication française — {}", section.source_title),
                claims: section
                    .claims
                    .iter()
                    .map(|claim| RealizedNarrativeClaim {
                        identity: claim.identity.clone(),
                        text: format!("Explication française : {}", claim.source_text),
                    })
                    .collect(),
            })
            .collect(),
        generator: GeneratorIdentity {
            generator: "volicord-codex-host".to_owned(),
            agent: Some("codex".to_owned()),
            model: Some("fixture-realizer".to_owned()),
        },
    };
    let realized = realize_narrative(
        &projection,
        &request,
        DocumentKind::ProjectArchitectureGuide,
        &realization,
    )?;
    assert!(realized
        .markdown
        .content
        .contains("Comprendre le projet et son architecture"));
    assert!(realized
        .body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .all(|claim| claim.text.starts_with("Explication française")));
    assert_eq!(
        realized.metadata.generator.model.as_deref(),
        Some("fixture-realizer")
    );
    assert!(matches!(
        realized.metadata.narrative_realization,
        NarrativeRealizationState::HostRealized { .. }
    ));
    for (realized_claim, plan_claim) in realized
        .body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .zip(plan.sections.iter().flat_map(|section| &section.claims))
    {
        assert_eq!(realized_claim.source_basis, plan_claim.source_basis);
        assert_eq!(realized_claim.decision_basis, plan_claim.decision_basis);
        assert_eq!(realized_claim.analysis_basis, plan_claim.analysis_basis);
    }

    let mut ungrounded = realization.clone();
    ungrounded.sections[0].claims[0].identity = "invented-claim".to_owned();
    assert!(realize_narrative(
        &projection,
        &request,
        DocumentKind::ProjectArchitectureGuide,
        &ungrounded,
    )
    .is_err());
    let (section_index, claim_index) = plan
        .sections
        .iter()
        .enumerate()
        .find_map(|(section_index, section)| {
            section
                .claims
                .iter()
                .position(|claim| !claim.protected_terms.is_empty())
                .map(|claim_index| (section_index, claim_index))
        })
        .ok_or("fixture plan has no protected code/path term")?;
    let mut translated_identifier = realization.clone();
    translated_identifier.sections[section_index].claims[claim_index].text =
        "Explicación sin el identificador requerido".to_owned();
    assert!(realize_narrative(
        &projection,
        &request,
        DocumentKind::ProjectArchitectureGuide,
        &translated_identifier,
    )
    .is_err());
    assert_eq!(
        documents
            .project_architecture_guide
            .markdown
            .requested_destination
            .as_deref(),
        Some(explicit_destination.to_string_lossy().as_ref())
    );
    assert!(!explicit_destination.exists());
    assert_eq!(files_under(&repository_root), before_files);

    let template_fact = analysis
        .structural_facts
        .first()
        .cloned()
        .ok_or("structural fact missing")?;
    let mut scaling_counts = Vec::new();
    for cardinality in [32_usize, 4_096] {
        let mut scaled_analysis = analysis.clone();
        scaled_analysis.structural_facts = (0..cardinality)
            .map(|index| {
                let mut fact = template_fact.clone();
                let entity_identity = format!("scale:entity:{index:08}");
                fact.entity.identity = entity_identity.clone();
                fact.entity.display_name = Some(entity_identity.clone());
                fact.entity.qualified_name = Some(entity_identity.clone());
                for (relation_index, relation) in fact.relations.iter_mut().enumerate() {
                    relation.identity = format!("scale:relation:{index:08}:{relation_index:04}");
                    relation.source_entity = entity_identity.clone();
                }
                fact
            })
            .collect();
        let scaled_projection = build_projection_fixture(
            &canonical,
            &candidates,
            &scaled_analysis,
            project.id,
            ProjectionBound {
                max_items_per_section: 4,
            },
        );
        let entity_omission = scaled_projection
            .issues
            .iter()
            .find(|issue| issue.affected_scope == "repository_map.entity")
            .ok_or("entity omission missing")?;
        assert_eq!(entity_omission.kind, ProjectionIssueKind::Bound);
        assert_eq!(entity_omission.identity, "bound:repository_map.entity");
        assert_eq!(entity_omission.omitted_count, cardinality - 4);
        assert_eq!(scaled_projection.repository_map.entities.len(), 4);
        assert!(scaled_projection.issues.iter().any(|issue| {
            issue.kind == ProjectionIssueKind::PartialCapability && issue.omitted_count == 0
        }));

        let scaled_documents = generate_documents(&scaled_projection, &request)?;
        let scaled_document = &scaled_documents.project_architecture_guide;
        assert_eq!(
            scaled_document.metadata.format_version,
            GENERATED_DOCUMENT_METADATA_VERSION
        );
        assert_eq!(
            scaled_document
                .metadata
                .omissions
                .iter()
                .find(|issue| issue.affected_scope == "repository_map.entity")
                .map(|issue| issue.omitted_count),
            Some(cardinality - 4)
        );
        let gap_claim_count = scaled_document
            .body
            .sections
            .iter()
            .find(|section| section.key == "gaps")
            .ok_or("gap section missing")?
            .claims
            .len();
        let exact_count_marker = format!("exact omitted count={}", cardinality - 4);
        assert!(scaled_document
            .markdown
            .content
            .contains(&exact_count_marker));
        assert!(scaled_document.html.content.contains(&exact_count_marker));
        scaling_counts.push((
            scaled_projection
                .issues
                .iter()
                .filter(|issue| issue.kind == ProjectionIssueKind::Bound)
                .count(),
            gap_claim_count,
            scaled_document.metadata.omissions.len(),
            scaled_document
                .markdown
                .content
                .matches("exact omitted count=")
                .count(),
            scaled_document
                .html
                .content
                .matches("exact omitted count=")
                .count(),
        ));
    }
    assert_eq!(scaling_counts[0], scaling_counts[1]);

    let template_report = projection
        .repository_map
        .capabilities
        .first()
        .cloned()
        .ok_or("capability report missing")?;
    let template_gap = projection
        .repository_map
        .gaps
        .first()
        .cloned()
        .ok_or("capability gap missing")?;
    let template_issue = projection
        .issues
        .first()
        .cloned()
        .ok_or("projection issue missing")?;
    let mut metadata_heavy = projection.clone();
    metadata_heavy.repository_map.capabilities = (0..512)
        .map(|index| {
            let mut report = template_report.clone();
            report.area.path = format!("bounded-area-{index:04}");
            report
        })
        .collect();
    metadata_heavy.repository_map.gaps = (0..512)
        .map(|index| {
            let mut gap = template_gap.clone();
            gap.area = format!("bounded-gap-{index:04}");
            gap.reason = format!("bounded gap reason {index:04}");
            gap
        })
        .collect();
    metadata_heavy.issues = (0..512)
        .map(|index| {
            let mut issue = template_issue.clone();
            issue.identity = format!("bounded-issue-{index:04}");
            issue.reason = format!("bounded issue reason {index:04}");
            issue.omitted_count = 0;
            issue
        })
        .collect();
    let metadata_heavy_documents = generate_documents(&metadata_heavy, &request)?;
    let rendered = &metadata_heavy_documents.project_architecture_guide;
    assert_eq!(rendered.metadata.capability_coverage.len(), 512);
    assert_eq!(rendered.metadata.capability_gaps.len(), 512);
    assert_eq!(rendered.metadata.omissions.len(), 512);
    assert!(rendered
        .metadata
        .capability_gaps
        .iter()
        .any(|gap| gap.reason == "bounded gap reason 0511"));
    assert!(!rendered
        .markdown
        .content
        .contains("bounded gap reason 0511"));
    assert!(rendered
        .markdown
        .content
        .contains("504 additional items omitted from rendered metadata"));
    assert!(rendered
        .body
        .sections
        .iter()
        .all(|section| section.claims.len() <= 13));
    assert!(rendered
        .body
        .sections
        .iter()
        .find(|section| section.key == "gaps")
        .is_some_and(|section| section
            .claims
            .iter()
            .any(|claim| claim.identity == "render-bound:gaps")));
    // Fixture-specific output regression: typed grounding may grow without
    // turning the portable rendering into a hardware or product ceiling.
    assert!(rendered.markdown.content.len() < 80_000);
    assert!(rendered.html.content.len() < 120_000);

    let attribute_language = "fr-CA\" data-unsafe=\"<&";
    let attribute_safe = generate_documents(
        &projection,
        &DocumentRequest {
            requested_language: attribute_language.to_owned(),
            requested_destinations: Vec::new(),
            ..request.clone()
        },
    )?;
    assert!(attribute_safe
        .project_architecture_guide
        .html
        .content
        .starts_with("<!doctype html><html lang=\"en\">"));
    assert_eq!(
        attribute_safe
            .project_architecture_guide
            .metadata
            .requested_language,
        attribute_language
    );
    assert_eq!(
        attribute_safe
            .project_architecture_guide
            .metadata
            .html_language_tag,
        "en"
    );
    assert!(!attribute_safe
        .project_architecture_guide
        .html
        .content
        .starts_with("<!doctype html><html lang=\"fr-CA"));

    let normalized_language = generate_documents(
        &projection,
        &DocumentRequest {
            requested_language: "ZH_hant_tw".to_owned(),
            requested_destinations: Vec::new(),
            ..request.clone()
        },
    )?;
    assert_eq!(
        normalized_language
            .handoff_resume
            .metadata
            .requested_language,
        "ZH_hant_tw"
    );
    assert_eq!(
        normalized_language
            .handoff_resume
            .metadata
            .html_language_tag,
        "zh-Hant-TW"
    );
    assert!(normalized_language
        .handoff_resume
        .html
        .content
        .starts_with("<!doctype html><html lang=\"zh-Hant-TW\">"));

    let huge_claim = "OVERSIZED-CLAIM-".repeat(400);
    let huge_name = "OVERSIZED-NAME-".repeat(400);
    let huge_diagnostic = "OVERSIZED-DIAGNOSTIC-".repeat(300);
    let huge_metadata = "OVERSIZED-METADATA-".repeat(300);
    for value in [&huge_claim, &huge_name, &huge_diagnostic, &huge_metadata] {
        assert!(value.len() > RENDERED_DOCUMENT_FIELD_BYTE_LIMIT);
    }
    let mut pathological = projection.clone();
    pathological.resume.goals_and_why[0].statement = huge_claim.clone();
    pathological.repository_map.entities[0].display_name = huge_name.clone();
    pathological.repository_map.gaps[0].reason = huge_diagnostic.clone();
    let pathological_request = DocumentRequest {
        requested_language: "Klingon in Latin script".to_owned(),
        fixed_locale: FixedLocale::English,
        generated_at: request.generated_at,
        generator: GeneratorIdentity {
            generator: huge_metadata.clone(),
            agent: Some(huge_metadata.clone()),
            model: Some(huge_metadata.clone()),
        },
        requested_destinations: vec![RequestedDestination {
            document_kind: DocumentKind::ProjectArchitectureGuide,
            output_format: OutputFormat::Html,
            path: huge_metadata.clone(),
        }],
    };
    let pathological_documents = generate_documents(&pathological, &pathological_request)?;
    assert_eq!(
        pathological_documents,
        generate_documents(&pathological, &pathological_request)?
    );
    let pathological_document = &pathological_documents.project_architecture_guide;
    assert_eq!(
        pathological_document.metadata.generator.generator,
        huge_metadata
    );
    assert_eq!(
        pathological_document.metadata.requested_language,
        "Klingon in Latin script"
    );
    assert_eq!(pathological_document.metadata.html_language_tag, "en");
    assert!(pathological_document
        .body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .any(|claim| claim.text.contains(&huge_claim)));
    for sentinel in [
        "OVERSIZED-CLAIM-OVERSIZED-CLAIM-",
        "OVERSIZED-NAME-OVERSIZED-NAME-",
        "OVERSIZED-DIAGNOSTIC-OVERSIZED-DIAGNOSTIC-",
        "OVERSIZED-METADATA-OVERSIZED-METADATA-",
    ] {
        assert!(!pathological_document.markdown.content.contains(sentinel));
        assert!(!pathological_document.html.content.contains(sentinel));
    }
    for field in ["claim text", "claim uncertainty", "metadata value"] {
        assert!(pathological_document.markdown.content.contains(&format!(
            "omitted oversized field: {field}; exact UTF-8 bytes="
        )));
        assert!(pathological_document.html.content.contains(&format!(
            "omitted oversized field: {field}; exact UTF-8 bytes="
        )));
    }
    assert!(pathological_document.markdown.content.contains(&format!(
        "rendered byte limit={RENDERED_DOCUMENT_FIELD_BYTE_LIMIT}"
    )));
    assert!(pathological_document.markdown.content.len() <= RENDERED_MARKDOWN_BYTE_LIMIT);
    assert!(pathological_document.html.content.len() <= RENDERED_HTML_BYTE_LIMIT);

    // Volicord-scale affected-path sets remain fully available in the typed
    // projection while the public realization plan carries representative
    // identifiers and exact omission accounting within its smaller budget.
    let affected_paths = (0..640)
        .map(|index| {
            format!("rebuild/crates/volicord-projections/src/volicord-scale-{index:04}.rs")
        })
        .collect::<Vec<_>>();
    let mut large_realization_projection = projection.clone();
    large_realization_projection.repository_map.gaps[0].affected_areas = affected_paths.clone();
    large_realization_projection.decision_context_code[0].declared_paths = affected_paths.clone();
    let oversized_goal = "Grounded project purpose remains typed. ".repeat(220);
    assert!(oversized_goal.len() > RENDERED_DOCUMENT_FIELD_BYTE_LIMIT);
    large_realization_projection.resume.goals_and_why[0].statement = oversized_goal.clone();

    let spanish_request = DocumentRequest {
        requested_language: "es-ES".to_owned(),
        requested_destinations: Vec::new(),
        ..request.clone()
    };
    let large_plan = prepare_narrative_plan(
        &large_realization_projection,
        &spanish_request,
        DocumentKind::ProjectArchitectureGuide,
    )?;
    assert_eq!(
        large_plan,
        prepare_narrative_plan(
            &large_realization_projection,
            &spanish_request,
            DocumentKind::ProjectArchitectureGuide,
        )?
    );
    let mut same_size_source_change = large_realization_projection.clone();
    same_size_source_change.resume.goals_and_why[0]
        .statement
        .replace_range(0..1, "g");
    let changed_plan = prepare_narrative_plan(
        &same_size_source_change,
        &spanish_request,
        DocumentKind::ProjectArchitectureGuide,
    )?;
    assert_ne!(large_plan.plan_fingerprint, changed_plan.plan_fingerprint);
    for claim in large_plan
        .sections
        .iter()
        .flat_map(|section| &section.claims)
    {
        assert!(claim.source_text.len() <= NARRATIVE_PLAN_SOURCE_TEXT_BYTE_LIMIT);
        assert!(claim.protected_terms.len() <= NARRATIVE_PLAN_PROTECTED_TERM_LIMIT);
        assert!(claim.protected_terms.iter().all(|term| term.len()
            <= NARRATIVE_PLAN_PROTECTED_TERM_BYTE_LIMIT
            && claim.source_text.contains(term)));
    }
    let goal_plan_claim = large_plan
        .sections
        .iter()
        .find(|section| section.key == "overview")
        .and_then(|section| section.claims.first())
        .expect("oversized goal claim must remain in the plan");
    let goal_omission = goal_plan_claim
        .source_text_omission
        .as_ref()
        .expect("oversized goal must use the bounded source representation");
    assert_eq!(goal_omission.exact_source_utf8_bytes, oversized_goal.len());
    assert_eq!(
        goal_omission.exact_source_character_count,
        oversized_goal.chars().count()
    );
    let gap_plan_claim = large_plan
        .sections
        .iter()
        .find(|section| section.key == "gaps")
        .and_then(|section| section.claims.first())
        .expect("large affected-path gap must remain in the plan");
    assert!(gap_plan_claim
        .source_text
        .contains("exact omitted item count=632"));
    assert!(gap_plan_claim.source_text.contains(&affected_paths[0]));
    assert!(gap_plan_claim.source_text.contains(&affected_paths[7]));
    assert!(!gap_plan_claim.source_text.contains(&affected_paths[8]));

    let spanish_realization = NarrativeRealization {
        plan_fingerprint: large_plan.plan_fingerprint.clone(),
        title: "Guía del proyecto y de la arquitectura".to_owned(),
        sections: large_plan
            .sections
            .iter()
            .map(|section| RealizedNarrativeSection {
                key: section.key.clone(),
                title: format!("Explicación en español: {}", section.source_title),
                claims: section
                    .claims
                    .iter()
                    .map(|claim| RealizedNarrativeClaim {
                        identity: claim.identity.clone(),
                        text: format!("Explicación en español: {}", claim.source_text),
                    })
                    .collect(),
            })
            .collect(),
        generator: GeneratorIdentity {
            generator: "volicord-codex-host".to_owned(),
            agent: Some("codex".to_owned()),
            model: Some("fixture-realizer-es".to_owned()),
        },
    };
    assert!(spanish_realization
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .all(|claim| claim.text.len() <= RENDERED_DOCUMENT_FIELD_BYTE_LIMIT));
    let spanish_document = realize_narrative(
        &large_realization_projection,
        &spanish_request,
        DocumentKind::ProjectArchitectureGuide,
        &spanish_realization,
    )?;
    assert!(spanish_document
        .body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .all(|claim| claim.text.starts_with("Explicación en español")));
    for (realized_claim, plan_claim) in spanish_document
        .body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .zip(
            large_plan
                .sections
                .iter()
                .flat_map(|section| &section.claims),
        )
    {
        assert_eq!(realized_claim.source_basis, plan_claim.source_basis);
        assert_eq!(realized_claim.decision_basis, plan_claim.decision_basis);
        assert_eq!(realized_claim.analysis_basis, plan_claim.analysis_basis);
    }

    let korean = generate_documents(
        &projection,
        &DocumentRequest {
            requested_language: "ko".to_owned(),
            fixed_locale: FixedLocale::Korean,
            generated_at: TimestampMicros::from_unix_micros(40_000),
            generator: request.generator.clone(),
            requested_destinations: Vec::new(),
        },
    )?;
    assert!(korean
        .project_architecture_guide
        .markdown
        .content
        .contains("프로젝트 및 아키텍처 가이드"));
    assert!(korean
        .project_architecture_guide
        .markdown
        .content
        .contains("일부 가능"));
    assert!(!korean
        .project_architecture_guide
        .markdown
        .content
        .contains("state=Partial"));
    assert!(korean
        .project_architecture_guide
        .html
        .content
        .starts_with("<!doctype html><html lang=\"ko\">"));
    assert_eq!(
        korean
            .project_architecture_guide
            .metadata
            .narrative_realization,
        NarrativeRealizationState::FixedLocale
    );
    assert_eq!(canonical, canonical_before);
    assert_eq!(
        store.read_canonical_basis(
            project.id,
            CanonicalReadOptions {
                include_checkpoint_history: true,
            },
        )?,
        canonical_before
    );
    assert_eq!(candidates, candidates_before);
    assert_eq!(candidate_store.read_basis(project.id)?, candidates_before);
    Ok(())
}
