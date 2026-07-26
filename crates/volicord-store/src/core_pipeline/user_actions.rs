use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::{
    ids::TaskId,
    schema::{
        effective_user_action_status as derive_user_action_status, validate_channel_submission_id,
        ArtifactRef, PersistedUserActionRequest, UserActionBasis, UserActionRequestBody,
        UserActionResolutionBody,
    },
    values::{
        MethodName, UserActionBasisStatus, UserActionChannelKind, UserActionKind,
        UserActionOptionAction, UserActionStatus, UtcTimestamp,
    },
};

use super::{
    facade::CoreProjectStore,
    record_refs::StoredRecordRef,
    validation::{
        parse_user_action_basis_status, parse_user_action_channel_kind, parse_user_action_kind,
        user_action_channel_kind_as_str, validate_identifier, validate_json_text,
        validate_stored_timestamp, validate_stored_user_action_request_column_agreement,
        validate_user_action_resolution_column_agreement,
        validate_user_action_resolution_provenance, validate_user_action_timestamp_order,
        UserActionRequestColumnFacts, UserActionTimestampOrderFailure,
    },
};
use crate::{StoreError, StoreResult};

const USER_ACTION_REQUEST_COLUMNS: &str = "
    project_id, user_action_request_id, task_id, change_unit_id, action_kind,
    request_json, basis_json, basis_status, required_for_json,
    requested_by_actor_source, source_method, source_idempotency_key,
    requested_at, expires_at, metadata_json";

const USER_ACTION_RESOLUTION_COLUMNS: &str = "
    project_id, user_action_resolution_id, user_action_request_id, action_kind,
    channel_kind, channel_submission_id, resolution_json,
    resolved_by_actor_source, resolved_verification_basis,
    resolved_assurance_level, resolved_at";

/// Stored user-action request row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionRequestRecord {
    pub project_id: String,
    pub user_action_request_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub action_kind: UserActionKind,
    pub request_json: String,
    pub basis_json: String,
    pub basis_status: UserActionBasisStatus,
    pub required_for_json: String,
    pub requested_by_actor_source: String,
    pub source_method: String,
    pub source_idempotency_key: String,
    pub requested_at: String,
    pub expires_at: Option<String>,
    pub metadata_json: String,
}

/// Stored immutable user-action resolution row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionResolutionRecord {
    pub project_id: String,
    pub user_action_resolution_id: String,
    pub user_action_request_id: String,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub channel_submission_id: String,
    pub resolution_json: String,
    pub resolved_by_actor_source: String,
    pub resolved_verification_basis: String,
    pub resolved_assurance_level: String,
    pub resolved_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserActionRequestRecordRaw {
    project_id: String,
    user_action_request_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    action_kind: String,
    request_json: String,
    basis_json: String,
    basis_status: String,
    required_for_json: String,
    requested_by_actor_source: String,
    source_method: String,
    source_idempotency_key: String,
    requested_at: String,
    expires_at: Option<String>,
    metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserActionResolutionRecordRaw {
    project_id: String,
    user_action_resolution_id: String,
    user_action_request_id: String,
    action_kind: String,
    channel_kind: String,
    channel_submission_id: String,
    resolution_json: String,
    resolved_by_actor_source: String,
    resolved_verification_basis: String,
    resolved_assurance_level: String,
    resolved_at: String,
}

/// Stored request and optional resolution with its derived current lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveUserActionRecord {
    pub request: UserActionRequestRecord,
    pub resolution: Option<UserActionResolutionRecord>,
    pub status: UserActionStatus,
}

impl CoreProjectStore<'_> {
    /// Lists effective pending user-action refs for a Task at the supplied instant.
    pub fn pending_user_action_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        effective_user_action_refs(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            UserActionStatus::Pending,
            state_version,
            now,
        )
    }

    /// Lists effective pending user-action records for a Task.
    pub fn pending_user_action_records(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        effective_user_action_records_for_task(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            Some(UserActionStatus::Pending),
            now,
        )
    }

    /// Lists all user-action records for a Task in stable creation order.
    pub fn user_action_records_for_task(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        effective_user_action_records_for_task(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            None,
            now,
        )
    }

    /// Lists stale or superseded user-action refs for a Task and action kind.
    pub fn non_current_user_action_refs(
        &self,
        task_id: &TaskId,
        action_kind: UserActionKind,
        state_version: u64,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        non_current_user_action_refs(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            action_kind,
            state_version,
            now,
        )
    }

    /// Reads one user-action request and optional resolution by request identity.
    pub fn user_action_record(
        &self,
        user_action_request_id: &str,
        now: &UtcTimestamp,
    ) -> StoreResult<Option<EffectiveUserActionRecord>> {
        effective_user_action_record(
            &self.conn,
            &self.project.project_id,
            user_action_request_id,
            now,
        )
    }

    /// Returns whether a user-action request id exists in this project.
    pub fn user_action_request_id_exists(&self, user_action_request_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM user_action_requests
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                params![self.project.project_id, user_action_request_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads one user-action resolution by its exact project-local identity.
    pub fn user_action_resolution_record(
        &self,
        user_action_resolution_id: &str,
    ) -> StoreResult<Option<UserActionResolutionRecord>> {
        user_action_resolution_record_by_id(
            &self.conn,
            &self.project.project_id,
            user_action_resolution_id,
        )
    }

    /// Reads one user-action resolution by its stable channel submission identity.
    pub fn user_action_resolution_for_channel_submission(
        &self,
        channel_kind: UserActionChannelKind,
        channel_submission_id: &str,
    ) -> StoreResult<Option<UserActionResolutionRecord>> {
        user_action_resolution_record_by_channel_submission(
            &self.conn,
            &self.project.project_id,
            channel_kind,
            channel_submission_id,
        )
    }

    /// Lists effective resolved user-action records for a Task and action kind.
    pub fn resolved_user_action_records(
        &self,
        task_id: &TaskId,
        action_kind: UserActionKind,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        effective_user_action_records_for_task_and_kind(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            action_kind,
            UserActionStatus::Resolved,
            now,
        )
    }
}

pub(crate) fn user_action_request_record(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
) -> StoreResult<Option<UserActionRequestRecord>> {
    let sql = format!(
        "SELECT {USER_ACTION_REQUEST_COLUMNS}
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2"
    );
    let raw = conn
        .query_row(
            &sql,
            params![project_id, user_action_request_id],
            user_action_request_record_raw_from_row,
        )
        .optional()?;
    raw.map(decode_user_action_request_record).transpose()
}

fn user_action_resolution_record_by_request(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let sql = format!(
        "SELECT {USER_ACTION_RESOLUTION_COLUMNS}
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_request_id = ?2"
    );
    let raw = conn
        .query_row(
            &sql,
            params![project_id, user_action_request_id],
            user_action_resolution_record_raw_from_row,
        )
        .optional()?;
    raw.map(decode_user_action_resolution_record).transpose()
}

fn user_action_resolution_record_by_id(
    conn: &Connection,
    project_id: &str,
    user_action_resolution_id: &str,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let sql = format!(
        "SELECT {USER_ACTION_RESOLUTION_COLUMNS}
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2"
    );
    let raw = conn
        .query_row(
            &sql,
            params![project_id, user_action_resolution_id],
            user_action_resolution_record_raw_from_row,
        )
        .optional()?;
    let resolution = raw.map(decode_user_action_resolution_record).transpose()?;
    validate_resolution_with_stored_request(conn, project_id, resolution)
}

fn user_action_resolution_record_by_channel_submission(
    conn: &Connection,
    project_id: &str,
    channel_kind: UserActionChannelKind,
    channel_submission_id: &str,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let sql = format!(
        "SELECT {USER_ACTION_RESOLUTION_COLUMNS}
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND channel_kind = ?2
            AND channel_submission_id = ?3"
    );
    let raw = conn
        .query_row(
            &sql,
            params![
                project_id,
                user_action_channel_kind_as_str(channel_kind),
                channel_submission_id
            ],
            user_action_resolution_record_raw_from_row,
        )
        .optional()?;
    let resolution = raw.map(decode_user_action_resolution_record).transpose()?;
    validate_resolution_with_stored_request(conn, project_id, resolution)
}

fn user_action_request_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<UserActionRequestRecordRaw> {
    Ok(UserActionRequestRecordRaw {
        project_id: row.get(0)?,
        user_action_request_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        action_kind: row.get(4)?,
        request_json: row.get(5)?,
        basis_json: row.get(6)?,
        basis_status: row.get(7)?,
        required_for_json: row.get(8)?,
        requested_by_actor_source: row.get(9)?,
        source_method: row.get(10)?,
        source_idempotency_key: row.get(11)?,
        requested_at: row.get(12)?,
        expires_at: row.get(13)?,
        metadata_json: row.get(14)?,
    })
}

fn decode_user_action_request_record(
    raw: UserActionRequestRecordRaw,
) -> StoreResult<UserActionRequestRecord> {
    let record_id = raw.user_action_request_id.as_str();
    let action_kind = parse_user_action_kind(
        record_id,
        "user_action_requests.action_kind",
        &raw.action_kind,
    )?;
    let basis_status = parse_user_action_basis_status(
        record_id,
        "user_action_requests.basis_status",
        &raw.basis_status,
    )?;
    validate_json_text("user_action_requests.metadata_json", &raw.metadata_json).map_err(|_| {
        StoreError::corrupt_owner_state_json("user_action_requests", record_id, "metadata_json")
    })?;
    if raw.source_method != MethodName::RequestUserAction.as_str()
        && raw.source_method != MethodName::ReconcileChanges.as_str()
    {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_id,
            "source_method",
        ));
    }
    validate_identifier(
        "user_action_requests.source_idempotency_key",
        &raw.source_idempotency_key,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_id,
            "source_idempotency_key",
        )
    })?;
    validate_stored_timestamp("user_action_requests.requested_at", &raw.requested_at)?;
    if let Some(expires_at) = &raw.expires_at {
        validate_stored_timestamp("user_action_requests.expires_at", expires_at)?;
    }
    validate_stored_user_action_request_column_agreement(
        record_id,
        UserActionRequestColumnFacts {
            task_id: &raw.task_id,
            change_unit_id: raw.change_unit_id.as_deref(),
            request_json: &raw.request_json,
            basis_json: &raw.basis_json,
            required_for_json: &raw.required_for_json,
            requested_at: &raw.requested_at,
            expires_at: raw.expires_at.as_deref(),
            action_kind,
            basis_status,
        },
    )?;
    Ok(UserActionRequestRecord {
        project_id: raw.project_id,
        user_action_request_id: raw.user_action_request_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        action_kind,
        request_json: raw.request_json,
        basis_json: raw.basis_json,
        basis_status,
        required_for_json: raw.required_for_json,
        requested_by_actor_source: raw.requested_by_actor_source,
        source_method: raw.source_method,
        source_idempotency_key: raw.source_idempotency_key,
        requested_at: raw.requested_at,
        expires_at: raw.expires_at,
        metadata_json: raw.metadata_json,
    })
}

fn user_action_resolution_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<UserActionResolutionRecordRaw> {
    Ok(UserActionResolutionRecordRaw {
        project_id: row.get(0)?,
        user_action_resolution_id: row.get(1)?,
        user_action_request_id: row.get(2)?,
        action_kind: row.get(3)?,
        channel_kind: row.get(4)?,
        channel_submission_id: row.get(5)?,
        resolution_json: row.get(6)?,
        resolved_by_actor_source: row.get(7)?,
        resolved_verification_basis: row.get(8)?,
        resolved_assurance_level: row.get(9)?,
        resolved_at: row.get(10)?,
    })
}

fn decode_user_action_resolution_record(
    raw: UserActionResolutionRecordRaw,
) -> StoreResult<UserActionResolutionRecord> {
    let record_id = raw.user_action_resolution_id.as_str();
    let action_kind = parse_user_action_kind(
        record_id,
        "user_action_resolutions.action_kind",
        &raw.action_kind,
    )?;
    let channel_kind = parse_user_action_channel_kind(
        record_id,
        "user_action_resolutions.channel_kind",
        &raw.channel_kind,
    )?;
    validate_user_action_resolution_column_agreement(
        &raw.resolution_json,
        action_kind,
        &raw.user_action_resolution_id,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            record_id,
            "resolution_json",
        )
    })?;
    if validate_channel_submission_id(&raw.channel_submission_id).is_err()
        || validate_user_action_resolution_provenance(
            channel_kind,
            &raw.resolved_by_actor_source,
            &raw.resolved_verification_basis,
            &raw.resolved_assurance_level,
        )
        .is_err()
    {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            record_id,
            "resolved_verification_basis",
        ));
    }
    validate_stored_timestamp("user_action_resolutions.resolved_at", &raw.resolved_at)?;
    Ok(UserActionResolutionRecord {
        project_id: raw.project_id,
        user_action_resolution_id: raw.user_action_resolution_id,
        user_action_request_id: raw.user_action_request_id,
        action_kind,
        channel_kind,
        channel_submission_id: raw.channel_submission_id,
        resolution_json: raw.resolution_json,
        resolved_by_actor_source: raw.resolved_by_actor_source,
        resolved_verification_basis: raw.resolved_verification_basis,
        resolved_assurance_level: raw.resolved_assurance_level,
        resolved_at: raw.resolved_at,
    })
}

pub(crate) fn effective_user_action_record(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
    now: &UtcTimestamp,
) -> StoreResult<Option<EffectiveUserActionRecord>> {
    let Some(request) = user_action_request_record(conn, project_id, user_action_request_id)?
    else {
        return Ok(None);
    };
    let resolution =
        user_action_resolution_record_by_request(conn, project_id, user_action_request_id)?;
    if let Some(resolution) = &resolution {
        validate_user_action_request_resolution_pair(&request, resolution)?;
    }
    let status = effective_user_action_status(&request, resolution.as_ref(), now)?;
    Ok(Some(EffectiveUserActionRecord {
        request,
        resolution,
        status,
    }))
}

fn effective_user_action_records_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    status_filter: Option<UserActionStatus>,
    now: &UtcTimestamp,
) -> StoreResult<Vec<EffectiveUserActionRecord>> {
    let mut statement = conn.prepare(
        "SELECT user_action_request_id
           FROM user_action_requests
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY volicord_utc_seconds(requested_at),
                   volicord_utc_subsec_nanos(requested_at),
                   user_action_request_id",
    )?;
    let rows = statement.query_map(params![project_id, task_id], |row| row.get::<_, String>(0))?;
    let mut records = Vec::new();
    for row in rows {
        let request_id = row?;
        let record =
            effective_user_action_record(conn, project_id, &request_id, now)?.ok_or_else(|| {
                StoreError::SchemaInvariant {
                    database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                    detail: format!("user action request {request_id} disappeared during read"),
                }
            })?;
        if status_filter.is_none_or(|expected| record.status == expected) {
            records.push(record);
        }
    }
    Ok(records)
}

fn effective_user_action_records_for_task_and_kind(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    action_kind: UserActionKind,
    status_filter: UserActionStatus,
    now: &UtcTimestamp,
) -> StoreResult<Vec<EffectiveUserActionRecord>> {
    Ok(
        effective_user_action_records_for_task(conn, project_id, task_id, None, now)?
            .into_iter()
            .filter(|record| {
                record.request.action_kind == action_kind && record.status == status_filter
            })
            .collect(),
    )
}

/// Derives the current lifecycle status from immutable resolution presence, basis status, and time.
pub fn effective_user_action_status(
    request: &UserActionRequestRecord,
    resolution: Option<&UserActionResolutionRecord>,
    now: &UtcTimestamp,
) -> StoreResult<UserActionStatus> {
    let created_at = UtcTimestamp::parse(&request.requested_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })?;
    let expires_at = request
        .expires_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| StoreError::CorruptStoredValue {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            field: "user_action_requests.expires_at",
        })?;
    if let Some(resolution) = resolution {
        let resolved_at = UtcTimestamp::parse(&resolution.resolved_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                &resolution.user_action_resolution_id,
                "resolved_at",
            )
        })?;
        if &resolved_at > now {
            return Err(StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                &resolution.user_action_resolution_id,
                "resolved_at",
            ));
        }
    }
    derive_user_action_status(
        request.basis_status,
        &created_at,
        expires_at.as_ref(),
        resolution.is_some(),
        now,
    )
    .ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })
}

pub(super) fn validate_user_action_request_resolution_pair(
    request: &UserActionRequestRecord,
    resolution: &UserActionResolutionRecord,
) -> StoreResult<()> {
    if request.project_id != resolution.project_id
        || request.user_action_request_id != resolution.user_action_request_id
        || request.action_kind != resolution.action_kind
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "user-action resolution does not match its request identity and kind"
                .to_owned(),
        });
    }
    validate_stored_user_action_timestamp_order(request, resolution)?;
    let persisted_request = serde_json::from_str::<PersistedUserActionRequest>(
        &request.request_json,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "request_json",
        )
    })?;
    let basis = serde_json::from_str::<UserActionBasis>(&request.basis_json).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "basis_json",
        )
    })?;
    let resolution_body = serde_json::from_str::<UserActionResolutionBody>(
        &resolution.resolution_json,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolution_json",
        )
    })?;

    let agrees = match (&persisted_request.body, &basis, &resolution_body) {
        (
            UserActionRequestBody::Choice(choice),
            UserActionBasis::Choice(choice_basis),
            UserActionResolutionBody::Choice {
                selected_option_id,
                machine_action,
                resolution_outcome,
                accepted_risk_ids,
                ..
            },
        ) => choice
            .options
            .iter()
            .find(|option| option.option_id == *selected_option_id)
            .is_some_and(|option| {
                let expected_risk_ids = if request.action_kind
                    == UserActionKind::ResidualRiskAcceptance
                    && option.machine_action == UserActionOptionAction::Accept
                {
                    choice_basis.residual_risk_ids.as_slice()
                } else {
                    &[]
                };
                option.machine_action == *machine_action
                    && option.resolution_outcome == *resolution_outcome
                    && accepted_risk_ids == expected_risk_ids
            }),
        (
            UserActionRequestBody::EvidenceObservation(observation_request),
            UserActionBasis::EvidenceObservation(observation_basis),
            UserActionResolutionBody::EvidenceObservation { observation },
        ) => {
            let unique_artifact_ids = observation
                .output_artifact_refs
                .iter()
                .map(|artifact| &artifact.artifact_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == observation.output_artifact_refs.len();
            observation_request.target_candidates == observation_basis.target_candidates
                && observation_request.artifact_candidates == observation_basis.artifact_candidates
                && observation_request
                    .target_candidates
                    .contains(&observation.target)
                && observation.output_artifact_refs.iter().all(|selected| {
                    observation_request
                        .artifact_candidates
                        .iter()
                        .any(|candidate| user_action_artifact_ref_agrees(candidate, selected))
                })
                && unique_artifact_ids
                && matches!(
                    observation.relevance_status,
                    volicord_types::values::EvidenceRelevanceStatus::Supported
                        | volicord_types::values::EvidenceRelevanceStatus::Contradicted
                )
                && !observation.summary.trim().is_empty()
        }
        _ => false,
    };
    if !agrees {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolution_json",
        ));
    }
    Ok(())
}

fn validate_stored_user_action_timestamp_order(
    request: &UserActionRequestRecord,
    resolution: &UserActionResolutionRecord,
) -> StoreResult<()> {
    let requested_at = UtcTimestamp::parse(&request.requested_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })?;
    let expires_at = request
        .expires_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "user_action_requests",
                &request.user_action_request_id,
                "expires_at",
            )
        })?;
    let resolved_at = UtcTimestamp::parse(&resolution.resolved_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolved_at",
        )
    })?;
    match validate_user_action_timestamp_order(
        &requested_at,
        expires_at.as_ref(),
        Some(&resolved_at),
    ) {
        Ok(()) => Ok(()),
        Err(UserActionTimestampOrderFailure::ExpiryNotAfterRequest) => {
            Err(StoreError::corrupt_owner_state_value(
                "user_action_requests",
                &request.user_action_request_id,
                "expires_at",
            ))
        }
        Err(
            UserActionTimestampOrderFailure::ResolutionBeforeRequest
            | UserActionTimestampOrderFailure::ResolutionAtOrAfterExpiry,
        ) => Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolved_at",
        )),
    }
}

fn user_action_artifact_ref_agrees(candidate: &ArtifactRef, selected: &ArtifactRef) -> bool {
    candidate == selected
}

fn validate_resolution_with_stored_request(
    conn: &Connection,
    project_id: &str,
    resolution: Option<UserActionResolutionRecord>,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let Some(resolution) = resolution else {
        return Ok(None);
    };
    let request = user_action_request_record(conn, project_id, &resolution.user_action_request_id)?
        .ok_or_else(|| StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "user-action resolution has no matching request".to_owned(),
        })?;
    validate_user_action_request_resolution_pair(&request, &resolution)?;
    Ok(Some(resolution))
}

fn non_current_user_action_refs(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    action_kind: UserActionKind,
    state_version: u64,
    now: &UtcTimestamp,
) -> StoreResult<Vec<StoredRecordRef>> {
    Ok(
        effective_user_action_records_for_task(conn, project_id, task_id, None, now)?
            .into_iter()
            .filter(|record| {
                record.request.action_kind == action_kind
                    && matches!(
                        record.status,
                        UserActionStatus::Stale | UserActionStatus::Superseded
                    )
            })
            .map(|record| StoredRecordRef {
                record_kind: "user_action_request".to_owned(),
                record_id: record.request.user_action_request_id,
                project_id: project_id.to_owned(),
                task_id: Some(task_id.to_owned()),
                state_version: Some(state_version),
            })
            .collect(),
    )
}

fn effective_user_action_refs(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    status: UserActionStatus,
    state_version: u64,
    now: &UtcTimestamp,
) -> StoreResult<Vec<StoredRecordRef>> {
    Ok(
        effective_user_action_records_for_task(conn, project_id, task_id, Some(status), now)?
            .into_iter()
            .map(|record| StoredRecordRef {
                record_kind: "user_action_request".to_owned(),
                record_id: record.request.user_action_request_id,
                project_id: project_id.to_owned(),
                task_id: Some(task_id.to_owned()),
                state_version: Some(state_version),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_decoder_rejects_an_unknown_source_method() {
        let error = decode_user_action_request_record(UserActionRequestRecordRaw {
            project_id: "project".to_owned(),
            user_action_request_id: "request".to_owned(),
            task_id: "task".to_owned(),
            change_unit_id: None,
            action_kind: "product_decision".to_owned(),
            request_json: "{}".to_owned(),
            basis_json: "{}".to_owned(),
            basis_status: "current".to_owned(),
            required_for_json: "[]".to_owned(),
            requested_by_actor_source: "local_user".to_owned(),
            source_method: "volicord.unknown".to_owned(),
            source_idempotency_key: "idempotency".to_owned(),
            requested_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: None,
            metadata_json: "{}".to_owned(),
        })
        .expect_err("unknown source method must fail closed");

        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "source_method",
                ..
            }
        ));
    }
}
