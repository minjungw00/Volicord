use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::from_hex(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

digest_identity!(RepositorySnapshotId);
digest_identity!(AnalysisSnapshotId);

pub(crate) fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 {
        return Err(format!(
            "identity must contain {} hexadecimal digits",
            N * 2
        ));
    }
    let mut bytes = [0_u8; N];
    for (slot, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| "identity contains a non-hexadecimal digit".to_owned())?;
        *slot = u8::from_str_radix(pair, 16)
            .map_err(|_| "identity contains a non-hexadecimal digit".to_owned())?;
    }
    Ok(bytes)
}

pub(crate) fn write_hex(bytes: &[u8], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AnalysisSnapshotId, RepositorySnapshotId};

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
}
