use super::*;

fn get_request(
    request_id: &str,
    operation_result_ref: OperationResultRef,
    cursor: Option<String>,
) -> GetOperationResultRequest {
    GetOperationResultRequest {
        envelope: envelope(request_id, None, false, None, None),
        operation_result_ref,
        cursor: RequiredNullable::new(cursor),
    }
}

fn operation_result_page(response: &PipelineResponse) -> GetOperationResultResult {
    serde_json::from_value(response.response_value.clone())
        .expect("operation-result response should decode")
}

fn assert_rejected_without_chunk(response: &PipelineResponse, code: &str) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(response.response_value["errors"][0]["code"], code);
    assert!(response.response_value.get("chunk_utf8").is_none());
    assert!(response.operation_result_ref.is_none());
}

#[test]
fn operation_result_dry_run_is_rejected_without_preview() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let mut request = get_request(
        "req_operation_result_dry_run",
        OperationResultRef {
            project_id: ProjectId::new(PROJECT_ID),
            source_method: MethodName::Intake,
            source_idempotency_key: IdempotencyKey::new("idem_operation_result_dry_run"),
            committed_state_version: 1,
            response_sha256:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            response_size_bytes: 1,
        },
        None,
    );
    request.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;

    let response = harness
        .service
        .get_operation_result(request, invocation(OperationCategory::Read))?;

    assert_rejected_without_chunk(&response, "VALIDATION_FAILED");
    assert_eq!(response.response_value["base"]["dry_run"], true);
    Ok(())
}

fn read_all_pages(
    harness: &MethodHarness,
    operation_result_ref: &OperationResultRef,
) -> Result<(String, usize), Box<dyn Error>> {
    let mut cursor = None;
    let mut reconstructed = String::new();
    let mut pages = 0usize;
    loop {
        let response = harness.service.get_operation_result(
            get_request(
                &format!("req_get_operation_result_{pages}"),
                operation_result_ref.clone(),
                cursor,
            ),
            invocation(OperationCategory::Read),
        )?;
        assert_typed_result_contract::<GetOperationResultResult>(&response);
        let page = operation_result_page(&response);
        assert_eq!(page.start_offset_bytes, reconstructed.len() as u64);
        assert!(page.chunk_utf8.len() <= MAX_OPERATION_RESULT_PAGE_BYTES);
        reconstructed.push_str(&page.chunk_utf8);
        assert_eq!(page.end_offset_bytes, reconstructed.len() as u64);
        assert!(page.historical);
        assert!(page.current_authority_refresh_required);
        pages += 1;
        if page.complete {
            assert!(page.next_cursor.is_none());
            break;
        }
        cursor = page.next_cursor.into_option();
        assert!(cursor.is_some());
    }
    Ok((reconstructed, pages))
}

#[test]
fn exact_result_reconstructs_unicode_across_pages_and_survives_state_advance(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake_request = intake_request(
        "req_operation_result_intake",
        "idem_operation_result_intake",
        false,
        Some(0),
        RequestedMode::Work,
    );
    let committed = harness.service.intake(
        intake_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replayed = harness
        .service
        .intake(intake_request, invocation(OperationCategory::AgentWorkflow))?;
    assert!(replayed.replayed);
    assert_eq!(
        committed.operation_result_ref,
        replayed.operation_result_ref
    );
    let intake_ref = committed
        .operation_result_ref
        .clone()
        .expect("agent-workflow commit should expose an operation result ref");
    assert_eq!(
        intake_ref.response_sha256,
        format!(
            "sha256:{:x}",
            Sha256::digest(committed.response_json.as_bytes())
        )
    );
    assert_eq!(
        intake_ref.response_size_bytes,
        committed.response_json.len() as u64
    );
    assert!(!committed.response_json.contains("operation_result_ref"));

    let task_id = response_record_id(&committed.response_value, "task_ref");
    let unicode_scope = "결과🙂".repeat(7_000);
    let advanced = harness.service.update_scope(
        update_scope_request(
            "req_operation_result_scope",
            "idem_operation_result_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            &unicode_scope,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let advanced_ref = advanced
        .operation_result_ref
        .clone()
        .expect("large agent-workflow commit should expose an operation result ref");
    assert!(advanced.response_json.len() > MAX_OPERATION_RESULT_PAGE_BYTES);

    let before_reads = harness.counts()?;
    let (old_exact, old_pages) = read_all_pages(&harness, &intake_ref)?;
    assert_eq!(old_exact.as_bytes(), committed.response_json.as_bytes());
    assert_eq!(old_pages, 1);

    let (large_exact, large_pages) = read_all_pages(&harness, &advanced_ref)?;
    assert_eq!(large_exact.as_bytes(), advanced.response_json.as_bytes());
    assert!(large_pages > 1);
    assert_eq!(harness.counts()?, before_reads);
    Ok(())
}

#[test]
fn corrupt_replay_identity_routes_operation_result_to_owner_state_unavailability(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let committed = harness.service.intake(
        intake_request(
            "req_operation_result_identity",
            "idem_operation_result_identity",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let operation_result_ref = committed
        .operation_result_ref
        .expect("committed agent result should have a lookup ref");
    let record_ref = format!(
        "{PROJECT_ID}/{}/{}",
        operation_result_ref.source_method.as_str(),
        operation_result_ref.source_idempotency_key.as_str()
    );
    let before = harness.counts()?;

    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET actor_source = 'not-an-actor'
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            operation_result_ref.source_method.as_str(),
            operation_result_ref.source_idempotency_key.as_str()
        ],
    )?;
    let actor = harness.service.get_operation_result(
        get_request(
            "req_operation_result_corrupt_actor",
            operation_result_ref.clone(),
            None,
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&actor, "PERSISTED_DATA_CORRUPT");
    assert_owner_state_value_rejection(
        &actor,
        "tool_invocations",
        &record_ref,
        "actor_source",
        &harness.runtime_home_path,
    );
    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET actor_source = ?4
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            operation_result_ref.source_method.as_str(),
            operation_result_ref.source_idempotency_key.as_str(),
            AGENT_ACTOR_SOURCE
        ],
    )?;

    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET verification_basis = ''
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            operation_result_ref.source_method.as_str(),
            operation_result_ref.source_idempotency_key.as_str()
        ],
    )?;
    let basis = harness.service.get_operation_result(
        get_request(
            "req_operation_result_corrupt_basis",
            operation_result_ref.clone(),
            None,
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&basis, "PERSISTED_DATA_CORRUPT");
    assert_owner_state_value_rejection(
        &basis,
        "tool_invocations",
        &record_ref,
        "verification_basis",
        &harness.runtime_home_path,
    );
    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET verification_basis = ?4
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            operation_result_ref.source_method.as_str(),
            operation_result_ref.source_idempotency_key.as_str(),
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING
        ],
    )?;

    {
        let conn = harness.conn()?;
        conn.pragma_update(None, "ignore_check_constraints", true)?;
        conn.execute(
            "UPDATE tool_invocations
                SET operation_category = 'unsupported'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            rusqlite::params![
                PROJECT_ID,
                operation_result_ref.source_method.as_str(),
                operation_result_ref.source_idempotency_key.as_str()
            ],
        )?;
        conn.pragma_update(None, "ignore_check_constraints", false)?;
    }
    let category = harness.service.get_operation_result(
        get_request(
            "req_operation_result_corrupt_category",
            operation_result_ref.clone(),
            None,
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&category, "PERSISTED_DATA_CORRUPT");
    assert_owner_state_value_rejection(
        &category,
        "tool_invocations",
        &record_ref,
        "operation_category",
        &harness.runtime_home_path,
    );
    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET operation_category = 'agent_workflow'
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            operation_result_ref.source_method.as_str(),
            operation_result_ref.source_idempotency_key.as_str()
        ],
    )?;

    let restored = harness.service.get_operation_result(
        get_request(
            "req_operation_result_identity_restored",
            operation_result_ref,
            None,
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(restored.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn operation_result_failures_return_no_chunk_and_have_no_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let committed = harness.service.intake(
        intake_request(
            "req_operation_result_failure",
            "idem_operation_result_failure",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let exact_ref = committed
        .operation_result_ref
        .clone()
        .expect("committed agent result should have a lookup ref");
    let task_id = response_record_id(&committed.response_value, "task_ref");
    let large_scope = "한글🙂".repeat(7_000);
    let large_result = harness.service.update_scope(
        update_scope_request(
            "req_operation_large_a",
            "idem_operation_large_a",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            &large_scope,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let large_ref = large_result
        .operation_result_ref
        .clone()
        .expect("first large result should have a lookup ref");
    let second_large_result = harness.service.update_scope(
        update_scope_request(
            "req_operation_large_b",
            "idem_operation_large_b",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            &format!("{large_scope}B"),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_large_ref = second_large_result
        .operation_result_ref
        .clone()
        .expect("second large result should have a lookup ref");
    let before = harness.counts()?;

    let wrong_actor = harness.service.get_operation_result(
        get_request("req_operation_wrong_actor", exact_ref.clone(), None),
        invocation_with_actor(
            ActorSource::agent_connection("connection_other"),
            OperationCategory::Read,
        ),
    )?;
    assert_rejected_without_chunk(&wrong_actor, "INVOCATION_CONTEXT_MISMATCH");

    let mut wrong_project_ref = exact_ref.clone();
    wrong_project_ref.project_id = ProjectId::new("project_other");
    let wrong_project = harness.service.get_operation_result(
        get_request("req_operation_wrong_project", wrong_project_ref, None),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&wrong_project, "INVOCATION_CONTEXT_MISMATCH");

    let mut missing_ref = exact_ref.clone();
    missing_ref.source_idempotency_key = IdempotencyKey::new("idem_operation_result_missing");
    let missing = harness.service.get_operation_result(
        get_request("req_operation_missing", missing_ref, None),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&missing, "OPERATION_RESULT_UNAVAILABLE");

    let malformed_cursor = harness.service.get_operation_result(
        get_request(
            "req_operation_malformed_cursor",
            exact_ref.clone(),
            Some("not-a-cursor".to_owned()),
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&malformed_cursor, "VALIDATION_FAILED");

    let first_page = harness.service.get_operation_result(
        get_request("req_operation_first_page", large_ref.clone(), None),
        invocation(OperationCategory::Read),
    )?;
    let cursor = operation_result_page(&first_page)
        .next_cursor
        .into_option()
        .expect("large result should return a cursor");

    let mut tampered_cursor = cursor.clone();
    let replacement = if tampered_cursor.ends_with('a') {
        'b'
    } else {
        'a'
    };
    tampered_cursor.pop();
    tampered_cursor.push(replacement);
    let tampered = harness.service.get_operation_result(
        get_request(
            "req_operation_tampered_cursor",
            large_ref.clone(),
            Some(tampered_cursor),
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&tampered, "OPERATION_RESULT_UNAVAILABLE");

    let cross_result = harness.service.get_operation_result(
        get_request(
            "req_operation_cross_cursor",
            second_large_ref,
            Some(cursor.clone()),
        ),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&cross_result, "OPERATION_RESULT_UNAVAILABLE");

    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET response_json = response_json || ' '
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            large_ref.source_method.as_str(),
            large_ref.source_idempotency_key.as_str()
        ],
    )?;
    let corrupt = harness.service.get_operation_result(
        get_request("req_operation_corrupt", large_ref.clone(), Some(cursor)),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&corrupt, "OPERATION_RESULT_UNAVAILABLE");

    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET response_json = 'not-json'
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            large_ref.source_method.as_str(),
            large_ref.source_idempotency_key.as_str()
        ],
    )?;
    let corrupt_owner_state = harness.service.get_operation_result(
        get_request("req_operation_corrupt_owner_state", large_ref.clone(), None),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&corrupt_owner_state, "PERSISTED_DATA_CORRUPT");

    harness.conn()?.execute(
        "UPDATE tool_invocations
            SET response_json = ?4,
                actor_source = 'local_user',
                operation_category = 'user_only'
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3",
        rusqlite::params![
            PROJECT_ID,
            large_ref.source_method.as_str(),
            large_ref.source_idempotency_key.as_str(),
            large_result.response_json
        ],
    )?;
    let user_only = harness.service.get_operation_result(
        get_request("req_operation_user_only", large_ref, None),
        invocation(OperationCategory::Read),
    )?;
    assert_rejected_without_chunk(&user_only, "OPERATION_RESULT_UNAVAILABLE");

    assert_eq!(harness.counts()?, before);
    Ok(())
}
