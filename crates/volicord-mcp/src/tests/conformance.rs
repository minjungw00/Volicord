use super::*;

#[test]
fn every_production_profile_executes_the_transport_and_eof_matrix() -> Result<(), Box<dyn Error>> {
    for (index, profile) in production_profiles().enumerate() {
        let revision = profile.revision().as_str();
        let fixture = CoreFixture::new(&format!("mcp-negotiate-production-{index}"))?;
        let connection_adapter = adapter(&fixture)?;
        let capabilities = json!({"experimental": {"fixture": true}});
        let mut state = session_state();

        let initialize = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialize_request_for_protocol(
                1,
                capabilities.clone(),
                "profile-test-client",
                "1.2.3",
                revision,
            ),
        )?
        .expect("initialize request should return a response");
        assert_eq!(initialize["result"]["protocolVersion"], revision);
        assert_eq!(initialize["result"]["capabilities"], json!({"tools": {}}));
        assert_eq!(
            initialize["result"].get("instructions").is_some(),
            profile.messages().initialize_result_instructions()
        );

        let selected = state
            .initialization_selection()
            .expect("valid initialize should select a session profile");
        assert_eq!(selected.requested_protocol_version, revision);
        assert_eq!(selected.selected_profile, profile);
        assert_eq!(selected.outcome, McpNegotiationOutcome::ExactMatch);
        assert_eq!(
            Value::Object(selected.client_capabilities.clone()),
            capabilities
        );
        assert_eq!(selected.attempted_client_name, "profile-test-client");
        assert_eq!(selected.attempted_client_version, "1.2.3");
        assert_eq!(state.phase(), SessionPhase::AwaitingInitializedNotification);

        let premature_tools = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(2, AgentToolId::STATUS.wire_name(), json!({})),
        )?
        .expect("premature tools/call should return an error");
        assert_eq!(premature_tools["error"]["code"], -32600);

        assert!(apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialized_notification()
        )?
        .is_none());
        assert_eq!(state.phase(), SessionPhase::InitializedAndReady);
        assert!(state.initialized_session().is_some());

        let input = Cursor::new(json_lines(&[
            initialize_request_for_protocol(
                10,
                json!({}),
                "recording-test-client",
                "4.5.6",
                revision,
            ),
            initialized_notification(),
            request(11, "ping", json!({})),
            request(12, "tools/list", json!({})),
            tools_call(13, AgentToolId::LIST_PROJECTS.wire_name(), json!({})),
        ])?);
        let mut output = Vec::new();
        run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 4);
        assert_eq!(responses[0]["result"]["protocolVersion"], revision);
        assert_eq!(responses[1]["result"], json!({}));
        assert!(responses[2]["result"]["tools"].is_array());
        let list_projects = projected_authoritative_tool_result(&responses[3]["result"])?;
        assert!(list_projects["projects"]
            .as_array()
            .is_some_and(|projects| !projects.is_empty()));

        let registry =
            open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
        let runtime_session_id = registry.query_row(
            "SELECT runtime_session_id
               FROM mcp_runtime_sessions
              WHERE connection_internal_id = ?1
                AND session_source = 'manual_cli'",
            [fixture.connection_id()],
            |row| row.get::<_, String>(0),
        )?;
        let recorded = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
            .expect("runtime session should be recorded");
        assert_eq!(
            recorded.negotiated_protocol_version.as_deref(),
            Some(revision)
        );
        assert!(recorded.initialized_notification_at.is_some());
        assert!(recorded.tools_list_observed_at.is_some());
        assert_eq!(recorded.required_tools_present, Some(true));
        assert!(recorded.verification_tool_name.is_none());
        assert!(recorded.verification_tool_observed_at.is_none());
        assert!(recorded.graceful_close_at.is_some());
        assert!(recorded.terminal_finding_id.is_none());
    }
    Ok(())
}

#[test]
fn every_production_profile_projects_tools_results_and_request_failures(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in production_profiles().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-revision-call-shape-{index}"))?;
        let connection_adapter = adapter(&fixture)?;
        let mut state = session_state();

        let initialize = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialize_request_for_protocol(
                1,
                json!({}),
                "revision-call-client",
                "1.0",
                profile.revision().as_str(),
            ),
        )?
        .expect("initialize response");
        assert_eq!(
            initialize["result"]["protocolVersion"],
            profile.revision().as_str()
        );
        assert!(apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialized_notification()
        )?
        .is_none());

        let ping = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(2, "ping", json!({})),
        )?
        .expect("ping response");
        assert_eq!(ping["result"], json!({}));

        let tools_list = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(3, "tools/list", json!({})),
        )?
        .expect("tools/list response");
        let tools = tools_list["result"]["tools"]
            .as_array()
            .expect("tools/list result array");
        let tool_names = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<BTreeSet<_>>();
        for required in AgentToolId::ALL.map(AgentToolId::wire_name) {
            assert!(
                tool_names.contains(required),
                "{} omitted required tool {required}",
                profile.revision()
            );
        }
        for tool in tools {
            let object = tool.as_object().expect("tool definition object");
            for required in ["name", "description", "inputSchema"] {
                assert!(object.contains_key(required), "missing {required}: {tool}");
            }
            assert_eq!(
                object.contains_key("annotations"),
                profile.tools().annotations(),
                "{} annotations projection",
                profile.revision()
            );
            assert_eq!(
                object.contains_key("outputSchema"),
                profile.tools().output_schema(),
                "{} outputSchema projection",
                profile.revision()
            );
            for absent in ["title", "_meta", "execution", "icons"] {
                assert!(!object.contains_key(absent), "fabricated {absent}: {tool}");
            }
        }

        let success = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(4, AgentToolId::LIST_PROJECTS.wire_name(), json!({})),
        )?
        .expect("list-projects response");
        let success_body = projected_authoritative_tool_result(&success["result"])?;
        assert!(success_body["projects"].is_array());
        assert_eq!(
            success["result"].get("structuredContent").is_some(),
            profile.tools().structured_content(),
            "{} structured-content projection",
            profile.revision()
        );
        assert_eq!(
            success["result"].get("toolResult").is_some(),
            profile
                .schema()
                .tool_result_fields()
                .contains(&ToolResultField::ToolResult),
            "{} legacy toolResult projection",
            profile.revision()
        );

        let invalid_arguments = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(
                5,
                AgentToolId::RECORD_RUN.wire_name(),
                json!({ "kind": "unsupported", "unexpected": true }),
            ),
        )?
        .expect("known-tool validation response");
        let invalid_body = projected_authoritative_tool_result(&invalid_arguments["result"])?;
        assert_eq!(invalid_body["code"], "MCP_INVALID_ARGUMENTS");
        assert_eq!(
            invalid_body["tool_name"],
            AgentToolId::RECORD_RUN.wire_name()
        );

        let invalid_params = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(6, "tools/list", json!([])),
        )?
        .expect("invalid tools/list params response");
        assert_eq!(invalid_params["error"]["code"], -32602);

        let invalid_request = apply_json_rpc_message(&connection_adapter, &mut state, json!(true))?
            .expect("invalid JSON-RPC request response");
        assert_eq!(invalid_request["error"]["code"], -32600);

        let unknown_method = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(7, "volicord/not-a-method", json!({})),
        )?
        .expect("unknown-method response");
        assert_eq!(unknown_method["error"]["code"], -32601);

        let execution_error = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(
                8,
                AgentToolId::STATUS.wire_name(),
                json!({"project_selector": "project_that_is_not_registered"}),
            ),
        )?
        .expect("tool execution error response");
        let execution_body = projected_authoritative_tool_result(&execution_error["result"])?;
        assert_eq!(execution_body["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
        assert_eq!(execution_body["tool_name"], AgentToolId::STATUS.wire_name());

        let unknown_tool = apply_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(9, "volicord.unknown", json!({})),
        )?
        .expect("unknown-tool response");
        assert_eq!(unknown_tool["error"]["code"], -32602);
        assert!(unknown_tool["result"].is_null());
    }
    Ok(())
}

#[test]
fn property_arbitrary_json_rpc_values_never_panic() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-arbitrary-json-rpc-property")?;
    let connection_adapter = adapter(&fixture)?;
    for seed in 0_u64..2_048 {
        let message = generated_json_rpc_value(seed, 0);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut state = session_state();
            let _ = apply_json_rpc_message(&connection_adapter, &mut state, message);
        }));
        assert!(result.is_ok(), "JSON-RPC input seed {seed} panicked");
    }
    Ok(())
}
