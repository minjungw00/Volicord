use std::error::Error;

use volicord_core::{CoreService, InvocationContext};
use volicord_store::{
    agent_connections::{set_connection_enabled, CONNECTION_MODE_READ_ONLY},
    core_pipeline::CoreProjectStore,
    workflow_records::ProjectWorkflowPolicyAuthorityApply,
};
use volicord_test_support::{
    core_fixtures::CoreFixture, seed_test_agent_session, transition_test_connection_mode,
};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId, AgentSessionId, ProjectId};
use volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID;
use volicord_types::values::OperationCategory;
use volicord_types::workflow_policy::{ProjectWorkflowPolicy, ProjectWorkflowPolicySource};

#[test]
fn current_agent_session_authorizes_a_typed_core_invocation() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core-agent-session-current")?;
    let session = seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
    )?;
    let service = CoreService::for_read_only(fixture.runtime_home_path());
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
        InvocationContext::agent_connection(OperationCategory::Read, validated.clone()),
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
    Ok(())
}

#[test]
fn wrong_agent_session_coordinates_fail_closed() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core-agent-session-wrong-coordinates")?;
    let session = seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
    )?;
    let service = CoreService::for_read_only(fixture.runtime_home_path());
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
fn disabled_connection_and_disallowed_capability_fail_closed() -> Result<(), Box<dyn Error>> {
    let disabled_fixture = CoreFixture::new("core-agent-session-disabled")?;
    let disabled_session = seed_test_agent_session(
        disabled_fixture.runtime_home_path(),
        disabled_fixture.project_id(),
        disabled_fixture.connection_id(),
        None,
    )?;
    set_connection_enabled(
        &disabled_fixture.mutation_context()?,
        disabled_fixture.connection_id(),
        false,
    )?;
    let error = CoreService::for_read_only(disabled_fixture.runtime_home_path())
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
    let error = CoreService::for_read_only(readonly_fixture.runtime_home_path())
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
    let error = CoreService::for_read_only(connection_fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(connection_fixture.connection_id()),
            ProjectId::new(connection_fixture.project_id()),
            connection_session.runtime_session_id,
            connection_session.project_session_id,
            OperationCategory::Read,
        )
        .expect_err("a session from the prior Connection revision must be stale");
    assert_eq!(error.reason(), "agent_runtime_session_not_current");

    let project_fixture = CoreFixture::new("core-agent-session-stale-project")?;
    let project_session = seed_test_agent_session(
        project_fixture.runtime_home_path(),
        project_fixture.project_id(),
        project_fixture.connection_id(),
        None,
    )?;
    let policy = serde_json::json!({
        "schema": WORKFLOW_POLICY_CONTRACT_ID,
        "managed_by": "volicord",
        "storage_scope": "local_overlay",
        "connection_intent": "shared",
        "host": "codex",
        "repo_root": "/tmp/core-agent-session-policy",
        "connection_id": project_fixture.connection_id(),
        "guard_installation_id": "guard_agent_session_test",
        "selected_profile": "record",
        "mcp": {
            "command": "volicord-mcp",
            "args": [],
            "env": {}
        },
        "host_hook": {
            "enabled": true,
            "commands": {
                "pre_tool": {"command": "volicord", "args": ["guard", "pre-tool"]},
                "post_tool": {"command": "volicord", "args": ["guard", "post-tool"]},
                "prompt_capture": {
                    "command": "volicord",
                    "args": ["guard", "prompt-capture"]
                }
            }
        },
        "workflow": {
            "default_direct_control": "tracked",
            "default_work_control": "tracked",
            "light": {
                "enabled": false,
                "max_intended_paths": 3,
                "allowed_path_patterns": [],
                "denied_path_patterns": [],
                "final_acceptance": "policy_dependent"
            },
            "write_ticket": {"idle_timeout_minutes": null}
        }
    });
    let policy_fingerprint = canonical_json_sha256(&policy)?.into_inner();
    let policy = serde_json::from_value::<ProjectWorkflowPolicy>(policy)?;
    let context = project_fixture.mutation_context()?;
    let mut store = CoreProjectStore::open_for_mutation(
        &context,
        &ProjectId::new(project_fixture.project_id()),
    )?;
    store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
        policy_version: 1,
        policy,
        policy_fingerprint,
        source: ProjectWorkflowPolicySource::ProjectDatabase,
        expected_prior_fingerprint: None,
    })?;
    let error = CoreService::for_read_only(project_fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(project_fixture.connection_id()),
            ProjectId::new(project_fixture.project_id()),
            project_session.runtime_session_id,
            project_session.project_session_id,
            OperationCategory::Read,
        )
        .expect_err("a session from the prior project revision must be stale");
    assert_eq!(error.reason(), "agent_project_session_revision_stale");
    Ok(())
}
