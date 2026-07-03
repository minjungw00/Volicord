use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    agent_connections::is_agent_connection_project_allowed,
    bootstrap::{list_projects, project_record_for_execution},
    schema::PROJECT_STATE_DATABASE_KIND,
    sqlite::{begin_immediate_transaction, open_project_state_database},
    StoreError, StoreResult,
};

const TOKEN_HASH_BYTES: usize = 32;
const TOKEN_HASH_HEX_LEN: usize = TOKEN_HASH_BYTES * 2;
const MAX_TOKEN_TEXT_BYTES: usize = 256;
const MAX_TOKEN_TTL_SECONDS: u64 = 30 * 60;

/// Input for creating one pending local web consent token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWebConsentTokenCreate {
    pub token: String,
    pub project_id: String,
    pub connection_internal_id: String,
    pub judgment_id: String,
    pub capture_basis: String,
    pub ttl_seconds: u64,
    pub created_metadata_json: String,
}

/// Input for checking one local web consent token against a selected endpoint context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWebConsentTokenCheck {
    pub token: String,
    pub expected_project_id: String,
    pub expected_connection_internal_id: String,
    pub now: String,
}

/// Stored local web consent token metadata. The raw token is never returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWebConsentTokenRecord {
    pub token_hash: String,
    pub project_id: String,
    pub connection_internal_id: String,
    pub judgment_id: String,
    pub capture_basis: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_metadata_json: String,
    pub completion_metadata_json: String,
}

/// Non-recording validation failure for a presented local web consent token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalWebConsentTokenRejection {
    Invalid,
    Expired(LocalWebConsentTokenRecord),
    Consumed(LocalWebConsentTokenRecord),
    WrongProject {
        expected_project_id: String,
        actual_project_id: String,
    },
    WrongConnection {
        expected_connection_internal_id: String,
        actual_connection_internal_id: String,
    },
}

/// Validation result for a local web consent token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalWebConsentTokenValidation {
    Valid(LocalWebConsentTokenRecord),
    Rejected(LocalWebConsentTokenRejection),
}

/// Creates one pending local web consent token record and stores only its hash.
pub fn create_local_web_consent_token(
    runtime_home: impl AsRef<Path>,
    input: LocalWebConsentTokenCreate,
) -> StoreResult<LocalWebConsentTokenRecord> {
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
    let token_hash = local_web_consent_token_hash(&input.token)?;
    let ttl_modifier = format!("+{} seconds", input.ttl_seconds);
    let mut conn = open_project_state_for_local_consent(&runtime_home, &input.project_id)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_pending_judgment_tx(&tx, &input.project_id, &input.judgment_id)?;
    let (created_at, expires_at): (String, String) = tx.query_row(
        "SELECT
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
        [&ttl_modifier],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    tx.execute(
        "INSERT INTO local_web_consent_tokens (
            project_id,
            token_hash,
            connection_internal_id,
            judgment_id,
            capture_basis,
            status,
            created_at,
            expires_at,
            consumed_at,
            completed_at,
            created_metadata_json,
            completion_metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?7, NULL, NULL, ?8, '{}')",
        params![
            input.project_id,
            token_hash,
            input.connection_internal_id,
            input.judgment_id,
            input.capture_basis,
            created_at,
            expires_at,
            input.created_metadata_json
        ],
    )?;
    let record = local_web_consent_token_record_tx(&tx, &token_hash)?.ok_or_else(|| {
        StoreError::SchemaInvariant {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            detail: "inserted local_web_consent_tokens row cannot be read".to_owned(),
        }
    })?;
    tx.commit()?;
    Ok(record)
}

/// Validates one local web consent token without consuming it.
pub fn validate_local_web_consent_token(
    runtime_home: impl AsRef<Path>,
    input: LocalWebConsentTokenCheck,
) -> StoreResult<LocalWebConsentTokenValidation> {
    validate_token_check(&input)?;
    let Some(token_hash) = local_web_consent_token_hash_for_lookup(&input.token) else {
        return Ok(LocalWebConsentTokenValidation::Rejected(
            LocalWebConsentTokenRejection::Invalid,
        ));
    };
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let mut conn =
        match open_project_state_for_local_consent(&runtime_home, &input.expected_project_id) {
            Ok(conn) => conn,
            Err(StoreError::NotFound { .. }) => {
                return token_wrong_project_or_invalid(&runtime_home, &token_hash, &input);
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
    )?;
    tx.commit()?;
    if matches!(
        validation,
        LocalWebConsentTokenValidation::Rejected(LocalWebConsentTokenRejection::Invalid)
    ) {
        token_wrong_project_or_invalid(&runtime_home, &token_hash, &input)
    } else {
        Ok(validation)
    }
}

/// Marks pending local web consent tokens expired at or before `now`.
pub fn expire_local_web_consent_tokens(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    now: &str,
) -> StoreResult<usize> {
    validate_identifier("project_id", project_id)?;
    validate_timestamp_text("now", now)?;
    let mut conn = open_project_state_for_local_consent(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let changed = expire_pending_tokens_tx(&tx, project_id, now)?;
    tx.commit()?;
    Ok(changed)
}

/// Returns the project-state clock in the public timestamp shape used by consent tokens.
pub fn local_web_consent_current_timestamp(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<String> {
    validate_identifier("project_id", project_id)?;
    let conn = open_project_state_for_local_consent(runtime_home, project_id)?;
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
    .map_err(StoreError::from)
}

/// Computes the stored hash for a raw local web consent token.
pub fn local_web_consent_token_hash(token: &str) -> StoreResult<String> {
    validate_token_text("token", token)?;
    Ok(local_web_consent_token_hash_unchecked(token))
}

fn local_web_consent_token_hash_for_lookup(token: &str) -> Option<String> {
    if validate_token_text("token", token).is_err() {
        return None;
    }
    Some(local_web_consent_token_hash_unchecked(token))
}

fn local_web_consent_token_hash_unchecked(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"volicord.local_web_consent_token.v1\0");
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
    tx.execute(
        "UPDATE local_web_consent_tokens
            SET status = 'expired'
          WHERE project_id = ?1
            AND status = 'pending'
            AND expires_at <= ?2",
        params![project_id, now],
    )
    .map_err(StoreError::from)
}

fn validate_record_for_context(
    tx: &Transaction<'_>,
    token_hash: &str,
    expected_project_id: &str,
    expected_connection_internal_id: &str,
) -> StoreResult<LocalWebConsentTokenValidation> {
    let Some(record) = local_web_consent_token_record_tx(tx, token_hash)? else {
        return Ok(LocalWebConsentTokenValidation::Rejected(
            LocalWebConsentTokenRejection::Invalid,
        ));
    };
    if record.project_id != expected_project_id {
        return Ok(LocalWebConsentTokenValidation::Rejected(
            LocalWebConsentTokenRejection::WrongProject {
                expected_project_id: expected_project_id.to_owned(),
                actual_project_id: record.project_id,
            },
        ));
    }
    if record.connection_internal_id != expected_connection_internal_id {
        return Ok(LocalWebConsentTokenValidation::Rejected(
            LocalWebConsentTokenRejection::WrongConnection {
                expected_connection_internal_id: expected_connection_internal_id.to_owned(),
                actual_connection_internal_id: record.connection_internal_id,
            },
        ));
    }
    match record.status.as_str() {
        "pending" => Ok(LocalWebConsentTokenValidation::Valid(record)),
        "expired" => Ok(LocalWebConsentTokenValidation::Rejected(
            LocalWebConsentTokenRejection::Expired(record),
        )),
        "consumed" => Ok(LocalWebConsentTokenValidation::Rejected(
            LocalWebConsentTokenRejection::Consumed(record),
        )),
        _ => Err(StoreError::CorruptStoredValue {
            database_kind: PROJECT_STATE_DATABASE_KIND,
            field: "local_web_consent_tokens.status",
        }),
    }
}

fn local_web_consent_token_record_tx(
    conn: &Connection,
    token_hash: &str,
) -> StoreResult<Option<LocalWebConsentTokenRecord>> {
    conn.query_row(
        "SELECT
            t.token_hash,
            t.project_id,
            t.connection_internal_id,
            t.judgment_id,
            t.capture_basis,
            t.status,
            t.created_at,
            t.expires_at,
            t.consumed_at,
            t.completed_at,
            t.created_metadata_json,
            t.completion_metadata_json
         FROM local_web_consent_tokens AS t
         WHERE t.token_hash = ?1",
        [token_hash],
        |row| {
            Ok(LocalWebConsentTokenRecord {
                token_hash: row.get(0)?,
                project_id: row.get(1)?,
                connection_internal_id: row.get(2)?,
                judgment_id: row.get(3)?,
                capture_basis: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                expires_at: row.get(7)?,
                consumed_at: row.get(8)?,
                completed_at: row.get(9)?,
                created_metadata_json: row.get(10)?,
                completion_metadata_json: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn open_project_state_for_local_consent(
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

fn require_pending_judgment_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    judgment_id: &str,
) -> StoreResult<()> {
    let status = tx
        .query_row(
            "SELECT status
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
            params![project_id, judgment_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match status.as_deref() {
        Some("pending") => Ok(()),
        Some(_) => Err(StoreError::Conflict {
            entity: "user_judgment",
            id: judgment_id.to_owned(),
            detail: "local web consent token issue requires a pending user judgment".to_owned(),
        }),
        None => Err(StoreError::NotFound {
            entity: "user_judgment",
            id: judgment_id.to_owned(),
        }),
    }
}

fn token_wrong_project_or_invalid(
    runtime_home: &Path,
    token_hash: &str,
    input: &LocalWebConsentTokenCheck,
) -> StoreResult<LocalWebConsentTokenValidation> {
    for project in list_projects(runtime_home)? {
        if project.project_id == input.expected_project_id {
            continue;
        }
        let mut conn = open_project_state_database(&project.state_db_path)?;
        let tx = begin_immediate_transaction(&mut conn)?;
        expire_pending_tokens_tx(&tx, &project.project_id, &input.now)?;
        let record = local_web_consent_token_record_tx(&tx, token_hash)?;
        tx.commit()?;
        if let Some(record) = record {
            return Ok(LocalWebConsentTokenValidation::Rejected(
                LocalWebConsentTokenRejection::WrongProject {
                    expected_project_id: input.expected_project_id.clone(),
                    actual_project_id: record.project_id,
                },
            ));
        }
    }
    Ok(LocalWebConsentTokenValidation::Rejected(
        LocalWebConsentTokenRejection::Invalid,
    ))
}

fn validate_token_create(input: &LocalWebConsentTokenCreate) -> StoreResult<()> {
    validate_token_text("token", &input.token)?;
    validate_identifier("project_id", &input.project_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("judgment_id", &input.judgment_id)?;
    validate_identifier("capture_basis", &input.capture_basis)?;
    if input.ttl_seconds == 0 || input.ttl_seconds > MAX_TOKEN_TTL_SECONDS {
        return Err(StoreError::InvalidInput {
            detail: format!("ttl_seconds must be between 1 and {MAX_TOKEN_TTL_SECONDS} seconds"),
        });
    }
    validate_json_object("created_metadata_json", &input.created_metadata_json)
}

fn validate_token_check(input: &LocalWebConsentTokenCheck) -> StoreResult<()> {
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
    if value.contains('\0') {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL bytes"),
        });
    }
    Ok(())
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
    const OTHER_TOKEN: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    const CAPTURE_BASIS: &str = "local_user_local_web";

    #[test]
    fn token_creation_stores_hash_and_validates_context() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-local-consent-create")?;
        insert_pending_judgment(&fixture)?;

        let record = create_local_web_consent_token(
            fixture.runtime_home_path(),
            create_input(&fixture, TOKEN, 600),
        )?;

        assert_eq!(record.token_hash, local_web_consent_token_hash(TOKEN)?);
        assert_ne!(record.token_hash, TOKEN);
        assert_eq!(record.project_id, fixture.project_id());
        assert_eq!(record.connection_internal_id, fixture.connection_id());
        assert_eq!(record.capture_basis, CAPTURE_BASIS);
        assert_eq!(record.status, "pending");

        let plaintext_matches = fixture.conn()?.query_row(
            "SELECT COUNT(*)
               FROM local_web_consent_tokens
              WHERE token_hash = ?1
                 OR created_metadata_json LIKE ?2
                 OR completion_metadata_json LIKE ?2",
            params![TOKEN, format!("%{TOKEN}%")],
            |row| row.get::<_, i64>(0),
        )?;
        assert_eq!(plaintext_matches, 0);

        let checked = validate_local_web_consent_token(
            fixture.runtime_home_path(),
            check_input(&fixture, TOKEN, &record.created_at),
        )?;
        assert!(matches!(checked, LocalWebConsentTokenValidation::Valid(_)));
        Ok(())
    }

    #[test]
    fn invalid_token_is_rejected_without_lookup_error() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-local-consent-invalid")?;
        let checked = validate_local_web_consent_token(
            fixture.runtime_home_path(),
            check_input(&fixture, "not valid", "2026-01-01T00:00:00Z"),
        )?;
        assert_eq!(
            checked,
            LocalWebConsentTokenValidation::Rejected(LocalWebConsentTokenRejection::Invalid)
        );
        Ok(())
    }

    #[test]
    fn expired_token_is_rejected_and_marked_expired() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-local-consent-expired")?;
        insert_pending_judgment(&fixture)?;
        create_local_web_consent_token(
            fixture.runtime_home_path(),
            create_input(&fixture, TOKEN, 1),
        )?;

        let checked = validate_local_web_consent_token(
            fixture.runtime_home_path(),
            check_input(&fixture, TOKEN, "9999-01-01T00:00:00Z"),
        )?;

        let LocalWebConsentTokenValidation::Rejected(LocalWebConsentTokenRejection::Expired(
            record,
        )) = checked
        else {
            panic!("expected expired rejection");
        };
        assert_eq!(record.status, "expired");
        Ok(())
    }

    #[test]
    fn wrong_project_and_connection_are_rejected() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-local-consent-wrong-context")?;
        insert_pending_judgment(&fixture)?;
        let record = create_local_web_consent_token(
            fixture.runtime_home_path(),
            create_input(&fixture, TOKEN, 600),
        )?;

        let wrong_project = validate_local_web_consent_token(
            fixture.runtime_home_path(),
            LocalWebConsentTokenCheck {
                expected_project_id: "project_other".to_owned(),
                ..check_input(&fixture, TOKEN, &record.created_at)
            },
        )?;
        assert!(matches!(
            wrong_project,
            LocalWebConsentTokenValidation::Rejected(
                LocalWebConsentTokenRejection::WrongProject { .. }
            )
        ));

        let wrong_connection = validate_local_web_consent_token(
            fixture.runtime_home_path(),
            LocalWebConsentTokenCheck {
                expected_connection_internal_id: "connection_other".to_owned(),
                ..check_input(&fixture, TOKEN, &record.created_at)
            },
        )?;
        assert!(matches!(
            wrong_connection,
            LocalWebConsentTokenValidation::Rejected(
                LocalWebConsentTokenRejection::WrongConnection { .. }
            )
        ));
        let still_valid = validate_local_web_consent_token(
            fixture.runtime_home_path(),
            check_input(&fixture, TOKEN, &record.created_at),
        )?;
        assert!(matches!(
            still_valid,
            LocalWebConsentTokenValidation::Valid(_)
        ));
        Ok(())
    }

    #[test]
    fn duplicate_token_hash_conflicts() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("store-local-consent-duplicate")?;
        insert_pending_judgment(&fixture)?;
        create_local_web_consent_token(
            fixture.runtime_home_path(),
            create_input(&fixture, TOKEN, 600),
        )?;

        let error = create_local_web_consent_token(
            fixture.runtime_home_path(),
            create_input(&fixture, TOKEN, 600),
        )
        .expect_err("duplicate token hash should reject");

        assert!(matches!(error, StoreError::Sqlite(_)));

        let other = create_local_web_consent_token(
            fixture.runtime_home_path(),
            create_input(&fixture, OTHER_TOKEN, 600),
        )?;
        assert_eq!(other.status, "pending");
        Ok(())
    }

    fn insert_pending_judgment(fixture: &CoreFixture) -> Result<(), Box<dyn Error>> {
        let conn = fixture.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO tasks (
                project_id,
                task_id,
                created_by_actor_source,
                mode,
                lifecycle_phase,
                created_at,
                updated_at
            )
            VALUES (?1, 'task_local_web', ?2, 'standard', 'ready', 't0', 't0')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        conn.execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                basis_json,
                requested_by_actor_source,
                requested_at
            )
            VALUES (
                ?1,
                'uj_local_web',
                'task_local_web',
                'product_decision',
                'pending',
                '{}',
                ?2,
                't0'
            )",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        Ok(())
    }

    fn create_input(
        fixture: &CoreFixture,
        token: &str,
        ttl_seconds: u64,
    ) -> LocalWebConsentTokenCreate {
        LocalWebConsentTokenCreate {
            token: token.to_owned(),
            project_id: fixture.project_id().to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            judgment_id: "uj_local_web".to_owned(),
            capture_basis: CAPTURE_BASIS.to_owned(),
            ttl_seconds,
            created_metadata_json: "{}".to_owned(),
        }
    }

    fn check_input(fixture: &CoreFixture, token: &str, now: &str) -> LocalWebConsentTokenCheck {
        LocalWebConsentTokenCheck {
            token: token.to_owned(),
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now: now.to_owned(),
        }
    }
}
