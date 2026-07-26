//! Public stdio facade.
//!
//! Process entry points bind a Runtime Home and Connection, then hand the
//! stream to the bounded transport. Protocol parsing, lifecycle, binding, and
//! tool dispatch stay in their responsibility-owned modules.

use crate::adapter::McpAdapter;
use crate::binding::{manual_adapter_from_process, repository_discovery_adapter_from_process};
use crate::errors::McpAdapterError;
use crate::transport::{run_stdio_transport, StdioRunOptions};
use std::io::{self, BufRead, Write};
use volicord_store::managed_launch_leases::ManagedMcpLaunchLeaseConsumption;
use volicord_types::integration_revision::McpRuntimeSessionSource;
use volicord_types::values::HostKind;

pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    run_stdio_transport(adapter, reader, writer, StdioRunOptions::default())
}

/// Runs the manual MCP stdio adapter from process environment and stdin/stdout.
pub fn run_stdio_from_env(
    connection_id: &str,
    project_id: Option<&str>,
    observed_host_executable_version: Option<String>,
) -> Result<(), McpAdapterError> {
    let adapter = manual_adapter_from_process(connection_id, project_id)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_transport(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManualCli,
            managed_lease: None,
            observed_host_executable_version,
        },
    )
}

/// Runs stdio from a clone-portable shared managed launch.
pub fn run_stdio_discover_repository_from_env(
    host: HostKind,
    observed_host_executable_version: Option<String>,
) -> Result<(), McpAdapterError> {
    let adapter = repository_discovery_adapter_from_process(host)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_transport(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManualCli,
            managed_lease: None,
            observed_host_executable_version,
        },
    )
}

/// Runs managed stdio only through an in-memory one-time launch-lease claim.
pub fn run_managed_stdio(
    adapter: McpAdapter,
    managed_lease: ManagedMcpLaunchLeaseConsumption,
    observed_host_executable_version: Option<String>,
) -> Result<(), McpAdapterError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_transport(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManagedHost,
            managed_lease: Some(managed_lease),
            observed_host_executable_version,
        },
    )
}

#[cfg(test)]
pub(crate) fn run_managed_stdio_with_test_lease<R, W>(
    adapter: McpAdapter,
    reader: R,
    writer: W,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    use volicord_store::agent_connections::agent_connection_record_read_only;
    use volicord_store::managed_launch_leases::{
        issue_managed_mcp_launch_lease, ManagedMcpLaunchLeaseIssue,
    };
    use volicord_store::operational_sessions::connection_integration_revision;

    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| McpAdapterError::Environment("test Connection disappeared".to_owned()))?;
    let revision = connection_integration_revision(&connection).map_err(McpAdapterError::Store)?;
    let lease = crate::mutation_admission::with_mcp_runtime_home_mutation(
        &adapter.runtime_home,
        "mcp.test_managed_launch",
        |context| {
            issue_managed_mcp_launch_lease(
                context,
                ManagedMcpLaunchLeaseIssue {
                    connection_internal_id: connection.connection_internal_id.clone(),
                    host_kind: HostKind::Codex,
                    expected_integration_revision: revision.as_str().to_owned(),
                    expected_launch_fingerprint: connection.managed_fingerprint.clone(),
                },
            )
            .map_err(McpAdapterError::Store)
        },
    )?;
    run_stdio_transport(
        adapter,
        reader,
        writer,
        StdioRunOptions {
            session_source: McpRuntimeSessionSource::ManagedHost,
            managed_lease: Some(ManagedMcpLaunchLeaseConsumption {
                launch_lease_id: lease.launch_lease_id,
                connection_internal_id: lease.connection_internal_id,
                host_kind: lease.host_kind,
                expected_integration_revision: lease.expected_integration_revision,
                expected_launch_fingerprint: lease.expected_launch_fingerprint,
            }),
            observed_host_executable_version: None,
        },
    )
}

#[cfg(test)]
pub(crate) fn run_manual_stdio_with_ignored_env_marker<R, W, F>(
    adapter: McpAdapter,
    reader: R,
    writer: W,
    env_var: F,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let _ = env_var;
    run_stdio(adapter, reader, writer)
}
