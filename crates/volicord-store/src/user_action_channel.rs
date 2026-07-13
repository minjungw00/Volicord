use std::path::Path;

use chrono::Duration;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{
    UserActionChannelKind, UserActionStatus, UtcTimestamp,
    USER_ACTION_CHANNEL_TOKEN_MAX_TTL_SECONDS,
};

use crate::core_pipeline::{
    advance_project_utc_floor_tx, effective_user_action_record,
    project_current_utc_timestamp_for_conn, user_action_request_record,
};
use crate::{
    agent_connections::is_agent_connection_project_allowed,
    bootstrap::project_record_for_execution,
    schema::PROJECT_STATE_DATABASE_KIND,
    sqlite::{begin_immediate_transaction, open_project_state_database},
    StoreError, StoreResult,
};

const TOKEN_HASH_BYTES: usize = 32;
const TOKEN_HASH_HEX_LEN: usize = TOKEN_HASH_BYTES * 2;
const MAX_TOKEN_TEXT_BYTES: usize = 256;

/// Input for creating one pending user-action channel token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionChannelTokenCreate {
    pub token: String,
    pub project_id: String,
    pub channel_kind: UserActionChannelKind,
    pub connection_internal_id: String,
    pub user_action_request_id: String,
    pub capture_basis: String,
    pub created_metadata_json: String,
}

/// Input for checking one channel token against a selected endpoint context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionChannelTokenCheck {
    pub token: String,
    pub expected_project_id: String,
    pub expected_connection_internal_id: String,
    pub now: String,
}

/// Stored channel-token metadata. The raw token is never returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionChannelTokenRecord {
    pub token_hash: String,
    pub project_id: String,
    pub channel_kind: UserActionChannelKind,
    pub connection_internal_id: String,
    pub user_action_request_id: String,
    pub capture_basis: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_metadata_json: String,
    pub completion_metadata_json: String,
}

/// Non-recording validation failure for a presented channel token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserActionChannelTokenRejection {
    Invalid,
    Expired(UserActionChannelTokenRecord),
    Consumed(UserActionChannelTokenRecord),
    WrongConnection {
        expected_connection_internal_id: String,
        actual_connection_internal_id: String,
    },
}

/// Validation result for a user-action channel token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserActionChannelTokenValidation {
    Valid(UserActionChannelTokenRecord),
    Rejected(UserActionChannelTokenRejection),
}

/// Creates one pending channel-token record and stores only its hash.
pub fn create_user_action_channel_token(
    runtime_home: impl AsRef<Path>,
    input: UserActionChannelTokenCreate,
) -> StoreResult<UserActionChannelTokenRecord> {
    validate_token_create(&input)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    if !is_agent_connection_project_allowed(
        &runtime_home,
        &input.connection_internal_id,
        &input.project_id,
    )? {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{}:{}", input.connection_internal_id, input.project_id),
        });
    }
    let token_hash = user_action_channel_token_hash(&input.token)?;
    let mut conn = open_project_state_for_user_action_channel(&runtime_home, &input.project_id)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let created_at = project_current_utc_timestamp_for_conn(&tx, &input.project_id, None)?;
    let ttl_expires_at = created_at
        .checked_add(Duration::seconds(
            i64::try_from(USER_ACTION_CHANNEL_TOKEN_MAX_TTL_SECONDS)
                .expect("owner token TTL fits in i64"),
        ))
        .map_err(|_| StoreError::InvalidInput {
            detail: "user-action channel token expiration exceeds the supported canonical RFC 3339 range"
                .to_owned(),
        })?;
    let created_at = created_at.to_string();
    let ttl_expires_at = ttl_expires_at.to_string();
    let request_expires_at = require_pending_user_action_request_tx(
        &tx,
        &input.project_id,
        &input.user_action_request_id,
        &created_at,
    )?;
    let expires_at = earlier_expiry(ttl_expires_at, request_expires_at)?;
    let created_at_timestamp =
        UtcTimestamp::parse(&created_at).map_err(|_| StoreError::SchemaInvariant {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            detail: "channel-token issue produced an invalid Core current UTC timestamp".to_owned(),
        })?;
    advance_project_utc_floor_tx(&tx, &input.project_id, &created_at_timestamp)?;
    tx.execute(
        "INSERT INTO user_action_channel_tokens (
            project_id,
            token_hash,
            channel_kind,
            connection_internal_id,
            user_action_request_id,
            capture_basis,
            status,
            created_at,
            expires_at,
            consumed_at,
            completed_at,
            created_metadata_json,
            completion_metadata_json
        ) VALUES (?1, ?2, 'local_web_consent', ?3, ?4, ?5, 'pending', ?6, ?7, NULL, NULL, ?8, '{}')",
        params![
            input.project_id,
            token_hash,
            input.connection_internal_id,
            input.user_action_request_id,
            input.capture_basis,
            created_at,
            expires_at,
            input.created_metadata_json
        ],
    )?;
    let record = user_action_channel_token_record_tx(&tx, &token_hash)?.ok_or_else(|| {
        StoreError::SchemaInvariant {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            detail: "inserted user_action_channel_tokens row cannot be read".to_owned(),
        }
    })?;
    validate_persisted_user_action_channel_token_window(
        &tx,
        &record.project_id,
        &record.user_action_request_id,
        &record.created_at,
        &record.expires_at,
    )?;
    tx.commit()?;
    Ok(record)
}

/// Validates one channel token without consuming it.
pub fn validate_user_action_channel_token(
    runtime_home: impl AsRef<Path>,
    input: UserActionChannelTokenCheck,
) -> StoreResult<UserActionChannelTokenValidation> {
    validate_token_check(&input)?;
    let now = UtcTimestamp::parse(&input.now).map_err(|_| StoreError::InvalidInput {
        detail: "now must be a valid RFC 3339 timestamp".to_owned(),
    })?;
    let Some(token_hash) = user_action_channel_token_hash_for_lookup(&input.token) else {
        return Ok(UserActionChannelTokenValidation::Rejected(
            UserActionChannelTokenRejection::Invalid,
        ));
    };
    let mut conn = match open_project_state_for_user_action_channel(
        runtime_home,
        &input.expected_project_id,
    ) {
        Ok(conn) => conn,
        Err(StoreError::NotFound { .. }) => {
            return Ok(UserActionChannelTokenValidation::Rejected(
                UserActionChannelTokenRejection::Invalid,
            ));
        }
        Err(error) => return Err(error),
    };
    let tx = begin_immediate_transaction(&mut conn)?;
    expire_pending_tokens_tx(&tx, &input.expected_project_id, &input.now)?;
    let validation = validate_record_for_context(
        &tx,
        &token_hash,
        &input.expected_project_id,
        &input.expected_connection_internal_id,
        &now,
    )?;
    tx.commit()?;
    Ok(validation)
}

/// Marks pending channel tokens expired at or before `now`.
pub fn expire_user_action_channel_tokens(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    now: &str,
) -> StoreResult<usize> {
    validate_identifier("project_id", project_id)?;
    validate_timestamp_text("now", now)?;
    let mut conn = open_project_state_for_user_action_channel(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let changed = expire_pending_tokens_tx(&tx, project_id, now)?;
    tx.commit()?;
    Ok(changed)
}

/// Returns the project-state clock in the public timestamp shape used by channel tokens.
pub fn user_action_channel_current_timestamp(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<String> {
    validate_identifier("project_id", project_id)?;
    let conn = open_project_state_for_user_action_channel(runtime_home, project_id)?;
    project_current_utc_timestamp_for_conn(&conn, project_id, None)
        .map(|timestamp| timestamp.to_string())
}

/// Computes the stored hash for a raw channel token.
pub fn user_action_channel_token_hash(token: &str) -> StoreResult<String> {
    validate_token_text("token", token)?;
    Ok(user_action_channel_token_hash_unchecked(token))
}

fn user_action_channel_token_hash_for_lookup(token: &str) -> Option<String> {
    validate_token_text("token", token)
        .is_ok()
        .then(|| user_action_channel_token_hash_unchecked(token))
}

fn user_action_channel_token_hash_unchecked(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"volicord.user_action_channel_token.v1\0");
    hasher.update(token.as_bytes());
    let hash = hex_encode(&hasher.finalize());
    debug_assert_eq!(hash.len(), TOKEN_HASH_HEX_LEN);
    hash
}

fn expire_pending_tokens_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    now: &str,
) -> StoreResult<usize> {
    validate_identifier("project_id", project_id)?;
    validate_timestamp_text("now", now)?;
    let now = UtcTimestamp::parse(now).map_err(|_| StoreError::InvalidInput {
        detail: "now must be a valid RFC 3339 timestamp".to_owned(),
    })?;
    let candidates = {
        let mut stmt = tx.prepare(
            "SELECT
                token.token_hash,
                token.user_action_request_id,
                token.created_at,
                token.expires_at
               FROM user_action_channel_tokens AS token
              WHERE token.project_id = ?1
                AND token.status = 'pending'",
        )?;
        let rows = stmt.query_map([project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // SQLite's julianday() rounds away sub-millisecond precision. Expiration is
    // an authority decision, so validate every candidate and compare the exact
    // RFC 3339 instants in Rust before updating by stable identity and status.
    let mut token_hashes_to_expire = Vec::new();
    for (token_hash, request_id, created_at, expires_at) in candidates {
        let (created_at, expires_at) = validate_persisted_user_action_channel_token_window(
            tx,
            project_id,
            &request_id,
            &created_at,
            &expires_at,
        )?;
        if now < created_at {
            continue;
        }

        let request_is_pending = effective_user_action_record(tx, project_id, &request_id, &now)?
            .is_some_and(|record| record.status == UserActionStatus::Pending);
        if now >= expires_at || !request_is_pending {
            token_hashes_to_expire.push(token_hash);
        }
    }

    let mut changed = 0;
    for token_hash in token_hashes_to_expire {
        changed += tx.execute(
            "UPDATE user_action_channel_tokens
                SET status = 'expired'
              WHERE project_id = ?1
                AND token_hash = ?2
                AND status = 'pending'",
            params![project_id, token_hash],
        )?;
    }
    Ok(changed)
}

fn validate_record_for_context(
    tx: &Transaction<'_>,
    token_hash: &str,
    expected_project_id: &str,
    expected_connection_internal_id: &str,
    now: &UtcTimestamp,
) -> StoreResult<UserActionChannelTokenValidation> {
    let Some(record) = user_action_channel_token_record_tx(tx, token_hash)? else {
        return Ok(UserActionChannelTokenValidation::Rejected(
            UserActionChannelTokenRejection::Invalid,
        ));
    };
    if record.project_id != expected_project_id {
        return Ok(UserActionChannelTokenValidation::Rejected(
            UserActionChannelTokenRejection::Invalid,
        ));
    }
    if record.connection_internal_id != expected_connection_internal_id {
        return Ok(UserActionChannelTokenValidation::Rejected(
            UserActionChannelTokenRejection::WrongConnection {
                expected_connection_internal_id: expected_connection_internal_id.to_owned(),
                actual_connection_internal_id: record.connection_internal_id,
            },
        ));
    }
    let (created_at, expires_at) = validate_persisted_user_action_channel_token_window(
        tx,
        &record.project_id,
        &record.user_action_request_id,
        &record.created_at,
        &record.expires_at,
    )?;
    match record.status.as_str() {
        "pending" => {
            if now < &created_at {
                return Ok(UserActionChannelTokenValidation::Rejected(
                    UserActionChannelTokenRejection::Invalid,
                ));
            }
            if now >= &expires_at {
                return Ok(UserActionChannelTokenValidation::Rejected(
                    UserActionChannelTokenRejection::Expired(record),
                ));
            }
            Ok(UserActionChannelTokenValidation::Valid(record))
        }
        "expired" => Ok(UserActionChannelTokenValidation::Rejected(
            UserActionChannelTokenRejection::Expired(record),
        )),
        "consumed" => Ok(UserActionChannelTokenValidation::Rejected(
            UserActionChannelTokenRejection::Consumed(record),
        )),
        _ => Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.status",
        }),
    }
}

pub(crate) fn user_action_channel_token_record_tx(
    conn: &Connection,
    token_hash: &str,
) -> StoreResult<Option<UserActionChannelTokenRecord>> {
    let raw = conn
        .query_row(
            "SELECT
                token_hash,
                project_id,
                channel_kind,
                connection_internal_id,
                user_action_request_id,
                capture_basis,
                status,
                created_at,
                expires_at,
                consumed_at,
                completed_at,
                created_metadata_json,
                completion_metadata_json
             FROM user_action_channel_tokens
             WHERE token_hash = ?1",
            [token_hash],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()?;
    let Some((
        token_hash,
        project_id,
        channel_kind,
        connection_internal_id,
        user_action_request_id,
        capture_basis,
        status,
        created_at,
        expires_at,
        consumed_at,
        completed_at,
        created_metadata_json,
        completion_metadata_json,
    )) = raw
    else {
        return Ok(None);
    };
    let channel_kind: UserActionChannelKind = serde_json::from_value(Value::String(channel_kind))
        .map_err(|_| StoreError::CorruptStoredValue {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        field: "user_action_channel_tokens.channel_kind",
    })?;
    if capture_basis != channel_kind.verification_basis() {
        return Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.capture_basis",
        });
    }
    Ok(Some(UserActionChannelTokenRecord {
        token_hash,
        project_id,
        channel_kind,
        connection_internal_id,
        user_action_request_id,
        capture_basis,
        status,
        created_at,
        expires_at,
        consumed_at,
        completed_at,
        created_metadata_json,
        completion_metadata_json,
    }))
}

fn open_project_state_for_user_action_channel(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Connection> {
    validate_identifier("project_id", project_id)?;
    let project = project_record_for_execution(runtime_home, project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        }
    })?;
    open_project_state_database(&project.state_db_path)
}

fn require_pending_user_action_request_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    request_id: &str,
    now: &str,
) -> StoreResult<Option<String>> {
    let now = UtcTimestamp::parse(now).map_err(|_| StoreError::SchemaInvariant {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        detail: "SQLite generated an invalid user-action channel timestamp".to_owned(),
    })?;
    let record =
        effective_user_action_record(tx, project_id, request_id, &now)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "user_action_request",
                id: request_id.to_owned(),
            }
        })?;
    if record.status == UserActionStatus::Pending {
        Ok(record.request.expires_at)
    } else {
        Err(StoreError::Conflict {
            entity: "user_action_request",
            id: request_id.to_owned(),
            detail: "channel-token issue requires an effective pending user action".to_owned(),
        })
    }
}

fn earlier_expiry(
    ttl_expires_at: String,
    request_expires_at: Option<String>,
) -> StoreResult<String> {
    let ttl = volicord_types::UtcTimestamp::parse(&ttl_expires_at).map_err(|_| {
        StoreError::SchemaInvariant {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            detail: "SQLite generated an invalid user-action channel-token expiry".to_owned(),
        }
    })?;
    let Some(request_expires_at) = request_expires_at else {
        return Ok(ttl_expires_at);
    };
    let request = volicord_types::UtcTimestamp::parse(&request_expires_at).map_err(|_| {
        StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_requests.expires_at",
        }
    })?;
    Ok(if request <= ttl {
        request_expires_at
    } else {
        ttl_expires_at
    })
}

pub(crate) fn validate_persisted_user_action_channel_token_window(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
    created_at: &str,
    expires_at: &str,
) -> StoreResult<(UtcTimestamp, UtcTimestamp)> {
    let created_at =
        parse_stored_channel_timestamp("user_action_channel_tokens.created_at", created_at)?;
    let expires_at =
        parse_stored_channel_timestamp("user_action_channel_tokens.expires_at", expires_at)?;
    let max_expires_at = created_at
        .checked_add(Duration::seconds(
            i64::try_from(USER_ACTION_CHANNEL_TOKEN_MAX_TTL_SECONDS)
                .expect("owner token TTL fits in i64"),
        ))
        .map_err(|_| StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.expires_at",
        })?;
    if expires_at <= created_at || expires_at > max_expires_at {
        return Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.expires_at",
        });
    }
    let request = user_action_request_record(conn, project_id, user_action_request_id)?
        .ok_or_else(|| StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.user_action_request_id",
        })?;
    let requested_at =
        parse_stored_channel_timestamp("user_action_requests.requested_at", &request.requested_at)?;
    if created_at < requested_at {
        return Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.created_at",
        });
    }
    let expected_expires_at = if let Some(request_expires_at) = request.expires_at.as_deref() {
        let request_expires_at =
            parse_stored_channel_timestamp("user_action_requests.expires_at", request_expires_at)?;
        request_expires_at.min(max_expires_at)
    } else {
        max_expires_at
    };
    if expires_at != expected_expires_at {
        return Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "user_action_channel_tokens.expires_at",
        });
    }
    Ok((created_at, expires_at))
}

fn validate_token_create(input: &UserActionChannelTokenCreate) -> StoreResult<()> {
    validate_token_text("token", &input.token)?;
    validate_identifier("project_id", &input.project_id)?;
    if input.channel_kind != UserActionChannelKind::LocalWebConsent {
        return Err(StoreError::InvalidInput {
            detail: "channel_kind must be local_web_consent for a bearer channel token".to_owned(),
        });
    }
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("user_action_request_id", &input.user_action_request_id)?;
    validate_identifier("capture_basis", &input.capture_basis)?;
    if input.capture_basis != input.channel_kind.verification_basis() {
        return Err(StoreError::InvalidInput {
            detail: "capture_basis must match the local-web User Channel".to_owned(),
        });
    }
    validate_json_object("created_metadata_json", &input.created_metadata_json)
}

fn validate_token_check(input: &UserActionChannelTokenCheck) -> StoreResult<()> {
    validate_identifier("expected_project_id", &input.expected_project_id)?;
    validate_identifier(
        "expected_connection_internal_id",
        &input.expected_connection_internal_id,
    )?;
    validate_timestamp_text("now", &input.now)
}

fn validate_token_text(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if value.len() > MAX_TOKEN_TEXT_BYTES
        || value.chars().any(|character| {
            character.is_ascii_whitespace() || character == '\0' || !character.is_ascii()
        })
    {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be visible ASCII without whitespace"),
        });
    }
    Ok(())
}

fn validate_timestamp_text(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if volicord_types::UtcTimestamp::parse(value)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| volicord_types::UtcTimestampParseError)
        })
        .is_err()
    {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be a valid RFC 3339 timestamp"),
        });
    }
    Ok(())
}

fn parse_stored_channel_timestamp(field: &'static str, value: &str) -> StoreResult<UtcTimestamp> {
    let timestamp = UtcTimestamp::parse(value).map_err(|_| StoreError::CorruptStoredValue {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        field,
    })?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field,
        })?;
    if value != timestamp.to_canonical_string() {
        return Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field,
        });
    }
    Ok(timestamp)
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL bytes"),
        })
    } else {
        Ok(())
    }
}

fn validate_json_object(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON object text: {error}"),
    })?;
    if value.is_object() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        })
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rusqlite::params;
    use volicord_test_support::core_fixtures::CoreFixture;

    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn channel_token_stores_only_hash_and_validates_context() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-create")?;
        insert_pending_action(&fixture, "action_channel")?;
        let before_state_version = fixture.conn()?.query_row(
            "SELECT state_version FROM project_state WHERE project_id = ?1",
            [fixture.project_id()],
            |row| row.get::<_, i64>(0),
        )?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;

        assert_eq!(record.token_hash, user_action_channel_token_hash(TOKEN)?);
        assert_eq!(
            record.token_hash,
            "dd1f3e8b6eff49476f48c8dfa9e412d85ac1cd4d684ea06ee066c74f93d6d61a"
        );
        assert_ne!(record.token_hash, TOKEN);
        assert_eq!(record.channel_kind, UserActionChannelKind::LocalWebConsent);
        let plaintext_matches = fixture.conn()?.query_row(
            "SELECT COUNT(*)
               FROM user_action_channel_tokens
              WHERE token_hash = ?1
                 OR created_metadata_json LIKE ?2
                 OR completion_metadata_json LIKE ?2",
            params![TOKEN, format!("%{TOKEN}%")],
            |row| row.get::<_, i64>(0),
        )?;
        assert_eq!(plaintext_matches, 0);
        let (state_version, updated_at) = fixture.conn()?.query_row(
            "SELECT state_version, updated_at FROM project_state WHERE project_id = ?1",
            [fixture.project_id()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;
        assert_eq!(state_version, before_state_version);
        assert_eq!(updated_at, record.created_at);

        let checked = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, &record.created_at),
        )?;
        assert!(matches!(
            checked,
            UserActionChannelTokenValidation::Valid(_)
        ));
        Ok(())
    }

    #[test]
    fn channel_token_rejects_validation_before_creation_without_consuming_state(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-before-creation")?;
        insert_pending_action(&fixture, "action_channel")?;
        let created_floor = "2999-07-13T12:34:56.789Z";
        fixture.conn()?.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![fixture.project_id(), created_floor],
        )?;
        fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = ?2
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            params![fixture.project_id(), created_floor],
        )?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        assert_eq!(record.created_at, created_floor);
        let (before_state_version, before_floor) = fixture.conn()?.query_row(
            "SELECT state_version, updated_at
               FROM project_state
              WHERE project_id = ?1",
            [fixture.project_id()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;

        let validation = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "2999-07-13T12:34:56.788Z"),
        )?;
        assert!(matches!(
            validation,
            UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Invalid)
        ));
        let (status, consumed_at) = fixture.conn()?.query_row(
            "SELECT status, consumed_at
               FROM user_action_channel_tokens
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )?;
        assert_eq!(status, "pending");
        assert_eq!(consumed_at, None);
        let (after_state_version, after_floor) = fixture.conn()?.query_row(
            "SELECT state_version, updated_at
               FROM project_state
              WHERE project_id = ?1",
            [fixture.project_id()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;
        assert_eq!(after_state_version, before_state_version);
        assert_eq!(after_floor, before_floor);
        assert_eq!(after_floor, created_floor);
        Ok(())
    }

    #[test]
    fn channel_token_uses_persisted_clock_floor_at_request_lower_bound(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-clock-floor")?;
        insert_pending_action(&fixture, "action_channel")?;
        let floor = "2999-07-13T12:34:56.789Z";
        fixture.conn()?.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![fixture.project_id(), floor],
        )?;
        fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = ?2
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            params![fixture.project_id(), floor],
        )?;

        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        assert_eq!(record.created_at, floor);
        let expected_expiry = UtcTimestamp::from_datetime(
            *UtcTimestamp::parse(floor)?.as_datetime() + Duration::seconds(600),
        );
        assert_eq!(record.expires_at, expected_expiry.to_string());
        assert!(matches!(
            validate_user_action_channel_token(
                fixture.runtime_home_path(),
                check_input(&fixture, floor),
            )?,
            UserActionChannelTokenValidation::Valid(_)
        ));
        assert!(matches!(
            validate_user_action_channel_token(
                fixture.runtime_home_path(),
                check_input(&fixture, &record.expires_at),
            )?,
            UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Expired(_))
        ));

        let future_fixture = CoreFixture::new("store-user-action-channel-clock-floor-future")?;
        insert_pending_action(&future_fixture, "action_channel")?;
        future_fixture.conn()?.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![future_fixture.project_id(), floor],
        )?;
        let requested_at = "2999-07-13T12:34:56.790Z";
        future_fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = ?2
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            params![future_fixture.project_id(), requested_at],
        )?;
        let error = create_user_action_channel_token(
            future_fixture.runtime_home_path(),
            create_input(&future_fixture, "action_channel"),
        )
        .expect_err("request timestamp above the persisted floor must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "requested_at",
                ..
            }
        ));
        let (stored_requested_at, token_count) = future_fixture.conn()?.query_row(
            "SELECT r.requested_at,
                    (SELECT COUNT(*) FROM user_action_channel_tokens t
                      WHERE t.project_id = r.project_id)
               FROM user_action_requests r
              WHERE r.project_id = ?1
                AND r.user_action_request_id = 'action_channel'",
            [future_fixture.project_id()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )?;
        assert_eq!(stored_requested_at, requested_at);
        assert_eq!(token_count, 0);
        Ok(())
    }

    #[test]
    fn channel_token_ttl_overflow_rejects_without_effects() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-ttl-overflow")?;
        insert_pending_action(&fixture, "action_channel")?;
        let floor = "9999-12-31T23:59:59Z";
        fixture.conn()?.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![fixture.project_id(), floor],
        )?;
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at,
                    (SELECT COUNT(*) FROM user_action_channel_tokens WHERE project_id = ?1)
               FROM project_state
              WHERE project_id = ?1",
            [fixture.project_id()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;

        let error = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )
        .expect_err("out-of-range token expiry must fail closed");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        let after = fixture.conn()?.query_row(
            "SELECT state_version, updated_at,
                    (SELECT COUNT(*) FROM user_action_channel_tokens WHERE project_id = ?1)
               FROM project_state
              WHERE project_id = ?1",
            [fixture.project_id()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn channel_token_rejects_out_of_canonical_range_times_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-range")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        let out_of_range = chrono::DateTime::<chrono::Utc>::MAX_UTC.to_rfc3339();
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at,
                    (SELECT status FROM user_action_channel_tokens
                      WHERE project_id = ?1 AND token_hash = ?2)
               FROM project_state
              WHERE project_id = ?1",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let error = expire_user_action_channel_tokens(
            fixture.runtime_home_path(),
            fixture.project_id(),
            &out_of_range,
        )
        .expect_err("out-of-canonical-range now must reject before token cleanup");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        let after_invalid_now = fixture.conn()?.query_row(
            "SELECT state_version, updated_at,
                    (SELECT status FROM user_action_channel_tokens
                      WHERE project_id = ?1 AND token_hash = ?2)
               FROM project_state
              WHERE project_id = ?1",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after_invalid_now, before);

        fixture.conn()?.execute(
            "UPDATE user_action_channel_tokens
                SET created_at = ?3
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash, out_of_range],
        )?;
        let before_corrupt_read = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "9999-12-31T23:59:59Z"),
        )
        .expect_err("out-of-range stored token time must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptStoredValue {
                field: "user_action_channel_tokens.created_at",
                ..
            }
        ));
        let after_corrupt_read = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after_corrupt_read, before_corrupt_read);
        assert_eq!(after_corrupt_read.2, "pending");
        Ok(())
    }

    #[test]
    fn expired_channel_token_is_rejected_and_marked_expired() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-expired")?;
        insert_pending_action(&fixture, "action_channel")?;
        create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        let checked = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "9999-01-01T00:00:00Z"),
        )?;
        assert!(matches!(
            checked,
            UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Expired(_))
        ));
        Ok(())
    }

    #[test]
    fn channel_token_expiry_preserves_subsecond_request_deadline() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-request-deadline")?;
        insert_pending_action(&fixture, "action_channel")?;
        let request_expires_at = fixture.conn()?.query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+60.9 seconds')",
            [],
            |row| row.get::<_, String>(0),
        )?;
        set_pending_action_expiry(&fixture, "action_channel", Some(&request_expires_at))?;

        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;

        assert_eq!(record.expires_at, request_expires_at);
        Ok(())
    }

    #[test]
    fn equal_request_and_ttl_expiry_preserves_request_deadline() -> Result<(), Box<dyn Error>> {
        let ttl_expiry = "2026-07-13T00:10:00.000Z".to_owned();
        let request_expiry = "2026-07-13T00:10:00Z".to_owned();
        assert_eq!(
            earlier_expiry(ttl_expiry, Some(request_expiry.clone()))?,
            request_expiry
        );
        Ok(())
    }

    #[test]
    fn channel_token_validation_expires_token_when_request_is_resolved(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-resolved-request")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        insert_resolution(&fixture, "action_channel")?;

        assert_expired(&fixture, &record.created_at)?;
        Ok(())
    }

    #[test]
    fn channel_token_validation_expires_token_when_request_basis_is_stale(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-stale-request")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        set_pending_action_basis_status(&fixture, "action_channel", "stale")?;

        assert_expired(&fixture, &record.created_at)?;
        Ok(())
    }

    #[test]
    fn channel_token_validation_rejects_token_past_tampered_request_deadline_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-expired-request")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        set_pending_action_expiry(&fixture, "action_channel", Some(&record.created_at))?;
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, &record.created_at),
        )
        .expect_err("token expiry after its immutable request deadline must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptStoredValue {
                field: "user_action_channel_tokens.expires_at",
                ..
            }
        ));
        let after = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn channel_token_validation_rejects_noncanonical_stored_time_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-noncanonical-time")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        fixture.conn()?.execute(
            "UPDATE user_action_channel_tokens
                SET created_at = '2026-07-13T00:00:00.000Z'
              WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "2026-07-13T00:00:00Z"),
        )
        .expect_err("noncanonical stored timestamps must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptStoredValue {
                field: "user_action_channel_tokens.created_at",
                ..
            }
        ));
        let after = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn channel_token_expiry_uses_exact_submillisecond_half_open_boundary(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-submillisecond-expiry")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        fixture.conn()?.execute(
            "UPDATE user_action_channel_tokens
                SET created_at = '2026-07-13T00:00:00.000000001Z',
                    expires_at = '2026-07-13T00:10:00.000000001Z'
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
        )?;
        fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = '2026-07-13T00:00:00.000000001Z'
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            [fixture.project_id()],
        )?;
        set_pending_action_expiry(
            &fixture,
            "action_channel",
            Some("2026-07-13T00:10:00.000000001Z"),
        )?;
        let before_state = fixture.conn()?.query_row(
            "SELECT state_version, updated_at
               FROM project_state
              WHERE project_id = ?1",
            [fixture.project_id()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;

        let just_before = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "2026-07-13T00:10:00.000000000Z"),
        )?;
        assert!(matches!(
            just_before,
            UserActionChannelTokenValidation::Valid(_)
        ));
        let status_before: String = fixture.conn()?.query_row(
            "SELECT status FROM user_action_channel_tokens
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| row.get(0),
        )?;
        assert_eq!(status_before, "pending");

        let at_expiry = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "2026-07-13T00:10:00.000000001Z"),
        )?;
        assert!(matches!(
            at_expiry,
            UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Expired(_))
        ));
        let after_expiry = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "2026-07-13T00:10:00.000000002Z"),
        )?;
        assert!(matches!(
            after_expiry,
            UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Expired(_))
        ));
        let after_state = fixture.conn()?.query_row(
            "SELECT state_version, updated_at
               FROM project_state
              WHERE project_id = ?1",
            [fixture.project_id()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;
        assert_eq!(after_state, before_state);
        Ok(())
    }

    #[test]
    fn channel_token_validation_rejects_window_over_ten_minutes_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-window-over-max")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        let extended_expiry = UtcTimestamp::parse(&record.created_at)?
            .checked_add(Duration::seconds(600) + Duration::nanoseconds(1))?
            .to_string();
        fixture.conn()?.execute(
            "UPDATE user_action_channel_tokens
                SET expires_at = ?3
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash, extended_expiry],
        )?;
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, &record.created_at),
        )
        .expect_err("token window over the owner maximum must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptStoredValue {
                field: "user_action_channel_tokens.expires_at",
                ..
            }
        ));
        let after = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn channel_token_validation_rejects_shortened_window_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-window-shortened")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        let shortened_expiry = UtcTimestamp::parse(&record.created_at)?
            .checked_add(Duration::seconds(599))?
            .to_canonical_string();
        fixture.conn()?.execute(
            "UPDATE user_action_channel_tokens
                SET expires_at = ?3
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash, shortened_expiry],
        )?;
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, &record.created_at),
        )
        .expect_err("a shortened owner token window must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptStoredValue {
                field: "user_action_channel_tokens.expires_at",
                ..
            }
        ));
        let after = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn channel_token_rejects_noncanonical_capture_basis() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-capture-basis")?;
        insert_pending_action(&fixture, "action_channel")?;
        let mut input = create_input(&fixture, "action_channel");
        input.capture_basis = "local_web".to_owned();
        let error = create_user_action_channel_token(fixture.runtime_home_path(), input)
            .expect_err("noncanonical capture basis must reject token creation");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        Ok(())
    }

    #[test]
    fn channel_token_requires_effective_pending_request() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-nonpending")?;
        insert_pending_action(&fixture, "action_channel")?;
        set_pending_action_basis_status(&fixture, "action_channel", "superseded")?;
        let error = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )
        .expect_err("non-pending user action must reject token issue");
        assert!(matches!(error, StoreError::Conflict { .. }));
        Ok(())
    }

    #[test]
    fn channel_token_request_lower_bound_errors_roll_back_without_partial_state(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-request-lower-bound")?;
        insert_pending_action(&fixture, "action_channel")?;
        fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = '9999-01-01T00:00:00Z'
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            [fixture.project_id()],
        )?;

        let error = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )
        .expect_err("token issue before requested_at must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "requested_at",
                ..
            }
        ));
        let token_count = fixture.conn()?.query_row(
            "SELECT COUNT(*) FROM user_action_channel_tokens WHERE project_id = ?1",
            [fixture.project_id()],
            |row| row.get::<_, i64>(0),
        )?;
        assert_eq!(
            token_count, 0,
            "failed issue must roll back token insertion"
        );

        fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = '2026-01-01T00:00:00Z'
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            [fixture.project_id()],
        )?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        fixture.conn()?.execute(
            "UPDATE user_action_requests
                SET requested_at = '9999-01-01T00:00:00Z'
              WHERE project_id = ?1
                AND user_action_request_id = 'action_channel'",
            [fixture.project_id()],
        )?;

        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, &record.created_at),
        )
        .expect_err("token validation before requested_at must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptStoredValue {
                field: "user_action_channel_tokens.created_at",
                ..
            }
        ));
        let status = fixture.conn()?.query_row(
            "SELECT status
               FROM user_action_channel_tokens
              WHERE project_id = ?1
                AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| row.get::<_, String>(0),
        )?;
        assert_eq!(status, "pending", "failed validation must roll back expiry");
        Ok(())
    }

    #[test]
    fn channel_token_cleanup_rolls_back_on_invalid_request_expiry_order(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-user-action-channel-invalid-request-expiry")?;
        insert_pending_action(&fixture, "action_channel")?;
        let record = create_user_action_channel_token(
            fixture.runtime_home_path(),
            create_input(&fixture, "action_channel"),
        )?;
        set_pending_action_expiry(
            &fixture,
            "action_channel",
            Some("2025-12-31T23:59:59.999999999Z"),
        )?;
        let before = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let error = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(&fixture, &record.created_at),
        )
        .expect_err("invalid request timestamp order must abort token cleanup");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "expires_at",
                ..
            }
        ));
        let after = fixture.conn()?.query_row(
            "SELECT state_version, updated_at, status
               FROM project_state
               JOIN user_action_channel_tokens USING (project_id)
              WHERE project_id = ?1 AND token_hash = ?2",
            params![fixture.project_id(), record.token_hash],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        assert_eq!(after, before);
        assert_eq!(after.2, "pending");
        Ok(())
    }

    fn insert_pending_action(
        fixture: &CoreFixture,
        request_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = fixture.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO tasks (
                project_id, task_id, created_by_actor_source, mode, work_phase,
                acceptance_policy, acceptance_policy_reason, carry_forward_json,
                lifecycle_phase, created_at, updated_at
            ) VALUES (
                ?1, 'task_user_action_channel', ?2, 'work', 'shaping',
                'required', 'Channel fixture requires an action.', '[]',
                'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
            )",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        let request_json = serde_json::json!({
            "body": {
                "action_type": "choice",
                "judgment_kind": "product_decision",
                "presentation": "short",
                "question": "Choose the current product direction.",
                "options": [{
                    "option_id": "accept",
                    "label": "Accept",
                    "description": "Accept the current direction.",
                    "consequence": "The work may continue.",
                    "machine_action": "accept",
                    "resolution_outcome": "accepted",
                    "is_default": true
                }],
                "context": {
                    "summary": "A bounded choice is required.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": []
                },
                "affected_refs": [],
                "sensitive_action_scope": null
            },
            "required_for": ["informational"],
            "expires_at": null
        })
        .to_string();
        let basis_json = serde_json::json!({
            "action_type": "choice",
            "coordinates": {
                "task_id": "task_user_action_channel",
                "change_unit_id": null,
                "scope_revision": 0,
                "baseline_ref": null,
                "created_at_state_version": 0,
                "compatibility_status": "current"
            },
            "close_basis_revision": null,
            "result_refs": [],
            "residual_risk_ids": [],
            "sensitive_action_scope": null
        })
        .to_string();
        conn.execute(
            "INSERT INTO user_action_requests (
                project_id, user_action_request_id, task_id, action_kind,
                request_json, basis_json, required_for_json,
                requested_by_actor_source, source_method,
                source_idempotency_key, requested_at
            ) VALUES (
                ?1, ?2, 'task_user_action_channel', 'product_decision',
                ?3, ?4, '[\"informational\"]', ?5, 'volicord.request_user_action',
                ?6, '2026-01-01T00:00:00Z'
            )",
            params![
                fixture.project_id(),
                request_id,
                request_json,
                basis_json,
                fixture.actor_source(),
                format!("idem_{request_id}")
            ],
        )?;
        Ok(())
    }

    fn insert_resolution(fixture: &CoreFixture, request_id: &str) -> Result<(), Box<dyn Error>> {
        fixture.conn()?.execute(
            "INSERT INTO user_action_resolutions (
                project_id, user_action_resolution_id, user_action_request_id,
                action_kind, channel_kind, channel_submission_id, resolution_json,
                resolved_by_actor_source, resolved_verification_basis,
                resolved_assurance_level, resolved_at
            ) VALUES (
                ?1, 'ures_channel', ?2, 'product_decision', 'cli',
                'submission_channel', ?3, 'local_user',
                'cli_direct_user_channel', 'verified', '2026-01-01T00:00:00Z'
            )",
            params![
                fixture.project_id(),
                request_id,
                serde_json::json!({
                    "resolution_type": "choice",
                    "selected_option_id": "accept",
                    "machine_action": "accept",
                    "resolution_outcome": "accepted",
                    "note": null,
                    "accepted_risk_ids": []
                })
                .to_string()
            ],
        )?;
        Ok(())
    }

    fn set_pending_action_expiry(
        fixture: &CoreFixture,
        request_id: &str,
        expires_at: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        let conn = fixture.conn()?;
        let request_json: String = conn.query_row(
            "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1 AND user_action_request_id = ?2",
            params![fixture.project_id(), request_id],
            |row| row.get(0),
        )?;
        let mut request_json = serde_json::from_str::<Value>(&request_json)?;
        request_json["expires_at"] =
            expires_at.map_or(Value::Null, |value| Value::String(value.to_owned()));
        conn.execute(
            "UPDATE user_action_requests
                SET expires_at = ?3, request_json = ?4
              WHERE project_id = ?1 AND user_action_request_id = ?2",
            params![
                fixture.project_id(),
                request_id,
                expires_at,
                request_json.to_string()
            ],
        )?;
        Ok(())
    }

    fn set_pending_action_basis_status(
        fixture: &CoreFixture,
        request_id: &str,
        basis_status: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = fixture.conn()?;
        let basis_json: String = conn.query_row(
            "SELECT basis_json
               FROM user_action_requests
              WHERE project_id = ?1 AND user_action_request_id = ?2",
            params![fixture.project_id(), request_id],
            |row| row.get(0),
        )?;
        let mut basis_json = serde_json::from_str::<Value>(&basis_json)?;
        basis_json["coordinates"]["compatibility_status"] = Value::String(basis_status.to_owned());
        conn.execute(
            "UPDATE user_action_requests
                SET basis_status = ?3, basis_json = ?4
              WHERE project_id = ?1 AND user_action_request_id = ?2",
            params![
                fixture.project_id(),
                request_id,
                basis_status,
                basis_json.to_string()
            ],
        )?;
        Ok(())
    }

    fn assert_expired(fixture: &CoreFixture, now: &str) -> Result<(), Box<dyn Error>> {
        let checked = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            check_input(fixture, now),
        )?;
        assert!(matches!(
            checked,
            UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Expired(_))
        ));
        Ok(())
    }

    fn create_input(fixture: &CoreFixture, request_id: &str) -> UserActionChannelTokenCreate {
        UserActionChannelTokenCreate {
            token: TOKEN.to_owned(),
            project_id: fixture.project_id().to_owned(),
            channel_kind: UserActionChannelKind::LocalWebConsent,
            connection_internal_id: fixture.connection_id().to_owned(),
            user_action_request_id: request_id.to_owned(),
            capture_basis: "local_user_local_web".to_owned(),
            created_metadata_json: "{}".to_owned(),
        }
    }

    fn check_input(fixture: &CoreFixture, now: &str) -> UserActionChannelTokenCheck {
        UserActionChannelTokenCheck {
            token: TOKEN.to_owned(),
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now: now.to_owned(),
        }
    }
}
