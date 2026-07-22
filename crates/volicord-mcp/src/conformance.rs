use volicord_mcp_protocol::McpProtocolRevision;

/// Revisions covered by Volicord's repository-owned offline runtime
/// conformance matrix.
///
/// This declaration drives the revision matrix exercised by the MCP adapter
/// and administrative CLI. It describes local repository coverage and is not
/// an upstream or third-party certification.
const VOLICORD_CONFORMANCE_COVERED_REVISIONS: [McpProtocolRevision; 5] = [
    McpProtocolRevision::V20241007,
    McpProtocolRevision::V20241105,
    McpProtocolRevision::V20250326,
    McpProtocolRevision::V20250618,
    McpProtocolRevision::V20251125,
];

/// Returns the exact revisions covered by Volicord's offline runtime
/// conformance matrix in its deterministic execution order.
pub const fn volicord_conformance_covered_revisions() -> &'static [McpProtocolRevision] {
    &VOLICORD_CONFORMANCE_COVERED_REVISIONS
}
