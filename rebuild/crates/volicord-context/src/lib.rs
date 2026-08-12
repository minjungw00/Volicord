//! Synchronous Canonical Context Kernel.
//!
//! The crate owns the durable Canonical Context responsibility boundary:
//! stable Projects, typed Sources and relations, Questions and explicit user
//! Decisions, Context Items, source-grounded Checkpoints, and replay-safe
//! SQLite operations at a caller-supplied path.

mod error;
mod identity;
mod merge;
mod model;
mod portable;
mod store;
mod time;

pub use error::{Error, ErrorKind};
pub use identity::{
    CheckpointId, ContextItemId, DecisionId, DeterministicIdGenerator, IdGenerator, LocalBindingId,
    OperationId, ProjectId, QuestionId, SourceId, SystemIdGenerator,
};
pub use merge::{
    BundleBasis, BundleComparison, BundleConflict, BundleConflictClass, BundleMerge,
    BundleMergeStatus, ConflictSourceBasis, MergeResolution, MergeResolutionMode,
    SourceBindingCandidate,
};
pub use model::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalInvalidation,
    CanonicalRecordId, CanonicalRecordKind, CanonicalRelation, CanonicalRelationKind, Checkpoint,
    CheckpointDraft, CheckpointKind, CommandOutcome, CommandTermination, ContextItem,
    ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole, CorrectionKind, Decision,
    DecisionChoice, DecisionCorrectionDraft, DecisionLifecycle, DecisionSupersessionDraft,
    ExplicitQuestionResponse, ForgetResult, LocalBinding, OperationResult, Principal,
    PrincipalKind, Project, Question, QuestionAlternative, QuestionDependency, QuestionDraft,
    QuestionReference, QuestionResponseDraft, QuestionResponseResult, QuestionState,
    QuestionTerminalOutcome, ReviewDue, ReviewDueDraft, ReviewDueKind, Source, SourceDraft,
    SourcePayload, SourceRelation, SourceRelationKind, StatementProvenanceRole, Tombstone,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource,
    VerificationFact, VerificationState, WorkState,
};
pub use portable::{
    BundleExport, BundleImport, BundleImportStatus, BUNDLE_FORMAT_VERSION, BUNDLE_KIND,
};
pub use store::{Store, SCHEMA_KIND, SCHEMA_VERSION};
pub use time::{Clock, FixedClock, SystemClock, TimestampMicros};
