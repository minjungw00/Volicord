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
    for (index, slot) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *slot = u8::from_str_radix(&value[offset..offset + 2], 16)
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
}
