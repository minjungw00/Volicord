use super::*;

#[test]
fn intake_commits_once_and_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let request = intake_request(
        "req_intake",
        "idem_intake",
        false,
        Some(0),
        RequestedMode::Auto,
    );

    let first = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;

    assert_eq!(first.response_value["base"]["response_kind"], "result");
    assert_eq!(
        first.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(first.response_value["base"]["state_version"], 1);
    assert_eq!(first.response_value["state"]["mode"], "work");
    assert_eq!(
        first.response_value["state"]["requested_control_level"],
        "auto"
    );
    assert_eq!(
        first.response_value["state"]["effective_control_level"],
        "tracked"
    );
    assert!(first.response_value["state"]["project_policy"].is_null());
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.tasks, before.tasks + 1);
    assert_eq!(after_first.authority_events, before.authority_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);

    let second = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn intake_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let response = harness.service.intake(
        intake_request(
            "req_intake_dry",
            "idem_intake_dry",
            true,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_persists_normalized_non_authoritative_source_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let mut request = intake_request(
        "req_intake_source",
        "idem_intake_source",
        false,
        Some(0),
        RequestedMode::Advisor,
    );
    request.initial_source_refs = vec![volicord_types::SourceRef::RepositoryFile(
        volicord_types::RepositoryFileSource {
            repository_path: "notes/./source.md".to_owned(),
            baseline_commit_sha: "A".repeat(40),
            content_sha256: "b".repeat(64),
            line_range: Some(volicord_types::SourceLineRange {
                start_line: 2,
                end_line: 4,
            })
            .into(),
        },
    )];

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    let task_id = response.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id should be present");
    let bounded_context: String = harness.conn()?.query_row(
        "SELECT bounded_context_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    let bounded_context: serde_json::Value = serde_json::from_str(&bounded_context)?;
    assert_eq!(
        bounded_context["initial_source_refs"][0]["source"]["repository_path"],
        "notes/source.md"
    );
    assert_eq!(
        bounded_context["initial_source_refs"][0]["source"]["baseline_commit_sha"],
        "a".repeat(40)
    );
    assert_eq!(bounded_context["initial_context_refs"], json!([]));
    Ok(())
}

#[test]
fn intake_rejects_structurally_invalid_source_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_intake_bad_source",
        "idem_intake_bad_source",
        false,
        Some(0),
        RequestedMode::Advisor,
    );
    request.initial_source_refs = vec![volicord_types::SourceRef::ExternalUri(
        volicord_types::ExternalUriSource {
            uri: "https://user@example.invalid/spec".to_owned(),
            retrieved_at: volicord_types::UtcTimestamp::parse("2026-07-12T00:00:00Z")?,
            content_sha256: "c".repeat(64),
        },
    )];

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_records_mode_default_phase_and_acceptance_policy() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let advisor = harness.service.intake(
        intake_request(
            "req_advisor_policy",
            "idem_advisor_policy",
            false,
            Some(0),
            RequestedMode::Advisor,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(advisor.response_value["state"]["work_phase"], "shaping");
    assert_eq!(
        advisor.response_value["state"]["effective_control_level"],
        "observe"
    );
    assert_eq!(
        advisor.response_value["state"]["acceptance_policy"],
        "not_required"
    );
    assert!(advisor.response_value["state"]["acceptance_policy_reason"]
        .as_str()
        .is_some_and(|reason| !reason.is_empty()));
    Ok(())
}

#[test]
fn intake_resolves_light_control_from_authoritative_project_policy() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let mut request = intake_request(
        "req_intake_light_policy",
        "idem_intake_light_policy",
        false,
        Some(harness.counts()?.state_version),
        RequestedMode::Direct,
    );
    request.requested_control_level = RequestedControlLevel::Light;

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["state"]["effective_control_level"],
        "light"
    );
    assert_eq!(
        response.response_value["state"]["acceptance_policy"],
        "policy_dependent"
    );
    assert_eq!(
        response.response_value["state"]["project_policy"]["policy_schema"],
        volicord_types::WORKFLOW_POLICY_CONTRACT_ID
    );
    assert_eq!(
        response.response_value["state"]["project_policy"]["policy_version"],
        1
    );
    Ok(())
}

#[test]
fn intake_applies_control_floor_and_policy_defaults() -> Result<(), Box<dyn Error>> {
    let disabled = MethodHarness::new()?;
    let mut disabled_policy = light_workflow_policy();
    disabled_policy["light"]["enabled"] = json!(false);
    disabled.set_workflow_policy(disabled_policy)?;
    let mut light_request = intake_request(
        "req_intake_disabled_light",
        "idem_intake_disabled_light",
        false,
        Some(disabled.counts()?.state_version),
        RequestedMode::Direct,
    );
    light_request.requested_control_level = RequestedControlLevel::Light;
    let light = disabled
        .service
        .intake(light_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        light.response_value["state"]["effective_control_level"],
        "tracked"
    );
    assert_eq!(
        light.response_value["state"]["acceptance_policy"],
        "required"
    );

    let sensitive = MethodHarness::new()?;
    let mut sensitive_request = intake_request(
        "req_intake_sensitive_control",
        "idem_intake_sensitive_control",
        false,
        Some(0),
        RequestedMode::Direct,
    );
    sensitive_request.requested_control_level = RequestedControlLevel::Sensitive;
    let sensitive = sensitive.service.intake(
        sensitive_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        sensitive.response_value["state"]["effective_control_level"],
        "sensitive"
    );
    assert_eq!(
        sensitive.response_value["state"]["acceptance_policy"],
        "required"
    );

    let direct_default = MethodHarness::new()?;
    direct_default.set_workflow_policy(light_workflow_policy())?;
    let direct = direct_default.service.intake(
        intake_request(
            "req_intake_direct_policy_default",
            "idem_intake_direct_policy_default",
            false,
            Some(direct_default.counts()?.state_version),
            RequestedMode::Direct,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        direct.response_value["state"]["effective_control_level"],
        "light"
    );
    Ok(())
}

#[test]
fn intake_resume_never_lowers_active_control_after_policy_relaxation() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let mut tracked_policy = light_workflow_policy();
    tracked_policy["default_direct_control"] = json!("tracked");
    harness.set_workflow_policy(tracked_policy)?;
    let created = harness.service.intake(
        intake_request(
            "req_intake_resume_no_lower_create",
            "idem_intake_resume_no_lower_create",
            false,
            Some(harness.counts()?.state_version),
            RequestedMode::Direct,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&created.response_value, "task_ref");
    harness.set_workflow_policy_version(2, light_workflow_policy())?;
    let mut resume = intake_request(
        "req_intake_resume_no_lower",
        "idem_intake_resume_no_lower",
        false,
        Some(harness.counts()?.state_version),
        RequestedMode::Direct,
    );
    resume.resume_policy = ResumePolicy::ResumeActive;
    resume.envelope.task_id = Some(TaskId::new(&task_id)).into();
    let resumed = harness
        .service
        .intake(resume, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        resumed.response_value["state"]["effective_control_level"],
        "tracked"
    );
    assert_eq!(
        response_record_id(&resumed.response_value, "task_ref"),
        task_id
    );
    Ok(())
}

#[test]
fn intake_resume_applies_preserved_strengthening_after_policy_relaxation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let created = harness.service.intake(
        intake_request(
            "req_intake_preserved_raise_create",
            "idem_intake_preserved_raise_create",
            false,
            Some(harness.counts()?.state_version),
            RequestedMode::Direct,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&created.response_value, "task_ref");
    assert_eq!(
        created.response_value["state"]["effective_control_level"],
        "light"
    );

    let mut sensitive_policy = light_workflow_policy();
    sensitive_policy["default_direct_control"] = json!("sensitive");
    harness.set_workflow_policy_version(2, sensitive_policy)?;
    harness.set_workflow_policy_version(3, light_workflow_policy())?;

    let mut resume = intake_request(
        "req_intake_preserved_raise_resume",
        "idem_intake_preserved_raise_resume",
        false,
        Some(harness.counts()?.state_version),
        RequestedMode::Direct,
    );
    resume.resume_policy = ResumePolicy::ResumeActive;
    resume.envelope.task_id = Some(TaskId::new(&task_id)).into();
    let resumed = harness
        .service
        .intake(resume, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        resumed.response_value["state"]["effective_control_level"],
        "sensitive"
    );
    assert_eq!(
        resumed.response_value["state"]["acceptance_policy"],
        "required"
    );
    let (effective, acceptance, metadata_json): (String, String, String) =
        harness.conn()?.query_row(
            "SELECT effective_control_level, acceptance_policy, metadata_json
           FROM tasks
          WHERE project_id = ?1 AND task_id = ?2",
            rusqlite::params![PROJECT_ID, task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    assert_eq!(effective, "sensitive");
    assert_eq!(acceptance, "required");
    assert!(serde_json::from_str::<Value>(&metadata_json)?
        .get("policy_control_reevaluation")
        .is_none());
    Ok(())
}

#[test]
fn intake_json_omission_defaults_requested_control_to_auto() -> Result<(), Box<dyn Error>> {
    let request = intake_request(
        "req_intake_omitted_control",
        "idem_intake_omitted_control",
        false,
        Some(0),
        RequestedMode::Work,
    );
    let mut value = serde_json::to_value(request)?;
    value
        .as_object_mut()
        .expect("intake request object")
        .remove("requested_control_level");
    let decoded: IntakeRequest = serde_json::from_value(value)?;
    assert_eq!(decoded.requested_control_level, RequestedControlLevel::Auto);
    Ok(())
}

#[test]
fn intake_does_not_weaken_required_light_acceptance_policy() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let mut policy = light_workflow_policy();
    policy["light"]["final_acceptance"] = json!("required");
    harness.set_workflow_policy(policy)?;
    let mut request = intake_request(
        "req_intake_required_light_acceptance",
        "idem_intake_required_light_acceptance",
        false,
        Some(harness.counts()?.state_version),
        RequestedMode::Direct,
    );
    request.requested_control_level = RequestedControlLevel::Light;
    request.acceptance_policy = RequiredNullable::some(AcceptancePolicy::PolicyDependent);

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["state"]["effective_control_level"],
        "light"
    );
    assert_eq!(
        response.response_value["state"]["acceptance_policy"],
        "required"
    );
    Ok(())
}

#[test]
fn intake_create_new_invalidates_replaced_task_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (old_task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "intake_create_new_ticket")?;
    let prepared = harness.service.prepare_write(
        prepare_write_request(
            "req_intake_create_new_ticket_prepare",
            "idem_intake_create_new_ticket_prepare",
            Some(2),
            Some(&old_task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");

    let response = harness.service.intake(
        intake_request(
            "req_intake_create_new_ticket",
            "idem_intake_create_new_ticket",
            false,
            Some(3),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "invalidated");
    let reason: String = harness.conn()?.query_row(
        "SELECT invalidation_reason FROM write_tickets
          WHERE project_id = ?1 AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, ticket_id],
        |row| row.get(0),
    )?;
    assert_eq!(reason, "task_closed");
    Ok(())
}

#[test]
fn intake_rejects_incompatible_advisor_control_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_advisor_tracked_control",
        "idem_advisor_tracked_control",
        false,
        Some(0),
        RequestedMode::Advisor,
    );
    request.requested_control_level = RequestedControlLevel::Tracked;

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_rejects_final_acceptance_waiver_for_write_capable_task_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_work_acceptance_waiver",
        "idem_work_acceptance_waiver",
        false,
        Some(0),
        RequestedMode::Work,
    );
    request.acceptance_policy = RequiredNullable::some(AcceptancePolicy::NotRequired);

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_rejects_selected_missing_predecessor_baseline_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let predecessor = harness.service.intake(
        intake_request(
            "req_lineage_missing_baseline_predecessor",
            "idem_lineage_missing_baseline_predecessor",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let predecessor_task_id = predecessor.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("predecessor task id")
        .to_owned();
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_lineage_missing_baseline_followup",
        "idem_lineage_missing_baseline_followup",
        false,
        Some(before.state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id),
        relation: TaskLineageRelation::Continues,
        creation_reason: "Continue only with a compatible baseline.".to_owned(),
        carry_forward: vec![CarryForwardKind::Baseline],
    });

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_rejects_baseline_carry_when_task_and_change_unit_baselines_diverge(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (predecessor_task_id, _) =
        create_task_with_change_unit(&harness, "lineage_divergent_baseline")?;
    set_task_baseline_owner_state(&harness, &predecessor_task_id, "baseline_other")?;
    let before = harness.counts()?;

    let mut request = intake_request(
        "req_lineage_divergent_followup",
        "idem_lineage_divergent_followup",
        false,
        Some(before.state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id),
        relation: TaskLineageRelation::Continues,
        creation_reason: "Carry only an exact compatible baseline.".to_owned(),
        carry_forward: vec![CarryForwardKind::Baseline],
    });
    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_lineage_selectively_carries_scope_and_status_shows_connected_flow(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let predecessor = harness.service.intake(
        intake_request(
            "req_lineage_predecessor",
            "idem_lineage_predecessor",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let predecessor_task_id = predecessor.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("predecessor task id")
        .to_owned();
    let predecessor_criterion_id = predecessor.response_value["state"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .expect("predecessor criterion id")
        .to_owned();

    let mut request = intake_request(
        "req_lineage_followup",
        "idem_lineage_followup",
        false,
        Some(1),
        RequestedMode::Work,
    );
    request.plain_language_request = "Continue the verified predecessor scope.".to_owned();
    request.initial_scope.boundary.clear();
    request.initial_scope.non_goals.clear();
    request.initial_scope.acceptance_criteria.clear();
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id.clone()),
        relation: TaskLineageRelation::Continues,
        creation_reason: "Continue the same bounded outcome.".to_owned(),
        carry_forward: vec![CarryForwardKind::Scope, CarryForwardKind::NonGoals],
    });
    let followup = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(followup.response_value["base"]["response_kind"], "result");
    assert_eq!(
        followup.response_value["state"]["lineage"]["relation"],
        "continues"
    );
    assert_eq!(
        followup.response_value["state"]["lineage"]["predecessor_task_ref"]["record_id"],
        predecessor_task_id
    );
    assert_ne!(
        followup.response_value["state"]["acceptance_criteria"][0]["acceptance_criterion_id"],
        predecessor_criterion_id
    );
    assert_eq!(
        followup.response_value["state"]["non_goals"],
        predecessor.response_value["state"]["non_goals"]
    );

    let followup_task_id = followup.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("followup task id");
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_lineage_status",
                None,
                false,
                None,
                Some(followup_task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                continuity: true,
                ..status_include()
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(
        status.response_value["task_flow"]
            .as_array()
            .expect("task flow")
            .len(),
        2
    );
    Ok(())
}

#[test]
fn intake_rejects_reference_only_carry_without_compatible_predecessor_record(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (predecessor_task_id, _) =
        create_task_with_change_unit(&harness, "lineage_missing_decision_record")?;
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_lineage_missing_decision_record",
        "idem_lineage_missing_decision_record",
        false,
        Some(before.state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id),
        relation: TaskLineageRelation::Continues,
        creation_reason: "Reference an existing durable decision only.".to_owned(),
        carry_forward: vec![CarryForwardKind::UserDecisions],
    });

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_reference_only_carry_points_to_active_continuity_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (predecessor_task_id, predecessor_change_unit_id) =
        create_task_with_change_unit(&harness, "lineage_decision_record")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_lineage_decision_request",
            "idem_lineage_decision_request",
            false,
            Some(2),
            &predecessor_task_id,
            Some(&predecessor_change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&requested.response_value, "user_action_request_ref");
    let recorded = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_lineage_decision_record",
            "idem_lineage_decision_record",
            None,
            &predecessor_task_id,
            &judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let recorded_state_version = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("record state version");
    let mut request = intake_request(
        "req_lineage_decision_followup",
        "idem_lineage_decision_followup",
        false,
        Some(recorded_state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id),
        relation: TaskLineageRelation::Continues,
        creation_reason: "Reference the durable predecessor decision.".to_owned(),
        carry_forward: vec![CarryForwardKind::UserDecisions],
    });

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    let disposition = &response.response_value["state"]["lineage"]["carry_forward"][0];
    assert_eq!(disposition["status"], "reference_only");
    assert_eq!(
        disposition["source_refs"][0]["record_kind"],
        "project_continuity_record"
    );
    Ok(())
}

#[test]
fn intake_carries_artifact_source_refs_as_predecessor_scoped_context() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (predecessor_task_id, predecessor_change_unit_id) =
        create_task_with_change_unit(&harness, "lineage_artifact_source")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &predecessor_task_id,
        &predecessor_change_unit_id,
        2,
        "lineage_artifact_source",
    )?;
    let carried_source = SourceRef::Command(volicord_types::CommandSource {
        invocation_id: "invocation_lineage_artifact_source".to_owned(),
        command_summary: "cargo test lineage source".to_owned(),
        exit_code: 0,
        output_artifact_ref: Some(artifact_ref.clone()).into(),
    });
    set_task_initial_source_refs_owner_state(
        &harness,
        &predecessor_task_id,
        std::slice::from_ref(&carried_source),
    )?;
    let mut request = intake_request(
        "req_lineage_artifact_source_followup",
        "idem_lineage_artifact_source_followup",
        false,
        Some(state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id.clone()),
        relation: TaskLineageRelation::Continues,
        creation_reason: "Carry non-authoritative predecessor source context.".to_owned(),
        carry_forward: vec![CarryForwardKind::SourceRefs],
    });

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let followup_task_id = response.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("follow-up Task id");
    let bounded_context: String = harness.conn()?.query_row(
        "SELECT bounded_context_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, followup_task_id],
        |row| row.get(0),
    )?;
    let bounded_context: Value = serde_json::from_str(&bounded_context)?;
    let carried_artifact =
        &bounded_context["initial_source_refs"][0]["source"]["output_artifact_ref"];
    assert_eq!(
        carried_artifact["artifact_id"],
        artifact_ref.artifact_id.as_str()
    );
    assert_eq!(carried_artifact["task_id"], predecessor_task_id);
    Ok(())
}

#[test]
fn intake_rejects_implements_advice_from_before_advisor_completion() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let predecessor = harness.service.intake(
        intake_request(
            "req_lineage_incomplete_advice",
            "idem_lineage_incomplete_advice",
            false,
            Some(0),
            RequestedMode::Advisor,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let predecessor_task_id = response_record_id(&predecessor.response_value, "task_ref");
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_lineage_incomplete_advice_followup",
        "idem_lineage_incomplete_advice_followup",
        false,
        Some(before.state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id),
        relation: TaskLineageRelation::ImplementsAdviceFrom,
        creation_reason: "Implement only completed advice.".to_owned(),
        carry_forward: Vec::new(),
    });

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_accepts_implements_advice_from_completed_advice_only_task() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (predecessor_task_id, predecessor_change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "lineage_completed_advice",
        RequestedMode::Advisor,
    )?;
    let mut run = record_run_request(
        "req_lineage_completed_advice_run",
        "idem_lineage_completed_advice_run",
        false,
        Some(2),
        &predecessor_task_id,
        &predecessor_change_unit_id,
    );
    run.kind = RunKind::ShapingUpdate;
    run.evidence_updates = vec![supported_evidence_update("Completed advice evidence.")];
    run.close_assessment = Some(close_assessment_with_risks(
        "Completed advice result.",
        Vec::new(),
    ))
    .into();
    let recorded = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    let recorded_state_version = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("recorded advice state version");
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_lineage_completed_advice_close",
            idempotency_key: Some("idem_lineage_completed_advice_close"),
            dry_run: false,
            expected_state_version: Some(recorded_state_version),
            task_id: &predecessor_task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        closed.response_value["state"]["lifecycle"]["result"],
        "advice_only"
    );
    let closed_state_version = closed.response_value["base"]["state_version"]
        .as_u64()
        .expect("closed advice state version");
    let mut request = intake_request(
        "req_lineage_completed_advice_followup",
        "idem_lineage_completed_advice_followup",
        false,
        Some(closed_state_version),
        RequestedMode::Work,
    );
    request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
        predecessor_task_id: TaskId::new(predecessor_task_id.clone()),
        relation: TaskLineageRelation::ImplementsAdviceFrom,
        creation_reason: "Implement the completed advice result.".to_owned(),
        carry_forward: Vec::new(),
    });

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["state"]["lineage"]["relation"],
        "implements_advice_from"
    );
    assert_eq!(
        response.response_value["state"]["lineage"]["predecessor_task_ref"]["record_id"],
        predecessor_task_id
    );
    Ok(())
}

#[test]
fn intake_rejects_applied_carry_categories_with_no_predecessor_material(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let predecessor = harness.service.intake(
        intake_request(
            "req_lineage_empty_material_predecessor",
            "idem_lineage_empty_material_predecessor",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let predecessor_task_id = response_record_id(&predecessor.response_value, "task_ref");
    let raw: String = harness.conn()?.query_row(
        "SELECT shaping_summary_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, predecessor_task_id],
        |row| row.get(0),
    )?;
    let mut shaping: Value = serde_json::from_str(&raw)?;
    shaping["scope_summary"] = Value::Null;
    shaping["non_goals"] = json!([]);
    shaping["initial_context_refs"] = json!([]);
    set_task_owner_json(
        &harness,
        &predecessor_task_id,
        "shaping_summary_json",
        Some(&serde_json::to_string(&shaping)?),
    )?;
    harness.conn()?.execute(
        "DELETE FROM acceptance_criteria WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, predecessor_task_id],
    )?;
    let before = harness.counts()?;

    for (index, kind) in [
        CarryForwardKind::Scope,
        CarryForwardKind::NonGoals,
        CarryForwardKind::ContextRefs,
    ]
    .into_iter()
    .enumerate()
    {
        let mut request = intake_request(
            &format!("req_lineage_empty_material_{index}"),
            &format!("idem_lineage_empty_material_{index}"),
            false,
            Some(before.state_version),
            RequestedMode::Work,
        );
        request.lineage = RequiredNullable::some(volicord_types::TaskLineageInput {
            predecessor_task_id: TaskId::new(predecessor_task_id.clone()),
            relation: TaskLineageRelation::Continues,
            creation_reason: "Carry only material that actually exists.".to_owned(),
            carry_forward: vec![kind],
        });

        let response = harness
            .service
            .intake(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, before);
    }
    Ok(())
}
