use crate::{Error, ErrorKind};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Persisted UTC time at fixed microsecond precision.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TimestampMicros(i64);

impl TimestampMicros {
    pub const fn from_unix_micros(value: i64) -> Self {
        Self(value)
    }

    pub const fn as_unix_micros(self) -> i64 {
        self.0
    }
}

pub trait Clock: Send {
    fn now(&mut self) -> Result<TimestampMicros, Error>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&mut self) -> Result<TimestampMicros, Error> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                Error::with_source(
                    ErrorKind::StorageUnavailable,
                    "system UTC clock is before the Unix epoch",
                    error,
                )
            })?;
        let micros = i64::try_from(duration.as_micros()).map_err(|_| {
            Error::new(
                ErrorKind::StorageUnavailable,
                "system UTC clock is outside the persisted range",
            )
        })?;
        Ok(TimestampMicros(micros))
    }
}

#[derive(Debug)]
pub struct FixedClock {
    value: TimestampMicros,
}

impl FixedClock {
    pub const fn new(value: TimestampMicros) -> Self {
        Self { value }
    }
}

impl Clock for FixedClock {
    fn now(&mut self) -> Result<TimestampMicros, Error> {
        Ok(self.value)
    }
}
