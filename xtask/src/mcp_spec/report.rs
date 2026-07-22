//! Deterministic MCP specification maintenance reports.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpSpecCheckReport {
    pub(super) pinned_revision_count: usize,
    pub(super) production_supported_count: usize,
    pub(super) pre_release_only_count: usize,
}

impl McpSpecCheckReport {
    pub fn pinned_revision_count(self) -> usize {
        self.pinned_revision_count
    }

    pub fn production_supported_count(self) -> usize {
        self.production_supported_count
    }

    pub fn pre_release_only_count(self) -> usize {
        self.pre_release_only_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpSpecSyncReport {
    pub(super) revision_count: usize,
    pub(super) artifact_count: usize,
}

impl McpSpecSyncReport {
    pub fn revision_count(self) -> usize {
        self.revision_count
    }

    pub fn artifact_count(self) -> usize {
        self.artifact_count
    }
}
