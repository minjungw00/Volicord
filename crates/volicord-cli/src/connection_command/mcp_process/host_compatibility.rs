use serde_json::{json, Value};
use volicord_host_contract::HostContractProfileId;
use volicord_mcp_protocol::McpProtocolRevision;

/// An independently reviewed managed-host compatibility family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostCompatibilityProfile {
    Codex,
}

impl HostCompatibilityProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HostCompatibilityFixture {
    pub(super) profile: HostCompatibilityProfile,
    pub(super) fixture_id: &'static str,
    pub(super) revision: McpProtocolRevision,
    client_name: &'static str,
    client_title: &'static str,
    client_version: &'static str,
}

impl HostCompatibilityFixture {
    pub(super) fn initialize_params(self) -> Value {
        json!({
            "protocolVersion": self.revision.as_str(),
            "capabilities": {},
            "clientInfo": {
                "name": self.client_name,
                "title": self.client_title,
                "version": self.client_version,
            }
        })
    }

    pub(super) fn call_metadata(self) -> Value {
        json!({
            "threadId": "codex.compatibility.thread",
            "x-codex-turn-metadata": {
                "session_id": "codex.compatibility.session",
                "thread_id": "codex.compatibility.thread",
                "turn_id": "codex.compatibility.turn",
            }
        })
    }
}

// The host fixture explicitly selects the semantic turn-metadata contract.
const REVIEWED_CODEX_MCP_FIXTURE: HostCompatibilityFixture = HostCompatibilityFixture {
    profile: HostCompatibilityProfile::Codex,
    fixture_id: HostContractProfileId::CodexMcpTurnMetadata.as_str(),
    revision: McpProtocolRevision::V20250618,
    client_name: "codex-mcp-client",
    client_title: "Codex",
    client_version: "0.108.0-alpha.12",
};

const HOST_COMPATIBILITY_FIXTURES: [HostCompatibilityFixture; 1] = [REVIEWED_CODEX_MCP_FIXTURE];

pub(super) fn fixtures() -> &'static [HostCompatibilityFixture] {
    &HOST_COMPATIBILITY_FIXTURES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_identity_names_the_semantic_wire_contract() {
        let fixture = fixtures()
            .iter()
            .find(|fixture| fixture.profile == HostCompatibilityProfile::Codex)
            .expect("Codex compatibility fixture");
        assert_eq!(fixture.revision, McpProtocolRevision::V20250618);
        assert_eq!(
            fixture.fixture_id,
            HostContractProfileId::CodexMcpTurnMetadata.as_str()
        );
        assert!(!fixture.fixture_id.contains("0.108.0"));
        assert_eq!(fixture.initialize_params()["capabilities"], json!({}));
        assert_eq!(
            fixture.initialize_params()["clientInfo"],
            json!({
                "name": "codex-mcp-client",
                "title": "Codex",
                "version": "0.108.0-alpha.12",
            })
        );
        assert_eq!(
            fixture.call_metadata()["x-codex-turn-metadata"]["thread_id"],
            fixture.call_metadata()["threadId"]
        );
    }
}
