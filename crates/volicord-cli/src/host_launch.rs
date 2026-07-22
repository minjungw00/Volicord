//! Hidden managed-host MCP bootstrap.

use std::{ffi::OsString, fmt, path::Path};

use serde_json::Value;
use volicord_mcp::{
    resolve_repository_discovery_runtime_home, resolve_runtime_home, run_managed_stdio,
    ManagedMcpLaunchSpec, McpAdapter, McpConnectionContext, RepositoryDiscoveryResolution,
};
use volicord_store::{
    agent_connections::{
        agent_connection_record_read_only, AgentConnectionRecord, CONNECTION_INTENT_PERSONAL,
        CONNECTION_INTENT_SHARED, HOST_SCOPE_PROJECT, HOST_SCOPE_USER,
    },
    bootstrap::require_installation_profile_read_only,
    managed_launch_leases::{
        cancel_managed_mcp_launch_lease, issue_managed_mcp_launch_lease,
        ManagedMcpLaunchLeaseConsumption, ManagedMcpLaunchLeaseIssue,
    },
    operational_sessions::connection_integration_revision,
};
use volicord_types::{ConnectionIntent, HostKind, HostScope};

use crate::host_integration::{
    codex::{
        managed_identity_evaluation_for_plan, CodexAdapter, CodexEnvironment,
        CodexExistingPlanRequest,
    },
    verification::ManagedConfigStatus,
};

/// Connection selection accepted by the internal launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostLaunchBinding {
    Connection(String),
    DiscoverRepository,
}

/// Runs the hidden launcher and transitions directly into managed stdio.
pub fn run_host_launch<F>(
    host: HostKind,
    binding: HostLaunchBinding,
    env_var: F,
    current_dir: &Path,
) -> Result<(), HostLaunchError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let (runtime_home, context) = match binding {
        HostLaunchBinding::Connection(connection_id) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            let context = McpConnectionContext::resolve(&runtime_home, connection_id)?;
            (runtime_home, context)
        }
        HostLaunchBinding::DiscoverRepository => {
            let runtime_home = resolve_repository_discovery_runtime_home(&env_var, current_dir)?;
            let resolution =
                RepositoryDiscoveryResolution::resolve(&runtime_home, current_dir, host)?;
            (runtime_home, resolution.context)
        }
    };

    let connection =
        agent_connection_record_read_only(&runtime_home, context.connection_internal_id.as_str())?
            .ok_or_else(|| HostLaunchError::contract("selected Agent Connection disappeared"))?;
    let plan = current_managed_plan(&runtime_home, host, &connection)?;
    let evaluation = managed_identity_evaluation_for_plan(&plan)?;
    if evaluation.status != ManagedConfigStatus::Match {
        return Err(HostLaunchError::contract(format!(
            "managed host configuration is not current: {}",
            evaluation.status.as_str()
        )));
    }
    if plan.fingerprint != connection.managed_fingerprint {
        return Err(HostLaunchError::contract(
            "managed host configuration fingerprint differs from the current Connection",
        ));
    }

    let revision = connection_integration_revision(&connection)?;
    let lease = issue_managed_mcp_launch_lease(
        &runtime_home,
        ManagedMcpLaunchLeaseIssue {
            connection_internal_id: connection.connection_internal_id.clone(),
            host_kind: host,
            expected_integration_revision: revision.as_str().to_owned(),
            expected_launch_fingerprint: connection.managed_fingerprint.clone(),
        },
    )?;
    let lease_id = lease.launch_lease_id.clone();
    let claim = ManagedMcpLaunchLeaseConsumption {
        launch_lease_id: lease.launch_lease_id,
        connection_internal_id: lease.connection_internal_id,
        host_kind: lease.host_kind,
        expected_integration_revision: lease.expected_integration_revision,
        expected_launch_fingerprint: lease.expected_launch_fingerprint,
    };
    let launch_result =
        run_managed_stdio(McpAdapter::new(runtime_home.clone(), context), claim, None)
            .map_err(HostLaunchError::from);
    let cleanup_result = cancel_managed_mcp_launch_lease(&runtime_home, &lease_id)
        .map(|_| ())
        .map_err(HostLaunchError::from);
    match (launch_result, cleanup_result) {
        (Err(error), _) => Err(error),
        (Ok(()), result) => result,
    }
}

fn current_managed_plan(
    runtime_home: &Path,
    host: HostKind,
    connection: &AgentConnectionRecord,
) -> Result<crate::host_integration::HostPlan, HostLaunchError> {
    if !connection.enabled {
        return Err(HostLaunchError::contract(
            "selected Agent Connection is disabled",
        ));
    }
    if connection.host_kind != host.as_str() {
        return Err(HostLaunchError::contract(
            "selected Agent Connection belongs to a different host",
        ));
    }
    let (intent, scope) = match (connection.intent.as_str(), connection.host_scope.as_str()) {
        (CONNECTION_INTENT_PERSONAL, HOST_SCOPE_USER) => {
            (ConnectionIntent::Personal, HostScope::User)
        }
        (CONNECTION_INTENT_SHARED, HOST_SCOPE_PROJECT) => {
            (ConnectionIntent::Shared, HostScope::Project)
        }
        _ => {
            return Err(HostLaunchError::contract(
                "selected Agent Connection has an invalid intent and scope binding",
            ))
        }
    };
    let metadata = metadata_object(&connection.metadata_json)?;
    let configured_command = metadata_string(&metadata, "mcp_command")?;
    let profile = require_installation_profile_read_only(runtime_home)?;
    let command = configured_command.unwrap_or(profile.volicord_mcp_command.as_str());
    let configured_runtime_home = metadata_string(&metadata, "host_runtime_home")?;
    let plan_runtime_home = match scope {
        HostScope::User => {
            let configured = configured_runtime_home.ok_or_else(|| {
                HostLaunchError::contract(
                    "personal Connection metadata is missing host_runtime_home",
                )
            })?;
            if Path::new(configured) != runtime_home {
                return Err(HostLaunchError::contract(
                    "personal Connection is bound to a different Runtime Home",
                ));
            }
            Some(runtime_home)
        }
        HostScope::Project => {
            if configured_runtime_home.is_some() {
                return Err(HostLaunchError::contract(
                    "shared Connection metadata must not bind a static Runtime Home",
                ));
            }
            None
        }
    };
    let command = if scope == HostScope::Project {
        Path::new(ManagedMcpLaunchSpec::PATH_COMMAND)
    } else {
        Path::new(command)
    };
    CodexAdapter::new(CodexEnvironment::default())
        .plan_existing(CodexExistingPlanRequest {
            connection_intent: intent,
            scope,
            connection_id: &connection.connection_internal_id,
            server_name: &connection.server_name,
            config_target: Path::new(&connection.config_target),
            mcp_command: command,
            runtime_home: plan_runtime_home,
            mode: &connection.mode,
        })
        .map_err(HostLaunchError::from)
}

fn metadata_object(text: &str) -> Result<serde_json::Map<String, Value>, HostLaunchError> {
    let value = serde_json::from_str::<Value>(text)
        .map_err(|_| HostLaunchError::contract("Connection metadata is not valid JSON"))?;
    let Value::Object(object) = value else {
        return Err(HostLaunchError::contract(
            "Connection metadata is not an object",
        ));
    };
    Ok(object)
}

fn metadata_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, HostLaunchError> {
    object
        .get(key)
        .map(|value| {
            value.as_str().ok_or_else(|| {
                HostLaunchError::contract(format!("Connection metadata field {key} is not text"))
            })
        })
        .transpose()
}

/// Bounded launcher failure without lease material.
#[derive(Debug)]
pub struct HostLaunchError {
    detail: String,
}

impl HostLaunchError {
    fn contract(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for HostLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for HostLaunchError {}

impl From<volicord_store::StoreError> for HostLaunchError {
    fn from(error: volicord_store::StoreError) -> Self {
        Self::contract(error.to_string())
    }
}

impl From<volicord_mcp::McpAdapterError> for HostLaunchError {
    fn from(error: volicord_mcp::McpAdapterError) -> Self {
        Self::contract(error.to_string())
    }
}

impl From<crate::host_integration::HostConfigError> for HostLaunchError {
    fn from(error: crate::host_integration::HostConfigError) -> Self {
        Self::contract(error.to_string())
    }
}
