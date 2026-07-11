use super::*;

#[test]
fn status_is_read_only_including_dry_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status", None, false, None, None),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], false);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(harness.counts()?, before);

    let dry_run = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_dry",
                Some("idem_status_dry"),
                true,
                Some(0),
                None,
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(dry_run.response_value["base"]["response_kind"], "result");
    assert_eq!(dry_run.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(dry_run.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_renders_effective_write_ticket_expiration_without_mutating_row(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_auth_expired")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_status_future",
        2,
        "2026-06-18T00:00:00.000Z",
        "2999-01-01T00:00:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_auth_expired", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["write_ticket_summary"]["status"],
        "expired"
    );
    assert_eq!(
        response.response_value["active_task"]["write_ticket_summary"]["status"],
        "expired"
    );
    assert_eq!(write_ticket_status(&harness, "wa_status_future")?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_include_evidence_returns_current_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_evidence")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_evidence",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_evidence", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: true,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["evidence_state"],
        "attached"
    );
    assert_eq!(
        response.response_value["summary_card"]["evidence"],
        "attached"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["claim"],
        "Close claim supported."
    );
    assert_eq!(
        response.response_value["active_task"]["evidence_summary"],
        response.response_value["evidence_summary"]
    );
    assert_field_absent(&response.response_value, "current_close_basis");
    assert_field_absent(&response.response_value, "close_state");
    assert_field_absent(&response.response_value, "close_blockers");
    assert_field_absent(&response.response_value, "risk_acceptance_coverage");
    assert_field_absent(&response.response_value["active_task"], "close_state");
    assert_field_absent(&response.response_value["active_task"], "close_blockers");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_evidence_without_close_basis_appears_attached() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "attached_evidence")?;
    let mut request = record_run_request(
        "req_attached_evidence",
        "idem_attached_evidence",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = vec![supported_evidence_update(
        "Attached evidence supports a recorded claim.",
    )];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["evidence_summary"]["evidence_state"],
        "attached"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert!(response.response_value["current_close_basis"].is_null());
    assert_eq!(
        response.response_value["state"]["evidence_summary"]["evidence_state"],
        "attached"
    );
    Ok(())
}

#[test]
fn status_close_include_matches_check_close_blockers() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_close")?;
    record_close_evidence(&harness, &task_id, &change_unit_id, 2, "status_close", true)?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_close", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_status_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(status.response_value["close_state"], "blocked");
    assert_eq!(
        status.response_value["summary_card"]["task"],
        "selected (ready)"
    );
    assert_eq!(
        status.response_value["summary_card"]["close_status"],
        "blocked"
    );
    assert_eq!(
        status.response_value["summary_card"]["evidence"],
        "accepted_for_close"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["evidence_state"],
        "accepted_for_close"
    );
    assert_authority_disclosure(&status.response_value);
    assert_authority_disclosure(&check.response_value);
    assert_eq!(
        check.response_value["summary_card"]["close_status"],
        "blocked"
    );
    assert_eq!(
        check.response_value["summary_card"]["evidence"],
        "accepted_for_close"
    );
    assert_eq!(
        check.response_value["evidence_summary"]["evidence_state"],
        "accepted_for_close"
    );
    assert_eq!(
        check.response_value["summary_card"]["next"],
        check.response_value["blockers"][0]["next_actions"][0]["label"]
    );
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(
        status.response_value["current_close_basis"],
        check.response_value["current_close_basis"]
    );
    assert_eq!(
        status.response_value["close_blockers"],
        check.response_value["blockers"]
    );
    assert_close_blocker(&status.response_value, "missing_final_acceptance");
    let next_actions = status.response_value["next_actions"]
        .as_array()
        .expect("status next_actions should be an array");
    let primary_actions = next_actions
        .iter()
        .filter(|action| action["presentation_role"] == "primary")
        .collect::<Vec<_>>();
    assert_eq!(primary_actions.len(), 1);
    assert!(
        next_actions
            .iter()
            .filter(|action| action["presentation_role"] == "additional")
            .count()
            >= 1
    );
    assert_eq!(
        status.response_value["summary_card"]["next_action"],
        *primary_actions[0]
    );
    let blocker_actions = status.response_value["close_blockers"]
        .as_array()
        .expect("close blockers should be an array")
        .iter()
        .flat_map(|blocker| {
            blocker["next_actions"]
                .as_array()
                .expect("blocker next_actions should be an array")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        blocker_actions
            .iter()
            .filter(|action| action["presentation_role"] == "primary")
            .count(),
        1
    );
    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_ne!(
        status.response_value["guarantee_display"]["level"],
        "detective"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn next_action_dedup_ignores_presentation_role_and_selection_uses_primary_role() {
    for owner_method in [
        MethodName::UpdateScope,
        MethodName::PrepareWrite,
        MethodName::StageArtifact,
        MethodName::RecordRun,
        MethodName::RequestUserJudgment,
        MethodName::CloseTask,
    ] {
        assert_eq!(
            allowed_operation_categories(Some(owner_method)),
            vec![OperationCategory::AgentWorkflow]
        );
    }
    assert_eq!(
        allowed_operation_categories(Some(MethodName::RecordUserJudgment)),
        vec![OperationCategory::UserOnly]
    );
    assert_eq!(
        allowed_operation_categories(Some(MethodName::ReconcileChanges)),
        vec![
            OperationCategory::AgentWorkflow,
            OperationCategory::LocalRecovery
        ]
    );
    assert!(allowed_operation_categories(None).is_empty());

    let primary = NextActionSummary {
        presentation_role: NextActionPresentationRole::Primary,
        action_kind: NextActionKind::RecordRun,
        owner_method: Some(MethodName::RecordRun),
        allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
        label: "Record the current result.".to_owned(),
        blocking_question: None,
        required_refs: Vec::new(),
    };
    let mut additional_duplicate = primary.clone();
    additional_duplicate.presentation_role = NextActionPresentationRole::Additional;

    let deduplicated = super::super::status::unique_next_actions(vec![
        additional_duplicate.clone(),
        primary.clone(),
    ]);
    assert_eq!(deduplicated.len(), 1);

    let distinct_additional = NextActionSummary {
        label: "Additional action.".to_owned(),
        ..additional_duplicate
    };
    let reordered = [distinct_additional, primary.clone()];
    let selected =
        primary_next_action(&reordered, &[]).expect("primary action should be selected by role");
    assert_eq!(selected, &primary);
}

#[test]
fn status_ready_close_uses_empty_blockers_only_after_computation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_ready_empty")?;
    let after_run = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_ready_empty",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_run,
        "status_ready_empty",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_ready_empty", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(status.response_value["close_state"], "ready");
    assert_eq!(status.response_value["close_blockers"], json!([]));
    assert_eq!(status.response_value["active_task"]["close_state"], "ready");
    assert_eq!(
        status.response_value["active_task"]["close_blockers"],
        json!([])
    );
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_include_false_omits_optional_sections_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_flags")?;
    record_close_evidence(&harness, &task_id, &change_unit_id, 2, "status_flags", true)?;
    let before = harness.counts()?;

    let none = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_flags_none", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert!(none.response_value["active_task"].is_null());
    assert!(none.response_value["write_ticket_summary"].is_null());
    assert_field_absent(&none.response_value, "evidence_summary");
    assert_field_absent(&none.response_value, "close_state");
    assert_field_absent(&none.response_value, "current_close_basis");
    assert_field_absent(&none.response_value, "risk_acceptance_coverage");
    assert_field_absent(&none.response_value, "close_blockers");
    assert_field_absent(&none.response_value, "guarantee_display");
    assert_no_close_next_actions(&none.response_value);

    let evidence_only = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_flags_evidence",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: true,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(evidence_only.response_value["active_task"].is_null());
    assert_eq!(
        evidence_only.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_field_absent(&evidence_only.response_value, "close_state");
    assert_field_absent(&evidence_only.response_value, "close_blockers");
    assert_field_absent(&evidence_only.response_value, "guarantee_display");
    assert_no_close_next_actions(&evidence_only.response_value);

    let close_only = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_flags_close", None, false, None, Some(&task_id)),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(close_only.response_value["active_task"].is_null());
    assert_field_absent(&close_only.response_value, "evidence_summary");
    assert_field_absent(&close_only.response_value, "guarantee_display");
    assert_close_blocker(&close_only.response_value, "missing_final_acceptance");

    let guarantees_only = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_flags_guarantee",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(guarantees_only.response_value["active_task"].is_null());
    assert_field_absent(&guarantees_only.response_value, "evidence_summary");
    assert_field_absent(&guarantees_only.response_value, "close_state");
    assert_field_absent(&guarantees_only.response_value, "close_blockers");
    assert_eq!(
        guarantees_only.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_false_does_not_read_corrupt_close_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_close_not_read")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let excluded = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_close_not_read_excluded",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(excluded.response_value["base"]["response_kind"], "result");
    assert_field_absent(&excluded.response_value, "close_state");
    assert_field_absent(&excluded.response_value, "current_close_basis");
    assert_field_absent(&excluded.response_value, "close_blockers");
    assert_field_absent(&excluded.response_value["active_task"], "close_state");
    assert_field_absent(&excluded.response_value["active_task"], "close_blockers");
    assert_no_close_next_actions(&excluded.response_value);
    assert_eq!(harness.counts()?, before);

    let selected = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_close_not_read_selected",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &selected,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn fresh_project_registration_creates_baseline_enforcement_profile() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let profile_json = harness.project_enforcement_profile_json()?;
    assert_eq!(profile_json, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON);

    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let record = store.project_enforcement_profile()?;
    assert_eq!(record.project_id, PROJECT_ID);
    assert_eq!(record.profile.profile_id, "baseline_cooperative");
    assert_eq!(record.profile.enabled_mechanisms.len(), 0);
    Ok(())
}

#[test]
fn status_guarantee_include_false_does_not_read_corrupt_profile() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_profile_skip")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_profile_skip",
        true,
    )?;
    harness.set_project_enforcement_profile_json(corrupt_owner_json())?;
    let before = harness.counts()?;

    let excluded = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_profile_skip_excluded",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(excluded.response_value["base"]["response_kind"], "result");
    assert_field_absent(&excluded.response_value, "guarantee_display");
    assert_field_absent(&excluded.response_value["active_task"], "guarantee_display");
    assert_eq!(harness.counts()?, before);

    let selected = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_profile_skip_selected",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &selected,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_guarantee_include_true_rejects_unsupported_profile_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_project_enforcement_profile_json(
        &json!({
            "profile_id": "baseline_cooperative",
            "guarantee_level": "detective",
            "enabled_mechanisms": [],
            "source": "baseline_scope",
            "status": "active"
        })
        .to_string(),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_profile_detective", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_guarantee_include_true_rejects_missing_profile_fields() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_project_enforcement_profile_json(
        &json!({
            "profile_id": "baseline_cooperative",
            "enabled_mechanisms": [],
            "source": "baseline_scope",
            "status": "active"
        })
        .to_string(),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_profile_missing", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarantee_display_uses_verified_invocation_without_profile_elevation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_guarantee_invocation", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_ne!(
        status.response_value["guarantee_display"]["level"],
        "detective"
    );
    assert!(status.response_value["guarantee_display"]["basis"]
        .as_str()
        .is_some_and(|basis| {
            basis.contains(AGENT_ACTOR_SOURCE)
                && basis.contains("baseline_cooperative")
                && basis.contains("read")
                && basis.contains("enabled mechanisms: none")
                && basis.contains("no stronger enforcement")
        }));
    assert_eq!(
        status.response_value["guarantee_display"]["capability_refs"][0]["record_kind"],
        "agent_connection"
    );
    assert_eq!(
        status.response_value["guarantee_display"]["capability_refs"][0]["record_id"],
        CONNECTION_ID
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_reports_exact_missing_residual_risk_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_risk")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_risk",
        vec![
            residual_risk_input("First status risk."),
            residual_risk_input("Second status risk."),
        ],
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "status_risk",
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_risk", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let coverage = response.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    let projected_ids = coverage
        .iter()
        .map(|item| item["risk_id"].as_str().expect("risk_id").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(projected_ids, risk_ids);
    assert!(coverage.iter().all(|item| item["accepted"] == false));
    assert!(coverage
        .iter()
        .all(|item| item["missing_reason"] == "acceptance_required"));
    assert_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_shows_stale_final_acceptance_blocker_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_stale_final")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_stale_final_old",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "status_stale_final",
    )?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "status_stale_final_new",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_stale_final", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(user_judgment_status(&harness, &final_judgment_id)?, "stale");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    let final_blocker = response.response_value["close_blockers"]
        .as_array()
        .expect("close blockers")
        .iter()
        .find(|blocker| blocker["code"] == "stale_final_acceptance")
        .expect("final acceptance blocker");
    assert!(final_blocker["related_refs"]
        .as_array()
        .expect("related refs")
        .iter()
        .any(|record_ref| record_ref["record_id"] == final_judgment_id));
    assert_eq!(harness.counts()?, before);
    Ok(())
}
