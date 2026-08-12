use crate::{Error, ErrorKind};
use std::collections::VecDeque;
use std::fmt;

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }

            pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
                let value: [u8; 16] = bytes.try_into().map_err(|_| {
                    Error::new(ErrorKind::CorruptState, "stored identity is not 128 bits")
                })?;
                Ok(Self(value))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, formatter)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}

opaque_id!(ProjectId);
opaque_id!(SourceId);
opaque_id!(OperationId);
opaque_id!(LocalBindingId);
opaque_id!(QuestionId);
opaque_id!(DecisionId);
opaque_id!(ContextItemId);
opaque_id!(CheckpointId);

/// Generates opaque 128-bit identity material.
pub trait IdGenerator: Send {
    fn next_id(&mut self) -> Result<[u8; 16], Error>;
}

#[derive(Debug, Default)]
pub struct SystemIdGenerator;

impl IdGenerator for SystemIdGenerator {
    fn next_id(&mut self) -> Result<[u8; 16], Error> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                "operating-system randomness is unavailable",
                error,
            )
        })?;
        Ok(bytes)
    }
}

/// A finite deterministic sequence intended for tests and controlled fixtures.
#[derive(Debug)]
pub struct DeterministicIdGenerator {
    values: VecDeque<[u8; 16]>,
}

impl DeterministicIdGenerator {
    pub fn new(values: impl IntoIterator<Item = [u8; 16]>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }
}

impl IdGenerator for DeterministicIdGenerator {
    fn next_id(&mut self) -> Result<[u8; 16], Error> {
        self.values.pop_front().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "deterministic identity sequence is exhausted",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CheckpointId, ContextItemId, DecisionId, DeterministicIdGenerator, IdGenerator,
        OperationId, ProjectId, QuestionId, SourceId,
    };
    use std::any::TypeId;

    #[test]
    fn identifiers_are_distinct_types() {
        assert_ne!(TypeId::of::<ProjectId>(), TypeId::of::<SourceId>());
        assert_ne!(TypeId::of::<ProjectId>(), TypeId::of::<OperationId>());
        assert_ne!(TypeId::of::<SourceId>(), TypeId::of::<OperationId>());
        assert_ne!(TypeId::of::<QuestionId>(), TypeId::of::<DecisionId>());
        assert_ne!(TypeId::of::<ContextItemId>(), TypeId::of::<CheckpointId>());
    }

    #[test]
    fn deterministic_generator_returns_the_supplied_sequence() -> Result<(), crate::Error> {
        let mut generator = DeterministicIdGenerator::new([[7; 16], [9; 16]]);
        assert_eq!(generator.next_id()?, [7; 16]);
        assert_eq!(generator.next_id()?, [9; 16]);
        Ok(())
    }
}
