use std::{collections::BTreeSet, error::Error};

use serde_json::Value;
use volicord_types::{
    ids::{BaselineRef, ChangeUnitId, ShapingCheckpointId, TaskId},
    methods::{AdvanceTaskRequest, FinalizeAdviceRequest, RecordShapingCheckpointRequest},
    schema::{
        RequiredNullable, ShapingCheckpointOperation, ShapingGapInput, ShapingUserActionDraft,
        StateRecordRef, WorkflowActionKey, WorkflowActionRole, WorkflowProjection,
    },
    values::{
        AuthorityNextActor, ChangeUnitOperation, JudgmentKind, OperationCategory, RequestedMode,
        ShapingGapKind, WorkflowActionSemanticVariant, WorkflowStateKind, WorkflowTransitionActor,
    },
};

use super::{
    advisor_update_scope_request, close_task_request, envelope, intake_request, invocation,
    record_close_evidence, record_final_acceptance, record_run_request,
    resolve_user_action_request, response_record_id, shaping_progression::record_user_owned_gap,
    shaping_progression::shaping_task, status_include, update_scope_request, user_action_status,
    CloseTaskFixture, MethodHarness,
};

#[derive(Default)]
struct ReachabilityCoverage {
    modes: BTreeSet<String>,
    work_phases: BTreeSet<String>,
    lifecycle_phases: BTreeSet<String>,
    workflow_states: BTreeSet<String>,
    checkpoint_readiness: BTreeSet<String>,
    action_variants: BTreeSet<WorkflowActionSemanticVariant>,
    next_actors: BTreeSet<String>,
    user_action_statuses: BTreeSet<String>,
    decision_dispositions: BTreeSet<String>,
}

impl ReachabilityCoverage {
    fn observe(
        &mut self,
        response: &Value,
        label: &str,
    ) -> Result<WorkflowProjection, Box<dyn Error>> {
        let (state, workflow_value) = if let Some(workflow) = response.pointer("/state/workflow") {
            (response.get("state"), workflow)
        } else if let Some(workflow) = response.get("workflow") {
            (response.get("state"), workflow)
        } else if let Some(workflow) = response.pointer("/active_task/workflow") {
            (response.get("active_task"), workflow)
        } else {
            return Err(format!("{label} omitted a workflow projection: {response}").into());
        };

        if let Some(state) = state {
            for (field, set) in [
                ("mode", &mut self.modes),
                ("work_phase", &mut self.work_phases),
            ] {
                if let Some(value) = state.get(field).and_then(Value::as_str) {
                    set.insert(value.to_owned());
                }
            }
            if let Some(value) = state
                .pointer("/lifecycle/lifecycle_phase")
                .and_then(Value::as_str)
            {
                self.lifecycle_phases.insert(value.to_owned());
            }
        }
        if let Some(value) = workflow_value
            .pointer("/checkpoint/readiness")
            .and_then(Value::as_str)
        {
            self.checkpoint_readiness.insert(value.to_owned());
        }
        if let Some(requirements) = workflow_value
            .pointer("/checkpoint/decision_recovery_requirements")
            .and_then(Value::as_array)
        {
            for requirement in requirements {
                if let Some(disposition) = requirement.get("disposition").and_then(Value::as_str) {
                    self.decision_dispositions.insert(disposition.to_owned());
                }
            }
        }

        let workflow: WorkflowProjection = serde_json::from_value(workflow_value.clone())?;
        assert_eq!(
            serde_json::to_value(&workflow)?,
            *workflow_value,
            "{label} must preserve the exact typed Core workflow projection"
        );
        self.workflow_states.insert(
            workflow_value["kind"]
                .as_str()
                .ok_or("workflow kind")?
                .to_owned(),
        );
        self.next_actors
            .insert(workflow.next_actor().as_str().to_owned());
        self.assert_liveness_and_admission(
            &workflow,
            workflow_value["expected_state_version"]
                .as_u64()
                .ok_or("workflow expected_state_version")?,
            label,
        );
        Ok(workflow)
    }

    fn assert_liveness_and_admission(
        &mut self,
        workflow: &WorkflowProjection,
        expected_state_version: u64,
        label: &str,
    ) {
        let catalog = workflow.transition_catalog();
        let required = catalog.required_transition();
        let mut agent_transition_count = 0;
        let mut user_transition_count = 0;
        let mut close_route_count = 0;

        for transition in &catalog.transitions {
            self.action_variants
                .insert(transition.action_key.semantic_variant);
            assert_eq!(
                transition.expected_state_version, expected_state_version,
                "{label}: every descriptor must bind the projected state version"
            );
            match transition.actor {
                WorkflowTransitionActor::Agent => {
                    agent_transition_count += 1;
                    crate::model_check_current_transition(workflow, transition)
                        .unwrap_or_else(|error| panic!("{label}: {error}"));
                }
                WorkflowTransitionActor::User => {
                    user_transition_count += 1;
                    assert_eq!(
                        transition.action_key.semantic_variant,
                        WorkflowActionSemanticVariant::ResolveUserAction,
                        "{label}: only the User Channel resolves user authority"
                    );
                }
                WorkflowTransitionActor::System => {
                    panic!("{label}: current public workflow must not advertise a System action")
                }
            }
            if transition.action_key.semantic_variant == WorkflowActionSemanticVariant::CloseTask {
                close_route_count += 1;
            }
        }

        match workflow.kind() {
            WorkflowStateKind::Terminal => {
                assert_eq!(workflow.next_actor(), AuthorityNextActor::None, "{label}");
                assert!(catalog.transitions.is_empty(), "{label}");
                assert!(required.is_none(), "{label}");
            }
            _ => match workflow.next_actor() {
                AuthorityNextActor::User => {
                    let required = required.unwrap_or_else(|| {
                        panic!("{label}: User authority needs one exact action")
                    });
                    assert_eq!(required.actor, WorkflowTransitionActor::User, "{label}");
                    assert_eq!(user_transition_count, 1, "{label}");
                }
                AuthorityNextActor::Agent => {
                    assert!(
                        agent_transition_count > 0,
                        "{label}: Agent has no executable action"
                    );
                    if let Some(required) = required {
                        assert_eq!(required.actor, WorkflowTransitionActor::Agent, "{label}");
                    }
                }
                AuthorityNextActor::None => {
                    assert!(
                        close_route_count > 0,
                        "{label}: nonterminal state without an actor needs an explicit close route"
                    );
                }
            },
        }
        assert!(
            catalog
                .transitions
                .iter()
                .filter(|transition| transition.role == WorkflowActionRole::Required)
                .count()
                <= 1,
            "{label}: the catalog cannot advertise competing required actions"
        );
    }

    fn assert_required_coverage(&self) {
        for expected in ["advisor", "direct", "work"] {
            assert!(self.modes.contains(expected), "missing mode {expected}");
        }
        for expected in ["shaping", "implementation"] {
            assert!(
                self.work_phases.contains(expected),
                "missing work phase {expected}"
            );
        }
        for expected in [
            "shaping_required",
            "awaiting_user_action",
            "decision_recovery_required",
            "ready_to_apply_decisions",
            "ready_for_change_unit",
            "ready_to_finalize_advice",
            "ready_for_implementation",
            "implementation",
            "close_review",
            "terminal",
        ] {
            assert!(
                self.workflow_states.contains(expected),
                "missing reachable workflow state {expected}; saw {:?}",
                self.workflow_states
            );
        }
        for expected in ["pending", "resolved", "stale", "expired", "superseded"] {
            assert!(
                self.user_action_statuses.contains(expected),
                "missing user-action status {expected}; saw {:?}",
                self.user_action_statuses
            );
        }
        for expected in ["blocked", "ready", "superseded"] {
            assert!(
                self.checkpoint_readiness.contains(expected),
                "missing checkpoint readiness {expected}; saw {:?}",
                self.checkpoint_readiness
            );
        }
        for expected in ["accepted", "rejected", "deferred"] {
            assert!(
                self.decision_dispositions.contains(expected),
                "missing decision disposition {expected}; saw {:?}",
                self.decision_dispositions
            );
        }
        for expected in [
            WorkflowActionSemanticVariant::CreateInitial,
            WorkflowActionSemanticVariant::ReplaceCurrent,
            WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
            WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit,
            WorkflowActionSemanticVariant::FinalizeAdvice,
            WorkflowActionSemanticVariant::AdvanceTask,
            WorkflowActionSemanticVariant::PrepareEvidenceCapture,
            WorkflowActionSemanticVariant::PrepareWrite,
            WorkflowActionSemanticVariant::StageArtifact,
            WorkflowActionSemanticVariant::RecordRun,
            WorkflowActionSemanticVariant::RequestUserAction,
            WorkflowActionSemanticVariant::ResolveUserAction,
            WorkflowActionSemanticVariant::ReconcileChanges,
            WorkflowActionSemanticVariant::CheckClose,
            WorkflowActionSemanticVariant::CloseTask,
        ] {
            assert!(
                self.action_variants.contains(&expected),
                "missing action variant {}; saw {:?}",
                expected.as_str(),
                self.action_variants
            );
        }
    }
}

fn checkpoint_request(
    label: &str,
    state_version: u64,
    task_id: &str,
    scope_revision: u64,
    baseline_ref: &str,
    checkpoint_operation: ShapingCheckpointOperation,
) -> RecordShapingCheckpointRequest {
    RecordShapingCheckpointRequest {
        envelope: envelope(
            &format!("req_{label}"),
            Some(&format!("idem_{label}")),
            false,
            Some(state_version),
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        checkpoint_operation,
        scope_revision,
        baseline_ref: RequiredNullable::some(
            BaselineRef::parse(baseline_ref).expect("canonical explorer BaselineRef"),
        ),
        summary: format!("Reachable workflow state {label}."),
        implementation_boundary: RequiredNullable::some(
            "Keep the state-space exploration inside its disposable fixture.".to_owned(),
        ),
        gaps: Vec::new(),
        source_refs: Vec::new(),
        evidence_refs: Vec::new(),
    }
}

fn status(harness: &MethodHarness, label: &str, task_id: &str) -> Result<Value, Box<dyn Error>> {
    Ok(harness
        .service
        .status(
            volicord_types::methods::StatusRequest {
                envelope: envelope(
                    &format!("req_{label}_status"),
                    None,
                    false,
                    None,
                    Some(task_id),
                ),
                include: status_include(),
                continuity_page: None,
            },
            invocation(OperationCategory::Read),
        )?
        .response_value)
}

fn action_keys(workflow: &WorkflowProjection) -> BTreeSet<(String, String)> {
    workflow
        .transition_catalog()
        .transitions
        .iter()
        .map(|transition| {
            (
                transition.action_key.method.as_str().to_owned(),
                transition.action_key.semantic_variant.as_str().to_owned(),
            )
        })
        .collect()
}

fn assert_recoverable_rejection(
    response: &Value,
    current_workflow: &WorkflowProjection,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(response["base"]["response_kind"], "rejected", "{label}");
    assert_eq!(response["base"]["effect_kind"], "no_effect", "{label}");
    assert_eq!(
        response["errors"][0]["details"]["state_change_applied"], false,
        "{label}"
    );
    let recovery: WorkflowActionKey =
        serde_json::from_value(response["errors"][0]["details"]["recovery_action_key"].clone())?;
    assert!(
        current_workflow
            .transition_catalog()
            .transition(&recovery)
            .is_some(),
        "{label}: recoverable rejection referenced a non-current action {recovery:?}"
    );
    Ok(())
}

fn explore_direct_mode(coverage: &mut ReachabilityCoverage) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_explorer_direct_intake",
            "idem_explorer_direct_intake",
            false,
            Some(0),
            RequestedMode::Direct,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&intake.response_value, "direct intake")?;
    let task_id = response_record_id(&intake.response_value, "task_ref");

    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_explorer_direct_scope",
            "idem_explorer_direct_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Direct state-space scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&scoped.response_value, "direct create Change Unit")?;

    let mut rebaseline = update_scope_request(
        "req_explorer_direct_rebaseline",
        "idem_explorer_direct_rebaseline",
        false,
        Some(2),
        &task_id,
        ChangeUnitOperation::ReplaceCurrent,
        "Direct explicit rebaseline.",
    );
    rebaseline.baseline_ref = RequiredNullable::some(
        BaselineRef::parse("baseline_explorer_revised").expect("canonical revised baseline"),
    );
    let rebaselined = harness
        .service
        .update_scope(rebaseline, invocation(OperationCategory::AgentWorkflow))?;
    coverage.observe(&rebaselined.response_value, "direct explicit rebaseline")?;
    Ok(())
}

fn explore_advisor_mode(coverage: &mut ReachabilityCoverage) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_explorer_advisor_intake",
            "idem_explorer_advisor_intake",
            false,
            Some(0),
            RequestedMode::Advisor,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&intake.response_value, "advisor intake")?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scoped = harness.service.update_scope(
        advisor_update_scope_request(
            "req_explorer_advisor_scope",
            "idem_explorer_advisor_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Advisor state-space scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&scoped.response_value, "advisor scope")?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let shaped = harness.service.record_shaping_checkpoint(
        checkpoint_request(
            "explorer_advisor_checkpoint",
            2,
            &task_id,
            1,
            "baseline_test",
            ShapingCheckpointOperation::CreateInitial,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&shaped.response_value, "advisor ready checkpoint")?;
    let checkpoint_id = shaped.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .ok_or("advisor checkpoint id")?;
    let finalized = harness.service.finalize_advice(
        FinalizeAdviceRequest {
            envelope: envelope(
                "req_explorer_finalize_advice",
                Some("idem_explorer_finalize_advice"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(checkpoint_id),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::parse("baseline_test")?,
            user_action_resolution_ids: Vec::new(),
            result_summary: "Bounded state-space advice is ready.".to_owned(),
            result_refs: Vec::new(),
            evidence_refs: Vec::new(),
            residual_risks: Vec::new(),
            recovery_constraints: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&finalized.response_value, "advisor finalization")?;
    Ok(())
}

fn explore_pre_change_unit_state(
    coverage: &mut ReachabilityCoverage,
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_explorer_pre_cu_intake",
            "idem_explorer_pre_cu_intake",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let mut request = checkpoint_request(
        "explorer_pre_cu_checkpoint",
        1,
        &task_id,
        0,
        "baseline_test",
        ShapingCheckpointOperation::CreateInitial,
    );
    request.baseline_ref = RequiredNullable::null();
    request.gaps = vec![ShapingGapInput {
        gap_kind: ShapingGapKind::UserTechnicalDecisionRequired,
        summary: "Choose the bounded technical direction before Change Unit creation.".to_owned(),
        affected_refs: Vec::new(),
        user_action: RequiredNullable::some(ShapingUserActionDraft {
            action: super::user_action_request(
                "unused",
                "unused",
                false,
                Some(1),
                &task_id,
                None,
                JudgmentKind::TechnicalDecision,
            )
            .action,
            expires_at: RequiredNullable::null(),
        }),
    }];
    let shaped = harness
        .service
        .record_shaping_checkpoint(request, invocation(OperationCategory::AgentWorkflow))?;
    coverage.observe(
        &shaped.response_value,
        "pending before Change Unit creation",
    )?;
    let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
        .as_str()
        .ok_or("pre-Change-Unit request id")?;
    let resolved = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_explorer_pre_cu_resolve",
            "submission_explorer_pre_cu_resolve",
            None,
            &task_id,
            request_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    coverage.observe(
        &resolved.response_value,
        "ready before Change Unit creation",
    )?;
    Ok(())
}

fn explore_work_and_history(coverage: &mut ReachabilityCoverage) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_explorer_work_intake",
            "idem_explorer_work_intake",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let initial = coverage.observe(&intake.response_value, "work intake")?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_explorer_work_scope",
            "idem_explorer_work_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Work state-space scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let shaping = coverage.observe(&scoped.response_value, "work scope")?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");

    let before_rejection = harness.counts()?;
    let premature = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_explorer_premature_advance",
                Some("idem_explorer_premature_advance"),
                false,
                Some(2),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_absent"),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::parse("baseline_test")?,
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_recoverable_rejection(&premature.response_value, &shaping, "premature advance")?;
    assert_eq!(harness.counts()?, before_rejection);

    let first_checkpoint = harness.service.record_shaping_checkpoint(
        checkpoint_request(
            "explorer_work_checkpoint",
            2,
            &task_id,
            1,
            "baseline_test",
            ShapingCheckpointOperation::CreateInitial,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_ready =
        coverage.observe(&first_checkpoint.response_value, "work ready checkpoint")?;
    let first_checkpoint_id = first_checkpoint.response_value["shaping_checkpoint"]
        ["shaping_checkpoint_id"]
        .as_str()
        .ok_or("work checkpoint id")?
        .to_owned();
    let first_keys = action_keys(&first_ready);

    let replacement = harness.service.record_shaping_checkpoint(
        checkpoint_request(
            "explorer_work_checkpoint_replace",
            3,
            &task_id,
            1,
            "baseline_test",
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new(&first_checkpoint_id),
                retired_non_authorizing_request_refs: Vec::new(),
                carry_forward_application_refs: Vec::new(),
                stale_authority_actions: Vec::new(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_ready =
        coverage.observe(&replacement.response_value, "work replacement checkpoint")?;
    assert_eq!(
        action_keys(&replacement_ready),
        first_keys,
        "superseded checkpoint history must not change the current action set"
    );
    let predecessor = harness
        .store()?
        .shaping_checkpoint_record(&TaskId::new(&task_id), &first_checkpoint_id)?
        .ok_or("superseded predecessor checkpoint")?;
    coverage
        .checkpoint_readiness
        .insert(predecessor.readiness.as_str().to_owned());
    let replacement_id = replacement.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .ok_or("replacement checkpoint id")?
        .to_owned();

    let advance_request = AdvanceTaskRequest {
        envelope: envelope(
            "req_explorer_work_advance",
            Some("idem_explorer_work_advance"),
            false,
            Some(4),
            Some(&task_id),
        ),
        task_id: TaskId::new(&task_id),
        shaping_checkpoint_id: ShapingCheckpointId::new(&replacement_id),
        change_unit_id: ChangeUnitId::new(&change_unit_id),
        scope_revision: 1,
        baseline_ref: BaselineRef::parse("baseline_test")?,
        user_action_resolution_ids: Vec::new(),
    };
    let advanced = harness.service.advance_task(
        advance_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let implementation = coverage.observe(&advanced.response_value, "work implementation")?;
    let replay = harness.service.advance_task(
        advance_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(replay.response_value, advanced.response_value);

    let before_run = harness.counts()?;
    let run_request = record_run_request(
        "req_explorer_history_run",
        "idem_explorer_history_run",
        false,
        Some(before_run.state_version),
        &task_id,
        &change_unit_id,
    );
    let run = harness.service.record_run(
        run_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_run = coverage.observe(&run.response_value, "work Run history")?;
    assert_eq!(
        action_keys(&after_run),
        action_keys(&implementation),
        "historical Runs must not change the current action set"
    );
    let after_run_counts = harness.counts()?;
    let run_replay = harness
        .service
        .record_run(run_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(run_replay.response_value, run.response_value);
    assert_eq!(harness.counts()?, after_run_counts);

    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        harness.counts()?.state_version,
        "explorer_close",
        true,
    )?;
    let close_review = status(&harness, "explorer_close_review", &task_id)?;
    coverage.observe(&close_review, "work close review")?;
    let after_acceptance = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "explorer_acceptance",
    )?;
    let accepted = status(&harness, "explorer_accepted", &task_id)?;
    coverage.observe(&accepted, "work final acceptance")?;
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_explorer_close",
            idempotency_key: Some("idem_explorer_close"),
            dry_run: false,
            expected_state_version: Some(after_acceptance),
            task_id: &task_id,
            intent: volicord_types::values::CloseIntent::Complete,
            close_reason: Some(volicord_types::values::CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    coverage.observe(&closed.response_value, "work terminal")?;

    assert!(initial.transition_catalog().required_transition().is_some());
    Ok(())
}

fn explore_decision_outcomes(coverage: &mut ReachabilityCoverage) -> Result<(), Box<dyn Error>> {
    for (label, selected, disposition) in [
        ("accepted", "accept", "accepted"),
        ("rejected", "reject", "rejected"),
        ("deferred", "defer", "deferred"),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = shaping_task(&harness, &format!("explorer_{label}"))?;
        let shaped = record_user_owned_gap(
            &harness,
            &format!("explorer_{label}"),
            &task_id,
            &change_unit_id,
            ShapingGapKind::UserScopeDecisionRequired,
            JudgmentKind::ScopeDecision,
        )?;
        coverage.observe(&shaped.response_value, &format!("{label} pending decision"))?;
        let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
            .as_str()
            .ok_or("decision request id")?
            .to_owned();
        coverage
            .user_action_statuses
            .insert(user_action_status(&harness, &request_id)?);
        let resolved = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!("req_explorer_{label}_resolve"),
                &format!("submission_explorer_{label}"),
                None,
                &task_id,
                &request_id,
                selected,
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        coverage.observe(&resolved.response_value, &format!("{label} decision"))?;
        coverage
            .user_action_statuses
            .insert(user_action_status(&harness, &request_id)?);
        coverage
            .decision_dispositions
            .insert(disposition.to_owned());

        if selected == "accept" {
            let resolution_ref: StateRecordRef = serde_json::from_value(
                resolved.response_value["user_action_resolution_ref"].clone(),
            )?;
            let before = harness.counts()?;
            let mut apply = update_scope_request(
                "req_explorer_apply_decision",
                "idem_explorer_apply_decision",
                false,
                Some(before.state_version),
                &task_id,
                ChangeUnitOperation::KeepCurrent,
                "Apply the exact accepted scope decision.",
            );
            apply.related_scope_decision_refs = vec![resolution_ref];
            let applied = harness
                .service
                .update_scope(apply, invocation(OperationCategory::AgentWorkflow))?;
            coverage.observe(&applied.response_value, "accepted applied decision")?;
            let mut rebaseline = update_scope_request(
                "req_explorer_stale_application",
                "idem_explorer_stale_application",
                false,
                Some(harness.counts()?.state_version),
                &task_id,
                ChangeUnitOperation::ReplaceCurrent,
                "Retarget the accepted authority onto a new baseline.",
            );
            rebaseline.baseline_ref =
                RequiredNullable::some(BaselineRef::parse("baseline_explorer_stale_application")?);
            let stale = harness
                .service
                .update_scope(rebaseline, invocation(OperationCategory::AgentWorkflow))?;
            coverage.observe(&stale.response_value, "stale application recovery")?;
            coverage
                .user_action_statuses
                .insert(user_action_status(&harness, &request_id)?);
        } else if selected == "reject" {
            let checkpoint_id = shaped.response_value["shaping_checkpoint"]
                ["shaping_checkpoint_id"]
                .as_str()
                .ok_or("rejected decision checkpoint id")?;
            let retired_request_ref: StateRecordRef = serde_json::from_value(
                resolved.response_value["state"]["workflow"]["checkpoint"]
                    ["decision_recovery_requirements"][0]["user_action_request_ref"]
                    .clone(),
            )?;
            let replaced = harness.service.record_shaping_checkpoint(
                checkpoint_request(
                    "explorer_rejected_retirement",
                    harness.counts()?.state_version,
                    &task_id,
                    1,
                    "baseline_test",
                    ShapingCheckpointOperation::ReplaceCurrent {
                        expected_current_checkpoint_id: ShapingCheckpointId::new(checkpoint_id),
                        retired_non_authorizing_request_refs: vec![retired_request_ref],
                        carry_forward_application_refs: Vec::new(),
                        stale_authority_actions: Vec::new(),
                    },
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            coverage.observe(&replaced.response_value, "rejected authority retirement")?;
            coverage
                .user_action_statuses
                .insert(user_action_status(&harness, &request_id)?);
        }
    }
    Ok(())
}

fn explore_expired_and_superseded_authority(
    coverage: &mut ReachabilityCoverage,
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = super::ManualClock::at(super::DEFAULT_METHOD_TEST_CLOCK);
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = shaping_task(&harness, "explorer_expired")?;
    let action = super::user_action_request(
        "unused",
        "unused",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    )
    .action;
    let mut request = checkpoint_request(
        "explorer_expiring_checkpoint",
        2,
        &task_id,
        1,
        "baseline_test",
        ShapingCheckpointOperation::CreateInitial,
    );
    request.gaps = vec![ShapingGapInput {
        gap_kind: ShapingGapKind::UserTechnicalDecisionRequired,
        summary: "Time-bounded decision authority.".to_owned(),
        affected_refs: Vec::new(),
        user_action: RequiredNullable::some(ShapingUserActionDraft {
            action,
            expires_at: RequiredNullable::some(volicord_types::values::UtcTimestamp::parse(
                "2026-06-18T00:01:00Z",
            )?),
        }),
    }];
    let shaped = harness
        .service
        .record_shaping_checkpoint(request, invocation(OperationCategory::AgentWorkflow))?;
    coverage.observe(&shaped.response_value, "expiring pending authority")?;
    let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
        .as_str()
        .ok_or("expiring request id")?
        .to_owned();
    clock.advance(chrono::Duration::minutes(2));
    let expired = status(&harness, "explorer_expired", &task_id)?;
    coverage.observe(&expired, "expired authority recovery")?;
    coverage.user_action_statuses.insert("expired".to_owned());

    let mut superseding = intake_request(
        "req_explorer_superseding_task",
        "idem_explorer_superseding_task",
        false,
        Some(harness.counts()?.state_version),
        RequestedMode::Direct,
    );
    superseding.resume_policy = volicord_types::values::ResumePolicy::SupersedeActive;
    let successor = harness
        .service
        .intake(superseding, invocation(OperationCategory::AgentWorkflow))?;
    coverage.observe(&successor.response_value, "successor task")?;
    coverage
        .user_action_statuses
        .insert(user_action_status(&harness, &request_id)?);
    let superseded = status(&harness, "explorer_superseded", &task_id)?;
    coverage.observe(&superseded, "superseded task terminal")?;
    Ok(())
}

#[test]
fn deterministic_reachable_workflow_state_space_preserves_cross_layer_invariants(
) -> Result<(), Box<dyn Error>> {
    let mut coverage = ReachabilityCoverage::default();
    explore_direct_mode(&mut coverage)?;
    explore_advisor_mode(&mut coverage)?;
    explore_pre_change_unit_state(&mut coverage)?;
    explore_work_and_history(&mut coverage)?;
    explore_decision_outcomes(&mut coverage)?;
    explore_expired_and_superseded_authority(&mut coverage)?;
    coverage.assert_required_coverage();
    Ok(())
}
