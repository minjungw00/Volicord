use std::{error::Error, fmt};

use sha2::{Digest, Sha256};

/// Maximum accepted UTF-8 byte length for each managed MCP `clientInfo` field.
pub const MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES: usize = 256;

/// Exact MCP `clientInfo.name` accepted for the managed Codex stdio boundary.
pub const CODEX_MANAGED_MCP_CLIENT_NAME: &str = "codex-mcp-client";

/// Maximum accepted byte length for one host-native managed stdio session identifier.
pub const MAX_MANAGED_HOST_NATIVE_SESSION_ID_BYTES: usize = 256;

const PROJECT_AGENT_SESSION_DOMAIN: &[u8] = b"volicord.project-agent-session\0";
const PROJECT_AGENT_SESSION_ID_PREFIX: &str = "agent_session_";

/// Validation failure for one host-native managed stdio session identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedHostNativeSessionIdError;

impl fmt::Display for ManagedHostNativeSessionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "native managed-host session id must be 1 through 256 bytes and match [A-Za-z0-9._:-]+",
        )
    }
}

impl Error for ManagedHostNativeSessionIdError {}

/// Validation failure for an internal project Agent Session coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAgentSessionIdError {
    InvalidConnectionInternalId,
    InvalidProjectIntegrationRevision,
    InvalidNativeSessionId,
    InvalidSessionId,
}

impl fmt::Display for ProjectAgentSessionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConnectionInternalId => {
                "project Agent Session requires a non-empty Agent Connection identity"
            }
            Self::InvalidProjectIntegrationRevision => {
                "project Agent Session requires a canonical project integration revision"
            }
            Self::InvalidNativeSessionId => {
                "project Agent Session requires an exact host-native session correlation coordinate"
            }
            Self::InvalidSessionId => {
                "project Agent Session must use the canonical internal digest coordinate"
            }
        })
    }
}

impl Error for ProjectAgentSessionIdError {}

/// Validates one exact host-native session identifier used by managed stdio.
pub fn validate_managed_host_native_session_id(
    native_session_id: &str,
) -> Result<(), ManagedHostNativeSessionIdError> {
    let bytes = native_session_id.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_MANAGED_HOST_NATIVE_SESSION_ID_BYTES
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(ManagedHostNativeSessionIdError);
    }
    Ok(())
}

/// Builds the private revision-scoped coordinate used for one project Agent Session.
pub fn project_agent_session_id(
    connection_internal_id: &str,
    project_integration_revision: &str,
    native_session_id: &str,
) -> Result<String, ProjectAgentSessionIdError> {
    if connection_internal_id.is_empty() || connection_internal_id.as_bytes().contains(&0) {
        return Err(ProjectAgentSessionIdError::InvalidConnectionInternalId);
    }
    crate::IntegrationRevision::parse(project_integration_revision.to_owned())
        .map_err(|_| ProjectAgentSessionIdError::InvalidProjectIntegrationRevision)?;
    validate_managed_host_native_session_id(native_session_id)
        .map_err(|_| ProjectAgentSessionIdError::InvalidNativeSessionId)?;
    let mut digest = Sha256::new();
    digest.update(PROJECT_AGENT_SESSION_DOMAIN);
    digest.update(connection_internal_id.as_bytes());
    digest.update([0]);
    digest.update(project_integration_revision.as_bytes());
    digest.update([0]);
    digest.update(native_session_id.as_bytes());
    Ok(format!(
        "{PROJECT_AGENT_SESSION_ID_PREFIX}{:x}",
        digest.finalize()
    ))
}

/// Validates one internal project Agent Session coordinate read from storage.
pub fn validate_project_agent_session_id(
    session_id: &str,
) -> Result<(), ProjectAgentSessionIdError> {
    let Some(digest) = session_id.strip_prefix(PROJECT_AGENT_SESSION_ID_PREFIX) else {
        return Err(ProjectAgentSessionIdError::InvalidSessionId);
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(ProjectAgentSessionIdError::InvalidSessionId);
    }
    Ok(())
}

/// One field in the closed managed MCP initialized-client information pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedMcpClientInfoField {
    Name,
    Version,
}

impl ManagedMcpClientInfoField {
    /// Returns the exact MCP field path represented by this value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Name => "clientInfo.name",
            Self::Version => "clientInfo.version",
        }
    }
}

/// Validation failure for one managed MCP initialized-client information field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedMcpClientInfoError {
    field: ManagedMcpClientInfoField,
}

impl ManagedMcpClientInfoError {
    /// Returns the invalid field without retaining its rejected value.
    pub const fn field(self) -> ManagedMcpClientInfoField {
        self.field
    }
}

impl fmt::Display for ManagedMcpClientInfoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} must be 1 through {} UTF-8 bytes, contain a non-whitespace character, and contain no control character",
            self.field.as_str(),
            MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES
        )
    }
}

impl Error for ManagedMcpClientInfoError {}

/// Validates one exact managed MCP `clientInfo` field without normalizing it.
pub fn validate_managed_mcp_client_info_field(
    field: ManagedMcpClientInfoField,
    value: &str,
) -> Result<(), ManagedMcpClientInfoError> {
    if value.is_empty()
        || value.len() > MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES
        || value.chars().all(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err(ManagedMcpClientInfoError { field });
    }
    Ok(())
}

/// Bounded diagnostic client information reported by one successful managed MCP initialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpClientInfo {
    name: String,
    version: String,
}

impl ManagedMcpClientInfo {
    /// Validates and retains the exact accepted `clientInfo` pair.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, ManagedMcpClientInfoError> {
        let name = name.into();
        let version = version.into();
        validate_managed_mcp_client_info_field(ManagedMcpClientInfoField::Name, &name)?;
        validate_managed_mcp_client_info_field(ManagedMcpClientInfoField::Version, &version)?;
        Ok(Self { name, version })
    }

    /// Returns the exact accepted `clientInfo.name`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the exact accepted `clientInfo.version`.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the exact accepted pair as owned strings.
    pub fn into_parts(self) -> (String, String) {
        (self.name, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_info_accepts_exact_byte_bound_and_preserves_strings() {
        let name = format!(" {} ", "n".repeat(254));
        let version = " 릴리스 1.0 ";
        let client_info = ManagedMcpClientInfo::new(name.clone(), version)
            .expect("bounded nonblank values should be accepted");

        assert_eq!(name.len(), MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES);
        assert_eq!(client_info.name(), name);
        assert_eq!(client_info.version(), version);
    }

    #[test]
    fn client_info_bound_is_utf8_bytes_not_characters() {
        let at_limit = "가".repeat(85) + "a";
        let over_limit = "가".repeat(85) + "ab";

        assert_eq!(at_limit.len(), MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES);
        assert_eq!(
            over_limit.len(),
            MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES + 1
        );
        assert!(ManagedMcpClientInfo::new(&at_limit, "1").is_ok());
        assert_eq!(
            ManagedMcpClientInfo::new("name", over_limit)
                .expect_err("an over-limit UTF-8 value must fail")
                .field(),
            ManagedMcpClientInfoField::Version
        );
    }

    #[test]
    fn native_session_validation_is_exact_and_bounded() {
        for valid in ["a", "A0._:-", &"x".repeat(256)] {
            assert!(validate_managed_host_native_session_id(valid).is_ok());
        }
        for invalid in [
            "",
            "has space",
            "has/slash",
            "line\nbreak",
            "세션",
            &"x".repeat(257),
        ] {
            assert_eq!(
                validate_managed_host_native_session_id(invalid),
                Err(ManagedHostNativeSessionIdError)
            );
        }
    }

    #[test]
    fn project_agent_session_coordinate_is_revision_scoped_and_exact() {
        let revision_a = format!("sha256:{}", "a".repeat(64));
        let revision_b = format!("sha256:{}", "b".repeat(64));
        let first = project_agent_session_id("connection-a", &revision_a, "native-session")
            .expect("valid context should bind");
        let replay = project_agent_session_id("connection-a", &revision_a, "native-session")
            .expect("same context should replay");
        let other_connection =
            project_agent_session_id("connection-b", &revision_a, "native-session")
                .expect("other connection should bind");
        let other_revision =
            project_agent_session_id("connection-a", &revision_b, "native-session")
                .expect("other revision should bind");
        let other_native =
            project_agent_session_id("connection-a", &revision_a, "native-session-other")
                .expect("other native session should bind");
        assert_eq!(first, replay);
        assert_ne!(first, other_connection);
        assert_ne!(first, other_revision);
        assert_ne!(first, other_native);
        assert!(
            project_agent_session_id("connection-a", "not-a-revision", "native-session").is_err()
        );
        assert!(validate_project_agent_session_id(&first).is_ok());
        assert!(validate_project_agent_session_id(&first.to_uppercase()).is_err());
    }

    #[test]
    fn client_info_rejects_empty_whitespace_control_and_oversize_fields() {
        for invalid in ["", " \t\u{2003}", "line\nbreak", &"x".repeat(257)] {
            assert_eq!(
                ManagedMcpClientInfo::new(invalid, "1")
                    .expect_err("invalid names must fail")
                    .field(),
                ManagedMcpClientInfoField::Name
            );
            assert_eq!(
                ManagedMcpClientInfo::new("name", invalid)
                    .expect_err("invalid versions must fail")
                    .field(),
                ManagedMcpClientInfoField::Version
            );
        }
    }
}
