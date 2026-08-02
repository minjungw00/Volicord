use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApprovalConsumerScenarioKind {
    ApprovalCurrent,
    ApprovalNotRequired,
    ApprovalNewlyRequired,
    ApprovalResolutionStale,
    ApprovalScopeChanged,
    TicketExpired,
    TicketConsumed,
    TicketRevoked,
    ExactlyOneCompatible,
    MultipleCompatible,
}

#[derive(Debug, Clone, Copy)]
struct ApprovalRecordFacts {
    name: &'static str,
    operation: &'static str,
    expires_at: Option<&'static str>,
    included_in_ticket_basis: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersistedTicketLifecycle {
    Active,
    Consumed,
    Revoked,
}

#[derive(Debug, Clone)]
struct ApprovalConsumerScenario {
    kind: ApprovalConsumerScenarioKind,
    name: &'static str,
    requested_control_level: RequestedControlLevel,
    sensitive_categories: &'static [&'static str],
    approval_records: Vec<ApprovalRecordFacts>,
    idle_expires_at: &'static str,
    lifecycle: PersistedTicketLifecycle,
    compatible_ticket_count: usize,
}

struct ApprovalConsumerFixture {
    harness: MethodHarness,
    task_id: String,
    change_unit_id: String,
    ticket_ids: Vec<String>,
}

fn approval_consumer_scenarios() -> Vec<ApprovalConsumerScenario> {
    let matching = |name, expires_at, included_in_ticket_basis| ApprovalRecordFacts {
        name,
        operation: "local_sensitive_step",
        expires_at,
        included_in_ticket_basis,
    };
    vec![
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::ApprovalCurrent,
            name: "approval_current",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &["network"],
            approval_records: vec![matching("basis", None, true)],
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::ApprovalNotRequired,
            name: "approval_not_required",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::ApprovalNewlyRequired,
            name: "approval_newly_required",
            requested_control_level: RequestedControlLevel::Sensitive,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::ApprovalResolutionStale,
            name: "approval_resolution_stale",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &["network"],
            approval_records: vec![
                matching("basis", Some("2026-06-18T00:05:00Z"), true),
                matching("replacement", None, false),
            ],
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::ApprovalScopeChanged,
            name: "approval_scope_changed",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &["network"],
            approval_records: vec![ApprovalRecordFacts {
                name: "basis",
                operation: "different_sensitive_step",
                expires_at: None,
                included_in_ticket_basis: true,
            }],
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::TicketExpired,
            name: "ticket_expired",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:05:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::TicketConsumed,
            name: "ticket_consumed",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Consumed,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::TicketRevoked,
            name: "ticket_revoked",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Revoked,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::ExactlyOneCompatible,
            name: "exactly_one_compatible",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 1,
        },
        ApprovalConsumerScenario {
            kind: ApprovalConsumerScenarioKind::MultipleCompatible,
            name: "multiple_compatible",
            requested_control_level: RequestedControlLevel::Auto,
            sensitive_categories: &[],
            approval_records: Vec::new(),
            idle_expires_at: "2026-06-18T00:30:00Z",
            lifecycle: PersistedTicketLifecycle::Active,
            compatible_ticket_count: 2,
        },
    ]
}

fn setup_approval_consumer_fixture(
    scenario: &ApprovalConsumerScenario,
) -> Result<ApprovalConsumerFixture, Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at(DEFAULT_METHOD_TEST_CLOCK);
    harness.use_generator_and_clock(
        CountingDurableIdGenerator::new((0..128).map(|index| format!("approval-consumer-{index}"))),
        clock.clone(),
    );
    enable_record_run_capabilities(&harness)?;

    let mut intake = intake_request(
        &format!("req_{}_task", scenario.name),
        &format!("idem_{}_task", scenario.name),
        false,
        Some(harness.counts()?.state_version),
        RequestedMode::Work,
    );
    intake.requested_control_level = scenario.requested_control_level;
    let intake = harness
        .service
        .intake(intake, invocation(OperationCategory::AgentWorkflow))?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scope = harness.service.update_scope(
        update_scope_request(
            &format!("req_{}_scope", scenario.name),
            &format!("idem_{}_scope", scenario.name),
            false,
            Some(harness.counts()?.state_version),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Approval consumer conformance scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = response_record_id(&scope.response_value, "change_unit_ref");
    advance_work_task_for_test(&harness, scenario.name, &task_id, &change_unit_id)?;

    let mut approval_basis_refs = Vec::new();
    for approval_facts in &scenario.approval_records {
        let mut request = user_action_request(
            &format!("req_{}_approval_{}", scenario.name, approval_facts.name),
            &format!("idem_{}_approval_{}", scenario.name, approval_facts.name),
            false,
            Some(harness.counts()?.state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        );
        let expires_at = approval_facts
            .expires_at
            .map(UtcTimestamp::parse)
            .transpose()?;
        request.expires_at = expires_at.clone().into();
        let volicord_types::schema::UserActionDraft::Choice(choice) = &mut request.action else {
            unreachable!("sensitive approval fixture is choice-shaped")
        };
        let sensitive_scope = choice
            .sensitive_action_scope
            .as_mut()
            .expect("sensitive approval fixture has an exact action scope");
        sensitive_scope.action_kind = approval_facts.operation.to_owned();
        sensitive_scope.intended_paths = vec!["src/export.rs".to_owned()];
        sensitive_scope.sensitive_categories = scenario
            .sensitive_categories
            .iter()
            .map(|category| (*category).to_owned())
            .collect();
        sensitive_scope.expires_at = expires_at.into();

        let requested = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
        let request_id = response_record_id(&requested.response_value, "user_action_request_ref");
        let resolved = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!(
                    "req_{}_approval_{}_resolve",
                    scenario.name, approval_facts.name
                ),
                &format!(
                    "submission_{}_approval_{}",
                    scenario.name, approval_facts.name
                ),
                None,
                &task_id,
                &request_id,
                "accept",
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        if approval_facts.included_in_ticket_basis {
            let resolution_ref = &resolved.response_value["user_action_resolution_ref"];
            approval_basis_refs.push(UserActionResolutionRef::new(
                ProjectId::new(PROJECT_ID),
                TaskId::new(&task_id),
                UserActionResolutionId::new(
                    resolution_ref["record_id"]
                        .as_str()
                        .expect("resolution ref identifies the stored resolution"),
                ),
                resolution_ref["produced_at_state_version"]
                    .as_u64()
                    .expect("resolution ref carries projection freshness"),
            ));
        }
    }

    let basis_state_version = harness.counts()?.state_version;
    let mut ticket_ids = Vec::new();
    for index in 0..scenario.compatible_ticket_count {
        let suffix = char::from(b'a' + u8::try_from(index)?);
        let write_ticket_id = format!("ticket_{}_{}", scenario.name, suffix);
        insert_active_write_ticket_with_scope(
            &harness,
            WriteTicketScopeFixture {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                write_ticket_id: &write_ticket_id,
                basis_state_version,
                created_at: DEFAULT_METHOD_TEST_CLOCK,
                expires_at: scenario.idle_expires_at,
                intended_operation: "local_sensitive_step",
                intended_paths: &["src/export.rs"],
                sensitive_categories: scenario.sensitive_categories,
                approval_basis_refs: approval_basis_refs.clone(),
            },
        )?;
        ticket_ids.push(write_ticket_id);
    }

    match scenario.lifecycle {
        PersistedTicketLifecycle::Active => {}
        PersistedTicketLifecycle::Consumed => {
            let consumed = harness.service.record_run(
                product_write_record_run_request(
                    &format!("req_{}_consume_fixture", scenario.name),
                    &format!("idem_{}_consume_fixture", scenario.name),
                    harness.counts()?.state_version,
                    &task_id,
                    &change_unit_id,
                    &ticket_ids[0],
                    "run_consumed_fixture",
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(consumed.response_value["base"]["response_kind"], "result");
        }
        PersistedTicketLifecycle::Revoked => {
            harness.mutation_store()?.revoke_write_ticket_fixture(
                &ticket_ids[0],
                WriteTicketInvalidationReason::ExplicitRevoke,
                &UtcTimestamp::parse("2026-06-18T00:01:00Z")?,
            )?;
        }
    }
    clock.advance(Duration::minutes(10));

    Ok(ApprovalConsumerFixture {
        harness,
        task_id,
        change_unit_id,
        ticket_ids,
    })
}

fn expected_summary(
    kind: ApprovalConsumerScenarioKind,
) -> (WriteTicketStatus, Option<WriteTicketInvalidationReason>) {
    match kind {
        ApprovalConsumerScenarioKind::ApprovalCurrent
        | ApprovalConsumerScenarioKind::ApprovalNotRequired
        | ApprovalConsumerScenarioKind::ExactlyOneCompatible
        | ApprovalConsumerScenarioKind::MultipleCompatible => (WriteTicketStatus::Active, None),
        ApprovalConsumerScenarioKind::ApprovalNewlyRequired
        | ApprovalConsumerScenarioKind::ApprovalResolutionStale
        | ApprovalConsumerScenarioKind::ApprovalScopeChanged => (
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ApprovalBasisChanged),
        ),
        ApprovalConsumerScenarioKind::TicketExpired => (
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::IdleTimeout),
        ),
        ApprovalConsumerScenarioKind::TicketConsumed => (WriteTicketStatus::Consumed, None),
        ApprovalConsumerScenarioKind::TicketRevoked => (
            WriteTicketStatus::Revoked,
            Some(WriteTicketInvalidationReason::ExplicitRevoke),
        ),
    }
}

fn summary_status_value(status: WriteTicketStatus) -> &'static str {
    match status {
        WriteTicketStatus::Active => "active",
        WriteTicketStatus::Consumed => "consumed",
        WriteTicketStatus::Invalidated => "invalidated",
        WriteTicketStatus::Revoked => "revoked",
    }
}

fn open_write_ticket_blocker_ids(response: &Value) -> Vec<String> {
    let mut ids = response["blockers"]
        .as_array()
        .expect("check_close blockers are an array")
        .iter()
        .filter(|blocker| blocker["code"] == "open_write_ticket")
        .flat_map(|blocker| {
            blocker["related_refs"]
                .as_array()
                .expect("close blocker related refs are an array")
        })
        .filter(|reference| reference["record_kind"] == "write_ticket")
        .filter_map(|reference| reference["record_id"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn assert_status_and_close_consumers(
    scenario: &ApprovalConsumerScenario,
    fixture: &ApprovalConsumerFixture,
) -> Result<(), Box<dyn Error>> {
    let before = fixture.harness.counts()?;
    let status = fixture.harness.service.status(
        StatusRequest {
            envelope: envelope(
                &format!("req_{}_status", scenario.name),
                None,
                false,
                None,
                Some(&fixture.task_id),
            ),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let (expected_status, expected_invalidation) = expected_summary(scenario.kind);
    let summary = &status.response_value["write_ticket_summary"];
    assert_eq!(
        summary["status"],
        summary_status_value(expected_status),
        "status summary for {}",
        scenario.name
    );
    assert_eq!(
        summary["invalidation_reason"],
        expected_invalidation
            .map(|reason| Value::String(reason.as_str().to_owned()))
            .unwrap_or(Value::Null),
        "status invalidation for {}",
        scenario.name
    );
    assert_eq!(
        summary["write_ticket_ref"]["record_id"], fixture.ticket_ids[0],
        "display selection for {}",
        scenario.name
    );
    assert_eq!(
        fixture
            .harness
            .store()?
            .write_tickets_for_task(&TaskId::new(&fixture.task_id))?
            .len(),
        scenario.compatible_ticket_count,
        "all persisted candidates remain inspectable for {}",
        scenario.name
    );

    let close = fixture.harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: &format!("req_{}_check_close", scenario.name),
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &fixture.task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    let expected_open_ids = if expected_status == WriteTicketStatus::Active {
        fixture.ticket_ids.clone()
    } else {
        Vec::new()
    };
    assert_eq!(
        open_write_ticket_blocker_ids(&close.response_value),
        expected_open_ids,
        "close-readiness Write Ticket blockers for {}",
        scenario.name
    );
    if scenario.kind == ApprovalConsumerScenarioKind::ApprovalNewlyRequired {
        assert_close_blocker(&close.response_value, "missing_sensitive_action_basis");
    }
    assert_eq!(
        fixture.harness.counts()?,
        before,
        "status and check_close remain read-only for {}",
        scenario.name
    );
    Ok(())
}

fn assert_prepare_write_consumer(
    scenario: &ApprovalConsumerScenario,
    fixture: &ApprovalConsumerFixture,
) -> Result<(), Box<dyn Error>> {
    let before = fixture.harness.counts()?;
    let mut request = prepare_write_request(
        &format!("req_{}_prepare", scenario.name),
        &format!("idem_{}_prepare", scenario.name),
        Some(before.state_version),
        Some(&fixture.task_id),
        Some(&fixture.change_unit_id),
    );
    request.sensitive_categories = scenario
        .sensitive_categories
        .iter()
        .map(|category| (*category).to_owned())
        .collect();
    let response = fixture
        .harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    match scenario.kind {
        ApprovalConsumerScenarioKind::ApprovalCurrent
        | ApprovalConsumerScenarioKind::ApprovalNotRequired
        | ApprovalConsumerScenarioKind::ExactlyOneCompatible => {
            assert_eq!(response.response_value["decision"], "allowed");
            assert_eq!(response.response_value["write_ticket_effect"], "reused");
            assert_eq!(
                response_record_id(&response.response_value, "write_ticket_ref"),
                fixture.ticket_ids[0]
            );
        }
        ApprovalConsumerScenarioKind::ApprovalNewlyRequired
        | ApprovalConsumerScenarioKind::ApprovalScopeChanged => {
            assert_eq!(response.response_value["decision"], "approval_required");
            assert_eq!(response.response_value["write_ticket_effect"], "none");
            assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
        }
        ApprovalConsumerScenarioKind::ApprovalResolutionStale => {
            assert_eq!(response.response_value["decision"], "allowed");
            assert_eq!(response.response_value["write_ticket_effect"], "issued");
            assert_ne!(
                response_record_id(&response.response_value, "write_ticket_ref"),
                fixture.ticket_ids[0]
            );
        }
        ApprovalConsumerScenarioKind::TicketExpired
        | ApprovalConsumerScenarioKind::TicketConsumed
        | ApprovalConsumerScenarioKind::TicketRevoked => {
            assert_eq!(response.response_value["decision"], "allowed");
            assert_eq!(response.response_value["write_ticket_effect"], "issued");
            assert_ne!(
                response_record_id(&response.response_value, "write_ticket_ref"),
                fixture.ticket_ids[0]
            );
        }
        ApprovalConsumerScenarioKind::MultipleCompatible => {
            assert_eq!(response.response_value["decision"], "blocked");
            assert_eq!(response.response_value["write_ticket_effect"], "none");
            assert!(response.response_value["write_ticket_ref"].is_null());
            let related_ids = response.response_value["write_decision_reasons"][0]["related_refs"]
                .as_array()
                .expect("ambiguity reason carries candidate refs")
                .iter()
                .map(|reference| {
                    reference["record_id"]
                        .as_str()
                        .expect("candidate ref identifies a Write Ticket")
                        .to_owned()
                })
                .collect::<Vec<_>>();
            assert_eq!(related_ids, fixture.ticket_ids);
        }
    }

    let after = fixture.harness.counts()?;
    assert_eq!(
        after.state_version,
        before.state_version + 1,
        "committed Prepare Write state version for {}",
        scenario.name
    );
    let expected_insertions = matches!(
        scenario.kind,
        ApprovalConsumerScenarioKind::ApprovalResolutionStale
            | ApprovalConsumerScenarioKind::TicketExpired
            | ApprovalConsumerScenarioKind::TicketConsumed
            | ApprovalConsumerScenarioKind::TicketRevoked
    );
    assert_eq!(
        after.write_tickets,
        before.write_tickets + u64::from(expected_insertions),
        "Prepare Write ticket insertion count for {}",
        scenario.name
    );
    for ticket_id in &fixture.ticket_ids {
        let expected_status = match scenario.kind {
            ApprovalConsumerScenarioKind::ApprovalNewlyRequired
            | ApprovalConsumerScenarioKind::ApprovalResolutionStale
            | ApprovalConsumerScenarioKind::ApprovalScopeChanged => "invalidated",
            ApprovalConsumerScenarioKind::TicketConsumed => "consumed",
            ApprovalConsumerScenarioKind::TicketRevoked => "revoked",
            ApprovalConsumerScenarioKind::ApprovalCurrent
            | ApprovalConsumerScenarioKind::ApprovalNotRequired
            | ApprovalConsumerScenarioKind::TicketExpired
            | ApprovalConsumerScenarioKind::ExactlyOneCompatible
            | ApprovalConsumerScenarioKind::MultipleCompatible => "active",
        };
        assert_eq!(
            write_ticket_status(&fixture.harness, ticket_id)?,
            expected_status,
            "persisted Prepare Write effect for {}",
            scenario.name
        );
    }
    Ok(())
}

fn assert_record_run_consumer(scenario: &ApprovalConsumerScenario) -> Result<(), Box<dyn Error>> {
    let fixture = setup_approval_consumer_fixture(scenario)?;
    let before = fixture.harness.counts()?;
    let mut request = product_write_record_run_request(
        &format!("req_{}_record", scenario.name),
        &format!("idem_{}_record", scenario.name),
        before.state_version,
        &fixture.task_id,
        &fixture.change_unit_id,
        &fixture.ticket_ids[0],
        &format!("run_{}_record", scenario.name),
    );
    request.observed_changes.sensitive_categories = scenario
        .sensitive_categories
        .iter()
        .map(|category| (*category).to_owned())
        .collect();
    let response = fixture
        .harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    match scenario.kind {
        ApprovalConsumerScenarioKind::ApprovalCurrent
        | ApprovalConsumerScenarioKind::ApprovalNotRequired
        | ApprovalConsumerScenarioKind::ExactlyOneCompatible
        | ApprovalConsumerScenarioKind::MultipleCompatible => {
            assert_eq!(
                response.response_value["base"]["response_kind"], "result",
                "Record Run admission for {}: {:#}",
                scenario.name, response.response_value
            );
            assert_eq!(
                write_ticket_status(&fixture.harness, &fixture.ticket_ids[0])?,
                "consumed"
            );
            for ticket_id in fixture.ticket_ids.iter().skip(1) {
                assert_eq!(
                    write_ticket_status(&fixture.harness, ticket_id)?,
                    "active",
                    "Record Run consumes only the explicitly named candidate"
                );
            }
            let after = fixture.harness.counts()?;
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
        }
        kind => {
            let expected_reason = match kind {
                ApprovalConsumerScenarioKind::ApprovalNewlyRequired
                | ApprovalConsumerScenarioKind::ApprovalResolutionStale
                | ApprovalConsumerScenarioKind::ApprovalScopeChanged => "approval_basis_changed",
                ApprovalConsumerScenarioKind::TicketExpired => "idle_timeout",
                ApprovalConsumerScenarioKind::TicketConsumed => "consumed",
                ApprovalConsumerScenarioKind::TicketRevoked => "revoked",
                ApprovalConsumerScenarioKind::ApprovalCurrent
                | ApprovalConsumerScenarioKind::ApprovalNotRequired
                | ApprovalConsumerScenarioKind::ExactlyOneCompatible
                | ApprovalConsumerScenarioKind::MultipleCompatible => {
                    unreachable!("admitted cases are handled above")
                }
            };
            assert_write_ticket_invalid_reason(&response, expected_reason);
            assert_eq!(
                fixture.harness.counts()?,
                before,
                "rejected Record Run has no effect for {}",
                scenario.name
            );
        }
    }
    Ok(())
}

#[test]
fn persisted_approval_scenarios_conform_across_actual_ticket_consumers(
) -> Result<(), Box<dyn Error>> {
    for scenario in approval_consumer_scenarios() {
        let fixture = setup_approval_consumer_fixture(&scenario)?;
        assert_status_and_close_consumers(&scenario, &fixture)?;
        assert_prepare_write_consumer(&scenario, &fixture)?;
        assert_record_run_consumer(&scenario)?;
    }
    Ok(())
}
