use std::error::Error;

use rusqlite::params;
use volicord_core::{CoreService, InvocationContext};
use volicord_host_contract::{
    CodexHookPromptCorrelation, CodexMcpCorrelation, HostNativeCorrelation, HostSessionId,
    HostThreadId, HostTurnId,
};
use volicord_store::{
    agent_connections::{set_connection_enabled, CONNECTION_MODE_READ_ONLY},
    guards::{
        agent_session, bind_agent_session_runtime, observe_host_correlation,
        AgentSessionRuntimeBinding, HostCorrelationObservation,
    },
    operational_sessions::{mcp_runtime_project_session_binding, McpRuntimeSessionStart},
    sqlite::registry_db_path,
};
use volicord_test_support::{
    core_fixtures::CoreFixture, seed_test_agent_session, transition_test_connection_mode,
};
use volicord_types::{
    ActorSource, AgentConnectionId, AgentRuntimeSessionId, AgentSessionId, FailureCategory,
    McpRuntimeSessionSource, OperationCategory, ProjectId,
};

fn mcp_correlation(session: &str, thread: &str, turn: &str) -> CodexMcpCorrelation {
    CodexMcpCorrelation {
        session_id: HostSessionId::parse(session).expect("valid test session"),
        thread_id: HostThreadId::parse(thread).expect("valid test thread"),
        turn_id: HostTurnId::parse(turn).expect("valid test turn"),
    }
}

fn prompt_correlation(session: &str, turn: &str) -> HostNativeCorrelation {
    HostNativeCorrelation::CodexHookPrompt(CodexHookPromptCorrelation {
        session_id: HostSessionId::parse(session).expect("valid test session"),
        turn_id: HostTurnId::parse(turn).expect("valid test turn"),
    })
}

#[test]
fn current_managed_session_authorizes_and_derives_deterministic_audit_basis(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core-agent-session-current")?;
    let session = seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
    )?;
    let service = CoreService::new(fixture.runtime_home_path());
    let validated = service.validate_agent_session(
        AgentConnectionId::new(fixture.connection_id()),
        ProjectId::new(fixture.project_id()),
        session.runtime_session_id.clone(),
        session.project_session_id.clone(),
        OperationCategory::Read,
    )?;

    assert_eq!(validated.connection_id().as_str(), fixture.connection_id());
    assert_eq!(validated.project_id().as_str(), fixture.project_id());
    assert_eq!(validated.runtime_session_id(), &session.runtime_session_id);
    assert_eq!(validated.project_session_id(), &session.project_session_id);

    let response = service.status(
        fixture.status_request("req_agent_session_audit", None),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::Read,
            "caller-controlled-label-is-ignored",
        )
        .with_validated_agent_session(validated.clone()),
    )?;
    let verified = response
        .verified_invocation
        .expect("Core should retain verified invocation diagnostics");
    assert_eq!(
        verified.verification_basis,
        format!(
            "connection:{}/session:{}/revision:{}",
            fixture.connection_id(),
            session.project_session_id,
            validated.integration_revision().as_str()
        )
    );
    assert_eq!(
        verified.session_id.as_deref(),
        Some(session.project_session_id.as_str())
    );

    let mismatched_actor = service.status(
        fixture.status_request("req_agent_session_wrong_actor", None),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection("connection_invented"),
            OperationCategory::Read,
            "",
        )
        .with_validated_agent_session(validated),
    )?;
    assert_eq!(
        mismatched_actor.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    Ok(())
}

#[test]
fn wrong_connection_project_runtime_and_project_session_coordinates_fail_closed(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core-agent-session-wrong-coordinates")?;
    let session = seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
    )?;
    let service = CoreService::new(fixture.runtime_home_path());

    let cases = [
        (
            AgentConnectionId::new("connection_invented"),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id.clone(),
            session.project_session_id.clone(),
            "agent_connection_not_current",
        ),
        (
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new("project_invented"),
            session.runtime_session_id.clone(),
            session.project_session_id.clone(),
            "connection_project_not_current",
        ),
        (
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            AgentRuntimeSessionId::new("runtime_invented"),
            session.project_session_id.clone(),
            "agent_runtime_session_not_current",
        ),
        (
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id,
            AgentSessionId::new("project_session_invented"),
            "agent_project_session_not_current",
        ),
    ];

    for (connection, project, runtime, project_session, reason) in cases {
        let error = service
            .validate_agent_session(
                connection,
                project,
                runtime,
                project_session,
                OperationCategory::Read,
            )
            .expect_err("invented or cross-scope coordinates must not authorize");
        assert_eq!(error.reason(), reason);
    }
    Ok(())
}

#[test]
fn disabled_connection_and_disallowed_mode_fail_closed() -> Result<(), Box<dyn Error>> {
    let disabled_fixture = CoreFixture::new("core-agent-session-disabled")?;
    let disabled_session = seed_test_agent_session(
        disabled_fixture.runtime_home_path(),
        disabled_fixture.project_id(),
        disabled_fixture.connection_id(),
        None,
    )?;
    set_connection_enabled(
        disabled_fixture.runtime_home_path(),
        disabled_fixture.connection_id(),
        false,
    )?;
    let error = CoreService::new(disabled_fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(disabled_fixture.connection_id()),
            ProjectId::new(disabled_fixture.project_id()),
            disabled_session.runtime_session_id,
            disabled_session.project_session_id,
            OperationCategory::Read,
        )
        .expect_err("disabled Connections cannot authorize calls");
    assert_eq!(error.reason(), "agent_connection_not_enabled");

    let readonly_fixture = CoreFixture::new("core-agent-session-readonly-mode")?;
    transition_test_connection_mode(
        readonly_fixture.runtime_home_path(),
        &readonly_fixture.product_repo_path(),
        readonly_fixture.project_id(),
        readonly_fixture.connection_id(),
        CONNECTION_MODE_READ_ONLY,
    )?;
    let readonly_session = seed_test_agent_session(
        readonly_fixture.runtime_home_path(),
        readonly_fixture.project_id(),
        readonly_fixture.connection_id(),
        None,
    )?;
    let error = CoreService::new(readonly_fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(readonly_fixture.connection_id()),
            ProjectId::new(readonly_fixture.project_id()),
            readonly_session.runtime_session_id,
            readonly_session.project_session_id,
            OperationCategory::AgentWorkflow,
        )
        .expect_err("read-only Connections cannot authorize workflow operations");
    assert_eq!(error.reason(), "agent_connection_mode_not_allowed");
    Ok(())
}

#[test]
fn stale_connection_and_project_revisions_fail_closed() -> Result<(), Box<dyn Error>> {
    let connection_fixture = CoreFixture::new("core-agent-session-stale-connection")?;
    let connection_session = seed_test_agent_session(
        connection_fixture.runtime_home_path(),
        connection_fixture.project_id(),
        connection_fixture.connection_id(),
        None,
    )?;
    transition_test_connection_mode(
        connection_fixture.runtime_home_path(),
        &connection_fixture.product_repo_path(),
        connection_fixture.project_id(),
        connection_fixture.connection_id(),
        CONNECTION_MODE_READ_ONLY,
    )?;
    let error = CoreService::new(connection_fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(connection_fixture.connection_id()),
            ProjectId::new(connection_fixture.project_id()),
            connection_session.runtime_session_id,
            connection_session.project_session_id,
            OperationCategory::Read,
        )
        .expect_err("a runtime session from the prior Connection revision must be stale");
    assert_eq!(error.reason(), "agent_runtime_session_not_current");

    let project_fixture = CoreFixture::new("core-agent-session-stale-project")?;
    let project_session = seed_test_agent_session(
        project_fixture.runtime_home_path(),
        project_fixture.project_id(),
        project_fixture.connection_id(),
        None,
    )?;
    let conn = project_fixture.conn()?;
    conn.execute(
        "INSERT INTO project_workflow_policies (
            project_id, policy_schema, policy_version, policy_json,
            policy_fingerprint, source, applied_at, created_at
         ) VALUES (?1, 'volicord.workflow_policy', 1, '{}', ?2, 'test', ?3, ?3)",
        params![
            project_fixture.project_id(),
            format!("sha256:{}", "9".repeat(64)),
            project_fixture.store()?.current_timestamp()?,
        ],
    )?;
    let error = CoreService::new(project_fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(project_fixture.connection_id()),
            ProjectId::new(project_fixture.project_id()),
            project_session.runtime_session_id,
            project_session.project_session_id,
            OperationCategory::Read,
        )
        .expect_err("a project session from the prior project revision must be stale");
    assert_eq!(error.reason(), "agent_project_session_revision_stale");
    Ok(())
}

#[test]
fn cli_preflight_and_invented_project_session_coordinates_never_authorize(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core-agent-session-preflight")?;
    let observed_at = fixture.store()?.current_timestamp()?;
    let runtime = volicord_test_support::start_test_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: McpRuntimeSessionSource::CliPreflight,
            observed_host_executable_version: Some("future-host-9999.0".to_owned()),
            process_id: std::process::id(),
            process_started_at: observed_at.clone(),
        },
    )?;
    let error = CoreService::new(fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            AgentRuntimeSessionId::new(runtime.runtime_session_id),
            AgentSessionId::new("agent_invented_project_session"),
            OperationCategory::Read,
        )
        .expect_err("CLI preflight and invented session coordinates cannot authorize");
    assert_eq!(error.reason(), "agent_runtime_session_not_current");
    Ok(())
}

#[test]
fn hook_only_host_session_cannot_authorize_until_mcp_runtime_attach() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("core-agent-session-guard-first")?;
    let host_session = "host.session.guard-first";
    let observed_at = fixture.store()?.current_timestamp()?;
    let observation = HostCorrelationObservation {
        connection_internal_id: fixture.connection_id().to_owned(),
        guard_installation_id: None,
        correlation: prompt_correlation(host_session, "host.turn.guard-first"),
        observed_at: observed_at.clone(),
    };
    let project_session_id = observe_host_correlation(
        fixture.runtime_home_path(),
        fixture.project_id(),
        observation,
    )?
    .session_id;
    let runtime = volicord_test_support::start_test_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: None,
            process_id: std::process::id(),
            process_started_at: observed_at.clone(),
        },
    )?;
    let service = CoreService::new(fixture.runtime_home_path());
    assert!(agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &project_session_id,
    )?
    .is_none());
    assert!(mcp_runtime_project_session_binding(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &project_session_id,
    )?
    .is_none());
    let error = service
        .validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            AgentRuntimeSessionId::new(&runtime.runtime_session_id),
            AgentSessionId::new(&project_session_id),
            OperationCategory::Read,
        )
        .expect_err("hook-only host records cannot authorize as a managed MCP session");
    assert_eq!(error.reason(), "agent_project_session_not_current");

    let attached = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionRuntimeBinding {
            runtime_session_id: runtime.runtime_session_id.clone(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            correlation: mcp_correlation(host_session, "host.thread.mcp", "host.turn.correct"),
            observed_at,
        },
    )?;
    assert_eq!(attached.session_id, project_session_id);
    assert_eq!(
        attached.runtime_session_id.as_deref(),
        Some(runtime.runtime_session_id.as_str())
    );
    let registry = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    let binding_count: i64 = registry.query_row(
        "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings WHERE session_id = ?1",
        [&project_session_id],
        |row| row.get(0),
    )?;
    assert_eq!(binding_count, 1);
    service.validate_agent_session(
        AgentConnectionId::new(fixture.connection_id()),
        ProjectId::new(fixture.project_id()),
        AgentRuntimeSessionId::new(runtime.runtime_session_id),
        AgentSessionId::new(project_session_id),
        OperationCategory::Read,
    )?;
    Ok(())
}

#[test]
fn registry_reservation_without_mcp_project_anchor_cannot_authorize() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("core-agent-session-reservation-only")?;
    let observed_at = fixture.store()?.current_timestamp()?;
    let host_session_id = "host.session.reservation-only";
    let session = observe_host_correlation(
        fixture.runtime_home_path(),
        fixture.project_id(),
        HostCorrelationObservation {
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            correlation: prompt_correlation(host_session_id, "host.turn.reservation-only"),
            observed_at: observed_at.clone(),
        },
    )?;
    let runtime = volicord_test_support::start_test_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: None,
            process_id: std::process::id(),
            process_started_at: observed_at.clone(),
        },
    )?;
    let registry = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    registry.execute(
        "INSERT INTO mcp_runtime_project_session_bindings (
            runtime_session_id, connection_internal_id, project_internal_id,
            session_id, project_integration_revision, host_session_id, bound_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            runtime.runtime_session_id,
            fixture.connection_id(),
            fixture.project_id(),
            session.session_id,
            session.project_integration_revision,
            host_session_id,
            observed_at,
        ],
    )?;

    let service = CoreService::new(fixture.runtime_home_path());
    let error = service
        .validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            AgentRuntimeSessionId::new(&runtime.runtime_session_id),
            AgentSessionId::new(&session.session_id),
            OperationCategory::Read,
        )
        .expect_err("a Registry reservation without an MCP project anchor cannot authorize Core");
    assert_eq!(error.reason(), "agent_project_session_not_current");

    let attached = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionRuntimeBinding {
            runtime_session_id: runtime.runtime_session_id.clone(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            correlation: mcp_correlation(
                host_session_id,
                "host.thread.reservation-only",
                "host.turn.reservation-only",
            ),
            observed_at,
        },
    )?;
    assert_eq!(
        attached.runtime_session_id.as_deref(),
        Some(runtime.runtime_session_id.as_str())
    );
    service.validate_agent_session(
        AgentConnectionId::new(fixture.connection_id()),
        ProjectId::new(fixture.project_id()),
        AgentRuntimeSessionId::new(runtime.runtime_session_id),
        AgentSessionId::new(session.session_id),
        OperationCategory::Read,
    )?;
    Ok(())
}

#[test]
fn mismatched_registry_host_session_binding_fails_closed() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core-agent-session-registry-host-mismatch")?;
    let session = seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
    )?;
    let registry = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    registry.execute(
        "UPDATE mcp_runtime_project_session_bindings
            SET host_session_id = 'host.session.conflicting'
          WHERE session_id = ?1",
        [session.project_session_id.as_str()],
    )?;
    let error = CoreService::new(fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            OperationCategory::Read,
        )
        .expect_err("a mismatched Registry host session must not authorize");
    assert_eq!(error.reason(), "agent_session_authority_unavailable");
    assert_eq!(error.category(), FailureCategory::Corrupt);
    Ok(())
}
