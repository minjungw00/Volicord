use super::*;

#[test]
fn codex_compatible_2025_06_18_initialize_succeeds() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-negotiate-codex-2025-06-18")?;
    let input = Cursor::new(json_lines(&[initialize_request_for_protocol(
        1,
        json!({}),
        CODEX_MANAGED_MCP_CLIENT_NAME,
        CODEX_TEST_CLIENT_VERSION,
        "2025-06-18",
    )])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-06-18");
    Ok(())
}

#[test]
fn unsupported_initialize_revision_is_rejected_without_selecting_a_profile(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-reject-unsupported-revision")?;
    let adapter = adapter(&fixture)?;
    let mut state = session_state();
    let response = apply_json_rpc_message(
        &adapter,
        &mut state,
        initialize_request_for_protocol(
            1,
            json!({}),
            "unsupported-revision-client",
            "1.0",
            "unsupported-revision",
        ),
    )?
    .expect("unsupported revision should receive an explicit error");

    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["data"]
        .as_str()
        .is_some_and(|data| data.contains("not a supported MCP revision")));
    assert_eq!(state.phase(), SessionPhase::AwaitingInitialization);
    assert!(state.initialization_selection().is_none());
    Ok(())
}

#[test]
fn malformed_initialize_fields_are_invalid_params_without_session_mutation(
) -> Result<(), Box<dyn Error>> {
    let valid_client = json!({"name": "valid-client", "version": "1.0"});
    let cases = [
        json!({"capabilities": {}, "clientInfo": valid_client}),
        json!({"protocolVersion": 1, "capabilities": {}, "clientInfo": valid_client}),
        json!({"protocolVersion": "2025-11-25", "clientInfo": valid_client}),
        json!({"protocolVersion": "2025-11-25", "capabilities": null, "clientInfo": valid_client}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": null}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"version": "1.0"}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": 7, "version": "1.0"}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "valid-client"}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "valid-client", "version": false}}),
    ];

    for (index, params) in cases.into_iter().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-malformed-initialize-{index}"))?;
        let adapter = adapter(&fixture)?;
        let mut state = session_state();
        let response =
            apply_json_rpc_message(&adapter, &mut state, request(1, "initialize", params))?
                .expect("malformed initialize should return an error");

        assert_eq!(response["error"]["code"], -32602, "case {index}");
        assert!(response["error"]["data"]
            .as_str()
            .is_some_and(|data| data.len() <= 512));
        assert_eq!(state.phase(), SessionPhase::AwaitingInitialization);
        assert!(state.initialization_selection().is_none());
    }
    Ok(())
}

#[test]
fn lifecycle_rejects_tools_before_initialize_and_a_second_initialize() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-negotiate-lifecycle-order")?;
    let adapter = adapter(&fixture)?;
    let mut state = session_state();

    for message in [
        request(1, "tools/list", json!({})),
        tools_call(2, AgentToolId::STATUS.wire_name(), json!({})),
    ] {
        let response = apply_json_rpc_message(&adapter, &mut state, message)?
            .expect("pre-initialize request should return an error");
        assert_eq!(response["error"]["code"], -32600);
    }
    let initialized =
        apply_json_rpc_message(&adapter, &mut state, initialize_request(3, json!({})))?
            .expect("first initialize should return a result");
    assert!(initialized["result"].is_object());
    let repeated = apply_json_rpc_message(&adapter, &mut state, initialize_request(4, json!({})))?
        .expect("second initialize should return an error");
    assert_eq!(repeated["error"]["code"], -32600);
    Ok(())
}

#[test]
fn invalid_notifications_and_closed_requests_cannot_advance_session_state(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-closed-state-transition")?;
    let adapter = adapter(&fixture)?;
    let mut state = session_state();

    assert!(apply_json_rpc_message(&adapter, &mut state, initialized_notification())?.is_none());
    assert_eq!(state.phase(), SessionPhase::AwaitingInitialization);
    assert!(state.initialization_selection().is_none());

    let runtime = state.runtime().clone();
    state = SessionState::Closed(ClosedSession {
        runtime,
        termination: SessionTermination::GracefulEof,
    });
    let response = apply_json_rpc_message(&adapter, &mut state, request(9, "ping", json!({})))?
        .expect("closed sessions reject requests");

    assert_eq!(response["error"]["code"], -32600);
    assert_eq!(state.phase(), SessionPhase::Closed);
    let SessionState::Closed(closed) = state else {
        panic!("closed request must preserve the closed state");
    };
    assert_eq!(closed.termination, SessionTermination::GracefulEof);
    Ok(())
}

#[test]
fn tracked_nonproduction_protocol_is_rejected_as_unsupported() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-reject-nonproduction-revision")?;
    let adapter = adapter(&fixture)?;
    let mut state = session_state();
    let response = apply_json_rpc_message(
        &adapter,
        &mut state,
        initialize_request_for_protocol(1, json!({}), "future-client", "1.0", "2026-07-28"),
    )?
    .expect("tracked nonproduction revision should return an error");

    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["data"]
        .as_str()
        .is_some_and(|data| data.contains("not production-supported")));
    assert_eq!(state.phase(), SessionPhase::AwaitingInitialization);
    assert!(state.initialization_selection().is_none());
    Ok(())
}

#[test]
fn initialize_client_info_enforces_utf8_byte_bounds_before_connection_state_mutation(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-initialize-client-info-bound")?;
    let before = fixture.counts()?;
    let at_limit_name = format!(" {} ", "n".repeat(254));
    let at_limit_version = "가".repeat(85) + "a";
    assert_eq!(at_limit_name.len(), 256);
    assert_eq!(at_limit_version.len(), 256);

    let mut output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[
            initialize_request_with_client_info(1, json!({}), &at_limit_name, &at_limit_version),
        ])?)),
        &mut output,
    )?;
    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 1);
    assert!(responses[0]["result"].is_object());

    let invalid_values = [
        String::new(),
        " \t\u{2003}".to_owned(),
        "line\nbreak".to_owned(),
        "x".repeat(257),
        "가".repeat(86),
    ];
    for (index, invalid) in invalid_values.iter().enumerate() {
        for invalid_name in [true, false] {
            let (name, version) = if invalid_name {
                (invalid.as_str(), "valid-version")
            } else {
                ("valid-name", invalid.as_str())
            };
            let fallback_id = 100 + (index as u64 * 2) + u64::from(invalid_name);
            let input = json_lines(&[
                initialize_request_with_client_info(1, json!({}), name, version),
                initialize_request_with_client_info(
                    fallback_id,
                    json!({}),
                    "valid-name",
                    "valid-version",
                ),
            ])?;
            let mut output = Vec::new();
            run_stdio(
                adapter(&fixture)?,
                BufReader::new(Cursor::new(input)),
                &mut output,
            )?;
            let responses = stdio_responses(&output)?;
            assert_eq!(responses.len(), 2);
            assert_eq!(responses[0]["error"]["code"], -32602);
            assert!(responses[1]["result"].is_object());
            assert_eq!(responses[1]["id"], fallback_id);
        }
    }
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_tools_list_is_available_after_initialize() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-list-after-init")?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        json!(ProtocolRegistry::production()
            .preferred_server_profile()
            .revision()
            .as_str())
    );
    let names = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();
    assert_eq!(names, expected);
    Ok(())
}

#[test]
fn mcp_tools_list_remains_available_after_initialized_notification() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-mode")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let names = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "volicord.status",
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        ]
    );
    Ok(())
}

#[test]
fn mcp_tools_call_requires_initialized_notification() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-call-requires-ready")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
        tools_call(3, "volicord.intake", intake_args(None)),
        initialized_notification(),
        tools_call(4, "volicord.status", json!({ "detail": "workflow" })),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 4);
    assert!(responses[1]["result"]["tools"].is_array());
    assert_eq!(responses[2]["error"]["code"], -32600);
    assert!(responses[2]["error"]["data"]
        .as_str()
        .unwrap_or_default()
        .contains("notifications/initialized"));
    let status = volicord_response_from_tool(&responses[3])?;
    assert_eq!(status["base"]["response_kind"], "result");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}
