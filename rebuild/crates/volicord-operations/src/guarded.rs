use crate::Error;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, path::Path};
use volicord_context::{
    Availability, CanonicalReadBasis, Principal, PrincipalKind, ProjectId, SourceId, SourcePayload,
    TimestampMicros,
};
use volicord_privacy::{
    AuthorizedProviderDispatch, BackgroundSemanticProvider, PrivacyStore, ProviderRequestOutcome,
    ProviderRequestRecord, TransmissionOutcome,
};

const GUARDED_SCHEMA_KIND: &str = "volicord-guarded-operations";
const GUARDED_SCHEMA_VERSION: u32 = 1;

macro_rules! guarded_identity {
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

guarded_identity!(ConfirmationRequestId);
guarded_identity!(ConfirmationResponseId);
guarded_identity!(GuardedOperationId);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardedEffectCategory {
    DestructiveFileOrDataDeletion,
    IrreversibleOrLargeScaleMigration,
    ExternalDeploymentOrPublicPublication,
    PaymentOrContinuingCost,
    SecretOrCredentialAccessOrChange,
    PersonalDataOrSourceCodeExternalTransmission,
    ExternalMessageEmailOrIssue,
    ProductionDataChange,
    PermissionAuthenticationOrSecuritySettingChange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GuardedRisk {
    pub category: GuardedEffectCategory,
    pub concrete_consequence: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestingProvenance {
    pub actor: Principal,
    pub host: Option<String>,
    pub session: Option<String>,
    pub basis: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackgroundProviderOperationDraft {
    pub project_id: ProjectId,
    pub provider: String,
    pub model: String,
    pub purpose: String,
    pub requested_capability: String,
    pub source_paths: Vec<String>,
    pub expires_at: TimestampMicros,
    pub requesting_provenance: RequestingProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardedEffectDraft {
    pub project_id: ProjectId,
    pub exact_action: String,
    pub target: String,
    pub expected_effect: String,
    pub risk: GuardedRisk,
    pub scope: Vec<String>,
    pub expires_at: TimestampMicros,
    pub requesting_provenance: RequestingProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GuardedEffectCandidate {
    pub confirmation_request_identity: ConfirmationRequestId,
    pub request_revision: u64,
    pub project_id: ProjectId,
    pub exact_action: String,
    pub target: String,
    pub expected_effect: String,
    pub risk: GuardedRisk,
    pub scope: Vec<String>,
    pub expires_at: TimestampMicros,
    pub requesting_provenance: RequestingProvenance,
    pub effect_fingerprint: String,
    pub created_at: TimestampMicros,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationDecision {
    Confirmed,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConfirmationResponse {
    pub confirmation_response_identity: ConfirmationResponseId,
    pub confirmation_request_identity: ConfirmationRequestId,
    pub request_revision: u64,
    pub project_id: ProjectId,
    pub exact_action: String,
    pub target: String,
    pub expected_effect: String,
    pub risk: GuardedRisk,
    pub scope: Vec<String>,
    pub effect_fingerprint: String,
    pub decision: ConfirmationDecision,
    pub user_response_source_id: SourceId,
    pub responded_at: TimestampMicros,
}

impl ConfirmationResponse {
    pub fn exact_for(
        candidate: &GuardedEffectCandidate,
        decision: ConfirmationDecision,
        user_response_source_id: SourceId,
        responded_at: TimestampMicros,
    ) -> Result<Self, Error> {
        Ok(Self {
            confirmation_response_identity: ConfirmationResponseId::from_bytes(random_identity()?),
            confirmation_request_identity: candidate.confirmation_request_identity,
            request_revision: candidate.request_revision,
            project_id: candidate.project_id,
            exact_action: candidate.exact_action.clone(),
            target: candidate.target.clone(),
            expected_effect: candidate.expected_effect.clone(),
            risk: candidate.risk.clone(),
            scope: candidate.scope.clone(),
            effect_fingerprint: candidate.effect_fingerprint.clone(),
            decision,
            user_response_source_id,
            responded_at,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchExpectation {
    pub exact_action: String,
    pub target: String,
    pub expected_effect: String,
    pub risk: GuardedRisk,
    pub scope: Vec<String>,
    pub effect_fingerprint: String,
}

impl From<&GuardedEffectCandidate> for DispatchExpectation {
    fn from(value: &GuardedEffectCandidate) -> Self {
        Self {
            exact_action: value.exact_action.clone(),
            target: value.target.clone(),
            expected_effect: value.expected_effect.clone(),
            risk: value.risk.clone(),
            scope: value.scope.clone(),
            effect_fingerprint: value.effect_fingerprint.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationRejection {
    Missing,
    Denied,
    Stale,
    Expired,
    Mismatched,
    Reused,
    InvalidUserSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DispatchObservation {
    NotDispatched { diagnostic: String },
    DispatchedAndCompleted { diagnostic: Option<String> },
    DispatchedAndFailed { diagnostic: String },
    ExecutionOutcomeIndeterminate { diagnostic: String },
}

pub trait GuardedEffectDispatcher {
    fn dispatch(
        &mut self,
        operation_id: GuardedOperationId,
        effect: &GuardedEffectCandidate,
    ) -> DispatchObservation;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GuardedOperationOutcome {
    NotDispatched {
        rejection: Option<ConfirmationRejection>,
        confirmation_consumed: bool,
        diagnostic: String,
    },
    DispatchedAndCompleted {
        diagnostic: Option<String>,
    },
    DispatchedAndFailed {
        diagnostic: String,
    },
    ExecutionOutcomeIndeterminate {
        diagnostic: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GuardedOperationResult {
    pub operation_identity: GuardedOperationId,
    pub confirmation_request_identity: ConfirmationRequestId,
    pub request_revision: u64,
    pub user_response_source_id: Option<SourceId>,
    pub outcome: GuardedOperationOutcome,
    pub started_at: TimestampMicros,
    pub completed_at: TimestampMicros,
}

/// An authorized provider request held only for the live Guarded interaction.
/// Filtered source bodies remain ephemeral and are never serialized into either
/// operational store.
pub struct GuardedProviderPreparation {
    pub candidate: GuardedEffectCandidate,
    pub provider_request: ProviderRequestRecord,
    pub(crate) authorized: Option<AuthorizedProviderDispatch>,
}

impl GuardedProviderPreparation {
    /// Reports whether this live preparation still retains the authorized,
    /// privacy-filtered provider payload. Once dispatch takes the payload the
    /// preparation is terminal even if durable outcome recording later fails.
    pub fn retains_authorized_payload(&self) -> bool {
        self.authorized.is_some()
    }
}

pub enum GuardedProviderPreparationOutcome {
    Ready(Box<GuardedProviderPreparation>),
    Rejected(Box<ProviderRequestRecord>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardedProviderInspection {
    pub request: GuardedEffectCandidate,
    pub operation: GuardedOperationResult,
    pub provider_request: ProviderRequestRecord,
}

pub struct GuardedStore {
    connection: Connection,
}

impl GuardedStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(Error::new(
                "Guarded operational store path must be explicit",
            ));
        }
        let mut connection = Connection::open(path)
            .map_err(|error| Error::with_source("cannot open Guarded operational store", error))?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;",
            )
            .map_err(|error| {
                Error::with_source("cannot configure Guarded operational durability", error)
            })?;
        initialize_or_validate(&mut connection)?;
        Ok(Self { connection })
    }

    pub fn create_request(
        &mut self,
        draft: GuardedEffectDraft,
        created_at: TimestampMicros,
    ) -> Result<GuardedEffectCandidate, Error> {
        validate_draft(&draft, created_at)?;
        let identity = ConfirmationRequestId::from_bytes(random_identity()?);
        let candidate = candidate_from_draft(identity, 1, draft, created_at);
        self.insert_candidate(&candidate)?;
        Ok(candidate)
    }

    pub fn revise_request(
        &mut self,
        request: ConfirmationRequestId,
        expected_revision: u64,
        draft: GuardedEffectDraft,
        created_at: TimestampMicros,
    ) -> Result<GuardedEffectCandidate, Error> {
        validate_draft(&draft, created_at)?;
        let current = self.current_request(request)?;
        if current.request_revision != expected_revision {
            return Err(Error::new(
                "Guarded confirmation request changed concurrently",
            ));
        }
        if current.project_id != draft.project_id {
            return Err(Error::new(
                "Guarded confirmation request cannot transfer to another Project",
            ));
        }
        let revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| Error::new("Guarded confirmation request revision is exhausted"))?;
        let candidate = candidate_from_draft(request, revision, draft, created_at);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| Error::with_source("cannot begin Guarded request revision", error))?;
        let updated = transaction.execute(
            "UPDATE guarded_requests SET is_current = 0 WHERE request_id = ?1 AND revision = ?2 AND is_current = 1",
            params![request.as_bytes().as_slice(), revision_i64(expected_revision)?],
        ).map_err(|error| Error::with_source("cannot retire Guarded request revision", error))?;
        if updated != 1 {
            return Err(Error::new(
                "Guarded confirmation request changed concurrently",
            ));
        }
        insert_candidate_transaction(&transaction, &candidate)?;
        transaction
            .commit()
            .map_err(|error| Error::with_source("cannot commit Guarded request revision", error))?;
        Ok(candidate)
    }

    pub fn current_request(
        &self,
        request: ConfirmationRequestId,
    ) -> Result<GuardedEffectCandidate, Error> {
        self.connection.query_row(
            "SELECT request_json FROM guarded_requests WHERE request_id = ?1 AND is_current = 1",
            [request.as_bytes().as_slice()],
            |row| row.get::<_, String>(0),
        ).optional().map_err(|error| Error::with_source("cannot read current Guarded request", error))?
            .ok_or_else(|| Error::new("Guarded confirmation request was not found"))
            .and_then(|encoded| decode(&encoded, "Guarded confirmation request"))
    }

    pub fn request(
        &self,
        request: ConfirmationRequestId,
        revision: u64,
    ) -> Result<GuardedEffectCandidate, Error> {
        self.connection
            .query_row(
                "SELECT request_json FROM guarded_requests WHERE request_id = ?1 AND revision = ?2",
                params![request.as_bytes().as_slice(), revision_i64(revision)?],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| Error::with_source("cannot read Guarded request revision", error))?
            .ok_or_else(|| Error::new("Guarded confirmation request revision was not found"))
            .and_then(|encoded| decode(&encoded, "Guarded confirmation request"))
    }

    pub fn record_response(
        &mut self,
        response: ConfirmationResponse,
    ) -> Result<ConfirmationResponse, Error> {
        let request = self.request(
            response.confirmation_request_identity,
            response.request_revision,
        )?;
        if request.project_id != response.project_id {
            return Err(Error::new(
                "confirmation response belongs to a different Project",
            ));
        }
        let encoded = encode(&response, "confirmation response")?;
        self.connection.execute(
            "INSERT INTO confirmation_responses(request_id, revision, response_id, response_json, user_source_id, consumed_operation_id) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![response.confirmation_request_identity.as_bytes().as_slice(), revision_i64(response.request_revision)?, response.confirmation_response_identity.as_bytes().as_slice(), encoded, response.user_response_source_id.as_bytes().as_slice()],
        ).map_err(|error| Error::with_source("a confirmation response already exists for this exact request revision", error))?;
        Ok(response)
    }

    pub fn response(
        &self,
        request: ConfirmationRequestId,
        revision: u64,
    ) -> Result<Option<ConfirmationResponse>, Error> {
        self.connection.query_row(
            "SELECT response_json FROM confirmation_responses WHERE request_id = ?1 AND revision = ?2",
            params![request.as_bytes().as_slice(), revision_i64(revision)?],
            |row| row.get::<_, String>(0),
        ).optional().map_err(|error| Error::with_source("cannot read confirmation response", error))?
            .map(|encoded| decode(&encoded, "confirmation response")).transpose()
    }

    pub fn operation(
        &self,
        operation: GuardedOperationId,
    ) -> Result<GuardedOperationResult, Error> {
        self.connection
            .query_row(
                "SELECT result_json FROM guarded_operations WHERE operation_id = ?1",
                [operation.as_bytes().as_slice()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| Error::with_source("cannot read Guarded operation", error))?
            .ok_or_else(|| Error::new("Guarded operation was not found"))
            .and_then(|encoded| decode(&encoded, "Guarded operation"))
    }

    pub fn dispatch(
        &mut self,
        request_identity: ConfirmationRequestId,
        request_revision: u64,
        expectation: &DispatchExpectation,
        canonical: &CanonicalReadBasis,
        now: TimestampMicros,
        dispatcher: &mut dyn GuardedEffectDispatcher,
    ) -> Result<GuardedOperationResult, Error> {
        let operation_identity = GuardedOperationId::from_bytes(random_identity()?);
        let current = self.current_request(request_identity)?;
        if current.request_revision != request_revision {
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                None,
                ConfirmationRejection::Stale,
                false,
                "request revision is no longer current",
                now,
            );
        }
        if !expectation_matches(&current, expectation) {
            return self.not_dispatched(operation_identity, &current, request_revision, None, ConfirmationRejection::Mismatched, false, "dispatch action, target, effect, risk, scope, or fingerprint does not exactly match", now);
        }
        if now >= current.expires_at {
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                None,
                ConfirmationRejection::Expired,
                false,
                "confirmation request expired before dispatch",
                now,
            );
        }
        let response = match self.response(request_identity, request_revision)? {
            Some(response) => response,
            None => {
                return self.not_dispatched(
                    operation_identity,
                    &current,
                    request_revision,
                    None,
                    ConfirmationRejection::Missing,
                    false,
                    "no explicit confirmation response exists",
                    now,
                )
            }
        };
        if !response_matches(&current, &response) {
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                Some(response.user_response_source_id),
                ConfirmationRejection::Mismatched,
                false,
                "confirmation response does not exactly match the request",
                now,
            );
        }
        if !valid_current_host_user_source(canonical, &current, &response) {
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                Some(response.user_response_source_id),
                ConfirmationRejection::InvalidUserSource,
                false,
                "confirmation is not linked to an available current-host user-authored Source",
                now,
            );
        }
        if response.decision == ConfirmationDecision::Denied {
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                Some(response.user_response_source_id),
                ConfirmationRejection::Denied,
                false,
                "current-host user explicitly denied the effect",
                now,
            );
        }
        if self
            .consumed_by(request_identity, request_revision)?
            .is_some()
        {
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                Some(response.user_response_source_id),
                ConfirmationRejection::Reused,
                false,
                "confirmation was already consumed and is single-use",
                now,
            );
        }

        let indeterminate = GuardedOperationResult {
            operation_identity,
            confirmation_request_identity: request_identity,
            request_revision,
            user_response_source_id: Some(response.user_response_source_id),
            outcome: GuardedOperationOutcome::ExecutionOutcomeIndeterminate { diagnostic: "confirmation was consumed; dispatch completion has not yet been durably observed".into() },
            started_at: now,
            completed_at: now,
        };
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| Error::with_source("cannot begin confirmation consumption", error))?;
        let consumed = transaction.execute(
            "UPDATE confirmation_responses SET consumed_operation_id = ?3 WHERE request_id = ?1 AND revision = ?2 AND consumed_operation_id IS NULL",
            params![request_identity.as_bytes().as_slice(), revision_i64(request_revision)?, operation_identity.as_bytes().as_slice()],
        ).map_err(|error| Error::with_source("cannot consume confirmation", error))?;
        if consumed != 1 {
            transaction.rollback().map_err(|error| {
                Error::with_source("cannot roll back raced confirmation consumption", error)
            })?;
            return self.not_dispatched(
                operation_identity,
                &current,
                request_revision,
                Some(response.user_response_source_id),
                ConfirmationRejection::Reused,
                false,
                "confirmation was consumed concurrently",
                now,
            );
        }
        insert_operation_transaction(&transaction, &indeterminate)?;
        transaction
            .commit()
            .map_err(|error| Error::with_source("cannot commit confirmation consumption", error))?;

        let observation = dispatcher.dispatch(operation_identity, &current);
        let completed_at = now;
        let outcome = match observation {
            DispatchObservation::NotDispatched { diagnostic } => {
                GuardedOperationOutcome::NotDispatched {
                    rejection: None,
                    confirmation_consumed: true,
                    diagnostic,
                }
            }
            DispatchObservation::DispatchedAndCompleted { diagnostic } => {
                GuardedOperationOutcome::DispatchedAndCompleted { diagnostic }
            }
            DispatchObservation::DispatchedAndFailed { diagnostic } => {
                GuardedOperationOutcome::DispatchedAndFailed { diagnostic }
            }
            DispatchObservation::ExecutionOutcomeIndeterminate { diagnostic } => {
                GuardedOperationOutcome::ExecutionOutcomeIndeterminate { diagnostic }
            }
        };
        let result = GuardedOperationResult {
            outcome,
            completed_at,
            ..indeterminate
        };
        self.update_operation(&result)?;
        Ok(result)
    }

    fn insert_candidate(&mut self, candidate: &GuardedEffectCandidate) -> Result<(), Error> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| Error::with_source("cannot begin Guarded request creation", error))?;
        insert_candidate_transaction(&transaction, candidate)?;
        transaction
            .commit()
            .map_err(|error| Error::with_source("cannot commit Guarded request creation", error))
    }

    fn consumed_by(
        &self,
        request: ConfirmationRequestId,
        revision: u64,
    ) -> Result<Option<GuardedOperationId>, Error> {
        let bytes: Option<Vec<u8>> = self.connection.query_row(
            "SELECT consumed_operation_id FROM confirmation_responses WHERE request_id = ?1 AND revision = ?2",
            params![request.as_bytes().as_slice(), revision_i64(revision)?],
            |row| row.get(0),
        ).optional().map_err(|error| Error::with_source("cannot inspect confirmation consumption", error))?.flatten();
        bytes
            .map(|bytes| id_from_slice(&bytes).map(GuardedOperationId::from_bytes))
            .transpose()
    }

    #[allow(clippy::too_many_arguments)]
    fn not_dispatched(
        &mut self,
        operation_identity: GuardedOperationId,
        request: &GuardedEffectCandidate,
        attempted_revision: u64,
        user_response_source_id: Option<SourceId>,
        rejection: ConfirmationRejection,
        confirmation_consumed: bool,
        diagnostic: &str,
        now: TimestampMicros,
    ) -> Result<GuardedOperationResult, Error> {
        let result = GuardedOperationResult {
            operation_identity,
            confirmation_request_identity: request.confirmation_request_identity,
            request_revision: attempted_revision,
            user_response_source_id,
            outcome: GuardedOperationOutcome::NotDispatched {
                rejection: Some(rejection),
                confirmation_consumed,
                diagnostic: diagnostic.into(),
            },
            started_at: now,
            completed_at: now,
        };
        self.insert_operation(&result)?;
        Ok(result)
    }

    fn insert_operation(&mut self, result: &GuardedOperationResult) -> Result<(), Error> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| Error::with_source("cannot begin Guarded operation record", error))?;
        insert_operation_transaction(&transaction, result)?;
        transaction
            .commit()
            .map_err(|error| Error::with_source("cannot commit Guarded operation record", error))
    }

    fn update_operation(&mut self, result: &GuardedOperationResult) -> Result<(), Error> {
        let encoded = encode(result, "Guarded operation")?;
        let updated = self.connection.execute(
            "UPDATE guarded_operations SET result_json = ?2, completed_at = ?3 WHERE operation_id = ?1",
            params![result.operation_identity.as_bytes().as_slice(), encoded, result.completed_at.as_unix_micros()],
        ).map_err(|error| Error::with_source("cannot record Guarded dispatch outcome", error))?;
        if updated != 1 {
            return Err(Error::new(
                "consumed Guarded operation disappeared before outcome recording",
            ));
        }
        Ok(())
    }
}

pub struct BackgroundProviderDispatcher<'a> {
    privacy: &'a mut PrivacyStore,
    prepared: &'a mut Option<AuthorizedProviderDispatch>,
    expected_fingerprint: String,
    provider: &'a mut dyn BackgroundSemanticProvider,
}

impl<'a> BackgroundProviderDispatcher<'a> {
    pub fn new(
        privacy: &'a mut PrivacyStore,
        prepared: &'a mut Option<AuthorizedProviderDispatch>,
        expected: &GuardedEffectCandidate,
        provider: &'a mut dyn BackgroundSemanticProvider,
    ) -> Self {
        Self {
            privacy,
            prepared,
            expected_fingerprint: expected.effect_fingerprint.clone(),
            provider,
        }
    }
}

impl GuardedEffectDispatcher for BackgroundProviderDispatcher<'_> {
    fn dispatch(
        &mut self,
        _operation_id: GuardedOperationId,
        effect: &GuardedEffectCandidate,
    ) -> DispatchObservation {
        if effect.effect_fingerprint != self.expected_fingerprint {
            return DispatchObservation::NotDispatched {
                diagnostic: "prepared provider request is not bound to this exact Guarded effect"
                    .into(),
            };
        }
        let Some(prepared) = self.prepared.take() else {
            return DispatchObservation::NotDispatched {
                diagnostic: "provider dispatch adapter is single-use".into(),
            };
        };
        match self.privacy.dispatch_background(prepared, self.provider) {
            Ok(record) => {
                let transmitted = record
                    .manifest
                    .iter()
                    .any(|entry| entry.transmission_outcome == TransmissionOutcome::Transmitted);
                let diagnostic = record.diagnostic.clone();
                match (transmitted, record.outcome) {
                    (false, _) => DispatchObservation::NotDispatched {
                        diagnostic: diagnostic.unwrap_or_else(|| {
                            format!(
                                "provider request ended as {:?} before transmission",
                                record.outcome
                            )
                        }),
                    },
                    (true, ProviderRequestOutcome::Completed | ProviderRequestOutcome::Partial) => {
                        DispatchObservation::DispatchedAndCompleted { diagnostic }
                    }
                    (
                        true,
                        ProviderRequestOutcome::ProviderFailed
                        | ProviderRequestOutcome::ProviderTimedOut
                        | ProviderRequestOutcome::ProviderCancelled
                        | ProviderRequestOutcome::Stale,
                    ) => DispatchObservation::DispatchedAndFailed {
                        diagnostic: diagnostic.unwrap_or_else(|| {
                            format!("provider request ended as {:?}", record.outcome)
                        }),
                    },
                    (true, outcome) => DispatchObservation::ExecutionOutcomeIndeterminate {
                        diagnostic: diagnostic.unwrap_or_else(|| {
                            format!("provider request transmitted source but ended as {outcome:?}")
                        }),
                    },
                }
            }
            Err(error) => DispatchObservation::ExecutionOutcomeIndeterminate {
                diagnostic: format!(
                    "provider boundary returned an error after dispatch began: {error}"
                ),
            },
        }
    }
}

fn candidate_from_draft(
    identity: ConfirmationRequestId,
    revision: u64,
    draft: GuardedEffectDraft,
    created_at: TimestampMicros,
) -> GuardedEffectCandidate {
    let fingerprint = effect_fingerprint(identity, revision, &draft);
    GuardedEffectCandidate {
        confirmation_request_identity: identity,
        request_revision: revision,
        project_id: draft.project_id,
        exact_action: draft.exact_action,
        target: draft.target,
        expected_effect: draft.expected_effect,
        risk: draft.risk,
        scope: draft.scope,
        expires_at: draft.expires_at,
        requesting_provenance: draft.requesting_provenance,
        effect_fingerprint: fingerprint,
        created_at,
    }
}

fn effect_fingerprint(
    identity: ConfirmationRequestId,
    revision: u64,
    draft: &GuardedEffectDraft,
) -> String {
    let mut digest = Sha256::new();
    hash_field(&mut digest, identity.as_bytes());
    hash_field(&mut digest, &revision.to_be_bytes());
    hash_field(&mut digest, draft.exact_action.as_bytes());
    hash_field(&mut digest, draft.target.as_bytes());
    hash_field(&mut digest, draft.expected_effect.as_bytes());
    hash_field(&mut digest, format!("{:?}", draft.risk.category).as_bytes());
    hash_field(&mut digest, draft.risk.concrete_consequence.as_bytes());
    for scope in &draft.scope {
        hash_field(&mut digest, scope.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

fn hash_field(digest: &mut Sha256, field: &[u8]) {
    digest.update((field.len() as u64).to_be_bytes());
    digest.update(field);
}

fn validate_draft(draft: &GuardedEffectDraft, created_at: TimestampMicros) -> Result<(), Error> {
    validate_text("exact action", &draft.exact_action)?;
    validate_text("target", &draft.target)?;
    validate_text("expected effect", &draft.expected_effect)?;
    validate_text("risk consequence", &draft.risk.concrete_consequence)?;
    if draft.scope.is_empty() {
        return Err(Error::new(
            "Guarded effect scope must be explicitly bounded",
        ));
    }
    for scope in &draft.scope {
        validate_text("Guarded effect scope", scope)?;
    }
    if draft.expires_at <= created_at {
        return Err(Error::new(
            "Guarded confirmation expiration must be after request creation",
        ));
    }
    if draft.requesting_provenance.basis.is_empty() {
        return Err(Error::new(
            "Guarded request must preserve requesting provenance",
        ));
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::new(format!("{label} must not be empty")));
    }
    if value.len() > 16_384 {
        return Err(Error::new(format!("{label} exceeds the operational bound")));
    }
    Ok(())
}

fn expectation_matches(
    candidate: &GuardedEffectCandidate,
    expectation: &DispatchExpectation,
) -> bool {
    candidate.exact_action == expectation.exact_action
        && candidate.target == expectation.target
        && candidate.expected_effect == expectation.expected_effect
        && candidate.risk == expectation.risk
        && candidate.scope == expectation.scope
        && candidate.effect_fingerprint == expectation.effect_fingerprint
}

fn response_matches(candidate: &GuardedEffectCandidate, response: &ConfirmationResponse) -> bool {
    response.confirmation_request_identity == candidate.confirmation_request_identity
        && response.request_revision == candidate.request_revision
        && response.project_id == candidate.project_id
        && response.exact_action == candidate.exact_action
        && response.target == candidate.target
        && response.expected_effect == candidate.expected_effect
        && response.risk == candidate.risk
        && response.scope == candidate.scope
        && response.effect_fingerprint == candidate.effect_fingerprint
}

fn valid_current_host_user_source(
    canonical: &CanonicalReadBasis,
    candidate: &GuardedEffectCandidate,
    response: &ConfirmationResponse,
) -> bool {
    canonical.project.id == candidate.project_id
        && response.project_id == candidate.project_id
        && canonical.sources.iter().any(|basis| {
            basis.source.id == response.user_response_source_id
                && basis.source.project_id == candidate.project_id
                && basis.source.actor.kind == PrincipalKind::User
                && basis.source.availability == Availability::Available
                && matches!(
                    basis.source.payload,
                    SourcePayload::CurrentHostUserTurn { .. }
                )
        })
}

fn insert_candidate_transaction(
    transaction: &rusqlite::Transaction<'_>,
    candidate: &GuardedEffectCandidate,
) -> Result<(), Error> {
    let encoded = encode(candidate, "Guarded confirmation request")?;
    transaction.execute(
        "INSERT INTO guarded_requests(request_id, revision, project_id, request_json, is_current) VALUES (?1, ?2, ?3, ?4, 1)",
        params![candidate.confirmation_request_identity.as_bytes().as_slice(), revision_i64(candidate.request_revision)?, candidate.project_id.as_bytes().as_slice(), encoded],
    ).map_err(|error| Error::with_source("cannot store Guarded confirmation request", error))?;
    Ok(())
}

fn insert_operation_transaction(
    transaction: &rusqlite::Transaction<'_>,
    result: &GuardedOperationResult,
) -> Result<(), Error> {
    let encoded = encode(result, "Guarded operation")?;
    transaction.execute(
        "INSERT INTO guarded_operations(operation_id, request_id, revision, result_json, started_at, completed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![result.operation_identity.as_bytes().as_slice(), result.confirmation_request_identity.as_bytes().as_slice(), revision_i64(result.request_revision)?, encoded, result.started_at.as_unix_micros(), result.completed_at.as_unix_micros()],
    ).map_err(|error| Error::with_source("cannot store Guarded operation", error))?;
    Ok(())
}

fn initialize_or_validate(connection: &mut Connection) -> Result<(), Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS guarded_meta(kind TEXT NOT NULL, version INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS guarded_requests(
             request_id BLOB NOT NULL, revision INTEGER NOT NULL, project_id BLOB NOT NULL,
             request_json TEXT NOT NULL, is_current INTEGER NOT NULL,
             PRIMARY KEY(request_id, revision)
         );
         CREATE UNIQUE INDEX IF NOT EXISTS one_current_guarded_request ON guarded_requests(request_id) WHERE is_current = 1;
         CREATE TABLE IF NOT EXISTS confirmation_responses(
             request_id BLOB NOT NULL, revision INTEGER NOT NULL, response_id BLOB NOT NULL UNIQUE,
             response_json TEXT NOT NULL, user_source_id BLOB NOT NULL,
             consumed_operation_id BLOB,
             PRIMARY KEY(request_id, revision),
             FOREIGN KEY(request_id, revision) REFERENCES guarded_requests(request_id, revision)
         );
         CREATE TABLE IF NOT EXISTS guarded_operations(
             operation_id BLOB PRIMARY KEY, request_id BLOB NOT NULL, revision INTEGER NOT NULL,
             result_json TEXT NOT NULL, started_at INTEGER NOT NULL, completed_at INTEGER NOT NULL
         );",
    ).map_err(|error| Error::with_source("cannot initialize Guarded operational schema", error))?;
    let current: Option<(String, i64)> = connection
        .query_row(
            "SELECT kind, version FROM guarded_meta LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| Error::with_source("cannot read Guarded schema version", error))?;
    match current {
        None => {
            connection.execute("INSERT INTO guarded_meta(kind, version) VALUES (?1, ?2)", params![GUARDED_SCHEMA_KIND, i64::from(GUARDED_SCHEMA_VERSION)])
                .map_err(|error| Error::with_source("cannot record Guarded schema version", error))?;
        }
        Some((kind, version)) if kind == GUARDED_SCHEMA_KIND && version == i64::from(GUARDED_SCHEMA_VERSION) => {}
        Some((kind, version)) => return Err(Error::new(format!("unsupported Guarded operational schema {kind} version {version}; expected {GUARDED_SCHEMA_KIND} version {GUARDED_SCHEMA_VERSION}"))),
    }
    Ok(())
}

fn encode<T: Serialize>(value: &T, label: &str) -> Result<String, Error> {
    serde_json::to_string(value)
        .map_err(|error| Error::with_source(format!("cannot encode {label}"), error))
}
fn decode<T: for<'de> Deserialize<'de>>(value: &str, label: &str) -> Result<T, Error> {
    serde_json::from_str(value)
        .map_err(|error| Error::with_source(format!("stored {label} is corrupt"), error))
}
fn random_identity() -> Result<[u8; 16], Error> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| Error::with_source("operating-system randomness is unavailable", error))?;
    Ok(bytes)
}
fn revision_i64(revision: u64) -> Result<i64, Error> {
    i64::try_from(revision)
        .map_err(|_| Error::new("revision exceeds the operational storage range"))
}
fn id_from_slice(value: &[u8]) -> Result<[u8; 16], Error> {
    value
        .try_into()
        .map_err(|_| Error::new("stored Guarded identity is not 128 bits"))
}
