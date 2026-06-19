#![forbid(unsafe_code)]

//! Shared administrative CLI implementation pieces.
//!
//! The binary owns process entry/exit. Library modules are kept reusable so
//! setup planning and orchestration can be tested without invoking the binary.

pub mod host_config;
pub mod local_mcp_command;
pub mod registration;
pub mod setup;
