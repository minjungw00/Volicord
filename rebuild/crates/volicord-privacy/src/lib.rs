//! Project-scoped privacy and optional background semantic-provider boundary.
//!
//! Local, current-host interactive, and background-provider authority remain
//! distinct. Background dispatch consumes a prepared authorization and checks
//! the current Project policy again, so interactive access and stale opt-in
//! state can never authorize transmission.

mod error;
mod model;
mod store;

pub use error::{Error, ErrorKind};
pub use model::{
    AuthorityKind, AuthorityObservation, AuthorizedProviderDispatch, BackgroundSemanticRequest,
    BackgroundSource, CanonicalForgettingCleanup, FilterOutcome, LocalDeletion,
    ManagedCanonicalLink, ManagedDeletionResult, ManagedDeletionScope, ManagedDerivedDraft,
    ManagedDerivedId, ManagedDerivedKind, ManagedDerivedRecord, ManagedDerivedState,
    PreparationOutcome, ProjectPrivacyInspection, ProviderAvailability, ProviderConfigurationState,
    ProviderDeletionOutcome, ProviderDeletionRequest, ProviderExecution,
    ProviderGeneratedAnnotation, ProviderIdentity, ProviderIntentProvenance, ProviderInvocation,
    ProviderInvocationSource, ProviderOptInEvent, ProviderOptInPolicy, ProviderOptInState,
    ProviderRequestId, ProviderRequestOutcome, ProviderRequestRecord, ProviderRetentionPolicy,
    ScopeOutcome, SecretFilteringPolicy, SourceClass, SourceExclusionPolicy, SourceManifestEntry,
    TransmissionOutcome,
};
pub use store::{
    BackgroundSemanticProvider, PrivacyStore, PRIVACY_SCHEMA_KIND, PRIVACY_SCHEMA_VERSION,
};
