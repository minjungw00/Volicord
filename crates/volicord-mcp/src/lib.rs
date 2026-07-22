#![forbid(unsafe_code)]

//! Local MCP adapter for public Volicord method calls.
//!
//! This crate owns transport dispatch. It binds one MCP server process to one
//! Agent Connection, derives adapter-owned invocation facts, decodes tool
//! arguments into `volicord-types` request structs, and hands execution to
//! `volicord-core`.

mod adapter;
mod build_info;
mod conformance;
mod constants;
mod diagnostics;
mod errors;
mod managed_launch;
mod prelude;
#[cfg(test)]
mod protocol_projection_tests;
mod routing;
mod schema_validation;
mod stdio;
#[cfg(test)]
mod tests;
mod tool_registry;
mod util;

pub use adapter::{McpAdapter, McpAdapterBoundary};
pub use build_info::{build_id, build_info, BuildInfo};
pub use conformance::volicord_conformance_covered_revisions;
pub use constants::{
    ADAPTER_UTILITY_TOOL_NAMES, PUBLIC_METHOD_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES,
};
pub use diagnostics::bootstrap_diagnostic_envelope;
pub use errors::McpAdapterError;
pub use managed_launch::{
    is_managed_mcp_launch_environment_name, LaunchEnvironment, ManagedMcpBinding,
    ManagedMcpInvocationPurpose, ManagedMcpLaunchError, ManagedMcpLaunchSpec,
    ManagedMcpMaterializationInput, ManagedMcpWorkingDirectory, MaterializedManagedMcpLaunch,
    RuntimeHomeBinding, MANAGED_MCP_LAUNCH_ENVIRONMENT_NAMES, MANAGED_MCP_LAUNCH_VALUE,
    MANAGED_MCP_PROCESS_ENVIRONMENT_NAMES, VOLICORD_HOME_ENV, VOLICORD_MCP_CONNECTION_ID_ENV,
    VOLICORD_MCP_HOST_ENV, VOLICORD_MCP_LAUNCH_ENV, VOLICORD_MCP_VERIFICATION_ENV,
    VOLICORD_MCP_VERIFICATION_VALUE,
};
pub use routing::{
    McpConnectionContext, McpConnectionStartupInspection, McpProjectAvailability,
    RepositoryDiscoveryResolution,
};
pub use stdio::{
    preflight_check, resolve_repository_discovery_runtime_home, resolve_runtime_home,
    resolve_runtime_home_from_env, run_preflight_check_from_env, run_stdio,
    run_stdio_discover_repository_from_env, run_stdio_from_env,
};
pub use tool_registry::{
    adapter_utility_tools, mcp_tools, mcp_tools_for_mode, public_method_tools, CanonicalContent,
    CanonicalToolAnnotations, CanonicalToolDefinition, CanonicalToolResult,
    VersionedToolDefinition, VersionedToolResult,
};
