//! Product-owned local process and filesystem mechanisms.

#![forbid(unsafe_code)]

mod filesystem;
mod process;
mod runtime;

pub use filesystem::{
    publish_file_no_replace, DirectoryEntryDurability, DirtyObservation, GitWorktreeLayout,
    LocalRepositoryCoordinate, NoReplacePublicationEffect, NoReplacePublicationError,
    NoReplacePublicationOutcome, NoReplacePublicationPhase, RepositoryPathError,
    RepositoryPathState, RepositoryRoot, ResolvedRepositoryPath, SourceFingerprint,
    SourceFingerprintError,
};
pub use process::{
    CancellationFlag, ProcessCompletion, ProcessObservation, ProcessRequest, ProcessStartError,
    ProcessStopTrigger, ProcessStreamArtifact, ProcessStreamCompleteness, ProcessTermination,
    ProcessTreeCleanup,
};
pub use runtime::{
    ensure_private_directory, ensure_private_file, MutationLockGuard, PrivateRuntimeError,
};
