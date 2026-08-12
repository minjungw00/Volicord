//! Synchronous Canonical Context Kernel.
//!
//! The crate currently owns only the first durable responsibility boundary:
//! stable Projects, local clone bindings, typed Sources, explicit relations,
//! and replay-safe SQLite operations at a caller-supplied path.

mod error;
mod identity;
mod model;
mod store;
mod time;

pub use error::{Error, ErrorKind};
pub use identity::{
    DeterministicIdGenerator, IdGenerator, LocalBindingId, OperationId, ProjectId, SourceId,
    SystemIdGenerator,
};
pub use model::{
    Availability, CommandOutcome, CommandTermination, LocalBinding, OperationResult, Principal,
    PrincipalKind, Project, Source, SourceDraft, SourcePayload, SourceRelation, SourceRelationKind,
};
pub use store::{Store, SCHEMA_KIND, SCHEMA_VERSION};
pub use time::{Clock, FixedClock, SystemClock, TimestampMicros};
