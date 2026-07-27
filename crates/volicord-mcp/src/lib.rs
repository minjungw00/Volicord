#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! Local MCP adapter for public Volicord method calls.
//!
//! This crate owns transport dispatch. It binds one MCP server process to one
//! Agent Connection, derives adapter-owned invocation facts, decodes tool
//! arguments into `volicord-types` request structs, and hands execution to
//! `volicord-core`.

mod adapter;
mod authority_refresh;
mod binding;
mod build_info;
mod committed_result_recovery;
mod constants;
mod diagnostics;
mod errors;
mod json_rpc;
mod lifecycle;
mod managed_launch;
mod mutation_admission;
mod mutation_projection;
#[cfg(test)]
mod prelude;
#[cfg(test)]
mod protocol_projection_tests;
mod routing;
mod schema_validation;
mod session_metrics;
mod stdio;
mod telemetry;
#[cfg(test)]
mod tests;
mod tool_dispatch;
mod tool_registry;
mod transport;
mod user_action_projection;
mod util;

pub use adapter::McpAdapter;
pub use binding::{
    preflight_check, resolve_repository_discovery_runtime_home, resolve_runtime_home,
    resolve_runtime_home_from_env, run_preflight_check_from_env,
};
pub use build_info::{build_id, build_info, BuildInfo};
pub use diagnostics::{bootstrap_diagnostic_envelope, diagnostic_codes};
pub use errors::McpAdapterError;
pub use managed_launch::{
    is_managed_mcp_launch_environment_name, LaunchEnvironment, ManagedMcpBinding,
    ManagedMcpInvocationPurpose, ManagedMcpLaunchError, ManagedMcpLaunchSpec,
    ManagedMcpMaterializationInput, ManagedMcpWorkingDirectory, MaterializedManagedMcpLaunch,
    RuntimeHomeBinding, MANAGED_MCP_LAUNCH_ENVIRONMENT_NAMES, VOLICORD_HOME_ENV,
};
pub use routing::{
    McpConnectionContext, McpConnectionStartupInspection, McpPreflightHostContract,
    McpPreflightHostToolIdentity, McpPreflightProject, McpPreflightReport,
    McpPreflightWriteability, McpProjectAvailability, RepositoryDiscoveryResolution,
};
pub use stdio::{
    run_managed_stdio, run_stdio, run_stdio_discover_repository_from_env, run_stdio_from_env,
};
pub use tool_registry::{
    adapter_utility_tools, canonical_mcp_tool_catalog, effective_mcp_tool_catalog, mcp_tools,
    mcp_tools_for_mode, public_method_tools, CanonicalContent, CanonicalToolDefinition,
    CanonicalToolResult,
};
