use serde_json::json;
use std::error::Error;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadBasis,
    CanonicalReadOptions, CheckpointDraft, CheckpointKind, ContextItemCorrectionDraft,
    ContextItemDraft, ContextItemRole, CorrectionKind, DecisionChoice, DecisionCorrectionDraft,
    DecisionSupersessionDraft, ExplicitQuestionResponse, OperationId, Principal, PrincipalKind,
    QuestionAlternative, QuestionDraft, QuestionResponseDraft, SourceDraft, SourcePayload,
    StatementProvenanceRole, Store, UserAcceptanceFact, UserAcceptanceState, UserReviewFact,
    UserReviewState, UserTurnSource, VerificationFact, VerificationState, WorkState,
};
use volicord_repository_intelligence::{
    analyze_repository_semantics, canonical_json, grounded_explanation_basis, search_local,
    CanonicalGrounding, CanonicalGroundingIssueKind, CanonicalLinkSelector, CanonicalReference,
    CanonicalSourceBasis, InventoryRequest, Language, SemanticAnalysisRequest,
    StructuralAnalysisRequest, ANALYSIS_SNAPSHOT_FORMAT_VERSION,
};

const OBSERVED_AT: i64 = 1_725_000_000_000_000;

fn operation(byte: u8) -> OperationId {
    OperationId::from_bytes([byte; 16])
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../validation/repository-intelligence/polyglot-structural/fixtures/typescript")
}

struct CanonicalFixture {
    store: Store,
    project: volicord_context::Project,
    repository: volicord_context::Source,
    no_snapshot_source: volicord_context::Source,
    user_turn: volicord_context::Source,
    decision: volicord_context::Decision,
    context_item: volicord_context::ContextItem,
    checkpoint: volicord_context::Checkpoint,
}

impl CanonicalFixture {
    fn basis(&self) -> Result<CanonicalReadBasis, volicord_context::Error> {
        self.store.read_canonical_basis(
            self.project.id,
            CanonicalReadOptions {
                include_checkpoint_history: true,
            },
        )
    }
}

fn canonical_fixture() -> Result<(tempfile::TempDir, CanonicalFixture), Box<dyn Error>> {
    let runtime = tempdir()?;
    let mut store = Store::open(runtime.path().join("context.sqlite3"))?;
    let project = store
        .create_project(operation(1), "Canonical grounding")?
        .value;
    let repository = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "typescript-fixture-snapshot".to_owned(),
                },
                actor: principal(PrincipalKind::Repository, "repository-observer"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    let no_snapshot_source = store
        .record_source(
            operation(3),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::Url {
                    url: "https://example.invalid/grounding".to_owned(),
                },
                actor: principal(PrincipalKind::Agent, "codex"),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let user_turn = store
        .record_source(
            operation(4),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "grounding-test".to_owned(),
                    turn: "choose-grounding".to_owned(),
                },
                actor: principal(PrincipalKind::User, "owner"),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let question = store
        .create_question(
            operation(5),
            project.id,
            QuestionDraft {
                expected_project_revision: project.revision,
                prompt_basis: "Which grounding policy applies?".to_owned(),
                source_basis: vec![repository.id],
                dependencies: vec![],
                alternatives: vec![
                    QuestionAlternative {
                        key: "exact".to_owned(),
                        label: "Exact basis".to_owned(),
                        consequence: "Historical references remain stable".to_owned(),
                    },
                    QuestionAlternative {
                        key: "latest".to_owned(),
                        label: "Latest only".to_owned(),
                        consequence: "Historical meaning can drift".to_owned(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("exact".to_owned()),
                    rationale: "Preserve the observed basis".to_owned(),
                    source_basis: vec![repository.id],
                },
                trade_offs: vec!["References carry more metadata".to_owned()],
                uncertainty: vec![],
                material_scope: vec!["repository analysis".to_owned()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 0,
                why_it_matters_now: "analysis grounding requires stable historical meaning"
                    .to_owned(),
                established_facts: vec![],
                assumptions: vec![],
                known_limits: vec![],
                what_the_answer_unlocks: vec!["grounded analysis".to_owned()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    let decision = store
        .record_question_response(
            operation(6),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: project.revision,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Existing(user_turn.id),
                displayed_alternative_keys: vec!["exact".to_owned(), "latest".to_owned()],
                displayed_recommendation_key: Some("exact".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "exact".to_owned(),
                    user_rationale: Some("Keep historical grounding".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["Canonical revision history remains inspectable".to_owned()],
                revisit_triggers: vec!["Canonical read contract changes".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or("Decision was not recorded")?;
    let context_item = store
        .record_context_item(
            operation(7),
            project.id,
            ContextItemDraft {
                expected_project_revision: project.revision,
                role: ContextItemRole::Constraint,
                statement: "Repository links preserve exact basis.".to_owned(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: principal(PrincipalKind::User, "owner"),
                source_basis: vec![user_turn.id],
                applicability: ApplicabilityScope::default(),
            },
        )?
        .value;
    let checkpoint = store
        .record_checkpoint(
            operation(8),
            project.id,
            CheckpointDraft {
                expected_project_revision: project.revision,
                kind: CheckpointKind::Handoff,
                goal: "Test canonical grounding".to_owned(),
                work_state: WorkState::Completed,
                state_change: Some("Initial grounding basis recorded".to_owned()),
                source_basis: vec![repository.id],
                changed_source_basis: vec![repository.id],
                changed_paths: vec!["src/index.ts".to_owned()],
                applied_decisions: vec![decision.id],
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
                known_limits: vec![],
                non_goals: vec![],
                open_questions: vec![],
                next_step: "Validate derived references".to_owned(),
                handoff_to: Some("Repository Intelligence".to_owned()),
            },
        )?
        .value;
    Ok((
        runtime,
        CanonicalFixture {
            store,
            project,
            repository,
            no_snapshot_source,
            user_turn,
            decision,
            context_item,
            checkpoint,
        },
    ))
}

fn selector() -> CanonicalLinkSelector {
    CanonicalLinkSelector::new(Language::TypeScript, "src/index.ts", "Greeter.greet")
}

fn analyze_with_links(
    root: &Path,
    grounding: &CanonicalGrounding,
    repository_source: volicord_context::SourceId,
    previous: Option<&volicord_repository_intelligence::AnalysisSnapshot>,
    links: &[CanonicalReference],
) -> Result<volicord_repository_intelligence::AnalysisSnapshot, Box<dyn Error>> {
    let inventory = InventoryRequest::new(root, grounding, repository_source, OBSERVED_AT)?;
    let structural = match previous {
        Some(previous) => StructuralAnalysisRequest::new(inventory).with_previous(previous),
        None => StructuralAnalysisRequest::new(inventory),
    };
    let mut request = SemanticAnalysisRequest::new(structural);
    for link in links {
        request = request.with_canonical_link(selector(), link.clone());
    }
    Ok(analyze_repository_semantics(request)?.1)
}

fn wire_revision_reference(
    kind: &str,
    project: volicord_context::ProjectId,
    identity: impl ToString,
    revision: u64,
) -> Result<CanonicalReference, serde_json::Error> {
    serde_json::from_value(json!({
        "kind": kind,
        "target": {
            "project": project.to_string(),
            "identity": identity.to_string(),
            "revision": revision
        }
    }))
}

fn linked_entity(
    analysis: &volicord_repository_intelligence::AnalysisSnapshot,
) -> Result<&volicord_repository_intelligence::CodeEntity, Box<dyn Error>> {
    analysis
        .structural_facts
        .iter()
        .find(|fact| fact.entity.qualified_name.as_deref() == Some("Greeter.greet"))
        .map(|fact| &fact.entity)
        .ok_or_else(|| "Greeter.greet missing".into())
}

#[test]
fn canonical_links_are_project_scoped_basis_bound_and_stable() -> Result<(), Box<dyn Error>> {
    let (_runtime, mut fixture) = canonical_fixture()?;
    let before = fixture.basis()?;
    let grounding = CanonicalGrounding::from_read_basis(&before)?;

    let repository_reference = grounding.source_reference(fixture.repository.id)?;
    assert_eq!(
        repository_reference.basis(),
        &CanonicalSourceBasis::Snapshot("typescript-fixture-snapshot".to_owned())
    );
    let no_snapshot_reference = grounding.source_reference(fixture.no_snapshot_source.id)?;
    assert_eq!(
        no_snapshot_reference.basis(),
        &CanonicalSourceBasis::NotApplicable
    );
    let mismatch = grounding
        .source_reference_at(
            fixture.repository.id,
            CanonicalSourceBasis::Snapshot("wrong-snapshot".to_owned()),
        )
        .err()
        .ok_or("Source basis mismatch was accepted")?;
    assert_eq!(
        mismatch.issues()[0].kind,
        CanonicalGroundingIssueKind::SourceBasisMismatch
    );
    assert_eq!(
        grounding
            .source_reference(volicord_context::SourceId::from_bytes([0xdd; 16]))
            .err()
            .ok_or("nonexistent Source was accepted")?
            .issues()[0]
            .kind,
        CanonicalGroundingIssueKind::DanglingTarget
    );

    let historical_links = vec![
        CanonicalReference::Source(no_snapshot_reference),
        CanonicalReference::Decision(
            grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
        ),
        CanonicalReference::ContextItem(
            grounding
                .context_item_reference(fixture.context_item.id, fixture.context_item.revision)?,
        ),
        CanonicalReference::Checkpoint(
            grounding.checkpoint_reference(fixture.checkpoint.id, fixture.checkpoint.revision)?,
        ),
    ];
    let root = fixture_root();
    let first = analyze_with_links(
        &root,
        &grounding,
        fixture.repository.id,
        None,
        &historical_links,
    )?;
    let repeated = analyze_with_links(
        &root,
        &grounding,
        fixture.repository.id,
        None,
        &historical_links,
    )?;
    assert_eq!(first.identity, repeated.identity);
    assert_eq!(canonical_json(&first)?, canonical_json(&repeated)?);
    assert_eq!(first.format_version, ANALYSIS_SNAPSHOT_FORMAT_VERSION);
    let serialized = String::from_utf8(canonical_json(&first)?)?;
    assert!(serialized.contains("\"revision\":1"));
    assert!(serialized.contains("\"kind\":\"snapshot\""));
    assert!(serialized.contains("\"kind\":\"not_applicable\""));
    let decoded: volicord_repository_intelligence::AnalysisSnapshot =
        serde_json::from_slice(&canonical_json(&first)?)?;
    assert_eq!(decoded, first);
    let mut previous_format = serde_json::to_value(&first)?;
    previous_format["format_version"] = json!(ANALYSIS_SNAPSHOT_FORMAT_VERSION - 1);
    assert!(
        serde_json::from_value::<volicord_repository_intelligence::AnalysisSnapshot>(
            previous_format
        )
        .is_err()
    );

    let linked_entity = first
        .structural_facts
        .iter()
        .find(|fact| fact.entity.qualified_name.as_deref() == Some("Greeter.greet"))
        .ok_or("linked entity missing")?;
    for link in &historical_links {
        assert!(linked_entity.entity.canonical_links.contains(link));
    }

    let corrected_decision = fixture
        .store
        .correct_decision(
            operation(9),
            fixture.project.id,
            fixture.decision.id,
            DecisionCorrectionDraft {
                expected_revision: fixture.decision.revision,
                corrected_user_rationale: Some("Keep  historical grounding".to_owned()),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_turn.id,
            },
        )?
        .value;
    let corrected_context = fixture
        .store
        .correct_context_item(
            operation(10),
            fixture.project.id,
            fixture.context_item.id,
            ContextItemCorrectionDraft {
                expected_revision: fixture.context_item.revision,
                corrected_statement: "Repository links  preserve exact basis.".to_owned(),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_turn.id,
            },
        )?
        .value;
    let successor = fixture
        .store
        .supersede_decision(
            operation(11),
            fixture.project.id,
            DecisionSupersessionDraft {
                expected_project_revision: fixture.project.revision,
                previous_decision_id: fixture.decision.id,
                user_turn_source: UserTurnSource::Existing(fixture.user_turn.id),
                choice: DecisionChoice::Alternative {
                    alternative_key: "latest".to_owned(),
                },
                user_rationale: Some("A new choice has distinct identity".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value;

    let after = fixture.basis()?;
    let current_grounding = CanonicalGrounding::from_read_basis(&after)?;
    current_grounding.validate_analysis_snapshot(&first)?;
    let historical_decision = current_grounding.decision_reference(fixture.decision.id, 1)?;
    let current_decision =
        current_grounding.decision_reference(fixture.decision.id, corrected_decision.revision)?;
    let active_decision = current_grounding.decision_reference(successor.id, successor.revision)?;
    assert_ne!(historical_decision, current_decision);
    assert_eq!(historical_decision.identity(), fixture.decision.id);
    assert_ne!(historical_decision.identity(), successor.id);
    assert_eq!(active_decision.identity(), successor.id);
    let historical_context =
        current_grounding.context_item_reference(fixture.context_item.id, 1)?;
    let current_context = current_grounding
        .context_item_reference(fixture.context_item.id, corrected_context.revision)?;
    assert_ne!(historical_context, current_context);

    let refreshed = analyze_with_links(
        &root,
        &current_grounding,
        fixture.repository.id,
        Some(&first),
        &historical_links,
    )?;
    let refreshed_entity = refreshed
        .structural_facts
        .iter()
        .find(|fact| fact.entity.qualified_name.as_deref() == Some("Greeter.greet"))
        .ok_or("refreshed linked entity missing")?;
    for link in &historical_links {
        assert!(refreshed_entity.entity.canonical_links.contains(link));
    }
    current_grounding.validate_analysis_snapshot(&refreshed)?;
    let explanation = grounded_explanation_basis(
        &refreshed,
        refreshed.repository_snapshot,
        &current_grounding,
    )?;
    assert!(explanation.evidence.iter().any(|evidence| historical_links
        .iter()
        .all(|link| evidence.canonical_links.contains(link))));
    assert_eq!(
        after,
        fixture.basis()?,
        "grounding reads mutated canonical state"
    );
    Ok(())
}

#[test]
fn invalid_ingress_and_read_side_audit_fail_closed_without_mutation() -> Result<(), Box<dyn Error>>
{
    let (_runtime, fixture) = canonical_fixture()?;
    let canonical_before = fixture.basis()?;
    let grounding = CanonicalGrounding::from_read_basis(&canonical_before)?;
    let root = fixture_root();
    let valid_links = vec![CanonicalReference::Decision(
        grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
    )];
    let analysis =
        analyze_with_links(&root, &grounding, fixture.repository.id, None, &valid_links)?;

    for error in [
        grounding
            .decision_reference(fixture.decision.id, 99)
            .err()
            .ok_or("wrong Decision revision was accepted")?,
        grounding
            .decision_reference(volicord_context::DecisionId::from_bytes([0xd1; 16]), 1)
            .err()
            .ok_or("nonexistent Decision was accepted")?,
        grounding
            .context_item_reference(fixture.context_item.id, 99)
            .err()
            .ok_or("wrong Context Item revision was accepted")?,
        grounding
            .checkpoint_reference(fixture.checkpoint.id, 99)
            .err()
            .ok_or("wrong Checkpoint revision was accepted")?,
    ] {
        assert!(matches!(
            error.issues()[0].kind,
            CanonicalGroundingIssueKind::RevisionMismatch
                | CanonicalGroundingIssueKind::DanglingTarget
        ));
    }

    let fabricated: CanonicalReference = serde_json::from_value(json!({
        "kind": "decision",
        "target": {
            "project": fixture.project.id.to_string(),
            "identity": volicord_context::DecisionId::from_bytes([0xfa; 16]).to_string(),
            "revision": 1
        }
    }))?;
    let error = analyze_with_links(
        &root,
        &grounding,
        fixture.repository.id,
        None,
        std::slice::from_ref(&fabricated),
    )
    .err()
    .ok_or("fabricated Decision link was accepted")?;
    assert!(error.to_string().contains("does not exist"));

    let analysis_before = analysis.clone();
    let mut dangling = analysis.clone();
    dangling.structural_facts[0]
        .entity
        .canonical_links
        .push(fabricated);
    let error = grounding
        .validate_analysis_snapshot(&dangling)
        .err()
        .ok_or("dangling persisted link was accepted")?;
    assert!(error
        .issues()
        .iter()
        .any(|issue| issue.kind == CanonicalGroundingIssueKind::DanglingTarget));

    let wrong_revision: CanonicalReference = serde_json::from_value(json!({
        "kind": "decision",
        "target": {
            "project": fixture.project.id.to_string(),
            "identity": fixture.decision.id.to_string(),
            "revision": 99
        }
    }))?;
    let mut mismatched = analysis.clone();
    mismatched.structural_facts[0]
        .entity
        .canonical_links
        .push(wrong_revision);
    assert!(grounding
        .validate_analysis_snapshot(&mismatched)
        .err()
        .is_some_and(|error| error
            .issues()
            .iter()
            .any(|issue| issue.kind == CanonicalGroundingIssueKind::RevisionMismatch)));

    let (_foreign_runtime, foreign_fixture) = canonical_fixture()?;
    let foreign_grounding = CanonicalGrounding::from_read_basis(&foreign_fixture.basis()?)?;
    let wrong_project = CanonicalReference::Decision(foreign_grounding.decision_reference(
        foreign_fixture.decision.id,
        foreign_fixture.decision.revision,
    )?);
    let mut foreign = analysis.clone();
    foreign.structural_facts[0]
        .entity
        .canonical_links
        .push(wrong_project);
    assert!(grounding
        .validate_analysis_snapshot(&foreign)
        .err()
        .is_some_and(|error| error
            .issues()
            .iter()
            .any(|issue| issue.kind == CanonicalGroundingIssueKind::WrongProject)));

    let mut source_value = serde_json::to_value(&analysis.repository_source)?;
    source_value["basis"] = json!({"kind": "snapshot", "value": "wrong-snapshot"});
    let mut wrong_source_basis = analysis.clone();
    wrong_source_basis.repository_source = serde_json::from_value(source_value)?;
    assert!(grounding
        .validate_analysis_snapshot(&wrong_source_basis)
        .err()
        .is_some_and(|error| error
            .issues()
            .iter()
            .any(|issue| issue.kind == CanonicalGroundingIssueKind::SourceBasisMismatch)));

    assert_eq!(
        analysis, analysis_before,
        "read-side audit mutated analysis"
    );
    assert_eq!(
        canonical_before,
        fixture.basis()?,
        "read-side audit mutated canonical state"
    );
    Ok(())
}

#[test]
fn repository_source_must_exist_in_the_analysis_project() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let missing = volicord_context::SourceId::from_bytes([0xce; 16]);
    let error = InventoryRequest::new(&fixture_root(), &grounding, missing, OBSERVED_AT)
        .err()
        .ok_or("missing or foreign repository Source was accepted")?;
    assert_eq!(
        error.issues()[0].kind,
        CanonicalGroundingIssueKind::DanglingTarget
    );
    let foreign = support::repository_grounding(0xa1, 0xa2)?;
    let foreign_error =
        InventoryRequest::new(&fixture_root(), &grounding, foreign.source_id, OBSERVED_AT)
            .err()
            .ok_or("repository Source from another Project was accepted")?;
    assert_eq!(
        foreign_error.issues()[0].kind,
        CanonicalGroundingIssueKind::DanglingTarget
    );
    let non_repository = InventoryRequest::new(
        &fixture_root(),
        &grounding,
        fixture.no_snapshot_source.id,
        OBSERVED_AT,
    )
    .err()
    .ok_or("non-repository Source was accepted as a Repository Snapshot basis")?;
    assert_eq!(
        non_repository.issues()[0].kind,
        CanonicalGroundingIssueKind::InvalidRepositorySource
    );
    Ok(())
}

#[test]
fn repository_source_project_and_snapshot_basis_are_grounded() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;

    let repository = grounding.source_reference(fixture.repository.id)?;
    assert_eq!(repository.project(), fixture.project.id);
    assert_eq!(
        repository.basis(),
        &CanonicalSourceBasis::Snapshot("typescript-fixture-snapshot".to_owned())
    );
    assert_eq!(
        grounding
            .source_reference(fixture.no_snapshot_source.id)?
            .basis(),
        &CanonicalSourceBasis::NotApplicable
    );
    assert_eq!(
        grounding
            .source_reference_at(
                fixture.repository.id,
                CanonicalSourceBasis::Snapshot("unobserved-snapshot".to_owned()),
            )
            .unwrap_err()
            .issues()[0]
            .kind,
        CanonicalGroundingIssueKind::SourceBasisMismatch
    );

    let analysis = analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        &[],
    )?;
    assert_eq!(analysis.repository_source, repository);
    grounding.validate_analysis_snapshot(&analysis)?;
    Ok(())
}

#[test]
fn dangling_canonical_targets_are_rejected() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let fabricated = [
        wire_revision_reference(
            "decision",
            fixture.project.id,
            volicord_context::DecisionId::from_bytes([0xd1; 16]),
            1,
        )?,
        wire_revision_reference(
            "context_item",
            fixture.project.id,
            volicord_context::ContextItemId::from_bytes([0xd2; 16]),
            1,
        )?,
        wire_revision_reference(
            "checkpoint",
            fixture.project.id,
            volicord_context::CheckpointId::from_bytes([0xd3; 16]),
            1,
        )?,
    ];
    for reference in &fabricated {
        assert_eq!(
            grounding
                .validate_reference(reference)
                .unwrap_err()
                .issues()[0]
                .kind,
            CanonicalGroundingIssueKind::DanglingTarget
        );
    }

    assert!(analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        &fabricated[..1],
    )
    .is_err());
    Ok(())
}

#[test]
fn cross_project_canonical_targets_are_rejected() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let (_foreign_runtime, foreign) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let foreign_grounding = CanonicalGrounding::from_read_basis(&foreign.basis()?)?;
    let foreign_references = [
        CanonicalReference::Source(foreign_grounding.source_reference(foreign.repository.id)?),
        CanonicalReference::Decision(
            foreign_grounding.decision_reference(foreign.decision.id, foreign.decision.revision)?,
        ),
        CanonicalReference::ContextItem(
            foreign_grounding
                .context_item_reference(foreign.context_item.id, foreign.context_item.revision)?,
        ),
        CanonicalReference::Checkpoint(
            foreign_grounding
                .checkpoint_reference(foreign.checkpoint.id, foreign.checkpoint.revision)?,
        ),
    ];
    for reference in &foreign_references {
        assert_eq!(
            grounding
                .validate_reference(reference)
                .unwrap_err()
                .issues()[0]
                .kind,
            CanonicalGroundingIssueKind::WrongProject
        );
    }
    assert!(InventoryRequest::new(
        &fixture_root(),
        &grounding,
        foreign.repository.id,
        OBSERVED_AT,
    )
    .is_err());
    Ok(())
}

#[test]
fn impossible_and_unknown_canonical_revisions_are_rejected() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let invalid = [
        wire_revision_reference("decision", fixture.project.id, fixture.decision.id, 0)?,
        wire_revision_reference("decision", fixture.project.id, fixture.decision.id, 2)?,
        wire_revision_reference(
            "context_item",
            fixture.project.id,
            fixture.context_item.id,
            0,
        )?,
        wire_revision_reference(
            "context_item",
            fixture.project.id,
            fixture.context_item.id,
            2,
        )?,
        wire_revision_reference("checkpoint", fixture.project.id, fixture.checkpoint.id, 0)?,
        wire_revision_reference("checkpoint", fixture.project.id, fixture.checkpoint.id, 2)?,
    ];
    for reference in &invalid {
        assert_eq!(
            grounding
                .validate_reference(reference)
                .unwrap_err()
                .issues()[0]
                .kind,
            CanonicalGroundingIssueKind::RevisionMismatch
        );
    }
    Ok(())
}

#[test]
fn historical_revisions_remain_grounded_after_non_semantic_correction() -> Result<(), Box<dyn Error>>
{
    let (_runtime, mut fixture) = canonical_fixture()?;
    let initial_grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let historical_decision = CanonicalReference::Decision(
        initial_grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
    );
    let historical_context = CanonicalReference::ContextItem(
        initial_grounding
            .context_item_reference(fixture.context_item.id, fixture.context_item.revision)?,
    );
    let analysis = analyze_with_links(
        &fixture_root(),
        &initial_grounding,
        fixture.repository.id,
        None,
        &[historical_decision.clone(), historical_context.clone()],
    )?;

    let corrected_decision = fixture
        .store
        .correct_decision(
            operation(0x81),
            fixture.project.id,
            fixture.decision.id,
            DecisionCorrectionDraft {
                expected_revision: fixture.decision.revision,
                corrected_user_rationale: Some("Keep  historical grounding".to_owned()),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_turn.id,
            },
        )?
        .value;
    let corrected_context = fixture
        .store
        .correct_context_item(
            operation(0x82),
            fixture.project.id,
            fixture.context_item.id,
            ContextItemCorrectionDraft {
                expected_revision: fixture.context_item.revision,
                corrected_statement: "Repository links  preserve exact basis.".to_owned(),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_turn.id,
            },
        )?
        .value;

    let current_grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    current_grounding.validate_reference(&historical_decision)?;
    current_grounding.validate_reference(&historical_context)?;
    current_grounding.validate_analysis_snapshot(&analysis)?;
    assert!(current_grounding
        .decision_reference(fixture.decision.id, corrected_decision.revision)
        .is_ok());
    assert!(current_grounding
        .context_item_reference(fixture.context_item.id, corrected_context.revision)
        .is_ok());
    assert!(linked_entity(&analysis)?
        .canonical_links
        .contains(&historical_decision));
    assert!(linked_entity(&analysis)?
        .canonical_links
        .contains(&historical_context));
    Ok(())
}

#[test]
fn analysis_refresh_does_not_rebind_historical_references() -> Result<(), Box<dyn Error>> {
    let (_runtime, mut fixture) = canonical_fixture()?;
    let initial_grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let historical = [
        CanonicalReference::Decision(
            initial_grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
        ),
        CanonicalReference::ContextItem(
            initial_grounding
                .context_item_reference(fixture.context_item.id, fixture.context_item.revision)?,
        ),
    ];
    let analysis = analyze_with_links(
        &fixture_root(),
        &initial_grounding,
        fixture.repository.id,
        None,
        &historical,
    )?;
    let corrected_decision = fixture
        .store
        .correct_decision(
            operation(0x83),
            fixture.project.id,
            fixture.decision.id,
            DecisionCorrectionDraft {
                expected_revision: fixture.decision.revision,
                corrected_user_rationale: Some("Keep  historical grounding".to_owned()),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_turn.id,
            },
        )?
        .value;
    let corrected_context = fixture
        .store
        .correct_context_item(
            operation(0x84),
            fixture.project.id,
            fixture.context_item.id,
            ContextItemCorrectionDraft {
                expected_revision: fixture.context_item.revision,
                corrected_statement: "Repository links  preserve exact basis.".to_owned(),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_turn.id,
            },
        )?
        .value;
    let current_grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let refreshed = analyze_with_links(
        &fixture_root(),
        &current_grounding,
        fixture.repository.id,
        Some(&analysis),
        &[],
    )?;
    let links = &linked_entity(&refreshed)?.canonical_links;
    assert!(historical.iter().all(|reference| links.contains(reference)));
    assert!(!links.iter().any(|reference| matches!(
        reference,
        CanonicalReference::Decision(value)
            if value.identity() == fixture.decision.id
                && value.revision() == corrected_decision.revision
    )));
    assert!(!links.iter().any(|reference| matches!(
        reference,
        CanonicalReference::ContextItem(value)
            if value.identity() == fixture.context_item.id
                && value.revision() == corrected_context.revision
    )));
    current_grounding.validate_analysis_snapshot(&refreshed)?;
    Ok(())
}

#[test]
fn decision_supersession_does_not_redirect_existing_analysis() -> Result<(), Box<dyn Error>> {
    let (_runtime, mut fixture) = canonical_fixture()?;
    let initial_grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let original = CanonicalReference::Decision(
        initial_grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
    );
    let analysis = analyze_with_links(
        &fixture_root(),
        &initial_grounding,
        fixture.repository.id,
        None,
        std::slice::from_ref(&original),
    )?;
    let successor = fixture
        .store
        .supersede_decision(
            operation(0x85),
            fixture.project.id,
            DecisionSupersessionDraft {
                expected_project_revision: fixture.project.revision,
                previous_decision_id: fixture.decision.id,
                user_turn_source: UserTurnSource::Existing(fixture.user_turn.id),
                choice: DecisionChoice::Alternative {
                    alternative_key: "latest".to_owned(),
                },
                user_rationale: Some("This is a distinct semantic choice".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value;
    let current_grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    current_grounding.validate_analysis_snapshot(&analysis)?;
    let links = &linked_entity(&analysis)?.canonical_links;
    assert!(links.contains(&original));
    assert!(!links.iter().any(|reference| matches!(
        reference,
        CanonicalReference::Decision(value) if value.identity() == successor.id
    )));
    assert_ne!(fixture.decision.id, successor.id);
    Ok(())
}

#[test]
fn persisted_analysis_snapshot_links_are_revalidated_on_read() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let decision = CanonicalReference::Decision(
        grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
    );
    let analysis = analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        std::slice::from_ref(&decision),
    )?;
    let bytes = canonical_json(&analysis)?;
    let reconstructed: volicord_repository_intelligence::AnalysisSnapshot =
        serde_json::from_slice(&bytes)?;
    grounding.validate_analysis_snapshot(&reconstructed)?;

    let mut wrong_source = serde_json::to_value(&reconstructed)?;
    wrong_source["repository_source"]["basis"] =
        json!({"kind": "snapshot", "value": "unobserved-snapshot"});
    let wrong_source: volicord_repository_intelligence::AnalysisSnapshot =
        serde_json::from_value(wrong_source)?;
    let error = grounding
        .validate_analysis_snapshot(&wrong_source)
        .unwrap_err();
    assert!(error
        .issues()
        .iter()
        .any(|issue| issue.kind == CanonicalGroundingIssueKind::SourceBasisMismatch));
    assert!(search_local(
        &wrong_source,
        "Greeter",
        wrong_source.repository_snapshot,
        10,
        &grounding,
    )
    .is_err());
    assert!(grounded_explanation_basis(
        &wrong_source,
        wrong_source.repository_snapshot,
        &grounding,
    )
    .is_err());

    let mut wrong_revision = serde_json::to_value(&reconstructed)?;
    let facts = wrong_revision["structural_facts"]
        .as_array_mut()
        .ok_or("serialized structural facts were not an array")?;
    let entity = facts
        .iter_mut()
        .find(|fact| fact["entity"]["qualified_name"] == json!("Greeter.greet"))
        .ok_or("serialized linked entity missing")?;
    let links = entity["entity"]["canonical_links"]
        .as_array_mut()
        .ok_or("serialized canonical links were not an array")?;
    let decision_link = links
        .iter_mut()
        .find(|reference| reference["kind"] == json!("decision"))
        .ok_or("serialized Decision link missing")?;
    decision_link["target"]["revision"] = json!(fixture.decision.revision + 1);
    let wrong_revision: volicord_repository_intelligence::AnalysisSnapshot =
        serde_json::from_value(wrong_revision)?;
    assert!(grounding
        .validate_analysis_snapshot(&wrong_revision)
        .unwrap_err()
        .issues()
        .iter()
        .any(|issue| issue.kind == CanonicalGroundingIssueKind::RevisionMismatch));
    Ok(())
}

#[test]
fn canonical_grounding_validation_has_no_mutation_authority() -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let before = fixture.basis()?;
    let grounding = CanonicalGrounding::from_read_basis(&before)?;
    let valid = CanonicalReference::Decision(
        grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
    );
    let analysis = analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        std::slice::from_ref(&valid),
    )?;
    grounding.validate_analysis_snapshot(&analysis)?;
    let fabricated = wire_revision_reference(
        "checkpoint",
        fixture.project.id,
        volicord_context::CheckpointId::from_bytes([0xef; 16]),
        1,
    )?;
    assert!(grounding.validate_reference(&fabricated).is_err());
    assert_eq!(before, fixture.basis()?);
    Ok(())
}

#[test]
fn grounded_reference_serialization_is_deterministic_and_current_only() -> Result<(), Box<dyn Error>>
{
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let links = [
        CanonicalReference::Source(grounding.source_reference(fixture.no_snapshot_source.id)?),
        CanonicalReference::Decision(
            grounding.decision_reference(fixture.decision.id, fixture.decision.revision)?,
        ),
        CanonicalReference::ContextItem(
            grounding
                .context_item_reference(fixture.context_item.id, fixture.context_item.revision)?,
        ),
        CanonicalReference::Checkpoint(
            grounding.checkpoint_reference(fixture.checkpoint.id, fixture.checkpoint.revision)?,
        ),
    ];
    let first = analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        &links,
    )?;
    let repeated = analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        &links,
    )?;
    let first_bytes = canonical_json(&first)?;
    assert_eq!(first.identity, repeated.identity);
    assert_eq!(first_bytes, canonical_json(&repeated)?);
    assert_eq!(first.format_version, ANALYSIS_SNAPSHOT_FORMAT_VERSION);
    let text = String::from_utf8(first_bytes.clone())?;
    assert!(text.contains("\"revision\":1"));
    assert!(text.contains("\"kind\":\"snapshot\""));
    assert!(text.contains("\"kind\":\"not_applicable\""));

    let mut prior = serde_json::to_value(&first)?;
    prior["format_version"] = json!(ANALYSIS_SNAPSHOT_FORMAT_VERSION - 1);
    assert!(
        serde_json::from_value::<volicord_repository_intelligence::AnalysisSnapshot>(prior)
            .is_err()
    );
    Ok(())
}

#[test]
fn automatic_and_manual_reference_ingress_is_grounded_before_consumption(
) -> Result<(), Box<dyn Error>> {
    let (_runtime, fixture) = canonical_fixture()?;
    let grounding = CanonicalGrounding::from_read_basis(&fixture.basis()?)?;
    let manual = CanonicalReference::Checkpoint(
        grounding.checkpoint_reference(fixture.checkpoint.id, fixture.checkpoint.revision)?,
    );
    let analysis = analyze_with_links(
        &fixture_root(),
        &grounding,
        fixture.repository.id,
        None,
        std::slice::from_ref(&manual),
    )?;
    let automatic = CanonicalReference::Source(analysis.repository_source.clone());
    assert!(analysis.structural_facts.iter().all(|fact| {
        fact.entity.source == analysis.repository_source
            && fact.entity.canonical_links.contains(&automatic)
            && fact
                .entity
                .source_range
                .as_ref()
                .is_some_and(|range| range.source == analysis.repository_source)
            && fact
                .provenance
                .analysis
                .source_basis
                .contains(&analysis.repository_source)
            && fact.relations.iter().all(|relation| {
                relation
                    .supporting_range
                    .as_ref()
                    .is_none_or(|range| range.source == analysis.repository_source)
            })
    }));
    assert!(linked_entity(&analysis)?.canonical_links.contains(&manual));
    grounding.validate_analysis_snapshot(&analysis)?;

    let explanation =
        grounded_explanation_basis(&analysis, analysis.repository_snapshot, &grounding)?;
    assert!(explanation
        .evidence
        .iter()
        .any(|evidence| evidence.canonical_links.contains(&manual)));
    assert!(!search_local(
        &analysis,
        "Greeter",
        analysis.repository_snapshot,
        10,
        &grounding,
    )?
    .is_empty());
    Ok(())
}
mod support;
