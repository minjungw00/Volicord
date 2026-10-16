//! Bounded browser presentation. The transport permits only this exact script.
use sha2::{Digest, Sha256};
pub(crate) const SCRIPT: &str = include_str!("reading.js");
pub(crate) fn csp_source() -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let digest = Sha256::digest(SCRIPT.as_bytes());
    let mut encoded = String::new();
    for chunk in digest.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        encoded.push(char::from(ALPHABET[usize::from(a >> 2)]));
        encoded.push(char::from(ALPHABET[usize::from((a & 3) << 4 | b >> 4)]));
        encoded.push(if chunk.len() > 1 {
            char::from(ALPHABET[usize::from((b & 15) << 2 | c >> 6)])
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            char::from(ALPHABET[usize::from(c & 63)])
        } else {
            '='
        });
    }
    format!("'sha256-{encoded}'")
}
