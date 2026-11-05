use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;

macro_rules! digest_identity {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);

        impl $name {
            pub(crate) fn digest(parts: &[&[u8]]) -> Self {
                let mut hasher = Sha256::new();
                for part in parts {
                    hasher.update((part.len() as u64).to_be_bytes());
                    hasher.update(part);
                }
                Self(hasher.finalize().into())
            }

            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            pub fn from_hex(value: &str) -> Result<Self, String> {
                decode_hex::<32>(value).map(Self)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, formatter)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_hex(&self.0, formatter)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let encoded = encode_hex(&self.0);
                let value = std::str::from_utf8(&encoded).map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct IdentityVisitor;
                impl Visitor<'_> for IdentityVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("a 64-digit hexadecimal identity")
                    }

                    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<$name, E> {
                        $name::from_hex(value).map_err(E::custom)
                    }
                }
                deserializer.deserialize_str(IdentityVisitor)
            }
        }
    };
}

digest_identity!(RepositorySnapshotId);
digest_identity!(AnalysisSnapshotId);

/// Hash the same length-framed JSON bytes without retaining the serialized body.
/// The caller must supply an immutable, deterministic serialization basis.
pub(crate) fn analysis_identity_from_json(
    parts: &[&[u8]],
    basis: &impl Serialize,
) -> Result<AnalysisSnapshotId, serde_json::Error> {
    struct Counter(u64);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.checked_add(bytes.len() as u64).ok_or_else(|| {
                std::io::Error::other("Analysis identity serialization length overflow")
            })?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct HashWriter<'a> {
        hasher: &'a mut Sha256,
        counter: Counter,
    }
    impl std::io::Write for HashWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let count = std::io::Write::write(&mut self.counter, bytes)?;
            self.hasher.update(bytes);
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut length = Counter(0);
    serde_json::to_writer(&mut length, basis)?;
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    hasher.update(length.0.to_be_bytes());
    let mut output = std::io::BufWriter::with_capacity(
        64 * 1024,
        HashWriter {
            hasher: &mut hasher,
            counter: Counter(0),
        },
    );
    serde_json::to_writer(&mut output, basis)?;
    std::io::Write::flush(&mut output).map_err(serde_json::Error::io)?;
    let written = output.get_ref().counter.0;
    drop(output);
    if written != length.0 {
        return Err(serde_json::Error::io(std::io::Error::other(
            "Analysis identity serialization changed length between passes",
        )));
    }
    Ok(AnalysisSnapshotId(hasher.finalize().into()))
}

pub(crate) fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 {
        return Err(format!(
            "identity must contain {} hexadecimal digits",
            N * 2
        ));
    }
    let mut bytes = [0_u8; N];
    for (slot, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        *slot = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_digit(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("identity contains a non-hexadecimal digit".into()),
    }
}

fn encode_hex(bytes: &[u8; 32]) -> [u8; 64] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = [0; 64];
    for (value, pair) in bytes.iter().zip(encoded.chunks_exact_mut(2)) {
        pair[0] = HEX[usize::from(value >> 4)];
        pair[1] = HEX[usize::from(value & 15)];
    }
    encoded
}

pub(crate) fn write_hex(bytes: &[u8; 32], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let encoded = encode_hex(bytes);
    formatter.write_str(std::str::from_utf8(&encoded).map_err(|_| fmt::Error)?)
}

#[cfg(test)]
mod tests {
    use super::{analysis_identity_from_json, AnalysisSnapshotId, RepositorySnapshotId};

    #[test]
    fn streamed_json_identity_matches_the_complete_framed_byte_oracle(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repeated = "한글 \\\"\n🦀".repeat(8192);
        for basis in [
            serde_json::json!(null),
            serde_json::json!([]),
            serde_json::json!({"relations": [], "coverage": {"unknown": null}}),
            serde_json::json!({"negative": -23, "bytes": [0, 255], "text": repeated}),
        ] {
            let bytes = serde_json::to_vec(&basis)?;
            for parts in [vec![b"ab".as_slice(), b"c"], vec![b"a".as_slice(), b"bc"]] {
                let mut framed = parts.clone();
                framed.push(&bytes);
                assert_eq!(
                    analysis_identity_from_json(&parts, &basis)?,
                    AnalysisSnapshotId::digest(&framed),
                );
            }
        }
        Ok(())
    }

    #[test]
    fn streamed_json_identity_rejects_serializer_failure_and_length_change() {
        struct Reject;
        impl serde::Serialize for Reject {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom(
                    "intentional identity serializer failure",
                ))
            }
        }
        struct Changing(std::cell::Cell<bool>);
        impl serde::Serialize for Changing {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let changed = self.0.replace(true);
                serializer.serialize_str(if changed { "longer" } else { "x" })
            }
        }
        assert!(analysis_identity_from_json(&[b"domain"], &Reject).is_err());
        assert!(analysis_identity_from_json(&[b"domain"], &Changing(false.into())).is_err());
    }

    #[test]
    fn digest_identity_is_stable_and_domain_separated_by_parts() {
        let first = RepositorySnapshotId::digest(&[b"ab", b"c"]);
        let repeated = RepositorySnapshotId::digest(&[b"ab", b"c"]);
        let differently_partitioned = RepositorySnapshotId::digest(&[b"a", b"bc"]);
        assert_eq!(first, repeated);
        assert_ne!(first, differently_partitioned);

        let encoded = first.to_string();
        assert_eq!(RepositorySnapshotId::from_hex(&encoded), Ok(first));
        assert!(AnalysisSnapshotId::from_hex("not-an-identity").is_err());
    }

    #[test]
    fn invalid_unicode_identities_are_errors_at_direct_and_json_boundaries() {
        for character in ['é', '가', '🦀', 'g'] {
            for prefix in 0..=(64 - character.len_utf8()) {
                let value = format!(
                    "{}{}{}",
                    "0".repeat(prefix),
                    character,
                    "0".repeat(64 - prefix - character.len_utf8())
                );
                assert_eq!(value.len(), 64);
                assert!(AnalysisSnapshotId::from_hex(&value).is_err());
                assert!(RepositorySnapshotId::from_hex(&value).is_err());
                let encoded = serde_json::to_string(&value).expect("JSON string");
                assert!(serde_json::from_str::<AnalysisSnapshotId>(&encoded).is_err());
                assert!(serde_json::from_str::<RepositorySnapshotId>(&encoded).is_err());
            }
        }
        let uppercase = "AB".repeat(32);
        let identity = AnalysisSnapshotId::from_hex(&uppercase).expect("uppercase hexadecimal");
        assert_eq!(identity.to_string(), uppercase.to_lowercase());
    }

    #[test]
    fn identity_codecs_preserve_every_byte_and_exact_json_strings() {
        for seed in 0..=u8::MAX {
            let bytes: [u8; 32] = std::array::from_fn(|index| seed.wrapping_add(index as u8));
            let text = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let identity = AnalysisSnapshotId::from_hex(&text).expect("hexadecimal bytes");
            assert_eq!(identity.as_bytes(), &bytes);
            assert_eq!(identity.to_string(), text);
            assert_eq!(format!("{identity:?}"), text);
            let expected_json = format!("\"{text}\"");
            assert_eq!(
                serde_json::to_string(&identity).expect("JSON"),
                expected_json
            );
            assert_eq!(
                serde_json::from_str::<AnalysisSnapshotId>(&expected_json).expect("JSON"),
                identity
            );
            assert_eq!(
                AnalysisSnapshotId::from_hex(&text.to_uppercase()).expect("uppercase"),
                identity
            );
        }
    }

    #[test]
    fn signed_pairs_cannot_alias_hexadecimal_identities() {
        for byte in [b'+', b'-', b' ', b'_', 0] {
            for index in 0..64 {
                let mut text = vec![b'1'; 64];
                text[index] = byte;
                let text = String::from_utf8(text).expect("ASCII");
                assert!(AnalysisSnapshotId::from_hex(&text).is_err());
                assert!(RepositorySnapshotId::from_hex(&text).is_err());
                let json = serde_json::to_string(&text).expect("JSON string");
                assert!(serde_json::from_str::<AnalysisSnapshotId>(&json).is_err());
            }
        }
    }
}
