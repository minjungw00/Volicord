use std::error::Error;

use volicord_types::ids::{
    AgentConnectionId, IdempotencyKey, ProjectId, RequestHash, RunId, TaskId,
};
use volicord_types::schema::{
    EvidenceProducerAnchor, EvidenceRelevanceAssessment, JsonObject, ObservedChanges,
    PersistedEvidenceObservationAuthority, SourceRef, UserContextSource, WorkflowActionKey,
};
use volicord_types::values::{
    ActorSource, EvidenceAssuranceLevel, EvidenceProducerKind, EvidenceRelevanceStatus,
    EvidenceSourceKind, MethodName, RunKind, UtcTimestamp, WorkflowActionSemanticVariant,
    WorkflowExpectedResultState, WorkflowTransitionEffectClass,
};

use crate::core_pipeline::test_support::{
    pending_event, pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, CoreStorageMutation, EvidenceClaimInsert, EvidenceMutation,
    EvidenceObservationInsert, MutationCommitOutcome, RunInsert, RunMutation, RunStatus,
    StoredRunMetadata, StoredRunSummary, StoredRunWriteTicketEffect,
    StoredRunWriteTicketEffectKind, TaskMutation, TransitionCommitExpectation,
};

#[test]
fn transition_effect_mismatch_rejects_before_any_aggregate_mutation() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let before = store.effect_counts()?;
    let task_id = "task_transition_effect_mismatch";
    let mut input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_transition_effect_mismatch")),
        &RequestHash::new("sha256:transition-effect-mismatch"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task(
            "transition_effect_mismatch",
            task_id,
        )],
    );
    input.transition_expectation = Some(TransitionCommitExpectation {
        project_id: PROJECT_ID.to_owned(),
        task_id: task_id.to_owned(),
        action_key: WorkflowActionKey::new(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
        )?,
        effect_class: WorkflowTransitionEffectClass::CoreStateMutation,
        expected_result_state: WorkflowExpectedResultState::ReevaluateCurrentAuthority,
        basis_state_version: 0,
    });
    let mutations = [CoreStorageMutation::Task(TaskMutation::insert(
        task_insert(task_id),
    ))];

    let error = store
        .commit_mutation(input, &mutations, response_json)
        .expect_err("CreateCurrentChangeUnit must include its Change Unit aggregate effect");
    assert!(error
        .to_string()
        .contains("contradict the admitted transition effect"));
    assert_eq!(store.effect_counts()?, before);
    assert!(store.task_record(&TaskId::new(task_id))?.is_none());
    Ok(())
}

#[test]
fn incompatible_transition_result_rolls_back_every_commit_effect() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let before = store.effect_counts()?;
    let task_id = "task_transition_result_rollback";
    let mut input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::CloseTask,
        Some(&IdempotencyKey::new("idem_transition_result_rollback")),
        &RequestHash::new("sha256:transition-result-rollback"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task(
            "transition_result_rollback",
            task_id,
        )],
    );
    input.transition_expectation = Some(TransitionCommitExpectation {
        project_id: PROJECT_ID.to_owned(),
        task_id: task_id.to_owned(),
        action_key: WorkflowActionKey::new(
            MethodName::CloseTask,
            WorkflowActionSemanticVariant::CloseTask,
        )?,
        effect_class: WorkflowTransitionEffectClass::TerminalMutation,
        expected_result_state: WorkflowExpectedResultState::Terminal,
        basis_state_version: 0,
    });
    let mutations = [CoreStorageMutation::Task(TaskMutation::insert(
        task_insert(task_id),
    ))];

    store
        .commit_mutation(input, &mutations, response_json)
        .expect_err("nonterminal post-state must fail the terminal transition contract");
    assert_eq!(store.effect_counts()?, before);
    assert!(store.task_record(&TaskId::new(task_id))?.is_none());
    Ok(())
}

#[test]
fn ordered_multi_aggregate_commit_is_versioned_replayable_and_durable() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_evidence_observation";
    let run_id = "run_evidence_observation";
    let observation_id = "evidence_observation_store";
    let idempotency_key = IdempotencyKey::new("idem_store_evidence_observation");

    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordRun,
        Some(&idempotency_key),
        &RequestHash::new("sha256:evidence-observation"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("evidence_observation", task_id)],
    );
    let mutations = [
        CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
        CoreStorageMutation::Run(RunMutation::Insert(run_insert(run_id, task_id))),
        CoreStorageMutation::Evidence(EvidenceMutation::EnsureClaim(EvidenceClaimInsert {
            task_id: task_id.to_owned(),
            evidence_claim_id: "claim_search_result_count".to_owned(),
            statement: "Search result count was verified.".to_owned(),
        })),
        CoreStorageMutation::Evidence(EvidenceMutation::InsertObservation(
            EvidenceObservationInsert {
                evidence_observation_id: observation_id.to_owned(),
                task_id: task_id.to_owned(),
                change_unit_id: None,
                run_id: Some(run_id.to_owned()),
                acceptance_criterion_id: None,
                evidence_claim_id: Some("claim_search_result_count".to_owned()),
                source_kind: EvidenceSourceKind::ExternalTool,
                assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
                observed_by_actor_source: Some(ActorSource::AgentConnection(
                    AgentConnectionId::new(CONNECTION_ID),
                )),
                tool_name: Some("local-test-runner".to_owned()),
                tool_invocation_id: Some("tool_invocation_001".to_owned()),
                tool_metadata: JsonObject::from_iter([("exit_code".to_owned(), 0.into())]),
                input_refs: Vec::new(),
                source_refs: vec![SourceRef::UserContext(UserContextSource {
                    context_id: "message_store_evidence".to_owned(),
                })],
                output_artifact_refs: Vec::new(),
                limitations: vec!["External tool result is not a proof.".to_owned()],
                observed_at: UtcTimestamp::parse("2026-06-18T00:00:00Z")
                    .expect("test timestamp must parse"),
                recorded_at: UtcTimestamp::parse("2026-06-18T00:00:01Z")
                    .expect("test timestamp must parse"),
                metadata: PersistedEvidenceObservationAuthority {
                    recorded_by_run_id: RunId::new(run_id),
                    invocation_verification_basis: "store_test_boundary".to_owned(),
                    producer_anchor: EvidenceProducerAnchor {
                        producer_kind: EvidenceProducerKind::UnverifiedCaller,
                        producer_ref: None.into(),
                        output_artifact_refs: Vec::new(),
                        verification_basis: None.into(),
                    },
                    relevance_assessment: EvidenceRelevanceAssessment {
                        status: EvidenceRelevanceStatus::Unassessed,
                        assessment_ref: None.into(),
                        assessed_by_actor_source: None.into(),
                    },
                },
            },
        )),
    ];
    let committed = store.commit_mutation(input.clone(), &mutations, response_json)?;
    let MutationCommitOutcome::Committed {
        response_json: committed_response,
        basis_state_version,
        committed_state_version,
        events,
    } = committed
    else {
        panic!("ordered aggregate batch must commit");
    };
    assert_eq!(basis_state_version, 0);
    assert_eq!(committed_state_version, 1);
    assert_eq!(events.len(), 1);

    let record = store
        .evidence_observation_record(observation_id)?
        .expect("evidence observation should be readable");
    assert_eq!(record.run_id.as_deref(), Some(run_id));
    assert_eq!(record.source_kind, EvidenceSourceKind::ExternalTool);
    assert_eq!(
        record.assurance_level,
        EvidenceAssuranceLevel::ExternalToolResult
    );
    assert_eq!(
        record.source_refs,
        vec![SourceRef::UserContext(UserContextSource {
            context_id: "message_store_evidence".to_owned(),
        })]
    );
    assert_eq!(
        record.limitations,
        vec!["External tool result is not a proof."]
    );
    let committed_counts = store.effect_counts()?;
    assert_eq!(committed_counts.state_version, 1);
    assert_eq!(committed_counts.tasks, 1);
    assert_eq!(committed_counts.runs, 1);
    assert_eq!(committed_counts.evidence_claims, 1);
    assert_eq!(committed_counts.evidence_observations, 1);
    assert_eq!(committed_counts.authority_events, 1);
    assert_eq!(committed_counts.tool_invocations, 1);

    let replay = store.commit_mutation(input, &mutations, |_| {
        panic!("eligible replay must reuse the stored response")
    })?;
    assert!(matches!(
        replay,
        MutationCommitOutcome::Replayed {
            response_json,
            basis_state_version: 0,
            committed_state_version: 1,
        } if response_json == committed_response
    ));
    assert_eq!(store.effect_counts()?, committed_counts);

    drop(store);
    let reopened = harness.store()?;
    assert_eq!(reopened.project_state()?.state_version, 1);
    assert!(reopened.task_record(&TaskId::new(task_id))?.is_some());
    assert!(reopened.run_record(run_id)?.is_some());
    assert!(reopened
        .evidence_claim_record(&TaskId::new(task_id), "claim_search_result_count")?
        .is_some());
    assert!(reopened
        .evidence_observation_record(observation_id)?
        .is_some());
    assert_eq!(
        reopened
            .tool_invocation(MethodName::RecordRun, &idempotency_key)?
            .expect("replay row must survive reopen")
            .response_json,
        committed_response
    );
    assert_eq!(reopened.effect_counts()?, committed_counts);
    Ok(())
}

#[test]
fn intermediate_aggregate_failure_rolls_back_every_commit_effect() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let before = store.effect_counts()?;
    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordRun,
        Some(&IdempotencyKey::new("idem_store_foreign_key")),
        &RequestHash::new("sha256:foreign-key"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event("foreign_key")],
    );
    let mutations = [
        CoreStorageMutation::Task(TaskMutation::insert(task_insert("task_before_failure"))),
        CoreStorageMutation::Run(RunMutation::Insert(run_insert_with_missing_task())),
    ];

    let error = store
        .commit_mutation(input, &mutations, response_json)
        .expect_err("missing run task should fail a foreign-key constraint");
    let classification = error.classification();

    assert_eq!(classification.category, "constraint_foreign_key");
    assert!(matches!(
        classification.route,
        crate::StoreFailureRoute::OperationalUnavailable
    ));
    assert_eq!(store.effect_counts()?, before);
    assert!(store
        .task_record(&TaskId::new("task_before_failure"))?
        .is_none());
    assert!(store
        .tool_invocation(
            MethodName::RecordRun,
            &IdempotencyKey::new("idem_store_foreign_key")
        )?
        .is_none());
    Ok(())
}
fn run_insert_with_missing_task() -> RunInsert {
    RunInsert {
        run_id: "run_missing_task".to_owned(),
        task_id: "missing_task".to_owned(),
        change_unit_id: None,
        scope_revision: 0,
        write_ticket_id: None,
        kind: RunKind::Implementation,
        status: RunStatus::Recorded,
        summary: StoredRunSummary {
            summary: String::new(),
        },
        observed_changes: empty_observed_changes(),
        evidence_updates: Vec::new(),
        write_ticket_effect: StoredRunWriteTicketEffect {
            write_ticket_id: None,
            effect: StoredRunWriteTicketEffectKind::None,
        },
        created_by_actor_source: ActorSource::AgentConnection(AgentConnectionId::new(
            CONNECTION_ID,
        )),
        metadata: StoredRunMetadata {
            verification_basis: "store_test_boundary".to_owned(),
        },
    }
}

fn run_insert(run_id: &str, task_id: &str) -> RunInsert {
    RunInsert {
        run_id: run_id.to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: None,
        scope_revision: 0,
        write_ticket_id: None,
        kind: RunKind::Implementation,
        status: RunStatus::Recorded,
        summary: StoredRunSummary {
            summary: String::new(),
        },
        observed_changes: empty_observed_changes(),
        evidence_updates: Vec::new(),
        write_ticket_effect: StoredRunWriteTicketEffect {
            write_ticket_id: None,
            effect: StoredRunWriteTicketEffectKind::None,
        },
        created_by_actor_source: ActorSource::AgentConnection(AgentConnectionId::new(
            CONNECTION_ID,
        )),
        metadata: StoredRunMetadata {
            verification_basis: "store_test_boundary".to_owned(),
        },
    }
}

fn empty_observed_changes() -> ObservedChanges {
    ObservedChanges {
        changed_paths: Vec::new(),
        product_file_write_observed: false,
        sensitive_categories: Vec::new(),
        baseline_ref: None.into(),
    }
}
