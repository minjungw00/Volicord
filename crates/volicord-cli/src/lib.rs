#![forbid(unsafe_code)]

//! Shared administrative CLI implementation pieces.
//!
//! The binary owns process entry/exit. Library modules are kept reusable so
//! administrative command behavior can be tested without invoking the binary.

pub mod changes_command;
pub mod cli;
pub mod connection_command;
pub mod diagnostics_command;
mod disclosure;
pub mod doctor_command;
pub mod evidence_command;
pub mod export_command;
pub mod guard_command;
mod guard_integration;
pub mod host_integration;
mod managed_block;
pub mod policy_command;
pub mod project_context;
pub mod registration;
pub mod setup_command;
mod setup_report;
mod shell_path;
mod summary_card;
pub mod user_command;
