use serde::{Deserialize, Serialize};
use std::fmt;
use volicord_context::{
    CanonicalInvalidation, CanonicalRecordId, CheckpointId, ContextItemId, DecisionId, Principal,
    ProjectId, QuestionId, SourceId, TimestampMicros,
};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, CanonicalSourceRef, RepositorySnapshotId, SemanticAnnotation, Uncertainty,
};

macro_rules! local_identity {
    ($name:ident) => {
        #[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        pub struct $name([u8; 16]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
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

local_identity!(ProviderRequestId);
local_identity!(ManagedDerivedId);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityKind {
    LocalStructuralProcessing,
    InteractiveCurrentHostAccess,
    BackgroundSemanticProviderProcessing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityObservation {
    pub project_id: ProjectId,
    pub kind: AuthorityKind,
    pub host: Option<String>,
    pub session: Option<String>,
    pub request_or_operation: String,
    pub source_basis: Vec<SourceId>,
    pub purpose: String,
    pub observed_at: TimestampMicros,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOptInState {
    Enabled,
    Disabled,
    Revoked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderConfigurationState {
    NeverEnabled,
    Enabled,
    Disabled,
    Revoked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceExclusionPolicy {
    pub path_prefixes: Vec<String>,
    pub file_classes: Vec<SourceClass>,
    pub basis: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    Source,
    Generated,
    Vendor,
    Binary,
    Configuration,
    Document,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretFilteringPolicy {
    pub enabled: bool,
    pub line_markers: Vec<String>,
    pub replacement: String,
    pub known_limits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderRetentionPolicy {
    pub local_annotation_retained_until: Option<TimestampMicros>,
    pub local_basis: String,
    pub provider_expectation: String,
    pub provider_known_limits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderOptInPolicy {
    pub project_id: ProjectId,
    pub provider: String,
    pub model: String,
    pub purpose: String,
    pub requested_capability: String,
    pub allowed_source_scopes: Vec<String>,
    pub exclusions: SourceExclusionPolicy,
    pub filtering: SecretFilteringPolicy,
    pub retention: ProviderRetentionPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderIntentProvenance {
    pub actor: Principal,
    pub host: String,
    pub session: String,
    pub user_turn_source: SourceId,
    pub basis: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderOptInEvent {
    pub revision: u64,
    pub state: ProviderOptInState,
    pub policy: ProviderOptInPolicy,
    pub intent: ProviderIntentProvenance,
    pub recorded_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackgroundSource {
    pub source: CanonicalSourceRef,
    pub locator: String,
    pub class: SourceClass,
    /// Ephemeral request input. It is filtered in memory and is never written
    /// to the privacy store or portable Canonical Context.
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackgroundSemanticRequest {
    pub project_id: ProjectId,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub provider: String,
    pub model: String,
    pub purpose: String,
    pub requested_capability: String,
    pub requested_source_scopes: Vec<String>,
    pub sources: Vec<BackgroundSource>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeOutcome {
    Included,
    Excluded,
    OutsideRequestedScope,
    OutsideOptInScope,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOutcome {
    NotApplied,
    NoMatch,
    Filtered,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransmissionOutcome {
    NotTransmitted,
    Transmitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceManifestEntry {
    pub source: CanonicalSourceRef,
    pub locator: String,
    pub class: SourceClass,
    pub scope_outcome: ScopeOutcome,
    pub filter_outcome: FilterOutcome,
    pub transmission_outcome: TransmissionOutcome,
    pub original_bytes: u64,
    pub transmitted_bytes: u64,
    pub filtered_line_count: u64,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRequestOutcome {
    Prepared,
    NotAuthorized,
    NotTransmitted,
    ProviderUnavailable,
    ProviderFailed,
    ProviderTimedOut,
    ProviderCancelled,
    Completed,
    Partial,
    Stale,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderRequestRecord {
    pub id: ProviderRequestId,
    pub project_id: ProjectId,
    pub opt_in_revision: Option<u64>,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub provider: String,
    pub model: String,
    pub purpose: String,
    pub requested_capability: String,
    pub requested_source_scopes: Vec<String>,
    pub manifest: Vec<SourceManifestEntry>,
    pub outcome: ProviderRequestOutcome,
    pub diagnostic: Option<String>,
    pub requested_at: TimestampMicros,
    pub completed_at: Option<TimestampMicros>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderIdentity {
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderAvailability {
    Available,
    Unavailable { diagnostic: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationSource {
    pub source: CanonicalSourceRef,
    pub locator: String,
    pub filtered_body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocation {
    pub request_id: ProviderRequestId,
    pub project_id: ProjectId,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub provider: String,
    pub model: String,
    pub purpose: String,
    pub requested_capability: String,
    pub sources: Vec<ProviderInvocationSource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderGeneratedAnnotation {
    pub included_sources: Vec<SourceId>,
    pub text: String,
    pub uncertainty: Uncertainty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderExecution {
    Completed {
        annotations: Vec<ProviderGeneratedAnnotation>,
        diagnostic: Option<String>,
    },
    Partial {
        annotations: Vec<ProviderGeneratedAnnotation>,
        diagnostic: String,
    },
    Stale {
        annotations: Vec<ProviderGeneratedAnnotation>,
        diagnostic: String,
    },
    Unavailable {
        diagnostic: String,
    },
    TimedOut {
        diagnostic: String,
    },
    Cancelled {
        diagnostic: String,
    },
    Failed {
        diagnostic: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDerivedKind {
    SemanticAnnotation,
    Embedding,
    CachedSummary,
    GeneratedPreview,
    ProviderResultCopy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDerivedState {
    Current,
    Stale,
    Invalidated,
    Deleted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "identity")]
pub enum ManagedCanonicalLink {
    Project(ProjectId),
    Source(SourceId),
    Question(QuestionId),
    Decision(DecisionId),
    ContextItem(ContextItemId),
    Checkpoint(CheckpointId),
}

impl ManagedCanonicalLink {
    pub fn from_invalidation(invalidation: &CanonicalInvalidation) -> Self {
        match invalidation.record {
            CanonicalRecordId::Project(value) => Self::Project(value),
            CanonicalRecordId::Source(value) => Self::Source(value),
            CanonicalRecordId::Question(value) => Self::Question(value),
            CanonicalRecordId::Decision(value) => Self::Decision(value),
            CanonicalRecordId::ContextItem(value) => Self::ContextItem(value),
            CanonicalRecordId::Checkpoint(value) => Self::Checkpoint(value),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalDeletion {
    pub deleted_at: TimestampMicros,
    pub basis: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDeletionOutcome {
    NotRequested,
    Succeeded { diagnostic: Option<String> },
    Unsupported { diagnostic: String },
    Failed { diagnostic: String },
    Unknown { diagnostic: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManagedDerivedRecord {
    pub id: ManagedDerivedId,
    pub project_id: ProjectId,
    pub kind: ManagedDerivedKind,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub purpose: String,
    pub analysis_snapshot: Option<AnalysisSnapshotId>,
    pub included_sources: Vec<CanonicalSourceRef>,
    pub canonical_links: Vec<ManagedCanonicalLink>,
    pub content: Option<String>,
    pub uncertainty: Option<Uncertainty>,
    pub created_at: TimestampMicros,
    pub retained_until: Option<TimestampMicros>,
    pub retention_basis: String,
    pub state: ManagedDerivedState,
    pub local_deletion: Option<LocalDeletion>,
    pub provider_deletion: ProviderDeletionOutcome,
}

impl ManagedDerivedRecord {
    pub fn semantic_annotation(&self) -> Option<SemanticAnnotation> {
        if self.kind != ManagedDerivedKind::SemanticAnnotation
            || self.state == ManagedDerivedState::Deleted
        {
            return None;
        }
        Some(SemanticAnnotation {
            identity: self.id.to_string(),
            analysis_snapshot: self.analysis_snapshot?,
            provider: self.provider.clone()?,
            model: self.model.clone()?,
            purpose: self.purpose.clone(),
            included_sources: self.included_sources.clone(),
            text: self.content.clone()?,
            generated_at_unix_micros: self.created_at.as_unix_micros(),
            uncertainty: self.uncertainty.clone()?,
            retention_state: format!("{:?}", self.state).to_lowercase(),
            provenance_class: volicord_repository_intelligence::ProvenanceClass::SemanticAnnotation,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedDerivedDraft {
    pub project_id: ProjectId,
    pub kind: ManagedDerivedKind,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub purpose: String,
    pub analysis_snapshot: Option<AnalysisSnapshotId>,
    pub included_sources: Vec<CanonicalSourceRef>,
    pub canonical_links: Vec<ManagedCanonicalLink>,
    pub content: String,
    pub uncertainty: Option<Uncertainty>,
    pub retained_until: Option<TimestampMicros>,
    pub retention_basis: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedDeletionScope {
    pub project_id: ProjectId,
    pub kinds: Vec<ManagedDerivedKind>,
    pub provider: Option<String>,
    pub source_ids: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderDeletionRequest {
    pub project_id: ProjectId,
    pub managed_ids: Vec<ManagedDerivedId>,
    pub source_ids: Vec<SourceId>,
    pub provider: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedDeletionResult {
    pub locally_deleted: Vec<ManagedDerivedId>,
    pub provider_outcome: ProviderDeletionOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalForgettingCleanup {
    pub candidate_ids: Vec<volicord_inquiry::CandidateId>,
    pub derived_ids: Vec<ManagedDerivedId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPrivacyInspection {
    pub project_id: ProjectId,
    pub configuration_state: ProviderConfigurationState,
    pub current_opt_in: Option<ProviderOptInEvent>,
    pub authority_observations: Vec<AuthorityObservation>,
    pub requests: Vec<ProviderRequestRecord>,
    pub managed_derived: Vec<ManagedDerivedRecord>,
    /// Managed content withheld by the read barrier while a local canonical
    /// forgetting operation still requires cleanup.
    pub withheld_for_canonical_forgetting: Vec<ManagedDerivedId>,
}

pub(crate) struct PreparedSource {
    pub source: CanonicalSourceRef,
    pub locator: String,
    pub filtered_body: String,
}

pub struct AuthorizedProviderDispatch {
    pub(crate) request: ProviderRequestRecord,
    pub(crate) sources: Vec<PreparedSource>,
}

impl AuthorizedProviderDispatch {
    pub const fn request_id(&self) -> ProviderRequestId {
        self.request.id
    }

    pub fn record(&self) -> &ProviderRequestRecord {
        &self.request
    }
}

pub enum PreparationOutcome {
    Ready(AuthorizedProviderDispatch),
    Rejected(ProviderRequestRecord),
}
