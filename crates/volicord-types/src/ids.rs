use std::{error::Error, fmt, sync::Mutex};

use schemars::JsonSchema;
use serde::{de, Deserialize, Deserializer, Serialize};

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
opaque_string_type!(AgentConnectionId, "Opaque Agent Connection identifier.");
opaque_string_type!(
    AgentRuntimeSessionId,
    "Opaque Agent Runtime Session identifier."
);
opaque_string_type!(AgentSessionId, "Opaque Agent Session identifier.");
opaque_string_type!(
    GuardInstallationId,
    "Opaque host-hook installation identifier."
);
opaque_string_type!(GuardEventId, "Opaque host-hook event identifier.");
opaque_string_type!(
    GuardIntegrationVerificationId,
    "Opaque Guard integration-verification run identifier."
);
opaque_string_type!(PromptCaptureId, "Opaque prompt-capture identifier.");
opaque_string_type!(
    RepositoryObservationId,
    "Opaque invocation-scoped repository-observation identifier."
);
opaque_string_type!(UnrecordedChangeId, "Opaque unrecorded-change identifier.");
opaque_string_type!(RequestId, "Opaque request identifier.");
opaque_string_type!(IdempotencyKey, "Opaque idempotency-key identifier.");
opaque_string_type!(EventId, "Opaque event identifier.");
opaque_string_type!(RecordId, "Opaque state-record identifier.");
opaque_string_type!(BaselineRef, "Opaque baseline identifier.");
opaque_string_type!(ChangeUnitId, "Opaque Change Unit identifier.");
opaque_string_type!(ShapingCheckpointId, "Opaque ShapingCheckpoint identifier.");
opaque_string_type!(ShapingGapId, "Opaque shaping-gap identifier.");
opaque_string_type!(
    ShapingDecisionApplicationId,
    "Deterministic shaping-decision-application identifier."
);
opaque_string_type!(
    ShapingAuthorityReauthorizationId,
    "Deterministic stale shaping-authority reauthorization identifier."
);

/// Derives the stable identity for one semantic-owner application of a resolution.
pub fn shaping_decision_application_id(
    resolution_id: &UserActionResolutionId,
    owner: crate::values::ShapingDecisionApplicationOwner,
) -> Result<ShapingDecisionApplicationId, serde_json::Error> {
    #[derive(Serialize)]
    struct IdentityBasis<'a> {
        user_action_resolution_id: &'a UserActionResolutionId,
        application_owner: crate::values::ShapingDecisionApplicationOwner,
    }

    let digest = crate::canonical::canonical_json_bare_sha256(&IdentityBasis {
        user_action_resolution_id: resolution_id,
        application_owner: owner,
    })?;
    Ok(ShapingDecisionApplicationId::new(format!(
        "shaping_application_{digest}"
    )))
}

/// Derives the stable identity for one terminal disposition of a stale application.
pub fn shaping_authority_reauthorization_id(
    stale_application_id: &ShapingDecisionApplicationId,
) -> Result<ShapingAuthorityReauthorizationId, serde_json::Error> {
    #[derive(Serialize)]
    struct IdentityBasis<'a> {
        stale_application_id: &'a ShapingDecisionApplicationId,
    }

    let digest = crate::canonical::canonical_json_bare_sha256(&IdentityBasis {
        stale_application_id,
    })?;
    Ok(ShapingAuthorityReauthorizationId::new(format!(
        "shaping_reauthorization_{digest}"
    )))
}
opaque_string_type!(WriteTicketId, "Opaque write ticket identifier.");
opaque_string_type!(RunId, "Opaque Run identifier.");
opaque_string_type!(
    AcceptanceCriterionId,
    "Opaque Core-generated acceptance-criterion identifier."
);
opaque_string_type!(
    EvidenceClaimId,
    "Opaque caller-assigned Task-scoped supplemental evidence-claim identifier."
);
opaque_string_type!(
    EvidenceObservationId,
    "Opaque evidence-observation identifier."
);
opaque_string_type!(
    EvidenceCaptureIntentId,
    "Opaque evidence-capture intent identifier."
);
opaque_string_type!(
    EvidenceCaptureReceiptId,
    "Opaque evidence-capture receipt identifier."
);
opaque_string_type!(EvidenceProducerId, "Opaque evidence-producer identifier.");
opaque_string_type!(ArtifactId, "Opaque artifact identifier.");
opaque_string_type!(
    ArtifactInputId,
    "Opaque request-local artifact input identifier."
);
opaque_string_type!(
    StagedArtifactHandleId,
    "Opaque staged-artifact handle identifier."
);
opaque_string_type!(
    UserActionRequestId,
    "Opaque user-action-request identifier."
);
opaque_string_type!(
    UserActionResolutionId,
    "Opaque user-action-resolution identifier."
);
opaque_string_type!(
    UserActionOptionId,
    "Opaque choice-action-local option identifier."
);
opaque_string_type!(RiskId, "Opaque residual-risk identifier.");
opaque_string_type!(
    ProjectContinuityRecordId,
    "Opaque project-continuity record identifier."
);
opaque_string_type!(StorageRef, "Opaque artifact storage reference.");
opaque_string_type!(RequestHash, "Deterministic canonical request hash string.");

/// Strict Store-generated lifecycle coordinate for one physical Agent Connection row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct ConnectionIntegrationInstanceId(String);

impl ConnectionIntegrationInstanceId {
    /// Validates and retains a canonical Store-generated integration-instance ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, ConnectionIntegrationInstanceIdError> {
        let value = value.into();
        let prefix = DurableIdKind::ConnectionIntegrationInstance.prefix();
        let Some(suffix) = value.strip_prefix(prefix) else {
            return Err(ConnectionIntegrationInstanceIdError);
        };
        let bytes = suffix.as_bytes();
        if bytes.len() != 36
            || bytes[8] != b'-'
            || bytes[13] != b'-'
            || bytes[18] != b'-'
            || bytes[23] != b'-'
            || bytes[14] != b'4'
            || !matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
            || bytes.iter().enumerate().any(|(index, byte)| {
                !matches!(index, 8 | 13 | 18 | 23) && !matches!(byte, b'0'..=b'9' | b'a'..=b'f')
            })
        {
            return Err(ConnectionIntegrationInstanceIdError);
        }
        Ok(Self(value))
    }

    /// Returns the canonical ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the canonical ID string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for ConnectionIntegrationInstanceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

impl AsRef<str> for ConnectionIntegrationInstanceId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ConnectionIntegrationInstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Failure to decode a noncanonical Connection integration-instance ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionIntegrationInstanceIdError;

impl fmt::Display for ConnectionIntegrationInstanceIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("Connection integration-instance ID must be a canonical prefixed UUIDv4")
    }
}

impl Error for ConnectionIntegrationInstanceIdError {}

/// Opaque provenance identifier for one prepared Runtime Home publication.
///
/// This value distinguishes publication invocations. It is not a credential,
/// an authorization secret, or an operating-system actor identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct RuntimeHomePublicationId(String);

const RUNTIME_HOME_PUBLICATION_ID_PREFIX: &str = "runtime_home_publication_";

impl RuntimeHomePublicationId {
    /// Generates one fresh publication identifier from the operating-system
    /// random source.
    pub fn generate() -> Result<Self, DurableIdError> {
        Ok(Self(format!(
            "{RUNTIME_HOME_PUBLICATION_ID_PREFIX}{}",
            random_uuid_v4_suffix()?
        )))
    }

    /// Validates and retains a canonical persisted publication identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, RuntimeHomePublicationIdError> {
        let value = value.into();
        let Some(suffix) = value.strip_prefix(RUNTIME_HOME_PUBLICATION_ID_PREFIX) else {
            return Err(RuntimeHomePublicationIdError);
        };
        if !is_uuid_v4_suffix(suffix) {
            return Err(RuntimeHomePublicationIdError);
        }
        Ok(Self(value))
    }

    /// Returns the canonical persisted spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the canonical persisted spelling.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for RuntimeHomePublicationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

impl AsRef<str> for RuntimeHomePublicationId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RuntimeHomePublicationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Failure to decode a noncanonical Runtime Home publication identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeHomePublicationIdError;

impl fmt::Display for RuntimeHomePublicationIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Runtime Home publication ID must be a canonical prefixed UUIDv4")
    }
}

impl Error for RuntimeHomePublicationIdError {}

/// Number of generated durable IDs to try before reporting an internal collision failure.
pub const DURABLE_ID_RETRY_LIMIT: usize = 8;

/// Core-owned durable record families that use generated opaque identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DurableIdKind {
    /// Core-generated Task ids.
    Task,
    /// Core-generated Change Unit ids.
    ChangeUnit,
    /// Core-generated ShapingCheckpoint ids.
    ShapingCheckpoint,
    /// Core-generated shaping-gap ids.
    ShapingGap,
    /// Core-generated user-action-request ids.
    UserActionRequest,
    /// Core-generated user-action-resolution ids.
    UserActionResolution,
    /// Core-generated write ticket ids.
    WriteTicket,
    /// Core-generated Run ids when the request does not supply one.
    Run,
    /// Core-generated committed event ids.
    Event,
    /// Core-generated Agent Session ids.
    AgentSession,
    /// Store-generated MCP Runtime Session ids.
    McpRuntimeSession,
    /// Store-generated managed MCP launch-lease ids.
    ManagedMcpLaunchLease,
    /// Producer-generated immutable diagnostic occurrence ids.
    DiagnosticOccurrence,
    /// Store-generated physical Agent Connection integration-instance ids.
    ConnectionIntegrationInstance,
    /// Core-generated host-hook installation ids.
    GuardInstallation,
    /// Core-generated host-hook event ids.
    GuardEvent,
    /// Store-generated Guard integration-verification run ids.
    GuardIntegrationVerification,
    /// Core-generated prompt-capture ids.
    PromptCapture,
    /// Core-generated unrecorded-change ids.
    UnrecordedChange,
    /// Core-generated transient staged artifact handles.
    StagedArtifact,
    /// Core-generated persistent artifact ids.
    Artifact,
    /// Core-generated evidence summary ids.
    Evidence,
    /// Core-generated acceptance-criterion ids.
    AcceptanceCriterion,
    /// Core-generated evidence observation ids.
    EvidenceObservation,
    /// Core-generated evidence-capture intent ids.
    EvidenceCaptureIntent,
    /// Authority-source-generated evidence-capture receipt ids.
    EvidenceCaptureReceipt,
    /// Core-generated evidence-producer ids.
    EvidenceProducer,
    /// Core-generated residual-risk ids for current close bases.
    Risk,
    /// Core-generated project-continuity record ids.
    ProjectContinuityRecord,
}

impl DurableIdKind {
    /// Returns the non-authoritative readable prefix for this generated id kind.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Task => "task_",
            Self::ChangeUnit => "cu_",
            Self::ShapingCheckpoint => "shaping_",
            Self::ShapingGap => "shaping_gap_",
            Self::UserActionRequest => "uar_",
            Self::UserActionResolution => "ures_",
            Self::WriteTicket => "wt_",
            Self::Run => "run_",
            Self::Event => "evt_",
            Self::AgentSession => "session_",
            Self::McpRuntimeSession => "mcp_runtime_",
            Self::ManagedMcpLaunchLease => "mcp_launch_lease_",
            Self::DiagnosticOccurrence => "finding.occurrence_",
            Self::ConnectionIntegrationInstance => "connection_instance_",
            Self::GuardInstallation => "guard_installation_",
            Self::GuardEvent => "guard_event_",
            Self::GuardIntegrationVerification => "guard_verification_",
            Self::PromptCapture => "prompt_capture_",
            Self::UnrecordedChange => "unrecorded_change_",
            Self::StagedArtifact => "staged_",
            Self::Artifact => "artifact_",
            Self::Evidence => "evidence_",
            Self::AcceptanceCriterion => "criterion_",
            Self::EvidenceObservation => "evidence_observation_",
            Self::EvidenceCaptureIntent => "evidence_capture_intent_",
            Self::EvidenceCaptureReceipt => "evidence_capture_receipt_",
            Self::EvidenceProducer => "evidence_producer_",
            Self::Risk => "risk_",
            Self::ProjectContinuityRecord => "continuity_",
        }
    }
}

impl fmt::Display for DurableIdKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Task => "task",
            Self::ChangeUnit => "change_unit",
            Self::ShapingCheckpoint => "shaping_checkpoint",
            Self::ShapingGap => "shaping_gap",
            Self::UserActionRequest => "user_action_request",
            Self::UserActionResolution => "user_action_resolution",
            Self::WriteTicket => "write_ticket",
            Self::Run => "run",
            Self::Event => "event",
            Self::AgentSession => "agent_session",
            Self::McpRuntimeSession => "mcp_runtime_session",
            Self::ManagedMcpLaunchLease => "managed_mcp_launch_lease",
            Self::DiagnosticOccurrence => "diagnostic_occurrence",
            Self::ConnectionIntegrationInstance => "connection_integration_instance",
            Self::GuardInstallation => "guard_installation",
            Self::GuardEvent => "guard_event",
            Self::GuardIntegrationVerification => "guard_integration_verification",
            Self::PromptCapture => "prompt_capture",
            Self::UnrecordedChange => "unrecorded_change",
            Self::StagedArtifact => "staged_artifact",
            Self::Artifact => "artifact",
            Self::Evidence => "evidence",
            Self::AcceptanceCriterion => "acceptance_criterion",
            Self::EvidenceObservation => "evidence_observation",
            Self::EvidenceCaptureIntent => "evidence_capture_intent",
            Self::EvidenceCaptureReceipt => "evidence_capture_receipt",
            Self::EvidenceProducer => "evidence_producer",
            Self::Risk => "risk",
            Self::ProjectContinuityRecord => "project_continuity_record",
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
    Ok(prefixed_durable_id(kind, &random_uuid_v4_suffix()?))
}

fn random_uuid_v4_suffix() -> Result<String, DurableIdError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| DurableIdError::RandomUnavailable {
        detail: error.to_string(),
    })?;

    // UUIDv4 layout is useful for collision resistance diagnostics only; it is
    // not public ordering, timing, or authority semantics.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(uuid_v4_suffix(bytes))
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

fn is_uuid_v4_suffix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && bytes[14] == b'4'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
        && !bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 8 | 13 | 18 | 23) && !matches!(byte, b'0'..=b'9' | b'a'..=b'f')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shaping_application_identity_is_deterministic_and_owner_bound() {
        let resolution_id = UserActionResolutionId::new("resolution_exact_application");
        let first = shaping_decision_application_id(
            &resolution_id,
            crate::values::ShapingDecisionApplicationOwner::AdvanceTask,
        )
        .expect("application ID");
        let replay = shaping_decision_application_id(
            &resolution_id,
            crate::values::ShapingDecisionApplicationOwner::AdvanceTask,
        )
        .expect("replayed application ID");
        let other_owner = shaping_decision_application_id(
            &resolution_id,
            crate::values::ShapingDecisionApplicationOwner::FinalizeAdvice,
        )
        .expect("other owner application ID");
        assert_eq!(first, replay);
        assert_ne!(first, other_owner);
        assert!(first.as_str().starts_with("shaping_application_"));
    }

    #[test]
    fn random_suffix_uses_uuid_v4_bits() {
        let suffix = uuid_v4_suffix([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x4a, 0xbb, 0x8c, 0xdd, 0xee, 0xff, 0x10, 0x20,
            0x30, 0x40,
        ]);
        assert_eq!(suffix, "00112233-4455-4abb-8cdd-eeff10203040");
    }

    #[test]
    fn runtime_home_publication_ids_are_bounded_prefixed_uuid_v4_values() {
        let publication_id =
            RuntimeHomePublicationId::generate().expect("publication ID should be generated");
        assert_eq!(publication_id.as_str().len(), 61);
        assert!(publication_id
            .as_str()
            .starts_with(RUNTIME_HOME_PUBLICATION_ID_PREFIX));
        assert_eq!(
            RuntimeHomePublicationId::parse(publication_id.as_str()).unwrap(),
            publication_id
        );
        for invalid in [
            "",
            "runtime_home_publication_not-a-uuid",
            "runtime_home_publication_00112233-4455-3abb-8cdd-eeff10203040",
            "runtime_home_publication_00112233-4455-4abb-7cdd-eeff10203040",
            "runtime_home_publication_00112233-4455-4ABB-8cdd-eeff10203040",
        ] {
            assert!(
                RuntimeHomePublicationId::parse(invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn sequence_generator_preserves_kind_prefixes() {
        let generator =
            SequenceDurableIdGenerator::new(["one", "two", "three", "four", "five", "six"]);
        assert_eq!(generator.generate(DurableIdKind::Task).unwrap(), "task_one");
        assert_eq!(generator.generate(DurableIdKind::Event).unwrap(), "evt_two");
        assert_eq!(
            generator
                .generate(DurableIdKind::EvidenceCaptureIntent)
                .unwrap(),
            "evidence_capture_intent_three"
        );
        assert_eq!(
            generator
                .generate(DurableIdKind::EvidenceCaptureReceipt)
                .unwrap(),
            "evidence_capture_receipt_four"
        );
        assert_eq!(
            generator.generate(DurableIdKind::EvidenceProducer).unwrap(),
            "evidence_producer_five"
        );
        assert_eq!(
            generator
                .generate(DurableIdKind::ConnectionIntegrationInstance)
                .unwrap(),
            "connection_instance_six"
        );
        assert_eq!(
            generator.generate(DurableIdKind::Run),
            Err(DurableIdError::DeterministicSequenceExhausted)
        );
    }

    #[test]
    fn connection_integration_instance_id_is_strict_and_canonical() {
        let canonical = "connection_instance_00112233-4455-4abb-8cdd-eeff10203040";
        let parsed = ConnectionIntegrationInstanceId::parse(canonical).unwrap();
        assert_eq!(parsed.as_str(), canonical);
        assert_eq!(parsed.to_string(), canonical);
        assert_eq!(
            serde_json::from_str::<ConnectionIntegrationInstanceId>(
                &serde_json::to_string(canonical).unwrap()
            )
            .unwrap(),
            parsed
        );

        for invalid in [
            "00112233-4455-4abb-8cdd-eeff10203040",
            "connection_instance_00112233-4455-3abb-8cdd-eeff10203040",
            "connection_instance_00112233-4455-4abb-7cdd-eeff10203040",
            "connection_instance_00112233-4455-4ABB-8cdd-eeff10203040",
            "connection_instance_001122334455-4abb-8cdd-eeff10203040",
            "connection_instance_00112233-4455-4abb-8cdd-eeff1020304g",
        ] {
            assert_eq!(
                ConnectionIntegrationInstanceId::parse(invalid),
                Err(ConnectionIntegrationInstanceIdError),
                "{invalid}"
            );
        }
    }
}
