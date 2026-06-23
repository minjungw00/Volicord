use std::collections::BTreeMap;

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

/// Computes the deterministic request hash used by later idempotency checks.
pub fn canonical_request_hash<T: Serialize>(request: &T) -> Result<RequestHash, serde_json::Error> {
    canonical_json_sha256(request)
}

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
