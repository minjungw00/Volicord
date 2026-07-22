//! MCP preflight and handshake check projection.

use super::*;

pub(in crate::connection_command) fn mcp_server_check(
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let step = &handshake.step;
    let (status, code, summary) = if preflight.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            preflight.code.as_str(),
            "Volicord CLI MCP preflight failed",
        )
    } else if step.status == StepStatus::Passed {
        (
            ConnectionCheckStatus::Passed,
            step.code.as_str(),
            "Volicord MCP server self-test passed",
        )
    } else if step.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            step.code.as_str(),
            "Volicord MCP server self-test failed",
        )
    } else {
        (
            ConnectionCheckStatus::Failed,
            "mcp_server_self_test_not_run",
            "Volicord MCP server self-test did not run",
        )
    };
    let progress = handshake
        .exchange
        .as_ref()
        .map(|exchange| &exchange.progress);
    let exchange = handshake.exchange.as_ref();
    let mut self_test = json!({
        "status": step.status.as_str(),
        "code": step.code,
        "diagnostic": step.details,
        "safe_read_only_tool": super::super::managed_host_round_trip_tool().wire_name(),
    });
    if exchange.is_some_and(|exchange| {
        !exchange.conformance.is_empty() || !exchange.host_compatibility.is_empty()
    }) {
        let exchange = exchange.expect("matrix exchange was checked");
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                (
                    "production_supported_revisions".to_owned(),
                    json!(exchange
                        .conformance
                        .iter()
                        .map(|probe| probe.revision.as_str())
                        .collect::<Vec<_>>()),
                ),
                (
                    "conformance".to_owned(),
                    Value::Array(
                        exchange
                            .conformance
                            .iter()
                            .map(|probe| {
                                probe_result_json(
                                    &probe.progress,
                                    probe.failure.as_ref(),
                                    probe.diagnostic.as_ref(),
                                    [("revision", json!(probe.revision))],
                                )
                            })
                            .collect(),
                    ),
                ),
                (
                    "host_compatibility_profiles".to_owned(),
                    json!(exchange
                        .host_compatibility
                        .iter()
                        .map(|probe| probe.profile.as_str())
                        .collect::<Vec<_>>()),
                ),
                (
                    "host_compatibility".to_owned(),
                    Value::Array(
                        exchange
                            .host_compatibility
                            .iter()
                            .map(|probe| {
                                probe_result_json(
                                    &probe.progress,
                                    probe.failure.as_ref(),
                                    probe.diagnostic.as_ref(),
                                    [
                                        ("profile", json!(probe.profile.as_str())),
                                        ("fixture", json!(probe.fixture_id)),
                                    ],
                                )
                            })
                            .collect(),
                    ),
                ),
            ]);
        if let Some(tools) = exchange
            .conformance
            .iter()
            .find_map(|probe| probe.progress.tools_list.as_ref())
        {
            self_test
                .as_object_mut()
                .expect("self-test details are an object")
                .insert("tools_list".to_owned(), json!(tools));
        }
    } else {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                (
                    "initialize".to_owned(),
                    json!(progress.is_some_and(|progress| progress.initialize_completed)),
                ),
                (
                    "tools_list_observed".to_owned(),
                    json!(progress.is_some_and(|progress| progress.tools_list.is_some())),
                ),
                (
                    "required_tools_validated".to_owned(),
                    json!(progress.is_some_and(|progress| progress.required_tools_validated)),
                ),
                (
                    "safe_read_only_tool_completed".to_owned(),
                    json!(progress.is_some_and(|progress| progress.safe_tool_call_completed)),
                ),
                (
                    "shutdown_completed".to_owned(),
                    json!(progress.is_some_and(|progress| progress.shutdown_completed)),
                ),
            ]);
    }
    if let Some(tools) = progress.and_then(|progress| progress.tools_list.as_ref()) {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .insert("tools_list".to_owned(), json!(tools));
    }
    if let Some(failure) = handshake
        .exchange
        .as_ref()
        .and_then(|exchange| exchange.failure.as_ref())
    {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                (
                    "diagnostic_code".to_owned(),
                    json!(failure.diagnostic_code()),
                ),
                ("failure_stage".to_owned(), json!(failure.stage().as_str())),
            ]);
    }
    if let Some(diagnostic) = handshake
        .exchange
        .as_ref()
        .and_then(|exchange| exchange.diagnostic.as_ref())
    {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                ("finding_id".to_owned(), json!(diagnostic.finding_id)),
                ("diagnostic_code".to_owned(), json!(diagnostic.code)),
            ]);
    }
    canonical_check(
        ConnectionCheckKind::McpServer,
        status,
        code,
        summary,
        Some(json!({
            "preflight": {
                "status": preflight.status.as_str(),
                "code": preflight.code,
                "diagnostic": preflight.details,
                "storage": preflight.preflight_diagnostics.as_ref().map(McpPreflightDiagnostics::to_json),
                "finding_id": preflight.diagnostic.as_ref().map(|diagnostic| diagnostic.finding_id.as_str()),
                "diagnostic_code": preflight.diagnostic.as_ref().map(|diagnostic| diagnostic.code.as_str()),
                "failure_stage": preflight.failure.as_ref().map(|failure| failure.stage().as_str()),
            },
            "self_test": self_test,
        })),
        None,
    )
}

pub(super) fn probe_result_json<const N: usize>(
    progress: &crate::connection_command::McpExchangeProgress,
    failure: Option<&McpProcessFailure>,
    diagnostic: Option<&McpPersistedDiagnostic>,
    identity: [(&str, Value); N],
) -> Value {
    let mut result = json!({
        "status": if failure.is_none() { "passed" } else { "failed" },
        "requested_revision": progress.requested_revision,
        "negotiated_revision": progress.negotiated_revision,
        "initialize": progress.initialize_completed,
        "initialized_notification": progress.initialized_notification_completed,
        "pinned_schema_validated": progress.pinned_schema_validated,
        "tools_list_observed": progress.tools_list.is_some(),
        "tools_returned": progress.tools_list.as_ref().map(Vec::len),
        "required_tools_validated": progress.required_tools_validated,
        "safe_read_only_tool": super::super::managed_host_round_trip_tool().wire_name(),
        "safe_read_only_tool_completed": progress.safe_tool_call_completed,
        "shutdown_completed": progress.shutdown_completed,
    });
    let object = result.as_object_mut().expect("probe result is an object");
    for (field, value) in identity {
        object.insert(field.to_owned(), value);
    }
    if let Some(failure) = failure {
        object.insert(
            "diagnostic_code".to_owned(),
            json!(failure.diagnostic_code()),
        );
        object.insert("failure_stage".to_owned(), json!(failure.stage().as_str()));
    }
    if let Some(diagnostic) = diagnostic {
        object.insert("finding_id".to_owned(), json!(diagnostic.finding_id));
        object.insert("diagnostic_code".to_owned(), json!(diagnostic.code));
    }
    result
}

pub(super) fn mcp_server_finding_ids(
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<Vec<DiagnosticFindingId>, ConnectionCommandError> {
    let mut ids = BTreeMap::<String, DiagnosticFindingId>::new();
    let mut insert = |value: &str| -> Result<(), ConnectionCommandError> {
        let id = DiagnosticFindingId::parse(value.to_owned())
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        ids.insert(value.to_owned(), id);
        Ok(())
    };
    if preflight.status == StepStatus::Failed {
        if let Some(diagnostic) = preflight.diagnostic.as_ref() {
            insert(&diagnostic.finding_id)?;
        }
    }
    if handshake.step.status == StepStatus::Failed {
        if let Some(exchange) = handshake.exchange.as_ref() {
            if let Some(diagnostic) = exchange.diagnostic.as_ref() {
                insert(&diagnostic.finding_id)?;
            }
            for probe in &exchange.conformance {
                if let Some(diagnostic) = probe.diagnostic.as_ref() {
                    insert(&diagnostic.finding_id)?;
                }
            }
            for probe in &exchange.host_compatibility {
                if let Some(diagnostic) = probe.diagnostic.as_ref() {
                    insert(&diagnostic.finding_id)?;
                }
            }
        }
    }
    Ok(ids.into_values().collect())
}
