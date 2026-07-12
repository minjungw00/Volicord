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
mod http;
mod local_http;
mod local_web_consent;
mod prelude;
mod repository_discovery;
mod routing;
mod schema_validation;
mod stdio;
#[cfg(test)]
mod tests;
mod tool_registry;
mod util;

pub use adapter::{
    LocalWebConsentContext, McpAdapter, McpAdapterBoundary, McpDerivedInvocationContext,
};
pub use build_info::{build_id, build_info, BuildInfo};
pub use constants::{
    ADAPTER_UTILITY_TOOL_NAMES, PUBLIC_METHOD_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES,
};
pub use errors::{LocalHttpError, McpAdapterError};
pub use local_http::{
    generate_bearer_token, local_http_listen_is_container_wildcard, local_http_listen_is_loopback,
    run_local_http_server, LocalHttpListenScope, LocalHttpServerConfig, LocalHttpTokenSource,
    LOCAL_HTTP_MCP_ENDPOINT_PATH,
};
pub use repository_discovery::{
    RepositoryDiscoveryDescriptor, RepositoryDiscoveryDescriptorError, RepositoryDiscoveryHost,
};
pub use routing::{
    McpConnectionContext, McpConnectionStartupInspection, McpProjectAvailability,
    RepositoryDiscoveryResolution,
};
pub use stdio::{
    preflight_check, resolve_runtime_home, resolve_runtime_home_from_env,
    run_preflight_check_from_env, run_stdio, run_stdio_discover_repository_from_env,
    run_stdio_from_env,
};
pub use tool_registry::{
    adapter_utility_tools, mcp_tools, mcp_tools_for_mode, public_method_tools, McpToolAnnotations,
    McpToolDefinition,
};

#[cfg(test)]
pub(crate) use http::{HttpRequest, HttpResponse};
#[cfg(test)]
pub(crate) use local_http::LocalHttpServer;
