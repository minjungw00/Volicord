use std::{error::Error, fmt, sync::Mutex};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! opaque_string_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates an opaque string identifier wrapper.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the wrapper and returns the underlying string.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

opaque_string_type!(ProjectId, "Opaque project identifier.");
opaque_string_type!(TaskId, "Opaque Task identifier.");
opaque_string_type!(SurfaceId, "Opaque local surface identifier.");
opaque_string_type!(
    SurfaceInstanceId,
    "Opaque local surface-instance identifier."
);
opaque_string_type!(RequestId, "Opaque request identifier.");
opaque_string_type!(IdempotencyKey, "Opaque idempotency-key identifier.");
opaque_string_type!(EventId, "Opaque event identifier.");
opaque_string_type!(RecordId, "Opaque state-record identifier.");
opaque_string_type!(BaselineRef, "Opaque baseline identifier.");
opaque_string_type!(ChangeUnitId, "Opaque Change Unit identifier.");
opaque_string_type!(
    WriteAuthorizationId,
    "Opaque Write Authorization identifier."
);
opaque_string_type!(RunId, "Opaque Run identifier.");
opaque_string_type!(ArtifactId, "Opaque artifact identifier.");
opaque_string_type!(
    ArtifactInputId,
    "Opaque request-local artifact input identifier."
);
opaque_string_type!(
    StagedArtifactHandleId,
    "Opaque staged-artifact handle identifier."
);
opaque_string_type!(UserJudgmentId, "Opaque user-judgment identifier.");
opaque_string_type!(
    UserJudgmentOptionId,
    "Opaque judgment-local option identifier."
);
opaque_string_type!(RiskId, "Opaque residual-risk identifier.");
opaque_string_type!(StorageRef, "Opaque artifact storage reference.");
opaque_string_type!(RequestHash, "Deterministic canonical request hash string.");

/// Number of generated durable IDs to try before reporting an internal collision failure.
pub const DURABLE_ID_RETRY_LIMIT: usize = 8;

/// Core-owned durable record families that use generated opaque identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DurableIdKind {
    /// Core-generated Task ids.
    Task,
    /// Core-generated Change Unit ids.
    ChangeUnit,
    /// Core-generated user-owned judgment ids.
    UserJudgment,
    /// Core-generated Write Authorization ids.
    WriteAuthorization,
    /// Core-generated Run ids when the request does not supply one.
    Run,
    /// Core-generated committed event ids.
    Event,
    /// Core-generated transient staged artifact handles.
    StagedArtifact,
    /// Core-generated persistent artifact ids.
    Artifact,
    /// Core-generated evidence summary ids.
    Evidence,
}

impl DurableIdKind {
    /// Returns the non-authoritative readable prefix for this generated id kind.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Task => "task_",
            Self::ChangeUnit => "cu_",
            Self::UserJudgment => "uj_",
            Self::WriteAuthorization => "wa_",
            Self::Run => "run_",
            Self::Event => "evt_",
            Self::StagedArtifact => "staged_",
            Self::Artifact => "artifact_",
            Self::Evidence => "evidence_",
        }
    }
}

impl fmt::Display for DurableIdKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Task => "task",
            Self::ChangeUnit => "change_unit",
            Self::UserJudgment => "user_judgment",
            Self::WriteAuthorization => "write_authorization",
            Self::Run => "run",
            Self::Event => "event",
            Self::StagedArtifact => "staged_artifact",
            Self::Artifact => "artifact",
            Self::Evidence => "evidence",
        })
    }
}

/// Error returned when Core cannot mint an opaque durable identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableIdError {
    /// The operating system random source could not produce bytes.
    RandomUnavailable { detail: String },
    /// A deterministic generator used for tests has no remaining suffixes.
    DeterministicSequenceExhausted,
}

impl fmt::Display for DurableIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RandomUnavailable { detail } => {
                write!(formatter, "durable id random source unavailable: {detail}")
            }
            Self::DeterministicSequenceExhausted => {
                formatter.write_str("deterministic durable id sequence exhausted")
            }
        }
    }
}

impl Error for DurableIdError {}

/// Generator for Core-owned opaque durable identifiers.
pub trait DurableIdGenerator: fmt::Debug + Send + Sync {
    /// Generates one full identifier for the requested durable record family.
    fn generate(&self, kind: DurableIdKind) -> Result<String, DurableIdError>;
}

/// Production generator backed by the operating system random source.
#[derive(Debug, Default)]
pub struct RandomDurableIdGenerator;

impl DurableIdGenerator for RandomDurableIdGenerator {
    fn generate(&self, kind: DurableIdKind) -> Result<String, DurableIdError> {
        random_durable_id(kind)
    }
}

/// Deterministic generator for focused tests.
#[derive(Debug)]
pub struct SequenceDurableIdGenerator {
    suffixes: Mutex<Vec<String>>,
}

impl SequenceDurableIdGenerator {
    /// Creates a deterministic generator that consumes the supplied suffixes in order.
    pub fn new(suffixes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut suffixes = suffixes
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        suffixes.reverse();
        Self {
            suffixes: Mutex::new(suffixes),
        }
    }
}

impl DurableIdGenerator for SequenceDurableIdGenerator {
    fn generate(&self, kind: DurableIdKind) -> Result<String, DurableIdError> {
        let suffix = self
            .suffixes
            .lock()
            .expect("deterministic durable id generator mutex should not be poisoned")
            .pop()
            .ok_or(DurableIdError::DeterministicSequenceExhausted)?;
        Ok(prefixed_durable_id(kind, &suffix))
    }
}

/// Builds a full durable id from a kind prefix and opaque suffix.
pub fn prefixed_durable_id(kind: DurableIdKind, suffix: &str) -> String {
    format!("{}{}", kind.prefix(), suffix)
}

fn random_durable_id(kind: DurableIdKind) -> Result<String, DurableIdError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| DurableIdError::RandomUnavailable {
        detail: error.to_string(),
    })?;

    // UUIDv4 layout is useful for collision resistance diagnostics only; it is
    // not public ordering, timing, or authority semantics.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(prefixed_durable_id(kind, &uuid_v4_suffix(bytes)))
}

fn uuid_v4_suffix(bytes: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_suffix_uses_uuid_v4_bits() {
        let suffix = uuid_v4_suffix([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x4a, 0xbb, 0x8c, 0xdd, 0xee, 0xff, 0x10, 0x20,
            0x30, 0x40,
        ]);
        assert_eq!(suffix, "00112233-4455-4abb-8cdd-eeff10203040");
    }

    #[test]
    fn sequence_generator_preserves_kind_prefixes() {
        let generator = SequenceDurableIdGenerator::new(["one", "two"]);
        assert_eq!(generator.generate(DurableIdKind::Task).unwrap(), "task_one");
        assert_eq!(generator.generate(DurableIdKind::Event).unwrap(), "evt_two");
        assert_eq!(
            generator.generate(DurableIdKind::Run),
            Err(DurableIdError::DeterministicSequenceExhausted)
        );
    }
}
