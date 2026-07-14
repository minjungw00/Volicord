use std::path::Path;

use chrono::Duration;
use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::UtcTimestamp;

use crate::{
    agent_connections::{HOST_KIND_CLAUDE_CODE, HOST_KIND_CODEX, HOST_KIND_GENERIC},
    schema::REGISTRY_DATABASE_KIND,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

/// The only host capability that can make model-invisible credential delivery eligible.
pub const HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE: &str = "model_invisible_user_surface";

/// Adapter profile that owns the local-web User Channel delivery contract.
pub const HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1: &str = "mcp_user_channel_local_web_v1";

/// A live-host capability observation passed.
pub const HOST_CAPABILITY_OUTCOME_PASSED: &str = "passed";
/// A live-host capability observation failed.
pub const HOST_CAPABILITY_OUTCOME_FAILED: &str = "failed";
/// A live-host capability observation could not be completed.
pub const HOST_CAPABILITY_OUTCOME_UNAVAILABLE: &str = "unavailable";
/// A prior live-host capability result was revoked.
pub const HOST_CAPABILITY_OUTCOME_REVOKED: &str = "revoked";

/// Maximum freshness window for one host-capability verification.
pub const HOST_CAPABILITY_VERIFICATION_MAX_TTL_SECONDS: i64 = 86_400;

/// One immutable host-capability verification to publish and make current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCapabilityVerificationInput {
    pub verification_internal_id: String,
    pub connection_internal_id: String,
    pub capability: String,
    pub outcome: String,
    pub host_kind: String,
    pub host_version: String,
    pub client_name: String,
    pub client_version: String,
    pub adapter_profile: String,
    pub adapter_version: String,
    pub managed_fingerprint: String,
    pub volicord_build_id: String,
    pub source_revision: String,
    pub target_triple: String,
    pub executable_sha256: String,
    pub evidence_artifact_sha256: String,
    pub observed_at: String,
    pub expires_at: String,
    pub metadata_json: String,
    pub created_at: String,
}

/// One immutable row from `host_capability_verifications`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCapabilityVerificationRecord {
    pub verification_internal_id: String,
    pub connection_internal_id: String,
    pub capability: String,
    pub outcome: String,
    pub host_kind: String,
    pub host_version: String,
    pub client_name: String,
    pub client_version: String,
    pub adapter_profile: String,
    pub adapter_version: String,
    pub managed_fingerprint: String,
    pub volicord_build_id: String,
    pub source_revision: String,
    pub target_triple: String,
    pub executable_sha256: String,
    pub evidence_artifact_sha256: String,
    pub observed_at: String,
    pub expires_at: String,
    pub metadata_json: String,
    pub created_at: String,
}

/// Exact runtime coordinates that a current passing verification must match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCapabilityVerificationExpectation {
    pub connection_internal_id: String,
    pub capability: String,
    pub host_kind: String,
    pub host_version: String,
    pub client_name: String,
    pub client_version: String,
    pub adapter_profile: String,
    pub adapter_version: String,
    pub managed_fingerprint: String,
    pub volicord_build_id: String,
    pub source_revision: String,
    pub target_triple: String,
    pub executable_sha256: String,
    pub evidence_artifact_sha256: String,
}

/// Appends one immutable verification and atomically makes it current.
pub fn publish_host_capability_verification(
    runtime_home: impl AsRef<Path>,
    input: HostCapabilityVerificationInput,
) -> StoreResult<HostCapabilityVerificationRecord> {
    validate_nonempty("verification_internal_id", &input.verification_internal_id)?;
    let registry_path = registry_db_path(runtime_home);
    let validated_before_open = if registry_path.exists() {
        None
    } else {
        Some(validate_verification_input(&input)?)
    };
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;

    if let Some(existing) = verification_by_global_id(&tx, &input.verification_internal_id)? {
        validate_stored_record(&existing)?;
        if record_matches_input(&existing, &input) {
            current_verification_from_conn(&tx, &input.connection_internal_id, &input.capability)?
                .ok_or_else(|| StoreError::CorruptStoredValue {
                    database_kind: REGISTRY_DATABASE_KIND,
                    field: "host_capability_state.current_verification_internal_id",
                })?;
            tx.commit()?;
            return Ok(existing);
        }
        return Err(StoreError::Conflict {
            entity: "host_capability_verification",
            id: input.verification_internal_id.clone(),
            detail: "verification_internal_id is already bound to different immutable content"
                .to_owned(),
        });
    }

    let validated = match validated_before_open {
        Some(validated) => validated,
        None => validate_verification_input(&input)?,
    };

    let connection = connection_identity(&tx, &input.connection_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_connection",
            id: input.connection_internal_id.clone(),
        }
    })?;
    if input.outcome == HOST_CAPABILITY_OUTCOME_PASSED
        && (input.host_kind == HOST_KIND_GENERIC
            || input.host_kind != connection.host_kind
            || input.managed_fingerprint != connection.managed_fingerprint)
    {
        return Err(StoreError::InvalidInput {
            detail: "a passing host capability must match the current non-generic Agent Connection host kind and managed fingerprint".to_owned(),
        });
    }

    if let Some(current) =
        current_verification_from_conn(&tx, &input.connection_internal_id, &input.capability)?
    {
        let current = parse_stored_timestamp(
            "host_capability_verifications.observed_at",
            &current.observed_at,
        )?;
        if validated.observed_at <= current {
            return Err(StoreError::Conflict {
                entity: "host_capability_verification",
                id: input.verification_internal_id.clone(),
                detail:
                    "a current verification can be replaced only by a strictly newer observation"
                        .to_owned(),
            });
        }
    }

    tx.execute(
        "INSERT INTO host_capability_verifications (
            verification_internal_id,
            connection_internal_id,
            capability,
            outcome,
            host_kind,
            host_version,
            client_name,
            client_version,
            adapter_profile,
            adapter_version,
            managed_fingerprint,
            volicord_build_id,
            source_revision,
            target_triple,
            executable_sha256,
            evidence_artifact_sha256,
            observed_at,
            expires_at,
            metadata_json,
            created_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20
        )",
        params![
            input.verification_internal_id,
            input.connection_internal_id,
            input.capability,
            input.outcome,
            input.host_kind,
            input.host_version,
            input.client_name,
            input.client_version,
            input.adapter_profile,
            input.adapter_version,
            input.managed_fingerprint,
            input.volicord_build_id,
            input.source_revision,
            input.target_triple,
            input.executable_sha256,
            input.evidence_artifact_sha256,
            input.observed_at,
            input.expires_at,
            input.metadata_json,
            input.created_at,
        ],
    )?;
    tx.execute(
        "INSERT INTO host_capability_state (
            connection_internal_id,
            capability,
            current_verification_internal_id,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT (connection_internal_id, capability) DO UPDATE SET
            current_verification_internal_id = excluded.current_verification_internal_id,
            updated_at = excluded.updated_at",
        params![
            input.connection_internal_id,
            input.capability,
            input.verification_internal_id,
            input.created_at,
        ],
    )?;

    let record = verification_by_id(
        &tx,
        &input.connection_internal_id,
        &input.capability,
        &input.verification_internal_id,
    )?
    .ok_or_else(|| {
        StoreError::schema_invariant(
            REGISTRY_DATABASE_KIND,
            "published host-capability verification is unavailable before commit",
        )
    })?;
    validate_stored_record(&record)?;
    tx.commit()?;
    Ok(record)
}

/// Reads the current immutable verification without creating or writing Registry state.
pub fn current_host_capability_verification_read_only(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    capability: &str,
) -> StoreResult<Option<HostCapabilityVerificationRecord>> {
    validate_nonempty("connection_internal_id", connection_internal_id)?;
    validate_capability(capability)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    current_verification_from_conn(&conn, connection_internal_id, capability)
}

/// Evaluates only the current verification against exact runtime coordinates.
///
/// Ordinary ineligibility returns `Ok(None)`. Malformed current owner state
/// returns an error so callers can fail closed while preserving diagnostics.
pub fn evaluate_current_host_capability_verification_read_only(
    runtime_home: impl AsRef<Path>,
    expectation: &HostCapabilityVerificationExpectation,
    now: &str,
) -> StoreResult<Option<HostCapabilityVerificationRecord>> {
    validate_expectation(expectation)?;
    let now = validate_input_timestamp("now", now)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    let Some(record) = current_verification_from_conn(
        &conn,
        &expectation.connection_internal_id,
        &expectation.capability,
    )?
    else {
        return Ok(None);
    };

    let Some(connection) = connection_identity(&conn, &expectation.connection_internal_id)? else {
        return Ok(None);
    };
    let observed_at = parse_stored_timestamp(
        "host_capability_verifications.observed_at",
        &record.observed_at,
    )?;
    let expires_at = parse_stored_timestamp(
        "host_capability_verifications.expires_at",
        &record.expires_at,
    )?;

    let eligible = record.outcome == HOST_CAPABILITY_OUTCOME_PASSED
        && record.host_kind != HOST_KIND_GENERIC
        && observed_at <= now
        && now < expires_at
        && connection.enabled
        && connection.host_kind == record.host_kind
        && connection.managed_fingerprint == record.managed_fingerprint
        && record.connection_internal_id == expectation.connection_internal_id
        && record.capability == expectation.capability
        && record.host_kind == expectation.host_kind
        && record.host_version == expectation.host_version
        && record.client_name == expectation.client_name
        && record.client_version == expectation.client_version
        && record.adapter_profile == expectation.adapter_profile
        && record.adapter_version == expectation.adapter_version
        && record.managed_fingerprint == expectation.managed_fingerprint
        && record.volicord_build_id == expectation.volicord_build_id
        && record.source_revision == expectation.source_revision
        && record.target_triple == expectation.target_triple
        && record.executable_sha256 == expectation.executable_sha256
        && record.evidence_artifact_sha256 == expectation.evidence_artifact_sha256;

    Ok(eligible.then_some(record))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedVerificationWindow {
    observed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectionIdentity {
    host_kind: String,
    managed_fingerprint: String,
    enabled: bool,
}

fn validate_verification_input(
    input: &HostCapabilityVerificationInput,
) -> StoreResult<ValidatedVerificationWindow> {
    validate_nonempty("verification_internal_id", &input.verification_internal_id)?;
    validate_nonempty("connection_internal_id", &input.connection_internal_id)?;
    validate_capability(&input.capability)?;
    validate_outcome(&input.outcome)?;
    validate_host_kind(&input.host_kind)?;
    for (field, value) in [
        ("host_version", input.host_version.as_str()),
        ("client_name", input.client_name.as_str()),
        ("client_version", input.client_version.as_str()),
        ("adapter_version", input.adapter_version.as_str()),
        ("managed_fingerprint", input.managed_fingerprint.as_str()),
        ("volicord_build_id", input.volicord_build_id.as_str()),
        ("source_revision", input.source_revision.as_str()),
        ("target_triple", input.target_triple.as_str()),
    ] {
        validate_nonempty(field, value)?;
    }
    validate_adapter_profile(&input.adapter_profile)?;
    validate_sha256("executable_sha256", &input.executable_sha256)?;
    validate_sha256("evidence_artifact_sha256", &input.evidence_artifact_sha256)?;
    if input.metadata_json != "{}" {
        return Err(StoreError::InvalidInput {
            detail: "host capability metadata_json must be the canonical empty object {}"
                .to_owned(),
        });
    }
    let observed_at = validate_input_timestamp("observed_at", &input.observed_at)?;
    let expires_at = validate_input_timestamp("expires_at", &input.expires_at)?;
    let created_at = validate_input_timestamp("created_at", &input.created_at)?;
    validate_window(&observed_at, &expires_at, StoreValueKind::Input)?;
    if observed_at > created_at {
        return Err(StoreError::InvalidInput {
            detail: "host capability verification requires observed_at <= created_at".to_owned(),
        });
    }

    if input.outcome == HOST_CAPABILITY_OUTCOME_PASSED && input.host_kind == HOST_KIND_GENERIC {
        return Err(StoreError::InvalidInput {
            detail: "a generic host capability cannot have outcome=passed".to_owned(),
        });
    }
    if input.outcome == HOST_CAPABILITY_OUTCOME_PASSED && !is_git_object_id(&input.source_revision)
    {
        return Err(StoreError::InvalidInput {
            detail: "a passing host capability requires source_revision to be an exact lowercase 40- or 64-character Git object ID".to_owned(),
        });
    }
    if input.outcome == HOST_CAPABILITY_OUTCOME_PASSED && input.host_version != input.client_version
    {
        return Err(StoreError::InvalidInput {
            detail: "a passing v1 host capability requires host_version to equal client_version"
                .to_owned(),
        });
    }
    if input.outcome == HOST_CAPABILITY_OUTCOME_PASSED && created_at >= expires_at {
        return Err(StoreError::InvalidInput {
            detail: "a passing host capability requires created_at < expires_at".to_owned(),
        });
    }
    Ok(ValidatedVerificationWindow { observed_at })
}

fn validate_expectation(expectation: &HostCapabilityVerificationExpectation) -> StoreResult<()> {
    validate_nonempty(
        "connection_internal_id",
        &expectation.connection_internal_id,
    )?;
    validate_capability(&expectation.capability)?;
    validate_host_kind(&expectation.host_kind)?;
    validate_adapter_profile(&expectation.adapter_profile)?;
    for (field, value) in [
        ("host_version", expectation.host_version.as_str()),
        ("client_name", expectation.client_name.as_str()),
        ("client_version", expectation.client_version.as_str()),
        ("adapter_version", expectation.adapter_version.as_str()),
        (
            "managed_fingerprint",
            expectation.managed_fingerprint.as_str(),
        ),
        ("volicord_build_id", expectation.volicord_build_id.as_str()),
        ("source_revision", expectation.source_revision.as_str()),
        ("target_triple", expectation.target_triple.as_str()),
    ] {
        validate_nonempty(field, value)?;
    }
    if expectation.host_version != expectation.client_version {
        return Err(StoreError::InvalidInput {
            detail: "the v1 host capability expects host_version to equal client_version"
                .to_owned(),
        });
    }
    validate_sha256("executable_sha256", &expectation.executable_sha256)?;
    validate_sha256(
        "evidence_artifact_sha256",
        &expectation.evidence_artifact_sha256,
    )
}

fn current_verification_from_conn(
    conn: &Connection,
    connection_internal_id: &str,
    capability: &str,
) -> StoreResult<Option<HostCapabilityVerificationRecord>> {
    let current = conn
        .query_row(
            "SELECT current_verification_internal_id, updated_at
               FROM host_capability_state
              WHERE connection_internal_id = ?1
                AND capability = ?2",
            params![connection_internal_id, capability],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((verification_internal_id, updated_at)) = current else {
        let history_count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM host_capability_verifications
              WHERE connection_internal_id = ?1
                AND capability = ?2",
            params![connection_internal_id, capability],
            |row| row.get(0),
        )?;
        if history_count != 0 {
            return Err(StoreError::CorruptStoredValue {
                database_kind: REGISTRY_DATABASE_KIND,
                field: "host_capability_state.current_verification_internal_id",
            });
        }
        return Ok(None);
    };
    parse_stored_timestamp("host_capability_state.updated_at", &updated_at)?;
    let record = verification_by_id(
        conn,
        connection_internal_id,
        capability,
        &verification_internal_id,
    )?
    .ok_or_else(|| StoreError::CorruptStoredValue {
        database_kind: REGISTRY_DATABASE_KIND,
        field: "host_capability_state.current_verification_internal_id",
    })?;
    validate_stored_record(&record)?;
    if updated_at != record.created_at {
        return Err(StoreError::CorruptStoredValue {
            database_kind: REGISTRY_DATABASE_KIND,
            field: "host_capability_state.updated_at",
        });
    }
    validate_current_pointer_is_unique_newest(
        conn,
        connection_internal_id,
        capability,
        &verification_internal_id,
    )?;
    Ok(Some(record))
}

fn validate_current_pointer_is_unique_newest(
    conn: &Connection,
    connection_internal_id: &str,
    capability: &str,
    current_verification_internal_id: &str,
) -> StoreResult<()> {
    let mut stmt = conn.prepare(
        "SELECT verification_internal_id, observed_at
           FROM host_capability_verifications
          WHERE connection_internal_id = ?1
            AND capability = ?2",
    )?;
    let rows = stmt.query_map(params![connection_internal_id, capability], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let observations = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    validate_unique_newest_host_capability_pointer(
        observations
            .iter()
            .map(|(id, observed_at)| (id.as_str(), observed_at.as_str())),
        current_verification_internal_id,
    )
}

pub(crate) fn validate_unique_newest_host_capability_pointer<'a>(
    observations: impl IntoIterator<Item = (&'a str, &'a str)>,
    current_verification_internal_id: &str,
) -> StoreResult<()> {
    let mut newest: Option<(UtcTimestamp, &'a str)> = None;
    let mut newest_count = 0usize;
    for (verification_internal_id, observed_at) in observations {
        validate_stored_nonempty(
            "host_capability_verifications.verification_internal_id",
            verification_internal_id,
        )?;
        let observed_at =
            parse_stored_timestamp("host_capability_verifications.observed_at", observed_at)?;
        match &mut newest {
            None => {
                newest = Some((observed_at, verification_internal_id));
                newest_count = 1;
            }
            Some((newest_observed_at, newest_id)) if &observed_at > newest_observed_at => {
                *newest_observed_at = observed_at;
                *newest_id = verification_internal_id;
                newest_count = 1;
            }
            Some((newest_observed_at, _)) if &observed_at == newest_observed_at => {
                newest_count += 1;
            }
            Some(_) => {}
        }
    }

    if newest_count != 1
        || newest.as_ref().map(|(_, id)| *id) != Some(current_verification_internal_id)
    {
        return corrupt("host_capability_state.current_verification_internal_id");
    }
    Ok(())
}

fn verification_by_global_id(
    conn: &Connection,
    verification_internal_id: &str,
) -> StoreResult<Option<HostCapabilityVerificationRecord>> {
    conn.query_row(
        "SELECT
            verification_internal_id,
            connection_internal_id,
            capability,
            outcome,
            host_kind,
            host_version,
            client_name,
            client_version,
            adapter_profile,
            adapter_version,
            managed_fingerprint,
            volicord_build_id,
            source_revision,
            target_triple,
            executable_sha256,
            evidence_artifact_sha256,
            observed_at,
            expires_at,
            metadata_json,
            created_at
         FROM host_capability_verifications
         WHERE verification_internal_id = ?1",
        [verification_internal_id],
        verification_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn verification_by_id(
    conn: &Connection,
    connection_internal_id: &str,
    capability: &str,
    verification_internal_id: &str,
) -> StoreResult<Option<HostCapabilityVerificationRecord>> {
    conn.query_row(
        "SELECT
            verification_internal_id,
            connection_internal_id,
            capability,
            outcome,
            host_kind,
            host_version,
            client_name,
            client_version,
            adapter_profile,
            adapter_version,
            managed_fingerprint,
            volicord_build_id,
            source_revision,
            target_triple,
            executable_sha256,
            evidence_artifact_sha256,
            observed_at,
            expires_at,
            metadata_json,
            created_at
         FROM host_capability_verifications
         WHERE connection_internal_id = ?1
           AND capability = ?2
           AND verification_internal_id = ?3",
        params![connection_internal_id, capability, verification_internal_id],
        verification_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn verification_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<HostCapabilityVerificationRecord> {
    Ok(HostCapabilityVerificationRecord {
        verification_internal_id: row.get(0)?,
        connection_internal_id: row.get(1)?,
        capability: row.get(2)?,
        outcome: row.get(3)?,
        host_kind: row.get(4)?,
        host_version: row.get(5)?,
        client_name: row.get(6)?,
        client_version: row.get(7)?,
        adapter_profile: row.get(8)?,
        adapter_version: row.get(9)?,
        managed_fingerprint: row.get(10)?,
        volicord_build_id: row.get(11)?,
        source_revision: row.get(12)?,
        target_triple: row.get(13)?,
        executable_sha256: row.get(14)?,
        evidence_artifact_sha256: row.get(15)?,
        observed_at: row.get(16)?,
        expires_at: row.get(17)?,
        metadata_json: row.get(18)?,
        created_at: row.get(19)?,
    })
}

fn connection_identity(
    conn: &Connection,
    connection_internal_id: &str,
) -> StoreResult<Option<ConnectionIdentity>> {
    conn.query_row(
        "SELECT host_kind, managed_fingerprint, enabled
           FROM agent_connections
          WHERE connection_internal_id = ?1",
        [connection_internal_id],
        |row| {
            Ok(ConnectionIdentity {
                host_kind: row.get(0)?,
                managed_fingerprint: row.get(1)?,
                enabled: row.get::<_, i64>(2)? == 1,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

pub(crate) fn validate_stored_record(record: &HostCapabilityVerificationRecord) -> StoreResult<()> {
    validate_stored_nonempty(
        "host_capability_verifications.verification_internal_id",
        &record.verification_internal_id,
    )?;
    validate_stored_nonempty(
        "host_capability_verifications.connection_internal_id",
        &record.connection_internal_id,
    )?;
    if record.capability != HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE {
        return corrupt("host_capability_verifications.capability");
    }
    if !is_outcome(&record.outcome) {
        return corrupt("host_capability_verifications.outcome");
    }
    if !is_host_kind(&record.host_kind)
        || (record.outcome == HOST_CAPABILITY_OUTCOME_PASSED
            && record.host_kind == HOST_KIND_GENERIC)
    {
        return corrupt("host_capability_verifications.host_kind");
    }
    if record.adapter_profile != HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1 {
        return corrupt("host_capability_verifications.adapter_profile");
    }
    for (field, value) in [
        (
            "host_capability_verifications.host_version",
            record.host_version.as_str(),
        ),
        (
            "host_capability_verifications.client_name",
            record.client_name.as_str(),
        ),
        (
            "host_capability_verifications.client_version",
            record.client_version.as_str(),
        ),
        (
            "host_capability_verifications.adapter_version",
            record.adapter_version.as_str(),
        ),
        (
            "host_capability_verifications.managed_fingerprint",
            record.managed_fingerprint.as_str(),
        ),
        (
            "host_capability_verifications.volicord_build_id",
            record.volicord_build_id.as_str(),
        ),
        (
            "host_capability_verifications.source_revision",
            record.source_revision.as_str(),
        ),
        (
            "host_capability_verifications.target_triple",
            record.target_triple.as_str(),
        ),
    ] {
        validate_stored_nonempty(field, value)?;
    }
    if !is_sha256(&record.executable_sha256) {
        return corrupt("host_capability_verifications.executable_sha256");
    }
    if !is_sha256(&record.evidence_artifact_sha256) {
        return corrupt("host_capability_verifications.evidence_artifact_sha256");
    }
    if record.outcome == HOST_CAPABILITY_OUTCOME_PASSED
        && !is_git_object_id(&record.source_revision)
    {
        return corrupt("host_capability_verifications.source_revision");
    }
    if record.outcome == HOST_CAPABILITY_OUTCOME_PASSED
        && record.host_version != record.client_version
    {
        return corrupt("host_capability_verifications.client_version");
    }
    if record.metadata_json != "{}" {
        return corrupt("host_capability_verifications.metadata_json");
    }
    let observed_at = parse_stored_timestamp(
        "host_capability_verifications.observed_at",
        &record.observed_at,
    )?;
    let expires_at = parse_stored_timestamp(
        "host_capability_verifications.expires_at",
        &record.expires_at,
    )?;
    let created_at = parse_stored_timestamp(
        "host_capability_verifications.created_at",
        &record.created_at,
    )?;
    validate_window(&observed_at, &expires_at, StoreValueKind::Stored)?;
    if observed_at > created_at {
        return corrupt("host_capability_verifications.created_at");
    }
    if record.outcome == HOST_CAPABILITY_OUTCOME_PASSED && created_at >= expires_at {
        return corrupt("host_capability_verifications.created_at");
    }
    Ok(())
}

fn record_matches_input(
    record: &HostCapabilityVerificationRecord,
    input: &HostCapabilityVerificationInput,
) -> bool {
    record.verification_internal_id == input.verification_internal_id
        && record.connection_internal_id == input.connection_internal_id
        && record.capability == input.capability
        && record.outcome == input.outcome
        && record.host_kind == input.host_kind
        && record.host_version == input.host_version
        && record.client_name == input.client_name
        && record.client_version == input.client_version
        && record.adapter_profile == input.adapter_profile
        && record.adapter_version == input.adapter_version
        && record.managed_fingerprint == input.managed_fingerprint
        && record.volicord_build_id == input.volicord_build_id
        && record.source_revision == input.source_revision
        && record.target_triple == input.target_triple
        && record.executable_sha256 == input.executable_sha256
        && record.evidence_artifact_sha256 == input.evidence_artifact_sha256
        && record.observed_at == input.observed_at
        && record.expires_at == input.expires_at
        && record.metadata_json == input.metadata_json
        && record.created_at == input.created_at
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreValueKind {
    Input,
    Stored,
}

fn validate_window(
    observed_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    kind: StoreValueKind,
) -> StoreResult<()> {
    let maximum = observed_at
        .checked_add(Duration::seconds(
            HOST_CAPABILITY_VERIFICATION_MAX_TTL_SECONDS,
        ))
        .map_err(|_| window_error(kind))?;
    if observed_at < expires_at && expires_at <= &maximum {
        Ok(())
    } else {
        Err(window_error(kind))
    }
}

fn window_error(kind: StoreValueKind) -> StoreError {
    match kind {
        StoreValueKind::Input => StoreError::InvalidInput {
            detail: format!(
                "host capability verification requires observed_at < expires_at <= observed_at + {HOST_CAPABILITY_VERIFICATION_MAX_TTL_SECONDS} seconds"
            ),
        },
        StoreValueKind::Stored => StoreError::CorruptStoredValue {
            database_kind: REGISTRY_DATABASE_KIND,
            field: "host_capability_verifications.expires_at",
        },
    }
}

fn validate_capability(value: &str) -> StoreResult<()> {
    if value == HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "capability must be model_invisible_user_surface".to_owned(),
        })
    }
}

fn validate_adapter_profile(value: &str) -> StoreResult<()> {
    if value == HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1 {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "adapter_profile must be mcp_user_channel_local_web_v1".to_owned(),
        })
    }
}

fn validate_outcome(value: &str) -> StoreResult<()> {
    if is_outcome(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "outcome must be passed, failed, unavailable, or revoked".to_owned(),
        })
    }
}

fn is_outcome(value: &str) -> bool {
    matches!(
        value,
        HOST_CAPABILITY_OUTCOME_PASSED
            | HOST_CAPABILITY_OUTCOME_FAILED
            | HOST_CAPABILITY_OUTCOME_UNAVAILABLE
            | HOST_CAPABILITY_OUTCOME_REVOKED
    )
}

fn validate_host_kind(value: &str) -> StoreResult<()> {
    if is_host_kind(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "host_kind must be codex, claude_code, or generic".to_owned(),
        })
    }
}

fn is_host_kind(value: &str) -> bool {
    matches!(
        value,
        HOST_KIND_CODEX | HOST_KIND_CLAUDE_CODE | HOST_KIND_GENERIC
    )
}

fn validate_nonempty(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be nonempty text without control characters"),
        })
    } else {
        Ok(())
    }
}

fn validate_stored_nonempty(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        corrupt(field)
    } else {
        Ok(())
    }
}

fn validate_sha256(field: &'static str, value: &str) -> StoreResult<()> {
    if is_sha256(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be lowercase 64-character SHA-256 hex"),
        })
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_input_timestamp(field: &'static str, value: &str) -> StoreResult<UtcTimestamp> {
    let timestamp = UtcTimestamp::parse(value)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map(|()| timestamp)
                .map_err(|_| volicord_types::UtcTimestampParseError)
        })
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be a canonical RFC 3339 UTC timestamp"),
        })?;
    if value != timestamp.to_canonical_string() {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must use canonical RFC 3339 UTC text"),
        });
    }
    Ok(timestamp)
}

fn parse_stored_timestamp(field: &'static str, value: &str) -> StoreResult<UtcTimestamp> {
    let timestamp = UtcTimestamp::parse(value).map_err(|_| StoreError::CorruptStoredValue {
        database_kind: REGISTRY_DATABASE_KIND,
        field,
    })?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::CorruptStoredValue {
            database_kind: REGISTRY_DATABASE_KIND,
            field,
        })?;
    if value != timestamp.to_canonical_string() {
        return Err(StoreError::CorruptStoredValue {
            database_kind: REGISTRY_DATABASE_KIND,
            field,
        });
    }
    Ok(timestamp)
}

fn corrupt<T>(field: &'static str) -> StoreResult<T> {
    Err(StoreError::CorruptStoredValue {
        database_kind: REGISTRY_DATABASE_KIND,
        field,
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::{
        agent_connections::{
            ensure_agent_connection, remove_agent_connection_if_unused,
            AgentConnectionRegistration, CONNECTION_INTENT_PERSONAL, CONNECTION_MODE_WORKFLOW,
            HOST_SCOPE_USER, VERIFIED_STATUS_COMPLETE,
        },
        bootstrap::initialize_runtime_home,
        sqlite::{open_registry_database, set_foreign_keys},
    };

    #[test]
    fn passing_verification_is_current_and_half_open() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-pass", HOST_KIND_CODEX)?;
        let input = verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED);
        let record = publish_host_capability_verification(fixture.path(), input.clone())?;

        assert_eq!(
            current_host_capability_verification_read_only(
                fixture.path(),
                "conn_a",
                HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE,
            )?,
            Some(record.clone())
        );
        assert_eq!(
            evaluate_current_host_capability_verification_read_only(
                fixture.path(),
                &expectation(),
                &input.observed_at,
            )?,
            Some(record.clone())
        );
        assert_eq!(
            evaluate_current_host_capability_verification_read_only(
                fixture.path(),
                &expectation(),
                "2026-07-14T23:59:59.999999999Z",
            )?,
            Some(record)
        );
        assert!(evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            &input.expires_at,
        )?
        .is_none());
        Ok(())
    }

    #[test]
    fn unavailable_or_revoked_current_never_falls_back_to_older_pass() -> Result<(), Box<dyn Error>>
    {
        let fixture = fixture("host-capability-superseded", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_pass", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let mut unavailable = verification(
            "verification_unavailable",
            HOST_CAPABILITY_OUTCOME_UNAVAILABLE,
        );
        unavailable.observed_at = "2026-07-14T01:00:00Z".to_owned();
        unavailable.expires_at = "2026-07-15T01:00:00Z".to_owned();
        unavailable.created_at = "2026-07-14T01:00:01Z".to_owned();
        unavailable.evidence_artifact_sha256 = digest('c');
        publish_host_capability_verification(fixture.path(), unavailable)?;

        assert!(evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            "2026-07-14T02:00:00Z",
        )?
        .is_none());

        let stale = verification("verification_stale", HOST_CAPABILITY_OUTCOME_PASSED);
        let error = publish_host_capability_verification(fixture.path(), stale)
            .expect_err("an older pass must not replace unavailable current state");
        assert!(matches!(error, StoreError::Conflict { .. }));

        let mut revoked = verification("verification_revoked", HOST_CAPABILITY_OUTCOME_REVOKED);
        revoked.observed_at = "2026-07-14T02:00:00Z".to_owned();
        revoked.expires_at = "2026-07-15T02:00:00Z".to_owned();
        revoked.created_at = "2026-07-14T02:00:01Z".to_owned();
        revoked.evidence_artifact_sha256 = digest('d');
        publish_host_capability_verification(fixture.path(), revoked)?;
        assert!(evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            "2026-07-14T03:00:00Z",
        )?
        .is_none());
        Ok(())
    }

    #[test]
    fn exact_publish_retry_is_idempotent_and_never_rolls_back_current() -> Result<(), Box<dyn Error>>
    {
        let fixture = fixture("host-capability-idempotent", HOST_KIND_CODEX)?;
        let first = verification("verification_pass", HOST_CAPABILITY_OUTCOME_PASSED);
        let first_record = publish_host_capability_verification(fixture.path(), first.clone())?;
        assert_eq!(
            publish_host_capability_verification(fixture.path(), first.clone())?,
            first_record
        );

        let mut newer = verification("verification_failed", HOST_CAPABILITY_OUTCOME_FAILED);
        newer.observed_at = "2026-07-14T01:00:00Z".to_owned();
        newer.expires_at = "2026-07-15T01:00:00Z".to_owned();
        newer.created_at = "2026-07-14T01:00:01Z".to_owned();
        newer.evidence_artifact_sha256 = digest('c');
        let newer_record = publish_host_capability_verification(fixture.path(), newer)?;

        assert_eq!(
            publish_host_capability_verification(fixture.path(), first.clone())?,
            first_record
        );
        assert_eq!(
            current_host_capability_verification_read_only(
                fixture.path(),
                "conn_a",
                HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE,
            )?,
            Some(newer_record)
        );

        let conn = open_registry_database(fixture.registry_db_path())?;
        let history_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM host_capability_verifications",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(history_count, 2);
        drop(conn);

        let mut conflicting = first;
        conflicting.metadata_json = r#"{"different":true}"#.to_owned();
        assert!(matches!(
            publish_host_capability_verification(fixture.path(), conflicting),
            Err(StoreError::Conflict { .. })
        ));
        Ok(())
    }

    #[test]
    fn every_exact_binding_mismatch_is_ineligible() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-mismatch", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let baseline = expectation();

        let mismatches = [
            HostCapabilityVerificationExpectation {
                host_kind: HOST_KIND_CLAUDE_CODE.to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                host_version: "2.0.0".to_owned(),
                client_version: "2.0.0".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                client_name: "other-client".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                adapter_version: "adapter-2".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                managed_fingerprint: "fingerprint-other".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                volicord_build_id: "build-other".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                source_revision: "revision-other".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                target_triple: "target-other".to_owned(),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                executable_sha256: digest('e'),
                ..baseline.clone()
            },
            HostCapabilityVerificationExpectation {
                evidence_artifact_sha256: digest('f'),
                ..baseline
            },
        ];
        for mismatch in mismatches {
            assert!(evaluate_current_host_capability_verification_read_only(
                fixture.path(),
                &mismatch,
                "2026-07-14T12:00:00Z",
            )?
            .is_none());
        }
        Ok(())
    }

    #[test]
    fn disabled_connection_makes_current_pass_ineligible() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-disabled", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let conn = open_registry_database(fixture.registry_db_path())?;
        conn.execute(
            "UPDATE agent_connections SET enabled = 0 WHERE connection_internal_id = 'conn_a'",
            [],
        )?;
        drop(conn);

        assert!(evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            "2026-07-14T12:00:00Z",
        )?
        .is_none());
        Ok(())
    }

    #[test]
    fn generic_or_connection_mismatched_pass_is_rejected() -> Result<(), Box<dyn Error>> {
        let generic_fixture = fixture("host-capability-generic", HOST_KIND_GENERIC)?;
        let mut generic = verification("verification_generic", HOST_CAPABILITY_OUTCOME_PASSED);
        generic.host_kind = HOST_KIND_GENERIC.to_owned();
        assert!(matches!(
            publish_host_capability_verification(generic_fixture.path(), generic),
            Err(StoreError::InvalidInput { .. })
        ));

        let fixture = fixture("host-capability-identity", HOST_KIND_CODEX)?;
        let mut mismatch = verification("verification_mismatch", HOST_CAPABILITY_OUTCOME_PASSED);
        mismatch.managed_fingerprint = "wrong-fingerprint".to_owned();
        assert!(matches!(
            publish_host_capability_verification(fixture.path(), mismatch),
            Err(StoreError::InvalidInput { .. })
        ));
        Ok(())
    }

    #[test]
    fn verification_window_is_positive_and_at_most_twenty_four_hours() -> Result<(), Box<dyn Error>>
    {
        let fixture = fixture("host-capability-window", HOST_KIND_CODEX)?;
        for (id, expires_at) in [
            ("verification_zero", "2026-07-14T00:00:00Z"),
            ("verification_long", "2026-07-15T00:00:00.000000001Z"),
        ] {
            let mut input = verification(id, HOST_CAPABILITY_OUTCOME_PASSED);
            input.expires_at = expires_at.to_owned();
            assert!(matches!(
                publish_host_capability_verification(fixture.path(), input),
                Err(StoreError::InvalidInput { .. })
            ));
        }
        Ok(())
    }

    #[test]
    fn publication_time_must_follow_observation_and_precede_pass_expiry(
    ) -> Result<(), Box<dyn Error>> {
        let equality_fixture = fixture("host-capability-publish-time", HOST_KIND_CODEX)?;

        let mut equality = verification("verification_equal", HOST_CAPABILITY_OUTCOME_PASSED);
        equality.created_at = equality.observed_at.clone();
        publish_host_capability_verification(equality_fixture.path(), equality)?;

        let other_fixture = fixture("host-capability-time-reject", HOST_KIND_CODEX)?;
        let mut before_observation =
            verification("verification_before", HOST_CAPABILITY_OUTCOME_PASSED);
        before_observation.created_at = "2026-07-13T23:59:59Z".to_owned();
        assert!(matches!(
            publish_host_capability_verification(other_fixture.path(), before_observation),
            Err(StoreError::InvalidInput { .. })
        ));

        let mut expired = verification("verification_expired", HOST_CAPABILITY_OUTCOME_PASSED);
        expired.created_at = expired.expires_at.clone();
        assert!(matches!(
            publish_host_capability_verification(other_fixture.path(), expired),
            Err(StoreError::InvalidInput { .. })
        ));
        Ok(())
    }

    #[test]
    fn corrupt_current_record_fails_closed() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-corrupt", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let conn = open_registry_database(fixture.registry_db_path())?;
        conn.execute(
            "UPDATE host_capability_verifications
                SET observed_at = 'not-a-timestamp'
              WHERE verification_internal_id = 'verification_a'",
            [],
        )?;
        drop(conn);

        let error = evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            "2026-07-14T12:00:00Z",
        )
        .expect_err("corrupt current state must not become ordinary eligibility");
        assert!(matches!(error, StoreError::CorruptStoredValue { .. }));
        Ok(())
    }

    #[test]
    fn nonempty_stored_v1_metadata_fails_closed() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-metadata-corrupt", HOST_KIND_CODEX)?;
        let mut rejected = verification("verification_rejected", HOST_CAPABILITY_OUTCOME_PASSED);
        rejected.metadata_json = r#"{"raw":true}"#.to_owned();
        assert!(matches!(
            publish_host_capability_verification(fixture.path(), rejected),
            Err(StoreError::InvalidInput { .. })
        ));
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let conn = open_registry_database(fixture.registry_db_path())?;
        conn.pragma_update(None, "ignore_check_constraints", "ON")?;
        conn.execute(
            "UPDATE host_capability_verifications
                SET metadata_json = '{\"raw\":true}'
              WHERE verification_internal_id = 'verification_a'",
            [],
        )?;
        drop(conn);

        let error = evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            "2026-07-14T12:00:00Z",
        )
        .expect_err("nonempty v1 metadata must not enter eligibility");
        assert!(matches!(error, StoreError::CorruptStoredValue { .. }));
        Ok(())
    }

    #[test]
    fn connection_deletion_cascades_current_state_and_history() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-cascade", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        assert!(remove_agent_connection_if_unused(fixture.path(), "conn_a")?);

        let conn = open_registry_database(fixture.registry_db_path())?;
        let history_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM host_capability_verifications",
            [],
            |row| row.get(0),
        )?;
        let state_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM host_capability_state", [], |row| {
                row.get(0)
            })?;
        assert_eq!((history_count, state_count), (0, 0));
        Ok(())
    }

    #[test]
    fn dangling_current_pointer_is_corrupt_even_when_foreign_keys_were_bypassed(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-dangling", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let conn = open_registry_database(fixture.registry_db_path())?;
        set_foreign_keys(&conn, false)?;
        conn.execute(
            "UPDATE host_capability_state
                SET current_verification_internal_id = 'missing_verification'",
            [],
        )?;
        drop(conn);

        assert!(current_host_capability_verification_read_only(
            fixture.path(),
            "conn_a",
            HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE,
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn current_pointer_timestamp_must_equal_immutable_creation_time() -> Result<(), Box<dyn Error>>
    {
        let fixture = fixture("host-capability-pointer-time", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let conn = open_registry_database(fixture.registry_db_path())?;
        conn.execute(
            "UPDATE host_capability_state
                SET updated_at = '2026-07-14T00:00:02Z'",
            [],
        )?;
        drop(conn);

        let error = current_host_capability_verification_read_only(
            fixture.path(),
            "conn_a",
            HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE,
        )
        .expect_err("a current pointer with unrelated update time must fail closed");
        assert!(matches!(error, StoreError::CorruptStoredValue { .. }));
        Ok(())
    }

    #[test]
    fn current_pointer_cannot_be_rolled_back_to_an_older_pass() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-pointer-rollback", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_pass", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let mut newer = verification("verification_failed", HOST_CAPABILITY_OUTCOME_FAILED);
        newer.observed_at = "2026-07-14T01:00:00Z".to_owned();
        newer.expires_at = "2026-07-15T01:00:00Z".to_owned();
        newer.created_at = "2026-07-14T01:00:01Z".to_owned();
        newer.evidence_artifact_sha256 = digest('c');
        publish_host_capability_verification(fixture.path(), newer)?;

        let conn = open_registry_database(fixture.registry_db_path())?;
        conn.execute(
            "UPDATE host_capability_state
                SET current_verification_internal_id = 'verification_pass',
                    updated_at = '2026-07-14T00:00:01Z'",
            [],
        )?;
        drop(conn);

        let error = evaluate_current_host_capability_verification_read_only(
            fixture.path(),
            &expectation(),
            "2026-07-14T02:00:00Z",
        )
        .expect_err("a structurally rolled-back pointer must fail closed");
        assert!(matches!(error, StoreError::CorruptStoredValue { .. }));
        Ok(())
    }

    #[test]
    fn current_pointer_requires_one_unique_newest_observation() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("host-capability-pointer-tie", HOST_KIND_CODEX)?;
        publish_host_capability_verification(
            fixture.path(),
            verification("verification_a", HOST_CAPABILITY_OUTCOME_PASSED),
        )?;
        let conn = open_registry_database(fixture.registry_db_path())?;
        conn.execute(
            "INSERT INTO host_capability_verifications (
                verification_internal_id, connection_internal_id, capability,
                outcome, host_kind, host_version, client_name, client_version,
                adapter_profile, adapter_version, managed_fingerprint,
                volicord_build_id, source_revision, target_triple,
                executable_sha256, evidence_artifact_sha256,
                observed_at, expires_at, metadata_json, created_at
            )
            SELECT
                'verification_tie', connection_internal_id, capability,
                'failed', host_kind, host_version, client_name, client_version,
                adapter_profile, adapter_version, managed_fingerprint,
                volicord_build_id, source_revision, target_triple,
                executable_sha256,
                'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
                observed_at, expires_at, metadata_json, '2026-07-14T00:00:02Z'
              FROM host_capability_verifications
             WHERE verification_internal_id = 'verification_a'",
            [],
        )?;
        drop(conn);

        let error = current_host_capability_verification_read_only(
            fixture.path(),
            "conn_a",
            HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE,
        )
        .expect_err("a tied newest observation must make the pointer corrupt");
        assert!(matches!(error, StoreError::CorruptStoredValue { .. }));
        Ok(())
    }

    fn fixture(name: &str, host_kind: &str) -> StoreResult<TempRuntimeHome> {
        let runtime_home = TempRuntimeHome::new(name)?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_test", "{}")?;
        ensure_agent_connection(
            runtime_home.path(),
            AgentConnectionRegistration {
                connection_internal_id: "conn_a".to_owned(),
                host_kind: host_kind.to_owned(),
                intent: CONNECTION_INTENT_PERSONAL.to_owned(),
                host_scope: if host_kind == HOST_KIND_GENERIC {
                    crate::agent_connections::HOST_SCOPE_EXPORT.to_owned()
                } else {
                    HOST_SCOPE_USER.to_owned()
                },
                server_name: "volicord".to_owned(),
                config_target: format!("{host_kind}-target"),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: "fingerprint-a".to_owned(),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok(runtime_home)
    }

    fn verification(id: &str, outcome: &str) -> HostCapabilityVerificationInput {
        HostCapabilityVerificationInput {
            verification_internal_id: id.to_owned(),
            connection_internal_id: "conn_a".to_owned(),
            capability: HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE.to_owned(),
            outcome: outcome.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            host_version: "1.2.3".to_owned(),
            client_name: "codex-mcp-client".to_owned(),
            client_version: "1.2.3".to_owned(),
            adapter_profile: HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1.to_owned(),
            adapter_version: "adapter-1".to_owned(),
            managed_fingerprint: "fingerprint-a".to_owned(),
            volicord_build_id: "build-a".to_owned(),
            source_revision: git_object_id('1'),
            target_triple: "x86_64-unknown-linux-gnu".to_owned(),
            executable_sha256: digest('a'),
            evidence_artifact_sha256: digest('b'),
            observed_at: "2026-07-14T00:00:00Z".to_owned(),
            expires_at: "2026-07-15T00:00:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
            created_at: "2026-07-14T00:00:01Z".to_owned(),
        }
    }

    fn expectation() -> HostCapabilityVerificationExpectation {
        HostCapabilityVerificationExpectation {
            connection_internal_id: "conn_a".to_owned(),
            capability: HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            host_version: "1.2.3".to_owned(),
            client_name: "codex-mcp-client".to_owned(),
            client_version: "1.2.3".to_owned(),
            adapter_profile: HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1.to_owned(),
            adapter_version: "adapter-1".to_owned(),
            managed_fingerprint: "fingerprint-a".to_owned(),
            volicord_build_id: "build-a".to_owned(),
            source_revision: git_object_id('1'),
            target_triple: "x86_64-unknown-linux-gnu".to_owned(),
            executable_sha256: digest('a'),
            evidence_artifact_sha256: digest('b'),
        }
    }

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn git_object_id(character: char) -> String {
        std::iter::repeat_n(character, 40).collect()
    }
}
