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
    analyze_repository_semantics, canonical_json, grounded_explanation_basis, CanonicalGrounding,
    CanonicalGroundingIssueKind, CanonicalLinkSelector, CanonicalReference, CanonicalSourceBasis,
    InventoryRequest, Language, SemanticAnalysisRequest, StructuralAnalysisRequest,
    ANALYSIS_SNAPSHOT_FORMAT_VERSION,
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
mod support;
