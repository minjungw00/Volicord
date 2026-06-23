#![forbid(unsafe_code)]

//! Shared administrative CLI implementation pieces.
//!
//! The binary owns process entry/exit. Library modules are kept reusable so
//! administrative command behavior can be tested without invoking the binary.

pub mod agent_command;
pub mod guidance_template;
pub mod host_integration;
pub mod registration;
pub mod repository_guidance;
