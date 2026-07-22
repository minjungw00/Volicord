use super::*;

#[test]
fn initialization_batches_are_rejected_without_selecting_any_production_profile(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in production_profiles().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-initialization-batch-{index}"))?;
        let connection_adapter = adapter(&fixture)?;
        let mut state = ConnectionState::default();
        let batch = json!([
            initialize_request_for_protocol(
                1,
                json!({}),
                "revision-batch-client",
                "1.0",
                profile.revision().as_str(),
            ),
            initialized_notification(),
            request(2, "tools/list", json!({})),
        ]);

        let response = handle_json_rpc_message(&connection_adapter, &mut state, batch.clone())?
            .expect("initialization batch should return one rejection");
        assert_eq!(response["error"]["code"], -32600);
        assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
        assert!(state.mcp_session.is_none());

        let mut output = Vec::new();
        run_stdio(
            connection_adapter,
            BufReader::new(Cursor::new(json_lines(&[batch])?)),
            &mut output,
        )?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["error"]["code"], -32600);

        let runtime_session_id = latest_runtime_session_id(&fixture)?;
        let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
            .expect("initialization-batch runtime session");
        assert!(runtime.attempted_client_name.is_none());
        assert!(runtime.attempted_client_version.is_none());
        assert!(runtime.requested_protocol_version.is_none());
        assert!(runtime.selected_protocol_version.is_none());
        assert!(runtime.negotiated_protocol_version.is_none());
        assert!(runtime.initialize_completed_at.is_none());
        assert!(runtime.initialized_notification_at.is_none());
        assert!(runtime.tools_list_observed_at.is_none());
        assert!(diagnostic_occurrences_for_runtime_session(
            fixture.runtime_home_path(),
            &runtime_session_id,
        )?
        .iter()
        .any(|finding| {
            finding.data().code().as_str() == "mcp.lifecycle.initialization_batch_forbidden"
        }));
    }
    Ok(())
}

#[test]
fn ready_2025_03_26_session_batches_operations_in_order_and_omits_notifications(
) -> Result<(), Box<dyn Error>> {
    let batching_revisions = production_profiles()
        .filter(|profile| profile.messages().json_rpc_batching() == JsonRpcBatching::Allowed)
        .map(|profile| profile.revision().as_str())
        .collect::<Vec<_>>();
    assert_eq!(batching_revisions, vec!["2025-03-26"]);

    let fixture = CoreFixture::new("mcp-ready-operation-batch")?;
    let operation_batch = json!([
        request(20, "ping", json!({})),
        notification(
            "notifications/progress",
            json!({"progressToken": "fixture", "progress": 1})
        ),
        request(21, "tools/list", json!({})),
    ]);
    let mut output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[
            initialize_request_for_protocol(
                1,
                json!({}),
                "operation-batch-client",
                "1.0",
                "2025-03-26",
            ),
            initialized_notification(),
            operation_batch,
        ])?)),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-03-26");
    let batch_responses = responses[1]
        .as_array()
        .expect("ready operation batch response array");
    assert_eq!(
        batch_responses
            .iter()
            .map(|response| response["id"].as_u64().expect("response id"))
            .collect::<Vec<_>>(),
        vec![20, 21]
    );
    assert_eq!(batch_responses[0]["result"], json!({}));
    assert!(batch_responses[1]["result"]["tools"].is_array());

    let runtime_session_id = latest_runtime_session_id(&fixture)?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("valid operation-batch runtime session");
    assert_eq!(
        runtime.selected_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert_eq!(
        runtime.negotiated_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert!(runtime.initialized_notification_at.is_some());
    assert!(runtime.tools_list_observed_at.is_some());
    assert!(runtime.graceful_close_at.is_some());
    assert!(diagnostic_occurrences_for_runtime_session(
        fixture.runtime_home_path(),
        &runtime_session_id,
    )?
    .is_empty());
    Ok(())
}

#[test]
fn non_batching_profiles_reject_ready_operation_batches_without_tool_observations(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in production_profiles()
        .filter(|profile| profile.messages().json_rpc_batching() == JsonRpcBatching::Disallowed)
        .enumerate()
    {
        let fixture = CoreFixture::new(&format!("mcp-non-batching-profile-{index}"))?;
        let revision = profile.revision().as_str();
        let mut output = Vec::new();
        run_stdio(
            adapter(&fixture)?,
            BufReader::new(Cursor::new(json_lines(&[
                initialize_request_for_protocol(
                    1,
                    json!({}),
                    "non-batching-client",
                    "1.0",
                    revision,
                ),
                initialized_notification(),
                json!([
                    request(2, "ping", json!({})),
                    request(3, "tools/list", json!({})),
                ]),
            ])?)),
            &mut output,
        )?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2, "{revision}");
        assert_eq!(responses[0]["result"]["protocolVersion"], revision);
        assert_eq!(responses[1]["error"]["code"], -32600);

        let runtime_session_id = latest_runtime_session_id(&fixture)?;
        let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
            .expect("non-batching profile runtime session");
        assert_eq!(runtime.selected_protocol_version.as_deref(), Some(revision));
        assert_eq!(
            runtime.negotiated_protocol_version.as_deref(),
            Some(revision)
        );
        assert!(runtime.initialized_notification_at.is_some());
        assert!(runtime.tools_list_observed_at.is_none());
        assert!(runtime.graceful_close_at.is_some());
        let findings = diagnostic_occurrences_for_runtime_session(
            fixture.runtime_home_path(),
            &runtime_session_id,
        )?;
        assert!(findings
            .iter()
            .any(|finding| finding.data().code().as_str() == "mcp.json_rpc.invalid_request"));
        assert!(!findings.iter().any(|finding| {
            finding.data().code().as_str() == "mcp.lifecycle.initialization_batch_forbidden"
        }));
    }
    Ok(())
}

#[test]
fn operation_batches_before_ready_do_not_advance_lifecycle() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-operation-batch-before-ready")?;
    let connection_adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();

    let rejected_before_initialize = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        json!([
            request(90, "ping", json!({})),
            request(91, "tools/list", json!({})),
        ]),
    )?
    .expect("pre-initialize operation batch rejection");
    assert_eq!(rejected_before_initialize["error"]["code"], -32600);
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
    assert!(state.mcp_session.is_none());

    let initialize = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        initialize_request_for_protocol(
            1,
            json!({}),
            "pre-ready-batch-client",
            "1.0",
            "2025-03-26",
        ),
    )?
    .expect("standalone initialize response");
    assert_eq!(initialize["result"]["protocolVersion"], "2025-03-26");
    let selected_before = state.mcp_session.clone();

    let rejected = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        json!([
            request(2, "ping", json!({})),
            request(3, "tools/list", json!({})),
        ]),
    )?
    .expect("pre-ready operation batch rejection");
    assert_eq!(rejected["error"]["code"], -32600);
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialized);
    assert_eq!(state.mcp_session, selected_before);

    let rejected_initialized_batch = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        json!([initialized_notification()]),
    )?
    .expect("initialized notification batch rejection");
    assert_eq!(rejected_initialized_batch["error"]["code"], -32600);
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialized);
    assert_eq!(state.mcp_session, selected_before);

    assert!(
        handle_json_rpc_message(&connection_adapter, &mut state, initialized_notification(),)?
            .is_none()
    );
    assert_eq!(state.phase, ConnectionPhase::Ready);
    Ok(())
}

#[test]
fn ready_batch_with_initialize_is_rejected_before_a_sibling_operation() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-ready-batch-duplicate-initialize")?;
    let mut output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[
            initialize_request_for_protocol(
                1,
                json!({}),
                "ready-batch-client",
                "1.0",
                "2025-03-26",
            ),
            initialized_notification(),
            json!([
                request(2, "tools/list", json!({})),
                initialize_request_for_protocol(
                    3,
                    json!({}),
                    "ready-batch-client",
                    "1.0",
                    "2025-03-26",
                ),
            ]),
            request(4, "ping", json!({})),
        ])?)),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(responses[1]["error"]["code"], -32600);
    assert_eq!(responses[2]["id"], 4);
    assert_eq!(responses[2]["result"], json!({}));

    let runtime_session_id = latest_runtime_session_id(&fixture)?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("ready duplicate-initialize batch runtime session");
    assert_eq!(
        runtime.selected_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert_eq!(
        runtime.negotiated_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert!(runtime.tools_list_observed_at.is_none());
    let findings = diagnostic_occurrences_for_runtime_session(
        fixture.runtime_home_path(),
        &runtime_session_id,
    )?;
    assert!(findings.iter().any(|finding| {
        finding.data().code().as_str() == "mcp.lifecycle.initialization_batch_forbidden"
    }));
    assert!(!findings
        .iter()
        .any(|finding| finding.data().code().as_str() == "mcp.lifecycle.duplicate_initialize"));
    Ok(())
}

#[test]
fn empty_batch_remains_a_json_rpc_invalid_request() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-empty-batch")?;
    let connection_adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();

    let response = handle_json_rpc_message(&connection_adapter, &mut state, json!([]))?
        .expect("empty batch rejection");
    assert_eq!(response["error"]["code"], -32600);
    assert_eq!(
        response["error"]["data"],
        "JSON-RPC batch must not be empty"
    );
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
    assert!(state.mcp_session.is_none());
    Ok(())
}
