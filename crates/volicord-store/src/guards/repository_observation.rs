use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_host_contract::HostNativeCorrelation;
use volicord_host_contract::HostTurnId;
use volicord_platform_fs::{
    ObservationUnavailableReason, ObserverLimits, RepositoryDelta, RepositoryObservationCheckpoint,
    SemanticObserverContractDigest,
};
use volicord_types::canonical::{canonical_json_string, is_canonical_sha256_digest};
use volicord_types::product_path::ProductRelativePath;
use volicord_types::schema::JsonObject;
use volicord_types::values::UtcTimestamp;

use super::{
    begin_immediate_transaction, current_guard_manifest, establish_host_correlation_in_transaction,
    guard_correlation_fields, guard_event_by_conn, guard_event_from_conn, guard_installation,
    open_guard_project, open_project_for_read, strict_stored_timestamp,
    validate_guard_event_insert, validate_identifier, validate_string_items,
    GuardCorrelationFields, GuardEventInsert, GuardEventRecord,
};
use crate::{RuntimeHomeMutationContext, StoreError, StoreResult};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepositoryObservationFaultPoint {
    PreAfterCorrelation,
    PreAfterGuardEvent,
    PreAfterObservation,
    PreAfterExpectedWrite,
    PostAfterExpectedWriteReconciliation,
    PostAfterUnrecordedChangeInsert,
    PostAfterGuardEvent,
    PostAfterObservation,
    TerminalizationAfterFirstUpdate,
}

#[cfg(test)]
thread_local! {
    static REPOSITORY_OBSERVATION_FAULT_POINT:
        std::cell::Cell<Option<RepositoryObservationFaultPoint>> =
            const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(super) fn set_repository_observation_fault_point(
    point: Option<RepositoryObservationFaultPoint>,
) {
    REPOSITORY_OBSERVATION_FAULT_POINT.set(point);
}

#[cfg(test)]
fn inject_repository_observation_fault(point: RepositoryObservationFaultPoint) -> StoreResult<()> {
    if REPOSITORY_OBSERVATION_FAULT_POINT.get() == Some(point) {
        REPOSITORY_OBSERVATION_FAULT_POINT.set(None);
        return Err(StoreError::InvalidInput {
            detail: format!("injected repository-observation fault at {point:?}"),
        });
    }
    Ok(())
}

/// Closed lifecycle state for one invocation-scoped repository observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryObservationState {
    Open,
    Complete,
    Unavailable,
}

impl RepositoryObservationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Complete => "complete",
            Self::Unavailable => "unavailable",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "complete" => Some(Self::Complete),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

/// Closed reason an invocation-scoped observation is terminal without a delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryObservationUnavailableReason {
    Observer(ObservationUnavailableReason),
    InvocationDenied,
    MissingOpenObservation,
    PostToolNotObserved,
    ManagedSessionTerminated,
}

impl RepositoryObservationUnavailableReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observer(reason) => reason.as_str(),
            Self::InvocationDenied => "invocation_denied",
            Self::MissingOpenObservation => "missing_open_observation",
            Self::PostToolNotObserved => "post_tool_not_observed",
            Self::ManagedSessionTerminated => "managed_session_terminated",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "invocation_denied" => Some(Self::InvocationDenied),
            "missing_open_observation" => Some(Self::MissingOpenObservation),
            "post_tool_not_observed" => Some(Self::PostToolNotObserved),
            "managed_session_terminated" => Some(Self::ManagedSessionTerminated),
            value => ObservationUnavailableReason::parse(value).map(Self::Observer),
        }
    }
}

/// Maximum number of exact open observations one aggregate operation may terminalize.
pub const REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT: usize = 256;

/// Exact authority boundary used to select open observations for terminalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRepositoryObservationTerminalizationScope {
    /// Select only established turns other than the accepted prompt's exact current turn.
    EarlierTurns { current_turn_id: HostTurnId },
    /// Select every open observation owned by one exact managed project session.
    ManagedSession,
}

/// Typed input for bounded open-observation terminalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalizeOpenRepositoryObservations {
    pub connection_internal_id: String,
    pub session_id: String,
    pub scope: OpenRepositoryObservationTerminalizationScope,
    pub reason: RepositoryObservationUnavailableReason,
    pub completed_at: UtcTimestamp,
}

/// Deterministically ordered result of one bounded terminalization transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryObservationTerminalizationResult {
    pub terminalized: Vec<RepositoryObservationRecord>,
}

/// Read-only lifecycle classification for one invocation-scoped observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryObservationDiagnosticStatus {
    OpenCurrentTurn,
    UnavailablePostToolNotObserved,
    UnavailableManagedSessionTerminated,
    OrphanOpenTerminalSession,
    CleanupFailed,
    CorruptObservation,
}

impl RepositoryObservationDiagnosticStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenCurrentTurn => "open_current_turn",
            Self::UnavailablePostToolNotObserved => "unavailable_post_tool_not_observed",
            Self::UnavailableManagedSessionTerminated => "unavailable_managed_session_terminated",
            Self::OrphanOpenTerminalSession => "orphan_open_terminal_session",
            Self::CleanupFailed => "cleanup_failed",
            Self::CorruptObservation => "corrupt_observation",
        }
    }
}

/// Bounded read-only diagnostic projection for one observation or failed binding read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepositoryObservationDiagnosticRecord {
    pub project_id: String,
    pub session_id: String,
    pub repository_observation_id: Option<String>,
    pub status: RepositoryObservationDiagnosticStatus,
}

impl From<ObservationUnavailableReason> for RepositoryObservationUnavailableReason {
    fn from(value: ObservationUnavailableReason) -> Self {
        Self::Observer(value)
    }
}

/// Exact expected Product Repository write attached to one observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryExpectedWriteInsert {
    pub expected_write_id: String,
    pub command_kind: String,
    pub expected_paths: Vec<ProductRelativePath>,
    pub task_id: String,
    pub change_unit_id: String,
    pub write_ticket_ids: Vec<String>,
    pub basis_state_version: u64,
    pub created_at: UtcTimestamp,
    pub metadata: JsonObject,
}

/// Atomic PreToolUse aggregate input.
#[derive(Debug, Clone)]
pub struct PreToolRepositoryObservationInsert {
    pub guard_event: GuardEventInsert,
    pub repository_observation_id: String,
    pub observer_contract_digest: String,
    pub checkpoint: Option<RepositoryObservationCheckpoint>,
    pub unavailable_reason: Option<RepositoryObservationUnavailableReason>,
    pub expected_write: Option<RepositoryExpectedWriteInsert>,
    pub metadata: JsonObject,
}

/// Complete or unavailable PostToolUse observation outcome.
#[derive(Debug, Clone)]
pub enum PostToolRepositoryObservationOutcome {
    Complete {
        post_snapshot: Box<RepositoryObservationCheckpoint>,
        delta: RepositoryDelta,
    },
    Unavailable {
        reason: RepositoryObservationUnavailableReason,
    },
}

/// Atomic PostToolUse aggregate input.
#[derive(Debug, Clone)]
pub struct PostToolRepositoryObservationInsert {
    pub guard_event: GuardEventInsert,
    pub repository_observation_id: String,
    pub observer_contract_digest: String,
    pub outcome: PostToolRepositoryObservationOutcome,
    pub task_id: Option<String>,
    pub metadata: JsonObject,
}

/// Strictly decoded repository-observation aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryObservationRecord {
    pub project_id: String,
    pub repository_observation_id: String,
    pub session_id: String,
    pub correlation: HostNativeCorrelation,
    pub connection_internal_id: String,
    pub guard_installation_id: String,
    pub observer_contract_digest: String,
    pub pre_tool_guard_event_id: Option<String>,
    pub post_tool_guard_event_id: Option<String>,
    pub state: RepositoryObservationState,
    pub pre_snapshot: Option<RepositoryObservationCheckpoint>,
    pub post_snapshot: Option<RepositoryObservationCheckpoint>,
    pub delta: Option<RepositoryDelta>,
    pub unavailable_reason: Option<RepositoryObservationUnavailableReason>,
    pub started_at: UtcTimestamp,
    pub completed_at: Option<UtcTimestamp>,
    pub terminal_result: Option<RepositoryObservationResult>,
    pub metadata: JsonObject,
}

/// Complete delta summary returned by Guard and reused during replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryDeltaSummary {
    pub digest: String,
    pub paths: Vec<ProductRelativePath>,
    pub transition_count: usize,
}

/// Exact expected-write match result for one complete delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryExpectedWriteMatchResult {
    pub expected_write_id: String,
    pub matched_paths: Vec<ProductRelativePath>,
}

/// Unrecorded Change created from one unmatched delta subset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryUnrecordedChangeResult {
    pub unrecorded_change_id: String,
    pub unmatched_delta_digest: String,
    pub observed_paths: Vec<ProductRelativePath>,
}

/// Stable terminal repository-observation result persisted with PostToolUse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryObservationResult {
    pub observation_state: RepositoryObservationState,
    pub repository_observation_id: String,
    pub delta: Option<RepositoryDeltaSummary>,
    pub unavailable_reason: Option<String>,
    pub expected_write_matches: Vec<RepositoryExpectedWriteMatchResult>,
    pub unrecorded_changes: Vec<RepositoryUnrecordedChangeResult>,
    pub transition_semantics: String,
}

/// Atomic PostToolUse aggregate result.
#[derive(Debug, Clone)]
pub struct PostToolRepositoryObservationRecord {
    pub guard_event: GuardEventRecord,
    pub observation: RepositoryObservationRecord,
    pub result: RepositoryObservationResult,
    pub replayed: bool,
}

/// Atomic PreToolUse aggregate result.
#[derive(Debug, Clone)]
pub struct PreToolRepositoryObservationRecord {
    pub guard_event: GuardEventRecord,
    pub observation: RepositoryObservationRecord,
    pub replayed: bool,
}

/// Derives the stable identity for one exact native host tool invocation.
pub fn repository_observation_id(
    project_id: &str,
    connection_internal_id: &str,
    session_id: &str,
    correlation: &HostNativeCorrelation,
) -> StoreResult<String> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_identifier("session_id", session_id)?;
    let HostNativeCorrelation::CodexHookTool(tool) = correlation else {
        return Err(StoreError::InvalidInput {
            detail: "repository observations require exact Codex hook tool correlation".to_owned(),
        });
    };
    let mut encoder = Vec::new();
    for field in [
        "volicord.repository-observation",
        project_id,
        connection_internal_id,
        session_id,
        tool.session_id.as_str(),
        tool.turn_id.as_str(),
        tool.tool_use_id.as_str(),
        tool.tool_name.as_str(),
    ] {
        encoder.extend_from_slice(&(field.len() as u64).to_be_bytes());
        encoder.extend_from_slice(field.as_bytes());
    }
    Ok(format!(
        "repository_observation_{:x}",
        Sha256::digest(encoder)
    ))
}

/// Atomically records PreToolUse, its baseline or unavailable outcome, and its expected write.
pub fn record_pre_tool_repository_observation(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: PreToolRepositoryObservationInsert,
) -> StoreResult<PreToolRepositoryObservationRecord> {
    validate_pre_input(project_id, &input)?;
    let runtime_home = context.runtime_home().as_path();
    let fields = validate_guard_ownership(runtime_home, project_id, &input.guard_event)?;
    let expected_id = repository_observation_id(
        project_id,
        &input.guard_event.connection_internal_id,
        &fields.session_id,
        input
            .guard_event
            .correlation
            .as_ref()
            .expect("validated exact tool correlation"),
    )?;
    if expected_id != input.repository_observation_id {
        return Err(StoreError::InvalidInput {
            detail: "repository observation identity does not match exact host coordinates"
                .to_owned(),
        });
    }
    let pre_snapshot_json = input
        .checkpoint
        .as_ref()
        .map(canonical_json_string)
        .transpose()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("pre-tool snapshot cannot be serialized: {error}"),
        })?;
    let pre_snapshot_digest = input
        .checkpoint
        .as_ref()
        .map(RepositoryObservationCheckpoint::semantic_digest)
        .transpose()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("pre-tool snapshot is invalid: {error}"),
        })?
        .map(|digest| digest.as_str().to_owned());
    let observer_contract_digest = input.observer_contract_digest.clone();
    let metadata_json =
        canonical_json_string(&input.metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("repository-observation metadata cannot be serialized: {error}"),
        })?;
    let mut project = open_guard_project(
        context,
        project_id,
        &input.guard_event.connection_internal_id,
    )?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    if let Some(existing) = repository_observation_from_conn(
        &tx,
        &project.project.project_id,
        &input.repository_observation_id,
    )? {
        let pre_event_id = existing.pre_tool_guard_event_id.as_deref().ok_or_else(|| {
            repository_observation_conflict(
                &input.repository_observation_id,
                "existing observation has no owning PreToolUse event",
            )
        })?;
        let stored_event = guard_event_by_conn(&tx, &project.project.project_id, pre_event_id)?;
        if !pre_tool_replay_matches(
            &tx,
            &project.project.project_id,
            &fields,
            &input,
            &existing,
            &stored_event,
        )? {
            return Err(repository_observation_conflict(
                &input.repository_observation_id,
                "PreToolUse replay conflicts with the persisted aggregate",
            ));
        }
        tx.commit()?;
        return Ok(PreToolRepositoryObservationRecord {
            guard_event: stored_event,
            observation: existing,
            replayed: true,
        });
    }
    establish_host_correlation_in_transaction(
        &tx,
        &project.project.project_id,
        &fields.session_id,
        &fields.project_integration_revision,
        &input.guard_event.connection_internal_id,
        input
            .guard_event
            .correlation
            .as_ref()
            .expect("validated exact tool correlation"),
        &input.guard_event.occurred_at,
    )?;
    #[cfg(test)]
    inject_repository_observation_fault(RepositoryObservationFaultPoint::PreAfterCorrelation)?;
    insert_guard_event_in_transaction(
        &tx,
        &project.project.project_id,
        &fields,
        &input.guard_event,
    )?;
    #[cfg(test)]
    inject_repository_observation_fault(RepositoryObservationFaultPoint::PreAfterGuardEvent)?;
    let state = if input.unavailable_reason.is_some() {
        RepositoryObservationState::Unavailable
    } else {
        RepositoryObservationState::Open
    };
    let unavailable_terminal_result = input
        .unavailable_reason
        .map(|reason| RepositoryObservationResult {
            observation_state: RepositoryObservationState::Unavailable,
            repository_observation_id: input.repository_observation_id.clone(),
            delta: None,
            unavailable_reason: Some(reason.as_str().to_owned()),
            expected_write_matches: Vec::new(),
            unrecorded_changes: Vec::new(),
            transition_semantics: "net_product_repository_transition_during_invocation".to_owned(),
        })
        .map(|result| canonical_json_string(&result))
        .transpose()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("unavailable observation result cannot be serialized: {error}"),
        })?;
    tx.execute(
        "INSERT INTO repository_observations (
            project_id, repository_observation_id, session_id,
            connection_internal_id, host_turn_id, host_tool_use_id,
            host_tool_name, guard_installation_id, observer_contract_digest,
            pre_tool_guard_event_id, state, pre_snapshot_json,
            pre_snapshot_digest, unavailable_reason, started_at, completed_at,
            terminal_result_json, metadata_json
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18
        )",
        params![
            project.project.project_id,
            input.repository_observation_id,
            fields.session_id,
            input.guard_event.connection_internal_id,
            fields.host_turn_id,
            fields.host_tool_use_id,
            fields.host_tool_name,
            input.guard_event.guard_installation_id,
            observer_contract_digest,
            input.guard_event.guard_event_id,
            state.as_str(),
            pre_snapshot_json,
            pre_snapshot_digest,
            input.unavailable_reason.map(|reason| reason.as_str()),
            input.guard_event.occurred_at,
            input
                .unavailable_reason
                .map(|_| input.guard_event.occurred_at.as_str()),
            unavailable_terminal_result,
            metadata_json,
        ],
    )?;
    #[cfg(test)]
    inject_repository_observation_fault(RepositoryObservationFaultPoint::PreAfterObservation)?;
    if let Some(expected_write) = input.expected_write {
        insert_expected_write_in_transaction(
            &tx,
            &project.project.project_id,
            &input.repository_observation_id,
            expected_write,
        )?;
    }
    #[cfg(test)]
    inject_repository_observation_fault(RepositoryObservationFaultPoint::PreAfterExpectedWrite)?;
    tx.commit()?;
    let guard_event = guard_event_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.guard_event.guard_event_id,
    )?;
    let observation = repository_observation_from_conn(
        &project.conn,
        &project.project.project_id,
        &input.repository_observation_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "repository_observation",
        id: input.repository_observation_id,
    })?;
    Ok(PreToolRepositoryObservationRecord {
        guard_event,
        observation,
        replayed: false,
    })
}

/// Reads the unique observation for one exact native tool invocation.
pub fn repository_observation_for_invocation(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
    session_id: &str,
    correlation: &HostNativeCorrelation,
) -> StoreResult<Option<RepositoryObservationRecord>> {
    let observation_id =
        repository_observation_id(project_id, connection_internal_id, session_id, correlation)?;
    repository_observation(runtime_home, project_id, &observation_id)
}

/// Reads and strictly decodes one repository-observation aggregate.
pub fn repository_observation(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    repository_observation_id: &str,
) -> StoreResult<Option<RepositoryObservationRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("repository_observation_id", repository_observation_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    repository_observation_from_conn(
        &project.conn,
        &project.project.project_id,
        repository_observation_id,
    )
}

/// Atomically terminalizes the exact bounded set of open repository observations.
pub fn terminalize_open_repository_observations(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: TerminalizeOpenRepositoryObservations,
) -> StoreResult<RepositoryObservationTerminalizationResult> {
    validate_terminalization_input(project_id, &input)?;
    let mut project = open_guard_project(context, project_id, &input.connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let terminalized = terminalize_open_repository_observations_in_transaction(
        &tx,
        &project.project.project_id,
        &input,
    )?;
    tx.commit()?;
    Ok(RepositoryObservationTerminalizationResult { terminalized })
}

/// Classifies lifecycle-relevant observations without mutating project state.
pub fn repository_observation_diagnostics_for_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
    session_id: &str,
    runtime_terminal: bool,
) -> StoreResult<Vec<RepositoryObservationDiagnosticRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_identifier("session_id", session_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Err(StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        });
    };
    let current_prompt_turn_id = project
        .conn
        .query_row(
            "SELECT host_turn_id
               FROM prompt_captures
              WHERE project_id = ?1
                AND connection_internal_id = ?2
                AND session_id = ?3
              ORDER BY volicord_utc_seconds(captured_at) DESC,
                       volicord_utc_subsec_nanos(captured_at) DESC,
                       prompt_capture_id DESC
              LIMIT 1",
            params![
                project.project.project_id,
                connection_internal_id,
                session_id
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let current_turn_id = if current_prompt_turn_id.is_some() {
        current_prompt_turn_id
    } else {
        project
            .conn
            .query_row(
                "SELECT last_host_turn_id
               FROM managed_mcp_sessions
              WHERE project_id = ?1
                AND connection_internal_id = ?2
                AND session_id = ?3",
                params![
                    project.project.project_id,
                    connection_internal_id,
                    session_id
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?
    };
    let limit = i64::try_from(REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT + 1).map_err(|_| {
        StoreError::InvalidInput {
            detail: "repository-observation diagnostic limit is not representable".to_owned(),
        }
    })?;
    let observation_ids = {
        let mut statement = project.conn.prepare(
            "SELECT repository_observation_id
               FROM repository_observations
              WHERE project_id = ?1
                AND connection_internal_id = ?2
                AND session_id = ?3
              ORDER BY repository_observation_id
              LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![
                project.project.project_id,
                connection_internal_id,
                session_id,
                limit
            ],
            |row| row.get::<_, String>(0),
        )?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    if observation_ids.len() > REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "repository-observation diagnostics exceed the bounded row limit of {}",
                REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT
            ),
        });
    }
    let mut diagnostics = Vec::new();
    for observation_id in observation_ids {
        let observation = match repository_observation_from_conn(
            &project.conn,
            &project.project.project_id,
            &observation_id,
        ) {
            Ok(Some(observation)) => observation,
            Ok(None) | Err(_) => {
                diagnostics.push(RepositoryObservationDiagnosticRecord {
                    project_id: project_id.to_owned(),
                    session_id: session_id.to_owned(),
                    repository_observation_id: Some(observation_id),
                    status: RepositoryObservationDiagnosticStatus::CorruptObservation,
                });
                continue;
            }
        };
        let status = match observation.state {
            RepositoryObservationState::Open if runtime_terminal => {
                Some(RepositoryObservationDiagnosticStatus::OrphanOpenTerminalSession)
            }
            RepositoryObservationState::Open
                if current_turn_id.as_deref()
                    == Some(observation.correlation.turn_id().as_str()) =>
            {
                Some(RepositoryObservationDiagnosticStatus::OpenCurrentTurn)
            }
            RepositoryObservationState::Open => {
                Some(RepositoryObservationDiagnosticStatus::CleanupFailed)
            }
            RepositoryObservationState::Unavailable
                if observation.unavailable_reason
                    == Some(RepositoryObservationUnavailableReason::PostToolNotObserved) =>
            {
                Some(RepositoryObservationDiagnosticStatus::UnavailablePostToolNotObserved)
            }
            RepositoryObservationState::Unavailable
                if observation.unavailable_reason
                    == Some(RepositoryObservationUnavailableReason::ManagedSessionTerminated) =>
            {
                Some(RepositoryObservationDiagnosticStatus::UnavailableManagedSessionTerminated)
            }
            RepositoryObservationState::Complete | RepositoryObservationState::Unavailable => None,
        };
        if let Some(status) = status {
            diagnostics.push(RepositoryObservationDiagnosticRecord {
                project_id: project_id.to_owned(),
                session_id: session_id.to_owned(),
                repository_observation_id: Some(observation_id),
                status,
            });
        }
    }
    Ok(diagnostics)
}

pub(super) fn terminalize_open_repository_observations_in_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    input: &TerminalizeOpenRepositoryObservations,
) -> StoreResult<Vec<RepositoryObservationRecord>> {
    validate_terminalization_input(project_id, input)?;
    let session_owner = tx
        .query_row(
            "SELECT connection_internal_id
               FROM host_sessions
              WHERE project_id = ?1 AND session_id = ?2",
            params![project_id, input.session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if session_owner.as_deref() != Some(input.connection_internal_id.as_str()) {
        return Err(StoreError::Conflict {
            entity: "host_session",
            id: input.session_id.clone(),
            detail:
                "repository-observation terminalization requires the exact project session owner"
                    .to_owned(),
        });
    }
    if let OpenRepositoryObservationTerminalizationScope::EarlierTurns { current_turn_id } =
        &input.scope
    {
        let established = tx
            .query_row(
                "SELECT 1
                   FROM host_turns
                  WHERE project_id = ?1
                    AND session_id = ?2
                    AND connection_internal_id = ?3
                    AND host_turn_id = ?4",
                params![
                    project_id,
                    input.session_id,
                    input.connection_internal_id,
                    current_turn_id.as_str()
                ],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !established {
            return Err(StoreError::Conflict {
                entity: "host_turn",
                id: current_turn_id.as_str().to_owned(),
                detail: "accepted prompt turn is not established for the exact project session"
                    .to_owned(),
            });
        }
    }

    let limit = i64::try_from(REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT + 1).map_err(|_| {
        StoreError::InvalidInput {
            detail: "repository-observation terminalization limit is not representable".to_owned(),
        }
    })?;
    let observation_ids = match &input.scope {
        OpenRepositoryObservationTerminalizationScope::EarlierTurns { current_turn_id } => {
            let mut statement = tx.prepare(
                "SELECT repository_observation_id
                   FROM repository_observations
                  WHERE project_id = ?1
                    AND connection_internal_id = ?2
                    AND session_id = ?3
                    AND state = 'open'
                    AND host_turn_id <> ?4
                  ORDER BY repository_observation_id
                  LIMIT ?5",
            )?;
            let rows = statement.query_map(
                params![
                    project_id,
                    input.connection_internal_id,
                    input.session_id,
                    current_turn_id.as_str(),
                    limit
                ],
                |row| row.get::<_, String>(0),
            )?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        OpenRepositoryObservationTerminalizationScope::ManagedSession => {
            let mut statement = tx.prepare(
                "SELECT repository_observation_id
                   FROM repository_observations
                  WHERE project_id = ?1
                    AND connection_internal_id = ?2
                    AND session_id = ?3
                    AND state = 'open'
                  ORDER BY repository_observation_id
                  LIMIT ?4",
            )?;
            let rows = statement.query_map(
                params![
                    project_id,
                    input.connection_internal_id,
                    input.session_id,
                    limit
                ],
                |row| row.get::<_, String>(0),
            )?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
    };
    if observation_ids.len() > REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "repository-observation terminalization exceeds the bounded row limit of {}",
                REPOSITORY_OBSERVATION_TERMINALIZATION_LIMIT
            ),
        });
    }

    let mut selected = Vec::with_capacity(observation_ids.len());
    for observation_id in &observation_ids {
        let observation = repository_observation_from_conn(tx, project_id, observation_id)?
            .ok_or_else(|| StoreError::NotFound {
                entity: "repository_observation",
                id: observation_id.clone(),
            })?;
        if observation.state != RepositoryObservationState::Open
            || observation.connection_internal_id != input.connection_internal_id
            || observation.session_id != input.session_id
            || matches!(
                &input.scope,
                OpenRepositoryObservationTerminalizationScope::EarlierTurns { current_turn_id }
                    if observation.correlation.turn_id() == current_turn_id
            )
            || input.completed_at < observation.started_at
        {
            return Err(StoreError::corrupt_owner_state_value(
                "repository_observations",
                observation_id.clone(),
                "state",
            ));
        }
        selected.push(observation);
    }

    for (index, observation) in selected.iter().enumerate() {
        let result = RepositoryObservationResult {
            observation_state: RepositoryObservationState::Unavailable,
            repository_observation_id: observation.repository_observation_id.clone(),
            delta: None,
            unavailable_reason: Some(input.reason.as_str().to_owned()),
            expected_write_matches: Vec::new(),
            unrecorded_changes: Vec::new(),
            transition_semantics: "net_product_repository_transition_during_invocation".to_owned(),
        };
        let changed = tx.execute(
            "UPDATE repository_observations
                SET state = 'unavailable',
                    unavailable_reason = ?3,
                    completed_at = ?4,
                    terminal_result_json = ?5
              WHERE project_id = ?1
                AND repository_observation_id = ?2
                AND state = 'open'",
            params![
                project_id,
                observation.repository_observation_id,
                input.reason.as_str(),
                input.completed_at.to_canonical_string(),
                canonical_json_string(&result).map_err(|error| StoreError::InvalidInput {
                    detail: format!(
                        "terminal repository-observation result cannot be serialized: {error}"
                    ),
                })?
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::Conflict {
                entity: "repository_observation",
                id: observation.repository_observation_id.clone(),
                detail: "repository observation did not terminalize exactly once".to_owned(),
            });
        }
        #[cfg(test)]
        if index == 0 {
            inject_repository_observation_fault(
                RepositoryObservationFaultPoint::TerminalizationAfterFirstUpdate,
            )?;
        }
        #[cfg(not(test))]
        let _ = index;
    }

    let mut terminalized = Vec::with_capacity(observation_ids.len());
    for observation_id in observation_ids {
        terminalized.push(
            repository_observation_from_conn(tx, project_id, &observation_id)?.ok_or_else(
                || StoreError::NotFound {
                    entity: "repository_observation",
                    id: observation_id,
                },
            )?,
        );
    }
    Ok(terminalized)
}

fn validate_terminalization_input(
    project_id: &str,
    input: &TerminalizeOpenRepositoryObservations,
) -> StoreResult<()> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("session_id", &input.session_id)?;
    input
        .completed_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "repository-observation completion time must be canonical RFC 3339".to_owned(),
        })?;
    let valid_boundary = matches!(
        (&input.scope, input.reason),
        (
            OpenRepositoryObservationTerminalizationScope::EarlierTurns { .. },
            RepositoryObservationUnavailableReason::PostToolNotObserved
        ) | (
            OpenRepositoryObservationTerminalizationScope::ManagedSession,
            RepositoryObservationUnavailableReason::ManagedSessionTerminated
        )
    );
    if !valid_boundary {
        return Err(StoreError::InvalidInput {
            detail: "repository-observation terminal reason does not match its authority boundary"
                .to_owned(),
        });
    }
    Ok(())
}

/// Atomically records PostToolUse and closes its exact repository observation.
pub fn record_post_tool_repository_observation(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    mut input: PostToolRepositoryObservationInsert,
) -> StoreResult<PostToolRepositoryObservationRecord> {
    validate_post_input(project_id, &input)?;
    let runtime_home = context.runtime_home().as_path();
    let fields = validate_guard_ownership(runtime_home, project_id, &input.guard_event)?;
    let expected_id = repository_observation_id(
        project_id,
        &input.guard_event.connection_internal_id,
        &fields.session_id,
        input
            .guard_event
            .correlation
            .as_ref()
            .expect("validated exact tool correlation"),
    )?;
    if expected_id != input.repository_observation_id {
        return Err(StoreError::InvalidInput {
            detail: "repository observation identity does not match exact host coordinates"
                .to_owned(),
        });
    }
    let mut project = open_guard_project(
        context,
        project_id,
        &input.guard_event.connection_internal_id,
    )?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let existing = repository_observation_from_conn(
        &tx,
        &project.project.project_id,
        &input.repository_observation_id,
    )?;
    let mut already_unavailable = false;
    let mut missing_observation = false;
    let existing = match existing {
        Some(existing) => {
            if let Some(post_event_id) = existing.post_tool_guard_event_id.as_deref() {
                let stored_event =
                    guard_event_by_conn(&tx, &project.project.project_id, post_event_id)?;
                if post_tool_replay_matches(&fields, &input, &existing, &stored_event)? {
                    let result = existing.terminal_result.clone().ok_or_else(|| {
                        StoreError::corrupt_owner_state_value(
                            "repository_observations",
                            existing.repository_observation_id.clone(),
                            "terminal_result_json",
                        )
                    })?;
                    tx.commit()?;
                    return Ok(PostToolRepositoryObservationRecord {
                        guard_event: stored_event,
                        observation: existing,
                        result,
                        replayed: true,
                    });
                }
                return Err(repository_observation_conflict(
                    &input.repository_observation_id,
                    "PostToolUse replay conflicts with the persisted terminal aggregate",
                ));
            }
            match existing.state {
                RepositoryObservationState::Open => {}
                RepositoryObservationState::Unavailable
                    if matches!(
                        input.outcome,
                        PostToolRepositoryObservationOutcome::Unavailable { reason }
                            if Some(reason) == existing.unavailable_reason
                    ) =>
                {
                    already_unavailable = true;
                }
                RepositoryObservationState::Complete | RepositoryObservationState::Unavailable => {
                    return Err(StoreError::Conflict {
                        entity: "repository_observation",
                        id: input.repository_observation_id,
                        detail: "repository observation is already terminal".to_owned(),
                    });
                }
            }
            existing
        }
        None => {
            let PostToolRepositoryObservationOutcome::Unavailable {
                reason: RepositoryObservationUnavailableReason::MissingOpenObservation,
            } = input.outcome
            else {
                return Err(StoreError::NotFound {
                    entity: "repository_observation",
                    id: input.repository_observation_id,
                });
            };
            missing_observation = true;
            RepositoryObservationRecord {
                project_id: project.project.project_id.clone(),
                repository_observation_id: input.repository_observation_id.clone(),
                session_id: fields.session_id.clone(),
                correlation: input.guard_event.correlation.clone().unwrap(),
                connection_internal_id: input.guard_event.connection_internal_id.clone(),
                guard_installation_id: input.guard_event.guard_installation_id.clone(),
                observer_contract_digest: input.observer_contract_digest.clone(),
                pre_tool_guard_event_id: None,
                post_tool_guard_event_id: None,
                state: RepositoryObservationState::Open,
                pre_snapshot: None,
                post_snapshot: None,
                delta: None,
                unavailable_reason: None,
                started_at: UtcTimestamp::parse(&input.guard_event.occurred_at).map_err(|_| {
                    StoreError::InvalidInput {
                        detail: "PostToolUse occurrence time is invalid".to_owned(),
                    }
                })?,
                completed_at: None,
                terminal_result: None,
                metadata: input.metadata.clone(),
            }
        }
    };
    if existing.connection_internal_id != input.guard_event.connection_internal_id
        || existing.session_id != fields.session_id
        || existing.guard_installation_id != input.guard_event.guard_installation_id
        || existing.observer_contract_digest != input.observer_contract_digest
        || existing.correlation != *input.guard_event.correlation.as_ref().unwrap()
    {
        return Err(StoreError::Conflict {
            entity: "repository_observation",
            id: input.repository_observation_id,
            detail: "PostToolUse coordinates do not match the open observation".to_owned(),
        });
    }
    if missing_observation {
        establish_host_correlation_in_transaction(
            &tx,
            &project.project.project_id,
            &fields.session_id,
            &fields.project_integration_revision,
            &input.guard_event.connection_internal_id,
            input
                .guard_event
                .correlation
                .as_ref()
                .expect("validated exact tool correlation"),
            &input.guard_event.occurred_at,
        )?;
    }

    let (result, post_snapshot_json, post_snapshot_digest, delta_json, delta_digest, reason) =
        terminal_result_in_transaction(
            &tx,
            &project.project.project_id,
            &input.repository_observation_id,
            &input.outcome,
            input.task_id.as_deref(),
            &input.guard_event.occurred_at,
        )?;
    let observation_result_value =
        serde_json::to_value(&result).map_err(|error| StoreError::InvalidInput {
            detail: format!("repository-observation result cannot be serialized: {error}"),
        })?;
    let base_result_value = input
        .guard_event
        .result_json
        .parse::<Value>()
        .map_err(|_| StoreError::InvalidInput {
            detail: "PostToolUse Guard result must be a JSON object".to_owned(),
        })?;
    let mut result_object =
        base_result_value
            .as_object()
            .cloned()
            .ok_or_else(|| StoreError::InvalidInput {
                detail: "PostToolUse Guard result must be a JSON object".to_owned(),
            })?;
    result_object.insert(
        "repository_observation".to_owned(),
        observation_result_value,
    );
    input.guard_event.result_json =
        canonical_json_string(&result_object).map_err(|error| StoreError::InvalidInput {
            detail: format!("PostToolUse Guard result cannot be serialized: {error}"),
        })?;

    insert_guard_event_in_transaction(
        &tx,
        &project.project.project_id,
        &fields,
        &input.guard_event,
    )?;
    #[cfg(test)]
    inject_repository_observation_fault(RepositoryObservationFaultPoint::PostAfterGuardEvent)?;
    let state = if reason.is_some() {
        RepositoryObservationState::Unavailable
    } else {
        RepositoryObservationState::Complete
    };
    let terminal_metadata =
        canonical_json_string(&input.metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("repository-observation metadata cannot be serialized: {error}"),
        })?;
    let terminal_result_json =
        canonical_json_string(&result).map_err(|error| StoreError::InvalidInput {
            detail: format!("terminal result cannot be serialized: {error}"),
        })?;
    let changed = if missing_observation {
        tx.execute(
            "INSERT INTO repository_observations (
                project_id, repository_observation_id, session_id,
                connection_internal_id, host_turn_id, host_tool_use_id,
                host_tool_name, guard_installation_id, observer_contract_digest,
                post_tool_guard_event_id, state, unavailable_reason, started_at,
                completed_at, terminal_result_json, metadata_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'unavailable',
                ?11, ?12, ?12, ?13, ?14
            )",
            params![
                project.project.project_id,
                input.repository_observation_id,
                fields.session_id,
                input.guard_event.connection_internal_id,
                fields.host_turn_id,
                fields.host_tool_use_id,
                fields.host_tool_name,
                input.guard_event.guard_installation_id,
                input.observer_contract_digest,
                input.guard_event.guard_event_id,
                reason.map(RepositoryObservationUnavailableReason::as_str),
                input.guard_event.occurred_at,
                terminal_result_json,
                terminal_metadata,
            ],
        )?
    } else if already_unavailable {
        tx.execute(
            "UPDATE repository_observations
                SET post_tool_guard_event_id = ?3,
                    completed_at = ?4,
                    terminal_result_json = ?5,
                    metadata_json = ?6
              WHERE project_id = ?1
                AND repository_observation_id = ?2
                AND state = 'unavailable'
                AND post_tool_guard_event_id IS NULL",
            params![
                project.project.project_id,
                input.repository_observation_id,
                input.guard_event.guard_event_id,
                input.guard_event.occurred_at,
                terminal_result_json,
                terminal_metadata,
            ],
        )?
    } else {
        tx.execute(
            "UPDATE repository_observations
            SET post_tool_guard_event_id = ?3,
                state = ?4,
                post_snapshot_json = ?5,
                post_snapshot_digest = ?6,
                delta_json = ?7,
                delta_digest = ?8,
                unavailable_reason = ?9,
                completed_at = ?10,
                terminal_result_json = ?11,
                metadata_json = ?12
          WHERE project_id = ?1
            AND repository_observation_id = ?2
            AND state = 'open'",
            params![
                project.project.project_id,
                input.repository_observation_id,
                input.guard_event.guard_event_id,
                state.as_str(),
                post_snapshot_json,
                post_snapshot_digest,
                delta_json,
                delta_digest,
                reason.map(RepositoryObservationUnavailableReason::as_str),
                input.guard_event.occurred_at,
                terminal_result_json,
                terminal_metadata,
            ],
        )?
    };
    if changed != 1 {
        return Err(StoreError::Conflict {
            entity: "repository_observation",
            id: input.repository_observation_id,
            detail: "repository observation did not close exactly once".to_owned(),
        });
    }
    #[cfg(test)]
    inject_repository_observation_fault(RepositoryObservationFaultPoint::PostAfterObservation)?;
    tx.commit()?;
    let observation = repository_observation_from_conn(
        &project.conn,
        &project.project.project_id,
        &input.repository_observation_id,
    )?
    .expect("committed repository observation");
    let guard_event = guard_event_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.guard_event.guard_event_id,
    )?;
    Ok(PostToolRepositoryObservationRecord {
        guard_event,
        observation,
        result,
        replayed: false,
    })
}

fn validate_guard_ownership(
    runtime_home: &Path,
    project_id: &str,
    input: &GuardEventInsert,
) -> StoreResult<GuardCorrelationFields> {
    validate_guard_event_insert(input)?;
    let installation =
        guard_installation(runtime_home, &input.guard_installation_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "guard_installation",
                id: input.guard_installation_id.clone(),
            }
        })?;
    let manifest = current_guard_manifest(&installation)?;
    if installation.connection_internal_id != input.connection_internal_id
        || installation.project_id != project_id
        || manifest.policy_hash.as_str() != input.policy_hash
        || manifest.integration_revision.as_str() != input.integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "guard_event",
            id: input.guard_event_id.clone(),
            detail: "Guard event ownership does not match the current installation manifest"
                .to_owned(),
        });
    }
    let correlation = input
        .correlation
        .as_ref()
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "repository observation Guard events require exact correlation".to_owned(),
        })?;
    let HostNativeCorrelation::CodexHookTool(_) = correlation else {
        return Err(StoreError::InvalidInput {
            detail: "repository observations require Codex hook tool correlation".to_owned(),
        });
    };
    guard_correlation_fields(
        runtime_home,
        project_id,
        &input.connection_internal_id,
        Some(&input.guard_installation_id),
        correlation,
    )
}

fn repository_observation_conflict(id: &str, detail: &str) -> StoreError {
    StoreError::Conflict {
        entity: "repository_observation",
        id: id.to_owned(),
        detail: detail.to_owned(),
    }
}

fn guard_event_matches_insert(
    stored: &GuardEventRecord,
    project_id: &str,
    fields: &GuardCorrelationFields,
    input: &GuardEventInsert,
    expected_result_json: &str,
) -> bool {
    stored.project_id == project_id
        && stored.guard_event_id == input.guard_event_id
        && stored.session_id.as_deref() == Some(fields.session_id.as_str())
        && stored.correlation.as_ref() == input.correlation.as_ref()
        && stored.connection_internal_id == input.connection_internal_id
        && stored.guard_installation_id == input.guard_installation_id
        && stored.policy_hash == input.policy_hash
        && stored.integration_revision == input.integration_revision
        && stored.event_kind == input.event_kind
        && stored.contract_status == input.contract_status
        && stored.decision == input.decision
        && stored.subject_json == input.subject_json
        && stored.result_json == expected_result_json
        && stored.occurred_at == input.occurred_at
        && stored.metadata_json == input.metadata_json
}

fn pre_tool_replay_matches(
    tx: &Transaction<'_>,
    project_id: &str,
    fields: &GuardCorrelationFields,
    input: &PreToolRepositoryObservationInsert,
    stored: &RepositoryObservationRecord,
    stored_event: &GuardEventRecord,
) -> StoreResult<bool> {
    let immutable_observation_matches = stored.project_id == project_id
        && stored.repository_observation_id == input.repository_observation_id
        && stored.session_id == fields.session_id
        && stored.connection_internal_id == input.guard_event.connection_internal_id
        && stored.guard_installation_id == input.guard_event.guard_installation_id
        && stored.observer_contract_digest == input.observer_contract_digest
        && stored.pre_tool_guard_event_id.as_deref()
            == Some(input.guard_event.guard_event_id.as_str())
        && stored.correlation == *input.guard_event.correlation.as_ref().expect("validated")
        && stored.pre_snapshot.as_ref() == input.checkpoint.as_ref()
        && input
            .unavailable_reason
            .is_none_or(|reason| stored.unavailable_reason == Some(reason));
    if !immutable_observation_matches
        || !guard_event_matches_insert(
            stored_event,
            project_id,
            fields,
            &input.guard_event,
            &input.guard_event.result_json,
        )
    {
        return Ok(false);
    }
    expected_write_matches_input(
        tx,
        project_id,
        &input.repository_observation_id,
        input.expected_write.as_ref(),
    )
}

fn expected_write_matches_input(
    tx: &Transaction<'_>,
    project_id: &str,
    repository_observation_id: &str,
    input: Option<&RepositoryExpectedWriteInsert>,
) -> StoreResult<bool> {
    let stored = tx
        .query_row(
            "SELECT
                expected_write_id, command_kind, expected_paths_json, task_id,
                change_unit_id, write_ticket_ids_json, basis_state_version,
                created_at, metadata_json
               FROM expected_writes
              WHERE project_id = ?1 AND repository_observation_id = ?2",
            params![project_id, repository_observation_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()?;
    let Some(input) = input else {
        return Ok(stored.is_none());
    };
    let Some(stored) = stored else {
        return Ok(false);
    };
    let expected_paths_json =
        canonical_json_string(&input.expected_paths).map_err(|error| StoreError::InvalidInput {
            detail: format!("expected-write paths cannot be serialized: {error}"),
        })?;
    let write_ticket_ids_json =
        canonical_json_string(&input.write_ticket_ids).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("write-ticket IDs cannot be serialized: {error}"),
            }
        })?;
    let metadata_json =
        canonical_json_string(&input.metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("expected-write metadata cannot be serialized: {error}"),
        })?;
    Ok(stored
        == (
            input.expected_write_id.clone(),
            input.command_kind.clone(),
            expected_paths_json,
            input.task_id.clone(),
            input.change_unit_id.clone(),
            write_ticket_ids_json,
            input.basis_state_version,
            input.created_at.to_canonical_string(),
            metadata_json,
        ))
}

fn post_tool_replay_matches(
    fields: &GuardCorrelationFields,
    input: &PostToolRepositoryObservationInsert,
    stored: &RepositoryObservationRecord,
    stored_event: &GuardEventRecord,
) -> StoreResult<bool> {
    let outcome_matches = match (&input.outcome, stored.state) {
        (
            PostToolRepositoryObservationOutcome::Complete {
                post_snapshot,
                delta,
            },
            RepositoryObservationState::Complete,
        ) => {
            stored.post_snapshot.as_ref() == Some(post_snapshot.as_ref())
                && stored.delta.as_ref() == Some(delta)
                && stored.unavailable_reason.is_none()
        }
        (
            PostToolRepositoryObservationOutcome::Unavailable { reason },
            RepositoryObservationState::Unavailable,
        ) => stored.unavailable_reason == Some(*reason),
        _ => false,
    };
    let immutable_observation_matches = stored.session_id == fields.session_id
        && stored.connection_internal_id == input.guard_event.connection_internal_id
        && stored.guard_installation_id == input.guard_event.guard_installation_id
        && stored.observer_contract_digest == input.observer_contract_digest
        && stored.post_tool_guard_event_id.as_deref()
            == Some(input.guard_event.guard_event_id.as_str())
        && stored.correlation == *input.guard_event.correlation.as_ref().expect("validated")
        && stored.metadata == input.metadata
        && outcome_matches;
    if !immutable_observation_matches {
        return Ok(false);
    }
    let terminal_result = stored.terminal_result.as_ref().ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "repository_observations",
            stored.repository_observation_id.clone(),
            "terminal_result_json",
        )
    })?;
    let mut result = input
        .guard_event
        .result_json
        .parse::<Value>()
        .map_err(|_| StoreError::InvalidInput {
            detail: "PostToolUse Guard result must be a JSON object".to_owned(),
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "PostToolUse Guard result must be a JSON object".to_owned(),
        })?;
    result.insert(
        "repository_observation".to_owned(),
        serde_json::to_value(terminal_result).map_err(|error| StoreError::InvalidInput {
            detail: format!("repository-observation result cannot be serialized: {error}"),
        })?,
    );
    let expected_result_json =
        canonical_json_string(&result).map_err(|error| StoreError::InvalidInput {
            detail: format!("PostToolUse Guard result cannot be serialized: {error}"),
        })?;
    Ok(guard_event_matches_insert(
        stored_event,
        &stored.project_id,
        fields,
        &input.guard_event,
        &expected_result_json,
    ))
}

fn validate_pre_input(
    project_id: &str,
    input: &PreToolRepositoryObservationInsert,
) -> StoreResult<()> {
    validate_identifier("project_id", project_id)?;
    validate_identifier(
        "repository_observation_id",
        &input.repository_observation_id,
    )?;
    if input.guard_event.event_kind != "pre_tool"
        || input.guard_event.contract_status != "compatible"
    {
        return Err(StoreError::InvalidInput {
            detail: "pre-tool repository observation requires a compatible PreToolUse event"
                .to_owned(),
        });
    }
    if !is_current_observer_contract(&input.observer_contract_digest) {
        return Err(StoreError::InvalidInput {
            detail: "observer contract digest must select the current semantic observer contract"
                .to_owned(),
        });
    }
    match (&input.checkpoint, input.unavailable_reason) {
        (Some(checkpoint), _) => {
            if checkpoint.contract_digest().as_str() != input.observer_contract_digest {
                return Err(StoreError::InvalidInput {
                    detail: "pre snapshot observer contract does not match the aggregate"
                        .to_owned(),
                });
            }
        }
        (None, Some(_)) => {}
        (None, None) => {
            return Err(StoreError::InvalidInput {
                detail: "open pre-tool observation requires a stable baseline".to_owned(),
            })
        }
    }
    if input.unavailable_reason.is_some() && input.expected_write.is_some() {
        return Err(StoreError::InvalidInput {
            detail: "terminal unavailable observations cannot carry expected writes".to_owned(),
        });
    }
    if let Some(expected) = &input.expected_write {
        validate_expected_write(expected)?;
    }
    Ok(())
}

fn validate_post_input(
    project_id: &str,
    input: &PostToolRepositoryObservationInsert,
) -> StoreResult<()> {
    validate_identifier("project_id", project_id)?;
    validate_identifier(
        "repository_observation_id",
        &input.repository_observation_id,
    )?;
    if input.guard_event.event_kind != "post_tool"
        || input.guard_event.contract_status != "compatible"
    {
        return Err(StoreError::InvalidInput {
            detail: "post-tool repository observation requires a compatible PostToolUse event"
                .to_owned(),
        });
    }
    if !is_current_observer_contract(&input.observer_contract_digest) {
        return Err(StoreError::InvalidInput {
            detail: "observer contract digest must select the current semantic observer contract"
                .to_owned(),
        });
    }
    if let PostToolRepositoryObservationOutcome::Complete {
        post_snapshot,
        delta: _,
    } = &input.outcome
    {
        if post_snapshot.contract_digest().as_str() != input.observer_contract_digest {
            return Err(StoreError::InvalidInput {
                detail: "post snapshot observer contract does not match the aggregate".to_owned(),
            });
        }
        post_snapshot
            .semantic_digest()
            .map_err(|error| StoreError::InvalidInput {
                detail: format!("post snapshot is invalid: {error}"),
            })?;
    }
    Ok(())
}

fn validate_expected_write(input: &RepositoryExpectedWriteInsert) -> StoreResult<()> {
    validate_identifier("expected_write_id", &input.expected_write_id)?;
    validate_identifier("task_id", &input.task_id)?;
    validate_identifier("change_unit_id", &input.change_unit_id)?;
    if input.command_kind.trim().is_empty() || input.expected_paths.is_empty() {
        return Err(StoreError::InvalidInput {
            detail: "expected writes require a command kind and non-empty exact paths".to_owned(),
        });
    }
    if input
        .expected_paths
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(StoreError::InvalidInput {
            detail: "expected-write paths must be strictly sorted and unique".to_owned(),
        });
    }
    validate_string_items("expected_writes.write_ticket_ids", &input.write_ticket_ids)?;
    if input.write_ticket_ids.is_empty()
        || input
            .write_ticket_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(StoreError::InvalidInput {
            detail: "expected-write ticket IDs must be strictly sorted and unique".to_owned(),
        });
    }
    Ok(())
}

fn is_current_observer_contract(value: &str) -> bool {
    is_canonical_sha256_digest(value)
        && value == SemanticObserverContractDigest::for_limits(&ObserverLimits::default()).as_str()
}

fn insert_guard_event_in_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    fields: &GuardCorrelationFields,
    input: &GuardEventInsert,
) -> StoreResult<()> {
    tx.execute(
        "INSERT INTO guard_events (
            project_id, guard_event_id, session_id, connection_internal_id,
            correlation_kind, host_turn_id, host_tool_use_id, host_tool_name,
            guard_installation_id, policy_hash, integration_revision, event_kind,
            contract_status, decision, subject_json, result_json, occurred_at,
            metadata_json
        ) VALUES (
            ?1, ?2, ?3, ?4, 'codex_hook_tool', ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17
        )",
        params![
            project_id,
            input.guard_event_id,
            fields.session_id,
            input.connection_internal_id,
            fields.host_turn_id,
            fields.host_tool_use_id,
            fields.host_tool_name,
            input.guard_installation_id,
            input.policy_hash,
            input.integration_revision,
            input.event_kind,
            input.contract_status,
            input.decision,
            input.subject_json,
            input.result_json,
            input.occurred_at,
            input.metadata_json,
        ],
    )?;
    Ok(())
}

fn insert_expected_write_in_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    repository_observation_id: &str,
    input: RepositoryExpectedWriteInsert,
) -> StoreResult<()> {
    let expected_paths_json =
        canonical_json_string(&input.expected_paths).map_err(|error| StoreError::InvalidInput {
            detail: format!("expected-write paths cannot be serialized: {error}"),
        })?;
    let write_ticket_ids_json =
        canonical_json_string(&input.write_ticket_ids).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("write-ticket IDs cannot be serialized: {error}"),
            }
        })?;
    let metadata_json =
        canonical_json_string(&input.metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("expected-write metadata cannot be serialized: {error}"),
        })?;
    tx.execute(
        "INSERT INTO expected_writes (
            project_id, expected_write_id, repository_observation_id,
            command_kind, path_policy, expected_paths_json, task_id,
            change_unit_id, write_ticket_ids_json, basis_state_version,
            status, created_at, metadata_json
        ) VALUES (
            ?1, ?2, ?3, ?4, 'exact_paths', ?5, ?6, ?7, ?8, ?9,
            'pending', ?10, ?11
        )",
        params![
            project_id,
            input.expected_write_id,
            repository_observation_id,
            input.command_kind,
            expected_paths_json,
            input.task_id,
            input.change_unit_id,
            write_ticket_ids_json,
            input.basis_state_version,
            input.created_at.to_canonical_string(),
            metadata_json,
        ],
    )?;
    Ok(())
}

type TerminalColumns = (
    RepositoryObservationResult,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<RepositoryObservationUnavailableReason>,
);

fn terminal_result_in_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    repository_observation_id: &str,
    outcome: &PostToolRepositoryObservationOutcome,
    task_id: Option<&str>,
    occurred_at: &str,
) -> StoreResult<TerminalColumns> {
    match outcome {
        PostToolRepositoryObservationOutcome::Unavailable { reason } => Ok((
            RepositoryObservationResult {
                observation_state: RepositoryObservationState::Unavailable,
                repository_observation_id: repository_observation_id.to_owned(),
                delta: None,
                unavailable_reason: Some(reason.as_str().to_owned()),
                expected_write_matches: Vec::new(),
                unrecorded_changes: Vec::new(),
                transition_semantics: "net_product_repository_transition_during_invocation"
                    .to_owned(),
            },
            None,
            None,
            None,
            None,
            Some(*reason),
        )),
        PostToolRepositoryObservationOutcome::Complete {
            post_snapshot,
            delta,
        } => {
            let post_snapshot_json =
                canonical_json_string(post_snapshot).map_err(|error| StoreError::InvalidInput {
                    detail: format!("post snapshot cannot be serialized: {error}"),
                })?;
            let post_snapshot_digest = post_snapshot
                .semantic_digest()
                .map_err(|error| StoreError::InvalidInput {
                    detail: format!("post snapshot is invalid: {error}"),
                })?
                .as_str()
                .to_owned();
            let delta_json =
                canonical_json_string(delta).map_err(|error| StoreError::InvalidInput {
                    detail: format!("repository delta cannot be serialized: {error}"),
                })?;
            let delta_digest = delta.digest().as_str().to_owned();
            let delta_paths = delta
                .transitions()
                .iter()
                .map(|transition| transition.path().clone())
                .collect::<BTreeSet<_>>();
            let expected =
                expected_write_for_observation(tx, project_id, repository_observation_id)?;
            let mut covered = BTreeSet::new();
            let mut expected_results = Vec::new();
            let linked_task = expected
                .as_ref()
                .map(|expected| expected.task_id.clone())
                .or_else(|| task_id.map(str::to_owned));
            if !delta.is_empty() {
                if let Some(expected) = expected {
                    covered = expected
                        .expected_paths
                        .iter()
                        .filter(|path| delta_paths.contains(*path))
                        .cloned()
                        .collect();
                    if !covered.is_empty() {
                        let matched_paths = covered.iter().cloned().collect::<Vec<_>>();
                        let changed = tx.execute(
                            "UPDATE expected_writes
                                SET status = 'matched',
                                    matched_paths_json = ?3,
                                    matched_at = ?4
                              WHERE project_id = ?1
                                AND repository_observation_id = ?2
                                AND status = 'pending'",
                            params![
                                project_id,
                                repository_observation_id,
                                canonical_json_string(&matched_paths).map_err(|error| {
                                    StoreError::InvalidInput {
                                        detail: format!(
                                            "matched expected paths cannot be serialized: {error}"
                                        ),
                                    }
                                })?,
                                occurred_at,
                            ],
                        )?;
                        if changed != 1 {
                            return Err(StoreError::corrupt_owner_state_value(
                                "expected_writes",
                                expected.expected_write_id,
                                "status",
                            ));
                        }
                        expected_results.push(RepositoryExpectedWriteMatchResult {
                            expected_write_id: expected.expected_write_id,
                            matched_paths,
                        });
                    }
                }
            }
            #[cfg(test)]
            inject_repository_observation_fault(
                RepositoryObservationFaultPoint::PostAfterExpectedWriteReconciliation,
            )?;
            let unmatched_paths = delta_paths
                .difference(&covered)
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut unrecorded_changes = Vec::new();
            if !unmatched_paths.is_empty() {
                let unmatched_delta = delta.restricted_to(&unmatched_paths);
                let unmatched_delta_digest = unmatched_delta.digest().as_str().to_owned();
                let unrecorded_change_id = stable_unrecorded_change_id(
                    project_id,
                    repository_observation_id,
                    &unmatched_delta_digest,
                );
                let observed_paths = unmatched_paths.into_iter().collect::<Vec<_>>();
                let detection = json!({
                    "repository_observation_id": repository_observation_id,
                    "unmatched_delta_digest": unmatched_delta_digest,
                    "transition_semantics":
                        "net_product_repository_transition_during_invocation"
                });
                tx.execute(
                    "INSERT INTO unrecorded_changes (
                        project_id, unrecorded_change_id, repository_observation_id,
                        task_id, status, summary, observed_paths_json,
                        unmatched_delta_digest, detection_json, detected_at,
                        metadata_json
                    ) VALUES (
                        ?1, ?2, ?3, ?4, 'unresolved', ?5, ?6, ?7, ?8, ?9, '{}'
                    )",
                    params![
                        project_id,
                        unrecorded_change_id,
                        repository_observation_id,
                        linked_task.as_deref(),
                        format!(
                            "Observed {} unmatched Product Repository path transition(s)",
                            observed_paths.len()
                        ),
                        canonical_json_string(&observed_paths).map_err(|error| {
                            StoreError::InvalidInput {
                                detail: format!(
                                    "unrecorded-change paths cannot be serialized: {error}"
                                ),
                            }
                        })?,
                        unmatched_delta_digest,
                        canonical_json_string(&detection).map_err(|error| {
                            StoreError::InvalidInput {
                                detail: format!(
                                    "unrecorded-change detection cannot be serialized: {error}"
                                ),
                            }
                        })?,
                        occurred_at,
                    ],
                )?;
                unrecorded_changes.push(RepositoryUnrecordedChangeResult {
                    unrecorded_change_id,
                    unmatched_delta_digest,
                    observed_paths,
                });
            }
            #[cfg(test)]
            inject_repository_observation_fault(
                RepositoryObservationFaultPoint::PostAfterUnrecordedChangeInsert,
            )?;
            Ok((
                RepositoryObservationResult {
                    observation_state: RepositoryObservationState::Complete,
                    repository_observation_id: repository_observation_id.to_owned(),
                    delta: Some(RepositoryDeltaSummary {
                        digest: delta_digest.clone(),
                        paths: delta_paths.into_iter().collect(),
                        transition_count: delta.transitions().len(),
                    }),
                    unavailable_reason: None,
                    expected_write_matches: expected_results,
                    unrecorded_changes,
                    transition_semantics: "net_product_repository_transition_during_invocation"
                        .to_owned(),
                },
                Some(post_snapshot_json),
                Some(post_snapshot_digest),
                Some(delta_json),
                Some(delta_digest),
                None,
            ))
        }
    }
}

#[derive(Debug)]
struct ExactExpectedWrite {
    expected_write_id: String,
    expected_paths: Vec<ProductRelativePath>,
    task_id: String,
    status: String,
    matched_paths: Option<Vec<ProductRelativePath>>,
}

fn expected_write_for_observation(
    conn: &Connection,
    project_id: &str,
    repository_observation_id: &str,
) -> StoreResult<Option<ExactExpectedWrite>> {
    let raw = conn
        .query_row(
            "SELECT
                expected_write_id, command_kind, path_policy,
                expected_paths_json, task_id, change_unit_id,
                write_ticket_ids_json, basis_state_version, status,
                matched_paths_json, created_at, matched_at, metadata_json
               FROM expected_writes
              WHERE project_id = ?1
                AND repository_observation_id = ?2",
            params![project_id, repository_observation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, u64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()?;
    raw.map(
        |(
            expected_write_id,
            command_kind,
            path_policy,
            paths_json,
            task_id,
            change_unit_id,
            write_ticket_ids_json,
            _basis_state_version,
            status,
            matched_paths_json,
            created_at,
            matched_at,
            metadata_json,
        )| {
            let corrupt_json = |field| {
                StoreError::corrupt_owner_state_json(
                    "expected_writes",
                    expected_write_id.clone(),
                    field,
                )
            };
            let corrupt_value = |field| {
                StoreError::corrupt_owner_state_value(
                    "expected_writes",
                    expected_write_id.clone(),
                    field,
                )
            };
            if expected_write_id.trim().is_empty()
                || command_kind.trim().is_empty()
                || path_policy != "exact_paths"
                || task_id.trim().is_empty()
                || change_unit_id.trim().is_empty()
            {
                return Err(corrupt_value("path_policy"));
            }
            let expected_paths = serde_json::from_str::<Vec<ProductRelativePath>>(&paths_json)
                .map_err(|_| corrupt_json("expected_paths_json"))?;
            if expected_paths.is_empty()
                || expected_paths.windows(2).any(|pair| pair[0] >= pair[1])
                || canonical_json_string(&expected_paths).ok().as_deref() != Some(&paths_json)
            {
                return Err(corrupt_json("expected_paths_json"));
            }
            let write_ticket_ids = serde_json::from_str::<Vec<String>>(&write_ticket_ids_json)
                .map_err(|_| corrupt_json("write_ticket_ids_json"))?;
            if write_ticket_ids.is_empty()
                || write_ticket_ids.iter().any(|value| value.trim().is_empty())
                || write_ticket_ids.windows(2).any(|pair| pair[0] >= pair[1])
                || canonical_json_string(&write_ticket_ids).ok().as_deref()
                    != Some(&write_ticket_ids_json)
            {
                return Err(corrupt_json("write_ticket_ids_json"));
            }
            let matched_paths = matched_paths_json
                .as_deref()
                .map(|value| {
                    let paths = serde_json::from_str::<Vec<ProductRelativePath>>(value)
                        .map_err(|_| corrupt_json("matched_paths_json"))?;
                    if paths.is_empty()
                        || paths.windows(2).any(|pair| pair[0] >= pair[1])
                        || canonical_json_string(&paths).ok().as_deref() != Some(value)
                        || paths.iter().any(|path| !expected_paths.contains(path))
                    {
                        return Err(corrupt_json("matched_paths_json"));
                    }
                    Ok(paths)
                })
                .transpose()?;
            let created_at = strict_stored_timestamp(
                "expected_writes",
                &expected_write_id,
                "created_at",
                &created_at,
            )?;
            let matched_at = matched_at
                .as_deref()
                .map(|value| {
                    strict_stored_timestamp(
                        "expected_writes",
                        &expected_write_id,
                        "matched_at",
                        value,
                    )
                })
                .transpose()?;
            let valid_status = match status.as_str() {
                "pending" => matched_paths.is_none() && matched_at.is_none(),
                "matched" => {
                    matched_paths.is_some()
                        && matched_at
                            .as_ref()
                            .is_some_and(|matched_at| matched_at >= &created_at)
                }
                _ => false,
            };
            if !valid_status {
                return Err(corrupt_value("status"));
            }
            decode_canonical_object(&metadata_json, "metadata_json", &corrupt_json)?;
            Ok(ExactExpectedWrite {
                expected_write_id,
                expected_paths,
                task_id,
                status,
                matched_paths,
            })
        },
    )
    .transpose()
}

pub(super) fn stable_unrecorded_change_id(
    project_id: &str,
    repository_observation_id: &str,
    unmatched_delta_digest: &str,
) -> String {
    let mut encoder = Vec::new();
    for field in [
        "volicord.unrecorded-change",
        project_id,
        repository_observation_id,
        unmatched_delta_digest,
    ] {
        encoder.extend_from_slice(&(field.len() as u64).to_be_bytes());
        encoder.extend_from_slice(field.as_bytes());
    }
    format!("unrecorded_change_{:x}", Sha256::digest(encoder))
}

#[derive(Debug)]
struct RepositoryObservationRaw {
    project_id: String,
    repository_observation_id: String,
    session_id: String,
    connection_internal_id: String,
    host_session_id: Option<String>,
    host_turn_id: String,
    host_tool_use_id: String,
    host_tool_name: String,
    guard_installation_id: String,
    observer_contract_digest: String,
    pre_tool_guard_event_id: Option<String>,
    post_tool_guard_event_id: Option<String>,
    state: String,
    pre_snapshot_json: Option<String>,
    pre_snapshot_digest: Option<String>,
    post_snapshot_json: Option<String>,
    post_snapshot_digest: Option<String>,
    delta_json: Option<String>,
    delta_digest: Option<String>,
    unavailable_reason: Option<String>,
    started_at: String,
    completed_at: Option<String>,
    terminal_result_json: Option<String>,
    metadata_json: String,
}

pub(super) fn repository_observation_from_conn(
    conn: &Connection,
    project_id: &str,
    repository_observation_id: &str,
) -> StoreResult<Option<RepositoryObservationRecord>> {
    let raw = conn
        .query_row(
            "SELECT
                o.project_id, o.repository_observation_id, o.session_id,
                o.connection_internal_id, h.host_session_id, o.host_turn_id,
                o.host_tool_use_id, o.host_tool_name, o.guard_installation_id,
                o.observer_contract_digest, o.pre_tool_guard_event_id,
                o.post_tool_guard_event_id, o.state, o.pre_snapshot_json,
                o.pre_snapshot_digest, o.post_snapshot_json,
                o.post_snapshot_digest, o.delta_json, o.delta_digest,
                o.unavailable_reason, o.started_at, o.completed_at,
                o.terminal_result_json, o.metadata_json
               FROM repository_observations AS o
               LEFT JOIN host_sessions AS h
                 ON h.project_id = o.project_id
                AND h.session_id = o.session_id
                AND h.connection_internal_id = o.connection_internal_id
              WHERE o.project_id = ?1
                AND o.repository_observation_id = ?2",
            params![project_id, repository_observation_id],
            |row| {
                Ok(RepositoryObservationRaw {
                    project_id: row.get(0)?,
                    repository_observation_id: row.get(1)?,
                    session_id: row.get(2)?,
                    connection_internal_id: row.get(3)?,
                    host_session_id: row.get(4)?,
                    host_turn_id: row.get(5)?,
                    host_tool_use_id: row.get(6)?,
                    host_tool_name: row.get(7)?,
                    guard_installation_id: row.get(8)?,
                    observer_contract_digest: row.get(9)?,
                    pre_tool_guard_event_id: row.get(10)?,
                    post_tool_guard_event_id: row.get(11)?,
                    state: row.get(12)?,
                    pre_snapshot_json: row.get(13)?,
                    pre_snapshot_digest: row.get(14)?,
                    post_snapshot_json: row.get(15)?,
                    post_snapshot_digest: row.get(16)?,
                    delta_json: row.get(17)?,
                    delta_digest: row.get(18)?,
                    unavailable_reason: row.get(19)?,
                    started_at: row.get(20)?,
                    completed_at: row.get(21)?,
                    terminal_result_json: row.get(22)?,
                    metadata_json: row.get(23)?,
                })
            },
        )
        .optional()?;
    let record = raw.map(decode_repository_observation).transpose()?;
    if let Some(record) = record.as_ref() {
        validate_repository_observation_relationships(conn, record)?;
    }
    Ok(record)
}

fn decode_repository_observation(
    raw: RepositoryObservationRaw,
) -> StoreResult<RepositoryObservationRecord> {
    let record_ref = raw.repository_observation_id.clone();
    let corrupt_value = |field| {
        StoreError::corrupt_owner_state_value("repository_observations", record_ref.clone(), field)
    };
    let corrupt_json = |field| {
        StoreError::corrupt_owner_state_json("repository_observations", record_ref.clone(), field)
    };
    if !is_current_observer_contract(&raw.observer_contract_digest) {
        return Err(corrupt_value("observer_contract_digest"));
    }
    let state =
        RepositoryObservationState::parse(&raw.state).ok_or_else(|| corrupt_value("state"))?;
    let pre_snapshot = decode_checkpoint(
        raw.pre_snapshot_json.as_deref(),
        raw.pre_snapshot_digest.as_deref(),
        &raw.observer_contract_digest,
        "pre_snapshot_json",
        "pre_snapshot_digest",
        &corrupt_json,
        &corrupt_value,
    )?;
    let post_snapshot = decode_checkpoint(
        raw.post_snapshot_json.as_deref(),
        raw.post_snapshot_digest.as_deref(),
        &raw.observer_contract_digest,
        "post_snapshot_json",
        "post_snapshot_digest",
        &corrupt_json,
        &corrupt_value,
    )?;
    let delta = match (raw.delta_json.as_deref(), raw.delta_digest.as_deref()) {
        (None, None) => None,
        (Some(value), Some(digest)) => {
            let delta = serde_json::from_str::<RepositoryDelta>(value)
                .map_err(|_| corrupt_json("delta_json"))?;
            if canonical_json_string(&delta).ok().as_deref() != Some(value)
                || delta.digest().as_str() != digest
            {
                return Err(corrupt_value("delta_digest"));
            }
            Some(delta)
        }
        _ => return Err(corrupt_value("delta_digest")),
    };
    let unavailable_reason = match raw.unavailable_reason.as_deref() {
        Some(value) => Some(
            RepositoryObservationUnavailableReason::parse(value)
                .ok_or_else(|| corrupt_value("unavailable_reason"))?,
        ),
        None => None,
    };
    let started_at = strict_stored_timestamp(
        "repository_observations",
        &record_ref,
        "started_at",
        &raw.started_at,
    )?;
    let completed_at = raw
        .completed_at
        .as_deref()
        .map(|value| {
            strict_stored_timestamp(
                "repository_observations",
                &record_ref,
                "completed_at",
                value,
            )
        })
        .transpose()?;
    if completed_at
        .as_ref()
        .is_some_and(|value| value < &started_at)
    {
        return Err(corrupt_value("completed_at"));
    }
    let terminal_result = raw
        .terminal_result_json
        .as_deref()
        .map(|value| {
            let result = serde_json::from_str::<RepositoryObservationResult>(value)
                .map_err(|_| corrupt_json("terminal_result_json"))?;
            if canonical_json_string(&result).ok().as_deref() != Some(value) {
                return Err(corrupt_json("terminal_result_json"));
            }
            Ok(result)
        })
        .transpose()?;
    let metadata = decode_canonical_object(&raw.metadata_json, "metadata_json", &corrupt_json)?;
    let valid_shape = match state {
        RepositoryObservationState::Open => {
            raw.pre_tool_guard_event_id.is_some()
                && pre_snapshot.is_some()
                && raw.post_tool_guard_event_id.is_none()
                && post_snapshot.is_none()
                && delta.is_none()
                && unavailable_reason.is_none()
                && completed_at.is_none()
                && terminal_result.is_none()
        }
        RepositoryObservationState::Complete => {
            raw.pre_tool_guard_event_id.is_some()
                && pre_snapshot.is_some()
                && raw.post_tool_guard_event_id.is_some()
                && post_snapshot.is_some()
                && delta.is_some()
                && unavailable_reason.is_none()
                && completed_at.is_some()
                && terminal_result.is_some()
        }
        RepositoryObservationState::Unavailable => {
            post_snapshot.is_none()
                && delta.is_none()
                && (pre_snapshot.is_none() || raw.pre_tool_guard_event_id.is_some())
                && unavailable_reason.is_some()
                && completed_at.is_some()
                && terminal_result.is_some()
        }
    };
    if !valid_shape {
        return Err(corrupt_value("state"));
    }
    if let Some(result) = terminal_result.as_ref() {
        let delta_paths = delta
            .as_ref()
            .map(|delta| {
                delta
                    .transitions()
                    .iter()
                    .map(|transition| transition.path().clone())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let result_delta_valid = match (&result.delta, delta.as_ref()) {
            (None, None) => true,
            (Some(summary), Some(delta)) => {
                is_canonical_sha256_digest(&summary.digest)
                    && summary.digest == delta.digest().as_str()
                    && summary.transition_count == delta.transitions().len()
                    && summary.paths.iter().cloned().collect::<BTreeSet<_>>() == delta_paths
                    && summary.paths.len() == delta_paths.len()
                    && summary.paths.windows(2).all(|pair| pair[0] < pair[1])
            }
            _ => false,
        };
        let result_reason_valid = match (result.unavailable_reason.as_deref(), unavailable_reason) {
            (None, None) => true,
            (Some(result_reason), Some(reason)) => result_reason == reason.as_str(),
            _ => false,
        };
        let result_paths_valid = result.expected_write_matches.iter().all(|matched| {
            !matched.expected_write_id.trim().is_empty()
                && !matched.matched_paths.is_empty()
                && matched
                    .matched_paths
                    .iter()
                    .all(|path| delta_paths.contains(path))
                && matched.matched_paths.iter().collect::<BTreeSet<_>>().len()
                    == matched.matched_paths.len()
                && matched
                    .matched_paths
                    .windows(2)
                    .all(|pair| pair[0] < pair[1])
        }) && result.unrecorded_changes.iter().all(|change| {
            !change.unrecorded_change_id.trim().is_empty()
                && is_canonical_sha256_digest(&change.unmatched_delta_digest)
                && !change.observed_paths.is_empty()
                && change
                    .observed_paths
                    .iter()
                    .all(|path| delta_paths.contains(path))
                && change.observed_paths.iter().collect::<BTreeSet<_>>().len()
                    == change.observed_paths.len()
                && change
                    .observed_paths
                    .windows(2)
                    .all(|pair| pair[0] < pair[1])
        });
        if result.observation_state != state
            || result.repository_observation_id != raw.repository_observation_id
            || result.transition_semantics != "net_product_repository_transition_during_invocation"
            || !result_delta_valid
            || !result_reason_valid
            || !result_paths_valid
            || (state == RepositoryObservationState::Unavailable
                && (!result.expected_write_matches.is_empty()
                    || !result.unrecorded_changes.is_empty()))
        {
            return Err(corrupt_json("terminal_result_json"));
        }
    }
    let correlation =
        HostNativeCorrelation::CodexHookTool(volicord_host_contract::CodexHookToolCorrelation {
            session_id: volicord_host_contract::HostSessionId::parse(
                raw.host_session_id
                    .as_deref()
                    .ok_or_else(|| corrupt_value("host_session_id"))?,
            )
            .map_err(|_| corrupt_value("host_session_id"))?,
            turn_id: volicord_host_contract::HostTurnId::parse(&raw.host_turn_id)
                .map_err(|_| corrupt_value("host_turn_id"))?,
            tool_use_id: volicord_host_contract::HostToolUseId::parse(&raw.host_tool_use_id)
                .map_err(|_| corrupt_value("host_tool_use_id"))?,
            tool_name: volicord_host_contract::CanonicalToolName::parse(&raw.host_tool_name)
                .map_err(|_| corrupt_value("host_tool_name"))?,
        });
    let derived_id = repository_observation_id(
        &raw.project_id,
        &raw.connection_internal_id,
        &raw.session_id,
        &correlation,
    )
    .map_err(|_| corrupt_value("repository_observation_id"))?;
    if derived_id != raw.repository_observation_id {
        return Err(corrupt_value("repository_observation_id"));
    }
    Ok(RepositoryObservationRecord {
        project_id: raw.project_id,
        repository_observation_id: raw.repository_observation_id,
        session_id: raw.session_id,
        correlation,
        connection_internal_id: raw.connection_internal_id,
        guard_installation_id: raw.guard_installation_id,
        observer_contract_digest: raw.observer_contract_digest,
        pre_tool_guard_event_id: raw.pre_tool_guard_event_id,
        post_tool_guard_event_id: raw.post_tool_guard_event_id,
        state,
        pre_snapshot,
        post_snapshot,
        delta,
        unavailable_reason,
        started_at,
        completed_at,
        terminal_result,
        metadata,
    })
}

fn validate_repository_observation_relationships(
    conn: &Connection,
    record: &RepositoryObservationRecord,
) -> StoreResult<()> {
    let corrupt_value = |field| {
        StoreError::corrupt_owner_state_value(
            "repository_observations",
            record.repository_observation_id.clone(),
            field,
        )
    };
    let corrupt_json = |field| {
        StoreError::corrupt_owner_state_json(
            "repository_observations",
            record.repository_observation_id.clone(),
            field,
        )
    };
    let pre_event = record
        .pre_tool_guard_event_id
        .as_deref()
        .map(|event_id| {
            linked_guard_event(
                conn,
                record,
                event_id,
                "pre_tool",
                "pre_tool_guard_event_id",
            )
        })
        .transpose()?;
    let post_event = record
        .post_tool_guard_event_id
        .as_deref()
        .map(|event_id| {
            linked_guard_event(
                conn,
                record,
                event_id,
                "post_tool",
                "post_tool_guard_event_id",
            )
        })
        .transpose()?;
    if let (Some(pre), Some(post)) = (&pre_event, &post_event) {
        if pre.policy_hash != post.policy_hash
            || pre.integration_revision != post.integration_revision
            || post.occurred_at < pre.occurred_at
        {
            return Err(corrupt_value("post_tool_guard_event_id"));
        }
    }
    match record.unavailable_reason {
        Some(RepositoryObservationUnavailableReason::MissingOpenObservation) => {
            if pre_event.is_some() || post_event.is_none() {
                return Err(corrupt_value("pre_tool_guard_event_id"));
            }
        }
        Some(_) if pre_event.is_none() => {
            return Err(corrupt_value("pre_tool_guard_event_id"));
        }
        Some(_) => {}
        None => {}
    }
    let expected_write = expected_write_for_observation(
        conn,
        &record.project_id,
        &record.repository_observation_id,
    )?;
    let linked_unrecorded_changes = linked_unrecorded_changes(conn, record)?;
    match record.state {
        RepositoryObservationState::Open => {
            if expected_write
                .as_ref()
                .is_some_and(|expected| expected.status != "pending")
                || !linked_unrecorded_changes.is_empty()
            {
                return Err(corrupt_value("state"));
            }
        }
        RepositoryObservationState::Unavailable => {
            if expected_write.as_ref().is_some_and(|expected| {
                record.unavailable_reason
                    == Some(RepositoryObservationUnavailableReason::MissingOpenObservation)
                    || expected.status != "pending"
            }) || !linked_unrecorded_changes.is_empty()
            {
                return Err(corrupt_value("state"));
            }
        }
        RepositoryObservationState::Complete => {
            let result = record
                .terminal_result
                .as_ref()
                .ok_or_else(|| corrupt_value("terminal_result_json"))?;
            let expected_matches = expected_write
                .as_ref()
                .filter(|expected| expected.status == "matched")
                .map(|expected| {
                    vec![RepositoryExpectedWriteMatchResult {
                        expected_write_id: expected.expected_write_id.clone(),
                        matched_paths: expected.matched_paths.clone().expect("validated matched"),
                    }]
                })
                .unwrap_or_default();
            if result.expected_write_matches != expected_matches
                || result.unrecorded_changes != linked_unrecorded_changes
            {
                return Err(corrupt_json("terminal_result_json"));
            }
        }
    }
    if let (Some(post), Some(result)) = (post_event, record.terminal_result.as_ref()) {
        let value = serde_json::from_str::<Value>(&post.result_json)
            .map_err(|_| corrupt_value("post_tool_guard_event_id"))?;
        if canonical_json_string(&value).ok().as_deref() != Some(&post.result_json)
            || value.get("repository_observation") != serde_json::to_value(result).ok().as_ref()
        {
            return Err(corrupt_value("post_tool_guard_event_id"));
        }
    }
    Ok(())
}

fn linked_guard_event(
    conn: &Connection,
    record: &RepositoryObservationRecord,
    event_id: &str,
    expected_kind: &str,
    field: &'static str,
) -> StoreResult<GuardEventRecord> {
    let corrupt = || {
        StoreError::corrupt_owner_state_value(
            "repository_observations",
            record.repository_observation_id.clone(),
            field,
        )
    };
    let event = guard_event_from_conn(conn, &record.project_id, event_id)
        .map_err(|_| corrupt())?
        .ok_or_else(corrupt)?;
    if event.project_id != record.project_id
        || event.session_id.as_deref() != Some(record.session_id.as_str())
        || event.connection_internal_id != record.connection_internal_id
        || event.guard_installation_id != record.guard_installation_id
        || event.correlation.as_ref() != Some(&record.correlation)
        || event.event_kind != expected_kind
        || event.contract_status != "compatible"
        || canonical_json_object(&event.subject_json).is_none()
        || canonical_json_object(&event.result_json).is_none()
        || canonical_json_object(&event.metadata_json).is_none()
        || strict_stored_timestamp("guard_events", event_id, "occurred_at", &event.occurred_at)
            .is_err()
    {
        return Err(corrupt());
    }
    Ok(event)
}

fn linked_unrecorded_changes(
    conn: &Connection,
    record: &RepositoryObservationRecord,
) -> StoreResult<Vec<RepositoryUnrecordedChangeResult>> {
    let corrupt_value = |field| {
        StoreError::corrupt_owner_state_value(
            "repository_observations",
            record.repository_observation_id.clone(),
            field,
        )
    };
    let corrupt_json = |field| {
        StoreError::corrupt_owner_state_json(
            "repository_observations",
            record.repository_observation_id.clone(),
            field,
        )
    };
    let Some(delta) = record.delta.as_ref() else {
        let count = conn.query_row(
            "SELECT count(*)
               FROM unrecorded_changes
              WHERE project_id = ?1 AND repository_observation_id = ?2",
            params![record.project_id, record.repository_observation_id],
            |row| row.get::<_, u64>(0),
        )?;
        return if count == 0 {
            Ok(Vec::new())
        } else {
            Err(corrupt_value("state"))
        };
    };
    let delta_paths = delta
        .transitions()
        .iter()
        .map(|transition| transition.path().clone())
        .collect::<BTreeSet<_>>();
    let mut stmt = conn.prepare(
        "SELECT unrecorded_change_id, observed_paths_json, unmatched_delta_digest
           FROM unrecorded_changes
          WHERE project_id = ?1 AND repository_observation_id = ?2
          ORDER BY unrecorded_change_id",
    )?;
    let rows = stmt.query_map(
        params![record.project_id, record.repository_observation_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?;
    let mut results = Vec::new();
    for row in rows {
        let (id, paths_json, digest) = row?;
        let paths = serde_json::from_str::<Vec<ProductRelativePath>>(&paths_json)
            .map_err(|_| corrupt_json("terminal_result_json"))?;
        let selected = paths.iter().cloned().collect::<BTreeSet<_>>();
        if paths.is_empty()
            || selected.len() != paths.len()
            || selected.iter().cloned().collect::<Vec<_>>() != paths
            || !selected.is_subset(&delta_paths)
            || canonical_json_string(&paths).ok().as_deref() != Some(&paths_json)
            || !is_canonical_sha256_digest(&digest)
            || delta.restricted_to(&selected).digest().as_str() != digest
            || stable_unrecorded_change_id(
                &record.project_id,
                &record.repository_observation_id,
                &digest,
            ) != id
        {
            return Err(corrupt_value("terminal_result_json"));
        }
        results.push(RepositoryUnrecordedChangeResult {
            unrecorded_change_id: id,
            unmatched_delta_digest: digest,
            observed_paths: paths,
        });
    }
    Ok(results)
}

fn canonical_json_object(value: &str) -> Option<JsonObject> {
    let object = serde_json::from_str::<JsonObject>(value).ok()?;
    (canonical_json_string(&object).ok().as_deref() == Some(value)).then_some(object)
}

fn decode_checkpoint(
    json: Option<&str>,
    digest: Option<&str>,
    observer_contract_digest: &str,
    json_field: &'static str,
    digest_field: &'static str,
    corrupt_json: &impl Fn(&'static str) -> StoreError,
    corrupt_value: &impl Fn(&'static str) -> StoreError,
) -> StoreResult<Option<RepositoryObservationCheckpoint>> {
    match (json, digest) {
        (None, None) => Ok(None),
        (Some(value), Some(digest)) => {
            let checkpoint = serde_json::from_str::<RepositoryObservationCheckpoint>(value)
                .map_err(|_| corrupt_json(json_field))?;
            if canonical_json_string(&checkpoint).ok().as_deref() != Some(value)
                || checkpoint.contract_digest().as_str() != observer_contract_digest
                || checkpoint
                    .semantic_digest()
                    .map_err(|_| corrupt_value(digest_field))?
                    .as_str()
                    != digest
            {
                return Err(corrupt_value(digest_field));
            }
            Ok(Some(checkpoint))
        }
        _ => Err(corrupt_value(digest_field)),
    }
}

fn decode_canonical_object(
    value: &str,
    field: &'static str,
    corrupt_json: &impl Fn(&'static str) -> StoreError,
) -> StoreResult<JsonObject> {
    let object = serde_json::from_str::<JsonObject>(value).map_err(|_| corrupt_json(field))?;
    if canonical_json_string(&object).ok().as_deref() != Some(value) {
        return Err(corrupt_json(field));
    }
    Ok(object)
}
