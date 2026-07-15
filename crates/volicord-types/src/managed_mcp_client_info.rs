use std::{error::Error, fmt};

/// Maximum accepted UTF-8 byte length for each managed MCP `clientInfo` field.
pub const MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES: usize = 256;

/// One field in the closed managed MCP initialized-client identity pair.
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

/// Validation failure for one managed MCP initialized-client identity field.
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

/// Closed, validated identity reported by one successful managed MCP initialize.
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
