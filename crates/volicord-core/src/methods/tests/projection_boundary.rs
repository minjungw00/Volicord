use super::prepare_evidence_capture::{capture_request, create_workspace_bound_task};
use super::*;

fn operation_result_request(
    request_id: &str,
    operation_result_ref: OperationResultRef,
) -> GetOperationResultRequest {
    GetOperationResultRequest {
        envelope: envelope(request_id, None, false, None, None),
        operation_result_ref,
        cursor: RequiredNullable::null(),
    }
}

fn assert_rejected_without_operation_result_chunk(response: &PipelineResponse, code: &str) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(response.response_value["errors"][0]["code"], code);
    assert!(response.response_value.get("chunk_utf8").is_none());
    assert!(response.operation_result_ref.is_none());
}

fn assert_exact_pending_summary(value: &Value, expected_request_id: &str, location: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{location} must be an object, got {value}"));
    let keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from(["next_actor", "status", "user_action_request_id"]),
        "{location} must expose exactly the agent-safe fields: {value}"
    );
    assert_eq!(
        value["user_action_request_id"], expected_request_id,
        "{location} request identity"
    );
    assert_eq!(value["status"], "pending", "{location} pending literal");
    assert_eq!(value["next_actor"], "user", "{location} next actor");
}

fn assert_exact_pending_summary_list(
    response: &Value,
    field: &str,
    expected_request_id: &str,
    location: &str,
) {
    let items = response[field]
        .as_array()
        .unwrap_or_else(|| panic!("{location}.{field} must be an array: {response}"));
    assert_eq!(
        items.len(),
        1,
        "{location}.{field} must contain the one pending request: {items:?}"
    );
    assert_exact_pending_summary(
        &items[0],
        expected_request_id,
        &format!("{location}.{field}[0]"),
    );
}

fn assert_no_user_channel_payload(value: &Value, location: &str) {
    const FORBIDDEN_KEYS: &[&str] = &[
        "answer_path_availability",
        "command",
        "context_summary",
        "fallbacks",
        "form",
        "inbox_item",
        "pending_user_action_inbox_items",
        "pending_user_action_refs",
        "preferred_capture_path",
        "question",
        "resolve_instruction",
        "url",
        "user_action_request",
        "user_action_request_ref",
        "user_channel_availability",
        "verification_code",
    ];
    const FORBIDDEN_TEXT: &[&str] = &[
        "/consent?",
        "token=",
        "Choose the focused test user-action outcome.",
        "A focused test user action needs a user-owned answer.",
        "Record the focused user-owned judgment.",
        "Only this judgment record is resolved.",
        "The Task remains unresolved for this question.",
        "Does the user accept the observed Product Repository change as intentional?",
        "Does the user accept this observed Product Repository change as intentional?",
    ];

    fn visit(value: &Value, path: &str) {
        match value {
            Value::Object(object) => {
                for (key, nested) in object {
                    assert!(
                        !FORBIDDEN_KEYS.contains(&key.as_str()),
                        "{path}.{key} exposed a User Channel-only field: {value}"
                    );
                    visit(nested, &format!("{path}.{key}"));
                }
            }
            Value::Array(items) => {
                for (index, nested) in items.iter().enumerate() {
                    visit(nested, &format!("{path}[{index}]"));
                }
            }
            Value::String(text) => {
                for forbidden in FORBIDDEN_TEXT {
                    assert!(
                        !text.contains(forbidden),
                        "{path} exposed User Channel-only text marker {forbidden:?}: {text:?}"
                    );
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    visit(value, location);
}

fn assert_state_uses_safe_pending_summaries(
    state: &Value,
    expected_request_id: &str,
    location: &str,
) {
    assert!(
        state.get("pending_user_action_refs").is_none(),
        "{location} must not expose pending request refs: {state}"
    );
    assert_exact_pending_summary_list(
        state,
        "pending_user_action_summaries",
        expected_request_id,
        location,
    );
}

fn replace_stored_response(
    harness: &MethodHarness,
    method: MethodName,
    idempotency_key: &str,
    response: &Value,
) -> Result<String, Box<dyn Error>> {
    let response_json = serde_json::to_string(response)?;
    replace_stored_response_json(harness, method, idempotency_key, &response_json)
}

fn replace_stored_response_json(
    harness: &MethodHarness,
    method: MethodName,
    idempotency_key: &str,
    response_json: &str,
) -> Result<String, Box<dyn Error>> {
    let changed = harness.conn()?.execute(
        "UPDATE tool_invocations
            SET response_json = ?4
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![PROJECT_ID, method.as_str(), idempotency_key, response_json],
    )?;
    assert_eq!(changed, 1, "fixture must replace exactly one replay row");
    Ok(response_json.to_owned())
}

fn operation_result_ref_for_stored_bytes(
    mut operation_result_ref: OperationResultRef,
    response_json: &str,
) -> OperationResultRef {
    operation_result_ref.response_sha256 =
        format!("sha256:{:x}", Sha256::digest(response_json.as_bytes()));
    operation_result_ref.response_size_bytes = response_json.len() as u64;
    operation_result_ref
}

#[test]
fn agent_workflow_public_projections_use_only_exact_safe_pending_summaries(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "safe_pending_projection")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_safe_pending_projection",
            "idem_safe_pending_projection",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let expected_request_id = requested
        .response_value
        .get("user_action_request_summary")
        .and_then(|summary| summary.get("user_action_request_id"))
        .and_then(Value::as_str)
        .or_else(|| {
            requested
                .response_value
                .get("user_action_request_ref")
                .and_then(|record_ref| record_ref.get("record_id"))
                .and_then(Value::as_str)
        })
        .expect("request result must identify the pending request")
        .to_owned();
    let state_version = requested.response_value["base"]["state_version"]
        .as_u64()
        .expect("committed request must expose state_version");

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_safe_pending_projection_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let close = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_safe_pending_projection_close",
            idempotency_key: Some("idem_safe_pending_projection_close"),
            dry_run: false,
            expected_state_version: Some(state_version),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let reconcile = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_safe_pending_projection_reconcile",
            "idem_safe_pending_projection_reconcile",
            Some(state_version),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_exact_pending_summary(
        &requested.response_value["user_action_request_summary"],
        &expected_request_id,
        "request_user_action.user_action_request_summary",
    );
    for forbidden in [
        "user_action_request_ref",
        "user_action_request",
        "inbox_item",
    ] {
        assert!(
            requested.response_value.get(forbidden).is_none(),
            "request_user_action must not expose {forbidden}: {}",
            requested.response_value
        );
    }
    assert_state_uses_safe_pending_summaries(
        &requested.response_value["state"],
        &expected_request_id,
        "request_user_action.state",
    );
    assert_no_user_channel_payload(&requested.response_value, "request_user_action");

    assert!(
        status.response_value.get("pending_user_actions").is_none(),
        "status must not expose pending request refs: {}",
        status.response_value
    );
    assert!(
        status
            .response_value
            .get("pending_user_action_inbox_items")
            .is_none(),
        "status must not expose User Channel inbox items: {}",
        status.response_value
    );
    assert_exact_pending_summary_list(
        &status.response_value,
        "pending_user_action_summaries",
        &expected_request_id,
        "status",
    );
    assert_state_uses_safe_pending_summaries(
        &status.response_value["active_task"],
        &expected_request_id,
        "status.active_task",
    );
    assert_no_user_channel_payload(&status.response_value, "status");

    assert!(
        close
            .response_value
            .get("pending_user_action_inbox_items")
            .is_none(),
        "close_task must not expose User Channel inbox items: {}",
        close.response_value
    );
    assert_exact_pending_summary_list(
        &close.response_value,
        "pending_user_action_summaries",
        &expected_request_id,
        "close_task",
    );
    assert_state_uses_safe_pending_summaries(
        &close.response_value["state"],
        &expected_request_id,
        "close_task.state",
    );
    assert_no_user_channel_payload(&close.response_value, "close_task");

    assert!(
        reconcile
            .response_value
            .get("pending_user_action_refs")
            .is_none(),
        "reconcile_changes must not expose pending request refs: {}",
        reconcile.response_value
    );
    assert_exact_pending_summary_list(
        &reconcile.response_value,
        "pending_user_action_summaries",
        &expected_request_id,
        "reconcile_changes",
    );
    assert_state_uses_safe_pending_summaries(
        &reconcile.response_value["state"],
        &expected_request_id,
        "reconcile_changes.state",
    );
    assert_no_user_channel_payload(&reconcile.response_value, "reconcile_changes");
    Ok(())
}

#[test]
fn legacy_full_form_request_result_is_corrupt_before_replay_or_first_page(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "legacy_full_form")?;
    let request = user_action_request(
        "req_legacy_full_form",
        "idem_legacy_full_form",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    let committed = harness.service.request_user_action(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let operation_result_ref = committed
        .operation_result_ref
        .clone()
        .expect("committed agent workflow result must expose an operation-result ref");
    let request_id = committed
        .response_value
        .get("user_action_request_summary")
        .and_then(|summary| summary.get("user_action_request_id"))
        .and_then(Value::as_str)
        .or_else(|| {
            committed
                .response_value
                .get("user_action_request_ref")
                .and_then(|record_ref| record_ref.get("record_id"))
                .and_then(Value::as_str)
        })
        .expect("committed response must identify its request")
        .to_owned();
    let mut legacy = committed.response_value.clone();
    let legacy_inbox_item = legacy.get("inbox_item").cloned().unwrap_or_else(|| {
        json!({
            "user_action_request_id": request_id,
            "request_ref": {
                "record_kind": "user_action_request",
                "record_id": request_id,
                "project_id": PROJECT_ID,
                "task_id": task_id,
                "produced_at_state_version": 3
            },
            "project_id": PROJECT_ID,
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "action_kind": "product_decision",
            "question": "Choose the focused test user-action outcome.",
            "context_summary": "A focused test user action needs a user-owned answer.",
            "form": {
                "form_type": "choice",
                "choices": [
                    {
                        "choice_id": "accept",
                        "label": "Accept",
                        "description": "Record the focused user-owned judgment.",
                        "consequence": "Only this judgment record is resolved.",
                        "is_default": true
                    },
                    {
                        "choice_id": "decline",
                        "label": "Decline",
                        "description": "Record that the focused judgment was not accepted.",
                        "consequence": "The Task remains unresolved for this question.",
                        "is_default": false
                    }
                ],
                "note_allowed": true,
                "note_max_chars": 4000
            },
            "required": true,
            "requirement_status": "required",
            "required_for": ["close_complete"],
            "status": "pending",
            "answer_path_availability": {
                "paths": [{
                    "kind": "cli",
                    "label": "Volicord CLI",
                    "available": true,
                    "status": "available",
                    "capture_basis": "cli_direct_user_channel",
                    "detail": null
                }],
                "recommended_path_kind": "cli",
                "recommended_path_label": "Volicord CLI",
                "recommendation": "Use the User Channel CLI."
            },
            "preferred_capture_path": {
                "kind": "cli",
                "label": "Volicord CLI",
                "available": true,
                "command": "volicord inbox",
                "url": null,
                "capture_basis": "cli_direct_user_channel",
                "expires_at": null,
                "detail": null
            },
            "fallbacks": [],
            "expires_at": null
        })
    });
    legacy
        .as_object_mut()
        .expect("committed response must be an object")
        .insert("inbox_item".to_owned(), legacy_inbox_item);
    let response_json = replace_stored_response(
        &harness,
        MethodName::RequestUserAction,
        "idem_legacy_full_form",
        &legacy,
    )?;
    let operation_result_ref =
        operation_result_ref_for_stored_bytes(operation_result_ref, &response_json);
    let before = harness.counts()?;
    let before_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;

    let replay = harness
        .service
        .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
    let first_page = harness.service.get_operation_result(
        operation_result_request("req_legacy_full_form_page", operation_result_ref),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(replay.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        replay.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert!(!replay.replayed);
    assert!(replay.operation_result_ref.is_none());
    assert_rejected_without_operation_result_chunk(&first_page, "OPERATION_RESULT_UNAVAILABLE");
    assert_eq!(harness.counts()?, before);
    let after_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_floor, before_floor);
    Ok(())
}

#[test]
fn legacy_state_summary_is_corrupt_before_replay_or_first_page() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request = intake_request(
        "req_legacy_state_summary",
        "idem_legacy_state_summary",
        false,
        Some(0),
        RequestedMode::Work,
    );
    let committed = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let operation_result_ref = committed
        .operation_result_ref
        .clone()
        .expect("committed agent workflow result must expose an operation-result ref");
    let mut legacy = committed.response_value.clone();
    let state = legacy["state"]
        .as_object_mut()
        .expect("intake response must expose a StateSummary");
    state.remove("pending_user_action_summaries");
    state.insert("pending_user_action_refs".to_owned(), json!([]));
    let response_json = replace_stored_response(
        &harness,
        MethodName::Intake,
        "idem_legacy_state_summary",
        &legacy,
    )?;
    let operation_result_ref =
        operation_result_ref_for_stored_bytes(operation_result_ref, &response_json);
    let before = harness.counts()?;
    let before_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;

    let replay = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    let first_page = harness.service.get_operation_result(
        operation_result_request("req_legacy_state_summary_page", operation_result_ref),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(replay.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        replay.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert!(!replay.replayed);
    assert!(replay.operation_result_ref.is_none());
    assert_rejected_without_operation_result_chunk(&first_page, "OPERATION_RESULT_UNAVAILABLE");
    assert_eq!(harness.counts()?, before);
    let after_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_floor, before_floor);
    Ok(())
}

#[test]
fn duplicate_json_members_are_corrupt_before_direct_replay_resume_or_first_page(
) -> Result<(), Box<dyn Error>> {
    for variant in ["duplicate_summary", "escaped_nested_member"] {
        let harness = MethodHarness::new()?;
        let suffix = format!("duplicate_stored_{variant}");
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, &suffix)?;
        let idempotency_key = format!("idem_{suffix}");
        let request = user_action_request(
            &format!("req_{suffix}"),
            &idempotency_key,
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        );
        let committed = harness.service.request_user_action(
            request.clone(),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let user_action_request_id = committed.response_value["user_action_request_summary"]
            ["user_action_request_id"]
            .as_str()
            .expect("committed request must identify the pending request")
            .to_owned();
        let operation_result_ref = committed
            .operation_result_ref
            .clone()
            .expect("committed request must expose an operation-result ref");
        let canonical_summary =
            serde_json::to_string(&committed.response_value["user_action_request_summary"])?;
        let canonical_member = format!("\"user_action_request_summary\":{canonical_summary}");
        assert!(
            committed.response_json.contains(&canonical_member),
            "fixture must find the canonical summary member"
        );
        let sentinel = format!("private_replay_sentinel_{variant}");
        let replacement = match variant {
            "duplicate_summary" => format!(
                "\"user_action_request_summary\":{{\"question\":\"{sentinel}\"}},{canonical_member}"
            ),
            "escaped_nested_member" => {
                let safe_next_actor = "\"next_actor\":\"user\"";
                assert!(
                    canonical_summary.contains(safe_next_actor),
                    "fixture must find the nested next_actor member"
                );
                let ambiguous_summary = canonical_summary.replacen(
                    safe_next_actor,
                    &format!("\"next_actor\":\"{sentinel}\",\"\\u006eext_actor\":\"user\""),
                    1,
                );
                format!("\"user_action_request_summary\":{ambiguous_summary}")
            }
            _ => unreachable!(),
        };
        let response_json = committed
            .response_json
            .replacen(&canonical_member, &replacement, 1);
        assert!(response_json.contains(&sentinel));
        let response_json = replace_stored_response_json(
            &harness,
            MethodName::RequestUserAction,
            &idempotency_key,
            &response_json,
        )?;
        let operation_result_ref =
            operation_result_ref_for_stored_bytes(operation_result_ref, &response_json);
        let before = harness.counts()?;
        let before_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;

        let replay = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(
            replay.response_value["base"]["response_kind"], "rejected",
            "variant {variant}"
        );
        assert_eq!(
            replay.response_value["errors"][0]["code"], "PERSISTED_DATA_CORRUPT",
            "variant {variant}"
        );
        assert!(!replay.replayed, "variant {variant}");
        assert!(replay.operation_result_ref.is_none(), "variant {variant}");
        assert!(
            !replay.response_json.contains(&sentinel),
            "variant {variant} must not return ambiguous stored bytes"
        );

        let resumed = harness
            .service
            .resume_user_action_request(
                ProjectId::new(PROJECT_ID),
                UserActionRequestId::new(&user_action_request_id),
                invocation(OperationCategory::AgentWorkflow),
            )?
            .expect("an ineligible origin must return a closed rejection");
        assert_eq!(resumed.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            resumed.response_value["errors"][0]["code"],
            "PERSISTED_DATA_CORRUPT"
        );
        assert!(!resumed.replayed);
        assert!(!resumed.response_json.contains(&sentinel));

        let first_page = harness.service.get_operation_result(
            operation_result_request(&format!("req_{suffix}_page"), operation_result_ref),
            invocation(OperationCategory::Read),
        )?;
        assert_rejected_without_operation_result_chunk(&first_page, "OPERATION_RESULT_UNAVAILABLE");
        assert!(!first_page.response_json.contains(&sentinel));
        assert_eq!(harness.counts()?, before, "variant {variant}");
        let after_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor, "variant {variant}");
    }
    Ok(())
}

#[test]
fn duplicate_members_inside_map_results_are_corrupt_before_replay_or_paging(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "duplicate_map_result")?;
    let idempotency_key = "idem_duplicate_map_result_capture";
    let request = capture_request(
        "req_duplicate_map_result_capture",
        Some(idempotency_key),
        false,
        2,
        (&task_id, &change_unit_id, &criterion_id),
        EvidenceCaptureSpec::VerifiedCommandExecution {
            command_sha256: "a".repeat(64),
            command_label: "cargo test".to_owned(),
            expected_exit_code: RequiredNullable::null(),
        },
    );
    let committed = harness.service.prepare_evidence_capture(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let operation_result_ref = committed
        .operation_result_ref
        .clone()
        .expect("committed capture intent must expose an operation-result ref");
    let canonical_map = "\"expected_outcome\":{\"expected_exit_code\":0}";
    assert!(
        committed.response_json.contains(canonical_map),
        "fixture must find the map-valued expected outcome"
    );
    let sentinel = "private_duplicate_map_value_must_not_escape";
    let ambiguous_map = format!(
        "\"expected_outcome\":{{\"expected_exit_code\":\"{sentinel}\",\"\\u0065xpected_exit_code\":0}}"
    );
    let response_json = committed
        .response_json
        .replacen(canonical_map, &ambiguous_map, 1);
    assert!(response_json.contains(sentinel));
    let response_json = replace_stored_response_json(
        &harness,
        MethodName::PrepareEvidenceCapture,
        idempotency_key,
        &response_json,
    )?;
    let operation_result_ref =
        operation_result_ref_for_stored_bytes(operation_result_ref, &response_json);
    let before = harness.counts()?;
    let before_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;

    let replay = harness.service.prepare_evidence_capture(
        request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace),
    )?;
    assert_eq!(replay.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        replay.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert!(!replay.replayed);
    assert!(replay.operation_result_ref.is_none());
    assert!(!replay.response_json.contains(sentinel));

    let first_page = harness.service.get_operation_result(
        operation_result_request("req_duplicate_map_result_page", operation_result_ref),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_operation_result_chunk(&first_page, "OPERATION_RESULT_UNAVAILABLE");
    assert!(!first_page.response_json.contains(sentinel));
    assert_eq!(harness.counts()?, before);
    let after_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_floor, before_floor);
    Ok(())
}

#[test]
fn stored_commit_envelope_mismatches_are_corrupt() -> Result<(), Box<dyn Error>> {
    for variant in ["response_kind", "effect_kind", "dry_run", "state_version"] {
        let harness = MethodHarness::new()?;
        let suffix = format!("stored_envelope_{variant}");
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, &suffix)?;
        let idempotency_key = format!("idem_{suffix}");
        let request = user_action_request(
            &format!("req_{suffix}"),
            &idempotency_key,
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        );
        let committed = harness.service.request_user_action(
            request.clone(),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let operation_result_ref = committed
            .operation_result_ref
            .clone()
            .expect("committed request must expose an operation-result ref");
        let mut tampered = committed.response_value.clone();
        match variant {
            "response_kind" => tampered["base"]["response_kind"] = json!("rejected"),
            "effect_kind" => tampered["base"]["effect_kind"] = json!("read_only"),
            "dry_run" => tampered["base"]["dry_run"] = json!(true),
            "state_version" => {
                let state_version = tampered["base"]["state_version"]
                    .as_u64()
                    .expect("committed result state version");
                tampered["base"]["state_version"] = json!(state_version + 1);
            }
            _ => unreachable!(),
        }
        let response_json = replace_stored_response(
            &harness,
            MethodName::RequestUserAction,
            &idempotency_key,
            &tampered,
        )?;
        let operation_result_ref =
            operation_result_ref_for_stored_bytes(operation_result_ref, &response_json);
        let before = harness.counts()?;

        let replay = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(replay.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            replay.response_value["errors"][0]["code"],
            "PERSISTED_DATA_CORRUPT"
        );
        assert!(!replay.replayed, "variant {variant}");
        assert!(replay.operation_result_ref.is_none(), "variant {variant}");

        let first_page = harness.service.get_operation_result(
            operation_result_request(&format!("req_{suffix}_page"), operation_result_ref),
            invocation(OperationCategory::Read),
        )?;
        assert_rejected_without_operation_result_chunk(&first_page, "OPERATION_RESULT_UNAVAILABLE");
        assert_eq!(harness.counts()?, before, "variant {variant}");
    }
    Ok(())
}
