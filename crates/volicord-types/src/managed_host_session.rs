use std::{error::Error, fmt};

use sha2::{Digest, Sha256};

/// Domain separator for managed-host session identifiers.
pub const MANAGED_HOST_SESSION_DOMAIN: &[u8] = b"volicord-managed-host-session-v1\0";

/// Prefix for opaque managed-host session identifiers.
pub const MANAGED_HOST_SESSION_ID_PREFIX: &str = "mhs_";

/// Maximum accepted byte length for a host-native session identifier.
pub const MAX_MANAGED_HOST_NATIVE_SESSION_ID_BYTES: usize = 256;

/// Validation failure for a managed-host session binding input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedHostSessionIdError {
    /// The host is not one of the managed built-in adapters.
    UnsupportedHostKind,
    /// The registered connection coordinate is missing or ambiguous.
    InvalidConnectionInternalId,
    /// The raw host-native session identifier is outside its strict value set.
    InvalidNativeSessionId,
    /// An internal override is not a canonical opaque managed-host session identifier.
    InvalidManagedHostSessionId,
}

impl fmt::Display for ManagedHostSessionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedHostKind => {
                "managed-host session binding requires host kind codex or claude_code"
            }
            Self::InvalidConnectionInternalId => {
                "managed-host session binding requires a non-empty connection internal id"
            }
            Self::InvalidNativeSessionId => {
                "native managed-host session id must be 1 through 256 bytes and match [A-Za-z0-9._:-]+"
            }
            Self::InvalidManagedHostSessionId => {
                "managed-host session id must be mhs_ followed by 64 lowercase hexadecimal characters"
            }
        })
    }
}

impl Error for ManagedHostSessionIdError {}

/// Validates one raw host-native session identifier without rendering its value.
pub fn validate_managed_host_native_session_id(
    native_session_id: &str,
) -> Result<(), ManagedHostSessionIdError> {
    let bytes = native_session_id.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_MANAGED_HOST_NATIVE_SESSION_ID_BYTES
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(ManagedHostSessionIdError::InvalidNativeSessionId);
    }
    Ok(())
}

/// Validates the canonical representation accepted for an internal session override.
pub fn validate_managed_host_session_id(
    managed_host_session_id: &str,
) -> Result<(), ManagedHostSessionIdError> {
    let Some(digest) = managed_host_session_id.strip_prefix(MANAGED_HOST_SESSION_ID_PREFIX) else {
        return Err(ManagedHostSessionIdError::InvalidManagedHostSessionId);
    };
    if digest.len() != 64
        || !digest
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(*byte, b'a'..=b'f'))
    {
        return Err(ManagedHostSessionIdError::InvalidManagedHostSessionId);
    }
    Ok(())
}

/// Maps a host-native session identifier to its opaque Volicord binding.
pub fn managed_host_session_id(
    host_kind: &str,
    connection_internal_id: &str,
    native_session_id: &str,
) -> Result<String, ManagedHostSessionIdError> {
    if !matches!(host_kind, "codex" | "claude_code") {
        return Err(ManagedHostSessionIdError::UnsupportedHostKind);
    }
    if connection_internal_id.is_empty() || connection_internal_id.as_bytes().contains(&0) {
        return Err(ManagedHostSessionIdError::InvalidConnectionInternalId);
    }
    validate_managed_host_native_session_id(native_session_id)?;

    let mut digest = Sha256::new();
    digest.update(MANAGED_HOST_SESSION_DOMAIN);
    digest.update(host_kind.as_bytes());
    digest.update([0]);
    digest.update(connection_internal_id.as_bytes());
    digest.update([0]);
    digest.update(native_session_id.as_bytes());
    Ok(format!(
        "{MANAGED_HOST_SESSION_ID_PREFIX}{:x}",
        digest.finalize()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_host_session_binding_has_stable_domain_separated_vector() {
        assert_eq!(
            managed_host_session_id("codex", "conn_alpha", "thread:alpha-1")
                .expect("valid coordinates should bind"),
            "mhs_a1ccf00f94f7344355eb8d42a73adbb7aa2cd4bb4f70ecff1b39b8e2830ed53d"
        );
    }

    #[test]
    fn every_binding_coordinate_changes_the_digest() {
        let baseline = managed_host_session_id("codex", "conn_alpha", "session.alpha")
            .expect("baseline coordinates should bind");
        for changed in [
            managed_host_session_id("claude_code", "conn_alpha", "session.alpha"),
            managed_host_session_id("codex", "conn_beta", "session.alpha"),
            managed_host_session_id("codex", "conn_alpha", "session.beta"),
        ] {
            assert_ne!(changed.expect("changed coordinates should bind"), baseline);
        }
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
                Err(ManagedHostSessionIdError::InvalidNativeSessionId)
            );
        }
    }

    #[test]
    fn canonical_override_validation_rejects_aliases_and_malformed_values() {
        let valid = managed_host_session_id("claude_code", "conn", "native")
            .expect("valid coordinates should bind");
        assert!(validate_managed_host_session_id(&valid).is_ok());
        for invalid in [
            valid.to_uppercase(),
            valid.replacen("mhs_", "session_", 1),
            "mhs_deadbeef".to_owned(),
            format!("mhs_{}g", "a".repeat(63)),
        ] {
            assert_eq!(
                validate_managed_host_session_id(&invalid),
                Err(ManagedHostSessionIdError::InvalidManagedHostSessionId)
            );
        }
    }

    #[test]
    fn unsupported_or_missing_coordinates_fail_closed() {
        assert_eq!(
            managed_host_session_id("generic", "conn", "native"),
            Err(ManagedHostSessionIdError::UnsupportedHostKind)
        );
        assert_eq!(
            managed_host_session_id("codex", "", "native"),
            Err(ManagedHostSessionIdError::InvalidConnectionInternalId)
        );
    }
}
