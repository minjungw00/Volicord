use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::{
    ids::TaskId,
    schema::{
        effective_user_action_status as derive_user_action_status, validate_channel_submission_id,
        ArtifactRef, PersistedUserActionRequest, PersistedUserActionRequestMetadata,
        PersistedUserActionResolution, UserActionBasis, UserActionRequestBody,
        UserActionResolutionBody,
    },
    values::{
        ActorSource, MethodName, StateRecordKind, UserActionBasisStatus, UserActionChannelKind,
        UserActionKind, UserActionOptionAction, UserActionRequiredFor, UserActionStatus,
        UserActionVerificationBasis, UtcTimestamp,
    },
};

use super::{
    facade::CoreProjectStore, mutations::MutationContext, record_refs::StoredRecordRef,
    validation::*,
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

/// User-action mutation applied inside one Core commit transaction.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum UserActionMutation {
    InsertRequest(UserActionRequestInsert),
    InsertResolution(UserActionResolutionInsert),
    UpdateBasis(UserActionBasisUpdate),
    MarkBasesStatus(UserActionBasisStatusMark),
    MarkSupersededOrStale(UserActionInvalidation),
}

/// Storage input for inserting a pending user-action request.
#[derive(Debug, Clone, PartialEq)]
pub struct UserActionRequestInsert {
    pub user_action_request_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub action_kind: UserActionKind,
    pub request: PersistedUserActionRequest,
    pub basis: UserActionBasis,
    pub basis_status: UserActionBasisStatus,
    pub required_for: Vec<UserActionRequiredFor>,
    pub requested_by_actor_source: ActorSource,
    pub source_method: MethodName,
    pub source_idempotency_key: String,
    pub requested_at: UtcTimestamp,
    pub expires_at: Option<UtcTimestamp>,
    pub metadata: PersistedUserActionRequestMetadata,
}

/// Storage input for inserting one immutable user-action resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct UserActionResolutionInsert {
    pub user_action_resolution_id: String,
    pub user_action_request_id: String,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub channel_submission_id: String,
    pub resolution: PersistedUserActionResolution,
    pub resolved_by_actor_source: ActorSource,
    pub resolved_verification_basis: UserActionVerificationBasis,
    pub resolved_assurance_level: String,
    pub resolved_at: UtcTimestamp,
}

/// Storage input for replacing one user-action basis snapshot and compatibility status.
#[derive(Debug, Clone, PartialEq)]
pub struct UserActionBasisUpdate {
    pub user_action_request_id: String,
    pub basis: UserActionBasis,
    pub basis_status: UserActionBasisStatus,
}

/// Storage input for marking selected user-action basis rows stale or superseded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionBasisStatusMark {
    pub user_action_request_ids: Vec<String>,
    pub basis_status: UserActionBasisStatus,
}

/// Storage input for invalidating current user-action authority after state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionInvalidation {
    pub task_id: String,
    pub action_kinds: Vec<UserActionKind>,
}

impl UserActionMutation {
    pub(super) fn apply(&self, context: &mut MutationContext<'_>) -> StoreResult<()> {
        match self {
            Self::InsertRequest(input) => context.insert_user_action_request(input),
            Self::InsertResolution(input) => context.insert_user_action_resolution(input),
            Self::UpdateBasis(input) => context.update_user_action_basis(input),
            Self::MarkBasesStatus(input) => context.mark_user_action_bases_status(input),
            Self::MarkSupersededOrStale(input) => {
                context.mark_user_actions_superseded_or_stale(input)
            }
        }
    }
}

/// Store-validated persisted UserAction request.
///
/// The fields are private so external crates can only obtain values that have
/// passed Store-owned persisted-record validation.
///
/// ```compile_fail,E0616
/// use volicord_store::core_pipeline::StoredUserActionRequest;
///
/// fn inspect(record: &StoredUserActionRequest) {
///     let _ = &record.project_id;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StoredUserActionRequest {
    project_id: String,
    user_action_request_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    action_kind: UserActionKind,
    request: PersistedUserActionRequest,
    basis: UserActionBasis,
    basis_status: UserActionBasisStatus,
    required_for: Vec<UserActionRequiredFor>,
    requested_by_actor_source: ActorSource,
    source_method: MethodName,
    source_idempotency_key: String,
    requested_at: UtcTimestamp,
    expires_at: Option<UtcTimestamp>,
    metadata: PersistedUserActionRequestMetadata,
}

impl StoredUserActionRequest {
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn user_action_request_id(&self) -> &str {
        &self.user_action_request_id
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn change_unit_id(&self) -> Option<&str> {
        self.change_unit_id.as_deref()
    }

    pub const fn action_kind(&self) -> UserActionKind {
        self.action_kind
    }

    pub const fn request(&self) -> &PersistedUserActionRequest {
        &self.request
    }

    pub const fn basis(&self) -> &UserActionBasis {
        &self.basis
    }

    pub const fn basis_status(&self) -> UserActionBasisStatus {
        self.basis_status
    }

    pub fn required_for(&self) -> &[UserActionRequiredFor] {
        &self.required_for
    }

    pub const fn requested_by_actor_source(&self) -> &ActorSource {
        &self.requested_by_actor_source
    }

    pub const fn source_method(&self) -> MethodName {
        self.source_method
    }

    pub fn source_idempotency_key(&self) -> &str {
        &self.source_idempotency_key
    }

    pub const fn requested_at(&self) -> &UtcTimestamp {
        &self.requested_at
    }

    pub const fn expires_at(&self) -> Option<&UtcTimestamp> {
        self.expires_at.as_ref()
    }

    pub const fn metadata(&self) -> &PersistedUserActionRequestMetadata {
        &self.metadata
    }
}

/// Store-validated persisted immutable UserAction resolution.
///
/// ```compile_fail,E0451
/// use volicord_store::core_pipeline::StoredUserActionResolution;
///
/// fn inspect(record: StoredUserActionResolution) {
///     let StoredUserActionResolution { resolution, .. } = record;
///     let _ = resolution;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StoredUserActionResolution {
    project_id: String,
    user_action_resolution_id: String,
    user_action_request_id: String,
    action_kind: UserActionKind,
    channel_kind: UserActionChannelKind,
    channel_submission_id: String,
    resolution: PersistedUserActionResolution,
    resolved_by_actor_source: ActorSource,
    resolved_verification_basis: UserActionVerificationBasis,
    resolved_assurance_level: String,
    resolved_at: UtcTimestamp,
}

impl StoredUserActionResolution {
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn user_action_resolution_id(&self) -> &str {
        &self.user_action_resolution_id
    }

    pub fn user_action_request_id(&self) -> &str {
        &self.user_action_request_id
    }

    pub const fn action_kind(&self) -> UserActionKind {
        self.action_kind
    }

    pub const fn channel_kind(&self) -> UserActionChannelKind {
        self.channel_kind
    }

    pub fn channel_submission_id(&self) -> &str {
        &self.channel_submission_id
    }

    pub const fn resolution(&self) -> &PersistedUserActionResolution {
        &self.resolution
    }

    pub const fn resolved_by_actor_source(&self) -> &ActorSource {
        &self.resolved_by_actor_source
    }

    pub const fn resolved_verification_basis(&self) -> UserActionVerificationBasis {
        self.resolved_verification_basis
    }

    pub fn resolved_assurance_level(&self) -> &str {
        &self.resolved_assurance_level
    }

    pub const fn resolved_at(&self) -> &UtcTimestamp {
        &self.resolved_at
    }
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

/// One validated persisted UserAction request and its optional resolution.
///
/// Supported consumers receive a validated set and inspect its semantic facts
/// through typed accessors.
///
/// ```
/// use volicord_store::core_pipeline::StoredUserActionRecordSet;
///
/// fn inspect(record: &StoredUserActionRecordSet) {
///     let request = record.request();
///     let _ = request.project_id();
///     let _ = request.action_kind();
///     let _ = record.status();
///
///     if let Some(resolution) = record.resolution() {
///         let _ = resolution.user_action_resolution_id();
///         let _ = resolution.resolution();
///     }
/// }
/// ```
///
/// ```compile_fail,E0616
/// use volicord_store::core_pipeline::StoredUserActionRecordSet;
///
/// fn inspect(record: &StoredUserActionRecordSet) {
///     let _ = &record.request;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StoredUserActionRecordSet {
    request: StoredUserActionRequest,
    resolution: Option<StoredUserActionResolution>,
    status: UserActionStatus,
}

impl StoredUserActionRecordSet {
    /// Constructs the pending projection for a canonical typed Store insert.
    pub fn from_pending_insert(
        project_id: impl Into<String>,
        input: &UserActionRequestInsert,
    ) -> StoreResult<Self> {
        let request = stored_user_action_request_from_insert(project_id.into(), input)?;
        Ok(Self {
            request,
            resolution: None,
            status: UserActionStatus::Pending,
        })
    }

    /// Projects one canonical typed resolution against this validated request.
    pub fn with_resolution(
        &self,
        input: &UserActionResolutionInsert,
        now: &UtcTimestamp,
    ) -> StoreResult<Self> {
        if self.resolution.is_some() {
            return Err(StoreError::InvalidInput {
                detail: "a UserAction request cannot have more than one resolution".to_owned(),
            });
        }
        let resolution =
            stored_user_action_resolution_from_insert(self.request.project_id(), input)?;
        validate_user_action_request_resolution_pair(&self.request, &resolution).map_err(|_| {
            StoreError::InvalidInput {
                detail: "UserAction resolution does not match the validated request".to_owned(),
            }
        })?;
        let status =
            effective_user_action_status(&self.request, Some(&resolution), now).map_err(|_| {
                StoreError::InvalidInput {
                    detail: "UserAction resolution is not valid at the supplied instant".to_owned(),
                }
            })?;
        Ok(Self {
            request: self.request.clone(),
            resolution: Some(resolution),
            status,
        })
    }

    pub const fn request(&self) -> &StoredUserActionRequest {
        &self.request
    }

    pub const fn resolution(&self) -> Option<&StoredUserActionResolution> {
        self.resolution.as_ref()
    }

    pub const fn status(&self) -> UserActionStatus {
        self.status
    }
}

fn stored_user_action_request_from_insert(
    project_id: String,
    input: &UserActionRequestInsert,
) -> StoreResult<StoredUserActionRequest> {
    if input.basis_status != UserActionBasisStatus::Current {
        return Err(StoreError::InvalidInput {
            detail: "a newly constructed UserAction request must have a current basis".to_owned(),
        });
    }
    let request_json = encode_json_column("user_action_requests.request_json", &input.request)?;
    let basis_json = encode_json_column("user_action_requests.basis_json", &input.basis)?;
    let required_for_json = encode_json_column(
        "user_action_requests.required_for_json",
        &input.required_for,
    )?;
    let metadata_json = encode_json_column("user_action_requests.metadata_json", &input.metadata)?;
    let requested_at = input.requested_at.to_string();
    let expires_at = input.expires_at.as_ref().map(ToString::to_string);
    validate_identifier("project_id", &project_id)?;
    validate_identifier("user_action_request_id", &input.user_action_request_id)?;
    validate_identifier("task_id", &input.task_id)?;
    if let Some(change_unit_id) = &input.change_unit_id {
        validate_identifier("change_unit_id", change_unit_id)?;
    }
    validate_user_action_request_column_agreement(UserActionRequestColumnFacts {
        task_id: &input.task_id,
        change_unit_id: input.change_unit_id.as_deref(),
        request_json: &request_json,
        basis_json: &basis_json,
        required_for_json: &required_for_json,
        requested_at: &requested_at,
        expires_at: expires_at.as_deref(),
        action_kind: input.action_kind,
        basis_status: input.basis_status,
    })?;
    validate_json_text("user_action_requests.metadata_json", &metadata_json)?;
    let origin_matches_source = matches!(
        (&input.source_method, &input.metadata),
        (
            MethodName::RequestUserAction,
            PersistedUserActionRequestMetadata::DirectRequest(_)
        ) | (
            MethodName::ReconcileChanges,
            PersistedUserActionRequestMetadata::Reconciliation(_)
        ) | (
            MethodName::RecordShaping,
            PersistedUserActionRequestMetadata::Shaping(_)
        )
    );
    if !origin_matches_source {
        return Err(StoreError::InvalidInput {
            detail: "UserAction request origin must match its source method".to_owned(),
        });
    }
    validate_identifier(
        "requested_by_actor_source",
        &input.requested_by_actor_source.to_canonical_string(),
    )?;
    validate_identifier("source_idempotency_key", &input.source_idempotency_key)?;
    Ok(StoredUserActionRequest {
        project_id,
        user_action_request_id: input.user_action_request_id.clone(),
        task_id: input.task_id.clone(),
        change_unit_id: input.change_unit_id.clone(),
        action_kind: input.action_kind,
        request: input.request.clone(),
        basis: input.basis.clone(),
        basis_status: input.basis_status,
        required_for: input.required_for.clone(),
        requested_by_actor_source: input.requested_by_actor_source.clone(),
        source_method: input.source_method,
        source_idempotency_key: input.source_idempotency_key.clone(),
        requested_at: input.requested_at.clone(),
        expires_at: input.expires_at.clone(),
        metadata: input.metadata.clone(),
    })
}

fn stored_user_action_resolution_from_insert(
    project_id: &str,
    input: &UserActionResolutionInsert,
) -> StoreResult<StoredUserActionResolution> {
    let resolution_json =
        encode_json_column("user_action_resolutions.resolution_json", &input.resolution)?;
    validate_identifier("project_id", project_id)?;
    validate_identifier(
        "user_action_resolution_id",
        &input.user_action_resolution_id,
    )?;
    validate_identifier("user_action_request_id", &input.user_action_request_id)?;
    validate_channel_submission_id(&input.channel_submission_id).map_err(|error| {
        StoreError::InvalidInput {
            detail: error.to_string(),
        }
    })?;
    validate_user_action_resolution_column_agreement(
        &resolution_json,
        input.action_kind,
        &input.user_action_resolution_id,
    )?;
    validate_user_action_resolution_provenance(
        input.channel_kind,
        &input.resolved_by_actor_source.to_canonical_string(),
        input.resolved_verification_basis,
        &input.resolved_assurance_level,
    )?;
    Ok(StoredUserActionResolution {
        project_id: project_id.to_owned(),
        user_action_resolution_id: input.user_action_resolution_id.clone(),
        user_action_request_id: input.user_action_request_id.clone(),
        action_kind: input.action_kind,
        channel_kind: input.channel_kind,
        channel_submission_id: input.channel_submission_id.clone(),
        resolution: input.resolution.clone(),
        resolved_by_actor_source: input.resolved_by_actor_source.clone(),
        resolved_verification_basis: input.resolved_verification_basis,
        resolved_assurance_level: input.resolved_assurance_level.clone(),
        resolved_at: input.resolved_at.clone(),
    })
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
    ) -> StoreResult<Vec<StoredUserActionRecordSet>> {
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
    ) -> StoreResult<Vec<StoredUserActionRecordSet>> {
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
    ) -> StoreResult<Option<StoredUserActionRecordSet>> {
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
    ) -> StoreResult<Option<StoredUserActionResolution>> {
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
    ) -> StoreResult<Option<StoredUserActionResolution>> {
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
    ) -> StoreResult<Vec<StoredUserActionRecordSet>> {
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
) -> StoreResult<Option<StoredUserActionRequest>> {
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
) -> StoreResult<Option<StoredUserActionResolution>> {
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
) -> StoreResult<Option<StoredUserActionResolution>> {
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
) -> StoreResult<Option<StoredUserActionResolution>> {
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
) -> StoreResult<StoredUserActionRequest> {
    let record_id = raw.user_action_request_id.as_str();
    let action_kind = parse_user_action_kind(
        "user_action_requests",
        record_id,
        "action_kind",
        &raw.action_kind,
    )?;
    let basis_status =
        parse_user_action_basis_status(record_id, "basis_status", &raw.basis_status)?;
    validate_json_text("user_action_requests.metadata_json", &raw.metadata_json).map_err(|_| {
        StoreError::corrupt_owner_state_json("user_action_requests", record_id, "metadata_json")
    })?;
    if raw.source_method != MethodName::RequestUserAction.as_str()
        && raw.source_method != MethodName::ReconcileChanges.as_str()
        && raw.source_method != MethodName::RecordShaping.as_str()
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
    let request =
        serde_json::from_str::<PersistedUserActionRequest>(&raw.request_json).map_err(|_| {
            StoreError::corrupt_owner_state_json("user_action_requests", record_id, "request_json")
        })?;
    let basis = serde_json::from_str::<UserActionBasis>(&raw.basis_json).map_err(|_| {
        StoreError::corrupt_owner_state_json("user_action_requests", record_id, "basis_json")
    })?;
    let required_for = serde_json::from_str::<Vec<UserActionRequiredFor>>(&raw.required_for_json)
        .map_err(|_| {
        StoreError::corrupt_owner_state_json("user_action_requests", record_id, "required_for_json")
    })?;
    let requested_by_actor_source = raw.requested_by_actor_source.parse().map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_id,
            "requested_by_actor_source",
        )
    })?;
    let source_method = match raw.source_method.as_str() {
        value if value == MethodName::RequestUserAction.as_str() => MethodName::RequestUserAction,
        value if value == MethodName::ReconcileChanges.as_str() => MethodName::ReconcileChanges,
        value if value == MethodName::RecordShaping.as_str() => MethodName::RecordShaping,
        _ => {
            return Err(StoreError::corrupt_owner_state_value(
                "user_action_requests",
                record_id,
                "source_method",
            ))
        }
    };
    let requested_at = UtcTimestamp::parse(&raw.requested_at).map_err(|_| {
        StoreError::corrupt_owner_state_value("user_action_requests", record_id, "requested_at")
    })?;
    let expires_at = raw
        .expires_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("user_action_requests", record_id, "expires_at")
        })?;
    let metadata = serde_json::from_str::<PersistedUserActionRequestMetadata>(&raw.metadata_json)
        .map_err(|_| {
        StoreError::corrupt_owner_state_json("user_action_requests", record_id, "metadata_json")
    })?;
    let origin_matches_source = matches!(
        (&source_method, &metadata),
        (
            MethodName::RequestUserAction,
            PersistedUserActionRequestMetadata::DirectRequest(_)
        ) | (
            MethodName::ReconcileChanges,
            PersistedUserActionRequestMetadata::Reconciliation(_)
        ) | (
            MethodName::RecordShaping,
            PersistedUserActionRequestMetadata::Shaping(_)
        )
    );
    if !origin_matches_source {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_id,
            "source_method",
        ));
    }
    Ok(StoredUserActionRequest {
        project_id: raw.project_id,
        user_action_request_id: raw.user_action_request_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        action_kind,
        request,
        basis,
        basis_status,
        required_for,
        requested_by_actor_source,
        source_method,
        source_idempotency_key: raw.source_idempotency_key,
        requested_at,
        expires_at,
        metadata,
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
) -> StoreResult<StoredUserActionResolution> {
    let record_id = raw.user_action_resolution_id.as_str();
    let action_kind = parse_user_action_kind(
        "user_action_resolutions",
        record_id,
        "action_kind",
        &raw.action_kind,
    )?;
    let channel_kind =
        parse_user_action_channel_kind(record_id, "channel_kind", &raw.channel_kind)?;
    let resolved_verification_basis =
        UserActionVerificationBasis::parse(&raw.resolved_verification_basis).ok_or_else(|| {
            StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                record_id,
                "resolved_verification_basis",
            )
        })?;
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
            resolved_verification_basis,
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
    let resolution = serde_json::from_str::<PersistedUserActionResolution>(&raw.resolution_json)
        .map_err(|_| {
            StoreError::corrupt_owner_state_json(
                "user_action_resolutions",
                record_id,
                "resolution_json",
            )
        })?;
    resolution.validate().map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "user_action_resolutions",
            record_id,
            "resolution_json",
        )
    })?;
    let resolved_by_actor_source = raw.resolved_by_actor_source.parse().map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            record_id,
            "resolved_by_actor_source",
        )
    })?;
    let resolved_at = UtcTimestamp::parse(&raw.resolved_at).map_err(|_| {
        StoreError::corrupt_owner_state_value("user_action_resolutions", record_id, "resolved_at")
    })?;
    Ok(StoredUserActionResolution {
        project_id: raw.project_id,
        user_action_resolution_id: raw.user_action_resolution_id,
        user_action_request_id: raw.user_action_request_id,
        action_kind,
        channel_kind,
        channel_submission_id: raw.channel_submission_id,
        resolution,
        resolved_by_actor_source,
        resolved_verification_basis,
        resolved_assurance_level: raw.resolved_assurance_level,
        resolved_at,
    })
}

pub(crate) fn effective_user_action_record(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
    now: &UtcTimestamp,
) -> StoreResult<Option<StoredUserActionRecordSet>> {
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
    Ok(Some(StoredUserActionRecordSet {
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
) -> StoreResult<Vec<StoredUserActionRecordSet>> {
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
) -> StoreResult<Vec<StoredUserActionRecordSet>> {
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
    request: &StoredUserActionRequest,
    resolution: Option<&StoredUserActionResolution>,
    now: &UtcTimestamp,
) -> StoreResult<UserActionStatus> {
    if let Some(resolution) = resolution {
        if &resolution.resolved_at > now {
            return Err(StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                &resolution.user_action_resolution_id,
                "resolved_at",
            ));
        }
    }
    derive_user_action_status(
        request.basis_status,
        &request.requested_at,
        request.expires_at.as_ref(),
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
    request: &StoredUserActionRequest,
    resolution: &StoredUserActionResolution,
) -> StoreResult<()> {
    if request.project_id != resolution.project_id {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "project_id",
        ));
    }
    if request.user_action_request_id != resolution.user_action_request_id {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "user_action_request_id",
        ));
    }
    if request.action_kind != resolution.action_kind {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "action_kind",
        ));
    }
    validate_stored_user_action_timestamp_order(request, resolution)?;
    let agrees = match (
        &request.request.body,
        &request.basis,
        &resolution.resolution,
    ) {
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
    request: &StoredUserActionRequest,
    resolution: &StoredUserActionResolution,
) -> StoreResult<()> {
    match validate_user_action_timestamp_order(
        &request.requested_at,
        request.expires_at.as_ref(),
        Some(&resolution.resolved_at),
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
    resolution: Option<StoredUserActionResolution>,
) -> StoreResult<Option<StoredUserActionResolution>> {
    let Some(resolution) = resolution else {
        return Ok(None);
    };
    let request = user_action_request_record(conn, project_id, &resolution.user_action_request_id)?
        .ok_or_else(|| {
            StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                &resolution.user_action_resolution_id,
                "user_action_request_id",
            )
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
                record_kind: StateRecordKind::UserActionRequest,
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
                record_kind: StateRecordKind::UserActionRequest,
                record_id: record.request.user_action_request_id,
                project_id: project_id.to_owned(),
                task_id: Some(task_id.to_owned()),
                state_version: Some(state_version),
            })
            .collect(),
    )
}

fn validate_user_action_resolution_timestamp_order_for_insert(
    request: &StoredUserActionRequest,
    resolved_at: &str,
) -> StoreResult<()> {
    let resolved_at = UtcTimestamp::parse(resolved_at).map_err(|_| StoreError::InvalidInput {
        detail: "user_action_resolutions.resolved_at must be a valid RFC 3339 timestamp".to_owned(),
    })?;
    match validate_user_action_timestamp_order(
        &request.requested_at,
        request.expires_at.as_ref(),
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
        Err(UserActionTimestampOrderFailure::ResolutionBeforeRequest) => {
            Err(StoreError::InvalidInput {
                detail: "user_action_resolutions.resolved_at must be at or after user_action_requests.requested_at".to_owned(),
            })
        }
        Err(UserActionTimestampOrderFailure::ResolutionAtOrAfterExpiry) => {
            Err(StoreError::InvalidInput {
                detail: "user_action_resolutions.resolved_at must be before user_action_requests.expires_at".to_owned(),
            })
        }
    }
}

impl MutationContext<'_> {
    fn insert_user_action_request(&mut self, input: &UserActionRequestInsert) -> StoreResult<()> {
        let request_json = encode_json_column("user_action_requests.request_json", &input.request)?;
        let basis_json = encode_json_column("user_action_requests.basis_json", &input.basis)?;
        let required_for_json = encode_json_column(
            "user_action_requests.required_for_json",
            &input.required_for,
        )?;
        let metadata_json =
            encode_json_column("user_action_requests.metadata_json", &input.metadata)?;
        let requested_by_actor_source = input.requested_by_actor_source.to_canonical_string();
        let source_method = input.source_method.as_str();
        let requested_at = input.requested_at.to_string();
        let expires_at = input.expires_at.as_ref().map(ToString::to_string);
        validate_identifier("user_action_request_id", &input.user_action_request_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_persisted_user_action_request_json(
            "user_action_requests.request_json",
            &request_json,
        )?;
        validate_user_action_basis_json("user_action_requests.basis_json", &basis_json)?;
        validate_user_action_required_for_json(
            "user_action_requests.required_for_json",
            &required_for_json,
        )?;
        validate_identifier("requested_by_actor_source", &requested_by_actor_source)?;
        validate_timestamp("requested_at", &requested_at)?;
        if let Some(expires_at) = &expires_at {
            validate_timestamp("expires_at", expires_at)?;
        }
        let origin_matches_source = matches!(
            (&input.source_method, &input.metadata),
            (
                MethodName::RequestUserAction,
                PersistedUserActionRequestMetadata::DirectRequest(_)
            ) | (
                MethodName::ReconcileChanges,
                PersistedUserActionRequestMetadata::Reconciliation(_)
            ) | (
                MethodName::RecordShaping,
                PersistedUserActionRequestMetadata::Shaping(_)
            )
        );
        if !origin_matches_source {
            return Err(StoreError::InvalidInput {
                detail: "user-action request origin metadata must match source_method".to_owned(),
            });
        }
        validate_identifier(
            "user_action_requests.source_idempotency_key",
            &input.source_idempotency_key,
        )?;
        validate_user_action_request_column_agreement(UserActionRequestColumnFacts {
            task_id: &input.task_id,
            change_unit_id: input.change_unit_id.as_deref(),
            request_json: &request_json,
            basis_json: &basis_json,
            required_for_json: &required_for_json,
            requested_at: &requested_at,
            expires_at: expires_at.as_deref(),
            action_kind: input.action_kind,
            basis_status: input.basis_status,
        })?;

        self.tx.execute(
            "INSERT INTO user_action_requests (
                project_id,
                user_action_request_id,
                task_id,
                change_unit_id,
                action_kind,
                request_json,
                basis_json,
                basis_status,
                required_for_json,
                requested_by_actor_source,
                source_method,
                source_idempotency_key,
                requested_at,
                expires_at,
                metadata_json
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
            )",
            params![
                self.project_id,
                input.user_action_request_id,
                input.task_id,
                input.change_unit_id,
                user_action_kind_as_str(input.action_kind),
                request_json,
                basis_json,
                user_action_basis_status_as_str(input.basis_status),
                required_for_json,
                requested_by_actor_source,
                source_method,
                input.source_idempotency_key,
                requested_at,
                expires_at,
                metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_user_action_resolution(
        &mut self,
        input: &UserActionResolutionInsert,
    ) -> StoreResult<()> {
        let resolution_json =
            encode_json_column("user_action_resolutions.resolution_json", &input.resolution)?;
        let resolved_by_actor_source = input.resolved_by_actor_source.to_canonical_string();
        let resolved_at = input.resolved_at.to_string();
        validate_identifier(
            "user_action_resolution_id",
            &input.user_action_resolution_id,
        )?;
        validate_identifier("user_action_request_id", &input.user_action_request_id)?;
        validate_channel_submission_id(&input.channel_submission_id).map_err(|error| {
            StoreError::InvalidInput {
                detail: error.to_string(),
            }
        })?;
        validate_persisted_user_action_resolution_json(
            "user_action_resolutions.resolution_json",
            &resolution_json,
        )?;
        if input.resolved_by_actor_source != ActorSource::LocalUser {
            return Err(StoreError::InvalidInput {
                detail: "user-action resolution actor must be local_user".to_owned(),
            });
        }
        validate_identifier(
            "resolved_verification_basis",
            input.resolved_verification_basis.as_str(),
        )?;
        validate_identifier("resolved_assurance_level", &input.resolved_assurance_level)?;
        validate_user_action_resolution_provenance(
            input.channel_kind,
            &resolved_by_actor_source,
            input.resolved_verification_basis,
            &input.resolved_assurance_level,
        )?;
        validate_timestamp("resolved_at", &resolved_at)?;
        validate_user_action_resolution_column_agreement(
            &resolution_json,
            input.action_kind,
            &input.user_action_resolution_id,
        )?;
        if let Some(request) =
            user_action_request_record(self.tx, self.project_id, &input.user_action_request_id)?
        {
            validate_user_action_resolution_timestamp_order_for_insert(&request, &resolved_at)?;
            let candidate = StoredUserActionResolution {
                project_id: self.project_id.to_owned(),
                user_action_resolution_id: input.user_action_resolution_id.clone(),
                user_action_request_id: input.user_action_request_id.clone(),
                action_kind: input.action_kind,
                channel_kind: input.channel_kind,
                channel_submission_id: input.channel_submission_id.clone(),
                resolution: input.resolution.clone(),
                resolved_by_actor_source: input.resolved_by_actor_source.clone(),
                resolved_verification_basis: input.resolved_verification_basis,
                resolved_assurance_level: input.resolved_assurance_level.clone(),
                resolved_at: input.resolved_at.clone(),
            };
            validate_user_action_request_resolution_pair(&request, &candidate).map_err(|_| {
                StoreError::InvalidInput {
                    detail:
                        "user-action resolution must exactly preserve its stored request authority"
                            .to_owned(),
                }
            })?;
        }

        self.tx.execute(
            "INSERT INTO user_action_resolutions (
                project_id,
                user_action_resolution_id,
                user_action_request_id,
                action_kind,
                channel_kind,
                channel_submission_id,
                resolution_json,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                self.project_id,
                input.user_action_resolution_id,
                input.user_action_request_id,
                user_action_kind_as_str(input.action_kind),
                user_action_channel_kind_as_str(input.channel_kind),
                input.channel_submission_id,
                resolution_json,
                resolved_by_actor_source,
                input.resolved_verification_basis.as_str(),
                input.resolved_assurance_level,
                resolved_at
            ],
        )?;
        Ok(())
    }

    fn update_user_action_basis(&mut self, input: &UserActionBasisUpdate) -> StoreResult<()> {
        validate_identifier("user_action_request_id", &input.user_action_request_id)?;
        let basis_json = encode_json_column(
            "user_action_requests.basis_json",
            &user_action_basis_with_status(&input.basis, input.basis_status),
        )?;
        let changed = self.tx.execute(
            "UPDATE user_action_requests
                SET basis_json = ?3,
                    basis_status = ?4
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![
                self.project_id,
                input.user_action_request_id,
                basis_json,
                user_action_basis_status_as_str(input.basis_status)
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "user-action basis update changed no rows".to_owned(),
            })
        }
    }

    fn mark_user_action_bases_status(
        &mut self,
        input: &UserActionBasisStatusMark,
    ) -> StoreResult<()> {
        let status = match input.basis_status {
            UserActionBasisStatus::Stale | UserActionBasisStatus::Superseded => {
                user_action_basis_status_as_str(input.basis_status)
            }
            UserActionBasisStatus::Current => {
                return Err(StoreError::InvalidInput {
                    detail: "selected user-action bases may only be marked stale or superseded"
                        .to_owned(),
                })
            }
        };

        for request_id in &input.user_action_request_ids {
            validate_identifier("user_action_request_id", request_id)?;
            let basis_json = self
                .tx
                .query_row(
                    "SELECT basis_json FROM user_action_requests
                      WHERE project_id = ?1 AND user_action_request_id = ?2",
                    params![self.project_id, request_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let Some(basis_json) = basis_json else {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "selected user-action basis request does not exist".to_owned(),
                });
            };
            let basis_json = user_action_basis_json_with_status(&basis_json, input.basis_status)?;
            let changed = self.tx.execute(
                "UPDATE user_action_requests
                    SET basis_status = ?3,
                        basis_json = ?4
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                params![self.project_id, request_id, status, basis_json],
            )?;
            if changed != 1 {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: format!(
                        "selected user-action basis status update changed {changed} rows"
                    ),
                });
            }
        }

        Ok(())
    }

    fn mark_user_actions_superseded_or_stale(
        &mut self,
        input: &UserActionInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if input.action_kinds.is_empty() {
            self.mark_user_actions_superseded_or_stale_for_kind(&input.task_id, None)?;
        } else {
            for action_kind in &input.action_kinds {
                self.mark_user_actions_superseded_or_stale_for_kind(
                    &input.task_id,
                    Some(*action_kind),
                )?;
            }
        }
        Ok(())
    }

    fn mark_user_actions_superseded_or_stale_for_kind(
        &mut self,
        task_id: &str,
        action_kind: Option<UserActionKind>,
    ) -> StoreResult<()> {
        let sql = if action_kind.is_some() {
            "SELECT
                a.user_action_request_id,
                a.basis_json,
                EXISTS (
                  SELECT 1 FROM user_action_resolutions AS r
                   WHERE r.project_id = a.project_id
                     AND r.user_action_request_id = a.user_action_request_id
                )
               FROM user_action_requests AS a
              WHERE a.project_id = ?1
                AND a.task_id = ?2
                AND a.action_kind = ?3
                AND a.basis_status = 'current'"
        } else {
            "SELECT
                a.user_action_request_id,
                a.basis_json,
                EXISTS (
                  SELECT 1 FROM user_action_resolutions AS r
                   WHERE r.project_id = a.project_id
                     AND r.user_action_request_id = a.user_action_request_id
                )
               FROM user_action_requests AS a
              WHERE a.project_id = ?1
                AND a.task_id = ?2
                AND (?3 IS NULL OR a.action_kind = ?3)
                AND a.basis_status = 'current'"
        };
        let kind = action_kind.map(user_action_kind_as_str);
        let rows = {
            let mut stmt = self.tx.prepare(sql)?;
            let mapped = stmt.query_map(params![self.project_id, task_id, kind], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };
        for (request_id, basis_json, has_resolution) in rows {
            let status = if has_resolution {
                UserActionBasisStatus::Stale
            } else {
                UserActionBasisStatus::Superseded
            };
            let basis_json = user_action_basis_json_with_status(&basis_json, status)?;
            self.tx.execute(
                "UPDATE user_action_requests
                    SET basis_status = ?3,
                        basis_json = ?4
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                params![
                    self.project_id,
                    request_id,
                    user_action_basis_status_as_str(status),
                    basis_json
                ],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod behavior_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_pipeline::mutations::with_empty_mutation_context;

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

    #[test]
    fn user_action_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            UserActionMutation::MarkBasesStatus(UserActionBasisStatusMark {
                user_action_request_ids: vec![" ".to_owned()],
                basis_status: UserActionBasisStatus::Stale,
            })
            .apply(context)
            .expect_err("blank user-action request id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
