#![forbid(unsafe_code)]

//! Shared administrative CLI implementation pieces.
//!
//! The binary owns process entry/exit. Library modules are kept reusable so
//! administrative command behavior can be tested without invoking the binary.

pub mod changes_command;
pub mod connection_command;
pub mod doctor_command;
pub mod export_command;
pub mod guard_command;
pub mod host_integration;
mod managed_block;
pub mod project_context;
pub mod registration;
pub mod serve_command;
pub mod setup_command;
mod setup_report;
mod shell_path;
pub mod user_command;
