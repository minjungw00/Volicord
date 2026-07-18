use std::error::Error;

use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, set_connection_mode,
        AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_INTENT_SHARED,
        CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
    },
    bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS},
    diagnostics::{start_diagnostic_session, DiagnosticSessionStart, DiagnosticTransport},
    guards::{insert_agent_session, AgentSessionInsert},
    operational_sessions::{
        current_managed_mcp_runtime_session_for_connection, latest_current_managed_runtime_session,
        latest_managed_runtime_session, latest_successful_managed_runtime_session,
        mcp_runtime_session, record_mcp_initialize, record_mcp_initialized_notification,
        record_mcp_safe_read_only_tool_call, record_mcp_terminal_protocol_failure,
        record_mcp_tools_list, start_mcp_runtime_session, McpRuntimeSessionStart,
    },
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{managed_stdio_session_id, ManagedMcpClientInfo, McpRuntimeSessionSource};

const START: &str = "2026-07-18T00:00:00Z";
const INIT: &str = "2026-07-18T00:00:01Z";
const INITIALIZED: &str = "2026-07-18T00:00:02Z";
const TOOLS: &str = "2026-07-18T00:00:03Z";
const SAFE: &str = "2026-07-18T00:00:04Z";

fn start(fixture: &CoreFixture, source: McpRuntimeSessionSource) -> Result<String, Box<dyn Error>> {
    Ok(start_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: source,
            observed_host_executable_version: Some("future-host-999.1".to_owned()),
            process_id: 42,
            process_started_at: START.to_owned(),
        },
    )?
    .runtime_session_id)
}

fn complete(
    fixture: &CoreFixture,
    runtime_session_id: &str,
    required_tools_present: bool,
) -> Result<(), Box<dyn Error>> {
    let client = ManagedMcpClientInfo::new("future-client", "999.123-preview+custom")?;
    record_mcp_initialize(
        fixture.runtime_home_path(),
        runtime_session_id,
        &client,
        "2025-11-25",
        INIT,
    )?;
    record_mcp_initialized_notification(
        fixture.runtime_home_path(),
        runtime_session_id,
        INITIALIZED,
    )?;
    record_mcp_tools_list(
        fixture.runtime_home_path(),
        runtime_session_id,
        required_tools_present,
        TOOLS,
    )?;
    record_mcp_safe_read_only_tool_call(fixture.runtime_home_path(), runtime_session_id, SAFE)?;
    Ok(())
}

#[test]
fn managed_host_and_cli_preflight_sessions_are_distinct_authority_sources(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-session-sources")?;
    let cli = start(&fixture, McpRuntimeSessionSource::CliPreflight)?;
    complete(&fixture, &cli, true)?;
    assert!(latest_successful_managed_runtime_session(
        fixture.runtime_home_path(),
        fixture.connection_id()
    )?
    .is_none());

    let managed = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    complete(&fixture, &managed, true)?;
    assert_ne!(cli, managed);
    assert_eq!(
        latest_successful_managed_runtime_session(
            fixture.runtime_home_path(),
            fixture.connection_id()
        )?
        .expect("managed observation")
        .runtime_session_id,
        managed
    );
    Ok(())
}

#[test]
fn latest_queries_expose_partial_current_and_stale_managed_observations(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-session-current-and-stale")?;
    let managed = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;

    assert_eq!(
        latest_current_managed_runtime_session(
            fixture.runtime_home_path(),
            fixture.connection_id()
        )?
        .expect("partial current managed session")
        .runtime_session_id,
        managed
    );

    set_connection_mode(
        fixture.runtime_home_path(),
        fixture.connection_id(),
        CONNECTION_MODE_READ_ONLY,
    )?;
    assert!(latest_current_managed_runtime_session(
        fixture.runtime_home_path(),
        fixture.connection_id()
    )?
    .is_none());
    assert_eq!(
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .expect("stale managed session remains diagnostic evidence")
            .runtime_session_id,
        managed
    );
    Ok(())
}

#[test]
fn milestones_enforce_order_and_initialized_notification_is_idempotent(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-session-order")?;
    let runtime = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    assert!(record_mcp_tools_list(fixture.runtime_home_path(), &runtime, true, INIT).is_err());
    let client = ManagedMcpClientInfo::new("unlisted-client", "2037.0")?;
    record_mcp_initialize(
        fixture.runtime_home_path(),
        &runtime,
        &client,
        "2025-11-25",
        INIT,
    )?;
    let first =
        record_mcp_initialized_notification(fixture.runtime_home_path(), &runtime, INITIALIZED)?;
    let duplicate =
        record_mcp_initialized_notification(fixture.runtime_home_path(), &runtime, TOOLS)?;
    assert_eq!(
        first.initialized_notification_at,
        duplicate.initialized_notification_at
    );
    assert_eq!(duplicate.client_name.as_deref(), Some("unlisted-client"));
    assert_eq!(duplicate.client_version.as_deref(), Some("2037.0"));
    Ok(())
}

#[test]
fn required_tools_safe_success_and_fatal_failure_are_authoritative() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-session-facts")?;
    let incomplete = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    complete(&fixture, &incomplete, false)?;
    assert!(latest_successful_managed_runtime_session(
        fixture.runtime_home_path(),
        fixture.connection_id()
    )?
    .is_none());
    let record =
        mcp_runtime_session(fixture.runtime_home_path(), &incomplete)?.expect("runtime session");
    assert_eq!(record.required_tools_present, Some(false));
    assert_eq!(
        record.last_safe_read_only_tool_call_at.as_deref(),
        Some(SAFE)
    );

    let fatal = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let fatal_record = record_mcp_terminal_protocol_failure(
        fixture.runtime_home_path(),
        &fatal,
        "protocol_eof",
        Some("initialize response could not be emitted"),
        INIT,
    )?;
    assert_eq!(
        fatal_record.terminal_protocol_failure_code.as_deref(),
        Some("protocol_eof")
    );
    assert!(fatal_record.graceful_close_at.is_none());
    Ok(())
}

#[test]
fn runtime_session_ownership_and_diagnostics_authority_are_separate() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("operational-session-authority")?;
    let runtime = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    assert!(current_managed_mcp_runtime_session_for_connection(
        fixture.runtime_home_path(),
        &runtime,
        "another_connection"
    )
    .is_err());
    let other_connection = "connection_other";
    ensure_agent_connection(
        fixture.runtime_home_path(),
        AgentConnectionRegistration {
            connection_internal_id: other_connection.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            intent: CONNECTION_INTENT_SHARED.to_owned(),
            host_scope: HOST_SCOPE_PROJECT.to_owned(),
            server_name: "volicord-other".to_owned(),
            config_target: fixture
                .runtime_home_path()
                .join("other-config.toml")
                .display()
                .to_string(),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: true,
            managed_fingerprint: "managed:other".to_owned(),
            verification_report_json: None,
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        fixture.runtime_home_path(),
        ConnectionProjectRegistration {
            connection_internal_id: other_connection.to_owned(),
            project_id: fixture.project_id().to_owned(),
        },
    )?;
    let other_host_session = "host.session.other";
    assert!(insert_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionInsert {
            session_id: managed_stdio_session_id(other_connection, other_host_session)?,
            runtime_session_id: runtime.clone(),
            connection_internal_id: other_connection.to_owned(),
            guard_installation_id: None,
            host_session_id: other_host_session.to_owned(),
            host_thread_id: "host.thread.other".to_owned(),
            host_turn_id: "host.turn.other".to_owned(),
            observed_at: INIT.to_owned(),
        }
    )
    .is_err());

    start_diagnostic_session(
        fixture.runtime_home_path(),
        DiagnosticSessionStart {
            session_id: "diagnostic-only-success-shaped-row",
            connection_id: None,
            project_id: None,
            transport: DiagnosticTransport::CliInbox,
            host_kind: None,
            package_version: "999.0",
            build_id: "diagnostic-build",
        },
    )?;
    assert!(latest_successful_managed_runtime_session(
        fixture.runtime_home_path(),
        fixture.connection_id()
    )?
    .is_none());
    Ok(())
}

#[test]
fn project_session_cannot_cross_projects() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-project-session-scope")?;
    let second_project = "project_second";
    register_project(
        fixture.runtime_home_path(),
        ProjectRegistration {
            project_id: second_project.to_owned(),
            repo_root: fixture.create_product_repo("repo-second")?,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        fixture.runtime_home_path(),
        ConnectionProjectRegistration {
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: second_project.to_owned(),
        },
    )?;
    let runtime_session_id = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let host_session_id = "host.session.shared";
    let session_id = managed_stdio_session_id(fixture.connection_id(), host_session_id)?;
    let input = |turn: &str| AgentSessionInsert {
        session_id: session_id.clone(),
        runtime_session_id: runtime_session_id.clone(),
        connection_internal_id: fixture.connection_id().to_owned(),
        guard_installation_id: None,
        host_session_id: host_session_id.to_owned(),
        host_thread_id: "host.thread.shared".to_owned(),
        host_turn_id: turn.to_owned(),
        observed_at: INIT.to_owned(),
    };
    insert_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input("host.turn.first"),
    )?;
    assert!(insert_agent_session(
        fixture.runtime_home_path(),
        second_project,
        input("host.turn.second")
    )
    .is_err());
    Ok(())
}
