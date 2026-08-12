use std::fs;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, CanonicalRecordId,
    CheckpointDraft, CheckpointKind, ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole,
    CorrectionKind, DecisionChoice, DecisionSupersessionDraft, DeterministicIdGenerator,
    ExplicitQuestionResponse, FixedClock, MergeResolution, MergeResolutionMode, OperationId,
    Principal, PrincipalKind, QuestionAlternative, QuestionDraft, QuestionResponseDraft,
    ReviewDueDraft, ReviewDueKind, SourceDraft, SourceFreshness, SourcePayload,
    StatementProvenanceRole, Store, TimestampMicros, UserAcceptanceFact, UserAcceptanceState,
    UserReviewFact, UserReviewState, UserTurnSource, VerificationFact, VerificationState,
    WorkState,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn store(path: &Path, ids: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_790_000_000_000_000)),
    )
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn user_turn(turn: &str) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload: SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "read-session".to_owned(),
            turn: turn.to_owned(),
        },
        actor: principal(PrincipalKind::User, "owner"),
        observer: None,
        availability: Availability::Available,
    }
}

fn fact(statement: &str, source: volicord_context::SourceId) -> ContextItemDraft {
    ContextItemDraft {
        expected_project_revision: 1,
        role: ContextItemRole::Fact,
        statement: statement.to_owned(),
        provenance_role: StatementProvenanceRole::Observed,
        author: principal(PrincipalKind::Agent, "codex"),
        source_basis: vec![source],
        applicability: ApplicabilityScope::default(),
    }
}

struct Fixture {
    project: volicord_context::Project,
    repository: volicord_context::Source,
    item: volicord_context::ContextItem,
    forgotten_item: volicord_context::ContextItem,
    question: volicord_context::Question,
    original_decision: volicord_context::Decision,
    active_decision: volicord_context::Decision,
    checkpoint: volicord_context::Checkpoint,
}

fn populate(store: &mut Store) -> Result<Fixture, Box<dyn std::error::Error>> {
    let project = store.create_project(operation(1), "Canonical read")?.value;
    let repository = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "read-commit".to_owned(),
                },
                actor: principal(PrincipalKind::Repository, "repository"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    let authorization = store
        .record_source(operation(3), project.id, user_turn("read-authorization"))?
        .value;
    let item = store
        .record_context_item(operation(4), project.id, fact("stable fact", repository.id))?
        .value;
    let forgotten_item = store
        .record_context_item(
            operation(5),
            project.id,
            fact("forgettable fact", repository.id),
        )?
        .value;
    let question = store
        .create_question(
            operation(6),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Choose read order".to_owned(),
                source_basis: vec![repository.id],
                dependencies: vec![],
                alternatives: vec![
                    QuestionAlternative {
                        key: "identity".to_owned(),
                        label: "Identity".to_owned(),
                        consequence: "Stable identity order".to_owned(),
                    },
                    QuestionAlternative {
                        key: "time".to_owned(),
                        label: "Time".to_owned(),
                        consequence: "Stable recorded time".to_owned(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("identity".to_owned()),
                    rationale: "Independent from environment".to_owned(),
                    source_basis: vec![repository.id],
                },
                trade_offs: vec!["No ranking".to_owned()],
                uncertainty: vec![],
                material_scope: vec!["canonical read".to_owned()],
            },
        )?
        .value;
    let original_decision = store
        .record_question_response(
            operation(7),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: question.id,
                question_revision: 1,
                user_turn_source: UserTurnSource::Existing(authorization.id),
                displayed_alternative_keys: vec!["identity".to_owned(), "time".to_owned()],
                displayed_recommendation_key: Some("identity".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "identity".to_owned(),
                    user_rationale: Some("Keep authority deterministic".to_owned()),
                },
                applicability: ApplicabilityScope {
                    paths: vec!["rebuild/".to_owned()],
                    components: vec!["context".to_owned()],
                    work_contexts: vec!["recall basis".to_owned()],
                },
                assumptions: vec!["canonical identities are stable".to_owned()],
                revisit_triggers: vec!["identity collision".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or("Decision missing")?;
    let checkpoint = store
        .record_checkpoint(
            operation(8),
            project.id,
            CheckpointDraft {
                expected_project_revision: 1,
                kind: CheckpointKind::Handoff,
                goal: "Expose canonical read input".to_owned(),
                work_state: WorkState::Completed,
                state_change: Some("Read basis prepared".to_owned()),
                source_basis: vec![repository.id],
                changed_source_basis: vec![repository.id],
                changed_paths: vec!["rebuild/crates/volicord-context".to_owned()],
                applied_decisions: vec![original_decision.id],
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
                known_limits: vec!["No natural-language Recall".to_owned()],
                non_goals: vec!["Ranking".to_owned()],
                open_questions: vec![],
                next_step: "Build projection later".to_owned(),
                handoff_to: Some("Recall".to_owned()),
            },
        )?
        .value;
    store.correct_context_item(
        operation(9),
        project.id,
        item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "stable  fact".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: authorization.id,
        },
    )?;
    let active_decision = store
        .supersede_decision(
            operation(10),
            project.id,
            DecisionSupersessionDraft {
                expected_project_revision: 1,
                previous_decision_id: original_decision.id,
                user_turn_source: UserTurnSource::Existing(authorization.id),
                choice: DecisionChoice::Alternative {
                    alternative_key: "time".to_owned(),
                },
                user_rationale: Some("Use time only for checkpoint chronology".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["UTC microseconds are durable".to_owned()],
                revisit_triggers: vec!["clock basis changes".to_owned()],
            },
        )?
        .value;
    store.mark_decision_review_due(
        operation(11),
        project.id,
        active_decision.id,
        ReviewDueDraft {
            kind: ReviewDueKind::AssumptionChanged,
            explanation: "Clock assumption needs review".to_owned(),
            source_basis: vec![repository.id],
        },
    )?;
    store.record_contradiction(
        operation(12),
        project.id,
        CanonicalRecordId::Decision(active_decision.id),
        CanonicalRecordId::Decision(original_decision.id),
    )?;
    store.forget_context_item(
        operation(13),
        project.id,
        forgotten_item.id,
        authorization.id,
    )?;
    Ok(Fixture {
        project,
        repository,
        item,
        forgotten_item,
        question,
        original_decision,
        active_decision,
        checkpoint,
    })
}

#[test]
fn read_basis_exposes_active_history_lifecycle_source_and_forgetting(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut value = store(
        &root.path().join("context.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let fixture = populate(&mut value)?;
    let basis = value.read_canonical_basis(
        fixture.project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    assert!(basis.active_questions.is_empty());
    assert_eq!(basis.terminal_question_history[0].id, fixture.question.id);
    assert_eq!(
        basis.active_decisions[0].decision.id,
        fixture.active_decision.id
    );
    assert!(basis.active_decisions[0].review_due.is_some());
    assert_eq!(
        basis.superseded_decisions[0].decision.id,
        fixture.original_decision.id
    );
    assert_eq!(basis.context_items[0].id, fixture.item.id);
    assert_eq!(
        basis.latest_checkpoint.as_ref().map(|value| value.id),
        Some(fixture.checkpoint.id)
    );
    assert_eq!(basis.checkpoint_history.len(), 1);
    assert!(basis
        .relations
        .iter()
        .any(|value| value.relation_kind == "supersedes"));
    assert!(basis
        .relations
        .iter()
        .any(|value| value.relation_kind == "contradicts"));
    assert!(basis.revisions.iter().any(|value| value.record_kind
        == volicord_context::CanonicalRecordKind::ContextItem
        && value.record_identity == fixture.item.id.to_string()
        && value.revisions == vec![1, 2]));
    assert_eq!(
        basis.forgotten[0].record_identity,
        fixture.forgotten_item.id.to_string()
    );
    let repository = basis
        .sources
        .iter()
        .find(|value| value.source.id == fixture.repository.id)
        .ok_or("Source missing")?;
    assert_eq!(repository.snapshot_basis.as_deref(), Some("read-commit"));
    assert_eq!(repository.freshness, SourceFreshness::Current);
    let bounded =
        value.read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    assert!(bounded.checkpoint_history.is_empty());
    assert_eq!(
        bounded.latest_checkpoint.as_ref().map(|value| value.id),
        Some(fixture.checkpoint.id)
    );
    Ok(())
}

#[test]
fn stable_order_survives_reopen_export_import_and_another_path_binding(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut value = store(&path, &[1, 2, 3, 4, 5, 6, 7, 8, 9])?;
    let fixture = populate(&mut value)?;
    let options = CanonicalReadOptions {
        include_checkpoint_history: true,
    };
    let before = value
        .read_canonical_basis(fixture.project.id, options)?
        .stable_ordering_identity;
    let bundle = root.path().join("context.json");
    value.export_bundle(fixture.project.id, &bundle)?;
    drop(value);
    let reopened = store(&path, &[])?;
    assert_eq!(
        reopened
            .read_canonical_basis(fixture.project.id, options)?
            .stable_ordering_identity,
        before
    );

    let destination_path = root.path().join("destination.sqlite3");
    let mut destination = store(&destination_path, &[44])?;
    destination.import_bundle(operation(40), &bundle)?;
    let imported = destination.read_canonical_basis(fixture.project.id, options)?;
    assert_eq!(imported.stable_ordering_identity, before);
    assert_eq!(
        imported
            .sources
            .iter()
            .find(|value| value.source.id == fixture.repository.id)
            .ok_or("imported Source missing")?
            .freshness,
        SourceFreshness::Unavailable
    );
    let clone = root.path().join("another-clone");
    fs::create_dir(&clone)?;
    destination.bind_clone(
        operation(41),
        fixture.project.id,
        None,
        &clone,
        Availability::Available,
    )?;
    assert_eq!(
        destination
            .read_canonical_basis(fixture.project.id, options)?
            .stable_ordering_identity,
        before
    );
    Ok(())
}

#[test]
fn merge_and_branch_basis_remain_ordered_and_portable_without_sources(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3])?;
    let project = origin.create_project(operation(50), "Merged read")?.value;
    let repository = origin
        .record_source(
            operation(51),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "merge-base".to_owned(),
                },
                actor: principal(PrincipalKind::Repository, "repository"),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let authorization = origin
        .record_source(operation(52), project.id, user_turn("merge-resolution"))?
        .value;
    let base = root.path().join("base.json");
    origin.export_bundle(project.id, &base)?;
    let mut local = store(&root.path().join("local.sqlite3"), &[21])?;
    local.import_bundle(operation(53), &base)?;
    let local_item = local
        .record_context_item(operation(54), project.id, fact("local", repository.id))?
        .value;
    let mut incoming = store(&root.path().join("incoming.sqlite3"), &[31])?;
    incoming.import_bundle(operation(55), &base)?;
    let incoming_item = incoming
        .record_context_item(operation(56), project.id, fact("incoming", repository.id))?
        .value;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(project.id, &incoming_bundle)?;
    local.merge_bundle(operation(57), Some(&base), &incoming_bundle, None, None)?;
    let unresolved = local.compare_bundle(None, &incoming_bundle, None)?;
    local.merge_bundle(
        operation(58),
        None,
        &incoming_bundle,
        None,
        Some(MergeResolution {
            conflict_set_identity: unresolved.conflict_set_identity,
            conflict_revision: 1,
            user_turn_source_id: authorization.id,
            mode: MergeResolutionMode::ContextBranch,
        }),
    )?;
    let basis = local.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    assert_eq!(
        basis
            .context_items
            .iter()
            .map(|value| value.id)
            .collect::<Vec<_>>(),
        {
            let mut ids = vec![local_item.id, incoming_item.id];
            ids.sort();
            ids
        }
    );
    assert_eq!(basis.bundle_merges.len(), 2);
    assert!(basis
        .bundle_merges
        .iter()
        .any(|value| value.branch_history_basis.is_some()
            && value.resolution_source_identity.as_deref() == Some(&authorization.id.to_string())));
    let merged_bundle = root.path().join("merged.json");
    local.export_bundle(project.id, &merged_bundle)?;
    let order = basis.stable_ordering_identity;
    let mut imported = store(&root.path().join("merged-import.sqlite3"), &[])?;
    imported.import_bundle(operation(59), &merged_bundle)?;
    assert_eq!(
        imported
            .read_canonical_basis(project.id, CanonicalReadOptions::default())?
            .stable_ordering_identity,
        order
    );
    Ok(())
}
