use serde_json::Value;
use std::{error::Error as StdError, fs, path::Path};
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, BundleConflictClass, DecisionChoice,
    DecisionSupersessionDraft, ExplicitQuestionResponse, MergeResolution, MergeResolutionMode,
    OperationId, Principal, PrincipalKind, QuestionAlternative, QuestionDraft,
    QuestionResponseDraft, SourceDraft, SourcePayload, Store, UserTurnSource,
};
use volicord_operations::{run_cli, CliExit, LocalOperations, RuntimeLayout};

#[test]
fn public_portable_flow_preserves_conflict_revision_source_and_all_resolution_modes(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let origin_path = root.path().join("origin.sqlite3");
    let mut origin = Store::open(&origin_path)?;
    let base = create_base(&mut origin)?;
    let base_bundle = root.path().join("base.json");
    origin.export_bundle(base.project_id, &base_bundle)?;

    let incoming_path = root.path().join("incoming.sqlite3");
    let mut incoming = Store::open(&incoming_path)?;
    incoming.import_bundle(operation(20), &base_bundle)?;
    supersede(&mut incoming, &base, "b", operation(21))?;
    let incoming_bundle = root.path().join("incoming.json");
    incoming.export_bundle(base.project_id, &incoming_bundle)?;

    let stale_runtime = root.path().join("runtime-stale");
    prepare_local(&stale_runtime, &base_bundle, &base)?;
    let stale_operations = LocalOperations::new(RuntimeLayout::new(&stale_runtime)?);
    let comparison =
        stale_operations.compare_portable_bundle(Some(&base_bundle), &incoming_bundle)?;
    assert_eq!(comparison.conflict_revision, 1);
    assert!(comparison.requires_user_resolution());
    assert!(comparison.conflicts.iter().any(|conflict| {
        conflict.class == BundleConflictClass::SemanticDecisionConflict
            && !conflict.automatic_resolution_allowed
            && !conflict.affected_identities.is_empty()
            && !conflict.consequence.is_empty()
            && !conflict.uncertainty.is_empty()
    }));
    let before = stale_operations.canonical_basis(base.project_id)?;
    let stale_error = stale_operations
        .merge_portable_bundle(
            Some(&base_bundle),
            &incoming_bundle,
            Some(MergeResolution {
                conflict_set_identity: "0".repeat(64),
                conflict_revision: comparison.conflict_revision,
                user_turn_source_id: base.authorization,
                mode: MergeResolutionMode::ChooseIncoming,
            }),
        )
        .expect_err("mismatched conflict set must fail closed");
    assert!(stale_error.source().is_some_and(|source| source
        .to_string()
        .contains("exact current conflict set and revision")));
    let after = stale_operations.canonical_basis(base.project_id)?;
    assert_eq!(before, after);

    for (index, mode, expected_status, expected_choice) in [
        (0, "choose-local", "resolved", "a"),
        (1, "choose-incoming", "resolved", "b"),
        (2, "context-branch", "branched", "a"),
        (3, "explicit-merged", "resolved", "b"),
    ] {
        let runtime = root.path().join(format!("runtime-{index}"));
        prepare_local(&runtime, &base_bundle, &base)?;
        let compared = cli(
            &runtime,
            &[
                "context",
                "compare",
                "--input",
                incoming_bundle.to_str().ok_or("incoming path")?,
                "--base",
                base_bundle.to_str().ok_or("base path")?,
            ],
        )?;
        assert_eq!(compared["operation"], "portable_compare");
        assert_eq!(compared["requires_user_resolution"], true);
        assert_eq!(
            compared["conflicts"][0]["base_basis"],
            compared["common_base"]["history_basis"]
        );
        assert!(compared["conflicts"]
            .as_array()
            .is_some_and(|conflicts| conflicts.iter().any(|conflict| {
                conflict["class"] == "semantic_decision_conflict"
                    && conflict["automatic_resolution_allowed"] == false
                    && conflict["affected_identities"]
                        .as_array()
                        .is_some_and(|identities| !identities.is_empty())
            })));

        let revision = compared["conflict_revision"]
            .as_u64()
            .ok_or("conflict revision")?
            .to_string();
        let conflict_set = compared["conflict_set_identity"]
            .as_str()
            .ok_or("conflict-set identity")?;
        let mut args = vec![
            "context",
            "resolve",
            "--input",
            incoming_bundle.to_str().ok_or("incoming path")?,
            "--conflict-set",
            conflict_set,
            "--revision",
            &revision,
            "--source",
            &base.authorization_text,
            "--mode",
            mode,
            "--base",
            base_bundle.to_str().ok_or("base path")?,
        ];
        if mode == "explicit-merged" {
            args.extend([
                "--merged-bundle",
                incoming_bundle.to_str().ok_or("incoming path")?,
            ]);
        }
        let resolved = cli(&runtime, &args)?;
        assert_eq!(resolved["operation"], "portable_resolve");
        assert_eq!(resolved["status"], expected_status);
        assert_eq!(resolved["conflict_set_identity"], conflict_set);
        assert_eq!(resolved["conflict_revision"], compared["conflict_revision"]);
        assert_eq!(resolved["resolution_source_id"], base.authorization_text);
        if mode == "context-branch" {
            assert_eq!(resolved["status"], "branched");
            assert_eq!(
                resolved["branch_history_basis"],
                compared["incoming"]["history_basis"]
            );
        }

        let store = Store::open(runtime.join("canonical.sqlite3"))?;
        let current = store.get_current_decision(base.project_id, base.question_id)?;
        assert_eq!(
            current.decision.choice,
            DecisionChoice::Alternative {
                alternative_key: expected_choice.into(),
            }
        );
    }
    Ok(())
}

fn cli(runtime: &Path, args: &[&str]) -> Result<Value, Box<dyn std::error::Error>> {
    let mut command = vec![
        "--runtime",
        runtime.to_str().ok_or("runtime path")?,
        "--json",
    ];
    command.extend_from_slice(args);
    let mut output = Vec::new();
    let mut error = Vec::new();
    let exit = run_cli(command, &mut output, &mut error);
    if exit != CliExit::SUCCESS {
        return Err(format!(
            "portable CLI failed with {}: {}",
            exit.code(),
            String::from_utf8_lossy(&error)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output)?)
}

struct Base {
    project_id: volicord_context::ProjectId,
    authorization: volicord_context::SourceId,
    authorization_text: String,
    question_id: volicord_context::QuestionId,
    decision_id: volicord_context::DecisionId,
}

fn create_base(store: &mut Store) -> Result<Base, Box<dyn std::error::Error>> {
    let project = store.create_project(operation(1), "Portable CLI")?.value;
    let repository = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "portable-base".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "fixture".into(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".into(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let authorization = store
        .record_source(
            operation(3),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".into(),
                    session: "portable-test".into(),
                    turn: "resolve this exact conflict".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-host-user".into(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".into(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let question = store
        .create_question(
            operation(4),
            project.id,
            QuestionDraft {
                expected_project_revision: project.revision,
                prompt_basis: "Which portable side is current?".into(),
                source_basis: vec![repository.id],
                dependencies: Vec::new(),
                alternatives: vec![
                    QuestionAlternative {
                        key: "a".into(),
                        label: "A".into(),
                        consequence: "retain local meaning".into(),
                    },
                    QuestionAlternative {
                        key: "b".into(),
                        label: "B".into(),
                        consequence: "retain incoming meaning".into(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("a".into()),
                    rationale: "base recommendation".into(),
                    source_basis: vec![repository.id],
                },
                trade_offs: Vec::new(),
                uncertainty: Vec::new(),
                material_scope: vec!["portable".into()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 1,
                why_it_matters_now: "the clones diverged".into(),
                established_facts: Vec::new(),
                assumptions: Vec::new(),
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["continued work".into()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    let response = store
        .record_question_response(
            operation(5),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: project.revision,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Existing(authorization.id),
                displayed_alternative_keys: vec!["a".into(), "b".into()],
                displayed_recommendation_key: Some("a".into()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "a".into(),
                    user_rationale: Some("base choice".into()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        )?
        .value;
    let decision = response.decision.ok_or("base Decision missing")?;
    Ok(Base {
        project_id: project.id,
        authorization: authorization.id,
        authorization_text: authorization.id.to_string(),
        question_id: question.id,
        decision_id: decision.id,
    })
}

fn prepare_local(
    runtime: &Path,
    base_bundle: &Path,
    base: &Base,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(runtime)?;
    let mut local = Store::open(runtime.join("canonical.sqlite3"))?;
    local.import_bundle(operation(30), base_bundle)?;
    supersede(&mut local, base, "a", operation(31))?;
    Ok(())
}

fn supersede(
    store: &mut Store,
    base: &Base,
    alternative: &str,
    operation_id: OperationId,
) -> Result<(), Box<dyn std::error::Error>> {
    store.supersede_decision(
        operation_id,
        base.project_id,
        DecisionSupersessionDraft {
            expected_project_revision: 1,
            previous_decision_id: base.decision_id,
            user_turn_source: UserTurnSource::Existing(base.authorization),
            choice: DecisionChoice::Alternative {
                alternative_key: alternative.into(),
            },
            user_rationale: Some(format!("{alternative} branch rationale")),
            applicability: ApplicabilityScope::default(),
            assumptions: Vec::new(),
            revisit_triggers: Vec::new(),
        },
    )?;
    Ok(())
}

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}
