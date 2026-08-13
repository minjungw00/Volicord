use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, BundleConflictClass, BundleMergeStatus,
    CanonicalReadOptions, CanonicalRecordId, ContextItemCorrectionDraft, ContextItemDraft,
    ContextItemRole, CorrectionKind, DecisionChoice, DecisionCorrectionDraft,
    DecisionSupersessionDraft, DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse,
    FixedClock, MergeResolution, MergeResolutionMode, NonUserQuestionOutcome, OperationId,
    Principal, PrincipalKind, QuestionAlternative, QuestionDispositionDraft, QuestionDraft,
    QuestionResponseDraft, SourceBindingCandidate, SourceDraft, SourcePayload,
    StatementProvenanceRole, Store, TimestampMicros, UserTurnSource,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn store(path: &Path, ids: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_780_000_000_000_000)),
    )
}

fn user_turn(turn: &str) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload: SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "merge-session".to_owned(),
            turn: turn.to_owned(),
        },
        actor: Principal {
            kind: PrincipalKind::User,
            identity: "project-owner".to_owned(),
        },
        observer: None,
        availability: Availability::Available,
    }
}

fn agent_context(statement: &str, source: volicord_context::SourceId) -> ContextItemDraft {
    ContextItemDraft {
        expected_project_revision: 1,
        role: ContextItemRole::Fact,
        statement: statement.to_owned(),
        provenance_role: StatementProvenanceRole::Observed,
        author: Principal {
            kind: PrincipalKind::Agent,
            identity: "codex".to_owned(),
        },
        source_basis: vec![source],
        applicability: ApplicabilityScope::default(),
    }
}

struct Base {
    project: volicord_context::Project,
    repository: volicord_context::Source,
    authorization: volicord_context::Source,
    item: volicord_context::ContextItem,
    decision: volicord_context::Decision,
}

fn create_base(store: &mut Store) -> Result<Base, Box<dyn std::error::Error>> {
    let project = store
        .create_project(operation(1), "Divergent context")?
        .value;
    let repository = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "base-commit".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".to_owned(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let authorization = store
        .record_source(operation(3), project.id, user_turn("base-authorization"))?
        .value;
    let item = store
        .record_context_item(
            operation(4),
            project.id,
            agent_context("base fact", repository.id),
        )?
        .value;
    let question = store
        .create_question(
            operation(5),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Which mode?".to_owned(),
                source_basis: vec![repository.id],
                dependencies: vec![],
                alternatives: vec![
                    QuestionAlternative {
                        key: "a".to_owned(),
                        label: "A".to_owned(),
                        consequence: "Use A".to_owned(),
                    },
                    QuestionAlternative {
                        key: "b".to_owned(),
                        label: "B".to_owned(),
                        consequence: "Use B".to_owned(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("a".to_owned()),
                    rationale: "Base recommendation".to_owned(),
                    source_basis: vec![repository.id],
                },
                trade_offs: vec!["compatibility".to_owned()],
                uncertainty: vec![],
                material_scope: vec!["merge".to_owned()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 0,
                why_it_matters_now: "merge behavior depends on this choice".to_owned(),
                established_facts: vec![],
                assumptions: vec![],
                known_limits: vec![],
                what_the_answer_unlocks: vec!["merge resolution".to_owned()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    let response = store
        .record_question_response(
            operation(6),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: question.id,
                question_revision: 1,
                user_turn_source: UserTurnSource::Existing(authorization.id),
                displayed_alternative_keys: vec!["a".to_owned(), "b".to_owned()],
                displayed_recommendation_key: Some("a".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "a".to_owned(),
                    user_rationale: Some("base rationale".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["base assumption".to_owned()],
                revisit_triggers: vec![],
            },
        )?
        .value;
    Ok(Base {
        project,
        repository,
        authorization,
        item,
        decision: response.decision.ok_or("Decision missing")?,
    })
}

fn clone_from(
    path: &Path,
    bundle: &Path,
    ids: &[u8],
    operation_byte: u8,
) -> Result<Store, Box<dyn std::error::Error>> {
    let mut value = store(path, ids)?;
    value.import_bundle(operation(operation_byte), bundle)?;
    Ok(value)
}

#[test]
fn verified_base_auto_merges_independent_additions_and_replays(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3, 4, 5, 6])?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;
    let mut local = clone_from(&root.path().join("local.sqlite3"), &base_bundle, &[31], 20)?;
    let local_item = local
        .record_context_item(
            operation(21),
            base.project.id,
            agent_context("local addition", base.repository.id),
        )?
        .value;
    let mut incoming = clone_from(
        &root.path().join("incoming.sqlite3"),
        &base_bundle,
        &[41],
        30,
    )?;
    let incoming_item = incoming
        .record_context_item(
            operation(31),
            base.project.id,
            agent_context("incoming addition", base.repository.id),
        )?
        .value;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(base.project.id, &incoming_bundle)?;

    let comparison = local.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    assert!(!comparison.requires_user_resolution());
    assert!(comparison.conflicts.iter().any(|value| {
        value.class == BundleConflictClass::IndependentAdditions
            && value.automatic_resolution_allowed
    }));
    let merged = local.merge_bundle(
        operation(40),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        None,
    )?;
    assert_eq!(merged.value.status, BundleMergeStatus::MergedAutomatically);
    assert_eq!(
        local
            .get_context_item(base.project.id, local_item.id)?
            .statement,
        "local addition"
    );
    assert_eq!(
        local
            .get_context_item(base.project.id, incoming_item.id)?
            .statement,
        "incoming addition"
    );
    assert!(
        local
            .merge_bundle(
                operation(40),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                None
            )?
            .replayed
    );
    let changed = SourceBindingCandidate {
        repository_identity: "different".to_owned(),
        source_basis: vec![base.repository.id],
    };
    assert_eq!(
        local
            .merge_bundle(
                operation(40),
                Some(&base_bundle),
                &incoming_bundle,
                Some(changed),
                None
            )
            .err()
            .ok_or("changed replay accepted")?
            .kind(),
        ErrorKind::DomainConflict
    );
    Ok(())
}

#[test]
fn semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3, 4, 5, 6])?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;
    let mut local = clone_from(
        &root.path().join("local.sqlite3"),
        &base_bundle,
        &[31, 32],
        20,
    )?;
    local.supersede_decision(
        operation(21),
        base.project.id,
        DecisionSupersessionDraft {
            expected_project_revision: 1,
            previous_decision_id: base.decision.id,
            user_turn_source: UserTurnSource::Existing(base.authorization.id),
            choice: DecisionChoice::Alternative {
                alternative_key: "a".to_owned(),
            },
            user_rationale: Some("local rationale".to_owned()),
            applicability: ApplicabilityScope::default(),
            assumptions: vec![],
            revisit_triggers: vec![],
        },
    )?;
    let mut incoming = clone_from(
        &root.path().join("incoming.sqlite3"),
        &base_bundle,
        &[41, 42],
        30,
    )?;
    incoming.supersede_decision(
        operation(31),
        base.project.id,
        DecisionSupersessionDraft {
            expected_project_revision: 1,
            previous_decision_id: base.decision.id,
            user_turn_source: UserTurnSource::Existing(base.authorization.id),
            choice: DecisionChoice::Alternative {
                alternative_key: "b".to_owned(),
            },
            user_rationale: Some("incoming rationale".to_owned()),
            applicability: ApplicabilityScope::default(),
            assumptions: vec![],
            revisit_triggers: vec![],
        },
    )?;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(base.project.id, &incoming_bundle)?;
    let comparison = local.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    assert_eq!(comparison.conflict_revision, 1);
    assert!(comparison.requires_user_resolution());
    let semantic_conflict = comparison
        .conflicts
        .iter()
        .find(|value| value.class == BundleConflictClass::SemanticDecisionConflict)
        .ok_or("semantic Decision conflict missing")?;
    assert!(!semantic_conflict.automatic_resolution_allowed);
    assert!(semantic_conflict.user_judgment_reason.is_some());
    assert_eq!(
        semantic_conflict.base_basis.as_deref(),
        comparison
            .common_base
            .as_ref()
            .map(|value| value.history_basis.as_str())
    );
    assert_eq!(
        semantic_conflict.local_basis,
        comparison.local.history_basis
    );
    assert_eq!(
        semantic_conflict.incoming_basis,
        comparison.incoming.history_basis
    );
    assert!(!semantic_conflict.consequence.is_empty());
    assert!(!semantic_conflict.affected_identities.is_empty());
    assert!(!semantic_conflict.sources.is_empty());
    assert_eq!(
        local
            .merge_bundle(
                operation(40),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                None
            )?
            .value
            .status,
        BundleMergeStatus::Unresolved
    );
    let stale = MergeResolution {
        conflict_set_identity: "0".repeat(64),
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseIncoming,
    };
    assert_eq!(
        local
            .merge_bundle(
                operation(41),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                Some(stale)
            )
            .err()
            .ok_or("stale resolution accepted")?
            .kind(),
        ErrorKind::StaleBasis
    );
    let stale_revision = MergeResolution {
        conflict_set_identity: comparison.conflict_set_identity.clone(),
        conflict_revision: comparison.conflict_revision + 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseIncoming,
    };
    assert_eq!(
        local
            .merge_bundle(
                operation(41),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                Some(stale_revision)
            )
            .err()
            .ok_or("stale conflict revision accepted")?
            .kind(),
        ErrorKind::StaleBasis
    );
    let resolution = MergeResolution {
        conflict_set_identity: comparison.conflict_set_identity.clone(),
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ContextBranch,
    };
    let branched = local.merge_bundle(
        operation(42),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        Some(resolution),
    )?;
    assert_eq!(branched.value.status, BundleMergeStatus::Branched);
    assert_eq!(
        branched.value.branch_history_basis.as_deref(),
        Some(comparison.incoming.history_basis.as_str())
    );
    assert_eq!(
        branched.value.resolution_source_id,
        Some(base.authorization.id)
    );

    let mut choose_incoming = clone_from(
        &root.path().join("choose-incoming.sqlite3"),
        &base_bundle,
        &[31],
        50,
    )?;
    choose_incoming.supersede_decision(
        operation(51),
        base.project.id,
        DecisionSupersessionDraft {
            expected_project_revision: 1,
            previous_decision_id: base.decision.id,
            user_turn_source: UserTurnSource::Existing(base.authorization.id),
            choice: DecisionChoice::Alternative {
                alternative_key: "a".to_owned(),
            },
            user_rationale: Some("local rationale".to_owned()),
            applicability: ApplicabilityScope::default(),
            assumptions: vec![],
            revisit_triggers: vec![],
        },
    )?;
    let choose_comparison =
        choose_incoming.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    let choose_resolution = MergeResolution {
        conflict_set_identity: choose_comparison.conflict_set_identity.clone(),
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseIncoming,
    };
    let chosen = choose_incoming.merge_bundle(
        operation(52),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        Some(choose_resolution.clone()),
    )?;
    assert_eq!(chosen.value.status, BundleMergeStatus::Resolved);
    assert_eq!(
        choose_incoming
            .get_current_decision(base.project.id, base.decision.question_id)?
            .decision
            .choice,
        DecisionChoice::Alternative {
            alternative_key: "b".to_owned()
        }
    );
    assert!(
        choose_incoming
            .merge_bundle(
                operation(52),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                Some(choose_resolution),
            )?
            .replayed
    );
    let changed_resolution = MergeResolution {
        conflict_set_identity: choose_comparison.conflict_set_identity,
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseLocal,
    };
    assert_eq!(
        choose_incoming
            .merge_bundle(
                operation(52),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                Some(changed_resolution),
            )
            .err()
            .ok_or("changed merge resolution replay accepted")?
            .kind(),
        ErrorKind::DomainConflict
    );

    let explicit_path = root.path().join("explicit.sqlite3");
    let mut explicit = clone_from(&explicit_path, &base_bundle, &[31], 60)?;
    explicit.supersede_decision(
        operation(61),
        base.project.id,
        DecisionSupersessionDraft {
            expected_project_revision: 1,
            previous_decision_id: base.decision.id,
            user_turn_source: UserTurnSource::Existing(base.authorization.id),
            choice: DecisionChoice::Alternative {
                alternative_key: "a".to_owned(),
            },
            user_rationale: Some("local rationale".to_owned()),
            applicability: ApplicabilityScope::default(),
            assumptions: vec![],
            revisit_triggers: vec![],
        },
    )?;
    let explicit_comparison =
        explicit.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    let mut curated = clone_from(
        &root.path().join("explicit-result.sqlite3"),
        &incoming_bundle,
        &[61],
        63,
    )?;
    let curated_item = curated
        .record_context_item(
            operation(64),
            base.project.id,
            agent_context("explicitly merged context", base.repository.id),
        )?
        .value;
    let explicit_bundle = root.path().join("explicit-result.json");
    curated.export_bundle(base.project.id, &explicit_bundle)?;
    let explicit_resolution = MergeResolution {
        conflict_set_identity: explicit_comparison.conflict_set_identity,
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ExplicitMerged {
            bundle_path: explicit_bundle,
        },
    };
    let explicit_result = explicit.merge_bundle(
        operation(62),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        Some(explicit_resolution),
    )?;
    assert_eq!(explicit_result.value.status, BundleMergeStatus::Resolved);
    assert_eq!(
        explicit
            .get_context_item(base.project.id, curated_item.id)?
            .statement,
        "explicitly merged context"
    );

    let interrupted_path = root.path().join("interrupted.sqlite3");
    let mut interrupted = clone_from(&interrupted_path, &base_bundle, &[31], 70)?;
    let local_decision = interrupted
        .supersede_decision(
            operation(71),
            base.project.id,
            DecisionSupersessionDraft {
                expected_project_revision: 1,
                previous_decision_id: base.decision.id,
                user_turn_source: UserTurnSource::Existing(base.authorization.id),
                choice: DecisionChoice::Alternative {
                    alternative_key: "a".to_owned(),
                },
                user_rationale: Some("local rationale".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value;
    let interrupted_comparison =
        interrupted.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    drop(interrupted);
    let connection = Connection::open(&interrupted_path)?;
    connection.execute_batch(
        "CREATE TRIGGER interrupt_merge BEFORE INSERT ON decisions
         BEGIN SELECT RAISE(ABORT, 'interrupted merge'); END;",
    )?;
    drop(connection);
    let mut interrupted = store(&interrupted_path, &[])?;
    let interrupted_resolution = MergeResolution {
        conflict_set_identity: interrupted_comparison.conflict_set_identity,
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseIncoming,
    };
    assert_eq!(
        interrupted
            .merge_bundle(
                operation(72),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                Some(interrupted_resolution),
            )
            .err()
            .ok_or("interrupted merge succeeded")?
            .kind(),
        ErrorKind::TransactionFailure
    );
    assert_eq!(
        interrupted
            .get_current_decision(base.project.id, base.decision.question_id)?
            .decision
            .id,
        local_decision.id
    );
    Ok(())
}

#[test]
fn delete_modify_binding_and_unavailable_base_are_never_automatic(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let clone_path = root.path().join("clone");
    fs::create_dir(&clone_path)?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3, 4, 5, 6, 7])?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;
    let mut local = clone_from(&root.path().join("local.sqlite3"), &base_bundle, &[31], 20)?;
    assert_eq!(
        local
            .get_source(base.project.id, base.repository.id)?
            .availability,
        Availability::Unavailable,
        "merge must not require the source repository to be locally available"
    );
    local.bind_clone(
        operation(22),
        base.project.id,
        None,
        &clone_path,
        Availability::Available,
    )?;
    local.forget_context_item(
        operation(23),
        base.project.id,
        base.item.id,
        base.authorization.id,
    )?;
    let mut incoming = clone_from(&root.path().join("incoming.sqlite3"), &base_bundle, &[], 30)?;
    incoming.correct_context_item(
        operation(31),
        base.project.id,
        base.item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "base  fact".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(base.project.id, &incoming_bundle)?;
    let binding = SourceBindingCandidate {
        repository_identity: "incoming-repository".to_owned(),
        source_basis: vec![base.repository.id],
    };
    let comparison =
        local.compare_bundle(Some(&base_bundle), &incoming_bundle, Some(binding.clone()))?;
    for class in [
        BundleConflictClass::DeleteModifyConflict,
        BundleConflictClass::SourceBindingConflict,
    ] {
        assert!(comparison
            .conflicts
            .iter()
            .any(|value| value.class == class && !value.automatic_resolution_allowed));
    }
    let unavailable = local.compare_bundle(None, &incoming_bundle, None)?;
    assert!(unavailable.conflicts.iter().any(|value| value.class
        == BundleConflictClass::CommonBaseUnavailable
        && !value.automatic_resolution_allowed));
    let deletion_resolution = MergeResolution {
        conflict_set_identity: comparison.conflict_set_identity,
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseLocal,
    };
    local.merge_bundle(
        operation(33),
        Some(&base_bundle),
        &incoming_bundle,
        Some(binding),
        Some(deletion_resolution.clone()),
    )?;
    assert!(
        local
            .merge_bundle(
                operation(33),
                Some(&base_bundle),
                &incoming_bundle,
                Some(SourceBindingCandidate {
                    repository_identity: "incoming-repository".to_owned(),
                    source_basis: vec![base.repository.id],
                }),
                Some(deletion_resolution),
            )?
            .replayed
    );
    let deletion_bundle = root.path().join("deletion-result.json");
    local.export_bundle(base.project.id, &deletion_bundle)?;
    let deletion_bytes = fs::read(&deletion_bundle)?;
    assert!(
        !deletion_bytes
            .windows("base  fact".len())
            .any(|window| window == b"base  fact"),
        "choosing the deletion side retained incoming modified content"
    );
    assert!(local
        .get_context_item(base.project.id, base.item.id)
        .is_err());
    assert_eq!(
        local
            .get_tombstone(
                base.project.id,
                CanonicalRecordId::ContextItem(base.item.id)
            )?
            .record,
        CanonicalRecordId::ContextItem(base.item.id)
    );

    let mut imported = store(&root.path().join("deletion-import.sqlite3"), &[])?;
    imported.import_bundle(operation(34), &deletion_bundle)?;
    let imported_basis =
        imported.read_canonical_basis(base.project.id, CanonicalReadOptions::default())?;
    assert!(!imported_basis
        .context_items
        .iter()
        .any(|value| value.id == base.item.id));
    assert!(imported_basis.forgotten.iter().any(|value| {
        value.record_identity == base.item.id.to_string()
            && value.record_kind == volicord_context::CanonicalRecordKind::ContextItem
    }));

    let mut choose_modified = clone_from(
        &root.path().join("context-choose-modified.sqlite3"),
        &base_bundle,
        &[],
        35,
    )?;
    choose_modified.forget_context_item(
        operation(36),
        base.project.id,
        base.item.id,
        base.authorization.id,
    )?;
    let modified_comparison =
        choose_modified.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    choose_modified.merge_bundle(
        operation(37),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        Some(MergeResolution {
            conflict_set_identity: modified_comparison.conflict_set_identity,
            conflict_revision: 1,
            user_turn_source_id: base.authorization.id,
            mode: MergeResolutionMode::ChooseIncoming,
        }),
    )?;
    assert_eq!(
        choose_modified
            .get_context_item(base.project.id, base.item.id)?
            .statement,
        "base  fact"
    );
    assert!(choose_modified
        .get_tombstone(
            base.project.id,
            CanonicalRecordId::ContextItem(base.item.id)
        )
        .is_err());
    Ok(())
}

#[test]
fn competing_context_corrections_require_user_resolution() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3, 4, 5, 6])?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;

    let mut local = clone_from(&root.path().join("local.sqlite3"), &base_bundle, &[], 20)?;
    local.correct_context_item(
        operation(21),
        base.project.id,
        base.item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "base\nfact".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    let mut incoming = clone_from(&root.path().join("incoming.sqlite3"), &base_bundle, &[], 30)?;
    incoming.correct_context_item(
        operation(31),
        base.project.id,
        base.item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "base\tfact".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(base.project.id, &incoming_bundle)?;

    let comparison = local.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    assert_eq!(comparison.conflict_revision, 1);
    assert!(comparison.conflicts.iter().any(|conflict| {
        conflict.class == BundleConflictClass::SameRecordRevision
            && !conflict.automatic_resolution_allowed
            && conflict.user_judgment_reason.is_some()
    }));
    assert!(comparison.requires_user_resolution());
    assert_eq!(
        local
            .merge_bundle(
                operation(32),
                Some(&base_bundle),
                &incoming_bundle,
                None,
                None,
            )?
            .value
            .status,
        BundleMergeStatus::Unresolved
    );
    assert_eq!(
        local
            .get_context_item(base.project.id, base.item.id)?
            .statement,
        "base\nfact"
    );
    Ok(())
}

#[test]
fn decision_delete_modify_selects_one_complete_closure_and_rolls_back_atomically(
) -> Result<(), Box<dyn std::error::Error>> {
    const MODIFIED_RATIONALE: &str = "base  rationale";
    let root = tempdir()?;
    let mut origin = store(
        &root.path().join("decision-origin.sqlite3"),
        &[1, 2, 3, 4, 5, 6],
    )?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("decision-base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;

    let mut incoming = clone_from(
        &root.path().join("decision-incoming.sqlite3"),
        &base_bundle,
        &[],
        80,
    )?;
    incoming.correct_decision(
        operation(81),
        base.project.id,
        base.decision.id,
        DecisionCorrectionDraft {
            expected_revision: 1,
            corrected_user_rationale: Some(MODIFIED_RATIONALE.to_owned()),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    let incoming_bundle = root.path().join("decision-incoming.json");
    incoming.export_bundle(base.project.id, &incoming_bundle)?;

    let mut choose_forgotten = clone_from(
        &root.path().join("decision-choose-forgotten.sqlite3"),
        &base_bundle,
        &[],
        82,
    )?;
    choose_forgotten.forget_decision(
        operation(83),
        base.project.id,
        base.decision.id,
        base.authorization.id,
    )?;
    let forgotten_comparison =
        choose_forgotten.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    assert!(forgotten_comparison.conflicts.iter().any(|value| {
        value.class == BundleConflictClass::DeleteModifyConflict
            && !value.automatic_resolution_allowed
    }));
    choose_forgotten.merge_bundle(
        operation(84),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        Some(MergeResolution {
            conflict_set_identity: forgotten_comparison.conflict_set_identity,
            conflict_revision: 1,
            user_turn_source_id: base.authorization.id,
            mode: MergeResolutionMode::ChooseLocal,
        }),
    )?;
    assert!(choose_forgotten
        .get_decision(base.project.id, base.decision.id)
        .is_err());
    assert_eq!(
        choose_forgotten
            .get_tombstone(
                base.project.id,
                CanonicalRecordId::Decision(base.decision.id)
            )?
            .record,
        CanonicalRecordId::Decision(base.decision.id)
    );
    let forgotten_bundle = root.path().join("decision-forgotten.json");
    choose_forgotten.export_bundle(base.project.id, &forgotten_bundle)?;
    let forgotten_bytes = fs::read(&forgotten_bundle)?;
    assert!(!forgotten_bytes
        .windows(MODIFIED_RATIONALE.len())
        .any(|window| window == MODIFIED_RATIONALE.as_bytes()));

    let mut choose_modified = clone_from(
        &root.path().join("decision-choose-modified.sqlite3"),
        &base_bundle,
        &[],
        85,
    )?;
    choose_modified.forget_decision(
        operation(86),
        base.project.id,
        base.decision.id,
        base.authorization.id,
    )?;
    let modified_comparison =
        choose_modified.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    choose_modified.merge_bundle(
        operation(87),
        Some(&base_bundle),
        &incoming_bundle,
        None,
        Some(MergeResolution {
            conflict_set_identity: modified_comparison.conflict_set_identity,
            conflict_revision: 1,
            user_turn_source_id: base.authorization.id,
            mode: MergeResolutionMode::ChooseIncoming,
        }),
    )?;
    let selected = choose_modified.get_decision(base.project.id, base.decision.id)?;
    assert_eq!(selected.revision, 2);
    assert_eq!(selected.user_rationale.as_deref(), Some(MODIFIED_RATIONALE));
    assert!(choose_modified
        .get_tombstone(
            base.project.id,
            CanonicalRecordId::Decision(base.decision.id)
        )
        .is_err());

    let interrupted_path = root.path().join("decision-interrupted.sqlite3");
    let mut interrupted = clone_from(&interrupted_path, &base_bundle, &[], 88)?;
    interrupted.forget_decision(
        operation(89),
        base.project.id,
        base.decision.id,
        base.authorization.id,
    )?;
    let interrupted_comparison =
        interrupted.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    drop(interrupted);
    let connection = Connection::open(&interrupted_path)?;
    connection.execute_batch(
        "CREATE TRIGGER interrupt_decision_restore BEFORE INSERT ON decisions
         BEGIN SELECT RAISE(ABORT, 'interrupted Decision closure restore'); END;",
    )?;
    drop(connection);
    let mut interrupted = store(&interrupted_path, &[])?;
    let error = interrupted
        .merge_bundle(
            operation(90),
            Some(&base_bundle),
            &incoming_bundle,
            None,
            Some(MergeResolution {
                conflict_set_identity: interrupted_comparison.conflict_set_identity,
                conflict_revision: 1,
                user_turn_source_id: base.authorization.id,
                mode: MergeResolutionMode::ChooseIncoming,
            }),
        )
        .err()
        .ok_or("interrupted Decision closure merge succeeded")?;
    assert_eq!(error.kind(), ErrorKind::TransactionFailure);
    assert!(interrupted
        .get_decision(base.project.id, base.decision.id)
        .is_err());
    assert_eq!(
        interrupted
            .get_tombstone(
                base.project.id,
                CanonicalRecordId::Decision(base.decision.id)
            )?
            .record,
        CanonicalRecordId::Decision(base.decision.id)
    );
    Ok(())
}

#[test]
fn one_sided_correction_is_bounded_automatic_but_question_state_is_user_owned(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3, 4, 5, 6])?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;
    let mut local = clone_from(&root.path().join("local.sqlite3"), &base_bundle, &[], 20)?;
    let mut incoming = clone_from(&root.path().join("incoming.sqlite3"), &base_bundle, &[], 30)?;
    incoming.correct_context_item(
        operation(31),
        base.project.id,
        base.item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "base  fact".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    let corrected_bundle = root.path().join("corrected.json");
    incoming.export_bundle(base.project.id, &corrected_bundle)?;
    let correction = local.compare_bundle(Some(&base_bundle), &corrected_bundle, None)?;
    assert!(!correction.requires_user_resolution());
    assert!(correction.conflicts.iter().any(|value| {
        value.class == BundleConflictClass::SameRecordRevision && value.automatic_resolution_allowed
    }));
    local.merge_bundle(
        operation(32),
        Some(&base_bundle),
        &corrected_bundle,
        None,
        None,
    )?;
    assert_eq!(
        local
            .get_context_item(base.project.id, base.item.id)?
            .statement,
        "base  fact"
    );

    let mut question_origin = store(
        &root.path().join("question-origin.sqlite3"),
        &[71, 72, 73, 74],
    )?;
    let project = question_origin
        .create_project(operation(70), "Question divergence")?
        .value;
    let source = question_origin
        .record_source(operation(71), project.id, user_turn("question-source"))?
        .value;
    let question = question_origin
        .create_question(
            operation(72),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Resolve later?".to_owned(),
                source_basis: vec![source.id],
                dependencies: vec![],
                alternatives: vec![QuestionAlternative {
                    key: "later".to_owned(),
                    label: "Later".to_owned(),
                    consequence: "Resolve later".to_owned(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: None,
                    rationale: "No recommendation".to_owned(),
                    source_basis: vec![source.id],
                },
                trade_offs: vec![],
                uncertainty: vec!["Needs judgment".to_owned()],
                material_scope: vec!["question".to_owned()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 0,
                why_it_matters_now: "the unresolved branch remains blocked".to_owned(),
                established_facts: vec![],
                assumptions: vec![],
                known_limits: vec![],
                what_the_answer_unlocks: vec!["later resolution".to_owned()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    let question_base = root.path().join("question-base.json");
    question_origin.export_bundle(project.id, &question_base)?;
    let mut question_local = clone_from(
        &root.path().join("question-local.sqlite3"),
        &question_base,
        &[],
        73,
    )?;
    question_local.dispose_question(
        operation(74),
        project.id,
        QuestionDispositionDraft {
            expected_project_revision: 1,
            question_id: question.id,
            question_revision: 1,
            outcome: NonUserQuestionOutcome::Deferred,
            source_basis: vec![source.id],
            reason: "defer local branch".to_owned(),
            replacement_question_id: None,
            revisit_basis: vec!["after evidence refresh".to_owned()],
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "inquiry".to_owned(),
            },
        },
    )?;
    let mut question_incoming = clone_from(
        &root.path().join("question-incoming.sqlite3"),
        &question_base,
        &[],
        75,
    )?;
    question_incoming.dispose_question(
        operation(76),
        project.id,
        QuestionDispositionDraft {
            expected_project_revision: 1,
            question_id: question.id,
            question_revision: 1,
            outcome: NonUserQuestionOutcome::OutOfScope,
            source_basis: vec![source.id],
            reason: "exclude incoming branch".to_owned(),
            replacement_question_id: None,
            revisit_basis: vec![],
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "inquiry".to_owned(),
            },
        },
    )?;
    let question_bundle = root.path().join("question-incoming.json");
    question_incoming.export_bundle(project.id, &question_bundle)?;
    let question_comparison =
        question_local.compare_bundle(Some(&question_base), &question_bundle, None)?;
    assert!(question_comparison.conflicts.iter().any(|value| {
        value.class == BundleConflictClass::SameRecordRevision
            && !value.automatic_resolution_allowed
            && value
                .user_judgment_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("Question"))
    }));
    Ok(())
}

#[test]
fn merge_selected_forgetting_requires_and_recovers_managed_sanitation(
) -> Result<(), Box<dyn std::error::Error>> {
    const SECRET: &str = "base  rationale";
    let root = tempdir()?;
    let mut origin = store(&root.path().join("origin.sqlite3"), &[1, 2, 3, 4, 5, 6])?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project.id, &base_bundle)?;

    let local_database = root.path().join("local.sqlite3");
    let local_bundle = root.path().join("local-managed.json");
    let local_temporary = root.path().join(".local-managed.json.volicord-context.tmp");
    let mut local = clone_from(&local_database, &base_bundle, &[], 40)?;
    local.correct_decision(
        operation(41),
        base.project.id,
        base.decision.id,
        DecisionCorrectionDraft {
            expected_revision: 1,
            corrected_user_rationale: Some(SECRET.to_owned()),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    local.export_bundle(base.project.id, &local_bundle)?;

    let mut incoming = clone_from(&root.path().join("incoming.sqlite3"), &base_bundle, &[], 42)?;
    incoming.forget_decision(
        operation(43),
        base.project.id,
        base.decision.id,
        base.authorization.id,
    )?;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(base.project.id, &incoming_bundle)?;
    let comparison = local.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    let resolution = MergeResolution {
        conflict_set_identity: comparison.conflict_set_identity,
        conflict_revision: 1,
        user_turn_source_id: base.authorization.id,
        mode: MergeResolutionMode::ChooseIncoming,
    };

    let interrupted_database = root.path().join("interrupted-before-commit.sqlite3");
    let mut interrupted = clone_from(&interrupted_database, &base_bundle, &[], 46)?;
    interrupted.correct_decision(
        operation(47),
        base.project.id,
        base.decision.id,
        DecisionCorrectionDraft {
            expected_revision: 1,
            corrected_user_rationale: Some(SECRET.to_owned()),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: base.authorization.id,
        },
    )?;
    let interrupted_comparison =
        interrupted.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
    drop(interrupted);
    let connection = Connection::open(&interrupted_database)?;
    connection.execute_batch(
        "CREATE TRIGGER interrupt_forgetting_merge BEFORE INSERT ON tombstones
         BEGIN SELECT RAISE(ABORT, 'interrupt before canonical merge commit'); END;",
    )?;
    drop(connection);
    let mut interrupted = store(&interrupted_database, &[])?;
    let interrupted_error = interrupted
        .merge_bundle(
            operation(48),
            Some(&base_bundle),
            &incoming_bundle,
            None,
            Some(MergeResolution {
                conflict_set_identity: interrupted_comparison.conflict_set_identity,
                conflict_revision: 1,
                user_turn_source_id: base.authorization.id,
                mode: MergeResolutionMode::ChooseIncoming,
            }),
        )
        .err()
        .ok_or("interruption before merge commit succeeded")?;
    assert_eq!(interrupted_error.kind(), ErrorKind::TransactionFailure);
    assert_eq!(
        interrupted
            .get_decision(base.project.id, base.decision.id)?
            .user_rationale
            .as_deref(),
        Some(SECRET)
    );
    assert!(interrupted
        .get_tombstone(
            base.project.id,
            CanonicalRecordId::Decision(base.decision.id)
        )
        .is_err());

    fs::create_dir(&local_temporary)?;
    let error = local
        .merge_bundle(
            operation(44),
            Some(&base_bundle),
            &incoming_bundle,
            None,
            Some(resolution.clone()),
        )
        .err()
        .ok_or("post-commit sanitation obstruction reported merge success")?;
    assert_eq!(error.kind(), ErrorKind::RepairRequired);
    assert!(local
        .get_decision(base.project.id, base.decision.id)
        .is_err());
    assert_eq!(
        local
            .get_tombstone(
                base.project.id,
                CanonicalRecordId::Decision(base.decision.id)
            )?
            .record,
        CanonicalRecordId::Decision(base.decision.id)
    );
    let connection = Connection::open(&local_database)?;
    let pending: String = connection.query_row(
        "SELECT sanitation_state FROM merge_sanitation WHERE operation_id = ?1",
        [operation(44).as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    assert_eq!(pending, "pending");
    let operation_basis: Vec<u8> = connection.query_row(
        "SELECT input_basis FROM operations WHERE operation_id = ?1",
        [operation(44).as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    assert!(operation_basis.is_empty());
    let merge_bases: (String, String, String) = connection.query_row(
        "SELECT local_history_basis, incoming_history_basis, result_history_basis
         FROM merge_events WHERE operation_id = ?1",
        [operation(44).as_bytes().as_slice()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(merge_bases.0, merge_bases.2);
    assert_eq!(merge_bases.1, merge_bases.2);
    drop(connection);
    assert!(fs::read(&local_bundle)?
        .windows(SECRET.len())
        .any(|window| window == SECRET.as_bytes()));

    fs::remove_dir(&local_temporary)?;
    let replay_error = local
        .merge_bundle(
            operation(44),
            Some(&base_bundle),
            &incoming_bundle,
            None,
            Some(resolution),
        )
        .err()
        .ok_or("sanitized merge replay reported unsafe success")?;
    assert_eq!(replay_error.kind(), ErrorKind::NotFound);
    let changed_error = local
        .merge_bundle(
            operation(44),
            Some(&base_bundle),
            &incoming_bundle,
            None,
            None,
        )
        .err()
        .ok_or("changed sanitized merge replay input succeeded")?;
    assert_eq!(changed_error.kind(), ErrorKind::NotFound);
    drop(local);

    let complete: String = Connection::open(&local_database)?.query_row(
        "SELECT sanitation_state FROM merge_sanitation WHERE operation_id = ?1",
        [operation(44).as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    assert_eq!(complete, "complete");
    for path in [
        local_database.clone(),
        root.path().join("local.sqlite3-wal"),
        root.path().join("local.sqlite3-shm"),
        local_bundle.clone(),
        local_temporary.clone(),
    ] {
        if path.is_file() {
            let bytes = fs::read(&path)?;
            assert!(
                !bytes
                    .windows(SECRET.len())
                    .any(|window| window == SECRET.as_bytes()),
                "merge-selected forgotten content remains in {}",
                path.display()
            );
        }
    }

    let mut imported = store(&root.path().join("imported.sqlite3"), &[])?;
    imported.import_bundle(operation(45), &local_bundle)?;
    assert!(imported
        .get_decision(base.project.id, base.decision.id)
        .is_err());
    let imported_bundle = root.path().join("imported.json");
    imported.export_bundle(base.project.id, &imported_bundle)?;
    assert_eq!(fs::read(&local_bundle)?, fs::read(&imported_bundle)?);
    Ok(())
}
