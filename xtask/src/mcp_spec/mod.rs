//! Pinned MCP specification maintenance ownership.

mod manifest;
mod report;
mod sync;
mod validation;

use anyhow::Result;
use std::path::Path;

pub use report::{McpSpecCheckReport, McpSpecSyncReport};
pub use sync::run_mcp_spec_sync;
pub use validation::{check_mcp_spec_fixture, check_mcp_spec_fixture_with_production_profiles};

const FIXTURE_PATH: &str = "tests/conformance/mcp-spec";

pub fn run_mcp_spec_check(root: &Path) -> Result<McpSpecCheckReport> {
    validation::require_repository_root(root)?;
    check_mcp_spec_fixture(&root.join(FIXTURE_PATH))
}
