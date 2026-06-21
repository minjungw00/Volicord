#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    Complete,
    ActionRequired,
    Missing,
    Rejected,
    Failed,
    NotVerified,
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Missing => "missing",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
            Self::NotVerified => "not_verified",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub status: VerificationStatus,
    pub details: String,
}

impl Verification {
    pub fn new(status: VerificationStatus, details: impl Into<String>) -> Self {
        Self {
            status,
            details: details.into(),
        }
    }
}

pub fn classify_claude_mcp_status(stdout: &str, stderr: &str, success: bool) -> Verification {
    let combined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if combined.contains("pending approval") {
        return Verification::new(
            VerificationStatus::ActionRequired,
            "Claude Code reports the MCP server is pending project approval",
        );
    }
    if combined.contains("rejected") {
        return Verification::new(
            VerificationStatus::Rejected,
            "Claude Code reports the MCP server was rejected",
        );
    }
    if combined.contains("not found")
        || combined.contains("no mcp server")
        || combined.contains("does not exist")
    {
        return Verification::new(
            VerificationStatus::Missing,
            "Claude Code did not report a configured MCP server with that name",
        );
    }
    if success
        && (combined.contains("connected")
            || combined.contains("✓")
            || combined.contains("stdio")
            || combined.contains("command"))
    {
        return Verification::new(
            VerificationStatus::Complete,
            "Claude Code reports a configured MCP server",
        );
    }
    Verification::new(
        VerificationStatus::Failed,
        "Claude Code MCP status could not be confirmed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_status_distinguishes_pending_and_rejected() {
        assert_eq!(
            classify_claude_mcp_status("⏸ Pending approval", "", true).status,
            VerificationStatus::ActionRequired
        );
        assert_eq!(
            classify_claude_mcp_status("✗ Rejected", "", true).status,
            VerificationStatus::Rejected
        );
        assert_eq!(
            classify_claude_mcp_status("Server not found", "", false).status,
            VerificationStatus::Missing
        );
    }
}
