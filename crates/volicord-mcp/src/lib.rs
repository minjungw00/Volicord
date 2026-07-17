#![forbid(unsafe_code)]

//! Local MCP adapter for public Volicord method calls.
//!
//! This crate owns transport dispatch. It binds one MCP server process to one
//! Agent Connection, derives adapter-owned invocation facts, decodes tool
//! arguments into `volicord-types` request structs, and hands execution to
//! `volicord-core`.

mod adapter;
mod build_info;
mod constants;
mod errors;
mod host_authority;
mod prelude;
mod repository_discovery;
mod routing;
mod schema_validation;
mod stdio;
#[cfg(test)]
mod tests;
mod tool_registry;
mod util;

pub use adapter::{McpAdapter, McpAdapterBoundary, McpDerivedInvocationContext};
pub use build_info::{build_id, build_info, BuildInfo};
pub use constants::{
    ADAPTER_UTILITY_TOOL_NAMES, PUBLIC_METHOD_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES,
};
pub use errors::McpAdapterError;
pub use repository_discovery::{
    RepositoryDiscoveryDescriptor, RepositoryDiscoveryDescriptorError, RepositoryDiscoveryHost,
};
pub use routing::{
    McpConnectionContext, McpConnectionStartupInspection, McpProjectAvailability,
    RepositoryDiscoveryResolution,
};
pub use stdio::{
    managed_host_authority_preparation_required_from_env, preflight_check,
    resolve_repository_discovery_runtime_home, resolve_runtime_home, resolve_runtime_home_from_env,
    run_preflight_check_from_env, run_stdio, run_stdio_discover_repository_from_env,
    run_stdio_from_env,
};
pub use tool_registry::{
    adapter_utility_tools, mcp_tools, mcp_tools_for_mode, public_method_tools, McpToolAnnotations,
    McpToolDefinition,
};
