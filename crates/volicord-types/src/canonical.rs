use std::{collections::BTreeMap, error::Error, fmt};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::ids::RequestHash;

/// Serializes a value to deterministic, whitespace-free canonical JSON bytes.
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut json = serde_json::to_value(value)?;
    sort_json_value(&mut json);
    serde_json::to_vec(&json)
}

/// Returns the byte length of the deterministic canonical JSON representation.
pub fn canonical_json_size_bytes<T: Serialize>(value: &T) -> Result<usize, serde_json::Error> {
    canonical_json_bytes(value).map(|bytes| bytes.len())
}

/// Serializes a value to a deterministic canonical JSON string.
pub fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_json_bytes(value)?;
    String::from_utf8(bytes).map_err(|err| {
        serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    })
}

/// Computes a SHA-256 hash over canonical JSON bytes.
pub fn canonical_json_sha256<T: Serialize>(value: &T) -> Result<RequestHash, serde_json::Error> {
    let bytes = canonical_json_bytes(value)?;
    let digest = Sha256::digest(bytes);
    Ok(RequestHash::new(format!(
        "sha256:{}",
        lowercase_hex(&digest)
    )))
}

/// Computes a bare lowercase SHA-256 hex digest over canonical JSON bytes.
///
/// Evidence-capture content digests use the bare 64-character representation,
/// while request idempotency hashes use the `sha256:`-prefixed `RequestHash`.
pub fn canonical_json_bare_sha256<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_json_bytes(value)?;
    Ok(lowercase_hex(&Sha256::digest(bytes)))
}

/// Returns whether `value` is exactly one bare lowercase SHA-256 digest.
pub fn is_canonical_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

/// Returns whether `value` is exactly `sha256:` followed by a lowercase digest.
pub fn is_canonical_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(is_canonical_sha256_hex)
}

/// Computes the deterministic request hash used by later idempotency checks.
pub fn canonical_request_hash<T: Serialize>(request: &T) -> Result<RequestHash, serde_json::Error> {
    canonical_json_sha256(request)
}

/// Validates a full Git object ID and returns its canonical lowercase spelling.
///
/// SHA-1 and SHA-256 repositories use exactly 40 and 64 ASCII hexadecimal
/// characters respectively. No intermediate length, prefix, separator, or
/// surrounding whitespace is accepted.
pub fn canonical_git_object_id(value: &str) -> Result<String, GitObjectIdError> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitObjectIdError);
    }

    Ok(value.to_ascii_lowercase())
}

/// Validation failure for a full Git object ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitObjectIdError;

impl fmt::Display for GitObjectIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("Git object ID must contain exactly 40 or 64 ASCII hexadecimal characters")
    }
}

impl Error for GitObjectIdError {}

fn sort_json_value(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                sort_json_value(item);
            }
        }
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, mut child) in std::mem::take(map) {
                sort_json_value(&mut child);
                sorted.insert(key, child);
            }
            *map = sorted.into_iter().collect();
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{canonical_git_object_id, is_canonical_sha256_digest, is_canonical_sha256_hex};

    #[test]
    fn sha256_validators_accept_only_exact_lowercase_encodings() {
        let hex = "a".repeat(64);
        assert!(is_canonical_sha256_hex(&hex));
        assert!(is_canonical_sha256_digest(&format!("sha256:{hex}")));

        for invalid in [
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            format!("{}g", "a".repeat(63)),
        ] {
            assert!(!is_canonical_sha256_hex(&invalid));
            assert!(!is_canonical_sha256_digest(&format!("sha256:{invalid}")));
        }
        assert!(!is_canonical_sha256_digest(&hex));
        assert!(!is_canonical_sha256_digest(&format!("SHA256:{hex}")));
    }

    #[test]
    fn git_object_id_accepts_exact_sha1_and_sha256_lengths() {
        assert_eq!(canonical_git_object_id(&"a".repeat(40)), Ok("a".repeat(40)));
        assert_eq!(canonical_git_object_id(&"b".repeat(64)), Ok("b".repeat(64)));
    }

    #[test]
    fn git_object_id_rejects_every_adjacent_and_intermediate_length() {
        for length in [39, 41, 42, 62, 63, 65] {
            assert!(
                canonical_git_object_id(&"a".repeat(length)).is_err(),
                "length {length} must be rejected"
            );
        }
    }

    #[test]
    fn git_object_id_rejects_non_hex_and_non_ascii_input() {
        assert!(canonical_git_object_id(&format!("{}g", "a".repeat(39))).is_err());
        assert!(canonical_git_object_id(&format!("{}０", "a".repeat(39))).is_err());
    }

    #[test]
    fn git_object_id_canonicalizes_uppercase_hex() {
        assert_eq!(canonical_git_object_id(&"A".repeat(40)), Ok("a".repeat(40)));
        assert_eq!(canonical_git_object_id(&"F".repeat(64)), Ok("f".repeat(64)));
    }
}
