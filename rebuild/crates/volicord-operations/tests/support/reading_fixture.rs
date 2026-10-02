//! Independent synthetic counterexamples through the actual Local Operations API.
use serde_json::Value;
use std::{collections::BTreeMap, fs};
use tempfile::{tempdir, TempDir};
use volicord_context::*;
use volicord_operations::{LocalOperations, RuntimeLayout};

pub const SCENARIO: &str = include_str!(
    "../../../../validation/end-to-end/multi-repository/fixtures/viewer-reading/scenario.json"
);

pub struct Fixture {
    pub _temporary: TempDir,
    pub operations: LocalOperations,
    pub repository: std::path::PathBuf,
    pub project: ProjectId,
    pub goals: BTreeMap<String, ContextItemId>,
    pub checkpoints: BTreeMap<String, CheckpointId>,
    pub decisions: BTreeMap<String, DecisionId>,
    pub purpose: ContextItemId,
}

fn op(counter: &mut u128) -> OperationId {
    *counter += 1;
    OperationId::from_bytes(counter.to_le_bytes())
}
fn actor(kind: PrincipalKind) -> Principal {
    Principal {
        kind,
        identity: "synthetic-fixture".into(),
    }
}
fn work_state(value: &str) -> WorkState {
    match value {
        "Completed" => WorkState::Completed,
        "Paused" => WorkState::Paused,
        "InProgress" => WorkState::InProgress,
        _ => panic!("invalid fixture work state"),
    }
}
fn verification_state(value: &str) -> VerificationState {
    match value {
        "Passed" => VerificationState::Passed,
        "Failed" => VerificationState::Failed,
        "NotRun" => VerificationState::NotRun,
        _ => panic!("invalid fixture verification state"),
    }
}
pub fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let input: Value = serde_json::from_str(SCENARIO)?;
    let temporary = tempdir()?;
    let repository = temporary.path().join("repository");
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../validation/end-to-end/multi-repository/fixtures/viewer-reading/repository");
    for path in [
        "native/query.c",
        "runtime/query_boundary.ts",
        "python/worker.py",
    ] {
        let target = repository.join(path);
        fs::create_dir_all(target.parent().ok_or("parent")?)?;
        fs::copy(fixture_root.join(path), target)?;
    }
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    let project = operations
        .initialize_project("Reading fixture", Some(&repository))?
        .project;
    operations.analyze(project.id, Vec::new())?;
    let mut store = Store::open(operations.layout().canonical_store())?;
    let mut counter = 50_000;
    let user = store
        .record_source(
            op(&mut counter),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "test".into(),
                    session: "synthetic".into(),
                    turn: "Self-authored Work reading fixture".into(),
                },
                actor: actor(PrincipalKind::User),
                observer: Some(actor(PrincipalKind::Agent)),
                availability: Availability::Available,
            },
        )?
        .value
        .id;
    let commit = store
        .record_source(
            op(&mut counter),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositoryCommit {
                    commit: input["shared_commit"].as_str().ok_or("commit")?.into(),
                },
                actor: actor(PrincipalKind::Repository),
                observer: Some(actor(PrincipalKind::Agent)),
                availability: Availability::Available,
            },
        )?
        .value
        .id;
    let mut commands = BTreeMap::new();
    for (key, exit) in [("Passed", 0), ("Failed", 1)] {
        let source = store
            .record_source(
                op(&mut counter),
                project.id,
                SourceDraft {
                    expected_project_revision: project.revision,
                    payload: SourcePayload::CommandExecution {
                        command_label: format!("synthetic {key}"),
                        invocation_fingerprint: format!(
                            "sha256:{}",
                            if exit == 0 {
                                "1".repeat(64)
                            } else {
                                "2".repeat(64)
                            }
                        ),
                        outcome: CommandOutcome {
                            exit_code: Some(exit),
                            termination: CommandTermination::Exited,
                        },
                    },
                    actor: actor(PrincipalKind::Command),
                    observer: Some(actor(PrincipalKind::Agent)),
                    availability: Availability::Available,
                },
            )?
            .value
            .id;
        commands.insert(key, source);
    }
    let record_goal =
        |store: &mut Store, counter: &mut u128, role, text: &str, paths: Vec<String>| {
            store
                .record_context_item(
                    op(counter),
                    project.id,
                    ContextItemDraft {
                        expected_project_revision: project.revision,
                        role,
                        statement: text.into(),
                        provenance_role: StatementProvenanceRole::UserStatement,
                        author: actor(PrincipalKind::User),
                        source_basis: vec![user],
                        applicability: ApplicabilityScope {
                            paths,
                            components: Vec::new(),
                            work_contexts: Vec::new(),
                        },
                    },
                )
                .map(|outcome| outcome.value.id)
        };
    let purpose = record_goal(
        &mut store,
        &mut counter,
        ContextItemRole::ProjectPurpose,
        input["purpose_present"].as_str().ok_or("Purpose")?,
        Vec::new(),
    )?;
    let distractor = record_goal(
        &mut store,
        &mut counter,
        ContextItemRole::Goal,
        "Other history",
        vec!["unrelated.rs".into()],
    )?;
    let mut goals = BTreeMap::new();
    for work in input["works"].as_array().ok_or("works")? {
        let id = record_goal(
            &mut store,
            &mut counter,
            ContextItemRole::Goal,
            work["title"].as_str().ok_or("title")?,
            Vec::new(),
        )?;
        goals.insert(work["key"].as_str().ok_or("work key")?.to_owned(), id);
    }
    let mut decisions = BTreeMap::new();
    for decision in input["decisions"].as_array().ok_or("decisions")? {
        let scope = match decision["scope"].as_str().ok_or("scope")? {
            "ProjectWide" => DecisionWorkScope::ProjectWide,
            "Unresolved" => DecisionWorkScope::Unresolved,
            key => DecisionWorkScope::WorkItem(goals[key]),
        };
        let question = store
            .create_question(
                op(&mut counter),
                project.id,
                QuestionDraft {
                    expected_project_revision: project.revision,
                    prompt_basis: format!("Which boundary? {}", decision["key"]),
                    source_basis: vec![commit],
                    dependencies: Vec::new(),
                    alternatives: vec![
                        QuestionAlternative {
                            key: "local".into(),
                            label: "Local".into(),
                            consequence: "Bounded local code".into(),
                        },
                        QuestionAlternative {
                            key: "remote".into(),
                            label: "Remote".into(),
                            consequence: "Remote boundary".into(),
                        },
                    ],
                    recommendation: AgentRecommendation {
                        alternative_key: Some("remote".into()),
                        rationale: decision["recommendation_rationale"]
                            .as_str()
                            .unwrap_or("Agent rationale")
                            .into(),
                        source_basis: vec![commit],
                    },
                    trade_offs: Vec::new(),
                    uncertainty: Vec::new(),
                    material_scope: vec!["native/query.c".into()],
                    materiality: QuestionMateriality::Material,
                    presentation_order: 1,
                    why_it_matters_now: "Select a bounded boundary".into(),
                    established_facts: Vec::new(),
                    assumptions: Vec::new(),
                    known_limits: Vec::new(),
                    what_the_answer_unlocks: vec!["Implement the boundary".into()],
                    allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                    research_state: QuestionResearchState::ReadyToAsk,
                },
            )?
            .value;
        let recorded = store
            .record_question_response(
                op(&mut counter),
                project.id,
                QuestionResponseDraft {
                    expected_project_revision: project.revision,
                    question_id: question.id,
                    question_revision: question.revision,
                    user_turn_source: UserTurnSource::Existing(user),
                    displayed_alternative_keys: vec!["local".into(), "remote".into()],
                    displayed_recommendation_key: Some("remote".into()),
                    response: ExplicitQuestionResponse::Choice {
                        alternative_key: "local".into(),
                        user_rationale: decision["user_rationale"].as_str().map(str::to_owned),
                    },
                    work_scope: scope,
                    applicability: ApplicabilityScope {
                        paths: vec!["native/query.c".into()],
                        components: Vec::new(),
                        work_contexts: Vec::new(),
                    },
                    assumptions: Vec::new(),
                    revisit_triggers: Vec::new(),
                },
            )?
            .value
            .decision
            .ok_or("Decision")?;
        if decision["state"] == "ReviewDue" {
            store.mark_decision_review_due(
                op(&mut counter),
                project.id,
                recorded.id,
                ReviewDueDraft {
                    kind: ReviewDueKind::RevisitTriggerMet,
                    explanation: "Fixture review trigger".into(),
                    source_basis: vec![commit],
                },
            )?;
        }
        if decision["state"] == "Superseded" {
            store.supersede_decision(
                op(&mut counter),
                project.id,
                DecisionSupersessionDraft {
                    expected_project_revision: project.revision,
                    previous_decision_id: recorded.id,
                    user_turn_source: UserTurnSource::Existing(user),
                    choice: DecisionChoice::Alternative {
                        alternative_key: "remote".into(),
                    },
                    user_rationale: None,
                    applicability: recorded.applicability.clone(),
                    assumptions: Vec::new(),
                    revisit_triggers: Vec::new(),
                },
            )?;
        }
        decisions.insert(
            decision["key"].as_str().ok_or("decision key")?.to_owned(),
            recorded.id,
        );
    }
    let record = |store: &mut Store,
                  counter: &mut u128,
                  work,
                  title: &str,
                  cp: &Value,
                  paths: Vec<String>,
                  applied|
     -> Result<CheckpointId, volicord_context::Error> {
        let verification = cp["verification"].as_str().unwrap_or("NotRun");
        let review = match cp["review"].as_str().unwrap_or("NotRequested") {
            "Pending" => UserReviewState::Pending,
            "Reviewed" => UserReviewState::Reviewed,
            _ => UserReviewState::NotRequested,
        };
        let acceptance = match cp["acceptance"].as_str().unwrap_or("NotRequested") {
            "Pending" => UserAcceptanceState::Pending,
            "Rejected" => UserAcceptanceState::Rejected,
            "Accepted" => UserAcceptanceState::Accepted,
            _ => UserAcceptanceState::NotRequested,
        };
        store
            .record_checkpoint(
                op(counter),
                project.id,
                CheckpointDraft {
                    expected_project_revision: project.revision,
                    work_item_id: work,
                    kind: CheckpointKind::Handoff,
                    goal: title.into(),
                    work_state: work_state(cp["work"].as_str().unwrap_or("Paused")),
                    state_change: cp["state_change"].as_str().map(str::to_owned),
                    source_basis: vec![commit, user],
                    changed_source_basis: if paths.is_empty() {
                        Vec::new()
                    } else {
                        vec![commit]
                    },
                    changed_paths: paths,
                    applied_decisions: applied,
                    verification: vec![VerificationFact {
                        state: verification_state(verification),
                        source_id: commands.get(verification).copied(),
                        outcome: commands
                            .contains_key(verification)
                            .then(|| "synthetic observed outcome".into()),
                    }],
                    user_review: UserReviewFact {
                        state: review,
                        source_id: (review == UserReviewState::Reviewed).then_some(user),
                    },
                    user_acceptance: UserAcceptanceFact {
                        state: acceptance,
                        source_id: matches!(
                            acceptance,
                            UserAcceptanceState::Accepted | UserAcceptanceState::Rejected
                        )
                        .then_some(user),
                    },
                    known_limits: vec!["Fixture only, not human evidence".into()],
                    non_goals: Vec::new(),
                    open_questions: Vec::new(),
                    next_step: cp["next_step"]
                        .as_str()
                        .unwrap_or("Continue other history")
                        .into(),
                    handoff_to: Some("next-agent".into()),
                },
            )
            .map(|outcome| outcome.value.id)
    };
    let empty = serde_json::json!({"state_change":"Other history","work":"Paused"});
    for _ in 0..input["prior_checkpoint_count"]
        .as_u64()
        .ok_or("prior count")?
    {
        record(
            &mut store,
            &mut counter,
            Some(distractor),
            "Other history",
            &empty,
            vec!["unrelated.rs".into()],
            Vec::new(),
        )?;
    }
    let mut checkpoints = BTreeMap::new();
    for work in input["works"].as_array().ok_or("works")? {
        let key = work["key"].as_str().ok_or("key")?;
        for cp in work["checkpoints"].as_array().ok_or("checkpoints")? {
            let paths = cp
                .get("paths")
                .unwrap_or(&work["paths"])
                .as_array()
                .ok_or("paths")?
                .iter()
                .map(|p| p.as_str().expect("fixture path").to_owned())
                .collect();
            let applied = if key == "older" {
                vec![
                    decisions["explicit"],
                    decisions["project"],
                    decisions["unresolved"],
                ]
            } else {
                vec![decisions["other_work"]]
            };
            let id = record(
                &mut store,
                &mut counter,
                Some(goals[key]),
                work["title"].as_str().ok_or("title")?,
                cp,
                paths,
                applied,
            )?;
            checkpoints.insert(cp["key"].as_str().ok_or("checkpoint key")?.to_owned(), id);
        }
    }
    for _ in 0..input["later_checkpoint_count"]
        .as_u64()
        .ok_or("later count")?
    {
        record(
            &mut store,
            &mut counter,
            Some(distractor),
            "Other history",
            &empty,
            vec!["unrelated.rs".into()],
            Vec::new(),
        )?;
    }
    let unassociated = record(
        &mut store,
        &mut counter,
        None,
        "Unassociated",
        &empty,
        Vec::new(),
        Vec::new(),
    )?;
    checkpoints.insert("unassociated".into(), unassociated);
    drop(store);
    Ok(Fixture {
        _temporary: temporary,
        operations,
        repository,
        project: project.id,
        goals,
        checkpoints,
        decisions,
        purpose,
    })
}
