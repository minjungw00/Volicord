use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointDraft,
    CheckpointKind, ContextItemDraft, ContextItemRole, DecisionSupersessionDraft,
    DeterministicIdGenerator, ExplicitQuestionResponse, FixedClock, NonUserQuestionOutcome,
    OperationId, Principal, PrincipalKind, Project, QuestionAlternative, QuestionDraft,
    QuestionMateriality, QuestionResearchState, QuestionResponseDraft, Source, SourceDraft,
    SourcePayload, StatementProvenanceRole, Store, TimestampMicros, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource, VerificationFact,
    VerificationState, WorkState,
};
use volicord_inquiry::{
    ApplicabilityQuery, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDraft, CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention,
    CandidateStore, SubmissionOutcome,
};
use volicord_projections::{
    build_project_projection, generate_documents, BriefDecisionState, CandidateContentAccess,
    CanonicalInspectionKind, ClaimClass, DocumentKind, DocumentRequest, FixedLocale,
    GeneratorIdentity, MapRelationClass, OutputFormat, ProjectProjectionInputs, ProjectionBound,
    ProjectionHealth, RequestedDestination,
};
use volicord_repository_intelligence::{
    analyze_repository_semantics, AgentInterpretation, CanonicalGrounding, Capability,
    CapabilityState, InventoryRequest, ProvenanceClass, SemanticAnalysisRequest,
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
        },
    })? {
        SubmissionOutcome::Stored(_) => {}
        SubmissionOutcome::CollectionDisabled { .. } => return Err("collection disabled".into()),
    }
    let candidates = candidate_store.read_basis(project.id)?;
    let candidates_before = candidates.clone();
    let analyses = [&analysis];
    let build = || {
        build_project_projection(ProjectProjectionInputs {
            canonical: &canonical,
            analyses: &analyses,
            applicability: ApplicabilityQuery {
                project_id: project.id,
                paths: vec!["src".to_owned()],
                components: vec!["MarkdownGuide".to_owned()],
                work_contexts: vec!["documents".to_owned()],
                current_assumptions: vec!["local-first".to_owned()],
                met_revisit_triggers: Vec::new(),
            },
            candidates: Some(&candidates),
            candidate_content_access: CandidateContentAccess::AllowBoundedSummary,
            observed_at: TimestampMicros::from_unix_micros(30_000),
            bound: ProjectionBound::default(),
        })
    };
    let projection = build();
    assert_eq!(projection, build());
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
            assert!(document.markdown.content.contains(&claim.identity));
            assert!(document.html.content.contains(&claim.identity));
            if claim.class == ClaimClass::AgentInterpretation {
                assert!(claim.explicit_inference);
            }
        }
    }
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
        .starts_with("<!doctype html><html lang=\"fr-CA&quot; data-unsafe=&quot;&lt;&amp;\">"));
    assert_eq!(
        attribute_safe
            .project_architecture_guide
            .metadata
            .requested_language,
        attribute_language
    );

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
        .html
        .content
        .starts_with("<!doctype html><html lang=\"ko\">"));
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
