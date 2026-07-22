use std::{
    error::Error,
    sync::{Arc, Barrier},
    thread,
};

use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
        ConnectionProjectRegistration, CONNECTION_INTENT_SHARED, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
    },
    bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS},
    diagnostics::{start_diagnostic_session, DiagnosticSessionStart, DiagnosticTransport},
    guards::{
        agent_session, bind_agent_session_runtime, current_project_agent_session_coordinates,
        observe_agent_session, AgentSessionObservation, AgentSessionRuntimeBinding,
    },
    operational_sessions::{
        current_managed_mcp_runtime_session_for_connection, current_managed_runtime_sessions,
        latest_current_managed_runtime_session, latest_managed_runtime_session,
        latest_successful_managed_runtime_session, mcp_runtime_project_session_binding,
        mcp_runtime_session, mcp_runtime_session_for_process, record_mcp_initialize_attempt,
        record_mcp_initialize_completion, record_mcp_initialized_notification,
        record_mcp_terminal_finding, record_mcp_tools_list,
        record_mcp_verification_tool_observation, start_mcp_runtime_session,
        McpRuntimeSessionStart,
    },
    sqlite::registry_db_path,
};
use volicord_test_support::{core_fixtures::CoreFixture, transition_test_connection_mode};
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, AgentToolId, DiagnosticCode, DiagnosticDomain,
    DiagnosticFacts, DiagnosticFindingData, DiagnosticSeverity, DiagnosticSource, DiagnosticStage,
    DiagnosticSubject, IntegrationRevision, ManagedMcpClientInfo, McpRuntimeSessionSource,
    OccurrenceDiagnosticFinding, UtcTimestamp,
};

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

#[test]
fn runtime_session_process_lookup_is_connection_scoped() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-runtime-process-lookup")?;
    let runtime_session_id = start(&fixture, McpRuntimeSessionSource::CliPreflight)?;
    let found =
        mcp_runtime_session_for_process(fixture.runtime_home_path(), fixture.connection_id(), 42)?
            .ok_or("runtime session for child process")?;
    assert_eq!(found.runtime_session_id, runtime_session_id);
    assert!(mcp_runtime_session_for_process(
        fixture.runtime_home_path(),
        fixture.connection_id(),
        43,
    )?
    .is_none());
    Ok(())
}

fn complete(
    fixture: &CoreFixture,
    runtime_session_id: &str,
    required_tools_present: bool,
) -> Result<(), Box<dyn Error>> {
    let client = ManagedMcpClientInfo::new("future-client", "999.123-preview+custom")?;
    record_mcp_initialize_attempt(
        fixture.runtime_home_path(),
        runtime_session_id,
        &client,
        "2025-11-25",
        INIT,
    )?;
    record_mcp_initialize_completion(
        fixture.runtime_home_path(),
        runtime_session_id,
        "2025-11-25",
        INIT,
    )?;
    record_mcp_initialized_notification(
        fixture.runtime_home_path(),
        runtime_session_id,
        "2025-11-25",
        INITIALIZED,
    )?;
    record_mcp_tools_list(
        fixture.runtime_home_path(),
        runtime_session_id,
        required_tools_present,
        TOOLS,
    )?;
    record_mcp_verification_tool_observation(
        fixture.runtime_home_path(),
        runtime_session_id,
        SAFE,
    )?;
    Ok(())
}

fn terminal_finding(
    fixture: &CoreFixture,
    runtime_session_id: &str,
    _finding_id: &str,
    code: &str,
    observed_at: &str,
) -> Result<OccurrenceDiagnosticFinding, Box<dyn Error>> {
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), runtime_session_id)?
        .ok_or("runtime session")?;
    let data = DiagnosticFindingData::try_new(
        DiagnosticCode::parse(code)?,
        DiagnosticDomain::parse("mcp")?,
        DiagnosticStage::parse("transport")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("store_test")?,
        DiagnosticSubject::try_new("runtime_session", runtime_session_id)?,
        DiagnosticFacts::empty(),
        UtcTimestamp::parse(observed_at)?,
    )?
    .with_connection_id(AgentConnectionId::new(runtime.connection_internal_id))?
    .with_integration_revision(IntegrationRevision::parse(
        runtime.connection_integration_revision,
    )?);
    Ok(OccurrenceDiagnosticFinding::try_new(
        data,
        Some(AgentRuntimeSessionId::new(runtime_session_id)),
    )?)
}

fn agent_session_count(fixture: &CoreFixture) -> Result<i64, Box<dyn Error>> {
    let project_state = rusqlite::Connection::open(
        fixture
            .runtime_home_path()
            .join("projects")
            .join(fixture.project_id())
            .join("state.sqlite"),
    )?;
    Ok(project_state.query_row("SELECT COUNT(*) FROM agent_sessions", [], |row| row.get(0))?)
}

fn runtime_project_binding_count(fixture: &CoreFixture) -> Result<i64, Box<dyn Error>> {
    let registry = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    Ok(registry.query_row(
        "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings",
        [],
        |row| row.get(0),
    )?)
}

#[test]
fn managed_host_and_cli_preflight_sessions_are_distinct_authority_sources(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-session-sources")?;
    let cli = start(&fixture, McpRuntimeSessionSource::CliPreflight)?;
    assert!(complete(&fixture, &cli, true).is_err());
    let cli_record = mcp_runtime_session(fixture.runtime_home_path(), &cli)?.expect("CLI session");
    assert!(cli_record.verification_tool_name.is_none());
    assert!(cli_record.verification_tool_observed_at.is_none());
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

    transition_test_connection_mode(
        fixture.runtime_home_path(),
        &fixture.product_repo_path(),
        fixture.project_id(),
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
    record_mcp_initialize_attempt(
        fixture.runtime_home_path(),
        &runtime,
        &client,
        "2025-06-18",
        INIT,
    )?;
    let attempted = mcp_runtime_session(fixture.runtime_home_path(), &runtime)?
        .expect("initialize attempt observation");
    assert_eq!(
        attempted.attempted_client_name.as_deref(),
        Some("unlisted-client")
    );
    assert_eq!(
        attempted.attempted_client_version.as_deref(),
        Some("2037.0")
    );
    assert_eq!(
        attempted.requested_protocol_version.as_deref(),
        Some("2025-06-18")
    );
    assert!(attempted.selected_protocol_version.is_none());
    assert!(attempted.initialize_completed_at.is_none());
    record_mcp_initialize_completion(fixture.runtime_home_path(), &runtime, "2025-11-25", INIT)?;
    let selected = mcp_runtime_session(fixture.runtime_home_path(), &runtime)?
        .expect("initialize observation");
    assert!(selected.initialize_completed_at.is_some());
    assert_eq!(
        selected.selected_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert!(selected.negotiated_protocol_version.is_none());
    let first = record_mcp_initialized_notification(
        fixture.runtime_home_path(),
        &runtime,
        "2025-11-25",
        INITIALIZED,
    )?;
    let duplicate = record_mcp_initialized_notification(
        fixture.runtime_home_path(),
        &runtime,
        "2025-11-25",
        TOOLS,
    )?;
    assert_eq!(
        first.initialized_notification_at,
        duplicate.initialized_notification_at
    );
    assert_eq!(
        duplicate.attempted_client_name.as_deref(),
        Some("unlisted-client")
    );
    assert_eq!(
        duplicate.attempted_client_version.as_deref(),
        Some("2037.0")
    );
    assert_eq!(
        duplicate.negotiated_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert!(record_mcp_initialized_notification(
        fixture.runtime_home_path(),
        &runtime,
        "2025-06-18",
        "2026-07-18T00:00:05Z",
    )
    .is_err());
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
        record.verification_tool_name.as_deref(),
        Some(AgentToolId::LIST_PROJECTS.wire_name())
    );
    assert_eq!(record.verification_tool_observed_at.as_deref(), Some(SAFE));

    let fatal = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let finding = terminal_finding(
        &fixture,
        &fatal,
        "finding.protocol_eof",
        "mcp.protocol_eof",
        INIT,
    )?;
    let fatal_record = record_mcp_terminal_finding(fixture.runtime_home_path(), &finding)?;
    assert_eq!(
        fatal_record.terminal_finding_id.as_deref(),
        Some(finding.id().as_str())
    );
    assert!(fatal_record.graceful_close_at.is_none());

    let completed_then_terminal = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    complete(&fixture, &completed_then_terminal, true)?;
    let finding = terminal_finding(
        &fixture,
        &completed_then_terminal,
        "finding.later_transport_failure",
        "mcp.later_transport_failure",
        "2026-07-18T00:00:05Z",
    )?;
    record_mcp_terminal_finding(fixture.runtime_home_path(), &finding)?;
    assert_eq!(
        latest_successful_managed_runtime_session(
            fixture.runtime_home_path(),
            fixture.connection_id()
        )?
        .expect("completed milestones remain valid evidence")
        .runtime_session_id,
        completed_then_terminal
    );
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
    assert!(bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionRuntimeBinding {
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
    let rejected_coordinates = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        fixture.project_id(),
        other_connection,
        None,
        other_host_session,
    )?;
    assert!(agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &rejected_coordinates.session_id,
    )?
    .is_none());
    assert!(mcp_runtime_project_session_binding(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &rejected_coordinates.session_id,
    )?
    .is_none());

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
    let input = |runtime_session_id: &str, turn: &str| AgentSessionRuntimeBinding {
        runtime_session_id: runtime_session_id.to_owned(),
        connection_internal_id: fixture.connection_id().to_owned(),
        guard_installation_id: None,
        host_session_id: host_session_id.to_owned(),
        host_thread_id: "host.thread.shared".to_owned(),
        host_turn_id: turn.to_owned(),
        observed_at: INIT.to_owned(),
    };
    bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input(&runtime_session_id, "host.turn.first"),
    )?;
    assert!(bind_agent_session_runtime(
        fixture.runtime_home_path(),
        second_project,
        input(&runtime_session_id, "host.turn.second")
    )
    .is_err());
    let second_coordinates = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        second_project,
        fixture.connection_id(),
        None,
        host_session_id,
    )?;
    let unbound = agent_session(
        fixture.runtime_home_path(),
        second_project,
        &second_coordinates.session_id,
    )?
    .expect("Phase 1 anchor remains after natural Registry uniqueness conflict");
    assert!(unbound.runtime_session_id.is_none());
    assert!(mcp_runtime_project_session_binding(
        fixture.runtime_home_path(),
        second_project,
        &second_coordinates.session_id,
    )?
    .is_none());

    let non_conflicting_runtime = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let attached = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        second_project,
        input(&non_conflicting_runtime, "host.turn.recovery"),
    )?;
    assert_eq!(attached.session_id, second_coordinates.session_id);
    assert_eq!(
        attached.runtime_session_id.as_deref(),
        Some(non_conflicting_runtime.as_str())
    );
    Ok(())
}

#[test]
fn guard_first_session_attaches_to_first_real_managed_runtime_idempotently(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-guard-first-binding")?;
    let host_session_id = "host.session.guard-first";
    let observation = |turn: &str, observed_at: &str| AgentSessionObservation {
        connection_internal_id: fixture.connection_id().to_owned(),
        guard_installation_id: None,
        host_session_id: host_session_id.to_owned(),
        host_thread_id: "host.thread.guard-first".to_owned(),
        host_turn_id: turn.to_owned(),
        observed_at: observed_at.to_owned(),
    };
    let binding =
        |runtime_session_id: &str, turn: &str, observed_at: &str| AgentSessionRuntimeBinding {
            runtime_session_id: runtime_session_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_session_id: host_session_id.to_owned(),
            host_thread_id: "host.thread.guard-first".to_owned(),
            host_turn_id: turn.to_owned(),
            observed_at: observed_at.to_owned(),
        };

    let unbound = observe_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        observation("host.turn.guard", START),
    )?;
    let session_id = unbound.session_id.clone();
    assert_eq!(unbound.runtime_session_id, None);
    assert_eq!(unbound.first_observed_at, START);
    assert!(mcp_runtime_project_session_binding(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &session_id
    )?
    .is_none());

    let runtime = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let bound = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        binding(&runtime, "host.turn.tool", INIT),
    )?;
    assert_eq!(bound.runtime_session_id.as_deref(), Some(runtime.as_str()));
    assert_eq!(bound.first_observed_at, START);
    assert_eq!(bound.last_observed_at, INIT);
    assert_eq!(bound.last_host_turn_id, "host.turn.tool");

    let replay = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        binding(&runtime, "host.turn.tool", INIT),
    )?;
    assert_eq!(replay, bound);
    let reservation = mcp_runtime_project_session_binding(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &session_id,
    )?
    .expect("runtime binding");
    assert_eq!(reservation.runtime_session_id, runtime);
    assert_eq!(reservation.host_session_id, host_session_id);
    assert_eq!(
        reservation.project_integration_revision,
        bound.project_integration_revision
    );
    let project_state_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let project_state = rusqlite::Connection::open(project_state_path)?;
    let immutable = project_state.execute(
        "UPDATE agent_sessions
            SET project_integration_revision = ?2
          WHERE session_id = ?1",
        rusqlite::params![&session_id, format!("sha256:{}", "f".repeat(64))],
    );
    assert!(
        immutable.is_err(),
        "stored Agent Session revision must be immutable"
    );
    Ok(())
}

#[test]
fn concurrent_runtimes_bind_distinct_host_sessions_without_guessing() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("operational-concurrent-runtime-binding")?;
    let runtime_a = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let runtime_b = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    for (runtime, host_session, thread) in [
        (
            &runtime_a,
            "host.session.concurrent-a",
            "host.thread.concurrent-a",
        ),
        (
            &runtime_b,
            "host.session.concurrent-b",
            "host.thread.concurrent-b",
        ),
    ] {
        observe_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            AgentSessionObservation {
                connection_internal_id: fixture.connection_id().to_owned(),
                guard_installation_id: None,
                host_session_id: host_session.to_owned(),
                host_thread_id: thread.to_owned(),
                host_turn_id: format!("{thread}.guard"),
                observed_at: START.to_owned(),
            },
        )?;
        bind_agent_session_runtime(
            fixture.runtime_home_path(),
            fixture.project_id(),
            AgentSessionRuntimeBinding {
                runtime_session_id: runtime.clone(),
                connection_internal_id: fixture.connection_id().to_owned(),
                guard_installation_id: None,
                host_session_id: host_session.to_owned(),
                host_thread_id: thread.to_owned(),
                host_turn_id: format!("{thread}.mcp"),
                observed_at: INIT.to_owned(),
            },
        )?;
    }
    let runtime_c = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let host_session_c = "host.session.concurrent-c";
    bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionRuntimeBinding {
            runtime_session_id: runtime_c,
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_session_id: host_session_c.to_owned(),
            host_thread_id: "host.thread.concurrent-c".to_owned(),
            host_turn_id: "host.turn.concurrent-c".to_owned(),
            observed_at: INIT.to_owned(),
        },
    )?;
    assert_eq!(
        current_managed_runtime_sessions(fixture.runtime_home_path(), fixture.connection_id())?
            .len(),
        3
    );
    Ok(())
}

#[test]
fn concurrent_runtimes_claiming_one_project_session_produce_one_winner(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-concurrent-single-session-claim")?;
    let host_session = "host.session.concurrent-claim";
    let host_thread = "host.thread.concurrent-claim";
    let unbound = observe_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionObservation {
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_session_id: host_session.to_owned(),
            host_thread_id: host_thread.to_owned(),
            host_turn_id: "host.turn.guard".to_owned(),
            observed_at: START.to_owned(),
        },
    )?;
    let runtimes = [
        start(&fixture, McpRuntimeSessionSource::ManagedHost)?,
        start(&fixture, McpRuntimeSessionSource::ManagedHost)?,
    ];
    let barrier = Arc::new(Barrier::new(runtimes.len()));
    let mut claims = Vec::new();
    for runtime_session_id in runtimes {
        let barrier = Arc::clone(&barrier);
        let runtime_home = fixture.runtime_home_path().to_path_buf();
        let project_id = fixture.project_id().to_owned();
        let connection_internal_id = fixture.connection_id().to_owned();
        claims.push(thread::spawn(move || {
            barrier.wait();
            bind_agent_session_runtime(
                runtime_home,
                &project_id,
                AgentSessionRuntimeBinding {
                    runtime_session_id,
                    connection_internal_id,
                    guard_installation_id: None,
                    host_session_id: host_session.to_owned(),
                    host_thread_id: host_thread.to_owned(),
                    host_turn_id: "host.turn.mcp".to_owned(),
                    observed_at: INIT.to_owned(),
                },
            )
        }));
    }
    let results = claims
        .into_iter()
        .map(|claim| claim.join().expect("claim thread must not panic"))
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let winner = results
        .into_iter()
        .find_map(Result::ok)
        .expect("one runtime wins");
    let stored = volicord_store::guards::agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &unbound.session_id,
    )?
    .expect("project Agent Session");
    assert_eq!(stored.runtime_session_id, winner.runtime_session_id);
    let registry = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    let binding_count: i64 = registry.query_row(
        "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings WHERE session_id = ?1",
        [&unbound.session_id],
        |row| row.get(0),
    )?;
    assert_eq!(binding_count, 1);
    Ok(())
}

#[test]
fn managed_binding_replay_is_idempotent_and_conflicting_runtime_fails() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("operational-reservation-recovery")?;
    let runtime = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    let host_session = "host.session.recovery";
    let input = |runtime_session_id: &str, turn: &str| AgentSessionRuntimeBinding {
        runtime_session_id: runtime_session_id.to_owned(),
        connection_internal_id: fixture.connection_id().to_owned(),
        guard_installation_id: None,
        host_session_id: host_session.to_owned(),
        host_thread_id: "host.thread.recovery".to_owned(),
        host_turn_id: turn.to_owned(),
        observed_at: INIT.to_owned(),
    };
    let attached = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input(&runtime, "host.turn.recovery"),
    )?;
    let replay = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input(&runtime, "host.turn.recovery"),
    )?;
    assert_eq!(replay, attached);

    let conflicting_runtime = start(&fixture, McpRuntimeSessionSource::ManagedHost)?;
    assert!(bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input(&conflicting_runtime, "host.turn.conflict"),
    )
    .is_err());
    Ok(())
}

#[test]
fn cli_preflight_runtime_cannot_attach_a_project_session() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-preflight-no-binding")?;
    let runtime = start(&fixture, McpRuntimeSessionSource::CliPreflight)?;
    let host_session = "host.session.preflight";
    let coordinates = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
        host_session,
    )?;
    assert!(bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionRuntimeBinding {
            runtime_session_id: runtime,
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_session_id: host_session.to_owned(),
            host_thread_id: "host.thread.preflight".to_owned(),
            host_turn_id: "host.turn.preflight".to_owned(),
            observed_at: INIT.to_owned(),
        },
    )
    .is_err());
    assert!(agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &coordinates.session_id,
    )?
    .is_none());
    assert!(mcp_runtime_project_session_binding(
        fixture.runtime_home_path(),
        fixture.project_id(),
        &coordinates.session_id,
    )?
    .is_none());
    Ok(())
}

#[test]
fn phase_zero_rejections_mutate_neither_project_nor_registry() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("operational-phase-zero-no-mutation")?;
    let runtime = start_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: None,
            process_id: 42,
            process_started_at: INIT.to_owned(),
        },
    )?;
    let input =
        |guard_installation_id: Option<&str>, observed_at: &str| AgentSessionRuntimeBinding {
            runtime_session_id: runtime.runtime_session_id.clone(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: guard_installation_id.map(str::to_owned),
            host_session_id: "host.session.phase-zero".to_owned(),
            host_thread_id: "host.thread.phase-zero".to_owned(),
            host_turn_id: "host.turn.phase-zero".to_owned(),
            observed_at: observed_at.to_owned(),
        };
    let project_count = agent_session_count(&fixture)?;
    let registry_count = runtime_project_binding_count(&fixture)?;

    let before_process_start = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input(None, START),
    )
    .expect_err("an observation before process start must fail in Phase 0");
    assert!(matches!(
        before_process_start,
        volicord_store::StoreError::InvalidInput { .. }
    ));
    assert_eq!(agent_session_count(&fixture)?, project_count);
    assert_eq!(runtime_project_binding_count(&fixture)?, registry_count);

    let wrong_guard = bind_agent_session_runtime(
        fixture.runtime_home_path(),
        fixture.project_id(),
        input(Some("guard_installation_missing"), INIT),
    )
    .expect_err("a non-current Guard installation must fail before Phase 1");
    assert!(matches!(
        wrong_guard,
        volicord_store::StoreError::Conflict { .. }
    ));
    assert_eq!(agent_session_count(&fixture)?, project_count);
    assert_eq!(runtime_project_binding_count(&fixture)?, registry_count);
    Ok(())
}
