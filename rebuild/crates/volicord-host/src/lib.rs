//! Reconstruction-owned high-level Codex/MCP Host Adapter.
//!
//! The adapter translates JSON-RPC transport into product use cases. It does
//! not expose database CRUD, invent user judgment, or own canonical meaning.

mod mcp;

pub use mcp::{run_stdio, HostAdapter, HostError, HOST_TOOL_NAMES};
