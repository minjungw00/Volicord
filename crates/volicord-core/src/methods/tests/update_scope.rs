use super::*;

#[test]
fn advisor_current_change_unit_next_action_is_record_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_advisor_next_task",
            "idem_advisor_next_task",
            false,
            Some(0),
            RequestedMode::Advisor,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");

    let response = harness.service.update_scope(
        update_scope_request(
            "req_advisor_next_scope",
            "idem_advisor_next_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Create an advisory Change Unit.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let next_actions = response.response_value["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    assert_eq!(next_actions.len(), 1);
    assert_eq!(next_actions[0]["action_kind"], "record_run");
    assert_eq!(next_actions[0]["owner_method"], "volicord.record_run");

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_advisor_next_status", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: true,
                ..status_include()
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let status_actions = status.response_value["next_actions"]
        .as_array()
        .expect("status next_actions should be an array");
    assert!(status_actions.iter().any(|action| {
        action["action_kind"] == "record_run" && action["owner_method"] == "volicord.record_run"
    }));
    assert!(status_actions
        .iter()
        .all(|action| action["action_kind"] != "prepare_write"));
    Ok(())
}

#[test]
fn update_scope_commits_once_and_creates_one_current_change_unit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_scope_task",
            "idem_scope_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_create",
            "idem_scope_create",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Create current export scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 2);
    assert!(response.response_value["change_unit_ref"].is_object());
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
    let revision = task_revision(&harness, &task_id)?;
    assert_eq!(revision.scope_revision, 1);
    assert_eq!(revision.close_basis_revision, 1);
    assert!(revision.current_close_basis.is_none());
    Ok(())
}

#[test]
fn update_scope_replaces_current_and_marks_write_ticket_stale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_replace_task",
            "idem_replace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let create = harness.service.update_scope(
        update_scope_request(
            "req_replace_create",
            "idem_replace_create",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = create.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    insert_active_write_ticket(&harness, &task_id, &change_unit_id)?;
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_replace_current",
            "idem_replace_current",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["stale_write_ticket_refs"]
            .as_array()
            .expect("stale refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
    assert_eq!(write_ticket_status(&harness, "wa_replace")?, "stale");
    Ok(())
}

#[test]
fn material_scope_change_increments_revision_and_invalidates_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_invalidates")?;
    let mut record = record_run_request(
        "req_scope_basis_run",
        "idem_scope_basis_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    record.close_assessment = Some(close_assessment_with_risks(
        "Established close basis.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(record, invocation(OperationCategory::AgentWorkflow))?;
    let before = task_revision(&harness, &task_id)?;
    assert!(before.current_close_basis.is_some());

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_material_change",
            "idem_scope_material_change",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Materially changed current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = task_revision(&harness, &task_id)?;
    let (_, event_payload, _) = latest_task_event(&harness)?;

    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(after.scope_revision, before.scope_revision + 1);
    assert_eq!(after.close_basis_revision, before.close_basis_revision + 1);
    assert!(after.current_close_basis.is_none());
    assert_eq!(response.response_value["state"]["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value["state"],
        "missing_current_close_basis",
    );
    assert_eq!(event_payload["scope_changed"], true);
    assert_eq!(event_payload["scope_revision"], after.scope_revision);
    assert_eq!(
        event_payload["close_basis_revision"],
        after.close_basis_revision
    );
    Ok(())
}

#[test]
fn semantic_noop_scope_update_does_not_increment_revisions() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "scope_noop")?;
    let before = task_revision(&harness, &task_id)?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_noop",
            "idem_scope_noop",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "  Initial current scope.  ",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = task_revision(&harness, &task_id)?;
    let (_, event_payload, _) = latest_task_event(&harness)?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(after.scope_revision, before.scope_revision);
    assert_eq!(after.close_basis_revision, before.close_basis_revision);
    assert_eq!(after.current_close_basis, before.current_close_basis);
    assert_eq!(event_payload["scope_changed"], false);
    Ok(())
}

#[test]
fn update_scope_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_dry_task",
            "idem_dry_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_dry",
            "idem_scope_dry",
            true,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Dry-run scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_ref_alone_does_not_change_current_scope() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_decision_task",
            "idem_decision_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let decision = harness.service.request_user_judgment(
        user_judgment_request(
            "req_scope_decision_ref_only",
            "idem_scope_decision_ref_only",
            false,
            Some(1),
            &task_id,
            None,
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let decision_ref: StateRecordRef =
        serde_json::from_value(decision.response_value["user_judgment_ref"].clone())?;
    let decision_id = decision_ref.record_id.as_str().to_owned();
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_scope_decision_ref_only_record",
            "idem_scope_decision_ref_only_record",
            Some(2),
            &task_id,
            &decision_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let response = harness.service.update_scope(
        UpdateScopeRequest {
            envelope: envelope(
                "req_decision_only",
                Some("idem_decision_only"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            goal_summary: None.into(),
            scope_update: None.into(),
            scope_boundary: None.into(),
            non_goals: None.into(),
            acceptance_criteria: None.into(),
            autonomy_boundary: None.into(),
            baseline_ref: None.into(),
            change_unit: ChangeUnitUpdate {
                operation: ChangeUnitOperation::KeepCurrent,
                effect_contract: None,
                fields: Map::new(),
            },
            related_scope_decision_refs: vec![decision_ref],
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["state"]["scope_summary"],
        "Initial test scope."
    );
    assert_eq!(
        response.response_value["linked_scope_decision_refs"]
            .as_array()
            .expect("linked refs should be an array")
            .len(),
        1
    );
    Ok(())
}

#[test]
fn accepted_current_user_scope_decision_links_scope_update() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_link_accept")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "link_accept",
        true,
    )?;

    let mut request = update_scope_request(
        "req_scope_link_accept_update",
        "idem_scope_link_accept_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Decision-backed material scope.",
    );
    request.related_scope_decision_refs = vec![decision_ref.clone()];
    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["linked_scope_decision_refs"],
        json!([decision_ref])
    );
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "stale");
    assert_eq!(user_judgment_basis_status(&harness, &decision_id)?, "stale");
    Ok(())
}

#[test]
fn rejected_scope_decision_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let suffix = "scope_link_rejected";
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
    let (state_version, decision_ref, decision_id) =
        record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, suffix, false)?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_link_rejected_update",
        "idem_scope_link_rejected_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Rejected scope decision must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &decision_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "resolved");
    Ok(())
}

#[test]
fn agent_or_unverified_scope_decision_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    for case in ["agent_actor", "agent_source", "missing_provenance"] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("scope_{case}"))?;
        let (state_version, decision_ref, decision_id) =
            record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, case, true)?;
        match case {
            "agent_actor" => {
                set_user_judgment_resolution_actor(&harness, &decision_id, AGENT_ACTOR_SOURCE)?
            }
            "agent_source" => {
                set_user_judgment_resolved_by_actor_source(&harness, &decision_id, "agent")?;
            }
            "missing_provenance" => {
                clear_user_judgment_actor_provenance(&harness, &decision_id)?;
            }
            _ => unreachable!("covered cases are exhaustive"),
        }
        let before = harness.counts()?;
        let mut request = update_scope_request(
            &format!("req_{case}_scope_link"),
            &format!("idem_{case}_scope_link"),
            false,
            Some(state_version),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Agent-recorded scope decision must not link.",
        );
        request.related_scope_decision_refs = vec![decision_ref];

        let response = harness
            .service
            .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, before);
        assert_eq!(user_judgment_status(&harness, &decision_id)?, "resolved");
    }
    Ok(())
}

#[test]
fn scope_decision_for_other_operation_cannot_authorize_scope_update() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_required_for")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "required_for",
        true,
    )?;
    set_user_judgment_required_for(
        &harness,
        &decision_id,
        &[volicord_types::JudgmentRequiredFor::PrepareWrite],
    )?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_required_for_update",
        "idem_scope_required_for_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Prepare-write decision must not authorize scope update.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "resolved");
    Ok(())
}

#[test]
fn stale_scope_decision_cannot_authorize_scope_update() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_old_revision")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "old_revision",
        true,
    )?;
    let autonomous = harness.service.update_scope(
        update_scope_request(
            "req_scope_old_revision_first",
            "idem_scope_old_revision_first",
            false,
            Some(state_version),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Autonomous material scope change before reuse.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let next_state_version = autonomous.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    assert_eq!(user_judgment_status(&harness, &decision_id)?, "stale");

    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_old_revision_reuse",
        "idem_scope_old_revision_reuse",
        false,
        Some(next_state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Attempt to reuse stale scope decision.",
    );
    request.related_scope_decision_refs = vec![decision_ref];
    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_for_another_change_unit_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_other_cu")?;
    let (state_version, decision_ref, decision_id) =
        record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, "other_cu", true)?;
    mutate_user_judgment_basis_json(&harness, &decision_id, |basis| {
        basis["change_unit_id"] = json!("cu_not_current");
    })?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_other_cu_update",
        "idem_scope_other_cu_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Other Change Unit decision must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_with_incompatible_affected_refs_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "scope_bad_affected_refs")?;
    let (state_version, decision_ref, decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_affected_refs",
        true,
    )?;
    let incompatible_ref = test_state_record_ref(
        StateRecordKind::ChangeUnit,
        "cu_not_current",
        PROJECT_ID,
        &task_id,
        Some(2),
    );
    set_user_judgment_affected_refs(&harness, &decision_id, &[incompatible_ref])?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_bad_affected_refs_update",
        "idem_scope_bad_affected_refs_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Incompatible affected refs must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn expired_scope_decision_cannot_be_linked() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_expired")?;
    let (state_version, decision_ref, decision_id) =
        record_scope_decision_authority(&harness, &task_id, &change_unit_id, 2, "expired", true)?;
    set_user_judgment_expires_at(&harness, &decision_id, "2000-01-01T00:00:00Z")?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_expired_update",
        "idem_scope_expired_update",
        false,
        Some(state_version),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Expired scope decision must not link.",
    );
    request.related_scope_decision_refs = vec![decision_ref];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn invalid_related_scope_decision_ref_has_no_update_scope_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "scope_invalid_ref")?;
    let original_scope = current_change_unit_scope(&harness, &task_id)?;
    let before = harness.counts()?;
    let mut request = update_scope_request(
        "req_scope_invalid_ref_update",
        "idem_scope_invalid_ref_update",
        false,
        Some(2),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Invalid ref must not update scope.",
    );
    request.related_scope_decision_refs = vec![test_state_record_ref(
        StateRecordKind::UserJudgment,
        "uj_missing_scope_decision",
        PROJECT_ID,
        &task_id,
        Some(2),
    )];

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        current_change_unit_scope(&harness, &task_id)?,
        original_scope
    );
    Ok(())
}

#[test]
fn autonomous_scope_update_still_succeeds_without_scope_decision() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "scope_autonomous")?;
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_autonomous_update",
            "idem_scope_autonomous_update",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Autonomous scope update with no decision ref.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?.state_version, before.state_version + 1);
    assert_eq!(
        response.response_value["linked_scope_decision_refs"],
        json!([])
    );
    Ok(())
}

#[test]
fn material_scope_update_invalidates_scope_decisions_atomically() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "scope_atomic_invalidation")?;
    let (after_resolved, _, resolved_decision_id) = record_scope_decision_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "atomic_resolved",
        true,
    )?;
    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_scope_atomic_pending",
            "idem_scope_atomic_pending",
            false,
            Some(after_resolved),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_decision_id = response_record_id(&pending.response_value, "user_judgment_ref");
    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_atomic_update",
            "idem_scope_atomic_update",
            false,
            Some(after_resolved + 1),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Material scope change invalidates scope decisions.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        user_judgment_status(&harness, &resolved_decision_id)?,
        "stale"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &resolved_decision_id)?,
        "stale"
    );
    assert_eq!(
        user_judgment_status(&harness, &pending_decision_id)?,
        "superseded"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &pending_decision_id)?,
        "superseded"
    );
    Ok(())
}
