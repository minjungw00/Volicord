mod adapter;
mod cli;
mod config;
mod parser;

use crate::host_integration::HostCapabilities;

pub use adapter::ClaudeCodeAdapter;
pub use cli::{CommandInvocation, CommandOutput, CommandRunner, ProductionCommandRunner};

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        http_mcp: false,
        session_start_hook: true,
        pre_tool_hook: true,
        post_tool_hook: true,
        user_prompt_submit_hook: true,
        stop_hook: true,
        rule_file_support: true,
        project_local_configuration: true,
    }
}

pub(crate) use config::{project_rule_block, project_rule_path, project_settings_path};
