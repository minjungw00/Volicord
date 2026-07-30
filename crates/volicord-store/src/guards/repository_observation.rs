use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_host_contract::HostNativeCorrelation;
use volicord_platform_fs::{
    ObservationUnavailableReason, RepositoryDelta, RepositoryObservationCheckpoint,
};
use volicord_types::canonical::{canonical_json_string, is_canonical_sha256_digest};
use volicord_types::product_path::ProductRelativePath;
use volicord_types::schema::JsonObject;
use volicord_types::values::UtcTimestamp;

use super::{
    begin_immediate_transaction, current_guard_manifest, guard_correlation_fields,
    guard_event_by_conn, guard_installation, open_guard_project, open_project_for_read,
    strict_stored_timestamp, validate_guard_event_insert, validate_identifier,
    validate_string_items, GuardCorrelationFields, GuardEventInsert, GuardEventRecord,
};
use crate::{RuntimeHomeMutationContext, StoreError, StoreResult};

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
}

impl RepositoryObservationUnavailableReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observer(reason) => reason.as_str(),
            Self::InvocationDenied => "invocation_denied",
            Self::MissingOpenObservation => "missing_open_observation",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "invocation_denied" => Some(Self::InvocationDenied),
            "missing_open_observation" => Some(Self::MissingOpenObservation),
            value => ObservationUnavailableReason::parse(value).map(Self::Observer),
        }
    }
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
) -> StoreResult<GuardEventRecord> {
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
    insert_guard_event_in_transaction(
        &tx,
        &project.project.project_id,
        &fields,
        &input.guard_event,
    )?;
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
    if let Some(expected_write) = input.expected_write {
        insert_expected_write_in_transaction(
            &tx,
            &project.project.project_id,
            &input.repository_observation_id,
            expected_write,
        )?;
    }
    tx.commit()?;
    guard_event_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.guard_event.guard_event_id,
    )
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
    let existing = match existing {
        Some(existing) => {
            match existing.state {
                RepositoryObservationState::Open => {}
                RepositoryObservationState::Unavailable
                    if existing.post_tool_guard_event_id.is_none()
                        && matches!(
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
            insert_missing_observation_in_transaction(
                &tx,
                &project.project.project_id,
                &fields,
                &input,
            )?;
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
    let state = if reason.is_some() {
        RepositoryObservationState::Unavailable
    } else {
        RepositoryObservationState::Complete
    };
    let terminal_metadata =
        canonical_json_string(&input.metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("repository-observation metadata cannot be serialized: {error}"),
        })?;
    let changed = if already_unavailable {
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
                canonical_json_string(&result).map_err(|error| {
                    StoreError::InvalidInput {
                        detail: format!("terminal result cannot be serialized: {error}"),
                    }
                })?,
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
                canonical_json_string(&result).map_err(|error| {
                    StoreError::InvalidInput {
                        detail: format!("terminal result cannot be serialized: {error}"),
                    }
                })?,
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
    if !is_canonical_sha256_digest(&input.observer_contract_digest) {
        return Err(StoreError::InvalidInput {
            detail: "observer contract digest must be canonical sha256".to_owned(),
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
    if !is_canonical_sha256_digest(&input.observer_contract_digest) {
        return Err(StoreError::InvalidInput {
            detail: "observer contract digest must be canonical sha256".to_owned(),
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
    if input.expected_paths.iter().collect::<BTreeSet<_>>().len() != input.expected_paths.len() {
        return Err(StoreError::InvalidInput {
            detail: "expected-write paths must be unique".to_owned(),
        });
    }
    validate_string_items("expected_writes.write_ticket_ids", &input.write_ticket_ids)?;
    if input.write_ticket_ids.iter().collect::<BTreeSet<_>>().len() != input.write_ticket_ids.len()
    {
        return Err(StoreError::InvalidInput {
            detail: "expected-write ticket IDs must be unique".to_owned(),
        });
    }
    Ok(())
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

fn insert_missing_observation_in_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    fields: &GuardCorrelationFields,
    input: &PostToolRepositoryObservationInsert,
) -> StoreResult<()> {
    let metadata_json =
        canonical_json_string(&input.metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("repository-observation metadata cannot be serialized: {error}"),
        })?;
    tx.execute(
        "INSERT INTO repository_observations (
            project_id, repository_observation_id, session_id,
            connection_internal_id, host_turn_id, host_tool_use_id,
            host_tool_name, guard_installation_id, observer_contract_digest,
            state, unavailable_reason, started_at, completed_at,
            terminal_result_json, metadata_json
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'open', NULL,
            ?10, NULL, NULL, ?11
        )",
        params![
            project_id,
            input.repository_observation_id,
            fields.session_id,
            input.guard_event.connection_internal_id,
            fields.host_turn_id,
            fields.host_tool_use_id,
            fields.host_tool_name,
            input.guard_event.guard_installation_id,
            input.observer_contract_digest,
            input.guard_event.occurred_at,
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
                        tx.execute(
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
                        expected_results.push(RepositoryExpectedWriteMatchResult {
                            expected_write_id: expected.expected_write_id,
                            matched_paths,
                        });
                    }
                }
            }
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
                    "INSERT OR IGNORE INTO unrecorded_changes (
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
                || expected_paths.iter().collect::<BTreeSet<_>>().len() != expected_paths.len()
                || canonical_json_string(&expected_paths).ok().as_deref() != Some(&paths_json)
            {
                return Err(corrupt_json("expected_paths_json"));
            }
            let write_ticket_ids = serde_json::from_str::<Vec<String>>(&write_ticket_ids_json)
                .map_err(|_| corrupt_json("write_ticket_ids_json"))?;
            if write_ticket_ids.is_empty()
                || write_ticket_ids.iter().any(|value| value.trim().is_empty())
                || write_ticket_ids.iter().collect::<BTreeSet<_>>().len() != write_ticket_ids.len()
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
                        || paths.iter().collect::<BTreeSet<_>>().len() != paths.len()
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
            })
        },
    )
    .transpose()
}

fn stable_unrecorded_change_id(
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
    host_session_id: String,
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

fn repository_observation_from_conn(
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
               JOIN host_sessions AS h
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
    raw.map(decode_repository_observation).transpose()
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
    if !is_canonical_sha256_digest(&raw.observer_contract_digest) {
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
            session_id: volicord_host_contract::HostSessionId::parse(&raw.host_session_id)
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
